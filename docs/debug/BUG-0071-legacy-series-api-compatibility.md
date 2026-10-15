# BUG-0071 — Compatibilité du web existant avec les séries homonymes

2026-10-15, déploiement serveur seul explicitement demandé, avant toute écriture sur le Pi. Source initiale be57d9f, client web commité aad34ce préservé.

Symptôme confirmé par lecture croisée : l'ancien web appelle GET/PUT `series/{titre}`, tandis que le nouveau domaine refuse le titre lorsqu'il correspond à plusieurs groupes. Les70 chemins/89 opérations hérités restent présents, mais cette différence sémantique casse la navigation et l'édition historiques des homonymes. La revue d'intégration initialement favorable a donc été corrigée en CHANGES_REQUESTED avant déploiement.

Correction : séparer explicitement la vue API historique adressée par titre et les nouveaux sélecteurs opaques. La vue titre conserve GET agrégé et PUT de métadonnées communes par lot, même CAS/refus de collision que l'ancien serveur ; elle ne change pas les identifiants TMDB, chemins ou positions saison/épisode. GET mixte ne revendique aucun TMDB/saisons communs. Les opérations du domaine et du nouveau client sur clés opaques restent isolées : une correction Doctor Who2005 ne modifie pas Doctor Who2023. Aucun changement de fichiers web, de dépendance ou de schéma requis.

Test API : les deux alias, deux séries homonymes/twoTMDB, portée vu par profil, lecture sans incrément de révision, édition historique conservée, conflit CAS, identités/chemins/saisons inchangés et modification opaque limitée à un groupe. Le premier essai du nouveau test envoyait une fixture PUT incomplète (champs SeriesPatch obligatoires absents) et obtenait422 ; fixture corrigée, pas d'assouplissement du contrat de requête.

Ne pas confondre ce pont de compatibilité avec un portage de la nouvelle interface web : les sources et assets web restent hors livraison.

Seconde revue sur02b6b8e CHANGES_REQUESTED : le préfixe du segment ne pouvait pas distinguer un titre littéral d'une clé. Une série réellement nommée `tmdb:tv:1001` pouvait ouvrir/modifier une autre série. Correction finale : `selector=title` par défaut, `selector=key` facultatif explicite, sous2026-10-14/v1. Le nouveau Mac ajoute le paramètre ; l'ancien web reste strictement sur ses titres, sans modification d'assets. Tests de collision GET/PUT pour les deux préfixes et les deux bases, groupe non sélectionné entièrement inchangé, sélection invalide400 ; test client du paramètre envoyé. Aucun écrasement de ces revues défavorables. [ADR0019](../architecture/decisions/ADR-0019-explicit-series-selector.md).
