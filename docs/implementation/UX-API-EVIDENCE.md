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
