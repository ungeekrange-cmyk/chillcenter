//! Typed playback operations over the same pinned transport as the catalogue.
use crate::{ClientError, Transport, is_opaque_id};
pub use chill_domain::{
    AcquirePlayback, Lease, NextVideo, PersonalState, PlaybackContext, PlaybackControl,
    PlaybackGrant, PlaybackInfo, Progress, ReleasePlayback,
};
use reqwest::Method;
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ExternalSubtitle {
    pub id: String,
    pub title: String,
    pub language: Option<String>,
    pub ambiguous: bool,
    pub unavailable: bool,
}
#[derive(serde::Deserialize)]
struct SubtitleList {
    items: Vec<ExternalSubtitle>,
}
fn subtitle_path(profile: &str, lease: &Lease) -> Result<String, ClientError> {
    profile_path(profile, "subtitles")?;
    if !is_opaque_id(&lease.media_id) {
        return Err(ClientError::InvalidRequestPath("média invalide".into()));
    }
    Ok(format!(
        "/api/v1/profiles/{profile}/media/{}/subtitles",
        lease.media_id
    ))
}
fn subtitle_query(lease: &Lease) -> [(&'static str, String); 2] {
    [
        ("session", lease.session_id.clone()),
        ("generation", lease.generation.to_string()),
    ]
}
fn profile_path(profile: &str, operation: &str) -> Result<String, ClientError> {
    if !is_opaque_id(profile) {
        return Err(ClientError::InvalidRequestPath("profil invalide".into()));
    }
    Ok(format!("/api/v1/profiles/{profile}/playback/{operation}"))
}
pub fn valid_speed_value(speed: f64) -> bool {
    chill_domain::valid_speed(speed)
}
impl Transport {
    pub async fn external_subtitles(
        &self,
        profile: &str,
        lease: &Lease,
    ) -> Result<Vec<ExternalSubtitle>, ClientError> {
        let list: SubtitleList = self
            .json_with_options::<(), _>(
                Method::GET,
                &subtitle_path(profile, lease)?,
                None,
                &subtitle_query(lease),
                &[],
            )
            .await?;
        if list.items.len() > 64 {
            return Err(ClientError::InvalidRangeResponse(
                "Trop de sous-titres".into(),
            ));
        }
        Ok(list.items)
    }
    pub async fn download_subtitle(
        &self,
        profile: &str,
        lease: &Lease,
        id: &str,
    ) -> Result<Vec<u8>, ClientError> {
        if id.len() != 20
            || !id.starts_with("sub-")
            || !id[4..].bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(ClientError::InvalidRequestPath(
                "sous-titre invalide".into(),
            ));
        }
        let bytes = self
            .get_binary(
                &format!("{}/{id}", subtitle_path(profile, lease)?),
                &subtitle_query(lease),
                16 * 1024 * 1024,
            )
            .await?;
        if bytes.contains(&0) || std::str::from_utf8(&bytes).is_err() {
            return Err(ClientError::InvalidRangeResponse(
                "Sous-titre non pris en charge".into(),
            ));
        }
        Ok(bytes)
    }
    pub async fn next_video(
        &self,
        profile: &str,
        media: &str,
        context: &PlaybackContext,
    ) -> Result<NextVideo, ClientError> {
        self.request_json(
            Method::POST,
            &profile_path(profile, "next")?,
            Some(&serde_json::json!({"media_id":media,"context":context})),
        )
        .await
    }
    pub async fn playback_info(
        &self,
        profile: &str,
        media: &str,
    ) -> Result<PlaybackInfo, ClientError> {
        profile_path(profile, "info")?;
        if !is_opaque_id(media) {
            return Err(ClientError::InvalidRequestPath("média invalide".into()));
        }
        self.get_json(&format!(
            "/api/v1/profiles/{profile}/media/{media}/playback-info"
        ))
        .await
    }
    pub async fn save_playback_speed(
        &self,
        profile: &str,
        lease: &PlaybackControl,
        speed: f64,
    ) -> Result<(), ClientError> {
        let _: serde_json::Value = self
            .request_json(
                Method::POST,
                &profile_path(profile, "speed")?,
                Some(&serde_json::json!({"lease":lease,"speed":speed})),
            )
            .await?;
        Ok(())
    }
    pub async fn acquire_playback(
        &self,
        profile: &str,
        body: &AcquirePlayback,
    ) -> Result<PlaybackGrant, ClientError> {
        self.request_json(Method::POST, &profile_path(profile, "acquire")?, Some(body))
            .await
    }
    pub async fn heartbeat_playback(
        &self,
        profile: &str,
        body: &PlaybackControl,
    ) -> Result<Lease, ClientError> {
        self.request_json(
            Method::POST,
            &profile_path(profile, "heartbeat")?,
            Some(body),
        )
        .await
    }
    pub async fn report_progress(
        &self,
        profile: &str,
        body: &Progress,
    ) -> Result<PersonalState, ClientError> {
        self.request_json(
            Method::POST,
            &profile_path(profile, "progress")?,
            Some(body),
        )
        .await
    }
    pub async fn release_playback(
        &self,
        profile: &str,
        body: &ReleasePlayback,
    ) -> Result<(), ClientError> {
        let _: serde_json::Value = self
            .request_json(Method::POST, &profile_path(profile, "release")?, Some(body))
            .await?;
        Ok(())
    }
    pub async fn playback_position(
        &self,
        profile: &str,
        media: &str,
    ) -> Result<PersonalState, ClientError> {
        if !is_opaque_id(profile) || !is_opaque_id(media) {
            return Err(ClientError::InvalidRequestPath(
                "profil ou média invalide".into(),
            ));
        }
        self.get_json(&format!("/api/v1/profiles/{profile}/media/{media}/state"))
            .await
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profile_scope_cannot_inject_paths() {
        for id in ["../other", "x/y", "x?q=1", ""] {
            assert!(profile_path(id, "acquire").is_err());
        }
        assert_eq!(
            profile_path("profile-id", "progress").unwrap(),
            "/api/v1/profiles/profile-id/playback/progress"
        );
    }
}
