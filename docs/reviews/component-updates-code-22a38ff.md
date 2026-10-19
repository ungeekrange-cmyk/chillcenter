# Independent code review — component updates `22a38ff`

- **Mode:** `DEV_STANDARD`, full first review (not an accelerated delta review)
- **Branch / frozen source:** `codex/component-updates-design` / `22a38ff50879837c32fc8d5ac7c7200f35c8eaa1`
- **Baseline:** `ac85794541a59b011be11ebbe839c3e30a293ea7`
- **Authors / integrator:** `/root/component_update_engine`, `/root`
- **Independent reviewer:** `/root/component_updates_code` (not an author; source-non-mutating)
- **Date:** 2026-10-19
- **Verdict:** **CHANGES_REQUESTED**

The approved ADR/user gate authorizes this implementation scope, but not merge,
publication, Pi/NAS access or deployment. The implementation has four functional
state-machine blockers below. The separate exact-SHA security review remains
`CHANGES_REQUESTED` with eight HIGH findings, so release is independently blocked
even after these code findings are corrected.

## Current review inputs and reused evidence

- Reviewed the exact base-to-candidate source delta (95 files, 8,962 insertions,
  44 deletions), ADR-0029 R2, `G-ADR-0029-R2`, and `docs/updates/CONTRACT.md`.
- Current independent peers on the same SHA: test review
  `docs/reviews/component-updates-tests-22a38ff.md` is `CHANGES_REQUESTED`;
  security review `docs/reviews/component-updates-security-22a38ff.md` is
  `CHANGES_REQUESTED`. Their test gaps and cryptographic/privileged-boundary
  findings are not duplicated here.
- Reused coordinator evidence: serialized workspace Rust suite PASS with two
  existing ignored NFR harnesses; Node 357/357 PASS; Chrome settings harness
  PASS after one sandbox-only launch retry. The mixed full-workspace player test
  has a recorded pass/fail/pass timing flake in BUG-0114 and no player source
  changed in this candidate.
- Reviewer checks: `git diff --check ac857945..22a38ff` PASS;
  `cargo test -p chill-updates --lib` 12/12 PASS; targeted component-release,
  update UI and web bridge Node suite 26/26 PASS. These suites do not cover the
  failing transitions identified below.
- Runtime lifecycle: the delta adds stable pinned updater/parser dependencies and
  no preview runtime. The unchanged repository toolchain pin is Rust 1.99.0; the
  current official patch is 1.99.1 (2026-10-15), a follow-up currency update to
  evaluate with regression evidence rather than an updater-specific blocker.
- `scripts/agent-context.py` was requested by repository instructions but is not
  present in this checkout; the command was therefore `NOT_RUN` after a direct
  failed lookup. This did not prevent exact-SHA/source/contract inspection.

## Findings

### `CU-CODE-001` — P1 — busy automatic installs are dropped instead of deferred

`apply_staged` returns `UpdateError::Busy` when work does not drain in the final
quiescence window (`crates/chill-updates/src/install.rs:85-104`). Linux
`perform_apply` maps every error, including `Busy`, to terminal `Failed`
(`crates/chill-updates/src/main.rs:676-706`). `tick` subsequently selects only
`Pending` operations and propagates an automatic apply error after the daily
attempt was already consumed (`main.rs:711-763`). Although `DeferredBusy` exists,
no production transition ever creates or resumes it
(`crates/chill-updates/src/state.rs:63-71,245-263`). A scheduled or LAN-requested
install during playback, scan/import/export/TMDB/fingerprint work therefore does
not queue until idle as ADR-0029 and `docs/updates/CONTRACT.md:35-37` require.

**Required correction:** transition a normal busy refusal to a durable bounded
deferred state, retry it while the same verified release and automatic/manual
intent remain valid, and preserve one global install slot. Add busy-to-idle,
restart-while-deferred, policy-change and retry-bound tests. Local-admin force
must remain a separate explicit path.

### `CU-CODE-002` — P1 — interrupted `running` operations wedge restart recovery

Checks durably transition to `Running` before transport/cache work
(`crates/chill-updates/src/main.rs:351-370`), and installs transition before trust
acceptance/application (`main.rs:543-640`). After a process or power interruption,
`tick` only resumes `Pending` operations (`main.rs:711-741`), while admission
coalesces `Running` operations forever (`crates/chill-updates/src/state.rs:245-263`).
A check then returns from `perform_check` without execution, and an install is
refused unless a separate accepted-release condition happens to be present
(`main.rs:355-362,560-568`). Crashes before trust acceptance have no supported
retry route at all.

**Required correction:** reconcile stale `Running` operations at helper startup
from durable transaction/trust/cache evidence. Resume only an exact safe phase;
otherwise mark the attempt terminal and admit a bounded retry. Add kill/restart
tests before and after each transition, including a crash before
`accept_pending`. This is distinct from the security review's cache-publication
and activation-authorization findings.

### `CU-CODE-003` — P1 — a cached failed/running check is reported as “available”

Within the five-minute check limit, `admit_intent` returns the most recent check
operation regardless of whether it succeeded or failed
(`crates/chill-updates/src/state.rs:274-285`). `perform_check` treats every
non-`Pending` result except the exact `no_update` code as `Available`
(`crates/chill-updates/src/main.rs:355-362`). The automatic scheduler then calls
`perform_apply` (`main.rs:745-752`), potentially consuming an older availability
receipt after the current check actually failed or while it is still running.

**Required correction:** derive `CheckResult::Available` only from a terminal
successful operation explicitly carrying `available`. Preserve `NoUpdate`
separately; running/deferred must coalesce without claiming a result, and failed
must remain an error so bounded retry applies. Test failed, running, available
and no-update cache hits under the rate window.

### `CU-CODE-004` — P1 — Mac/theme acceptance has no exact-release retry after install failure

The desktop path advances `highest_sequence` before invoking either the Tauri
SDK installer or theme application (`apps/desktop/src/update_bridge.rs:479-509,
517-562`). `record_sequence` permanently rejects the same sequence afterward
(`crates/chill-updates/src/state.rs:348-358`). Thus disk/install errors, a crash,
or the final theme playback race can leave the old app/pack active while the
only verified release is no longer retryable. Unlike Linux's pending
digest/version record and local `--retry-accepted`, the desktop command/state
model has no exact accepted-release recovery path. Native confirmation itself
is correctly placed immediately before Mac installation and cannot be supplied
by renderer fields (`update_bridge.rs:444-478,715-726`).

**Required correction:** persist an exact manifest digest/version/sequence
pending acceptance for desktop components and allow only that exact release to
resume after a failed/interrupted install, or commit the monotonic floor only
after success without weakening anti-rollback. Add SDK failure, crash, disk-full
and theme busy-race retry tests; retain native accept/cancel confirmation.

### `CU-CODE-005` — P2 — native transient server errors are mislabeled as old-server support gaps

The Rust bridge correctly converts only a `404` status response into the
unsupported-old-server view and preserves other failures
(`apps/desktop/src/main.rs:118-145`). The shared UI then catches every native
`server_status` rejection and replaces it with `{unsupported:true}`
(`ui/connected/updates.js:48`). The web bridge is more precise and only falls
back on `404`/`ROUTE_NOT_FOUND` (`web/src/bridge.js:482-496`), which its current
test covers. On Mac, a 503, TLS/network failure, or incompatible response is
therefore presented as “server too old,” losing the actionable error.

**Required correction:** preserve the last good status and surface transient
errors; use `unsupported` only for the bridge's explicit old-server response.
Add a DOM/native-bridge test for 404 fallback versus 503/network failure.

### `CU-CODE-006` — P2 — the generic scheduler mutates unsupported component ledgers

Every new state contains all four component policies, and `due_attempts` admits
all four (`crates/chill-updates/src/state.rs:105-117`;
`crates/chill-updates/src/scheduler.rs:32-79`). Linux rejects `mac`, so it records
daily failed Mac retries (`crates/chill-updates/src/main.rs:745-760,822-832`).
Conversely, the Mac loop filters out server/web only after `due_attempts` has
already persisted those dates (`apps/desktop/src/update_bridge.rs:572-615`).
This produces misleading state/log churn and makes the scheduler boundary harder
to reason about.

**Required correction:** scope scheduler admission to the caller's supported
component set (Linux: server/web/themes; desktop: mac/themes), and prove ignored
components do not mutate their daily/retry ledgers.

## Quality matrix

| Area | Status | Evidence |
| :--- | :--- | :--- |
| Acceptance behavior | **FAIL** | Busy-work deferral and recoverable install semantics are not met. |
| Correctness / failure modes | **FAIL** | Six state/error/scheduling findings above, four P1. |
| Architecture / trust boundaries | **PASS_SCOPED** | Fixed component DTOs, native Mac confirmation, declarative themes and separate helper boundaries match ADR-0029; security certification is separately rejected. |
| Compatibility / migration | **FAIL** | Old-server web fallback works, but native transient errors are misclassified. |
| Maintainability / scope | **PARTIAL** | Components are separated, but generic scheduling/result handling is state-string-driven and missing reconciliation. |
| Dependencies / runtime | **PASS_SCOPED** | Stable pinned additions; unchanged Rust patch-level currency follow-up noted above. |

## Verdict and routing

**CHANGES_REQUESTED.** Route the six localized state/UI issues to the builder.
Any correction creates a new source SHA and makes this report historical; code,
test and security reviews must all be current for that exact SHA. The eight HIGH
security findings and missing native/Linux execution evidence independently
block merge/release/publication/deployment. No Pi, NAS, GitHub release, tag or
public install is authorized by this report.
