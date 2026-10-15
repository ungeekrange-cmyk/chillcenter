# Exécution observée — REL-SERVER-ONLY-F3FDAC5

2026-10-15, déployeur distinct `/root/deploy_server_only_f3fdac5`, coordinateur `/root`. Gate0d0d951, sourcef3fdac5, opérateur62081da7, binaire ARM64d77e9e16. Aucun fichier web fourni/uploadé, aucun checkout web modifié.

Transfert dans `/var/tmp/chillcenter-server-only-be57d9f` privé root0700, hashes distants de l'artefact et de l'opérateur vérifiés avant exécution selon le déployeur. Lancement de l'opérateur approuvé. **Son stdout/code de sortie terminal n'a pas été capturé** : ne pas inventer un rc0 ni une exécution sans incident. [BUG0072](../debug/BUG-0072-deployment-session-observation.md) conserve le redémarrage manuel prématuré pendant la sauvegarde, puis l'arrêt de sa boucle et la fin ultérieure de l'opérateur. Aucune relance de l'opérateur, restauration de données ou suppression de verrou.

À22:14:48Z déployeur puis22:14:57Z coordinateur, état final observé : service actif/running PID3117/NRestarts0, SHA installé/processus `d77e9e1669287a6d10f279c17f33d7f776efe7e194acc01cd542bcea33511c90`. La vérification indépendante doit confirmer chaque élément suivant ; observations du déployeur, pas gate post-déploiement :

| Élément | Valeur observée |
| --- | --- |
| Sauvegarde privée | `/var/lib/chillcenter/data/backups/5dd8df08-adcb-4d09-a6c1-2eb549c0c414` |
| Rollback binaire | `/root/chillcenter-rollback/chill-server-ffcc634923f6`, SHAffcc634923f637331cac25fec1e5283b062ef5e2943778bcab59460b6269b0f7 |
| Empreinte arbre web avant/après | `07868e61cd75dc36d5bf62eefd258b7409b6ed264bde726cdcf626d45b57fe2b` (contenus, inodes, modes/uid/gid, tailles, mtime) |
| Configuration | `d6167cb1624b6d5888473af76d5470102afedce25b2e29ddbaa032730cd0ceb1` |
| Unité | `76f7d34921ff344e344ba2bffa51263248ec6118dad9e15a39ef7078877d72e2` |
| TLS SPKI | `3018fb4d28af3899ae9a0261c89ef92d7ebfe508ecbd613b38f6dd6d7d4740df` |
| Montage NAS | `1935a8db9901c414782eaa05dc5ee81f6ec092a499755034e90e562f8dee0d37` |

Statut à ce checkpoint : DEPLOYED_OBSERVED, vérification indépendante PENDING. Pas de requête vidéo/scan/fournisseur ni mutation d'un profil du foyer. Les réponses de base HTTP8080/HTTPS8443 observées par le déployeur ne remplacent pas les tests API/client externes et la validation privée de la sauvegarde.
