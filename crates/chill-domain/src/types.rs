use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        Self { audio_language: "en".into(), subtitle_language: "en".into(), autoplay_seconds: Some(10) }
    }
}
impl Preferences {
    pub fn validate(&self) -> Result<(), String> {
        if self.audio_language.is_empty() || self.audio_language.len() > 32 || self.subtitle_language.len() > 32
            || self.autoplay_seconds.is_some_and(|s| ![0, 5, 10, 20, 30].contains(&s)) {
            return Err("Préférences invalides.".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    fn default() -> Self { Self { kind: "standalone".into(), id: None } }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lease {
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
pub struct Progress {
    pub session_id: String,
    pub generation: u64,
    pub media_id: String,
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
}

pub fn valid_speed(speed: f64) -> bool {
    [0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0].contains(&speed)
}
