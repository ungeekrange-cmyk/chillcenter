# Validation visuelle CC-UX-002

- ID/type : G-UI-PROJECT-CC-UX-002, UI projet.
- Date d'enregistrement : 2026-10-14 13:59:58 UTC, auteur `agent_instance_id=/root`.
- Statut **PENDING** pour r2 ; décision : maquette mise à jour suivant les corrections humaines, pas encore approuvée après présentation.
- Branche `codex/cinefolio-ui-rework`, base `386dbc43d54ac4f35b4d68755393abe247bdf34c` ; [spec](../design/DESIGN_SYSTEM.md), [prototype](../design/prototype.html), révision exacte CC-UX-002. Source native/Pi non modifiée.
- Scope : actions Recharger/Compléter/Organiser uniquement Administration ; sidebar Demande de conseil sans mutation avec filtres Tous/Vus/Non vus et Films et séries/Films/Séries ; nom/slogan retirés à côté du logo, logo agrandi ; reste du design conservé.
- Evidence humaine : « Oui bien, mais la partie recharger, compléter les fiches et organiser avec l'IA doit être dans administration. Dans la partie gauche, Organiser avec l'IA est à remplacer par demande de conseil, qui a pour but de proposer des vidéos correspondant à tes envies. » Puis « et la mention votre bibliothèque ensemble, tu retires. Pareil pour le chill center à côté du logo. Par contre logo plus grand ».
- Approbateur après présentation r2 / date UTC de cette approbation : non reçus. Ne pas confondre accord de principe r1 et validation de la maquette r2.
- Critères exécutés : checks Playwright de maquette PASS, rendu clair/sombre inspecté par l'auteur. Pas de revue indépendante, vidéo/IA réelle NOT_RUN.
- Ajout ultérieur avant validation r2 : « dans la demande de conseil, on peut faire des filtres aussi (vu/non vu), films séries ». Maquette et checks adaptés : AND des deux critères, suppression des anciennes suggestions, état vide et absence d'appel IA implicite. Ce message demande une modification, ne valide pas toute la révision.
- Retour suivant : afficher nombre de saisons/épisodes/non vus et navigation dans saisons/épisodes ; enlever Disponible, garder Indisponible ; création et ajout de vidéos dans sagas et listes. Parcours de démonstration ajoutés à r2. Checks Playwright PASS : compteurs et mise à jour vu/non vu, navigation, indisponible avec Lire désactivé, création+ajout sans doublon, saga commune/liste personnelle ; inspection visuelle par l'auteur. Pas d'approbation humaine nouvelle inférée, PENDING inchangé.
- Remplace [r1](G-UI-PROJECT-CC-UX-001.md) pour placement/branding uniquement ; aucun changement d'ADR accepté, ni invalidation rétroactive des preuves runtime inchangées.
- Next : validation visuelle r2 puis refonte applicative par builder distinct des diagnostics. Expiry/invalidation : changement substantiel du design/révision ; pas une autorisation de contourner TLS, baux, secret TMDB ou de muter le NAS.
