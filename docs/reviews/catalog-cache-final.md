# Revues indépendantes — catalogue/cache

2026-10-14. Le coordinateur consigne les verdicts, sans s'auto-approuver. Périmètre : corrections demandées, pas toute la backlog ni les NFR Pi/NAS. Source finale `7a0f480a286d00729af3d8f04b6300c4eeade499`.

## Historique conservé

- `8d612fecd0bc5af663d2344df9eea3abf87167af` : CHANGES_REQUESTED. `/root/review_cache_server` trouve le HIGH scan concurrent/lignes anciennes sous nouvel epoch ; `/root/review_catalog_client_final` trouve les P1 nouvelle tentative réseau répétée après échec et suggestions encore réseau, ainsi que le P2 cache de profil supprimé conservé.
- `07358f73b5a6e378f1c3c0aad45cf6e4efc65aaf` : serveur APPROVED après garde optimiste avant/après,409/retry borné et test déterministe seconde connexion ; client CHANGES_REQUESTED pour le dernier P2 : nettoyage différé au GET profils suivant, pouvant échouer après DELETE réussi.
- `7a0f480a286d00729af3d8f04b6300c4eeade499` : client APPROVED par `/root/review_catalog_client_final`. Nettoyage exact immédiat après DELETE confirmé, déconnexion du profil/mémoire/préchargement/choix local avant GET, liens refusés, voisins préservés. Tests ciblés indépendants navigation locale avec échec du GET suivant et purge/symlinks PASS, diff propre/SHA confirmé.
- Même source finale : serveur/domaine APPROVED par `/root/review_cache_server`. Aucun changement serveur/domaine/API depuis07358f7, diff des sources vérifié ; preuve miroir/race précédente reste applicable, pas de relance complète inutile. ADR0013 peut passer ACCEPTED.

Autres preuves indépendantes sur073 : contrat70routes, test miroir serveur, concurrence scan, navigation locale et purge PASS. Clippy trois crates/all-targets PASS par l'auteur ; clippy client final PASS. Aucun test GUI ou lecteur général relancé après la restriction explicite de l'utilisateur.

## Réserve non bloquante

P2 : certaines affiches personnalisées serveur peuvent rester orphelines après suppression de profil/import conflictuel. Documenté dans ADR0013, pas de suppression générale engagée. Les plafonds de cache client et instantanés serveur ne constituent pas un GC de ces fichiers.

## Opérateur

`/root/review_cache_operator_final`, distinct des auteurs : APPROVED pour le script privé112lignes SHA256 `1b4169cf81dedfb818f68226e3d9e8ba75a339f3c5c022f0e8f36f33e2a65bd6`, inspection intégrale, bash-n et parser backup PASS. Root/empreintes/staging atomique/backup offline privé/rollback/config-unité-SPKI inchangés vérifiés statiquement. Ce verdict ne prouve pas un déploiement. Le script refuse toute connexion établie du processus avant arrêt ; les connexions sortantes TMDB ont effectivement suspendu l'opération, service ancien préservé.
