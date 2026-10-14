# BUG-0056 — catalogue/images redemandés à chaque navigation

14octobre2026, base `ae0ce44`, même branche que BUG0055. FIXED, mesures Pi en attente. Attendu utilisateur : catalogue/images locaux et contrôle des nouveautés au démarrage. H1 confirmée : loadPage repartait sur les pages HTTP ; pas de persistance des images.

Correction : miroir SQLite/images privés par serveur/profil, transaction atomique, révision ciblée et instantané immuable. Progression/préférences d'un autre profil sans copie complète. Vidéos/données d'exploitation exclues. Ancien cache conservé et signalé si actualisation échoue.

Tests `persisted_catalog_navigation_never_needs_a_reachable_server`, `image_cache_survives_restart_without_network_and_rejects_links`, `catalog_mirror_is_coherent_scoped_and_ignores_unrelated_progress` : navigation sans serveur joignable, persistance, filtres, isolation et refus après suppression du profil. Pas une preuve de performance Pi. Retour ancien client ; aucune suppression de données réelles nécessaire. ADR0013 et preuves du lot.
