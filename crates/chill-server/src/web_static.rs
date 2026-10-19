//! Read-only static files for the same-origin web client (ADR-0014).
//!
//! Files live in `web/` next to the server binary (`/usr/lib/chillcenter/web` once installed):
//! root-owned, read-only for the service, untouched by data backups and restores, replaced
//! independently through signed component slots. No directory means no web client: every path answers 404 and the
//! service behaves exactly as before. No listing, no traversal, no symbolic links, closed MIME
//! table, bounded size.
use crate::api::{ApiError, AppState};
use axum::{
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    response::{IntoResponse, Response},
};
use std::path::{Path, PathBuf};

#[cfg(test)]
thread_local! {
    static TEST_ROOT: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}
/// Tests point the handler at a temporary directory (runs on the test's own thread).
#[cfg(test)]
pub(crate) fn set_test_root(root: Option<PathBuf>) {
    TEST_ROOT.with(|r| *r.borrow_mut() = root);
}

fn web_root() -> PathBuf {
    #[cfg(test)]
    if let Some(root) = TEST_ROOT.with(|r| r.borrow().clone()) {
        return root;
    }
    let base = crate::updates_api::installation_root();
    let current = base.join("web/current");
    if current.is_dir() {
        current
    } else {
        base.join("web")
    }
}

const MAX_FILE_BYTES: u64 = 8 * 1024 * 1024; // whole file is read into memory: keep it small on a 1 GiB device
const PAGE_POLICY: &str = "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' blob: data:; media-src 'self' blob:; worker-src 'self' blob:; connect-src 'self'; object-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'";

fn content_type(name: &str) -> Option<&'static str> {
    Some(match name.rsplit_once('.')?.1 {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "webmanifest" => "application/manifest+json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "wasm" => "application/wasm",
        "woff2" => "font/woff2",
        "txt" => "text/plain; charset=utf-8",
        _ => return None,
    })
}

/// Maps a request path to a safe relative name, or `None`.
fn relative_name(path: &str) -> Option<String> {
    let name = if path == "/" {
        "index.html"
    } else {
        path.strip_prefix('/')?
    };
    let safe = !name.is_empty()
        && name.len() <= 200
        && !name.contains(['\\', '\0', '%'])
        && name
            .split('/')
            .all(|part| !part.is_empty() && !part.starts_with('.'));
    safe.then(|| name.to_owned())
}

fn read_file(root: &Path, name: &str) -> Option<Vec<u8>> {
    let root = root.canonicalize().ok()?;
    let expected = root.join(name);
    // A symbolic link anywhere in the path makes the canonical form differ.
    if expected.canonicalize().ok()? != expected {
        return None;
    }
    let metadata = std::fs::metadata(&expected).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
        return None;
    }
    std::fs::read(&expected).ok()
}

fn declared_version(root: &Path, name: &str, version: &str, themes: bool) -> bool {
    let filename = if themes {
        "manifest.json"
    } else {
        "build.json"
    };
    if !root
        .join(filename)
        .metadata()
        .is_ok_and(|m| m.len() <= 65536)
    {
        return false;
    }
    read_file(root, filename)
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .is_some_and(|manifest| {
            if themes {
                manifest["assets"][name]["sha256"] == version
            } else {
                manifest["build"] == version
            }
        })
}

fn theme_resource(root: &Path, name: &str) -> bool {
    if name == "manifest.json" {
        return true;
    }
    let Some(leaf) = name
        .strip_prefix("assets/logos/")
        .or_else(|| name.strip_prefix("assets/backgrounds/"))
        .and_then(|n| n.strip_suffix(".png"))
    else {
        return false;
    };
    if leaf.is_empty()
        || leaf.len() > 80
        || !leaf
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return false;
    }
    if !root
        .join("manifest.json")
        .metadata()
        .is_ok_and(|m| m.len() <= 65536)
    {
        return false;
    }
    read_file(root, "manifest.json")
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .is_some_and(|m| {
            let asset = &m["assets"][name];
            asset["size"]
                .as_u64()
                .is_some_and(|n| n > 0 && n <= 5 * 1024 * 1024)
                && asset["sha256"]
                    .as_str()
                    .is_some_and(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()))
        })
}

/// A page opened before activation keeps requesting its exact build. Never
/// silently return new JS under an old cache key. Only retained root-controlled
/// release directories are candidates; the query cannot supply a path.
fn resource_root(root: &Path, name: &str, version: Option<&str>, themes: bool) -> Option<PathBuf> {
    let Some(version) = version else {
        return (!themes || theme_resource(root, name)).then(|| root.to_owned());
    };
    if version.len() != if themes { 64 } else { 12 }
        || !version.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return None;
    }
    if (!themes || theme_resource(root, name)) && declared_version(root, name, version, themes) {
        return Some(root.to_owned());
    }
    let bundled = themes
        && root.file_name().is_some_and(|n| n == "themes")
        && root
            .parent()
            .and_then(|p| p.file_name())
            .is_some_and(|n| n == "current");
    let slot_root = if bundled { root.parent()? } else { root };
    if slot_root.file_name()? != "current" {
        return None;
    }
    let releases = slot_root.parent()?.join("releases").canonicalize().ok()?;
    for entry in std::fs::read_dir(&releases).ok()?.take(128).flatten() {
        if !entry.file_type().ok()?.is_dir() {
            continue;
        }
        let candidate = if bundled {
            entry.path().join("themes")
        } else {
            entry.path()
        };
        if candidate.canonicalize().ok()? != candidate {
            continue;
        }
        if (!themes || theme_resource(&candidate, name))
            && declared_version(&candidate, name, version, themes)
        {
            return Some(candidate);
        }
    }
    None
}

pub async fn serve(
    State(_state): State<AppState>,
    request: Request<Body>,
) -> Result<Response, ApiError> {
    let not_found = || ApiError(StatusCode::NOT_FOUND, "NOT_FOUND");
    // An unknown API route keeps its previous behaviour for every method: a bare 404 that
    // `normalize_response` turns into the JSON `ROUTE_NOT_FOUND` envelope.
    let path = request.uri().path();
    if path == "/api" || path.starts_with("/api/") {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }
    if request.method() != Method::GET && request.method() != Method::HEAD {
        return Err(ApiError(
            StatusCode::METHOD_NOT_ALLOWED,
            "METHOD_NOT_ALLOWED",
        ));
    }
    let head = request.method() == Method::HEAD;
    let name = relative_name(request.uri().path()).ok_or_else(not_found)?;
    let kind = content_type(&name).ok_or_else(not_found)?;
    let version = request
        .uri()
        .query()
        .and_then(|q| q.strip_prefix("v="))
        .map(str::to_owned);
    let mut root = web_root();
    let mut lookup = name.clone();
    if let Some(asset) = name.strip_prefix("themes/") {
        let independent = crate::updates_api::installation_root().join("themes/current");
        if independent.is_dir() {
            root = independent;
            lookup = asset.to_owned();
        }
    }
    let bytes = tokio::task::spawn_blocking(move || {
        if let Some(asset) = lookup.strip_prefix("themes/") {
            // Bundled fallback themes have their own SHA query, not the web
            // build query. Independent packs use the same manifest contract.
            let themes = root.join("themes");
            let selected = resource_root(&themes, asset, version.as_deref(), true)?;
            read_file(&selected, asset)
        } else {
            let themes = root.file_name().is_some_and(|name| name == "current")
                && root
                    .parent()
                    .and_then(|p| p.file_name())
                    .is_some_and(|name| name == "themes");
            let selected = resource_root(&root, &lookup, version.as_deref(), themes)?;
            read_file(&selected, &lookup)
        }
    })
    .await
    .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "BUSY"))?
    .ok_or_else(not_found)?;
    let volatile = matches!(
        name.as_str(),
        "index.html" | "sw.js" | "manifest.webmanifest" | "build.json" | "themes/manifest.json"
    );
    let mut response = Response::builder()
        .status(StatusCode::OK)
        .header("content-type", kind)
        .header("content-length", bytes.len().to_string())
        .header("x-content-type-options", "nosniff")
        .header("referrer-policy", "no-referrer")
        .header(
            "cache-control",
            if volatile {
                "no-cache"
            } else {
                "public, max-age=3600"
            },
        );
    if kind.starts_with("text/html") {
        response = response.header("content-security-policy", PAGE_POLICY);
    }
    response
        .body(if head {
            Body::empty()
        } else {
            Body::from(bytes)
        })
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "BUSY"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_plain_relative_names_are_served() {
        assert_eq!(relative_name("/").as_deref(), Some("index.html"));
        assert_eq!(relative_name("/js/app.js").as_deref(), Some("js/app.js"));
        for bad in [
            "/../secret.json",
            "/a/../b.js",
            "/.env",
            "/a//b.js",
            "/a%2e%2e/b.js",
            "/a\\b.js",
            "/dir/",
        ] {
            assert_eq!(relative_name(bad), None, "{bad}");
        }
    }

    #[test]
    fn mime_table_is_closed() {
        assert_eq!(content_type("a.wasm"), Some("application/wasm"));
        assert_eq!(content_type("a.sqlite"), None);
        assert_eq!(content_type("noextension"), None);
    }

    #[cfg(unix)]
    #[test]
    fn opened_pages_resolve_their_retained_build_not_new_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().canonicalize().unwrap();
        for (slot, build) in [("old", "aaaaaaaaaaaa"), ("new", "bbbbbbbbbbbb")] {
            let root = base.join("releases").join(slot);
            std::fs::create_dir_all(&root).unwrap();
            std::fs::write(
                root.join("build.json"),
                format!("{{\"build\":\"{build}\"}}"),
            )
            .unwrap();
            std::fs::write(root.join("app.js"), slot).unwrap();
        }
        std::os::unix::fs::symlink("releases/new", base.join("current")).unwrap();
        let selected =
            resource_root(&base.join("current"), "app.js", Some("aaaaaaaaaaaa"), false).unwrap();
        assert_eq!(read_file(&selected, "app.js").unwrap(), b"old");
        assert!(
            resource_root(&base.join("current"), "app.js", Some("../../escape"), false).is_none()
        );
        assert!(
            resource_root(&base.join("current"), "app.js", Some("cccccccccccc"), false).is_none()
        );
    }

    #[cfg(unix)]
    #[test]
    fn opened_pages_resolve_retained_bundled_theme_hashes() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().canonicalize().unwrap();
        for (slot, hash) in [("old", "a".repeat(64)), ("new", "b".repeat(64))] {
            let themes = base.join("releases").join(slot).join("themes");
            std::fs::create_dir_all(themes.join("assets/logos")).unwrap();
            std::fs::write(
                themes.join("manifest.json"),
                format!(
                    "{{\"assets\":{{\"assets/logos/x.png\":{{\"size\":3,\"sha256\":\"{hash}\"}}}}}}"
                ),
            )
            .unwrap();
            std::fs::write(themes.join("assets/logos/x.png"), slot).unwrap();
        }
        std::os::unix::fs::symlink("releases/new", base.join("current")).unwrap();
        let selected = resource_root(
            &base.join("current/themes"),
            "assets/logos/x.png",
            Some(&"a".repeat(64)),
            true,
        )
        .unwrap();
        assert_eq!(read_file(&selected, "assets/logos/x.png").unwrap(), b"old");
    }

    #[test]
    fn theme_route_never_serves_active_or_undeclared_content() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("manifest.json"), b"{\"assets\":{}}").unwrap();
        for name in [
            "evil.js",
            "evil.css",
            "evil.html",
            "evil.svg",
            "assets/logos/undeclared.png",
        ] {
            assert!(
                resource_root(dir.path(), name, None, true).is_none(),
                "{name}"
            );
        }
        assert!(resource_root(dir.path(), "manifest.json", None, true).is_some());
    }

    #[cfg(unix)]
    #[test]
    fn symbolic_links_and_missing_roots_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("web");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("index.html"), b"ok").unwrap();
        std::fs::write(dir.path().join("secret.json"), b"{}").unwrap();
        std::os::unix::fs::symlink(dir.path().join("secret.json"), root.join("link.json")).unwrap();
        assert_eq!(read_file(&root, "index.html").as_deref(), Some(&b"ok"[..]));
        assert_eq!(read_file(&root, "link.json"), None);
        assert_eq!(read_file(&root, "missing.js"), None);
        assert_eq!(read_file(&dir.path().join("absent"), "index.html"), None);
        // A symlinked directory inside the web root is refused as well (not only a symlinked file).
        let outside = dir.path().join("outside");
        std::fs::create_dir(&outside).unwrap();
        std::fs::write(outside.join("secret.json"), b"{}").unwrap();
        std::os::unix::fs::symlink(&outside, root.join("dirlink")).unwrap();
        assert_eq!(read_file(&root, "dirlink/secret.json"), None);
    }
}
