# Livraison serveur seul et Mac — df2870f

16 octobre 2026. Source produit **df2870f5776d20213bcb8c6c88a5bcb967091e97**, preuves initiales44c355c ; branche `codex/fix-next-posters-scroll-matching`. Binaire ARM64 immuable SHA **14d84a0478b9f394bfe59b3b88588edb4a9305dfedfc0fb0b66449b39d034169**. Livraison corrective locale/foyer explicitement autorisée, sans migration, changement d'identité, coût ou publication ; mode DEV_FAST ciblé, protocole propre Chill Center conservé. [Tests et bundle](../implementation/NEXT-POSTERS-SCROLL-MATCHING-20261016.md), [revues source et opérateur](../reviews/next-posters-scroll-matching-df2870f.md).

**Vérification indépendante post-bascule APPROVED_SCOPED** : [rapport](REL-NEXT-POSTERS-SCROLL-MATCHING-001-verification.md), service stable et mêmes octets, backup exact, web/configuration/unité/TLS/NAS inchangés, API datée/v1 et cas réel001→002 PASS. Pas une validation vidéo/GUI humaine.

## Exécution observée

- Préflight lecture seule16octobre14:37UTC : ancien binaire installé/processus **d77e9e1669287a6d10f279c17f33d7f776efe7e194acc01cd542bcea33511c90**, PID1082/NRestarts0. Web/configuration/unité/SPKI conformes. Aucune connexion active, plus de8Gio libres sur le volume de données. Empreinte de montage capturée à nouveau : le champ autofs contient des identifiants de processus/inodes variables après redémarrage ; CIFS toujours en lecture seule. Pas de modification du montage.
- Transfert du **binaire seul** et de l'opérateur privé revu SHA **2b9b7804cf71a0235a37c2c263bac23d2e7158d4cca3de6f624974676eb1902b** dans un dossier root0700. Hashes distants vérifiés avant exécution. Aucun fichier web, unité ou configuration transféré.
- Opérateur suivi avec journal privé et fichier terminal indépendants de SSH, PID2080. Arrêt puis sauvegarde synchrone avec l'ancien binaire ; aucune relance manuelle pendant cette sauvegarde. Validation exacte du manifeste, tailles/hashes et SQLite avant bascule.
- **16octobre2026 à14:44:03UTC**, code terminal **0**, `PRIVATE_BACKUP_VERIFIED_EXACT` et `DEPLOYED_UTC` capturés. Installation/processus nouveauSHA14d84…, PID2326 ; contrôles opérateur HTTPS/API datée/v1 et webHTTP/HTTPS PASS. Vérification indépendante à consigner séparément ; ce paragraphe ne constitue pas son verdict.

| Élément conservé | Valeur observée après bascule |
| --- | --- |
| Sauvegarde privée | `/var/lib/chillcenter/data/backups/9b975f1e-1667-4565-aa87-a4e7e3da089e` |
| Ancien binaire pour rollback | `/root/chillcenter-rollback/chill-server-d77e9e166928` |
| Arbre web, contenu/inodes/modes/mtime | `07868e61cd75dc36d5bf62eefd258b7409b6ed264bde726cdcf626d45b57fe2b` |
| Configuration | `d6167cb1624b6d5888473af76d5470102afedce25b2e29ddbaa032730cd0ceb1` |
| Unité | `76f7d34921ff344e344ba2bffa51263248ec6118dad9e15a39ef7078877d72e2` |
| TLS SPKI | `3018fb4d28af3899ae9a0261c89ef92d7ebfe508ecbd613b38f6dd6d7d4740df` |
| Montage NAS avant/après cette opération | `4524c19a4c739c525b97bd9ef41160541d2dd1b077f241e4e491647149d9223c` |

Aucune nouvelle lecture vidéo, requête fournisseur TMDB, reconnaissance massive, scan, édition de profil/état personnel ou écriture NAS lancée. Une sonde initiale GET `/admin/tmdb/jobs` a reçu405 car cette route ne permet quePOST ; corrigée en consultant GET `/admin/tmdb`, sans lancer de job. Ce405 d'outillage n'est ni une régression serveur ni une création de tâche.

## Compatibilité, test humain et retour arrière

API canonique2026-10-14 et aliasv1 restent servis, anciens champs/requêtes conservés. Seul ajout contractuel : indice facultatif `NextVideo.order_unknown`, omis lorsqu'il est faux. Le web installé n'est pas modifié. Correction de numérotation plate et règle courte approuvée incluses ; pas de migration ni réécriture en masse des données existantes.

Client à tester : **`artifacts/macos/Chill Center-test-df2870f.app`**, interne0.5.1/build6, SHA exécutable **20316cb655d45327c283184fab3c1675c3c0f9d99a6fb2a3b423c7e61e47d182**, signature/48dylibs arm64/libmpvAPI2.5 PASS. Anciennes versions et tags préservés, pas de nouvelle release numérotée implicite.

Rollback : arrêt du service, remplacement atomique du binaire uniquement par la copie d77e validée, redémarrage et nouvelles sondes API/web/empreintes ; conserver configuration et web. Aucune restauration automatique de base et aucun effacement des vus déjà enregistrés. Sauvegarde opérationnelle reste privée surPi, pas exportée. Validation humaine de la proposition suivante, du marquage court, des fiches et du défilement Tauri restant distincte des preuves unitaires/DOM/API.
