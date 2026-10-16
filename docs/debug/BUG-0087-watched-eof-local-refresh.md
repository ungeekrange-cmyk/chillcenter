# BUG-0087 — État vu à EOF non actualisé et cas des vidéos courtes

16 octobre 2026. **CORRECTED_LOCAL — exception courte approuvée, serveur Pi non mis à jour.**

Signalement : premier épisode d'« Avez-vous déjà vu » non marqué vu. L'utilisateur confirme une durée inférieure à 1 min 40 ; fichier/durée exacts non sondés.

Deux mécanismes distincts :

- La règle initiale impose dix secondes réelles lues dans les derniers 10 %. À vitesse ×1, une vidéo de moins de 100s ne peut pas atteindre ce crédit en une seule lecture. Ce n'est pas une preuve d'échec serveur. L'utilisateur a ensuite explicitement approuvé « Adapter la règle aux vidéos courtes ».
- Déficit d'actualisation confirmé indépendamment : le rapport EOF peut enregistrer `watched` au serveur, mais sa réponse personnelle est ignorée par le pont lecteur ; le miroir local n'est pas actualisé par les simples polls de statut. Les anciennes preuves avec un stop/rechargement explicite ne couvrent pas cet EOF naturel.

Correction client : synchronisation delta détachée une seule fois par EOF confirmé (rapport/libération/résolution réussis), sans verrou global ni attente du compte à rebours. Au retour, la fiche est relue après cette synchronisation. Gardes de serveur/profil et navigation contre les retours tardifs. Annuler réutilise la synchronisation de cet EOF (revue initiale défavorable conservée).

Correction serveur approuvée : durée <100s, fin atteinte après couverture continue du dernier dixième, suivie dans le bail actif avec séquences et temps monotone. Pas de nouveau champ/protocole ni migration ; pas de crédit artificiel. Pause immobile conservée, saut/buffering avec avance/durée modifiée/retour arrière invalident la couverture. Nouveau bail = preuve remise à zéro. Règle dix secondes cumulées inchangée à partir de 100s. Cas natif EOF en lecture et web EOF en pause pris en compte ; sources web non modifiées.

Preuves : revue causale indépendante de `player_bridge::report/tick`, `UiClient::prepare_catalog_sync`, déclencheur personnel serveur et `syncAfterMutation`. Tests `player.dom.test.js` : une synchronisation malgré plusieurs rendus EOF, aucun stop si vidéo suivante, retour non périmé. DOM connecté : retour avec compteurs du miroir mis à jour (pont synthétique). Les tests Rust existants du crédit restent inchangés ; pas de nouveau visionnage Pi/NAS.

Limites : si le rapport réussit mais la résolution suivante échoue (`next` absent), cet EOF ne confirme pas la complétion et ne déclenche pas cette actualisation. Hors connexion, pas de nouvelle garantie de marquage. Validation humaine Tauri et installation du nouveau serveur nécessaires pour annoncer la correction effective sur le Pi ; aucun déploiement effectué dans ce lot.
