# REL-SERVER-ONLY-F3FDAC5

## Statut courant : DEPLOYED / VERIFIED_SCOPED

Serveur démarré le15octobre à22:14:29UTC, sourcef3fdac5/SHA d77e9e16 ; vérification indépendante22:18–22:20UTC APPROVED_SCOPED, [rapport](REL-SERVER-ONLY-F3FDAC5-verification.md) et [gate post-déploiement](../gates/G-POST-REL-SERVER-ONLY-F3FDAC5.md). Actif PID3117/NRestarts0, web/config/unité/TLS/montage identiques, sauvegarde privée5dd8df08… exacte1746entrées, rollbackffcc validé. Vrai UiClient2496fiches/révision série/reconnexion/delta inchangé21ms PASS. Aucun nouveau lecteur/GUI/fournisseur/scan.

Bundle Mac publié : `artifacts/macos/Chill Center-f3fdac5.app`, exécutable arm64 SHA `d31bfff6e722c086b57a135fcd5744c1f8d0fe6383bb821a32e5cad019a7327c`,48dylibs arm64, signature ad hoc profonde et chargement libmpvAPI2.5 PASS, sans lancement graphique. Remplace11aa529 comme bundle recommandé pour les sélecteurs explicites ; ancienne version conservée. Archive serveur publiée `artifacts/linux/chillcenter-server-f3fdac5-pi64.tar.gz`, SHA `35310c439f54fc0b45cbd4342c9154ec7302d8a86148fc7a4975252beb2ccd6f`, sans fichier web.

[Exécution](REL-SERVER-ONLY-F3FDAC5-deployment.md) : code terminal/stdout de l'opérateur non conservé et démarrage manuel prématuré pendant sauvegarde ([BUG0072](../debug/BUG-0072-deployment-session-observation.md)). L'état finalement déployé et la sauvegarde ont été relus intégralement par un vérificateur distinct ; pas de rc0 inventé, pas de rollback recommandé. Aucun travail web en cours n'a été modifié. Les étapes ci-dessous sont le dossier de readiness historique.

## Readiness et autorité

2026-10-15, source `f3fdac503c4b845571981c1fd1bc00bd0b13ec21`, serveur Linux ARM64. Livraison explicitement autorisée par l'utilisateur, web strictement exclu. Gate [G-DEPLOY](../gates/G-DEPLOY-REL-SERVER-ONLY-F3FDAC5.md) et trois reviewers distincts du constructeur. Coordinateur `/root`, déployeur `/root/deploy_server_only_f3fdac5`, vérification indépendante requise après bascule. Statut à ce checkpoint : READY, pas encore DEPLOYED/VERIFIED.

Inclut les correctifs Mac/serveur11aa529, l'intégration des listeners/statique du serveur web, les routes delta/images série/saison, le GET léger de révision et le pont API [BUG0071](../debug/BUG-0071-legacy-series-api-compatibility.md). [ADR0019](../architecture/decisions/ADR-0019-explicit-series-selector.md) préserve les titres historiques sans namespace réservé ; nouvelles identités par `selector=key`. API2026-10-14/v1 inchangées pour l'ancien web, aucun champ de requête devenu obligatoire.

## Artefact et migration

Binaire local privé `chill-server-f3fdac5`, SHA `d77e9e1669287a6d10f279c17f33d7f776efe7e194acc01cd542bcea33511c90`. Compilation Rust1.99/Zig existants, `cargo zigbuild --release --locked --offline -p chill-server --target aarch64-unknown-linux-gnu.2.28`, ELF validé, avertissement linker préexistant seulement. Aucun encodeur ni bibliothèque Mac dans le serveur.

SQLite user_version3 maintenu ; tables et triggers de révisions/saisons/groupes/checkpoints additifs, marqueur privé sync-generation. Sauvegarde hors ligne AVANT ouverture du nouveau binaire. Retour ancien binaire possible sans restauration destructive de la base ; sauvegarde privée disponible séparément. Aucun import/scan/fournisseur demandé ou lancé.

Opérateur privé relu, SHA `62081da7b574c4cd04c0ed3d09e238ea88dca4b4d4ba293b6963bc31a9d7f06e` : préflight état/proc/TLS/config/connexions/web, staging root, arrêt, sauvegarde vérifiée, copie rollbackffcc, remplacement atomique du binaire seul, redémarrage, health local8080/8443 et comparaison web/config/unité/TLS/montage. Aucun paquet web fourni. État inattendu = refus avant arrêt ; échec après arrêt = récupération de l'ancien service vérifiée. Aucune restauration automatique des données.

## Preuves et limites

296 tests Rust, Clippy et contrat76 chemins PASS au SHA exact ; tests de collisions titres/clés sous les deux bases, GET/PUT, profil/CAS et groupe non sélectionné inchangé. Les verdicts défavorables be57/02b et opérateur31de restent conservés. [Code](../reviews/server-only-code-f3fdac5.md), [sécurité](../reviews/server-only-security-f3fdac5.md), [intégration](../reviews/server-only-integration-f3fdac5.md).

Le Mac11aa529 reste partiel pour les opérations opaques sans paramètre ; nouveau bundlef3fdac5 nécessaire pour les corrections finales. Reconstruction du bundle Mac en cours, sans toucher aux anciennes versions. Pas de nouvelle validation lecteur/GUI/vidéo ni de benchmarkNAS/Wi-Fi, conformément à la demande de tests ciblés.

Après déploiement : consigner les hashes installés/processus, PID/redémarrages, sauvegarde/rollback, web identique, API datée/v1 et statut HTTP/HTTPS externe, vrai UiClient/cache/delta/séries sans mutation du foyer. Ne pas conclure VERIFIED avant le rapport indépendant.
