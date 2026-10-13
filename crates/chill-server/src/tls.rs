//! Calendar-based renewal with stable service key and explicitly configured SANs.
use crate::config::{ServerConfig, atomic_private_write};
use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, PublicKeyData};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use time::{Date, Month, OffsetDateTime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateSchedule {
    pub server_id: String,
    pub issued_at: i64,
    pub renew_at: i64,
    pub expires_at: i64,
    pub names: Vec<String>,
}

pub struct Certificates {
    pub certificate_path: PathBuf,
    pub key_path: PathBuf,
    pub schedule: CertificateSchedule,
}

pub async fn load(
    certificates: &Certificates,
) -> Result<axum_server::tls_rustls::RustlsConfig, String> {
    // Workspace features can include both providers through the native client.
    // Select the server's provider deliberately, never rely on feature inference.
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    axum_server::tls_rustls::RustlsConfig::from_pem_file(
        &certificates.certificate_path,
        &certificates.key_path,
    )
    .await
    .map_err(|_| "Chargement HTTPS impossible.".to_string())
}

pub fn ensure(config: &ServerConfig, now: OffsetDateTime) -> Result<Certificates, String> {
    config.validate()?;
    if now.year() < 2024 {
        return Err("Horloge non initialisée : synchronisez Linux avant de démarrer HTTPS.".into());
    }
    let directory = config.data_dir.join("tls");
    fs::create_dir_all(&directory).map_err(|_| "Création du dossier TLS impossible.")?;
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
        .map_err(|_| "Protection du dossier TLS impossible.")?;
    let certificate_path = directory.join("certificate.pem");
    let key_path = directory.join("service-key.pem");
    let schedule_path = directory.join("schedule.json");
    let prior = match fs::read(&schedule_path) {
        Ok(bytes) => Some(
            serde_json::from_slice::<CertificateSchedule>(&bytes)
                .map_err(|_| "Calendrier TLS invalide.")?,
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => return Err("Calendrier TLS inaccessible.".into()),
    };
    if prior
        .as_ref()
        .is_some_and(|schedule| now.unix_timestamp() < schedule.issued_at - 300)
    {
        return Err("Horloge antérieure au dernier certificat : synchronisation requise.".into());
    }
    let key =
        match fs::read_to_string(&key_path) {
            Ok(pem) => KeyPair::from_pem(&pem).map_err(|_| "Clé TLS invalide.")?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && prior.is_none() => {
                let key = KeyPair::generate().map_err(|_| "Génération de clé TLS impossible.")?;
                atomic_private_write(&key_path, key.serialize_pem().as_bytes())?;
                key
            }
            // Never silently replace a missing established key.
            Err(_) => return Err(
                "Clé TLS existante inaccessible ; restauration ou remplacement explicite requis."
                    .into(),
            ),
        };
    fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600))
        .map_err(|_| "Protection de clé TLS impossible.")?;
    let schedule = if prior.as_ref().is_some_and(|p| {
        p.renew_at > now.unix_timestamp()
            && p.names == config.tls_names
            && persisted_certificate_matches(&certificate_path, &key, p, now)
    }) {
        prior.unwrap()
    } else {
        let mut params =
            CertificateParams::new(config.tls_names.clone()).map_err(|_| "Noms TLS invalides.")?;
        params.distinguished_name = DistinguishedName::new();
        params
            .distinguished_name
            .push(DnType::CommonName, "Chill Center");
        params.not_before = now - time::Duration::minutes(5);
        params.not_after = add_months(now, 18)?;
        let certificate = params
            .self_signed(&key)
            .map_err(|_| "Génération du certificat impossible.")?;
        let schedule = CertificateSchedule {
            server_id: prior
                .map(|p| p.server_id)
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            issued_at: now.unix_timestamp(),
            renew_at: add_months(now, 12)?.unix_timestamp(),
            expires_at: params.not_after.unix_timestamp(),
            names: config.tls_names.clone(),
        };
        atomic_private_write(&certificate_path, certificate.pem().as_bytes())?;
        atomic_private_write(
            &schedule_path,
            &serde_json::to_vec_pretty(&schedule).map_err(|_| "Calendrier TLS invalide.")?,
        )?;
        schedule
    };
    Ok(Certificates {
        certificate_path,
        key_path,
        schedule,
    })
}

fn persisted_certificate_matches(
    path: &std::path::Path,
    key: &KeyPair,
    schedule: &CertificateSchedule,
    now: OffsetDateTime,
) -> bool {
    use x509_parser::{extensions::GeneralName, prelude::*};
    let Ok(bytes) = fs::read(path) else {
        return false;
    };
    if bytes.len() > 64 * 1024 {
        return false;
    }
    let Ok((_, pem)) = x509_parser::pem::parse_x509_pem(&bytes) else {
        return false;
    };
    let Ok((_, cert)) = X509Certificate::from_der(&pem.contents) else {
        return false;
    };
    let Ok(time) = ASN1Time::from_timestamp(now.unix_timestamp()) else {
        return false;
    };
    if !cert.validity().is_valid_at(time)
        || cert.verify_signature(None).is_err()
        || cert.public_key().raw != key.subject_public_key_info().as_slice()
        || cert.validity().not_after.timestamp() != schedule.expires_at
        || cert.validity().not_before.timestamp() != schedule.issued_at - 300
    {
        return false;
    }
    let Ok(Some(san)) = cert.subject_alternative_name() else {
        return false;
    };
    san.value.general_names.len() == schedule.names.len()
        && schedule.names.iter().all(|name| {
            san.value.general_names.iter().any(|entry| {
                match (entry, name.parse::<std::net::IpAddr>()) {
                    (GeneralName::DNSName(dns), Err(_)) => dns.eq_ignore_ascii_case(name),
                    (GeneralName::IPAddress(bytes), Ok(std::net::IpAddr::V4(ip))) => {
                        *bytes == ip.octets()
                    }
                    (GeneralName::IPAddress(bytes), Ok(std::net::IpAddr::V6(ip))) => {
                        *bytes == ip.octets()
                    }
                    _ => false,
                }
            })
        })
}

fn add_months(now: OffsetDateTime, months: i32) -> Result<OffsetDateTime, String> {
    let total = now
        .year()
        .checked_mul(12)
        .and_then(|y| y.checked_add(now.month() as i32 - 1 + months))
        .ok_or("Date TLS invalide.")?;
    let year = total.div_euclid(12);
    let month =
        Month::try_from((total.rem_euclid(12) + 1) as u8).map_err(|_| "Date TLS invalide.")?;
    for day in (1..=now.day()).rev() {
        if let Ok(date) = Date::from_calendar_date(year, month, day) {
            return Ok(now.replace_date(date));
        }
    }
    Err("Date TLS invalide.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cold_boot_clock_and_corrupted_restored_certificate_are_not_reused() {
        let dir = tempfile::tempdir().unwrap();
        let config = ServerConfig {
            data_dir: dir.path().to_owned(),
            ..ServerConfig::default()
        };
        assert!(ensure(&config, OffsetDateTime::UNIX_EPOCH).is_err());
        let now = OffsetDateTime::now_utc();
        let first = ensure(&config, now).unwrap();
        let key = fs::read(&first.key_path).unwrap();
        assert!(ensure(&config, now - time::Duration::days(1)).is_err());
        fs::write(&first.certificate_path, b"corrupt restored certificate").unwrap();
        let repaired = ensure(&config, now + time::Duration::minutes(1)).unwrap();
        assert_eq!(key, fs::read(&repaired.key_path).unwrap());
        assert_eq!(first.schedule.server_id, repaired.schedule.server_id);
        load_certificate_for_test(&repaired.certificate_path);
    }
    fn load_certificate_for_test(path: &std::path::Path) {
        let bytes = fs::read(path).unwrap();
        x509_parser::pem::parse_x509_pem(&bytes).unwrap();
    }
    #[tokio::test]
    async fn https_config_loads_with_workspace_crypto_features() {
        let dir = tempfile::tempdir().unwrap();
        let config = ServerConfig {
            data_dir: dir.path().to_owned(),
            ..ServerConfig::default()
        };
        let certificates = ensure(&config, OffsetDateTime::now_utc()).unwrap();
        load(&certificates).await.unwrap();
    }
    #[test]
    fn renewal_keeps_key_and_identity_but_changes_certificate() {
        let dir = tempfile::tempdir().unwrap();
        let config = ServerConfig {
            data_dir: dir.path().to_owned(),
            ..ServerConfig::default()
        };
        let first_time = Date::from_calendar_date(2026, Month::August, 31)
            .unwrap()
            .midnight()
            .assume_utc();
        let first = ensure(&config, first_time).unwrap();
        let key = fs::read(&first.key_path).unwrap();
        let certificate = fs::read(&first.certificate_path).unwrap();
        assert_eq!(
            OffsetDateTime::from_unix_timestamp(first.schedule.expires_at)
                .unwrap()
                .date(),
            Date::from_calendar_date(2028, Month::February, 29).unwrap()
        );
        let reused = ensure(&config, first_time + time::Duration::days(1)).unwrap();
        assert_eq!(fs::read(&reused.certificate_path).unwrap(), certificate);
        let renewed = ensure(&config, add_months(first_time, 12).unwrap()).unwrap();
        assert_eq!(fs::read(&renewed.key_path).unwrap(), key);
        assert_ne!(fs::read(&renewed.certificate_path).unwrap(), certificate);
        assert_eq!(first.schedule.server_id, renewed.schedule.server_id);
    }

    #[test]
    fn changed_address_reissues_with_same_key() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = ServerConfig {
            data_dir: dir.path().to_owned(),
            ..ServerConfig::default()
        };
        let first = ensure(&config, OffsetDateTime::now_utc()).unwrap();
        let key = fs::read(&first.key_path).unwrap();
        config.tls_names = vec!["another-server.home".into(), "10.0.0.12".into()];
        let changed = ensure(&config, OffsetDateTime::now_utc()).unwrap();
        assert_eq!(changed.schedule.names, config.tls_names);
        assert_eq!(fs::read(&changed.key_path).unwrap(), key);
        assert_eq!(changed.schedule.server_id, first.schedule.server_id);
    }
}
