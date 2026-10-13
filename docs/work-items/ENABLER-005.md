# ENABLER-005 — Coordonner lecture, progression et transfert

- Type : ENABLER ; aucune user story technique déguisée.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.
- Estimation : 5 points ; Machine d'états et événements retardés nécessitent une validation de concurrence ciblée.
- Depends On : ENABLER-002, ENABLER-003, ENABLER-004.
- Blocks : US-003, US-004, US-016, US-021, US-023.
- Trace : F-06, F-07, F-08, F-12, F-13 ; décisions ADR-0003 ; vérifications V-04, V-05, V-06, V-12.
- Auteur : /root ; gate G-03 en attente, travail non commencé.

## Preuves de sortie requises

- ENABLER-005-EXIT-01 : Bail lié profil, média/révision, appareil, session/génération, séquence et epoch serveur ; 30 s, renouvellement 5 s, nouvelle vidéo = nouvelle génération.
- ENABLER-005-EXIT-02 : TRANSFERRING gèle deadline, refuse renouvellement ancien et accepte un seul ACK final lié au média avant échéance ; arbitrage atomique des transferts concurrents.
- ENABLER-005-EXIT-03 : Révocation des flux, échéance conservatrice client même avec buffer, suspension/réveil, pause/stop et restart serveur.
- ENABLER-005-EXIT-04 : Progression périodique 5 s et transitions ; cumuler temps réel dans dernier dixième, exclure seek/pause/buffering, réinitialiser compteur lors de non vu manuel.
- ENABLER-005-EXIT-05 : Tests déterministes des courses et horloges, puis vraie interruption entre deux clients ; pas de preuve d'attention humaine ni garantie DRM.

## Contraintes

Les interfaces approuvées autorisent fixtures et tests locaux pour préparer l'intégration. Elles ne prouvent pas le serveur déployé, la lecture native ou le Wi-Fi réel. Enregistrer environnement, SHA, commandes, résultats et limites ; pas de modification de Cinéfolio, des vidéos ou d'identités/credentials supposés. Toute preuve non exécutée reste NOT_RUN.

