# BUG-0080 — Clipping pendant le défilement du catalogue

**OPEN — signalé, non corrigé dans ce lot.** 16 octobre 2026, client Mac, version exacte et dimensions de fenêtre à relever.

L’utilisateur observe des découpes visuelles au scroll dans « Toute la collection » ; extension aux autres vues suspectée mais **non confirmée**. Attendu : vignettes et contenu stables, sans coupure ni espace intempestif pendant le défilement.

Cause inconnue : ne pas attribuer sans preuve le défaut au fenêtrage du catalogue, au sticky header ou aux couches compositées. À qualifier sur un catalogue assez long, plusieurs largeurs, scroll rapide/lent et retour de fiche ; conserver la position et ne pas remplacer la liste par une pagination. Aucune régression vidéo à rejouer pour cette investigation.
