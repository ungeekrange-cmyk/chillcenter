# Mac/web 0.6.1 candidate — handoff evidence

**English** | [Français](REL-WEB-PARITY-001.md)

Later17October supplement: [web-only deployment](REL-WEB-PARITY-001-web-deployment.en.md),
explicitly authorized after this build. Undeployed statements below retain the
initial handoff state, not the state recorded by that supplement.

See [scope, verification and rollback](../implementation/WEB-PARITY-20261017.en.md).

## Source and artifacts

Compiled source: **a13902ea73aa51fb09977f1e6649e252ff39d984**, merging bases
3b2e28b and e6e23db. Internal version **0.6.1 / build 8**, without a new tag.

- Apple Silicon Mac: `artifacts/macos/Chill Center-test-a13902e.app`, 81 MiB.
  Executable SHA256:
  `84524a7e67f4dc743797a82d969d1cc9b0a3a2c0671cec993ffbd02c19241b08`.
- Web: `artifacts/web/chillcenter-web-test-a13902e.tar`, 6,587,392 bytes,
  build **1cce6dd6cd9d**, 79 files including integrity manifest.
  Archive SHA256:
  `32176d8fabb110d0889187a7bcbed7ad19ee1d8a25530043fc6a714b2dc885e4`.

Both artifacts were copied to the canonical project using fresh names, without
overwriting a delivery. Source/copied Mac executable hashes match; web copy matches.

## Executed checks

- **265/265 Node tests**, **22/22 documentation/publication checks** PASS.
- Targeted connected DOM: media/library conflicts, retained drafts, explicit
  refresh, closing during a held response, icon sizing/centering, nonblocking
  sync and reduced motion PASS.
- Built web with its real bridge and IndexedDB against a synthetic API:
  same-name shows, TMDB, artwork import inside the open sheet, settings, no AI
  and persisted language PASS. Not a household-network test.
- Locked, offline Rust release build, strict deep ad-hoc signature PASS,
  **48 relocated ARM64 dylibs**, bundled libmpv client API **2.5** loaded.
  Two previously known dead_code warnings, no build failure.
- [Final independent APPROVED_SCOPED review](../reviews/web-parity-20261017.md).
- Current diff checked without new detected secrets/private certificates or
  household addresses; imported web examples anonymized. No history rewrite
  or claim of a new exhaustive audit of all private history.

## Preservation and limitations

Published v0.6 remains unchanged: executable
`7eb9dd979f473f1dc09309bdc314fe93c9890d8eabf6a42203c7a5232787e3f7`,
v0.6 tag at69da608. The other web worktree remains clean at e6e23db.
**No Pi/NAS access/deployment, GitHub publication or visibility change.**
API2026-10-14 and `/api/v1` are unchanged.

Human validation remains: Tauri appearance/usability, real-browser navigation
with HTTPS/certificate trust, household-network playback and resources.
Browser codec/fullscreen/permission limitations are not equated to libmpv.
To test web: serve the archive's `web/` contents from an HTTPS instance;
do not open `index.html` over `file://`.

Rollback: use the retained v0.6 bundle. The Pi's current web distribution was
not changed, so no remote restoration is needed.
