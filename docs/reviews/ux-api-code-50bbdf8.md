# Revue delta indépendante — CC-UX-002 / API 2026-10-14

- Verdict code : **CHANGES_REQUESTED**
- Verdict ADR-0011 : **CHANGES_REQUESTED**
- Candidat revu : `50bbdf800a6c46c67ddd3cce74d8358286cd8eb6`
- Parent du delta : `258db0ad27f05fafb12bd3592013ef300ead93fd`
- Base historique : `5665f60bc59ceb3b2a9caa1622af053af0645d47`
- Auteur : `/root` ; reviewer distinct : `/root/ux_api_code_review`
- Revue UTC : `2026-10-14T16:22:19Z`
- ADR-0011 exact : SHA-256 `4e8456c60c1cc980a246f8f7192a15d642affce57a67346760ff49f49a9acaff`
- Périmètre : lecture statique du delta exact, contrat/handlers/DTO, UI connectée, worker TMDB et ADR ; validation Node ciblée. Aucun GUI, Pi, NAS, déploiement, secret ni mutation source.

## Findings

### MEDIUM — L'OpenAPI ne tient pas sa promesse d'extensibilité pour plusieurs objets de réponse imbriqués

Le guide public impose aux clients d'ignorer tout champ de réponse inconnu (`docs/api/INTEGRATION-FR.md:11,36`) et l'ADR engage le contrat daté à tolérer les ajouts (`docs/architecture/decisions/ADR-0011-date-versioned-api.md:33`). Les enveloppes de réponse principales ont bien été rendues extensibles dans ce delta, mais plusieurs objets qu'elles contiennent gardent `additionalProperties: false` :

- `Subtitle`, `Saga` et `Playlist` (`docs/api/openapi-2026-10-14.json:4423-4452`, `4469-4502`, `4531-4557`), pourtant atteints par les réponses `SubtitleList`, `SagaList` et `PlaylistList` (`:850`, `:1110`, `:1607`) ;
- `PlaybackContext` (`:4613-4635`), présent dans la réponse `Lease` (`:4665-4716`) ;
- `Candidate` (`:4874-4907`) dans la réponse TMDB (`:2913`) ;
- les objets imbriqués de `ImportPreview`, dont `ImportCounts` et les profils (`:5803-5829`, `5831-5867`) ;
- les items de `UnavailablePage` (`:6046-6079`), `ReassociationProposal` (`:6160-6205`) et l'objet `Server.tls` (`:6092-6146`).

Avec ce schéma, ajouter un champ dans l'un de ces objets sous la même version date rend la réponse invalide selon le contrat 2026-10-14, même si le conteneur est ouvert. Un validateur/générateur strict peut donc refuser précisément l'évolution présentée comme compatible. Le PASS OpenAPI formel prouve que le document est syntaxiquement valide, pas que cette politique de compatibilité est respectée.

**Correction requise :** rendre extensibles tous les objets atteignables depuis une réponse, en séparant les schémas entrée/sortie quand un même type doit rester strict en requête (notamment `PlaybackContext`), puis ajouter au validateur une traversée récursive des schémas de réponse. L'alternative est de restreindre explicitement le guide et l'ADR à une extensibilité de premier niveau, mais ce serait une décision de compatibilité sensiblement plus faible à faire valider.

### LOW — Le défaut `autoplay_seconds = 10` n'est pas exprimé dans le contrat machine

`PreferencesInput` dit qu'un champ omis reprend son défaut et déclare bien `default: "en"` pour les deux langues (`docs/api/openapi-2026-10-14.json:6219-6232`). Son `autoplay_seconds` autorise `null/0/5/10/20/30` mais ne porte aucun `default` (`:6234-6243`), tandis que le guide garantit explicitement 10 secondes à l'omission (`docs/api/INTEGRATION-FR.md:44`) et que le serveur applique ce défaut.

Un client tiers ne peut donc pas déduire entièrement la sémantique de remplacement depuis l'OpenAPI et un SDK généré ne présente pas le bon défaut.

**Correction requise :** ajouter `default: 10` à `PreferencesInput.autoplay_seconds` et conserver un test statique de cette assertion avec les deux défauts de langue.

### LOW — Le registre BUG-0048 est resté à l'état « correction en cours » après intégration du correctif

`docs/debug/BUG-0048-tmdb-unbounded-jobs.md:3` conserve `ROOT_CAUSE_CONFIRMED` et nomme une correction « en cours », alors que le même candidat contient les bornes, tests et preuves finales (`docs/implementation/UX-API-EVIDENCE.md:28-34`). Cela brouille la traçabilité demandée : le défaut historique reste réel, mais son état courant n'est plus celui du candidat exact.

**Correction requise :** passer le bug à l'état corrigé/en attente de revue selon le vocabulaire du projet, référencer le SHA exact et conserver explicitement la réserve RSS/Pi non exécutée. Ne pas le marquer vérifié sur matériel.

## Delta historique résolu

Le delta corrige les défauts bloquants de la revue `258db0a` sans les réécrire rétroactivement :

- headers de suppression et `If-Match`, catégories, pagination, nullabilité/defaults, corps de réassociation, réponses réelles, identité date/v1 et métadonnées d'erreur sont maintenant alignés ;
- les requêtes ont leurs schémas stricts dédiés et les réponses principales sont ouvertes ; le défaut restant est la récursion d'extensibilité décrite ci-dessus ;
- le validateur vérifie maintenant refs, routes/méthodes, unicité des paramètres/operationId, corps stricts et formes critiques ; le smoke HTTPS et la validation OAS31 documentés couvrent les DTO réels ;
- les jobs TMDB possèdent deux réservations réelles jusqu'à fin de tâche, historique 8/15 min, worklist 3 000, état 2 Mio et désérialisation bornée (`crates/chill-server/src/tmdb_api.rs:45-51`, `216-267`, `269-369`, `383-567`). Les chemins implicite, auto, retry, annulation et saturation ont des régressions ciblées ; aucun nouveau défaut concret n'a été identifié dans les verrous/ownership inspectés ;
- les statuts du design et du gate UI sont maintenant cohérents à `APPROVED_BY_USER` (`docs/design/DESIGN_SYSTEM.md:5`, `docs/gates/G-UI-PROJECT-CC-UX-002.md:5-12`) ; la fenêtre catalogue conserve des hauteurs déterministes et aucun pager visible ; les erreurs TMDB expiré/saturé sont prises en charge.

## Vérifications et limites

- Exécuté sur le SHA exact : `git diff --check 258db0a..50bbdf8` → PASS ; `node scripts/validate-api-contract.mjs` → PASS, 66 chemins ; `node --test ui/connected/*.test.js ui/connected/tests/*.test.js` → **40 PASS**.
- Réutilisé comme preuve attachée au même SHA, sans le présenter comme ma propre exécution : **289 tests Rust PASS**, clippy `-D warnings` PASS, plein écran Tauri natif PASS après correction de l'oracle QA safe-area, validateur OpenAPI 0.9 PASS, **30 réponses HTTPS réelles + Range 206/32 octets**, puis **38 payloads OAS31 PASS** (`docs/implementation/UX-API-EVIDENCE.md:30-34`).
- Une tentative locale de relancer `validate-api-responses.py` n'était pas probante : la dépendance QA-only `jsonschema` n'est pas installée dans cet environnement et le capture-file requis n'était pas fourni. Le résultat formel cité reste celui de la session privée exacte documentée, pas un nouveau PASS de ce reviewer.
- Le QA natif mesure la safe area AppKit, distincte de l'écran physique ; son humain et multi-écran restent NOT_RUN. Aucun fournisseur TMDB/IA réel, RSS physique, Pi/NAS, Wi-Fi, deux Dune Full HD, trois flux, codecs/HDR ou veille n'est validé ici.
- Le Pi reste sur l'ancien binaire avec le profil foyer protégé ; ce rapport n'autorise aucune mise à jour matérielle ni suppression de données.

## Revue ADR-0011

**Verdict : CHANGES_REQUESTED.** Structure, options, réversibilité, ordre de migration, rollback coordonné, observabilité et frontière LAN sans authentification sont cohérents et restent dans le choix utilisateur (`ADR-0011:14-35`). Le routeur unique date/v1 et les preuves contractuelles ont désormais une base solide.

La conséquence normative « réponses tierces [qui] tolèrent les champs additionnels » (`:33`) n'est toutefois pas réalisée récursivement par le contrat exact. Comme cette promesse porte directement sur la stratégie de compatibilité de la version datée, l'ADR doit rester `PROPOSED` jusqu'à correction du finding MEDIUM et nouvelle revue du nouveau SHA. L'auteur renseignera ensuite reviewer, commit/UTC et lien du rapport ; le reviewer ne modifie ni ADR ni index.

## Disposition

Ne pas promouvoir ni déployer `50bbdf800a6c46c67ddd3cce74d8358286cd8eb6`. Produire un nouveau commit limité à l'extensibilité récursive, au défaut OpenAPI et à l'état documentaire BUG-0048, relancer les validations contrat/fixtures, puis demander une revue delta indépendante. Les revues test et sécurité restent des gates séparés.
