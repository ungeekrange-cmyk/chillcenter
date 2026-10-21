# BUG0128 — independent test execution review / revue indépendante des tests

- **Review mode / lane:** `EXECUTION_REVIEW` / `DEV_STANDARD`
- **Branch / exact source:** `codex/bug0128-durable-operator` /
  `a6f0fc0c0c98ee1d4a4ecfb2236177fa157a2415`
- **Base:** `0122c22d5b00030fc0cdcd08763b133985371be0`
- **Author and test author:** `/root`
- **Independent reviewer:** `/root/bug0128_test_review`
- **Environment:** disposable local Ubuntu ARM64 VM `colima-v07-security`, Linux
  `6.8.0-117-generic`, Python `3.12.3`, util-linux `flock 2.39.3`; host
  Python `3.9.6`, Lima `2.2.1`, loopback SSH only
- **Verdict:** **APPROVED**

The bounded local BUG0128 test gate is approved for the exact source above.
The 20 Linux tests and the real loopback SSH-disconnect proof passed without
skip, retry, quarantine or flakiness. The five reviewed source/test files in the
worktree had the same Git blob hashes as the frozen candidate before execution;
the unrelated untracked `docs/storage/` directory was not touched or used.

FR : le gate de tests local et borné est approuvé pour ce SHA exact. Cette
approbation ne vaut ni essai sur Pi/NAS, ni validation systemd complète, ni
déploiement, publication ou modification d'une version existante.

## Reproducible evidence / Preuves reproductibles

| Exact command | Exit | Duration | Passed / failed / skipped | Retries | Evidence |
| --- | ---: | ---: | --- | ---: | --- |
| `LIMA_HOME=/private/tmp/chillcenter-v07-linux.yvKFaz/lima limactl shell colima-v07-security -- sudo python3 -B /private/tmp/chillcenter-fixes-web-parity/scripts/test-pi-v07-legacy-operator.py` | 0 | 1.831 s test body | 10 / 0 / 0 | 0 | All named unittest cases `ok`; legacy deploy/recovery, pair preservation, frozen-clock collision and hostile ZIP/compatibility checks |
| `LIMA_HOME=/private/tmp/chillcenter-v07-linux.yvKFaz/lima limactl shell colima-v07-security -- sudo python3 -B /private/tmp/chillcenter-fixes-web-parity/scripts/test-pi-operator-job.py` | 0 | 6.354 s test body | 10 / 0 / 0 | 0 | All named unittest cases `ok`; real Linux locks, signals, UID switch, durable state and permissions |
| `python3 scripts/test-pi-operator-ssh.py --ssh-config /private/tmp/chillcenter-v07-linux.yvKFaz/lima/colima-v07-security/ssh.config` | 0 | 0.457 s command runtime | 1 journey / 0 / 0 | 0 | `real_ssh_disconnect=PASS`, concurrent recovery `REFUSED`, terminal exit `0`, job `03ab3732-997e-4bd3-9cf8-04a847db9c36` |
| `git diff --check 0122c22d5b00030fc0cdcd08763b133985371be0..a6f0fc0c0c98ee1d4a4ecfb2236177fa157a2415` plus Bash/Python AST syntax checks | 0 | <0.1 s | PASS | 0 | Clean delta and parseable reviewed scripts/tests |

## Acceptance and risk assessment / Matrice risques-preuves

| ID | Evidence and oracle | Result / assertion quality |
| --- | --- | --- |
| `128-01` | `test_live_owner_blocks_recovery_before_any_service_action` and every process-level busy check require exit `42`, unchanged service log and retained journal. | **PASS** — proves refusal before service action, not merely an error message. |
| `128-02` | SIGKILL of the exact operator PID leaves the real kernel lock busy until the orphan backup completes; the real `runuser -u nobody` child performs `fstat(9)`/`flock(9)` before the parent is killed. | **PASS** — meaningful FD9 inheritance and different-UID boundary. |
| `128-03` | The backup wrapper is identified through `/proc`, killed while its child has closed stdout/stderr, and the oracle requires service state `stopped`, no `start fixture`, busy refusal, then authorized recovery. TERM separately requires exit `143`, exact old payload and active service only after the backup releases. | **PASS** — exercises the independent backup lease and EXIT recovery overlap. |
| `128-04` | Killing the supervisor requires `SUPERVISOR_LOST` with JSON `exit_code:null`, even after the operator later installs the new pair. Fabricated stale boot/start identity and unobserved queue states must become `SUPERVISOR_LOST` / `START_NOT_OBSERVED`, never success. | **PASS** — PID reuse and lost-observer oracles are explicit. |
| `128-05` | Host harness proves SSH resolves to `127.0.0.1`, has no proxy/jump or agent forwarding, kills the local SSH process with SIGKILL, reconnects, observes `RUNNING` plus busy ownership, rejects recovery with `42`, then receives durable success/exit `0`. Guest hostname/root guard limits the target to the disposable VM. | **PASS** — real transport loss and reattachment, not a mocked disconnect. |
| `128-06` | SIGKILL at the real `mv` boundary plus recovery asserts exact old server bytes and old web build; legacy fixtures cover prepared, pre-rename, web-swapped and binary-swapped durable states. | **PASS** — post-state bytes/build are asserted, not transcript-only. |
| `128-07` | Wrong operator hash, UUID replay, symlink/hardlink/public lock files and public launcher are refused before service activity; job/state permissions are asserted `0700`/`0600`. Exact-source inspection confirms only `CC_*` request values are retained and the detached launcher/operator environment is rebuilt from fixed `PATH`/`LANG` plus that allowlist. | **PASS** — dynamic tamper/permission assertions plus direct static proof of the small environment allowlist. |
| `128-08` | Normal deployment asserts exact new server/web pair, service generation change, preserved config/library/token, valid backup path/manifest flow, closed CLI modes, hostile ZIP rejection and bounded semver compatibility. | **PASS** within the declared fixture layer — service, TLS and payload operations are controlled fixtures, not full systemd/household integration. |
| `128-09` | Frozen `date` forces all attempts into one clock tick while multiple deploy/recovery paths run; success requires exact prior-pair restoration. The candidate uses transaction UUIDs for retained/replaced/quarantined names. | **PASS** — this is the regression that would alias the former timestamp paths and fail restoration. |

## Assertion, isolation and determinism assessment

- The decisive oracles inspect hashes/bytes, web build, service action log,
  transition journal, lock occupancy, durable JSON state, actual signal exit
  codes, process identity and filesystem modes. Success strings alone cannot
  make these tests pass.
- Temporary roots are owned by each fixture and cleaned after their lock owner
  releases. Teardown refuses to erase a fixture while authority is still held.
  No broad `pgrep`, substring-selected kill or lock-file deletion is used.
- The process suite uses real Linux `flock`, `setsid`, signals, `/proc` identity
  and util-linux `runuser`; only service control, TLS, backup payload and package
  commands are fixtures. The operator fixture changes only the fixed PATH,
  root predicate and `/usr/lib/chillcenter` fixture path.
- Recovery is covered from several durable pre/post-swap states and repeated
  rapid attempts. A kill of the recovery process itself after
  `rollback_complete`, and a physical power cut/fsync-loss experiment, were not
  fault-injected; those remain outside this local matrix.
- No percentage coverage was supplied or used as an approval criterion.

## Limits / NOT_RUN

- No Pi, NAS, household host, real service configuration, real TLS endpoint,
  signed published payload, deployment, package rebuild, GitHub operation or
  published-version mutation was performed.
- Full real-systemd integration, physical power failure, filesystem corruption,
  disk-full/fsync failure and independent post-deployment health/data checks are
  **NOT_RUN**, not passes.
- The clean-environment property has direct exact-source evidence but no
  dedicated hostile `BASH_ENV`/`LD_PRELOAD` runtime injection case. This does
  not block the bounded verdict because the runtime environment construction is
  explicit and the incident-critical lock/transport boundaries are exercised;
  it must not be cited as deployed-host evidence.

## Findings and routing / Constats

No blocking test finding was identified for the approved local-only scope.
Any source change makes this verdict stale. A future real target operation still
requires exact reviewed bytes, authorization, independent health/data checks
and the existing no-Pi/no-NAS/no-published-version boundary to be lifted
explicitly.
