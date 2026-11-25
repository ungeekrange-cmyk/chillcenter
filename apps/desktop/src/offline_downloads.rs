//! Private Mac download store. The webview only supplies opaque identifiers;
//! all paths, pinned HTTPS headers and filesystem handles stay in Rust.
use base64::Engine;
use chill_client::{
    DownloadPlanItem, DownloadSession, Transport,
    ui::{PlaybackConnection, UiError},
};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

const INDEX_LIMIT: u64 = 16 * 1024 * 1024;
const CHUNK: usize = 1024 * 1024;
const MAX_ITEMS: usize = 3_000;
const MAX_SUBTITLES: usize = 64;
const SUBTITLE_LIMIT: u64 = 16 * 1024 * 1024;
const POSTER_LIMIT: usize = 2 * CHUNK;

fn error(message: impl Into<String>, code: &str) -> UiError {
    UiError {
        message: message.into(),
        status: None,
        code: code.into(),
    }
}

#[derive(Clone)]
pub struct OfflineContext {
    pub server_id: String,
    pub profile_id: String,
    pub device_id: Option<String>,
    pub transport: Option<Transport>,
}
impl OfflineContext {
    pub fn online(connection: PlaybackConnection) -> Result<Self, UiError> {
        let value = Self {
            server_id: connection.server_id,
            profile_id: connection.profile_id,
            device_id: Some(connection.device_id),
            transport: Some(connection.transport),
        };
        value.validate()?;
        Ok(value)
    }
    /// The caller must obtain these UUIDs from the private local index, never
    /// directly from JavaScript state.
    pub fn offline(server_id: String, profile_id: String) -> Result<Self, UiError> {
        let value = Self {
            server_id,
            profile_id,
            device_id: None,
            transport: None,
        };
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<(), UiError> {
        uuid::Uuid::parse_str(&self.server_id)
            .and_then(|_| uuid::Uuid::parse_str(&self.profile_id))
            .map(|_| ())
            .map_err(|_| error("Profil hors connexion invalide.", "OFFLINE_CONTEXT"))
    }
    fn scope(&self) -> String {
        format!("{}:{}", self.server_id, self.profile_id)
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum DownloadCommand {
    List,
    OpenFolder,
    Artwork {
        media_id: String,
    },
    Plan {
        media_id: String,
        scope: String,
    },
    Start {
        media_id: String,
        scope: String,
        #[serde(default)]
        automatic_limit: Option<u64>,
    },
    Sync,
    Segments { media_id: String },
    Cancel {
        #[serde(default)]
        media_id: Option<String>,
    },
    Delete {
        ids: Vec<String>,
    },
    Keep {
        media_id: String,
        enabled: bool,
    },
    SetAuto {
        enabled: bool,
    },
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct Index {
    copies: BTreeMap<String, CopyRecord>,
    partials: BTreeMap<String, PartialRecord>,
    preferences: BTreeMap<String, bool>,
    progress: BTreeMap<String, ProgressRecord>,
}
#[derive(Clone, Serialize, Deserialize)]
struct CopyRecord {
    scope: String,
    media: Value,
    series_key: Option<String>,
    bytes: u64,
    version: String,
    #[serde(default = "default_video_file")]
    file: String,
    keep: bool,
    #[serde(default)]
    automatic: bool,
    missing_subtitles: u64,
    #[serde(default)]
    poster: Option<u64>,
    #[serde(default)]
    delete_pending: bool,
    #[serde(default)]
    deletion_failed: bool,
    subtitles: Vec<SubtitleRecord>,
    preferences: Value,
    personal: Value,
    progress_revision: Option<String>,
    #[serde(default)]
    segments: Vec<chill_domain::segments::Segment>,
}
#[derive(Clone, Serialize, Deserialize)]
struct PartialRecord {
    scope: String,
    media: Value,
    series_key: Option<String>,
    bytes: u64,
    version: String,
    transferred: u64,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct SubtitleRecord {
    pub id: String,
    pub title: String,
    pub language: Option<String>,
    pub bytes: u64,
    file: String,
}
#[derive(Clone, Serialize, Deserialize)]
struct ProgressRecord {
    position_ms: u64,
    duration_ms: u64,
    watched: bool,
    base_revision: Option<String>,
    event_id: String,
    pending: bool,
}
#[derive(Clone, Serialize)]
struct JobView {
    media: Value,
    bytes: u64,
    transferred: u64,
    status: String,
}
struct Active {
    scope: String,
    media_id: String,
    cancelled: Arc<AtomicBool>,
}

pub struct OfflineLease {
    pub media: Value,
    #[allow(dead_code)] // retained for local next-episode grouping
    pub series_key: Option<String>,
    pub bytes: u64,
    #[allow(dead_code)] // retained for stale-copy diagnostics and repair
    pub version: String,
    pub file: File,
    pub subtitles: Vec<OfflineSubtitle>,
    pub preferences: Value,
    pub personal: Value,
    pub progress_revision: Option<String>,
    pub progress: Option<Value>,
    _reader: ReaderGuard,
}
pub struct OfflineSubtitle {
    pub id: String,
    pub title: String,
    pub language: Option<String>,
    pub path: PathBuf,
}
struct ReaderGuard {
    key: String,
    readers: Arc<Mutex<BTreeMap<String, usize>>>,
}
impl Drop for ReaderGuard {
    fn drop(&mut self) {
        if let Ok(mut readers) = self.readers.lock()
            && let Some(count) = readers.get_mut(&self.key)
        {
            *count = count.saturating_sub(1);
            if *count == 0 {
                readers.remove(&self.key);
            }
        }
    }
}

#[derive(Clone)]
pub struct OfflineDownloads {
    root: PathBuf,
    database: Arc<Mutex<Connection>>,
    index: Arc<Mutex<Index>>,
    jobs: Arc<Mutex<BTreeMap<String, JobView>>>,
    active: Arc<tokio::sync::Mutex<Option<Active>>>,
    queue: Arc<tokio::sync::Semaphore>,
    readers: Arc<Mutex<BTreeMap<String, usize>>>,
}
impl OfflineDownloads {
    pub fn open(app_data: &Path) -> Result<Self, UiError> {
        let root = app_data.canonicalize().map_err(local_io)?.join("downloads");
        private_directory(&root)?;
        let database = open_index(&root)?;
        let index = read_index(&database)?;
        Ok(Self {
            root,
            database: Arc::new(Mutex::new(database)),
            index: Arc::new(Mutex::new(index)),
            jobs: Arc::new(Mutex::new(BTreeMap::new())),
            active: Arc::new(tokio::sync::Mutex::new(None)),
            queue: Arc::new(tokio::sync::Semaphore::new(1)),
            readers: Arc::new(Mutex::new(BTreeMap::new())),
        })
    }

    pub async fn command(
        self: &Arc<Self>,
        context: OfflineContext,
        command: DownloadCommand,
    ) -> Result<Value, UiError> {
        context.validate()?;
        match command {
            DownloadCommand::List => self.list(&context),
            DownloadCommand::Artwork { media_id } => {
                let key = copy_key(&context, &media_id)?;
                let size = self
                    .lock_index()?
                    .copies
                    .get(&key)
                    .and_then(|copy| copy.poster);
                let Some(size) = size.filter(|size| *size <= POSTER_LIMIT as u64) else {
                    return Ok(Value::Null);
                };
                let folder = self.media_directory(&context, &media_id, false)?;
                let mut bytes = Vec::new();
                regular_file(&folder.join("poster"), size)?
                    .take(size + 1)
                    .read_to_end(&mut bytes)
                    .map_err(local_io)?;
                if let Some(mime) = poster_mime(&bytes) {
                    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
                    return Ok(json!({"src":format!("data:{mime};base64,{encoded}")}));
                }
                Ok(Value::Null)
            }
            DownloadCommand::OpenFolder => {
                let server = self.root.join(&context.server_id);
                private_directory(&server)?;
                let profile = server.join(&context.profile_id);
                private_directory(&profile)?;
                std::process::Command::new("/usr/bin/open")
                    .arg(&profile)
                    .spawn()
                    .map_err(local_io)?;
                Ok(json!({}))
            }
            DownloadCommand::Plan { media_id, scope } => {
                let items = plan_all(&context, &media_id, &scope).await?;
                Ok(json!({"items":items.0,"skipped":items.1}))
            }
            DownloadCommand::Sync => self.sync(&context).await,
            DownloadCommand::Segments { media_id } => {
                let key = copy_key(&context, &media_id)?;
                let segments = self.lock_index()?.copies.get(&key).map(|copy| copy.segments.clone()).unwrap_or_default();
                Ok(json!({"segments":segments}))
            }
            DownloadCommand::Start { media_id, scope, automatic_limit } => {
                valid_uuid(&media_id)?;
                valid_scope(&scope)?;
                if automatic_limit.is_some_and(|limit| scope != "video" || limit == 0 || limit > 1_000_000_000_000) {
                    return Err(error("Plafond de téléchargement invalide.", "DOWNLOAD_INVALID"));
                }
                if context.transport.is_none() || context.device_id.is_none() {
                    return Err(error(
                        "Connectez le serveur pour télécharger.",
                        "DOWNLOAD_OFFLINE",
                    ));
                }
                let mut active = self.active.lock().await;
                if active.is_some() {
                    return Err(error(
                        "Un téléchargement est déjà en cours.",
                        "DOWNLOAD_BUSY",
                    ));
                }
                let cancelled = Arc::new(AtomicBool::new(false));
                *active = Some(Active {
                    scope: context.scope(),
                    media_id: media_id.clone(),
                    cancelled: cancelled.clone(),
                });
                drop(active);
                self.job(
                    &context,
                    &json!({"id":media_id.clone(),"title":""}),
                    0,
                    0,
                    "planning",
                )?;
                let host = self.clone();
                tokio::spawn(async move {
                    let _permit = host.queue.clone().acquire_owned().await.ok();
                    let result = host
                        .run_queue(context.clone(), media_id, scope, cancelled, automatic_limit)
                        .await;
                    if let Err(problem) = result {
                        let scope = context.scope();
                        if let Ok(mut jobs) = host.jobs.lock()
                            && let Some(job) = jobs.get_mut(&scope)
                        {
                            job.status = format!("error:{}", problem.code);
                        }
                    }
                    *host.active.lock().await = None;
                });
                Ok(json!({"started":true}))
            }
            DownloadCommand::Cancel { media_id } => {
                let active = self.active.lock().await;
                if let Some(active) = active.as_ref()
                    && active.scope == context.scope()
                    && media_id.as_ref().is_none_or(|id| id == &active.media_id)
                {
                    active.cancelled.store(true, Ordering::Release);
                }
                Ok(json!({}))
            }
            DownloadCommand::Delete { ids } => {
                if ids.len() > MAX_ITEMS {
                    return Err(error("Trop de copies à supprimer.", "DOWNLOAD_LIMIT"));
                }
                if self
                    .active
                    .lock()
                    .await
                    .as_ref()
                    .is_some_and(|active| active.scope == context.scope())
                {
                    return Err(error(
                        "Annulez le téléchargement avant de supprimer une copie.",
                        "DOWNLOAD_BUSY",
                    ));
                }
                for id in ids {
                    self.delete_one(&context, &id, false)?;
                }
                Ok(json!({}))
            }
            DownloadCommand::Keep { media_id, enabled } => {
                let key = copy_key(&context, &media_id)?;
                let mut index = self.lock_index()?;
                if let Some(copy) = index.copies.get_mut(&key) {
                    copy.keep = enabled;
                    self.save_index(&index)?;
                }
                Ok(json!({}))
            }
            DownloadCommand::SetAuto { enabled } => {
                let mut index = self.lock_index()?;
                index.preferences.insert(context.scope(), enabled);
                self.save_index(&index)?;
                Ok(json!({"enabled":enabled}))
            }
        }
    }

    fn list(&self, context: &OfflineContext) -> Result<Value, UiError> {
        let scope = context.scope();
        let pending = self
            .lock_index()?
            .copies
            .values()
            .filter(|copy| copy.scope == scope && copy.delete_pending)
            .filter_map(|copy| media_id(&copy.media).ok().map(str::to_owned))
            .collect::<Vec<_>>();
        for id in pending {
            let _ = self.auto_delete(context, &id);
        }
        let snapshot = self.lock_index()?.clone();
        let mut items = Vec::new();
        let mut stale = Vec::new();
        let mut total = 0u64;
        for (key, copy) in snapshot.copies.iter().filter(|(_, c)| c.scope == scope) {
            let media_id = media_id(&copy.media)?;
            let file = self
                .media_directory(context, media_id, false)
                .and_then(|directory| {
                    let file = regular_file(&directory.join(&copy.file), copy.bytes)?;
                    Ok((directory, file))
                });
            match file {
                Ok((directory, file)) => {
                    let mut local = file.metadata().map_err(local_io)?.len();
                    for subtitle in &copy.subtitles {
                        if let Ok(file) = regular_file(
                            &directory.join(&subtitle.file),
                            subtitle.bytes.min(SUBTITLE_LIMIT),
                        ) {
                            local = local.saturating_add(file.metadata().map_err(local_io)?.len());
                        }
                    }
                    let has_poster = copy.poster.is_some_and(|bytes| regular_file(&directory.join("poster"), bytes).is_ok());
                    if has_poster {
                        local = local.saturating_add(copy.poster.unwrap_or(0));
                    }
                    total = total.saturating_add(local);
                    items.push(json!({"media":copy.media,"series_key":copy.series_key,"bytes":copy.bytes,"version":copy.version,"local_bytes":local,"automatic":copy.automatic,"keep":copy.keep,"missing_subtitles":copy.missing_subtitles,"has_poster":has_poster,"deletion_failed":copy.deletion_failed}));
                }
                Err(problem) if problem.code == "OFFLINE_MISSING" => stale.push(key.clone()),
                Err(_) => items.push(json!({"media":copy.media,"series_key":copy.series_key,"bytes":copy.bytes,"version":copy.version,"local_bytes":copy.bytes,"automatic":copy.automatic,"keep":copy.keep,"missing_subtitles":copy.missing_subtitles,"access_error":true})),
            }
        }
        if !stale.is_empty() {
            let mut index = self.lock_index()?;
            for key in stale {
                if let Some(copy) = index.copies.get(&key) {
                    let id = media_id(&copy.media)?;
                    let current = self
                        .media_directory(context, id, false)
                        .and_then(|folder| regular_file(&folder.join(&copy.file), copy.bytes));
                    if current.is_err_and(|error| error.code == "OFFLINE_MISSING") {
                        index.copies.remove(&key);
                    }
                }
            }
            self.save_index(&index)?;
        }
        let mut jobs: Vec<Value> = snapshot
            .partials
            .values()
            .filter(|p| p.scope == scope)
            .map(|p| json!({"media":p.media,"bytes":p.bytes,"version":p.version,"transferred":p.transferred,"status":"interrupted"}))
            .collect();
        if let Ok(current) = self.jobs.lock()
            && let Some(job) = current.get(&scope)
            && job.status != "complete"
        {
            jobs.retain(|old| old["media"]["id"] != job.media["id"]);
            jobs.push(
                serde_json::to_value(job)
                    .map_err(|_| error("État local invalide.", "OFFLINE_INDEX"))?,
            );
        }
        let busy = jobs
            .iter()
            .any(|job| matches!(job["status"].as_str(), Some("planning" | "downloading")));
        let (free, capacity) = volume_capacity(&self.root);
        let enabled = snapshot.preferences.get(&scope).copied().unwrap_or(true);
        Ok(
            json!({"items":items,"jobs":jobs,"total":total,"free":free,"capacity":capacity,"enabled":enabled,"supported":true,"busy":busy}),
        )
    }

    async fn run_queue(
        &self,
        context: OfflineContext,
        media_id: String,
        scope: String,
        cancelled: Arc<AtomicBool>,
        automatic_limit: Option<u64>,
    ) -> Result<(), UiError> {
        let (items, _) = plan_all(&context, &media_id, &scope).await?;
        if items.is_empty() {
            self.job(
                &context,
                &json!({"id":media_id,"title":""}),
                0,
                0,
                "complete",
            )?;
        }
        let mut last_error = None;
        for item in items {
            if cancelled.load(Ordering::Acquire) {
                break;
            }
            let allowance = if let Some(limit) = automatic_limit {
                let snapshot = self.list(&context)?;
                if snapshot["items"].as_array().into_iter().flatten().any(|copy| copy["media"]["id"] == item.media["id"]) {
                    self.job(&context, &item.media, item.bytes, item.bytes, "complete")?;
                    continue;
                }
                let used: u64 = snapshot["items"].as_array().into_iter().flatten()
                    .filter(|copy| copy["automatic"] == true)
                    .filter_map(|copy| copy["local_bytes"].as_u64()).fold(0, u64::saturating_add);
                let available = limit.saturating_sub(used);
                if item.bytes > available {
                    return Err(error("Plafond ou espace libre insuffisant.", "DOWNLOAD_AHEAD_SPACE"));
                }
                Some(available)
            } else {
                None
            };
            if let Err(problem) = self.download_one(&context, item, cancelled.clone(), allowance).await {
                if cancelled.load(Ordering::Acquire) {
                    break;
                }
                last_error = Some(problem);
            }
        }
        last_error.map_or(Ok(()), Err)
    }

    async fn download_one(
        &self,
        context: &OfflineContext,
        item: DownloadPlanItem,
        cancelled: Arc<AtomicBool>,
        automatic_allowance: Option<u64>,
    ) -> Result<(), UiError> {
        let media = media_id(&item.media)?.to_owned();
        valid_version(&item.version)?;
        let download_version = item.version.clone();
        if item.bytes == 0 {
            return Err(error("Vidéo vide.", "DOWNLOAD_INVALID"));
        }
        let key = copy_key(context, &media)?;
        let existing = self.lock_index()?.copies.get(&key).cloned();
        if let Some(copy) = existing.as_ref().filter(|copy| {
            copy.version == item.version && copy.bytes == item.bytes && copy.missing_subtitles == 0
        }) && self
            .media_directory(context, &media, false)
            .and_then(|folder| regular_file(&folder.join(&copy.file), item.bytes))
            .is_ok()
        {
            return Ok(());
        }
        let resume_allowed = self
            .lock_index()?
            .partials
            .get(&key)
            .is_some_and(|partial| {
                partial.scope == context.scope()
                    && partial.version == item.version
                    && partial.bytes == item.bytes
            });
        // Publish the partial marker while excluding a new local reader. Acquire
        // rechecks this marker under the same reader lock, so replacement cannot
        // unlink a source or sidecar while the native player owns it.
        {
            let readers = self
                .readers
                .lock()
                .map_err(|_| error("Lecture locale occupée.", "OFFLINE_BUSY"))?;
            if readers.get(&key).copied().unwrap_or(0) > 0 {
                return Err(error(
                    "Fermez la vidéo avant de remplacer sa copie.",
                    "OFFLINE_IN_USE",
                ));
            }
            let transferred = if resume_allowed {
                self.lock_index()?
                    .partials
                    .get(&key)
                    .map(|partial| partial.transferred)
                    .unwrap_or(0)
            } else {
                0
            };
            self.save_partial(context, &key, &item, transferred)?;
            drop(readers);
        }
        let transport = context
            .transport
            .clone()
            .ok_or_else(|| error("Serveur indisponible.", "DOWNLOAD_OFFLINE"))?;
        let device = context
            .device_id
            .as_deref()
            .ok_or_else(|| error("Appareil invalide.", "DOWNLOAD_CONTEXT"))?;
        let mut session = transport
            .begin_download(&context.profile_id, &media, &item.version, device)
            .await
            .map_err(UiError::from)?;
        let result: Result<(), UiError> = async {
            validate_session(&item, &session)?;
            let directory = self.media_directory(context, &media, true)?;
            let (video_name, part_name) = video_names(&item.version)?;
            let part = directory.join(&part_name);
            let mut file = writable_file(&part)?;
            let mut position = file.metadata().map_err(local_io)?.len();
            if !resume_allowed || position > item.bytes {
                file.set_len(0).map_err(local_io)?;
                position = 0;
            }
            let automatic_allowance = automatic_allowance.map(|limit| {
                let free = volume_capacity(&self.root).0.unwrap_or(u64::MAX);
                limit.min(free.saturating_add(position))
            });
            if automatic_allowance.is_some_and(|limit| item.bytes > limit) {
                return Err(error("Plafond ou espace libre insuffisant.", "DOWNLOAD_AHEAD_SPACE"));
            }
            file.seek(SeekFrom::Start(position)).map_err(local_io)?;
            self.save_partial(context, &key, &item, position)?;
            self.job(context, &item.media, item.bytes, position, "downloading")?;
            let mut renewed = Instant::now();
            while position < item.bytes {
                if cancelled.load(Ordering::Acquire) {
                    self.job(context, &item.media, item.bytes, position, "interrupted")?;
                    return Ok(());
                }
                if renewed.elapsed() >= Duration::from_secs(60) {
                    let _ = transport
                        .cancel_download(&context.profile_id, &media, &session.token)
                        .await;
                    session = transport
                        .begin_download(&context.profile_id, &media, &item.version, device)
                        .await
                        .map_err(UiError::from)?;
                    validate_session(&item, &session)?;
                    renewed = Instant::now();
                }
                let wanted = (item.bytes - position).min(CHUNK as u64) as usize;
                let transport2 = transport.clone();
                let profile = context.profile_id.clone();
                let media2 = media.clone();
                let token = session.token.clone();
                let version = item.version.clone();
                let start = position;
                let (buffer, read) = tokio::task::spawn_blocking(move || {
                    let mut buffer = vec![0_u8; wanted];
                    let read = transport2.read_download_range_blocking(
                        &profile,
                        &media2,
                        &token,
                        start,
                        &mut buffer,
                        Some(&version),
                    )?;
                    Ok::<_, chill_client::ClientError>((buffer, read))
                })
                .await
                .map_err(|_| error("Téléchargement interrompu.", "DOWNLOAD_INTERRUPTED"))?
                .map_err(UiError::from)?;
                if read.bytes_read != wanted || read.total_size != Some(item.bytes) {
                    return Err(error("Plage vidéo incohérente.", "DOWNLOAD_RANGE"));
                }
                file.write_all(&buffer).map_err(local_io)?;
                position = position.saturating_add(wanted as u64);
                if position == item.bytes || position % (8 * CHUNK as u64) == 0 {
                    self.save_partial(context, &key, &item, position)?;
                }
                self.job(context, &item.media, item.bytes, position, "downloading")?;
            }
            file.sync_all().map_err(local_io)?;
            if file.metadata().map_err(local_io)?.len() != item.bytes {
                return Err(error("Copie vidéo incomplète.", "DOWNLOAD_RANGE"));
            }
            drop(file);

            let mut subtitles = Vec::new();
            let mut missing = 0u64;
            let mut stored_bytes = item.bytes;
            for subtitle in session.subtitles.iter().take(MAX_SUBTITLES) {
                if automatic_allowance.is_some_and(|limit| stored_bytes.saturating_add(subtitle.bytes) > limit) {
                    missing += 1;
                    continue;
                }
                if subtitle.bytes > SUBTITLE_LIMIT || uuid::Uuid::parse_str(&subtitle.id).is_err() {
                    missing += 1;
                    continue;
                }
                match transport
                    .download_offline_subtitle(
                        &context.profile_id,
                        &media,
                        &subtitle.id,
                        &session.token,
                    )
                    .await
                {
                    Ok(bytes) if bytes.len() as u64 == subtitle.bytes => {
                        let name = format!("subtitle-{}", subtitle.id);
                        if atomic_bytes(&directory, &name, &bytes).is_ok() {
                            stored_bytes = stored_bytes.saturating_add(subtitle.bytes);
                            subtitles.push(SubtitleRecord {
                                id: subtitle.id.clone(),
                                title: subtitle.title.clone(),
                                language: subtitle.language.clone(),
                                bytes: subtitle.bytes,
                                file: name,
                            });
                        } else {
                            missing += 1;
                        }
                    }
                    _ => missing += 1,
                }
            }
            let mut poster = None;
            if item
                .media
                .get("poster")
                .is_some_and(|value| !value.is_null())
                && let Ok(bytes) = transport
                    .get_binary(
                        &format!(
                            "{}/profiles/{}/media/{media}/artwork",
                            chill_client::API_PREFIX,
                            context.profile_id
                        ),
                        &[],
                        POSTER_LIMIT,
                    )
                    .await
                && poster_mime(&bytes).is_some()
                && automatic_allowance.is_none_or(|limit| stored_bytes.saturating_add(bytes.len() as u64) <= limit)
                && atomic_bytes(&directory, "poster", &bytes).is_ok()
            {
                poster = Some(bytes.len() as u64);
            }
            if cancelled.load(Ordering::Acquire) {
                return Err(error("Téléchargement annulé.", "DOWNLOAD_CANCELLED"));
            }
            fs::rename(&part, directory.join(&video_name)).map_err(local_io)?;
            let video_durability = sync_directory(&directory);
            let mut index = self.lock_index()?;
            let keep = index.copies.get(&key).is_some_and(|copy| copy.keep);
            let previous = index.copies.get(&key).cloned();
            index.copies.insert(
                key.clone(),
                CopyRecord {
                    scope: context.scope(),
                    media: item.media.clone(),
                    series_key: item.series_key,
                    bytes: item.bytes,
                    version: item.version,
                    file: video_name.clone(),
                    keep,
                    automatic: automatic_allowance.is_some(),
                    missing_subtitles: missing,
                    poster,
                    delete_pending: false,
                    deletion_failed: false,
                    subtitles: subtitles.clone(),
                    preferences: session.preferences.clone(),
                    personal: session.personal.clone(),
                    progress_revision: session.progress_revision.clone(),
                    segments: session.segments.clone(),
                },
            );
            index.partials.remove(&key);
            self.save_index(&index)?;
            drop(index);
            if let Some(previous) = previous {
                if previous.file != video_name {
                    let _ = remove_regular_if_present(&directory.join(previous.file));
                }
                for old in previous.subtitles {
                    if !subtitles.iter().any(|current| current.file == old.file) {
                        let _ = remove_regular_if_present(&directory.join(old.file));
                    }
                }
            }
            video_durability?;
            self.job(context, &item.media, item.bytes, item.bytes, "complete")?;
            Ok(())
        }
        .await;
        let _ = transport
            .cancel_download(&context.profile_id, &media, &session.token)
            .await;
        if (result.is_err() || cancelled.load(Ordering::Acquire))
            && self.lock_index()?.copies.contains_key(&key)
        {
            let mut index = self.lock_index()?;
            index.partials.remove(&key);
            self.save_index(&index)?;
            drop(index);
            let (_, part_name) = video_names(&download_version)?;
            remove_regular_if_present(
                &self
                    .media_directory(context, &media, false)?
                    .join(part_name),
            )?;
        }
        result
    }

    fn save_partial(
        &self,
        context: &OfflineContext,
        key: &str,
        item: &DownloadPlanItem,
        transferred: u64,
    ) -> Result<(), UiError> {
        let mut index = self.lock_index()?;
        index.partials.insert(
            key.into(),
            PartialRecord {
                scope: context.scope(),
                media: item.media.clone(),
                series_key: item.series_key.clone(),
                bytes: item.bytes,
                version: item.version.clone(),
                transferred,
            },
        );
        self.save_index(&index)
    }

    fn job(
        &self,
        context: &OfflineContext,
        media: &Value,
        bytes: u64,
        transferred: u64,
        status: &str,
    ) -> Result<(), UiError> {
        self.jobs
            .lock()
            .map_err(|_| error("État local indisponible.", "OFFLINE_BUSY"))?
            .insert(
                context.scope(),
                JobView {
                    media: media.clone(),
                    bytes,
                    transferred,
                    status: status.into(),
                },
            );
        Ok(())
    }

    pub fn acquire(
        &self,
        context: &OfflineContext,
        media_id: &str,
    ) -> Result<OfflineLease, UiError> {
        let key = copy_key(context, media_id)?;
        let copy = self
            .lock_index()?
            .copies
            .get(&key)
            .cloned()
            .ok_or_else(|| error("Copie locale indisponible.", "OFFLINE_MISSING"))?;
        let mut readers = self
            .readers
            .lock()
            .map_err(|_| error("Lecture locale occupée.", "OFFLINE_BUSY"))?;
        if self.lock_index()?.partials.contains_key(&key) {
            return Err(error(
                "La copie locale est en cours de remplacement.",
                "OFFLINE_BUSY",
            ));
        }
        *readers.entry(key.clone()).or_default() += 1;
        drop(readers);
        let reader = ReaderGuard {
            key: key.clone(),
            readers: self.readers.clone(),
        };
        let directory = self.media_directory(context, media_id, false)?;
        let file = regular_file(&directory.join(&copy.file), copy.bytes)?;
        let mut subtitles = Vec::new();
        for subtitle in &copy.subtitles {
            if regular_file(&directory.join(&subtitle.file), subtitle.bytes).is_ok() {
                subtitles.push(OfflineSubtitle {
                    id: subtitle.id.clone(),
                    title: subtitle.title.clone(),
                    language: subtitle.language.clone(),
                    path: directory.join(&subtitle.file),
                });
            }
        }
        let progress = self
            .lock_index()?
            .progress
            .get(&key)
            .map(serde_json::to_value)
            .transpose()
            .map_err(|_| error("Progression locale invalide.", "OFFLINE_INDEX"))?;
        Ok(OfflineLease {
            media: copy.media,
            series_key: copy.series_key,
            bytes: copy.bytes,
            version: copy.version,
            file,
            subtitles,
            preferences: copy.preferences,
            personal: copy.personal,
            progress_revision: copy.progress_revision,
            progress,
            _reader: reader,
        })
    }

    pub fn save_progress(
        &self,
        context: &OfflineContext,
        media_id: &str,
        position_ms: u64,
        duration_ms: u64,
        watched: bool,
        base_revision: Option<String>,
        pending: bool,
    ) -> Result<(), UiError> {
        let key = copy_key(context, media_id)?;
        let mut index = self.lock_index()?;
        if !index.copies.contains_key(&key) {
            return Err(error("Copie locale absente.", "OFFLINE_MISSING"));
        }
        let auto = index
            .preferences
            .get(&context.scope())
            .copied()
            .unwrap_or(true);
        if watched
            && auto
            && let Some(copy) = index.copies.get_mut(&key)
            && !copy.keep
        {
            copy.delete_pending = true;
        }
        index.progress.insert(
            key,
            ProgressRecord {
                position_ms,
                duration_ms,
                watched,
                base_revision,
                event_id: uuid::Uuid::new_v4().to_string(),
                pending,
            },
        );
        self.save_index(&index)
    }

    pub async fn sync(&self, context: &OfflineContext) -> Result<Value, UiError> {
        context.validate()?;
        let transport = context
            .transport
            .as_ref()
            .ok_or_else(|| error("Serveur indisponible.", "DOWNLOAD_OFFLINE"))?;
        let scope = context.scope();
        let pending: Vec<(String, String, ProgressRecord)> = self
            .lock_index()?
            .progress
            .iter()
            .filter_map(|(key, progress)| {
                let media = key.strip_prefix(&format!("{scope}:"))?;
                (progress.pending && valid_uuid(media).is_ok())
                    .then(|| (key.clone(), media.to_owned(), progress.clone()))
            })
            .collect();
        if pending.is_empty() {
            return Ok(json!({"synced":0}));
        }
        let remote: Value = transport.get_json("/api/2026-10-14/server").await?;
        if remote["server_id"].as_str() != Some(&context.server_id) {
            return Ok(json!({"synced":0}));
        }
        let mut synced = 0u64;
        for (key, media, progress) in pending {
            let Some(base_revision) = progress.base_revision.as_deref() else {
                continue;
            };
            let response = match transport
                .sync_download_progress(
                    &context.profile_id,
                    &media,
                    &progress.event_id,
                    base_revision,
                    progress.position_ms,
                    progress.duration_ms,
                    progress.watched,
                )
                .await
            {
                Ok(response) => response,
                Err(chill_client::ClientError::Api { status, .. })
                    if matches!(status.as_u16(), 404 | 409) =>
                {
                    continue;
                }
                Err(chill_client::ClientError::Http(status))
                    if matches!(status.as_u16(), 404 | 409) =>
                {
                    continue;
                }
                Err(problem) => return Err(UiError::from(problem)),
            };
            let revision = response["revision"].as_str().map(str::to_owned);
            let mut index = self.lock_index()?;
            if let Some(current) = index.progress.get_mut(&key)
                && current.event_id == progress.event_id
            {
                current.pending = false;
                if revision.is_some() {
                    current.base_revision = revision;
                }
                self.save_index(&index)?;
                synced += 1;
            }
        }
        Ok(json!({"synced":synced}))
    }

    pub fn auto_delete(&self, context: &OfflineContext, media_id: &str) -> Result<bool, UiError> {
        let key = copy_key(context, media_id)?;
        match self.delete_one(context, media_id, true) {
            Ok(()) => Ok(true),
            Err(problem) if problem.code == "OFFLINE_IN_USE" => Ok(false),
            Err(problem) => {
                let mut index = self.lock_index()?;
                if let Some(copy) = index.copies.get_mut(&key) {
                    copy.delete_pending = false;
                    copy.deletion_failed = problem.code != "OFFLINE_MISSING";
                    self.save_index(&index)?;
                }
                Err(problem)
            }
        }
    }

    fn delete_one(
        &self,
        context: &OfflineContext,
        media_id: &str,
        automatic: bool,
    ) -> Result<(), UiError> {
        let key = copy_key(context, media_id)?;
        let readers = self
            .readers
            .lock()
            .map_err(|_| error("Lecture locale occupée.", "OFFLINE_BUSY"))?;
        if readers.get(&key).copied().unwrap_or(0) > 0 {
            return Err(error(
                "Fermez la vidéo avant de supprimer sa copie.",
                "OFFLINE_IN_USE",
            ));
        }
        let mut index = self.lock_index()?;
        let copy = index.copies.get(&key).cloned();
        if automatic {
            let enabled = index
                .preferences
                .get(&context.scope())
                .copied()
                .unwrap_or(true);
            let Some(copy) = index.copies.get_mut(&key) else {
                return Ok(());
            };
            if !copy.delete_pending {
                return Ok(());
            }
            if !enabled || copy.keep {
                copy.delete_pending = false;
                return self.save_index(&index);
            }
        }
        let directory = self.media_directory(context, media_id, false)?;
        remove_known_files(&directory, copy.as_ref())?;
        index.copies.remove(&key);
        index.partials.remove(&key);
        let saved = self.save_index(&index);
        drop(readers);
        saved
    }

    fn media_directory(
        &self,
        context: &OfflineContext,
        media_id: &str,
        create: bool,
    ) -> Result<PathBuf, UiError> {
        context.validate()?;
        valid_uuid(media_id)?;
        let mut current = self.root.clone();
        for component in [&context.server_id, &context.profile_id, media_id] {
            current.push(component);
            if create {
                private_directory(&current)?;
            } else {
                require_directory(&current)?;
            }
        }
        Ok(current)
    }

    fn save_index(&self, index: &Index) -> Result<(), UiError> {
        let snapshot = serde_json::to_string(index).map_err(|_| index_error())?;
        if snapshot.len() as u64 > INDEX_LIMIT {
            return Err(error("Index local trop volumineux.", "OFFLINE_INDEX"));
        }
        let mut database = self.database.lock().map_err(|_| index_error())?;
        let transaction = database.transaction().map_err(|_| index_error())?;
        transaction
            .execute(
                "INSERT INTO download_index(kind, entry_key, payload)
             SELECT groups.key, entries.key, CASE entries.type
               WHEN 'true' THEN 'true' WHEN 'false' THEN 'false' ELSE entries.value END
             FROM json_each(?1) AS groups, json_each(groups.value) AS entries WHERE true
             ON CONFLICT(kind, entry_key) DO UPDATE SET payload=excluded.payload
             WHERE payload <> excluded.payload",
                [&snapshot],
            )
            .map_err(|_| index_error())?;
        transaction.execute(
            "DELETE FROM download_index WHERE (kind, entry_key) NOT IN
             (SELECT groups.key, entries.key FROM json_each(?1) AS groups, json_each(groups.value) AS entries)",
            [&snapshot],
        ).map_err(|_| index_error())?;
        transaction.commit().map_err(|_| index_error())
    }

    fn lock_index(&self) -> Result<std::sync::MutexGuard<'_, Index>, UiError> {
        self.index
            .lock()
            .map_err(|_| error("Index local indisponible.", "OFFLINE_BUSY"))
    }
}

async fn plan_all(
    context: &OfflineContext,
    media_id: &str,
    scope: &str,
) -> Result<(Vec<DownloadPlanItem>, u64), UiError> {
    valid_uuid(media_id)?;
    valid_scope(scope)?;
    let transport = context
        .transport
        .as_ref()
        .ok_or_else(|| error("Serveur indisponible.", "DOWNLOAD_OFFLINE"))?;
    let mut items = Vec::new();
    let mut skipped = 0u64;
    let mut offset = 0u64;
    loop {
        let page = transport
            .download_plan(&context.profile_id, media_id, scope, offset)
            .await
            .map_err(UiError::from)?;
        if items.len().saturating_add(page.items.len()) > MAX_ITEMS {
            return Err(error("Sélection trop importante.", "DOWNLOAD_LIMIT"));
        }
        items.extend(page.items);
        skipped = skipped.saturating_add(page.skipped);
        match page.next_offset {
            Some(next) if next > offset && next <= MAX_ITEMS as u64 => offset = next,
            Some(_) => return Err(error("Pagination invalide.", "DOWNLOAD_INVALID")),
            None => break,
        }
    }
    Ok((items, skipped))
}

fn validate_session(item: &DownloadPlanItem, session: &DownloadSession) -> Result<(), UiError> {
    if session.bytes != item.bytes
        || session.version != item.version
        || session.subtitles.len() > MAX_SUBTITLES
    {
        return Err(error(
            "La vidéo a changé pendant le téléchargement.",
            "DOWNLOAD_CHANGED",
        ));
    }
    Ok(())
}
fn valid_uuid(id: &str) -> Result<(), UiError> {
    uuid::Uuid::parse_str(id).map(|_| ()).map_err(|_| {
        error(
            "Identifiant de téléchargement invalide.",
            "DOWNLOAD_INVALID",
        )
    })
}
fn valid_scope(scope: &str) -> Result<(), UiError> {
    if matches!(scope, "video" | "season" | "series") {
        Ok(())
    } else {
        Err(error("Sélection inconnue.", "DOWNLOAD_INVALID"))
    }
}
fn file_version_label(version: &str) -> &str {
    version
        .strip_prefix('"')
        .and_then(|label| label.strip_suffix('"'))
        .unwrap_or(version)
}
fn valid_version(version: &str) -> Result<(), UiError> {
    let version = file_version_label(version);
    if !version.is_empty()
        && version.len() <= 128
        && version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':'))
    {
        Ok(())
    } else {
        Err(error(
            "Version de téléchargement invalide.",
            "DOWNLOAD_INVALID",
        ))
    }
}
fn default_video_file() -> String {
    "video".into()
}
fn video_names(version: &str) -> Result<(String, String), UiError> {
    valid_version(version)?;
    let name = format!("video-{}", file_version_label(version));
    Ok((name.clone(), format!("{name}.part")))
}
fn media_id(media: &Value) -> Result<&str, UiError> {
    let id = media["id"]
        .as_str()
        .ok_or_else(|| error("Vidéo de téléchargement invalide.", "DOWNLOAD_INVALID"))?;
    valid_uuid(id)?;
    Ok(id)
}
fn copy_key(context: &OfflineContext, media_id: &str) -> Result<String, UiError> {
    context.validate()?;
    valid_uuid(media_id)?;
    Ok(format!("{}:{}", context.scope(), media_id))
}

fn private_directory(path: &Path) -> Result<(), UiError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => {}
        Err(problem) if problem.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(path).map_err(local_io)?;
        }
        _ => return Err(error("Dossier de téléchargements non sûr.", "OFFLINE_PATH")),
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(local_io)
}
fn require_directory(path: &Path) -> Result<(), UiError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => Ok(()),
        Err(problem) if problem.kind() == std::io::ErrorKind::NotFound => {
            Err(error("Copie locale absente.", "OFFLINE_MISSING"))
        }
        _ => Err(error("Dossier de téléchargements non sûr.", "OFFLINE_PATH")),
    }
}
fn no_follow() -> i32 {
    #[cfg(target_os = "macos")]
    {
        0x100
    }
    #[cfg(not(target_os = "macos"))]
    {
        0x20000
    }
}
fn writable_file(path: &Path) -> Result<File, UiError> {
    let file = fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .mode(0o600)
        .custom_flags(no_follow())
        .open(path)
        .map_err(local_io)?;
    if !file.metadata().map_err(local_io)?.is_file() {
        return Err(error("Fichier local invalide.", "OFFLINE_PATH"));
    }
    Ok(file)
}
fn regular_file(path: &Path, expected: u64) -> Result<File, UiError> {
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(no_follow())
        .open(path)
        .map_err(|problem| {
            if problem.kind() == std::io::ErrorKind::NotFound {
                error("Copie locale absente.", "OFFLINE_MISSING")
            } else {
                local_io(problem)
            }
        })?;
    let meta = file.metadata().map_err(local_io)?;
    if !meta.is_file() || meta.len() != expected {
        return Err(error("Copie locale incomplète.", "OFFLINE_MISSING"));
    }
    Ok(file)
}
fn atomic_bytes(directory: &Path, name: &str, bytes: &[u8]) -> Result<(), UiError> {
    let temporary = directory.join(format!(".{name}-{}.part", uuid::Uuid::new_v4()));
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .custom_flags(no_follow())
        .open(&temporary)
        .map_err(local_io)?;
    let written = file
        .write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(local_io)
        .and_then(|_| fs::rename(&temporary, directory.join(name)).map_err(local_io))
        .and_then(|_| sync_directory(directory));
    if written.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    written
}
fn sync_directory(path: &Path) -> Result<(), UiError> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(local_io)
}
fn poster_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        Some("image/webp")
    } else {
        None
    }
}
fn index_error() -> UiError {
    error("Index local indisponible ou invalide.", "OFFLINE_INDEX")
}

fn open_index(root: &Path) -> Result<Connection, UiError> {
    let path = root.join("index.sqlite3");
    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
        | OpenFlags::SQLITE_OPEN_CREATE
        | OpenFlags::SQLITE_OPEN_NOFOLLOW;
    let connection = Connection::open_with_flags(&path, flags).map_err(|_| index_error())?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(local_io)?;
    let version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|_| index_error())?;
    if version > 1 {
        return Err(error("Version d’index local inconnue.", "OFFLINE_INDEX"));
    }
    connection
        .execute_batch(
            "PRAGMA synchronous=FULL;
         CREATE TABLE IF NOT EXISTS download_index(
           kind TEXT NOT NULL,
           entry_key TEXT NOT NULL,
           payload TEXT NOT NULL,
           PRIMARY KEY(kind, entry_key)
         ) WITHOUT ROWID;
         PRAGMA user_version=1;",
        )
        .map_err(|_| index_error())?;
    Ok(connection)
}

fn read_index(database: &Connection) -> Result<Index, UiError> {
    let bytes: i64 = database
        .query_row(
            "SELECT coalesce(sum(length(payload)), 0) FROM download_index",
            [],
            |row| row.get(0),
        )
        .map_err(|_| index_error())?;
    if bytes > INDEX_LIMIT as i64 {
        return Err(error("Index local trop volumineux.", "OFFLINE_INDEX"));
    }
    let mut statement = database
        .prepare("SELECT kind, entry_key, payload FROM download_index")
        .map_err(|_| index_error())?;
    let mut rows = statement.query([]).map_err(|_| index_error())?;
    let mut index = Index::default();
    while let Some(row) = rows.next().map_err(|_| index_error())? {
        let kind: String = row.get(0).map_err(|_| index_error())?;
        let key: String = row.get(1).map_err(|_| index_error())?;
        let payload: String = row.get(2).map_err(|_| index_error())?;
        match kind.as_str() {
            "copies" => {
                index.copies.insert(
                    key,
                    serde_json::from_str(&payload).map_err(|_| index_error())?,
                );
            }
            "partials" => {
                index.partials.insert(
                    key,
                    serde_json::from_str(&payload).map_err(|_| index_error())?,
                );
            }
            "preferences" => {
                index.preferences.insert(
                    key,
                    serde_json::from_str(&payload).map_err(|_| index_error())?,
                );
            }
            "progress" => {
                index.progress.insert(
                    key,
                    serde_json::from_str(&payload).map_err(|_| index_error())?,
                );
            }
            _ => return Err(index_error()),
        }
    }
    Ok(index)
}
fn remove_known_files(directory: &Path, copy: Option<&CopyRecord>) -> Result<(), UiError> {
    let mut allowed = vec![
        "video".to_owned(),
        "video.part".to_owned(),
        "poster".to_owned(),
    ];
    if let Some(copy) = copy {
        allowed.push(copy.file.clone());
        allowed.extend(copy.subtitles.iter().map(|subtitle| subtitle.file.clone()));
    }
    let entries: Vec<_> = fs::read_dir(directory)
        .map_err(local_io)?
        .take(130)
        .collect::<Result<_, _>>()
        .map_err(local_io)?;
    if entries.len() > 128
        || entries.iter().any(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            !allowed.iter().any(|allowed| allowed == &name) && !managed_orphan(&name)
        })
    {
        return Err(error(
            "Le dossier contient une entrée inattendue ; suppression refusée.",
            "OFFLINE_PATH",
        ));
    }
    for entry in entries {
        let kind = entry.file_type().map_err(local_io)?;
        if !kind.is_file() || kind.is_symlink() {
            return Err(error("Entrée locale non sûre.", "OFFLINE_PATH"));
        }
        fs::remove_file(entry.path()).map_err(local_io)?;
    }
    fs::remove_dir(directory).map_err(local_io)?;
    Ok(())
}
fn managed_orphan(name: &str) -> bool {
    if let Some(version) = name.strip_prefix("video-") {
        let version = version.strip_suffix(".part").unwrap_or(version);
        return valid_version(version).is_ok();
    }
    if let Some(id) = name.strip_prefix("subtitle-") {
        return uuid::Uuid::parse_str(id).is_ok();
    }
    name.strip_prefix(".subtitle-")
        .and_then(|value| value.strip_suffix(".part"))
        .is_some_and(|value| {
            value.len() <= 160
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}
fn remove_regular_if_present(path: &Path) -> Result<(), UiError> {
    match fs::symlink_metadata(path) {
        Err(problem) if problem.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() => {
            fs::remove_file(path).map_err(local_io)
        }
        _ => Err(error("Entrée locale non sûre.", "OFFLINE_PATH")),
    }
}
fn local_io(_: std::io::Error) -> UiError {
    error("Stockage local inaccessible.", "OFFLINE_IO")
}
fn volume_capacity(path: &Path) -> (Option<u64>, Option<u64>) {
    let Ok(path) = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()) else {
        return (None, None);
    };
    let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(path.as_ptr(), &mut stats) } != 0 {
        return (None, None);
    }
    let unit = stats.f_frsize as u64;
    (
        Some((stats.f_bavail as u64).saturating_mul(unit)),
        Some((stats.f_blocks as u64).saturating_mul(unit)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SERVER: &str = "11111111-1111-4111-8111-111111111111";
    const PROFILE: &str = "22222222-2222-4222-8222-222222222222";
    const MEDIA: &str = "33333333-3333-4333-8333-333333333333";

    fn context() -> OfflineContext {
        OfflineContext::offline(SERVER.into(), PROFILE.into()).unwrap()
    }
    fn ready(host: &OfflineDownloads, context: &OfflineContext) {
        let directory = host.media_directory(context, MEDIA, true).unwrap();
        atomic_bytes(&directory, "video-v1", b"video").unwrap();
        let key = copy_key(context, MEDIA).unwrap();
        let mut index = host.lock_index().unwrap();
        index.copies.insert(
            key,
            CopyRecord {
                scope: context.scope(),
                media: json!({"id":MEDIA,"title":"Fixture","category":"film"}),
                series_key: None,
                bytes: 5,
                version: "v1".into(),
                file: "video-v1".into(),
                keep: false,
                automatic: false,
                missing_subtitles: 0,
                poster: None,
                delete_pending: false,
                deletion_failed: false,
                subtitles: Vec::new(),
                preferences: json!({}),
                personal: json!({}),
                progress_revision: Some("r1".into()),
                segments: Vec::new(),
            },
        );
        host.save_index(&index).unwrap();
    }

    #[tokio::test]
    async fn settings_and_ready_copy_survive_reopen() {
        let directory = tempfile::tempdir().unwrap();
        let host = Arc::new(OfflineDownloads::open(directory.path()).unwrap());
        let context = context();
        ready(&host, &context);
        host.command(context.clone(), DownloadCommand::SetAuto { enabled: false })
            .await
            .unwrap();
        let reopened = OfflineDownloads::open(directory.path()).unwrap();
        let list = reopened.list(&context).unwrap();
        assert_eq!(list["items"].as_array().unwrap().len(), 1);
        assert_eq!(list["total"], 5);
        assert_eq!(list["enabled"], false);
        assert_eq!(list["busy"], false);
    }

    #[test]
    fn acquired_file_blocks_delete_and_progress_outlives_copy() {
        let directory = tempfile::tempdir().unwrap();
        let host = OfflineDownloads::open(directory.path()).unwrap();
        let context = context();
        ready(&host, &context);
        host.save_progress(
            &context,
            MEDIA,
            1_000,
            5_000,
            false,
            Some("r1".into()),
            true,
        )
        .unwrap();
        let lease = host.acquire(&context, MEDIA).unwrap();
        assert_eq!(lease.progress_revision.as_deref(), Some("r1"));
        assert_eq!(
            host.delete_one(&context, MEDIA, false).unwrap_err().code,
            "OFFLINE_IN_USE"
        );
        drop(lease);
        host.delete_one(&context, MEDIA, false).unwrap();
        assert_eq!(
            host.lock_index()
                .unwrap()
                .progress
                .values()
                .filter(|progress| progress.pending)
                .count(),
            1
        );
    }

    #[test]
    fn missing_media_directory_is_reconciled() {
        let directory = tempfile::tempdir().unwrap();
        let host = OfflineDownloads::open(directory.path()).unwrap();
        let context = context();
        ready(&host, &context);
        let media = host.media_directory(&context, MEDIA, false).unwrap();
        fs::remove_file(media.join("video-v1")).unwrap();
        fs::remove_dir(media).unwrap();
        let list = host.list(&context).unwrap();
        assert_eq!(list["items"], json!([]));
        assert_eq!(list["total"], 0);
        assert!(host.lock_index().unwrap().copies.is_empty());
    }

    #[test]
    fn download_root_symlink_is_refused() {
        use std::os::unix::fs::symlink;
        let directory = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        symlink(target.path(), directory.path().join("downloads")).unwrap();
        assert_eq!(
            OfflineDownloads::open(directory.path()).err().unwrap().code,
            "OFFLINE_PATH"
        );
    }
}
