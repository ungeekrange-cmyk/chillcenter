# BUG-0092 — Message d'annulation erroné à l'expiration du compte à rebours

16 octobre 2026. **OPEN — référencement uniquement, traitement explicitement différé par l'utilisateur.**

## Signalement et reproduction rapportée

Client Mac, enchaînement d'une série ; version installée non relevée, dernière remise df2870f dans ce chat. Laisser se terminer le compte à rebours de la proposition suivante : le message « vidéo suivante annulée » apparaît, puis l'épisode suivant démarre quand même. Observation dynamique rapportée par l'utilisateur ; la capture jointe montre la proposition à5s, pas le message ni la transition. Fréquence, durée du message et réglage exact du délai non relevés.

## Attendu

À l'expiration sans intervention : indiquer un lancement, jamais une annulation, puis lancer une seule fois le bon épisode. Après une annulation réelle : ne pas lancer plus tard et appliquer le retour à la fiche prévu. Aucun résultat périmé ne doit remplacer l'état de la lecture suivante.

## Suivi

Cause inconnue, aucun diagnostic ou test exécuté, aucune correction. Vérifications futures ciblées : expiration naturelle, « Lancer maintenant », « Ne pas lancer », clic proche de zéro et retour tardif ; cohérence du message et du nombre de lancements. Périmètre observé : séries ; sagas/listes non vérifiées, ne pas leur attribuer ce défaut sans preuve. Liés : BUG0088 (ordre API corrigé), BUG0091/0093 (présentation). Le démarrage effectif rapporté n'est pas une validation de tous les cas d'enchaînement ou du marquage vu.
