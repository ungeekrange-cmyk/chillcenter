use crate::{Result, UpdateError, parse_strict_json};
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::{BufReader, Cursor, Read};
use std::path::{Component, Path};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

const MAX_MANIFEST: u64 = 1024 * 1024;
const MAX_ASSETS: usize = 128;
const MAX_PNG: u64 = 5 * 1024 * 1024;
const MAX_TOTAL: u64 = 12 * 1024 * 1024;
const MAX_DECODE_BYTES: usize = 32 * 1024 * 1024;
const MAX_DIMENSION: u32 = 4096;
const MAX_PIXELS: u64 = 8 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeManifest {
    format_version: u32,
    renderer_version: u32,
    version: Version,
    default_logo: String,
    assets: BTreeMap<String, ThemeAsset>,
    themes: serde_json::Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeAsset {
    size: u64,
    sha256: String,
}

/// Validates the complete declarative theme tree. Only exact manifest-declared
/// PNG files are permitted, and each image is bounded and fully decoded so CRC,
/// truncation, decompression, dimensions, and APNG are checked before publish.
pub fn validate_theme_package(root: &Path) -> Result<()> {
    let manifest_bytes = read_regular(&root.join("manifest.json"), MAX_MANIFEST)?;
    let manifest: ThemeManifest = parse_strict_json(&manifest_bytes)?;
    if manifest.format_version != 1
        || manifest.renderer_version != 1
        || !manifest.version.pre.is_empty()
        || manifest.assets.is_empty()
        || manifest.assets.len() > MAX_ASSETS
        || !manifest.themes.is_array()
    {
        return Err(UpdateError::Security(
            "invalid theme manifest identity".into(),
        ));
    }
    let mut referenced = BTreeSet::from([manifest.default_logo.clone()]);
    collect_references(&manifest.themes, &mut referenced)?;
    let declared: BTreeSet<_> = manifest.assets.keys().cloned().collect();
    if referenced != declared {
        return Err(UpdateError::Security(
            "theme references and declared assets differ".into(),
        ));
    }

    let mut expected = BTreeSet::from(["manifest.json".to_owned()]);
    let mut total = 0u64;
    for (relative, asset) in &manifest.assets {
        validate_asset_path(relative)?;
        if asset.size == 0
            || asset.size > MAX_PNG
            || asset.sha256.len() != 64
            || !asset
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(UpdateError::Security("invalid theme asset receipt".into()));
        }
        total = total
            .checked_add(asset.size)
            .ok_or_else(|| UpdateError::Security("theme size overflow".into()))?;
        if total > MAX_TOTAL {
            return Err(UpdateError::Security(
                "theme package exceeds total byte bound".into(),
            ));
        }
        let path = root.join(relative);
        let bytes = read_regular(&path, MAX_PNG)?;
        if bytes.len() as u64 != asset.size || hex::encode(Sha256::digest(&bytes)) != asset.sha256 {
            return Err(UpdateError::Security(
                "theme asset hash/size mismatch".into(),
            ));
        }
        validate_theme_png(&bytes)?;
        expected.insert(relative.clone());
    }
    let actual = actual_files(root)?;
    if actual != expected {
        return Err(UpdateError::Security(
            "theme tree contains undeclared or missing files".into(),
        ));
    }
    Ok(())
}

fn collect_references(value: &serde_json::Value, output: &mut BTreeSet<String>) -> Result<()> {
    match value {
        serde_json::Value::Array(items) => {
            for item in items {
                collect_references(item, output)?;
            }
        }
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                if key == "logo" || key == "background" {
                    let path = value.as_str().ok_or_else(|| {
                        UpdateError::Security("theme image reference is not a string".into())
                    })?;
                    output.insert(path.to_owned());
                } else {
                    collect_references(value, output)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_asset_path(value: &str) -> Result<()> {
    let path = Path::new(value);
    let mut parts = path.components();
    let valid_prefix = parts.next() == Some(Component::Normal("assets".as_ref()))
        && matches!(
            parts.next(),
            Some(Component::Normal(value)) if value == "logos" || value == "backgrounds"
        );
    let Some(Component::Normal(file)) = parts.next() else {
        return Err(UpdateError::Security("invalid theme asset path".into()));
    };
    if !valid_prefix
        || parts.next().is_some()
        || Path::new(file).extension().and_then(|value| value.to_str()) != Some("png")
        || file.to_str().is_none_or(|name| {
            name.is_empty()
                || name.len() > 128
                || !name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
        })
    {
        return Err(UpdateError::Security("invalid theme asset path".into()));
    }
    Ok(())
}

pub fn validate_theme_png(bytes: &[u8]) -> Result<()> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_PNG {
        return Err(UpdateError::Security("PNG file exceeds bound".into()));
    }
    let limits = png::Limits {
        bytes: MAX_DECODE_BYTES,
    };
    let decoder = png::Decoder::new_with_limits(BufReader::new(Cursor::new(bytes)), limits);
    let mut reader = decoder
        .read_info()
        .map_err(|error| UpdateError::Security(format!("invalid PNG: {error}")))?;
    let info = reader.info();
    if info.width == 0
        || info.height == 0
        || info.width > MAX_DIMENSION
        || info.height > MAX_DIMENSION
        || u64::from(info.width) * u64::from(info.height) > MAX_PIXELS
        || info.animation_control.is_some()
    {
        return Err(UpdateError::Security(
            "PNG dimensions or animation are forbidden".into(),
        ));
    }
    let output = reader
        .output_buffer_size()
        .filter(|size| *size <= MAX_DECODE_BYTES)
        .ok_or_else(|| UpdateError::Security("PNG decode buffer exceeds bound".into()))?;
    let mut decoded = vec![0; output];
    reader
        .next_frame(&mut decoded)
        .map_err(|error| UpdateError::Security(format!("invalid PNG data: {error}")))?;
    Ok(())
}

fn actual_files(root: &Path) -> Result<BTreeSet<String>> {
    let mut files = BTreeSet::new();
    let mut stack = vec![root.to_path_buf()];
    let mut entries = 0usize;
    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            entries += 1;
            if entries > MAX_ASSETS + 4 {
                return Err(UpdateError::Security(
                    "theme tree entry bound exceeded".into(),
                ));
            }
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                return Err(UpdateError::Security("theme symlink refused".into()));
            }
            if kind.is_dir() {
                stack.push(entry.path());
            } else if kind.is_file() {
                files.insert(
                    entry
                        .path()
                        .strip_prefix(root)
                        .map_err(|_| UpdateError::Security("theme path escaped".into()))?
                        .to_string_lossy()
                        .into_owned(),
                );
            } else {
                return Err(UpdateError::Security("theme special file refused".into()));
            }
        }
    }
    Ok(files)
}

fn read_regular(path: &Path, maximum: u64) -> Result<Vec<u8>> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    options.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > maximum {
        return Err(UpdateError::Security("theme file is outside bounds".into()));
    }
    let mut bytes = Vec::new();
    file.take(maximum + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > maximum {
        return Err(UpdateError::Security("theme file exceeds bound".into()));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    fn png_bytes() -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 1, 1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&[1, 2, 3, 255]).unwrap();
        }
        bytes
    }

    #[test]
    fn exact_theme_tree_decodes_and_rejects_undeclared_file() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join("assets/logos")).unwrap();
        let png = png_bytes();
        fs::write(temp.path().join("assets/logos/default.png"), &png).unwrap();
        let mut manifest = serde_json::json!({
            "format_version": 1,
            "renderer_version": 1,
            "version": "0.7.0",
            "default_logo": "assets/logos/default.png",
            "assets": {"assets/logos/default.png": {
                "size": png.len(), "sha256": hex::encode(Sha256::digest(&png))
            }},
            "themes": []
        });
        File::create(temp.path().join("manifest.json"))
            .unwrap()
            .write_all(&serde_json::to_vec(&manifest).unwrap())
            .unwrap();
        assert!(validate_theme_package(temp.path()).is_ok());
        fs::write(temp.path().join("assets/logos/undeclared.png"), &png).unwrap();
        assert!(validate_theme_package(temp.path()).is_err());
        fs::remove_file(temp.path().join("assets/logos/undeclared.png")).unwrap();
        let truncated = &png[..20];
        fs::write(temp.path().join("assets/logos/default.png"), truncated).unwrap();
        manifest["assets"]["assets/logos/default.png"]["size"] = serde_json::json!(truncated.len());
        manifest["assets"]["assets/logos/default.png"]["sha256"] =
            serde_json::json!(hex::encode(Sha256::digest(truncated)));
        fs::write(
            temp.path().join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        assert!(validate_theme_package(temp.path()).is_err());
    }
}
