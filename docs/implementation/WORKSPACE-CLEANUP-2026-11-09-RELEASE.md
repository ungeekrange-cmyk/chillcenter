# Nettoyage après livraison — 9 novembre 2026

Demande explicite : publier/installer les trois composants, puis supprimer les
éléments devenus inutiles. Périmètre confirmé : dépôt ChillCenter et espace
`/private/tmp/chillcenter-playback-wake.BEMWYx`. Aucun nettoyage global du Mac,
des sessions Codex, de ses bases partagées, du NAS ou des données du Pi.

Après installations réelles vérifiées (Mac0.7.7, serveur/web0.7.6), le scanner
storage-check a mesuré les racines confirmées sans suivre les liens symboliques.
Inventaires Git et processus contrôlés avant nettoyage ; `lsof` ne trouve aucun
utilisateur du cache Rust. Les quatre worktrees et leur état non committé sont
strictement conservés. Les paquets historiques restent accessibles sur GitHub.

Suppression autorisée et exécutée :

- `target/` de cette tâche, par `cargo clean`, après tous les tests et builds ;
- `release-0.7.7/tooling` et `zig-cache`, outils cross isolés recréables et figés ;
- bundles et paquets locaux Mac0.7.6, serveur0.7.5 et web0.7.5 remplacés ;
- deux copies `.app` des parcours SDK isolés, après arrêt des processus QA.

La sauvegarde personnelle `release-0.7.6/local-install-backup` est déplacée
intacte sous `personal-backups/mac-before-0.7.6`, répertoire privé0700.
Les petits journaux, inventaires signés, reçus et scripts historiques restent
conservés. La release actuelle, ses sources correspondantes, le pack de thèmes
inchangé et les entrées multimédias figées réutilisables restent conservés.

Première phase : espace libre observé de147 792 842 752 à157 343 318 016 octets,
soit **9 550 475 264 octets (9,55 Go)** supplémentaires. Ce gain mesuré ne reprend
ni les5,70 Go libérés avant cette livraison ni le nettoyage du8novembre.
Le compteur logique de Cargo n’est pas ajouté à cette mesure. La variation du
système de fichiers peut inclure de petites écritures concurrentes ; les liens
physiques ne sont jamais comptés comme des gains distincts.

Après cette phase, l’espace temporaire confirmé occupe environ1,46 Go alloués,
dont les téléchargements de contrôle encore nécessaires à la dernière
vérification publique. Le dépôt permanent occupe environ135 Mo, sources/Git
inclus. Aucun défaut d’accès du scanner ; hachage temporaire limité à64 Mio,
sans conclusion globale d’absence de doublons.

Après vérification publique finale PASS des 51 assets/35 fichiers uniques et de
Latest `v0.7.7`, les copies `release-0.7.7/public-verify`, les étapes de packaging
`server-inputs`, `server-package`, `web-installer` et les deux archives d’entrée
recréables ont été retirées. Aucun fichier encore ouvert (`lsof`) ; les 51 assets
canoniques ont été revérifiés avant suppression des copies. Les paquets signés
actuels et les sources correspondantes complètes restent conservés.

Seconde phase : espace libre de 178 946 064 384 à 179 525 959 680 octets,
soit 579 895 296 octets supplémentaires. Total des deux
variations observées pendant les opérations : **10 130 370 560 octets
(10,13 Go)**. Le changement d’espace libre survenu
entre ces deux opérations n’est pas attribué au nettoyage et n’est pas ajouté.
