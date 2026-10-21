# Audit de stockage — 21 octobre 2026

## Nettoyage confirmé et exécuté ensuite

Le bilan initial ci-dessous est conservé comme preuve d'audit. Après proposition
des chemins exacts, l'utilisateur a confirmé uniquement les quatre caches :

- `/private/tmp/chillcenter-v07-mac-tests-target` ;
- `/private/tmp/chillcenter-v07-mac-qa-build-target` ;
- `/private/tmp/chillcenter-security-e85f5bb-target` ;
- `/private/tmp/chillcenter-security-88dab67-target`.

Revalidation juste avant suppression : racines réelles sans alias, signatures
Cargo `CACHEDIR.TAG`, quatre scans fournis (zéro erreur/montage traversé), aucun
fichier ouvert observé par `lsof`, aucun processus Cargo/rustc actif observé.
Inventaire commun :42636 inodes de fichiers, aucun lien physique vers une copie
hors de ces quatre racines,16477310976 octets alloués estimés. Hachage limité à
1Mio par cache (nature recréable, pas une preuve exhaustive de doublons).

Suppression des **quatre chemins explicites seulement**, aucun glob de purge.
Leur absence a été vérifiée ensuite. Les13 worktrees restent enregistrés ; Git
commun, branches, historique, `target/` courant, VM de test active, sources,
livrables signés, sauvegardes, sessions et bases sont conservés. Aucun Pi/NAS ou
paquet/tag/release distant n'a été contacté ou modifié.

Après la fin des revues BUG0128, la VM locale démarrée pour ces tests a été
arrêtée proprement par son propriétaire ; disque et preuves conservés. Cet arrêt
relève du cycle de test, pas d'une purge autorisée par le seul audit.

| Mesure du volume, `df -k` | Disponible (Kio) |
| --- | ---: |
| Immédiatement avant | 69194492 |
| Immédiatement après | 84301268 |
| Différence observée | **15106776** |

Gain libre observé : **15469338624 octets ≈15,47Go /14,41Gio**. Ce delta du volume
n'est pas une garantie d'attribution exclusive : APFS/clones/snapshots et les
autres écritures simultanées expliquent l'écart avec16,48Go de blocs inventoriés.
Ce n'est pas un simple déplacement à la corbeille ; les caches supprimés ne sont
pas sauvegardés dans ce nettoyage, mais sont régénérables par compilation.

English: only the four explicitly confirmed external Cargo caches were removed
after fresh activity/path checks. Observed free-space increase:15,469,338,624
bytes. All worktrees, source/history, releases/backups, active VM, current cache
and household systems remain untouched. Removed build caches are regenerable.

## Résultat

Audit **lecture seule** effectué le 21 octobre 2026. Aucune suppression, arrêt de
processus, purge de cache, modification Git ou contact distant n'a été fait.

Le périmètre confirmé est le dépôt ChillCenter du worktree courant
`/private/tmp/chillcenter-fixes-web-parity`, son Git commun et ses worktrees
connus, ainsi que les entrées `/private/tmp` dont le nom et/ou les métadonnées
Git démontrent ce rattachement. Le dépôt principal historique
`/Users/ghillaumesoubigou/Documents/Dev/Perso/ChillCenter` est mesuré seulement
comme stockage distinct à conserver. Les autres projets, le stockage global
Codex, les historiques/sessions, cookies, bases, sauvegardes et releases
distantes ne sont pas attribués à ce travail et n'ont pas été parcourus.

| Élément | Taille allouée/mesurée | État |
| --- | ---: | --- |
| Worktree courant | 37.35 Go | **À protéger** : modifications non commités et cache de build actif pour BUG0128. |
| VM Linux `chillcenter-v07-linux.yvKFaz` | 2.10 Go | **À protéger** : `limactl` et le processus VM ont les disques et journaux ouverts. |
| Worktree `chillcenter-v07-native.LwCXEw/repo` | 3.14 Go | À conserver : 12 641 fichiers ignorés, branche locale sans upstream. |
| Worktree `chillcenter-v07-mac-tests` | 2.55 Go | Candidat conditionnel : arbre Git propre, 8 298 fichiers ignorés, principalement `target/`. |
| Worktree `chillcenter-update-engine.yHnUcF/repo` | 626 Mo | À conserver : 2 863 fichiers ignorés, branche locale sans upstream. |
| Git commun | packs : 40.43 Mo | **Partagé, à protéger** ; un avertissement Git de 64 octets dans l'administration d'un worktree existe, sans nettoyage manuel proposé. |
| Dépôt historique `Documents/Dev/Perso/ChillCenter` | 6.21 Go | **À protéger** : `main...origin/main`, autre racine, aucun changement demandé. |

Les tailles sont des blocs `du`/estimations du scanner : APFS, clones, snapshots
et fichiers ouverts peuvent réduire le gain réellement récupéré.

## Mesures et preuves

Le scanner fourni par `storage-check` a été exécuté sur le worktree courant et
sur `chillcenter-v07-mac-tests`. Le scan courant indique environ 37.35 Go
alloués et a atteint sa limite de hachage de 2 Gio (70 442 candidats non hachés) :
il ne permet donc pas de conclure à l'absence de doublons. Le scanner de
`chillcenter-v07-mac-tests` mesure 2 553 647 104 octets alloués, dont
2 504 724 480 octets sous `target/`, et identifie ces objets Rust comme sorties
intermédiaires. Aucun des deux scans ne suit les liens symboliques ni ne mesure
les doublons entre racines.

Le worktree courant a deux modifications suivies non commités (`scripts/pi-v07-legacy-server-web-operator.sh` et `scripts/test-pi-v07-legacy-operator.py`) ainsi
que 171 268 fichiers ignorés. Il ne doit pas être nettoyé dans ce lot, y compris
son cache `target/`.

Treize entrées de worktree (dont la racine Git historique) sont enregistrées dans
le Git commun. Toutes leurs branches locales
n'ont pas d'upstream configuré ; leurs commits restent dans le Git commun, mais
ne doivent pas être assimilés à des copies publiées. Les arbres suivants sont
propres et sans fichiers ignorés :

- `/private/tmp/chillcenter-docs-fr-en` (40.7 Mo) ;
- `/private/tmp/chillcenter-theme-packs.7xAgPD/repo` (47.8 Mo) ;
- `/private/tmp/chillcenter-v07-hostile-tests` (48.5 Mo) ;
- `/private/tmp/chillcenter-v072-source.GmMsg4/repo` (60.7 Mo, détaché).

Ils restent toutefois des sources/branches locales sans upstream : le fait d'être
propre ne démontre pas qu'ils sont devenus inutiles.

Les racines de cache externes les plus volumineuses dont le nom et le contenu
(`CACHEDIR.TAG`, `.rustc_info.json`, `debug/`) confirment une sortie Cargo sont :

| Chemin | Taille | Indice | Activité |
| --- | ---: | --- | --- |
| `/private/tmp/chillcenter-v07-mac-tests-target` | 7.19 Go | cache Cargo `debug/` | aucun descripteur ouvert observé |
| `/private/tmp/chillcenter-v07-mac-qa-build-target` | 3.13 Go | cache Cargo `debug/` | aucun descripteur ouvert observé |
| `/private/tmp/chillcenter-security-e85f5bb-target` | 3.11 Go | cache Cargo `debug/` | aucun descripteur ouvert observé |
| `/private/tmp/chillcenter-security-88dab67-target` | 3.04 Go | cache Cargo `debug/` | aucun descripteur ouvert observé |

Ces caches ont été créés les 19–20 octobre. Leur nature recréable est établie,
mais leur absence de besoin par les revues/archives en cours ne l'est pas ; ils
sont donc des candidats **à vérifier**, pas une autorisation de purge.

## Propositions — aucune exécutée

| Priorité | Candidat exact | Gain alloué estimé | Conditions impératives et mécanisme natif |
| --- | --- | ---: | --- |
| À vérifier | `/private/tmp/chillcenter-v07-mac-tests-target` | 7.19 Go | Confirmer qu'aucune revue ne requiert ce cache ; supprimer seulement le répertoire de cache exact après une nouvelle vérification d'activité. |
| À vérifier | les trois caches `*-target` listés ci-dessus | 9.28 Go cumulés | Même confirmation ; ce sont des sorties Cargo, sans fichiers ouverts observés au moment du relevé. |
| À vérifier | `/private/tmp/chillcenter-v07-mac-tests` | 2.55 Go | Après confirmation humaine que le worktree/branche `codex/v07-native-archive` n'est plus requis, employer `git worktree remove /private/tmp/chillcenter-v07-mac-tests` depuis le dépôt principal. Cette opération retire le checkout et ses ignorés ; elle ne supprime ni la branche ni les objets du Git commun. Revalider juste avant l'action. |

Gain conditionnel maximal des quatre caches : **16.47 Go**. Avec le worktree
Mac ensuite : **19.02 Go**. Ce ne sont pas des gains approuvés ni garantis.
Les autres anciens worktrees propres peuvent être retirés uniquement par le même
mécanisme Git, chemin par chemin, après confirmation que leur branche locale est
conservée et que leurs références ne sont plus nécessaires.

## Protections et limites

- Protéger intégralement le worktree courant et son `target/`, le Git commun,
  les branches/commits locaux, les changements non commités, les sources
  ignorées uniques, les artefacts publiés/signés, sauvegardes, sessions et bases.
- Ne pas toucher à `/private/tmp/chillcenter-v07-linux.yvKFaz` : la VM
  `colima-v07-security` est active et ses disques de 12 Gio sont ouverts.
- Ne pas supprimer manuellement `.git`, objets Git, reflogs ni le petit élément
  signalé comme "garbage" par `git count-objects`.
- L'inventaire `/private/tmp` a été fait au premier niveau puis ciblé ; les
  nombreuses entrées ChillCenter non démontrées actives restent non attribuées.
  Aucun scan récursif global de `/private/tmp` ou de `CODEX_HOME` n'a été fait.
- L'accès au relevé de processus global était refusé par le sandbox ; les
  contrôles `lsof` ciblés confirment l'activité de la VM, et ne prouvent pas
  l'inactivité globale des autres éléments.
