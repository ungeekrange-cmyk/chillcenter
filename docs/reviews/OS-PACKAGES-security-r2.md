# Revue sécurité accélérée R2 — maintenance des paquets OS

- **Candidat exact :** `1d0f03395dc600a2c289701cf0e42928edf4d284`
- **Parent / candidat R1 :** `6abfc719bff2e8eadd4658d34fb6925a4762df4d`
- **Base source initiale :** `efac44d0a14aa4341f0b2c65c5b8955ef159930b`
- **Reviewer indépendant :** `/root/os_maintenance_security`
- **Revu à :** `2026-10-21T15:20:01Z`
- **Verdict :** `CHANGES_REQUESTED`

## Delta figé / Frozen delta

Réévaluation accélérée limitée à `6abfc719..1d0f033`. Les quatre blobs du
runtime et du guide sont byte-identical :

- cron `1258229035e17b4880b94f492925c93cf47c58ee`;
- unité `2e74911f44e3f470568c23060c171c7cfd402aad`;
- script `23bb0f72b2c18ed07cccbeb8b2a39fa4330284ef`;
- guide `2fad05181fd6cd8e1121a185f9d8e4ab1e8e37ee`.

Le delta modifie seulement `scripts/test-os-package-maintenance.py` et ajoute
trois rapports historiques liés à `6abfc719`. Ces rapports conservent leurs
verdicts et périmètres d'origine; ils ne sont pas transposés en approbations R2.
La revue sécurité R1 reste historique, tandis que le présent rapport lie le
verdict sécurité au nouveau SHA exact.

## Analyse sécurité / Security analysis

Les nouveaux tests sont uniquement des fakes dans un répertoire temporaire
privé; ils ne peuvent appeler ni APT ni reboot. Ils renforcent plusieurs oracles
sans élargir les privilèges ou le périmètre runtime :

- séquence exacte de succès, reboot uniquement en dernière étape;
- arrêt exact à chaque panne, y compris échec du second `dpkg --audit`;
- garde root et garde d'exécutables avant toute commande;
- observation effective de `DEBIAN_FRONTEND=noninteractive`, `LC_ALL=C` et
  `umask 0077` par chaque fake;
- sur l'exécution observée, le scénario TERM atteint l'upgrade, puis sort avec
  `143`, journalise la suppression du reboot et n'exécute aucune étape suivante.

L'analyse R1 du runtime demeure applicable : chemins absolus, unité root
`oneshot`, sérialisation systemd, chaîne de confiance APT par défaut, aucune
permission insecure/unauthenticated, aucune permission de changer les holds,
rétrograder, retirer ou autoremove, `--no-remove`, conservation des conffiles,
audits/checks fail-closed et redémarrage seulement après succès complet. Aucun
secret, donnée de profil, adresse privée, changement d'authentification ou
journal sensible n'est ajouté par le delta R2.

## Preuves reproductibles / Reproducible evidence

| Contrôle | Résultat | Limite |
| :--- | :--- | :--- |
| `git rev-parse HEAD` et parent | `1d0f033…`, parent `6abfc719…` | Checkout local |
| `git diff --name-status 6abfc719..1d0f033` | un test modifié, trois rapports ajoutés | Statique |
| `git diff --quiet 6abfc719..1d0f033 -- deploy/linux/maintenance docs/operations/OS-PACKAGE-MAINTENANCE.md` | PASS, aucune différence | Ne prouve pas l'installation |
| `python3 -B scripts/test-os-package-maintenance.py` (Python 3.9.6) | 8/8 PASS sur une exécution | Fakes; le succès ponctuel ne ferme pas le défaut de cleanup TERM |
| `bash -n deploy/linux/maintenance/chillcenter-os-package-maintenance.sh` | PASS | Parse Bash macOS 3.2.57 |
| parsing AST Python et `git diff --check 6abfc719..1d0f033` | PASS | Statique |
| scan ciblé secrets sur le delta | Aucun résultat | Pas un scan complet du dépôt |

Préflight Pi additionnel transmis par le coordinateur, en lecture seule et non
exécuté par ce reviewer : Debian 13.7 arm64, APT 3.0.3, aucun paquet retenu,
`Acquire::AllowInsecureRepositories=0` et
`Acquire::AllowDowngradeToInsecureRepositories=0`; fichiers de sources APT
root-owned mode `0644`; timers APT quotidiens et unattended security actifs et
activés, reboot automatique désactivé. Aucun apt install/upgrade ni reboot n'a
été effectué.

## Finding bloquant, limites et gate de déploiement

`SEC-TEST-R2-001` — **LOW, bloquant pour le candidat exact**. Dans
`test_term_suppresses_reboot`, l'attente de préparation est bornée à seulement
50 × 20 ms. Si le fake n'atteint pas `upgrade` dans cette seconde, l'assertion
échoue avant `os.killpg` et `p.communicate`. Aucun `try/finally` ne garantit alors
l'arrêt du groupe de processus et la collecte du fils. Précondition : machine ou
scheduler momentanément lent. Effet : processus factice/sleep orphelin, test
flaky et preuve TERM non fiable; les chemins de commandes restent ceux du
répertoire temporaire, donc aucun APT/reboot réel n'est exposé. Remédiation :
attente de readiness raisonnable puis cleanup inconditionnel dans un `finally`
qui termine le groupe encore actif et collecte toujours le processus; exécuter
le test TERM de façon répétée et la suite complète, puis demander une nouvelle
revue du SHA corrigé.

Les octets runtime et guide conservent l'analyse sécurité favorable R1, mais le
nouveau SHA exact n'est pas approuvé tant que ce défaut du harnais n'est pas
corrigé et revu indépendamment. Aucun déploiement ne doit partir de ce gate.
Après correction, avant `daemon-reload`, vérifier sans suivre de lien inattendu :
script `root:root`/`0750`, unité et cron `root:root`/`0644`; installer le cron
atomiquement en dernier. Ne lancer ni service, ni APT, ni reboot pendant
l'installation.

Ce reviewer n'a effectué aucune écriture Pi/SSH/GitHub, aucun déploiement et
aucun test APT, systemd, cron ou reboot réel. L'application, le web, le NAS, les
configurations Chill Center et les approbations historiques BUG0128 restent hors
périmètre et inchangés.

English summary: **CHANGES_REQUESTED** for exact SHA `1d0f033…`. Runtime and
guide bytes are unchanged from the approved R1 candidate, but the TERM test can
fail its one-second readiness assertion before cleanup and leave its fake process
group running. Add unconditional termination/reaping and repeat the exact-SHA
reviews. Live installation and APT behavior remain outside this verdict.
