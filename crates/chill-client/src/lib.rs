//! HTTPS transport shared by the API, media, and subtitle paths.
//!
//! Trust is scoped to a configured server and address.  There is deliberately
//! no "accept invalid certificates" switch: the only unpinned connection is a
//! certificate-only probe whose result must be approved before HTTP is sent.

use reqwest::{Method, StatusCode, Url, header};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, Error as TlsError, SignatureScheme};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::net::{IpAddr, TcpStream, ToSocketAddrs};
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use x509_parser::extensions::GeneralName;
use x509_parser::prelude::*;
pub mod playback;
pub mod ui;
pub mod appearance;
pub mod watch_clock;
pub use reqwest::Method as HttpMethod;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("adresse de serveur invalide: {0}")]
    InvalidServerUrl(String),
    #[error("chemin de requête refusé: {0}")]
    InvalidRequestPath(String),
    #[error("certificat refusé: {0}")]
    Certificate(String),
    #[error("le certificat de ce serveur doit être approuvé explicitement")]
    ApprovalRequired,
    #[error("la clé du serveur a changé avant l'expiration du certificat approuvé")]
    KeyChangedBeforeExpiry,
    #[error("serveur inconnu: {0}")]
    UnknownServer(String),
    #[error("erreur réseau: {0}")]
    Network(#[from] reqwest::Error),
    #[error("erreur TLS: {0}")]
    Tls(#[from] TlsError),
    #[error("erreur d'E/S: {0}")]
    Io(#[from] io::Error),
    #[error("réponse HTTP inattendue: {0}")]
    Http(StatusCode),
    #[error("opération serveur refusée: {code} ({status})")]
    Api { status: StatusCode, code: String },
    #[error("réponse Range invalide: {0}")]
    InvalidRangeResponse(String),
    #[error("réponse JSON invalide: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PresentedCertificate {
    der: Vec<u8>,
    spki_sha256: [u8; 32],
    not_before_unix: i64,
    not_after_unix: i64,
    subject: String,
}

impl PresentedCertificate {
    pub fn fingerprint_hex(&self) -> String {
        self.spki_sha256
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<Vec<_>>()
            .join(":")
    }

    pub fn subject(&self) -> &str {
        &self.subject
    }
    pub fn not_before_unix(&self) -> i64 {
        self.not_before_unix
    }
    pub fn not_after_unix(&self) -> i64 {
        self.not_after_unix
    }

    fn validate(der: &[u8], host: &str, now: SystemTime) -> Result<Self, ClientError> {
        let (_, certificate) = X509Certificate::from_der(der)
            .map_err(|error| ClientError::Certificate(format!("DER invalide: {error}")))?;
        let timestamp = now
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ClientError::Certificate("horloge système antérieure à 1970".into()))?
            .as_secs() as i64;
        let asn1_now = ASN1Time::from_timestamp(timestamp)
            .map_err(|error| ClientError::Certificate(format!("horloge invalide: {error}")))?;
        if !certificate.validity().is_valid_at(asn1_now) {
            return Err(ClientError::Certificate(
                "certificat hors de sa période de validité".into(),
            ));
        }
        certificate.verify_signature(None).map_err(|error| {
            ClientError::Certificate(format!("signature du certificat invalide: {error}"))
        })?;
        if !certificate_matches_host(&certificate, host)? {
            return Err(ClientError::Certificate(format!(
                "le SAN ne contient pas l'adresse {host}"
            )));
        }
        let spki_sha256 = Sha256::digest(certificate.public_key().raw)
            .as_slice()
            .try_into()
            .expect("SHA-256 has a fixed length");
        Ok(Self {
            der: der.to_vec(),
            spki_sha256,
            not_before_unix: certificate.validity().not_before.timestamp(),
            not_after_unix: certificate.validity().not_after.timestamp(),
            subject: certificate.subject().to_string(),
        })
    }
}

fn certificate_matches_host(
    certificate: &X509Certificate<'_>,
    host: &str,
) -> Result<bool, ClientError> {
    let san = certificate
        .subject_alternative_name()
        .map_err(|error| ClientError::Certificate(format!("SAN invalide: {error}")))?
        .ok_or_else(|| ClientError::Certificate("certificat sans SAN".into()))?;
    if let Ok(expected_ip) = host.parse::<IpAddr>() {
        return Ok(san.value.general_names.iter().any(|name| match name {
            GeneralName::IPAddress(bytes) => match expected_ip {
                IpAddr::V4(ip) => bytes.len() == 4 && bytes == &ip.octets(),
                IpAddr::V6(ip) => bytes.len() == 16 && bytes == &ip.octets(),
            },
            _ => false,
        }));
    }
    let expected = host.trim_end_matches('.').to_ascii_lowercase();
    Ok(san.value.general_names.iter().any(|name| match name {
        GeneralName::DNSName(name) => dns_name_matches(name, &expected),
        _ => false,
    }))
}

fn dns_name_matches(pattern: &str, host: &str) -> bool {
    let pattern = pattern.trim_end_matches('.').to_ascii_lowercase();
    if pattern == host {
        return true;
    }
    let Some(suffix) = pattern.strip_prefix("*.") else {
        return false;
    };
    host.strip_suffix(suffix)
        .is_some_and(|prefix| prefix.ends_with('.') && !prefix[..prefix.len() - 1].contains('.'))
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrustedServer {
    pub server_id: String,
    pub aliases: BTreeSet<String>,
    pub certificate_der: Vec<u8>,
    pub spki_sha256: [u8; 32],
    pub certificate_not_after_unix: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrustEventKind {
    CertificateRenewedWithSameKey,
    KeyReplacedAfterKnownExpiry,
    ExplicitKeyReplacement,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrustEvent {
    pub server_id: String,
    pub at_unix: i64,
    pub kind: TrustEventKind,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrustStore {
    pub servers: BTreeMap<String, TrustedServer>,
    pub events: Vec<TrustEvent>,
}

impl TrustStore {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ClientError> {
        match fs::read(path) {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error.into()),
        }
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), ClientError> {
        let path = path.as_ref();
        let temporary = path.with_extension("tmp");
        fs::write(&temporary, serde_json::to_vec_pretty(self)?)?;
        fs::rename(temporary, path)?;
        Ok(())
    }

    pub fn approve_initial(
        &mut self,
        server_id: impl Into<String>,
        alias: &str,
        certificate: PresentedCertificate,
    ) {
        let server_id = server_id.into();
        self.servers.insert(
            server_id.clone(),
            TrustedServer {
                server_id,
                aliases: BTreeSet::from([normalize_alias(alias)]),
                certificate_der: certificate.der,
                spki_sha256: certificate.spki_sha256,
                certificate_not_after_unix: certificate.not_after_unix,
            },
        );
    }

    pub fn approve_key_replacement(
        &mut self,
        server_id: &str,
        alias: &str,
        certificate: PresentedCertificate,
        now: SystemTime,
    ) -> Result<(), ClientError> {
        let trusted = self
            .servers
            .get_mut(server_id)
            .ok_or_else(|| ClientError::UnknownServer(server_id.into()))?;
        trusted.aliases.insert(normalize_alias(alias));
        apply_certificate(trusted, &certificate);
        self.events.push(TrustEvent {
            server_id: server_id.into(),
            at_unix: unix_seconds(now),
            kind: TrustEventKind::ExplicitKeyReplacement,
        });
        Ok(())
    }
}

fn normalize_alias(alias: &str) -> String {
    alias.trim_end_matches('.').to_ascii_lowercase()
}

fn unix_seconds(now: SystemTime) -> i64 {
    now.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64
}

fn apply_certificate(trusted: &mut TrustedServer, certificate: &PresentedCertificate) {
    trusted.certificate_der.clone_from(&certificate.der);
    trusted.spki_sha256 = certificate.spki_sha256;
    trusted.certificate_not_after_unix = certificate.not_after_unix;
}

#[derive(Clone, Debug)]
pub struct ServerEndpoint {
    pub server_id: String,
    pub base_url: Url,
}

impl ServerEndpoint {
    pub fn parse(server_id: impl Into<String>, value: &str) -> Result<Self, ClientError> {
        let mut base_url =
            Url::parse(value).map_err(|error| ClientError::InvalidServerUrl(error.to_string()))?;
        if base_url.scheme() != "https" || base_url.host_str().is_none() {
            return Err(ClientError::InvalidServerUrl(
                "une URL HTTPS avec nom DNS ou adresse IP est requise".into(),
            ));
        }
        if !base_url.username().is_empty()
            || base_url.password().is_some()
            || base_url.query().is_some()
            || base_url.fragment().is_some()
        {
            return Err(ClientError::InvalidServerUrl(
                "identifiants, requête et fragment sont interdits dans l'adresse du serveur".into(),
            ));
        }
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        Ok(Self {
            server_id: server_id.into(),
            base_url,
        })
    }

    pub fn host(&self) -> &str {
        self.base_url.host_str().expect("validated endpoint")
    }

    fn request_url(&self, path: &str) -> Result<Url, ClientError> {
        if !path.starts_with('/')
            || path.starts_with("//")
            || path.split('/').any(|part| part == "..")
        {
            return Err(ClientError::InvalidRequestPath(path.into()));
        }
        let mut url = self.base_url.clone();
        url.set_path(path);
        url.set_query(None);
        Ok(url)
    }
}

/// Performs a TLS handshake only. No HTTP request is sent.
pub fn inspect_certificate(
    server_url: &str,
    timeout: Duration,
) -> Result<PresentedCertificate, ClientError> {
    let endpoint = ServerEndpoint::parse("unapproved", server_url)?;
    let host = endpoint.host().to_owned();
    let port = endpoint.base_url.port_or_known_default().unwrap_or(443);
    let capture = Arc::new(CaptureVerifier::new(host.clone()));
    let config = tls_config(capture.clone());
    let server_name = ServerName::try_from(host.clone())
        .map_err(|error| ClientError::InvalidServerUrl(error.to_string()))?;
    let mut connection = rustls::ClientConnection::new(config, server_name)?;
    let addresses = (host.as_str(), port).to_socket_addrs()?;
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(mut socket) => {
                socket.set_read_timeout(Some(timeout))?;
                socket.set_write_timeout(Some(timeout))?;
                connection.complete_io(&mut socket)?;
                return capture
                    .captured
                    .read()
                    .expect("capture lock poisoned")
                    .clone()
                    .ok_or(ClientError::ApprovalRequired);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error
        .unwrap_or_else(|| io::Error::new(io::ErrorKind::NotFound, "aucune adresse DNS"))
        .into())
}

#[derive(Debug)]
struct CaptureVerifier {
    host: String,
    captured: RwLock<Option<PresentedCertificate>>,
    provider: Arc<rustls::crypto::CryptoProvider>,
}

impl CaptureVerifier {
    fn new(host: String) -> Self {
        Self {
            host,
            captured: RwLock::new(None),
            provider: Arc::new(rustls::crypto::ring::default_provider()),
        }
    }
}

macro_rules! delegate_signature_verification {
    () => {
        fn verify_tls12_signature(
            &self,
            message: &[u8],
            cert: &CertificateDer<'_>,
            dss: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, TlsError> {
            rustls::crypto::verify_tls12_signature(
                message,
                cert,
                dss,
                &self.provider.signature_verification_algorithms,
            )
        }
        fn verify_tls13_signature(
            &self,
            message: &[u8],
            cert: &CertificateDer<'_>,
            dss: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, TlsError> {
            rustls::crypto::verify_tls13_signature(
                message,
                cert,
                dss,
                &self.provider.signature_verification_algorithms,
            )
        }
        fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
            self.provider
                .signature_verification_algorithms
                .supported_schemes()
        }
    };
}

impl ServerCertVerifier for CaptureVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        _: &ServerName<'_>,
        _: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, TlsError> {
        let presented = PresentedCertificate::validate(
            end_entity.as_ref(),
            &self.host,
            UNIX_EPOCH + Duration::from_secs(now.as_secs()),
        )
        .map_err(|error| TlsError::General(error.to_string()))?;
        *self
            .captured
            .write()
            .map_err(|_| TlsError::General("trust lock poisoned".into()))? = Some(presented);
        Ok(ServerCertVerified::assertion())
    }

    delegate_signature_verification!();
}

#[derive(Debug)]
struct PinnedVerifier {
    server_id: String,
    host: String,
    store: Arc<RwLock<TrustStore>>,
    provider: Arc<rustls::crypto::CryptoProvider>,
}

impl ServerCertVerifier for PinnedVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        _: &ServerName<'_>,
        _: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, TlsError> {
        let system_time = UNIX_EPOCH + Duration::from_secs(now.as_secs());
        let presented =
            PresentedCertificate::validate(end_entity.as_ref(), &self.host, system_time)
                .map_err(|error| TlsError::General(error.to_string()))?;
        let mut store = self
            .store
            .write()
            .map_err(|_| TlsError::General("trust lock poisoned".into()))?;
        let trusted = store
            .servers
            .get_mut(&self.server_id)
            .ok_or_else(|| TlsError::General(ClientError::ApprovalRequired.to_string()))?;
        if !trusted.aliases.contains(&normalize_alias(&self.host)) {
            return Err(TlsError::General(
                "adresse absente des alias approuvés".into(),
            ));
        }
        let event_kind = if trusted.spki_sha256 == presented.spki_sha256 {
            if trusted.certificate_der == presented.der {
                return Ok(ServerCertVerified::assertion());
            }
            Some(TrustEventKind::CertificateRenewedWithSameKey)
        } else if trusted.certificate_not_after_unix <= now.as_secs() as i64 {
            Some(TrustEventKind::KeyReplacedAfterKnownExpiry)
        } else {
            return Err(TlsError::General(
                ClientError::KeyChangedBeforeExpiry.to_string(),
            ));
        };
        apply_certificate(trusted, &presented);
        store.events.push(TrustEvent {
            server_id: self.server_id.clone(),
            at_unix: now.as_secs() as i64,
            kind: event_kind.expect("changed certificate"),
        });
        Ok(ServerCertVerified::assertion())
    }

    delegate_signature_verification!();
}

fn tls_config(verifier: Arc<dyn ServerCertVerifier>) -> Arc<ClientConfig> {
    Arc::new(
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_protocol_versions(&[&rustls::version::TLS13, &rustls::version::TLS12])
            .expect("ring supports TLS 1.2 and 1.3")
            .dangerous()
            .with_custom_certificate_verifier(verifier)
            .with_no_client_auth(),
    )
}

#[derive(Clone)]
pub struct Transport {
    endpoint: ServerEndpoint,
    trust_store: Arc<RwLock<TrustStore>>,
    asynchronous: reqwest::Client,
    blocking: reqwest::blocking::Client,
}

impl Transport {
    pub fn new(
        endpoint: ServerEndpoint,
        trust_store: Arc<RwLock<TrustStore>>,
        timeout: Option<Duration>,
    ) -> Result<Self, ClientError> {
        let verifier = Arc::new(PinnedVerifier {
            server_id: endpoint.server_id.clone(),
            host: endpoint.host().to_owned(),
            store: trust_store.clone(),
            provider: Arc::new(rustls::crypto::ring::default_provider()),
        });
        let tls = tls_config(verifier);
        let timeout = timeout.unwrap_or(DEFAULT_TIMEOUT);
        let asynchronous = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .timeout(timeout)
            .tls_backend_preconfigured(tls.as_ref().clone())
            .build()?;
        let blocking = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .timeout(timeout)
            .tls_backend_preconfigured(tls.as_ref().clone())
            .build()?;
        Ok(Self {
            endpoint,
            trust_store,
            asynchronous,
            blocking,
        })
    }

    pub fn trust_store(&self) -> &Arc<RwLock<TrustStore>> {
        &self.trust_store
    }

    pub async fn request_json<B: Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<T, ClientError> {
        self.json_with_options(method, path, body, &[], &[]).await
    }

    pub async fn json_with_options<B: Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
        query: &[(&str, String)],
        headers: &[(&str, String)],
    ) -> Result<T, ClientError> {
        let mut request = self
            .asynchronous
            .request(method, self.endpoint.request_url(path)?)
            .query(query);
        for (key, value) in headers {
            request = request.header(*key, value);
        }
        if let Some(body) = body {
            request = request.json(body);
        }
        let mut response = request.send().await?;
        let status = response.status();
        const MAX_JSON: usize = 4 * 1024 * 1024;
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if bytes.len().saturating_add(chunk.len()) > MAX_JSON {
                return Err(ClientError::InvalidRangeResponse(
                    "Réponse JSON trop volumineuse".into(),
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        if !status.is_success() {
            return Err(api_response_error(status, &bytes));
        }
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub async fn get_json<T: DeserializeOwned>(&self, path: &str) -> Result<T, ClientError> {
        self.request_json::<(), T>(Method::GET, path, None).await
    }

    pub fn read_range_blocking(
        &self,
        path: &str,
        start: u64,
        buffer: &mut [u8],
        etag: Option<&str>,
    ) -> Result<RangeRead, ClientError> {
        self.read_range_with_headers(path, start, buffer, etag, &[])
    }

    pub fn read_media_range_blocking(
        &self,
        media_id: &str,
        session_id: &str,
        generation: u64,
        start: u64,
        buffer: &mut [u8],
        etag: Option<&str>,
    ) -> Result<RangeRead, ClientError> {
        if !is_opaque_id(media_id) || !is_opaque_id(session_id) {
            return Err(ClientError::InvalidRequestPath(
                "identifiant média ou session invalide".into(),
            ));
        }
        let path = format!("/api/v1/media/{media_id}/stream");
        let read = self.read_range_with_headers(
            &path,
            start,
            buffer,
            etag,
            &[
                ("X-Chill-Session", session_id.to_owned()),
                ("X-Chill-Generation", generation.to_string()),
            ],
        )?;
        if read.etag.is_none() {
            return Err(ClientError::Certificate(
                "le flux média ne fournit pas d'ETag".into(),
            ));
        }
        Ok(read)
    }

    fn read_range_with_headers(
        &self,
        path: &str,
        start: u64,
        buffer: &mut [u8],
        etag: Option<&str>,
        extra_headers: &[(&str, String)],
    ) -> Result<RangeRead, ClientError> {
        if buffer.is_empty() {
            return Ok(RangeRead {
                bytes_read: 0,
                total_size: None,
                etag: etag.map(str::to_owned),
            });
        }
        let end = start.saturating_add(buffer.len() as u64 - 1);
        let mut request = self
            .blocking
            .get(self.endpoint.request_url(path)?)
            .header(header::RANGE, format!("bytes={start}-{end}"));
        for (name, value) in extra_headers {
            request = request.header(*name, value);
        }
        if let Some(etag) = etag {
            request = request.header(header::IF_MATCH, etag);
        }
        let mut response = request.send()?;
        if response.status() == StatusCode::RANGE_NOT_SATISFIABLE {
            return Err(ClientError::Http(response.status()));
        }
        if response.status() != StatusCode::PARTIAL_CONTENT {
            return Err(ClientError::Http(response.status()));
        }
        let response_etag = response
            .headers()
            .get(header::ETAG)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        if etag.is_some() && response_etag.as_deref() != etag {
            return Err(ClientError::Certificate(
                "le contenu média a changé pendant la lecture".into(),
            ));
        }
        let content_range_value = response
            .headers()
            .get(header::CONTENT_RANGE)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| {
                ClientError::InvalidRangeResponse("en-tête Content-Range absent".into())
            })?;
        let content_range = validate_content_range(content_range_value, start, end)?;
        let expected_length = expected_range_body_length(content_range, response.content_length())?;
        let mut bytes_read = 0;
        while bytes_read < expected_length {
            let count = io::Read::read(&mut response, &mut buffer[bytes_read..expected_length])?;
            if count == 0 {
                break;
            }
            bytes_read += count;
        }
        let mut extra = [0_u8; 1];
        let has_extra = io::Read::read(&mut response, &mut extra)? != 0;
        validate_range_body_length(expected_length, bytes_read, has_extra)?;
        Ok(RangeRead {
            bytes_read,
            total_size: Some(content_range.total),
            etag: response_etag,
        })
    }
}

fn api_response_error(status: StatusCode, bytes: &[u8]) -> ClientError {
    #[derive(Deserialize)]
    struct Envelope {
        code: String,
    }
    match serde_json::from_slice::<Envelope>(bytes) {
        Ok(envelope)
            if envelope.code.len() <= 80
                && envelope
                    .code
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_') =>
        {
            ClientError::Api {
                status,
                code: envelope.code,
            }
        }
        _ => ClientError::Http(status),
    }
}

fn is_opaque_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RangeRead {
    pub bytes_read: usize,
    pub total_size: Option<u64>,
    pub etag: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ContentRange {
    start: u64,
    end: u64,
    total: u64,
}

impl ContentRange {
    fn length(self) -> Result<u64, ClientError> {
        self.end
            .checked_sub(self.start)
            .and_then(|length| length.checked_add(1))
            .ok_or_else(|| ClientError::InvalidRangeResponse("bornes incohérentes".into()))
    }
}

fn validate_content_range(
    value: &str,
    requested_start: u64,
    requested_end: u64,
) -> Result<ContentRange, ClientError> {
    let (unit, remainder) = value
        .split_once(' ')
        .ok_or_else(|| ClientError::InvalidRangeResponse("format Content-Range invalide".into()))?;
    if unit != "bytes" {
        return Err(ClientError::InvalidRangeResponse(
            "unité Content-Range différente de bytes".into(),
        ));
    }
    let (bounds, total) = remainder
        .split_once('/')
        .ok_or_else(|| ClientError::InvalidRangeResponse("total Content-Range absent".into()))?;
    if total.contains('/') || bounds == "*" || total == "*" {
        return Err(ClientError::InvalidRangeResponse(
            "plage ou total Content-Range indéterminé".into(),
        ));
    }
    let (start, end) = bounds
        .split_once('-')
        .ok_or_else(|| ClientError::InvalidRangeResponse("bornes Content-Range absentes".into()))?;
    let parsed = ContentRange {
        start: start.parse().map_err(|_| {
            ClientError::InvalidRangeResponse("début Content-Range invalide".into())
        })?,
        end: end
            .parse()
            .map_err(|_| ClientError::InvalidRangeResponse("fin Content-Range invalide".into()))?,
        total: total.parse().map_err(|_| {
            ClientError::InvalidRangeResponse("total Content-Range invalide".into())
        })?,
    };
    if parsed.start != requested_start
        || parsed.end < parsed.start
        || parsed.end > requested_end
        || parsed.end >= parsed.total
    {
        return Err(ClientError::InvalidRangeResponse(
            "plage retournée différente de la plage demandée".into(),
        ));
    }
    parsed.length()?;
    Ok(parsed)
}

fn expected_range_body_length(
    range: ContentRange,
    declared_length: Option<u64>,
) -> Result<usize, ClientError> {
    let expected = range.length()?;
    if declared_length.is_some_and(|length| length != expected) {
        return Err(ClientError::InvalidRangeResponse(
            "Content-Length ne correspond pas à Content-Range".into(),
        ));
    }
    usize::try_from(expected).map_err(|_| {
        ClientError::InvalidRangeResponse("plage trop grande pour cette plateforme".into())
    })
}

fn validate_range_body_length(
    expected: usize,
    observed: usize,
    has_extra: bool,
) -> Result<(), ClientError> {
    if observed != expected {
        return Err(ClientError::InvalidRangeResponse(
            "corps tronqué par rapport à Content-Range".into(),
        ));
    }
    if has_extra {
        return Err(ClientError::InvalidRangeResponse(
            "corps plus long que Content-Range".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn pinned_transport_can_be_constructed_without_http_or_approval_bypass() {
        let endpoint = super::ServerEndpoint::parse("fixture", "https://localhost:19443").unwrap();
        let trust = std::sync::Arc::new(std::sync::RwLock::new(super::TrustStore::default()));
        assert!(super::Transport::new(endpoint, trust, None).is_ok());
    }
    use super::*;
    #[test]
    fn api_errors_keep_only_bounded_safe_codes_not_server_details() {
        assert!(
            matches!(api_response_error(StatusCode::NOT_FOUND, br#"{"code":"MEDIA_NOT_FOUND","message":"/nas/private"}"#), ClientError::Api {code, ..} if code == "MEDIA_NOT_FOUND")
        );
        assert!(matches!(
            api_response_error(StatusCode::BAD_REQUEST, br#"{"code":"/nas/private"}"#),
            ClientError::Http(_)
        ));
    }

    fn certificate_for(host: &str) -> PresentedCertificate {
        let generated = rcgen::generate_simple_self_signed(vec![host.to_owned()]).unwrap();
        PresentedCertificate::validate(generated.cert.der(), host, SystemTime::now()).unwrap()
    }

    #[test]
    fn wildcard_dns_matches_one_label_only() {
        assert!(dns_name_matches("*.lan", "pi.lan"));
        assert!(!dns_name_matches("*.lan", "video.pi.lan"));
        assert!(!dns_name_matches("*.lan", "lan"));
    }

    #[test]
    fn endpoint_rejects_http_and_path_escape() {
        assert!(ServerEndpoint::parse("id", "http://pi.lan").is_err());
        let endpoint = ServerEndpoint::parse("id", "https://pi.lan:8443").unwrap();
        assert!(endpoint.request_url("/api/v1/server").is_ok());
        assert!(endpoint.request_url("//attacker.invalid/x").is_err());
        assert!(endpoint.request_url("/api/../admin").is_err());
    }

    #[test]
    fn trust_store_round_trip_contains_public_material_only() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("trust.json");
        let mut store = TrustStore::default();
        store.approve_initial(
            "server",
            "pi.lan",
            PresentedCertificate {
                der: vec![1, 2],
                spki_sha256: [7; 32],
                not_before_unix: 1,
                not_after_unix: 2,
                subject: "CN=pi".into(),
            },
        );
        store.save(&path).unwrap();
        assert_eq!(TrustStore::load(&path).unwrap(), store);
        let encoded = fs::read_to_string(path).unwrap();
        assert!(!encoded.contains("private"));
    }

    #[test]
    fn content_range_must_match_requested_interval() {
        assert_eq!(
            validate_content_range("bytes 10-19/42", 10, 19).unwrap(),
            ContentRange {
                start: 10,
                end: 19,
                total: 42
            }
        );
        assert!(validate_content_range("items 10-19/42", 10, 19).is_err());
        assert!(validate_content_range("bytes 9-19/42", 10, 19).is_err());
        assert!(validate_content_range("bytes 10-20/42", 10, 19).is_err());
        assert!(validate_content_range("bytes 10-9/42", 10, 19).is_err());
        assert!(validate_content_range("bytes 10-19/19", 10, 19).is_err());
        assert!(validate_content_range("bytes */42", 10, 19).is_err());
        assert!(validate_content_range("bytes 10-19/*", 10, 19).is_err());
    }

    #[test]
    fn content_range_body_must_be_exactly_the_declared_length() {
        let range = ContentRange {
            start: 10,
            end: 19,
            total: 42,
        };
        assert_eq!(expected_range_body_length(range, Some(10)).unwrap(), 10);
        assert!(expected_range_body_length(range, Some(9)).is_err());
        assert!(validate_range_body_length(10, 9, false).is_err());
        assert!(validate_range_body_length(10, 10, true).is_err());
        assert!(validate_range_body_length(10, 10, false).is_ok());
    }

    #[test]
    fn opaque_ids_cannot_inject_paths_or_headers() {
        assert!(is_opaque_id("0199f21e-lease_1"));
        assert!(!is_opaque_id("../../video"));
        assert!(!is_opaque_id("lease\r\nX-Evil: yes"));
    }

    #[test]
    fn certificate_requires_matching_san_and_current_validity() {
        let generated = rcgen::generate_simple_self_signed(vec!["pi.lan".to_owned()]).unwrap();
        assert!(
            PresentedCertificate::validate(generated.cert.der(), "pi.lan", SystemTime::now())
                .is_ok()
        );
        assert!(
            PresentedCertificate::validate(generated.cert.der(), "other.lan", SystemTime::now())
                .is_err()
        );
        assert!(
            PresentedCertificate::validate(
                generated.cert.der(),
                "pi.lan",
                UNIX_EPOCH + Duration::from_secs(1)
            )
            .is_err()
        );
    }

    #[test]
    fn key_change_before_expiry_is_refused() {
        let first = certificate_for("pi.lan");
        let second = certificate_for("pi.lan");
        let mut store = TrustStore::default();
        store.approve_initial("server", "pi.lan", first);
        let store = Arc::new(RwLock::new(store));
        let verifier = PinnedVerifier {
            server_id: "server".into(),
            host: "pi.lan".into(),
            store: store.clone(),
            provider: Arc::new(rustls::crypto::ring::default_provider()),
        };
        let now = unix_seconds(SystemTime::now()) as u64;
        let result = verifier.verify_server_cert(
            &CertificateDer::from(second.der),
            &[],
            &ServerName::try_from("pi.lan").unwrap(),
            &[],
            UnixTime::since_unix_epoch(Duration::from_secs(now)),
        );
        assert!(result.is_err());
        assert!(store.read().unwrap().events.is_empty());
    }

    #[test]
    fn key_change_after_known_expiry_is_recorded() {
        let first = certificate_for("pi.lan");
        let second = certificate_for("pi.lan");
        let mut store = TrustStore::default();
        store.approve_initial("server", "pi.lan", first);
        store
            .servers
            .get_mut("server")
            .unwrap()
            .certificate_not_after_unix = 1;
        let expected_pin = second.spki_sha256;
        let store = Arc::new(RwLock::new(store));
        let verifier = PinnedVerifier {
            server_id: "server".into(),
            host: "pi.lan".into(),
            store: store.clone(),
            provider: Arc::new(rustls::crypto::ring::default_provider()),
        };
        let now = unix_seconds(SystemTime::now()) as u64;
        verifier
            .verify_server_cert(
                &CertificateDer::from(second.der),
                &[],
                &ServerName::try_from("pi.lan").unwrap(),
                &[],
                UnixTime::since_unix_epoch(Duration::from_secs(now)),
            )
            .unwrap();
        let updated = store.read().unwrap();
        assert_eq!(updated.servers["server"].spki_sha256, expected_pin);
        assert_eq!(
            updated.events.last().unwrap().kind,
            TrustEventKind::KeyReplacedAfterKnownExpiry
        );
    }
}
