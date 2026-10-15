# Revue code/opérateur — livraison serveur seul

Reviewer indépendant `/root/review_mac_web_compat`, compte rendu de sa conclusion consigné par `/root` ; aucune auto-approbation du coordinateur.

Source exacte `f3fdac503c4b845571981c1fd1bc00bd0b13ec21`, arbre `f88ae4d26c07f72cb150b948e58fa0c08b904bfa` : APPROVED, aucun finding concret dans le delta depuis02b6b8e. L'opt-in explicite `selector=key` résout la confusion titre/identifiant, les requêtes historiques restent en mode titre et les query parameters inconnus continuent d'être ignorés. Le client natif envoie le sélecteur pour ses clés ; aucun fichier web n'est modifié.

Preuves indépendantes :7 tests serveur `legacy_` PASS, diff-check et JSON OpenAPI PASS. Le test client n'a pas été rejoué par ce reviewer (cache de dépendances indisponible dans sa tentative) ; les40 tests client de la suite296 du coordinateur sont donc des preuves fournies, pas indépendantes.

Opérateur privé `/private/tmp/chillcenter-server-only.30Fmwa/deploy-server-only.sh`, SHA `62081da7b574c4cd04c0ed3d09e238ea88dca4b4d4ba293b6963bc31a9d7f06e` : APPROVED, `bash -n` PASS, ports locaux8080/8443 corrigés. Aucune écriture web/config/NAS, sauvegarde privée/rollback et gardes conservés.

Historique conservé : opérateur31de45 refusé P1 (probes Pi-locales80/443 incompatibles avec prerouting), source02b6b8e refusée P1 (préfixes des titres détournant la cible). Rien n'a été déployé sous ces verdicts. L'approbation source ne vaut pas validation graphique ou post-déploiement du Pi.
