//! Allowlisted UI operations. No JS-supplied HTTP paths, headers or local/NAS paths.
use crate::{
    ClientError, HttpMethod, PresentedCertificate, ServerEndpoint, Transport, TrustStore,
    inspect_certificate,
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
    StartScan,
    ScanStatus,
    Theme {
        theme: String,
    },
}
#[derive(Debug, Serialize)]
pub struct UiError {
    pub message: String,
    pub status: Option<u16>,
}
impl From<ClientError> for UiError {
    fn from(error: ClientError) -> Self {
        let status = if let ClientError::Http(status) = &error {
            Some(status.as_u16())
        } else {
            None
        };
        let message = match error {
            ClientError::Http(status) if status.as_u16() == 409 => {
                "Modification concurrente : rechargez les données avant de réessayer."
            }
            ClientError::Http(status) if status.as_u16() == 404 => {
                "Profil ou élément supprimé : reconnectez/rechargez le serveur."
            }
            ClientError::Http(status) if status.as_u16() == 400 => {
                "Valeur invalide ou nom déjà utilisé. Vérifiez les champs."
            }
            _ => {
                "Connexion HTTPS impossible ou réponse invalide. Vérifiez le serveur et son certificat."
            }
        };
        Self {
            message: message.into(),
            status,
        }
    }
}
fn invalid(message: &str) -> UiError {
    UiError {
        message: message.into(),
        status: None,
    }
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    servers: BTreeMap<String, String>,
    last_profiles: BTreeMap<String, String>,
    last_address: Option<String>,
    theme: String,
    trust: TrustStore,
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
}
pub struct UiClient {
    directory: PathBuf,
    settings: Settings,
    trust: Arc<RwLock<TrustStore>>,
    runtime: tokio::runtime::Runtime,
    pending: Option<Pending>,
    connected: Option<Connected>,
}
impl UiClient {
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
        self.save()?;
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
                json!({"stage":"server","address":self.settings.last_address,"servers":self.settings.servers.keys().collect::<Vec<_>>(),"theme":self.theme()}),
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
            json!({"stage":"connected","address":address,"server_id":id,"profile_id":active,"profiles":profiles,"library":library,"preferences":preferences,"theme":self.theme()}),
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
        });
        self.save()?;
        self.view()
    }
    pub fn dispatch(&mut self, command: UiCommand) -> Result<Value, UiError> {
        match command {
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
                self.finish_connect(pending.address, key)
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
                conn.active = Some(id.clone());
                self.settings.last_profiles.insert(conn.id.clone(), id);
                self.save()?;
                self.view()
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
