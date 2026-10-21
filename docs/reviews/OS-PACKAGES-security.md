# Revue sécurité — maintenance des paquets OS / OS package maintenance

- **Candidat exact :** `6abfc719bff2e8eadd4658d34fb6925a4762df4d`
- **Base :** `efac44d0a14aa4341f0b2c65c5b8955ef159930b`
- **Reviewer indépendant :** `/root/os_maintenance_security`
- **Revu à :** `2026-10-21T15:15:03Z`
- **Verdict :** `APPROVED`

## Périmètre / Scope

Revue limitée aux trois nouveaux fichiers sous `deploy/linux/maintenance/`, à
`scripts/test-os-package-maintenance.py` et à
`docs/operations/OS-PACKAGE-MAINTENANCE.md`. Les autres deltas du commit, les
approbations historiques de BUG0128 sur `a6f0fc0`, l'application Chill Center,
le web, le NAS et leurs configurations sont hors périmètre et inchangés par ce
verdict.

## Contrôles et conclusions / Checks and conclusions

- Le cron utilise le format système Debian, inclut explicitement l'utilisateur
  `root`, vise dimanche 03:00 en heure locale et porte un nom sans point.
- L'unité `oneshot` sérialise les départs concurrents. `KillMode=control-group`
  associe l'arrêt du shell et du processus apt; le trap `HUP/INT/TERM` quitte en
  erreur sans atteindre le redémarrage.
- Le script refuse un EUID non root, emploie des chemins absolus, un `PATH`
  fermé, `DEBIAN_FRONTEND=noninteractive`, `LC_ALL=C` et `umask 077`.
- APT conserve sa chaîne de confiance configurée : aucun drapeau insecure ou
  unauthenticated n'est ajouté. `APT::Update::Error-Mode=any` refuse une mise à
  jour partielle. `-y` n'est accompagné d'aucune permission de modifier les
  paquets retenus, de rétrograder ou de retirer les paquets essentiels.
  `--no-remove` force l'abandon si une suppression serait nécessaire;
  `--force-confold` conserve les conffiles locaux.
- Le redémarrage n'est atteint qu'après succès de `update`, de l'upgrade, de
  `apt-get check` et de deux `dpkg --audit` vides. Toute erreur testée et TERM
  suppriment le redémarrage.
- Aucun secret, donnée de profil, adresse du foyer, nouveau paquet logiciel ou
  changement d'authentification/autorisation n'est présent. Les journaux ne
  contiennent que la sortie opérationnelle d'APT/dpkg et les messages fixes.
- Les timers quotidiens `unattended-upgrades` ne sont ni modifiés ni désactivés;
  les verrous APT/dpkg restent l'arbitrage avec leurs éventuelles exécutions.

Références normatives consultées : manuels Debian 13/Trixie
[`apt-get(8)`](https://manpages.debian.org/trixie/apt/apt-get.8.en.html),
[`apt-secure(8)`](https://manpages.debian.org/trixie/apt/apt-secure.8.en.html) et
[`cron(8)`](https://manpages.debian.org/trixie/cron/cron.8.en.html).

## Preuves reproductibles / Reproducible evidence

| Outil | Commande et portée | Résultat | Limite |
| :--- | :--- | :--- | :--- |
| Git | `git rev-parse HEAD`; `git diff --name-status efac44d..6abfc719`; `git ls-tree -r 6abfc719 -- <scope>` | HEAD exact, fichiers réguliers; script et test `100755`, cron/unité/doc `100644` | Modes Git, pas les modes installés |
| Bash 3.2.57 | `/bin/bash -n deploy/linux/maintenance/chillcenter-os-package-maintenance.sh` | PASS | Analyse syntaxique macOS seulement |
| Python 3.9.6 | `/usr/bin/python3 scripts/test-os-package-maintenance.py` | 7/7 PASS | Commandes factices; aucun apt/reboot réel |
| Git | `git diff --check efac44d..6abfc719 -- <scope>` | PASS | Statique |
| Git grep | motifs de clés privées, jetons et mots de passe sur les cinq fichiers | Aucun résultat | Scan ciblé, pas un scanner complet du dépôt |

Préflight Pi en lecture seule transmis par le coordinateur, et non exécuté par
ce reviewer : Debian 13.7, fuseau `Europe/Dublin`, NTP synchronisé, cron actif;
`apt-daily`, `apt-daily-upgrade` et les mises à jour de sécurité unattended sont
actifs avec redémarrage automatique désactivé. Les trois destinations nouvelles
sont absentes, sans lien pendant. Les hashes des sources préparées correspondent
au candidat; leur installation root et les contrôles de mode restent planifiés.

## Invariant avant déploiement / Pre-deployment invariant

Ce verdict approuve les octets source, pas une installation. Avant
`daemon-reload`, le déployeur doit vérifier les trois destinations sans suivre
de lien inattendu : script `root:root` mode `0750`, unité et cron `root:root`
mode `0644`; leurs répertoires parents doivent rester administrés par root. En
cas d'écart, il faut abandonner avant activation. Ne pas démarrer le service,
ne pas lancer apt et ne pas redémarrer pendant l'installation. Ce contrôle est
notamment requis pour l'unité, dont le guide ne répète pas explicitement le mode
à côté du chemin.

## Findings et limites / Findings and limits

Aucun constat sécurité ouvert dans le périmètre figé. Ce reviewer n'a touché ni
Pi, SSH, dépôt GitHub, service systemd, cron, gestionnaire de paquets, ni
redémarrage réel. Aucune upgrade ou concurrence apt réelle n'a été testée. La
configuration détaillée des sources APT, leurs clés et les paquets retenus ne
fait pas partie des preuves reçues.

English summary: the exact scoped source is **APPROVED**. It preserves APT's
default authentication and package holds, forbids removals and dangerous
override flags, fails closed before reboot, and serializes cron starts through a
root oneshot unit. Deployment remains unapproved until root ownership and modes
`0750`/`0644` are verified; no live Pi or APT behavior was claimed.
