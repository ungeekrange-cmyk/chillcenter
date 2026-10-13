# Contexte du projet

- Révision documentaire : `CC-DOC-001`, 2026-10-13, auteur `/root`.
- Produit : Chill Center ; dépôt cible `https://github.com/ungeekrange-cmyk/chillcenter.git`.
- Source : `/Users/ghillaumesoubigou/Documents/Dev/Perso/Video catalog/`, Cinéfolio 0.5.0.
- Baseline historique vérifiée : `90c149b797b45cba8a56dca2b74eed06c4225060` dans `Cinefolio-source.bundle`. Les fichiers de travail restent distincts d'un checkout Git : leur égalité complète au commit n'est pas revendiquée.
- Destination confirmée : `/Users/ghillaumesoubigou/Documents/Dev/Perso/ChillCenter/`. Preuve : « erreur de ma part si j'ai dit ChilleCenter, c'est ChillCenter ». Aucun dossier existant n'a été renommé.
- Étape 1 : `APPROVED_BY_USER`, périmètre `CC-FONC-002`. Preuve : « vise directement un serveur sur linux, on va zapper la version mac, le reste ok ». Cette correction retire le serveur macOS ; le client Mac Apple Silicon validé reste prévu.
- Étape 2 : `APPROVED_BY_USER`. Preuve : « oui étape suivante », consignée dans `docs/gates/G-02-CC-DOC-001.md` ; sept ADR et contrats acceptés sans modification de conception.
- Étape 3 : `REVIEW_REQUESTED`, dossier `CC-STORIES-001` ; validation utilisateur en attente après revue indépendante.
- Étape 4 : `NOT_STARTED`, dépend de la validation explicite des user stories.

## Contraintes

Serveur Linux uniquement, matériel de référence Raspberry Pi 3 B (éventuellement B+), 1 Go de RAM ; NAS sur réseau 1 Gb/s, Pi probablement en Wi-Fi. Deux à trois profils simultanés, surtout 1080p ou moins. Client Mac Apple Silicon d'abord, web plus tard. Usage local au foyer, sans authentification. Maximum de logique en Rust, aucun transcodage serveur, stockage vidéo lu uniquement par le serveur.

## État des capacités

Lecture des sources et de l'archive Git : exécutée. Documentation primaire web : consultée le 2026-10-13. `git bundle verify` : réussi. Accès GitHub par le connecteur lié : 404 ; accès par `gh repo view` après autorisation réseau : réussi, dépôt vide. Aucun push, clone du dépôt distant, changement de Cinéfolio, test applicatif, accès NAS ou déploiement effectué pour Chill Center.

Les anciens `docs/PROJECT_CONTEXT.md` et `scripts/agent-context.py` ne sont pas présents dans le dossier Cinéfolio déplacé. Le présent fichier devient le point d'entrée durable du nouveau projet.

## Prochaine action autorisée

Faire vérifier les user stories, les remettre à l'utilisateur et attendre sa validation de `CC-STORIES-001` avant l'implémentation. L'installation réelle du NAS, les preuves Pi/Wi-Fi, le packaging du lecteur et les dépendances appartiennent à l'étape 4. Son démarrage sera autorisé par validation de l'étape 3 ; sa propre validation intervient à la remise du résultat.
