# BUG0097 — Dernière image précédente pendant le chargement

16octobre2026, FIXED_LOCAL en préparation, branche `codex/configuration-matching`, base66aef9c. VidéoA, retour liste, lancementB : l'ancienne image reste visible avant la première imageB. Attendu : noir, sans ancienne affiche ni imageA.

H1 confirmée par inspection : le drawable natif pouvait conserver sa dernière présentation, et render_tick autorisait encore des frames sans vérifier que le nouveau fichier était chargé. Le cycle de rechargement présente un vrai clear noir OpenGL+flush, puis interdit le rendu de frame avant FileLoaded/préférences de la nouvelle génération. Montage d'une surface non chargée également noir. Pas un simple masque CSS qui cacherait ensuite la nouvelle vidéo.

Test unitaire de garde de rendu et compilation native ; contrôle humain Tauri reste distinct, aucun nouveau parcours de lecture générale revendiqué. [Preuves/bundle/rollback](../implementation/CONFIGURATION-MATCHING-20261016.md).
