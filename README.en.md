# Chill Center

[Français](README.md) | **English**

Your video library, on your own network. Derived from Cinéfolio: a lightweight
Linux server and a macOS Apple Silicon client, written primarily in Rust.

Reference Mac client: **v0.6 — 0.6.0, build 7**.
Local delivery: `artifacts/macos/Chill Center-v0.6.app`.
See the [release notes and evidence](docs/releases/REL-V0.6.en.md) for fixes,
executed checks and known limitations. The project is still in development;
this release neither publishes nor deploys the server or web client.

## Features

- Films, series, seasons, sagas and playlists.
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

TMDB and optional AI features use external services. Configure your own keys;
never add them to the repository or issues. The web client is developed
separately: this preparation neither modifies nor deploys it.

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
```

Local bundles are ad hoc signed, not notarized. Licences for bundled libraries
must accompany them.

## API and documentation

The HTTPS API is versioned: `/api/2026-10-14`. Compatible additions stay within
that version; a breaking change requires a new version and continued support
for the version used by existing clients.

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
