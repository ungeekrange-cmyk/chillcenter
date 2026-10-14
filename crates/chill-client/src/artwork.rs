//! Closed, local conversion of an already fetched poster into a webview-safe preview.

use base64::Engine;

/// Builds a JPEG data URI limited by `chill_catalog::poster::MAX_PREVIEW_BYTES`.
/// No URL, path, network request, or server-side image mutation is involved.
pub fn preview_data_uri(source: &[u8]) -> Result<String, String> {
    let preview = chill_catalog::poster::thumbnail(source)?;
    Ok(format!(
        "data:image/jpeg;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(preview)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    #[test]
    fn preview_is_a_closed_jpeg_data_uri() {
        let source = include_bytes!("../../../icons/icon.png");
        let before = source.to_vec();
        let uri = preview_data_uri(source).unwrap();
        assert_eq!(source, before.as_slice());
        let payload = uri.strip_prefix("data:image/jpeg;base64,").unwrap();
        let preview = base64::engine::general_purpose::STANDARD
            .decode(payload)
            .unwrap();
        assert!(preview.starts_with(&[0xff, 0xd8, 0xff]));
        assert!(preview.len() <= chill_catalog::poster::MAX_PREVIEW_BYTES);
    }
    #[test]
    fn preview_rejects_non_images_without_a_url_fallback() {
        assert!(preview_data_uri(b"<svg onload='evil()'/>").is_err());
    }
}
