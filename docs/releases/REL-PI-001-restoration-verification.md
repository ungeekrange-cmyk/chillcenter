# Independent restoration verification — Chill Center Pi

- **Environment:** household Raspberry Pi 3 B+ (`chillcenter.local`), Debian 13
- **Release / artifact:** R7, source `126da0af167ee62588d92d90297f2dfc51962676`; ARM64 binary SHA-256 `9a17ffe55e3d4128b9d483d170f5d2ede71382cef42d8637917c8f31e63b36a6`
- **Verifier instance:** `/root/pi_restoration_verifier` (independent restoration verifier; not builder, deployer, or prior reviewer)
- **Checked at UTC:** 2026-10-14T10:58:58Z, with a second stability sample immediately afterwards
- **Mode:** read-only SSH through the user-authorized shared control socket. No service start, file change, mount change, credential access, or NAS content read was performed.
- **Verdict:** **APPROVED_RESTORATION**

## Restoration evidence

| Check | Expected after safety stop | Observed | Result |
| :--- | :--- | :--- | :--- |
| Chill Center service | inactive and disabled | `ActiveState=inactive`, `SubState=dead`, `UnitFileState=disabled` | PASS |
| Restart stability | no continuing crash/restart loop | `NRestarts=0` at both samples | PASS |
| Port 8443 | no Chill Center listener | `ss -ltnH "sport = :8443"` returned no listener at both samples | PASS |
| Installed binary preserved | exact reviewed ARM64 artifact | `/usr/lib/chillcenter/chill-server` SHA-256 matches `9a17…36a6` | PASS |
| Service account | non-login account with approved NAS group only | `uid=997`, primary `gid=983`, supplementary `gid=984 (marvin-admin)`, shell `/usr/sbin/nologin` | PASS |
| Local state protection | state is private to service account | `/var/lib/chillcenter` is `chillcenter:chillcenter`, mode `0700` | PASS |
| Configuration protection | configuration is readable by server group, not public | `/etc/chillcenter/server.json` is `root:chillcenter`, mode `0640` | PASS |
| NAS mount | original NAS remains a read-only CIFS mount | `//nas.local/Video`, `cifs`, including `ro,nosuid,nodev,noexec` | PASS |
| Existing services | SSH and Cockpit remain available | `ssh.service=active`; `cockpit.socket=active` | PASS |

## Scope and follow-up

This is a restoration-only verdict. It confirms the service is safely stopped without altering the NAS or existing services; it does **not** verify the product journeys or approve all user stories.

The deployed R7 instance had been stopped because it failed before readiness due to its workspace-current-directory assumption. The next change must correct that `WorkingDirectory`/runtime metadata issue, receive a fresh integration review, and be deployed and verified by a distinct post-deployment verifier before the release can be considered operational.
