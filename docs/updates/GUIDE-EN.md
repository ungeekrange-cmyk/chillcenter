# Versions and updates

[Français](GUIDE-FR.md) | **English**

Status: 0.7.0 development/review candidate, **not a delivery**.
The only Mac bundle published at this stage is [v0.6](https://github.com/ungeekrange-cmyk/chillcenter/releases/tag/v0.6).
Do not use the installers/helper in production before security approval and
documented deployment testing.

## Four components

| Component | Source version | Release tag | Installation |
| --- | --- | --- | --- |
| Apple Silicon Mac | `versions.json.mac` | `mac-vX.Y.Z` | `.app` archive, then native “Install now” confirmation |
| Linux server | `versions.json.server` | `server-vX.Y.Z` | Local administrator installer, non-root service |
| Web/PWA client | `versions.json.web` | `web-vX.Y.Z` | Static server files, without replacing its binary |
| Themes | `versions.json.themes` | `themes-vX.Y.Z` | Data/image pack, without rebuilding the application |

The API remains `/api/2026-10-14`, with `/api/v1` retained. Product versions are
not API versions. Packs also declare format and renderer versions. See the
[theme calendar](../../themes/README.en.md).

## Settings

**Versions and updates**, in General, shows actual installed client, server,
theme and API versions. The Mac also shows the web version installed on the
server. An older server remains usable: only unsupported update controls are disabled.

- Manual: no scheduled check; use Check now.
- Automatic checks: check at the selected time, without installing.
- Automatic installation: check then install a compatible signed release.
  This choice is unavailable for the Mac.

Server/web/themes default to automatic at **02:00 server local time**. The
timezone is Linux's, not the browser's. One scheduled attempt per local date
prevents duplicates during daylight-saving changes. Mac checks run while the
application is open and always require native confirmation before installation
and restart. No hidden Mac task runs while the application is closed.

The catalogue remains usable during checks/downloads. Playback and active jobs
defer installation; no LAN button can forcibly interrupt them. Reload the page
after a web update to use the new interface; an already-open tab retains its
previous-version resources. Theme changes are detected in the background.

## Installation and trust

An initial 0.7 installation is required to enable the mechanism: the 0.6 client
cannot silently add a new trust key to itself. Artifacts will include a signed
manifest, signature and provenance. Verify before execution; never use
`curl | sudo sh`.

Inspectable scripts: [install-server.sh](../../scripts/install-server.sh),
[install-web.sh](../../scripts/install-web.sh),
[install-themes.sh](../../scripts/install-themes.sh). The first refuses to
overwrite an existing installation and requires explicit library location,
listen address and TLS name. It neither mounts nor modifies the NAS. The other
two require the configured helper and replace only their own component.
[ENGINE.md](ENGINE.md) describes units, privilege boundaries and testing limitations.

Mac packages remain ad-hoc signed and unnotarized: update signatures authenticate
bytes but are not Apple notarization. The release public key is
[release.pub](../../updates/release.pub), identity `18FD790B48C202F8`. The private
key is never distributed or stored in Git.

## Maintainer publication

Before publication: independent review, tests, clean committed source, verified
packages and licenses. Increment the component in `versions.json` and reserve
its strictly increasing sequence in [sequences.json](../../updates/sequences.json).
[component-release.mjs](../../scripts/component-release.mjs) verifies published
signed receipts from the fixed repository and refuses equal/lower sequences,
version collisions and network failures. A corrected system clock therefore
cannot decrease the sequence. It signs locally and does not publish anything.
Never modify a stable release in place; publish a new version.

Accepted sequences remain recorded after installation failure. A faulty package
requires a new corrective release or verified local administrator recovery,
not disabled signatures. Preserve old packages and private backups. Back up the
signing key offline: losing it prevents signing new updates with that identity.

References: [contract](CONTRACT.md), [ADR0029](../architecture/decisions/ADR-0029-component-releases-updates.md),
[third-party API](../api/INTEGRATION-EN.md).
