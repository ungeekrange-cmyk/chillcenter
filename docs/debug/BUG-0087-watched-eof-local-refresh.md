# BUG-0087 — État vu à EOF non actualisé et cas des vidéos courtes

16 octobre 2026. **CORRECTED_LOCAL — exception courte approuvée, serveur Pi non mis à jour.**

Addendum16octobre14:44UTC : règle courte incluse dans le serveur df2870f désormais déployé et vérifié indépendamment ([rapport](../releases/REL-NEXT-POSTERS-SCROLL-MATCHING-001-verification.md)). Aucun nouveau visionnage ni état vu fabriqué pour le test : le marquage court réel reste à confirmer humainement. Les mentions non-déployé plus bas sont les checkpoints antérieurs.

Signalement : premier épisode d'« Avez-vous déjà vu » non marqué vu. L'utilisateur confirme une durée inférieure à 1 min 40 ; fichier/durée exacts non sondés.

Deux mécanismes distincts :

- La règle initiale impose dix secondes réelles lues dans les derniers 10 %. À vitesse ×1, une vidéo de moins de 100s ne peut pas atteindre ce crédit en une seule lecture. Ce n'est pas une preuve d'échec serveur. L'utilisateur a ensuite explicitement approuvé « Adapter la règle aux vidéos courtes ».
- Déficit d'actualisation confirmé indépendamment : le rapport EOF peut enregistrer `watched` au serveur, mais sa réponse personnelle est ignorée par le pont lecteur ; le miroir local n'est pas actualisé par les simples polls de statut. Les anciennes preuves avec un stop/rechargement explicite ne couvrent pas cet EOF naturel.

Correction client : synchronisation delta détachée une seule fois par EOF confirmé (rapport/libération/résolution réussis), sans verrou global ni attente du compte à rebours. Au retour, la fiche est relue après cette synchronisation. Gardes de serveur/profil et navigation contre les retours tardifs. Annuler réutilise la synchronisation de cet EOF (revue initiale défavorable conservée).

Correction serveur approuvée : durée <100s, fin atteinte après couverture continue du dernier dixième, suivie dans le bail actif avec séquences et temps monotone. Pas de nouveau champ/protocole ni migration ; pas de crédit artificiel. Pause immobile conservée, saut/buffering avec avance/durée modifiée/retour arrière invalident la couverture. Nouveau bail = preuve remise à zéro. Règle dix secondes cumulées inchangée à partir de 100s. Cas natif EOF en lecture et web EOF en pause pris en compte ; sources web non modifiées.

Preuves : revue causale indépendante de `player_bridge::report/tick`, `UiClient::prepare_catalog_sync`, déclencheur personnel serveur et `syncAfterMutation`. Tests `player.dom.test.js` : une synchronisation malgré plusieurs rendus EOF, aucun stop si vidéo suivante, retour non périmé. DOM connecté : retour avec compteurs du miroir mis à jour (pont synthétique). Tests Rust : seuil long/manual reset conservés ; segment court aux sept vitesses, pause/seek/buffering et durées de 1ms à 99,999s ; HTTP sur les deux bases API avec EOF natif et EOF web en pause, sortie progress/release et isolation des profils. Pas de nouveau visionnage Pi/NAS.

Revue défavorable `dfd41f6` conservée : une simple continuité avec la tolérance de position +1000ms pouvait créditer le dernier dixième d'un clip de 1s après 1ms. Correction : couverture bornée par le temps monotone réel converti en temps média **et** par la distance effectivement parcourue, aucune seconde de tolérance ajoutée. Les fractions de millisecondes sont conservées, tolérance de calcul limitée à 1ns. Régressions : saut d'un coup et dix petits sauts sans temps suffisant restent non vus ; attendre immobile à EOF n'ajoute rien.

Limites : si le rapport réussit mais la résolution suivante échoue (`next` absent), cet EOF ne confirme pas la complétion et ne déclenche pas cette actualisation. Hors connexion, pas de nouvelle garantie de marquage. Validation humaine Tauri et installation du nouveau serveur nécessaires pour annoncer la correction effective sur le Pi ; aucun déploiement effectué dans ce lot.
