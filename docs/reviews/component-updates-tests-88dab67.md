# Independent test execution review — component updates `88dab67`

- **Review mode:** `EXECUTION_REVIEW`
- **Delivery lane:** `FULL_ASSURANCE` (public v0.7 publication and privileged updater)
- **Branch / exact source:** `codex/release-v0.7-fixes` /
  `88dab67c8dc4bcbc781d0cd772830f71aba341d6`
- **Previous review:** `docs/reviews/component-updates-tests-22a38ff.md`
- **Authors / coordinator:** component-update builders and `/root`
- **Independent reviewer:** `/root/v07_test_recheck`
- **Environment:** macOS 27.0.1 (26A434), Apple Silicon; Rust/Cargo
  1.99.1; Node 24.13.0
- **Verdict:** **CHANGES_REQUESTED**

The exact source substantially improves the focused proof: the production
manifest entry point now receives signed synthetic fixtures, activation permits
are one-use and process-bound, prepared slots are asserted before maintenance,
and recovery/idempotence tests are meaningful. The focused suites are green.

Approval is nevertheless not justified. The four prior P1 state/retry fixes are
not exercised through their production orchestration branches, accepted-release
expiry recovery is not asserted, and no real Tauri/AppKit confirmation/install
journey ran. The committed Linux harness also explicitly excludes signed apply
and TERM/KILL fault injection. A stronger harness was edited after the source
freeze, but it was uncommitted and had no completed evidence artifact at review
time; it is not proof contained in `88dab67`.

## Reproducible evidence

| Command / source | Exit | Duration | Result | Skips / retries | Scope and limit |
| :--- | :---: | :---: | :--- | :--- | :--- |
| `cargo test -p chill-updates --locked --offline` | 0 | 2.19 s wall | 21 passed: 18 library, 2 helper, 1 installer | Linux root module ran 0 tests on macOS | Reviewer rerun at exact HEAD; production parser, recovery core, bounded backup and static installer check |
| `node --test ui/connected/updates.test.js` | 0 | 0.28 s wall | 4 passed | 0 | Reviewer rerun; exact closed UI behavior and transient-vs-404 classification |
| `node scripts/test-update-helper-hardening.cjs` | 0 | 0.05 s wall | script PASS | N/A | Reviewer rerun; source-text assertions only, not execution of helper orchestration |
| `CHILL_PREVIEW_NODE_MODULES=/private/tmp/marvin-cockpit.NmYyTL/package-manager/node_modules node scripts/test-component-settings.cjs` | 1 | 0.56 s wall | Chrome launch aborted | one sandbox-only attempt | Tooling failure (`SIGABRT`/kill `EPERM`), not a candidate assertion failure; coordinator log records the same synthetic-bridge suite passing outside the sandbox |
| `/private/tmp/chillcenter-v07-focused-tests.log` | 0 reported | command/duration not fully recorded | 18 updater library + 2 helper + 1 installer + 33 desktop passed | Linux root 0 on macOS | Coordinator evidence; includes 5 desktop update tests but no native AppKit journey |
| `/private/tmp/chillcenter-v07-node-tests.log` | 0 reported | 5.71 s Node runner | 310 passed | 0 | Broad Node regression; exact aggregate shell invocation absent from the log |
| `cargo test --locked --offline --workspace --all-targets -- --test-threads=1` | 0 reported by coordinator | not captured | latest serial workspace PASS; player 19/19 | two established ignored NFR harnesses in prior baseline | Useful regression evidence, but no exact final log artifact was supplied to this reviewer |

The earlier `/private/tmp/chillcenter-v07-tests-unrestricted.log` is retained as
adverse history: it failed on the stale Minisign shell literal before the final
fix. The later exact updater rerun above is green. Earlier sandboxed player socket
failures are environment failures; the coordinator's unrestricted serial rerun
reports all 19 player tests green. `BUG-0114` remains historical flaky evidence:
the latest pass does not retroactively turn the prior pass/fail/pass sequence
into deterministic proof, although no player test was skipped or relaxed here.

## Risk-to-evidence matrix

| Risk / contract | Result | Assertion quality and limit |
| :--- | :--- | :--- |
| Production signed manifest path | **PASS_SCOPED** | `production_parser_verifies_signed_fixture_matrix` invokes `parse_and_verify_manifest` with 13 signed synthetic server cases and checks tamper/invalid key/oversize. It has one valid server fixture, not a valid-fixture matrix for mac/web/themes. |
| Activation authorization and rollback core | **PASS_SCOPED** | Strong observable assertions cover stale/forged/expired/PID/identity/operation/target permits, one-use consumption, rollback and idempotence. Rootless unit hooks do not substitute for every real systemd kill point. |
| Prepared slot permissions/outage boundary | **PASS_SCOPED** | The test proves preparation and mode before `begin_quiescence`, and proves busy exits before backup/switch. Maximum-archive outage and slow/disk-error injection are not measured. |
| Bounded backup child | **PASS_SCOPED** | Actual child success, nonzero exit, excessive output and hung process-group cases execute. The systemd TERM/KILL path is outside the committed exact-source suite. |
| Durable busy/retry/origin state | **PARTIAL** | The storage test proves exact digest and `automatic=true` survive reopen, but not the inverse manual-origin binding, production Busy-to-Deferred transition, tick-to-idle resume, policy cancellation, retry exhaustion, or global-slot races. |
| Crash reconciliation | **NOT_RUN at production branch** | No test calls `reconcile_store`, `perform_check`, `perform_apply` or `tick` across pre/post-acceptance crash states. Static regex checks merely prove source tokens exist. |
| Exact accepted-release retry after expiry | **NOT_RUN** | Linux and desktop code use persisted `accepted_unix`, but tests validate storage/digest only; none verifies the exact signed bytes after wall-clock expiry while rejecting a different or future acceptance. |
| Native Mac confirmation/install/restart | **NOT_RUN** | Rust and DOM tests prove the renderer cannot inject consent and the SDK path compiles. No real Tauri/AppKit cancel/accept, busy-after-consent, signed-package rejection, installer failure, preservation or restart-consent journey ran; no GUI driver was installed. |
| Linux root/systemd service boundary | **PARTIAL / external** | Coordinator reports a disposable Ubuntu ARM64 root/service-boundary pass. The committed harness at `88dab67` states signed cache/apply and TERM/KILL are not covered. A post-freeze uncommitted harness (`SHA-256 05fdd2bd440886e6580f1c6030f8fc3c89395ad3bdf45e8633f50221b42a8643`) adds those cases, but no completed command/log/exit evidence was available here. No Pi is required or claimed. |
| Transport/archive/state attack matrix | **PARTIAL** | Fixed URL and one tar traversal case pass. Redirect behavior, downloaded byte/hash mismatch, ZIP special/duplicate/limit cases, cache publication crash points, operation-history bound and corrupt service-state restart remain without representative execution evidence. |

## Findings and routing

| ID | Severity / type | Evidence | Required action | Route |
| :--- | :--- | :--- | :--- | :--- |
| `CU88-TEST-001` | **HIGH — missing production state-machine execution** | `state.rs` tests directly transition storage, while `main.rs` orchestration has no behavioral tests; the helper JS only regex-matches source. The manual-origin branch and crash reconciliation are unasserted. | Add a test-only production harness covering manual and automatic origin binding, Busy-to-Deferred-to-idle, policy change, three-attempt bound, global slot, cached check states, and crashes before/after root acceptance. | Builder/test author, then fresh independent test review |
| `CU88-TEST-002` | **HIGH — accepted retry time semantics unproved** | Pending receipts persist `accepted_unix`, but neither Linux nor desktop tests invoke verification at that persisted instant after current-time expiry. | With only synthetic pre-generated signatures, prove exact accepted bytes retry after expiry, future acceptance rejection, and rejection of changed digest/sequence/version/source; cover both Linux and desktop seams. | Builder/test author, then fresh independent test review |
| `CU88-TEST-003` | **HIGH — native Mac journey absent** | All Mac consent evidence is Rust/source/Chrome with a synthetic bridge; AppKit and the real updater SDK were not driven. | Execute a real candidate bundle journey for cancel/accept, no install without click, busy-before/after consent, bad metadata/package, failed install preserving current app and restart consent. Record bundle/source hashes and visible results. | Native test owner; fresh independent execution review |
| `CU88-TEST-004` | **HIGH — frozen Linux fault evidence incomplete** | The committed VM harness explicitly prints `NOT COVERED` for signed apply and TERM/KILL. The post-freeze harness was dirty and unevidenced at review time. Atomic cache-publication crash points remain outside it. | Freeze the test-only harness in a new candidate, record exact Ubuntu image/tool versions, command, exit and full log; execute TERM/KILL around stop/backup/marker/switch/start and cache-write/rename faults, proving old healthy service recovery and cleanup. | Builder/test author + integration manager; fresh security and test reviews |
| `CU88-TEST-005` | **MEDIUM — remaining hostile-input matrix is narrow** | Only a valid server manifest, one URL unit and one tar traversal test represent broader component/transport/archive/state risks. | Add valid fixtures for each component and targeted redirect, byte/hash, ZIP/raw limit/symlink/duplicate, corrupt state and bounded-history cases, or document an independently reviewed exclusion. | Test strategist/builder |

## Gate conclusion

`88dab67` is materially stronger than `22a38ff` and its focused suites are green,
but missing or skipped critical checks are not passes. **CHANGES_REQUESTED** is
required before public v0.7 publication. This is not `BLOCKED`: the missing
proof can be supplied by a test-only candidate plus native Mac execution. Any
new commit, including a harness-only commit, makes this exact-SHA verdict
historical and requires a fresh review; a report-only carrier would not.
