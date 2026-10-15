# Revue d’intégration — remplacement serveur seul

- **Mode** : `INTEGRATION_REVIEW`
- **Reviewer** : `/root/server_only_integration_gate`
- **Coordinateur / intégrateur** : `/root` (instance distincte)
- **Base web déployée comparée** : `aad34cec60974782df959cf44c668e470fc4c658` (arbre `0eaf2c6af1aa39f1e85572ba39c78210c3e63a6b`)
- **Source candidate** : `d95d5eaed963dc73b1e57529598085b0367b8553` (arbre `0ffb0d4b2af4bfa016e86218b487e4b6be647fd8`)
- **Carrier documentaire** : `be57d9f5ca535ffc48d5beb42d18a0adbc2df884`; aucun delta hors `docs/**` depuis `d95d5ea`
- **Environnement** : macOS Darwin arm64 27.0.0, Node `v24.13.0`, Git `2.54.0`, jq `1.7.1`
- **Horodatage** : `2026-10-15T21:48:31Z`
- **Verdict** : `CHANGES_REQUESTED`

## Portée du verdict

La compatibilité structurelle du binaire intégré est favorable, mais la promotion **binaire seulement** est refusée dans ce candidat : une requête titre-only émise par les assets web conservés change de succès à erreur pour les séries homonymes. Le dossier `/usr/lib/chillcenter/web`, la configuration, l’unité, TLS et les redirections doivent rester inchangés. Ce rapport n’approuve aucun artefact ARM64 et ne remplace pas le smoke post-déploiement après correction et nouvelle revue.

## Contrat et ancien client web

- Les 70 chemins et 89 opérations du contrat `aad34ce` restent présents dans le candidat ; le candidat expose 76 chemins et 95 opérations.
- Pour chaque opération héritée, les paramètres et corps de requête OpenAPI sont inchangés. Les namespaces `/api/2026-10-14` et `/api/v1` montent toujours le même routeur.
- Une seule réponse d’opération héritée change dans OpenAPI : `PUT /profiles/{profile}/series/{series}` corrige l’ancien schéma déclaré. La réponse runtime historique contenait déjà `episodes` et `revision`; le candidat les conserve et ajoute `key`.
- Les autres changements de schémas hérités sont additifs : `CatalogPage.series_counters.*.series_key`, `SeriesResponse.tmdb_id`, `SeriesResponse.season_metadata`, et la valeur `season` dans deux enums TMDB. Aucun champ historique requis n’est supprimé.
- Les révisions `uint64` restent numériques. Le web déployé possède déjà `json64.js`, dont les assertions vérifient `18446744073709551615` sans perte en `BigInt`; aucun nouveau rétrécissement n’a été trouvé.
- Les objets Git de `config.rs`, `connection.rs`, `main.rs` et `web_static.rs` sont identiques entre `aad34ce` et `d95d5ea`. Le listener HTTP optionnel, le service statique, la garde d’origine et la prise en charge de `http_listen` sont donc conservés par le remplacement du binaire.

## Finding bloquant

| ID | Type | Preuve | Action requise | Route |
| --- | --- | --- | --- | --- |
| `TEST-F-001` | Régression de compatibilité web | Le web `aad34ce` ouvre une vignette série par `m.series || m.title` (`ui/connected/app.js:99`), puis `GET /profiles/{profile}/series/{title}` (`catalog.js:84-88`, `web/src/bridge.js:171`). L’ancien `Store::series` retournait tous les membres de ce titre. Dans `d95d5ea`, `Store::series` retourne une erreur si ce titre correspond à plus d’un groupe (`store.rs:688-700`), transformée en `422 INVALID_OPERATION`; `PUT` titre-only passe par la même résolution (`catalog_api.rs:538-545`). | Préserver les sémantiques titre-only de l’API datée/v1 pour les assets web existants, ou obtenir un opt-in/version explicite de l’utilisateur et livrer les assets compatibles dans une opération coordonnée. Ajouter des régressions avec deux groupes de même titre couvrant GET et PUT legacy ainsi que les clés opaques groupées. | Builder, puis nouvelle revue exacte-SHA |

Le fait que l’ancien comportement fusionne incorrectement des homonymes ne permet pas de le retirer implicitement : l’utilisateur a exigé l’absence de rupture du web existant et le déploiement prévu conserve volontairement ses assets. Les nouveaux clients peuvent continuer à utiliser `tmdb:tv:<id>` ou `media:<uuid>` pour la sémantique sûre. La correction la plus locale consiste à réserver cette nouvelle résolution aux clés opaques et à conserver le fallback historique pour un segment de titre brut ; le choix reste au builder/coordonateur, pas au reviewer.

## Migration et retour arrière

Le démarrage ajoute seulement des objets compatibles au schéma SQLite existant (`CREATE TABLE IF NOT EXISTS` pour groupes/saisons et checkpoints, triggers associés) et conserve `PRAGMA user_version=3`. Les tables nouvelles sont vides par défaut ; les routes historiques n’exigent pas leur préremplissage. Le candidat crée aussi un sidecar `catalog.sync-generation` privé `0600`, stable entre redémarrages et refusant les symlinks. L’ancien binaire ignore ces tables et ce sidecar.

La migration sur la vraie base du foyer n’a pas été exécutée par ce reviewer. Elle est donc `NOT_RUN`, pas `PASS` : sauvegarde offline fraîche et smoke de premier démarrage restent obligatoires. Un échec de création/lecture du sidecar bloque volontairement le démarrage et doit déclencher le rollback du binaire, sans supprimer le sidecar ni restaurer la base sauf preuve d’altération.

## Preuves rejouées indépendamment

| Commande | Exit | Durée | Résultat |
| --- | ---: | ---: | --- |
| `node scripts/validate-api-contract.mjs` | 0 | 0,07 s | 76 chemins, OpenAPI `2026-10-14` |
| `node --test web/test/bridge.test.js ui/connected/sync.dom.test.js ui/connected/catalog-loading.dom.test.js` | 0 | 0,15 s | 27 passés, 0 échec, 0 ignoré/retry |
| `git diff --check 1b251cd..d95d5ea` | 0 | <1 s | PASS |
| `git diff --check aad34ce..d95d5ea -- . ':(exclude)docs/**'` | 0 | <1 s | PASS source |

Les assertions vérifient notamment l’exactitude `uint64`, les corps mutatifs stricts, la pagination 0/48/96, la conservation de l’ancienne vue lors d’une page tardive en échec, l’acquittement uniquement après publication complète, le retry après 503 et l’isolation profil/contexte. Les 293 tests Rust, 59 tests Node, Clippy, build web 54 fichiers et build ARM64 sont des preuves du coordinateur/précédent reviewer ; ils n’ont pas été réexécutés ici et ne sont pas requalifiés comme indépendants.

## Smoke exigé après correction, nouvelle revue et remplacement du seul binaire

1. Lier l’ELF ARM64 installé à `d95d5ea` par SHA/provenance, confirmer le SHA préflight `ffcc6349…`, créer une sauvegarde offline et conserver le rollback binaire.
2. Comparer avant/après le hash ou manifeste complet de `/usr/lib/chillcenter/web`; aucune copie, reconstruction ou permutation du dossier web n’est permise. Confirmer aussi les SHA configuration/unité/SPKI et la présence inchangée de `http_listen`.
3. Confirmer service actif, nouveau PID, `NRestarts=0`, listeners HTTPS `8443` et HTTP `8080`, redirections 443/80, et absence d’erreur migration/sidecar. Vérifier le sidecar service-owned `0600` et `user_version=3` sans lire de données de profil sensibles.
4. Vérifier en TLS strict `/api/2026-10-14/server`, `/api/v1/server`, OpenAPI 76 chemins, puis `GET catalog/revision` pour un profil autorisé. Un POST delta n’est nécessaire que pour le smoke Mac synchronisé et doit être consigné comme écriture de checkpoint.
5. Charger **les assets web déjà installés** par HTTPS et HTTP, puis vérifier le parcours lecture seule : démarrage, choix du profil, catalogue, fiche, saga/liste, série non ambiguë et fixture/entrée homonyme représentative selon la sémantique legacy approuvée. Vérifier qu’aucun asset n’a changé pendant ce parcours.
6. Smoke Mac réel sans lecteur : connexion TLS épinglée, catalogue/cache, fiche/sagas/listes et synchronisation delta. Aucun lecteur, vidéo, scan NAS, TMDB/IA réel ou mutation de catalogue n’est requis par ce gate.

Le candidat `d95d5ea`/carrier `be57d9f` ne doit pas être promu tant que `TEST-F-001` n’est pas corrigé ou explicitement accepté/versionné par l’utilisateur. Après correction, toute perte d’asset web, refus de `http_listen`, disparition d’une route historique, échec de migration, restart inexpliqué ou échec du parcours web critique impose le rollback du binaire.
