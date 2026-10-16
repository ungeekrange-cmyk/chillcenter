# BUG-0087 — État vu à EOF non actualisé et cas des vidéos courtes

16 octobre 2026. **PARTIALLY_CORRECTED_LOCAL — règle courte en attente de choix humain.**

Signalement : premier épisode d'« Avez-vous déjà vu » non marqué vu. L'utilisateur confirme une durée inférieure à 1 min 40 ; fichier/durée exacts non sondés.

Deux mécanismes distincts :

- La règle approuvée reste dix secondes réelles lues dans les derniers 10 %. À vitesse ×1, une vidéo de moins de 100s ne peut pas atteindre ce crédit en une seule lecture. Ce n'est pas une preuve d'échec serveur. Une adaptation a été proposée à l'utilisateur, non appliquée sans validation.
- Déficit d'actualisation confirmé indépendamment : le rapport EOF peut enregistrer `watched` au serveur, mais sa réponse personnelle est ignorée par le pont lecteur ; le miroir local n'est pas actualisé par les simples polls de statut. Les anciennes preuves avec un stop/rechargement explicite ne couvrent pas cet EOF naturel.

Correction client : synchronisation delta détachée une seule fois par EOF confirmé (rapport/libération/résolution réussis), sans verrou global ni attente du compte à rebours. Au retour, la fiche est relue après cette synchronisation. Gardes de serveur/profil et navigation contre les retours tardifs. Pas de commande/API nouvelle, aucune modification du serveur/web ni du seuil.

Preuves : revue causale indépendante de `player_bridge::report/tick`, `UiClient::prepare_catalog_sync`, déclencheur personnel serveur et `syncAfterMutation`. Tests `player.dom.test.js` : une synchronisation malgré plusieurs rendus EOF, aucun stop si vidéo suivante, retour non périmé. DOM connecté : retour avec compteurs du miroir mis à jour (pont synthétique). Les tests Rust existants du crédit restent inchangés ; pas de nouveau visionnage Pi/NAS.

Limites : si le rapport réussit mais la résolution suivante échoue (`next` absent), cet EOF ne confirme pas la complétion et ne déclenche pas cette actualisation. Hors connexion, pas de nouvelle garantie de marquage. La règle courte et une validation humaine de l'état vu réel restent ouvertes.
