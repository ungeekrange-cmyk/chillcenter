use chill_client::{ServerEndpoint, Transport, TrustStore, inspect_certificate};
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};
fn main() {
    let address = std::env::args().nth(1).expect("fixture address");
    let certificate = inspect_certificate(&address, Duration::from_secs(5)).expect("TLS probe");
    let endpoint = ServerEndpoint::parse("test", &address).unwrap();
    let mut trust = TrustStore::default();
    trust.approve_initial("test", endpoint.host(), certificate);
    let transport = Transport::new(endpoint, Arc::new(RwLock::new(trust)), None);
    match transport {
        Ok(_) => println!("TRANSPORT_BUILD_OK"),
        Err(error) => {
            eprintln!("TRANSPORT_BUILD_FAILED {error}");
            std::process::exit(1);
        }
    }
}
