# Dependencies and security advisories

[Français](DEPENDENCIES.md) | **English**

## GHSA-wrw7-89jp-8q8g — glib,2026-10-21

Real **medium** advisory; Dependabot alert1 remains open. The lockfile retains
glib0.18.5 because Tauri/GTK3 constrain that release series. Forcing glib0.20
violates those constraints and is not a valid fix.
[Official advisory](https://github.com/advisories/GHSA-wrw7-89jp-8q8g),
[verified upstream Tauri constraint](https://github.com/tauri-apps/tauri/blob/0007a43ae74551655357aced98a000965b759ff3/crates/tauri/Cargo.toml).

Independent graph checks prove glib is absent from the shipped Mac Apple Silicon
client and ARM64 Linux server. A future native Linux desktop remains blocked
until its GTK chain supplies a patched version. Web/themes contain no Rust binary.

Concrete improvements without behavior changes or forced lockfile edits:

- Weekly Dependabot checks for Cargo/GitHub Actions, no ignore list or auto-merge.
- `scripts/security-shipping-dependencies.mjs` uses locked resolution for the
  two exact shipping graphs, excluding dev dependencies. It fails if an affected
  glib becomes a compiled dependency of either package.
- Read-only CI runs tests/guard on PRs, main and weekly. Official actions pinned
  by SHA, no secrets, installation or deployment authority.
- CodeQL and Dependabot remain general controls. This targeted guard is not an
  exhaustive scanner for future advisories and does not suppress alerts.

Owner: project maintainer. Reassess on the next Dependabot/Tauri proposal and
before any native Linux desktop release. Do not claim the upstream advisory is
fixed or that it affects the shipped Pi server.
