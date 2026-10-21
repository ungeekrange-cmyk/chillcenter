# Revue code indépendante R2 — macOS 15 et reprises mesurées

- **Review Type / Reviewer Instance** : code / `/root/mac15_progress_code`
- **Parent Reviewed SHA / Verdict** : `82836674ad1c7d0f12b8744d8e26e71cff14ce30` / `CHANGES_REQUESTED`, rapport `docs/reviews/MACOS15-PROGRESS-code.md`
- **Base fonctionnelle** : `e8d6f53c6b9d2dedf4a89afc4851af2d001830f4`
- **New Source SHA** : `e54eb35726431bea2c01e6ad85d01e21da379de8`
- **Source Delta Scope** : `ui/connected/progress.js`, sa régression ; durcissement isolé de `macos_media_inputs.py` et sa régression
- **Current Verdict** : `APPROVED`

## Réévaluation du constat R1

Le blocage R1 est fermé. Sur `404/405`, `refreshProgress` vérifie toujours le contexte et la requête courante, puis vide `entries`, remet `revision` à `null`, supprime uniquement `chillcenter-measured-progress:<server>:<profile>` et redessine la vue. Une panne `503` conserve le dernier cache mesuré ; un nouvel essai forcé repart sans révision après ancien serveur et repopule le cache lorsque l'endpoint redevient disponible. Le reste de l'application n'est pas bloqué.

La nouvelle régression couvre les réponses positives, le maintien hors ligne `503`, la purge `404/405`, la disparition des barres/détails, le redraw de la rubrique et le repeuplement après mise à niveau. Elle ferme précisément le scénario absent de R1.

## Autre delta inspecté

Les alias dylib extraits doivent désormais se résoudre vers un fichier régulier portant le suffixe `.dylib`, directement sous le répertoire `lib/` de la formule épinglée. Un alias `libmpv.dylib` vers une notice de licence est refusé. Ce changement resserre la préparation des entrées sans modifier les octets officiels verrouillés, la résolution de dépendances, les minima Mach-O ou le flux de publication. Cette observation de code n'est pas une certification sécurité.

## Preuves et validité du parent

- Delta exact `8283667..e54eb357` : 4 fichiers, 36 insertions/2 suppressions ; `git diff --check` PASS.
- Reviewer : 37 tests Node ciblés PASS (`progress`, chargement catalogue DOM, bridge web) et 16 tests Python ciblés PASS (`macos_media_inputs`, minimum macOS).
- Les analyses et preuves R1 restent valides pour le reste de `e8d6f53..8283667` : persistance transactionnelle après Progress validé, endpoint groupé distinct du catalogue, garde de contexte, HTTPS native hors mutex, épisodes/autoplay série, resolver isolé et plafond Mach-O 15.0.
- Les preuves coordonnées précédentes sont réutilisées sans les étendre : suites Node/Python/domaine/serveur et smoke Chromium du build partagé. Le delta R2 ne touche ni serveur Rust, schéma, bridge natif/web, catalogue, autoplay, OpenAPI ni versions.

## Limites / suivi

- Tests physiques macOS 15 et 26 : `NOT_RUN` ; aucun Cargo, build, lancement natif, Pi/NAS, GitHub ou déploiement exécuté par ce reviewer.
- L'observation documentaire R1 sur la formulation ADR0034 (« dylibs/notices only » alors que les recettes `.brew/*.rb` sont conservées sans exécution pour provenance) reste non bloquante et à aligner lors de la revue formelle de l'ADR.
- L'approbation code vaut uniquement pour la source exacte `e54eb35726431bea2c01e6ad85d01e21da379de8`. Toute correction source ultérieure rend ce verdict historique et exige une nouvelle réévaluation exacte.
