# BUG0094 — Suffixes de fichiers encore présents dans la recherche

16octobre2026, FIXED_LOCAL, `codex/configuration-matching`, base66aef9c. Nouveau retour après BUG0062/0089 : séparateurs répétés entre titre, année, audio et qualité. Aucun renommage de fichier demandé ou effectué.

H1 confirmée : nettoyage primaire conservant certains blocs inconnus, et recherche automatique en une seule requête. Une recherche de secours est bornée à une tentative après résultat vide ; les vrais tirets/titres parenthétiques sont préservés, les années marquées restent des contraintes. La provenance interne du secours accompagne les candidats : l'enrichissement ne peut accepter le nom raccourci que si cette requête a effectivement eu lieu. Résultats initiaux non vides, ambiguïtés et mauvais millésimes ne deviennent pas de faux accords. Une saisie manuelle modifiée reste intentionnelle ; le libellé prérempli inchangé et la file de reconnaissance utilisent le nettoyage automatique.

Régressions catalogue/serveur hors ligne sur tirets, parenthèses, Blade Runner2049, ambiguïtés et provenance. Pas de fournisseur réel ni d'enrichissement global déclenché. Détails/commit et déploiement restant : [lot](../implementation/CONFIGURATION-MATCHING-20261016.md). Rollback binaire ; aucune réécriture des métadonnées existantes ou du NAS.
