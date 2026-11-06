# BUG-0133 — Dépendance ZIP signalée au contrôle de publication

2026-11-06 · détecté après push `671fafe`, avant publication de tout paquet0.7.4.
GHSA-m665-v7pm-7grj : ZIP <=9.0.4, différentiel entre les noms d'entrées du
répertoire central et ceux de l'en-tête local, lors d'une validation puis
d'une extraction par deux lecteurs différents. Correctif amont9.0.5.

Qualification indépendante par `/root/release_review` : Chill Center utilise
`ZipArchive`, valide son nom et crée chaque fichier sans écrasement ; aucun
`ZipStreamReader`. Les mises à jour exigent un manifeste Minisign et une
empreinte signée avant extraction. Le chemin précis décrit n'est pas exposé ;
la dépendance directe est néanmoins mise à jour avant toute nouvelle publication.

Correction ciblée : pin `chill-updates`9.0.5 et lockfile9.0.5. Aucun autre
paquet du lockfile ni comportement TLS n'est modifié. Régression avec CDFH
`safe.txt` et LFH `../owned` : aucun fichier hors du stage ; refus propre ou
extraction cohérente du seul nom validé. Aucun nouveau binaire serveur publié :
le serveur public0.7.3 reste inchangé et n'est pas présenté comme corrigé ici.

`zip`4.6.1 reste transitive de `tauri-plugin-updater`2.14.0, par la feature
partagée avec tar/flate2. Revue indépendante : ses appels d'extraction et erreurs
ZIP sont sous `cfg(windows)` ; le chemin Mac utilise TAR signé. Aucun paquet
Windows n'est construit. Ce résidu du lockfile n'est pas annoncé corrigé ;
à réévaluer lors d'une future mise à jour Tauri. Les autres alertes GitHub,
notamment rustls/glib, ne sont pas fermées ni annoncées corrigées par ce lot.

Premier bundle0.7.4 (`671fafe`) conservé mais exclu de la publication. Tous
les paquets/signatures finaux sont reconstruits depuis le commit correctif.
Contrôles finaux et SHA : [release0.7.4](../releases/REL-0.7.4.md).

Référence primaire : [advisory GitHub](https://github.com/advisories/GHSA-m665-v7pm-7grj).
Pas de reproduction d'exploitation sur installation réelle ni de changement Pi/NAS.
