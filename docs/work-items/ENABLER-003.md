# ENABLER-003 — Relier le client Mac au serveur en HTTPS

- Type : ENABLER ; aucune user story technique déguisée.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.
- Estimation : 5 points ; Deux plateformes, certificat personnalisé et flux de fichiers demandent tests de compatibilité.
- Depends On : ENABLER-002.
- Blocks : US-001, US-016, US-030, ENABLER-005.
- Trace : F-02, F-03, F-19 ; décisions ADR-0001, ADR-0004, ADR-0005 ; vérifications V-03, V-09.
- Auteur : /root ; gate G-03 en attente, travail non commencé.

## Preuves de sortie requises

- ENABLER-003-EXIT-01 : Adaptateur Rust de contrats API versionnés, magasin de serveurs et pins SPKI, adresses DNS/IP et persistance locale distincte par serveur.
- ENABLER-003-EXIT-02 : Politique TLS commune API/vidéo/sous-titres ; SAN, période, signatures, ancien pin avant/après expiration et remplacement d'urgence couverts.
- ENABLER-003-EXIT-03 : Transport Range annulable, 206/416/ETag et changement de contenu ; données indexées seulement, aucun chemin arbitraire.
- ENABLER-003-EXIT-04 : Pont Tauri limité aux capacités utiles, messages français, aucun runtime Chromium ni exécutable serveur configurable depuis le client.
- ENABLER-003-EXIT-05 : Preuves fixture client/serveur réel local à distinguer des preuves Mac/Pi/NAS de l'intégration finale.

## Contraintes

Les interfaces approuvées autorisent fixtures et tests locaux pour préparer l'intégration. Elles ne prouvent pas le serveur déployé, la lecture native ou le Wi-Fi réel. Enregistrer environnement, SHA, commandes, résultats et limites ; pas de modification de Cinéfolio, des vidéos ou d'identités/credentials supposés. Toute preuve non exécutée reste NOT_RUN.

