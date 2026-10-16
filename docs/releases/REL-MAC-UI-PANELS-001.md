# REL-MAC-UI-PANELS-001 — lecteur et collections

2026-10-14. État : LOCAL_DELIVERED / USER_TEST_PENDING. Source compilée `574d2557e0a20f0cfbd23cfd99f546e28168613e`, branche de travail isolée `codex/ui-panels-player`, base utilisateur0d0da0e. Auteur et exécuteur des tests : `/root` ; aucune revue indépendante de ce nouveau delta revendiquée. Les approbations9277 restent historiques et ne couvrent pas ce client modifié.

## Résultat

[CC-UX-003](../design/CC-UX-003.md) : plein écran initial réglable parMac, mode fenêtre sur tout le panneau droit, flèche gauche/titre droit, commandes homogènes et fondu400ms. Pages sagas/listes/membres avec retour et `+`, sélecteur à affiches et filtres, sélection persistante ; révisions et contextes de lecture conservés. [ADR0012](../architecture/decisions/ADR-0012-client-player-and-collection-pages.md).

Bundle neuf : `artifacts/macos/Chill Center-574d255.app`. ARM64,75Mio,48dylibs autonomes dont libmpv/FFmpeg, signature ad-hoc deep/strict et fermeture du loader vérifiées. Exécutable SHA256 `f35a30f68f50b59e8cb469b5c54cfbbcbedda0a69349d3c502cf8e1c6ad4678e`. Copie publiée atomiquement sans écrasement, hash signé identique source/destination ; `--verify-bundle` charge libmpv API2.5 sans ouvrir AppSupport. Ancien bundle9277 intact, pas d'installation /Applications ou de lancement sur les réglages réels de l'utilisateur.

Le HTML ouvert via `file://.../ui/connected/index.html` ne peut pas se connecter : il nécessite le pont Rust Tauri. C'est un aperçu, **pas une application web livrée**. Aucun essai utilisateur n'est requis dans cette page. L'essai connecté se fait dans la `.app`, serveur `https://chillcenter.local:8443`.

## Preuves exécutées

- `cargo test --workspace --all-targets --locked --offline --quiet`, session5808exit0 : **290 tests PASS**, aucun ignoré (69+33+51+17+97+2+21). Première exécution sandbox refusait deux `TcpListener::bind(127.0.0.1:0)` ; la relance autorisée avec socketslocalhost passe, sans modifier de code pour contourner le test.
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`, session1444exit0 : PASS sur le commit final. Le buildrelease conserve deux warnings préexistants de helpers de diagnostic inutilisés (`nonblank`, `glReadPixels`), pas présentés comme un clippyrelease sans warning.
- `node --test ui/connected/*.test.js ui/connected/tests/*.test.js` : **55 tests PASS**,0ignoré, dont persistance locale, rollback d'erreur, ordre de navigation, baseline, pages, filtres, sélection, réponses périmées et virtualisation103titres sans trous0/48/96.
- `NODE_PATH=<runtime fourni>/node_modules node scripts/test-ui-panels.cjs` : PASS, navigateur isolé et pont **synthétique**. Pages/retour/plus, images réellement chargées (deux de saga, au moins cinq du sélecteur), catégories/vus/sélection et réglage local. Pas un serveur réel ou une preuve de décodage. Captures inspectées sous `/private/tmp/chillcenter-panels-qa.8RnBro/`, aucune donnée personnelle.
- Tauri réel, source574d255, nouveau serveurHTTPSlocal18623, nouvelle base/profil/client et MP4/SRT générés : session96836exit0. Plein écran par défaut, Échap puis rectangle couvrant toute la zone droite, flèche à gauche/titre à droite, fondu et inactivité/focus, image native non vide, hit-test/z-order AppKit, pistes/pause/reprise/seeks/vitesse/volume, plein écran natif frame1710×1073 et safe-area1073,5, dix secondes réelles finales/vu/EOF, préférence localefalse puis réouverture en panneau intégral. Dernière phase à25913ms ; pas de retry du parcours ou de saut d'assertion. Un état brièvement masqué pendant l'animationOS a repris normalement.
- Tauri réel, source574d255, nouveau serveurHTTPSlocal18624 et base/client distincts, quatre vidéos générées : session96095exit0. Sidebar/recherche/admin/logo/fiche, séries/saisons, création et ajout saga/liste, sélecteur à affiches, TMDB avec clé **fictive** ensuite retirée, conseil et filtres avec CLI **synthétique sans réseau**. Pas de fournisseur réel.
- Packaging `bash scripts/package-macos.sh <nouvelle.app>` session97570exit0, puis signature/hash/loader de la copie publiée PASS. Rust1.99 et Cargo.lock conservés, aucun téléchargement de dépendance.

## Défauts rencontrés

[BUG0052](../debug/BUG-0052-admin-navigation-order.md) : première session36308 échoue à l'ouverture Administration ; callbacks concurrents puis ordre de sélection de section corrigés, test RED/GREEN et parcours final neuf PASS. [BUG0053](../debug/BUG-0053-catalog-context-bootstrap.md) : baseline d'un boot déjà terminé, RED/GREEN. [BUG0054](../debug/BUG-0054-detached-poster-observers.md) : première carte sans image, assertion d'images réelles REDtimeout2500ms puis GREEN après report du nettoyage d'observation. Les assertions et échecs ne sont pas effacés ou remplacés par des succès supposés.

## Périmètre préservé et limites

Aucune commande SSH, écriture Pi, modification NAS, configuration serveur, certificat, scan foyer, profil ou donnée personnelle dans ce lot. Pi conserve le binaire1eb24… de la source9277 et l'API2026-10-14, inchangés. Les tests natifs utilisent uniquement des fichiers générés sousprivate/tmp. Les serveurs de test18620/21/22/23/24 et leurs applications sont arrêtés après les parcours ; fixtures conservées. MasterSSH utilisateur non touché.

Nouvelle lecture NAS, charge Wi-Fi2–3vidéos, son humain, veille/HDR/multi-écran, RSS maximal et fournisseurs réels non testés dans ce lot ; réserves précédentes inchangées. Pas de validation133ACtotale ou d'approbation visuelle de l'utilisateur inventée. Pas de push/main/reset ; le dossier utilisateur non suivi `.claude/` apparu dans la cible est préservé intact et exclu des commits.

Retour : fermer cette nouvelle application et rouvrir le bundle9277, sans changer le serveur. Le champ local nouveau peut être ignoré/perdu si l'ancien client réenregistre ses réglages ; le réappliquer dans Administration ensuite. Ni restauration SQLite ni suppression de vidéo nécessaires.
