# ADR-0011 — API HTTPS datée et contrat public OpenAPI

- Status : `ACCEPTED` ; date : `2026-10-14`.
- Decision Owner / Author Instance : `/root` ; Reviewer Instance : `/root/ux_api_code_review`.
- Branche / base : `codex/cinefolio-ui-rework` / `5665f60bc59ceb3b2a9caa1622af053af0645d47`.
- Révision : `CC-API-2026-10-14` ; proposition revue : `0a7e8f300d0e1f1b0723cf91d6aade15412027f5` / `2026-10-14T16:27:00Z` ; contenu confirmé inchangé à `9277ac4d51281b3e6d72475294b1e7822cb8e913`.
- Supersedes / Superseded By : `NONE` / `NONE`.
- Related Items : ADR-0001/0003/0005/0006/0007 ; API de tous les clients et US connectées ; CC-UX-002.

## Contexte et facteurs

L'utilisateur demande explicitement d'uniformiser l'API avec la version `2026-10-14` et de la documenter pour une utilisation tierce. Le client Rust utilise déjà HTTPS/JSON et les vidéos passent par des Range HTTP, pas par un montage NAS client. L'ancien contrat `/api/v1` et le client installé doivent rester utilisables après mise à jour du serveur. Aucun changement d'authentification, d'exposition réseau, de TLS ou de transcodage n'est demandé.

## Options examinées

| Option | Intérêt | Risque / réversibilité |
| --- | --- | --- |
| Garder seulement v1 | Aucun changement de client | Ne satisfait pas la date demandée ; documentation non versionnée |
| Remplacer v1 sans alias | Une adresse unique | Casse immédiatement les clients installés ; rollback coordonné nécessaire |
| Route datée + alias, un routeur commun | Date explicite, compatibilité, aucune duplication métier | Alias à maintenir ; nouveau client nécessite serveur mis à jour ; réversible |

## Décision

Adopter `/api/2026-10-14` comme base canonique, date de **contrat**, non calendrier roulant. Monter le même routeur Axum également sous `/api/v1`. L'alias est déprécié sans date de retrait inventée ; `/v1/server` conserve son `api_version: 1`, l'adresse canonique annonce `"2026-10-14"`. Le header `X-Chill-API-Version` indique le contrat réellement servi. Le transport client central utilise la base datée, y compris vidéo, SSE et images.

Publier un OpenAPI 3.1 embarqué à `/openapi.json` sous chaque base, et sa source dans `docs/api/openapi-2026-10-14.json`. Le guide tiers décrit TLS, filtres et limites, révisions, unités, disponibilité, baux, erreurs, archives et absence d'authentification/CORS. Les rejets de framework deviennent l'enveloppe JSON commune `code/message/request_id` sans fuite des détails internes ; les statuts et headers HTTP utiles restent conservés. Ne pas ajouter une enveloppe de succès qui casserait les DTO existants ; ne pas prétendre que toutes les mutations utilisent le même mécanisme CAS.

## Conséquences

- Contrat lisible et exploitable par des clients tiers, avec test de couverture routes/documentation et tests date/alias/rejets HTTP.
- Aucune nouvelle dépendance ou migration SQLite. Coût : document embarqué et faible duplication de métadonnées de routage, pas de traitement vidéo supplémentaire sur le Pi.
- Sans authentification, profils et administration sont accessibles aux clients admis sur le LAN : l'isolation de profils n'est pas une frontière de sécurité. Pas de publication Internet ni assouplissement TLS/CORS. La clé TMDB reste secrète côté serveur ; elle n'est pas un jeton d'authentification de cette API.
- Les réponses tierces doivent tolérer les champs additionnels ; une rupture future nécessite un autre contrat daté et une transition documentée. L'alias ancien ne prouve pas la compatibilité avec tous les anciens consommateurs inconnus.
- Observabilité : header de version et request_id des erreurs. Aucun contenu sensible de requête n'est journalisé pour faciliter les tests.
- Migration : mettre à jour le serveur avant d'activer le nouveau bundle ; ancien client fonctionne via v1. Préserver binaire/app précédents et données. Rollback coordonné vers anciens serveur **et client**, sinon le nouveau client attend une route absente. Aucune restauration destructive ou NAS à modifier pour ce changement.

## Validation et approbation

- Revue indépendante : APPROVED sur la proposition SHA256 `4e8456c60c1cc980a246f8f7192a15d642affce57a67346760ff49f49a9acaff`, [rapport initial](../../reviews/ux-api-code-0a7e8f3.md) et [confirmation exacte finale](../../reviews/ux-api-code-9277ac4.md). Acceptation documentaire, sans réécriture de la décision ; [gate de livraison](../../gates/G-DEV-CC-UX-API-001.md).
- User Gate Applicability : `NOT_APPLICABLE` pour ce détail explicitement demandé, sans nouvelle exception de sécurité ; demande humaine citée ci-dessus. La revue indépendante reste nécessaire avant promotion.
- Preuves : [contrat OpenAPI](../../api/openapi-2026-10-14.json), [guide](../../api/README.md), tests `api::tests::dated_contract_and_legacy_alias_share_data_and_errors` et `api_version::tests`. Ne pas présenter leur présence comme une exécution réussie avant le rapport de livraison.
