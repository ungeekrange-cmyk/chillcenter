# BUG-0079 — Ancienne affiche sur une fiche sans match TMDB

**OPEN — signalé, non corrigé dans ce lot.** 16 octobre 2026, client Mac, version exacte à confirmer. Régression utilisateur du comportement suivi par [BUG-0065](BUG-0065-stale-detail-and-artwork.md) ; les anciennes preuves de correction ne valent pas validation du présent cas.

Reproduction rapportée : ouvrir une fiche avec match/affiche, puis une fiche sans match TMDB ; cette dernière montre l’ancienne affiche. Attendu : image propre à la nouvelle fiche ou placeholder, jamais celle de la fiche précédente.

Impact : présentation trompeuse et risque d’attribution erronée perçue. Cause **non confirmée** ; distinguer rendu, liaison/cache d’image et métadonnées réellement stockées avant toute correction. À reproduire avec deux fiches identifiées explicitement et des réponses retardées ; vérifier fermeture/réouverture et changement de profil. Aucun diagnostic ou correctif revendiqué ici.
