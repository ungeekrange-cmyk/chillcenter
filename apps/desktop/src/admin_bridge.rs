//! Native-only archive file boundary. Webview code receives opaque job results,
//! never a local path or archive bytes.
use chill_client::ui::UiError;
use std::{fs::{self, File, OpenOptions}, io::Read, path::Path};

pub const MAX_ARCHIVE: u64 = 1024 * 1024 * 1024;
fn error(message: &str, code: &str) -> UiError { UiError { message: message.into(), status: None, code: code.into() } }

pub fn selected_zip(path: &Path) -> Result<File, UiError> {
    if !path.is_absolute() { return Err(error("Archive invalide.", "ARCHIVE_INVALID")); }
    let meta=fs::symlink_metadata(path).map_err(|_|error("Archive inaccessible.", "ARCHIVE_UNAVAILABLE"))?;
    if meta.file_type().is_symlink() || !meta.is_file() || meta.len()>MAX_ARCHIVE { return Err(error("Archive ZIP invalide ou trop volumineuse.", "ARCHIVE_INVALID")); }
    let file=File::open(path).map_err(|_|error("Archive inaccessible.", "ARCHIVE_UNAVAILABLE"))?;
    let after=file.metadata().map_err(|_|error("Archive inaccessible.", "ARCHIVE_UNAVAILABLE"))?;
    if !after.is_file() || after.len()!=meta.len() { return Err(error("Archive modifiée pendant la sélection.", "ARCHIVE_CHANGED")); }
    Ok(file)
}
#[allow(dead_code)]
pub fn save_new_zip(path: &Path, mut source: impl Read) -> Result<(), UiError> {
    if !path.is_absolute() || path.extension().and_then(|v|v.to_str())!=Some("zip") { return Err(error("Destination ZIP invalide.", "ARCHIVE_INVALID")); }
    let parent=path.parent().ok_or_else(||error("Destination ZIP invalide.", "ARCHIVE_INVALID"))?;
    let part=parent.join(format!(".{}.partial", uuid::Uuid::new_v4()));
    use std::os::unix::fs::OpenOptionsExt;
    let mut output=OpenOptions::new().write(true).create_new(true).mode(0o600).open(&part).map_err(|_|error("Destination déjà existante ou inaccessible.", "ARCHIVE_DESTINATION"))?;
    let copied=std::io::copy(&mut source.by_ref().take(MAX_ARCHIVE+1),&mut output).map_err(|_|error("Écriture ZIP interrompue.", "ARCHIVE_WRITE"))?;
    if copied>MAX_ARCHIVE { let _=fs::remove_file(&part); return Err(error("Export ZIP trop volumineux.", "ARCHIVE_TOO_LARGE")); }
    output.sync_all().map_err(|_|error("Écriture ZIP interrompue.", "ARCHIVE_WRITE"))?;
    // rename refuses an existing user-selected destination (no overwrite).
    if path.exists() { let _=fs::remove_file(&part); return Err(error("Destination déjà existante.", "ARCHIVE_DESTINATION")); }
    fs::rename(&part,path).map_err(|_| { let _=fs::remove_file(&part); error("Publication ZIP impossible.", "ARCHIVE_WRITE") })
}
#[cfg(test)]
mod tests { use super::*; #[test] fn rejects_links_and_never_overwrites() { let d=tempfile::tempdir().unwrap(); let src=d.path().join("a.zip"); fs::write(&src,b"zip").unwrap(); assert!(selected_zip(&src).is_ok()); let link=d.path().join("l.zip"); std::os::unix::fs::symlink(&src,&link).unwrap(); assert!(selected_zip(&link).is_err()); let out=d.path().join("out.zip"); save_new_zip(&out,&b"ok"[..]).unwrap(); assert!(save_new_zip(&out,&b"no"[..]).is_err()); assert_eq!(fs::read(out).unwrap(),b"ok"); } }
