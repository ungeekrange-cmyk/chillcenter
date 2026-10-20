# Architecture — v0.6 reference

[Français](OVERVIEW.md) | **English**

A documentation summary of delivered behaviour, not a new decision or test
result. The [ADRs](decisions/INDEX.md), [approved historical architecture](ARCHITECTURE.md)
and [v0.6 release notes](../releases/REL-V0.6.en.md) remain the traceability sources.
Where historical behaviour has been superseded, apply the later ADR.

## Responsibilities

The Linux Rust server (Axum/Tokio/rustls) reads configured libraries, stores
catalogue and personal data in SQLite, and serves the API, artwork and video
bytes over HTTPS. Linux mounts the NAS; the client never does. The server
does not transcode video.

The Mac Apple Silicon client uses Tauri and an HTML/CSS/JavaScript interface
without another framework. Rust owns HTTPS transport, trust, caching, rules
and the native libmpv/FFmpeg player adapter. The webview must not share a
blocking lock with subtitle downloads or catalogue synchronization.

The workspace separates `chill-domain` (rules/contracts), `chill-catalog`
(scan/metadata/persistence), `chill-server` (API/TLS/leases/streams),
`chill-client` (transport/cache/settings/AI), `chill-player` (libmpv)
and `apps/desktop` (Tauri integration). The web workstream remains independent.

## Synchronization and identity

The client mirror is private to each server and profile. Grids and detail views
read it, not the NAS; videos are never stored in that mirror. Opaque cursors
and durable deltas enable lightweight checks. A complete delta is published
atomically without removing the old catalogue while loading; failures retain
the last complete copy. Downloads and status animations do not block navigation.

Never identify a series solely by its title. Opaque keys distinguish same-name
series; `selector=key` is explicit. Historical title-based behaviour remains
supported for older clients. Sagas are shared; playlists, ratings, watched
status and resume positions belong to a profile.

## Playback and resources

Playback requires a server lease tied to the profile, media, its revision and
a generation. Rust transport supplies HTTP ranges to the native player.
The interface handles tracks, progress and next-video transitions; the server
arbitrates profile exclusivity and persists validated progress.

The Mac client retains its engine and buffer while paused (ADR0022), renewing
the lease; transfer or expiry still requires stopping. The cache targets 60 s
ahead and approximately 30 s back, with configurable MiB limits: those durations
are not guaranteed at every bitrate. External subtitle downloads are bounded
and cancellable, without pausing video.

RAM measurements and artwork deduplication are documented separately. A
synthetic benchmark is neither a full native application measurement nor a
guarantee of multiple streams over the Pi's Wi-Fi.

## Security and compatibility

A trusted LAN, without authentication: profiles do not isolate household
members' access. HTTPS and initial certificate approval remain necessary.
Renewal retains the approved key; the accepted exception after expiry of the
previous certificate carries a LAN impersonation risk described in ADR0005.
TLS is never globally disabled.

Since server0.7.1 only `/api/2026-10-14` is served; the old `/api/v1` alias is
retired at the user's explicit request (ADR0031). Compatible additions stay
in this version; no second parallel API is served. Read the [API guide](../api/INTEGRATION-EN.md) and OpenAPI before
integration. Real keys and addresses stay outside Git; exports exclude secrets.

## Documentation and delivery

Local interface language is `auto/en/fr`; user metadata is unchanged.
Reference documentation is available in both languages through the
[index](../README.en.md). Historical decisions and evidence retain their original
language and status. This supplement neither rebuilds nor redeploys v0.6 and
does not change web client files.
