# Revue tests indépendante / Independent test review — OS packages

- **Mode / lane:** `EXECUTION_REVIEW`, `DEV_STANDARD` safety invariants.
- **Branche / branch:** `codex/bug0128-durable-operator`.
- **Base:** `efac44d`; **candidat exact / exact candidate:**
  `6abfc719bff2e8eadd4658d34fb6925a4762df4d`.
- **Reviewer:** `/root/os_maintenance_tests`; distinct de l'auteur du candidat.
- **Verdict:** **CHANGES_REQUESTED**.

FR : le script observé exécute actuellement les six appels dans le bon ordre,
s'arrête après chaque panne simulée et supprime le reboot sur `TERM`. Cependant,
les assertions commises ne prouvent pas complètement ces propriétés critiques.
EN: reviewer probes observed the correct current behavior, but the committed
assertions do not fully lock the reboot-safety contract.

## Preuves reproductibles / Reproducible evidence

Environnement reviewer : macOS Darwin 27 arm64, Python 3.9.6, GNU Bash
3.2.57. Aucun apt, reboot, SSH, Pi, GitHub ou déploiement réel n'a été appelé.

| Commande / Command | Exit | Durée | Résultat |
| :--- | ---: | ---: | :--- |
| `python3 -B scripts/test-os-package-maintenance.py` | 0 | 7.560 s | 7 PASS, 0 FAIL, 0 skip/retry/quarantine |
| `bash -n deploy/linux/maintenance/chillcenter-os-package-maintenance.sh` | 0 | <0.1 s | syntaxe Bash PASS |
| `git diff --check efac44d..6abfc719bff2e8eadd4658d34fb6925a4762df4d` | 0 | <0.1 s | PASS |
| Probe reviewer via `runpy.run_path("scripts/test-os-package-maintenance.py")`, `MaintenanceTest.setUp()` puis `execute()` pour `success`, `dpkg_fail`, `audit_before`, `update_fail`, `upgrade_fail`, `check_fail`, `audit_after` | 0 | 5.368 s | ordre exact observé; chaque échec s'arrête au préfixe attendu; reboot seulement en succès |
| Même fixture, processus en nouvelle session, attente de l'appel `upgrade`, puis `os.killpg(pid, SIGTERM)` | 0 | 0.406 s | job rc 143, message `interrupted; reboot suppressed`, aucun reboot |

Ordre observé en succès : `dpkg --audit` → `apt-get ... update` →
`apt-get ... --with-new-pkgs --no-remove -y upgrade` → `apt-get check` →
`dpkg --audit` → `reboot`. Les six scénarios d'échec ont produit un code 1
et aucun appel après la commande fautive.

## Couverture et qualité / Coverage and quality

| Risque | Résultat | Qualité de l'oracle |
| :--- | :--- | :--- |
| Reboot uniquement après succès complet | comportement actuel PASS | insuffisant dans le test commis : inclusions partielles + dernière ligne seulement |
| Arrêt au premier échec apt/dpkg | comportement actuel PASS | insuffisant dans le test commis : seulement rc non nul et absence de `reboot` |
| Audits dpkg avant/après, sortie non vide ou rc non nul | PASS sur fakes exécutables | scénarios pertinents; compte/ordre non assertés en succès |
| `TERM` pendant upgrade | comportement actuel PASS | le test ne prouve pas que sa boucle a atteint `upgrade`, ni rc 143/message exact |
| Non interactif, locale et umask | présents dans la source | recherches textuelles seulement; les fakes n'observent pas l'environnement effectif |
| Cron dimanche 03:00 local et noms cohérents | PASS statique | ligne exacte et destination sans point documentées; aucun daemon cron réel |
| Unité systemd | NOT_RUN | Darwin n'est pas une preuve systemd; valider la syntaxe sur le Pi sans démarrer l'unité avant installation |

Le fichier source versionné finit en `.cron`, mais la destination documentée
est bien `/etc/cron.d/chillcenter-os-package-maintenance`, nom Debian sans point.
Le delta retire aussi une ligne vide finale de `docs/reviews/BUG-0128-code.md` ;
aucun contenu ni verdict BUG0128 n'est réinterprété par cette revue.

## Findings et routage / Findings and routing

| ID | Preuve | Action requise | Route |
| :--- | :--- | :--- | :--- |
| `TEST-F-001` | `test_success_runs_exact_safe_commands_then_reboots` n'asserte pas la liste/ordre complets; `test_failures_never_reboot` n'asserte pas l'arrêt immédiat. Un script continuant après une panne, sans rebooter, pourrait passer. | Ajouter des oracles automatisés exacts pour la séquence de succès et le préfixe attendu de chacun des six échecs. | Builder/test author; nouvelle revue exacte requise |
| `TEST-F-002` | Le test `TERM` peut passer sans prouver qu'il a atteint `upgrade`; rc/message ne sont pas assertés. `DEBIAN_FRONTEND`, `LC_ALL` et `umask` sont seulement recherchés comme texte source. | Faire observer et asserter la borne effective : upgrade atteint, rc 143/message, aucun reboot, environnement non interactif reçu par les fakes. | Builder/test author; nouvelle revue exacte requise |

Limites : les fakes prouvent le contrôle de flux du script transformé, pas apt,
dpkg, cron, systemd ni un reboot sur Debian/Pi. Les validations Pi doivent rester
sans démarrage de l'unité jusqu'au gate d'installation explicite.
