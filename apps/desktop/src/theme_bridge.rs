//! Data-only theme resources. No downloaded CSS, JS, filesystem path or URL is
//! exposed to the webview; invalid installed data falls back to bundled assets.
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};

const MAX_TOTAL: usize = 16 * 1024 * 1024;
const EMBEDDED: &str = include_str!("../../../themes/default/manifest.json");
const ASSETS: &[(&str, &[u8])] = &[
    (
        "assets/backgrounds/christmas.png",
        include_bytes!("../../../themes/default/assets/backgrounds/christmas.png"),
    ),
    (
        "assets/backgrounds/halloween.png",
        include_bytes!("../../../themes/default/assets/backgrounds/halloween.png"),
    ),
    (
        "assets/logos/default.png",
        include_bytes!("../../../themes/default/assets/logos/default.png"),
    ),
    (
        "assets/logos/spring-light.png",
        include_bytes!("../../../themes/default/assets/logos/spring-light.png"),
    ),
    (
        "assets/logos/spring-dark.png",
        include_bytes!("../../../themes/default/assets/logos/spring-dark.png"),
    ),
    (
        "assets/logos/summer-light.png",
        include_bytes!("../../../themes/default/assets/logos/summer-light.png"),
    ),
    (
        "assets/logos/summer-dark.png",
        include_bytes!("../../../themes/default/assets/logos/summer-dark.png"),
    ),
    (
        "assets/logos/autumn-light.png",
        include_bytes!("../../../themes/default/assets/logos/autumn-light.png"),
    ),
    (
        "assets/logos/autumn-dark.png",
        include_bytes!("../../../themes/default/assets/logos/autumn-dark.png"),
    ),
    (
        "assets/logos/winter-light.png",
        include_bytes!("../../../themes/default/assets/logos/winter-light.png"),
    ),
    (
        "assets/logos/winter-dark.png",
        include_bytes!("../../../themes/default/assets/logos/winter-dark.png"),
    ),
    (
        "assets/logos/christmas-light.png",
        include_bytes!("../../../themes/default/assets/logos/christmas-light.png"),
    ),
    (
        "assets/logos/christmas-dark.png",
        include_bytes!("../../../themes/default/assets/logos/christmas-dark.png"),
    ),
    (
        "assets/logos/halloween-light.png",
        include_bytes!("../../../themes/default/assets/logos/halloween-light.png"),
    ),
    (
        "assets/logos/halloween-dark.png",
        include_bytes!("../../../themes/default/assets/logos/halloween-dark.png"),
    ),
];

fn safe_asset(path: &str) -> bool {
    let Some(name) = path
        .strip_prefix("assets/logos/")
        .or_else(|| path.strip_prefix("assets/backgrounds/"))
    else {
        return false;
    };
    let Some(name) = name.strip_suffix(".png") else {
        return false;
    };
    !name.is_empty()
        && name.len() <= 80
        && name
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}

fn materialize(manifest: Value, read: impl Fn(&str) -> Option<Vec<u8>>) -> Option<Value> {
    if manifest["format_version"] != 1
        || manifest["renderer_version"] != 1
        || !manifest["version"].is_string()
    {
        return None;
    }
    let declarations = manifest["assets"].as_object()?;
    if declarations.is_empty() || declarations.len() > 128 {
        return None;
    }
    if !valid_metadata(&manifest) {
        return None;
    }
    let mut assets = BTreeMap::new();
    let mut total = 0usize;
    for (path, declaration) in declarations {
        if !safe_asset(path) {
            return None;
        }
        let expected = declaration["size"].as_u64()?;
        if expected == 0 || expected > 5 * 1024 * 1024 {
            return None;
        }
        let bytes = read(path)?;
        if bytes.len() as u64 != expected || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            return None;
        }
        let digest = format!("{:x}", Sha256::digest(&bytes));
        if declaration["sha256"].as_str()? != digest {
            return None;
        }
        let value = format!("data:image/png;base64,{}", STANDARD.encode(bytes));
        total = total.checked_add(value.len())?;
        if total > MAX_TOTAL {
            return None;
        }
        assets.insert(path.clone(), value);
    }
    Some(json!({"manifest": manifest, "assets": assets}))
}

fn valid_metadata(manifest: &Value) -> bool {
    let Some(version) = manifest["version"].as_str() else {
        return false;
    };
    if semver::Version::parse(version).is_err() {
        return false;
    }
    let Some(assets) = manifest["assets"].as_object() else {
        return false;
    };
    if assets
        .values()
        .try_fold(0u64, |sum, v| sum.checked_add(v["size"].as_u64()?))
        .is_none_or(|sum| sum > 12 * 1024 * 1024)
    {
        return false;
    }
    if !manifest["default_logo"]
        .as_str()
        .is_some_and(|p| safe_asset(p) && assets.contains_key(p))
    {
        return false;
    }
    let Some(themes) = manifest["themes"].as_array().filter(|t| t.len() == 6) else {
        return false;
    };
    let mut ids = std::collections::BTreeSet::new();
    let date = |v: &Value| -> Option<u16> {
        let parts = v.as_array().filter(|a| a.len() == 2)?;
        let month = parts[0].as_u64()?;
        let day = parts[1].as_u64()?;
        let maximum = *([31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
            .get(month.checked_sub(1)? as usize)?);
        (day >= 1 && day <= maximum).then_some((month * 32 + day) as u16)
    };
    let palette = |v: &Value| {
        v.as_object().is_some_and(|p| {
            p.iter().all(|(name, color)| {
                matches!(
                    name.as_str(),
                    "bg" | "panel"
                        | "raised"
                        | "text"
                        | "muted"
                        | "accent"
                        | "button"
                        | "sidebar"
                        | "line"
                ) && color.as_str().is_some_and(|c| {
                    matches!(c.len(), 7 | 9)
                        && c.starts_with('#')
                        && c.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
                })
            })
        })
    };
    let mut ranges = Vec::new();
    for theme in themes {
        let Some(id) = theme["id"].as_str() else {
            return false;
        };
        if !matches!(
            id,
            "spring" | "summer" | "autumn" | "winter" | "christmas" | "halloween"
        ) || !ids.insert(id)
        {
            return false;
        }
        let (Some(start), Some(end), Some(priority)) = (
            date(&theme["start"]),
            date(&theme["end"]),
            theme["priority"].as_i64(),
        ) else {
            return false;
        };
        if !(-1000..=1000).contains(&priority) {
            return false;
        }
        ranges.push((start, end, priority));
        for tone in ["light", "dark"] {
            let variant = &theme[tone];
            if !palette(&variant["palette"])
                || !palette(&variant["sidebar"])
                || !variant["logo"]
                    .as_str()
                    .is_some_and(|p| safe_asset(p) && assets.contains_key(p))
            {
                return false;
            }
            if let Some(background) = variant.get("background")
                && !background
                    .as_str()
                    .is_some_and(|p| safe_asset(p) && assets.contains_key(p))
            {
                return false;
            }
        }
    }
    // Require one unambiguous winner on every day, including leap day and
    // ranges crossing New Year; calendar errors must not commit a bad pack.
    for (month, days) in [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        .into_iter()
        .enumerate()
    {
        for day in 1..=days {
            let point = ((month + 1) * 32 + day) as u16;
            let mut active = std::collections::BTreeSet::new();
            for &(start, end, priority) in &ranges {
                if ((start <= end && point >= start && point <= end)
                    || (start > end && (point >= start || point <= end)))
                    && !active.insert(priority)
                {
                    return false;
                }
            }
            if active.is_empty() {
                return false;
            }
        }
    }
    true
}

fn read_directory(root: &Path) -> Option<Value> {
    let path = root.join("manifest.json");
    if path.canonicalize().ok()? != path || std::fs::metadata(&path).ok()?.len() > 64 * 1024 {
        return None;
    }
    let manifest: Value = chill_updates::parse_strict_json(&std::fs::read(path).ok()?).ok()?;
    let declared = manifest["assets"].as_object()?;
    // Theme signing authority is data-only. No undeclared file, active content
    // or unexpected directory is permitted even when all PNG hashes are valid.
    let mut directories = vec![root.to_owned()];
    let mut entries = 0;
    while let Some(directory) = directories.pop() {
        for entry in std::fs::read_dir(directory).ok()? {
            let entry = entry.ok()?;
            entries += 1;
            if entries > 132 {
                return None;
            }
            let path = entry.path();
            let relative = path.strip_prefix(root).ok()?.to_str()?;
            let kind = entry.file_type().ok()?;
            if kind.is_dir() {
                if !matches!(relative, "assets" | "assets/logos" | "assets/backgrounds") {
                    return None;
                }
                directories.push(path);
            } else if !kind.is_file()
                || (relative != "manifest.json"
                    && (!safe_asset(relative) || !declared.contains_key(relative)))
            {
                return None;
            }
        }
    }
    materialize(manifest, |name| {
        let file = root.join(name);
        if file.canonicalize().ok()? != file
            || std::fs::metadata(&file).ok()?.len() > 5 * 1024 * 1024
        {
            return None;
        }
        std::fs::read(file).ok()
    })
}

pub fn validate_directory(root: &Path) -> bool {
    root.canonicalize()
        .ok()
        .and_then(|root| read_directory(&root))
        .is_some()
}

pub fn load(directory: &Path) -> Value {
    // This location is owned by the signed local theme installer. Canonical
    // equality rejects symlinks in every asset component before reading it.
    let root = directory.join("updates/themes/current");
    let external = (|| {
        let canonical = root.canonicalize().ok()?;
        if let Ok(target) = std::fs::read_link(&root) {
            if target.components().count() != 2
                || target
                    .components()
                    .any(|part| !matches!(part, std::path::Component::Normal(_)))
                || !target.starts_with("releases")
                || canonical != root.parent()?.canonicalize().ok()?.join(target)
            {
                return None;
            }
        } else if canonical != root {
            return None;
        }
        read_directory(&canonical)
    })();
    external.unwrap_or_else(|| {
        materialize(
            serde_json::from_str(EMBEDDED).expect("bundled manifest"),
            |name| {
                ASSETS
                    .iter()
                    .find(|(path, _)| *path == name)
                    .map(|(_, bytes)| bytes.to_vec())
            },
        )
        .expect("bundled theme integrity")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundled_theme_hashes_and_fallback_are_valid() {
        let value = load(Path::new("/nonexistent/chillcenter-theme-test"));
        assert_eq!(value["manifest"]["version"], "0.7.0");
        assert_eq!(value["assets"].as_object().unwrap().len(), 15);
    }
    #[test]
    fn only_known_png_names_are_read() {
        for path in [
            "../x.png",
            "assets/logos/../../x.png",
            "https://example.invalid/x.png",
            "assets/logos/x.svg",
        ] {
            assert!(!safe_asset(path));
        }
        assert!(safe_asset("assets/logos/default.png"));
    }
    #[test]
    fn invalid_calendar_tokens_and_references_are_refused() {
        let baseline: Value = serde_json::from_str(EMBEDDED).unwrap();
        assert!(valid_metadata(&baseline));
        for pointer in [
            "/themes/0/start",
            "/themes/0/light/palette/bg",
            "/themes/0/light/logo",
        ] {
            let mut invalid = baseline.clone();
            *invalid.pointer_mut(pointer).unwrap() = json!("url(https://invalid.example)");
            assert!(!valid_metadata(&invalid));
        }
    }
}
