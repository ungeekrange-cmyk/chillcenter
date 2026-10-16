use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Preferences {
    pub audio_language: String,
    pub subtitle_language: String,
    pub autoplay_seconds: Option<u8>,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            audio_language: "en".into(),
            subtitle_language: "en".into(),
            // Historical API default. The current desktop explicitly saves 20
            // when creating a profile; old clients retain their old semantics.
            autoplay_seconds: Some(10),
        }
    }
}
impl Preferences {
    pub fn validate(&self) -> Result<(), String> {
        if self.audio_language.is_empty()
            || self.audio_language.len() > 32
            || self.subtitle_language.len() > 32
            || self
                .autoplay_seconds
                .is_some_and(|s| ![0, 5, 10, 15, 20, 30].contains(&s))
        {
            return Err("Préférences invalides.".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersonalState {
    pub position_ms: u64,
    pub final_watched_ms: u64,
    pub watched: bool,
    pub rating: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackContext {
    pub kind: String,
    pub id: Option<String>,
}
impl Default for PlaybackContext {
    fn default() -> Self {
        Self {
            kind: "standalone".into(),
            id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lease {
    pub server_epoch: String,
    pub session_id: String,
    pub profile_id: String,
    pub device_id: String,
    pub media_id: String,
    pub media_revision: u64,
    pub generation: u64,
    pub remaining_ms: u64,
    pub transferring: bool,
    pub context: PlaybackContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcquirePlayback {
    pub media_id: String,
    pub device_id: String,
    #[serde(default)]
    pub context: PlaybackContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackGrant {
    pub lease: Lease,
    pub personal: PersonalState,
}

/// A validated successor requested for a lease held by another device.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransferProposal {
    pub target: AcquirePlayback,
}

/// Short-lived confirmation capability.  It is bound to the active lease
/// generation rather than being a reusable playback credential.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferOffer {
    pub token: String,
    pub active: Lease,
    pub expires_in_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransferConfirmation {
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransferPhase {
    AwaitingAck,
    Activated,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferStatus {
    pub token: String,
    pub phase: TransferPhase,
    pub remaining_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lease: Option<Lease>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub personal: Option<PersonalState>,
}

/// The sole final progress event accepted from the old generation while a
/// confirmed transfer is waiting for it to pause.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransferAck {
    pub token: String,
    pub lease: PlaybackControl,
    pub progress: Progress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackInfo {
    pub media: chill_catalog::model::Media,
    pub preferences: Preferences,
    pub speed: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaybackControl {
    pub session_id: String,
    pub generation: u64,
    pub media_id: String,
    pub media_revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleasePlayback {
    pub lease: PlaybackControl,
    pub progress: Option<Progress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Progress {
    pub session_id: String,
    pub generation: u64,
    pub media_id: String,
    pub media_revision: u64,
    pub sequence: u64,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub speed: f64,
    pub final_elapsed_ms: u64,
    pub playing: bool,
    pub buffering: bool,
    pub seek: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextVideo {
    pub media: Option<chill_catalog::model::Media>,
    pub new_season: bool,
    pub available: bool,
    /// Additive response hint: no successor could be ordered, NOT a confirmed end.
    #[serde(default, skip_serializing_if = "is_false")]
    pub order_unknown: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

pub fn valid_speed(speed: f64) -> bool {
    [0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0].contains(&speed)
}

/// Start of the final tenth, without overflowing for long durations.
pub fn final_tenth_start(duration_ms: u64) -> u64 {
    (duration_ms / 10) * 9 + (duration_ms % 10) * 9 / 10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_video_legacy_response_remains_compatible_and_hint_is_additive() {
        let legacy = serde_json::json!({"media":null,"new_season":false,"available":false});
        let mut response: NextVideo = serde_json::from_value(legacy.clone()).unwrap();
        assert!(!response.order_unknown);
        assert_eq!(serde_json::to_value(&response).unwrap(), legacy);
        response.order_unknown = true;
        assert_eq!(
            serde_json::to_value(&response).unwrap()["order_unknown"],
            true
        );
    }

    #[test]
    fn final_tenth_boundary_handles_large_durations() {
        for duration in [0, 9, 10, 101, i64::MAX as u64, u64::MAX] {
            assert_eq!(
                final_tenth_start(duration) as u128,
                duration as u128 * 9 / 10
            );
        }
    }
}
