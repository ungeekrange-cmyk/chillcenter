# Independent code delta review — component updates `6041f4d`

- **Review type / lane:** code correctness, `FULL_ASSURANCE` operational delta review
- **Branch / exact source:** `codex/release-v0.7-fixes` / `6041f4dc8acb6e14ea59bcb4e8ce6cec5d9edcaa`
- **Parent reviewed SHA / verdict:** `88dab67c8dc4bcbc781d0cd772830f71aba341d6` / `APPROVED_SCOPED` in `component-updates-code-88dab67.md`
- **Independent reviewer:** `/root/v07_final_code` (not an author; source-non-mutating)
- **Date:** 2026-10-19
- **Verdict:** **APPROVED_SCOPED**

No P0, P1 or P2 correctness, architecture, compatibility or maintainability
finding remains in the `88dab67..6041f4d` source delta. The prior C001–C006
closures remain valid. This code verdict does not replace the pairwise-distinct
test and security verdicts required for the same exact source before publication.

## Delta and review result

The executable delta is 17 paths (1,127 insertions, 89 deletions) and adds no
runtime dependency or runtime-version change.

| Area | Result | Exact-source basis |
| :--- | :--- | :--- |
| Mac accepted-release recovery | **PASS** | Production still calls only the embedded key wrapper. The private injected-key seam exercises the same verifier with an ephemeral test key. Exact manifest digest, sequence, version and source commit remain bound; future acceptance is rejected and each install still requires native confirmation (`apps/desktop/src/update_bridge.rs:631-784`). |
| Linux cache interruption | **PASS** | The extraction to `publish_cached_release` preserves the production fixed GitHub downloader, verifies length/digest before rename, fsyncs the stage and parent, and never exposes a partial content-addressed directory (`crates/chill-updates/src/main.rs:425-575`). |
| Linux stop/recovery | **PASS** | Recovery stops an active uncommitted candidate before rollback, uses a 45-second supervisor beyond the service's 30-second graceful stop, falls back to control-group KILL, and confirms inactive/failed with zero main/control PIDs before removing the slot (`crates/chill-updates/src/main.rs:1205-1286`; `deploy/updates/chillcenter.service:18-24`). |
| Recovery unit budget | **PASS** | `TimeoutStopSec=150s` exceeds the bounded degraded rescue path (approximately 115 seconds including restart), so PID 1 no longer truncates the helper's own bounds. The exact-source static contract asserts the setting and read-only component roots (`deploy/updates/chillcenter-update.service:14-31`; `scripts/test-update-helper-hardening.cjs:13-16`). |
| Service privilege/filesystem boundary | **PASS_SCOPED** | Main service write access is reduced to `/var/lib/chillcenter`; fixed `+ExecStartPre` recovery remains the only privileged bootstrap into root-owned component slots. The updater's bounded `CAP_SETUID` is used only to drop the offline backup child to the configured service UID/GID; no client-supplied command/path was added. Security certification remains separate. |
| Startup health | **PASS** | The fixed CA-pinned endpoint retries under one 15-second global deadline, each request is capped by the remaining budget, redirects remain disabled, response size is bounded and reported version must match the candidate slot (`crates/chill-updates/src/main.rs:2018-2078`). |
| Hostile inputs and state | **PASS_SCOPED** | Public-only signed fixtures cover Mac/web/themes, ZIP special/duplicate/size/ratio cases, exact raw bytes/hash, corrupt state and bounded history. Test-vector generation deletes its temporary private key; no private key fixture is committed (`crates/chill-updates/tests/hostile_inputs.rs:20-245`; `scripts/generate-update-test-vectors.mjs`). |

The terminal commits after the behavior change are narrow and preserve the
reviewed semantics: `91addb3` removes a needless borrow for Rust 1.99 Clippy;
`8d8a7f2`/`e85f5bb` name and format a test tuple; `fd382a4` raises only the
updater stop budget; `6041f4d` updates only the corresponding static contract.

## Evidence

Reviewer-run against exact `6041f4d`:

- `git diff --check 88dab67..6041f4d` — PASS.
- `cargo fmt --check` — PASS.
- `cargo test -p chill-updates --locked --offline --all-targets` — PASS: 18
  library, 3 helper, 4 hostile-input and 1 installer tests; Linux-root tests are
  platform-gated and correctly ran zero on macOS.
- `cargo test -p chillcenter-desktop --locked --offline update_bridge::tests
  -- --test-threads=1` — PASS 7/7, including real ephemeral Minisign
  accepted-expiry/replacement cases.
- `node --test scripts/test-update-helper-hardening.cjs` — PASS.

Reused evidence, with delta validity checked explicitly:

- `/private/tmp/chillcenter-v07-linux.yvKFaz/fault-test-4.log` — disposable
  Ubuntu ARM64 PASS for real HTTP Busy→Deferred→same-intent idle completion,
  automatic-policy cancellation, retry cap, corrupt preflight terminalization,
  service-UID backup with cleared groups/capabilities, TERM/KILL descendant
  cleanup, active-candidate `ExecStopPost` rollback, exact expired accepted-time
  retry through backup/activation/health mismatch/rollback, and monotonic-floor
  preservation. Later executable deltas are formatting/lint only; the later unit
  delta increases, rather than decreases, the rescue budget.
- `/private/tmp/chillcenter-v07-linux.yvKFaz/root-ledger-tests.log` — Linux root
  ledger PASS 2/2.
- `/private/tmp/chillcenter-v07-clippy-final-3.log` — workspace all-targets
  `-D warnings` PASS. Later deltas contain no Rust source after formatting.
- `/private/tmp/chillcenter-v07-workspace-final-2.log` — 433 Rust tests PASS,
  zero failures and two established NFR harnesses ignored; the Linux root module
  ran zero on macOS and is covered separately by the 2/2 Linux log above.
- `/private/tmp/chillcenter-v07-node-all-final.log` — 359 Node tests PASS with
  zero failures. The final static-helper expectation was rerun directly above.

## Residual proof limits

The Linux VM is not Pi/NAS or production evidence, and no production host was
in scope. The exact final `TimeoutStopSec=150s` unit delta was checked statically;
the earlier VM run exercised the same rescue implementation with the shorter
unit default. No physical native Mac cancel/accept/install/restart journey,
power-cut/disk-full fault, or live GitHub publication/transport failure is
claimed here. Those are test/release-proof limits, not observed source defects.

## Verdict and routing

**APPROVED_SCOPED for code at exact SHA
`6041f4dc8acb6e14ea59bcb4e8ce6cec5d9edcaa`.** No code correction is requested.
Publication remains gated on current exact-SHA test and security approvals and
immutable package/release evidence. Any later production-source or unit change
makes this verdict stale; a report-only carrier commit does not.
