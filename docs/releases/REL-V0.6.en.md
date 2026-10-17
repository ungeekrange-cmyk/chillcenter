# Chill Center v0.6

[Français](REL-V0.6.md) | **English**

Date: 17 October 2026. Authority: explicit user approval of the candidate,
followed by a request to commit and merge with documentation. Mac version
**0.6.0**, build **7**, local tag **v0.6**.

## Scope and provenance

This release consolidates approved fixes since v0.5.1. The freeze changes only
Mac client version metadata and documentation; it does not rebuild the player
design or network contracts. Exact base: `73cbc8c9fec3fbddf3f239d00db64ca450a7d9f6`.
Latest test change: `095631ad55d1502480c874016a24f3b79968ff28`.
Latest functional Mac change: `15c2d63c2bca5058456a16c8ac7d0320d440ad4c`.

Delivery branch: `codex/release-v0.6`. Local integration branch:
`codex/player-streaming`, fast-forwarded from `37a5b35` without rewriting
history. The tag was created after recording the evidence. The compiled source
and executable SHA256 appear under Delivery below; a later documentation-only
commit does not imply a different compiled source.

## Included changes

- External subtitles are fetched without blocking playback; obsolete selections
  are cancelled. Neither the engine nor its buffer is reset.
- Returning from playback and closing a detail view invalidate hidden layout
  measurements and rebuild the poster grid instead of stretched strips.
- Local `auto`, `en` or `fr` language in Settings. Automatic chooses French when
  the primary system language is French, otherwise English. Titles, metadata
  and personal data are not artificially translated.
- Two outdated QA tests repaired at the responsible layer, with negative tests
  and interface readiness checks without additional real playback.
- Previously delivered fixes: immediate TMDB association feedback, filename
  suffix cleanup, distinct same-name series and spelling canonicalization,
  multiple libraries, missing files at launch, paused playback retaining its
  buffer, horizontal recommendations and next-video transitions.
- Previously delivered RAM optimizations and artwork deduplication, without
  reducing the buffer or changing the cache contract.

Detailed evidence, not a claim of fresh global validation:
[language, subtitles and grid](../implementation/PLAYBACK-LANGUAGE-20261016.md),
[QA](../implementation/QA-CONTRACTS-20261017.md),
[Settings and matching](../implementation/CONFIGURATION-MATCHING-20261016.md),
[recommendations](../implementation/RECOMMENDATION-RAILS-20261016.md),
[transition](../implementation/NEXT-TRANSITION-20261016.md),
[resource measurements](../performance/RESOURCE-OPTIMIZATION-20261016.md).
These historical reports remain in their original language.

## Compatibility and unchanged components

- Dated API `/api/2026-10-14` and legacy alias `/api/v1` retained.
- No new deployment, SSH access, NAS change or Pi configuration change.
  Last historically verified server delivery: source `f980355`,
  [exact report](REL-RESOURCE-OPTIMIZATION-001.md). This freeze is not a new
  online verification of its installed version.
- No change to web client files, branch or deployment. Its in-progress version
  remains independent; `codex/web-pwa` is not merged.
- No schema, dependency, TLS, authentication or preference changes. Server,
  crate and web versions are not artificially aligned with the Mac version.
- No push, GitHub publication, visibility change or Pages deployment.

## Freeze verification

Checks executed on 17 October 2026 for the freeze:

| Check | Result |
| --- | --- |
| Cargo/Tauri/plist/lock | Consistent 0.6.0, build 7, en/fr locales |
| `node --test --test-reporter=spec ui/connected/*.test.js ui/connected/tests/*.test.js` | 130/130 PASS, none skipped/cancelled |
| `git diff --check` and local links in updated documents | PASS; repaired an old link to a renumbered web ADR |
| Targeted added-line scan and inventory of tracked sensitive files | No secret, certificate or private address detected in additions; not a complete history audit |
| `scripts/package-macos.sh` with existing Cargo cache and `CARGO_NET_OFFLINE=true` | rc0, `--release --locked` compilation, dependencies unchanged |
| Deep ad hoc signature verification and published copy | PASS; identical SHA256 before/after exclusive copy |
| Bundled libraries and load check | 48 relocated ARM64 dylibs, bundled libmpv client API 2.5 loaded |

Two pre-existing release warnings (unused `nonblank`, `glReadPixels`) remain
visible; no compilation error. No fresh general playback, native launch,
provider request or NAS access.

Earlier evidence remains attributed to its sources: 7/7 QA tests and 130/130
Node tests on `095631a`, Rust and targeted DOM checks in the linked reports.
Changing the version number does not turn these into new physical tests.
No new independent review is claimed for this freeze.

## Delivery and rollback

Distinct delivered bundle: `artifacts/macos/Chill Center-v0.6.app` in the canonical
repository. Compiled source: **`bac22cafaa31639e68d508b2c95365f924ab8540`**.
Signed executable SHA256:
`7eb9dd979f473f1dc09309bdc314fe93c9890d8eabf6a42203c7a5232787e3f7`.
Published through staging and exclusive rename, without overwriting a bundle.
Apple Silicon application, macOS 13 minimum; ad hoc signed, not notarized.
Bundled library licences are included. Quit the previous app before opening it.

Older bundles and tags are preserved. Rollback: quit v0.6 and reopen
`Chill Center-test-095631a.app` or another compatible delivered version; no
server/database restoration or NAS intervention is needed for this freeze.
The older `095631a` executable retains SHA256
`2865690fef80cc01ec00e8c7475f34cc81b01c5c2017f811376f966f4f915ab1`.

The web workstream was recorded on `codex/web-pwa`, source
`e6e23dbc967bcfe340c06985519c91155eb1f22c`, with a clean state before integration.
The Mac merge neither moves that branch nor writes into its worktree. Web
sources already inherited from the approved candidate do not deliver its
new in-progress version. No remote push is performed.

## Known limitations

- BUG0082 (unhelpful conflict recovery), BUG0083 (small magnifier) and BUG0084
  (off-centre sync rotation) remain open, not hidden by this release.
- Remaining visual/physical Tauri/NAS checks explicitly listed in the reports,
  particularly changing subtitles during playback and returning to the real
  collection grid, are not claimed as rerun here.
- Concurrent Full HD capacity still depends on NAS and Wi-Fi; synthetic RAM
  measurements do not establish a new throughput guarantee.
- Preparing a public repository is neither an audit nor permission to publish
  the entire history. This freeze's checks cover its additions, without deleting
  private evidence or rewriting older commits.
