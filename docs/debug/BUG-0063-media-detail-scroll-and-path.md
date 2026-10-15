# BUG-0063 — défilement derrière une fiche et chemin source absent

2026-10-15, base initiale `eb73711`, branche `codex/filename-parsing`, reprise isolée71bd7a0 sur la base livrée5bebf10. État courant : FIXED / HUMAN_TAURI_PASS / DELIVERED c610ea9. [Release et vérification](../releases/REL-FILENAME-SCROLL-001.md).

Validation humaine du15octobre : « test concluant sur le scrolling de mon côté ». Fenêtre Tauri debug isolée réellement ouverte, `MANUAL_SCROLL_READY`, données synthétiques et réglages privés `/private/tmp/chillcenter-manual-scroll.elhYaH/client`, client Rust déconnecté, aucun lecteur. Le test porte sur le correctif71bd7a0 d'origine, pas les variantes expérimentales28b1c64. Les gestes individuels ne sont pas instrumentés ; aucune réussite automatique du harnais n'est prétendue. Le précédent échec automatique reste conservé dans l'historique de livraison et sur la branche diagnostique.

## Symptômes et cause

H1 confirmée par navigateur : la fiche utilise un dialogue modal, mais rien ne bloque le défilement de la racine. Arrivée en bas, la molette enchaîne vers le catalogue derrière : position verticale 500 → 1400 dans la régression Chromium avant correction. La fiche ne rendait pas non plus le champ `media.path` déjà fourni par le contrat.

## Correction

CSS : verrouiller overflow/overscroll de la racine et du body lorsqu'un dialogue est ouvert ; permettre son défilement interne avec `overscroll-behavior: contain`. Pas de body fixé ni de remise à zéro du scroll ; fermeture = restauration naturelle du défilement catalogue.

Fiche : afficher `media.path` via `textContent`, sans interpréter le nom comme HTML, URL ou accès disque. C'est le chemin **relatif** au dossier vidéo serveur : `Store::profile_view` convertit déjà le chemin privé. Aucun accès NAS côté client ni nouvelle divulgation du montage absolu. Masquer le champ s'il est absent.

## Preuve et limites

`scripts/test-media-detail.cjs` charge la véritable interface/CSS avec un pont Tauri synthétique, 160 vidéos et une longue fiche fictive. HTTP(S) bloqués, profil navigateur headless jetable, aucun compte ou onglet utilisateur. Vérifie chemin affiché, scroll interne, absence de propagation en bas/sur backdrop, puis défilement rétabli après fermeture. Aucun besoin de lecture vidéo.

```sh
NODE_PATH=/Users/ghillaumesoubigou/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules node scripts/test-media-detail.cjs
node --test ui/connected/tests/series-command.test.js ui/connected/tests/series-tmdb.test.js ui/connected/tests/collection-pages.test.js ui/connected/tests/catalog-query.test.js ui/connected/tests/saga-loading.test.js
```

Chromium : RED avant patch, GREEN après. 7 tests Node voisins PASS sur la branche initiale. Premier lancement navigateur refusé par le sandbox macOS (MachPort) : incident d'outillage, pas RED produit ; lancement isolé ensuite autorisé. WebKit Playwright non installé : NOT_AVAILABLE. La preuve native distincte est désormais le test humain Tauri ci-dessus, pas une validation Playwright/WebKit ni un PASS du simulateur AppKit.

Au moment du correctif initial : pas de nouveau bundle ni de déploiement. Depuis la validation humaine : bundlec610ea9 publié et serveur mis à jour ; configuration et données préservées, sauvegarde/rollback documentés dans la release.
