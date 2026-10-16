# Nettoyage autorisé — huit sorties de compilation

16 octobre 2026, auteur `/root`. Autorisation explicite utilisateur : uniquement
les huit dossiers de l’audit, après contrôle d’activité, sources/worktrees/historiques/
versions livrées/client web conservés.

Avant action : aucun cargo/rustc/zig actif, aucun fichier ouvert dans ces chemins
selon lsof. L’application ouverte utilisait le bundle livré hors de ces chemins.
Chemins réels, non symlinks, sans `.git` ni bundle `.app` imbriqué ; sorties Cargo
identifiées lors de l’audit. Seuls ces huit dossiers ont été supprimés :

1. `/private/tmp/chillcenter-series-target.ZXNxst`
2. `/private/tmp/chillcenter-resource-target`
3. `/private/tmp/chillcenter-resource-opt.fmChs3/repo/target`
4. `/private/tmp/chillcenter-ui-rework.4oZp3D/repo/target`
5. `/private/tmp/chillcenter-series-rereview.2LsQSZ`
6. `/private/tmp/chillcenter-library-verify.cXU9LT/target`
7. `/private/tmp/chillcenter-sync-rereview.LIgoDM/target`
8. `/private/tmp/chillcenter-filename-release.aEa4OM/linux-target`

Mesure POSIX sur le même volume, juste avant/après et avant nouvelle compilation :

| Mesure | Octets |
|---|---:|
| Allocation unique des huit chemins, avant | 36 479 254 528 |
| Disponible volume, avant | 151 961 600 000 |
| Disponible volume, après | 187 187 200 000 |
| Augmentation disponible observée | **35 225 600 000 (32,81 Gio)** |

Les huit chemins sont absents après. Les worktrees sources, le chantier web et le
bundle f980355 existent toujours ; Git n’a pas été nettoyé/pruné. Aucun cache client,
SQLite/WAL/SHM, Cargo partagé, historique Codex, sauvegarde ou fichier NAS supprimé.

Les 33,97 Gio d’allocation estimée ne sont pas confondus avec les 32,81 Gio disponibles
réellement observés : APFS et l’activité concurrente du volume influent sur le résultat.
Pas de restauration depuis la corbeille : sorties régénérables, sources conservées.
Les compilations du lot suivant recréent un cache distinct ; le gain ci-dessus est
celui du nettoyage, pas une promesse de gain net après reconstruction.

Après tests, Clippy et packaging, nouveau cache de compilation distinct
`/private/tmp/chillcenter-playback-locale-target` : environ **1,7Gio** ; il ne s’agit
pas d’un des huit chemins supprimés. Anciennes versions livrées toujours conservées.
