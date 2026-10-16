# Revue indépendante — suite, affiches, scroll, matching

Source exacte **df2870f5776d20213bcb8c6c88a5bcb967091e97**, base99afef9430473792910314956bfa4cd8f2601226. Reviewer `/root/review_series_completion`, distinct des auteurs, lecture seule. Compte rendu transcrit par le coordinateur.

Verdict source **APPROVED_SCOPED**, aucun P1/P2 : repli de numérotation conservateur sans écriture de métadonnées ; `order_unknown` facultatif et réponses historiques préservées ; retrait des cibles d'affiche et gardes asynchrones ; rétention bornée des cartes avec invalidation sur changement de liste/filtre ; parenthèses valides conservées. Contrôles du reviewer : diff check,42Node ciblés,68domaine,11serveur/playback,24web PASS. Pas d'accès Pi, de lecture vidéo réelle ni de certification du rendu natif.

## Opérateur privé serveur seul

Première révision SHA256 **5feffd297933a33ab3ad6eb5af2af874420d3cdad6d08d46656484828d8fa633**, **CHANGES_REQUESTED P2** : URL de sondes seulement validées syntaxiquement, pouvant cibler un autre hôte. Cette révision est conservée, non exécutée.

Révision2 SHA256 **2b9b7804cf71a0235a37c2c263bac23d2e7158d4cca3de6f624974676eb1902b**, **APPROVED_SCOPED** : autorités HTTP/HTTPS vérifiées contre la configuration, même hôte, résolution IPv4 intersectée avec les interfaces locales ; les deux sondes forcées sur cette adresse avec `--resolve`. Syntaxe PASS, aucun nouveau blocage. Séquence arrêt/sauvegarde synchrone/bascule binaire seule/rollback et gardes web/configuration/NAS maintenues. Aucun opérateur exécuté ; pas une vérification post-déploiement.
