//! Typed playback operations over the same pinned transport as the catalogue.
use crate::{ClientError, Transport, is_opaque_id};
pub use chill_domain::{
    AcquirePlayback, Lease, NextVideo, PersonalState, PlaybackContext, PlaybackControl,
    PlaybackGrant, PlaybackInfo, Progress, ReleasePlayback, TransferAck, TransferConfirmation,
    TransferOffer, TransferPhase, TransferProposal, TransferStatus,
};
use reqwest::Method;
#[derive(Debug, serde::Deserialize)]
pub struct PauseInstruction {
    pub token: String,
    pub remaining_ms: u64,
}
pub struct ControlSubscription {
    pub receiver: tokio::sync::mpsc::Receiver<PauseInstruction>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for ControlSubscription {
    fn drop(&mut self) {
        self.task.abort();
    }
}
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
    pub async fn propose_transfer(
        &self,
        profile: &str,
        target: &AcquirePlayback,
    ) -> Result<TransferOffer, ClientError> {
        self.request_json(
            Method::POST,
            &profile_path(profile, "transfer/propose")?,
            Some(&TransferProposal {
                target: target.clone(),
            }),
        )
        .await
    }
    pub async fn confirm_transfer(
        &self,
        profile: &str,
        token: &str,
    ) -> Result<TransferStatus, ClientError> {
        self.request_json(
            Method::POST,
            &profile_path(profile, "transfer/confirm")?,
            Some(&TransferConfirmation {
                token: token.into(),
            }),
        )
        .await
    }
    pub async fn transfer_status(
        &self,
        profile: &str,
        token: &str,
    ) -> Result<TransferStatus, ClientError> {
        self.json_with_options::<(), _>(
            Method::GET,
            &profile_path(profile, "transfer/status")?,
            None,
            &[("token", token.into())],
            &[],
        )
        .await
    }
    pub async fn acknowledge_transfer(
        &self,
        profile: &str,
        ack: &TransferAck,
    ) -> Result<TransferStatus, ClientError> {
        self.request_json(
            Method::POST,
            &profile_path(profile, "transfer/ack")?,
            Some(ack),
        )
        .await
    }
    pub fn subscribe_controls(&self, profile: String, lease: Lease) -> ControlSubscription {
        let transport = self.clone();
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        let task = tokio::spawn(async move {
            loop {
                if sender.is_closed() {
                    break;
                }
                if let Ok(Some(event)) = transport.wait_pause(&profile, &lease).await {
                    let _ = sender.send(event).await;
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });
        ControlSubscription { receiver, task }
    }
    async fn wait_pause(
        &self,
        profile: &str,
        lease: &Lease,
    ) -> Result<Option<PauseInstruction>, ClientError> {
        let path = profile_path(profile, "events")?;
        let mut response = self
            .asynchronous
            .get(self.endpoint.request_url(&path)?)
            .query(&subtitle_query(lease))
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(ClientError::Http(response.status()));
        }
        let mut pending = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if pending.len().saturating_add(chunk.len()) > 8192 {
                return Err(ClientError::InvalidRangeResponse(
                    "Événement de contrôle trop volumineux".into(),
                ));
            }
            pending.extend_from_slice(&chunk);
            while let Some(end) = pending.windows(2).position(|w| w == b"\n\n") {
                let block: String = String::from_utf8_lossy(&pending[..end]).into();
                pending.drain(..end + 2);
                if block.lines().any(|l| l.trim() == "event: pause") {
                    let data = block
                        .lines()
                        .find_map(|l| l.strip_prefix("data:"))
                        .ok_or_else(|| {
                            ClientError::InvalidRangeResponse("Contrôle invalide".into())
                        })?;
                    return Ok(Some(serde_json::from_str(data.trim())?));
                }
            }
        }
        Ok(None)
    }
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
