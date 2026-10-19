# Independent security review — component updates `22a38ff`

- **Reviewed source:** `22a38ff50879837c32fc8d5ac7c7200f35c8eaa1`
- **Baseline:** `ac85794541a59b011be11ebbe839c3e30a293ea7`
- **Integrated engine ancestor:** `1001c0d166c44375e3241661c73734c39495a23f`
- **Authors/integrator:** `/root/component_update_engine`, `/root`
- **Independent reviewer:** `/root/component_updates_security` (read-only except for this report)
- **Reviewed at:** `2026-10-19T20:40:27Z`
- **Verdict:** **CHANGES_REQUESTED — publication, deployment and release approval blocked**

This was a static review of the exact clean source SHA. No private key was read;
no source, lockfile, network service, Pi, NAS, release, deployment or cloud state
was changed. No Linux root integration or crash-fault evidence was provided or
executed. Existing test comments and author-reported results are not treated as
evidence for the privileged Linux path.

## Controls confirmed in the frozen source

- The Mac path downloads exact signed bytes, presents a native dialog immediately
  before installation, re-verifies the cached signed manifest, durably advances
  local acceptance before the SDK install, and feeds the SDK the signed version,
  URL and Tauri signature (`apps/desktop/src/update_bridge.rs:395-510`). Renderer
  IPC alone cannot bypass that native confirmation.
- Theme packages are data-only and fail closed to the embedded pack: strict JSON,
  exact six IDs, bounded calendar/priority, known palette tokens, safe local PNG
  paths, exact declarations, SHA/size checks, full bounded PNG decode and no
  undeclared active files (`crates/chill-updates/src/theme.rs:42-214,268-353`;
  `apps/desktop/src/theme_bridge.rs:90-129,258-348`).
- Server update requests accept only closed component/policy DTOs and no caller
  URL, command, path, key, tag or force field (`crates/chill-server/src/updates_api.rs:78-160`).
  Same-origin middleware remains the browser boundary; unauthenticated household
  LAN administration is the explicitly accepted ADR-0029 residual risk, not a
  newly introduced tenant or identity boundary.
- Maintenance admission reserves mutating HTTP/background work before checking
  the root-owned flag and publishes a fresh generation-bound idle pulse
  (`crates/chill-server/src/api.rs:244-265`;
  `crates/chill-server/src/updates_api.rs:18-65,162-170,222-243`).
- Root anti-rollback acceptance, fixed-repository signed manifests, redirect-free
  bounded health, strict archive traversal checks and exact backup-receipt
  validation are present. These controls remain useful but do not close the
  findings below.

## Open blocking findings

### `CU-SEC-001` — HIGH — stale activation authorization can run an uncommitted candidate

- **Precondition/abuse path:** the privileged updater crashes or is killed after
  writing the runtime authorization, or after `systemctl start`, but before the
  authorization is removed. A later service start in the same boot sees matching
  bytes and treats the still-unhealthy transaction as authorized.
- **Evidence:** `recover_interrupted` accepts only byte equality, with no age,
  updater PID/start-time, boot identity or one-shot consumption
  (`crates/chill-updates/src/install.rs:300-319`). `activate` creates the file
  before `systemctl` and removes it only after the command returns
  (`crates/chill-updates/src/main.rs:1365-1388`). All three recover calls run at
  service start (`deploy/updates/chillcenter.service:12-14`).
- **Asset/impact:** executable integrity and service availability; an accepted
  but uncommitted/unhealthy slot may run, and its marker/release can remain wedged.
- **Required remediation/retest:** bind authorization to the live updater
  invocation and atomically consume it for exactly the expected start. Any stale
  authorization must trigger rollback. Fault-test kill points before write,
  after write, after slot switch, after start and before removal.

### `CU-SEC-002` — HIGH — interrupted cache publication permanently blocks the release

- **Precondition/abuse path:** process/OS interruption or disk error between
  creation of the final digest directory and completion of its three files, or
  before the `available.json` rename.
- **Evidence:** the final digest directory is populated in place
  (`crates/chill-updates/src/main.rs:398-422`). A retry preserves that digest as
  `keep` and only verifies its incomplete payload; cleanup rejects the updater's
  own `.available-*.tmp` name (`main.rs:842-872,1081-1089`).
- **Asset/impact:** update availability; the same valid signed release fails
  forever until manual root cleanup.
- **Required remediation/retest:** populate and verify a bounded temporary
  directory, fsync it, then atomically rename it to the digest. Recognize and
  safely remove only the updater's exact bounded temporary patterns. Inject a
  crash after each cache write and before each rename, then prove automatic retry.

### `CU-SEC-003` — HIGH — bootstrap verification inherits attacker-controlled root environment

- **Precondition/abuse path:** an administrator invokes the root installer from
  an environment/cwd with attacker-controlled `PATH`, `PYTHONPATH`, user-site or
  Python startup modules.
- **Evidence:** `dirname` is resolved through `PATH` before tool checks
  (`scripts/install-server.sh:8`); `python3`, `minisign` and other tools are also
  PATH-selected, and Python runs without isolated mode or environment cleanup
  before and during external signature verification (`install-server.sh:55-59,
  89-92,140-153`).
- **Asset/impact:** root code execution and bootstrap trust; hostile environment
  code can run before the downloaded helper is externally verified.
- **Required remediation/retest:** establish a fixed safe `PATH` and unset Python
  injection variables at line 1, use absolute root-owned tools (or verify their
  ownership), and invoke `/usr/bin/python3 -I`. Test hostile PATH, PYTHONPATH,
  user-site and cwd fixtures and prove none executes.

### `CU-SEC-004` — HIGH — offline backup has no internal time/output bound

- **Precondition/abuse path:** the previously installed signed server backup
  command hangs or emits unbounded stdout/stderr because of a defect, corrupt
  state or resource exhaustion.
- **Evidence:** backup uses `Command::output()` with no timeout and buffers both
  pipes without limit; stdout is checked only after child exit
  (`crates/chill-updates/src/main.rs:1311-1345`). The server is already stopped.
- **Asset/impact:** service availability and root-helper memory; the final update
  window can hang or exhaust memory while playback is unavailable.
- **Required remediation/retest:** supervise the child with a bounded deadline,
  bounded stdout, discarded/bounded stderr and reliable process cleanup; on any
  failure clear maintenance and restore/start the prior slot. Test hung and
  verbose backup children.

### `CU-SEC-005` — HIGH — archive staging occurs after quiescence and service stop

- **Precondition/abuse path:** a valid large archive or slow/erroring storage
  makes bounded extraction/PNG validation lengthy.
- **Evidence:** `apply_staged` begins quiescence first
  (`crates/chill-updates/src/install.rs:68-89`); only then does it extract, fsync
  and validate the archive (`install.rs:97-135`). Linux quiescence stops the
  service before this work (`crates/chill-updates/src/main.rs:1271-1309`).
- **Asset/impact:** service availability; staging blocks playback contrary to
  `docs/updates/CONTRACT.md:35-37` and makes the final window unnecessarily large.
- **Required remediation/retest:** extract, validate and fsync into a trusted
  immutable stage before maintenance, then re-verify/pin that stage during the
  short backup/switch window. Measure maximum outage with the largest accepted
  archive and slow/error-injected storage.

### `CU-SEC-006` — HIGH — normal service recovery cannot write the rollback roots

- **Precondition/abuse path:** an interrupted transaction exists and systemd
  attempts the normal server restart.
- **Evidence:** the server unit invokes privileged recovery
  (`deploy/updates/chillcenter.service:12-14`) but `ProtectSystem=strict` and its
  `ReadWritePaths` exclude `/usr/lib/chillcenter/{server,web,themes}`
  (`chillcenter.service:23-24`). The `+` prefix changes credentials/capabilities;
  it does not bypass unit mount-namespace read-only restrictions.
- **Asset/impact:** recovery availability; rollback rename/removal fails and the
  service restart loops until a distinct writable update unit later runs.
- **Required remediation/retest:** use a dedicated privileged recovery unit with
  narrowly writable component roots and explicit ordering, or otherwise grant
  only the recovery process the required paths. Prove rollback from each marker
  state under the actual installed systemd sandbox.

### `CU-SEC-007` — HIGH — Linux release trees are unreadable by the service account

- **Precondition/abuse path:** any server, web or theme update is extracted by
  the root update unit.
- **Evidence:** the unit uses root with `UMask=0077`
  (`deploy/updates/chillcenter-update.service:7-10`). Archive directory/file
  creation does not set service-readable ownership or modes
  (`crates/chill-updates/src/archive.rs:35-79,249-275`). Only the server payload
  is chmod `0755`; its parent slot remains root `0700`, and web/theme files remain
  root `0600` (`crates/chill-updates/src/install.rs:119-134`). The runtime service
  is `User=chillcenter` (`deploy/updates/chillcenter.service:7-15`).
- **Asset/impact:** deterministic Linux update failure; the service cannot
  traverse or read the activated slot, so start/health fails and rolls back.
- **Required remediation/retest:** finalize every staged tree with exact
  root ownership and intended service group/modes (for example directories
  `0750`, data `0640`, executable `0750`), reject special links, verify the whole
  tree and fsync before switch. Test reads/exec as the real service UID/GID.

### `CU-SEC-008` — HIGH — updater failure after stop has no immediate recovery orchestration

- **Precondition/abuse path:** the update oneshot is killed, crashes or reaches
  systemd's start timeout after stopping `chillcenter.service`.
- **Evidence:** `chillcenter-update.service` runs the full network check/download/
  apply in one oneshot and has no explicit `TimeoutStartSec`, `ExecStopPost` or
  failure-recovery unit (`deploy/updates/chillcenter-update.service:1-33`). Once
  the server is intentionally stopped, its own pre-start recovery cannot run.
- **Asset/impact:** prolonged service outage with stale maintenance/authorization
  state; recovery may wait for manual action or the next daily timer.
- **Required remediation/retest:** define bounded phase timeouts and guaranteed
  failure cleanup/recovery/server restart through safe unit orchestration. Under
  the installed sandbox, send TERM/KILL and simulate timeout/power interruption
  at every stop/marker/switch/authorization point; the old healthy service must
  return without waiting for the next timer.

## Evidence and limitations

| Evidence | Result | Limitation |
| :--- | :--- | :--- |
| `git rev-parse HEAD` and `git status --short` | exact `22a38ff50879837c32fc8d5ac7c7200f35c8eaa1`, clean before this report | Source identity only |
| `git diff --check ac85794..22a38ff` | PASS | Whitespace only |
| `cargo fmt --all -- --check` | PASS | Formatting only; no build or behavior proof |
| Targeted line-by-line static review of updater, Mac bridge, theme bridge, server API/static serving, installers and systemd units | Eight open HIGH findings; controls above present | No compiler, unit, integration, crash, root/systemd or device execution |

No exception or accepted-risk ADR covers these eight implementation defects.
ADR-0029's accepted household-LAN/no-auth residual risk does not accept update
integrity or prolonged-outage failures. A new exact SHA plus Linux root/systemd
integration and fault-injection evidence requires a fresh independent review.
