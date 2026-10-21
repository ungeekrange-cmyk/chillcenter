# Chill Center 0.7.2 — Pi execution record

## Status

**`PENDING_REVIEW`**. This record covers only the deployment-operator
compatibility correction. It is not permission to stage artifacts, stop the
service, create a backup, swap payloads, or contact the Pi.

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
