# BUG-0058 — relire un épisode terminé repart à EOF

14octobre2026, fixture Tauri/HTTPS locale isolée, base ae0ce44 + candidat cache. ROOT_CAUSE_CONFIRMED puis FIXED. Première tentative native masquée : timeout150s, preuve d'outillage seulement. Nouvelle fenêtre visible : proposition20s/nouvelle saison et clic vignette PASS, mais relire le premier épisode puis chercher donne PLAYER_ERROR.

H1 unique confirmée : état SQLite de la fixture position16000ms pour durée16s ; native apply_pending envoyait sans contrôle cette position à seek_absolute après FileLoaded. Fin immédiate avant la navigation suivante. Aucun état foyer/NAS consulté ou modifié.

RED : test `a_completed_position_restarts_but_partial_playback_keeps_its_resume` avec ancien calcul renvoie16 au lieu de0. Correction : au FileLoaded, durée réelle connue, une position dans les250dernières millisecondes ou au-delà repart à0. Positions partielles, durée inconnue et validation NaN conservées. Marquage vu inchangé : ce correctif ne transforme pas un seek en visionnage.

GREEN et parcours EOF/clic/annulation/auto à consigner dans les preuves du lot avant VERIFIED. Retour ancien bundle possible ; aucune migration serveur liée à ce défaut. ADR0013, BUG0057.
