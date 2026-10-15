# BUG-0059 — sagas héritées sans révision, synchronisation422

15octobre2026, serveur07358f7/hash11cb… surPi, client7a0f480. OPEN→ROOT_CAUSE_CONFIRMED→FIXED_LOCAL, contrôle Pi final requis. La première vérification post-bascule CHANGES_REQUESTED est conservée.

Attendu : GETsagas et capture catalogue fonctionnent après enrichissement TMDB. Réel : POSTsnapshot et GETsagas422INVALID_OPERATION ; profils/bibliothèque/préférences/playlists200. Aucune lecture vidéo dans la sonde.

H1 confirmée : `Database::collections` importe paresseusement les sagas issues des métadonnées après `Database::open`, mais n'initialise pas `collection_revisions`. `Store::sagas` appelle `saga_revision` qui échoue. Relevé SQL lecture seule surPi :37sagas/30révisions absentes/0membre sans média/0membre sans identité. La copie locale ne peut donc pas s'initialiser.

RED ciblé `lazily_imported_sagas_have_revisions_without_resetting_existing_cas` reproduit « Saga introuvable ». Correction minimale : INSERT OR IGNORE des révisions absentes à1 dans la transaction `collections`, sans réinitialiser les CAS existants ni toucher les médias. Première assertion de fixture supposait à tort que renommer la collection réécrivait le champ historique du média ; corrigée pour vérifier sa conservation, sans changement de politique produit.

GREEN local/revue/build/déploiement encore à compléter à cette rédaction. Aucun SQL manuel de réparation dans le foyer ; les données existantes passent par la logique normale du serveur. Retour binaire073 ou ancien9277 possible, sauvegardes privées préservées ; la restauration des données exige une validation explicite distincte.
