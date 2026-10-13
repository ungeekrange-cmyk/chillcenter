# Chill Center Mac interface/server security re-review R3

- **Reviewer:** independent security-reviewer agent `/root/ui_security_review`
- **Date:** 2026-10-13
- **Candidate:** `c2a9d72d634cc7282eb8a20ca9fa138ed92e2fec`
- **Immediate base:** `d361eba6d1b1b24432a4cb793b07538c35b657d0` (R2); original interface base `b8737bba65f010e349a8387201faf8d73cebe2b0`
- **Scope:** approved Mac connected UI/Tauri bridge, native client TLS/transport and local trust/settings, exposed server/profile/catalog/manual-state/scan endpoints, and development macOS bundle script/plist. The R3 delta is six files: catalog error classification/tests, a trust rollback persistence test, HTTP fault-injection coverage, one existing locked dev dependency, and BUG-0008 documentation.
- **Verdict:** **APPROVED for this bounded slice, with the two previously owned non-blocking LOW findings below.** No CRITICAL, HIGH, MEDIUM, or new LOW security finding remains in the candidate. This does not certify the full app, Pi/Linux/NAS behavior, player/import/export, production distribution, or all 133 acceptance criteria.

## R3 delta conclusion

The `d361eba6..c2a9d72` change is narrow and security-positive:

- Exact `crates/chill-catalog/src/database.rs:69-80` now maps only `rusqlite::Error::QueryReturnedNoRows` to the media-absent sentinel. Other SQLite failures and corrupt stored JSON map to the fixed safe database error.
- Existing `crates/chill-domain/src/lib.rs:18-30` converts that safe sentinel to `StoreFailure::DatabaseOffline`; existing `crates/chill-server/src/api.rs:127-146` returns HTTP 503 / `DATABASE_OFFLINE` with a fixed message and random request ID. Raw SQLite, SQL, filesystem, NAS, media, and provider details do not cross the HTTP/webview boundary.
- Exact catalog tests at `database.rs:965-987` distinguish missing, corrupt, and unavailable table reads. Exact HTTP coverage at `api.rs:837-854` injects a table-read failure in a temporary database and proves 503 / `DATABASE_OFFLINE`, not a misleading 404 / `MEDIA_NOT_FOUND`.
- Exact `crates/chill-client/src/ui.rs:773-801` now directly proves a failed initial approval restores live trust, leaves no connected/server context, and does not persist provisional trust across reopen. It covers the R2 implementation at `ui.rs:530-576` without changing UI/TLS policy.
- `rusqlite.workspace = true` is dev-only for the server test and reuses locked `rusqlite 0.40.2`; no new runtime dependency, trust boundary, permission, endpoint, migration, or secret is introduced.

The earlier code-review MEDIUM for database failures masquerading as deleted media is therefore **RESOLVED**. The correction improves availability diagnosis and safe recovery without increasing error disclosure.

## Open inherited findings

### UISEC-002 — LOW — client rename is not directory-durable

Exact `crates/chill-client/src/ui.rs:221-244` writes a unique mode-`0600` temporary file, file-fsyncs and atomically renames it, but does not fsync the containing directory. A local crash/power loss in the durability window may restore older trust/settings after success was reported. Required follow-up: fsync the client configuration directory after rename and add failure/reopen coverage. This does not enable a network trust bypass during the live process.

### SRV-SEC-005 — LOW — server config parent-fsync failure can follow a successful rename

Exact `crates/chill-server/src/config.rs:195-220` renames before parent-directory fsync. If that fsync fails, the call reports failure and leaves memory unchanged although the destination may already hold the new revision. Required follow-up: distinguish/reconcile post-rename durability failure and test both sides of rename. This is a local storage/retry consistency risk, not an error-detail or credential disclosure.

## Revalidated security controls

| Boundary | Exact candidate result |
|---|---|
| Tauri/webview | Restrictive self/IPC CSP; exact local-origin navigation allowlist; one typed deny-unknown-fields `ui_command` bridge; no filesystem/shell/network plugin. Server/user strings use `textContent`/form values, not HTML injection. Release registers only `ui_command`; debug-only QA code additionally requires explicit QA environment variables and isolated client data. |
| Trust/identity | Current-time, self-signature, SAN and handshake-signature checks remain enforced; pre-expiry key change fails. Address-to-registered-server and existing-identity/different-key collisions fail. Failed identity/profile/view/save and failed approval restore trust/settings/context. The accepted post-known-expiry fallback appends a durable event only on successful connection and produces a separate session-persistent visible banner. |
| Transport | Async and blocking clients receive clones of the same configured rustls `ClientConfig`; redirects and proxies are disabled; paths/opaque IDs are validated; JSON success/error bodies are capped at 4 MiB; only bounded safe API codes reach UI recovery text. |
| Local state | Client directory symlinks are refused; directory is `0700`; unique stages are create-new `0600`; settings input is capped. Persisted trust contains public certificate/key fingerprints and audit events, not credentials/private keys. Exception: `UISEC-002`. |
| Server request boundary | JSON request bodies are capped at 1 MiB; browser Origin/Sec-Fetch requests and mismatched Host/port are refused; current JSON routes have bounded request time; errors are fixed/typed. No cookie, session, CSRF token, redirect, or cloud identity surface exists in this slice. |
| Profile isolation | Every catalog/personal-state path requires the route profile before media access; personal rows remain keyed by `(profile_id, stable_media_id)`; queries are SQL-filtered and capped at 100; serialized paths are relative; common catalog rows survive profile deletion. This is profile data separation inside the accepted household-LAN no-auth model. |
| Cancellation/scan | Closed/expired queued actor work is skipped; live work has cancellation, SQLite progress and pre-commit hooks on both connections. Scan runs only `current_exe --scan-worker <root>` without a shell, with null stdin/stderr, capped stdout, kill-on-drop and timeout. A distinct writer commits media, last-scan marker and stable identities atomically. |
| UI notice | Trust replacement notice is distinct from transient messages and remains through catalog/admin refresh. Coordinator-reported exact headed modes `first`, `reopen_delete`, and `renewal_notice` all passed on fresh isolated fixtures; the renewal proof is a controlled expired-pin mutation, not a real 18-month wait. |
| Development package | Script requires a new absolute `.app`, refuses overwrite and non-arm64 macOS, uses a locked release build, lints the minimal plist, ad-hoc signs and strictly verifies. Plist has local-network purpose and no ATS exception, URL handler or entitlement. This remains a development bundle, not production notarization/installer approval. |

## Verification evidence

| Check | Result |
|---|---|
| `git rev-parse HEAD`; status before and after review | Exact `c2a9d72d634cc7282eb8a20ca9fa138ed92e2fec`; clean |
| `git diff --check d361eba6..c2a9d72`; exact blob/delta review | PASS; 89 insertions / 2 deletions across the stated six files |
| `cargo test --workspace --all-targets --locked` with Rust/Cargo 1.99.0 and isolated target | PASS: 124 tests (`65 + 18 + 11 + 10 + 20`), zero failures |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| Independent `scripts/package-macos.sh /private/tmp/ChillCenterSecurityR3-c2a9.app` | PASS: release build, plist lint, ad-hoc signature, strict verification; signed arm64 binary SHA-256 `b9966ce02cb60ab23cc22f47db76f145fb2167478ca462dc8c75adf0b9acd459` |
| Independent release binary inspection | No exported `qa_result`/`chillQa` symbol and no `CHILL_UI_QA_*`, `UI_QA_RESULT`, or `chillQaSmoke` string; no signing entitlement emitted |
| Coordinator's separate exact headed GUI/package proof | All three GUI modes PASS; bundle plist/signature PASS; separately signed binary SHA-256 `6aad62ca783248d365f84c7df8322de44f0b2d36bfce35e49ebae0617b0f1102` |
| Exact-tree credential/key filename scan | No committed private key, certificate, credential file, or high-confidence credential pattern found; pattern scan is not entropy analysis |

## Residual constraints

- Household LAN no-auth and ADR-0005 key replacement after a **known** expired certificate remain accepted decisions; this review does not expand them.
- A timeout/connection loss after a completed database commit is not a distributed exactly-once guarantee; the UI explicitly instructs reload before retry. The fixed cancellation path prevents the demonstrated queued/started mutation from committing after cancellation/deadline.
- Current JSON-route deadlines do not approve future media streams; those require separate per-read/idle controls.
- No real NAS/user data, physical Pi/Linux, real certificate-expiry interval, Wi-Fi load, production signing/notarization, installer, update channel, full acceptance suite, legacy import/export, or full player surface was exercised or approved.

## Report ownership

This review wrote only `/private/tmp/chillcenter-interface-security-review-r3.md` and preserved R2. It made no candidate source, test, configuration, cloud, NAS, or user-data change.
