# Mobile web R2 — content and iPhone playback

[Français](REL-WEB-MOBILE-002.md) · **English**

2026-10-17. Fixes requested after testing on iPhone, Safari and Vivaldi, over
both HTTP and HTTPS. No Mac bundle, server binary, configuration, certificate,
API or NAS changed. No GitHub push.

## Changes

- Catalogues show posters immediately; a magnifier expands search and filters.
  Navigation folds them without clearing selections. Collection-add picker
  filters remain independent.
- Phone landscape: category navigation moves to a left rail with a compact header.
- Season/show TMDB tools and extra details follow episodes and start collapsed
  on mobile; desktop order and behavior return after resizing.
- `ManagedMediaSource` fallback when classic MediaSource is absent. Remote
  playback is disabled before attaching this fallback, as required by
  [WebKit](https://webkit.org/blog/14735/webkit-features-in-safari-17-1/).
- A blocked start offers a touch action on the prepared player without acquiring
  another lease or reloading. Server pauses and invalid leases remain enforced.
  Landscape has no rotation guidance; video fullscreen is requested in the
  gesture when panel fullscreen is unavailable.

## Provenance and evidence

| Item | Value |
| --- | --- |
| Source | `8b950334a17641aaa1a20e2129ccfe9a449b813a` |
| Build | `95ebee4fbe0a` |
| Archive | `artifacts/web/chillcenter-web-mobile-8b95033.tar` |
| SHA256 | `0da5d3962cbccabc966f71f1b9901aa61c4a2ef74508bb82ba1909694a1d7a50` |
| Size / manifest | 6,628,864 bytes; 81/81 hashes, 82 files including manifest |

**304/304 unit tests PASS**; three Chromium journeys PASS: mobile UI
(320/390/430/736/844px), desktop web regression, and real synthetic MKV
H.264/AAC plus AVI Xvid/MP3 playback. Chromium's MSE constructor is deliberately
exposed as ManagedMediaSource to test fallback wiring; no native Apple API or
NAS video is used. Initial refusal is mocked; resumed playback and image
decoding are real in Chromium.

[Independent source/archive review](../reviews/mobile-web-content-first-20261017.md):
APPROVED_SCOPED for source and archive. [ADR0027](../architecture/decisions/ADR-0027-mobile-content-and-iphone-playback.en.md),
[BUG0109](../debug/BUG-0109-mobile-content-obscured.md), [BUG0110](../debug/BUG-0110-iphone-web-playback.md).

## Installation

Only web assets were replaced on 17 October 2026 at **18:07:25 UTC**, replacing
build `b1722f00c0a6`. Prior private backup, mode `0700`:
`/root/chillcenter-rollback/predeploy-mobile-r2-20261017T180658Z`.
Retained old tree: `/root/chillcenter-rollback/web-20261017T180724Z`.
Rollback restores only these assets through the dedicated operator, without
replacing the server or its configuration.

[Independent verification](REL-WEB-MOBILE-002-verification.md) at 18:08:14 UTC:
**APPROVED_SCOPED**, all 81 installed checksums and backup verified, HTTP/HTTPS
and dated/v1 API reachable. PID `5420` unchanged, zero restarts; binary,
configuration, unit, certificate and SPKI unchanged. Only the temporary transfer
archive was removed after its hash was verified; backups, old tree and local
archive were retained.

## iPhone retest required

Close the old tab and open `/?v=95ebee4fbe0a` on the server to bypass an older
cached page, then retest the two reported videos, in portrait and
landscape. If the browser blocks startup, tap the offered Play button.
The iPhone case remains **DEVICE_RETEST_PENDING**, not CLOSED.

Container compatibility does not guarantee audio-codec compatibility: Xvid/MP3
AVI may remain unreadable if Safari rejects MP3 through MediaSource/MMS. The
existing transcoder does not convert MP3 to AAC. These tests do not claim new
support for that codec or a successful physical Safari playback test.
