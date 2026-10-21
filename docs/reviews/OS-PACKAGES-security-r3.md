# Revue sécurité accélérée R3 — maintenance des paquets OS

- **Candidat exact :** `1811c9342aef13d451105f144b73137bc595b871`
- **Parent R2 :** `1d0f03395dc600a2c289701cf0e42928edf4d284`
- **Runtime approuvé initial :** `6abfc719bff2e8eadd4658d34fb6925a4762df4d`
- **Reviewer indépendant :** `/root/os_maintenance_security`
- **Revu à :** `2026-10-21T15:24:20Z`
- **Verdict :** `APPROVED`

## Delta et remédiation / Delta and remediation

Réévaluation accélérée de `1d0f033..1811c934`. Les trois fichiers runtime et le
guide sont byte-identical à `6abfc719`; leurs objets Git, leurs commandes APT et
leurs permissions attendues ne changent pas. Le delta corrige le harnais TERM,
porte les trois rapports R2 historiques, ajoute un plan de livraison daté et
référence le guide dans les deux index documentaires.

`SEC-TEST-R2-001` est **REMEDIATED** dans le candidat exact :

- attente de readiness par horloge monotone jusqu'à 10 s, avec contrôle de la
  sortie prématurée du processus;
- `finally` couvrant réussite, timeout, assertion et sortie anticipée;
- TERM du groupe encore actif, escalade KILL si nécessaire, puis
  `communicate()` afin de collecter systématiquement le fils;
- fermeture explicite de stdout/stderr;
- régression `readiness_stall` qui force l'échec de préparation et vérifie que
  le processus est collecté et les deux pipes fermés.

Le test reste entièrement borné à des commandes factices dans un répertoire
temporaire privé; aucun chemin APT/reboot réel n'est substitué pendant les
preuves. Aucun nouveau privilège, secret, log sensible, dépendance ou surface
réseau n'est introduit.

## Plan de livraison / Delivery plan

`docs/releases/REL-OS-PACKAGES-20261021.md` est cohérent avec les invariants
sécurité approuvés : sauvegarde root privée, copie de staging root privée,
vérification des trois hashes épinglés, parents non inscriptibles, script
`root:root`/`0750`, unité et cron `root:root`/`0644`, vérification systemd sans
exécution, cron atomique installé en dernier et contrôles post-installation de
l'application/configuration/PID. Le plan interdit apt, démarrage du service et
reboot immédiats; il ne prétend pas avoir testé la première exécution planifiée.
Les SHA-256 publiés pour les trois fichiers correspondent aux octets du candidat.

## Preuves reproductibles / Reproducible evidence

| Contrôle | Résultat | Limite |
| :--- | :--- | :--- |
| `git rev-parse HEAD` / parent et `git diff --name-status` | SHA/parent exacts; delta annoncé confirmé | Checkout local |
| `git diff --quiet 6abfc719..1811c934 -- deploy/linux/maintenance docs/operations/OS-PACKAGE-MAINTENANCE.md` | PASS | Identité source, pas installation |
| `shasum -a 256` des trois fichiers runtime | `84397c…d9a`, `5513a7…33f`, `5ff359…8bc`, identiques au plan | Local |
| `python3 -B scripts/test-os-package-maintenance.py` | 9/9 PASS, exécution fraîche | Fakes; aucun apt/reboot réel |
| test TERM isolé, trois processus frais | 3 × PASS | macOS/Python 3.9.6, pas Debian réel |
| `git diff --check 1d0f033..1811c934` | PASS | Statique |
| scan ciblé de secrets sur le delta | Aucun résultat | Pas un scan complet du dépôt |

Le préflight Pi en lecture seule reste celui consigné en R2 : Debian 13.7 arm64,
APT 3.0.3, aucun hold, options insecure/downgrade-to-insecure à `0`, sources APT
root-owned `0644`, timers APT/unattended security actifs et reboot automatique
désactivé. Aucun apt install/upgrade ni reboot n'a été effectué.

## Findings et limites / Findings and limits

Aucun constat sécurité ouvert dans le périmètre R3. Le rapport R2
`CHANGES_REQUESTED` reste immuable pour son SHA; cette approbation concerne
uniquement `1811c9342aef13d451105f144b73137bc595b871` et ne remplace pas les
revues code/tests exactes.

Ce reviewer n'a effectué aucune écriture Pi/SSH/GitHub, aucun déploiement et
aucun test réel d'APT, cron, systemd ou reboot. L'application, le web, le NAS,
les configurations Chill Center et les approbations BUG0128 restent hors
périmètre. Toute modification ultérieure du candidat exige une nouvelle revue.

English summary: **APPROVED** for exact SHA `1811c934…`. Runtime and guide bytes
remain unchanged; the R2 process-leak risk is closed with bounded monotonic
readiness, unconditional termination/reaping and a stalled-readiness regression.
Fresh evidence is 9/9 plus three isolated TERM passes. No live package action,
service start or reboot is claimed.
