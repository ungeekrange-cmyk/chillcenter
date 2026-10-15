# Compatibilité Pi / Mac11aa529 / client web

Contrôle du15octobre2026, demandé sans redéploiement. Ne pas confondre la vérification du Pi et le candidat intégré localement.

## Pi actif — lecture seule

SSH à21:11:21UTC : service actif/running, PID1634, NRestarts0, démarrage20:22:10UTC. Fichier installé et processus actif :

```
ffcc634923f637331cac25fec1e5283b062ef5e2943778bcab59460b6269b0f7
```

Identique au binaire Linux ARM64 du worktree `codex/web-pwa` ; sources/documents inspectés à `aad34cec60974782df959cf44c668e470fc4c658`. Le serveur n'embarque pas de SHA Git public : correspondance d'artefact vérifiée, pas preuve d'une compilation reproductible de ce commit.

- API : `/api/2026-10-14`, alias `/api/v1`, package0.1.0 ; OpenAPI actif70 chemins, identique au contrat du worktree web.
- HTTPS192.168.1.42:8443 et HTTP192.168.1.42:8080 actifs ; pages web via443/80 accessibles. Redirections et répertoire statique existants conservés.
- SHA configuration : `d6167cb1624b6d5888473af76d5470102afedce25b2e29ddbaa032730cd0ceb1`.
- SHA unité : `76f7d34921ff344e344ba2bffa51263248ec6118dad9e15a39ef7078877d72e2`.
- TLS SPKI : `3018fb4d28af3899ae9a0261c89ef92d7ebfe508ecbd613b38f6dd6d7d4740df`.

Aucun arrêt/redémarrage, remplacement de binaire, sauvegarde, modification NAS/configuration/TLS/pare-feu ou changement de données du foyer pour ce contrôle. Les captures de synchronisation historiques restent des opérations temporaires du protocole, pas des mutations du catalogue.

## Mac livré et compatibilité constatée

Bundle à utiliser : `artifacts/macos/Chill Center-11aa529.app`, source `11aa529f2eced060156a3017456bf1d9634f8a56`. SHA exécutable : `d07bc889e077c1e2710caab01b66d9058941ee6ac11e5938e113667da31ec59d`.

Sonde Rust réelle utilisant le même `UiClient`11aa, cache de test privé distinct des réglages utilisateur, TLS épinglé vérifié, aucun appel fournisseur ni lecture vidéo :

```
PINNED_CONNECT_OK cold_ms=5979 offline=false
LOCAL_CATALOG_OK total=2496 detail_sagas_playlists=true
DETACHED_SYNC_OK elapsed_ms=238 changed=false discarded=false legacy_fallback_expected=true
LOCAL_SERIES_READ_OK revision_present=false episodes=53
WARM_REOPEN_OK elapsed_ms=1100 offline=false household_mutations=false playback=false provider_calls=false
```

Compatibilité **partielle**, pas approbation globale du client :

| Fonction | Pi ffcc6349 avec Mac11aa529 |
| --- | --- |
| Connexion, catalogue, fiches, sagas, listes, cache local | Vérifiés par vrai UiClient |
| Synchronisation détachée | Fonctionne via repli historique ; route delta absente |
| Édition série/saison, identité des homonymes | Correctifs récents non présents ; révision de série absente |
| Affiches personnalisées série/saison | Nouvelles routes absentes |
| Lecteur | Contrats conservés en source, pas de nouvelle lecture réelle |

Les quatre familles de routes Mac manquantes sont le POST `catalog/changes` (et son GET de pages), les PUT `media/{media}/series/poster` et `media/{media}/season/poster`, et le GET `series/{tv}/seasons/{season}/artwork`. Les erreurs404 ont été distinguées des erreurs de profil. IA CLI, TMDB réel, rendu graphique du bundle, performance NAS/Wi-Fi et tests multiprofils vidéo non rejoués.

## Intégration locale — NON DÉPLOYÉE

Branche `codex/mac-web-compat-20261015` : fusion de `37a5b3596cbac757694c83eaccfe805e2c09b008` (Mac11aa + preuve documentaire) et `aad34cec60974782df959cf44c668e470fc4c658` (web). Aucun checkout utilisateur changé. Le fichier non suivi `web/src/catalog-query.js` du worktree web est préservé et exclu : seuls ses commits ont été intégrés.

La fusion conserve serveur statique, listener HTTP optionnel, TLS, protections d'origine et limites HTTP du web, tout en ajoutant les corrections serveur Mac de synchronisation et de séries. Ajout compatible à l'API2026-10-14 : GET `catalog/revision`, sans transfert ni matérialisation de fiches. OpenAPI local76 chemins ; le contrat actif du Pi reste70. Adaptateur web mis à jour pour les commandes communes, les révisions et les images tardives : [BUG0069](../debug/BUG-0069-web-bridge-catalog-sync.md).

Le web n'a toujours pas de miroir SQLite/IndexedDB et conserve son lecteur navigateur. IA, recherches TMDB et choix natif de fichiers non portés dans le web restent hors de ce correctif ; aucune parité complète revendiquée. Les ADR acceptés des deux branches sont conservés, y compris leurs numéros historiques14/15 dupliqués : citer leurs noms complets.

Ne pas installer l'ancien artefact serveur11aa isolément : il ignore `http_listen` et ne contient pas le web. Une future livraison nécessitera un serveur intégré, ses fichiers web, la configuration existante et un retour arrière coordonné. **Aucune livraison autorisée/exécutée ici**. Pas de nouveau bundle Mac : le code natif livré11aa reste inchangé.

## Vérifications locales ciblées

Dans le clone isolé, Rust verrouillé/hors ligne, aucun téléchargement :

```
cargo test --locked --offline -p chill-catalog -p chill-domain -p chill-client -p chill-server --lib --quiet
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
node --test web/test/bridge.test.js ui/connected/run-availability.dom.test.js ui/connected/ai.test.js ui/connected/ai.dom.test.js ui/connected/catalog-loading.dom.test.js ui/connected/admin-navigation.dom.test.js ui/connected/sync.dom.test.js ui/connected/tmdb.dom.test.js ui/connected/tests/*.test.js
node scripts/validate-api-contract.mjs
node scripts/build-web.mjs /private/tmp/chillcenter-compat-readonly.YrFzIc/web-dist
git diff --check
```

Résultats :293 tests Rust PASS (75catalogue/39client/64domaine/115serveur),56Node PASS (15bridge/41interface), Clippy sans avertissement, contrat76 chemins PASS, build web54 fichiers identifiant636b424253f4, diff sans erreur. Le premier essai Rust en sandbox refusait le bind loopback d'un test de connexion ; suite relancée avec permission loopback,293/293 PASS. Ce refus d'outillage n'est pas un bug serveur.

Ces tests couvrent routage et protections existantes, nouveau GET (profil/révision/alias/absence de checkpoint), commandes bridge et réponses tardives. Ce n'est ni un test graphique complet PWA/Mac ni un déploiement simulé du nouveau binaire sur le Pi. Revue indépendante ciblée consignée séparément après gel du code.
