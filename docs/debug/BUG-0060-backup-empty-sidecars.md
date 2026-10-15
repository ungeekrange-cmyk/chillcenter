# BUG-0060 — fichiers temporaires SQLite dans une sauvegarde

15octobre2026, sauvegardePi privée `c8897485-a1b3-4a44-8afa-8711a32ec7f7`, source CLI9277. ROOT_CAUSE_CONFIRMED_STRUCTURE / REMEDIATION_PENDING. Aucune donnée vidéo/NAS concernée.

Le vérificateur indépendant constate deux fichiers non déclarés : `catalog.sqlite-wal`0octet et `catalog.sqlite-shm`32768octets. Ils existent avant son quick_check immuable, owner997/horodatage02:06:43UTC. La base7774208octets a toujours le hash du manifeste `9c25561210a8a4f4cb590c8daec16f978a0eb90712eb4b53c5df10e38b7b00b2`, quick_check immuableOK. Le validateur de restauration exige l'égalité exacte du jeu de fichiers et refuserait cette sauvegarde.

Mécanisme confirmé : journaux auxiliaires SQLite sans transactionsWAL. Leur créateur précis dans la chaîne backup/validation n'est pas prouvé ; ne pas attribuer à tort au vérificateur indépendant. La validation opérateur initiale en mode=ro sansimmutable n'interdisait pas de tels effets auxiliaires et ne contrôlait pas les fichiers non déclarés.

Remédiation opérateur préparée, non exécutée à cette rédaction : vérifierUUID/parents/permissions/manifeste/ensemble exact/tailles/hashDB/quick_checkimmutable/absenceFD, puis déplacer uniquement ces deux fichiers vers une quarantaine privée horsbackup. Réversible, aucun unlink, base/manifeste/service/NAS inchangés. Revue premièreversion CHANGES_REQUESTED parce que Pythonoptimisé supprimait lesassertions ; garde inconditionnel `if not __debug__: raise` ajouté et refus python-O prouvé avant tout accès fichier. Attendre la nouvelle approbation avant exécution.

Limite : pas de restauration destructive réelle effectuée ; contrôle strict structure/hash et préparation du retour seulement. Future validation de copie figée doit utiliser `mode=ro&immutable=1` et contrôler le jeu exact des fichiers.
