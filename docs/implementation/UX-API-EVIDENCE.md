# Intégration CC-UX-002 et API 2026-10-14 — preuves en cours

2026-10-14, auteur/coordinatrice `/root`, branche isolée `codex/cinefolio-ui-rework`, base `5665f60bc59ceb3b2a9caa1622af053af0645d47`. Validation humaine de la maquette enregistrée dans G-UI-PROJECT-CC-UX-002. Ce checkpoint n'est ni une remise finale ni une approbation indépendante.

## Commandes exécutées

- `cargo test --workspace --all-targets --locked --offline --quiet`, toolchain/cache privés, CARGO_INCREMENTAL=0, jobs=1 : PASS282 (69catalogue+32client+51domain+17player+90server+2CLI+21desktop), aucun échec/ignoré, session46069exit0. Première tentative confinée session31241 : deux sockets loopback refusés dans tests anti-références libmpv ; relance autorisée réussie sans assouplir TLS ni supprimer les tests. Les avertissements VideoToolbox de la première tentative ne prouvent pas le décodage matériel sur tous les Macs.
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` : session71576exit0.
- Node `--test ui/connected/*.test.js ui/connected/tests/*.test.js` :37PASS, dont conseil filtré/lecture seule, invalidation résultats au changement de profil, viewport catalogue3000borné, sélection défilante multi-batch sans trous, commandes lecture et autoplay. Premier test ancien sélecteur paginé remplacé pour exercer le nouveau sélecteur continu, pas retiré sans remplacement.
- `node scripts/validate-api-contract.mjs` : PASS66chemins/85opérations. Vérifie routes/méthodes déclarées, refs et paramètres, absence de corps/réponse objet vide hors autodescription ; pas une preuve de conformité exhaustive de toutes réponses réelles.

## Vrai Tauri / HTTPS local

Serveur propre loopback18548, données `/private/tmp/chillcenter-ux-api-qa.mSH19h/server`, clients privés séparés. Quatre vidéos et sous-titres synthétiques copiés depuis la fixture antérieure, aucun accès NAS. `curl --cacert .../tls/certificate.pem .../api/2026-10-14/server` PASS version/capabilities, sans `-k`. Le document OpenAPI embarqué devra être reconstruit après finalisation de son contenu ; succès du header ne suffit pas à prouver son contenu.

`CHILL_UI_QA_MODE=shell`, URLloopback, DATA=shell-client, ROOT=videos et CLI=`scripts/fixtures/ai-cli.sh` : session7745exit0, `UI_QA_RESULT true native-Tauri-shell-sidebar-search-admin-logo-wide-detail-series-drilldown-saga-playlist-picker-TMDB-secret-advice-filters`. Parcours réelle webview→IPC fermé→Rust→HTTPS : placement Administration, logo96, recherche globale, fiche>850px, absence Disponible, stats2saisons/2épisodes/2nonvus et navigation, création saga+deuxfilms, playlist+épisode, saisie/effacement clé **fictive** TMDBsansécho, conseil **CLI synthétique sans réseau** et invalidation après filtre. N'est pas une vérification d'un fournisseur TMDB ou IA réel, ni une mesure de mémoire d'un catalogue3000 dans la webview.

`CHILL_UI_QA_MODE=player` : overlay AppKit hit-test/z-order, image non vide, disparition2,5s/réapparition/focus, pistes conservées après pause/reload, navigation/vitesses/volume exécutés avant échec du contrôle plein écran. Échec conservé dans [BUG0046](../debug/BUG-0046-native-fullscreen-geometry.md). Aucun PASSglobal annoncé avant correction de l'oracle ou de la cause démontrée.

## Pi / remise

14octobre, lecture seule SSH après réouverture par utilisateur pour24h : masterPID16081, servicePID3016active/NRestarts0, ancien binaire9a17ffe…36a6, NASCIFSRO/montageinchangé,9,5Gio disponibles. Aucun déploiement de ce candidat à ce checkpoint. Pas de nouvelle mesure physique Wi-Fi/deuxDune/3flux : réserves des preuves REL-PI-001 restent valables.

Prochaines étapes : gel exact SHA, revues indépendantes code/tests/sécurité/ADR, revalidation Tauri lecteur, bundle signé ad-hoc ARM64 et mise à jour coordonnée serveur daté avant activation client. Préserver ancienne application et binaire ; pas de reset/push/renommage NAS. Retour serveur et client coordonné, aucun changement SQLite destructif nécessaire. Les fournisseurs réels, son humain, veille physique, HDR/HEVC/RSSmax et capacité3flux restent NOT_RUN tant que non exécutés.

## Correctifs après le candidat258 — 14 octobre

Les revues [code/ADR258](../reviews/ux-api-code-258db0a.md) et [sécurité258](../reviews/ux-api-security-258db0a.md) restent CHANGES_REQUESTED historiques, pas requalifiées en approbation. Correction : vrai contrat DTO/headers/params/nullabilité/defaults/extensibilité, worker TMDB deux réservations jusqu'à sa fin réelle (annulation n'ouvre pas un troisième slot), historique8/15min à prochain accès, worklist3000/état2Mio/lecture bornée et pruning durable, refus429/413. Tests ciblés26TMDB et clippy par auteur distinct du reviewer. Aucun RSS réel revendiqué ; ancien+nouveau fichier peuvent brièvement coexister, chacun≤2Mio.

Suite globale corrigée : `cargo test --workspace --all-targets --locked --offline --quiet` session9708exit0, **289PASS** (69+32+51+17+97+2+21), zéro ignoré. Clippyworkspace/alltargets-Dwarnings session36376exit0. Node40PASS, dont trois régressions formulaireTMDB, saturation explicite et arrêt du polling sur job expiré. Les ajustements documentaires des defaults de préférences sont testés par les validations de schéma et recompilation finale, pas une modification de règle Rust.

Plein écran BUG0046 : vrai Tauri session44318exit0, trace frame/bounds1710×1073, safe1710×1073,5, physique1710×1107, AppKit hit-test/z-order vrais ; résultat complet `native-image-AppKit-z-order-hit-test-overlay-idle-focus-volume-fullscreen-geometry-Escape-tracks-resume-watched-EOF` PASS. Correction d'oracle debug seulement, pas de vidéo sousencoche ni simulation de géométrie. Source de plateforme [Apple](https://developer.apple.com/documentation/appkit/nsscreen/safeareainsets). Son humain et multi-écran physiques restent NOT_RUN.

OpenAPI : formel `openapi-spec-validator0.9.0` (venvPython3.12.14privé `/private/tmp/chillcenter-openapi-validator-20261014`) PASS. Versions validationJSON : openapi-schema-validator0.9.0/jsonschema4.26.0 ; aucune dépendance produit ou Cargo.lock modifiée. `scripts/smoke-api-contract.mjs ... --synthetic-fixture-consent` sessionccdf90exit0 : **30réponsesJSONréelles + Range206/32octets** ; `scripts/validate-api-responses.py` sessionbfc498exit0 : **38payloadsrequête/réponse OAS31PASS**. Comprend suppression profil avec UUIDheader, révisions/collections, AIcontext, sériesmetadata null/counters, baux/progression/release avec entiers64bits exacts. Pas85opérations E2E, pas TMDBprovider réel, pas mesuredebitNAS.

Pi préflightRO via nouveau master utilisateur24h : un profil du foyer (nom non lu),2496vidéos, aucune connexion8443établie observée (ss headerseul), binaireR7PID3016active/NRestarts0, montagero/confinementinchangés. Ce profil ne doit jamais être supprimé par les QA. Mise à jour uniquement après nouveau triple-review exactSHA, binaire immuable et sauvegarde/rollback ; pas encore déployée ici.
