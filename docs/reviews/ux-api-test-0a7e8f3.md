# Revue indépendante tests / intégration — CC-UX-002 et API 2026-10-14

- **Review Mode** : `INTEGRATION_REVIEW`
- **Branche et commit** : `codex/cinefolio-ui-rework` / `0a7e8f300d0e1f1b0723cf91d6aade15412027f5`
- **Base** : `5665f60bc59ceb3b2a9caa1622af053af0645d47`
- **Integration manager / auteurs** : `/root`, `/root/native_player_builder` et auteurs documentaires associés
- **Reviewer Instance** : `/root/ux_api_test_review`, distinct des auteurs, du test author et de l'integration manager
- **Environnement** : macOS 27.0.1 (26A434), arm64 ; rustc 1.99.0, cargo 1.99.0 ; Node 24.19.0 ; Python 3.12.14 ; OpenAPI validators 0.9.0
- **Verdict** : **CHANGES_REQUESTED**

Le candidat reste bloqué avant bundle/promotion : la nouvelle exécution Tauri native sur fixture vierge échoue au hit-test de l'overlay lecteur. Une exécution précédente du même SHA était en outre restée active plus de dix minutes sans résultat, avec le thread principal observé dans la lecture de framebuffer de diagnostic. L'ancien PASS natif antérieur au gel final ne convertit ni le timeout ni le nouvel échec en réussite.

## Reproducible Evidence

| Commande / session | Sortie | Durée | Réussis | Échoués | Ignorés | Retries / quarantaine | Preuve |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `cargo test --workspace --all-targets --locked --offline --quiet` avec `CARGO_HOME=/private/tmp/video-catalog-cargo`, `RUSTUP_HOME=/private/tmp/video-catalog-rustup`, `CARGO_TARGET_DIR=/private/tmp/chillcenter-root-target`, `CARGO_INCREMENTAL=0`, `CARGO_BUILD_JOBS=1` | 0 | 38,41 s | 289 | 0 | 0 | aucune | Exécution indépendante exacte 0a7e8f3 : 69+32+51+17+97+2+21 |
| `cargo test -p chill-server --lib --locked --offline --quiet`, même environnement | 0 | 30,07 s | 97 | 0 | 0 | aucune | Recompile et vérifie notamment l'OpenAPI embarquée par `include_str!` |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | 0 | 4,59 s | N/A | 0 warning | N/A | aucune | Exact 0a7e8f3 |
| `node --test ui/connected/*.test.js ui/connected/tests/*.test.js` | 0 | 0,31 s | 40 | 0 | 0 | aucune | Exact 0a7e8f3 |
| `node scripts/validate-api-contract.mjs` | 0 | 0,17 s | 66 chemins / 85 opérations inspectées | 0 | 0 | aucune | Routes/méthodes, refs, paramètres, requêtes strictes, graphes de réponses extensibles |
| `python -c '... validate_spec(...) ...'` avec `openapi-spec-validator==0.9.0` | 0 | 0,50 s | 1 document OpenAPI 3.1 | 0 | 0 | aucune | `OPENAPI_FORMAL_OK` |
| `node scripts/smoke-api-contract.mjs https://127.0.0.1:18548/api/2026-10-14 ... --synthetic-fixture-consent /private/tmp/chillcenter-ux-api-qa.mSH19h/responses-r3-final.json` (session `2eb5bf`) | 0 | non fournie | 30 JSON + Range 206/32 octets | 0 | 0 | aucune | Serveur reconstruit au SHA exact, fixture privée ; OpenAPI HTTPS SHA-256 `d12c606e4d922c1a52f0fe6f12f72940b1498c442dce7ba43ac9f83f946778f4`, identique au fichier source |
| `python scripts/validate-api-responses.py /private/tmp/chillcenter-ux-api-qa.mSH19h/responses-r3-final.json` | 0 | 0,13 s lors de la relecture indépendante | 38 payloads, 30 réponses HTTPS | 0 | 0 | aucune | Validation OAS 3.1 exacte sur la capture finale |
| Tauri réel, `CHILL_UI_QA_MODE=shell`, serveur loopback 18548, données `shell-client-r2`, CLI IA synthétique locale (session `70741`) | 0 | non fournie | 1 parcours critique | 0 | 0 | aucune | `UI_QA_RESULT true native-Tauri-shell-sidebar-search-admin-logo-wide-detail-series-drilldown-saga-playlist-picker-TMDB-secret-advice-filters` |
| Tauri réel, `CHILL_UI_QA_MODE=player`, SHA exact, fixture partagée (session `52212`, PID 33137) | timeout / arrêté pour diagnostic | > 10 min | 0 | 1 | 0 | aucun PASS par relance | stdout vide ; état atteint la lecture ; sample `/private/tmp/chillcenter-player-hang-33137.txt` : thread principal dans `SurfacePlayer::snapshot -> Framebuffer::nonblank -> glReadPixels -> Metal wait` |
| Premier lancement de contrôle vierge (session `89154`) | 1 | non fournie | 0 | setup | 0 | non compté comme défaut candidat | Serveur loopback non démarré à cause du confinement ; consentement impossible, incident d'environnement conservé |
| Tauri réel, `CHILL_UI_QA_MODE=player`, serveur escaladé 18549, DB/client vierges, uniquement `Fixture (2026).mp4` + SRT (session `22914`) | 1 | non fournie | 0 | 1 | 0 | aucune | `UI_QA_RESULT false native overlay hit-test: overlay DOM hit-test failed` |

Le worktree était propre au SHA exact avant et après les vérifications. Les fixtures sont sous `/private/tmp`; aucune donnée du foyer, du Pi ou du NAS n'a été mutée. Les messages FFmpeg/libmpv lors des tests Rust n'ont produit ni échec ni skip.

## Matrice critères / risques / preuves

| Source | Critère / risque | Couche et oracle | Résultat | Qualité de l'assertion / preuve |
| :--- | :--- | :--- | :--- | :--- |
| CC-UX-002 | Sidebar Cinéfolio, recherche haute, logo seul 96 px, actions seulement Administration, grande fiche | Tauri réel shell | **PASS** | Vérifications DOM sur la vraie webview, puis IPC Rust et HTTPS local ; pas une simple maquette |
| CC-UX-002 | Conseil lecture seule + filtres vu/non vu et film/série, invalidation de contexte | Node + Tauri shell avec CLI synthétique | **PASS_SCOPED** | Commandes et absence d'application vérifiées ; fournisseur IA réel non appelé |
| US-007 / UX | Catalogue sans pagination visible, viewport borné à 3 000 | Node composant + Tauri shell 4 médias | **PASS_SCOPED** | Calcul de fenêtre < 50 cartes et absence de contrôle de pagination vérifiés ; le chargeur principal multi-batch n'est pas exercé avec >48 éléments (finding TEST-F-002) |
| US-008/011/014 | Compteurs séries, drilldown, création saga et playlist | Tauri réel shell | **PASS** | 2 saisons/2 épisodes/non-vus, navigation fiche, saga deux films et playlist avec épisode réellement traversés |
| F-09/US-016 | Rendu natif, overlay cliquable, lecture immédiate, fullscreen, pistes, navigation, volume, EOF/vu | Tauri réel player | **FAIL** | Le run exact et vierge échoue avant les assertions suivantes sur `elementFromPoint`; un run exact antérieur s'est bloqué sans résultat |
| API-2026-10-14 | Base datée, alias v1, header, erreurs JSON et métadonnées protocolaires | Rust intégration + HTTPS | **PASS** | Deux prefixes partagent données ; 404/405/422, request_id, Allow/Retry-After/Content-Range/HEAD vérifiés |
| API tiers | OpenAPI 3.1 valide, 66 chemins/85 opérations, requêtes strictes, réponses extensibles | Validation structurelle/formelle + HTTPS/schema | **PASS_SCOPED** | 30 opérations représentatives exécutées via HTTPS et 38 payloads validés ; les 85 opérations ne sont pas toutes E2E |
| TMDB sécurité/ressources | 2 tâches, worklist 3 000, état 2 Mio, historique 8/15 min, annulation/retry/restart | Rust avec SQLite réelle et provider privé injectable | **PASS** | Cas 3001 explicite/implicite/auto, saturation, persistance/pruning, échec d'écriture et absence secret/export vérifiés ; fournisseur TMDB réel non appelé |
| Qualité build | Workspace et lint | Rust exact SHA | **PASS** | 289 tests, zéro échec/ignoré ; clippy `-D warnings` |
| Cible physique | Pi/NAS, Wi-Fi, 2/3 Full HD, codecs/HDR, son, veille, RSS | Déploiement / NFR | **NOT_RUN** | L'ancien serveur Pi R7 n'est pas ce candidat ; les limites de REL-PI-001 restent applicables |

## Évaluation de la qualité

- Les assertions Rust TMDB ne sont pas des tautologies : elles traversent la SQLite réelle, la persistance privée, les transitions de worker et les limites 3 001/2 Mio/2 tâches. Le provider réseau est volontairement remplacé par une seam privée ; cela ne prouve pas TMDB réel.
- Les tests OpenAPI combinent validation formelle, inspection exhaustive routes/méthodes et validation JSON Schema de vraies réponses HTTPS. Le sous-ensemble HTTP est représentatif mais non exhaustif : 55 opérations ne possèdent pas de capture de succès E2E.
- Les tests Node couvrent états tardifs, filtres, compte à rebours, commandes du lecteur et virtualisation. Ils ne remplacent pas la frontière AppKit, précisément celle qui échoue.
- Le shell Tauri est une preuve intégrée reproductible sur fixture isolée. L'IA et la clé TMDB y sont synthétiques et ne déclenchent aucun coût ni appel fournisseur.
- Aucun pourcentage de couverture n'est fourni et aucun seuil projet n'est défini ; le verdict ne s'appuie pas sur un pourcentage.
- Aucun échec critique n'est masqué, ignoré ou requalifié par retry. Les deux exécutions player défavorables sont conservées.

## Findings et routage

| ID | Type | Evidence | Action requise | Route |
| :--- | :--- | :--- | :--- | :--- |
| `TEST-F-001` | **Échec critique / flakiness native reproductible** | Session 52212 sans résultat (>10 min, sample Metal/glReadPixels) puis session 22914 sur DB/client/MP4 vierges : `overlay DOM hit-test failed` | Geler le second symptôme et l'environnement ; diagnostiquer une hypothèse falsifiable à la fois (ordre AppKit, rectangle DOM, overlay masqué ou oracle), sans confondre le hang framebuffer et le hit-test avant preuve commune. Ajouter/conserver la régression causale, puis obtenir un run Tauri player vierge complet et déterministe au nouveau SHA : image non vide, hit-test/z-order, auto-hide/focus, pistes, seek, vitesse, volume, fullscreen/safe-area/Escape, 10 s finales, EOF et reprise. Une relance verte seule ne suffit pas à effacer les échecs. | `agent-debugger`, puis builder distinct et nouvelle triple revue exacte |
| `TEST-F-002` | **Couverture manquante** | `catalog-window.test.js` teste 3 000 positions et le picker teste 103 éléments, mais aucun test ne traverse le `loadPage()` principal sur plusieurs lots ; le shell n'a que 4 médias | Ajouter un test composant du chargeur catalogue réel avec au moins 3 lots (offsets 0/48/96), sans trou/doublon, compte final, abandon d'un contexte périmé et DOM borné. Ne pas remplacer ce test par celui du picker, qui exerce une autre fonction. | Builder / test author ; le reviewer ne modifie pas les tests |

## Limites à conserver après correction

- Appels fournisseurs TMDB et IA réels : `NOT_RUN`.
- Catalogue principal réel de 2 496/3 000 éléments dans la webview et mesure mémoire : `NOT_RUN`.
- Déploiement exact sur Pi, lecture NAS, capacité 2/3 flux Full HD et comparaison Wi-Fi/Ethernet : `NOT_RUN` avant release verification.
- Son humain, veille/réveil, multi-écran, HDR/HEVC et inspection Finder/Dock : `NOT_RUN`.
- Le bundle Mac n'est pas construit/promu dans cette revue ; sa signature, ses 48 dylibs arm64 et son chargement doivent être vérifiés sur l'artefact issu du prochain SHA approuvé.

## Disposition

Ne pas promouvoir, reconstruire comme remise finale ni déployer `0a7e8f300d0e1f1b0723cf91d6aade15412027f5`. Le serveur/API, le shell et les suites automatisées sont favorables, mais le lecteur — parcours central — n'est pas reproductiblement vert. Après correction causale et test multi-batch, geler un nouveau SHA et refaire au minimum les tests ciblés, le workspace/clippy, le shell/player Tauri propres et les trois revues indépendantes exactes.
