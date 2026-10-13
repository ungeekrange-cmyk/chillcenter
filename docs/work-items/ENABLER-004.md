# ENABLER-004 — Préparer le modèle partagé et personnel

- Type : ENABLER ; aucune user story technique déguisée.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.
- Estimation : 5 points ; Séparation des anciennes données Media/collections et identité stable traversent plusieurs modules.
- Depends On : aucune.
- Blocks : US-002, US-006, US-027, ENABLER-002, ENABLER-005, MIG-001, MIG-002.
- Trace : F-05, F-06, F-14, F-16, F-17 ; décisions ADR-0002, ADR-0006 ; vérifications V-07, V-08, V-10, V-12.
- Auteur : /root ; gate G-03 en attente, travail non commencé.

## Preuves de sortie requises

- ENABLER-004-EXIT-01 : UUID stables et localisations séparées, séries/sagas communes, notes/reprises/playlists/préférences explicitement liées au profil.
- ENABLER-004-EXIT-02 : Accès SQLite borné hors workers IO, FK, WAL local, transactions brèves et révisions de concurrence.
- ENABLER-004-EXIT-03 : Catalogue paginé et filtré en base, tailles de messages/champs/images/import conformes aux contrats approuvés.
- ENABLER-004-EXIT-04 : Règles de renumérotation et suppression préservent profils et fichiers ; tests d'isolation et conflits.
- ENABLER-004-EXIT-05 : Importer/exporter ne restaure jamais des lectures actives ; toute nouvelle migration dispose d'un schéma et d'une procédure de rollback.

## Contraintes

Les interfaces approuvées autorisent fixtures et tests locaux pour préparer l'intégration. Elles ne prouvent pas le serveur déployé, la lecture native ou le Wi-Fi réel. Enregistrer environnement, SHA, commandes, résultats et limites ; pas de modification de Cinéfolio, des vidéos ou d'identités/credentials supposés. Toute preuve non exécutée reste NOT_RUN.

