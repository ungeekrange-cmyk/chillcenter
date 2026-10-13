# ENABLER-007 — Intégrer, distribuer et vérifier la version complète

- Type : ENABLER ; aucune user story technique déguisée.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.
- Estimation : 5 points ; Packaging natif, intégration croisée et preuves finales ; ne remplace pas les tests de chaque story.
- Depends On : US-001, US-002, US-003, US-004, US-005, US-006, US-007, US-008, US-009, US-010, US-011, US-012, US-013, US-014, US-015, US-016, US-017, US-018, US-019, US-020, US-021, US-022, US-023, US-024, US-025, US-026, US-027, US-028, US-029, US-030, US-031, US-032, US-033, ENABLER-006.
- Blocks : aucun.
- Trace : F-01, F-02, F-03, F-04 ; décisions ADR-0001, ADR-0004, ADR-0005, ADR-0006 ; vérifications V-01, V-02, V-03, V-04, V-05, V-06, V-07, V-08, V-09, V-10, V-11, V-12.
- Auteur : /root ; gate G-03 en attente, travail non commencé.

## Preuves de sortie requises

- ENABLER-007-EXIT-01 : Versions stables compatibles verrouillées, build Linux ARM64 et bundle Mac autonome avec dépendances et licences contrôlées.
- ENABLER-007-EXIT-02 : Tests ciblés/régressions du code et UI adaptés de Cinéfolio ; parcours réels Mac/Pi/NAS, Range/TLS/pistes, trois profils, reprise/transfert/import/apparence.
- ENABLER-007-EXIT-03 : Logs bornés sans secrets/prompts/chemins absolus/noms de profils ; préservation des sources et NAS ; snapshots et rollback documentés.
- ENABLER-007-EXIT-04 : Artefacts liés au SHA exact, commandes/cibles/résultats/limites, smoke après installation autorisée ; pas de cible de déploiement supposée.
- ENABLER-007-EXIT-05 : Présentation de l'implémentation et de ses preuves à l'utilisateur pour validation finale de l'étape 4 ; une feature non vérifiée est déclarée telle.

## Contraintes

Les interfaces approuvées autorisent fixtures et tests locaux pour préparer l'intégration. Elles ne prouvent pas le serveur déployé, la lecture native ou le Wi-Fi réel. Enregistrer environnement, SHA, commandes, résultats et limites ; pas de modification de Cinéfolio, des vidéos ou d'identités/credentials supposés. Toute preuve non exécutée reste NOT_RUN.

