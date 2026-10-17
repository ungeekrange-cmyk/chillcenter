# Mobile web R3 — AVI MP3 and mobile controls

[Français](REL-WEB-MOBILE-003.md) · **English**

2026-10-17. Web-only correction. Mac client, server binary, API, configuration
and NAS unchanged. **Deployed on the Pi** after explicit authorization to back
up and install only web assets. No GitHub push
or visibility change.

## Changes

- AVI MP3: stereo AAC-LC fallback in the existing FFmpeg Worker when the browser
  rejects both native MP3 forms. No server transcoding. Native MP3 remains the
  selected path when supported.
- Mobile catalogue posters have no overlaid actions: tapping opens details;
  desktop actions and detail buttons remain.
- Mobile fullscreen runs inside the touch gesture, before the network queue;
  the iPhone video API is used when panel fullscreen is disabled/unavailable.
  A fresh touch may be necessary once the video becomes ready.
- Search survives category navigation: report not reproduced; added E2E coverage
  checks both query and filtered cards, including a category click before debounce.

## Version and immutable archive

| Item | Value |
| --- | --- |
| Source | `acd7aebfc3c9b902eb7b4b414a14750d86017e08` |
| Build | `a15795b5c4cd` |
| Local archive | `artifacts/web/chillcenter-web-mobile-acd7aeb.tar` |
| Archive SHA256 | `34a1663eda23e972ecef6e8b68ac7641432465f8d751eeaea0de8bffe9316bda` |
| Size / manifest | 6,683,648 bytes; 82 files, 81 manifested |
| Audio WASM SHA256 | `5075105bfdfcde924be0163a785a9d5fa0d2f1eb2c0d52aecf5ea571497d47d5` |

Audio WASM grows from 967,016 to 1,011,837 bytes (+44,821). FFmpeg 9.0.2,
pinned source and Emscripten image unchanged; only the MP3 decoder is added,
network-disabled build, LGPL licensing, no new JavaScript dependency.
The local build VM was initially stopped, started for compilation, then stopped;
the original Docker context was restored.

## Executed verification

- `node --test ui/connected/*.test.js ui/connected/tests/*.test.js web/test/*.test.js`: **311/311 PASS**, no skips.
- Three built-client journeys: `scripts/test-web-mobile.cjs`,
  `scripts/test-web-parity.cjs`, `scripts/test-web-mobile-playback.cjs`: **PASS**.
  Local synthetic data/media only, no NAS files or real profiles.
- MP3: real WASM, audio correlation >0.97 against independent FFmpeg, full
  duration, multiple fragments, frames split across AVI chunks, stale seek/close
  rejected, native route preserved. Browser playback forces native MP3 refusal
  to exercise AAC actually produced by the Worker.
- Real Chromium fullscreen and mocked iPhone-native calls check ordering and
  actual state; no physical Safari success is inferred from these mocks.
- Mac/server/shared paths have no diff; no new private address, key or
  certificate found in the checked added lines.

[Independent source/archive review](../reviews/mobile-web-mp3-r3-20261017.md).
[ADR0028](../architecture/decisions/ADR-0028-web-mp3-fallback.en.md),
[BUG0110](../debug/BUG-0110-iphone-web-playback.md),
[BUG0111](../debug/BUG-0111-mobile-posters-fullscreen-search.md).

## Installation and rollback

On 17 October 2026 at **19:31:14 UTC**, the unchanged web-assets operator
(`b0baea0b2a572b65c9a79cd792276143b45092dc95523eb47815b8742c743996`)
replaced R2 `95ebee4fbe0a` with `a15795b5c4cd`, exit `0`.
Prior private backup, mode `0700`:
`/root/chillcenter-rollback/predeploy-mobile-r3-20261017T193045Z`.
SHA256 of `web-before.tar`:
`2f3afc29058d5ab91a428d22a6bbae9b57b86073ed0289963bfe95fc411bf7cc`.
Backup manifest:
`47cc29dd349854baefca0d95e70f3ee0b8156019ddbb9eafd8aae1a6cfe84c1b`.
Retained old tree: `/root/chillcenter-rollback/web-20261017T193113Z`.

PID `5420` unchanged, zero restarts; binary, configuration, unit, certificate
and SPKI hashes match the baseline. No media opened, no lease or profile
modified. The default-port HTTP entry and versioned URL were also checked
from the LAN.

Rollback restores retained previous assets only, without migration or server
changes. [Independent post-deployment verification](REL-WEB-MOBILE-003-verification.md)
at **19:31:50 UTC: APPROVED_SCOPED**, all 81 manifest hashes, HTTP/HTTPS resources
and APIs, rollback and protected hashes verified; no error signal in the bounded journal.
After verification, only the temporary transfer archive was removed after its
hash was checked; backups, old tree and local archive were retained.

## Limits and next validation

Physically retest the reported AVI/MKV on iPhone Safari/Vivaldi, including sound,
seeking, pause and fullscreen control. **DEVICE_RETEST_PENDING**.
Canvas-rendered Xvid cannot enter native video-only fullscreen if Safari lacks
panel fullscreen: playback stays across the available screen with an explicit
limit, never a false fullscreen state. This release does not claim to remove
that platform restriction.

Close the old tab, then open `/?v=a15795b5c4cd` on the server to bypass an older
cached page. HTTP/HTTPS smoke checks and installed-file verification do not
constitute a physical iPhone test.
