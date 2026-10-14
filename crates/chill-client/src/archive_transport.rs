//! Native archive streams, never archive-sized webview messages.
use crate::{ClientError, Transport, api_response_error};
use serde_json::Value;
use std::{fs::File, time::Duration};
use tokio::io::AsyncWriteExt;
const MAX: u64 = 1024 * 1024 * 1024;
async fn json(mut response: reqwest::Response) -> Result<Value, ClientError> {
    let status = response.status();
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if bytes.len() + chunk.len() > 1024 * 1024 {
            return Err(ClientError::InvalidRangeResponse(
                "Réponse archive trop volumineuse.".into(),
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    if !status.is_success() {
        return Err(api_response_error(status, &bytes));
    }
    Ok(serde_json::from_slice(&bytes)?)
}
impl Transport {
    pub async fn upload_archive_file(&self, file: File, size: u64) -> Result<Value, ClientError> {
        if size > MAX {
            return Err(ClientError::InvalidRangeResponse(
                "Archive trop volumineuse.".into(),
            ));
        }
        json(
            self.asynchronous
                .post(self.endpoint.request_url("/api/v1/admin/imports")?)
                .header(reqwest::header::CONTENT_TYPE, "application/zip")
                .header(reqwest::header::CONTENT_LENGTH, size)
                .timeout(Duration::from_secs(600))
                .body(reqwest::Body::from(tokio::fs::File::from_std(file)))
                .send()
                .await?,
        )
        .await
    }
    pub async fn download_archive_file(&self, id: &str, file: File) -> Result<u64, ClientError> {
        uuid::Uuid::parse_str(id)
            .map_err(|_| ClientError::InvalidRangeResponse("Export invalide.".into()))?;
        let mut response = self
            .asynchronous
            .get(
                self.endpoint
                    .request_url(&format!("/api/v1/admin/exports/{id}"))?,
            )
            .timeout(Duration::from_secs(600))
            .send()
            .await?;
        if !response.status().is_success() {
            return json(response).await.map(|_| 0);
        }
        if response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            != Some("application/zip")
            || response.content_length().is_none_or(|n| n > MAX)
        {
            return Err(ClientError::InvalidRangeResponse(
                "Réponse ZIP invalide.".into(),
            ));
        }
        let expected = response.content_length().unwrap();
        let mut file = tokio::fs::File::from_std(file);
        let mut size = 0u64;
        while let Some(chunk) = response.chunk().await? {
            size = size.checked_add(chunk.len() as u64).ok_or_else(|| {
                ClientError::InvalidRangeResponse("Archive trop volumineuse.".into())
            })?;
            if size > expected || size > MAX {
                return Err(ClientError::InvalidRangeResponse(
                    "Archive trop volumineuse.".into(),
                ));
            }
            file.write_all(&chunk).await.map_err(|_| {
                ClientError::InvalidRangeResponse("Export non enregistrable.".into())
            })?;
        }
        if size != expected {
            return Err(ClientError::InvalidRangeResponse(
                "Export interrompu.".into(),
            ));
        }
        file.flush()
            .await
            .map_err(|_| ClientError::InvalidRangeResponse("Export non enregistré.".into()))?;
        file.sync_all()
            .await
            .map_err(|_| ClientError::InvalidRangeResponse("Export non enregistré.".into()))?;
        Ok(size)
    }
}
