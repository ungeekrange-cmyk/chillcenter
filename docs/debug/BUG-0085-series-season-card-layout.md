# BUG-0085 — Navigation et disposition de la fiche série

**CLOSED — confirmation utilisateur du 16 octobre 2026 : « bugs 78, 79/80, 85/86/87 et 88 corrigés ».** Clôture sur ce retour humain après la remise df2870f ; aucune nouvelle exécution de test revendiquée pour cette clôture. Les statuts et limites ci-dessous conservent les checkpoints précédents.

16 octobre 2026. **CORRECTED_LOCAL**, validation humaine Tauri à faire.

Symptômes : toutes les saisons sont repliées à l'entrée, épisodes empilés ; résumé, compteurs et note ne sont pas à côté de l'affiche. L'utilisateur demande la première saison contenant du non-vu ouverte et des cartes en grille.

Causes : aucun choix initial de saison ; cartes directement dans le `<details>` doté d'un `display:flex`, au lieu d'un vrai conteneur grille dans son contenu natif ; outils communs dans l'en-tête.

Correction : première saison non vue dans l'ordre saison/épisode, repli vers la première si tout est vu ; saisons inconnues en dernier. Une actualisation de la même série conserve les plis manuels. Grille interne réutilisant les cartes ; affiche et résumé/compteurs/cinq étoiles côte à côte ; outils communs TMDB/images/édition après les saisons. Les outils de chaque saison sont conservés.

Preuves : cinq tests initialement RED ; six tests `series-layout.test.js` GREEN. `scripts/test-series-layout.cjs` valide le DOM connecté à 1440 et 760px, plusieurs colonnes, ordre des épisodes/outils, replis manuels et série déjà vue. Captures locales inspectées, pas de lecture ni accès Pi/NAS. Pas de changement API ou données.
