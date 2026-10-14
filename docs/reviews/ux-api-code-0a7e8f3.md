# Revue delta indépendante finale — CC-UX-002 / API 2026-10-14

- Verdict code : **APPROVED**
- Verdict ADR-0011 : **APPROVED**
- Candidat revu : `0a7e8f300d0e1f1b0723cf91d6aade15412027f5`
- Parent du delta : `50bbdf800a6c46c67ddd3cce74d8358286cd8eb6`
- Base historique : `5665f60bc59ceb3b2a9caa1622af053af0645d47`
- Auteur : `/root` ; reviewer distinct : `/root/ux_api_code_review`
- Revue UTC : `2026-10-14T16:27:00Z`
- ADR-0011 exact : SHA-256 `4e8456c60c1cc980a246f8f7192a15d642affce57a67346760ff49f49a9acaff`
- Périmètre : relecture statique du delta final OpenAPI/validateur/registre de bug et réconciliation des findings des revues `258db0a` puis `50bbdf8`. Aucun GUI, Pi, NAS, déploiement, secret ni mutation source.

## Findings

Aucun finding code ou ADR ouvert sur le candidat exact.

## Résolution du delta `50bbdf8..0a7e8f3`

- L'extensibilité est maintenant appliquée récursivement à tout objet atteignable depuis les réponses. `Subtitle`, `Saga`, `Playlist`, `Candidate`, `ImportCounts`, items AI/import/unavailable/reassociation et `Server.tls` sont ouverts ; les références UUID de réponse pointent vers `UuidResponse` (`docs/api/openapi-2026-10-14.json:3698-3716`, `4453-4558`, `4908-4912`, `5830-5871`, `6067-6149`, `6176-6211`, `6252-6278`).
- La stricte séparation entrée/sortie est préservée pour le type partagé important : `PlaybackContext` de requête reste `additionalProperties:false` (`:4617-4639`) tandis que `Lease.context` utilise `PlaybackContextResponse`, extensible (`:4714-4718`, `6256-6278`). La normalisation mécanique clone les schémas partagés avant d'ouvrir le graphe de réponse (`scripts/openapi-response-extensibility.mjs:1-34`).
- Le validateur parcourt désormais récursivement chaque graphe de réponse, résout les `$ref` de schémas/réponses et échoue sur tout objet imbriqué fermé (`scripts/validate-api-contract.mjs:94-110`). Il protège donc directement la conséquence de compatibilité de l'ADR, au-delà de la simple validité syntaxique OpenAPI.
- `PreferencesInput.autoplay_seconds` expose bien `default:10` (`docs/api/openapi-2026-10-14.json:6238-6248`), en accord avec le guide et le défaut serveur.
- BUG-0048 est passé à `VERIFIED_CODE` sur le SHA porteur du correctif, avec preuves et réserves Pi/RSS explicites (`docs/debug/BUG-0048-tmdb-unbounded-jobs.md:3-11`). Il ne prétend ni vérification matérielle ni déploiement.

Le delta ne modifie aucun handler Rust, UI, worker TMDB, protocole de lecture ni état persistant. Les conclusions favorables de la relecture statique `50bbdf8` sur DTO/headers/status/query/nullabilité, identité date/v1, métadonnées d'erreur, navigation/UI, concurrence et bornes TMDB restent applicables ; seuls ses trois findings documentaires/contractuels ont été corrigés. Les rapports historiques défavorables demeurent inchangés et ne sont pas requalifiés rétroactivement.

## Vérifications

- `git status --short` : propre ; `git rev-parse HEAD` : SHA exact ci-dessus.
- `git diff --check 50bbdf8..0a7e8f3` : PASS.
- `node scripts/validate-api-contract.mjs` : PASS, 66 chemins, version 2026-10-14 ; inclut la nouvelle garde récursive.
- `openapi-spec-validator 0.9.0` depuis l'environnement QA privé existant : PASS sur le document exact.
- `node --test ui/connected/*.test.js ui/connected/tests/*.test.js` : **40 PASS**, 0 échec.
- Le normaliseur a été relancé sur le document exact : sortie idempotente, worktree resté propre.
- Preuves exactes fournies par le coordinateur et cohérentes avec le delta : validation OAS31 des 38 payloads PASS et recompilation de l'OpenAPI embarqué PASS. Je n'attribue pas ces deux exécutions à ce reviewer.
- Preuves inchangées du parent : 289 tests Rust, clippy, smoke 30 réponses HTTPS + Range 206/32 octets et Tauri natif plein écran PASS. Aucun changement runtime dans ce delta ne les invalide ; la certification tests/sécurité finale reste portée par les reviewers dédiés.

## Limites non bloquantes et gates séparés

- Aucun fournisseur TMDB/IA réel, son humain, multi-écran, RSS physique, Pi/NAS, Wi-Fi, deux Dune Full HD, trois flux, codecs/HDR ou veille n'est validé par cette revue. Ces points restent NOT_RUN et doivent être présentés comme tels.
- Le Pi reste sur l'ancien binaire et son profil foyer réel doit rester protégé. Ce rapport n'autorise aucune suppression de données ni ne vaut vérification de déploiement.
- Une promotion exige encore les verdicts test et sécurité applicables au même candidat/delta et la procédure d'intégration ; l'approbation code ne les remplace pas.

## Revue ADR-0011

**Verdict : APPROVED** sur la révision SHA-256 exacte ci-dessus. L'ADR compare trois options, choisit une base datée avec alias sur routeur commun, documente migration serveur-avant-client, rollback coordonné, observabilité, coût faible, sécurité LAN sans authentification et absence de migration SQLite (`docs/architecture/decisions/ADR-0011-date-versioned-api.md:14-35`). Le `User Gate Applicability: NOT_APPLICABLE` est cohérent avec la version datée explicitement demandée et n'introduit aucune exception de sécurité (`:37-40`).

La promesse centrale d'extensibilité des réponses (`:33`) est désormais réalisée récursivement et protégée automatiquement ; les requêtes restent strictes grâce aux schémas entrée/sortie distincts. Le propriétaire peut renseigner le reviewer, le commit/UTC, lier ce rapport et effectuer la transition non sémantique `PROPOSED` → `ACCEPTED` après confirmation des autres gates courants. Toute modification sémantique de l'ADR ou du contrat exige une nouvelle revue.

## Disposition

Code et ADR approuvés pour `0a7e8f300d0e1f1b0723cf91d6aade15412027f5` uniquement. Cette approbation n'est transitive vers aucun SHA ultérieur et ne vaut ni validation matérielle ni autorisation de déploiement.
