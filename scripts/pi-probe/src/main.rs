// Operator-only controlled-data probe, not a product source modification.
// No filesystem NAS access, certificate bypass, or household profile edits.
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
type Result<T> = std::result::Result<T, Box<dyn Error>>;
const ADDRESS: &str = "https://192.168.1.42:8443";

fn catalog_path(profile: &str) -> String {
    format!("/api/v1/profiles/{profile}/catalog")
}

fn catalog_query() -> [(&'static str, String); 2] {
    [("limit", "100".into()), ("available", "true".into())]
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 || args[1] != "--authorized-pi-tests" {
        return Err(
            "Expected explicit Pi test consent, SSH-verified SPKI, prepare or play arguments"
                .into(),
        );
    }
    let cert = inspect_certificate(ADDRESS, Duration::from_secs(8))?;
    let normalize = |s: &str| s.replace(':', "").to_ascii_lowercase();
    if normalize(&cert.fingerprint_hex()) != normalize(&args[2]) {
        return Err("SSH-verified public certificate SPKI mismatch".into());
    }
    let endpoint = ServerEndpoint::parse("pi-operator-test", ADDRESS)?;
    let mut trust = TrustStore::default();
    trust.approve_initial("pi-operator-test", endpoint.host(), cert);
    let transport = Transport::new(endpoint, Arc::new(RwLock::new(trust)), None)?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    match args[3].as_str() {
        "prepare" => runtime.block_on(prepare(&transport)),
        "play" if args.len() == 7 => {
            play(&transport, &runtime, &args[4], &args[5], args[6].parse()?)
        }
        _ => Err("Invalid operator probe mode".into()),
    }
}
async fn prepare(t: &Transport) -> Result<()> {
    let identity: Value = t.get_json("/api/v1/server").await?;
    println!(
        "SERVER {}",
        json!({"version":identity["version"],"api":identity["api_version"]})
    );
    let old: Value = t.get_json("/api/v1/profiles").await?;
    // First-install target only: refuse a household populated server.
    if old.as_array().is_none_or(|a| !a.is_empty()) {
        return Err("Prepare requires new empty server; existing profiles preserved".into());
    }
    let stamp = uuid::Uuid::new_v4().simple().to_string();
    let mut profiles = Vec::new();
    for i in 0..3 {
        let p: Value = t
            .request_json(
                HttpMethod::POST,
                "/api/v1/profiles",
                Some(&json!({"name":format!("QA Pi {} {}",&stamp[..8],i+1)})),
            )
            .await?;
        profiles.push(p["id"].as_str().ok_or("Missing profile id")?.to_owned());
    }
    println!("OWNED_TEST_PROFILES {}", json!(profiles));
    let _: Value = t
        .request_json::<(), Value>(HttpMethod::POST, "/api/v1/admin/scan", None)
        .await?;
    let start = Instant::now();
    loop {
        let s: Value = t.get_json("/api/v1/admin/scan").await?;
        if s["running"] == false {
            if !s["last_error"].is_null() {
                return Err("NAS scan failed; previous catalogue preserved".into());
            }
            println!(
                "NAS_SCAN {}",
                json!({"seconds":start.elapsed().as_secs_f64(),"count":s["count"]})
            );
            break;
        }
        if start.elapsed() > Duration::from_secs(330) {
            return Err("NAS scan timeout".into());
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    let page: Value = t
        .json_with_options::<(), Value>(
            HttpMethod::GET,
            &catalog_path(&profiles[0]),
            None,
            &catalog_query(),
            &[],
        )
        .await?;
    let media: Vec<Value> = page["items"]
        .as_array()
        .ok_or("Missing catalogue")?
        .iter()
        .take(3)
        .map(|m| json!({"id":m["id"],"category":m["category"]}))
        .collect();
    if media.len() < 3 {
        return Err("Fewer than three available test videos".into());
    }
    println!("CANDIDATES {}", json!(media));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{catalog_path, catalog_query};

    #[test]
    fn catalog_options_are_encoded_as_query_not_route() {
        let path = catalog_path("qa-profile");
        assert_eq!(path, "/api/v1/profiles/qa-profile/catalog");
        assert!(!path.contains('?'));
        assert_eq!(
            catalog_query(),
            [("limit", "100".into()), ("available", "true".into())]
        );
    }
}
fn control(g: &chill_client::playback::PlaybackGrant) -> PlaybackControl {
    PlaybackControl {
        session_id: g.lease.session_id.clone(),
        generation: g.lease.generation,
        media_id: g.lease.media_id.clone(),
        media_revision: g.lease.media_revision,
    }
}
fn event(
    c: &PlaybackControl,
    sequence: u64,
    position: f64,
    duration: f64,
    playing: bool,
    buffering: bool,
    seek: bool,
) -> Progress {
    Progress {
        session_id: c.session_id.clone(),
        generation: c.generation,
        media_id: c.media_id.clone(),
        media_revision: c.media_revision,
        sequence,
        position_ms: (position * 1000.).round() as u64,
        duration_ms: (duration * 1000.).round() as u64,
        speed: 1.,
        final_elapsed_ms: 0,
        playing,
        buffering,
        seek,
    }
}
fn play(
    t: &Transport,
    rt: &tokio::runtime::Runtime,
    profile: &str,
    media: &str,
    seconds: u64,
) -> Result<()> {
    if !(10..=3600).contains(&seconds) {
        return Err("Test duration must be10..3600seconds".into());
    }
    let all: Value = rt.block_on(t.get_json("/api/v1/profiles"))?;
    if !all
        .as_array()
        .ok_or("Missing profiles")?
        .iter()
        .any(|p| p["id"] == profile && p["name"].as_str().is_some_and(|n| n.starts_with("QA Pi ")))
    {
        return Err("Only explicitly created QA Pi profiles are allowed".into());
    }
    let grant = rt.block_on(t.acquire_playback(
        profile,
        &AcquirePlayback {
            media_id: media.into(),
            device_id: uuid::Uuid::new_v4().to_string(),
            context: Default::default(),
        },
    ))?;
    let c = control(&grant);
    let outcome = run_player(t, rt, profile, media, &c, seconds);
    // Release also on errors; never leave a test lease alive deliberately.
    rt.block_on(t.release_playback(
        profile,
        &ReleasePlayback {
            lease: c.clone(),
            progress: None,
        },
    ))?;
    let (position, duration) = outcome?;
    let resumed = rt.block_on(t.acquire_playback(
        profile,
        &AcquirePlayback {
            media_id: media.into(),
            device_id: uuid::Uuid::new_v4().to_string(),
            context: Default::default(),
        },
    ))?;
    let expected = (position * 1000.).round() as u64;
    let resume_ok = resumed.personal.position_ms == expected;
    rt.block_on(t.release_playback(
        profile,
        &ReleasePlayback {
            lease: control(&resumed),
            progress: None,
        },
    ))?;
    if !resume_ok {
        return Err("Resume position differs on second device".into());
    }
    let mut bytes = [0u8; 32];
    if t.read_media_range_blocking(media, &c.session_id, c.generation, 0, &mut bytes, None)
        .is_ok()
    {
        return Err("Released lease still serves media".into());
    }
    println!(
        "PLAYBACK_PASS {}",
        json!({"profile":profile,"media":media,"elapsed_seconds":seconds,"last_position":position,"duration":duration,"resume_other_device":true,"released_lease_refused":true})
    );
    Ok(())
}
fn run_player(
    t: &Transport,
    rt: &tokio::runtime::Runtime,
    profile: &str,
    media: &str,
    c: &PlaybackControl,
    seconds: u64,
) -> Result<(f64, f64)> {
    let mut header = [0u8; 32];
    let raw =
        t.read_media_range_blocking(media, &c.session_id, c.generation, 0, &mut header, None)?;
    let container = if header[..4] == [0x1a, 0x45, 0xdf, 0xa3] {
        "MKV/EBML"
    } else if &header[4..8] == b"ftyp" {
        "MP4"
    } else if &header[..4] == b"RIFF" && &header[8..12] == b"AVI " {
        "AVI"
    } else {
        "other"
    };
    let factory = HttpMediaFactory::new(t.clone());
    factory.set_playback(PlaybackStream {
        media_id: media.into(),
        session_id: c.session_id.clone(),
        generation: c.generation,
    });
    let mut options = PlayerOptions::default();
    options.extra.insert("vo".into(), "null".into());
    options.extra.insert("ao".into(), "null".into());
    let mut player = Player::new(options)?;
    player.register_stream("chill-media", Arc::new(factory))?;
    player.load(&format!("chill-media://{media}"))?;
    let load = Instant::now();
    loop {
        match player.poll_event(Duration::from_millis(50)) {
            PlayerEvent::FileLoaded => break,
            PlayerEvent::Ended { error, .. } => {
                return Err(format!("Decoder load ended: {error}").into());
            }
            _ => {}
        }
        if load.elapsed() > Duration::from_secs(25) {
            return Err("Decoder HTTPS load exceeded25s".into());
        }
    }
    player.pause(true)?;
    let duration = player.duration_seconds()?.ok_or("Video has no duration")?;
    let tracks = player.tracks()?;
    if !tracks.iter().any(|t| t.kind == TrackKind::Video) {
        return Err("No video track decoded".into());
    }
    for track in tracks.iter().filter(|t| t.kind == TrackKind::Audio) {
        player.select_audio(track.id)?;
    }
    for track in tracks.iter().filter(|t| t.kind == TrackKind::Subtitle) {
        player.select_subtitle(Some(track.id))?;
    }
    player.select_subtitle(None)?;
    for speed in [0.5, 0.75, 1., 1.25, 1.5, 1.75, 2.] {
        player.set_speed(speed)?;
    }
    player.set_speed(1.)?;
    let seek = 45f64.min(duration / 4.);
    player.seek_absolute(seek)?;
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        player.poll_event(Duration::from_millis(50));
        if player
            .position_seconds()?
            .is_some_and(|p| (p - seek).abs() < 1.)
        {
            break;
        }
        if Instant::now() > until {
            return Err("HTTPS seek did not converge".into());
        }
    }
    println!(
        "DECODE_READY {}",
        json!({"profile":profile,"load_seconds":load.elapsed().as_secs_f64(),"duration":duration,"codec":player.decoder()?,"container":container,"file_bytes":raw.total_size,"estimated_average_mbps":raw.total_size.map(|b|b as f64*8./duration/1_000_000.),"hardware":player.hardware_decoder()?,"audio_tracks":tracks.iter().filter(|t|t.kind==TrackKind::Audio).count(),"subtitle_tracks":tracks.iter().filter(|t|t.kind==TrackKind::Subtitle).count(),"speed_controls":true,"seek":seek,"headless":true})
    );
    player.pause(false)?;
    let start = Instant::now();
    let mut checkpoint = Instant::now();
    let mut sequence = 0;
    let mut buffer_samples = 0;
    let mut samples = 0;
    let mut last = seek;
    while start.elapsed() < Duration::from_secs(seconds) {
        if let PlayerEvent::Ended { reason, error } = player.poll_event(Duration::from_millis(100))
        {
            return Err(format!("Playback ended early reason{reason} error{error}").into());
        }
        samples += 1;
        if player.is_buffering()? {
            buffer_samples += 1
        }
        if checkpoint.elapsed() >= Duration::from_secs(5) {
            rt.block_on(t.heartbeat_playback(profile, c))?;
            let position = player.position_seconds()?.unwrap_or(last);
            last = position;
            sequence += 1;
            rt.block_on(t.report_progress(
                profile,
                &event(
                    c,
                    sequence,
                    position,
                    duration,
                    true,
                    player.is_buffering()?,
                    false,
                ),
            ))?;
            checkpoint = Instant::now();
            if sequence % 6 == 0 {
                println!(
                    "PROGRESS {}",
                    json!({"profile":profile,"elapsed":start.elapsed().as_secs(),"position":position,"buffer_samples":buffer_samples,"samples":samples})
                );
            }
        }
    }
    player.pause(true)?;
    let position = player.position_seconds()?.unwrap_or(last);
    if position < seek + seconds as f64 * 0.8 {
        return Err("Decoder made less than80percent real-time progress".into());
    }
    sequence += 1;
    rt.block_on(t.report_progress(
        profile,
        &event(c, sequence, position, duration, false, false, false),
    ))?;
    println!(
        "DECODE_COMPLETE {}",
        json!({"profile":profile,"wall_seconds":start.elapsed().as_secs_f64(),"position":position,"buffer_samples":buffer_samples,"samples":samples})
    );
    Ok((position, duration))
}
