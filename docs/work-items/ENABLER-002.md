# ENABLER-002 — Préparer le service Linux et sa distribution

- Type : ENABLER ; aucune user story technique déguisée.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.
- Estimation : 3 points ; Réutilisation du cœur Rust ; nouvelle installation Linux mais périmètre fondation restreint.
- Depends On : ENABLER-004.
- Blocks : US-001, ENABLER-003, ENABLER-005, ENABLER-006.
- Trace : F-01, F-03, F-18, F-19 ; décisions ADR-0001, ADR-0002, ADR-0005 ; vérifications V-01, V-09, V-12.
- Auteur : /root ; gate G-03 en attente, travail non commencé.

## Preuves de sortie requises

- ENABLER-002-EXIT-01 : Binaire serveur Linux ARM64 séparé de Tauri/macOS, version Rust stable et dépendances épinglées, configuration explicite et unité systemd non privilégiée.
- ENABLER-002-EXIT-02 : SQLite/cache/certificats sur disque local ; montage NAS Linux lecture seule, credentials hors API et archives.
- ENABLER-002-EXIT-03 : HTTPS avec création initiale du certificat, validité dix-huit mois calendaires, renouvellement annuel même clé, contrôle boot/quotidien, remplacement atomique.
- ENABLER-002-EXIT-04 : Contrôles de noms, permissions des clés, Host/Origin/corps et erreurs bornées ; endpoints de santé distinguent service et NAS.
- ENABLER-002-EXIT-05 : Build et smoke Linux reproductibles ; accès au Pi, montage et horloge réels restent à obtenir avant preuve de livraison matérielle.

## Contraintes

Les interfaces approuvées autorisent fixtures et tests locaux pour préparer l'intégration. Elles ne prouvent pas le serveur déployé, la lecture native ou le Wi-Fi réel. Enregistrer environnement, SHA, commandes, résultats et limites ; pas de modification de Cinéfolio, des vidéos ou d'identités/credentials supposés. Toute preuve non exécutée reste NOT_RUN.

