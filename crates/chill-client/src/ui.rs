//! Allowlisted UI operations. No JS-supplied HTTP paths, headers or local/NAS paths.
use crate::{
    ClientError, HttpMethod, PresentedCertificate, ServerEndpoint, Transport, TrustEventKind,
    TrustStore, inspect_certificate,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Arc, RwLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum UiCommand {
    TmdbStatus,
    TmdbConfigure {
        credential_type: String,
        credential: SecretCredential,
        auto_enrich: bool,
    },
    TmdbClear,
    TmdbJobStart,
    TmdbJobStatus {
        id: String,
    },
    TmdbJobCancel {
        id: String,
    },
    TmdbJobRetry {
        id: String,
    },
    Boot,
    Connect {
        address: String,
    },
    Approve {
        token: String,
    },
    Disconnect,
    SelectProfile {
        id: String,
    },
    CreateProfile {
        name: String,
    },
    RenameProfile {
        id: String,
        name: String,
        expected_revision: u64,
    },
    DeleteProfile {
        id: String,
        confirm_id: String,
    },
    Refresh,
    /// Host-intercepted: transport runs after the UI client mutex is released.
    SyncCatalog,
    SaveLibrary {
        video_root: Option<String>,
        /// `None` preserves additional locations for clients compiled before
        /// multi-library support; an explicit list replaces them.
        additional_video_roots: Option<Vec<String>>,
        expected_revision: u64,
    },
    SavePreferences {
        audio_language: String,
        subtitle_language: String,
        autoplay_seconds: Option<u8>,
    },
    Catalog {
        offset: usize,
        category: Option<String>,
        q: Option<String>,
        genre: Option<String>,
        year: Option<u16>,
        watched: Option<bool>,
        sort: Option<String>,
        grouped: Option<bool>,
        available: Option<bool>,
    },
    SetWatched {
        media_id: String,
        watched: bool,
    },
    SetRating {
        media_id: String,
        rating: Option<u8>,
    },
    MediaDetail {
        media_id: String,
    },
    EditMedia {
        media_id: String,
        expected_revision: u64,
        patch: Value,
    },
    Series {
        name: String,
    },
    SeasonArtwork {
        tv_id: i64,
        season: i32,
    },
    EditSeries {
        series: String,
        expected_revision: u64,
        patch: Value,
    },
    RelatedMedia {
        media_id: String,
    },
    Sagas,
    Playlists,
    Playlist {
        id: String,
    },
    CreatePlaylist {
        name: String,
        media_id: Option<String>,
    },
    AddPlaylist {
        id: String,
        expected_revision: u64,
        media_ids: Vec<String>,
    },
    RemovePlaylistMember {
        id: String,
        media_id: String,
        expected_revision: u64,
    },
    ReorderPlaylist {
        id: String,
        expected_revision: u64,
        expected_order: Vec<String>,
        media_ids: Vec<String>,
    },
    DeletePlaylist {
        id: String,
    },
    RenamePlaylist {
        id: String,
        expected_revision: u64,
        name: String,
    },
    Saga {
        id: String,
    },
    CreateSaga {
        name: String,
    },
    RenameSaga {
        id: String,
        expected_revision: u64,
        name: String,
    },
    AddSaga {
        id: String,
        expected_revision: u64,
        media_ids: Vec<String>,
    },
    RemoveSagaMember {
        id: String,
        expected_revision: u64,
        media_id: String,
    },
    ReorderSaga {
        id: String,
        expected_revision: u64,
        expected_order: Vec<String>,
        media_ids: Vec<String>,
    },
    DeleteSaga {
        id: String,
        expected_revision: u64,
    },
    Artwork {
        media_id: String,
    },
    SagaArtwork {
        id: String,
    },
    PlaylistArtwork {
        id: String,
    },
    SagaMemberArtwork {
        id: String,
        expected_revision: u64,
        media_id: String,
    },
    GroupRating {
        media_id: String,
        rating: Option<u8>,
        scope: String,
    },
    StartScan,
    ScanStatus,
    TmdbCandidates {
        media_id: String,
        query: Option<String>,
    },
    TmdbSelect {
        media_id: String,
        expected_revision: u64,
        tmdb_id: i64,
        kind: String,
        scope: String,
    },
    TmdbRefresh {
        media_id: String,
        expected_revision: u64,
        scope: String,
    },
    TmdbRestorePoster {
        media_id: String,
        expected_revision: u64,
    },
    Unavailable {
        offset: usize,
        limit: usize,
    },
    ForgetUnavailable {
        media_ids: Vec<String>,
        expected_epoch: u64,
        confirmed: bool,
    },
    ReassociationProposals {
        offset: usize,
    },
    ConfirmReassociation {
        source_id: String,
        target_id: String,
        expected_source_revision: u64,
        expected_target_revision: u64,
        expected_epoch: u64,
        confirmed: bool,
    },
    FingerprintStatus {
        media_id: String,
    },
    RequestFullFingerprint {
        media_id: String,
        confirmed: bool,
    },
    Theme {
        theme: String,
    },
    SaveLanguage {
        language: UiLanguage,
    },
    SaveAppearance {
        appearance: crate::appearance::Appearance,
    },
    CacheSettings,
    SaveCacheSettings {
        cache: crate::playback_cache::PlaybackCache,
    },
    SavePlayerSettings {
        fullscreen_on_start: bool,
    },
    ResolveAppearance {
        year: i32,
        month: u8,
        day: u8,
    },
}
/// A transient provider secret must not leak through Debug or client settings.
#[derive(Deserialize)]
#[serde(transparent)]
pub struct SecretCredential(String);
impl std::fmt::Debug for SecretCredential {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}
#[derive(Debug, Serialize)]
pub struct UiError {
    pub message: String,
    pub status: Option<u16>,
    pub code: String,
}
impl From<ClientError> for UiError {
    fn from(error: ClientError) -> Self {
        let (status, code) = match &error {
            ClientError::Api { status, code } => (Some(status.as_u16()), code.clone()),
            ClientError::Http(status) => (Some(status.as_u16()), "HTTP_ERROR".into()),
            ClientError::InvalidServerUrl(_) => (None, "ADDRESS_INVALID".into()),
            ClientError::Network(error) if error.is_timeout() => (None, "NETWORK_TIMEOUT".into()),
            ClientError::Certificate(_)
            | ClientError::Tls(_)
            | ClientError::ApprovalRequired
            | ClientError::KeyChangedBeforeExpiry => (None, "TLS_REFUSED".into()),
            _ => (None, "CONNECTION_FAILED".into()),
        };
        let message = match code.as_str() {
            "SCAN_ACTIVE" => {
                "Un scan est déjà en cours. Attendez sa fin avant de relancer ou de modifier le dossier."
            }
            "ROOT_REQUIRED" => {
                "Configurez un emplacement vidéo du serveur dans Configuration › Bibliothèque avant de scanner."
            }
            "CONFIG_INVALID_OR_UNWRITABLE" => {
                "Chemin invalide ou réglages non enregistrables. Vérifiez le chemin et les droits du serveur."
            }
            "PROFILE_NAME_TAKEN" => "Ce nom de profil est déjà utilisé. Choisissez un autre nom.",
            "PROFILE_NOT_FOUND" => {
                "Ce profil a été supprimé. Rechargez et choisissez un profil existant."
            }
            "MEDIA_NOT_FOUND" => "Cet élément n'existe plus. Rechargez le catalogue.",
            "MEDIA_FILE_MISSING" => {
                "Fichier absent : la vidéo est marquée indisponible. Retrouvez-la dans Configuration › Bibliothèque, sans perdre ses historiques."
            }
            "IMPORT_MULTIPLE_LIBRARY_LOCATIONS_UNSUPPORTED" => {
                "Cette archive contient plusieurs emplacements. Une correspondance explicite des emplacements est nécessaire ; aucun import n’a été appliqué."
            }
            "BUSY" | "DATABASE_OFFLINE" => {
                "Serveur occupé ou base indisponible. Réessayez dans quelques instants."
            }
            "REQUEST_TIMEOUT" | "NETWORK_TIMEOUT" => {
                "Délai dépassé. Rechargez pour vérifier le résultat avant de réessayer."
            }
            "ADDRESS_INVALID" => {
                "Adresse invalide. Saisissez un DNS ou une IP, éventuellement avec le port HTTPS."
            }
            "INVALID_OPERATION" => "Valeur invalide. Vérifiez les champs avant de réessayer.",
            "PROFILE_CONFLICT" | "CONFIG_CONFLICT" => {
                "Modification concurrente : rechargez les données avant de réessayer."
            }
            "CONFIG_DURABILITY_UNKNOWN" => {
                "Réglage serveur appliqué, mais durabilité disque non confirmée. Rechargez son état avant de réessayer."
            }
            "FINGERPRINT_BUSY" => {
                "Une empreinte est déjà en cours. Attendez son résultat puis relancez explicitement si nécessaire."
            }
            "TMDB_NOT_CONFIGURED" => {
                "TMDB n’est pas configuré sur le serveur. Ajoutez le jeton côté serveur puis réessayez."
            }
            "TMDB_UNAVAILABLE" => {
                "TMDB est momentanément indisponible. Les informations et affiches actuelles sont conservées."
            }
            "TMDB_ASSOCIATION_MISSING" => {
                "Cette fiche n’est pas encore associée à TMDB. Recherchez puis choisissez une fiche."
            }
            "TMDB_SCOPE_INVALID" => "Cette portée ne convient pas à la fiche sélectionnée.",
            _ if status == Some(404) => {
                "Profil ou élément supprimé : reconnectez/rechargez le serveur."
            }
            _ if status == Some(400) || status == Some(422) => {
                "Valeur invalide ou nom déjà utilisé. Vérifiez les champs."
            }
            _ if status == Some(409) => "Modification concurrente : rechargez les données.",
            _ if status.is_some() => {
                "Le serveur a refusé l'opération. Rechargez puis vérifiez sa disponibilité."
            }
            _ => {
                "Connexion HTTPS impossible ou réponse invalide. Vérifiez le serveur et son certificat."
            }
        };
        Self {
            message: message.into(),
            status,
            code,
        }
    }
}
fn invalid(message: &str) -> UiError {
    UiError {
        message: message.into(),
        status: None,
        code: "CLIENT_INVALID".into(),
    }
}

fn library_update_body(
    video_root: Option<String>,
    additional_video_roots: Option<Vec<String>>,
    expected_revision: u64,
) -> Value {
    let mut body = json!({"video_root":video_root,"expected_revision":expected_revision});
    // Old servers reject unknown request fields: absence must stay absence,
    // not an explicitly serialized null, until the user edits extra locations.
    if let Some(roots) = additional_video_roots {
        body["additional_video_roots"] = json!(roots);
    }
    body
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    servers: BTreeMap<String, String>,
    last_profiles: BTreeMap<String, String>,
    last_address: Option<String>,
    theme: String,
    language: UiLanguage,
    trust: TrustStore,
    device_id: Option<String>,
    appearance: crate::appearance::Appearance,
    playback_cache: crate::playback_cache::PlaybackCache,
    player_presentation: PlayerPresentation,
}

/// Device-local interface language; never changes media, audio or API fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UiLanguage {
    #[default]
    Auto,
    En,
    Fr,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct PlayerPresentation {
    fullscreen_on_start: bool,
}
impl Default for PlayerPresentation {
    fn default() -> Self {
        Self {
            fullscreen_on_start: true,
        }
    }
}

/// Native-host-only snapshot. Never serialized into the webview.
pub struct PlaybackConnection {
    pub transport: Transport,
    pub profile_id: String,
    pub server_id: String,
    pub device_id: String,
    pub cache: crate::playback_cache::PlaybackCache,
}
struct Pending {
    address: String,
    certificate: PresentedCertificate,
    known_id: Option<String>,
    token: String,
    expires: Instant,
}
struct Connected {
    address: String,
    id: String,
    transport: Transport,
    profiles: Value,
    active: Option<String>,
    notice: Option<String>,
}
pub struct UiClient {
    directory: PathBuf,
    settings: Settings,
    trust: Arc<RwLock<TrustStore>>,
    runtime: tokio::runtime::Runtime,
    pending: Option<Pending>,
    connected: Option<Connected>,
    persist_requests: bool,
    catalog_cache: Option<crate::catalog_cache::LocalCatalog>,
    artwork_gate: Arc<tokio::sync::Semaphore>,
    prefetch: Option<tokio::task::JoinHandle<()>>,
    background_markers: BTreeMap<String, String>,
}
/// A closed catalogue transfer. It is intentionally independent of UiClient so
/// the desktop host can await it without holding its UI mutex or single-flight
/// slot.
pub struct CatalogSyncTask {
    server: String,
    profile: String,
    base_revision: Option<String>,
    mutation_generation: u64,
    mutation_guard: Arc<AtomicU64>,
    publish_gate: Arc<std::sync::Mutex<()>>,
    task: Option<crate::catalog_cache::SyncTask>,
}
pub struct CatalogSyncResult(crate::catalog_cache::SyncResult);
/// Closed local SQLite publication. It owns no UiClient borrow and is run by
/// the desktop host on a blocking worker after HTTPS has completed.
pub struct CatalogPublishTask {
    task: CatalogSyncTask,
    // Consumed by the detached worker. Completion only needs task metadata,
    // not a second full catalogue retained alongside the SQLite publication.
    result: Option<CatalogSyncResult>,
    directory: PathBuf,
    artwork_gate: Arc<tokio::sync::Semaphore>,
}
fn sync_commit_stale(
    current: Option<&str>,
    base: Option<&str>,
    generation: u64,
    prepared: u64,
) -> bool {
    current != base || generation != prepared
}
/// Closed allowlisted TMDB work. It owns only a pinned transport and a fixed
/// path/body prepared from a typed UI command.
pub struct ProviderTask {
    server: String,
    profile: String,
    transport: Transport,
    method: HttpMethod,
    path: String,
    body: Option<Value>,
    query: Option<String>,
    mutation: bool,
}
impl ProviderTask {
    pub async fn fetch(self) -> Result<(Self, Value), UiError> {
        let query = self
            .query
            .as_ref()
            .map(|value| vec![("query", value.clone())])
            .unwrap_or_default();
        let value = self
            .transport
            .json_with_options(
                self.method.clone(),
                &self.path,
                self.body.as_ref(),
                &query,
                &[],
            )
            .await?;
        Ok((self, value))
    }
}
impl CatalogSyncTask {
    pub async fn fetch(mut self) -> Result<(Self, CatalogSyncResult), UiError> {
        // Keep the context fingerprint alongside the result for a CAS commit.
        let server = self.server.clone();
        let profile = self.profile.clone();
        let base_revision = self.base_revision.clone();
        let result = self
            .task
            .take()
            .ok_or_else(|| invalid("Synchronisation déjà consommée."))?
            .fetch()
            .await?;
        Ok((
            Self {
                server,
                profile,
                base_revision,
                mutation_generation: self.mutation_generation,
                mutation_guard: self.mutation_guard,
                publish_gate: self.publish_gate,
                task: None,
            },
            CatalogSyncResult(result),
        ))
    }
    pub fn server(&self) -> &str {
        &self.server
    }
    pub fn profile(&self) -> &str {
        &self.profile
    }
    pub fn base_revision(&self) -> Option<&str> {
        self.base_revision.as_deref()
    }
}
impl CatalogPublishTask {
    pub fn publish_owned(
        mut self,
        context_epoch: Arc<AtomicU64>,
        expected_epoch: u64,
    ) -> Result<(Self, crate::catalog_cache::CatalogPublication), UiError> {
        let result = self
            .result
            .take()
            .ok_or_else(|| invalid("Publication déjà consommée."))?
            .0;
        // The overwhelmingly common revision-only check must not reopen a
        // database, walk cached images, or rewrite a mirror.
        if !result.changed {
            let revision = result.revision;
            return Ok((self, Some((Value::Null, false, revision))));
        }
        let published = crate::catalog_cache::LocalCatalog::publish_sync_detached(
            self.directory.clone(),
            self.task.server.clone(),
            self.task.profile.clone(),
            self.artwork_gate.clone(),
            self.task.mutation_guard.clone(),
            self.task.publish_gate.clone(),
            self.task.base_revision.clone(),
            self.task.mutation_generation,
            context_epoch,
            expected_epoch,
            result,
        )?;
        Ok((self, published))
    }
}
impl UiClient {
    fn open_catalog_cache(&mut self) -> Result<(), UiError> {
        let profile = self.profile()?;
        let connection = self
            .connected
            .as_ref()
            .ok_or_else(|| invalid("Connectez un serveur."))?;
        let server = connection.id.clone();
        if self
            .catalog_cache
            .as_ref()
            .is_none_or(|c| c.server != server || c.profile != profile)
        {
            if let Some(task) = self.prefetch.take() {
                task.abort();
            }
            self.catalog_cache = Some(crate::catalog_cache::LocalCatalog::open(
                &self.directory,
                &server,
                &profile,
                self.artwork_gate.clone(),
            )?);
        }
        Ok(())
    }
    /// Prepare only immutable transport state. The caller must invoke `fetch`
    /// outside this client's Mutex, then use `commit_catalog_sync`.
    pub fn prepare_catalog_sync(&mut self) -> Result<CatalogSyncTask, UiError> {
        self.open_catalog_cache()?;
        let connection = self
            .connected
            .as_ref()
            .ok_or_else(|| invalid("Connectez un serveur."))?;
        let cache = self.catalog_cache.as_ref().unwrap();
        Ok(CatalogSyncTask {
            server: connection.id.clone(),
            profile: cache.profile.clone(),
            base_revision: cache
                .document
                .as_ref()
                .and_then(|d| d["revision"].as_str())
                .map(str::to_owned),
            mutation_generation: cache.mutation_generation,
            mutation_guard: cache.mutation_guard.clone(),
            publish_gate: cache.publish_gate.clone(),
            task: Some(cache.sync_task(connection.transport.clone())),
        })
    }
    pub fn prepare_provider_task(&mut self, command: UiCommand) -> Result<ProviderTask, UiError> {
        let profile = self.profile()?;
        let connection = self
            .connected
            .as_ref()
            .ok_or_else(|| invalid("Connectez un serveur."))?;
        let (method, path, body, query, mutation) = match command {
            UiCommand::TmdbCandidates { media_id, query } => {
                valid_id(&media_id)?;
                let query = query.unwrap_or_default();
                valid_tmdb_query(&query)?;
                (
                    HttpMethod::GET,
                    format!("/api/v1/admin/profiles/{profile}/media/{media_id}/tmdb/candidates"),
                    None,
                    Some(query),
                    false,
                )
            }
            UiCommand::TmdbSelect {
                media_id,
                expected_revision,
                tmdb_id,
                kind,
                scope,
            } => {
                valid_id(&media_id)?;
                if tmdb_id <= 0
                    || !["movie", "tv"].contains(&kind.as_str())
                    || !["media", "series", "season"].contains(&scope.as_str())
                {
                    return Err(invalid("Choix TMDB invalide."));
                }
                (
                    HttpMethod::POST,
                    format!("/api/v1/admin/profiles/{profile}/media/{media_id}/tmdb"),
                    Some(
                        json!({"expected_revision":expected_revision,"tmdb_id":tmdb_id,"kind":kind,"scope":scope}),
                    ),
                    None,
                    true,
                )
            }
            UiCommand::TmdbRefresh {
                media_id,
                expected_revision,
                scope,
            } => {
                valid_id(&media_id)?;
                if !["media", "series", "season"].contains(&scope.as_str()) {
                    return Err(invalid("Portée TMDB invalide."));
                }
                (
                    HttpMethod::POST,
                    format!("/api/v1/admin/profiles/{profile}/media/{media_id}/tmdb/refresh"),
                    Some(json!({"expected_revision":expected_revision,"scope":scope})),
                    None,
                    true,
                )
            }
            _ => return Err(invalid("Opération fournisseur invalide.")),
        };
        Ok(ProviderTask {
            server: connection.id.clone(),
            profile,
            transport: connection.transport.clone(),
            method,
            path,
            body,
            query,
            mutation,
        })
    }
    pub fn commit_provider_task(
        &mut self,
        task: ProviderTask,
        value: Value,
    ) -> Result<Value, UiError> {
        let connection = self
            .connected
            .as_ref()
            .ok_or_else(|| invalid("Connectez un serveur."))?;
        if connection.id != task.server
            || connection.active.as_deref() != Some(task.profile.as_str())
        {
            return Ok(json!({"discarded":true}));
        }
        if task.mutation
            && let Some(cache) = &mut self.catalog_cache
        {
            cache.mark_dirty();
        }
        Ok(value)
    }
    /// CAS commit: a profile/server switch or a local mutation since prepare
    /// discards the fetched result. In particular, stale transport never clears
    /// a dirty marker created by a personal write.
    pub fn prepare_catalog_publish(
        &mut self,
        task: CatalogSyncTask,
        result: CatalogSyncResult,
    ) -> Result<CatalogPublishTask, UiError> {
        let connection = self
            .connected
            .as_ref()
            .ok_or_else(|| invalid("Connectez un serveur."))?;
        if connection.id != task.server
            || connection.active.as_deref() != Some(task.profile.as_str())
        {
            return Err(invalid("Synchronisation obsolète."));
        }
        self.open_catalog_cache()?;
        let cache = self.catalog_cache.as_mut().unwrap();
        let current = cache.document.as_ref().and_then(|d| d["revision"].as_str());
        if sync_commit_stale(
            current,
            task.base_revision.as_deref(),
            cache.mutation_generation,
            task.mutation_generation,
        ) {
            return Err(invalid("Synchronisation obsolète."));
        }
        Ok(CatalogPublishTask {
            task,
            result: Some(result),
            directory: self.directory.clone(),
            artwork_gate: self.artwork_gate.clone(),
        })
    }
    pub fn finish_catalog_publish(
        &mut self,
        task: CatalogPublishTask,
        published: Option<(Value, bool, String)>,
    ) -> Result<Value, UiError> {
        let connection = self
            .connected
            .as_ref()
            .ok_or_else(|| invalid("Connectez un serveur."))?;
        if connection.id != task.task.server
            || connection.active.as_deref() != Some(task.task.profile.as_str())
        {
            return Ok(json!({"changed":false,"discarded":true}));
        }
        let Some((document, changed, revision)) = published else {
            return Ok(json!({"changed":false,"discarded":true}));
        };
        let stale_mutation =
            task.task.mutation_guard.load(Ordering::Acquire) != task.task.mutation_generation;
        let Some(cache) = self
            .catalog_cache
            .as_mut()
            .filter(|cache| cache.server == task.task.server && cache.profile == task.task.profile)
        else {
            return Ok(json!({"changed":false,"discarded":true}));
        };
        // Its existing WAL connection already sees the atomic commit. Only
        // publish the small in-memory manifest; reopening here held the UI
        // mutex through a directory walk and schema/identity setup.
        if !document.is_null() {
            cache.document = Some(document);
        }
        cache.dirty = stale_mutation;
        cache.notice = None;
        Ok(json!({"changed":changed,"revision":revision,"discarded":false,"dirty":stale_mutation}))
    }
    pub fn invalidate_catalog(&mut self) {
        if let Some(cache) = &mut self.catalog_cache {
            cache.mark_dirty();
        }
    }
    pub fn upload_artwork(
        &mut self,
        path: &str,
        expected_revision: u64,
        bytes: Vec<u8>,
        mime: &str,
    ) -> Result<Value, UiError> {
        let connection = self.playback_connection()?;
        let result = self
            .runtime
            .block_on(connection.transport.put_binary_json(
                path,
                &[("expected_revision", expected_revision.to_string())],
                bytes,
                mime,
            ))
            .map_err(Into::into);
        if result.is_ok()
            && let Some(cache) = &mut self.catalog_cache
        {
            cache.mark_dirty();
        }
        result
    }
    pub fn playback_connection(&mut self) -> Result<PlaybackConnection, UiError> {
        let profile_id = self.profile()?;
        let conn = self
            .connected
            .as_ref()
            .ok_or_else(|| invalid("Connectez un serveur."))?;
        let transport = conn.transport.clone();
        let server_id = conn.id.clone();
        let device_id = match &self.settings.device_id {
            Some(id) => {
                valid_id(id)?;
                id.clone()
            }
            None => {
                let id = uuid::Uuid::new_v4().to_string();
                self.settings.device_id = Some(id.clone());
                if let Err(error) = self.save() {
                    self.settings.device_id = None;
                    return Err(error);
                }
                id
            }
        };
        Ok(PlaybackConnection {
            transport,
            profile_id,
            server_id,
            device_id,
            cache: self.settings.playback_cache,
        })
    }
    /// Short snapshot for independent settings requests, including before a
    /// profile is selected. The caller releases the UI mutex before network I/O.
    pub fn server_transport(&self) -> Result<Transport, UiError> {
        self.connected
            .as_ref()
            .map(|connection| connection.transport.clone())
            .ok_or_else(|| invalid("Connectez un serveur."))
    }
    /// Advice captures a coherent, grouped local snapshot only on explicit submit.
    /// Never download the library or call a provider while holding the UI lock.
    pub fn recommendation_catalog(
        &mut self,
        query: &chill_domain::recommendations::RecommendationQuery,
    ) -> Result<Vec<chill_domain::recommendations::Recommendation>, UiError> {
        self.open_catalog_cache()?;
        let cache = self
            .catalog_cache
            .as_ref()
            .ok_or_else(|| invalid("Catalogue local absent."))?;
        if cache.document.is_none() {
            return Err(invalid(
                "Synchronisez le catalogue avant de demander conseil.",
            ));
        }
        let items = cache
            .store
            .recommendation_catalog(&cache.profile, query)
            .map_err(|_| invalid("Conseil local indisponible."))?;
        let items: Vec<_> = items
            .into_iter()
            .filter(|i| {
                !i.media.genres.is_empty()
                    || !i.media.directors.is_empty()
                    || !i.media.cast.is_empty()
            })
            .collect();
        if items.len() > 3000 {
            return Err(invalid(
                "Le conseil accepte 3 000 films ou séries par demande. Précisez les filtres.",
            ));
        }
        Ok(items)
    }
    /// Call from the host setup thread, never from an async task (blocking HTTP runtime).
    pub fn open(directory: &Path) -> Result<Self, UiError> {
        use std::os::unix::fs::PermissionsExt;
        fs::create_dir_all(directory)
            .map_err(|_| invalid("Stockage des réglages locaux inaccessible."))?;
        if fs::symlink_metadata(directory)
            .map_err(|_| invalid("Stockage local inaccessible."))?
            .file_type()
            .is_symlink()
        {
            return Err(invalid(
                "Le stockage des réglages ne doit pas être un lien symbolique.",
            ));
        }
        fs::set_permissions(directory, fs::Permissions::from_mode(0o700))
            .map_err(|_| invalid("Protection des réglages impossible."))?;
        let settings: Settings = match fs::read(directory.join("client.json")) {
            Ok(bytes) if bytes.len() <= 4 * 1024 * 1024 => {
                serde_json::from_slice(&bytes).map_err(|_| invalid("Réglages locaux invalides."))?
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Settings::default(),
            _ => return Err(invalid("Réglages locaux inaccessibles ou trop volumineux.")),
        };
        settings.playback_cache.validate().map_err(invalid)?;
        let trust = Arc::new(RwLock::new(settings.trust.clone()));
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .map_err(|_| invalid("Transport indisponible."))?;
        Ok(Self {
            directory: directory.into(),
            settings,
            trust,
            runtime,
            pending: None,
            connected: None,
            persist_requests: true,
            catalog_cache: None,
            artwork_gate: Arc::new(tokio::sync::Semaphore::new(1)),
            prefetch: None,
            background_markers: BTreeMap::new(),
        })
    }
    /// Runs immediately after the confirmed DELETE, before any fallible GET.
    fn finish_profile_deletion(&mut self, profile: &str) -> Result<(), UiError> {
        let conn = self
            .connected
            .as_mut()
            .ok_or_else(|| invalid("Connectez un serveur."))?;
        let server = conn.id.clone();
        if conn.active.as_deref() == Some(profile) {
            conn.active = None;
        }
        if let Some(profiles) = conn.profiles.as_array_mut() {
            profiles.retain(|p| p["id"].as_str() != Some(profile));
        }
        if self
            .catalog_cache
            .as_ref()
            .is_some_and(|c| c.server == server && c.profile == profile)
        {
            if let Some(task) = self.prefetch.take() {
                task.abort();
            }
            self.catalog_cache = None;
        }
        if self.settings.last_profiles.get(&server).map(String::as_str) == Some(profile) {
            self.settings.last_profiles.remove(&server);
        }
        let cleanup = crate::catalog_cache::purge_profile(&self.directory, &server, profile);
        let saved = self.save();
        cleanup?;
        saved
    }
    fn save(&mut self) -> Result<(), UiError> {
        self.save_with_directory_sync(|path| fs::File::open(path).and_then(|f| f.sync_all()))
    }
    fn save_with_directory_sync(
        &mut self,
        sync: impl FnOnce(&std::path::Path) -> std::io::Result<()>,
    ) -> Result<(), UiError> {
        use std::os::unix::fs::OpenOptionsExt;
        self.settings.trust = self
            .trust
            .read()
            .map_err(|_| invalid("Réglages indisponibles."))?
            .clone();
        let temporary = self
            .directory
            .join(format!(".client-{}.tmp", uuid::Uuid::new_v4()));
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|_| invalid("Enregistrement local impossible."))?;
        file.write_all(
            &serde_json::to_vec(&self.settings).map_err(|_| invalid("Réglages invalides."))?,
        )
        .and_then(|_| file.sync_all())
        .map_err(|_| invalid("Enregistrement local impossible."))?;
        fs::rename(temporary, self.directory.join("client.json"))
            .map_err(|_| invalid("Enregistrement local impossible."))?;
        sync(&self.directory).map_err(|_|UiError{message:"Réglages publiés, mais durabilité non confirmée. Rouvrez l’application pour vérifier leur état avant de réessayer.".into(),status:None,code:"LOCAL_DURABILITY_UNKNOWN".into()})?;
        Ok(())
    }
    fn request(
        &mut self,
        method: HttpMethod,
        path: &str,
        body: Option<&Value>,
        query: &[(&str, String)],
        headers: &[(&str, String)],
    ) -> Result<Value, UiError> {
        let transport = &self
            .connected
            .as_ref()
            .ok_or_else(|| invalid("Connectez un serveur."))?
            .transport;
        let mutation = method != HttpMethod::GET;
        let result: Value = self
            .runtime
            .block_on(transport.json_with_options(method, path, body, query, headers))?;
        if mutation && let Some(cache) = &mut self.catalog_cache {
            cache.mark_dirty();
        }
        if !mutation
            && (path == "/api/v1/admin/scan" || path.starts_with("/api/v1/admin/tmdb/jobs/"))
        {
            let terminal = result.get("running") == Some(&Value::Bool(false))
                || result["status"]
                    .as_str()
                    .is_some_and(|s| matches!(s, "completed" | "failed" | "cancelled"));
            let marker = serde_json::to_string(&result).unwrap_or_default();
            let key = format!("{}:{path}", self.connected.as_ref().unwrap().id);
            if self.background_markers.len() > 256 {
                self.background_markers.clear();
            }
            let changed = self
                .background_markers
                .insert(key, marker.clone())
                .as_deref()
                != Some(&marker);
            if terminal
                && changed
                && let Some(cache) = &mut self.catalog_cache
            {
                cache.mark_dirty();
            }
        }
        if self.persist_requests
            && *self
                .trust
                .read()
                .map_err(|_| invalid("Confiance indisponible."))?
                != self.settings.trust
        {
            self.save()?;
        }
        Ok(result)
    }
    fn profile(&self) -> Result<String, UiError> {
        self.connected
            .as_ref()
            .and_then(|c| c.active.clone())
            .ok_or_else(|| invalid("Choisissez un profil."))
    }
    fn view(&mut self) -> Result<Value, UiError> {
        if self.connected.is_none() {
            return Ok(
                json!({"stage":"server","address":self.settings.last_address,"servers":self.settings.servers.keys().collect::<Vec<_>>(),"language":self.settings.language,"theme":self.theme(),"appearance":self.settings.appearance,"playback_cache":self.settings.playback_cache,"player_presentation":self.settings.player_presentation}),
            );
        }
        let profiles = self.request(HttpMethod::GET, "/api/v1/profiles", None, &[], &[])?;
        let conn = self.connected.as_mut().unwrap();
        if conn
            .active
            .as_ref()
            .is_some_and(|id| !has_profile(&profiles, id))
        {
            conn.active = None;
        }
        conn.profiles = profiles;
        let (address, id, active, profiles) = (
            conn.address.clone(),
            conn.id.clone(),
            conn.active.clone(),
            conn.profiles.clone(),
        );
        if let Some(list) = profiles.as_array() {
            let keep = list
                .iter()
                .filter_map(|p| p["id"].as_str())
                .filter(|id| uuid::Uuid::parse_str(id).is_ok())
                .map(str::to_owned)
                .collect::<std::collections::HashSet<_>>();
            // Never treat a malformed response as authority to erase caches.
            if keep.len() == list.len() {
                if self
                    .catalog_cache
                    .as_ref()
                    .is_some_and(|c| c.server == id && !keep.contains(&c.profile))
                {
                    if let Some(task) = self.prefetch.take() {
                        task.abort();
                    }
                    self.catalog_cache = None;
                }
                if let Err(error) = crate::catalog_cache::purge_orphans(&self.directory, &id, &keep)
                {
                    let conn = self.connected.as_mut().unwrap();
                    conn.notice = Some(match &conn.notice {
                        Some(old) => format!("{old}\n{}", error.message),
                        None => error.message,
                    });
                }
            }
        }
        let library = self.request(HttpMethod::GET, "/api/v1/admin/library", None, &[], &[])?;
        let preferences = match &active {
            Some(profile) => self.request(
                HttpMethod::GET,
                &format!("/api/v1/profiles/{profile}/preferences"),
                None,
                &[],
                &[],
            )?,
            None => Value::Null,
        };
        if active.is_some() {
            self.ensure_catalog(true)?;
        }
        let cache_status=self.catalog_cache.as_ref().filter(|c|Some(&c.profile)==active.as_ref()).map(|c|json!({"revision":c.document.as_ref().map(|d|&d["revision"]),"offline":c.notice.is_some(),"notice":c.notice}));
        Ok(
            json!({"stage":"connected","address":address,"server_id":id,"profile_id":active,"profiles":profiles,"library":library,"preferences":preferences,"catalog_cache":cache_status,"language":self.settings.language,"theme":self.theme(),"appearance":self.settings.appearance,"playback_cache":self.settings.playback_cache,"player_presentation":self.settings.player_presentation,"notice":self.connected.as_ref().unwrap().notice}),
        )
    }
    fn theme(&self) -> &str {
        if ["light", "dark"].contains(&self.settings.theme.as_str()) {
            &self.settings.theme
        } else {
            "system"
        }
    }
    fn connect(&mut self, address: &str) -> Result<Value, UiError> {
        if let Some(task) = self.prefetch.take() {
            task.abort();
        }
        self.catalog_cache = None;
        self.connected = None;
        self.pending = None;
        let address = normalize_address(address)?;
        let certificate = inspect_certificate(&address, Duration::from_secs(5))?;
        let known_id = self.settings.servers.get(&address).cloned();
        let known = known_id
            .as_ref()
            .and_then(|id| self.trust.read().ok()?.servers.get(id).cloned());
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let same_key = known
            .as_ref()
            .is_some_and(|k| fingerprint(&k.spki_sha256) == certificate.fingerprint_hex());
        if known
            .as_ref()
            .is_some_and(|k| same_key || k.certificate_not_after_unix <= now)
        {
            return self.finish_connect(address, known_id.unwrap());
        }
        let token = uuid::Uuid::new_v4().to_string();
        let result = json!({"stage":"trust","address":address,"fingerprint":certificate.fingerprint_hex(),"expires_at":certificate.not_after_unix(),"replacement":known.is_some(),"token":token,"language":self.settings.language,"theme":self.theme()});
        self.pending = Some(Pending {
            address,
            certificate,
            known_id,
            token,
            expires: Instant::now() + Duration::from_secs(120),
        });
        Ok(result)
    }
    fn finish_connect(
        &mut self,
        address: String,
        provisional_id: String,
    ) -> Result<Value, UiError> {
        let previous = self.settings.clone();
        let trust = self
            .trust
            .read()
            .map_err(|_| invalid("Confiance indisponible."))?
            .clone();
        self.persist_requests = false;
        let result = self.finish_connect_inner(address, provisional_id);
        self.persist_requests = true;
        if result.is_err() {
            self.connected = None;
            self.settings = previous;
            *self
                .trust
                .write()
                .map_err(|_| invalid("Confiance indisponible."))? = trust;
        }
        result
    }
    fn finish_connect_inner(
        &mut self,
        address: String,
        provisional_id: String,
    ) -> Result<Value, UiError> {
        let events_before = self
            .trust
            .read()
            .map_err(|_| invalid("Confiance indisponible."))?
            .events
            .len();
        let endpoint = ServerEndpoint::parse(&provisional_id, &address)?;
        let transport = Transport::new(endpoint, self.trust.clone(), None)?;
        // Transport lives outside block_on so its blocking runtime never drops in an async context.
        let identity: Value = self
            .runtime
            .block_on(transport.get_json("/api/v1/server"))?;
        if identity["api_version"] != crate::API_VERSION && identity["api_version"] != 1 {
            return Err(invalid("Version serveur incompatible."));
        }
        let id = identity["server_id"]
            .as_str()
            .filter(|id| uuid::Uuid::parse_str(id).is_ok())
            .ok_or_else(|| invalid("Identité serveur invalide."))?
            .to_owned();
        if self
            .settings
            .servers
            .get(&address)
            .is_some_and(|known| known != &id)
        {
            return Err(invalid(
                "L'identité du serveur enregistré a changé ; utilisez une nouvelle adresse ou restaurez son identité.",
            ));
        }
        if id != provisional_id {
            let mut trust = self
                .trust
                .write()
                .map_err(|_| invalid("Confiance indisponible."))?;
            let mut entry = trust
                .servers
                .remove(&provisional_id)
                .ok_or_else(|| invalid("Confiance absente."))?;
            // Never replace another registered identity's key implicitly.
            if trust
                .servers
                .get(&id)
                .is_some_and(|old| old.spki_sha256 != entry.spki_sha256)
            {
                return Err(invalid(
                    "Clé différente pour une identité déjà enregistrée.",
                ));
            }
            entry.server_id = id.clone();
            if let Some(old) = trust.servers.get(&id) {
                entry.aliases.extend(old.aliases.clone());
            }
            trust.servers.insert(id.clone(), entry);
        }
        drop(transport);
        let transport = Transport::new(
            ServerEndpoint::parse(&id, &address)?,
            self.trust.clone(),
            None,
        )?;
        let profiles: Value = self
            .runtime
            .block_on(transport.get_json("/api/v1/profiles"))?;
        if !profiles.is_array() {
            return Err(invalid("Liste des profils invalide."));
        }
        let active = self
            .settings
            .last_profiles
            .get(&id)
            .filter(|p| has_profile(&profiles, p))
            .cloned();
        self.settings.servers.insert(address.clone(), id.clone());
        self.settings.last_address = Some(address.clone());
        self.connected = Some(Connected {
            address,
            id,
            transport,
            profiles,
            active,
            notice: renewal_notice(
                &*self
                    .trust
                    .read()
                    .map_err(|_| invalid("Confiance indisponible."))?,
                events_before,
            ),
        });
        let view = self.view()?;
        self.save()?;
        Ok(view)
    }
    fn commit_selected_view(
        &mut self,
        server_id: String,
        previous: Option<String>,
        view: Result<Value, UiError>,
    ) -> Result<Value, UiError> {
        let view = match view {
            Ok(view) => view,
            Err(error) => {
                self.connected.as_mut().unwrap().active = previous;
                return Err(error);
            }
        };
        let old_choice = self.settings.last_profiles.get(&server_id).cloned();
        if let Some(active) = self.connected.as_ref().unwrap().active.clone() {
            self.settings
                .last_profiles
                .insert(server_id.clone(), active);
        } else {
            self.settings.last_profiles.remove(&server_id);
        }
        if let Err(error) = self.save() {
            self.connected.as_mut().unwrap().active = previous;
            match old_choice {
                Some(choice) => {
                    self.settings.last_profiles.insert(server_id, choice);
                }
                None => {
                    self.settings.last_profiles.remove(&server_id);
                }
            }
            return Err(error);
        }
        Ok(view)
    }
    fn ensure_catalog(&mut self, _check_server: bool) -> Result<(), UiError> {
        let connection = self
            .connected
            .as_ref()
            .ok_or_else(|| invalid("Connectez un serveur."))?;
        let transport = connection.transport.clone();
        self.open_catalog_cache()?;
        let cache = self.catalog_cache.as_mut().unwrap();
        // A warm mirror is always navigable locally. Refresh/focus no longer
        // performs HTTP under the UiClient mutex; SyncCatalog owns that work.
        if cache.document.is_none() {
            match self.runtime.block_on(cache.download(&transport)) {
                Ok(Some((items, document))) => {
                    cache.commit(&items, document)?;
                    if let Some(task) = self.prefetch.take() {
                        task.abort();
                    }
                    let tasks = cache.prefetch(transport);
                    self.prefetch = Some(self.runtime.spawn(async move {
                        for task in tasks {
                            let _ = task.fetch().await;
                            tokio::task::yield_now().await;
                        }
                    }));
                }
                Ok(None) => {
                    cache.dirty = false;
                    cache.notice = None;
                    if self.prefetch.as_ref().is_none_or(|t| t.is_finished()) {
                        let tasks = cache.prefetch(transport);
                        self.prefetch = Some(self.runtime.spawn(async move {
                            for task in tasks {
                                let _ = task.fetch().await;
                                tokio::task::yield_now().await;
                            }
                        }));
                    }
                }
                Err(error) if cache.document.is_some() => {
                    // A failed refresh never replaces the last complete mirror.
                    // Stale is not retry-now: navigation must remain local.
                    // Boot/focus/manual Refresh explicitly retries the server.
                    cache.dirty = false;
                    cache.notice=Some("Synchronisation indisponible : dernière copie locale affichée. Les modifications nécessitent le serveur.".into());
                    let _ = error;
                }
                Err(error) if error.status == Some(404) => {
                    return Err(UiError {
                        message:
                            "Le serveur doit être mis à jour pour synchroniser le catalogue local."
                                .into(),
                        status: Some(404),
                        code: "SERVER_UPDATE_REQUIRED".into(),
                    });
                }
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
    /// Prepare a closed artwork request under the context lock, then fetch it
    /// outside the navigation/player single-flight gate in the native host.
    pub fn artwork_task(
        &mut self,
        kind: &str,
        id: &str,
    ) -> Result<crate::catalog_cache::ArtworkTask, UiError> {
        match kind {
            "media" | "playlists" => valid_id(id)?,
            "sagas" => valid_collection_id(id)?,
            "season" => {
                let (tv, season) = id
                    .split_once(':')
                    .ok_or_else(|| invalid("Saison invalide."))?;
                if tv.parse::<i64>().ok().is_none_or(|v| v <= 0)
                    || season.parse::<i32>().ok().is_none_or(|v| v < 0)
                {
                    return Err(invalid("Saison invalide."));
                }
            }
            _ => return Err(invalid("Type d’affiche invalide.")),
        }
        self.ensure_catalog(false)?;
        let transport = self.connected.as_ref().unwrap().transport.clone();
        self.catalog_cache
            .as_mut()
            .unwrap()
            .artwork(transport, kind, id)
    }
    fn cached_command(&mut self, command: &UiCommand) -> Result<Option<Value>, UiError> {
        use UiCommand::*;
        if let SeasonArtwork { tv_id, season } = command {
            let task = self.artwork_task("season", &format!("{tv_id}:{season}"))?;
            return self.runtime.block_on(task.fetch()).map(Some);
        }
        if !matches!(
            command,
            Catalog { .. }
                | MediaDetail { .. }
                | RelatedMedia { .. }
                | Series { .. }
                | Sagas
                | Saga { .. }
                | Playlists
                | Playlist { .. }
                | Artwork { .. }
                | SagaArtwork { .. }
                | PlaylistArtwork { .. }
        ) {
            return Ok(None);
        }
        let artwork = match command {
            Artwork { media_id } => Some(("media", media_id)),
            SagaArtwork { id } => Some(("sagas", id)),
            PlaylistArtwork { id } => Some(("playlists", id)),
            _ => None,
        };
        if let Some((kind, id)) = artwork {
            let task = self.artwork_task(kind, id)?;
            return self.runtime.block_on(task.fetch()).map(Some);
        }
        self.ensure_catalog(false)?;
        let cache = self.catalog_cache.as_mut().unwrap();
        let result = match command {
            RelatedMedia { media_id } => {
                valid_id(media_id)?;
                json!({"items":cache.store.recommendations(&cache.profile,media_id).map_err(|_|invalid("Suggestions locales indisponibles."))?})
            }
            Catalog {
                offset,
                category,
                q,
                genre,
                year,
                watched,
                sort,
                grouped,
                available,
            } => {
                catalog_query(
                    *offset,
                    category.clone(),
                    q.clone(),
                    genre.clone(),
                    *year,
                    *watched,
                    sort.clone(),
                    *grouped,
                    *available,
                )?;
                cache.browse(&chill_domain::catalog_browser::BrowserQuery {
                    offset: *offset,
                    limit: 48,
                    category: category.clone().filter(|s| !s.is_empty()),
                    q: q.clone(),
                    genre: genre.clone(),
                    year: year.map(i32::from),
                    watched: *watched,
                    sort: sort.clone(),
                    grouped: grouped.unwrap_or(false),
                    available: *available,
                })?
            }
            MediaDetail { media_id } => {
                valid_id(media_id)?;
                let media = cache
                    .store
                    .media(&cache.profile, media_id)
                    .map_err(|_| invalid("Vidéo introuvable."))?;
                let revision = cache
                    .store
                    .media_revision(media_id)
                    .map_err(|_| invalid("Révision inconnue."))?;
                json!({"media":media,"revision":revision})
            }
            Series { name } => {
                valid_series_name(name)?;
                let members = cache
                    .store
                    .series(&cache.profile, name)
                    .map_err(|_| invalid("Série introuvable."))?;
                let watched = members.iter().filter(|m| m.watched).count();
                let total = members.len();
                let revision = cache
                    .document
                    .as_ref()
                    .map(|d| &d["series_revisions"][name]);
                let tv_id = members.first().and_then(|m| m.tmdb_id);
                let mut seasons = serde_json::Map::new();
                if let Some(tv_id) = tv_id {
                    let prefix = format!("tmdb:tv:{tv_id}:season:");
                    if let Some(values) = cache
                        .document
                        .as_ref()
                        .and_then(|document| document["season_metadata"].as_object())
                    {
                        for (key, value) in values {
                            if let Some(number) = key.strip_prefix(&prefix) {
                                seasons.insert(number.into(), value.clone());
                            }
                        }
                    }
                }
                json!({"name":name,"revision":revision,"metadata":members.first().and_then(|m|m.series_metadata.clone()),"tmdb_id":tv_id,"season_metadata":seasons,"episodes":members,"counters":{"total":total,"watched":watched,"unwatched":total-watched}})
            }
            Sagas => cache.collection("sagas", None)?,
            Playlists => cache.collection("playlists", None)?,
            Saga { id } => {
                valid_collection_id(id)?;
                cache.collection("sagas", Some(id))?
            }
            Playlist { id } => {
                valid_id(id)?;
                cache.collection("playlists", Some(id))?
            }
            _ => unreachable!(),
        };
        Ok(Some(result))
    }
    pub fn dispatch(&mut self, command: UiCommand) -> Result<Value, UiError> {
        if let Some(value) = self.cached_command(&command)? {
            return Ok(value);
        }
        match command {
            UiCommand::PlaylistArtwork { id } => {
                let task = self.artwork_task("playlists", &id)?;
                self.runtime.block_on(task.fetch())
            }
            UiCommand::TmdbStatus => {
                self.request(HttpMethod::GET, "/api/v1/admin/tmdb", None, &[], &[])
            }
            UiCommand::TmdbConfigure {
                credential_type,
                credential,
                auto_enrich,
            } => {
                if !["api_key", "bearer"].contains(&credential_type.as_str())
                    || credential.0.is_empty()
                    || credential.0.len() > 4096
                    || credential.0.chars().any(char::is_control)
                {
                    return Err(invalid("Clé ou jeton TMDB invalide."));
                }
                let profile_id = if auto_enrich {
                    Some(self.profile()?)
                } else {
                    None
                };
                self.request(HttpMethod::PUT, "/api/v1/admin/tmdb", Some(&json!({"credential_type":credential_type,"credential":credential.0,"auto_enrich":auto_enrich,"profile_id":profile_id})), &[], &[])
            }
            UiCommand::TmdbClear => {
                self.request(HttpMethod::DELETE, "/api/v1/admin/tmdb", None, &[], &[])
            }
            UiCommand::TmdbJobStart => {
                let profile = self.profile()?;
                self.request(
                    HttpMethod::POST,
                    "/api/v1/admin/tmdb/jobs",
                    Some(&json!({"profile_id":profile})),
                    &[],
                    &[],
                )
            }
            UiCommand::TmdbJobStatus { id } => {
                valid_id(&id)?;
                self.request(
                    HttpMethod::GET,
                    &format!("/api/v1/admin/tmdb/jobs/{id}"),
                    None,
                    &[],
                    &[],
                )
            }
            UiCommand::TmdbJobCancel { id } => {
                valid_id(&id)?;
                self.request(
                    HttpMethod::POST,
                    &format!("/api/v1/admin/tmdb/jobs/{id}/cancel"),
                    Some(&json!({})),
                    &[],
                    &[],
                )
            }
            UiCommand::TmdbJobRetry { id } => {
                valid_id(&id)?;
                self.request(
                    HttpMethod::POST,
                    &format!("/api/v1/admin/tmdb/jobs/{id}/retry"),
                    Some(&json!({})),
                    &[],
                    &[],
                )
            }
            UiCommand::TmdbCandidates { media_id, query } => {
                valid_id(&media_id)?;
                let query = query.unwrap_or_default();
                valid_tmdb_query(&query)?;
                let p = self.profile()?;
                self.request(
                    HttpMethod::GET,
                    &format!("/api/v1/admin/profiles/{p}/media/{media_id}/tmdb/candidates"),
                    None,
                    &[("query", query)],
                    &[],
                )
            }
            UiCommand::TmdbSelect {
                media_id,
                expected_revision,
                tmdb_id,
                kind,
                scope,
            } => {
                valid_id(&media_id)?;
                if tmdb_id <= 0
                    || !["movie", "tv"].contains(&kind.as_str())
                    || !["media", "series", "season"].contains(&scope.as_str())
                {
                    return Err(invalid("Choix TMDB invalide."));
                }
                let p = self.profile()?;
                self.request(HttpMethod::POST,&format!("/api/v1/admin/profiles/{p}/media/{media_id}/tmdb"),Some(&json!({"expected_revision":expected_revision,"tmdb_id":tmdb_id,"kind":kind,"scope":scope})),&[],&[])
            }
            UiCommand::TmdbRefresh {
                media_id,
                expected_revision,
                scope,
            } => {
                valid_id(&media_id)?;
                if !["media", "series", "season"].contains(&scope.as_str()) {
                    return Err(invalid("Portée TMDB invalide."));
                }
                let p = self.profile()?;
                self.request(
                    HttpMethod::POST,
                    &format!("/api/v1/admin/profiles/{p}/media/{media_id}/tmdb/refresh"),
                    Some(&json!({"expected_revision":expected_revision,"scope":scope})),
                    &[],
                    &[],
                )
            }
            UiCommand::TmdbRestorePoster {
                media_id,
                expected_revision,
            } => {
                valid_id(&media_id)?;
                let p = self.profile()?;
                self.request(
                    HttpMethod::POST,
                    &format!("/api/v1/admin/profiles/{p}/media/{media_id}/tmdb/poster/restore"),
                    Some(&json!({"expected_revision":expected_revision})),
                    &[],
                    &[],
                )
            }
            UiCommand::Unavailable { offset, limit } => {
                if limit == 0 || limit > 48 {
                    return Err(invalid("Page indisponible invalide."));
                }
                self.request(
                    HttpMethod::GET,
                    "/api/v1/admin/unavailable",
                    None,
                    &[("offset", offset.to_string()), ("limit", limit.to_string())],
                    &[],
                )
            }
            UiCommand::ForgetUnavailable {
                media_ids,
                expected_epoch,
                confirmed,
            } => {
                if !confirmed || media_ids.is_empty() || media_ids.len() > 48 {
                    return Err(invalid("Confirmation ou sélection indisponible invalide."));
                }
                self.request(HttpMethod::POST,"/api/v1/admin/unavailable",Some(&json!({"media_ids":media_ids,"expected_epoch":expected_epoch,"confirmed":true})),&[],&[])
            }
            UiCommand::ReassociationProposals { offset } => {
                if offset > i64::MAX as usize {
                    return Err(invalid("Page de réassociation invalide."));
                }
                self.profile()?;
                self.request(
                    HttpMethod::GET,
                    "/api/v1/admin/reassociations",
                    None,
                    &[("offset", offset.to_string()), ("limit", "48".into())],
                    &[],
                )
            }
            UiCommand::ConfirmReassociation {
                source_id,
                target_id,
                expected_source_revision,
                expected_target_revision,
                expected_epoch,
                confirmed,
            } => {
                valid_id(&source_id)?;
                valid_id(&target_id)?;
                if !confirmed || source_id == target_id {
                    return Err(invalid("Confirmation de réassociation invalide."));
                }
                self.profile()?;
                self.request(HttpMethod::POST,"/api/v1/admin/reassociations",Some(&json!({"confirmed":true,"source_id":source_id,"target_id":target_id,"expected_source_revision":expected_source_revision,"expected_target_revision":expected_target_revision,"expected_epoch":expected_epoch})),&[],&[])
            }
            UiCommand::FingerprintStatus { media_id } => {
                valid_id(&media_id)?;
                self.profile()?;
                self.request(
                    HttpMethod::GET,
                    &format!("/api/v1/admin/media/{media_id}/fingerprint"),
                    None,
                    &[],
                    &[],
                )
            }
            UiCommand::RequestFullFingerprint {
                media_id,
                confirmed,
            } => {
                valid_id(&media_id)?;
                if !confirmed {
                    return Err(invalid("Confirmation d’empreinte complète requise."));
                }
                self.profile()?;
                self.request(
                    HttpMethod::POST,
                    &format!("/api/v1/admin/media/{media_id}/fingerprint"),
                    Some(&json!({"complete":true})),
                    &[],
                    &[],
                )
            }
            UiCommand::Artwork { media_id } => {
                valid_id(&media_id)?;
                let p = self.profile()?;
                let c = self
                    .connected
                    .as_ref()
                    .ok_or_else(|| invalid("Serveur absent."))?;
                let data = self.runtime.block_on(c.transport.get_binary(
                    &format!("/api/v1/profiles/{p}/media/{media_id}/artwork"),
                    &[],
                    8 * 1024 * 1024,
                ))?;
                let src = crate::artwork::preview_data_uri(&data)
                    .map_err(|_| invalid("Affiche non prise en charge."))?;
                Ok(json!({"src": src}))
            }
            UiCommand::SagaArtwork { id } => {
                valid_collection_id(&id)?;
                let p = self.profile()?;
                let c = self
                    .connected
                    .as_ref()
                    .ok_or_else(|| invalid("Serveur absent."))?;
                let data = self.runtime.block_on(c.transport.get_binary(
                    &format!("/api/v1/profiles/{p}/sagas/{id}/artwork"),
                    &[],
                    8 * 1024 * 1024,
                ))?;
                let src = crate::artwork::preview_data_uri(&data)
                    .map_err(|_| invalid("Affiche non prise en charge."))?;
                Ok(json!({"src": src}))
            }
            UiCommand::SagaMemberArtwork {
                id,
                expected_revision,
                media_id,
            } => {
                valid_collection_id(&id)?;
                valid_id(&media_id)?;
                let p = self.profile()?;
                self.request(
                    HttpMethod::PUT,
                    &format!("/api/v1/profiles/{p}/sagas/{id}/poster/member"),
                    Some(&json!({"expected_revision":expected_revision,"media_id":media_id})),
                    &[],
                    &[],
                )
            }
            UiCommand::Saga { id } => {
                valid_collection_id(&id)?;
                let p = self.profile()?;
                self.request(
                    HttpMethod::GET,
                    &format!("/api/v1/profiles/{p}/sagas/{id}"),
                    None,
                    &[],
                    &[],
                )
            }
            UiCommand::CreateSaga { name } => {
                let p = self.profile()?;
                self.request(
                    HttpMethod::POST,
                    &format!("/api/v1/profiles/{p}/sagas"),
                    Some(&json!({"name":name})),
                    &[],
                    &[],
                )
            }
            UiCommand::RenameSaga {
                id,
                expected_revision,
                name,
            } => {
                valid_collection_id(&id)?;
                let p = self.profile()?;
                self.request(
                    HttpMethod::PUT,
                    &format!("/api/v1/profiles/{p}/sagas/{id}"),
                    Some(&json!({"expected_revision":expected_revision,"name":name})),
                    &[],
                    &[],
                )
            }
            UiCommand::AddSaga {
                id,
                expected_revision,
                media_ids,
            } => {
                valid_collection_id(&id)?;
                validate_media_ids(&media_ids)?;
                let p = self.profile()?;
                self.request(
                    HttpMethod::POST,
                    &format!("/api/v1/profiles/{p}/sagas/{id}/members"),
                    Some(&json!({"expected_revision":expected_revision,"media_ids":media_ids})),
                    &[],
                    &[],
                )
            }
            UiCommand::RemoveSagaMember {
                id,
                expected_revision,
                media_id,
            } => {
                valid_collection_id(&id)?;
                valid_id(&media_id)?;
                let p = self.profile()?;
                self.request(
                    HttpMethod::DELETE,
                    &format!("/api/v1/profiles/{p}/sagas/{id}/members"),
                    Some(&json!({"expected_revision":expected_revision,"media_ids":[media_id]})),
                    &[],
                    &[],
                )
            }
            UiCommand::ReorderSaga {
                id,
                expected_revision,
                expected_order,
                media_ids,
            } => {
                valid_collection_id(&id)?;
                validate_media_ids(&media_ids)?;
                validate_media_ids(&expected_order)?;
                let p = self.profile()?;
                self.request(HttpMethod::PUT,&format!("/api/v1/profiles/{p}/sagas/{id}/order"),Some(&json!({"expected_revision":expected_revision,"expected_order":expected_order,"media_ids":media_ids})),&[],&[])
            }
            UiCommand::DeleteSaga {
                id,
                expected_revision,
            } => {
                valid_collection_id(&id)?;
                let p = self.profile()?;
                self.request(
                    HttpMethod::DELETE,
                    &format!("/api/v1/profiles/{p}/sagas/{id}"),
                    Some(&json!({"expected_revision":expected_revision})),
                    &[],
                    &[],
                )
            }
            UiCommand::MediaDetail { media_id } => {
                valid_id(&media_id)?;
                let p = self.profile()?;
                self.request(
                    HttpMethod::GET,
                    &format!("/api/v1/profiles/{p}/media/{media_id}"),
                    None,
                    &[],
                    &[],
                )
            }
            UiCommand::EditMedia {
                media_id,
                expected_revision,
                patch,
            } => {
                valid_id(&media_id)?;
                let allowed = [
                    "title",
                    "category",
                    "year",
                    "genres",
                    "series",
                    "season",
                    "episode",
                    "saga",
                    "overview",
                    "runtime_minutes",
                    "cast",
                    "directors",
                    "series_overview",
                ];
                let mut object = patch
                    .as_object()
                    .filter(|o| o.keys().all(|k| allowed.contains(&k.as_str())))
                    .ok_or_else(|| invalid("Champs de fiche invalides."))?
                    .clone();
                object.insert("expected_revision".into(), json!(expected_revision));
                let p = self.profile()?;
                self.request(
                    HttpMethod::PUT,
                    &format!("/api/v1/profiles/{p}/media/{media_id}"),
                    Some(&Value::Object(object)),
                    &[],
                    &[],
                )
            }
            UiCommand::Series { name } => {
                valid_series_name(&name)?;
                let p = self.profile()?;
                let segment = percent_segment(&name);
                let selector = series_selector_query(&name);
                self.request(
                    HttpMethod::GET,
                    &format!("/api/v1/profiles/{p}/series/{segment}"),
                    None,
                    &selector,
                    &[],
                )
            }
            UiCommand::EditSeries {
                series,
                expected_revision,
                patch,
            } => {
                valid_series_name(&series)?;
                let payload = series_edit_payload(expected_revision, patch)?;
                let p = self.profile()?;
                let segment = percent_segment(&series);
                let selector = series_selector_query(&series);
                self.request(
                    HttpMethod::PUT,
                    &format!("/api/v1/profiles/{p}/series/{segment}"),
                    Some(&payload),
                    &selector,
                    &[],
                )
            }
            UiCommand::RelatedMedia { media_id } => {
                valid_id(&media_id)?;
                let p = self.profile()?;
                self.request(
                    HttpMethod::GET,
                    &format!("/api/v1/profiles/{p}/media/{media_id}/related"),
                    None,
                    &[],
                    &[],
                )
            }
            UiCommand::Sagas => {
                let p = self.profile()?;
                self.request(
                    HttpMethod::GET,
                    &format!("/api/v1/profiles/{p}/sagas"),
                    None,
                    &[],
                    &[],
                )
            }
            UiCommand::Playlists => {
                let p = self.profile()?;
                self.request(
                    HttpMethod::GET,
                    &format!("/api/v1/profiles/{p}/playlists"),
                    None,
                    &[],
                    &[],
                )
            }
            UiCommand::Playlist { id } => {
                valid_id(&id)?;
                let p = self.profile()?;
                self.request(
                    HttpMethod::GET,
                    &format!("/api/v1/profiles/{p}/playlists/{id}"),
                    None,
                    &[],
                    &[],
                )
            }
            UiCommand::CreatePlaylist { name, media_id } => {
                if let Some(id) = &media_id {
                    valid_id(id)?;
                }
                let p = self.profile()?;
                self.request(
                    HttpMethod::POST,
                    &format!("/api/v1/profiles/{p}/playlists"),
                    Some(&json!({"name":name,"media_id":media_id})),
                    &[],
                    &[],
                )
            }
            UiCommand::AddPlaylist {
                id,
                expected_revision,
                media_ids,
            } => {
                valid_id(&id)?;
                validate_media_ids(&media_ids)?;
                let p = self.profile()?;
                self.request(
                    HttpMethod::POST,
                    &format!("/api/v1/profiles/{p}/playlists/{id}/members"),
                    Some(&json!({"expected_revision":expected_revision,"media_ids":media_ids})),
                    &[],
                    &[],
                )
            }
            UiCommand::RemovePlaylistMember {
                id,
                media_id,
                expected_revision,
            } => {
                valid_id(&id)?;
                valid_id(&media_id)?;
                let p = self.profile()?;
                self.request(
                    HttpMethod::DELETE,
                    &format!("/api/v1/profiles/{p}/playlists/{id}/members/{media_id}"),
                    Some(&json!({"expected_revision":expected_revision})),
                    &[],
                    &[],
                )
            }
            UiCommand::ReorderPlaylist {
                id,
                expected_revision,
                expected_order,
                media_ids,
            } => {
                valid_id(&id)?;
                validate_media_ids(&media_ids)?;
                validate_media_ids(&expected_order)?;
                let p = self.profile()?;
                self.request(HttpMethod::PUT,&format!("/api/v1/profiles/{p}/playlists/{id}/order"),Some(&json!({"expected_revision":expected_revision,"expected_order":expected_order,"media_ids":media_ids})),&[],&[])
            }
            UiCommand::DeletePlaylist { id } => {
                valid_id(&id)?;
                let p = self.profile()?;
                self.request(
                    HttpMethod::DELETE,
                    &format!("/api/v1/profiles/{p}/playlists/{id}"),
                    None,
                    &[],
                    &[],
                )
            }
            UiCommand::RenamePlaylist {
                id,
                expected_revision,
                name,
            } => {
                valid_id(&id)?;
                let p = self.profile()?;
                self.request(
                    HttpMethod::PUT,
                    &format!("/api/v1/profiles/{p}/playlists/{id}"),
                    Some(&json!({"expected_revision":expected_revision,"name":name})),
                    &[],
                    &[],
                )
            }
            UiCommand::GroupRating {
                media_id,
                rating,
                scope,
            } => {
                valid_id(&media_id)?;
                if !["video", "season", "series"].contains(&scope.as_str()) {
                    return Err(invalid("Portée de note invalide."));
                }
                let p = self.profile()?;
                self.request(
                    HttpMethod::PUT,
                    &format!("/api/v1/profiles/{p}/media/{media_id}/rating"),
                    Some(&json!({"rating":rating,"scope":scope})),
                    &[],
                    &[],
                )
            }
            UiCommand::CacheSettings => Ok(json!({"cache":self.settings.playback_cache})),
            UiCommand::SavePlayerSettings {
                fullscreen_on_start,
            } => {
                let previous = self.settings.player_presentation;
                self.settings.player_presentation.fullscreen_on_start = fullscreen_on_start;
                if let Err(error) = self.save() {
                    self.settings.player_presentation = previous;
                    return Err(error);
                }
                Ok(json!({"player_presentation":self.settings.player_presentation}))
            }
            UiCommand::SaveCacheSettings { cache } => {
                cache.validate().map_err(invalid)?;
                let previous = self.settings.playback_cache;
                self.settings.playback_cache = cache;
                if let Err(error) = self.save() {
                    self.settings.playback_cache = previous;
                    return Err(error);
                }
                Ok(json!({"cache":cache}))
            }
            UiCommand::SaveAppearance { appearance } => {
                let previous = self.settings.appearance.clone();
                self.settings.appearance = appearance;
                if let Err(e) = self.save() {
                    self.settings.appearance = previous;
                    return Err(e);
                }
                Ok(json!({"appearance":self.settings.appearance}))
            }
            UiCommand::SaveLanguage { language } => {
                let previous = self.settings.language;
                self.settings.language = language;
                if let Err(error) = self.save() {
                    self.settings.language = previous;
                    return Err(error);
                }
                Ok(json!({"language": language}))
            }
            UiCommand::ResolveAppearance { year, month, day } => Ok(
                json!({"ambiance":self.settings.appearance.resolve(year,month,day).map_err(|e|invalid(&e))?,"appearance":self.settings.appearance}),
            ),
            UiCommand::Boot => {
                if let Some(address) = self.settings.last_address.clone() {
                    return match self.connect(&address) {
                        Ok(view) => Ok(view),
                        Err(error) => {
                            self.connected = None;
                            let mut view = self.view()?;
                            view["notice"] = json!(error.message);
                            Ok(view)
                        }
                    };
                }
                self.view()
            }
            UiCommand::Connect { address } => self.connect(&address),
            UiCommand::Approve { token } => {
                let pending = self
                    .pending
                    .take()
                    .filter(|p| p.token == token && p.expires > Instant::now())
                    .ok_or_else(|| invalid("Confirmation expirée ; reconnectez le serveur."))?;
                let host = ServerEndpoint::parse("probe", &pending.address)?
                    .host()
                    .to_owned();
                let key = pending
                    .known_id
                    .unwrap_or_else(|| format!("pending:{}", pending.token));
                let previous_trust = self
                    .trust
                    .read()
                    .map_err(|_| invalid("Confiance indisponible."))?
                    .clone();
                if self
                    .trust
                    .read()
                    .map_err(|_| invalid("Confiance indisponible."))?
                    .servers
                    .contains_key(&key)
                {
                    self.trust
                        .write()
                        .map_err(|_| invalid("Confiance indisponible."))?
                        .approve_key_replacement(
                            &key,
                            &host,
                            pending.certificate,
                            SystemTime::now(),
                        )?;
                } else {
                    self.trust
                        .write()
                        .map_err(|_| invalid("Confiance indisponible."))?
                        .approve_initial(&key, &host, pending.certificate);
                }
                let result = self.finish_connect(pending.address, key);
                if result.is_err() {
                    *self
                        .trust
                        .write()
                        .map_err(|_| invalid("Confiance indisponible."))? = previous_trust;
                }
                result
            }
            UiCommand::Disconnect => {
                if let Some(task) = self.prefetch.take() {
                    task.abort();
                }
                self.catalog_cache = None;
                self.pending = None;
                self.connected = None;
                self.view()
            }
            UiCommand::Refresh => self.view(),
            UiCommand::SyncCatalog => Err(invalid("Synchronisation gérée par l’hôte.")),
            UiCommand::SeasonArtwork { .. } => Err(invalid("Affiche gérée par le cache.")),
            UiCommand::SelectProfile { id } => {
                valid_id(&id)?;
                let conn = self
                    .connected
                    .as_mut()
                    .ok_or_else(|| invalid("Connectez un serveur."))?;
                if !has_profile(&conn.profiles, &id) {
                    return Err(invalid("Profil absent ; rechargez la liste."));
                }
                let server_id = conn.id.clone();
                let previous = conn.active.replace(id.clone());
                // Fetch the prospective view before persisting its identity. Requests
                // may persist trust events, but must not remember an unrendered profile.
                let view = self.view();
                self.commit_selected_view(server_id, previous, view)
            }
            UiCommand::CreateProfile { name } => {
                self.request(
                    HttpMethod::POST,
                    "/api/v1/profiles",
                    Some(&json!({"name":name,"initial_preferences":{"audio_language":"en","subtitle_language":"en","autoplay_seconds":20}})),
                    &[],
                    &[],
                )?;
                self.view()
            }
            UiCommand::RenameProfile {
                id,
                name,
                expected_revision,
            } => {
                valid_id(&id)?;
                self.request(
                    HttpMethod::PUT,
                    &format!("/api/v1/profiles/{id}"),
                    Some(&json!({"name":name,"expected_revision":expected_revision})),
                    &[],
                    &[],
                )?;
                self.view()
            }
            UiCommand::DeleteProfile { id, confirm_id } => {
                valid_id(&id)?;
                if id != confirm_id {
                    return Err(invalid("Suppression non confirmée."));
                }
                self.request(
                    HttpMethod::DELETE,
                    &format!("/api/v1/profiles/{id}"),
                    None,
                    &[],
                    &[("x-chill-confirm-delete", id.clone())],
                )?;
                self.finish_profile_deletion(&id)?;
                self.view()
            }
            UiCommand::SaveLibrary {
                video_root,
                additional_video_roots,
                expected_revision,
            } => {
                let body =
                    library_update_body(video_root, additional_video_roots, expected_revision);
                self.request(
                    HttpMethod::PUT,
                    "/api/v1/admin/library",
                    Some(&body),
                    &[],
                    &[],
                )?;
                self.view()
            }
            UiCommand::SavePreferences {
                audio_language,
                subtitle_language,
                autoplay_seconds,
            } => {
                let profile = self.profile()?;
                self.request(HttpMethod::PUT,&format!("/api/v1/profiles/{profile}/preferences"),Some(&json!({"audio_language":audio_language,"subtitle_language":subtitle_language,"autoplay_seconds":autoplay_seconds})),&[],&[])?;
                self.view()
            }
            UiCommand::Catalog {
                offset,
                category,
                q,
                genre,
                year,
                watched,
                sort,
                grouped,
                available,
            } => {
                let profile = self.profile()?;
                let query = catalog_query(
                    offset, category, q, genre, year, watched, sort, grouped, available,
                )?;
                self.request(
                    HttpMethod::GET,
                    &format!("/api/v1/profiles/{profile}/catalog"),
                    None,
                    &query,
                    &[],
                )
            }
            UiCommand::SetWatched { media_id, watched } => {
                valid_id(&media_id)?;
                let profile = self.profile()?;
                self.request(
                    HttpMethod::PUT,
                    &format!("/api/v1/profiles/{profile}/media/{media_id}/watched"),
                    Some(&json!({"watched":watched})),
                    &[],
                    &[],
                )
            }
            UiCommand::SetRating { media_id, rating } => {
                valid_id(&media_id)?;
                let profile = self.profile()?;
                self.request(
                    HttpMethod::PUT,
                    &format!("/api/v1/profiles/{profile}/media/{media_id}/rating"),
                    Some(&json!({"rating":rating})),
                    &[],
                    &[],
                )
            }
            UiCommand::StartScan => self.request(
                HttpMethod::POST,
                "/api/v1/admin/scan",
                Some(&json!({})),
                &[],
                &[],
            ),
            UiCommand::ScanStatus => {
                self.request(HttpMethod::GET, "/api/v1/admin/scan", None, &[], &[])
            }
            UiCommand::Theme { theme } => {
                if !["system", "light", "dark"].contains(&theme.as_str()) {
                    return Err(invalid("Thème invalide."));
                }
                self.settings.theme = theme;
                self.save()?;
                self.view()
            }
        }
    }
}
fn renewal_notice(trust: &TrustStore, after: usize) -> Option<String> {
    trust.events.iter().skip(after).any(|e| e.kind == TrustEventKind::KeyReplacedAfterKnownExpiry)
        .then(|| "Le certificat précédemment approuvé était expiré : la nouvelle clé de ce serveur a été acceptée automatiquement, conformément au réglage du foyer. Vérifiez le serveur si ce changement est inattendu.".into())
}
fn fingerprint(bytes: &[u8; 32]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(":")
}
// Mirrors the closed catalogue IPC fields; no hidden mutable context.
#[allow(clippy::too_many_arguments)]
fn catalog_query(
    offset: usize,
    category: Option<String>,
    q: Option<String>,
    genre: Option<String>,
    year: Option<u16>,
    watched: Option<bool>,
    sort: Option<String>,
    grouped: Option<bool>,
    available: Option<bool>,
) -> Result<Vec<(&'static str, String)>, UiError> {
    let mut query = vec![("offset", offset.to_string()), ("limit", "48".into())];
    if let Some(category) = category.filter(|s| !s.is_empty()) {
        if !["film", "serie", "spectacle", "documentaire"].contains(&category.as_str()) {
            return Err(invalid("Catégorie invalide."));
        }
        query.push(("category", category));
    }
    if let Some(q) = q.filter(|s| !s.is_empty()) {
        query.push(("q", q));
    }
    if let Some(genre) = genre.filter(|s| !s.is_empty()) {
        if genre.chars().count() > 200 {
            return Err(invalid("Genre trop long."));
        }
        query.push(("genre", genre));
    }
    if let Some(year) = year {
        if year != 0 && !(1850..=2200).contains(&year) {
            return Err(invalid("Année invalide."));
        }
        query.push(("year", year.to_string()));
    }
    if let Some(watched) = watched {
        query.push(("watched", watched.to_string()));
    }
    if let Some(sort) = sort.filter(|s| !s.is_empty()) {
        if ![
            "title_asc",
            "title_desc",
            "year_asc",
            "year_desc",
            "added_desc",
            "rating_desc",
        ]
        .contains(&sort.as_str())
        {
            return Err(invalid("Tri invalide."));
        }
        query.push(("sort", sort));
    }
    if let Some(grouped) = grouped {
        query.push(("grouped", grouped.to_string()));
    }
    if let Some(available) = available {
        query.push(("available", available.to_string()));
    }
    Ok(query)
}
fn valid_id(id: &str) -> Result<(), UiError> {
    uuid::Uuid::parse_str(id)
        .map(|_| ())
        .map_err(|_| invalid("Identifiant invalide."))
}
fn valid_tmdb_query(query: &str) -> Result<(), UiError> {
    if query.chars().count() > 500 {
        return Err(invalid("Recherche TMDB trop longue."));
    }
    Ok(())
}
fn series_selector_query(name: &str) -> Vec<(&'static str, String)> {
    if name.starts_with("media:") || name.starts_with("tmdb:tv:") {
        vec![("selector", "key".into())]
    } else {
        vec![]
    }
}
fn valid_series_name(name: &str) -> Result<(), UiError> {
    if name.trim().is_empty() || name.len() > 500 {
        return Err(invalid("Série invalide."));
    }
    Ok(())
}
fn series_edit_payload(expected_revision: u64, patch: Value) -> Result<Value, UiError> {
    let allowed = ["name", "year", "genres", "overview", "cast", "directors"];
    let object = patch
        .as_object()
        .filter(|o| o.keys().all(|key| allowed.contains(&key.as_str())))
        .ok_or_else(|| invalid("Champs communs de série invalides."))?;
    if let Some(name) = object.get("name").and_then(Value::as_str) {
        valid_series_name(name)?;
    }
    Ok(json!({"expected_revision":expected_revision,"patch":patch}))
}
fn valid_collection_id(id: &str) -> Result<(), UiError> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err(invalid("Identifiant de saga invalide."));
    }
    Ok(())
}
fn validate_media_ids(ids: &[String]) -> Result<(), UiError> {
    if ids.len() > 3000 {
        return Err(invalid("Liste trop longue."));
    }
    for id in ids {
        valid_id(id)?;
    }
    Ok(())
}
fn percent_segment(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
fn has_profile(profiles: &Value, id: &str) -> bool {
    profiles
        .as_array()
        .is_some_and(|p| p.iter().any(|p| p["id"].as_str() == Some(id)))
}
pub fn normalize_address(input: &str) -> Result<String, UiError> {
    let raw = input.trim();
    if raw.is_empty() || raw.len() > 500 {
        return Err(invalid("Saisissez un DNS ou une IP de serveur."));
    }
    let explicit = raw.contains("://");
    let full = if explicit {
        raw.to_owned()
    } else {
        format!("https://{raw}")
    };
    let mut endpoint = ServerEndpoint::parse("probe", &full)?;
    if endpoint.base_url.path() != "/" {
        return Err(invalid("L'adresse ne doit pas contenir de chemin."));
    }
    if !explicit && endpoint.base_url.port().is_none() {
        endpoint
            .base_url
            .set_port(Some(8443))
            .map_err(|_| invalid("Port invalide."))?;
    }
    Ok(endpoint.base_url.to_string())
}
#[cfg(test)]
mod tests {
    fn publication_fixture(count: usize, overview_bytes: usize) -> super::CatalogPublishTask {
        use super::*;
        let server = "00000000-0000-4000-8000-000000000001".to_owned();
        let profile = "00000000-0000-4000-8000-000000000002".to_owned();
        let items = (0..count).map(|index| {
            serde_json::from_value(serde_json::json!({"media":{
                "id":format!("00000000-0000-4000-8000-{index:012}"),
                "path":format!("fixture/{index}.mkv"),"title":format!("Fixture {index}"),
                "category":"film","year":2024,"series":null,"season":null,"episode":null,
                "genres":["Drama"],"overview":"x".repeat(overview_bytes),"poster":null,
                "saga":null,"watched":false,"rating":null,"available":true,"added_at":1,"tmdb_id":null
            },"revision":1})).unwrap()
        }).collect();
        CatalogPublishTask {
            task: CatalogSyncTask {
                server,
                profile,
                base_revision: None,
                mutation_generation: 0,
                mutation_guard: Arc::new(AtomicU64::new(0)),
                publish_gate: Arc::new(std::sync::Mutex::new(())),
                task: None,
            },
            result: CatalogSyncResult(crate::catalog_cache::SyncResult {
                changed: true,
                reset: true,
                revision: "fixture-r1".into(),
                items,
                deleted_ids: vec![],
                document: serde_json::json!({"revision":"fixture-r1",
                    "sagas":[],"playlists":[],"series_revisions":{},"season_metadata":{}}),
            })
            .into(),
            directory: PathBuf::new(),
            artwork_gate: Arc::new(tokio::sync::Semaphore::new(1)),
        }
    }
    /// Run this exact ignored test in a fresh process under /usr/bin/time -l.
    /// The same deterministic fixture and publication path are used on both revisions.
    #[test]
    #[ignore = "isolated before/after memory benchmark, not a routine unit test"]
    fn benchmark_catalog_publication_memory() {
        let root = tempfile::tempdir().unwrap();
        let mut task = publication_fixture(10_000, 2_048);
        task.directory = root.path().to_owned();
        let epoch = super::Arc::new(super::AtomicU64::new(7));
        let started = std::time::Instant::now();
        let (completed, publication) = task.publish_owned(epoch, 7).unwrap();
        let (document, changed, revision) = publication.unwrap();
        assert!(changed);
        assert_eq!(revision, "fixture-r1");
        assert_eq!(document["revision"], "fixture-r1");
        let mut cache = crate::catalog_cache::LocalCatalog::open(
            root.path(),
            &completed.task.server,
            &completed.task.profile,
            completed.artwork_gate.clone(),
        )
        .unwrap();
        assert_eq!(
            cache.store.catalog(&completed.task.profile).unwrap().len(),
            10_000
        );
        println!(
            "client_publication_fixture=10000 overview_bytes=2048 elapsed_ms={} rows=10000 revision=fixture-r1",
            started.elapsed().as_millis()
        );
        std::hint::black_box(completed);
    }
    #[test]
    fn owned_publication_preserves_mirror_when_context_or_mutation_is_stale() {
        for stale_context in [true, false] {
            let root = tempfile::tempdir().unwrap();
            let mut task = publication_fixture(2, 16);
            task.directory = root.path().to_owned();
            let epoch = super::Arc::new(super::AtomicU64::new(if stale_context { 2 } else { 1 }));
            if !stale_context {
                task.task.mutation_guard.store(1, super::Ordering::Release);
            }
            let (_, publication) = task.publish_owned(epoch, 1).unwrap();
            assert!(publication.is_none());
            assert!(!root.path().join("catalog-cache").exists());
        }
    }
    #[test]
    fn unchanged_publication_is_single_use_and_never_opens_a_cache() {
        let root = tempfile::tempdir().unwrap();
        let mut task = publication_fixture(0, 0);
        task.directory = root.path().join("intentionally-absent");
        task.result.as_mut().unwrap().0.changed = false;
        let epoch = super::Arc::new(super::AtomicU64::new(1));
        let (completed, publication) = task.publish_owned(epoch.clone(), 1).unwrap();
        assert_eq!(
            publication.unwrap(),
            (serde_json::Value::Null, false, "fixture-r1".into())
        );
        assert!(!completed.directory.exists());
        assert!(completed.publish_owned(epoch, 1).is_err());
    }
    #[test]
    fn legacy_library_update_does_not_send_the_new_optional_field() {
        assert_eq!(
            super::library_update_body(Some("/fixture/films".into()), None, 2),
            serde_json::json!({"video_root":"/fixture/films","expected_revision":2})
        );
        assert_eq!(
            super::library_update_body(None, Some(vec![]), 3),
            serde_json::json!({"video_root":null,"additional_video_roots":[],"expected_revision":3})
        );
    }
    #[test]
    fn opaque_series_keys_are_explicit_on_the_wire() {
        assert_eq!(
            super::series_selector_query("tmdb:tv:1001"),
            vec![("selector", "key".into())]
        );
        assert_eq!(
            super::series_selector_query("media:11111111-1111-4111-8111-111111111111"),
            vec![("selector", "key".into())]
        );
        assert!(super::series_selector_query("Doctor Who").is_empty());
    }
    use super::*;
    #[test]
    fn sync_generation_accepts_dirty_base_but_rejects_late_mutation() {
        // Dirty is a hint, not a CAS conflict: an explicit sync begun after a
        // mutation must be allowed to clear it. A second mutation is stale.
        assert!(!sync_commit_stale(Some("r1"), Some("r1"), 7, 7));
        assert!(sync_commit_stale(Some("r1"), Some("r1"), 8, 7));
    }
    #[test]
    fn persisted_catalog_navigation_never_needs_a_reachable_server() {
        let dir = tempfile::tempdir().unwrap();
        let server = uuid::Uuid::new_v4().to_string();
        let profile = uuid::Uuid::new_v4().to_string();
        let media_id = uuid::Uuid::new_v4().to_string();
        let gate = Arc::new(tokio::sync::Semaphore::new(1));
        let mut mirror =
            crate::catalog_cache::LocalCatalog::open(dir.path(), &server, &profile, gate).unwrap();
        let item:chill_domain::SyncMedia=serde_json::from_value(json!({"media":{"id":media_id,"path":"Shows/E01.mkv","title":"Pilot","category":"serie","series":"Show","season":1,"episode":1,"year":2026,"genres":["Drama"],"overview":"Local","poster":null,"saga":null,"watched":true,"available":true,"added_at":1,"tmdb_id":null,"rating":4},"revision":7})).unwrap();
        let a = uuid::Uuid::new_v4().to_string();
        let b = uuid::Uuid::new_v4().to_string();
        let mut film_a = item.clone();
        film_a.media.id = a.clone();
        film_a.media.title = "Film A".into();
        film_a.media.category = "film".into();
        film_a.media.series = None;
        film_a.media.season = None;
        film_a.media.episode = None;
        film_a.media.cast = vec!["Shared Actor".into()];
        film_a.media.year = Some(1900);
        film_a.media.genres.clear();
        let mut film_b = film_a.clone();
        film_b.media.id = b.clone();
        film_b.media.title = "Film B".into();
        film_b.media.year = Some(2000);
        let mut document = json!({"revision":"fixture:1","sagas":[{"id":"1","name":"Saga","revision":5,"media_ids":[a,b],"poster":null}],"playlists":[],"series_revisions":{"Show":9}});
        document["series_revisions"][format!("media:{media_id}")] = json!(7);
        mirror.commit(&[item, film_a, film_b], document).unwrap();
        drop(mirror);
        let mut client = UiClient::open(dir.path()).unwrap();
        // Port 9 is deliberately unreachable. Any accidental HTTP makes this
        // navigation fail; no mocks replace the cache/SQLite/filter path.
        let transport = Transport::new(
            ServerEndpoint::parse(&server, "https://127.0.0.1:9").unwrap(),
            Arc::new(RwLock::new(TrustStore::default())),
            Some(Duration::from_millis(100)),
        )
        .unwrap();
        client.connected = Some(Connected {
            address: "https://127.0.0.1:9".into(),
            id: server.clone(),
            transport,
            profiles: json!([{ "id":profile,"name":"Test","revision":1}]),
            active: Some(profile.clone()),
            notice: None,
        });
        let page = client
            .dispatch(UiCommand::Catalog {
                offset: 0,
                category: Some("serie".into()),
                q: Some("Show".into()),
                genre: None,
                year: None,
                watched: Some(true),
                sort: None,
                grouped: Some(true),
                available: None,
            })
            .unwrap();
        assert_eq!(page["items"][0]["id"], media_id);
        assert_eq!(page["items"][0]["title"], "Show");
        assert_eq!(page["series_counters"][&media_id]["unwatched"], 0);
        let detail = client
            .dispatch(UiCommand::MediaDetail {
                media_id: media_id.clone(),
            })
            .unwrap();
        assert_eq!(detail["revision"], 7);
        assert_eq!(detail["media"]["rating"], 4);
        let series = client
            .dispatch(UiCommand::Series {
                name: "Show".into(),
            })
            .unwrap();
        assert_eq!(series["revision"], 9);
        assert_eq!(series["episodes"].as_array().unwrap().len(), 1);
        let media_series = client
            .dispatch(UiCommand::Series {
                name: format!("media:{media_id}"),
            })
            .unwrap();
        assert_eq!(media_series["revision"], 7);
        assert_eq!(
            client.dispatch(UiCommand::Sagas).unwrap()["items"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let related = client
            .dispatch(UiCommand::RelatedMedia {
                media_id: a.clone(),
            })
            .unwrap();
        assert_eq!(
            related["items"][0]["id"], b,
            "shared cast recommendations must work entirely from the mirror"
        );
        let advice = client.recommendation_catalog(&Default::default()).unwrap();
        assert_eq!(
            advice.len(),
            3,
            "advice reads the same grouped mirror without HTTP"
        );
        assert!(
            advice
                .iter()
                .all(|item| item.media.episode.is_none() && item.media.season.is_none())
        );
        client.invalidate_catalog();
        client.ensure_catalog(false).unwrap();
        assert!(client.catalog_cache.as_ref().unwrap().notice.is_none());
        assert!(
            client.catalog_cache.as_ref().unwrap().dirty,
            "local reads preserve a pending mutation until explicit sync"
        );
        assert_eq!(
            client
                .dispatch(UiCommand::RelatedMedia { media_id: a })
                .unwrap()["items"][0]["id"],
            b
        );
        assert_eq!(
            client.dispatch(UiCommand::Playlists).unwrap()["items"],
            json!([])
        );
        let sibling_id = uuid::Uuid::new_v4().to_string();
        let other = crate::catalog_cache::LocalCatalog::open(
            dir.path(),
            &server,
            &sibling_id,
            Arc::new(tokio::sync::Semaphore::new(1)),
        )
        .unwrap();
        assert!(other.document.is_none());
        let sibling = dir
            .path()
            .join("catalog-cache")
            .join(&server)
            .join(sibling_id);
        drop(other);
        // Confirmed DELETE boundary: subsequent GET /profiles fails on port9.
        // Cleanup must already be complete, never deferred to view().
        client.finish_profile_deletion(&profile).unwrap();
        assert!(client.view().is_err());
        assert!(client.catalog_cache.is_none());
        assert!(client.connected.as_ref().unwrap().active.is_none());
        assert!(
            !dir.path()
                .join("catalog-cache")
                .join(&server)
                .join(&profile)
                .exists()
        );
        assert!(sibling.is_dir());
        assert!(client.dispatch(UiCommand::Sagas).is_err());
    }
    #[test]
    fn player_presentation_defaults_fullscreen_and_is_local_durable_and_transactional() {
        let legacy: Settings = serde_json::from_value(json!({"theme":"dark"})).unwrap();
        assert!(legacy.player_presentation.fullscreen_on_start);
        let dir = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let mut client = UiClient::open(dir.path()).unwrap();
        assert_eq!(
            client.view().unwrap()["player_presentation"]["fullscreen_on_start"],
            true
        );
        let command: UiCommand = serde_json::from_value(
            json!({"action":"save_player_settings","fullscreen_on_start":false}),
        )
        .unwrap();
        assert_eq!(
            client.dispatch(command).unwrap()["player_presentation"]["fullscreen_on_start"],
            false
        );
        assert!(
            !UiClient::open(dir.path())
                .unwrap()
                .settings
                .player_presentation
                .fullscreen_on_start
        );
        assert!(
            UiClient::open(other.path())
                .unwrap()
                .settings
                .player_presentation
                .fullscreen_on_start
        );
        assert!(
            client.connected.is_none(),
            "setting must not contact a server"
        );
        assert!(
            serde_json::from_value::<UiCommand>(
                json!({"action":"save_player_settings","fullscreen_on_start":"false"})
            )
            .is_err()
        );
        client.directory = dir.path().join("missing-directory");
        assert!(
            client
                .dispatch(UiCommand::SavePlayerSettings {
                    fullscreen_on_start: true
                })
                .is_err()
        );
        assert!(!client.settings.player_presentation.fullscreen_on_start);
    }
    #[test]
    fn cache_settings_are_local_persistent_and_default_for_legacy_settings() {
        use crate::playback_cache::PlaybackCache;
        let legacy: Settings = serde_json::from_value(json!({"theme":"dark"})).unwrap();
        assert_eq!(legacy.playback_cache, PlaybackCache::default());
        let dir = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let mut client = UiClient::open(dir.path()).unwrap();
        let cache = PlaybackCache {
            forward_mib: 96,
            back_mib: 0,
        };
        client
            .dispatch(UiCommand::SaveCacheSettings { cache })
            .unwrap();
        assert_eq!(
            UiClient::open(dir.path()).unwrap().settings.playback_cache,
            cache
        );
        assert_eq!(
            UiClient::open(other.path())
                .unwrap()
                .settings
                .playback_cache,
            PlaybackCache::default()
        );
        assert!(
            client.connected.is_none(),
            "no server required or contacted"
        );
        assert_eq!(
            client.dispatch(UiCommand::CacheSettings).unwrap()["cache"]["back_mib"],
            0
        );
        assert_eq!(client.view().unwrap()["playback_cache"]["forward_mib"], 96);
        let invalid_cache = PlaybackCache {
            forward_mib: 513,
            back_mib: 0,
        };
        assert!(
            client
                .dispatch(UiCommand::SaveCacheSettings {
                    cache: invalid_cache
                })
                .is_err()
        );
        assert_eq!(
            UiClient::open(dir.path()).unwrap().settings.playback_cache,
            cache
        );
        client.directory = dir.path().join("missing-directory");
        assert!(
            client
                .dispatch(UiCommand::SaveCacheSettings {
                    cache: PlaybackCache::default()
                })
                .is_err()
        );
        assert_eq!(client.settings.playback_cache, cache);
    }
    #[test]
    fn settings_directory_sync_failure_is_explicit_and_reopen_reads_published_state() {
        let dir = tempfile::tempdir().unwrap();
        let mut client = UiClient::open(dir.path()).unwrap();
        client.settings.theme = "dark".into();
        let error = client
            .save_with_directory_sync(|parent| {
                assert_eq!(parent, dir.path());
                assert!(parent.join("client.json").is_file());
                Err(std::io::Error::other("injected directory fsync failure"))
            })
            .unwrap_err();
        assert_eq!(error.code, "LOCAL_DURABILITY_UNKNOWN");
        assert_eq!(UiClient::open(dir.path()).unwrap().settings.theme, "dark");
        client.save().unwrap();
        assert_eq!(UiClient::open(dir.path()).unwrap().settings.theme, "dark");
    }
    #[test]
    fn provider_and_unavailable_commands_are_closed_and_bounded() {
        let search: UiCommand = serde_json::from_value(
            json!({"action":"tmdb_candidates","media_id":"abc","query":"film"}),
        )
        .unwrap();
        assert!(matches!(search, UiCommand::TmdbCandidates { .. }));
        assert!(valid_tmdb_query(&"x".repeat(500)).is_ok());
        assert!(valid_tmdb_query(&"x".repeat(501)).is_err());
        assert!(
            serde_json::from_value::<UiCommand>(
                json!({"action":"tmdb_candidates","media_id":"x","url":"https://evil"})
            )
            .is_err()
        );
        let remove: UiCommand = serde_json::from_value(json!({"action":"forget_unavailable","media_ids":["a"],"expected_epoch":1,"confirmed":true})).unwrap();
        assert!(matches!(
            remove,
            UiCommand::ForgetUnavailable {
                confirmed: true,
                ..
            }
        ));
        let source = uuid::Uuid::new_v4().to_string();
        let target = uuid::Uuid::new_v4().to_string();
        let confirm: UiCommand=serde_json::from_value(json!({"action":"confirm_reassociation","source_id":source,"target_id":target,"expected_source_revision":1,"expected_target_revision":2,"expected_epoch":3,"confirmed":true})).unwrap();
        assert!(matches!(confirm, UiCommand::ConfirmReassociation { .. }));
        assert!(serde_json::from_value::<UiCommand>(json!({"action":"request_full_fingerprint","media_id":"bad","confirmed":true,"path":"/nas/x"})).is_err());
    }
    #[test]
    fn saga_artwork_commands_are_closed_and_scoped() {
        let media_id = uuid::Uuid::new_v4().to_string();
        let preview: UiCommand = serde_json::from_value(json!({
            "action":"saga_artwork",
            "id":"saga_42"
        }))
        .unwrap();
        assert!(matches!(preview, UiCommand::SagaArtwork { .. }));
        let copy: UiCommand = serde_json::from_value(json!({
            "action":"saga_member_artwork",
            "id":"saga_42",
            "expected_revision":7,
            "media_id":media_id
        }))
        .unwrap();
        assert!(matches!(copy, UiCommand::SagaMemberArtwork { .. }));
        assert!(
            serde_json::from_value::<UiCommand>(json!({
                "action":"saga_member_artwork",
                "id":"saga_42",
                "expected_revision":7,
                "media_id":uuid::Uuid::new_v4().to_string(),
                "url":"https://example.invalid/poster.jpg"
            }))
            .is_err()
        );
    }
    #[test]
    fn failed_initial_connection_discards_provisional_trust_and_never_persists_it() {
        let dir = tempfile::tempdir().unwrap();
        let mut client = UiClient::open(dir.path()).unwrap();
        client.save().unwrap();
        let before = client.trust.read().unwrap().clone();
        client.pending = Some(Pending {
            address: "https://127.0.0.1:1/".into(),
            known_id: None,
            token: "approval-token".into(),
            expires: Instant::now() + Duration::from_secs(1),
            certificate: PresentedCertificate {
                der: vec![1],
                spki_sha256: [1; 32],
                not_before_unix: 1,
                not_after_unix: i64::MAX,
                subject: "isolated fixture".into(),
            },
        });
        assert!(
            client
                .dispatch(UiCommand::Approve {
                    token: "approval-token".into()
                })
                .is_err()
        );
        assert!(client.connected.is_none());
        assert_eq!(*client.trust.read().unwrap(), before);
        assert_eq!(UiClient::open(dir.path()).unwrap().settings.trust, before);
        assert!(client.settings.servers.is_empty());
    }
    #[test]
    fn expired_key_replacement_is_visible_but_same_key_renewal_is_not_mislabelled() {
        let mut trust = TrustStore::default();
        trust.events.push(crate::TrustEvent {
            server_id: "server".into(),
            at_unix: 1,
            kind: TrustEventKind::CertificateRenewedWithSameKey,
        });
        assert!(renewal_notice(&trust, 0).is_none());
        trust.events.push(crate::TrustEvent {
            server_id: "server".into(),
            at_unix: 2,
            kind: TrustEventKind::KeyReplacedAfterKnownExpiry,
        });
        assert!(
            renewal_notice(&trust, 1)
                .unwrap()
                .contains("acceptée automatiquement")
        );
        assert!(renewal_notice(&trust, 2).is_none());
    }
    #[test]
    fn safe_api_codes_have_operation_specific_recovery() {
        for (code, text) in [
            ("SCAN_ACTIVE", "scan"),
            ("ROOT_REQUIRED", "Configuration"),
            ("PROFILE_CONFLICT", "concurrente"),
            ("MEDIA_NOT_FOUND", "élément"),
            ("MEDIA_FILE_MISSING", "indisponible"),
            (
                "IMPORT_MULTIPLE_LIBRARY_LOCATIONS_UNSUPPORTED",
                "aucun import",
            ),
            ("DATABASE_OFFLINE", "base"),
            ("BUSY", "occupé"),
        ] {
            let error = UiError::from(ClientError::Api {
                status: reqwest::StatusCode::CONFLICT,
                code: code.into(),
            });
            assert_eq!(error.code, code);
            assert!(error.message.contains(text));
        }
    }
    #[test]
    fn failed_profile_switch_preserves_displayed_context_and_last_choice() {
        let dir = tempfile::tempdir().unwrap();
        let mut client = UiClient::open(dir.path()).unwrap();
        let a = uuid::Uuid::new_v4().to_string();
        let b = uuid::Uuid::new_v4().to_string();
        let transport = Transport::new(
            ServerEndpoint::parse("server", "https://127.0.0.1:1").unwrap(),
            client.trust.clone(),
            Some(Duration::from_millis(1)),
        )
        .unwrap();
        client
            .settings
            .last_profiles
            .insert("server".into(), a.clone());
        client.save().unwrap();
        client.connected = Some(Connected {
            address: "https://127.0.0.1:1/".into(),
            id: "server".into(),
            transport,
            profiles: json!([{"id":a,"name":"A","revision":1},{"id":b,"name":"B","revision":1}]),
            active: Some(a.clone()),
            notice: None,
        });
        assert!(client.dispatch(UiCommand::SelectProfile { id: b }).is_err());
        assert_eq!(client.profile().unwrap(), a);
        assert_eq!(client.settings.last_profiles["server"], a);
        let saved = UiClient::open(dir.path()).unwrap();
        assert_eq!(saved.settings.last_profiles["server"], a);
        // Also exercise failure after a successful prospective GET/view, at the
        // persistence boundary, without a mock HTTP stack or production test hook.
        client.connected.as_mut().unwrap().active = Some(uuid::Uuid::new_v4().to_string());
        client.directory = dir.path().join("unavailable-settings-directory");
        assert!(
            client
                .commit_selected_view(
                    "server".into(),
                    Some(a.clone()),
                    Ok(json!({"stage":"connected"}))
                )
                .is_err()
        );
        assert_eq!(client.profile().unwrap(), a);
        assert_eq!(client.settings.last_profiles["server"], a);
        assert_eq!(
            UiClient::open(dir.path()).unwrap().settings.last_profiles["server"],
            a
        );
    }
    #[test]
    fn addresses_are_editable_but_not_arbitrary_urls() {
        assert_eq!(
            normalize_address("media.home:9443").unwrap(),
            "https://media.home:9443/"
        );
        assert_eq!(
            normalize_address("10.1.0.12").unwrap(),
            "https://10.1.0.12:8443/"
        );
        for url in [
            "",
            "http://host",
            "https://u:p@host",
            "https://host/path",
            "https://host/?secret=x",
        ] {
            assert!(normalize_address(url).is_err());
        }
    }
    #[test]
    fn profile_selection_and_consent_are_not_caller_forgeable() {
        let dir = tempfile::tempdir().unwrap();
        let mut client = UiClient::open(dir.path()).unwrap();
        assert_eq!(client.dispatch(UiCommand::Boot).unwrap()["stage"], "server");
        assert!(
            client
                .dispatch(UiCommand::Approve {
                    token: "forged".into()
                })
                .is_err()
        );
        assert!(
            client
                .dispatch(UiCommand::Catalog {
                    offset: 0,
                    category: None,
                    q: None,
                    genre: None,
                    year: None,
                    watched: None,
                    sort: None,
                    grouped: None,
                    available: None,
                })
                .is_err()
        );
        assert!(
            client
                .dispatch(UiCommand::SelectProfile {
                    id: uuid::Uuid::new_v4().to_string()
                })
                .is_err()
        );
    }
    #[test]
    fn catalog_query_only_emits_allowlisted_filters() {
        let query = catalog_query(
            48,
            Some("serie".into()),
            Some("episode".into()),
            Some("Drame".into()),
            Some(0),
            Some(false),
            Some("rating_desc".into()),
            Some(true),
            Some(true),
        )
        .unwrap();
        assert_eq!(
            query,
            vec![
                ("offset", "48".into()),
                ("limit", "48".into()),
                ("category", "serie".into()),
                ("q", "episode".into()),
                ("genre", "Drame".into()),
                ("year", "0".into()),
                ("watched", "false".into()),
                ("sort", "rating_desc".into()),
                ("grouped", "true".into()),
                ("available", "true".into())
            ]
        );
        assert!(
            catalog_query(
                0,
                None,
                None,
                Some("x".repeat(201)),
                None,
                None,
                None,
                None,
                None
            )
            .is_err()
        );
        assert!(catalog_query(0, None, None, None, Some(1849), None, None, None, None).is_err());
        assert!(
            catalog_query(
                0,
                None,
                None,
                None,
                None,
                None,
                Some("random".into()),
                None,
                None
            )
            .is_err()
        );
    }
    #[test]
    fn series_edit_payload_is_closed_and_keeps_the_revision_outside_the_patch() {
        let payload = series_edit_payload(9, json!({"name":"Série", "year":0, "genres":["Drame"], "overview":"Commun", "cast":[], "directors":[]})).unwrap();
        assert_eq!(payload["expected_revision"], 9);
        assert_eq!(payload["patch"]["name"], "Série");
        assert!(series_edit_payload(1, json!({"series":"wrong key"})).is_err());
        assert!(series_edit_payload(1, json!({"name":""})).is_err());
    }
    #[test]
    fn theme_persists_on_this_device() {
        let dir = tempfile::tempdir().unwrap();
        {
            let mut c = UiClient::open(dir.path()).unwrap();
            c.dispatch(UiCommand::Theme {
                theme: "dark".into(),
            })
            .unwrap();
        }
        assert_eq!(
            UiClient::open(dir.path())
                .unwrap()
                .dispatch(UiCommand::Boot)
                .unwrap()["theme"],
            "dark"
        );
    }
    #[test]
    fn language_is_local_persisted_and_legacy_settings_default_to_auto() {
        let legacy: Settings = serde_json::from_value(json!({"theme":"dark"})).unwrap();
        assert_eq!(legacy.language, UiLanguage::Auto);
        let dir = tempfile::tempdir().unwrap();
        for language in [UiLanguage::Fr, UiLanguage::En, UiLanguage::Auto] {
            let mut client = UiClient::open(dir.path()).unwrap();
            client
                .dispatch(UiCommand::SaveLanguage { language })
                .unwrap();
            assert_eq!(
                UiClient::open(dir.path()).unwrap().settings.language,
                language
            );
        }
        assert!(
            serde_json::from_value::<UiCommand>(json!({"action":"save_language","language":"de"}))
                .is_err()
        );
    }
}
