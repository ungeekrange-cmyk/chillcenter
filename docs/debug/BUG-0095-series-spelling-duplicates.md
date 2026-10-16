# BUG0095 — Variantes casse/accents dupliquant une série

16octobre2026, FIXED_LOCAL, branche `codex/configuration-matching`, base66aef9c ; F14, ADR0015/0023. Des séries dont noms/dossiers ne diffèrent que par accents, casse ou espaces sont présentées séparément.

H1 confirmée : comparaisons exactes du titre et de la source ; cache valide seulement par révision commune, sans version d'algorithme. Clés dérivées désormais canoniques et cache versionné reconstruit une seule fois. Les titres/chemins/UUID et états personnels restent conservés. Les sources distinctes et identifiants TMDB distincts ne sont jamais réunis par la seule ressemblance du titre.

Tests : variantes d'une même source réunies, plusieurs saisons, remakes/Doctor Who et IDs TMDB distincts séparés, sélecteur historique ambigu toujours refusé, ancien cache invalidé puis cache chaud réutilisé. Pas de fusion destructive ou parcours du NAS. [Preuves/rollback](../implementation/CONFIGURATION-MATCHING-20261016.md).
