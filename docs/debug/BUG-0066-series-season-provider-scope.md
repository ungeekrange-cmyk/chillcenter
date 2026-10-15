# BUG-0066 — TMDB série/saison et homonymes

Auteur coordinateur `/root`, implémentation `/root/build_series_season_tmdb`, 2026-10-15.

Le client livré n’exposait pas de recherche série/saison. La revue indépendante `/root/review_series_sync` a demandé : édition par clé identifiée, séparation des homonymes non identifiés, recherche/tri par nom visible, révision d’import saison correcte, rejet de parent absent avant TMDB et nettoyage des images après échec CAS. Le premier candidat n’est donc pas approuvé.

Correction en cours : identité distincte du titre, membres figés, métadonnées saison indépendantes des épisodes, transfert vers miroir local et import contrôlé. Saison obtenue par série TMDB et numéro, pas recherche texte indépendante. Tests/résultats finaux à lier au SHA avant clôture ; pas de livraison nouvelle revendiquée ici.

Revue du candidat 3bbb87d : une ancienne régression `chill-catalog` reproduit un nouveau membre oublié ; l’attente de rejet d’un simple titre homonyme était aussi devenue obsolète. Correction : transaction IMMEDIATE, CAS commun/média relu avant écriture et groupe complet recalculé depuis l’ancre (association et imports série/saison). Test à deux connexions : scan committé après CAS préliminaire, publication refusée sans modification. Le test de collision vérifie désormais deux groupes distincts malgré le même titre, tout en conservant identifiant invalide/rollback SQL. Pas d’essai TMDB réel.
