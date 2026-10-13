# Chill Center Mac interface/server security re-review R2

- **Reviewer:** independent security-reviewer agent `/root/ui_security_review`
- **Date:** 2026-10-13
- **Candidate:** `d361eba6d1b1b24432a4cb793b07538c35b657d0`
- **Comparison base:** `b8737bba65f010e349a8387201faf8d73cebe2b0`
- **Scope:** the approved Mac connected UI/Tauri bridge, client trust and transport, local client settings, exposed server/profile/catalog/manual-state/scan endpoints, SQLite cancellation/scan transaction changes, and the development macOS bundle script/plist.
- **Verdict:** **APPROVED for this bounded slice, with two non-blocking inherited LOW findings.** No new CRITICAL, HIGH, or MEDIUM security finding was identified. This is not approval of the full app, player/import/export surfaces, production distribution/notarization, Linux/Pi operation, physical NAS behavior, or all 133 acceptance criteria.

## Threat model and trust boundaries

The reviewed boundary is a local Tauri webview driving a typed Rust command bridge, a native HTTPS client holding local TOFU/pin state, and an unauthenticated household-LAN server. Relevant adversaries are untrusted webview/server strings, a different TLS endpoint or server identity, browser/DNS-rebinding requests to the LAN service, oversized/slow inputs, cross-profile object substitution, cancellation/retry races, and local filesystem failure. The accepted household-LAN no-auth model and ADR-0005's accepted key-after-known-expiry behavior are recorded constraints, not reopened findings.

## Open findings

### UISEC-002 — LOW — client rename is not made directory-durable

- **Evidence:** exact `crates/chill-client/src/ui.rs:221-244` writes a unique mode-`0600` temporary file, `sync_all`s it, and atomically renames it to `client.json`, but does not `fsync` the containing directory.
- **Preconditions / impact:** a local crash or power loss in the filesystem durability window may restore an older settings/trust file after the application reported success. This is local durability/integrity risk; it does not give a network peer direct file access and does not bypass the live TLS checks.
- **Required follow-up:** open and `sync_all` the client configuration directory after rename; retain failure-injection/reopen coverage. Owner: client settings implementation.
- **Disposition:** inherited from R1 and explicitly still owned; non-blocking LOW.

### SRV-SEC-005 — LOW — server config fsync failure can be reported after rename committed

- **Evidence:** exact `crates/chill-server/src/config.rs:195-220` renames the staged file before syncing the parent directory. A parent-sync failure returns an error, removes only the now-absent stage path, and prevents the in-memory `LibrarySettings` update even though the destination may already contain the new revision.
- **Preconditions / impact:** local storage failure after rename can leave disk and memory/reported outcome inconsistent, creating an operator retry/conflict hazard. No remote error details or secrets are exposed.
- **Required follow-up:** distinguish pre-rename failure from post-rename durability failure or reload/reconcile the destination before returning; add before/after-rename fault injection.
- **Disposition:** inherited server finding; non-blocking LOW.

## Closed prior findings and positive controls

| Area | Exact-candidate evidence | Result |
|---|---|---|
| `UISEC-001` replacement visibility | `ui.rs:385-476` snapshots the event count, derives a notice only from a newly appended `KeyReplacedAfterKnownExpiry`, stores it in the connected state, obtains the complete view, then persists the trust event/settings. `ui/connected/app.js:4,13-15,48-61,70,75,79` renders the notice through `textContent` in a separate element; transient messages, catalog loads, and admin refreshes do not clear it. | **RESOLVED.** Coordinator-reported headed exact-SHA fixture: `UI_QA_RESULT true renewal_notice`, exit 0, and catalog/admin refresh preserved the banner. The fixture used an isolated expired-pin/current-certificate setup, not an actual 18-month wait/rotation. |
| `UISEC-003` provisional trust/context rollback | `ui.rs:356-378` snapshots both settings and trust, disables intermediate persistence, and clears/restores on identity/profile/view/save failure. `:530-576` additionally restores pre-approval trust. `:478-511,584-599` restores active profile and remembered choice when prospective view or persistence fails. | **RESOLVED.** Failed connection cannot retain an orphan pending pin, replacement event, registered server mapping, or partially selected profile in live state. |
| Typed Tauri action boundary | `ui.rs:17-71` is a deny-unknown-fields tagged command enum; path-bearing operations are constructed in Rust and IDs are validated. `apps/desktop/src/main.rs:11-40` exposes one serialized bridge with single-operation backpressure. UI renders server/user data through `textContent`/form values, not `innerHTML`. | PASS. No arbitrary URL, header, filesystem, shell, or command-name bridge was found. |
| Tauri navigation/CSP/capabilities | `main.rs:58-63` accepts only the two Tauri local origins. `tauri.conf.json:8-12` has a restrictive CSP (`default-src 'self'`, no objects/frames/forms, IPC-only connect). `capabilities/main.json` adds no filesystem, shell, dialog, updater, or network plugin. | PASS. The release capability metadata still contains the inert `allow-qa-result` identifier, but release registers only `ui_command`; no QA native handler is compiled or reachable. |
| Debug QA isolation | `main.rs:45-52,66-86,91-102` compiles QA directory override, script evaluation, result printing, and exit command only under `debug_assertions`; use also requires explicit `CHILL_UI_QA_*` variables and an isolated data directory. | PASS. Coordinator-reported exact release bundle inspection found no QA native handler. No real user/NAS data was used by GUI fixtures. |
| TLS trust and identity separation | `crates/chill-client/src/lib.rs:86-156,399-479` validates current certificate time, self-signature, SAN, handshake signatures, alias and pinned SPKI; pre-expiry key changes fail. `ui.rs:394-450` requires API v1, UUID server identity, refuses an address mapped to a different registered identity, and refuses an implicit different-key merge into an existing identity. | PASS within ADR-0005. The accepted post-known-expiry fallback remains logged and now visibly surfaced after successful connection. |
| Transport (`BUG-0004`) | `lib.rs:493-526` clones one configured `ClientConfig` into both async and blocking clients, disables redirects and proxies, and applies a total timeout. `:539-582` caps success and error JSON bodies at 4 MiB. API errors retain only bounded safe codes; server details are discarded. | PASS. No redirect/proxy escape or divergent TLS configuration was found. |
| Local settings | `ui.rs:183-205,221-244` rejects a symlink data directory, sets directory `0700`, caps input at 4 MiB, uses a unique create-new `0600` stage, file fsync, and atomic rename. Persisted trust contains certificate/public-key material, aliases and audit events, not private keys or credentials. | PASS except `UISEC-002`. |
| Server browser/host boundary and errors | `crates/chill-server/src/api.rs:127-205` returns fixed messages plus typed codes/request IDs, caps bodies at 1 MiB, rejects every `Origin`/`Sec-Fetch-Site`, validates Host/authority against configured TLS names and exact port, and applies a 30 s JSON-route budget. | PASS for current native JSON API. No cookie/session/CSRF or redirect surface exists in this slice. |
| Profile/catalog isolation | `catalog_api.rs:40-91` passes the route profile to store operations. `store.rs:244-299` requires that profile before catalog/media state access, caps pages at 100, validates category/query length, returns relative paths, and overlays state by `(profile_id, stable_media_id)`. Profile deletion is FK-scoped and common media remains separate. | PASS within the accepted LAN user model. Cross-profile object substitution did not return or mutate another profile's personal state in the exact tests. |
| Cancellation and retry safety (`BUG-0006`) | `api.rs:66-115` rejects closed/expired queued work and installs a drop cancellation token. `store.rs:311-338` and catalogue `database.rs:862-885` install progress and pre-commit hooks on both SQLite connections, fail closed if hook installation fails, and clear hooks after each job. Timeout UI tells the user to reload before retry. | PASS for the demonstrated late queued/started mutation path. As documented, this is not a distributed exactly-once guarantee after a commit followed by connection loss. |
| Scan process/resource boundary | `catalog_api.rs:102-175` serializes scans, snapshots the configured root under the same lock order as settings updates, executes only `current_exe --scan-worker <root>` with null stdin/stderr, piped stdout, kill-on-drop, 300 s timeout, and bounded output. | PASS. No shell interpretation or arbitrary executable selection occurs. |
| Scan/catalog/identity atomicity (`BUG-0007`) | `catalog_api.rs:160-167` uses a distinct blocking `Store` so the interactive actor is not monopolized. `store.rs:307-309` extends the catalogue snapshot with identity inserts, while `database.rs:819-859` serializes outside the lock then commits media, scan marker and identities in one transaction. | PASS. Extension failure rolls back all three; the prior catalogue is retained. |
| Development package/plist | `scripts/package-macos.sh:1-25` labels the output development-only, requires a new absolute `.app` path, refuses overwrite/non-Darwin/non-arm64, uses locked release build, lints the plist, ad-hoc signs, and strictly verifies it. `Info.plist:3-14` is minimal, declares local-network purpose, and adds no ATS relaxation, URL handler, document association, or entitlement. | PASS for a development bundle only. Not production signing/notarization or a final libmpv payload. |

## Verification evidence

| Check | Result | Qualification |
|---|---|---|
| `git rev-parse HEAD` / initial `git status --short` | exact `d361eba6...`; clean at review start | Later coordinator remediation dirtied the shared checkout, so all source conclusions above are bound to `git show d361eba6:<path>` blobs. Those later edits are excluded. |
| `git diff --check b8737bba..d361eba6` | PASS | No whitespace error in the reviewed delta. |
| `cargo test --workspace --all-targets --locked` with Rust/Cargo 1.99.0 and isolated target | PASS: 122 tests (`64 + 17 + 11 + 10 + 20`) | Command began from the clean candidate; no Pi/NAS/18-month timing claim. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS | Same locked toolchain and isolated target. |
| Headed GUI modes supplied by coordinator | PASS: `first`, `reopen_delete`, `renewal_notice` | Exact-SHA, isolated fixtures; reported evidence. The renewal fixture flips stored SPKI bytes and uses known expiry `1`, then connects to the real current local HTTPS verifier. |
| Exact release development bundle supplied by coordinator | PASS: compile, plist lint, ad-hoc `codesign --verify --strict`; no QA native handler | Reported evidence. My later local package attempt overlapped subsequent uncommitted coordinator work and is deliberately not used as exact-SHA evidence. |
| Exact-tree secret/key filename and token-pattern scan | No committed key/certificate/credential artifact found | Matches were field names, generated approval tokens, or explicit test fixture text such as `"secret"`; scan is pattern-based, not entropy analysis. |

## Residual constraints

- Household LAN no-auth and ADR-0005 key replacement after a **known** expired certificate are accepted decisions. This report confirms enforcement/visibility; it does not newly accept or expand them.
- The server's current JSON deadlines are not approval for future media streaming; streams need their separate per-read/idle controls.
- Legacy unexposed import/export/player functions are not made complete or approved by this slice.
- No Linux/Pi, real NAS, Wi-Fi performance, full 133-AC, production signing, notarization, installer, or update-channel evidence was reviewed.

## Report ownership

This review wrote only `/private/tmp/chillcenter-interface-security-review-r2.md`. It made no candidate source, test, configuration, cloud, NAS, or user-data change.
