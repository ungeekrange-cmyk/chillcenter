# BUG-0079 — Ancienne affiche sur une fiche sans match TMDB

**FIXED_LOCAL — revue et remise en cours, test Tauri humain restant.** 16 octobre 2026. Régression utilisateur du comportement suivi par [BUG-0065](BUG-0065-stale-detail-and-artwork.md) ; les anciennes preuves de correction ne valent pas validation du présent cas.

Reproduction rapportée : ouvrir une fiche avec match/affiche, puis une fiche sans match TMDB ; cette dernière montre l’ancienne affiche. Attendu : image propre à la nouvelle fiche ou placeholder, jamais celle de la fiche précédente.

Cause confirmée : `invalidateArtwork` changeait le token mais gardait l'ancien `data-media-id` et l'observation du même élément réutilisé. Après ouverture de B sans affiche, un nouvel événement IntersectionObserver pouvait donc demander A sous le token neuf de B, contournant la protection des anciennes requêtes. Pas de preuve de mauvaise affectation persistée en base.

Correction : retirer cible/identité/file/observation avant d'attendre la nouvelle fiche, et à la fermeture ; élément d'affiche vide pendant chargement, puis placeholder propre à B. Les événements d'une cible retirée sont ignorés. Réponse déjà en vol rejetée par son ancien token. Une vraie affiche personnalisée sans TMDB reste autorisée.

Preuves ciblées : observer tardif A après B, réponse A déjà en vol, fiche vide avant réponse de B, affiche personnalisée sans TMDB. Tests Node déterministes dans `detail-lifecycle.test.js`, aucun fournisseur réel ni modification du catalogue foyer. Rendu natif à confirmer dans le nouveau bundle, pas dans l'ancien94495c1.
