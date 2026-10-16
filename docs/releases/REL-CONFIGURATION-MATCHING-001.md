# Livraison Configuration / matching / tampon —16 octobre2026

Coordinateur `/root`, déployeur distinct `/root/configuration_deployer`, vérificateur distinct `/root/configuration_release_verifier`. Livraison familiale ciblée DEV_FAST explicitement autorisée ; pas de push, exposition publique ou remplacement du chantier web. Sources historiques, worktrees, bundles et tags conservés.

## Candidat et artefacts immuables

- Base66aef9c251cc150bd8d0cfe310ac6056d573766c ; source **b36c916f0ea24fe40fe90df61971fd3e8e6044a0**, branche `codex/configuration-matching`.
- Mac : `artifacts/macos/Chill Center-test-b36c916.app`,0.5.1/build6,81Mio. SHA256 exécutable **1c1f4bbe1b957ab593601041e5ea7807b7bc1263b27923cae82cde229305d344** ; release/signature ad-hoc/48dylibsarm64/libmpvAPI2.5 PASS.
- Linux : `artifacts/linux/chill-server-only-b36c916/{chill-server,manifest.json}`, binaire serveur seul ARM64/glibc2.28, SHA256 **dbda76e377a0f8ce84262396e6c339e44474846434351f9b4ec5bb5db4f5f100**. `cargo zigbuild --release --locked --offline -p chill-server --target aarch64-unknown-linux-gnu.2.28` et `linux_package.py validate-elf` PASS. Avertissement linkeroptimisation1 dépréciée sans échec.
- [Revues code/sécurité finales](../reviews/configuration-b36c916.md) APPROVED_SCOPED, [acceptation et suites](../implementation/CONFIGURATION-MATCHING-20261016.md). API2026-10-14/v1 additive, web installé inchangé. Le numéro interne partagé avec d'anciens tests ne suffit pas à identifier le bon bundle.

## Périmètre

Feedback TMDB immédiat/non bloquant, secours prudent pour suffixes de release, séries canoniques sans fusion d'homonymes, Configuration catégorisée/recherche et plusieurs emplacements. Absence finale vérifiée au lancement avec mise à jour ciblée/delta et retour stable ; écran noir avant nouvelle vidéo ; moteur/bail/tampon conservés en pause, progression/indicateur conservés et contrôles masqués après inactivité. Recommandations des fiches et conseils héritées du candidat de base.

## Exécution et garde-fous

Ancien serveur/processus SHA **14d84a0478b9f394bfe59b3b88588edb4a9305dfedfc0fb0b66449b39d034169**. Configuration statique/unité/TLS, réglages de bibliothèque, web installé et montage NAS épinglés avant bascule. Refus de connexions actives et jobs TMDB non terminés ; sauvegarde privée exacte et contrôlée avant remplacement atomique du seul binaire. Aucune écriture NAS ou réglage autorisée.

Premier essai V3 échoué en backup rc2 avant remplacement : [BUG0102](../debug/BUG-0102-private-deployment-wrapper-syntax.md). Ancien service redémarré et restauration indépendante APPROVED_SCOPED ; pas de sauvegarde complète produite à cette phase. V4 rejetée pour couverture syntaxique incomplète des fichiers générés. Scripts/logs privés conservés horsGit, sans publier IP, certificat, secret ou contenu catalogue.

Seconde tentative V5SHA **a673a7a4d6494210f12140652363368787f3588e7486c2466405eb7083018c8d**, revue sécurité indépendante APPROVED_SCOPED : démarrage21:09:27UTC, fin durable **21:14:22UTC, rc0**. Environ4min55 pour l'ensemble sauvegarde/bascule/contrôles, dont une attente I/O pendant la sauvegarde ; pas une durée de sauvegarde isolée mesurée. Service actif PID4060, NRestarts0, ExecMainStatus0 ; fichier installé **et** `/proc/4060/exe` = dbda76…f100.

Sauvegarde privée opaque **ff48d745-b7f9-48e5-a4d7-9ddacc97a763**, mode0700/compte service ; contrôles exacts par l'opérateur avant bascule. Ancien exécutable14d84…4169 conservé dans le répertoire privé de rollback. Empreintes après bascule : configurationd6167cb1624b6d5888473af76d5470102afedce25b2e29ddbaa032730cd0ceb1 ; unité76f7d34921ff344e344ba2bffa51263248ec6118dad9e15a39ef7078877d72e2 ; SPKI3018fb4d28af3899ae9a0261c89ef92d7ebfe508ecbd613b38f6dd6d7d4740df ; web07868e61cd75dc36d5bf62eefd258b7409b6ed264bde726cdcf626d45b57fe2b ; montagebdfed6860cca749f250293cab6df1ace5b2ba0457e246afd956fa0d1245cb92a, toujoursCIFSlecture seule. `library.json` toujours absent, inchangé. Journaux/rc/opérateurs et chemins privés restent horsGit.

[Vérification post-déploiement indépendante](REL-CONFIGURATION-MATCHING-001-verification.md) : verdict et preuves exactes gouvernent la remise ; l'opérateur ne s'auto-certifie pas.

Verdict final **APPROVED_SCOPED**21:16:08UTC : manifeste de sauvegardeSHA **f3deafece5f28763eeee4454bb8fae6d4561059aae6eda0c42cfad0b8ed4a654**,1905entrées vérifiées et1SQLitequick_check PASS. Six sondes API/web200, aucune entrée journal warning+ depuis démarrage. Cas Avatar autorisé :503MEDIA_FILE_MISSING sans bail, puis état disponible=false/révision2 persisté ; unique écriture applicative ciblée du vérificateur, pas un scan ou une lecture vidéo. RSS15064Kio ponctuel, pas une mesure de gain d'optimisation.

## Retour arrière

Avant toute nouvelle utilisation des réglages multi-emplacements, rollback opérateur conditionnel vers l'ancien binaire sauvegardé, redémarrage, sondes API/web strictTLS et comparaison des invariants ; pas de restauration aveugle de données utilisateur concurrentes. Une fois `library.json` enrichi par le nouveau client, un ancien serveur à désérialisation fermée peut exiger le fichier de réglages ancien sauvegardé : traiter explicitement, sans écraser progression/historique ultérieurs ni NAS. Ne jamais relancer aveuglément un opérateur ayant échoué après une bascule potentiellement réussie.

## Limites conservées

Tests ciblés Rust/Node/DOM et libmpv synthétique ne sont pas une nouvelle validation humaine Tauri ni un benchmark de deux/trois flux Pi/Wi-Fi/NAS. Imports multi-espaces refusés en attendant un mapping explicite ; import historique principal conservé. [Audit ressources](../performance/RESOURCE-AUDIT-20261016.md) en lecture seule : propositions, aucun nettoyage/compactage, aucun gain RAM prétendu.
