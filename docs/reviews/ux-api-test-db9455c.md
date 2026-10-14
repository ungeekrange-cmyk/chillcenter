# Revue indépendante tests / intégration — CC-UX-002 et API 2026-10-14

- **Review Mode** : `INTEGRATION_REVIEW`
- **Branche et commit** : `codex/cinefolio-ui-rework` / `db9455cf202a59be7ec17079ab3ad75cbcb07d44`
- **Base historique** : `5665f60bc59ceb3b2a9caa1622af053af0645d47`
- **Base de correction** : `0a7e8f300d0e1f1b0723cf91d6aade15412027f5`
- **Reviewer Instance** : `/root/ux_api_test_review`, distinct des auteurs, du debugger, du builder et de l'integration manager
- **Verdict** : **CHANGES_REQUESTED**

Le correctif de l'oracle hit-test est causal et `TEST-F-002` est couvert par quatre tests du chargeur réellement livré. Le gate lecteur reste néanmoins fermé : le premier des deux parcours Tauri frais requis a de nouveau atteint un timeout externe à 120 s, après le hit-test corrigé et dans la phase `overlay inactivity`. Le second parcours n'a correctement pas été lancé, puisque la condition de deux succès consécutifs était déjà impossible.

## Preuves reproductibles au SHA exact

| Vérification | Résultat | Détail |
| :--- | :--- | :--- |
| `cargo test --workspace --all-targets --locked --offline --quiet` | **PASS** | 289 réussis, 0 échec, 0 ignoré ; 17,56 s hors confinement pour autoriser les deux sockets loopback de sécurité. L'exécution confinée initiale avait uniquement échoué avec `Operation not permitted` sur ces deux bind et n'est pas un défaut candidat. |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | **PASS** | 0 warning ; 4,32 s. |
| `node --test ui/connected/*.test.js ui/connected/tests/*.test.js` | **PASS** | 45 réussis, 0 échec/skip/todo ; 0,50 s. |
| `node scripts/validate-api-contract.mjs` | **PASS** | 66 chemins, 85 opérations inspectées. |
| `openapi-spec-validator==0.9.0` | **PASS** | `OPENAPI_FORMAL_OK`. L'OpenAPI conserve le SHA-256 `d12c606e4d922c1a52f0fe6f12f72940b1498c442dce7ba43ac9f83f946778f4`. |
| `git diff --check`, statut exact | **PASS** | Aucun défaut de whitespace ; worktree propre à `db9455c`. |
| Tauri player frais A, fixture minimale 1 MP4 + SRT, serveur 18550, répertoire app neuf, watchdog externe 120 s, sessions 99088/46972 | **FAIL** | Traces `boot`, `profile`, `root`, `native load`, `native overlay hit-test` à 2 052 ms, puis `overlay inactivity` à 2 074 ms (`document_hidden=true`). Le hit-test corrigé passe ; aucune phase ni résultat terminal ultérieur. Watchdog : enfant `SIGTERM` à 120 023 ms, aucun PASS, aucun retry. |
| Tauri player frais B représentatif | **NOT_RUN** | Non lancé : après l'échec A, deux succès frais consécutifs étaient impossibles. |

Les preuves API HTTPS réelles, shell Tauri, 30 réponses JSON + Range 206, 38 payloads OAS et suites runtime de `0a7e8f3` restent applicables aux chemins produit inchangés ; elles ne remplacent pas le gate player exact du nouveau SHA.

## Qualité des nouvelles assertions

### Catalogue multi-lots — `TEST-F-002` levé

`ui/connected/catalog-loading.dom.test.js` exécute les fonctions réellement livrées `catalogCommand`, `catalogFilters`, `catalogRequest`, `loadPage` et `renderCatalogWindow` dans une fixture DOM contrôlée. Les assertions sont significatives :

- 103 médias sont récupérés aux offsets exacts `0/48/96`, dans l'ordre, avec 103 identités uniques ;
- le compteur final vaut `103 titres`, le dernier média est rendu après déplacement du viewport et chaque fenêtre DOM reste strictement inférieure à 48 cartes ;
- un chevauchement de lots ne crée ni doublon ni perte ;
- une réponse tardive d'un ancien profil et une réponse tardive d'une ancienne recherche ne remplacent jamais le contexte courant.

Ces tests ferment la lacune de couverture constatée dans la revue précédente sans prétendre mesurer la mémoire d'une vraie webview chargée à 3 000 éléments.

### Oracle natif hit-test — cause confirmée et correction ciblée

Le rapport `/private/tmp/chillcenter-ux-native-diagnostic.md` démontre que l'oracle pouvait mesurer après l'auto-masquage intentionnel à 2,5 s : contrôles cachés, `pointer-events:none`, point résolu vers `#native-video`. L'appel existant `showPlayerControls()` restaure immédiatement le bouton au hit-test. Le delta appelle cette fonction juste avant le rectangle et conserve l'assertion `elementFromPoint`; il ne relâche pas l'oracle et ne change pas le produit. Le nouveau test statique contrôle l'ordre phase → révélation → rectangle → hit-test et l'usage des traces bornées.

Le parcours frais confirme que ce hit-test passe désormais. Il révèle toutefois un incident distinct après l'entrée dans `overlay inactivity` ; il ne permet donc pas de déclarer le lecteur reproductiblement vert.

## Findings

| ID | Statut | Preuve | Action requise | Route |
| :--- | :--- | :--- | :--- | :--- |
| `TEST-F-001` | **OUVERT — bloquant** | Timeout exact `db9455c` du run A à 120 023 ms, sans résultat terminal, après trace `overlay inactivity` ; `document_hidden=true`. Le timeout historique 52212 reste également conservé, sans cause commune présumée. | Geler ce nouveau symptôme et diagnostiquer une hypothèse falsifiable à la fois. Corriger causalement avec un builder distinct. Puis recommencer depuis zéro les deux parcours complets consécutifs sous watchdog : minimal et représentatif, aucun retry, jusqu'au PASS final de toutes les phases. | `agent-debugger`, builder distinct, puis nouvelle revue exacte |
| `TEST-F-002` | **FERMÉ** | 4/4 tests `catalog-loading.dom.test.js` verts avec lots 0/48/96, déduplication, stale profil/requête et DOM borné. | Conserver ces assertions dans la suite. | aucune |

## Limites restantes

- Fournisseurs TMDB et IA réels : `NOT_RUN`.
- Catalogue principal réel de 2 496/3 000 éléments dans une webview et mesure mémoire : `NOT_RUN`.
- Bundle final, signature et chargement dylibs : non promus par cette revue.
- Déploiement exact Pi/NAS, Wi-Fi, deux/trois flux Full HD, son, HDR/HEVC, veille/réveil et multi-écran : `NOT_RUN` sur ce SHA.

## Disposition

Ne pas promouvoir, remettre comme bundle final ni déployer `db9455cf202a59be7ec17079ab3ad75cbcb07d44`. Les suites automatisées, l'API et la correction du hit-test sont favorables ; le lecteur demeure non déterministe sur un parcours central et reproduit un timeout borné. Un nouveau SHA exige deux parcours player frais complets et consécutifs sans retry, puis les validations exactes et les revues indépendantes applicables.
