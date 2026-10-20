# Chill Center v0.7 — release notes

[Français](REL-V0.7.md) | **English**

A distribution of four 0.7.0 components. GitHub publication does not update
existing installations: **no Pi, NAS or deployed web client is modified**.
Tags, signed manifests and package `PROVENANCE.json` bind the exact delivery
source. The v0.6 release and its packages remain preserved.

## Components 0.7.0

- Apple Silicon Mac, ARM64 Linux server, responsive web/PWA and themes each have
  an independent version and release stream.
- Actual installed versions and server API version appear in Configuration;
  update checks do not hold the catalogue lock.
- Configurable manual, check-only or automatic GitHub updates. Server defaults
  to 02:00 local time and defers installation during use. Mac always requires
  native “Install now” confirmation.
- Independent declarative theme packs: seasons, Halloween and Christmas,
  light/dark variants, documented calendar/priority, validation and embedded fallback.
- Inspectable Linux installers, Minisign manifests, API/database compatibility,
  monotonic sequences, slots and interrupted-operation recovery. Existing dated
  API `2026-10-14` and its alias are retained.

## Corrections and verification

The eight security findings and four updater functional defects are corrected.
Reviewed runtime/packaging source: `3e174b9`, with independent code, security and
test reviews. Adverse reviews of earlier candidates remain in `docs/reviews`.

Final runtime suite: 435 Rust tests passed, two pre-existing NFR harnesses ignored;
359 JavaScript/web/documentation tests passed. All 19 Python packaging tests
passed. The updater delta passes nine targeted tests, including real SDK
deserialization and ephemeral-key Minisign verification. Two actual Linux root
tests and the systemd TERM/KILL, busy/retry
and rollback matrix passed in an isolated Ubuntu ARM64 VM.

The genuine AppKit/SDK journey verified invalid-signature rejection, busy
admission before requests, rendering starting during the native prompt, signed
installation, actual restart, fresh consent for a later release and cancellation
without download or mutation. Only ephemeral test keys were used. Earlier QA
failures remain adverse evidence, not relabelled passes. BUG0115 (SDK config),
BUG0116 (QA TAR AppleDouble) and BUG0117 (incorrect macOS minimum) were corrected
before publication.

[Detailed bilingual evidence and limits](../implementation/COMPONENT-UPDATES-FIXES-20261019.en.md).

## Installation and limits

Manual Mac installation: read-only DMG with the app, an Applications shortcut
for drag-and-drop, and FR/EN instructions. The signed `.app.tar.gz` remains the
internal update payload; the DMG does not replace that mechanism.

The supplied Apple Silicon DMG **requires macOS 27+** due to its bundled media
libraries; its app is ad-hoc signed and not notarized. Verify each downloaded
package's Minisign signature. Do not download a standalone installer script
without its required files: server
installation also needs `deploy/updates` units, helper and signed manifests.
The server package targets ARM64 Linux with glibc 2.31+, systemd and Python 3.10+.
The initial installer **refuses an existing installation**: migration of a legacy
server must be planned separately. Web/theme installers require server/helper 0.7.
Read the [bilingual update instructions](../updates/GUIDE-EN.md) before installing.

Trusted household LAN only, without authentication. Do not expose the server
to the Internet. No server transcoding or NAS writes. Mac bundles are ad-hoc
signed, not notarized; no private certificate is distributed. VM/Chromium tests
do not prove physical Safari, Pi/NAS throughput, power-loss or disk-full behavior.

Previous releases, backups and installed versions are retained. Rollback must
never bypass signature anti-rollback protection.
