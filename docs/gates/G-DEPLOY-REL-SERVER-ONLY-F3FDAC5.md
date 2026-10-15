# Gate de déploiement — serveur seul, web préservé

2026-10-15 ; coordinateur `/root`. FULL_ASSURANCE pour le matériel réel du foyer ; périmètre/design déjà approuvés, aucune nouvelle découverte, frontière d'identité, exposition publique, dépendance ou dépense. Gate ouverte pour le seul remplacement de binaire ci-dessous, pas pour une livraison web.

- Autorité utilisateur actuelle : « déploie mais il ne faut pas toucher à la version web […] bonne version de l'API […] sans rendre obsolète celle de la version web ». Autorisation de sauvegarde privée/arrêt/remplacement/redémarrage confirmée dans l'historique ; pas de modification NAS/configuration.
- Source immuable : `f3fdac503c4b845571981c1fd1bc00bd0b13ec21` ; commits ultérieurs de rapports seuls ne changent pas ce candidat.
- Artefact ELFARM64/glibc2.28 : `/private/tmp/chillcenter-server-only.30Fmwa/chill-server-f3fdac5`, SHA `d77e9e1669287a6d10f279c17f33d7f776efe7e194acc01cd542bcea33511c90`, cross-build verrouillé/hors ligne réussi et ELF validé. Manifeste privé lié à la source.
- Code/opérateur : [APPROVED](../reviews/server-only-code-f3fdac5.md), `/root/review_mac_web_compat`.
- Sécurité : [APPROVED_SCOPED](../reviews/server-only-security-f3fdac5.md), `/root/server_only_security_gate`.
- Tests/intégration : [APPROVED](../reviews/server-only-integration-f3fdac5.md), `/root/server_only_integration_gate`.
- Coordinateur :296Rust (75catalogue/40client/64domaine/117serveur), Clippy workspace/all-targets `-D warnings`, contrat76 chemins PASS ;59Node de l'interface inchangée réutilisés.70 chemins/89 opérations hérités conservés,95 opérations locales ;2026-10-14/v1 préservés, paramètre de sélection facultatif.
- Opérateur exact SHA `62081da7b574c4cd04c0ed3d09e238ea88dca4b4d4ba293b6963bc31a9d7f06e`, sauvegarde/rollback/health bornés. AncienSHAffcc6349 attendu obligatoirement ; refus si état changé/connexion active.
- Cible : admin@192.168.1.42, servicechillcenter, TLS8443 et HTTP8080, redirections externes443/80 conservées. Web `/usr/lib/chillcenter/web` interdit en écriture ; contenu/métadonnées/inodes comparés avant/après.
- Préflightlecture seule21:43Z : PID1634/actif/NRestarts0,ffcc6349, configd6167cb1, unité76f7d349, pas de connexion lecteur.22h : disque commun `/dev/mmcblk0p2`,9Go libres, données85Mio horsbackups. Refaire immédiatement avant arrêt.
- Déployeur distinct : `/root/deploy_server_only_f3fdac5`. Vérificateur post-déploiement distinct à désigner, sans mutation du Pi.

Retour arrière : copie privée du binaireffcc, restauration atomique seulement si le nouveau binaire est celui attendu, redémarrage/HTTPS strict vérifiés. Schéma3 et ajouts compatibles ; aucune restauration automatique de données qui pourrait perdre de nouvelles écritures. Une restauration de données requiert une autorité spécifique. L'absence de sauvegarde complète ou l'échec de health interdit la promotion.
