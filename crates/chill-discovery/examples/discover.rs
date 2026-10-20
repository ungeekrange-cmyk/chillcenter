//! Manual LAN verification probe; it never connects to a discovered endpoint.
//!
//! Run on the Mac after a server announcement is deployed:
//! `cargo run -p chill-discovery --example discover --locked`

use std::time::Duration;

fn main() {
    match chill_discovery::browse(Duration::from_secs(4)) {
        Ok(servers) if servers.is_empty() => {
            println!("No Chill Center server discovered in 4 seconds.")
        }
        Ok(servers) => {
            for server in servers {
                println!(
                    "server_id={} name={} version={} candidates={}",
                    server.server_id,
                    server.name,
                    server.version,
                    server.addresses.join(",")
                );
            }
        }
        Err(error) => {
            eprintln!("Discovery unavailable: {error}");
            std::process::exit(1);
        }
    }
}
