# BUG-0051 — parsing de sortie dans la commande de mise à jour Pi

2026-10-14. Environnement : Pi Linux ARM64, service R7, source compilée nouvelle `9277ac4d51281b3e6d72475294b1e7822cb8e913`. Défaut **opérateur**, pas une régression du serveur ni du lecteur.

## Attendu et observation

Après autorisation humaine explicite, arrêter le service, effectuer une sauvegarde privée offline, préserver l'ancien binaire, remplacer seulement l'exécutable et redémarrer. La première tentative a bien créé la sauvegarde, mais l'expression `sed` transportée par plusieurs niveaux de quoting SSH a échoué avant toute substitution du binaire.

R7 a été vérifié puis immédiatement redémarré à `2026-10-14T18:45:50Z`, PID4980, hash `9a17ffe55e3d4128b9d483d170f5d2ede71382cef42d8637917c8f31e63b36a6`, `NRestarts=0`. Aucun restore de données, changement NAS/configuration/unité/TLS ou suppression de profil. La sauvegarde `ba950d51-fc88-49ee-b801-4641fac531ff` est un répertoire privé sur le Pi, pas un fichier archive à copier.

## Diagnostic falsifiable

H1 : le sur-échappement dans l'argument SSH, et non la sortie française ni le moteur de sauvegarde, altère l'expression du parseur. Expérience locale sur une sortie synthétique : expression intacte → extraction du chemin attendu PASS ; argument avec le même sur-échappement → erreur de syntaxe du parseur RED. Cause confirmée ; aucune autre hypothèse ou modification du produit nécessaire.

Vérification indépendante de la sauvegarde : parents et racine0700 UID997, manifeste final format1/schema3, cinq entrées uniques/six fichiers et tailles cohérentes, aucun symlink, SQLite `quick_check=ok` en lecture seule immutable. Le fichier SQLite0644 reste privé derrière les ancêtres0700. Aucun secret ou contenu de profil affiché.

## Correction et revue

Un déployeur distinct a remplacé le parseur par un préfixe littéral et une validation ancrée du chemin UUID, dans un script Bash transmis par stdin : aucun `sed`, `eval` ou niveau de quoting supplémentaire autour du script. Les variantes V2/V3 non exécutées restent historiques : la revue a aussi exigé un rollback armé avant l'arrêt, le refus des symlinks avant mutation, une copie ancienne sans écrasement, des traps HUP/INT/TERM et une restauration même après un échec post-redémarrage.

V4 exacte SHA256 `55c6be484d50a8065115ae4b1bf867377c24b255640c033e5fc2faf6f3289587`, 6933octets : revue indépendante `/root/pi_operator_review` **APPROVED**, `bash -n`, auto-test parseur, provenance et scan secrets PASS. Aucun HIGH/CRITICAL ouvert. Toute modification de ces octets invalide l'approbation. ShellCheck indisponible, non revendiqué.

V4 exécutée à19:12:40UTC : bascule réussie, nouveau binaire1eb24…/PID5145 ; [vérification post-déploiement indépendante APPROVED](../releases/REL-UX-API-001-pi-verification.md) dans le périmètre connexion/catalogue/runtime/backup. BUG opérateur **VERIFIED** par exécution réelle ; la correction syntaxique seule n'aurait pas suffi. [REL-UX-API-001](../releases/REL-UX-API-001.md). L'ancien script et les rapports défavorables sont conservés ; aucun rerun aveugle, reset de base ou modification du NAS n'est autorisé.

## Autre symptôme associé, sans bug client

Avant bascule, le Pi répondait200 sur `/api/v1/server` mais404 sur `/api/2026-10-14/server`. La sonde utilisant le même `UiClient` que Tauri a reproduit `HTTP_ERROR status=Some(404)` avec des réglages privés temporaires. Cela explique l'écran de connexion du nouveau client face à R7 ; aucune désactivation de TLS ni fallback silencieux du contrat daté n'est justifiée. Après mise à jour, même code : connexionAPI2026-10-14, catalogue48 et reconnexion PASS/exit0 ; répétés indépendamment sur un autre dossier privé frais, sans écrire sur le serveur. Pas une revendication de clicGUI ou lectureNAS.

Rollback : revenir au seul binaire R7 vérifié et à l'ancien client, sans restaurer automatiquement la base ou la configuration. Sauvegardes privées et ancien exécutable restent sur le Pi.
