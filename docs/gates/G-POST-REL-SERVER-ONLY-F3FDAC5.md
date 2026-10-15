# Gate post-déploiement — VERIFIED_SCOPED

2026-10-15, coordinateur `/root`, verdict indépendant `/root/verify_server_only_f3fdac5` APPROVED_SCOPED sur la sourcef3fdac5 et le binaire d77e9e1669287a6d10f279c17f33d7f776efe7e194acc01cd542bcea33511c90. [Rapport immuable](../releases/REL-SERVER-ONLY-F3FDAC5-verification.md), [autorité/readiness](G-DEPLOY-REL-SERVER-ONLY-F3FDAC5.md).

Contrôles réels22:18:01–22:20:30UTC : service/PID3117 stable/NRestarts0, hashes fichier et processus exacts, web complet identique, config/unité/SPKI/montage inchangés, backup5dd8df08 exact1746entrées/SQLitequick_check, rollbackffcc vérifié, zéro warning/error après22:14:57UTC. API datée/v1 et racines externes80/443/direct8080/8443 PASS, TLS strict et gardes origine/hôte PASS, OpenAPI76/95 conserve70/89 hérités. Vrai UiClient2496fiches/détails/listes/sagas/séries avec révision, réouverture et delta inchangé21ms PASS. Bundle Macf3 signature/arm64 vérifiés sans lancementGUI.

La livraison **serveur seul** passe à VERIFIED_SCOPED ; pas de rollback recommandé. Le premier incident de surveillance/démarrage pendant sauvegarde reste conservé, ainsi que l'absence de code retour opérateur capturé ; le verdict repose sur l'état final et ses preuves indépendantes. Aucun profil du foyer modifié, aucune vidéo lue et aucun fournisseur appelé.

Cette gate ne valide ni une nouvelle version web, ni le rendu GUI/lecteur, ni les performances NAS/Wi-Fi, ni l'intégralité des US. Une modification ultérieure du binaire ou des éléments préservés invalide ces observations et requiert une nouvelle vérification.
