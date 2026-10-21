# Chill Center 0.7.2 — Pi execution record

## Status

The historical **`PENDING_REVIEW`** operator-correction status in an earlier
revision is superseded by the factual execution record below; it remains
historical evidence, not an approval of the interrupted attempt.

**Coordinator final state: `VERIFIED`.** The distinct
[release verifier](REL-0.7.2-verification.md) subsequently returned `APPROVED`
for the live pair, backup/rollback, protected invariants and real update checks.
The deployer's original handoff was `DEPLOYED_PENDING_INDEPENDENT_VERIFICATION`;
the execution evidence below is not itself independent approval.

## Execution attempt and recovery — 21 October 2026

- Reviewed operator SHA-256:
  `12a87083be5c18cdbabade89f8a248beb7952defe2f219d25138d0b4d4e56d0c`.
- Authorized target pair: server SHA-256
  `cdde3ed064b79258ac17de3734c6750dd844e0f150bfcad8652f9d7e9b4cadea`;
  web ZIP SHA-256
  `5b2131a3ac8a20ca3a6398f69ff36e3697810f06f065971558d52a97e89b8a17`,
  build `ad12a307191b`.
- The local artifact pins and Pi transfer checks passed. The reviewed operator
  preflight passed immediately before execution; its private receipt was
  captured at `2026-10-21T12:34:00Z`.
- During the bounded offline-backup window, the local SSH execution transport
  yielded without a terminal receipt or observable exit status. This is not
  interpreted as either success or failure. An intermediate observation found
  the durable Pi journal at `stopping` and the old pair still present; it did
  not prove recovery or final transaction outcome.
- While that original backup/transaction was still in flight, this deployer
  invoked the reviewed `--recover` path. That overlapping recovery attempt
  restarted the service while the owned backup still held the data lock, causing
  the temporary expected "service must be stopped" restart loop. No lock file
  was removed and no backup process was killed. The original transaction later
  continued to completion. Its SSH transport exit status is
  **`NOT_OBSERVED`**; missing stdout is not treated as a failure result.
- Final read-only state proved late transaction completion: service
  `active/running`, server hash
  `cdde3ed064b79258ac17de3734c6750dd844e0f150bfcad8652f9d7e9b4cadea`,
  web build `ad12a307191b`, dated strict-TLS API `0.7.2`, durable journal
  absent, and no executable-path backup process remaining.
- A fresh private backup receipt `d3340e19-29f9-43d4-a91e-71acc8750839`
  was validated for the manifest, exact regular-file tree and hashes, schema 3,
  SQLite `quick_check`, and absence of undeclared WAL/SHM. It was not restored.
- Private before/after comparison passed for configuration, TLS certificate and
  key, the AF_NETLINK drop-in, and the effective unit. NAS, settings,
  helper/timer, update policy, and TLS material were not changed.

French summary: la sauvegarde privée fraîche est valide et la bascule 0.7.2
est finalement active. La vérification indépendante post-déploiement reste
obligatoire; l'absence de reçu terminal SSH ne doit pas être interprétée seule.

English summary: the new private backup is valid and the 0.7.2 swap eventually
became active. Independent post-deployment verification remains required; the
missing terminal SSH receipt alone is not a final outcome.

## Outstanding post-gate

- A distinct verifier must approve the live 0.7.2 pair, fresh backup,
  protected invariants, API/web behavior and service stability before release
  completion is claimed.
- Future long-running transactions need a durable remote transcript/job-wait
  mechanism; do not infer an SSH transport callback from missing stdout.

## Bounded operator correction

The legacy combined server/web operator previously required the web manifest's
`compatibility.required_server.min` to equal the target server version. The
released component-manifest contract instead supplies an inclusive compatibility
range. That valid contract would reject a 0.7.2 pair where the web range is
`0.7.0` through `0.999.999`.

The operator now accepts the target only when all three versions are strict,
stable `MAJOR.MINOR.PATCH` values, both `min` and `max` exist, `min <= max`, and
`min <= target <= max`. Missing, malformed, inverted, or out-of-range bounds
fail before the payload archive is extracted or the service is stopped.

No backup, swap, recovery, TLS, configuration, data, NAS, account, systemd
unit/drop-in, helper, timer, or update-policy behavior changed.

## Local evidence

- Operator SHA-256 after this correction:
  `12a87083be5c18cdbabade89f8a248beb7952defe2f219d25138d0b4d4e56d0c`.
- `bash -n scripts/pi-v07-legacy-server-web-operator.sh` — PASS.
- `bash scripts/pi-v07-legacy-server-web-operator.sh --self-test` — PASS.
- `python3 scripts/test-pi-v07-legacy-operator.py` — PASS, 8 tests.
- The full fixture deploys a 0.7.2 server/web pair with the declared inclusive
  range; focused cases reject missing bounds, malformed versions, inverted
  bounds, and a target outside the range.

## Required gates before any Pi mutation

1. Independent code, security, and test reviews approve the exact new
   operator bytes and candidate revision.
2. The release coordinator binds the exact signed 0.7.2 server and web
   artifacts, manifests, public key, hashes, expected build and target.
3. A fresh strict-TLS, quiet preflight passes immediately before execution.
4. A distinct verifier checks the deployed pair, fresh backup and protected
   invariants after execution.
