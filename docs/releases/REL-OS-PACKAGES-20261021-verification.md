# Vérification indépendante — maintenance paquets OS / Independent verification — OS package maintenance

- **Environnement / Environment:** Pi Debian 13, contrôle SSH administrateur existant, lecture seule
- **Périmètre / Scope:** les trois nouveaux fichiers de maintenance OS uniquement
- **Entrées source / Source inputs:** runtime `6abfc719bff2e8eadd4658d34fb6925a4762df4d`; candidat tests/revues R3 `1811c9342aef13d451105f144b73137bc595b871`; carrier documentaire `c1104eb6a5ec0926558f88fc818600e770b62595`
- **Vérificateur / Verifier:** `/root/os_maintenance_live_verify`, distinct des auteurs, déployeur et reviewers R3
- **Vérifié à / Verified at:** `2026-10-21` (UTC; session de contrôle indépendante)
- **Verdict:** **APPROVED_SCOPED**

## Preuves / Evidence

| Contrôle / Check | Attendu / Expected | Observé / Observed | Résultat |
| :--- | :--- | :--- | :--- |
| Script installé | SHA `84397c…d9a`, `root:root`, `0750` | Hash exact, fichier régulier, `root:root`, `0750` | PASS |
| Unité systemd | SHA `5513a7…733f`, `root:root`, `0644` | Hash exact, fichier régulier, `root:root`, `0644` | PASS |
| Cron Debian | SHA `5ff359…e38bc`, `root:root`, `0644`, sans extension | Hash exact, fichier régulier, `root:root`, `0644`; ligne `0 3 * * 0 root systemctl --no-block start …` | PASS |
| Temps et déclencheur | Europe/Dublin, NTP, dimanche 03:00 local | `Europe/Dublin`, NTP synchronisé; prochaine occurrence calculée: dimanche 25 octobre 2026, 03:00 GMT | PASS |
| Service de maintenance | chargé mais jamais lancé | `loaded/inactive/dead`, PID principal `0`, horodatage de début vide; journal de l’unité: 0 ligne | PASS |
| Validation statique sur Pi | syntaxe Bash et unité valides, sans exécution | `bash -n` et `systemd-analyze verify` réussis | PASS |
| Services existants | cron et timers APT journaliers inchangés | cron actif/activé; `apt-daily.timer` et `apt-daily-upgrade.timer` actifs/activés | PASS |
| État paquet | snapshots avant/après et état actuel | snapshots avant/après identiques; inventaire actuel identique au snapshot (comparaison normalisée `binary:Package` + version) | PASS |
| État Chill Center | processus, binaire et configuration inchangés | service actif, PID `133136`, `NRestarts=24`; hashes binaire et configuration correspondent au manifeste privé pré-installation | PASS |
| Préservation | root crontab, e2scrub et sauvegarde/receipt | root crontab vide; `e2scrub_all` identique à l’archive pré-installation; sauvegarde privée `root:root/0700`, archive et reçu présents | PASS |

Les valeurs de configuration, le contenu des snapshots privés, l’adresse de l’hôte et le canal SSH ne sont pas reproduits ici.

## Limites et actions non exécutées / Limits and actions not run

- **NOT_RUN:** la maintenance APT planifiée (`apt-get update`, upgrade, `apt-get check`), le redémarrage, et la vérification post-boot. La première exécution planifiée demeure la preuve opérationnelle future, non une preuve anticipée.
- **NOT_RUN:** accès NAS, démarrage manuel de l’unité, activation/désactivation de services, changement de paquets ou de configuration Chill Center.
- Cette approbation ne revendique donc pas un succès réel d’APT ou de reboot; elle approuve exclusivement l’installation immuable, le déclencheur planifié, l’absence d’exécution prématurée et la préservation constatée.

## Retour arrière / Rollback

Pour arrêter les exécutions futures: retirer uniquement le cron installé. Une fois l’unité inactive, retirer ensuite l’unité et le script, puis exécuter `systemctl daemon-reload`; conserver la sauvegarde privée et le reçu. Ne pas interrompre une transaction APT en cours.
