# Chill Center v0.5 — Mac Apple Silicon

16 octobre 2026. Demande/approbation : « validé. Lance la version et on la fige en v0.5 ». Périmètre : filtres genres/années et ambiances proposés dans CC-CONCEPT-20261016 révision5. Version application **0.5.0**, API **2026-10-14** inchangée. Statut initial : candidat intégré, revue indépendante et bundle en cours ; aucune livraison déclarée avant preuves finales ci-dessous.

## Inclus / exclus

- Genres disponibles en sélection multiple OU, combinés en ET avec la période et les autres filtres ; années disponibles décroissantes, une année exacte/deux bornes inclusives/troisième année recommence, clic sur une sélection la retire, réinitialisation. Résultats immédiats depuis le catalogue local déjà chargé, aucun endpoint ou paramètre API nouveau.
- Cache local de la base filtrable réutilisé entre clics, invalidé après changement de contexte/mutation/delta ; conservation de la publication atomique et des gardes de réponses tardives. Grille virtualisée inchangée.
- Décors photographiques rapprochés approuvés embarqués dans le client : sapin peu chargé et citrouille au premier plan droit ; recadrage ancré à droite. Noël rouge franc/rouge profond, Halloween abricot/brun-orangé, neige/chauves-souris bornées derrière la navigation. Désactivation via ambiance off, réduction système des animations, pause masqué/lecture. Aucun téléchargement décoratif ni bibliothèque nouvelle.
- Versions Cargo du desktop/Tauri/Info.plist synchronisées en0.5.0 (build5), versions des crates partagées et de l'API conservées. Ni serveur/Pi/NAS, ni chantier web/PWA, ni certificats/configuration/états personnels modifiés. Aucun tag distant/push implicite.

## Preuves initiales

- `node --test ui/connected/filters.test.js ui/connected/filters.dom.test.js ui/connected/catalog-loading.dom.test.js ui/connected/sync.dom.test.js ui/connected/tests/catalog-query.test.js` : **17/17 PASS**.
- `CHILL_PREVIEW_NODE_MODULES=… node scripts/test-filters-v05.cjs` : **PASS**, vraie interface connectée avec pont synthétique ; ouvertures/période/OU/ET/focus/Escape, aucun IPC catalogue après chargement chaud, invalidation puis relecture. Aucun serveur réel dans cette preuve.
- `CHILL_PREVIEW_NODE_MODULES=… CHILL_V05_EVIDENCE=… node scripts/test-appearance-v05.cjs` : **PASS**, quatre palettes/décors ancrés, limites des particules, arrêt réduit/masqué/lecture/off, layout760–1440px. Captures inspectées, pas une preuve WebKit native.
- `CARGO_HOME=/private/tmp/video-catalog-cargo CARGO_TARGET_DIR=/private/tmp/chillcenter-series-target.ZXNxst cargo check --locked --offline -p chillcenter-desktop` : **PASS**, Rust1.99.0, aucune dépendance modifiée.
- Suite Node globale : **74/76 PASS**, deux assertions textuelles du QA lecteur déjà en échec sur la base livrée f3fdac5. Reproduites à l'identique dans une extraction immuable de cette base (`/private/tmp/chillcenter-v05-baseline.vbLZ2Q`), trois tests dont1PASS/2FAIL ; aucun changement aux sources/tests QA ou lecteur. Pas présentées comme réussite complète ou comme régression v0.5. Pas de nouvelle lecture vidéo réelle lancée.
- Diff nul contre f3fdac5 sur serveur, API, web et sources lecteur natives. Ces vérifications n'actualisent pas l'état réel du Pi depuis sa dernière livraison.

## Livraison / gel

Delta de revue avant gel : la paire de tokens génériques accent/bouton de la barre Noël clair est rendue cohérente (`--button:#701019`). La première attribution à l'avatar était erronée : ce dernier est dans le panneau droit et n'hérite pas de ces tokens locaux ; aucun défaut visible de l'avatar n'est revendiqué. Tests de contraste calculé ajoutés pour la paire locale et l'avatar réel, sur les quatre combinaisons. Le bundle candidat cef6 reste non livré, remplacé par un build au nouveau SHA.

Source finale, revue indépendante, chemin du bundle, versions/signature/hash, tag local et lancement à compléter après exécution réelle. Ne pas remplacer un ancien bundle ni utiliser la maquette comme application. Repli : relancer `Chill Center-f3fdac5.app`, conservé ; aucune migration de données ou changement serveur dans ce lot.

Après lancement, restent à vérifier humainement dans Tauri : apparence/filtres sur la bibliothèque du foyer, rendu et fluidité du décor sur Mac, consommation réelle ; pas de nouveau benchmark Pi/NAS nécessaire pour ce delta client seul. Le fonctionnement physique Pi/NAS/Wi-Fi et les réserves de la release précédente ne sont pas requalifiés par ces tests locaux.
