//! Explicit local image import. No source file is changed and no URL is accepted.
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

const MAX_BYTES: usize = 8 * 1024 * 1024;
const MAX_WIDTH: u32 = 4096;
const MAX_HEIGHT: u32 = 4096;
const MAX_ALLOC: u64 = 64 * 1024 * 1024;
const PREVIEW_WIDTH: u32 = 360;
const PREVIEW_HEIGHT: u32 = 540;
pub const MAX_PREVIEW_BYTES: usize = 512 * 1024;

fn decode(bytes: &[u8]) -> Result<image::DynamicImage, String> {
    if bytes.len() > MAX_BYTES {
        return Err("Image trop volumineuse (8 MiB maximum).".into());
    }
    extension(bytes)?;
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| "Image invalide.")?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_WIDTH);
    limits.max_image_height = Some(MAX_HEIGHT);
    limits.max_alloc = Some(MAX_ALLOC);
    reader.limits(limits);
    let decoded = reader
        .decode()
        .map_err(|_| "Image invalide ou dimensions trop grandes (4096 × 4096 maximum).")?;
    if decoded.width() == 0 || decoded.height() == 0 {
        return Err("Image vide.".into());
    }
    Ok(decoded)
}

/// Validate the actual decoded image, not merely its signature. The pixel
/// budget bounds work on the Pi; allocation limits additionally cap decoders.
pub fn validate(bytes: &[u8]) -> Result<(), String> {
    decode(bytes)?;
    Ok(())
}

/// Create a webview-safe JPEG preview from an already bounded poster.
///
/// The source is never altered. Decoding happens once under the same limits as
/// validation; repeated JPEG encodes only operate on that in-memory thumbnail.
pub fn thumbnail(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let decoded = decode(bytes)?;
    let preview = decoded.resize(
        PREVIEW_WIDTH,
        PREVIEW_HEIGHT,
        image::imageops::FilterType::Triangle,
    );
    for quality in [85, 75, 65, 55, 45, 35, 25, 15, 10] {
        let mut encoded = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded, quality)
            .encode_image(&preview)
            .map_err(|_| "Impossible de préparer l’aperçu de l’affiche.")?;
        if encoded.len() <= MAX_PREVIEW_BYTES {
            return Ok(encoded);
        }
    }
    Err("Aperçu d’affiche trop volumineux.".into())
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
    use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};

    fn encoded(format: ImageFormat, width: u32, height: u32) -> Vec<u8> {
        let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(
            width,
            height,
            Rgba([30, 120, 210, 255]),
        ));
        let mut bytes = Vec::new();
        image
            .write_to(&mut std::io::Cursor::new(&mut bytes), format)
            .unwrap();
        bytes
    }
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
    #[test]
    fn thumbnail_is_a_bounded_jpeg_for_png_and_jpeg_without_mutating_source() {
        for format in [ImageFormat::Png, ImageFormat::Jpeg] {
            let source = encoded(format, 1440, 1080);
            let before = source.clone();
            let preview = thumbnail(&source).unwrap();
            assert_eq!(source, before);
            assert!(preview.starts_with(&[0xff, 0xd8, 0xff]));
            assert!(preview.len() <= MAX_PREVIEW_BYTES);
            let decoded = image::load_from_memory(&preview).unwrap();
            assert!(decoded.width() <= PREVIEW_WIDTH);
            assert!(decoded.height() <= PREVIEW_HEIGHT);
        }
    }
    #[test]
    fn thumbnail_rejects_svg_and_oversize_before_decode() {
        assert!(thumbnail(b"<svg onload='evil()'/>").is_err());
        let mut oversized_svg = vec![b'x'; MAX_BYTES + 1];
        oversized_svg[..5].copy_from_slice(b"<svg ");
        assert!(thumbnail(&oversized_svg).is_err());
    }
}
