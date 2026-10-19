# Component update engine

Status: implementation candidate only. No Pi, NAS, GitHub release, tag, service,
production release, or live configuration was changed by this work.

## Boundary

`chill-updates` verifies and stages the four component streams from the single
fixed public repository `ungeekrange-cmyk/chillcenter`. It never accepts a
repository, URL, signing key, shell command, destination path, downgrade, or
force flag from the LAN API. The server owns service-user policy and bounded
intent state. The Linux helper owns installation and reads root-owned config,
the pinned full Minisign public-key file, and a separate root-owned monotonic
sequence ledger. Service-owned state is never trusted for anti-rollback.

The production private key is neither generated nor read by this crate. The
only test signature is the public synthetic vector from `minisign-verify`; no
matching private key is present. Mac assets carry an optional manifest field
`tauri_signature`; it is mandatory for `mac` and forbidden for other component
assets. The Mac integration must still use Tauri's mandatory explicit install
confirmation.

## Public Rust seams

- `parse_and_verify_manifest`: verifies the detached Minisign signature over
  exact bytes, rejects duplicate JSON keys and unknown manifest fields, then
  enforces component, tag, target, SemVer, sequence, expiry, key id, SHA-256,
  byte length, API/server/database/renderer/theme compatibility and no
  downgrade.
- `GithubReleaseClient`: bounded pagination over the fixed repository release
  API, stable component tags only, no global latest endpoint, HTTPS allowlisted
  redirects, bounded metadata and streamed on-disk payload hashing.
- `StateStore`: service-owner-preserving atomic policy/intents, CAS revisions,
  operation coalescing, one running install globally, five-minute component
  check rate limit, and bounded terminal history.
- `AntiRollbackStore`: distinct root-owned monotonic component sequences.
- `due_attempts` / `record_retry`: explicit OS-local offset, one attempt ledger
  per local date, missed-time admission, DST-repeat suppression, and at most
  three fixed retries. The Linux CLI obtains the offset through `localtime_r`;
  it never silently falls back to UTC.
- `extract_archive`: brand-new staging only; raw, ZIP and uncompressed TAR;
  traversal, duplicate paths, links, special files, truncated entries, unsafe
  ratios and expanded-size/entry bombs are rejected. Gzip TAR is intentionally
  unsupported rather than reported as installed.
- `apply_staged`: repeats artifact size/hash, performs final idle check under
  quiescence, saves a backup, atomically moves a versioned slot and symlink,
  verifies component version health, and restores/restarts the previous slot on
  failure. Previous release directories and versioned previous links remain.

The dated API route `2026-10-14` is represented as SemVer `2026.10.14` only
inside compatibility ranges. This is a mechanical numeric mapping, not a new
public API name.

## Linux paths and CLI

Default root-owned inputs:

- `/etc/chillcenter/updates.json` (0640 root-owned)
- `/etc/chillcenter/release.pub` (full Minisign public file)
- `/var/lib/chillcenter-update/trust` (0700 root anti-rollback ledger)
- `/var/lib/chillcenter-update/cache` (0700 verified staging)

Service-owned policy/intents remain in `<data_dir>/updates`. A root-written,
UI-only `available.json` there contains component/version/source/sequence and
is never used for trust decisions. Installed slots are:

- `/usr/lib/chillcenter/server/{releases,current,previous-*}`
- `/usr/lib/chillcenter/web/{releases,current,previous-*}`
- `/usr/lib/chillcenter/themes/{releases,current,previous-*}`

`chill-update` supports `--help`, `--version`, `inspect`, `check`, `manifest`,
`policy`, `intent`, `tick`, `apply`, and first-install-only `install`. `tick`
processes bounded pending intents and scheduled admissions. `apply --force` is
local-root-only and requires the literal second confirmation
`--confirm-interrupt=INTERRUPT_ACTIVE_WORK`; no API field maps to it.

Quiescence uses `/run/chillcenter-update/maintenance` created root/0600. The
helper accepts the server's status only when it is at most three seconds old,
reports `quiescing=true`, `busy=false`, `active_mutations=0`, and contains the
exact nanosecond mtime generation of the current flag. It then stops the server
for the offline backup and binary switch. The old server binary executes
`--backup --config ...` under the configured service UID/GID, never as root.

`deploy/updates` contains the generic service and minutely persistent timer.
The engine, not systemd, owns local 02:00, DST, missed-run and retry semantics.
The timer grants no general sudo rule.

## Installers

`scripts/install-server.sh` is a local-file, first-install-only bootstrap. Before
either downloaded binary executes, an independently installed system Minisign
CLI verifies the detached server manifest with the pinned key and a strict
bounded parser binds the local server and updater byte lengths/SHA-256 to their
distinct signed targets. It requires explicit library/listen/TLS values, creates
a non-root account, refuses existing account/config/binary/unit/state paths, and
never mounts, remounts, chowns, writes, or enumerates the library/NAS. It does
not use `curl | sh` and fails if service/TLS/API/helper smoke does not complete.
Future helper replacement needs a separately signed updater target and a helper
version compatible with the manifest schema; no helper self-update path claims
success in this candidate.

`scripts/install-web.sh` and `scripts/install-themes.sh` invoke only fixed
component operations and compare server binary, server config and TLS
certificate hashes before/after. Their roots are separate from server,
configuration, database and NAS. Theme install additionally rejects every file
type except a single root `manifest.json` and bounded raster images; downloaded
CSS, JS, HTML, SVG, links and special files fail closed.

## Dependency/runtime evidence

Checked against primary package documentation on 2026-10-19 and pinned where
security or parsing behavior matters:

- Rust 1.99.0 / edition 2024 (workspace supported toolchain)
- `minisign-verify` 0.2.5 (exact; prehashed verification, legacy disabled)
- `semver` 1.0.28, `zip` 9.0.3 and `time` 0.3.55 (exact)
- workspace `reqwest` 0.13 (resolved 0.13.5, Rustls)

No preview runtime, private signing dependency, GitHub Actions secret or key
generation was added.

## Verification and remaining delivery gates

Focused crate tests cover exact-byte signature/tamper, recursive duplicate keys,
fixed release URLs, safe TAR/traversal, policy CAS/coalescing, once-per-date and
bounded retry scheduling, and a rootless health-failure rollback simulation.
Network GitHub release discovery, real systemd/account installation, real Pi,
NAS, TLS rotation, disk-full and published package smoke remain `NOT_RUN`.
These unsupported/unexecuted paths return errors; they are not stub successes.

This security-sensitive candidate requires independent security approval of the
exact commit before any DEV merge or deployment. Code/test review and packaging
remain separately owned by the integration coordinator.
