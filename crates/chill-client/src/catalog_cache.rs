//! Private per-server/per-profile catalogue mirror. Never caches video bytes.
use crate::{HttpMethod, Transport, ui::UiError};
use base64::Engine;
use chill_domain::{Store, SyncMedia, catalog_browser::BrowserQuery};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

fn error(message: &str) -> UiError {
    UiError {
        message: message.into(),
        status: None,
        code: "CATALOG_CACHE".into(),
    }
}
pub(crate) fn purge_profile(root: &Path, server: &str, profile: &str) -> Result<(), UiError> {
    uuid::Uuid::parse_str(server).map_err(|_| error("Serveur invalide."))?;
    uuid::Uuid::parse_str(profile).map_err(|_| error("Profil invalide."))?;
    let base = root.join("catalog-cache");
    let server_dir = base.join(server);
    let target = server_dir.join(profile);
    for path in [&base, &server_dir, &target] {
        match fs::symlink_metadata(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Ok(m) if m.is_dir() && !m.file_type().is_symlink() => {}
            _ => {
                return Err(error(
                    "Profil supprimé côté serveur ; cache local non effacé : dossier non sûr. Recharger pour réessayer.",
                ));
            }
        }
    }
    fs::remove_dir_all(target).map_err(|_| {
        error("Profil supprimé côté serveur ; cache local non effacé. Recharger pour réessayer.")
    })
}
/// Only disposable catalogue caches are removed, after an authoritative profile
/// list. Never recurse from an unvalidated root or follow a cache symlink.
pub(crate) fn purge_orphans(
    root: &Path,
    server: &str,
    keep: &std::collections::HashSet<String>,
) -> Result<(), UiError> {
    uuid::Uuid::parse_str(server).map_err(|_| error("Serveur invalide."))?;
    let base = root.join("catalog-cache");
    let server_dir = base.join(server);
    for directory in [&base, &server_dir] {
        match fs::symlink_metadata(directory) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Ok(m) if m.is_dir() && !m.file_type().is_symlink() => {}
            _ => {
                return Err(error(
                    "Cache de profil supprimé non nettoyé : dossier non sûr.",
                ));
            }
        }
    }
    for entry in
        fs::read_dir(&server_dir).map_err(|_| error("Cache de profil supprimé non nettoyé."))?
    {
        let entry = entry.map_err(|_| error("Cache de profil supprimé non nettoyé."))?;
        let Some(id) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if uuid::Uuid::parse_str(&id).is_err() || keep.contains(&id) {
            continue;
        }
        purge_profile(root, server, &id)?;
    }
    Ok(())
}
pub(crate) fn private_directory(path: &Path) -> Result<(), UiError> {
    use std::os::unix::fs::PermissionsExt;
    match fs::symlink_metadata(path) {
        Ok(m) if m.is_dir() && !m.file_type().is_symlink() => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(path).map_err(|_| error("Cache local inaccessible."))?
        }
        _ => {
            return Err(error(
                "Le cache doit être un dossier privé, sans lien symbolique.",
            ));
        }
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|_| error("Protection du cache impossible."))
}
fn bounded_read(path: &Path, max: usize) -> Result<Option<Vec<u8>>, UiError> {
    let file = match fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc_no_follow())
        .open(path)
    {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(error("Cache image non lisible.")),
    };
    let meta = file
        .metadata()
        .map_err(|_| error("Cache image inaccessible."))?;
    if !meta.is_file() || meta.len() > max as u64 {
        return Err(error("Cache image invalide."));
    }
    let mut bytes = Vec::new();
    file.take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| error("Cache image inaccessible."))?;
    if bytes.len() > max {
        return Err(error("Cache image trop volumineux."));
    }
    Ok(Some(bytes))
}
// No additional dependency: the libc values differ between the two supported hosts.
#[cfg(target_os = "macos")]
fn libc_no_follow() -> i32 {
    0x100
}
#[cfg(not(target_os = "macos"))]
fn libc_no_follow() -> i32 {
    0x20000
}
fn publish(path: &Path, bytes: &[u8]) -> Result<(), UiError> {
    let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|_| error("Écriture du cache impossible."))?;
        f.write_all(bytes)
            .and_then(|_| f.sync_all())
            .map_err(|_| error("Écriture du cache impossible."))?;
        fs::rename(&temporary, path).map_err(|_| error("Publication du cache impossible."))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
pub struct ArtworkTask {
    transport: Transport,
    directory: PathBuf,
    key: String,
    endpoint: String,
    gate: Arc<tokio::sync::Semaphore>,
    budget: Arc<Mutex<u64>>,
}
impl ArtworkTask {
    pub async fn fetch(self) -> Result<Value, UiError> {
        // Separate from navigation/player lock; one decoder/download at a time.
        let _permit = self
            .gate
            .acquire()
            .await
            .map_err(|_| error("Cache arrêté."))?;
        let preview = self.directory.join(format!("{}.jpg", self.key));
        let bytes = if let Some(bytes) =
            bounded_read(&preview, chill_catalog::poster::MAX_PREVIEW_BYTES)?
        {
            bytes
        } else {
            let original = self.directory.join(format!("{}.image", self.key));
            let source = match bounded_read(&original, 8 * 1024 * 1024)? {
                Some(data) => data,
                None => {
                    self.transport
                        .get_binary(&self.endpoint, &[], 8 * 1024 * 1024)
                        .await?
                }
            };
            let (source, thumbnail) = tokio::task::spawn_blocking(move || {
                chill_catalog::poster::thumbnail(&source).map(|p| (source, p))
            })
            .await
            .map_err(|_| error("Préparation image interrompue."))?
            .map_err(|_| error("Affiche non prise en charge."))?;
            {
                let mut used = self
                    .budget
                    .lock()
                    .map_err(|_| error("Cache image occupé."))?;
                let added = source.len() as u64 + thumbnail.len() as u64;
                if used.saturating_add(added) > 2 * 1024 * 1024 * 1024 {
                    return Err(error("Cache d’images plein (2 Gio par profil)."));
                }
                publish(&original, &source)?;
                *used += source.len() as u64;
                publish(&preview, &thumbnail)?;
                *used += thumbnail.len() as u64;
            }
            thumbnail
        };
        Ok(
            json!({"src":format!("data:image/jpeg;base64,{}",base64::engine::general_purpose::STANDARD.encode(bytes))}),
        )
    }
}
pub(crate) struct LocalCatalog {
    pub server: String,
    pub profile: String,
    pub store: Store,
    pub document: Option<Value>,
    directory: PathBuf,
    pub dirty: bool,
    pub notice: Option<String>,
    gate: Arc<tokio::sync::Semaphore>,
    budget: Arc<Mutex<u64>>,
}
impl LocalCatalog {
    pub fn open(
        root: &Path,
        server: &str,
        profile: &str,
        gate: Arc<tokio::sync::Semaphore>,
    ) -> Result<Self, UiError> {
        uuid::Uuid::parse_str(server).map_err(|_| error("Serveur invalide."))?;
        uuid::Uuid::parse_str(profile).map_err(|_| error("Profil invalide."))?;
        let base = root.join("catalog-cache");
        private_directory(&base)?;
        let base = base.join(server);
        private_directory(&base)?;
        let directory = base.join(profile);
        private_directory(&directory)?;
        let image_dir = directory.join("images");
        private_directory(&image_dir)?;
        let mut used = 0u64;
        for (count, entry) in fs::read_dir(&image_dir)
            .map_err(|_| error("Cache image inaccessible."))?
            .enumerate()
        {
            if count > 30_000 {
                return Err(error("Trop de fichiers dans le cache image."));
            }
            let meta = entry
                .map_err(|_| error("Cache image inaccessible."))?
                .metadata()
                .map_err(|_| error("Cache image inaccessible."))?;
            used = used.saturating_add(meta.len());
        }
        let path = directory.join("catalog.sqlite");
        if let Ok(m) = fs::symlink_metadata(&path)
            && (!m.is_file() || m.file_type().is_symlink())
        {
            return Err(error("Base de cache invalide."));
        }
        let store = Store::open(&path).map_err(|_| error("Base de cache inaccessible."))?;
        let document = store
            .client_mirror_document(profile)
            .map_err(|_| error("Cache catalogue illisible."))?;
        Ok(Self {
            server: server.into(),
            profile: profile.into(),
            store,
            document,
            directory: image_dir,
            dirty: false,
            notice: None,
            gate,
            budget: Arc::new(Mutex::new(used)),
        })
    }
    pub async fn download(
        &self,
        transport: &Transport,
    ) -> Result<Option<(Vec<SyncMedia>, Value)>, UiError> {
        for attempt in 0..3 {
            match self.download_once(transport).await {
                Err(error) if error.code == "SYNC_SNAPSHOT_CHANGED" && attempt < 2 => {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await
                }
                result => return result,
            }
        }
        unreachable!()
    }
    async fn download_once(
        &self,
        transport: &Transport,
    ) -> Result<Option<(Vec<SyncMedia>, Value)>, UiError> {
        let path = format!("/api/v1/profiles/{}/catalog/snapshot", self.profile);
        let known = self.document.as_ref().and_then(|v| v["revision"].as_str());
        let mut first: Value = transport
            .json_with_options(
                HttpMethod::POST,
                &path,
                Some(&json!({"known_revision":known})),
                &[],
                &[],
            )
            .await?;
        if first["unchanged"] == true {
            if known != first["revision"].as_str() || self.document.is_none() {
                return Err(error("Révision de catalogue incohérente."));
            }
            return Ok(None);
        }
        let revision = first["revision"]
            .as_str()
            .filter(|r| r.len() < 256)
            .ok_or_else(|| error("Révision de catalogue invalide."))?
            .to_owned();
        let snapshot = first["snapshot_id"]
            .as_str()
            .ok_or_else(|| error("Instantané de catalogue absent."))?
            .to_owned();
        uuid::Uuid::parse_str(&snapshot).map_err(|_| error("Instantané de catalogue invalide."))?;
        let total = first["total"]
            .as_u64()
            .filter(|n| *n <= 10_000)
            .ok_or_else(|| error("Catalogue trop volumineux."))? as usize;
        let mut items: Vec<SyncMedia> = serde_json::from_value(first["items"].take())
            .map_err(|_| error("Catalogue invalide."))?;
        if items.len() > 100 {
            return Err(error("Page de catalogue trop volumineuse."));
        }
        let mut size = serde_json::to_vec(&items)
            .map_err(|_| error("Catalogue invalide."))?
            .len()
            + serde_json::to_vec(&first)
                .map_err(|_| error("Catalogue invalide."))?
                .len();
        let mut page = first.clone();
        while page["has_more"] == true {
            let offset = page["next_offset"]
                .as_u64()
                .ok_or_else(|| error("Page de catalogue invalide."))?
                as usize;
            if offset != items.len() || offset >= total {
                return Err(error("Page de catalogue incohérente."));
            }
            page = transport
                .json_with_options(
                    HttpMethod::GET,
                    &format!("{path}/{snapshot}"),
                    None::<&Value>,
                    &[("offset", offset.to_string())],
                    &[],
                )
                .await?;
            if page["revision"] != revision {
                return Err(error("Le catalogue a changé pendant la copie."));
            }
            let part: Vec<SyncMedia> = serde_json::from_value(page["items"].take())
                .map_err(|_| error("Page catalogue invalide."))?;
            size = size.saturating_add(
                serde_json::to_vec(&part)
                    .map_err(|_| error("Catalogue invalide."))?
                    .len(),
            );
            if size > 34 * 1024 * 1024 {
                return Err(error("Catalogue trop volumineux."));
            }
            if part.is_empty() || part.len() > 100 {
                return Err(error("Page de catalogue invalide."));
            }
            items.extend(part);
            if items.len() > total {
                return Err(error("Catalogue incohérent."));
            }
        }
        if items.len() != total {
            return Err(error("Copie du catalogue incomplète."));
        }
        first.as_object_mut().unwrap().remove("items");
        Ok(Some((items, first)))
    }
    pub fn commit(&mut self, items: &[SyncMedia], document: Value) -> Result<(), UiError> {
        self.store
            .replace_client_mirror(&self.profile, items, &document)
            .map_err(|_| error("Synchronisation locale impossible, ancienne copie conservée."))?;
        self.document = Some(document);
        self.dirty = false;
        self.notice = None;
        Ok(())
    }
    pub fn browse(&self, q: &BrowserQuery) -> Result<Value, UiError> {
        let page = self
            .store
            .browse_catalog(&self.profile, q)
            .map_err(|_| error("Filtres catalogue invalides."))?;
        Ok(
            json!({"items":page.items,"has_more":page.has_more,"series_counters":page.series_counters}),
        )
    }
    pub fn collection(&self, kind: &str, id: Option<&str>) -> Result<Value, UiError> {
        let items = &self
            .document
            .as_ref()
            .ok_or_else(|| error("Catalogue absent."))?[kind];
        match id {
            None => Ok(json!({"items":items})),
            Some(id) => {
                let item = items
                    .as_array()
                    .and_then(|a| a.iter().find(|v| v["id"] == id))
                    .ok_or_else(|| error("Liste ou saga introuvable."))?;
                Ok(json!({if kind=="sagas"{"saga"}else{"playlist"}:item}))
            }
        }
    }
    pub fn artwork(
        &mut self,
        transport: Transport,
        kind: &str,
        id: &str,
    ) -> Result<ArtworkTask, UiError> {
        let version = if kind == "media" {
            let m = self
                .store
                .media(&self.profile, id)
                .map_err(|_| error("Vidéo introuvable."))?;
            if m.poster.is_none() {
                return Err(error("Pas d’affiche."));
            }
            self.store
                .media_revision(id)
                .map_err(|_| error("Affiche inconnue."))?
        } else {
            self.document
                .as_ref()
                .and_then(|d| d[kind].as_array())
                .and_then(|a| a.iter().find(|v| v["id"] == id))
                .filter(|v| !v["poster"].is_null())
                .and_then(|v| v["revision"].as_u64())
                .ok_or_else(|| error("Pas d’affiche personnalisée."))?
        };
        let key = Sha256::digest(format!("{kind}:{id}:{version}"))
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        Ok(ArtworkTask {
            transport,
            directory: self.directory.clone(),
            key,
            endpoint: format!("/api/v1/profiles/{}/{kind}/{id}/artwork", self.profile),
            gate: self.gate.clone(),
            budget: self.budget.clone(),
        })
    }
    pub fn prefetch(&mut self, transport: Transport) -> Vec<ArtworkTask> {
        let Ok(media) = self.store.catalog(&self.profile) else {
            return vec![];
        };
        media
            .into_iter()
            .filter(|m| m.poster.is_some())
            .filter_map(|m| self.artwork(transport.clone(), "media", &m.id).ok())
            .filter(|task| !task.directory.join(format!("{}.jpg", task.key)).is_file())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deleted_profile_cache_is_purged_without_touching_siblings_or_symlink_targets() {
        let root = tempfile::tempdir().unwrap();
        let server = uuid::Uuid::new_v4().to_string();
        let keep = uuid::Uuid::new_v4().to_string();
        let deleted = uuid::Uuid::new_v4().to_string();
        let gate = Arc::new(tokio::sync::Semaphore::new(1));
        drop(LocalCatalog::open(root.path(), &server, &keep, gate.clone()).unwrap());
        drop(LocalCatalog::open(root.path(), &server, &deleted, gate).unwrap());
        let base = root.path().join("catalog-cache").join(&server);
        purge_orphans(
            root.path(),
            &server,
            &std::collections::HashSet::from([keep.clone()]),
        )
        .unwrap();
        assert!(base.join(&keep).is_dir());
        assert!(!base.join(&deleted).exists());
        let outside = tempfile::tempdir().unwrap();
        let link_id = uuid::Uuid::new_v4().to_string();
        std::os::unix::fs::symlink(outside.path(), base.join(&link_id)).unwrap();
        assert!(
            purge_orphans(
                root.path(),
                &server,
                &std::collections::HashSet::from([keep.clone()])
            )
            .is_err()
        );
        assert!(outside.path().is_dir());
        assert!(base.join(keep).is_dir());
    }
    #[test]
    fn image_cache_survives_restart_without_network_and_rejects_links() {
        let dir = tempfile::tempdir().unwrap();
        private_directory(&dir.path().join("images")).unwrap();
        let directory = dir.path().join("images");
        let thumbnail =
            chill_catalog::poster::thumbnail(include_bytes!("../../../icons/icon.png")).unwrap();
        publish(&directory.join("key.jpg"), &thumbnail).unwrap();
        let endpoint = crate::ServerEndpoint::parse("fixture", "https://127.0.0.1:9").unwrap();
        let transport = Transport::new(
            endpoint,
            Arc::new(std::sync::RwLock::new(crate::TrustStore::default())),
            None,
        )
        .unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let task = ArtworkTask {
            transport,
            directory: directory.clone(),
            key: "key".into(),
            endpoint: "/api/v1/no-network".into(),
            gate: Arc::new(tokio::sync::Semaphore::new(1)),
            budget: Arc::new(Mutex::new(0)),
        };
        assert!(
            runtime.block_on(task.fetch()).unwrap()["src"]
                .as_str()
                .unwrap()
                .starts_with("data:image/jpeg;base64,")
        );
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(directory.join("key.jpg"), &link).unwrap();
        assert!(bounded_read(&link, 1024 * 1024).is_err());
        assert!(private_directory(&link).is_err());
    }
}
