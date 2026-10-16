# BUG-0065 — Fiche et image d’un autre média

16 octobre 2026 : comportement de nouveau signalé par l’utilisateur ; suivi **OPEN** dans [BUG-0079](BUG-0079-unmatched-poster-regression.md). Les preuves historiques ci-dessous sont conservées, pas présentées comme validation de cette réapparition.

Auteur `/root`, 2026-10-15 ; client livré c610ea9.

Causes confirmées : lecture ignorée par `run` occupé, réponses sans génération de fiche, file d’images vérifiant le profil mais pas l’identité de son élément réutilisé.

Correction : vidage immédiat des champs/actions obsolètes, génération fiche/contexte/identité, jeton par liaison d’image contrôlé avant/après téléchargement ; actualisation de la fiche ouverte après publication du miroir. Une réponse périmée ne rouvre pas une fiche fermée/remplacée. Cinq étoiles immédiates, fermeture extérieure et position de catalogue conservée.

RED Node : A retardé remplaçait B, zéro étoile dédiée. GREEN : tests de cycle et navigateur synthétique import/matching sans fermeture, B sans ancienne affiche, clic extérieur et défilement conservé. Pas de nouvelle lecture vidéo. Livraison/Tauri/Pi de ce lot encore à vérifier.
