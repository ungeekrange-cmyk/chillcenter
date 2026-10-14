# BUG-0054 — observation des premières affiches annulée avant insertion

2026-10-14, candidat local `codex/ui-panels-player`, base0d0da0e, avant remise. Défaut introduit lors du nettoyage des observations détachées ; pas une panne TMDB ou NAS.

Observation : la capture synthétique de saga montre une affiche chargée sur la dernière carte, mais une initiale sur la première. H1 : chaque appel `observeArtwork` nettoie les cibles déconnectées alors que la nouvelle grille est encore construite hors DOM ; il désinscrit donc les frères précédents avant l'insertion du lot.

Expérience falsifiable : attendre deux vraies balisesimage dans les deux cartes, au lieu de valider les seuls conteneurs. Avant correction, `scripts/test-ui-panels.cjs` RED timeout2500ms à cette assertion ; la capture et le code correspondent au mécanisme. Cause confirmée.

Correction : enregistrer toutes les cibles du lot, puis ne nettoyer les éléments détachés que dans le callback IntersectionObserver après insertion. Ignorer les anciennes cibles réellement détachées ; conserver la file séquentielle, le rejet d'ancien contexte et les placeholders hors écran. Aucun téléchargement complet ni accès NAS direct.

GREEN : le même parcours navigateur attend deux images de saga et au moins cinq images du sélecteur, puis valide filtres, sélection, retour et réglage local. Pas un appel TMDB réel ni une preuve de décodage natif. Capture synthétique et source/test reproductible liés au rapport de livraison ; retour à l'ancien bundle sans migration.
