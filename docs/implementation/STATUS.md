# État d'implémentation — ne pas confondre fondation et produit fini

2026-10-13 ; G-03 approuvé, G-04 non remis. Candidat source fondation : `3062ffec596c797bf4713433c34d5cf7bc54484f`. Les fichiers de documentation suivants ne changent pas ce SHA source. Les exigences restent celles de CC-STORIES-001, aucun périmètre réduit implicitement. Le workspace de fondation exclut explicitement `apps/desktop` jusqu'à son raccordement ; cela n'en fait pas une application terminée.

## Disponible dans le code

- Workspace Rust ; extraction des parseurs/catalogue/TMDB/affiches/IA Cinéfolio, 64 tests hérités exécutés.
- Store SQLite personnel : profils, préférences, états et notes, vitesses série, playlists, pagination SQL, réassociation explicitement confirmée.
- Service HTTPS : identité, profils CRUD, préférences et administration de la racine, config fichier, persistance anti-conflit et renouvellement calendaire à clé stable.
- Transport Mac : DNS/IP HTTPS éditable, consentement initial, pin SPKI, exception de clé après expiration connue, Range lié session/média/génération.
- Adaptateur libmpv : décodage client, callbacks de flux, contrôles/pistes/sous-titres, seek et annulation ; preuve headless avec un média généré. libmpv client API ≥2.0 requis pour l'ABI OpenGL à deux champs (0.41/API2.5 testé). Preuve réelle AppKit NSOpenGL/FBO avec vidéo générée, pause/seek et deux tailles réussie ; pas encore de surface Tauri.

## Pas terminé / pas prouvé

- L'interface Cinéfolio copiée n'est pas encore adaptée au nouveau transport ; `apps/desktop` est un squelette. Pas d'application `.app` utilisable livrée.
- Routes scan/catalogue/métadonnées/médias/sous-titres/affiches, arbitrage des baux/transferts et enchaînement : intégration serveur-client à faire. Une méthode Store ne prouve pas la validation du bail côté API.
- Export ZIP/affiches en streaming et migration Cinéfolio, plan complet de conflits d'import et restauration : à faire. Import mutatif volontairement désactivé plutôt que d'écraser silencieusement des données. JSON de compatibilité borné à 1 MiB, pas l'export final attendu.
- IA client avec aperçu/révisions et application serveur, ambiances/paramètres UI : à faire.
- Surface Tauri du lecteur, fullscreen/redimensionnement de la fenêtre, HTTPS vidéo réel de bout en bout, matrice MKV/MP4/AVI/pistes externes, bundle de dépendances : à prouver. Le rendu AppKit/FBO isolé est une preuve limitée, pas cette intégration.
- Build Linux ARM64/release/systemd, installation Pi, NAS/Wi-Fi/2–3 profils, budgets RSS/CPU/débit : NOT_RUN. Pi non prêt, pas d'accès SSH supposé.
- Revue sécurité du slice serveur APPROVED à `a06d95f`, avec un LOW ouvert (reconciliation mémoire/disque après fsync de répertoire en échec). Revues code/test des nouvelles corrections en cours ; une compilation n'est pas une approbation indépendante. Restent aussi erreurs API typées/CAS de renommage de profil, export de compatibilité borné seulement après allocation et preuves complètes de transport TLS. Aucun merge de livraison, push distant ou déploiement physique.

## Reprise

Revues bornées : test-execution APPROVED r2 à `3062ffe` (108 tests, check all-targets et probe AppKit locked) ; code delta worker APPROVED r3 au même SHA, historique HIGH clos ; sécurité serveur APPROVED r2 à `a06d95f`. **Aucune de ces approbations n'est celle de l'application entière ni des 133 AC.** Tous les rapports et leurs limites sont conservés dans `docs/reviews/`.

Checkpoint local enregistré dans le dossier ChillCenter et sa branche `codex/implementation`, origin GitHub configuré mais **pas de push**. `MANIFEST.sha256` décrit les fichiers suivis de ce checkpoint, hors lui-même ; génération mécanique par `bash scripts/update-manifest.sh` après ajout des fichiers suivis. Les manifestes historiques restent séparés. Ne pas annoncer une continuation en arrière-plan : reprendre le travail de l'étape 4 lors de la prochaine exécution autorisée.

Lire les rapports et le journal des bugs. Finir l'enabler de rendu Mac avant d'annoncer le lecteur intégré ; corriger les findings puis intégrer les routes et l'application sur une branche `codex/`. Ne demander G-04 qu'après couverture des AC et remise identifiable des artefacts. Conserver source Cinéfolio et vidéos intacts.
