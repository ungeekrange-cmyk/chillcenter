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

const MAX_MANIFEST: u64 = 64 * 1024;
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
    let value: serde_json::Value = parse_strict_json(&manifest_bytes)?;
    validate_theme_metadata(&value)?;
    let manifest: ThemeManifest = serde_json::from_value(value)?;
    if manifest.format_version != 1
        || !matches!(manifest.renderer_version, 1 | 2)
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

/// Shared pre-activation calendar/token validation for Linux and native Mac.
/// Web mirrors these same limits; a malformed signed pack is still rejected.
pub fn validate_theme_metadata(value: &serde_json::Value) -> Result<()> {
    let valid = (|| -> Option<()> {
        use serde_json::Value;
        let renderer = value["renderer_version"].as_u64()?;
        if value["format_version"] != 1 || !matches!(renderer, 1 | 2) {
            return None;
        }
        let version = Version::parse(value["version"].as_str()?).ok()?;
        if !version.pre.is_empty() || !version.build.is_empty() {
            return None;
        }
        let assets = value["assets"].as_object()?;
        if assets.is_empty() || assets.len() > 128 {
            return None;
        }
        validate_asset_path(value["default_logo"].as_str()?).ok()?;
        if !assets.contains_key(value["default_logo"].as_str()?) {
            return None;
        }
        let date = |value: &Value| -> Option<u16> {
            let parts = value.as_array().filter(|p| p.len() == 2)?;
            let month = parts[0].as_u64()?;
            let day = parts[1].as_u64()?;
            if !(1..=12).contains(&month) {
                return None;
            }
            let maximum = *[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
                .get(month.checked_sub(1)? as usize)?;
            (day >= 1 && day <= maximum).then_some((month * 32 + day) as u16)
        };
        let palette = |value: &Value| {
            value.as_object().is_some_and(|p| {
                p.iter().all(|(key, value)| {
                    matches!(
                        key.as_str(),
                        "bg" | "panel"
                            | "raised"
                            | "text"
                            | "muted"
                            | "accent"
                            | "button"
                            | "sidebar"
                            | "line"
                    ) && value.as_str().is_some_and(|c| {
                        matches!(c.len(), 7 | 9)
                            && c.starts_with('#')
                            && c.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
                    })
                })
            })
        };
        let themes = value["themes"].as_array()?;
        if themes.len() != if renderer == 1 { 6 } else { 8 } { return None; }
        let mut ids = BTreeSet::new();
        let mut ranges = Vec::new();
        for theme in themes {
            let id = theme["id"].as_str()?;
            if !matches!(
                id,
                "spring" | "summer" | "autumn" | "winter" | "christmas" | "halloween" | "easter" | "valentine"
            ) || !ids.insert(id)
            {
                return None;
            }
            let priority = theme["priority"]
                .as_i64()
                .filter(|p| (-1000..=1000).contains(p))?;
            if matches!(id, "easter" | "valentine") {
                if renderer != 2 || theme.get("start").is_some() || theme.get("end").is_some()
                    || !matches!((id, theme["calendar"].as_str()?), ("easter", "western_easter") | ("valentine", "valentine"))
                    || (id == "easter") != theme.get("decoration").is_some() { return None; }
                if let Some(decoration) = theme.get("decoration") {
                    let path = decoration.as_str()?;
                    if !path.starts_with("assets/decorations/") || validate_asset_path(path).is_err() || !assets.contains_key(path) { return None; }
                }
            } else {
                ranges.push((date(&theme["start"])?, date(&theme["end"])?, priority));
            }
            for tone in ["light", "dark"] {
                let variant = &theme[tone];
                if !palette(&variant["palette"]) || !palette(&variant["sidebar"]) {
                    return None;
                }
                for field in ["logo", "background"] {
                    if field == "background" && variant.get(field).is_none() {
                        continue;
                    }
                    let path = variant[field].as_str()?;
                    validate_asset_path(path).ok()?;
                    if !assets.contains_key(path) {
                        return None;
                    }
                }
            }
        }
        for year in 2026..=2038 { for (month, days) in [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31].into_iter().enumerate() { for day in 1..=days {
                if month == 1 && day == 29 && !(year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)) { continue; }
                let point = ((month + 1) * 32 + day) as u16;
                let mut priorities = BTreeSet::new();
                for &(start, end, priority) in &ranges {
                    if ((start <= end && point >= start && point <= end)
                        || (start > end && (point >= start || point <= end)))
                        && !priorities.insert(priority)
                    {
                        return None;
                    }
                }
                for theme in themes.iter().filter(|theme| theme.get("calendar").is_some()) {
                    if event_active(year, (month + 1) as u8, day as u8, theme["calendar"].as_str()?) && !priorities.insert(theme["priority"].as_i64()?) { return None; }
                }
                if priorities.is_empty() {
                    return None;
                }
            } } }
        Some(())
    })();
    valid.ok_or_else(|| UpdateError::Security("invalid theme calendar, variants or tokens".into()))
}

fn event_active(year: i32, month: u8, day: u8, calendar: &str) -> bool {
    let (end_month, end_day, span) = match calendar {
        "valentine" => (2, 14, 14),
        "western_easter" => {
            let a = year % 19; let b = year / 100; let c = year % 100; let d = b / 4; let e = b % 4;
            let f = (b + 8) / 25; let g = (b - f + 1) / 3; let h = (19 * a + b - d - g + 15) % 30;
            let i = c / 4; let k = c % 4; let l = (32 + 2 * e + 2 * i - h - k) % 7; let m = (a + 11 * h + 22 * l) / 451;
            ((h + l - 7 * m + 114) / 31, (h + l - 7 * m + 114) % 31 + 1, 30)
        }
        _ => return false,
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let ordinal = |month: i32, day: i32| {
        let month_days = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        month_days[..(month - 1) as usize].iter().sum::<i32>() + day
    };
    let point = ordinal(i32::from(month), i32::from(day));
    let end = ordinal(end_month, end_day);
    point >= end - span && point <= end
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
                if key == "logo" || key == "background" || key == "decoration" {
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
            Some(Component::Normal(value)) if value == "logos" || value == "backgrounds" || value == "decorations"
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
            "themes": serde_json::from_str::<serde_json::Value>(include_str!("../../../themes/default/manifest.json")).unwrap()["themes"].clone()
        });
        for theme in manifest["themes"].as_array_mut().unwrap() {
            for tone in ["light", "dark"] {
                theme[tone]["logo"] = serde_json::json!("assets/logos/default.png");
                theme[tone].as_object_mut().unwrap().remove("background");
            }
        }
        manifest["themes"].as_array_mut().unwrap().retain(|theme| {
            !matches!(theme["id"].as_str(), Some("easter") | Some("valentine"))
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

    #[test]
    fn metadata_refuses_gaps_ambiguous_dates_and_active_tokens() {
        let valid: serde_json::Value =
            serde_json::from_str(include_str!("../../../themes/default/manifest.json")).unwrap();
        assert!(validate_theme_metadata(&valid).is_ok());
        for (pointer, value) in [
            ("/themes/0/start", serde_json::json!([3, 2])),
            ("/themes/0/priority", serde_json::json!(1001)),
            ("/themes/0/light/background", serde_json::Value::Null),
            (
                "/themes/0/light/palette/bg",
                serde_json::json!("url(http://invalid.example)"),
            ),
        ] {
            let mut invalid = valid.clone();
            if pointer.ends_with("background") {
                invalid["themes"][0]["light"]["background"] = value;
            } else {
                *invalid.pointer_mut(pointer).unwrap() = value;
            }
            assert!(validate_theme_metadata(&invalid).is_err(), "{pointer}");
        }
        let mut malformed_event = valid.clone();
        malformed_event["themes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|theme| theme["id"] == "easter")
            .unwrap()["calendar"] = serde_json::json!("fixed");
        assert!(validate_theme_metadata(&malformed_event).is_err());
    }
}
