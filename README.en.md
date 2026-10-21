# Chill Center

[Français](README.md) | **English**

Your video library, on your own network: a lightweight
Linux server and a macOS Apple Silicon client, written primarily in Rust,
plus a responsive web client installable on the Home Screen (PWA).

**[Download the Apple Silicon Mac app v0.7.1 — DMG](https://github.com/ungeekrange-cmyk/chillcenter/releases/download/v0.7.1/Chill-Center-0.7.1-macos-arm64.dmg)** · [FR/EN notes and packages](https://github.com/ungeekrange-cmyk/chillcenter/releases/tag/v0.7.1)

Mac, server, web and themes are independently versioned in
[versions.json](versions.json). Installed versions and update policies appear
under **Settings → General**. Mac always requires “Install now”; server/web/themes
can update at 02:00 local time, outside playback and active jobs.
[Theme packs](themes/README.en.md) document their calendar and precedence.

| Component 0.7.1 | Package and instructions |
| --- | --- |
| Apple Silicon Mac, build 10 | [DMG installer](https://github.com/ungeekrange-cmyk/chillcenter/releases/download/v0.7.1/Chill-Center-0.7.1-macos-arm64.dmg), macOS 27+ for this bundle |
| ARM64 Linux server | [Archive with installers](https://github.com/ungeekrange-cmyk/chillcenter/releases/download/v0.7.1/Chill-Center-server-0.7.1-linux-aarch64.tar.gz), 64-bit systemd host |
| Web/PWA client | [Package with installer](https://github.com/ungeekrange-cmyk/chillcenter/releases/download/v0.7.1/Chill-Center-web-0.7.1-install.zip), requires server 0.7 |
| Independent themes | [Pack with installer](https://github.com/ungeekrange-cmyk/chillcenter/releases/download/v0.7.1/Chill-Center-themes-0.7.1-install.zip), requires renderer 2 |

Read the [verification/installation guide](docs/updates/GUIDE-EN.md) and
[release notes and limits](docs/releases/REL-V0.7.1.en.md). Signed manifests are
published in each component's release. Publication does not update an existing
server; v0.6 and previous packages remain available. Do not run the initial
installer on an existing installation: it deliberately refuses to overwrite it.

## Features

- Films, series, seasons, sagas and playlists.
- Mac server discovery with Bonjour, with manual entry as a fallback.
- Profiles with synchronized resume positions, history, watched status and personal ratings.
- Native libmpv/FFmpeg player: audio, subtitles, playback speed and next episode.
- TMDB metadata using your own key, manual corrections and artwork imports.
- Genre and year filters, light/dark themes and seasonal ambiences.
- Client-side catalogue and artwork cache, with background differential synchronization.
- English or French interface, automatic detection and a choice in Settings.

The server supplies media over HTTPS from configured storage. **No server-side
transcoding**: the client decodes video without mounting the NAS. Raspberry Pi 3
is a target; smooth playback depends on the network and files. Two or three
users are a goal, not a throughput guarantee for every bitrate.

## Security and privacy

Intended for a **trusted home LAN**. Profiles are not authenticated accounts.
Do not expose the server to the Internet. The Mac client asks you to accept
the self-generated certificate on first connection.

TMDB and optional Mac-client AI features use external services. Configure your
own keys; never add them to the repository or issues. AI features are not
offered in the web client.

## Development

Prerequisites: the Rust version in `rust-toolchain.toml`, platform build tools
and libmpv for playback. Mac packaging uses Xcode tools and a local mpv
installation as a build input.

```sh
cargo test --workspace --all-targets --locked
cargo run --locked -p chill-server -- --config /path/to/config.json
cargo run --locked -p chillcenter-desktop
```

Adapt configuration to your environment and keep it outside Git (for example,
the ignored `server.local.json`). No server address or NAS path is imposed.
The server defaults to loopback only, port `8443`; LAN listening must be
configured explicitly. Linux remains the product's server target.

Build a Mac application at a **new destination**, without overwriting a delivered bundle:

```sh
bash scripts/package-macos.sh /absolute/path/Chill-Center-test.app
python3 scripts/package-macos-dmg.py /absolute/path/Chill-Center-test.app /absolute/path/Chill-Center-test.dmg
```

Local bundles are ad hoc signed, not notarized. Licences for bundled libraries
must accompany them.

## Web client and PWA

```sh
node scripts/build-web.mjs
node --test ui/connected/*.test.js ui/connected/tests/*.test.js web/test/*.test.js
```

Generated files in `web/dist` are served by Chill Center on the same origin as
the API; they are not a standalone server. On iPhone, open the site in Safari,
then **Share → Add to Home Screen**; enable **Open as Web App** when offered.
On Android, use the browser’s installation action when PWA requirements are met.

The app still depends on the home server and browser capabilities. Safe areas
keep controls clear of notches and camera cutouts; Safari may still refuse
native fullscreen for canvas-rendered Xvid. Device tests remain separate from
Chromium tests. No native iOS/Android app or Face ID authentication is included.

## API and documentation

The sole HTTPS API is `/api/2026-10-14`. Compatible additions stay within that
version; since server0.7.1, the old `/api/v1` alias is retired (404).
Third-party clients must use the dated base.

- [OpenAPI contract](docs/api/openapi-2026-10-14.json)
- [Bilingual documentation](docs/README.en.md) and [API integration guide](docs/api/INTEGRATION-EN.md)
- [Contributing](CONTRIBUTING.en.md) and [security](SECURITY.en.md)
- [Preparing the public GitHub repository](docs/publication/GITHUB-REPOSITORY.en.md)
- [Preparing and verifying GitHub Pages](docs/publication/GITHUB-PAGES.en.md)

Architecture, decisions and working logs remain in `docs/`. They are **not**
automatically included in the public site. Making the repository and its
history public requires a separate audit.

## GitHub Pages (optional)

The main deliverable is the GitHub project repository and its README. A static
presentation site may later be hosted on GitHub Pages, without application
dependencies, tracking or access to the home server:

```sh
node --test scripts/test-public-site.mjs
node scripts/build-public-site.mjs
```

Open `dist/public/index.html` to preview it. The Pages workflow is manual:
commits and pull requests do not trigger deployment. Its presence is not
authorization to publish.

## Licence

Code is under the [MIT licence](LICENSE). Third-party libraries, media artwork
and external services retain their own licences and terms.
