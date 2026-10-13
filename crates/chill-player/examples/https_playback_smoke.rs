//! Writes only to a dedicated loopback fixture server, never to the household server.
use chill_client::{
    HttpMethod, ServerEndpoint, Transport, TrustStore, inspect_certificate,
    playback::{AcquirePlayback, PlaybackControl, Progress, ReleasePlayback},
};
use chill_player::{
    HttpMediaFactory, PlaybackStream, Player, PlayerEvent, PlayerOptions, TrackKind,
};
use serde_json::{Value, json};
use std::{
    error::Error,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 || args[2] != "--fixture-consent" {
        return Err("Usage: https_playback_smoke https://127.0.0.1:PORT --fixture-consent".into());
    }
    let endpoint = ServerEndpoint::parse("playback-fixture", &args[1])?;
    if endpoint.host() != "127.0.0.1" && endpoint.host() != "localhost" {
        return Err("This mutating smoke test accepts loopback fixtures only.".into());
    }
    let certificate = inspect_certificate(&args[1], Duration::from_secs(5))?;
    let mut trust = TrustStore::default();
    // The explicit fixture-consent flag approves this temporary test certificate.
    trust.approve_initial("playback-fixture", endpoint.host(), certificate);
    let transport = Transport::new(endpoint, Arc::new(RwLock::new(trust)), None)?;
    let runtime = tokio::runtime::Runtime::new()?;
    let (profile, other, media, grant) = runtime.block_on(async {
        let existing: Value = transport.get_json("/api/v1/profiles").await?;
        if existing.as_array().is_none_or(|p| !p.is_empty()) {
            return Err::<_, Box<dyn Error>>("Fixture must have an empty profiles list.".into());
        }
        let mut ids = Vec::new();
        for name in ["Playback fixture A", "Playback fixture B"] {
            let p: Value = transport
                .request_json(
                    HttpMethod::POST,
                    "/api/v1/profiles",
                    Some(&json!({"name":name})),
                )
                .await?;
            ids.push(p["id"].as_str().ok_or("profile id")?.to_owned());
        }
        let _: Value = transport
            .request_json::<(), Value>(HttpMethod::POST, "/api/v1/admin/scan", None)
            .await?;
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let s: Value = transport.get_json("/api/v1/admin/scan").await?;
            if s["running"] == false {
                if s["last_error"] != Value::Null {
                    return Err("Fixture scan failed.".into());
                }
                break;
            }
            if Instant::now() >= deadline {
                return Err("Fixture scan timeout.".into());
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        let page: Value = transport
            .get_json(&format!("/api/v1/profiles/{}/catalog", ids[0]))
            .await?;
        let media = page["items"][0]["id"]
            .as_str()
            .ok_or("catalog id")?
            .to_owned();
        let grant = transport
            .acquire_playback(
                &ids[0],
                &AcquirePlayback {
                    media_id: media.clone(),
                    device_id: "00000000-0000-4000-8000-000000000001".into(),
                    context: Default::default(),
                },
            )
            .await?;
        Ok((ids.remove(0), ids.remove(0), media, grant))
    })?;
    let control = PlaybackControl {
        session_id: grant.lease.session_id.clone(),
        generation: grant.lease.generation,
        media_id: media.clone(),
        media_revision: grant.lease.media_revision,
    };
    let mut bytes = [0u8; 64];
    let read = transport.read_media_range_blocking(
        &media,
        &control.session_id,
        control.generation,
        0,
        &mut bytes,
        None,
    )?;
    assert!(read.etag.is_some());
    assert_eq!(&bytes[4..8], b"ftyp", "fixture must be a real MP4");
    let factory = HttpMediaFactory::new(transport.clone());
    factory.set_playback(PlaybackStream {
        media_id: media.clone(),
        session_id: control.session_id.clone(),
        generation: control.generation,
    });
    let mut options = PlayerOptions::default();
    options.extra.insert("vo".into(), "null".into());
    options.extra.insert("ao".into(), "null".into());
    let mut player = Player::new(options)?;
    player.register_stream("chill-media", Arc::new(factory.clone()))?;
    player.load(&format!("chill-media://{media}"))?;
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match player.poll_event(Duration::from_millis(50)) {
            PlayerEvent::FileLoaded => break,
            PlayerEvent::Ended { error, .. } if error < 0 => {
                return Err(format!("libmpv load error {error}").into());
            }
            _ => {}
        }
        if Instant::now() >= deadline {
            return Err("libmpv HTTPS load timeout".into());
        }
    }
    player.pause(true)?;
    assert!(player.tracks()?.iter().any(|t| t.kind == TrackKind::Audio));
    assert!(
        player
            .tracks()?
            .iter()
            .any(|t| t.kind == TrackKind::Subtitle)
    );
    let duration_ms = (player.duration_seconds()?.ok_or("duration")? * 1000.0).round() as u64;
    player.seek_absolute(12.0)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let position_ms = loop {
        player.poll_event(Duration::from_millis(50));
        let position = player.position_seconds()?.unwrap_or_default();
        if (position - 12.0).abs() < 0.5 {
            break (position * 1000.0).round() as u64;
        }
        if Instant::now() >= deadline {
            return Err("seek timeout".into());
        }
    };
    let event = Progress {
        session_id: control.session_id.clone(),
        generation: control.generation,
        media_id: media.clone(),
        media_revision: control.media_revision,
        sequence: 1,
        position_ms,
        duration_ms,
        speed: 1.0,
        final_elapsed_ms: 0,
        playing: false,
        buffering: false,
        seek: true,
    };
    runtime.block_on(async {
        transport.heartbeat_playback(&profile, &control).await?;
        assert_eq!(
            transport
                .report_progress(&profile, &event)
                .await?
                .position_ms,
            position_ms
        );
        Ok::<_, Box<dyn Error>>(())
    })?;
    player.seek_absolute(duration_ms as f64 / 1000.0 - 0.5)?;
    player.pause(false)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let PlayerEvent::Ended { reason, error } = player.poll_event(Duration::from_millis(50)) {
            assert_eq!(reason, 0, "natural EOF expected");
            assert_eq!(error, 0, "EOF must not become a Range error");
            break;
        }
        if Instant::now() >= deadline {
            return Err("natural EOF timeout".into());
        }
    }
    runtime.block_on(async {
        transport
            .release_playback(
                &profile,
                &ReleasePlayback {
                    lease: control.clone(),
                    progress: None,
                },
            )
            .await?;
        let resumed = transport
            .acquire_playback(
                &profile,
                &AcquirePlayback {
                    media_id: media.clone(),
                    device_id: "00000000-0000-4000-8000-000000000002".into(),
                    context: Default::default(),
                },
            )
            .await?;
        assert_eq!(resumed.personal.position_ms, position_ms);
        assert!(resumed.lease.generation > control.generation);
        assert!(transport.report_progress(&profile, &event).await.is_err());
        assert_eq!(
            transport
                .playback_position(&other, &media)
                .await?
                .position_ms,
            0
        );
        transport
            .release_playback(
                &profile,
                &ReleasePlayback {
                    lease: PlaybackControl {
                        session_id: resumed.lease.session_id,
                        generation: resumed.lease.generation,
                        media_id: media.clone(),
                        media_revision: resumed.lease.media_revision,
                    },
                    progress: None,
                },
            )
            .await?;
        Ok::<_, Box<dyn Error>>(())
    })?;
    assert!(
        transport
            .read_media_range_blocking(
                &media,
                &control.session_id,
                control.generation,
                0,
                &mut bytes,
                None
            )
            .is_err()
    );
    factory.clear_playback();
    drop(player);
    println!(
        "HTTPS_PLAYBACK_OK: pinned TLS, real MP4/audio/subtitles, seek, natural EOF, heartbeat, resume across devices, profile isolation, stale lease refusal"
    );
    Ok(())
}
