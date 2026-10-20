# Revue code indépendante R3 — delta opérateur/guides 0.7.1

- **Mode** : `DEV_STANDARD`, revue opérationnelle delta, non mutante
- **Base du delta** : `fdb80b502db45d10c5384104912f5f246678c32a`
- **Candidat exact** : `f48b351724cb9e62b03fa199599ecb4bb24638a0`
- **Base release historique** : `50733ecf749225449c1aac5a6064125c3a9d249c`
- **Reviewer** : `/root/discovery_code_review`, distinct des auteurs
- **Verdict** : **CHANGES_REQUESTED**

Le code Mac/serveur/web/UI est byte-identique à R2 (`git diff --quiet
fdb80b5..f48b351 -- apps crates web ui` : PASS) et n'a pas été réaudité. Cette
revue couvre uniquement l'opérateur manuel et les guides modifiés. Aucun Pi,
NAS, service, réglage ou certificat n'a été lu ou modifié.

Les deux blocages HIGH de R2 sont fermés dans le principe : l'opérateur pose une
maintenance avant le stop, attend l'état et la pulse générationnelle
`busy=false`/`active_mutations=0` (`scripts/pi-server-binary-operator.py:117-157`,
`:267-277`), et la récupération valide désormais racine privée, journal
root/0600, cible, service et deux pins avant toute action (`:198-220`, `:243-255`).
Le rollback ne stoppe pas le vieux service aux états `prepared`/`quiescing`
(`:225-228`). Les renames synchronisent leurs deux répertoires (`:173-177`) et la
sauvegarde SQLite réellement validée est comparée au snapshot pré-stop
(`:158-172`, `:275-280`). Aucun constat `CRITICAL` ou `HIGH` ne reste ouvert dans
ce delta.

## Constats actionnables

### CR3-F-001 — MEDIUM / P1 — la racine de maintenance legacy n'est pas garantie traversable par le serveur

Le Pi legacy n'a volontairement pas le helper/timer privilégié
(`docs/PROJECT_CONTEXT.md:26-30`), donc `/run/chillcenter-update` peut être absent.
L'opérateur le crée avec `Path.mkdir(mode=0o755)`
(`scripts/pi-server-binary-operator.py:117-122`), mais ce mode est filtré par
l'`umask` du processus root. Avec un `umask 077`, le résultat est `0700`. La
validation accepte également un répertoire root-owned existant quel que soit son
mode. Le serveur non-root doit pourtant traverser cette racine pour observer les
métadonnées du flag (`crates/chill-server/src/updates_api.rs:42-56`).

**Impact** : sur le chemin legacy précisément visé, le serveur peut ne jamais
voir la maintenance ; aucune pulse de même génération n'est créée et chaque
déploiement expire après douze secondes. C'est un refus sûr avant arrêt, mais le
chemin de remplacement production n'est pas fiable selon l'`umask`/mode local.

**Correction requise** : créer puis imposer explicitement la racine root:root
`0755` (ou un mode traversable approuvé) indépendamment de l'`umask`, et refuser
une racine existante group/other-writable ou non traversable par le compte de
service. Tester racine absente sous `umask 077`, racine existante `0755`, `0700`,
`0777` et symlink, puis prouver que le serveur non-root voit la génération.

### CR3-F-002 — MEDIUM / P1 — la durabilité du stage précède ses dernières métadonnées

Le stage est copié puis `fsync`, mais `chown` et `chmod 0755` sont effectués
**après** ce `fsync` sans nouvelle synchronisation du fichier
(`scripts/pi-server-binary-operator.py:261-266`). `durable_replace()` synchronise
ensuite les entrées des répertoires, pas les métadonnées encore sales de l'inode.

**Impact** : après panne machine au voisinage du rename, le nom et le journal
peuvent être durables tandis que le bit exécutable ne l'est pas. Un redémarrage
peut donc laisser le binaire exact mais non exécutable, alors que le reçu annonce
une transition récupérable. Le correctif R3 ferme bien la durabilité des renames,
mais pas entièrement celle du fichier publié.

**Correction requise** : effectuer `chown`/`chmod` avant le `fsync` final du
stage (ou refaire `fsync` après), puis seulement renommer et synchroniser les
deux répertoires. Le test doit vérifier l'ordre, pas seulement compter deux
appels `fsync` dans `durable_replace()`.

### CR3-F-003 — MEDIUM / P2 — CR2-F-004 n'est que partiellement fermé

README, overview, guides de mise à jour et chemins de `CONTRACTS.md` sont migrés.
Mais des contrats d'implémentation actifs recommandent encore des routes qui
retournent maintenant 404 :

- `docs/implementation/PLAYBACK-API.md:3-9` présente toujours une « mise en œuvre
  v1 » et trois routes `/api/v1` ;
- `docs/implementation/ARCHIVE-API.md:3-8` documente encore les exports/imports
  sous `/api/v1` ;
- `docs/implementation/EXPORT-JOBS.md:3-7` conserve quatre opérations v1 ;
- le titre `docs/architecture/CONTRACTS.md:22` reste « API HTTPS v1 » malgré le
  chemin daté corrigé dessous.

Ces fichiers ne sont ni ADR ni release reports historiques, les seules catégories
qu'ADR-0031 demande de conserver comme preuves (`docs/architecture/decisions/ADR-0031-single-canonical-api.md:20-32`).

**Impact** : un mainteneur suivant ces contrats actifs construit encore des
requêtes désormais invalides ; la documentation reste contradictoire malgré la
suppression réseau réussie.

**Correction requise** : migrer ces contrats vers `/api/2026-10-14` et qualifier
explicitement comme historiques les documents réellement conservés. Étendre le
contrôle documentaire au-delà des seules paires FR/EN principales.

## Couverture et preuves

| Contrôle | Résultat |
| --- | --- |
| `python3 -m unittest scripts/test_pi_server_binary_operator.py` | **PASS 14/14** |
| `PYTHONPYCACHEPREFIX=/private/tmp/chill-r3-pycache python3 -m py_compile ...` | **PASS** ; l'invocation initiale sans préfixe a rencontré uniquement le cache Python macOS interdit par le sandbox |
| `cargo fmt --all -- --check` | **PASS** |
| `node scripts/test-bilingual-docs.mjs` | **PASS 23/23** |
| `git diff --quiet fdb80b5..f48b351 -- apps crates web ui` | **PASS**, runtime inchangé |
| `git diff --check fdb80b5..f48b351` | **FAIL documentaire** : ligne blanche finale supplémentaire dans `docs/releases/REL-0.7.1-security-review.md:94` |

Les 14 tests opérateur n'appellent ni `acquire_maintenance()`, ni `quiescent()`,
ni `release_maintenance()`. Les tests de recovery mockent `validate_recovery()` :
ils ne constituent pas encore la régression CLI demandée pour la racine, la cible,
le service et les pins divergents (`scripts/test_pi_server_binary_operator.py:51-73`).
Cette faiblesse de couverture n'invalide pas les gardes visibles dans le code,
mais elle laisse les frontières R3 les plus importantes sans preuve dynamique.
Les 182 tests Node R2 et les builds Mac/Linux remis par le coordinateur concernent
des octets runtime inchangés ; ils ne prouvent ni systemd/root/Pi réel ni les
propriétés d'`umask`/durabilité ci-dessus.

## Verdict

**CHANGES_REQUESTED** pour
`f48b351724cb9e62b03fa199599ecb4bb24638a0`. Les anciens HIGH de quiescence et de
confiance recovery sont résolus par inspection, mais `CR3-F-001` empêche de
qualifier le chemin legacy comme opérable de façon déterministe et `CR3-F-002`
laisse la publication exécutable incomplètement durable. `CR3-F-003` conserve
une contradiction utilisateur visible. Aucun déploiement Pi ne doit utiliser ce
script avant nouveau SHA, tests ciblés puis nouvelle revue exacte.
