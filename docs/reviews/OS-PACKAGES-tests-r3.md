# Revue tests indépendante R3 / Independent test review R3 — OS packages

- **Mode:** `EXECUTION_REVIEW`, réévaluation bornée.
- **Base R2:** `1d0f03395dc600a2c289701cf0e42928edf4d284`.
- **Candidat exact / exact candidate:**
  `1811c9342aef13d451105f144b73137bc595b871`.
- **Branche:** `codex/bug0128-durable-operator`.
- **Test author:** `/root/pi_os_maintenance`.
- **Reviewer:** `/root/os_maintenance_tests`, distinct de l'auteur.
- **Verdict:** **APPROVED**.

FR : le constat R2 est fermé. L'attente `TERM` utilise une échéance monotone
de 10 s et le `finally` termine, attend et ferme les pipes sur tous les chemins.
Une régression séparée force une readiness bloquée et prouve ce nettoyage.
EN: the R2 flaky-readiness and child-reaping gap is closed by meaningful,
executed assertions.

## Preuves reproductibles / Reproducible evidence

Environnement reviewer : macOS Darwin 27 arm64, Python 3.9.6, GNU Bash
3.2.57. Aucun apt, dpkg, reboot, SSH, Pi, GitHub ou déploiement réel n'a été
appelé.

| Commande | Exit | Durée | Résultat |
| :--- | ---: | ---: | :--- |
| `python3 -B scripts/test-os-package-maintenance.py` | 0 | 4.163 s | 9 PASS, 0 FAIL, 0 skip/retry/quarantine; aucun `ResourceWarning` |
| `bash -n deploy/linux/maintenance/chillcenter-os-package-maintenance.sh` | 0 | <0.1 s | syntaxe Bash PASS |
| `git diff --check 1d0f03395dc600a2c289701cf0e42928edf4d284..1811c9342aef13d451105f144b73137bc595b871` | 0 | <0.1 s | PASS |
| Comparaison `git show 6abfc719:<fichier> \| cmp - <fichier>` des trois fichiers runtime et du guide | 0 | <0.1 s | quatre fichiers octet pour octet identiques |

## Assertions et fermeture / Assertions and closure

- `TEST-R2-F-001` est **RESOLVED** : la readiness normale attend jusqu'à 10 s
  avec horloge monotone; le chemin nominal asserte toujours rc 143, diagnostic,
  préfixe exact, environnement et absence de reboot.
- `test_stalled_term_readiness_is_reaped` force le faux `dpkg` à bloquer,
  attend l'échec de readiness, puis vérifie `poll() != None` et les deux pipes
  fermés. Le `finally` prévoit TERM puis KILL borné si nécessaire.
- Les protections R2 restent actives : ordre exact du succès; arrêt, code et
  diagnostic exacts pour sept pannes; audits dpkg avant/après; commande
  inexécutable; frontière `noninteractive|C|0077`; cron/service statiques.
- Hashes runtime inchangés depuis R1 : script
  `84397c9fcf5139242c226cd40781a3b678c124b0b9c87d37a15afba55cdd3d9a`,
  unité `5513a7515e96a0a72aecbf93991a564b6c15732913d1eccf54350b75ea7c733f`,
  cron `5ff35905d4e0f3521ac1589489039bfc738160ae8c174664552c7ea5865e38bc`.
- Guide identique à R1 :
  `7d687ce07e6669b9c04d072df58b0d62607c33f90732c10569157ca6a9722344`.

Aucun finding test ouvert sur le candidat exact. Limite inchangée : les fakes
valident le contrôle de flux du script transformé, pas de vrais apt/dpkg/cron/
systemd ni un reboot Debian/Pi. La syntaxe systemd réelle doit être validée sur
le Pi sans démarrer l'unité avant l'installation du cron.
