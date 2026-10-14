//! Read-only static files for the same-origin web client (ADR-0012).
//!
//! Files live under `<data_dir>/web`. No directory means no web client: every path
//! answers 404 and the service behaves exactly as before. No listing, no traversal,
//! no symbolic links, closed MIME table, bounded size.
use crate::api::{ApiError, AppState};
use axum::{
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    response::Response,
};
use std::path::Path;

const MAX_FILE_BYTES: u64 = 32 * 1024 * 1024;
const PAGE_POLICY: &str = "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' blob: data:; media-src 'self' blob:; worker-src 'self' blob:; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'";

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

pub async fn serve(
    State(state): State<AppState>,
    request: Request<Body>,
) -> Result<Response, ApiError> {
    let not_found = || ApiError(StatusCode::NOT_FOUND, "NOT_FOUND");
    if request.method() != Method::GET && request.method() != Method::HEAD {
        return Err(ApiError(
            StatusCode::METHOD_NOT_ALLOWED,
            "METHOD_NOT_ALLOWED",
        ));
    }
    let head = request.method() == Method::HEAD;
    let name = relative_name(request.uri().path()).ok_or_else(not_found)?;
    let kind = content_type(&name).ok_or_else(not_found)?;
    let root = state.config.data_dir.join("web");
    let lookup = name.clone();
    let bytes = tokio::task::spawn_blocking(move || read_file(&root, &lookup))
        .await
        .map_err(|_| ApiError(StatusCode::SERVICE_UNAVAILABLE, "BUSY"))?
        .ok_or_else(not_found)?;
    let volatile = matches!(
        name.as_str(),
        "index.html" | "sw.js" | "manifest.webmanifest"
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
    }
}
