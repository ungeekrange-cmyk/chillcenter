# Revue de code indépendante — maintenance OS / OS package maintenance

- **Mode :** `DEV_STANDARD`, revue exacte avant installation.
- **Base :** `efac44d0a14aa4341f0b2c65c5b8955ef159930b`.
- **Candidat :** `6abfc719bff2e8eadd4658d34fb6925a4762df4d`.
- **Branche observée :** `codex/bug0128-durable-operator`.
- **Reviewer :** `/root/os_maintenance_code`, distinct de l'auteur.
- **Verdict :** **APPROVED_SCOPED**.

FR : aucun constat correctif ouvert dans le périmètre figé. EN: no actionable
code finding remains in the frozen scope. Ce verdict approuve les octets source,
pas une installation Pi ni une exécution réelle d'APT ou du redémarrage.

## Périmètre et analyse du delta / Scope and delta analysis

Revue limitée aux trois fichiers nouveaux de `deploy/linux/maintenance/`, à
`scripts/test-os-package-maintenance.py` et au guide bilingue
`docs/operations/OS-PACKAGE-MAINTENANCE.md`, dans le delta exact
`efac44d..6abfc719`. Les changements documentaires BUG0128/stockage portés par
le même commit sont hors périmètre. Aucun fichier d'API, serveur, client, helper
de mise à jour applicative, timer existant ou configuration APT n'est modifié.

- Le cron système `0 3 * * 0 root ...` exprime dimanche 03:00 dans l'heure
  locale de cron et soumet uniquement l'unité `oneshot`. Le guide impose la
  destination Debian sans point
  `/etc/cron.d/chillcenter-os-package-maintenance`; le suffixe `.cron` reste un
  nom de source et ne doit pas être conservé à destination.
- L'unité est statique (aucune section `[Install]`), root, `Type=oneshot`, avec
  `KillMode=control-group`. Elle ne démarre rien à l'installation; les départs
  concurrents du même service sont sérialisés/coalescés par systemd. Le
  `--no-block` du cron ne transforme pas la soumission en preuve de succès : le
  résultat reste dans le journal de l'unité.
- Le script fixe les chemins, l'environnement non interactif et l'umask, refuse
  un EUID non root, puis exige successivement un audit dpkg vide, `apt-get
  update`, un `upgrade --with-new-pkgs --no-remove`, `apt-get check` et un second
  audit vide. Toute erreur ou HUP/INT/TERM sort avant la ligne de redémarrage;
  seul le chemin de succès exécute `/usr/sbin/reboot`.
- La stratégie reste conservatrice : aucune suppression, dist-upgrade,
  rétrogradation ou désactivation des timers quotidiens n'est introduite. Les
  verrous APT/dpkg arbitrent la concurrence avec la maintenance Debian
  existante; un conflit ou un dépôt incomplet fait échouer le job sans reboot.
- Le guide demande uniquement `daemon-reload` après copie et interdit start,
  enable, apt et reboot pendant l'installation. La désactivation retire le
  cron sans prétendre interrompre une exécution déjà active.

## Permissions et cycle d'installation / Permissions and lifecycle

Les modes Git observés sont `100755` pour le script et `100644` pour l'unité et
le cron; Git ne porte ni propriétaire ni mode `0750`. L'installation doit donc
appliquer et revérifier explicitement `root:root`/`0750` au script et
`root:root`/`0644` à l'unité et au cron, sans suivre de lien inattendu, avant la
copie finale du cron. Le plan transmis au reviewer prévoit ces contrôles et
`systemd-analyze verify` avant activation; ils restent un gate de déploiement,
pas une propriété prouvée par ce commit.

## Preuves, réutilisation et limites / Evidence, reuse and limits

- `git rev-parse HEAD` et les objets Git confirment le candidat/base exacts;
  `git diff --check efac44d..6abfc719` : **PASS**.
- `bash -n deploy/linux/maintenance/chillcenter-os-package-maintenance.sh` :
  **PASS**; parsing AST du test Python : **PASS**.
- `python3 scripts/test-os-package-maintenance.py` : **7/7 PASS**. Les fakes
  couvrent succès, garde root, six classes d'échec, TERM, ordre/flags et contrat
  cron/unité; ils ne lancent ni APT ni reboot.
- La revue sécurité indépendante exacte
  [OS-PACKAGES-security.md](OS-PACKAGES-security.md) est **APPROVED** et conserve
  les mêmes invariants de permissions. Sa certification reste distincte.
- Aucune preuve historique BUG0128 n'est transposée : ce sont de nouveaux
  fichiers à partir de `efac44d`. Les observations Pi en lecture seule et le
  plan d'installation sont réutilisés seulement comme préconditions, jamais
  comme preuve d'exécution du job.

`systemd-analyze`, cron Debian 13, APT réel, concurrence réelle avec
`unattended-upgrades`, redémarrage et état post-boot restent **NOT_RUN** dans
cette revue macOS. Ils doivent être vérifiés par le déployeur/test reviewer sans
lancer le service pendant l'installation. Toute modification des cinq fichiers
revus rend ce verdict caduc.
