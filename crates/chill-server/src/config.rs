//! Static service settings and independently persisted, administrator-editable library settings.
use serde::{Deserialize, Serialize};
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
    pub data_dir: PathBuf,
    pub tls_names: Vec<String>,
    pub video_root: Option<PathBuf>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen: "127.0.0.1:8443".parse().expect("literal socket address"),
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

/// This file is owned by the service user, unlike the static /etc configuration.
/// Its presence, even with `None`, takes precedence over the bootstrap root.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct LibrarySettings {
    pub video_root: Option<PathBuf>,
    pub revision: u64,
}

impl LibrarySettings {
    pub fn load(config: &ServerConfig) -> Result<Self, String> {
        let path = config.data_dir.join("library.json");
        match fs::read(&path) {
            Ok(data) if data.len() <= 64 * 1024 => {
                let settings: Self = serde_json::from_slice(&data)
                    .map_err(|_| "Réglages de bibliothèque invalides.")?;
                if let Some(root) = &settings.video_root {
                    validate_absolute(root)?;
                }
                Ok(settings)
            }
            Ok(_) => Err("Réglages de bibliothèque trop volumineux.".into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self {
                video_root: config.video_root.clone(),
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
        expected_revision: u64,
    ) -> Result<(), String> {
        if self.revision != expected_revision {
            return Err("CONFIG_CONFLICT".into());
        }
        if let Some(root) = &root {
            validate_absolute(root)?;
        }
        let updated = Self {
            video_root: root,
            revision: self.revision.checked_add(1).ok_or("Révision invalide.")?,
        };
        let data = serde_json::to_vec_pretty(&updated).map_err(|_| "Sérialisation impossible.")?;
        atomic_private_write(&config.data_dir.join("library.json"), &data)?;
        *self = updated;
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
        fs::File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| "Synchronisation de configuration impossible.")?;
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
    fn address_and_mount_are_not_fixed() {
        let mut config = ServerConfig::default();
        config.listen = "10.10.0.8:9443".parse().unwrap();
        config.tls_names = vec!["media.home".into(), "10.10.0.8".into()];
        config.video_root = Some("/mnt/other-nas/films".into());
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
            .update(&config, Some("/mnt/new-nas".into()), 0)
            .unwrap();
        assert_eq!(LibrarySettings::load(&config).unwrap(), settings);
        assert!(
            settings
                .update(&config, Some("/mnt/stale".into()), 0)
                .is_err()
        );
        assert_eq!(settings.video_root, Some("/mnt/new-nas".into()));
        settings.update(&config, None, 1).unwrap();
        assert_eq!(LibrarySettings::load(&config).unwrap().video_root, None);
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
    fn failed_write_does_not_change_memory() {
        let dir = tempfile::tempdir().unwrap();
        let config = ServerConfig {
            data_dir: dir.path().join("missing"),
            ..ServerConfig::default()
        };
        let mut settings = LibrarySettings::default();
        assert!(
            settings
                .update(&config, Some("/srv/videos".into()), 0)
                .is_err()
        );
        assert_eq!(settings, LibrarySettings::default());
    }
}
