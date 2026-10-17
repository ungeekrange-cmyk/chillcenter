# ADR0028 — Client-side AVI MP3 fallback

[Français](ADR-0028-web-mp3-fallback.md) · **English**

2026-10-17. Status: ACCEPTED_BY_USER_CHANGE_REQUEST; implementation and evidence
belong to the R3 delivery. Extends [web audio ADR0020](ADR-0020-web-audio-transcode-to-aac.md)
and [ADR0027](ADR-0027-mobile-content-and-iphone-playback.en.md), without rewriting
their historical evidence. Author: `/root`; separate independent review.

## Decision

For MP3 AVI, preserve native playback when MediaSource/ManagedMediaSource accepts
MP3 inside MP4 or `audio/mpeg`. Otherwise, convert only audio to stereo AAC-LC
inside the existing FFmpeg WebAssembly Worker. Xvid decoding remains client-side.
No direct NAS access, server transcoding, API changes, Mac changes or JavaScript
dependency are introduced.

FFmpeg 9.0.2 is rebuilt with its MP3 decoder using the same pinned source and
Emscripten image: network-disabled compilation, no GPL/nonfree, provenance and
license in `web/vendor/audio/`. Existing on-demand loading and memory limit
remain. Conversion uses phone CPU; this delivery makes no battery benchmark
claim or guarantee for every possible AVI file.

## Presentation and platform limits

The mobile web fullscreen button calls the browser within the touch gesture,
before any network command queue. Use panel fullscreen when available, otherwise
iPhone video-element fullscreen. Report fullscreen only from actual browser
state. An Xvid canvas without panel fullscreen cannot use the video-only API:
explain that limit and retain playback across the available screen, rather than
opening a black native player.

References: [Apple, Safari video](https://developer.apple.com/documentation/webkit/delivering-video-content-for-safari),
[WebKit, non-video iPhone fullscreen](https://bugs.webkit.org/show_bug.cgi?id=240312).

## Verification and rollback

Require real WASM, synthetic MP3 AVI, independent audio comparison, lifecycle
tests and browser playback with simulated MP3 rejection. These do not replace
physical iPhone/Safari tests. Restoring the previous web archive requires no
data migration or server change.
