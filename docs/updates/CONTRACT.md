# Component update integration contract — R1

Approved architecture: [ADR-0029](../architecture/decisions/ADR-0029-component-releases-updates.md),
[user approval](../gates/G-ADR-0029-R2.md). This contract scopes implementation;
it is not evidence of completed functionality.

- `versions.json` owns Mac/server/web/theme SemVer and API/renderer/schema versions.
- Rust library `crates/chill-updates` owns signed manifest validation, bounded
  fixed-repository GitHub transport, download integrity, safe archive extraction,
  compatibility, persisted policy/status/intent state and scheduling admission.
  A CLI binary provides root-owned install/update operations; no shell from intents.
- Shared trust key: `updates/release.pub`, public Minisign-format verification key.
  Private key is generated locally outside Git, never by worker/test fixtures.
  Test keys must be explicitly synthetic, never accepted in production.
- Component IDs: `mac`, `server`, `web`, `themes`; tag `<component>-v<semver>`.
  Never use GitHub global latest release; skip drafts/prereleases, no downgrade.
- Additive server endpoints below existing dated API and v1 alias:
  `GET /admin/updates` (versions/policies/status),
  `PUT /admin/updates/policy` (component,mode,hour,minute,expected_revision),
  `POST /admin/updates/check` and `/install` (component only).
  Endpoints accept no URL/path/key/command/tag/force and coalesce bounded intents.
  Unsupported older server controls are disabled, not fatal to connection.
- Policy modes `manual`, `check`, `automatic`. Server/web/themes default automatic
  at 02:00 server local time; Mac defaults check-only with mandatory explicit
  install confirmation regardless of settings. Default time is shown with zone.
- Settings module owns a separate async bridge, not the global UI semaphore.
  Display actual installed client/server/theme/API versions and next attempt.
- Theme pack under `themes/default`: declarative `manifest.json`, images/logos,
  no executable JS/CSS/HTML. One shared client resolver applies dates/priority,
  palette tokens and image references. Rendering/animation CSS stays application
  code; downloaded pack only uses predefined tokens/effects and safe local assets.
  Browser reads `/themes/manifest.json` plus immutable local assets; desktop
  `theme_command` bridge returns validated manifest plus safe image data/URLs.
  Embedded fallback remains, preferences stay device-local.
- Download/staging never resets catalog or blocks playback. Server final commit
  quiesces new conflicting work, verifies idle again, saves consistent private
  backup, atomically switches, smokes API/version, and reverts on failure.
  Incompatible database migrations require explicit manual review.
- Web asset/theme installers never overwrite server binary/config/certificates/
  database/NAS. Keep old content-hashed web resources for already open clients.
- No Pi access/deploy, public release, tag replacement or key transmission by
  workers. Root integration owner serializes merges, reviews and publication.
- Test layers: parser/signature/URL/archive attacks; policy/CAS/rate/scheduler;
  busy/race and rollback simulation; theme boundary/priority/invalid fallback;
  settings nonblocking/old-server/Mac confirmation; packaging/version consistency.
