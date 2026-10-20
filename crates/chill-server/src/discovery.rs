//! Server-side wiring for the optional LAN mDNS announcement.
//!
//! Failure to acquire UDP/5353 never prevents HTTPS from starting; manual
//! connection remains available and the client still validates TLS itself.
pub use chill_discovery::{Advertiser, DiscoveryError};

use std::net::SocketAddr;

pub fn start(server_id: &str, version: &str, listen: SocketAddr) -> Option<Advertiser> {
    match Advertiser::start(server_id, version, listen) {
        Ok(advertiser) => Some(advertiser),
        Err(error) => {
            eprintln!("MDNS_ANNOUNCEMENT_UNAVAILABLE: {error}");
            None
        }
    }
}
