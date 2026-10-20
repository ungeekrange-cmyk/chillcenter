use crate::{
    api::{AppState, DatabaseActor, router},
    config::{LibrarySettings, ServerConfig},
};
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use chill_domain::{
    AcquirePlayback, PlaybackControl, PlaybackGrant, Progress, ReleasePlayback, TransferAck,
    TransferConfirmation, TransferProposal,
};
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
    database: DatabaseActor,
    bytes: Vec<u8>,
    playback: crate::playback::PlaybackService,
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
    let playback = crate::playback::PlaybackService::default();
    let database = db.clone();
    let state = AppState {
        tmdb_jobs: Arc::new(crate::tmdb_api::TmdbJobService::default()),
        playback: playback.clone(),
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
        database,
        bytes,
        playback,
    }
}

#[tokio::test]
async fn missing_indexed_leaf_marks_only_that_media_unavailable_and_never_grants_a_lease() {
    let f = fixture().await;
    let id = f.media[0].clone();
    std::fs::remove_file(f.root.join("FilmA.mp4")).unwrap();
    let (status, body) = json_reply(
        &f.app,
        "POST",
        &format!(
            "/api/2026-10-14/profiles/{}/playback/acquire",
            f.profiles[0]
        ),
        json!({"media_id":id,"device_id":uuid::Uuid::new_v4().to_string()}),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["code"], "MEDIA_FILE_MISSING");
    assert!(body.get("lease").is_none());
    let (missing, untouched) = f
        .database
        .call({
            let profile = f.profiles[0].clone();
            let id = id.clone();
            let other = f.media[1].clone();
            move |store| Ok((store.media(&profile, &id)?, store.media(&profile, &other)?))
        })
        .await
        .unwrap();
    assert!(!missing.available);
    assert!(untouched.available);
}

#[tokio::test]
async fn release_reclaims_all_unpolled_stream_slots() {
    let f = fixture().await;
    let g = grant(&f, 0, 0).await;
    let mut responses = Vec::new();
    for _ in 0..8 {
        responses.push(
            f.app
                .clone()
                .oneshot(stream_request(&g, "bytes=0-"))
                .await
                .unwrap(),
        );
    }
    assert_eq!(f.playback.streams.available_permits(), 0);
    let release = ReleasePlayback {
        lease: ctrl(&g),
        progress: None,
    };
    assert_eq!(
        json_reply(
            &f.app,
            "POST",
            &format!(
                "/api/2026-10-14/profiles/{}/playback/release",
                f.profiles[0]
            ),
            serde_json::to_value(release).unwrap()
        )
        .await
        .0,
        StatusCode::OK
    );
    await_free_stream_slots(&f).await;
    // Stale bodies must not resume after capacity has been returned.
    for r in responses {
        assert!(r.into_body().collect().await.is_err());
    }
}

async fn await_free_stream_slots(f: &Fixture) {
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while f.playback.streams.available_permits() != 8 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("capacity must recover without polling stale bodies");
}

#[tokio::test]
async fn root_change_and_profile_deletion_reclaim_unpolled_streams() {
    for delete in [false, true] {
        let f = fixture().await;
        let g = grant(&f, 0, 0).await;
        let mut responses = Vec::new();
        for _ in 0..8 {
            responses.push(
                f.app
                    .clone()
                    .oneshot(stream_request(&g, "bytes=0-"))
                    .await
                    .unwrap(),
            );
        }
        assert_eq!(f.playback.streams.available_permits(), 0);
        if delete {
            let request = Request::builder()
                .method("DELETE")
                .uri(format!("/api/2026-10-14/profiles/{}", f.profiles[0]))
                .header("host", "localhost:8443")
                .header("x-chill-confirm-delete", &f.profiles[0])
                .body(Body::empty())
                .unwrap();
            assert_eq!(
                f.app.clone().oneshot(request).await.unwrap().status(),
                StatusCode::OK
            );
        } else {
            assert_eq!(
                json_reply(
                    &f.app,
                    "PUT",
                    "/api/2026-10-14/admin/library",
                    json!({"video_root":null,"expected_revision":0})
                )
                .await
                .0,
                StatusCode::OK
            );
        }
        await_free_stream_slots(&f).await;
        for r in responses {
            assert!(r.into_body().collect().await.is_err());
        }
    }
}

#[tokio::test]
async fn idle_streams_reclaim_capacity_even_if_the_lease_stays_active() {
    let f = fixture().await;
    let g = grant(&f, 0, 0).await;
    let mut responses = Vec::new();
    for _ in 0..8 {
        responses.push(
            f.app
                .clone()
                .oneshot(stream_request(&g, "bytes=0-"))
                .await
                .unwrap(),
        );
    }
    assert_eq!(f.playback.streams.available_permits(), 0);
    tokio::time::timeout(std::time::Duration::from_secs(7), async {
        while f.playback.streams.available_permits() != 8 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("independent five-second idle deadline");
    let path = format!(
        "/api/2026-10-14/profiles/{}/playback/heartbeat",
        f.profiles[0]
    );
    assert_eq!(
        json_reply(
            &f.app,
            "POST",
            &path,
            serde_json::to_value(ctrl(&g)).unwrap()
        )
        .await
        .0,
        StatusCode::OK
    );
    let other = grant(&f, 1, 0).await;
    let r = f
        .app
        .clone()
        .oneshot(stream_request(&other, "bytes=0-9"))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        &r.into_body().collect().await.unwrap().to_bytes()[..],
        &f.bytes[..10]
    );
    for r in responses {
        assert!(r.into_body().collect().await.is_err());
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
        &format!(
            "/api/2026-10-14/profiles/{}/playback/acquire",
            f.profiles[p]
        ),
        serde_json::to_value(body).unwrap(),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    serde_json::from_value(v).unwrap()
}

#[tokio::test]
async fn streaming_body_slots_are_bounded_and_released_on_disconnect() {
    let f = fixture().await;
    let g = grant(&f, 0, 0).await;
    let mut responses = Vec::new();
    for _ in 0..8 {
        let r = f
            .app
            .clone()
            .oneshot(stream_request(&g, "bytes=0-"))
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::PARTIAL_CONTENT);
        responses.push(r);
    }
    let r = f
        .app
        .clone()
        .oneshot(stream_request(&g, "bytes=0-"))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::SERVICE_UNAVAILABLE);
    let error: Value =
        serde_json::from_slice(&r.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(error["code"], "MEDIA_BUSY");
    drop(responses.pop());
    let r = f
        .app
        .clone()
        .oneshot(stream_request(&g, "bytes=0-9"))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        &r.into_body().collect().await.unwrap().to_bytes()[..],
        &f.bytes[..10]
    );
}

#[tokio::test]
async fn five_real_seconds_at_double_speed_accumulate_across_two_leases() {
    let f = fixture().await;
    for session in 0..2 {
        let g = grant(&f, 0, 0).await;
        assert_eq!(g.personal.final_watched_ms, session * 5_000);
        let path = format!(
            "/api/2026-10-14/profiles/{}/playback/progress",
            f.profiles[0]
        );
        let mut e = event(&g, 1, 90_000);
        e.playing = true;
        e.seek = false;
        e.speed = 2.0;
        assert_eq!(
            json_reply(&f.app, "POST", &path, serde_json::to_value(&e).unwrap())
                .await
                .0,
            StatusCode::OK
        );
        tokio::time::sleep(std::time::Duration::from_millis(5_010)).await;
        e.sequence = 2;
        e.position_ms = 100_000;
        e.final_elapsed_ms = 5_000;
        let (status, saved) =
            json_reply(&f.app, "POST", &path, serde_json::to_value(&e).unwrap()).await;
        assert_eq!(status, StatusCode::OK, "{saved}");
        assert_eq!(saved["final_watched_ms"], (session + 1) * 5_000);
        assert_eq!(saved["watched"], session == 1);
        let release = ReleasePlayback {
            lease: ctrl(&g),
            progress: None,
        };
        assert_eq!(
            json_reply(
                &f.app,
                "POST",
                &format!(
                    "/api/2026-10-14/profiles/{}/playback/release",
                    f.profiles[0]
                ),
                serde_json::to_value(release).unwrap()
            )
            .await
            .0,
            StatusCode::OK
        );
    }
    let (_, other) = json_reply(
        &f.app,
        "GET",
        &format!(
            "/api/2026-10-14/profiles/{}/media/{}/state",
            f.profiles[1], f.media[0]
        ),
        Value::Null,
    )
    .await;
    assert_eq!(other["final_watched_ms"], 0);
    assert_eq!(other["watched"], false);
}

#[tokio::test]
async fn short_video_eof_works_with_unchanged_native_and_web_payloads_on_both_api_bases() {
    for (base, via_release) in [("/api/2026-10-14", false), ("/api/2026-10-14", true)] {
        let f = fixture().await;
        let g = grant(&f, 0, 0).await;
        let path = format!("{base}/profiles/{}/playback/progress", f.profiles[0]);
        let mut e = event(&g, 1, 900);
        e.duration_ms = 1_000;
        e.playing = true;
        e.seek = false;
        assert_eq!(
            json_reply(&f.app, "POST", &path, serde_json::to_value(&e).unwrap())
                .await
                .0,
            StatusCode::OK
        );
        tokio::time::sleep(std::time::Duration::from_millis(110)).await;
        e.sequence = 2;
        e.position_ms = 1_000;
        // Native EOF reports playing; web EOF reports paused with zero credit.
        e.playing = !via_release;
        e.final_elapsed_ms = if via_release { 0 } else { 100 };
        let (status, reply) = if via_release {
            json_reply(
                &f.app,
                "POST",
                &format!("{base}/profiles/{}/playback/release", f.profiles[0]),
                serde_json::to_value(ReleasePlayback {
                    lease: ctrl(&g),
                    progress: Some(e),
                })
                .unwrap(),
            )
            .await
        } else {
            json_reply(&f.app, "POST", &path, serde_json::to_value(e).unwrap()).await
        };
        assert_eq!(status, StatusCode::OK, "{reply}");
        let (_, saved) = json_reply(
            &f.app,
            "GET",
            &format!(
                "{base}/profiles/{}/media/{}/state",
                f.profiles[0], f.media[0]
            ),
            Value::Null,
        )
        .await;
        assert_eq!(saved["watched"], true, "{base}: {saved}");
        assert_eq!(
            saved["final_watched_ms"],
            if via_release { 0 } else { 100 },
            "real-time counter is not inflated to ten seconds"
        );
        let (_, other) = json_reply(
            &f.app,
            "GET",
            &format!(
                "{base}/profiles/{}/media/{}/state",
                f.profiles[1], f.media[0]
            ),
            Value::Null,
        )
        .await;
        assert_eq!(other["watched"], false);
    }
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
        .uri(format!("/api/2026-10-14/media/{}/stream", g.lease.media_id))
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
        &format!(
            "/api/2026-10-14/profiles/{}/playback/acquire",
            f.profiles[0]
        ),
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
    *bad.uri_mut() = format!("/api/2026-10-14/media/{}/stream", f.media[1])
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
async fn transfer_endpoints_require_confirmation_then_ack_once() {
    let f = fixture().await;
    let old = grant(&f, 0, 0).await;
    let base = format!(
        "/api/2026-10-14/profiles/{}/playback/transfer",
        f.profiles[0]
    );
    let proposal = TransferProposal {
        target: AcquirePlayback {
            media_id: f.media[1].clone(),
            device_id: uuid::Uuid::new_v4().to_string(),
            context: Default::default(),
        },
    };
    let (status, offer) = json_reply(
        &f.app,
        "POST",
        &format!("{base}/propose"),
        serde_json::to_value(proposal).unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{offer}");
    // A proposal is refusal-safe: A remains active until confirmation.
    assert_eq!(
        json_reply(
            &f.app,
            "POST",
            &format!(
                "/api/2026-10-14/profiles/{}/playback/heartbeat",
                f.profiles[0]
            ),
            serde_json::to_value(ctrl(&old)).unwrap(),
        )
        .await
        .0,
        StatusCode::OK
    );
    let token = offer["token"].as_str().unwrap().to_owned();
    assert_eq!(
        json_reply(
            &f.app,
            "POST",
            &format!(
                "/api/2026-10-14/profiles/{}/playback/transfer/confirm",
                f.profiles[1]
            ),
            serde_json::to_value(TransferConfirmation {
                token: token.clone()
            })
            .unwrap(),
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        json_reply(
            &f.app,
            "POST",
            &format!("{base}/confirm"),
            serde_json::to_value(TransferConfirmation {
                token: token.clone()
            })
            .unwrap(),
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        json_reply(
            &f.app,
            "POST",
            &format!(
                "/api/2026-10-14/profiles/{}/playback/heartbeat",
                f.profiles[0]
            ),
            serde_json::to_value(ctrl(&old)).unwrap(),
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    // A reconnect after confirmation gets a snapshot rather than relying on
    // having observed the original broadcast.
    let sse = Request::builder()
        .uri(format!(
            "/api/2026-10-14/profiles/{}/playback/events?session={}&generation={}",
            f.profiles[0], old.lease.session_id, old.lease.generation
        ))
        .header("host", "localhost:8443")
        .body(Body::empty())
        .unwrap();
    let response = f.app.clone().oneshot(sse).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let frame = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        response.into_body().frame(),
    )
    .await
    .expect("pending pause snapshot must not wait for keepalive")
    .unwrap()
    .unwrap()
    .into_data()
    .unwrap();
    let pause = std::str::from_utf8(&frame).unwrap();
    assert!(pause.contains("event: pause"));
    assert!(pause.contains(&token));
    let ack = TransferAck {
        token: token.clone(),
        lease: ctrl(&old),
        progress: event(&old, 1, 12_345),
    };
    let (status, activated) = json_reply(
        &f.app,
        "POST",
        &format!("{base}/ack"),
        serde_json::to_value(&ack).unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{activated}");
    assert_eq!(activated["phase"], "activated");
    assert!(activated["personal"].is_object());
    assert_eq!(
        json_reply(
            &f.app,
            "POST",
            &format!("{base}/ack"),
            serde_json::to_value(ack).unwrap()
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
}

#[tokio::test]
async fn ordered_resume_release_rewind_and_late_device() {
    let f = fixture().await;
    let a = grant(&f, 0, 0).await;
    let p = &f.profiles[0];
    let path = format!("/api/2026-10-14/profiles/{p}/playback/progress");
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
        &format!(
            "/api/2026-10-14/profiles/{}/playback/progress",
            f.profiles[1]
        ),
        serde_json::to_value(event(&a, 2, 20_000)).unwrap(),
    )
    .await;
    assert_eq!(s, StatusCode::CONFLICT);
    let (s, _) = json_reply(
        &f.app,
        "POST",
        &format!("/api/2026-10-14/profiles/{p}/playback/release"),
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
        &format!("/api/2026-10-14/profiles/{p}/playback/release"),
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
            "/api/2026-10-14/profiles/{}/media/{}/state",
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
        &format!(
            "/api/2026-10-14/profiles/{}/playback/release",
            f.profiles[0]
        ),
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
        "/api/2026-10-14/admin/library",
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
            &format!(
                "/api/2026-10-14/profiles/{}/playback/heartbeat",
                f.profiles[0]
            ),
            serde_json::to_value(ctrl(&a)).unwrap()
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let b = grant(&f, 0, 0).await;
    let mut req = request(
        "DELETE",
        &format!("/api/2026-10-14/profiles/{}", f.profiles[0]),
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
