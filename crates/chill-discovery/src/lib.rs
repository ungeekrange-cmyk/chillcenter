//! Bounded, local-network-only mDNS discovery for Chill Center HTTPS servers.
//!
//! Discovery records are untrusted hints: callers must still perform their normal
//! HTTPS certificate/SAN validation and pinning before connecting.

use mdns_sd::{RecvTimeoutError, ResolvedService, ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fmt,
    net::{IpAddr, SocketAddr},
    time::{Duration, Instant},
};
use uuid::Uuid;

pub const SERVICE_TYPE: &str = "_chillcenter._tcp.local.";
const MAX_SERVERS: usize = 16;
const MAX_ADDRESSES: usize = 8;
const MAX_NAME_BYTES: usize = 96;
const MAX_VERSION_BYTES: usize = 64;
const MAX_BROWSE: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscoveredServer {
    pub server_id: String,
    pub name: String,
    /// Bare authorities, never URLs: `192.168.1.4:8443` or `[fd00::4]:8443`.
    pub addresses: Vec<String>,
    pub version: String,
}

#[derive(Debug)]
pub enum DiscoveryError {
    Mdns(mdns_sd::Error),
    InvalidServerId,
    InvalidVersion,
    InvalidListenAddress,
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mdns(error) => write!(f, "mDNS unavailable: {error}"),
            Self::InvalidServerId => f.write_str("invalid discovery server id"),
            Self::InvalidVersion => f.write_str("invalid discovery version"),
            Self::InvalidListenAddress => f.write_str("discovery requires a LAN unicast listener"),
        }
    }
}

impl std::error::Error for DiscoveryError {}

impl From<mdns_sd::Error> for DiscoveryError {
    fn from(value: mdns_sd::Error) -> Self {
        Self::Mdns(value)
    }
}

/// Browse mDNS for at most `timeout` (and never more than five seconds).
/// Malformed, public, loopback, and oversized records are ignored.
pub fn browse(timeout: Duration) -> Result<Vec<DiscoveredServer>, DiscoveryError> {
    let daemon = ServiceDaemon::new()?;
    let receiver = daemon.browse(SERVICE_TYPE)?;
    let guard = BrowseGuard { daemon: &daemon };
    let deadline = Instant::now() + timeout.min(MAX_BROWSE);
    let mut records = Vec::new();
    let mut ids = BTreeSet::new();

    while records.len() < MAX_SERVERS {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        match receiver.recv_timeout(remaining) {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                if let Some(record) = record_from_info(&info)
                    && ids.insert(record.server_id.clone())
                {
                    records.push(record);
                }
            }
            Ok(_) => {}
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => break,
        }
    }
    drop(guard);
    Ok(records)
}

struct BrowseGuard<'a> {
    daemon: &'a ServiceDaemon,
}
impl Drop for BrowseGuard<'_> {
    fn drop(&mut self) {
        let _ = self.daemon.stop_browse(SERVICE_TYPE);
        let _ = self.daemon.shutdown();
    }
}

/// A registered LAN announcement. Dropping it unregisters and shuts down its daemon.
pub struct Advertiser {
    daemon: ServiceDaemon,
    fullname: String,
}

impl Advertiser {
    /// Starts an announcement for the already-configured HTTPS socket only.
    pub fn start(
        server_id: &str,
        version: &str,
        listen: SocketAddr,
    ) -> Result<Self, DiscoveryError> {
        let server_id = Uuid::parse_str(server_id).map_err(|_| DiscoveryError::InvalidServerId)?;
        if !bounded_text(version, MAX_VERSION_BYTES) {
            return Err(DiscoveryError::InvalidVersion);
        }
        if !is_advertisable_address(listen.ip()) || listen.port() == 0 {
            return Err(DiscoveryError::InvalidListenAddress);
        }
        let short = server_id.simple().to_string();
        let instance = format!("ChillCenter-{}", &short[..8]);
        let hostname = format!("chillcenter-{server_id}.local.");
        let server_id_text = server_id.as_hyphenated().to_string();
        let properties = [
            ("protocol", "https"),
            ("api", "2026-10-14"),
            ("server_id", server_id_text.as_str()),
            ("version", version),
        ];
        // ServiceInfo owns/copies the supplied TXT values.
        let info = ServiceInfo::new(
            SERVICE_TYPE,
            &instance,
            &hostname,
            listen.ip(),
            listen.port(),
            &properties[..],
        )?;
        let fullname = info.get_fullname().to_owned();
        let daemon = ServiceDaemon::new()?;
        daemon.register(info)?;
        Ok(Self { daemon, fullname })
    }
}

impl Drop for Advertiser {
    fn drop(&mut self) {
        let _ = self.daemon.unregister(&self.fullname);
        let _ = self.daemon.shutdown();
    }
}

fn record_from_info(info: &ResolvedService) -> Option<DiscoveredServer> {
    let property = |key| info.get_property_val_str(key);
    if property("protocol")? != "https" || property("api")? != "2026-10-14" {
        return None;
    }
    record_from_parts(
        property("server_id")?,
        instance_name(info.get_fullname())?,
        property("version")?,
        info.get_addresses().iter().map(|ip| ip.to_ip_addr()),
        info.get_port(),
    )
}

fn instance_name(fullname: &str) -> Option<&str> {
    let name = fullname.split('.').next()?;
    bounded_text(name, MAX_NAME_BYTES).then_some(name)
}

fn record_from_parts(
    server_id: &str,
    name: &str,
    version: &str,
    addresses: impl IntoIterator<Item = IpAddr>,
    port: u16,
) -> Option<DiscoveredServer> {
    if Uuid::parse_str(server_id).is_err()
        || !bounded_text(name, MAX_NAME_BYTES)
        || !bounded_text(version, MAX_VERSION_BYTES)
        || port == 0
    {
        return None;
    }
    let addresses: Vec<_> = addresses
        .into_iter()
        .filter(|ip| is_advertisable_address(*ip))
        .map(|ip| SocketAddr::new(ip, port).to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .take(MAX_ADDRESSES)
        .collect();
    (!addresses.is_empty()).then(|| DiscoveredServer {
        server_id: server_id.to_owned(),
        name: name.to_owned(),
        addresses,
        version: version.to_owned(),
    })
}

fn bounded_text(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() || byte == b' ')
}

fn is_advertisable_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(ip) => ip.is_private() || ip.is_link_local(),
        // Link-local IPv6 lacks a scope in an mDNS A/AAAA record. Omitting it is
        // safer than constructing an authority that cannot be connected to.
        IpAddr::V6(ip) => ip.is_unique_local(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const ID: &str = "f47ac10b-58cc-4372-a567-0e02b2c3d479";

    #[test]
    fn accepts_only_expected_lan_record() {
        let result = record_from_parts(
            ID,
            "ChillCenter-f47ac10b",
            "0.7.1",
            ["192.168.1.44".parse().unwrap(), "fd12::44".parse().unwrap()],
            8443,
        )
        .unwrap();
        assert_eq!(result.addresses, ["192.168.1.44:8443", "[fd12::44]:8443"]);
    }

    #[test]
    fn rejects_bad_txt_port_and_non_lan_addresses() {
        assert!(
            record_from_parts(
                "not-a-uuid",
                "name",
                "0.7.1",
                ["192.168.1.1".parse().unwrap()],
                443
            )
            .is_none()
        );
        assert!(
            record_from_parts(ID, "name", "0.7.1", ["192.168.1.1".parse().unwrap()], 0).is_none()
        );
        assert!(
            record_from_parts(
                ID,
                "name",
                "0.7.1",
                [
                    "8.8.8.8".parse().unwrap(),
                    "127.0.0.1".parse().unwrap(),
                    "fe80::1".parse().unwrap()
                ],
                443
            )
            .is_none()
        );
    }

    #[test]
    fn limits_addresses_and_publisher_refuses_loopback() {
        let ips = (1..=12).map(|last| IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, last)));
        assert_eq!(
            record_from_parts(ID, "name", "0.7.1", ips, 8443)
                .unwrap()
                .addresses
                .len(),
            MAX_ADDRESSES
        );
        assert!(matches!(
            Advertiser::start(ID, "0.7.1", "127.0.0.1:8443".parse().unwrap()),
            Err(DiscoveryError::InvalidListenAddress)
        ));
    }
}
