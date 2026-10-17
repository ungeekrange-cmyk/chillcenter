# Mobile web R4 — transient notice and camera cutouts

[Français](REL-WEB-MOBILE-004.md) · **English**

2026-10-17. Scoped web correction following Safari feedback. No Mac, server,
API, NAS, dependency or audio-engine change. Native-app question handled as
feasibility only: no native iOS/Android app created or signed.

## Version and evidence

- Source `fa13d447abd4c35e9ba1f14f85ef05eaa2b25070`, build `67773f7877d3`.
- Retained archive: `artifacts/web/chillcenter-web-mobile-fa13d44.tar`.
- SHA256 `c139dca29fe63c5bb9ed016b8ac7f180b83f8f01475ff379be8eb298dba80182`;
  6,687,744 bytes, 82 files, 81 manifested.
- **315/315 Node tests PASS**, built mobile UI and desktop web parity QA PASS.
  Real notice timeout checked in Chromium; deterministic timer under polling,
  resize/language and close; mocked WebKit entry, injected left/right insets
  check geometry. No new NAS playback.
- [Independent source/archive review](../reviews/mobile-web-fullscreen-r4-20261017.md).

## Changes

The fullscreen message disappears after **2.5 seconds**, without reappearing on
player status updates. Only a new attempt may display it briefly again. Video
fullscreen also tries `webkitSetPresentationMode("fullscreen")` when the previous
API fails; the standard API requests hidden browser navigation. Returned state
remains actual browser state, without a forcing loop or fake CSS success.

All four system insets protect video, controls, details and landscape navigation
from notches/Face ID/camera cutouts; backgrounds remain edge-to-edge. Floating
point rounding that intermittently dropped a column on resize was reproduced
and fixed only in the web adapter. [BUG0112](../debug/BUG-0112-persistent-fullscreen-notice-cutouts.md).

## Installation and limits

Package prepared; installation and verification after backup are recorded here
once executed. Rollback restores retained previous web assets, without migration
or restart.

**DEVICE_RETEST_PENDING**: physical Safari, fullscreen by codec/API and actual
camera cutouts still require confirmation. Native fullscreen may still reject
an Xvid canvas; no bypass of that restriction is claimed. Close the old tab and
use `/?v=67773f7877d3` after installation.
