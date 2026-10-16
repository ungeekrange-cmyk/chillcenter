# ADR-0023 — Variantes typographiques d'une même série

16octobre2026, IMPLEMENTED_DETAIL, auteur `/root`, demande utilisateur explicite. Précise ADR0015-series-season-identity et0019 sans remplacer leurs identités opaques ni fusionner par titre seul.

Avant identification, la clé dérivée normalise accents, casse et espaces du libellé ET des composants du dossier source. Ponctuation, frontières de chemins et années de dossiers restent significatives. Des variantes de la même source se rejoignent, pas deux arbres distincts ou deux identifiants TMDB différents. Après identification, TMDB TV reste prioritaire : Doctor Who2005/2023 restent distincts. Les titres affichés, UUID et chemins physiques ne sont pas réécrits.

Le cache dérivé porte une version de règle indépendante de la révision commune. Première lecture après mise à jour : reconstruction une fois ; lectures suivantes sans changement : réutilisation. Pas de parcours complet supplémentaire à chaque page. Les sélecteurs historiques normalisent les variantes de libellé, mais refusent toujours un nom ambigu ; les sélecteurs opaques restent disponibles.

Les noms identiques ne prouvent toujours pas qu'il s'agit d'une même œuvre : sources différentes non équivalentes ou identifiants TMDB contradictoires exigent une association explicite, jamais une fusion destructive des vidéos ou historiques. Tests et retour arrière : [lot Configuration](../../implementation/CONFIGURATION-MATCHING-20261016.md).
