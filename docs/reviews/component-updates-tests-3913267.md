# Independent test execution review — component updates `3913267`

- **Review mode / lane:** `EXECUTION_REVIEW` / `FULL_ASSURANCE`
- **Branch / exact source:** `codex/release-v0.7-fixes` /
  `3913267e98f82eeca5f7c627490fbdd019636177`
- **Parent test verdict:** `88dab67c8dc4bcbc781d0cd772830f71aba341d6` /
  `CHANGES_REQUESTED` in `component-updates-tests-88dab67.md`
- **Authors / coordinator:** component-update builders and `/root`
- **Independent reviewer:** `/root/v07_final_tests`
- **Environment:** macOS 27.0.1 (26A434), arm64; Rust/Cargo 1.99.1;
  Node 24.13.0; Minisign 0.12; disposable systemd Ubuntu ARM64 VM for the
  root boundary (not Pi/NAS)
- **Verdict:** **CHANGES_REQUESTED**

Four of the five prior test findings are closed with meaningful behavioral
assertions. The Linux harness now drives the production binaries and units
through real HTTP busy/deferred recovery, durable origin and retry semantics,
accepted-time recovery, root ledger, privilege drop, TERM/KILL, descendant
cleanup, activation, health failure and rollback. Public-only hostile fixtures
also reach the production verifier and archive/state boundaries.

Public v0.7 publication is still not approved. The required native Mac journey
has not completed. Its first launch aborted before any dialog because the
embedded updater configuration lacked the SDK-required public key. The source
fix at this exact SHA has a strong deserialization regression test and passes
the focused Rust suite, but no rebuilt exact-source app has yet shown cancel,
accept, install/failure preservation, and restart-consent behavior through real
Tauri/AppKit. Missing execution is not a pass.

## Reproducible evidence

| Command / evidence | Exit | Duration | Result | Skips / retries | Scope and limit |
| :--- | :---: | :---: | :--- | :--- | :--- |
| `cargo test -p chillcenter-desktop --locked --offline update_bridge::tests -- --test-threads=1` | 0 | 1.39 s wall | 8 passed | 0 | Reviewer rerun at exact SHA; includes real SDK config deserialization and ephemeral Minisign accepted-time cases |
| `cargo clippy -p chillcenter-desktop --locked --offline --all-targets -- -D warnings` | 0 | 28.73 s | PASS | 0 | Reviewer rerun at exact SHA |
| `cargo fmt --check` and `node scripts/test-update-helper-hardening.cjs` | 0 | 1.23 s combined | PASS | 0 | Reviewer rerun; formatter plus static unit contract, not systemd execution |
| `/private/tmp/chillcenter-v07-workspace-final-2.log` (SHA-256 `51920ea3…aef4`) | 0 reported | 133.67 s test bodies plus 28.48 s build | 433 passed, 0 failed | 2 established manual NFR harnesses ignored | Broad Rust regression at parent `6041f4d`; exact-SHA desktop delta independently rerun above |
| `/private/tmp/chillcenter-v07-node-all-final.log` (SHA-256 `68b624a2…808f`) | 0 reported | 11.70 s | 359 passed, 0 failed | 0 | Broad Node regression; exact-SHA delta changes only native updater config/test |
| `/private/tmp/chillcenter-v07-settings-final.log` (SHA-256 `0d1b2181…52a`) | 0 reported | not captured | PASS | 0 | Synthetic Chromium bridge only; explicitly not native consent proof |
| `/private/tmp/chillcenter-v07-linux.yvKFaz/fault-test-4.log` (SHA-256 `81395fae…681`) | 0 reported | not captured | all harness checkpoints PASS | none reported | Disposable Ubuntu ARM64/systemd, real binaries and production orchestration; no Pi/NAS, physical Mac, power-cut/disk-full, or GitHub fault |
| `/private/tmp/chillcenter-v07-linux.yvKFaz/root-ledger-tests.log` (SHA-256 `76105186…903`) | 0 reported | 0.01 s | 2 passed | 0 | Real Linux root ownership and on-disk ledger |
| `/private/tmp/chillcenter-qa-native-updates.w7d2xJ/launch.log` | abort | before dialog | **FAIL at `6041f4d` bundle** | no retry counted as pass | Real Tauri startup exposed missing SDK `pubkey`; no consent/install assertion executed |

The Linux VM exercised the updater/server implementation after the functional
fixes. Subsequent Linux-relevant deltas are a Rust 1.99 lint-only change,
test formatting/naming, a larger `TimeoutStopSec` budget, and static enforcement
of read-only component mounts. The exact-source Mac-only `3913267` delta cannot
invalidate those Linux results. The final 150-second updater stop budget is
asserted statically; the earlier VM run exercised the same bounded rescue code,
not that exact unit timeout value.

## Prior-finding closure matrix

| Prior ID | Result | Assertion quality and evidence |
| :--- | :--- | :--- |
| `CU88-TEST-001` production state machine | **CLOSED** | The VM uses actual `chill-update`, server HTTP status, systemd and persisted state. It asserts manual `automatic=false`, bound digest, Busy→`deferred_busy`, unchanged slot/floor, same operation ID succeeding on the next idle tick, automatic origin surviving Busy then cancellation after policy change, retry cap, corrupt-preflight terminalization, and active-candidate reconciliation. These are observable state/orchestration assertions, not regex checks. |
| `CU88-TEST-002` accepted retry time semantics | **CLOSED** | Desktop exact-source tests generate an ephemeral Minisign key, accept signed bytes before expiry, reject fresh discovery after expiry, allow only the exact persisted bytes at accepted time, reject a separately valid same-sequence substitution and future acceptance. VM production CLI additionally requires local `--retry-accepted`, preserves cached bytes, reaches real backup/activation/health rollback, and preserves the monotonic floor. No test private key is tracked. |
| `CU88-TEST-003` native Mac journey | **OPEN / HIGH** | Synthetic Chrome and Rust seams are insufficient. The only real launch aborted before the prompt at the parent SHA. `3913267` now proves the exact embedded config deserializes with the production public key, signed-version requirement, empty endpoints and all insecure/downgrade flags false, but a rebuilt exact-source Tauri/AppKit journey has not run. |
| `CU88-TEST-004` frozen Linux fault evidence | **CLOSED_SCOPED** | The committed guarded harness refuses non-root/non-disposable hosts and executes actual signed offline cache, service-UID backup with no supplementary groups or capabilities, KILL and TERM in the backup window, child cleanup, active-candidate `ExecStopPost` recovery including an ignore-TERM candidate, healthy old-slot restoration and idempotent recovery. Root ledger is 2/2. Scope is Ubuntu ARM64 VM, not Pi or production. |
| `CU88-TEST-005` hostile-input matrix | **CLOSED_SCOPED** | Four Rust integration tests use signed Mac/web/theme fixtures through `parse_and_verify_manifest`, reject ZIP symlink/special and canonical duplicate entries, entry/file/total/ratio limits with no staging residue, wrong raw byte length/hash, corrupt state and unbounded history. Fixed GitHub URL/redirect policy and atomic cache publication are additionally exercised by code-integrated and VM paths; live GitHub transport remains release evidence, not claimed here. |

## Quality assessment

- Assertions inspect durable state, operation identity/origin/attempts, symlink
  targets, service health, process credentials/capabilities, child liveness,
  filesystem residue and anti-rollback floors. They are not success-message-only
  or mock-only substitutes for the Linux boundary.
- Test data is isolated in temporary directories or the hostname-guarded,
  disposable VM. The Linux harness explicitly refuses Pi/NAS/workstation use.
- The two ignored workspace tests are pre-existing manual memory/RSS NFR
  harnesses and do not cover updater release safety. They are not counted as
  passes and do not explain the native gap.
- The historical subtitle timing instability remains documented separately;
  the final serial workspace log is green, but this review does not erase the
  earlier pass/fail/pass history.
- No coverage percentage was supplied. Approval is based on boundary and branch
  assertions, not aggregate counts.

## Finding and route

| ID | Severity / type | Evidence | Required action | Route |
| :--- | :--- | :--- | :--- | :--- |
| `CU391-TEST-001` | **HIGH — required native journey not executed** | Parent bundle launch aborted before any prompt (`launch.log`); exact-source config regression is green, but no exact-source AppKit result exists. | Build and hash the `3913267` candidate, then execute and record real cancel/accept, no install before click, busy before/after consent, invalid metadata/package rejection, installer failure preserving the current app, successful install/restart, and fresh consent after restart. Preserve the failed launch as adverse history. | Native test owner; fresh independent execution review |

## Gate conclusion

`3913267` closes the Linux/state/time/hostile-input findings from the previous
test review, and the newly discovered startup defect has an appropriate focused
regression test. It does not close the native end-to-end requirement. Therefore
the exact-source test gate remains **CHANGES_REQUESTED** and public v0.7
publication remains blocked. No Pi deployment is required or claimed. A later
source commit makes this verdict historical; a report-only carrier does not.
