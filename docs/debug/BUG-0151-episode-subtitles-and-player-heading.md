# BUG-0151 — Sous-titres et titre du lecteur entre épisodes

26 novembre 2026 · Lecteurs web et Mac

À l’enchaînement, les deux lecteurs recréaient la session et chargeaient les
préférences générales du profil. Le choix manuel de la piste ou de l’état
désactivé disparaissait. Le plein écran affichait seulement le titre de la
vidéo, sans indice de saison ni d’épisode.

La lecture suivante d’une même série transporte le choix manuel. La piste
est recherchée par langue et titre, avec repli sur une correspondance de
langue unique, y compris pour les fichiers externes. Si aucune piste
équivalente n’est trouvée, les préférences du profil s’appliquent. L’état
« Désactivés » est repris explicitement. Le titre en plein écran utilise les
numéros du catalogue et affiche le seul nom de la série si un numéro manque.

Les tests de régression couvrent le changement d’identifiant de piste, la
désactivation au passage à l’épisode suivant et le titre en plein écran.
