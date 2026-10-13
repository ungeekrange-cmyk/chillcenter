use crate::{
    api::{AppState, DatabaseActor, router},
    config::{LibrarySettings, ServerConfig},
};
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use chill_domain::{AcquirePlayback, PlaybackControl, PlaybackGrant, Progress, ReleasePlayback};
use futures_util::StreamExt;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tower::ServiceExt;

struct Fixture {
    _dir: tempfile::TempDir,
    app: Router,
    profiles: Vec<String>,
    media: Vec<String>,
    root: PathBuf,
    bytes: Vec<u8>,
}
async fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().canonicalize().unwrap();
    let root = base.join("videos");
    std::fs::create_dir(&root).unwrap();
    let bytes: Vec<_> = (0..300_000).map(|n| (n % 251) as u8).collect();
    std::fs::write(root.join("FilmA.mp4"), &bytes).unwrap();
    std::fs::write(root.join("FilmB.mp4"), b"second").unwrap();
    let config = ServerConfig {
        data_dir: base.join("data"),
        video_root: Some(root.clone()),
        ..Default::default()
    };
    std::fs::create_dir(&config.data_dir).unwrap();
    let cert = crate::tls::ensure(&config, time::OffsetDateTime::now_utc()).unwrap();
    let db = DatabaseActor::open(config.data_dir.join("catalog.sqlite")).unwrap();
    let roots = root.clone();
    let (profiles, media) = db
        .call(move |store| {
            let a = store.create_profile("Alice")?;
            let b = store.create_profile("Bob")?;
            store.snapshot(
                ["FilmA.mp4", "FilmB.mp4"]
                    .iter()
                    .map(|n| chill_catalog::scanner::parse_file(&roots, &roots.join(n)))
                    .collect(),
            )?;
            let media = store.catalog(&a.id)?.into_iter().map(|m| m.id).collect();
            Ok((vec![a.id, b.id], media))
        })
        .await
        .unwrap();
    let state = AppState {
        playback: Default::default(),
        database: db,
        library: Arc::new(Mutex::new(LibrarySettings::load(&config).unwrap())),
        config: Arc::new(config),
        certificate: Arc::new(Mutex::new(cert.schedule)),
        scan: Arc::new(Mutex::new(Default::default())),
    };
    Fixture {
        _dir: dir,
        app: router(state),
        profiles,
        media,
        root,
        bytes,
    }
}
fn request(method: &str, path: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header("host", "localhost:8443")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}
async fn json_reply(app: &Router, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
    let r = app
        .clone()
        .oneshot(request(method, path, body))
        .await
        .unwrap();
    let status = r.status();
    let bytes = r.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap())
}
async fn grant(f: &Fixture, p: usize, m: usize) -> PlaybackGrant {
    let body = AcquirePlayback {
        media_id: f.media[m].clone(),
        device_id: uuid::Uuid::new_v4().to_string(),
        context: Default::default(),
    };
    let (s, v) = json_reply(
        &f.app,
        "POST",
        &format!("/api/v1/profiles/{}/playback/acquire", f.profiles[p]),
        serde_json::to_value(body).unwrap(),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    serde_json::from_value(v).unwrap()
}
fn ctrl(g: &PlaybackGrant) -> PlaybackControl {
    PlaybackControl {
        session_id: g.lease.session_id.clone(),
        generation: g.lease.generation,
        media_id: g.lease.media_id.clone(),
        media_revision: g.lease.media_revision,
    }
}
fn event(g: &PlaybackGrant, seq: u64, pos: u64) -> Progress {
    Progress {
        session_id: g.lease.session_id.clone(),
        generation: g.lease.generation,
        media_id: g.lease.media_id.clone(),
        media_revision: g.lease.media_revision,
        sequence: seq,
        position_ms: pos,
        duration_ms: 100_000,
        speed: 1.0,
        final_elapsed_ms: 0,
        playing: false,
        buffering: false,
        seek: true,
    }
}
fn stream_request(g: &PlaybackGrant, range: &str) -> Request<Body> {
    Request::builder()
        .uri(format!("/api/v1/media/{}/stream", g.lease.media_id))
        .header("host", "localhost:8443")
        .header("x-chill-session", &g.lease.session_id)
        .header("x-chill-generation", g.lease.generation)
        .header("range", range)
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn two_profiles_range_and_active_conflict() {
    let f = fixture().await;
    let a = grant(&f, 0, 0).await;
    let b = grant(&f, 1, 0).await;
    assert_ne!(a.lease.session_id, b.lease.session_id);
    let (s, v) = json_reply(
        &f.app,
        "POST",
        &format!("/api/v1/profiles/{}/playback/acquire", f.profiles[0]),
        json!({"media_id":f.media[0],"device_id":uuid::Uuid::new_v4().to_string()}),
    )
    .await;
    assert_eq!(s, StatusCode::CONFLICT);
    assert_eq!(v["code"], "PLAYBACK_ACTIVE");
    let r = f
        .app
        .clone()
        .oneshot(stream_request(&a, "bytes=11-211"))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(r.headers()["content-range"], "bytes 11-211/300000");
    assert_eq!(r.headers()["cache-control"], "no-store");
    assert!(r.headers().contains_key("etag"));
    assert_eq!(
        r.into_body().collect().await.unwrap().to_bytes().as_ref(),
        &f.bytes[11..212]
    );
    let mut bad = stream_request(&a, "bytes=0-3");
    *bad.uri_mut() = format!("/api/v1/media/{}/stream", f.media[1])
        .parse()
        .unwrap();
    assert_eq!(
        f.app.clone().oneshot(bad).await.unwrap().status(),
        StatusCode::CONFLICT
    );
    let r = f
        .app
        .clone()
        .oneshot(stream_request(&a, "bytes=400000-"))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::RANGE_NOT_SATISFIABLE);
    assert_eq!(r.headers()["content-range"], "bytes */300000");
    let mut bad = stream_request(&a, "bytes=0-3");
    bad.headers_mut()
        .insert("if-match", "\"wrong\"".parse().unwrap());
    assert_eq!(
        f.app.clone().oneshot(bad).await.unwrap().status(),
        StatusCode::PRECONDITION_FAILED
    );
}

#[tokio::test]
async fn ordered_resume_release_rewind_and_late_device() {
    let f = fixture().await;
    let a = grant(&f, 0, 0).await;
    let p = &f.profiles[0];
    let path = format!("/api/v1/profiles/{p}/playback/progress");
    let (s, v) = json_reply(
        &f.app,
        "POST",
        &path,
        serde_json::to_value(event(&a, 1, 12_345)).unwrap(),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["position_ms"], 12_345);
    assert_eq!(v["watched"], false);
    let (s, _) = json_reply(
        &f.app,
        "POST",
        &path,
        serde_json::to_value(event(&a, 1, 20_000)).unwrap(),
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    let (s, _) = json_reply(
        &f.app,
        "POST",
        &format!("/api/v1/profiles/{}/playback/progress", f.profiles[1]),
        serde_json::to_value(event(&a, 2, 20_000)).unwrap(),
    )
    .await;
    assert_eq!(s, StatusCode::CONFLICT);
    let (s, _) = json_reply(
        &f.app,
        "POST",
        &format!("/api/v1/profiles/{p}/playback/release"),
        serde_json::to_value(ReleasePlayback {
            lease: ctrl(&a),
            progress: None,
        })
        .unwrap(),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let b = grant(&f, 0, 0).await;
    assert_eq!(b.personal.position_ms, 12_345);
    assert!(b.lease.generation > a.lease.generation);
    assert_eq!(
        json_reply(
            &f.app,
            "POST",
            &path,
            serde_json::to_value(event(&a, 999, 99_000)).unwrap()
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let (s, _) = json_reply(
        &f.app,
        "POST",
        &format!("/api/v1/profiles/{p}/playback/release"),
        serde_json::to_value(ReleasePlayback {
            lease: ctrl(&b),
            progress: Some(event(&b, 1, 0)),
        })
        .unwrap(),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(grant(&f, 0, 0).await.personal.position_ms, 0);
    let (s, v) = json_reply(
        &f.app,
        "GET",
        &format!(
            "/api/v1/profiles/{}/media/{}/state",
            f.profiles[1], f.media[0]
        ),
        Value::Null,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["position_ms"], 0);
}

#[tokio::test]
async fn release_revokes_remaining_body_and_root_change_revokes_all() {
    let f = fixture().await;
    let a = grant(&f, 0, 0).await;
    let r = f
        .app
        .clone()
        .oneshot(stream_request(&a, "bytes=0-299999"))
        .await
        .unwrap();
    let mut chunks = r.into_body().into_data_stream();
    assert_eq!(chunks.next().await.unwrap().unwrap().len(), 64 * 1024);
    let (s, _) = json_reply(
        &f.app,
        "POST",
        &format!("/api/v1/profiles/{}/playback/release", f.profiles[0]),
        serde_json::to_value(ReleasePlayback {
            lease: ctrl(&a),
            progress: None,
        })
        .unwrap(),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert!(chunks.next().await.unwrap().is_err());
    let a = grant(&f, 0, 0).await;
    let (s, _) = json_reply(
        &f.app,
        "PUT",
        "/api/v1/admin/library",
        json!({"video_root":f.root,"expected_revision":0}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(
        f.app
            .clone()
            .oneshot(stream_request(&a, "bytes=0-3"))
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
}

#[tokio::test]
async fn changed_file_fails_closed_and_profile_deletion_revokes() {
    let f = fixture().await;
    let a = grant(&f, 0, 0).await;
    let response = f
        .app
        .clone()
        .oneshot(stream_request(&a, "bytes=0-299999"))
        .await
        .unwrap();
    std::fs::write(f.root.join("FilmA.mp4"), b"replacement").unwrap();
    assert!(response.into_body().collect().await.is_err());
    assert_eq!(
        json_reply(
            &f.app,
            "POST",
            &format!("/api/v1/profiles/{}/playback/heartbeat", f.profiles[0]),
            serde_json::to_value(ctrl(&a)).unwrap()
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let b = grant(&f, 0, 0).await;
    let mut req = request(
        "DELETE",
        &format!("/api/v1/profiles/{}", f.profiles[0]),
        Value::Null,
    );
    req.headers_mut()
        .insert("x-chill-confirm-delete", f.profiles[0].parse().unwrap());
    assert_eq!(
        f.app.clone().oneshot(req).await.unwrap().status(),
        StatusCode::OK
    );
    assert_eq!(
        f.app
            .clone()
            .oneshot(stream_request(&b, "bytes=0-3"))
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
}
