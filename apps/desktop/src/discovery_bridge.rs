//! Native-only Bonjour bridge. Discovery data is deliberately only a connection
//! hint: certificate approval, SAN/SPKI checks, and API identity remain in
//! `UiClient::Connect` exactly as they are for a manually entered address.

use chill_client::ui::UiError;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum DiscoveryCommand {
    Browse,
}

pub fn unavailable() -> UiError {
    UiError {
        message: "Recherche locale indisponible.".into(),
        status: None,
        code: "DISCOVERY_UNAVAILABLE".into(),
    }
}

pub async fn browse(singleflight: Arc<tokio::sync::Semaphore>) -> Result<Value, UiError> {
    // Refuse a concurrent click rather than queueing background mDNS work.
    let _permit = singleflight.try_acquire_owned().map_err(|_| UiError {
        message: "Une recherche locale est déjà en cours.".into(),
        status: None,
        code: "DISCOVERY_BUSY".into(),
    })?;
    let servers =
        tauri::async_runtime::spawn_blocking(|| chill_discovery::browse(Duration::from_secs(4)))
            .await
            .map_err(|_| unavailable())
            .and_then(|value| value.map_err(|_| unavailable()))?;
    Ok(json!({"servers":servers}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_browse_is_an_accepted_command() {
        assert!(serde_json::from_value::<DiscoveryCommand>(json!({"action":"browse"})).is_ok());
        assert!(
            serde_json::from_value::<DiscoveryCommand>(
                json!({"action":"browse","address":"https://bad"})
            )
            .is_err()
        );
        assert!(
            serde_json::from_value::<DiscoveryCommand>(
                json!({"action":"connect","address":"https://bad"})
            )
            .is_err()
        );
    }
}
