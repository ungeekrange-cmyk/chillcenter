# Revue indépendante — CC-UX-002 / API 2026-10-14

- Verdict code : **CHANGES_REQUESTED**
- Verdict ADR-0011 : **CHANGES_REQUESTED**
- Candidat revu : `258db0ad27f05fafb12bd3592013ef300ead93fd`
- Base : `5665f60bc59ceb3b2a9caa1622af053af0645d47`
- Auteur : `/root` ; reviewer distinct : `/root/ux_api_code_review`
- Revue UTC : `2026-10-14T15:51:40Z`
- ADR-0011 exact : SHA-256 `4e8456c60c1cc980a246f8f7192a15d642affce57a67346760ff49f49a9acaff`
- Périmètre : lecture statique du diff exact et des handlers/DTO, exécution du validateur de contrat uniquement. Aucun GUI, Pi, NAS, déploiement, credential ni mutation source.

## Findings

### HIGH — Le contrat OpenAPI publié fait échouer ou ment à plusieurs clients tiers

Le document annoncé comme contrat canonique ne représente pas plusieurs requêtes/réponses réellement servies :

1. `DELETE /profiles/{profile}` documente `X-Chill-Confirm-Delete-Profile: true` (`docs/api/openapi-2026-10-14.json:163-194`), alors que le handler exige `x-chill-confirm-delete: <UUID du profil>` (`crates/chill-server/src/api.rs:347-360`). Une requête construite depuis le contrat reçoit donc `400 DELETE_CONFIRMATION_REQUIRED`.
2. `POST /admin/reassociations` documente `source`/`target` et une réponse `RevisionedMedia` (`openapi:3291-3321`, `5472-5505`), alors que le handler `deny_unknown_fields` exige `source_id`/`target_id` et renvoie `{ "reassociated": true }` (`crates/chill-server/src/identity_api.rs:37-69`). Le corps généré par le contrat est rejeté avant même la mutation.
3. Les deux opérations PUT d'affiche contiennent chacune cinq paramètres identiques `(query, expected_revision)` (`openapi:736-804`, `1337-1405`). OpenAPI exige l'unicité du couple nom/emplacement ; un générateur ou validateur conforme peut refuser tout le document.
4. `Media.category` et `RevisionPatch.category` n'autorisent que `film`/`serie` (`openapi:3794-3799`, `4885-4890`), mais le domaine accepte aussi `spectacle`/`documentaire` (`crates/chill-catalog/src/model.rs:3`). Ces valeurs valides deviennent irreprésentables pour un client généré.
5. Le flux conseille explicitement `If-Match` (`docs/api/INTEGRATION-FR.md:66`) et le handler le vérifie (`crates/chill-server/src/media_stream.rs:443-446`), mais `/media/{media}/stream` ne déclare que session, génération et Range (`openapi:2644-2721`).
6. `SharedSeriesEdit.patch` autorise `null` pour `year` et `overview` (`openapi:4971-5030`), mais `SeriesPatch` les désérialise en `i32`/`String` (`crates/chill-catalog/src/model.rs:156-165`) : un corps déclaré valide obtient un rejet 422. Inversement, `RatingRequest.scope` et `AcquireRequest.context` sont documentés obligatoires (`openapi:5189-5213`, `5214-5233`) alors que les handlers leur donnent respectivement `video` et `PlaybackContext::default()` quand ils sont absents (`crates/chill-server/src/catalog_api.rs:716-724`, `crates/chill-domain/src/types.rs:77-84`).
7. `TmdbConfig.credential` documente seulement une longueur 1..4096 (`openapi:4073-4105`), mais le handler exige soit une clé API exactement 32 caractères hexadécimaux, soit un Bearer graphique de 20..2048 caractères (`crates/chill-server/src/tmdb_api.rs:597-625`). Là encore, des valeurs valides selon le contrat sont rejetées.

**Correction requise :** corriger noms/valeurs de headers, champs et réponses, supprimer les paramètres dupliqués, publier les quatre catégories et documenter `If-Match`. Valider ensuite le JSON avec un validateur OpenAPI 3.1 réel et des requêtes HTTPS construites depuis le contrat.

### HIGH — Plusieurs schémas de réponse rejettent la charge utile Rust réelle

Les divergences ne sont pas seulement documentaires, car beaucoup de schémas ferment les réponses avec `additionalProperties: false` :

- `GET /profiles` référence l'objet `ProfileList {items}` (`openapi:73-95`, `4228-4240`), tandis que le handler renvoie directement `Vec<Profile>` (`crates/chill-server/src/api.rs:305-306`).
- `SeriesResponse` n'autorise que `episodes`/`revision` (`openapi:4273-4292`), mais le handler renvoie aussi `name`, `metadata` et `counters` (`crates/chill-server/src/catalog_api.rs:208-221`).
- `UnavailablePage` attend des items `{id,path}` et omet `epoch` (`openapi:5858-5889`), alors que le DTO émet `{id,title,relative_path,revision}` et `epoch` (`crates/chill-domain/src/administration.rs:15-27`).
- `ReassociationPage` attend `{source,candidates}` et omet `epoch` (`openapi:5507-5540`), alors que le DTO émet une proposition aplatie avec chemins, révisions, label et raison (`crates/chill-domain/src/reassociation.rs:174-199`). Les `offset`/`limit` effectivement consommés par les deux listes administratives ne sont pas documentés (`catalog_api.rs:244-266`, `identity_api.rs:19-34`; opérations `openapi:377-399`, `3268-3290`).
- `Saga` interdit le champ `poster` (`openapi:4339-4365`), pourtant émis par `SagaView` (`crates/chill-domain/src/store.rs:68-75`).
- L'item `AiContext.media` exige `available` et n'autorise que cinq champs (`openapi:5394-5447`), tandis que `AiMedia` n'émet pas `available` et émet année, série, saison, épisode, genres, saga, note et révision (`crates/chill-domain/src/ai.rs:21-44`). Un validateur de réponse échoue systématiquement.
- `POST .../playback/release` annonce `DeletedResponse {deleted}` (`openapi:2514-2545`, `4180-4190`), mais renvoie `{released:true}` (`crates/chill-server/src/playback.rs:1122-1171`).

Cette fermeture contredit en outre l'engagement d'extensibilité du guide (`docs/api/INTEGRATION-FR.md:11`) et de l'ADR (`ADR-0011:33`).

**Correction requise :** dériver ou comparer automatiquement les schémas aux DTO sérialisés, réserver `additionalProperties:false` aux requêtes strictes (ou garantir la politique voulue), ajouter les paramètres de pagination, puis exécuter des validations de réponse sur les handlers réels.

### MEDIUM — Le validateur donne un PASS de couverture, pas de conformité

`scripts/validate-api-contract.mjs:30-74` rapproche seulement les littéraux de routes et méthodes ; `:91-120` vérifie un corps/résultat minimal et les seuls paramètres de chemin. Il ne compare ni DTO, ni status, ni headers/query, ni unicité des paramètres d'opération. Il laisse ainsi passer tous les écarts HIGH ci-dessus tout en affichant `API contract OK: 66 chemins, OpenAPI 2026-10-14`.

Le document de preuves reconnaît honnêtement cette limite (`docs/implementation/UX-API-EVIDENCE.md:10`), mais elle ne permet pas de soutenir l'utilisation tierce demandée.

**Correction requise :** ajouter au minimum unicité `(name,in)`, paramètres/header/query et fixtures de sérialisation Rust contre schémas ; idéalement générer le contrat depuis les types ou lancer des contract tests HTTPS pour les 85 opérations.

### MEDIUM — Le bornage TMDB de 3 000 éléments est contourné par le chemin implicite/automatique

`start_job` refuse plus de 3 000 IDs seulement lorsque `media_ids` est fourni (`crates/chill-server/src/tmdb_api.rs:497-505`). Quand il est omis, `eligible_ids` parcourt tout le catalogue sans plafond (`:314-345`, appel `:515-520`) ; le scan automatique utilise le même chemin (`:347-366`). Le job garde ensuite la liste complète persistée. Sur Raspberry Pi, un catalogue supérieur à 3 000 perd donc la borne annoncée et augmente mémoire, fichier de jobs et durée.

**Correction requise :** appliquer la même limite/batch au chemin implicite et automatique, avec continuation explicite ou plusieurs lots bornés. Ajouter un test >3 000 pour les deux modes.

### LOW — Les artefacts de validation UI se contredisent

Le gate indique `APPROVED_BY_USER` (`docs/gates/G-UI-PROJECT-CC-UX-002.md:5`), puis affirme encore `PENDING inchangé` (`:12`), tandis que la spécification reste `REVIEW_REQUESTED` (`docs/design/DESIGN_SYSTEM.md:5`). La citation utilisateur soutient bien l'approbation ; c'est l'état documentaire qui est incohérent.

**Correction requise :** harmoniser le statut de la spec et du gate sans inventer d'heure ni élargir la portée de l'approbation.

## Vérifications positives

- Un seul routeur est monté sous la date et `/api/v1` (`crates/chill-server/src/api.rs:162-193`) ; `/server` différencie correctement l'identité v1/date (`:236-247`).
- La normalisation des erreurs conserve les métadonnées HTTP et le header de version (`crates/chill-server/src/api_version.rs:18-55`), avec tests pour `Content-Range`, `Retry-After`, `Allow` et HEAD (`:63-127`).
- Le guide rappelle clairement LAN sans authentification, TLS sans `-k`, absence de CORS, baux/Range et absence d'accès NAS direct. Ce choix est conforme au périmètre explicitement accepté ; aucun élargissement cloud n'est introduit.
- L'interface charge le catalogue par lots internes sans pager visible (`ui/connected/app.js:57`, `108-126`) et borne le DOM par fenêtre virtuelle (`:61-97`). Recherche, sidebar, Administration, grandes fiches, filtres de conseil, séries/sagas/playlists et boutons lecteur sont présents dans le candidat.
- Les changements de profil invalident dialogues/lecteur (`ui/connected/catalog.js:22-23`) ; les chargements catalogue, sélecteur et affiche de saga utilisent serveur+profil. La file d'affiches simples ne mémorise que le profil (`:198-213`), mais le parcours normal de changement de serveur passe par déconnexion, retire les cibles et recharge : pas de défaut reproductible retenu sur ce point. Un test serveur identique/profil UUID identique renforcerait néanmoins cette garantie.
- Le worker TMDB est sérialisé et temporisé, protège le secret, gère annulation/reprise et pause pendant lecture. Les preuves restent synthétiques : aucun fournisseur TMDB réel n'est démontré (`docs/implementation/UX-API-EVIDENCE.md:16,24`).

## Évidence et limites de vérification

- Exécuté sur le SHA exact : `node scripts/validate-api-contract.mjs` → PASS 66 chemins, mais insuffisant comme décrit ci-dessus ; `git diff --check 5665f60..258db0a` → PASS.
- Réutilisé, sans le présenter comme ma propre exécution : 282 tests Rust, clippy et 37 tests Node PASS, plus shell Tauri HTTPS local PASS (`docs/implementation/UX-API-EVIDENCE.md:7-16`).
- Le parcours lecteur natif du candidat exact n'est **pas PASS** : l'évidence s'arrête sur l'oracle plein écran (`UX-API-EVIDENCE.md:18`). Le diagnostic post-gel identifie un écart safe-area (frame 1073,5 vs écran physique 1107, soit ~34 px) dans le harnais ; il faut néanmoins refaire le QA natif sur le nouveau SHA avant toute conclusion. Cette réserve est distincte des défauts API statiques.
- Pi/NAS, deux Dune Full HD, 3 flux, codecs/HDR, son humain, veille, RSS webview, vraie clé TMDB et vraie IA : **NOT_RUN** pour ce candidat.
- Le worktree contenait des modifications postérieures au SHA pendant la revue. Toutes les références ci-dessus proviennent de `git show 258db0a:<fichier>` ; elles ne valent pas revue du prochain candidat.

## Revue ADR-0011

**Verdict : CHANGES_REQUESTED.** L'ADR est bien structurée : trois options, décision réversible, migration serveur-avant-client, rollback coordonné, coût/observabilité/sécurité LAN et absence de migration SQLite sont explicités (`ADR-0011:14-35`). Le user gate `NOT_APPLICABLE` est cohérent avec une version datée explicitement demandée et ne crée pas une nouvelle frontière de sécurité (`:37-40`).

Cependant, ses conséquences centrales — contrat « lisible et exploitable » (`:30`) et réponses extensibles (`:33`) — sont contredites par le contrat exact et ne sont pas prouvées par le validateur. L'ADR doit rester `PROPOSED` jusqu'à correction des findings HIGH, validation OpenAPI/contract tests et nouvelle revue indépendante du SHA corrigé. Après correction, l'auteur devra renseigner le commit exact/UTC et lier le rapport distinct ; le reviewer ne modifie ni ADR ni index.

## Disposition

Pas de promotion, bundle final ni déploiement de `258db0a`. Produire un nouveau commit qui corrige contrat/validation, borne TMDB et harmonise les statuts, puis demander une revue delta indépendante et refaire le QA lecteur natif. Les revues test et sécurité restent des gates séparés.
