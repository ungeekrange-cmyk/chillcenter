use chill_client::ui::{UiClient, UiError};
use serde::Deserialize;
use serde_json::Value;
use std::{
    fs::OpenOptions,
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};

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
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| err("Image non lisible."))?;
    let opened = file.metadata().map_err(|_| err("Image non lisible."))?;
    let version = |m: &std::fs::Metadata| {
        (
            m.dev(),
            m.ino(),
            m.len(),
            m.mtime(),
            m.mtime_nsec(),
            m.ctime(),
            m.ctime_nsec(),
        )
    };
    if !opened.is_file() || version(&opened) != version(&before) {
        return Err(err("Image modifiée avant la lecture."));
    }
    let mut bytes = Vec::with_capacity(before.len() as usize);
    file.by_ref()
        .take(MAX as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| err("Image non lisible."))?;
    let after = file.metadata().map_err(|_| err("Image non lisible."))?;
    let named =
        std::fs::symlink_metadata(path).map_err(|_| err("Image modifiée pendant la lecture."))?;
    if bytes.len() > MAX
        || version(&after) != version(&opened)
        || version(&named) != version(&opened)
        || bytes.len() as u64 != opened.len()
    {
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
    let (path, revision) = target_path(&profile, target)?;
    client.upload_artwork(&path, revision, bytes, mime)
}
fn target_path(profile: &str, target: ArtworkTarget) -> Result<(String, u64), UiError> {
    match target {
        ArtworkTarget::Media {
            media_id,
            expected_revision,
        } => {
            uuid::Uuid::parse_str(&media_id).map_err(|_| err("Média invalide."))?;
            Ok((
                format!("/api/v1/profiles/{profile}/media/{media_id}/poster"),
                expected_revision,
            ))
        }
        ArtworkTarget::Saga {
            id,
            expected_revision,
        } => {
            if id.parse::<u64>().is_err() {
                return Err(err("Saga invalide."));
            };
            Ok((
                format!("/api/v1/profiles/{profile}/sagas/{id}/poster"),
                expected_revision,
            ))
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::symlink};
    fn temp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("chill-artwork-{name}-{}", uuid::Uuid::new_v4()))
    }
    #[test]
    fn reads_regular_image_without_mutating_source_and_rejects_links_or_oversize() {
        let file = temp("image");
        let source = [0xff, 0xd8, 0xff, 1, 2, 3];
        fs::write(&file, source).unwrap();
        let (got, mime) = read_selected(&file).unwrap();
        assert_eq!(got, source);
        assert_eq!(mime, "image/jpeg");
        assert_eq!(fs::read(&file).unwrap(), source);
        let link = temp("link");
        symlink(&file, &link).unwrap();
        assert!(read_selected(&link).is_err());
        let oversized = temp("oversize");
        let handle = fs::File::create(&oversized).unwrap();
        handle.set_len(MAX as u64 + 1).unwrap();
        assert!(read_selected(&oversized).is_err());
        fs::remove_file(file).unwrap();
        fs::remove_file(link).unwrap();
        fs::remove_file(oversized).unwrap();
    }
    #[test]
    fn closed_target_rejects_unknown_fields_and_invalid_ids() {
        assert!(serde_json::from_value::<ArtworkTarget>(serde_json::json!({"kind":"media","media_id":"x","expected_revision":1,"path":"/tmp/x"})).is_err());
        assert!(
            target_path(
                "profile",
                ArtworkTarget::Media {
                    media_id: "x".into(),
                    expected_revision: 1
                }
            )
            .is_err()
        );
        assert!(
            target_path(
                "profile",
                ArtworkTarget::Saga {
                    id: "../1".into(),
                    expected_revision: 1
                }
            )
            .is_err()
        );
        assert!(
            target_path(
                "profile",
                ArtworkTarget::Saga {
                    id: "7".into(),
                    expected_revision: 2
                }
            )
            .unwrap()
            .0
            .contains("/sagas/7/poster")
        );
    }
}
