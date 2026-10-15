# BUG-0060 — fichiers temporaires SQLite dans une sauvegarde

15octobre2026, sauvegardePi privée `c8897485-a1b3-4a44-8afa-8711a32ec7f7`, source CLI9277. ROOT_CAUSE_CONFIRMED_STRUCTURE / REMEDIATION_VERIFIED_SCOPED. Aucune donnée vidéo/NAS concernée.

Le vérificateur indépendant constate deux fichiers non déclarés : `catalog.sqlite-wal`0octet et `catalog.sqlite-shm`32768octets. Ils existent avant son quick_check immuable, owner997/horodatage02:06:43UTC. La base7774208octets a toujours le hash du manifeste `9c25561210a8a4f4cb590c8daec16f978a0eb90712eb4b53c5df10e38b7b00b2`, quick_check immuableOK. Le validateur de restauration exige l'égalité exacte du jeu de fichiers et refuserait cette sauvegarde.

Mécanisme confirmé : journaux auxiliaires SQLite sans transactionsWAL. Leur créateur précis dans la chaîne backup/validation n'est pas prouvé ; ne pas attribuer à tort au vérificateur indépendant. La validation opérateur initiale en mode=ro sansimmutable n'interdisait pas de tels effets auxiliaires et ne contrôlait pas les fichiers non déclarés.

Remédiation opérateur exécutée après revue indépendante APPROVED de `/root/review_sidecar_quarantine`, scriptSHA256 `9270f9e417cd7cf5a6827b8fb632fee86ec1996363db5905e0e69ec0d866d842`. UUID/parents/permissions/manifeste/ensemble exact/tailles/hashDB/quick_checkimmutable/absenceFD vérifiés, puis déplacement des deux fichiers vers `/var/lib/chillcenter/backup-validation-quarantine-c8897485-a1b3-4a44-8afa-8711a32ec7f7`, root0700. Exit0 : `BACKUP_SIDECARS_QUARANTINED exact=2 wal_bytes=0 database_unchanged=true manifest_files_exact=true reversible=true service_untouched=true`. Aucun unlink ; base/manifeste/service/NAS inchangés. Première revue CHANGES_REQUESTED conservée parce que Pythonoptimisé supprimait les assertions ; garde inconditionnel `if not __debug__: raise` ajouté et refus python-O prouvé avant tout accès fichier.

Déployeur correctif suivant SHA256 `fd162e9920b0d9c3f0ca55c97cf132cb96f8909d4f8660da64709855c7f2cedc` également APPROVED : Pythonisolé/-I et garde explicite, quick_checkimmutable, contrôle fichiersexacts, quarantaine conditionnelle de ces seuls auxiliaires vides. Les anciennes sauvegardes ne sont ni effacées ni remplacées. Recontrôle indépendant final encore en attente.

Limite : pas de restauration destructive réelle effectuée ; contrôle strict structure/hash et préparation du retour seulement. Future validation de copie figée doit utiliser `mode=ro&immutable=1` et contrôler le jeu exact des fichiers.

Final : nouveau déploiement02:31:51UTC avec backupprivé8bbb9079… valide exactement (`PRIVATE_BACKUP_VERIFIED_EXACT`), puis vérification indépendante APPROVED_SCOPED :0700/1710fichiers déclarés=réels/zérosymlink, rollback073 vérifié. Le finding de readiness est fermé pour les deux sauvegardes de ce lot ; pas une correction générale du générateur CLI ni une restauration de données réellement exécutée. Les deux fichiers initiaux restent récupérables en quarantaine.
