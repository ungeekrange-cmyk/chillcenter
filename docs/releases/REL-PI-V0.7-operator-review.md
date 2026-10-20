# REL-PI-V0.7 — independent operator security review

## Verdict

**`CHANGES_REQUESTED`** for source commit
`d589725e9b1ba0fc824352091493840218d7759e` (operator SHA-256
`97f0cf28922ff8321e87954d2af8a2700b845a1f659ac17aa6158db455f3525a`).
Finding `SEC-PI07-001` is `HIGH` and blocks use of this operator for the
authorized Pi deployment. This review is independent of the operator author.

The lane is `FULL_ASSURANCE`: the intended target is the existing production
Pi. No SSH connection, Pi/NAS read or write, staging, service operation, or
private-key access was performed by this review.

## Findings

### SEC-PI07-001 — `HIGH` — interrupted replacement can leave a mixed pair and report only an attempted recovery

**Scenario.** At
`scripts/pi-v07-legacy-server-web-operator.sh:89`, the operator moves the new
web tree into service and only then sets `web_swapped=1`; it likewise moves the
new binary before setting `bin_swapped=1`. A catchable signal in either window
enters `recover` with a false flag. After the web window, recovery leaves the
new web in place and its `mv -T old_web current_web` cannot restore over that
non-empty destination. After the binary window, recovery skips restoring the
old binary. `recover` also ignores every stop/move/install/start failure and
does not verify the restored binary hash, web build, or service response
(`:79-80`). A `SIGKILL` or power loss between the two moves has the same mixed
pair outcome because there is no durable transaction marker.

**Impact.** An ordinary interruption or recovery error can restart an
old-server/new-web or new-server/old-web combination. The script therefore
does not meet its grouped replacement and rollback claims and can leave the
household service unavailable or semantically inconsistent.

**Required change.** Make the two-component transition recoverable across
every instruction and restart: record durable intended/current state before
the first rename, make recovery idempotent from filesystem reality rather than
post-move shell flags, fail loudly on restoration errors, and verify both
restored hashes/build plus service health before declaring rollback complete.
Add deterministic signal/fault tests at each transition boundary, including
process death between the two replacements.

### SEC-PI07-002 — `MEDIUM` — the archive safety predicate accepts traversal and symlink entries

**Evidence.** The exact `safe_zip` logic at line 14 accepted both disposable
fixtures below:

- entries `../escape.txt`, then `safe.txt` -> `operator_safe_zip=ACCEPTED`;
- a Unix-mode `120777` symlink entry, then `safe.txt` ->
  `operator_safe_zip=ACCEPTED`.

The pipeline returns the status of the final loop iteration, so an earlier bad
name is forgotten when the last entry is safe. The symlink expression expects
`": 120777"`, while this platform's `unzip -Z -v` emits
`"Unix file attributes (120777 octal):"`. Extraction and recursive ownership
changes subsequently run as root (`:76-77`).

The exact pinned v0.7 web ZIP independently hashes to
`0fa1aabf20d9f84b3e2722b0af7017dd09de9503da4524140a19f593072d7b57` and its
108 entries contain neither a traversal/absolute name nor a symlink. The
signature and exact-hash boundary therefore limits this release's immediate
exposure, but the claimed root-side safety control is ineffective.

**Required change.** Make any unsafe member fail the whole check; parse archive
entry types without output-format-dependent matching; reject links and special
files; and retain regression fixtures where the unsafe entry is not last.

### SEC-PI07-003 — `MEDIUM` — the idle decision is incomplete and becomes stale before service stop

The operator checks scan state, persisted TMDB-job state, and established
sockets at lines 53-64, then performs database probing, fingerprinting,
artifact staging, extraction, chmod, and chown before stopping the service at
line 85. It neither closes admission nor repeats a complete busy check
immediately before the stop. A playback/client connection or new background
operation can therefore begin after the gate; established sockets alone do
not prove the absence of paused/buffered playback or every in-memory job.

**Required change.** Establish a bounded no-new-work/maintenance gate, or use a
complete application busy contract, and perform the final check immediately
before stop. Refuse the deployment if complete activity state cannot be
established.

### SEC-PI07-004 — `MEDIUM` — backup execution is unbounded while the service is stopped

At line 86 the backup runs synchronously inside command substitution with no
timeout or child-process-group cleanup. A signal trap may be deferred while
the foreground command is still running. A blocked SQLite/storage operation
can therefore leave the service stopped indefinitely without reaching
`recover`.

**Required change.** Bound backup duration, terminate and reap the whole backup
process group on timeout/signal, and prove that the recovery path restarts the
unchanged service after this failure.

### SEC-PI07-005 — `MEDIUM` — the documented normal invocation has no valid mode contract

With `set -u`, line 31 dereferences `$1` unconditionally. Invoking the operator
with the documented environment and no argument exits before preflight with
`line 31: $1: unbound variable` (exit 1). Any arbitrary argument currently
selects the mutating path, while the handoff documents no deployment argument.

**Required change.** Parse an explicit, closed mode set such as
`--self-test`, `--preflight`, and `--deploy`; reject missing, unknown, or extra
arguments; and document the exact invocation.

## Controls and limitations confirmed

- The frozen commit changes only the operator and its handoff. `bash -n` and
  `--self-test` pass; `shellcheck` was unavailable.
- The candidate contains no household IP literal or private-key material. It
  accepts only a public release key, verifies its expected SHA-256, checks both
  Minisign manifests, and verifies exact component hashes before extraction.
- Candidate paths are required to be absolute and existing inputs are required
  to be regular non-symlink files; rollback/data/web roots must be non-symlink
  directories. No initial installer, helper/timer, unit, account, config, TLS,
  mount, NAS, or updater-policy mutation appears in the reviewed diff.
- The post-stop backup is run under the service identity. `backup_ok` requires
  a private non-symlink directory, schema 3, an exact regular-file set,
  config/TLS entries, matching size/hash metadata, and no stray WAL/SHM. The
  server backup implementation uses its coherent snapshot path, so this is the
  authoritative SQLite/WAL consistency control.
- The active-database probe uses SQLite `immutable=1` and consequently does not
  establish the state of an active WAL. This is not an additional blocker
  because the validated post-stop coherent backup is authoritative, but the
  probe must not be represented as a current-WAL integrity proof.
- Settings preservation and successful live behavior remain **unverified**.
  The operator records a pre-state fingerprint but does not compare it before
  `deployment=PASS`. The handoff correctly keeps SSH preflight, target/SHA
  authorization, mutation, and independent post-deployment verification at
  `NOT_RUN`/`PENDING`. A later verifier must compare config, library, TLS,
  unit identity and all inventoried persistent settings, without reading NAS
  content, before the release can be called complete.

## Bounded evidence

Commands were run from `/private/tmp/chillcenter-fixes-web-parity` against the
exact frozen commit:

```text
git rev-parse HEAD
# d589725e9b1ba0fc824352091493840218d7759e
shasum -a 256 scripts/pi-v07-legacy-server-web-operator.sh
# 97f0cf28922ff8321e87954d2af8a2700b845a1f659ac17aa6158db455f3525a
bash -n scripts/pi-v07-legacy-server-web-operator.sh
# PASS
bash scripts/pi-v07-legacy-server-web-operator.sh --self-test
# pi_v07_legacy_operator_self_test=PASS
bash scripts/pi-v07-legacy-server-web-operator.sh
# exit 1: line 31: $1: unbound variable
```

Review limit: no live-target assertion is made. Re-review is required for the
exact corrected source commit before any SSH staging or Pi mutation.

---

## Re-review — `24f62ac2f4da95fd51bf01f81ab934b35762f888`

### Current verdict

**`APPROVED`** for exact source commit
`24f62ac2f4da95fd51bf01f81ab934b35762f888`, operator SHA-256
`37648528be5ecf6c20de39d80e24a79fd6b765cc784084e2b44084e3eeb866fe`.
This verdict supersedes the historical `CHANGES_REQUESTED` verdict above only
for this exact corrected commit. Any source change requires another review.

No `HIGH` or `CRITICAL` finding remains in the reviewed scope.

### Finding closure

| Finding | Result | Exact closure evidence |
| --- | --- | --- |
| `SEC-PI07-001` interrupted/mixed replacement | `CLOSED` | A root-owned, fsynced receipt is written before stop and advanced to `stopping` before mutation. Recovery accepts the real durable states and reconciles filesystem reality without post-move flags. It handles old binary copied/current binary still old, web-only replacement, and binary-between-replacements, verifies the restored old hash/build, starts the service, waits for strict-TLS health, and removes the receipt only after success. Focused failure tests cover these boundaries and an operator-created backup-failure recovery path. |
| `SEC-PI07-002` unsafe ZIP predicate | `CLOSED` | Python `zipfile` validation rejects empty/absolute/traversal names and every non-regular/non-directory Unix type. Regression fixtures place traversal, absolute, symlink and device entries before a safe final member. The exact signed v0.7 ZIP was independently shown safe in the historical review. |
| `SEC-PI07-003` stale/incomplete idle gate | `CLOSED` | Immediately before recording stop intent, the operator rechecks scan state, the bounded regular TMDB-jobs file, and established server sockets; socket inspection now fails closed. A busy result occurs before traps or service mutation. The full-flow fixture proves the second busy check refuses with exit 42 and no `systemctl stop`. The legacy limitation is covered by the already completed read-only live preflight and required human-idle execution window; no helper, unit, timer or config migration was introduced. |
| `SEC-PI07-004` unbounded backup/signal handling | `CLOSED` | Backup is bounded by GNU `timeout` and runs as a dedicated `setsid runuser` session/process group. TERM cleanup targets that exact group, waits two seconds, sends KILL, and reaps it before operator recovery can restart the service. The coordinator's authorized stateless real-Linux proof used the exact supervisor shape with a TERM-ignoring child and grandchild: exit `124`, elapsed `4.63s`, both recorded descendants absent/non-running afterward. It made no Pi file, config, service or NAS change. |
| `SEC-PI07-005` undefined mode contract | `CLOSED` | Exactly one of `--self-test`, `--preflight`, `--deploy`, or `--recover` is required; missing, unknown and extra arguments fail closed. The handoff documents the modes. |

### Current security and operational assessment

- The Pi has no `minisign` binary and this scoped deployment does not install
  one. This does not weaken the immutable-input boundary for this run: the
  public key and both manifest/signature pairs are hash-pinned to the exact
  receipt whose Minisign signatures were independently verified off-Pi, and
  the server/web payload hashes are then checked against the parsed pinned
  manifests. No production private key is read, transferred or printed.
- `--recover` deliberately does not re-download or revalidate release
  manifests. It restores the old pair from the durable receipt produced only
  after the deployment prechecks. Approval assumes the authorized invocation
  reuses the exact existing root-owned `0700` rollback path and the bound
  `CC_EXPECTED_OLD_BINARY_SHA256`; an arbitrary or user-writable recovery path
  is outside this approval.
- The offline backup remains authoritative for SQLite/WAL consistency and is
  validated for its exact regular-file set, hashes, schema, config/TLS content,
  and absence of stray WAL/SHM. The active `immutable=1` probe is only a
  preflight signal, not the backup proof.
- The successful-flow test proves the intended binary/web pair changes while
  config, library and TMDB token remain unchanged. The operator's before/after
  persistent fingerprint also covers config, certificate, optional
  library/token, and systemd identity. Final live preservation of config,
  library, TLS, unit and all inventoried settings remains a mandatory distinct
  post-deployment verification gate; the script does not touch NAS content.
- No installer, updater helper/timer, unit, account/group, TLS/config, mount,
  NAS or update-policy change is introduced. No household IP, credential or
  private-key material appears in the changed source.

### Independent bounded checks

Executed locally against the frozen commit, without reviewer SSH or Pi/NAS
access:

```text
git rev-parse HEAD
# 24f62ac2f4da95fd51bf01f81ab934b35762f888
shasum -a 256 scripts/pi-v07-legacy-server-web-operator.sh
# 37648528be5ecf6c20de39d80e24a79fd6b765cc784084e2b44084e3eeb866fe
bash -n scripts/pi-v07-legacy-server-web-operator.sh
# PASS
bash scripts/pi-v07-legacy-server-web-operator.sh --self-test
# pi_v07_legacy_operator_self_test=PASS
python3 scripts/test-pi-v07-legacy-operator.py
# Ran 7 tests ... OK
git diff --check 121cef8..HEAD
# PASS
```

The authorized deployment may proceed only with the exact reviewed commit,
operator hash, target/path set, pinned receipt values and idle confirmation.
This source approval does not claim that staging, mutation or independent
post-deployment verification has already occurred.
