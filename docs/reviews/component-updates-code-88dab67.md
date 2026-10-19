# Independent code re-review — component updates `88dab67`

- **Mode:** `DEV_STANDARD`, exact-SHA operational delta re-review
- **Branch / frozen source:** `codex/release-v0.7-fixes` / `88dab67c8dc4bcbc781d0cd772830f71aba341d6`
- **Baseline:** `ac85794541a59b011be11ebbe839c3e30a293ea7`
- **Prior rejected code review:** `component-updates-code-22a38ff.md`
- **Independent reviewer:** `/root/v07_correctness_recheck` (not an author; source-non-mutating)
- **Date:** 2026-10-19
- **Verdict:** **APPROVED_SCOPED**

No unresolved P0, P1 or P2 code finding remains in C001–C006. This closes the
six findings from the historical `22a38ff` code review for the exact source SHA
above. It does not reclassify that historical report and does not substitute for
current exact-SHA security/test approval or immutable release verification.

## Inputs and evidence

- Read the repository instructions, project context, ADR-0029 R2 and its user
  gate, the update contract, the prior code/security/test reports, the applicable
  `DEV_STANDARD` framework lane, code-quality standard, operational-delta
  template and canonical `code-reviewer` skill.
- Inspected the executable delta `22a38ff..88dab67` (63 files, 3,768 insertions,
  211 deletions) and the final clean source directly at full SHA.
- Reviewer execution on `88dab67`: `git diff --check ac857945..88dab67` PASS;
  `cargo fmt --check` PASS; `cargo test -p chill-updates --all-targets` PASS
  (18 library, 2 helper and 1 installer tests; Linux-root target has 0 runnable
  tests on macOS); `cargo test -p chillcenter-desktop update_bridge::tests` PASS
  5/5; `node --test ui/connected/updates.test.js` PASS 4/4; and
  `node --test scripts/test-update-helper-hardening.cjs` PASS 1/1.
- Reused only as scoped supporting evidence: the coordinator reported a real
  disposable-systemd-VM pass for the production sandbox, one-shot
  `+ExecStartPre` activation permit, active-candidate rescue and idempotent
  rollback. That run preceded the final rebuilt binary, so exact-final VM/fault
  execution remains test/release evidence rather than a claim of this code
  review.
- No Pi, NAS, GitHub release, tag, deployment or network state was touched by
  this review.

## Closure of prior findings

| Finding | Result on `88dab67` | Executable basis |
| :--- | :--- | :--- |
| C001 busy install dropped | **CLOSED** | Install identity and automatic/manual origin are durably and atomically claimed in `StateStore::admit_install`; busy work becomes `DeferredBusy`, retries wait 15 minutes and all nonterminal attempts terminate at three. Policy changes cancel automatic work both during reconciliation and immediately before root acceptance. Preflight/cache errors now terminalize the queued intent instead of retrying forever (`state.rs:250-354`; `main.rs:640-719,860-1076,1817-1847`). |
| C002 interrupted `Running` wedge | **CLOSED** | `reconcile_store` handles every nonterminal operation from the root trust ledger, bounds repeated crashes, and routes post-acceptance states to local recovery. Direct `--retry-accepted` performs rescue plus reconciliation without requiring a separate timer tick. The running guard terminalizes ordinary early-return failures; filesystem recovery is atomic, idempotent and cleans bounded transaction temporaries (`main.rs:650-719,892-1036`; `install.rs:484-581`). |
| C003 failed check reported available | **CLOSED** | Only `Succeeded + available` maps to `CheckResult::Available`; failed/running states return an error. `NoUpdate` and successful install also remove the component's UI availability snapshot (`main.rs:403-417,520-550,808-851,1533-1561`). |
| C004 no exact Mac/theme retry | **CLOSED** | The desktop persists exact sequence/version/source commit/manifest digest/signed bytes and acceptance time before mutation. Retry verification is limited to those exact signed bytes at the original valid acceptance time; a changed same-sequence identity is rejected. Mac still requires a fresh native confirmation before each SDK install, and restart reconciliation also binds the installed Mac build to its embedded source commit (`update_bridge.rs:90-104,263-380,631-759,827-1025`; `build.rs:1-36`). |
| C005 transient native error mislabeled unsupported | **CLOSED** | The UI treats only the explicit old-server codes/404 as unsupported and retains prior status for transient errors (`ui/connected/updates.js:1-3,49`). |
| C006 unsupported host scheduler mutation | **CLOSED** | Scheduler admission is host-scoped: Linux supplies server/web/themes and desktop supplies Mac/themes, so foreign component ledgers are not consumed (`scheduler.rs:40-96`; Linux/desktop call sites reviewed). |

## Additional adversarial paths reviewed

- A crash between selection and durable admission cannot silently retarget an
  already claimed retry: the manifest digest and origin are stored in the same
  state transaction. A generic LAN intent intentionally means the latest
  root-verified release at its first privileged claim; all later retries remain
  bound to that digest.
- A crash before root acceptance is recoverable but bounded. A crash after root
  acceptance cannot be authorized by the service-owned operation status: it is
  failed closed and requires the local root recovery flag.
- Root anti-rollback acceptance occurs only after idle/quiescence and validated
  backup, directly before transaction publication. First acceptance rechecks the
  manifest time window; exact accepted recovery alone may use its durable
  acceptance time.
- Successful operation state is written before clearing the root pending record.
  A crash in that gap therefore retains the stronger root recovery evidence and
  cannot become an untracked automatic retry.
- The systemd `+` executable prefix is intentionally relied upon for the
  one-shot pre-start permit consumer; primary systemd semantics exempt that
  command from the unit's user/group and filesystem namespacing restrictions.
  The earlier reviewer concern about `/run` writability is withdrawn.

## Quality matrix

| Area | Status | Evidence |
| :--- | :--- | :--- |
| Acceptance behavior | **PASS_SCOPED** | C001–C006 now follow the approved bounded/exact recovery semantics. |
| Correctness and crash behavior | **PASS_SCOPED** | Durable intent identity/origin, three-attempt bound, root-ledger reconciliation, exact accepted retry and idempotent rollback inspected. |
| Architecture / trust boundary | **PASS_SCOPED** | Service state cannot authorize a root-accepted retry; Mac consent remains native. Security certification is separate. |
| Compatibility / migration | **PASS_SCOPED** | New persisted fields are backward-readable through serde defaults; this updater state is not a previously published v0.7 schema. |
| Maintainability | **PASS** | State transitions and recovery ownership are localized; stale availability and bounded temporary cleanup are explicit. |
| Tests observed here | **PASS_SCOPED** | Focused macOS suites are green; Linux-root tests are platform-gated and need exact-final external evidence. |

## Verdict and routing

**APPROVED_SCOPED for code at exact SHA
`88dab67c8dc4bcbc781d0cd772830f71aba341d6`.** The historical C001–C006 code
blockers are closed. Publication still requires current independent security and
test verdicts plus exact-final Linux VM/fault and immutable artifact evidence.
Any production-source change after this SHA invalidates this approval; later
documentation-only review/release records do not alter the reviewed executable
source. This report authorizes no Pi/NAS access, deployment, tag or GitHub
publication by itself.
