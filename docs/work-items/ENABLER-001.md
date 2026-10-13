# ENABLER-001 — Prouver le lecteur Mac et le flux sécurisé

- Type : ENABLER ; aucune user story technique déguisée.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.
- Estimation : 8 points ; Rendu natif, FFI instable et annulation justifient huit points ; enabler volontairement risqué, à traiter tôt.
- Depends On : aucune.
- Blocks : US-016.
- Trace : F-02, F-03, F-09, F-19, F-22 ; décisions ADR-0004, ADR-0005 ; vérifications V-03, V-09.
- Auteur : /root ; gate G-03 en attente, travail non commencé.

## Preuves de sortie requises

- ENABLER-001-EXIT-01 : Sur un Mac Apple Silicon, prototype libmpv embarqué avec rendu natif, pistes intégrées/externes et flux HTTPS de fixture utilisant la même politique TLS que l'API.
- ENABLER-001-EXIT-02 : Vérifier version stable du moteur/ABI et client API ≥1.106 ; read/seek hors UI, cancel concurrent et destruction sans deadlock ni fuite.
- ENABLER-001-EXIT-03 : Mesurer mémoire et décodage matériel/logiciel ; tester les formats/codecs observés plutôt que toutes les possibilités du conteneur.
- ENABLER-001-EXIT-04 : Pas de fonction catalogue complète requise pour cette preuve ; une fixture HTTPS contrôlée ne prouve ni serveur ARM64 ni débit NAS.
- ENABLER-001-EXIT-05 : Si la preuve échoue, ouvrir un BUG si reproductible et demander un ADR de remplacement avant d'étendre l'intégration ; aucune désactivation TLS ou transcodage serveur comme contournement.

## Contraintes

Les interfaces approuvées autorisent fixtures et tests locaux pour préparer l'intégration. Elles ne prouvent pas le serveur déployé, la lecture native ou le Wi-Fi réel. Enregistrer environnement, SHA, commandes, résultats et limites ; pas de modification de Cinéfolio, des vidéos ou d'identités/credentials supposés. Toute preuve non exécutée reste NOT_RUN.

