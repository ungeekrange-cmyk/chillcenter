# BUG-0080 — Clipping pendant le défilement du catalogue

**FIXED_LOCAL — mécanismes corrigés, validation humaine Tauri restante.** 16 octobre 2026, client Mac ; signalement réitéré après le bundle94495c1.

L’utilisateur observe des découpes visuelles au scroll dans « Toute la collection » ; extension aux autres vues suspectée mais **non confirmée**. Attendu : vignettes et contenu stables, sans coupure ni espace intempestif pendant le défilement.

Cause confirmée dans le chemin de rendu : chaque passage de ligne virtuelle détruisait aussi les cartes/images encore visibles, les réobservait et relançait leur récupération sérielle. Sortir de l'intersection supprimait encore l'image. Un écart fractionnaire entre hauteur virtuelle arrondie et vraie ligne CSS ajoutait un décalage.

Correction : conserver les mêmes nœuds/cartes/images dans le recouvrement des fenêtres catalogue et collection, retirer seulement les sortants, conserver l'image décodée hors intersection dans la marge bornée. Pistes CSS explicites égales au pas virtuel ; liste DOM toujours bornée. Invalidation après mutation, nouveau profil ou changement d'ordre/filtre conservée pour ne pas garder les actions de déplacement périmées.

Preuve navigateur connecté fictif2496affiches : les changements de fenêtres sont réellement attendus (deux frames + premier ID modifié), aller/retour catalogue et liste, mêmes images/nœuds et aucun appel répété pour l'élément commun, moins de50cartes. Tests de cache/profil/chargement et de géométrie conservés ; pas de pagination ajoutée. Aucune lecture vidéo rejouée. Cela ne prouve pas l'absence de tout artefact de composition WKWebView natif ; confirmation humaine nécessaire avant CLOSED.
