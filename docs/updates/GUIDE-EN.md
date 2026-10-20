# Versions and updates

[Français](GUIDE-FR.md) | **English**

The [v0.7 release](https://github.com/ungeekrange-cmyk/chillcenter/releases/tag/v0.7)
groups the four independently versioned 0.7.0 components. Publication does not
install them on an existing server. Earlier releases remain available; see the
[release evidence and limits](../releases/REL-V0.7.en.md).

## Four components

| Component | Source version | Release tag | Installation |
| --- | --- | --- | --- |
| Apple Silicon Mac | `versions.json.mac` | `mac-vX.Y.Z` | Manual DMG; native “Install now” confirmation for subsequent updates |
| Linux server | `versions.json.server` | `server-vX.Y.Z` | Local administrator installer, non-root service |
| Web/PWA client | `versions.json.web` | `web-vX.Y.Z` | Static server files, without replacing its binary |
| Themes | `versions.json.themes` | `themes-vX.Y.Z` | Data/image pack, without rebuilding the application |

The sole API remains `/api/2026-10-14`; `/api/v1` is retired since server0.7.1.
Product versions are
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
cannot silently add a new trust key to itself. Artifacts include a signed
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

### Download and verify

Use the complete distribution archives from v0.7, not GitHub's automatically
generated source ZIP. Download the matching `.minisig` alongside each archive.
Verify with an independently installed Minisign and the pinned public key below;
do not establish trust merely from a key inside the downloaded archive.

```sh
minisign -Vm Chill-Center-server-0.7.0-linux-aarch64.tar.gz \
  -P 'RWT4AsJIC3n9GIPorIeouS7Ycc5ROkeCsMXypDtOGnikR2Jvm6cuxeQA'
tar -xzf Chill-Center-server-0.7.0-linux-aarch64.tar.gz
```

The same verification applies to the Mac DMG, web install ZIP and themes install
ZIP using their respective filenames. The Mac distribution is Apple Silicon
only and **requires macOS 27+**, due to its bundled media libraries. Open the DMG,
quit the old app, then drag **Chill Center** onto the **Applications** shortcut.
Eject the disk and open the app from Applications. Keep the previous bundle for
local rollback. This first install is manual, not an update from v0.6. The signed
`.app.tar.gz` remains the internal updater payload; a DMG does not change trust
keys, consent or update protocols.

The server archive contains both ARM64 binaries, signed manifest/public key,
`scripts/` and `deploy/updates/`. Keep that layout. A fresh Linux installation
requires systemd, glibc 2.31+, Python 3.10+, Minisign, curl and a configured
readable library; the helper does not mount storage. Inspect the script, then
run it from the extracted root with your own absolute paths and LAN settings:

```sh
sudo ./scripts/install-server.sh \
  --server "$PWD/chill-server" --updater "$PWD/chill-update" \
  --release-pub "$PWD/release.pub" --manifest "$PWD/server-manifest.json" \
  --manifest-signature "$PWD/server-manifest.json.minisig" \
  --version 0.7.0 --library /absolute/read-only-library \
  --listen LAN_ADDRESS:8443 --tls-name LAN_DNS_OR_IP
```

Replace the placeholders; no household address is built in. **Do not run this
bootstrap against an existing installation.** Migration of a legacy server
requires an administrator-controlled backup and separately planned installation.

After a verified 0.7 server/helper installation, verify and extract the web or
themes install ZIP and run its inspectable `scripts/install-web.sh` or
`scripts/install-themes.sh` with sudo. These wrappers fetch the corresponding
signed component stream and refuse active work; they do not replace the server
binary or its configuration. A bundled payload ZIP can also be retained for
audit; the wrapper does not take arbitrary local paths or release URLs.

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
