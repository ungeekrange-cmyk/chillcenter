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
    sync::{Arc, RwLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum UiCommand {
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
    SaveLibrary {
        video_root: Option<String>,
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
    GroupRating {
        media_id: String,
        rating: Option<u8>,
        scope: String,
    },
    StartScan,
    ScanStatus,
    Theme {
        theme: String,
    },
    SaveAppearance {
        appearance: crate::appearance::Appearance,
    },
    ResolveAppearance {
        year: i32,
        month: u8,
        day: u8,
    },
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
                "Configurez le dossier vidéo du serveur dans Administration avant de scanner."
            }
            "CONFIG_INVALID_OR_UNWRITABLE" => {
                "Chemin invalide ou réglages non enregistrables. Vérifiez le chemin et les droits du serveur."
            }
            "PROFILE_NAME_TAKEN" => "Ce nom de profil est déjà utilisé. Choisissez un autre nom.",
            "PROFILE_NOT_FOUND" => {
                "Ce profil a été supprimé. Rechargez et choisissez un profil existant."
            }
            "MEDIA_NOT_FOUND" => "Cet élément n'existe plus. Rechargez le catalogue.",
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

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    servers: BTreeMap<String, String>,
    last_profiles: BTreeMap<String, String>,
    last_address: Option<String>,
    theme: String,
    trust: TrustStore,
    device_id: Option<String>,
    appearance: crate::appearance::Appearance,
}

/// Native-host-only snapshot. Never serialized into the webview.
pub struct PlaybackConnection {
    pub transport: Transport,
    pub profile_id: String,
    pub server_id: String,
    pub device_id: String,
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
}
impl UiClient {
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
        })
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
        let settings = match fs::read(directory.join("client.json")) {
            Ok(bytes) if bytes.len() <= 4 * 1024 * 1024 => {
                serde_json::from_slice(&bytes).map_err(|_| invalid("Réglages locaux invalides."))?
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Settings::default(),
            _ => return Err(invalid("Réglages locaux inaccessibles ou trop volumineux.")),
        };
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
        })
    }
    fn save(&mut self) -> Result<(), UiError> {
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
        let result = self
            .runtime
            .block_on(transport.json_with_options(method, path, body, query, headers))?;
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
                json!({"stage":"server","address":self.settings.last_address,"servers":self.settings.servers.keys().collect::<Vec<_>>(),"theme":self.theme(),"appearance":self.settings.appearance}),
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
        Ok(
            json!({"stage":"connected","address":address,"server_id":id,"profile_id":active,"profiles":profiles,"library":library,"preferences":preferences,"theme":self.theme(),"appearance":self.settings.appearance,"notice":self.connected.as_ref().unwrap().notice}),
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
        let result = json!({"stage":"trust","address":address,"fingerprint":certificate.fingerprint_hex(),"expires_at":certificate.not_after_unix(),"replacement":known.is_some(),"token":token,"theme":self.theme()});
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
        if identity["api_version"] != 1 {
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
    pub fn dispatch(&mut self, command: UiCommand) -> Result<Value, UiError> {
        match command {
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
                let mime = if data.starts_with(b"\x89PNG\r\n\x1a\n") {
                    "image/png"
                } else if data.starts_with(&[0xff, 0xd8, 0xff]) {
                    "image/jpeg"
                } else if data.len() >= 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WEBP" {
                    "image/webp"
                } else {
                    return Err(invalid("Affiche non prise en charge."));
                };
                use base64::Engine;
                Ok(
                    json!({"src":format!("data:{mime};base64,{}",base64::engine::general_purpose::STANDARD.encode(data))}),
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
                if name.is_empty() || name.len() > 500 {
                    return Err(invalid("Série invalide."));
                }
                let p = self.profile()?;
                let segment = percent_segment(&name);
                self.request(
                    HttpMethod::GET,
                    &format!("/api/v1/profiles/{p}/series/{segment}"),
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
            UiCommand::SaveAppearance { appearance } => {
                let previous = self.settings.appearance.clone();
                self.settings.appearance = appearance;
                if let Err(e) = self.save() {
                    self.settings.appearance = previous;
                    return Err(e);
                }
                Ok(json!({"appearance":self.settings.appearance}))
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
                self.pending = None;
                self.connected = None;
                self.view()
            }
            UiCommand::Refresh => self.view(),
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
                    Some(&json!({"name":name})),
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
                self.view()
            }
            UiCommand::SaveLibrary {
                video_root,
                expected_revision,
            } => {
                self.request(
                    HttpMethod::PUT,
                    "/api/v1/admin/library",
                    Some(&json!({"video_root":video_root,"expected_revision":expected_revision})),
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
            } => {
                let profile = self.profile()?;
                let mut query = vec![("offset", offset.to_string()), ("limit", "50".into())];
                if let Some(category) = category.filter(|s| !s.is_empty()) {
                    query.push(("category", category));
                }
                if let Some(q) = q.filter(|s| !s.is_empty()) {
                    query.push(("q", q));
                }
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
fn valid_id(id: &str) -> Result<(), UiError> {
    uuid::Uuid::parse_str(id)
        .map(|_| ())
        .map_err(|_| invalid("Identifiant invalide."))
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
    use super::*;
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
            ("ROOT_REQUIRED", "dossier"),
            ("PROFILE_CONFLICT", "concurrente"),
            ("MEDIA_NOT_FOUND", "élément"),
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
                    q: None
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
}
