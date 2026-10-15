# BUG-0059 — sagas héritées sans révision, synchronisation422

15octobre2026, serveur07358f7/hash11cb… surPi, client7a0f480. OPEN→ROOT_CAUSE_CONFIRMED→FIXED_DEPLOYED→VERIFIED_SCOPED. La première vérification post-bascule CHANGES_REQUESTED est conservée.

Attendu : GETsagas et capture catalogue fonctionnent après enrichissement TMDB. Réel : POSTsnapshot et GETsagas422INVALID_OPERATION ; profils/bibliothèque/préférences/playlists200. Aucune lecture vidéo dans la sonde.

H1 confirmée : `Database::collections` importe paresseusement les sagas issues des métadonnées après `Database::open`, mais n'initialise pas `collection_revisions`. `Store::sagas` appelle `saga_revision` qui échoue. Relevé SQL lecture seule surPi :37sagas/30révisions absentes/0membre sans média/0membre sans identité. La copie locale ne peut donc pas s'initialiser.

RED ciblé `lazily_imported_sagas_have_revisions_without_resetting_existing_cas` reproduit « Saga introuvable ». Correction minimale : INSERT OR IGNORE des révisions absentes à1 dans la transaction `collections`, sans réinitialiser les CAS existants ni toucher les médias. Première assertion de fixture supposait à tort que renommer la collection réécrivait le champ historique du média ; corrigée pour vérifier sa conservation, sans changement de politique produit.

GREEN ciblé local :1testPASS ; test serveur `catalog_mirror_is_coherent_scoped_and_ignores_unrelated_progress` PASS ; clippy catalogue/all-targets PASS. Source figée `cfc87dec21aee553d72f5016f6c9dfe600a6e1f3`, revue code `/root/review_cache_server` APPROVED (ne revendique pas rerunRust), revue sécurité `/root/review_sidecar_quarantine` APPROVED avec rerun ciblé1PASS. BuildARM64/glibc2.28 horsligne PASS, binaireSHA256 `f85375ad4ac33fd0993151b547e8ca731d0ffa49441beb8b85756f0433f842cd`. Déploiement et sonde finale encore en cours à cette rédaction. Aucun SQL manuel de réparation dans le foyer ; les données existantes passent par la logique normale du serveur. Retour binaire073 ou ancien9277 possible, sauvegardes privées préservées ; la restauration des données exige une validation explicite distincte.

Final15octobre02:31:51UTC : CFC/hashf853… déployéPID6808, puis vérificateur distinct APPROVED_SCOPED02:33–02:34. La même sonde UiClient passe une seule fois sans retry/skip :2496médias, fiche/sagas/listes, unchanged et réouverturecache ; exit0/9,16s. Le422 initial est fermé sur cet artefact exact, sans lire de vidéo. Voir release/gate post-déploiement ; aucune validation visuelle ou NFR supplémentaire déduite.
