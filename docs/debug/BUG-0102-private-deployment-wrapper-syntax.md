# BUG0102 — wrapper privé de sauvegarde invalide

16 octobre 2026. Incident de livraison, pas un défaut du binaire produit. Source applicative b36c916 inchangée. Propriétaire déploiement `/root/configuration_deployer`, coordinateur `/root`.

## Observation et hypothèse confirmée

Premier essai de l'opérateur V3 privé : rc2 à la phase backup. Le wrapper runuser généré contient `if ! python ... <<'PY'`, puis son heredoc, mais pas `then` avant la branche d'erreur. L'analyse syntaxique du seul script générateur et son ancien self-test de parsing JSON ne couvraient pas ce fichier généré.

Cause confirmée par inspection de la sortie générée, pas une panne de sauvegarde Rust ou du NAS. Le binaire nouveau n'a pas été installé, aucune sauvegarde complète ou copie de rollback n'a été produite à cette phase. V2 a redémarré l'ancien service automatiquement.

## Restauration indépendante

`/root/configuration_release_verifier` : **APPROVED_SCOPED** restauration. Ancien hash installé et processus14d84a0478b9f394bfe59b3b88588edb4a9305dfedfc0fb0b66449b39d034169, service actif PID3399/NRestarts0 ; API datée etv1, webHTTP/HTTPS200, TLS strict. Empreintes configuration, unité, clé TLS, web installé et montage NAS conservées. Aucune modification de réglages, NAS ou client web. Journaux et opérateurs rejetés restent privés horsGit.

## Correction bornée et prévention

V3/V2 conservés immuables. V4 vérifie leur provenance, applique uniquement l'insertion manquante dans une copie temporaire privée puis extrait et vérifie syntaxiquement les deux wrappers générés **avant** toute exécution V2. Nouveau self-test : vraie invocation locale des wrappers contre des stubs sans service, états manquant/idle/actif/invalide ; ancien parser conservé. Nouvelle revue indépendante de cet opérateur exact exigée avant seconde tentative. Aucun redéploiement aveugle du script en échec.

Première révision V4 rejetée indépendamment : `bash -n script1 script2` ne vérifie que script1, script2 devenant un argument. Même si le self-test exécutait réellement les deux wrappers, le garde-fou de production promis était incomplet. Réparation demandée : deux commandes de parsing distinctes. Conserver également les scripts rejetés, et ne pas extrapoler un test de générateur à ses sorties.

V5SHA **a673a7a4d6494210f12140652363368787f3588e7486c2466405eb7083018c8d** : correction bornée des deux commandes et propagation du statut avec nettoyage des copies temporaires. Revue indépendante sécurité **APPROVED_SCOPED**, chaîne immuableV4/V3/V2 et tests réels des wrappers sur stubs PASS ; aucun Pi opéré par le reviewer.

État : **FIXED / déploiement corrigé rc0**21:14:22UTC, nouveau hash installé/processus confirmé ; [exécution](../releases/REL-CONFIGURATION-MATCHING-001.md) et [vérification distincte](../releases/REL-CONFIGURATION-MATCHING-001-verification.md). Les deux scripts rejetés et le premier incident restent conservés ; aucun test passé reconstitué rétroactivement.
