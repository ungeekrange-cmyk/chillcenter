# Household OS maintenance / Maintenance OS du foyer — 2026-10-21

## Authority and scope / Autorisation et périmètre

Explicit user request: commit and merge the reviewed work into main; on the Pi,
add automatic OS package updates Sunday around 03:00, followed by reboot.
Coordinator/deployer: `/root`; maintenance author: `/root/pi_os_maintenance`.
This is **not** a Chill Center component update or a release publication.

Preflight target: Debian 13.7 ARM64, APT 3.0.3, cron active, NTP synchronized,
`Europe/Dublin`. No held packages or insecure-repository override observed.
Existing daily security updates, automatic-reboot=false policy and APT timers
remain unchanged. Chill Center application updater/helper remains unconfigured
on the legacy installation; merging main alone does not activate it or publish
a newer signed package. Existing server/web 0.7.2 packages remain unchanged.

## Frozen inputs / Entrées figées

Runtime files were introduced at `6abfc719bff2e8eadd4658d34fb6925a4762df4d`;
subsequent test-harness changes do not change these bytes:

| Source | SHA-256 |
| --- | --- |
| `deploy/linux/maintenance/chillcenter-os-package-maintenance.sh` | `84397c9fcf5139242c226cd40781a3b678c124b0b9c87d37a15afba55cdd3d9a` |
| `deploy/linux/maintenance/chillcenter-os-package-maintenance.service` | `5513a7515e96a0a72aecbf93991a564b6c15732913d1eccf54350b75ea7c733f` |
| `deploy/linux/maintenance/chillcenter-os-package-maintenance.cron` | `5ff35905d4e0f3521ac1589489039bfc738160ae8c174664552c7ea5865e38bc` |

## Installation plan / Plan d'installation

1. Require the exact source code/test/security approvals, no unexpected
   pre-existing destination or symlink, and root-owned non-writable parents.
2. Retain a root-private backup of existing cron/APT configuration and the
   pre-install service/package fingerprints. Copy the new files to a private
   root-owned snapshot and verify all three pinned hashes there.
3. Install only the root-owned script (0750) and unit (0644). Run Bash syntax
   and real `systemd-analyze verify` checks without executing the script.
4. Reload systemd; install the root-owned cron (0644) atomically **last**, as
   `/etc/cron.d/chillcenter-os-package-maintenance` (no extension).
5. Independently verify installed hashes, permissions, inactive maintenance
   unit, live cron, unchanged Chill Center binary/config/web and running PID.

No package-manager execution, service start, immediate reboot, NAS access,
application configuration edit or application binary replacement is authorized
as part of the installation. Scheduled execution later may restart services as
packages are upgraded and will reboot the host after complete success.

Expected schedule: `0 3 * * 0`, Sunday 03:00 Pi local time. First expected run
after this installation: **2026-10-25 03:00 Europe/Dublin (GMT)**. No missed-run
catch-up at boot is added. APT/dpkg failure or nonempty audit suppresses reboot;
logs remain in `journalctl -u chillcenter-os-package-maintenance.service`.

## Rollback and limits / Retour arrière et limites

Disable future runs by removing only the installed cron. Do not abort a running
APT transaction merely to disable scheduling. Once inactive, remove only the
new unit/script and reload systemd; keep the private backup/receipt. Existing
root crontab, daily security updates and Chill Center units are not replaced.
OS package rollback is not guaranteed: a full system backup/snapshot is needed
for that separate recovery. No automatic package upgrade/reboot is claimed as
tested before the first scheduled run.

FR : périmètre limité aux trois nouveaux fichiers de maintenance OS, avec
sauvegarde privée et contrôle indépendant. Le serveur/client web, le NAS et
les réglages Chill Center ne sont pas modifiés; aucune mise à jour ni aucun
redémarrage immédiat n'est effectué. La mise à jour automatique des composants
Chill Center est un mécanisme distinct, non activé par cette opération.

## Execution status / État d'exécution

`INSTALLED`, 2026-10-21 around 15:26 UTC. Source `1811c93` approved by distinct
[code](../reviews/OS-PACKAGES-code-r3.md),
[test](../reviews/OS-PACKAGES-tests-r3.md) and
[security](../reviews/OS-PACKAGES-security-r3.md) reviewers. The integrated
documentation carrier `c1104eb` does not change any reviewed source bytes.
Earlier R1/R2 findings are retained in their original reports, not erased.

The root-private backup includes previous cron/APT configuration, source
snapshots, package inventory, running-service state and protected application
hashes. Backup archive listing and all three pinned snapshots passed. The
installed script is root:root 0750; unit and extensionless cron are root:root
0644. Real `crontab -n` syntax validation and `systemd-analyze verify` passed.
Cron was installed atomically last, after `daemon-reload`, without starting the
maintenance unit.

Installation receipt `INSTALLATION=PASS`: application binary, config and web
build hashes match baseline; current application service state matches exactly
(same PID and restart count); before/after package inventories match. Maintenance
unit: inactive/dead, `ExecMainPID=0`, empty start timestamp. Cron is active.
No package update or reboot was requested during deployment.

Independent post-install verification is **APPROVED_SCOPED** by the distinct
instance `/root/os_maintenance_live_verify`:
[FR/EN evidence](REL-OS-PACKAGES-20261021-verification.md). Installed hashes,
permissions, real syntax checks, local schedule, no premature invocation,
existing cron/APT jobs, package inventory and Chill Center state passed. The
actual first scheduled APT transaction, resulting reboot and post-boot health
remain **NOT_RUN**.

Main integration: fast-forward from `0122c22` to `c1104eb`, pushed to GitHub and
read back at that exact hash. The subsequent documentation-only carrier records
this installation/verification without changing the reviewed source, public
release tags, signed packages, application versions or repository visibility.
The pre-existing moderate non-shipping Linux dependency alert reported by
GitHub remains open; this change modifies no dependency graph.

FR : installation seule réussie après sauvegarde privée, aucun paquet ni
service applicatif remplacé ou redémarré; première exécution automatique et
redémarrage dominical restent à observer. La vérification indépendante est
favorable dans ce seul périmètre et distincte du reçu émis par le déployeur.
