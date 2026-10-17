# BUG0109 — Contenu mobile masqué par les outils

2026-10-17, signalement utilisateur sur iPhone, build `b1722f00c0a6`.
Statut : FIXED / VERIFIED_SYNTHETIC ; validation humaine du nouveau rendu restante.

Recherche et filtres occupaient le premier écran ; en paysage la navigation
basse réduisait encore la place. Les outils TMDB de saison précédaient les
épisodes. Causes vérifiées dans les sélecteurs et le DOM construits.

Correction exclusivement web : loupe ouvrant recherche/filtres du catalogue,
rail gauche en paysage téléphone, détails saison/série repliés après les
épisodes. Les filtres propres à l'ajout de vidéos restent distincts et visibles.
Les listeners existants sont conservés par déplacement des nœuds ; retour
desktop et résultats asynchrones tardifs testés. Aucun fichier Mac modifié.

Preuve : `scripts/test-web-mobile.cjs`, vrais assets avec API synthétique,
320/390/430/736/844px et retour desktop ; séries et collections non vides.
