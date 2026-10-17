# Delivery — smartphone web client

[Français](REL-WEB-MOBILE-001.md) · **English**

Date: 2026-10-17. The user explicitly approved concept R1 and static-web-assets
deployment. This delivery does not change tag `v0.6`, Mac bundles or GitHub
visibility. No GitHub push is part of this delivery.

## Delivered behavior

- Bottom navigation on smartphones: Films, TV shows, Documentaries, Shows and
  More. More opens the full collection, sagas, playlists and settings.
- Logo, upper-right profile name/initial, and accessible search. Genre/year
  filters, detail sheets and settings adapted to touch.
- Virtualized grid without pagination: two columns up to 360px, three above
  that on phones; the final row clears navigation and safe areas.
- Actual fullscreen requested in the Play gesture, then landscape requested
  according to browser capabilities. Refusal offers rotation guidance and a
  real retry without blocking or restarting playback.
- Navigation and decoration hidden during playback; navigation returns after
  closing, including after resize. Christmas/Halloween artwork fills the panel.
- FR/EN languages, themes and profiles retained. AI remains excluded from web.

Wide web layouts retain their sidebar. Common `ui/connected` sources, Mac
sources, the server and API contracts remain unchanged. The dated
`2026-10-14` API and historical `v1` alias remain available.

## Immutable provenance

| Item | Value |
| --- | --- |
| Feature commit | `98ee3e37e8b19c0d2e8e92991b3fb917eb0a13b8` |
| Web build | `b1722f00c0a6` |
| Previous build | `1cce6dd6cd9d` |
| Preserved archive | `artifacts/web/chillcenter-web-mobile-98ee3e3.tar` |
| Archive SHA256 | `281cf6eb6babca616004f425ac9e13332f38ef9a3af59f7793398640c81cc0ed` |
| Size | 6,616,576 bytes |
| Manifest | 81/81 hashes valid; 82 files including `SHA256SUMS` |
| Static-assets-only operator | `b0baea0b2a572b65c9a79cd792276143b45092dc95523eb47815b8742c743996` |

## Evidence and scope

- `node --test ui/connected/*.test.js ui/connected/tests/*.test.js web/test/*.test.js`: **283/283 PASS**.
- `scripts/test-web-mobile.cjs`: actual built client with synthetic API,
  320/390/430/736/844px and 1280px restoration, filters, retained scroll,
  non-empty collections, settings, FR/EN, touch targets, event themes,
  real Chromium fullscreen and navigation restoration: **PASS**.
- `scripts/test-web-parity.cjs`: TMDB details, distinct same-name shows,
  image picker, settings/languages and AI exclusion regression: **PASS**.
- [Independent source and archive review](../reviews/mobile-web-responsive-20261017.md): **APPROVED_SCOPED**.
- FR/EN documentation, links, privacy and publication checks: **24/24 PASS**.
- [ADR0026](../architecture/decisions/ADR-0026-mobile-web-presentation.en.md)
  and [BUG0108](../debug/BUG-0108-mobile-web-lifecycle.md): defects found and fixed before delivery.

Local test data is synthetic; it does not prove NAS playback, hardware
performance or browser policy on a physical phone.

## Deployment and backup

Static assets replaced at **2026-10-17 14:36:13 UTC**, operator exit=0.
The service remained active at PID `5420`, with zero restarts.

Private backup verified before replacement:
`/root/chillcenter-rollback/predeploy-mobile-20261017T143553Z/web-before.tar`,
SHA256 `e340656ef3fdae44ed80aaca2e10c35a8cbdf8fb26bfade0a67f976ea470bca4`.
Preserved previous tree: `/root/chillcenter-rollback/web-20261017T143612Z`.
The private directory is root:root, mode 0700; no NAS was traversed or modified.

[Independent post-deployment verification](REL-WEB-MOBILE-001-verification.md):
**APPROVED_SCOPED**. All 81 hashes match installed files; strict HTTPS, the dated
API and v1 alias return 200. Binary, configuration, unit, certificate and SPKI
remain unchanged; no warning was found in the bounded journal. Default-port
HTTP and application-port HTTP were also verified from the Mac.

Only the temporary transfer archive was deleted after its hash was checked.
The local deliverable, backup and previous tree remain available. Both retained
Mac executables keep their previous hashes.

Rollback: after validating targets and hashes, restore only the previous static
assets from this tree or the verified backup. No data rollback, settings change
or binary replacement is required.

## User testing and limitations

Reload the web client at its usual local-network address. If the browser keeps
old content, fully reload the page or reopen the tab. Check `build.json` to
identify the installed build.

Physical-device tests remain: Safari/iPhone and Android, video playback,
landscape rotation, fullscreen exit/re-entry and safe areas. Browsers may refuse
orientation locking or fullscreen; the fallback remains usable, but universal
forced rotation is not promised. Canvas playback cannot use WebKit's native
video fullscreen. No Pi/NAS throughput test was part of this delivery.
