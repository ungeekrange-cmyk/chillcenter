# Independent security review — component updates `3913267`

- **Reviewed source:** `3913267e98f82eeca5f7c627490fbdd019636177`
- **Security baseline:** `22a38ff50879837c32fc8d5ac7c7200f35c8eaa1`
- **Authors/integrator:** `/root` and assigned implementation workers
- **Independent reviewer:** `/root/v07_security_recheck` (not an author)
- **Reviewed at:** `2026-10-19T22:15:59Z`
- **Lane:** production/material-risk, full assurance
- **Verdict:** **APPROVED**

This verdict is bound only to the exact source SHA above. It approves the reviewed
security controls; it is not a tag, publication, deployment, native-GUI QA, Pi or
release-verification approval. The reviewer did not modify candidate source,
tests, VM, Pi, NAS, GitHub or cloud state. The only reviewer write is this report.
The production signing private key was never read; all reviewed signing tests use
committed public fixtures or disposable synthetic keys.

## Threat boundaries and security posture

Protected assets are the release trust root, installed Mac/server/web/theme
bytes, the Linux root-owned slots and trust ledger, the server database/config/TLS
backup, availability during activation, and the Mac user's install consent.
Relevant actors are a household LAN caller, the unprivileged `chillcenter` service,
local root/administrator, the fixed GitHub release origin, and an interrupted or
faulty signed candidate.

The change does not introduce an authentication, tenant, session, cookie, CSRF,
redirect or customer-data boundary. The server update API still accepts closed
component/policy/intent DTOs, not caller-supplied URL, command, path, key, force or
tag values. Existing unauthenticated household-LAN administration and its bounded
denial-of-service residual are explicitly accepted by ADR-0029/F-18; Internet
exposure remains forbidden. Downloads and health probes retain fixed origins,
redirect refusal, pinned trust, byte bounds and exact signed identity checks.
Changed diagnostics do not emit manifest signatures, tokens, private-key bytes,
backup contents or PII.

## Prior HIGH findings — closure

| Finding | Disposition and exact-source evidence | Runtime evidence |
| :--- | :--- | :--- |
| `CU-SEC-001` stale activation authorization | **REMEDIATED.** The one-shot authorization binds boot ID, live helper PID/start ticks, transaction operation and candidate; recovery consumes it by unlink plus parent fsync and otherwise rolls back. | Ubuntu/systemd fault matrix proves one use, stale/missing authorization rejection and active-candidate rollback with a different PID. |
| `CU-SEC-002` non-atomic cache publication | **REMEDIATED.** Complete bytes are written and verified in a UUID staging directory, fsynced, atomically renamed to the digest path, and only exact bounded updater temporaries are cleaned. | Injected interruption test proves no partial digest becomes visible and exact retry succeeds. |
| `CU-SEC-003` hostile bootstrap environment | **REMEDIATED.** Linux installers start with `#!/bin/bash -p`, set an absolute safe `PATH`, unset shell/Python injection variables, use `/usr/bin/python3 -I` and an absolute Minisign binary before privileged installation. | Installer test plus hostile PATH/environment fixtures pass; no production private key is involved. |
| `CU-SEC-004` unbounded backup process | **REMEDIATED.** The runner uses a separate process group, deadline, bounded stdout, discarded/bounded diagnostic handling and forced descendant cleanup. UID/GID/groups and capability removal are checked after the root helper drops privilege. | Actual VM TERM/KILL cases pass; backup child runs as the service UID/GID with empty supplementary groups and zero `CapPrm`, `CapEff`, `CapAmb`. |
| `CU-SEC-005` staging after outage | **REMEDIATED.** Extraction, validation, exact permissions and durable fsync complete before maintenance/stop; the short window performs backup and atomic switch only. | Signed web/server apply and health/rollback paths pass in the VM matrix. |
| `CU-SEC-006` recovery under server sandbox | **REMEDIATED.** `ExecStartPre=+` is retained for fixed root recovery while the main service now has only `ReadWritePaths=/var/lib/chillcenter`; component roots are not writable to the service. The earlier review premise that `+` retained the filesystem namespace was incorrect per systemd command-prefix semantics. | Ubuntu systemd 255 proves root pre-start recovery with `ProtectSystem=strict`, `ProtectProc=invisible`, `ProcSubset=pid`, and service-UID read/no-write slot access. |
| `CU-SEC-007` unreadable Linux slots | **REMEDIATED.** Finalization rejects links/special entries and fixes root ownership with traversable directories, readable regular files and executable server payload before switch. | Real service UID reads/executes the slot and cannot create a file in the root-owned component root. |
| `CU-SEC-008` no immediate failure recovery | **REMEDIATED.** Atomic no-replace transaction markers, fixed `ExecStopPost=rescue`, verified inactive state with zero main/control PIDs, rollback, flag unlink+fsync and service restart provide immediate recovery. `KillMode=control-group`, bounded escalation, and updater `TimeoutStopSec=150s` cover the approximately 115-second pessimistic rescue budget. | Genuine updater TERM/KILL, descendant cleanup, post-activation kill, and a candidate ignoring TERM all restore the prior active service; marker cleanup and already-deleted-candidate rollback are idempotent. |

No Critical, High, Medium or Low finding remains open in the reviewed scope.

## Additional controls reviewed

- Root trust acceptance occurs only after idle admission and durable backup.
  Failed accepted transactions remain exact local receipts rather than advancing
  general release trust. Retry is available only through root CLI
  `--retry-accepted`, revalidates signature, digest, sequence, version, current
  compatibility and `issued_at <= accepted_at < expires_at`, and is capped without
  LAN-triggered reset. Normal checks and new releases remain current-expiry strict.
- Mac pending receipts bind exact signed bytes, digest, sequence, version, source
  commit and acceptance time. Every actual SDK install still requires immediate
  native confirmation. Test-only key injection is a private function seam; all
  production calls use the embedded fixed key.
- Tauri bootstrap configuration now deserializes with the exact base64-encoded
  production public key, `requireSignedVersion=true`, no endpoints, insecure
  transport/certificate/hostname options false and downgrades false. A unit test
  decodes it and compares it byte-for-byte with `updates/release.pub`; the builder
  uses the same constant.
- `AmbientCapabilities=CAP_SETUID` is the minimum added capability needed for the
  root helper to drop to the service account under `NoNewPrivileges=true`; it is
  already in the narrow bounding set. The child clears UID/GID, supplementary
  groups and permitted/effective/ambient capabilities before executing backup.
- Health startup retry has one 15-second global deadline, at most two seconds per
  fixed CA-pinned request, no redirects, a 1 MiB body limit and exact installed
  version matching. Cache/cache-receipt, archive, backup and state entry/byte
  bounds remain explicit.
- The Rust toolchain is pinned to `1.99.1`, confirmed current stable for this
  release evidence. No dependency lockfile or new third-party dependency changed
  in the remediation delta.

## Reproducible evidence

| Tool/version | Exact command or artifact | Result | Scope / limitation |
| :--- | :--- | :--- | :--- |
| Git 2.54.0 | `git rev-parse HEAD`; `git diff --check 22a38ff..3913267` | PASS; exact `3913267e98f82eeca5f7c627490fbdd019636177` | Unrelated pre-existing/untracked review/documentation work was preserved and excluded. |
| Rust/Cargo 1.99.1 | `cargo fmt --all -- --check` | PASS | Formatting only. |
| Rust/Cargo 1.99.1 | `CARGO_TARGET_DIR=/private/tmp/chillcenter-security-e85f5bb-target cargo test --locked -p chill-updates` | PASS: 18 library, 3 helper, 4 hostile-input, 1 installer; Linux-root tests correctly filter to 0 on macOS | Exact final SHA; local non-root/macOS execution. |
| Rust/Cargo 1.99.1 | `CARGO_TARGET_DIR=/private/tmp/chillcenter-security-e85f5bb-target cargo test --locked -p chillcenter-desktop update_bridge` | PASS: 8, 28 filtered | Exact final SHA; exercises real Minisign with an ephemeral test key, not native GUI interaction. |
| Node 24.13.0 | `node scripts/test-update-helper-hardening.cjs` | PASS | Exact final SHA; static contract only. |
| Git pattern scan | private-key/token patterns over tracked final tree, excluding public signed fixtures | No match | Pattern scan is not full history or entropy scanning. |
| Ubuntu ARM64 / systemd 255 evidence supplied by coordinator | `/private/tmp/chillcenter-v07-linux.yvKFaz/fault-test-4.log`, SHA-256 `81395fae0f13c2fd11c758e872ba8bdf989ef4404ab1ff01079e61187a8ad681` | PASS matrix: sandbox/one-shot activation, TERM/KILL backup, process-group cleanup, active candidate ignoring TERM, rollback/start, Busy→Deferred→idle, policy origin/cancellation, exact accepted-expiry retry, local consent, signed install and health rollback | Reviewer inspected, but did not operate, the disposable VM. Linux runtime binaries were built from functionally identical runtime source at `91addb3`: server SHA-256 `68d526cd3f3af7cee9b07b06c62f46260656dda7cccc4c12994dee85431771e5`, updater `24d728b48f29884d4f55a05773800a7091f2531e07564944e8b10f4993afdbac`. Later deltas are tests, unit timeout and Mac bootstrap configuration only. |
| Ubuntu ARM64 root ledger | `/private/tmp/chillcenter-v07-linux.yvKFaz/root-ledger-tests.log`, SHA-256 `76105186de85251a09323fb7a5650b525b14d63dbc05f143f7d66b3484ec6903` | PASS: 2/2 | Corrupt/symlink ledger refusal and exact pending persistence. |

## Limitations and release boundary

No Pi/NAS action was requested or performed. The matrix does not simulate a
physical power cut, disk exhaustion or GitHub transport failure. A physical/native
Mac launch and confirmation flow is a separate functional/release-verification
gate and is not claimed here; the final bootstrap regression is covered by exact
configuration deserialization and targeted Rust tests. Publication must still use
immutable artifacts built from this exact source, verify downloaded hashes and
signatures, preserve the local private signing key outside Git/GitHub, and complete
the distinct code, test, packaging and release-verification gates.
