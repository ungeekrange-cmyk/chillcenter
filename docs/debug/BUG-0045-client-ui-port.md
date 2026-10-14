# BUG-0045 — écarts de portage UI Cinéfolio

2026-10-14. **ROOT_CAUSE_CONFIRMED statiquement** pour composition/fullscreen/absence de configuration ; démarrage GUI exact **non reproduit**. Source `8e83922982c8efc9a4ea7b3dd31e8cc1c998c4a4`, client `624f244`, baseline Cinéfolio `90c149b`. Aucun correctif applicatif, accès au Pi/NAS/profils réels ou lancement de l'app personnelle dans ce diagnostic.

## Attendu / réel / preuves

- Sidebar/recherche Cinéfolio non portées, catégories en select et pages visibles : `ui/connected/index.html`, `catalog.js`, `styles.css`.
- Fiche trop petite : `dialog max-width:520px` général toujours actif malgré largeur catalogue 850px ; fiche film sans affiche : `styles.css`, `catalog.css`, `catalog.js`.
- IA existe mais cachée dans `.admin-grid`, après profil connecté : `ai.js:6–21`. CLI Mac, aperçu et confirmation présents.
- TMDB existe par fiche ; credential uniquement `CHILL_TMDB_TOKEN` serveur, aucun setter UI ; scan sans TMDB : `tmdb_api.rs:34–53`, `catalog_api.rs:750–824`, `scanner.rs`. Pas de panne réseau supposée.
- Contrôles frères du rectangle vidéo en flux normal, aucun auto-hide : `index.html:10`, `player.css`, `player.js`.
- Fullscreen fenêtre seulement ; rectangle vidéo limité à `min(55vh,560px)` : `player_bridge.rs:802–807`, `native_player.rs:492–525`, `player.css`.
- Second clic structurel réfuté : unpause automatique après chargement/heartbeat, mais découverte/téléchargement sous-titres attendu avant unpause : `player_bridge.rs:434–505` ; timeout transport20s. Chemin de délai prouvé, symptôme GUI pas reproduit.

## Hypothèses / expériences indépendantes

`/root/player_ui_diagnosis`, lecture seule : H1 overlay absent CONFIRMED ; H1 fullscreen contraint CONFIRMED ; H1 second clic requis REFUTED ; H2 séquence sous-titres avant unpause CONFIRMED statiquement. Quatre hypothèses, arrêt après cause, zéro patch spéculatif. `node --test ui/connected/player.dom.test.js` : 8/8 PASS, enchaînement uniquement. GUI native NOT_RUN.

`/root/tmdb_ui_diagnosis`, lecture seule : traçage API/IPC/UI et comparaison Cinéfolio, aucun provider/credential/appel live. Rapport source ne constitue pas une revue approbative de la correction.

## Correction / régression / rollback

[CC-UX-001](../design/DESIGN_SYSTEM.md) attend validation visuelle. Builder distinct pour correctifs causaux. Maintenir Rust/libmpv, vidéo serveur HTTPS, client sans NAS et aucun transcodage Pi. Prouver overlay au-dessus de NSOpenGLView, géométrie fullscreen, démarrage avec sous-titres lents et tous les garde-fous baux/TLS. Tester catalogue continu/mémoire, accessibilité IA/confirmation, TMDB setter secret sans fuite + automatisme exact-match borné.

Maquette Playwright PASS, **simulation seulement**, ne ferme pas le bug. Commit de correction et preuve native non disponibles. Rollback actuel : aucun runtime changé ; bundle `Chill Center-624f244.app` et serveur `126da0a/R7` inchangés. Documents ignorables sans action sur NAS/données.
