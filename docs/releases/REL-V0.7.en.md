# Chill Center v0.7 — candidate release notes

[Français](REL-V0.7.md) | **English**

**Unpublished.** Candidate sources and packages do not replace delivered v0.6.
Publication awaits native Mac validation; no Pi installation is part of this request.

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

The candidate corrects the eight security findings and four updater functional
defects. Code and security reviews approve `3913267`; no deployed web files,
Pi binary/configuration or NAS were touched. The test review retains one blocker:
the genuine AppKit journey.

Reference suite: 433 Rust tests passed, two pre-existing NFR harnesses ignored;
359 JavaScript/web/documentation tests passed. The native delta passes eight
targeted tests, including real SDK deserialization and ephemeral-key Minisign
verification. Two actual Linux root tests and the systemd TERM/KILL, busy/retry
and rollback matrix passed in an isolated Ubuntu ARM64 VM.

The first native launch exposed a missing mandatory SDK public-key field:
BUG0115 was corrected before publication. The consent/install/restart journey
still requires human validation.

[Detailed bilingual evidence and limits](../implementation/COMPONENT-UPDATES-FIXES-20261019.en.md).

## Installation and limits

Packages will not be advertised as installable until signed and published.
Do not download a standalone installer script without its required files: server
installation also needs `deploy/updates` units, helper and signed manifests.
Read the [bilingual update instructions](../updates/GUIDE-EN.md) before installing.

Trusted household LAN only, without authentication. Do not expose the server
to the Internet. No server transcoding or NAS writes. Mac bundles are ad-hoc
signed, not notarized; no private certificate is distributed. VM/Chromium tests
do not prove physical Safari, Pi/NAS throughput, power-loss or disk-full behavior.

Previous releases, backups and installed versions are retained. Rollback must
never bypass signature anti-rollback protection.
