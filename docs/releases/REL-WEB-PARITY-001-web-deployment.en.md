# Web-only deployment — 17 October 2026

**English** | [Français](REL-WEB-PARITY-001-web-deployment.md)

Explicit user authorization: deploy the new web client after backing it up,
without changing the server binary, configuration or NAS. Static assets only,
without stopping or restarting the service.

## Artifact and technical approval

Source a13902ea73aa51fb09977f1e6649e252ff39d984, documentation carrier a0ba4e2.
Archive `artifacts/web/chillcenter-web-test-a13902e.tar`, SHA256
`32176d8fabb110d0889187a7bcbed7ad19ee1d8a25530043fc6a714b2dc885e4`,
build **1cce6dd6cd9d**. Operator `scripts/pi-update-web-assets-operator.sh`, SHA256
`b0baea0b2a572b65c9a79cd792276143b45092dc95523eb47815b8742c743996`.
[Independent APPROVED_SCOPED review](../reviews/web-assets-deployment-a13902e.md)
before switching. No rebuild or archive alteration during deployment.

## Execution and backup

Deployer distinct from authors/reviewers: `/root/web_assets_deployer`.
Operator exit **0**, **17 October at 12:56:36 Europe/Dublin**
(`2026-10-17T11:56:36Z`). Previous build 9091c7934eda → **1cce6dd6cd9d**.
PID **5420** retained, service active, **0** restarts before/after.

Complete private backup created and verified before replacement:
`/root/chillcenter-rollback/predeploy-complete-20261017T115606Z/web-before.tar`,
1,126,400 bytes, 0700 directory, SHA256
`050aead4ce44c2f698e4793893bae90e4d5bd56c017e4c007550d31de05e567e`.
Sorted 60-entry manifest, 56 file hashes; member and metadata descriptions retained
alongside. Entire previous web directory retained at
`/root/chillcenter-rollback/web-20261017T115634Z`.

Preserved preparation failure: the first checksum capture exited 123 (relative
paths resolved from the wrong directory), before TAR creation, incoming copy or
replacement. Partial directory
`/root/chillcenter-rollback/predeploy-20261017T115544Z` remains failure evidence,
**not a backup**. The second capture explicitly uses the correct cwd.
This was a tooling preparation error, not a player/server bug.

## Operator-checked invariants

Hashes captured before switching and again at 11:56:45Z match:

| Item | Retained SHA256 |
| --- | --- |
| Server binary | `6fecabc69258e372d71730a23451f5ff47565f60cbbc352552c12d8ac39424ea` |
| Configuration | `d6167cb1624b6d5888473af76d5470102afedce25b2e29ddbaa032730cd0ceb1` |
| systemd unit | `76f7d34921ff344e344ba2bffa51263248ec6118dad9e15a39ef7078877d72e2` |
| Certificate public key (SPKI) | `3018fb4d28af3899ae9a0261c89ef92d7ebfe508ecbd613b38f6dd6d7d4740df` |

No video-file access, scanning, data import, new keys or NAS writes. Operator
HTTP/HTTPS checks PASS, using strict TLS with the local public certificate,
without bypassing validation. API 2026-10-14 and its v1 alias are unchanged.
Mac bundle and GitHub v0.6 release/tag retained; no push or visibility change.

## Independent verification and limitations

**APPROVED_SCOPED**, `/root/web_assets_post_verifier`, distinct from the deployer,
authors and reviewers, 17 October 12:58–13:01 Europe/Dublin (11:58–12:01 UTC).
No rollback recommended.

- 79 installed files, 78/78 valid checksums, exact coverage and no special files;
  root-owned with no group/world write permission.
- Backup and retained old directory: exact member list, 56/56 identical files,
  old build 9091c7934eda, valid TAR and 0700 directory. Incomplete first attempt
  confirmed at the corrected path above: manifest only and empty hash file.
- Same PID 5420, 0 restarts, matching binary/configuration/unit/SPKI.
- Strict HTTPS: page, build, API 2026-10-14 and v1 PASS; required capabilities present.
  WASM 200 `application/wasm`, 652,895 bytes; served `bridge.js` matches installed file.
- Existing HTTP 8080: page, build and both APIs PASS. Cross-site 403 and `.env` 404.
- No warning/error log entries since preflight began. Current CIFS mount is
  read-only; no NAS-file traversal was performed.

After approval, only the temporary transfer copy `/var/tmp/chillcenter-web.tar`
was removed, after checking its type and exact SHA. The original local archive,
backups and previous web directory remain available. Build and PID were checked
again and remained unchanged after this scoped cleanup.

The complete certificate-file hash and normalized mount-state hash were not
captured beforehand, so no unchanged claim is made for them. The TLS comparison
covers SPKI. No NAS operation was performed within the executed command scope;
this is not an integrity audit of the entire NAS.

Player walkthroughs, progress writes and NAS tests are not rerun for this static
web deployment. Decoding and fullscreen capabilities remain browser-dependent.
Web AI remains excluded.

## Rollback

On a failed check during switching, the operator automatically restores the old
directory. After delivery, retain that directory and private TAR: rename the old
web directory back into place while preserving the new one separately, without
replacing the binary, changing configuration or restarting the server.
Do not overwrite the old backup; serialize restoration with any new deployment,
then recheck build, PID and hashes.
