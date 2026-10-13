# Independent Security Review — CC-UI-001 Mac UI/server slice

- **Branch / candidate / base**: `codex/interface-server` / `b8737bba65f010e349a8387201faf8d73cebe2b0` / `fe85cea00da8a7e1f35abf0a5f057907eb235b47`
- **Author instance**: `/root`
- **Reviewer instance**: `/root/ui_security_review`
- **Reviewed at UTC**: `2026-10-13T17:49:02Z`
- **Lane**: `DEV_STANDARD`; independent read-only security review
- **Scoped verdict**: `CHANGES_REQUESTED`

This verdict is limited to the changed Tauri interface, Rust UI bridge/client transport, local client settings, catalogue/profile HTTP routes, and scan-subprocess wiring in `CC-UI-001`. It does not certify the full app, player/lease/stream/import/export paths, Linux/Pi deployment, NAS behavior, 133 acceptance criteria, or G-04. The household-LAN/no-auth choice and automatic key replacement after known-certificate expiry remain accepted constraints under F-18/F-19 and ADR-0005 and are not reopened here.

## Threat boundary result

- **Webview → Rust**: one `UiCommand` enum uses `deny_unknown_fields`; capability scope is the `main` window and only `ui_command` plus the debug-only QA result permission. No shell/file/network plugin is present. Navigation is restricted to Tauri local origins and CSP disables remote scripts, frames, objects, forms, and arbitrary connects. Server-derived text is assigned with `textContent`/`value`, not HTML.
- **Rust client → server**: initial certificate inspection performs TLS only; approval uses a 120-second random pending token. The pinned transport validates certificate structure, current validity, SAN, self-signature and TLS handshake signatures. `BUG-0004` correctly passes cloned `ClientConfig` values to both reqwest clients. Redirects and proxies are disabled; request paths are fixed/validated; JSON responses are streamed with a 4 MiB cap.
- **Profiles/server registration**: `server_id`, profile IDs, and media IDs are UUID-validated before becoming formatted paths. Last profile is keyed by registered server UUID. Profile data is partitioned in storage and addressed explicitly; profiles are selectors, not confidentiality tenants, as required by the accepted no-auth design.
- **Server HTTP/CSRF**: browser `Origin`/`Sec-Fetch-Site` requests and unexpected Host/authority values are refused. There are no cookies or authenticated sessions, so no session fixation/logout/CSRF token invariant exists in this slice. JSON bodies are capped at 1 MiB and public errors/logs are generic and redacted.
- **Catalogue/scan**: queries are parameterized, page/search/category bounds are enforced, and catalogue views replace absolute paths with stored relative paths. Scan uses `current_exe` with separate argv (no shell), one active scan, a frozen root, null stdin/stderr, `kill_on_drop`, 300-second timeout and 64 MiB output cap; the prior catalogue is retained on error. No transcode or NAS write is introduced.
- **Local storage**: the app directory is rejected if it is a symlink and is forced to `0700`; staged settings files use unique `create_new` names and `0600`. Stored trust material is public certificate/pin data, not private keys or credentials.

## Findings

### `UISEC-001` — MEDIUM — post-expiry key replacement is not visible

- **Asset / preconditions**: user awareness of the accepted LAN substitution risk; a registered certificate has expired and a different currently valid SAN-matching key is presented.
- **Scenario and evidence**: `crates/chill-client/src/lib.rs:456-474` automatically applies the new certificate and appends `KeyReplacedAfterKnownExpiry`. `crates/chill-client/src/ui.rs:231-266,291-306` never reads trust events or adds a connected-state notice, and `ui/connected/app.js:9-31` has no trust-history/event surface. The connection therefore looks ordinary. ADR-0005 requires an **event visible** when this accepted fallback occurs.
- **Impact**: removes the designed detection signal after the explicitly accepted automatic replacement; a household user cannot distinguish normal reconnect from fallback pin replacement.
- **Required remediation/retest**: expose each unseen `KeyReplacedAfterKnownExpiry` event prominently after connection (with server/address, time and new fingerprint), persist acknowledgement separately, and add a simulated-expiry regression proving SAN/time/signature checks still apply and the event is both persisted and visible. Do not change the accepted fallback or add authentication.
- **Disposition**: `OPEN`, blocks this exact SHA's security approval. Owner: builder/coordinator before re-review. Accepted-risk ADR: ADR-0005 governs the fallback, not omission of its visibility control.

### `UISEC-002` — LOW — local trust/settings rename lacks directory durability

- **Asset / preconditions**: persisted server pin/mapping and last-profile settings; crash or power loss immediately after a successful save.
- **Scenario and evidence**: `crates/chill-client/src/ui.rs:181-204` fsyncs the staged file and renames it to `client.json`, but does not fsync the parent directory or reconcile a post-rename sync failure. The UI can report success without evidence that the directory entry is durable.
- **Impact**: a restart can lose a newly approved pin or setting and force a new explicit trust prompt. This fails closed, but does not meet the stated persistence invariant.
- **Required remediation/retest**: fsync the settings directory after rename, clean staging files on all pre-commit failures, and add fault-injection tests distinguishing pre-rename failure from post-rename durability failure. Coordinate semantics with the already-open server finding `SRV-SEC-005`.
- **Disposition**: `OPEN`, nonblocking by severity but must remain owned. Owner: builder/coordinator. Accepted-risk ADR: N/A.

### `UISEC-003` — LOW — registered pin mutations are not transactional with identity validation

- **Asset / preconditions**: a registered server pin; either explicit replacement approval or the accepted post-expiry replacement path, followed by `/api/v1/server` failure or a different/invalid `server_id`.
- **Scenario and evidence**: `crates/chill-client/src/ui.rs:409-443` applies an explicit replacement before `finish_connect`; `crates/chill-client/src/lib.rs:456-474` also mutates the shared pin during the TLS handshake. The expected server identity is checked later at `ui.rs:313-360`. On failure, no rollback occurs; a later unrelated `save()` can persist the mutated entry. Initial approvals can similarly leave orphan `pending:<token>` entries after failed identity retrieval.
- **Impact**: a connection that is reported as rejected can still poison or bloat local trust state and impede the legitimate server. This does not expand the already accepted post-expiry impersonation risk, because an attacker in that scenario can return the public expected UUID, but it violates fail-atomic registered-server separation.
- **Required remediation/retest**: stage certificate/pin/event changes until the expected UUID and API version are validated, then commit them atomically; discard staged/orphan entries on every failure. Test identity mismatch, network failure, collision, and restart persistence.
- **Disposition**: `OPEN`, nonblocking LOW but must remain owned. Owner: builder/coordinator. Accepted-risk ADR: N/A.

`SRV-SEC-005` from the prior server review remains an open nonblocking LOW for server `library.json` post-rename fsync reconciliation; this candidate does not resolve it.

## Reproducible evidence and limitations

| Tool/evidence | Exact action | Result | Limitation |
| :--- | :--- | :--- | :--- |
| Git 2.54.0 / rg 15.2.0 | `git rev-parse HEAD`; clean-status check at review freeze; `git diff --check fe85cea..b8737bb`; exact diff/static trace across all 23 committed paths | Exact candidate and base confirmed; committed diff clean; no source mutation by this reviewer | Static review cannot prove GUI/runtime behavior. After the freeze, the shared checkout acquired an uncommitted test-only edit in `crates/chill-client/src/ui.rs`; it is outside this exact-SHA verdict and was not reverted or reviewed. |
| Git/rg credential scan | Changed `apps`, `crates`, and `ui` blobs scanned for private-key and credential assignment patterns; exact-tree sensitive filename scan | No committed credential, private key/certificate, or customer data found | Pattern/filename scan is not entropy-based secret detection |
| Lockfile inspection | Package/source/checksum blocks and primary runtime entries inspected | 512 packages; 506 registry sources with checksums; 0 Git sources; locked Rust `1.99.0`, Tauri `2.12.3`, Tauri Build `2.7.2`, reqwest `0.13.5`, rustls `0.23.45` | No comprehensive advisory/license scanner was installed; current-version primary verification was supplied by the coordinator |
| Prior independent review | `docs/reviews/SERVER-SECURITY-REVIEW-r2.md` at `a06d95f29da099a7a93769420fa5ce62a6478581` | Prior bounded server foundation approved; one LOW retained | Not approval of this candidate's new routes/UI |
| Rust checks | `rustc --version && cargo --version && CARGO_TARGET_DIR=/private/tmp/chillcenter-security-target cargo test --workspace --all-targets --locked` | `NOT_RUN`: required non-login shell returned `rustc: command not found` (exit 127) before Cargo execution | Coordinator reports 114 tests, clippy, and two real-webview loopback fixture flows, but no raw transcript is committed, so they were not independently rerun/attested here |
| Runtime environment | Read-only source and fixture evidence | QA hook is debug-gated, requires `CHILL_UI_QA_URL` plus `CHILL_UI_QA_DATA`, and uses no real NAS/user data in the reviewed design | Linux/Pi, real NAS, physical LAN, player/video decoding, power-loss, and scan race tests unavailable/not run |

No open HIGH or CRITICAL issue was found. `UISEC-001` nevertheless prevents an `APPROVED` DEV_STANDARD security verdict because the exact candidate omits a compensating control required by its accepted TLS ADR. Any source change makes this report historical and requires a current exact-SHA review.

## Owned path

- `/private/tmp/chillcenter-interface-security-review.md` only. Candidate source/tests/cloud state were not modified.
