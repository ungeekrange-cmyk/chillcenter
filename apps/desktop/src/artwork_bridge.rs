use chill_client::ui::{UiClient, UiError};
use serde::Deserialize;
use serde_json::Value;
use std::{fs::OpenOptions, io::Read, os::unix::fs::OpenOptionsExt, path::Path};

const MAX: usize = 8 * 1024 * 1024;
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ArtworkTarget {
    Media {
        media_id: String,
        expected_revision: u64,
    },
    Saga {
        id: String,
        expected_revision: u64,
    },
}
fn err(message: &str) -> UiError {
    UiError {
        message: message.into(),
        status: None,
        code: "ARTWORK_INVALID".into(),
    }
}
pub fn read_selected(path: &Path) -> Result<(Vec<u8>, &'static str), UiError> {
    let before = std::fs::symlink_metadata(path).map_err(|_| err("Image introuvable."))?;
    if !before.file_type().is_file() {
        return Err(err("Sélectionnez un fichier image ordinaire."));
    }
    if before.len() > MAX as u64 {
        return Err(err("Image trop volumineuse (8 Mio maximum)."));
    }
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(|_| err("Image non lisible."))?;
    let mut bytes = Vec::with_capacity(before.len() as usize);
    file.by_ref()
        .take(MAX as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| err("Image non lisible."))?;
    let after = file.metadata().map_err(|_| err("Image non lisible."))?;
    if bytes.len() > MAX || after.len() != before.len() {
        return Err(err("Image modifiée pendant la lecture."));
    }
    let mime = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        "image/jpeg"
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        "image/webp"
    } else {
        return Err(err("Format d'image non pris en charge."));
    };
    Ok((bytes, mime))
}
pub fn upload(
    client: &mut UiClient,
    target: ArtworkTarget,
    bytes: Vec<u8>,
    mime: &str,
) -> Result<Value, UiError> {
    let profile = client.playback_connection()?.profile_id;
    let (path, revision) = match target {
        ArtworkTarget::Media {
            media_id,
            expected_revision,
        } => {
            uuid::Uuid::parse_str(&media_id).map_err(|_| err("Média invalide."))?;
            (
                format!("/api/v1/profiles/{profile}/media/{media_id}/poster"),
                expected_revision,
            )
        }
        ArtworkTarget::Saga {
            id,
            expected_revision,
        } => {
            if id.parse::<u64>().is_err() {
                return Err(err("Saga invalide."));
            };
            (
                format!("/api/v1/profiles/{profile}/sagas/{id}/poster"),
                expected_revision,
            )
        }
    };
    client.upload_artwork(&path, revision, bytes, mime)
}
