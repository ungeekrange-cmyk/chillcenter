//! Typed playback operations over the same pinned transport as the catalogue.
use crate::{ClientError, Transport, is_opaque_id};
pub use chill_domain::{
    AcquirePlayback, Lease, PersonalState, PlaybackControl, PlaybackGrant, PlaybackInfo, Progress,
    ReleasePlayback,
};
use reqwest::Method;
fn profile_path(profile: &str, operation: &str) -> Result<String, ClientError> {
    if !is_opaque_id(profile) {
        return Err(ClientError::InvalidRequestPath("profil invalide".into()));
    }
    Ok(format!("/api/v1/profiles/{profile}/playback/{operation}"))
}
impl Transport {
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
