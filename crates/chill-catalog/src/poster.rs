//! Explicit local image import. No source file is changed and no URL is accepted.
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

const MAX_BYTES: usize = 8 * 1024 * 1024;
/// Validate the actual decoded image, not merely its signature. The pixel
/// budget bounds work on the Pi; allocation limits additionally cap decoders.
pub fn validate(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > MAX_BYTES {
        return Err("Image trop volumineuse (8 MiB maximum).".into());
    }
    extension(bytes)?;
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| "Image invalide.")?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let decoded = reader
        .decode()
        .map_err(|_| "Image invalide ou dimensions trop grandes (4096 × 4096 maximum).")?;
    if decoded.width() == 0 || decoded.height() == 0 {
        return Err("Image vide.".into());
    }
    Ok(())
}
fn extension(bytes: &[u8]) -> Result<&'static str, String> {
    if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Ok("jpg")
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Ok("png")
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Ok("webp")
    } else {
        Err("Choisissez une image JPEG, PNG ou WebP (8 Mo maximum).".into())
    }
}

pub fn import(source: &Path, cache: &Path) -> Result<String, String> {
    if !source.is_absolute() {
        return Err("Chemin d’image invalide.".into());
    }
    let file = File::open(source).map_err(|_| "Image inaccessible.")?;
    let info = file.metadata().map_err(|_| "Image inaccessible.")?;
    if !info.is_file() || info.len() > MAX_BYTES as u64 {
        return Err("Choisissez une image de 8 Mo maximum.".into());
    }
    let mut bytes = Vec::new();
    file.take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "Lecture de l’image interrompue.")?;
    if bytes.len() > MAX_BYTES {
        return Err("Image trop volumineuse.".into());
    }
    store(&bytes, cache)
}

/// Store a fresh immutable cache file; never overwrite an image already in use.
pub fn store(bytes: &[u8], cache: &Path) -> Result<String, String> {
    if bytes.len() > MAX_BYTES {
        return Err("Image trop volumineuse.".into());
    }
    let ext = extension(bytes)?;
    validate(bytes)?;
    std::fs::create_dir_all(cache).map_err(|_| "Cache des affiches inaccessible.")?;
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "Horloge indisponible.")?
        .as_nanos();
    let destination = cache.join(format!("custom-{unique}.{ext}"));
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&destination)
        .map_err(|_| "Impossible de créer l’affiche locale.")?;
    use std::os::unix::fs::OpenOptionsExt;
    let result = output
        .write_all(bytes)
        .and_then(|()| output.sync_all())
        .and_then(|()| File::open(cache)?.sync_all());
    if result.is_err() {
        let _ = std::fs::remove_file(&destination);
        return Err("Impossible d’enregistrer l’affiche.".into());
    }
    Ok(destination.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn imports_a_copy_without_changing_source_and_rejects_active_content() {
        let folder = tempfile::tempdir().unwrap();
        let source = folder.path().join("chosen.png");
        let bytes = include_bytes!("../../../icons/icon.png");
        std::fs::write(&source, bytes).unwrap();
        let copied = import(&source, &folder.path().join("posters")).unwrap();
        assert_ne!(Path::new(&copied), source);
        assert_eq!(std::fs::read(&copied).unwrap(), bytes);
        assert_eq!(std::fs::read(&source).unwrap(), bytes);
        std::fs::write(&source, b"<svg onload='evil()'/>").unwrap();
        assert!(import(&source, &folder.path().join("posters")).is_err());
        assert!(import(Path::new("relative.jpg"), folder.path()).is_err());
    }
    #[test]
    fn limits_image_bytes_before_allocating() {
        let folder = tempfile::tempdir().unwrap();
        let source = folder.path().join("oversize.jpg");
        File::create(&source)
            .unwrap()
            .set_len((MAX_BYTES + 1) as u64)
            .unwrap();
        assert!(import(&source, &folder.path().join("posters")).is_err());
        assert!(!folder.path().join("posters").exists());
    }
    #[test]
    fn downloaded_images_use_distinct_immutable_cache_files() {
        let folder = tempfile::tempdir().unwrap();
        let bytes = include_bytes!("../../../icons/icon.png");
        let first = store(bytes, folder.path()).unwrap();
        let second = store(bytes, folder.path()).unwrap();
        assert_ne!(first, second);
        assert_eq!(std::fs::read(&first).unwrap(), bytes);
        assert_eq!(std::fs::read(&second).unwrap(), bytes);
        assert!(store(b"<svg onload='evil()'/>", folder.path()).is_err());
        assert!(store(b"\x89PNG\r\n\x1a\ninvalid", folder.path()).is_err());
        assert!(store(b"\xff\xd8\xffinvalid", folder.path()).is_err());
    }
}
