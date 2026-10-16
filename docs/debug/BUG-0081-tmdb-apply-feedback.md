# BUG-0081 — Validation TMDB sans retour d’état clair

**OPEN — signalé, non corrigé dans ce lot.** 16 octobre 2026, fiche média/recherche TMDB, version exacte à confirmer.

Après sélection d’un film dans les propositions, l’utilisateur ne sait pas si le choix est pris en compte. Attendu : fermer les propositions, afficher la référence choisie, montrer un indicateur de traitement local à la fiche, puis mettre à jour les données et l’image ouvertes. L’usage du reste de l’application doit rester possible.

Prévoir succès, erreur actionnable et résultat périmé si l’utilisateur change de fiche ; ne pas marquer comme réussi avant confirmation serveur ni utiliser un verrou global de l’interface. Cause et comportement exacts non diagnostiqués dans ce lot. Test attendu : réponse retardée, erreur, changement de fiche et navigation concurrente avec assertions sur l’état visible.
