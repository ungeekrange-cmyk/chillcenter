# REL-CATALOG-CACHE-001 — corrections ciblées catalogue et profils

2026-10-14. État : BUILT_REVIEWED / PI_UPDATE_WAITING_FOR_TMDB. Source client `7a0f480a286d00729af3d8f04b6300c4eeade499`, branche `codex/catalog-cache-and-client-fixes`, base utilisateur `ae0ce44`. Livraison locale ciblée, pas une validation globale des US.

## Résultat

Administration : retour immédiat de progression/rechargement/enrichissement, erreurs réelles sans faux succès. Sagas : membres visibles sans attendre l'image ; commandes mises en file au lieu de faux conflit occupé. Listes : libellés adaptés, mosaïque des neuf premières affiches, image personnalisée. Profil : nom et menu sur rond initiale. Catalogue complet et images persistants par serveur/profil ; navigation/fiches/suggestions locales, vérification de révision au démarrage/focus/rechargement et après mutations, fallback périmé explicite sans répétition réseau à chaque clic. Suppression locale immédiate après confirmation de suppression du profil, même si le GET suivant échoue.

Suite d'épisodes : vignette cliquable, nouvelle saison signalée, annuler ferme le lecteur, défaut20s pour les nouveaux profils Mac, choix0/5/10/15/20/30/off. Les profils existants gardent leurs réglages. API2026-10-14 conservée avec ajouts ; ancien défaut API10s préservé. Guide tiers et OpenAPI70chemins mis à jour.

## Artefacts immuables

- Mac : `artifacts/macos/Chill Center-7a0f480.app`,77Mio,48dylibsARM64 autonomes, signature ad-hoc stricte et loader libmpvAPI2.5 PASS. Exécutable SHA256 `eabc8099459bf413e98dd862748b6e8630f5fb83bb002b97f0b2b2286d241a40`. Pas notarized. Build hors réseau Rust1.99/Cargo.lock, deux warnings préexistants de helpers release inutilisés.
- Serveur : `artifacts/linux/chillcenter-server-07358f7-pi64.tar.gz`, archive SHA256 `110e8fd9226eb03d2fe23e46cf336fcae7f3de02f7a88b12261def655ea8162c`. Binaire SHA256 `11cb14459c0f1339a9bbf6889fa4e85e997b5214a7fc12e40e651e1a42122424`, ELFARM64/glibc2.28, source compilée `07358f73b5a6e378f1c3c0aad45cf6e4efc65aaf`. La source finale7a0 ne change que deux fichiers client/manifest ; identité des sources serveur/domaine/API vérifiée indépendamment, aucune recompilation serveur artificiellement revendiquée. cargo-zigbuild0.23.4/Zig0.17 ; warning linker historique conservé.
- OpenAPI SHA256 `01423e6a3ace79453671a59cb67674db2c575b27bd295a1c2a83965906d9acae`.

Les bundles8d/073 sont intermédiaires non remis. Anciens bundles utilisateur conservés ; aucune installation /Applications ni ouverture sur les réglages réels. Le HTML file:// est un aperçu sans pont Rust, pas le client web connecté.

## Vérification ciblée

[Revues indépendantes](../reviews/catalog-cache-final.md), [preuves détaillées](../implementation/CATALOG-CACHE-EVIDENCE.md), ADR0013 et BUG0055–0058. Derniers tests après restriction utilisateur :8Node sagas/mosaïque/profil/TMDB,1Node ordre Administration, contrat70routes, test scanner deuxième connexion, test serveur miroir cohérent, navigation locale/cache supprimé/symlinks et clippy ciblés PASS. Aucun nouveau parcours général lecteur, aucune vidéo NAS lancée.

## Pi — bascule non encore effectuée

Autorisation explicite sauvegarder/arrêter/remplacer/redémarrer reçue. Deux tentatives opérateur ont refusé l'arrêt avant toute mutation du service : connexions HTTPS sortantes du processus. Le service ancien reste actifPID5145/hash `1eb24e97bcdf8f66440c1681af92d6202c151ebe336c2890ea42a7f8179ac181`,NRestarts0. Pas de nouvelle sauvegarde revendiquée.

Diagnostic lecture seule : un enrichissement TMDB running1348/2496, un autre queued0/2483 au dernier relevé. Aucun jeton ni titre affiché/copiat ; aucune nouvelle tâche TMDB/IA lancée. Un arrêt convertirait les tâches non terminales en échec relançable, sans effacer les fiches déjà enregistrées. Question utilisateur posée : attendre ou interrompre pour mettre à jour. Ne pas contourner silencieusement ce garde.

Fichiers préparés privés `/var/tmp/chillcenter-cache-07358f7/` surPi, sans installer le binaire. Script revu SHA1b4169… ; nouvelle sauvegarde offline contrôlée et rollback non écrasant `/root/chillcenter-rollback/chill-server-9277ac4-1eb24e97bcdf8f66` prévus mais PAS encore créés. Config/unité/TLS/NAS inchangés. SSH partagé utilisateur conservé.

Après bascule autorisée sûre : sonde réelle UiClient avec TLS épinglé vérifié SSH, bases client privées, catalogue complet/fiche/sagas/listes et réouverture inchangée ; pas de lecture vidéo ou mutation du foyer. Vérification indépendante post-bascule encore NOT_RUN. Ne pas proposer de lancer le nouveau Mac contre l'ancien serveur qui n'a pas les routes snapshot.

## Retour et réserves

Avant bascule, rien à restaurer surPi. Après bascule : fermer le nouveau Mac, arrêter le service, réinstaller l'ancien binaire vérifié et redémarrer ; sauvegarde offline pré-bascule nécessaire pour retour intégral des données/archives2. Ne jamais restaurer la DB ou effacer des données sans validation explicite. NAS exclu de toute procédure.

Reste : GC des affiches serveur orphelines dans certains imports/suppressions de profil(P2), validation visuelle humaine, fournisseurs réels hors tâche déjà en cours, charge/latence Pi-WiFi/NAS et réserves son/HDR/veille/multi-écran historiques. Pas de claim « toutes les US validées ».
