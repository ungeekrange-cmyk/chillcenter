# BUG-0081 — Validation TMDB sans retour d’état clair

**FIXED_LOCAL — remise en préparation.** 16octobre2026, branche `codex/configuration-matching`, base66aef9c. L'état OPEN ci-dessous est le signalement initial, pas une preuve de correction ancienne.

Après sélection d’un film dans les propositions, l’utilisateur ne sait pas si le choix est pris en compte. Attendu : fermer les propositions, afficher la référence choisie, montrer un indicateur de traitement local à la fiche, puis mettre à jour les données et l’image ouvertes. L’usage du reste de l’application doit rester possible.

Prévoir succès, erreur actionnable et résultat périmé si l’utilisateur change de fiche ; ne pas marquer comme réussi avant confirmation serveur ni utiliser un verrou global de l’interface. Cause et comportement exacts non diagnostiqués dans ce lot. Test attendu : réponse retardée, erreur, changement de fiche et navigation concurrente avec assertions sur l’état visible.

## Cause et correction du16octobre

H1 confirmée par inspection puis réponse retardée : `renderTmdbPanel.mutate` attendait d'abord une fiche fraîche puis l'association, sans modifier les propositions ou afficher d'état de traitement. Désormais les choix se ferment immédiatement ; « Association en cours avec XXX… » et roue locale précèdent le premier await. Garde anti-double soumission, état non confirmé sur échec, référence affichée après réponse serveur et fiche/image ouvertes rafraîchies. La file de matching dans Configuration utilise le même comportement observable. Aucun verrou global ; navigation autorisée et retours tardifs gardés par contexte.

Tests Node : média/série/saison, réponse retardée, double clic, erreur et nouvelle fiche. Vrai DOM connecté synthétique : panneau utilisable à côté et ancienne confirmation incapable de repeindre une autre fiche. Aucun vrai appel TMDB. Preuves finales, commit, bundle et limites dans [le lot](../implementation/CONFIGURATION-MATCHING-20261016.md). Rollback : client précédent, sans mutation de contrat API.
