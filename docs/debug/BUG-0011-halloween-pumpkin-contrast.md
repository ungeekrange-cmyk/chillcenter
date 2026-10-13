# BUG-0011 — Citrouille Halloween sombre peu lisible

2026-10-13 · VERIFIED · auteur /root · base7766a46d46fdad7fe844b62049dff12697043ac7.

Signalement utilisateur confirmé sur le SVG/PNG : corps identique au disque ocre et tige aubergine identique au dossier. H1 : palette du motif, pas défaut de géométrie. Expérience : lecture des couleurs du générateur et inspection du rendu navigateur ; hypothèse confirmée.

Correction : orange vif distinct pour le corps, tige claire et contour fin ; nervures sombres conservées. Aucun déplacement du canapé ou du motif ; paths communs identiques. Générateur Rust + export Chromium sans réseau. Quatre tests du générateur PASS, dont régression sur les couleurs de tige/corps ; rendu1254px inspecté. Ancien PNG préservé dans icons/branding/raster-r5 ; source/hashes avant/après dans le manifeste r6. Rollback : version du générateur/SVG/PNG de la base, dans une branche isolée. Pas d'icône runtime ou code applicatif modifié par cette retouche.
