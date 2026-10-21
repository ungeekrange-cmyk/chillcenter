# Revue tests indépendante R2 / Independent test review R2 — OS packages

- **Mode:** `EXECUTION_REVIEW`, réévaluation bornée.
- **Branche:** `codex/bug0128-durable-operator`.
- **Base R1:** `6abfc719bff2e8eadd4658d34fb6925a4762df4d`.
- **Candidat exact / exact candidate:**
  `1d0f03395dc600a2c289701cf0e42928edf4d284`.
- **Test author:** `/root/pi_os_maintenance`.
- **Reviewer:** `/root/os_maintenance_tests`, distinct de l'auteur.
- **Verdict:** **CHANGES_REQUESTED**.

FR : les nouveaux oracles ferment les deux constats R1 sur l'ordre, l'arrêt,
les diagnostics et l'environnement. Le premier run exact a cependant échoué
sur le check critique `TERM`; sa relance a réussi. Une relance ne transforme pas
un test critique flaky en PASS. EN: R1 assertion gaps are addressed, but the
critical TERM check failed once and passed only on retry, so the gate remains
open.

## Delta et identité / Delta and identity

Le seul delta exécutable depuis R1 est
`scripts/test-os-package-maintenance.py`; trois rapports historiques sont ajoutés.
Les trois fichiers runtime sont octet pour octet identiques à R1 :

- script : `84397c9fcf5139242c226cd40781a3b678c124b0b9c87d37a15afba55cdd3d9a` ;
- unité : `5513a7515e96a0a72aecbf93991a564b6c15732913d1eccf54350b75ea7c733f` ;
- cron : `5ff35905d4e0f3521ac1589489039bfc738160ae8c174664552c7ea5865e38bc`.

## Preuves reproductibles / Reproducible evidence

Environnement : macOS Darwin 27 arm64, Python 3.9.6, GNU Bash 3.2.57.
Aucun apt, dpkg, reboot, SSH, Pi, GitHub ou déploiement réel n'a été appelé.

| Commande | Exit | Durée | Pass/Fail/Skip | Retry | Preuve |
| :--- | ---: | ---: | :--- | :--- | :--- |
| `python3 -B scripts/test-os-package-maintenance.py` | 1 | 7.463 s | 7/1/0 | 0 | `test_term_suppresses_reboot` : readiness absente après 50×20 ms; log limité à `dpkg --audit`; `ResourceWarning` indique le sous-processus encore actif au teardown |
| `python3 -B scripts/test-os-package-maintenance.py` | 0 | 5.042 s | 8/0/0 | 1 | relance complète verte, non utilisée pour masquer le premier échec |
| `bash -n deploy/linux/maintenance/chillcenter-os-package-maintenance.sh` | 0 | <0.1 s | PASS | 0 | syntaxe Bash |
| `git diff --check 6abfc719bff2e8eadd4658d34fb6925a4762df4d..1d0f03395dc600a2c289701cf0e42928edf4d284` | 0 | <0.1 s | PASS | 0 | delta propre |

Le groupe de processus signalé par le premier run n'existait plus lors du
nettoyage ciblé (`kill -TERM -97596` : `no such process`). Aucune modification
de source n'a été effectuée par le reviewer.

## Qualité des assertions / Assertion quality

- `TEST-F-001` R1 est **RESOLVED** : liste exacte de succès et préfixe exact de
  chaque panne, code 1 et diagnostic attendu, y compris échec du second audit.
- La partie environnement de `TEST-F-002` R1 est **RESOLVED** : les fakes
  observent `noninteractive|C|0077` à chaque frontière.
- Garde exécutable manquant : PASS avant toute commande.
- La partie signal de `TEST-F-002` R1 asserte désormais readiness, rc 143,
  diagnostic, préfixe et absence de reboot, mais son attente fixe d'environ une
  seconde est flaky et l'échec de readiness sort avant `kill`/`communicate`.
- systemd/cron réels restent `NOT_RUN`; la validation de syntaxe sur le Pi doit
  rester sans démarrage de l'unité avant le gate d'installation.

## Finding et routage / Finding and routing

| ID | Type | Preuve | Action requise | Route |
| :--- | :--- | :--- | :--- | :--- |
| `TEST-R2-F-001` | Test critique flaky / cleanup incomplet | Premier run : `TERM fixture did not reach upgrade`, seulement `dpkg --audit`, puis `ResourceWarning: subprocess ... is still running`; second run PASS. | Rendre la readiness tolérante à la charge et garantir terminaison/attente du groupe dans tous les chemins, y compris assertion de readiness en échec; rerun indépendant sans retry masquant. | Builder/test author puis nouvelle revue exacte |

Limite inchangée : les fakes prouvent le contrôle de flux du script transformé,
pas le comportement de vrais apt/dpkg/cron/systemd ou d'un reboot Debian/Pi.
