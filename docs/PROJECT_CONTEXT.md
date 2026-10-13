# Contexte du projet

- Révision documentaire : `CC-DOC-001`, 2026-10-13, auteur `/root`.
- Produit : Chill Center ; dépôt cible `https://github.com/ungeekrange-cmyk/chillcenter.git`.
- Source : `/Users/ghillaumesoubigou/Documents/Dev/Perso/Video catalog/`, Cinéfolio 0.5.0.
- Baseline historique vérifiée : `90c149b797b45cba8a56dca2b74eed06c4225060` dans `Cinefolio-source.bundle`. Les fichiers de travail restent distincts d'un checkout Git : leur égalité complète au commit n'est pas revendiquée.
- Destination confirmée : `/Users/ghillaumesoubigou/Documents/Dev/Perso/ChillCenter/`. Preuve : « erreur de ma part si j'ai dit ChilleCenter, c'est ChillCenter ». Aucun dossier existant n'a été renommé.
- Étape 1 : `APPROVED_BY_USER`, périmètre `CC-FONC-002`. Preuve : « vise directement un serveur sur linux, on va zapper la version mac, le reste ok ». Cette correction retire le serveur macOS ; le client Mac Apple Silicon validé reste prévu.
- Étape 2 : `APPROVED_BY_USER`. Preuve : « oui étape suivante », consignée dans `docs/gates/G-02-CC-DOC-001.md` ; sept ADR et contrats acceptés sans modification de conception.
- Étape 3 : `APPROVED_BY_USER`, dossier `CC-STORIES-001` ; preuve « validé, go implémentation » dans G-03 après revue indépendante APPROVED r2.
- Étape 4 : `IN_PROGRESS`. Lot interface Mac/serveur HTTPS implémenté et revu à `c2a9d72d634cc7282eb8a20ca9fa138ed92e2fec`, 124 tests et trois parcours GUI ; pas de remise finale ni validation G-04 demandée. Voir `docs/implementation/STATUS.md` avant reprise.

## Contraintes

Serveur Linux uniquement, matériel de référence Raspberry Pi 3 B (éventuellement B+), 1 Go de RAM ; NAS sur réseau 1 Gb/s, Pi probablement en Wi-Fi. Deux à trois profils simultanés, surtout 1080p ou moins. Client Mac Apple Silicon d'abord, web plus tard. Usage local au foyer, sans authentification. Maximum de logique en Rust, aucun transcodage serveur, stockage vidéo lu uniquement par le serveur.

## État des capacités

Lecture des sources et archive Git vérifiées ; documentation primaire consultée le 2026-10-13. Accès GitHub initial par connecteur : 404, puis CLI autorisée : succès, dépôt vide à l'intake. Implémentation sur branches isolées `codex/`, source Cinéfolio préservée. Lot actuel : 124 tests, revues indépendantes code/test/sécurité r3 approuvées pour son SHA, vraie webview/HTTPS loopback et paquet de développement arm64 vérifiés ; détails dans INTERFACE-EVIDENCE. Aucun push distant, accès NAS ou déploiement Pi effectué. Pi déclaré non prêt, IP prévue `192.168.1.42` et montage `/srv/volumes/video` : exemples configurables, jamais des constantes imposées.

Correction UI utilisateur du 2026-10-13 (capture de la barre Cinéfolio) : scan réservé à Administration. Implémenté dans `ui/connected` ; le bouton Actualiser du catalogue ne fait aucun scan. Aucun nouvel examen de périmètre nécessaire.

Les anciens `docs/PROJECT_CONTEXT.md` et `scripts/agent-context.py` ne sont pas présents dans le dossier Cinéfolio déplacé. Le présent fichier devient le point d'entrée durable du nouveau projet.

## Prochaine action autorisée

### Branding — choix du 2026-10-13

L'utilisateur retient comme logo par défaut le **premier des deux derniers rendus**, avant la seconde translation vers le bas : image source `exec-df820cb4-e6c6-4ce6-8266-d301ab7fe79d.png`, copie `icons/branding/default.png`. Preuve : « oui on garde celui-ci par défaut » puis « quand je dis celui-ci, le premier des deux ». Douze propositions statiques pour printemps/été/automne/hiver/Noël/Halloween × clair/sombre sont dans `icons/branding/`, avec galerie, hashes et prompts. Leur validation visuelle et raccordement runtime restent distincts de cette génération ; ne pas prétendre que l'application change déjà automatiquement de logo.

### Implémentation

Poursuivre l'implémentation autorisée : setup/profils/admin/catalogue Tauri raccordés ; intégrer maintenant surface lecteur Tauri, routes vidéo/sous-titres, baux/progression/enchaînement puis autres stories. Demander G-04 uniquement à la remise complète. Ne pas recommencer discovery/architecture/stories ni affirmer que 124 tests couvrent 133 AC ou le Pi/Wi-Fi. Les LOW et fonctionnalités non terminées restent listés dans STATUS ; ce checkpoint n'annonce pas de travail en arrière-plan.
