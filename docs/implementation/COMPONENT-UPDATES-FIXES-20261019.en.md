# v0.7 — corrections and recovery evidence

[Français](COMPONENT-UPDATES-FIXES-20261019.md) | **English**

Reference source: `6041f4dc8acb6e14ea59bcb4e8ce6cec5d9edcaa`.
This supplements the [initial checkpoint](COMPONENT-UPDATES-20261019.en.md),
without overwriting adverse reviews. **Unpublished candidate: native Mac
validation and final independent verdicts remain publication gates.**
The Pi, its configuration, deployed web client and NAS have not been contacted.

Delta `3913267`: the first actual Tauri launch exposed
[BUG0115](../debug/BUG-0115-native-updater-bootstrap-config.md), a mandatory SDK
public-key field missing before builder overrides. JSON now pins the same public
key and a real SDK deserialization regression was added; native retest is pending.

Independent `3913267` reviews: [code APPROVED_SCOPED](../reviews/component-updates-code-3913267.md),
[security APPROVED](../reviews/component-updates-security-3913267.md),
[tests CHANGES_REQUESTED](../reviews/component-updates-tests-3913267.md).
Only CU88-TEST-003 remains open: the genuine native journey. All eight HIGH
findings and four functional defects are closed in their respective reviews.
The native delta passes 8/8 focused tests. Candidate Mac bundle built and
verified (0.7.0/build9, 48 arm64 libraries, libmpv API2.5), not published.
[Preserved logs](../releases/evidence/v0.7-candidate/) distinguish baseline suites,
the native delta and VM evidence.

## Eight security findings

| Finding | Correction and regression evidence |
| --- | --- |
| Stale activation permit | One-use, age-limited permit bound to transaction, target, PID, boot and process start time; durably consumed before execution. Replay refused under actual systemd sandbox. |
| Partially published cache | Private temporary directory, verified bytes/hash and synced files before atomic publication; interrupted downloads and reuse of complete cache tested. |
| Bootstrap environment | Privileged Bash, fixed PATH, cleared startup variables, resolved/verified system Python in isolated mode and absolute system Minisign. |
| Unbounded backup | Bounded elapsed time/stdout/stderr, process-group termination and reap; noisy output and stuck descendants tested. Backup runs without residual privileges. |
| Preparation during outage | Extraction, validation and final modes prepared before maintenance. Busy admission neither consumes a root sequence nor changes the active slot. |
| Recovery/systemd mismatch | Privileged `+` pre-start, read-only component roots in the server process, root writes reserved for the helper. Tested with actual production units in a VM. |
| Unreadable slots | Directories 0755, data 0644, server executable 0755, links/special files refused. Actual service UID can read but cannot write root-owned slots. |
| Failure without restart | Bounded `ExecStopPost`, old slot restored before restart, maintenance flag cleared and idempotent recovery. TERM/KILL during backup and already-active candidates tested. |

The VM also exposed two real defects: the external stop deadline must outlive
systemd's 30-second deadline, and a `Type=simple` process can be started before
its HTTPS listener is ready. Supervised stopping now has a bounded fallback;
health readiness waits at most 15 seconds with fixed CA, resolution and exact
version. The helper has an explicit 150-second stop budget for a pessimistic
recovery of approximately 115 seconds. A candidate ignoring TERM was killed at
30 seconds and the old healthy service restarted in the real fault test.

`CAP_SETUID`, already in the bounding set, is retained across helper execution
to allow dropping to the service account. During both deliberately blocked
backups, real/effective/saved UID/GID matched the service, supplementary groups
were empty, and `CapPrm`, `CapEff`, `CapAmb` were zero. `NoNewPrivileges` remains on.

## Four functional corrections

- Busy work retains the same intent, digest and manual/automatic origin,
  waits 15 minutes between retries and caps attempts at three. Policy changes
  cancel pending automatic work.
- Crash reconciliation and a global helper lease prevent wedged Running work.
  Interrupted root acceptance requires explicit local recovery, not a LAN click.
- Cached checks report availability only after successful completion with an
  available result. Exact accepted recovery bytes survive subsequent discovery.
- Mac/theme recovery uses a durable private receipt bound to signed bytes,
  signature, version, source, sequence and original acceptance time. An expired
  exact retry is accepted only if originally valid; new expired discovery,
  substitution and future acceptance are refused. Mac installation still
  requires native confirmation every time.

Transient failures preserve known server status, unsupported APIs are distinguished
from outages, and scheduling consumes only components belonging to the current host.

## Executed evidence

- Apple Silicon macOS / Rust 1.99.1: serialized Rust suite **433 PASS, 0 failed,
  2 pre-existing NFR harnesses ignored**; desktop 35 tests, including 7 updates.
- JavaScript/web/documentation: **359 PASS**, none skipped.
- Workspace/all-targets Clippy `-D warnings`, Rust formatting, diff and static
  hardening contract: PASS.
- Local Chromium: real configuration UI with a synthetic bridge, preserved edit,
  CAS, 503/404 handling and usable catalogue during a held request: PASS.
  This does not prove AppKit.
- Disposable local Ubuntu 24.04.4 LTS ARM64 VM, kernel 6.8.0-117, systemd 255,
  Minisign 0.11: full fault matrix PASS; actual root ledger tests **2/2 PASS**,
  not silently substituted by macOS tests.
- A real held TLS HTTP mutation produced Busy. The same manual intent was then
  resumed by `tick`, completed on its second attempt, installed a signed web
  package and exposed the new version through the real server. Only the fixture
  timestamp was advanced to avoid waiting 15 minutes.
- Synthetic manifests/signatures only: valid Mac/web/theme cases, tamper,
  expiry, incompatibility, hostile ZIPs, bounds, corrupt state and bounded history.
  No production private key was used for tests.

Main Linux command, inside the hostname-guarded disposable VM:

```sh
sudo env CHILL_UPDATE_DISPOSABLE_VM=1 python3 -u scripts/test-updates-linux-root.py \
  --updater /absolute/path/chill-update --server /absolute/path/chill-server \
  --resume-fixture --reset-fixture-ledger
```

The harness refuses ordinary hosts. The reset option archives only the synthetic
ledger of this VM. Never run it on a real installation. Final units also pass
`systemd-analyze verify`.

## Limits and publication

These tests do not prove physical Safari, power loss, disk-full handling or every
GitHub transport failure. No Pi/NAS execution is claimed. AppKit confirmation,
installation and restart still require observation in an isolated QA bundle;
missing evidence is not an implicit pass.

Sources and previous deliveries are preserved. Packages must be built from a
clean source, verified and signed before independent component releases and the
v0.7 umbrella release are published. Rollback preserves prior slots and backups;
anti-rollback protections must never be disabled.
