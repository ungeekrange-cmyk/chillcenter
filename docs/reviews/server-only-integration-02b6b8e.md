# Revue d’intégration delta — compatibilité série legacy

- **Mode** : `INTEGRATION_REVIEW`
- **Reviewer** : `/root/server_only_integration_gate`
- **Coordinateur / intégrateur** : `/root` (instance distincte)
- **Parent revu** : `be57d9f5ca535ffc48d5beb42d18a0adbc2df884` / `CHANGES_REQUESTED`
- **Candidat source** : `02b6b8e6c542c869759558a02e53fa61aaa80ce3` (arbre `35232d8724698e1c2b66a767761e2ef49f382fee`)
- **Base web conservée** : `aad34cec60974782df959cf44c668e470fc4c658`
- **Environnement** : macOS Darwin arm64 27.0.0, Node `v24.13.0`, Git `2.54.0`
- **Verdict** : `CHANGES_REQUESTED`

## Delta et preuves valides

Le delta est borné à la compatibilité titre de série : deux méthodes domaine, branchement GET/PUT du handler, un test API, guide et BUG-0071. Aucun fichier `web/**`, `ui/**`, OpenAPI, dépendance ou migration ne change. Les gates structurels précédents restent applicables : 70 chemins/89 opérations historiques conservés dans le contrat à 76 chemins, listeners/statique/configuration web inchangés, tables/sidecar additifs et `user_version=3`.

Le test ajouté est significatif pour son scénario principal : deux groupes `Doctor Who` (`tmdb_id` 1001/2002), deux profils, GET daté et v1, agrégat de deux épisodes, état vu isolé, `tmdb_id:null`, `season_metadata:{}`, GET sans révision, clés opaques isolées, PUT titre réussi, CAS stale `409`, invariants identité/chemin/saison/épisode, puis PUT opaque limité à un groupe. Les assertions valident le comportement plutôt qu’un simple status.

Preuves rejouées sans Cargo pendant le build ARM64 du coordinateur :

| Commande | Exit | Durée | Résultat |
| --- | ---: | ---: | --- |
| `node scripts/validate-api-contract.mjs` | 0 | 0,05 s | 76 chemins, API `2026-10-14` |
| `git diff --check be57d9f..02b6b8e` | 0 | <1 s | PASS |
| `git diff --quiet be57d9f..02b6b8e -- web ui` | 0 | <1 s | aucun delta assets |
| `git diff --quiet be57d9f..02b6b8e -- docs/api/openapi-2026-10-14.json` | 0 | <1 s | contrat structurel inchangé |

La suite Rust/Clippy exacte-SHA est `NOT_RUN` par ce reviewer afin de ne pas concurrencer le build en cours. Son résultat du coordinateur ne devra pas être présenté comme une réexécution indépendante.

## Finding bloquant

| ID | Type | Preuve | Action requise | Route |
| --- | --- | --- | --- | --- |
| `TEST-F-002` | Compatibilité legacy incomplète / couverture manquante | Le contrat historique décrit `{series}` comme un nom URL-encodé (`INTEGRATION-FR.md:40`) et `SeriesPatch` accepte tout nom non vide jusqu’à 500 caractères, sans préfixe réservé. Pourtant GET classe tout segment commençant par `media:` ou `tmdb:tv:` comme clé opaque (`catalog_api.rs:494-497`) et PUT fait de même (`553-566`). Une série legacy réellement nommée `tmdb:tv:1001` ou `media:<texte>` n’atteint donc pas `legacy_series_view` et ne conserve pas ses opérations historiques. Le test ne couvre que `Doctor Who`. | Rendre l’opt-in opaque non ambigu par rapport à tout titre legacy autorisé, ou obtenir une restriction/version explicite de l’utilisateur. Ajouter les régressions GET/PUT datée et v1 pour des titres ayant ces préfixes, sans affaiblir l’isolation des clés opaques. | Builder, puis nouvelle revue exacte-SHA |

Une validation de forme (`tmdb:tv:<entier>` / `media:<uuid>`) ne suffit pas à préserver **tous** les titres : un titre peut légalement être exactement cette chaîne. Si le même segment doit représenter à la fois un titre historique et une nouvelle identité, il faut une règle de priorité/opt-in ou un endpoint distinct explicitement contractualisé.

## Documentation et livraison

Le guide ajouté promet que les segments titre conservent GET et PUT sans exception, ce que le préfixage actuel ne garantit pas. ADR-0015 affirme encore qu’un nom ambigu doit échouer ; la décision utilisateur inverse désormais ce comportement pour l’API legacy. Conserver l’ADR historique, mais enregistrer l’autorité qui le supersède avant la gate finale selon les règles du dépôt.

Aucun binaire de `02b6b8e` ne doit être promu sur le Pi sous ce verdict. Après correction et nouvelle revue : installer uniquement l’ELF lié au nouveau SHA, conserver octet pour octet `/usr/lib/chillcenter/web`, config/unité/SPKI/http_listen/redirections, puis exécuter les smokes web legacy GET/PUT homonymes et préfixés, API datée/v1, migration/sidecar, et Mac delta — sans lecteur, vidéo, scan NAS ni fournisseur réel.
