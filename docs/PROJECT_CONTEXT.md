# Contexte du projet

- Révision documentaire : `CC-DOC-001`, 2026-10-13, auteur `/root`.
- Produit : Chill Center ; dépôt cible `https://github.com/ungeekrange-cmyk/chillcenter.git`.
- Source : `/Users/ghillaumesoubigou/Documents/Dev/Perso/Video catalog/`, Cinéfolio 0.5.0.
- Baseline historique vérifiée : `90c149b797b45cba8a56dca2b74eed06c4225060` dans `Cinefolio-source.bundle`. Les fichiers de travail restent distincts d'un checkout Git : leur égalité complète au commit n'est pas revendiquée.
- Destination confirmée : `/Users/ghillaumesoubigou/Documents/Dev/Perso/ChillCenter/`. Preuve : « erreur de ma part si j'ai dit ChilleCenter, c'est ChillCenter ». Aucun dossier existant n'a été renommé.
- Étape 1 : `APPROVED_BY_USER`, périmètre `CC-FONC-002`. Preuve : « vise directement un serveur sur linux, on va zapper la version mac, le reste ok ». Cette correction retire le serveur macOS ; le client Mac Apple Silicon validé reste prévu.
- Étape 2 : `APPROVED_BY_USER`. Preuve : « oui étape suivante », consignée dans `docs/gates/G-02-CC-DOC-001.md` ; sept ADR et contrats acceptés sans modification de conception.
- Étape 3 : `APPROVED_BY_USER`, dossier `CC-STORIES-001` ; preuve « validé, go implémentation » dans G-03 après revue indépendante APPROVED r2.
- Étape 4 : `IN_PROGRESS`. Fondations Rust et service configurable implémentés ; pas de remise finale, ni de validation G-04 demandée. Voir `docs/implementation/STATUS.md` avant toute reprise.

## Contraintes

Serveur Linux uniquement, matériel de référence Raspberry Pi 3 B (éventuellement B+), 1 Go de RAM ; NAS sur réseau 1 Gb/s, Pi probablement en Wi-Fi. Deux à trois profils simultanés, surtout 1080p ou moins. Client Mac Apple Silicon d'abord, web plus tard. Usage local au foyer, sans authentification. Maximum de logique en Rust, aucun transcodage serveur, stockage vidéo lu uniquement par le serveur.

## État des capacités

Lecture des sources et de l'archive Git : exécutée. Documentation primaire web : consultée le 2026-10-13. `git bundle verify` : réussi. Accès GitHub par connecteur lié : 404 ; `gh repo view` autorisé : succès, dépôt vide. Implémentation sur branches isolées `codex/`, source Cinéfolio préservée. 105 tests de crates passés, workspace fondation vérifié et smoke HTTPS loopback exécuté ; voir preuves exactes. Aucun push distant, accès NAS ou déploiement Pi effectué. Le Pi déclaré non prêt sera prévu à `192.168.1.42`, montage `/srv/volumes/video` ; ces valeurs sont des exemples configurables, comme corrigé explicitement par l'utilisateur.

Les anciens `docs/PROJECT_CONTEXT.md` et `scripts/agent-context.py` ne sont pas présents dans le dossier Cinéfolio déplacé. Le présent fichier devient le point d'entrée durable du nouveau projet.

## Prochaine action autorisée

Poursuivre l'implémentation déjà autorisée : rendu AppKit/FBO isolé prouvé, maintenant intégrer Tauri/serveur/lecteur et compléter les stories/tests. Demander G-04 à la remise réelle, pas au présent checkpoint. Ne pas recommencer discovery/architecture/stories. Ne pas affirmer que 108 tests locaux couvrent les 133 AC ni les conditions Pi/Wi-Fi. Les limites/findings de revue non résolus bloquent toute remise finale, pas le travail indépendant sûr.
