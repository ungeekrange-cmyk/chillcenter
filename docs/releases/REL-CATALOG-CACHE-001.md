# REL-CATALOG-CACHE-001 — corrections ciblées catalogue et profils

2026-10-14, actualisé2026-10-15. État : DELIVERED / VERIFIED_SCOPED. Source client `7a0f480a286d00729af3d8f04b6300c4eeade499`, serveur correctif `cfc87dec21aee553d72f5016f6c9dfe600a6e1f3`, branche `codex/catalog-cache-and-client-fixes`, base utilisateur `ae0ce44`. Livraison locale ciblée, pas une validation globale des US.

## Résultat

Administration : retour immédiat de progression/rechargement/enrichissement, erreurs réelles sans faux succès. Sagas : membres visibles sans attendre l'image ; commandes mises en file au lieu de faux conflit occupé. Listes : libellés adaptés, mosaïque des neuf premières affiches, image personnalisée. Profil : nom et menu sur rond initiale. Catalogue complet et images persistants par serveur/profil ; navigation/fiches/suggestions locales, vérification de révision au démarrage/focus/rechargement et après mutations, fallback périmé explicite sans répétition réseau à chaque clic. Suppression locale immédiate après confirmation de suppression du profil, même si le GET suivant échoue.

Suite d'épisodes : vignette cliquable, nouvelle saison signalée, annuler ferme le lecteur, défaut20s pour les nouveaux profils Mac, choix0/5/10/15/20/30/off. Les profils existants gardent leurs réglages. API2026-10-14 conservée avec ajouts ; ancien défaut API10s préservé. Guide tiers et OpenAPI70chemins mis à jour.

## Artefacts immuables

- Mac : `artifacts/macos/Chill Center-7a0f480.app`,77Mio,48dylibsARM64 autonomes, signature ad-hoc stricte et loader libmpvAPI2.5 PASS. Exécutable SHA256 `eabc8099459bf413e98dd862748b6e8630f5fb83bb002b97f0b2b2286d241a40`. Pas notarized. Build hors réseau Rust1.99/Cargo.lock, deux warnings préexistants de helpers release inutilisés.
- Serveur initial, historique après correctif : `artifacts/linux/chillcenter-server-07358f7-pi64.tar.gz`, archive SHA256 `110e8fd9226eb03d2fe23e46cf336fcae7f3de02f7a88b12261def655ea8162c`. Binaire SHA256 `11cb14459c0f1339a9bbf6889fa4e85e997b5214a7fc12e40e651e1a42122424`, ELFARM64/glibc2.28, source compilée `07358f73b5a6e378f1c3c0aad45cf6e4efc65aaf`. cargo-zigbuild0.23.4/Zig0.17 ; warning linker historique conservé. Ne plus installer cet artefact, remplacé parCFC ci-dessous.
- OpenAPI SHA256 `01423e6a3ace79453671a59cb67674db2c575b27bd295a1c2a83965906d9acae`.

Correctif serveur ciblé après première vérification : source `cfc87dec21aee553d72f5016f6c9dfe600a6e1f3`, archive `artifacts/linux/chillcenter-server-cfc87de-pi64.tar.gz`, SHA256 `a8d9549abfbf6e1cc822e6229450769931167e7af24442f59e6b26386a1067ea`, binaire `f85375ad4ac33fd0993151b547e8ca731d0ffa49441beb8b85756f0433f842cd`. BuildARM64/glibc2.28 horsligne PASS, même warning linker préexistant. Seul delta produit depuis le client7a0 : initialisation transactionnelle des révisions de sagas héritées dans le catalogue serveur ; API/DTO/UI inchangés. Le bundle Mac7a0 est conservé, pas de nouveau build ni parcours lecteur injustifié.

Les bundles8d/073 sont intermédiaires non remis. Anciens bundles utilisateur conservés ; aucune installation /Applications ni ouverture sur les réglages réels. Le HTML file:// est un aperçu sans pont Rust, pas le client web connecté.

## Vérification ciblée

[Revues indépendantes](../reviews/catalog-cache-final.md), [preuves détaillées](../implementation/CATALOG-CACHE-EVIDENCE.md), ADR0013 et BUG0055–0058. Derniers tests après restriction utilisateur :8Node sagas/mosaïque/profil/TMDB,1Node ordre Administration, contrat70routes, test scanner deuxième connexion, test serveur miroir cohérent, navigation locale/cache supprimé/symlinks et clippy ciblés PASS. Aucun nouveau parcours général lecteur, aucune vidéo NAS lancée.

## Pi — historique du report, puis déploiement effectif

Le14octobre, après autorisation sauvegarder/arrêter/remplacer/redémarrer, deux tentatives opérateur ont refusé l'arrêt avant toute mutation du service : connexions HTTPS sortantes du processus. L'ancien servicePID5145/hash `1eb24e97bcdf8f66440c1681af92d6202c151ebe336c2890ea42a7f8179ac181`,NRestarts0 avait été préservé, sans nouvelle sauvegarde à ce stade.

Diagnostic lecture seule du14octobre : enrichissement TMDB running1348/2496, autre queued0/2483 au dernier relevé. Aucun jeton ni titre affiché/copié ; aucune nouvelle tâche TMDB/IA lancée. Question utilisateur attendre/interrompre posée. Le15octobre, réponse explicite : « Non déploie la nouvelle version ». Avant bascule, aucune connexion établie du processus ; le script revu a pu être exécuté sans changement de garde.

Déploiement par `/root`, procédure ciblée DEV_FAST/ship-deployer et script revu SHA256 `1b4169cf81dedfb818f68226e3d9e8ba75a339f3c5c022f0e8f36f33e2a65bd6` depuis le dossier privé `/var/tmp/chillcenter-cache-07358f7/` : arrêt, ancienne CLI backup hors ligne, contrôle manifeste/hashes/SQLite quick_check, conservation de l'ancien binaire sans écrasement, remplacement atomique puis redémarrage. Exit0 à **2026-10-15T02:06:48Z**, PID6451, hash installé `11cb14459c0f1339a9bbf6889fa4e85e997b5214a7fc12e40e651e1a42122424`.

Sauvegarde privée vérifiée **sur le Pi uniquement** : `/var/lib/chillcenter/data/backups/c8897485-a1b3-4a44-8afa-8711a32ec7f7`. Rollback binaire créé `/root/chillcenter-rollback/chill-server-9277ac4-1eb24e97bcdf8f66`, hash1eb24… contrôlé. Anciens rollbacks/sauvegardes préservés. Config/unité et SPKI TLS contrôlés identiques avant/après par l'opérateur ; NAS/SSH/montage inchangés. Trois essais curl pendant le démarrage ont eu connexion refusée avant ouverture du listener ; boucle bornée aboutie puis second contrôle HTTPS strict daté PASS, pas de contournement du certificat.

La vérification initiale du vérificateur distinct `/root/catalog_pi_verifier` a échoué sur le catalogue/sagas malgré le démarrage réussi : preuve conservée ci-dessous. La vérification corrective finale passe, sans lecture vidéo, nouveau scan, tâche fournisseur ou commande de mutation du foyer.

## Première vérification défavorable et correction ciblée

[Rapport indépendant initial](REL-CATALOG-CACHE-001-pi-verification.md) : identité/service/TLS/API PASS, mais GETsagas et POSTsnapshot422, donc client froid FAIL. Données réelles37sagas/30révisions absentes. [BUG0059](../debug/BUG-0059-lazy-saga-revisions.md) corrigé dansCFC : révisions1 absentes initialisées dans la transaction d'import lazy, CAS existants et métadonnées conservés. TestRED/GREEN1PASS, test serveur miroirPASS, clippy cataloguePASS. Revue code `/root/review_cache_server` et sécurité `/root/review_sidecar_quarantine` APPROVED exactesCFC, avec limites explicites de rerun.

Le rapport relève aussi deux auxiliaires SQLite vides non déclarés dans la sauvegarde ; [BUG0060](../debug/BUG-0060-backup-empty-sidecars.md). Quarantaine privée réversible après revue9270f9… et contrôles exacts, exit0 : manifeste/base inchangés, aucun fichier supprimé, aucun service touché. Script de seconde bascule `fd162e9920b0d9c3f0ca55c97cf132cb96f8909d4f8660da64709855c7f2cedc` revu intégralement APPROVED : nouvelle sauvegarde offline, contrôle fichiersexacts/SQLiteimmuable, rollback distinct073. Les échecs initiaux ne deviennent pas rétrospectivement des PASS.

Déploiement correctif réussi à **2026-10-15T02:31:51Z**, exit0, PID6808, hashf85375… exact. Nouvelle sauvegarde privée `/var/lib/chillcenter/data/backups/8bbb9079-37f5-4c0f-8a8d-cbab15974a22` vérifiée par l'opérateur : tous les hashes/tailles, quick_checkimmuable et jeu exact de fichiers PASS (`PRIVATE_BACKUP_VERIFIED_EXACT`). Rollback073 créé sans écraser l'ancien : `/root/chillcenter-rollback/chill-server-07358f7-11cb14459c0f1339`. Deux connexions curl refusées avant disponibilité du listener puis HTTPS strict PASS ; config/unité/SPKI identiques avant/après.

Vérification indépendante finale **APPROVED_SCOPED**,02:33–02:34UTC : hash installé/exécuté f853… exact, PID6808/NRestarts0 stables, SPKI inchangée, zéro journal warning+, RSS après sonde29652Kio. Sauvegarde corrective0700/1710fichiers déclarés=réels/zérosymlink et rollback073 vérifiés. Sonde réelle UiClient dans un dossier neuf privé, une seule exécution sans retry/skip : exit0/9,16s, `CLIENT_CACHE_OK total=2496 local_browse_ms=4426 elapsed_ms=9123 pinned_tls=true server_unchanged=true warm_reopen=true household_writes=false`. Catalogue complet/fiche/sagas/listes, vérification inchangée et réouverture chaude PASS ; pas une preuve de latence par carte ni un test GUI/lecteur. L'absence de commandes de mutation du harnais n'exclut pas l'initialisation SQLite interne et revue des révisions manquantes. [Gate post-déploiement](../gates/G-POST-CC-CATALOG-CACHE-001.md) ouvert par ce verdict distinct ; aucun rollback recommandé.

## Retour et réserves

Retour : fermer le nouveau Mac, arrêter le service, réinstaller l'ancien binaire vérifié et redémarrer ; sauvegarde offline pré-bascule nécessaire pour retour intégral des données/archives2. Ne jamais restaurer la DB ou effacer des données sans validation explicite. NAS exclu de toute procédure.

Reste : GC des affiches serveur orphelines dans certains imports/suppressions de profil(P2), validation visuelle humaine, fournisseurs réels hors tâche déjà en cours, charge/latence Pi-WiFi/NAS et réserves son/HDR/veille/multi-écran historiques. Pas de claim « toutes les US validées ».
