# ChillCenter Pi deployment — authorized attempt

Date: 2026-10-14. Deployer: distinct integration/deployment agent.

## Inputs and preflight

* Source gate: `G-DEV-CC-UX-API-001`, code/test/security independently
  approved for `9277ac4d51281b3e6d72475294b1e7822cb8e913`.
* Local target was clean at `a3f969f2d6c5d9a3c70beadbb1872bce0556fa8e`.
* Human authorization explicitly covered service stop, private local backup,
  binary-only installation, restart, and unchanged NAS/configuration.
* Immediately before interruption: service active PID 3016; installed R7 SHA
  `9a17ffe55e3d4128b9d483d170f5d2ede71382cef42d8637917c8f31e63b36a6`;
  staged new SHA
  `1eb24e97bcdf8f66440c1681af92d6202c151ebe336c2890ea42a7f8179ac181`;
  no established TCP connection for `chill-server`. Stage was a regular file;
  binary, config and unit were regular non-symlinks with expected modes.

## Result: reverted before binary replacement

* The service was stopped and the prescribed offline command was invoked under
  `chillcenter` from `/usr/lib/chillcenter`:
  `chill-server --backup --config /etc/chillcenter/server.json`.
* The command reached successful backup completion, but the surrounding
  deployment script then failed while parsing the French backup-output line
  (`sed` expression error). This was **before** rollback-copy creation and
  before `mv` of the new binary.
* No binary, configuration, unit, NAS, TLS material, profile or data restore
  was performed. No secret was displayed or copied. The private backup remains
  on the Pi; its path was not captured due to the parser failure.
* To avoid a prolonged outage, the original R7 binary was immediately verified
  and the service restarted. At `2026-10-14T18:45:50Z`: active, PID 4980,
  R7 SHA unchanged, `NRestarts=0`.

## Required next action

This is not a deployment and no live client smoke is claimed. Do not retry the
installation blindly. A distinct reviewer/operator should inspect the private
backup result and authorize a corrected, independently reviewed deployment
command before another interruption. The SSH control master remains open.

## Successful V4 deployment

V4 exact script SHA-256
`55c6be484d50a8065115ae4b1bf867377c24b255640c033e5fc2faf6f3289587`
received an independent `APPROVED` verdict before execution. It was sent only
over the existing strict/batch SSH control connection via stdin to root Bash,
with the explicit public certificate path; no nested command-string quoting was
used.

* Deployed UTC: `2026-10-14T19:12:40Z`.
* Service: active, PID `5145`, `NRestarts=0` at read-only capture.
* Installed binary SHA-256:
  `1eb24e97bcdf8f66440c1681af92d6202c151ebe336c2890ea42a7f8179ac181`.
* Preserved binary-only rollback path:
  `/root/chillcenter-rollback/chill-server-r7-9a17ffe55e3d`, SHA-256
  `9a17ffe55e3d4128b9d483d170f5d2ede71382cef42d8637917c8f31e63b36a6`.
* Fresh private backup:
  `/var/lib/chillcenter/data/backups/62584da8-ce12-45b6-9e1a-50e6accb5767`,
  mode/UID/GID `0700:997:983`. Its contents were neither displayed nor copied.
* V4 completed its pre/post equality checks for configuration digest, systemd
  unit digest and public-certificate SPKI. It did not alter configuration,
  unit, NAS, TLS keys, identity/profile data or databases.

This is startup/hash evidence only. A distinct verifier must still conduct the
authorized real client-connection check and post-deployment smoke; no local
mock or deployer check is presented as live client validation.
