# BUG-0093 — Fond blanc du lecteur entre deux épisodes

**FIXED_LOCAL — traitement demandé ensuite le16octobre, test Tauri humain restant.** H1 confirmée : surface native fermée à EOF, découpes DOM transparentes et aucune couche de transition ; le fond clair apparaît autour de la proposition. Un calque dédié prépare une seule affiche courante via le cache existant dans les huit dernières secondes, reste transparent pendant la lecture et devient visible uniquement à EOF. Affiche floutée/assombrie ; repli dégradé sombre si absente ou retardée. Aucun préchargement vidéo, nouvelle route ou calcul serveur ; invalidation via les gardes d'images existantes au changement de média/profil/navigation/fermeture.32tests Node du contrôleur et vrai DOM connecté : anticipation, absence de peinture avant EOF, nettoyage/repli, clair/sombre, fenêtre/plein écran, redimensionnement PASS ; non-régression du décor événementiel PASS. Aucun lecteur réel ou accès Pi/NAS lancé.

16 octobre 2026. **OPEN — référencement uniquement, traitement explicitement différé par l'utilisateur.**

## Signalement et preuve

Client Mac, proposition suivante d'une série ; version du processus non relevée, dernière remise df2870f dans ce chat. Capture utilisateur `codex-clipboard-f4225cca-5306-46a0-bcc1-20d5ce250691.png` : carte « À suivre » sombre entourée d'une grande zone blanche avec dégradé gris en bas. L'utilisateur décrit l'écran du lecteur blanc pendant cette transition. Cause et durée non diagnostiquées.

## Attendu demandé

Pendant la proposition et la transition, conserver un fond dans les tons de la série, par exemple une image de la série floutée. **Anticiper la préparation du décor quelques secondes avant la fin de la vidéo**, pour éviter l'apparition du fond blanc ; durée précise à déterminer lors du traitement. Ne pas faire apparaître le décor par-dessus la vidéo encore en lecture (régression BUG0078 à éviter), ni perturber le reste de l'application.

Préparation visuelle seulement : aucune demande de préchargement complet de la vidéo suivante, transcodage serveur ou accès NAS direct depuis le client. Prévoir un repli sobre et cohérent si l'image manque ou tarde, jamais l'affiche de la série précédemment consultée.

## Suivi

Aucun code, CSS, asset ou réglage modifié ; aucune hypothèse testée ni nouveau test exécuté. Vérification future : fin naturelle et passage suivant avec/sans image, retard de chargement, modes clair/sombre, redimensionnement et plein écran ; absence de flash blanc et de décor devant l'image vidéo. Liés : BUG0091/0092, BUG0078/0079. Aucun nouveau bundle ou déploiement.
