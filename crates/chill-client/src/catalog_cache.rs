//! Private per-server/per-profile catalogue mirror. Never caches video bytes.
use crate::{HttpMethod, Transport, ui::UiError};
use base64::Engine;
use chill_domain::{Store, SyncMedia, catalog_browser::BrowserQuery};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Read,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

pub type CatalogPublication = Option<(Value, bool, String)>;

fn account_page(previous: usize, page: &Value) -> Result<usize, UiError> {
    let size = serde_json::to_vec(page)
        .map_err(|_| error("Catalogue invalide."))?
        .len();
    let total = previous.saturating_add(size);
    if total > 32 * 1024 * 1024 {
        return Err(error("Catalogue trop volumineux."));
    }
    Ok(total)
}

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
pub struct ArtworkTask {
    transport: Transport,
    directory: PathBuf,
    key: String,
    endpoint: String,
    gate: Arc<tokio::sync::Semaphore>,
    budget: Arc<Mutex<u64>>,
    pool: PathBuf,
}

/// Closed transport work prepared while the native host holds its short context
/// lock. `fetch` owns everything it needs and never borrows the SQLite cache.
pub struct SyncTask {
    transport: Transport,
    profile: String,
    known_revision: Option<String>,
}
#[derive(Clone)]
pub struct SyncResult {
    pub changed: bool,
    pub reset: bool,
    pub revision: String,
    pub items: Vec<SyncMedia>,
    pub deleted_ids: Vec<String>,
    pub document: Value,
}
impl SyncTask {
    pub async fn fetch(self) -> Result<SyncResult, UiError> {
        match self.fetch_changes().await {
            Err(error) if error.status == Some(404) && error.code == "ROUTE_NOT_FOUND" => {
                self.fetch_legacy_snapshot().await
            }
            result => result,
        }
    }
    async fn fetch_changes(&self) -> Result<SyncResult, UiError> {
        let path = format!("/api/2026-10-14/profiles/{}/catalog/changes", self.profile);
        let mut first: Value = self
            .transport
            .json_with_options(
                HttpMethod::POST,
                &path,
                Some(&json!({"known_revision":self.known_revision})),
                &[],
                &[],
            )
            .await?;
        if first["unchanged"] == true {
            let revision = first["revision"]
                .as_str()
                .filter(|r| r.len() < 256)
                .ok_or_else(|| error("Révision de catalogue invalide."))?
                .to_owned();
            if Some(revision.as_str()) != self.known_revision.as_deref() {
                return Err(error("Révision de catalogue incohérente."));
            }
            return Ok(SyncResult {
                changed: false,
                reset: false,
                revision,
                items: Vec::new(),
                deleted_ids: Vec::new(),
                document: Value::Null,
            });
        }
        let revision = first["revision"]
            .as_str()
            .filter(|r| r.len() < 256)
            .ok_or_else(|| error("Révision de catalogue invalide."))?
            .to_owned();
        let reset = first["reset"] == true;
        let snapshot = first["snapshot_id"]
            .as_str()
            .ok_or_else(|| error("Instantané de catalogue absent."))?
            .to_owned();
        uuid::Uuid::parse_str(&snapshot).map_err(|_| error("Instantané de catalogue invalide."))?;
        let total = first["total"]
            .as_u64()
            .filter(|n| *n <= 10_000)
            .ok_or_else(|| error("Catalogue trop volumineux."))? as usize;
        let deleted_ids: Vec<String> = serde_json::from_value(first["deleted_ids"].clone())
            .map_err(|_| error("Suppressions de catalogue invalides."))?;
        if deleted_ids.len() > 10_000
            || deleted_ids
                .iter()
                .any(|id| uuid::Uuid::parse_str(id).is_err())
        {
            return Err(error("Suppressions de catalogue invalides."));
        }
        let mut bytes = account_page(0, &first)?;
        let mut items: Vec<SyncMedia> = serde_json::from_value(first["items"].take())
            .map_err(|_| error("Catalogue invalide."))?;
        if items.len() > 100 {
            return Err(error("Page de catalogue trop volumineuse."));
        }
        let mut page = first.clone();
        while page["has_more"] == true {
            let offset = page["next_offset"]
                .as_u64()
                .ok_or_else(|| error("Page de catalogue invalide."))?
                as usize;
            if offset != items.len() || offset >= total {
                return Err(error("Page de catalogue incohérente."));
            }
            page = self
                .transport
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
            bytes = account_page(bytes, &page)?;
            let part: Vec<SyncMedia> = serde_json::from_value(page["items"].take())
                .map_err(|_| error("Page catalogue invalide."))?;
            if part.is_empty() || part.len() > 100 {
                return Err(error("Page de catalogue invalide."));
            }
            items.extend(part);
        }
        if items.len() != total {
            return Err(error("Copie du catalogue incomplète."));
        }
        first
            .as_object_mut()
            .ok_or_else(|| error("Catalogue invalide."))?
            .remove("items");
        first.as_object_mut().unwrap().remove("deleted_ids");
        first["revision"] = Value::String(revision.clone());
        Ok(SyncResult {
            changed: true,
            reset,
            revision,
            items,
            deleted_ids,
            document: first,
        })
    }
    /// Compatibility only: an old server has no additive changes route. This
    /// still runs as a detached SyncTask, never under UiClient's mutex.
    async fn fetch_legacy_snapshot(&self) -> Result<SyncResult, UiError> {
        let path = format!("/api/2026-10-14/profiles/{}/catalog/snapshot", self.profile);
        let mut first: Value = self
            .transport
            .json_with_options(
                HttpMethod::POST,
                &path,
                Some(&json!({"known_revision":self.known_revision})),
                &[],
                &[],
            )
            .await?;
        if first["unchanged"] == true {
            let revision = first["revision"]
                .as_str()
                .filter(|r| r.len() < 256)
                .ok_or_else(|| error("Révision de catalogue invalide."))?
                .to_owned();
            if Some(revision.as_str()) != self.known_revision.as_deref() {
                return Err(error("Révision de catalogue incohérente."));
            }
            return Ok(SyncResult {
                changed: false,
                reset: false,
                revision,
                items: Vec::new(),
                deleted_ids: Vec::new(),
                document: Value::Null,
            });
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
        let mut bytes = account_page(0, &first)?;
        let mut items: Vec<SyncMedia> = serde_json::from_value(first["items"].take())
            .map_err(|_| error("Catalogue invalide."))?;
        if items.len() > 100 {
            return Err(error("Page de catalogue trop volumineuse."));
        }
        let mut page = first.clone();
        while page["has_more"] == true {
            let offset = page["next_offset"]
                .as_u64()
                .ok_or_else(|| error("Page de catalogue invalide."))?
                as usize;
            if offset != items.len() || offset >= total {
                return Err(error("Page de catalogue incohérente."));
            }
            page = self
                .transport
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
            bytes = account_page(bytes, &page)?;
            let part: Vec<SyncMedia> = serde_json::from_value(page["items"].take())
                .map_err(|_| error("Page catalogue invalide."))?;
            if part.is_empty() || part.len() > 100 {
                return Err(error("Page de catalogue invalide."));
            }
            items.extend(part);
        }
        if items.len() != total {
            return Err(error("Copie du catalogue incomplète."));
        }
        first
            .as_object_mut()
            .ok_or_else(|| error("Catalogue invalide."))?
            .remove("items");
        first["revision"] = Value::String(revision.clone());
        Ok(SyncResult {
            changed: true,
            reset: true,
            revision,
            items,
            deleted_ids: Vec::new(),
            document: first,
        })
    }
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
                crate::artwork_cache::publish(&self.pool, &original, &source, false)
                    .map_err(|_| error("Écriture du cache impossible."))?;
                *used += source.len() as u64;
                crate::artwork_cache::publish(&self.pool, &preview, &thumbnail, true)
                    .map_err(|_| error("Publication du cache impossible."))?;
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
    pub mutation_generation: u64,
    pub mutation_guard: Arc<AtomicU64>,
    pub publish_gate: Arc<Mutex<()>>,
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
        Self::open_with_sync(
            root,
            server,
            profile,
            gate,
            Arc::new(AtomicU64::new(0)),
            Arc::new(Mutex::new(())),
        )
    }
    pub fn open_with_sync(
        root: &Path,
        server: &str,
        profile: &str,
        gate: Arc<tokio::sync::Semaphore>,
        mutation_guard: Arc<AtomicU64>,
        publish_gate: Arc<Mutex<()>>,
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
            mutation_generation: 0,
            mutation_guard,
            publish_gate,
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
    pub fn sync_task(&self, transport: Transport) -> SyncTask {
        SyncTask {
            transport,
            profile: self.profile.clone(),
            known_revision: self
                .document
                .as_ref()
                .and_then(|d| d["revision"].as_str())
                .map(str::to_owned),
        }
    }
    /// Runs the expensive mirror transaction on an independently opened
    /// SQLite connection.  The caller owns final in-memory publication; the
    /// probe vetoes SQLite's commit if a newer local mutation/context wins.
    #[allow(clippy::too_many_arguments)] // Explicit immutable CAS/context captured by the detached task.
    pub fn publish_sync_detached(
        root: PathBuf,
        server: String,
        profile: String,
        gate: Arc<tokio::sync::Semaphore>,
        mutation_guard: Arc<AtomicU64>,
        publish_gate: Arc<Mutex<()>>,
        base_revision: Option<String>,
        expected_generation: u64,
        context_epoch: Arc<AtomicU64>,
        expected_epoch: u64,
        result: SyncResult,
    ) -> Result<CatalogPublication, UiError> {
        let _serial = publish_gate
            .lock()
            .map_err(|_| error("Publication catalogue occupée."))?;
        if mutation_guard.load(Ordering::Acquire) != expected_generation
            || context_epoch.load(Ordering::Acquire) != expected_epoch
        {
            return Ok(None);
        }
        let mut cache = Self::open_with_sync(
            &root,
            &server,
            &profile,
            gate,
            mutation_guard.clone(),
            publish_gate.clone(),
        )?;
        let current = cache.document.as_ref().and_then(|d| d["revision"].as_str());
        if current != base_revision.as_deref() {
            return Ok(None);
        }
        let guard = mutation_guard.clone();
        let epoch = context_epoch.clone();
        cache
            .store
            .cancellation_probe(Some(Arc::new(move || {
                guard.load(Ordering::Acquire) != expected_generation
                    || epoch.load(Ordering::Acquire) != expected_epoch
            })))
            .map_err(|_| error("Synchronisation locale annulée."))?;
        let changed = result.changed;
        let revision = result.revision.clone();
        let applied = cache.commit_sync(result);
        let _ = cache.store.cancellation_probe(None);
        if applied.is_err()
            && (mutation_guard.load(Ordering::Acquire) != expected_generation
                || context_epoch.load(Ordering::Acquire) != expected_epoch)
        {
            return Ok(None);
        }
        applied?;
        Ok(cache.document.map(|document| (document, changed, revision)))
    }
    /// Applies a complete immutable transfer in one SQLite transaction. Delta
    /// pages are merged with the old mirror first, so readers never observe a
    /// partial page or a delete-before-upsert state.
    pub fn commit_sync(&mut self, result: SyncResult) -> Result<(), UiError> {
        if !result.changed {
            self.dirty = false;
            self.notice = None;
            return Ok(());
        }
        let mut merged: HashMap<String, SyncMedia> = if result.reset {
            HashMap::new()
        } else {
            self.mirror_items()?
                .into_iter()
                .map(|item| (item.media.id.clone(), item))
                .collect()
        };
        if !result.reset {
            let deleted: HashSet<&str> = result.deleted_ids.iter().map(String::as_str).collect();
            merged.retain(|id, _| !deleted.contains(id.as_str()));
        }
        for item in result.items {
            merged.insert(item.media.id.clone(), item);
        }
        if merged.len() > 10_000 {
            return Err(error("Catalogue trop volumineux."));
        }
        self.commit(&merged.into_values().collect::<Vec<_>>(), result.document)
    }
    fn mirror_items(&mut self) -> Result<Vec<SyncMedia>, UiError> {
        self.store
            .catalog(&self.profile)
            .map_err(|_| error("Cache catalogue illisible."))?
            .into_iter()
            .map(|media| {
                let revision = self
                    .store
                    .media_revision(&media.id)
                    .map_err(|_| error("Cache catalogue illisible."))?;
                Ok(SyncMedia { media, revision })
            })
            .collect()
    }
    async fn download_once(
        &self,
        transport: &Transport,
    ) -> Result<Option<(Vec<SyncMedia>, Value)>, UiError> {
        let path = format!("/api/2026-10-14/profiles/{}/catalog/snapshot", self.profile);
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
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
        self.mutation_generation = self.mutation_generation.wrapping_add(1);
        self.mutation_guard.fetch_add(1, Ordering::AcqRel);
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
        let version = if kind == "season" {
            let (tv, season) = id
                .split_once(':')
                .ok_or_else(|| error("Saison invalide."))?;
            let document = self
                .document
                .as_ref()
                .ok_or_else(|| error("Catalogue absent."))?;
            if !document["season_metadata"][format!("tmdb:tv:{tv}:season:{season}")]["poster"]
                .is_string()
            {
                return Err(error("Pas d’affiche de saison."));
            }
            let hash = Sha256::digest(document["revision"].as_str().unwrap_or_default().as_bytes());
            u64::from_be_bytes(hash[..8].try_into().unwrap())
        } else if kind == "media" {
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
            pool: self
                .directory
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join(".artwork-blobs"),
            key,
            endpoint: if kind == "season" {
                let (tv, season) = id.split_once(':').unwrap();
                format!(
                    "/api/2026-10-14/profiles/{}/series/{tv}/seasons/{season}/artwork",
                    self.profile
                )
            } else {
                format!(
                    "/api/2026-10-14/profiles/{}/{kind}/{id}/artwork",
                    self.profile
                )
            },
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
    fn cumulative_budget_counts_auxiliary_documents_and_first_page_items() {
        let page = json!({"items":[{"title":"first"}],"sagas":[{"name":"metadata"}],"season_metadata":{"x":"value"}});
        let size = serde_json::to_vec(&page).unwrap().len();
        assert_eq!(account_page(0, &page).unwrap(), size);
        assert!(account_page(32 * 1024 * 1024 - size + 1, &page).is_err());
    }
    fn sync_item(id: &str, title: &str) -> SyncMedia {
        serde_json::from_value(json!({"media":{"id":id,"path":format!("/{id}"),"title":title,"category":"film","year":null,"series":null,"season":null,"episode":null,"genres":[],"overview":"","poster":null,"saga":null,"watched":false,"rating":null,"available":true,"added_at":1,"tmdb_id":null},"revision":1})).unwrap()
    }
    #[test]
    fn detached_publish_cancels_before_write_when_local_mutation_wins() {
        let root = tempfile::tempdir().unwrap();
        let server = uuid::Uuid::new_v4().to_string();
        let profile = uuid::Uuid::new_v4().to_string();
        let gate = Arc::new(tokio::sync::Semaphore::new(1));
        let guard = Arc::new(AtomicU64::new(0));
        let publish_gate = Arc::new(Mutex::new(()));
        let id = uuid::Uuid::new_v4().to_string();
        let mut cache = LocalCatalog::open_with_sync(
            root.path(),
            &server,
            &profile,
            gate.clone(),
            guard.clone(),
            publish_gate.clone(),
        )
        .unwrap();
        cache.commit(&[sync_item(&id, "before")], json!({"revision":"r0","sagas":[],"playlists":[],"series_revisions":{},"season_metadata":{}})).unwrap();
        // This is the deterministic delayed-publication race: transport has
        // finished but a personal action invalidates the prepared context.
        guard.fetch_add(1, Ordering::AcqRel);
        let published = LocalCatalog::publish_sync_detached(
            root.path().to_path_buf(), server.clone(), profile.clone(), gate, guard.clone(), publish_gate,
            Some("r0".into()), 0, Arc::new(AtomicU64::new(1)), 1,
            SyncResult { changed:true, reset:true, revision:"r1".into(), items:vec![sync_item(&id, "after")], deleted_ids:vec![], document:json!({"revision":"r1","sagas":[],"playlists":[],"series_revisions":{},"season_metadata":{}}) },
        ).unwrap();
        assert!(published.is_none());
        assert_eq!(cache.store.media(&profile, &id).unwrap().title, "before");
    }
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
        crate::artwork_cache::publish(
            &dir.path().join("pool"),
            &directory.join("key.jpg"),
            &thumbnail,
            true,
        )
        .unwrap();
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
            endpoint: "/api/2026-10-14/no-network".into(),
            gate: Arc::new(tokio::sync::Semaphore::new(1)),
            budget: Arc::new(Mutex::new(0)),
            pool: dir.path().join("pool"),
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
