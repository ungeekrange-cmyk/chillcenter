//! Static service settings and independently persisted, administrator-editable library settings.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    net::SocketAddr,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct ServerConfig {
    pub listen: SocketAddr,
    /// Optional extra listener in plain HTTP, for the bundled web client only (ADR-0018). Off unless configured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_listen: Option<SocketAddr>,
    pub data_dir: PathBuf,
    pub tls_names: Vec<String>,
    pub video_root: Option<PathBuf>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen: "127.0.0.1:8443".parse().expect("literal socket address"),
            http_listen: None,
            data_dir: PathBuf::from("/var/lib/chillcenter"),
            tls_names: vec!["localhost".into(), "127.0.0.1".into()],
            video_root: None,
        }
    }
}

impl ServerConfig {
    pub fn load(path: &Path) -> Result<Self, String> {
        let data = fs::read(path).map_err(|_| "Configuration du serveur inaccessible.")?;
        if data.len() > 64 * 1024 {
            return Err("Configuration trop volumineuse.".into());
        }
        let config: Self =
            serde_json::from_slice(&data).map_err(|_| "Configuration JSON invalide.")?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_absolute(&self.data_dir)?;
        if let Some(root) = &self.video_root {
            validate_absolute(root)?;
        }
        if self.listen.port() == 0 {
            return Err("Le port doit être supérieur à zéro.".into());
        }
        if !is_local_address(self.listen.ip()) {
            return Err("Sans authentification, écoutez une adresse LAN explicite ou loopback, pas toutes les interfaces/public Internet.".into());
        }
        if let Some(http) = self.http_listen {
            if http.port() == 0 || !is_local_address(http.ip()) {
                return Err("Le listener HTTP doit utiliser une adresse LAN explicite ou loopback et un port non nul.".into());
            }
            if http.port() == self.listen.port() {
                return Err(
                    "Le listener HTTP doit avoir un port différent du listener HTTPS.".into(),
                );
            }
        }
        if self.tls_names.is_empty() || self.tls_names.len() > 32 {
            return Err("Configurez entre 1 et 32 noms DNS/adresses IP pour HTTPS.".into());
        }
        for name in &self.tls_names {
            let ip = name.parse::<std::net::IpAddr>().is_ok();
            let dns = !name.is_empty()
                && name.len() <= 253
                && !name.ends_with('.')
                && name.split('.').all(|label| {
                    !label.is_empty()
                        && label.len() <= 63
                        && !label.starts_with('-')
                        && !label.ends_with('-')
                        && label
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
                });
            if !ip && !dns {
                return Err("Nom DNS/adresse IP HTTPS invalide (sans protocole ni port).".into());
            }
        }
        Ok(())
    }
}

pub fn is_local_address(address: std::net::IpAddr) -> bool {
    match address {
        std::net::IpAddr::V4(ip) => ip.is_loopback() || ip.is_private() || ip.is_link_local(),
        std::net::IpAddr::V6(ip) => {
            ip.is_loopback() || ip.is_unique_local() || ip.is_unicast_link_local()
        }
    }
}

/// Refuse symlink components and unsafe service-directory ownership before opening SQLite/TLS.
pub fn prepare_data_dir(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    validate_absolute(path)?;
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err("Le stockage local ne doit pas traverser un lien symbolique.".into());
            }
            Ok(metadata) => {
                let own = metadata.uid() == unsafe { libc::geteuid() } || metadata.uid() == 0;
                let writable_without_sticky =
                    metadata.mode() & 0o022 != 0 && metadata.mode() & 0o1000 == 0;
                if !own || writable_without_sticky {
                    return Err("Un parent du stockage local est insuffisamment protégé.".into());
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("Stockage local inaccessible.".into()),
        }
    }
    let existed = path.exists();
    fs::create_dir_all(path).map_err(|_| "Stockage local inaccessible.")?;
    let metadata = fs::symlink_metadata(path).map_err(|_| "Stockage local inaccessible.")?;
    // SAFETY: geteuid has no pointer arguments and cannot fail.
    if !metadata.is_dir()
        || metadata.uid() != unsafe { libc::geteuid() }
        || (existed && metadata.mode() & 0o022 != 0)
    {
        return Err(
            "Le stockage doit appartenir au compte du service, sans écriture groupe/autres.".into(),
        );
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|_| "Protection du stockage impossible.")?;
    Ok(())
}

/// This file is owned by the service user, unlike the static /etc configuration.
/// Its presence, even with `None`, takes precedence over the bootstrap root.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct LibrarySettings {
    pub video_root: Option<PathBuf>,
    /// Extra independent library roots.  The primary root deliberately keeps
    /// its historic ids; extra roots receive a virtual, non-filesystem prefix.
    #[serde(default)]
    pub additional_video_roots: Vec<PathBuf>,
    pub revision: u64,
}

impl LibrarySettings {
    pub fn load(config: &ServerConfig) -> Result<Self, String> {
        let path = config.data_dir.join("library.json");
        match fs::read(&path) {
            Ok(data) if data.len() <= 64 * 1024 => {
                let settings: Self = serde_json::from_slice(&data)
                    .map_err(|_| "Réglages de bibliothèque invalides.")?;
                settings.validate_roots()?;
                Ok(settings)
            }
            Ok(_) => Err("Réglages de bibliothèque trop volumineux.".into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self {
                video_root: config.video_root.clone(),
                additional_video_roots: Vec::new(),
                revision: 0,
            }),
            Err(_) => Err("Réglages de bibliothèque inaccessibles.".into()),
        }
    }

    /// Validation is syntactic: an temporarily offline NAS must remain configurable.
    /// Availability is reported separately and never triggers a catalogue purge.
    pub fn update(
        &mut self,
        config: &ServerConfig,
        root: Option<PathBuf>,
        additional_roots: Option<Vec<PathBuf>>,
        expected_revision: u64,
    ) -> Result<(), String> {
        self.update_with_writer(
            config,
            root,
            additional_roots,
            expected_revision,
            atomic_private_write,
        )
    }
    fn update_with_writer(
        &mut self,
        config: &ServerConfig,
        root: Option<PathBuf>,
        additional_roots: Option<Vec<PathBuf>>,
        expected_revision: u64,
        writer: impl FnOnce(&Path, &[u8]) -> Result<(), String>,
    ) -> Result<(), String> {
        if self.revision != expected_revision {
            return Err("CONFIG_CONFLICT".into());
        }
        let additional_video_roots =
            additional_roots.unwrap_or_else(|| self.additional_video_roots.clone());
        let updated = Self {
            video_root: root,
            additional_video_roots,
            revision: self.revision.checked_add(1).ok_or("Révision invalide.")?,
        };
        updated.validate_roots()?;
        let data = serde_json::to_vec_pretty(&updated).map_err(|_| "Sérialisation impossible.")?;
        let published = writer(&config.data_dir.join("library.json"), &data);
        if let Err(error) = &published
            && error != "CONFIG_DURABILITY_UNKNOWN"
        {
            return published;
        }
        *self = updated;
        published
    }

    pub fn roots(&self) -> Vec<PathBuf> {
        self.video_root
            .iter()
            .chain(self.additional_video_roots.iter())
            .cloned()
            .collect()
    }

    /// Return the configured root owning an indexed absolute path.  Roots are
    /// non-overlapping, so this cannot accidentally select a broader mount.
    pub fn root_for_path(&self, path: &Path) -> Option<PathBuf> {
        self.roots().into_iter().find(|root| path.starts_with(root))
    }

    /// Primary media ids remain unchanged for compatibility.  Additional roots
    /// live in a reserved virtual namespace which can never be a primary file.
    pub fn indexed_id(&self, root: &Path, path: &Path) -> Result<String, String> {
        let relative = path
            .strip_prefix(root)
            .map_err(|_| "Chemin hors bibliothèque.")?;
        if root == self.video_root.as_deref().unwrap_or(Path::new("")) {
            let id = relative.to_string_lossy().into_owned();
            if id.starts_with(".chillcenter-locations/") {
                return Err(
                    "Le préfixe .chillcenter-locations est réservé par Chill Center.".into(),
                );
            }
            return Ok(id);
        }
        let mut digest = Sha256::new();
        digest.update(root.as_os_str().as_encoded_bytes());
        let namespace = format!(".chillcenter-locations/{:x}", digest.finalize());
        Ok(format!("{}/{}", namespace, relative.to_string_lossy()))
    }

    fn validate_roots(&self) -> Result<(), String> {
        let roots = self.roots();
        if roots.len() > 16 {
            return Err("Configurez au plus 16 emplacements de bibliothèque.".into());
        }
        for root in &roots {
            validate_absolute(root)?;
        }
        for (index, root) in roots.iter().enumerate() {
            if roots.iter().enumerate().any(|(other, candidate)| {
                other != index
                    && (candidate == root
                        || candidate.starts_with(root)
                        || root.starts_with(candidate))
            }) {
                return Err("Les emplacements de bibliothèque doivent être distincts et ne pas se chevaucher.".into());
            }
        }
        Ok(())
    }
}

fn validate_absolute(path: &Path) -> Result<(), String> {
    if !path.is_absolute()
        || path == Path::new("/")
        || path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
        || path.as_os_str().len() > 4096
    {
        return Err("Utilisez un chemin absolu dédié, sans composant '..'.".into());
    }
    Ok(())
}

/// Unique staging file, restrictive permissions, fsync, rename, parent fsync.
/// No truncation of an existing file and no private key visible through a permissive umask.
pub(crate) fn atomic_private_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    atomic_private_write_with_sync(path, bytes, |parent| {
        fs::File::open(parent).and_then(|directory| directory.sync_all())
    })
}
fn atomic_private_write_with_sync(
    path: &Path,
    bytes: &[u8],
    sync: impl FnOnce(&Path) -> std::io::Result<()>,
) -> Result<(), String> {
    use std::os::unix::fs::OpenOptionsExt;
    let parent = path.parent().ok_or("Chemin de configuration invalide.")?;
    let stage = parent.join(format!(".chillcenter-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&stage)
            .map_err(|_| "Écriture de configuration impossible.")?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| "Écriture de configuration impossible.")?;
        fs::rename(&stage, path).map_err(|_| "Enregistrement de configuration impossible.")?;
        sync(parent).map_err(|_| "CONFIG_DURABILITY_UNKNOWN")?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&stage);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rename_publication_and_directory_durability_are_distinguished() {
        let dir = tempfile::tempdir().unwrap();
        let config = ServerConfig {
            data_dir: dir.path().into(),
            ..Default::default()
        };
        let mut settings = LibrarySettings::load(&config).unwrap();
        let error = settings
            .update_with_writer(&config, Some("/mnt/new".into()), None, 0, |path, bytes| {
                atomic_private_write_with_sync(path, bytes, |_| {
                    Err(std::io::Error::other("injected fsync"))
                })
            })
            .unwrap_err();
        assert_eq!(error, "CONFIG_DURABILITY_UNKNOWN");
        assert_eq!(settings.revision, 1);
        assert_eq!(LibrarySettings::load(&config).unwrap(), settings);
        assert_eq!(
            settings
                .update(&config, Some("/mnt/stale".into()), None, 0)
                .unwrap_err(),
            "CONFIG_CONFLICT"
        );
        let before = settings.clone();
        let published = fs::read(dir.path().join("library.json")).unwrap();
        assert!(
            settings
                .update_with_writer(&config, None, None, 1, |_, _| Err("before rename".into()))
                .is_err()
        );
        assert_eq!(settings, before);
        assert_eq!(
            fs::read(dir.path().join("library.json")).unwrap(),
            published
        );
        assert_eq!(
            fs::read_dir(dir.path()).unwrap().count(),
            1,
            "owned stage cleaned after error"
        );
    }

    #[test]
    fn address_and_mount_are_not_fixed() {
        let config = ServerConfig {
            listen: "10.10.0.8:9443".parse().unwrap(),
            tls_names: vec!["media.home".into(), "10.10.0.8".into()],
            video_root: Some("/mnt/other-nas/films".into()),
            ..ServerConfig::default()
        };
        config.validate().unwrap();
        assert_eq!(
            serde_json::from_str::<ServerConfig>(&serde_json::to_string(&config).unwrap()).unwrap(),
            config
        );
    }

    #[test]
    fn admin_change_is_persistent_and_revision_checked() {
        let dir = tempfile::tempdir().unwrap();
        let config = ServerConfig {
            data_dir: dir.path().to_owned(),
            video_root: Some("/mnt/initial".into()),
            ..ServerConfig::default()
        };
        let mut settings = LibrarySettings::load(&config).unwrap();
        settings
            .update(&config, Some("/mnt/new-nas".into()), None, 0)
            .unwrap();
        assert_eq!(LibrarySettings::load(&config).unwrap(), settings);
        assert!(
            settings
                .update(&config, Some("/mnt/stale".into()), None, 0)
                .is_err()
        );
        assert_eq!(settings.video_root, Some("/mnt/new-nas".into()));
        settings.update(&config, None, None, 1).unwrap();
        assert_eq!(LibrarySettings::load(&config).unwrap().video_root, None);
    }

    #[test]
    fn extra_locations_are_persisted_and_legacy_updates_preserve_them() {
        let dir = tempfile::tempdir().unwrap();
        let config = ServerConfig {
            data_dir: dir.path().to_owned(),
            video_root: Some("/mnt/films".into()),
            ..ServerConfig::default()
        };
        let mut settings = LibrarySettings::load(&config).unwrap();
        settings
            .update(
                &config,
                Some("/mnt/films".into()),
                Some(vec!["/mnt/series".into(), "/mnt/documentaires".into()]),
                0,
            )
            .unwrap();
        // The v1-shaped update has no extra-roots field and must not erase it.
        settings
            .update(&config, Some("/mnt/films-2".into()), None, 1)
            .unwrap();
        assert_eq!(
            settings.additional_video_roots,
            vec![
                PathBuf::from("/mnt/series"),
                PathBuf::from("/mnt/documentaires")
            ]
        );
        assert_eq!(LibrarySettings::load(&config).unwrap(), settings);
    }

    #[test]
    fn extra_root_ids_are_distinct_and_primary_namespace_is_reserved() {
        let settings = LibrarySettings {
            video_root: Some("/library/films".into()),
            additional_video_roots: vec!["/library/series".into()],
            revision: 4,
        };
        settings.validate_roots().unwrap();
        assert_eq!(
            settings
                .indexed_id(
                    Path::new("/library/films"),
                    Path::new("/library/films/same.mkv")
                )
                .unwrap(),
            "same.mkv"
        );
        let additional = settings
            .indexed_id(
                Path::new("/library/series"),
                Path::new("/library/series/same.mkv"),
            )
            .unwrap();
        assert!(additional.starts_with(".chillcenter-locations/"));
        assert!(additional.ends_with("/same.mkv"));
        assert_eq!(
            settings.root_for_path(Path::new("/library/series/same.mkv")),
            Some(PathBuf::from("/library/series"))
        );
        assert!(
            settings
                .indexed_id(
                    Path::new("/library/films"),
                    Path::new("/library/films/.chillcenter-locations/x.mkv")
                )
                .is_err()
        );
    }

    #[test]
    fn duplicate_or_overlapping_locations_are_refused() {
        for roots in [
            vec![PathBuf::from("/mnt/films")],
            vec![PathBuf::from("/mnt/films/child")],
        ] {
            let settings = LibrarySettings {
                video_root: Some("/mnt/films".into()),
                additional_video_roots: roots,
                revision: 0,
            };
            assert!(settings.validate_roots().is_err());
        }
    }

    #[test]
    fn invalid_roots_and_names_are_refused() {
        for root in ["relative", "/", "/srv/../etc"] {
            assert!(validate_absolute(Path::new(root)).is_err());
        }
        for name in [
            "https://media.home",
            "media.home:8443",
            "-bad.home",
            "*.home",
        ] {
            let config = ServerConfig {
                tls_names: vec![name.into()],
                ..ServerConfig::default()
            };
            assert!(config.validate().is_err());
        }
    }

    #[test]
    fn distributed_examples_default_to_loopback_not_a_household_target() {
        for source in [
            include_str!("../../../deploy/server.example.json"),
            include_str!("../../../deploy/linux/server.json.example"),
        ] {
            let example: ServerConfig = serde_json::from_str(source).unwrap();
            assert!(example.listen.ip().is_loopback());
            assert_eq!(example.listen.port(), 8443);
            assert_eq!(example.tls_names, ["localhost", "127.0.0.1"]);
            example.validate().unwrap();
        }
    }

    #[test]
    fn http_listener_is_optional_local_and_on_another_port() {
        let base = ServerConfig::default();
        assert!(base.validate().is_ok());
        let ok = ServerConfig {
            http_listen: Some("127.0.0.1:8080".parse().unwrap()),
            ..base.clone()
        };
        assert!(ok.validate().is_ok());
        for bad in [
            "127.0.0.1:8443",
            "127.0.0.1:0",
            "0.0.0.0:8080",
            "8.8.8.8:8080",
        ] {
            let config = ServerConfig {
                http_listen: Some(bad.parse().unwrap()),
                ..base.clone()
            };
            assert!(config.validate().is_err(), "{bad}");
        }
        // Absent field: parsed as None and never written back, so a config without it stays byte-compatible.
        let parsed: ServerConfig = serde_json::from_str(r#"{"listen":"127.0.0.1:8443"}"#).unwrap();
        assert_eq!(parsed.http_listen, None);
        assert!(
            !serde_json::to_string(&parsed)
                .unwrap()
                .contains("http_listen")
        );
        let with: ServerConfig = serde_json::from_str(
            r#"{"http_listen":"192.168.50.9:8080","listen":"192.168.50.9:8443"}"#,
        )
        .unwrap();
        assert_eq!(with.http_listen, Some("192.168.50.9:8080".parse().unwrap()));
    }

    #[test]
    fn wildcard_and_public_unauthenticated_listeners_are_refused() {
        for address in [
            "0.0.0.0:8443",
            "[::]:8443",
            "8.8.8.8:8443",
            "[2001:4860:4860::8888]:8443",
        ] {
            let config = ServerConfig {
                listen: address.parse().unwrap(),
                ..ServerConfig::default()
            };
            assert!(config.validate().is_err());
        }
        let config = ServerConfig {
            listen: "192.168.50.9:9443".parse().unwrap(),
            ..ServerConfig::default()
        };
        config.validate().unwrap();
    }

    #[test]
    fn data_directory_rejects_symlinks_and_unsafe_permissions() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let dir = tempfile::tempdir().unwrap();
        let canonical = dir.path().canonicalize().unwrap();
        let actual = canonical.join("actual");
        prepare_data_dir(&actual).unwrap();
        let alias = canonical.join("alias");
        symlink(&actual, &alias).unwrap();
        assert!(prepare_data_dir(&alias).is_err());
        fs::set_permissions(&actual, fs::Permissions::from_mode(0o777)).unwrap();
        assert!(prepare_data_dir(&actual).is_err());
    }

    #[test]
    fn failed_write_does_not_change_memory() {
        let dir = tempfile::tempdir().unwrap();
        let config = ServerConfig {
            data_dir: dir.path().join("missing"),
            ..ServerConfig::default()
        };
        let mut settings = LibrarySettings::default();
        assert!(
            settings
                .update(&config, Some("/srv/videos".into()), None, 0)
                .is_err()
        );
        assert_eq!(settings, LibrarySettings::default());
    }
}
