# Optional OS package maintenance / Maintenance optionnelle des paquets OS

## Français

Cette option distincte maintient **Debian 13 du Pi** chaque dimanche à 03:00,
dans son heure locale configurée. Ce n'est pas le mécanisme de mise à jour de
Chill Center. Elle effectue `apt-get update`, une mise à niveau conservatrice
(`upgrade --with-new-pkgs --no-remove`), puis `apt-get check`; un redémarrage
n'arrive qu'après toutes ces étapes et deux audits `dpkg --audit` vides.

L'installation est facultative et n'adopte pas l'installateur initial. Copier
seulement les trois nouveaux fichiers séparés : le script vers
`/usr/local/sbin/chillcenter-os-package-maintenance` (root:root, 0750), l'unité
vers `/etc/systemd/system/`, et le fichier cron source vers
`/etc/cron.d/chillcenter-os-package-maintenance` (root:root, 0644). La
destination n'a volontairement pas de point dans son nom, exigence de cron Debian.
Après vérification de leur contenu :

```sh
systemctl daemon-reload
```

Ne pas lancer ni activer immédiatement le service, et ne pas redémarrer lors de
l'installation. Le cron est le seul déclencheur prévu; il soumet le service de
façon non bloquante. L'unité oneshot sérialise les déclenchements concurrents et
le journal est consultable avec :

```sh
journalctl -u chillcenter-os-package-maintenance.service
```

Les timers quotidients `unattended-upgrades` et mises à jour de sécurité déjà
présents restent inchangés. Pour désactiver : retirer le fichier cron. Cela
n'interrompt pas une exécution apt déjà en cours. Pour annuler l'intégration :
retirer ce cron, puis les deux autres fichiers et lancer `systemctl daemon-reload`.
Une restauration de paquets OS n'est pas promise : prévoir un snapshot ou une
sauvegarde complète avant installation.

## English

This separate optional job maintains the Pi's **Debian 13 host OS** every Sunday
at 03:00 in its configured local time. It is not a Chill Center application
updater. It runs `apt-get update`, a conservative
`upgrade --with-new-pkgs --no-remove`, and `apt-get check`; it reboots only after
all steps and two empty `dpkg --audit` checks succeed.

Installation is optional and does not adopt the initial installer. Copy only the
three separate new files: the script to
`/usr/local/sbin/chillcenter-os-package-maintenance` (root:root, 0750), the unit
to `/etc/systemd/system/`, and the source cron file to
`/etc/cron.d/chillcenter-os-package-maintenance` (root:root, 0644). The target
intentionally has no dot in its name, as required by Debian cron. After inspecting
them, run `systemctl daemon-reload` only. Do not start or enable
the service, and do not reboot during installation. Cron is the intended trigger
and submits the unit non-blockingly. The oneshot unit serializes concurrent runs;
inspect it with `journalctl -u chillcenter-os-package-maintenance.service`.

Existing daily `unattended-upgrades` and security-update timers remain unchanged.
To disable, remove the cron file; this does not abort apt already running. To
remove/roll back this integration, remove the cron, then the other two files, and
run `systemctl daemon-reload`. OS package rollback is not promised; take a
snapshot or full backup before installation.

## Test limitation / Limite de test

`python3 scripts/test-os-package-maintenance.py` uses executable fake commands.
It changes fixed command paths and the root guard only in a generated temporary
test copy. It never invokes a real package manager or reboot, so it proves the
job's control flow, not behavior against a live Pi or apt repository.
