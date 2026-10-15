# BUG-0063 — défilement derrière une fiche et chemin source absent

2026-10-15, base `eb73711`, branche `codex/filename-parsing`. État : FIXED_LOCAL, NON_LIVRÉ ; confirmation Tauri/WebKit restante.

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

Chromium : RED avant patch, GREEN après. 7 tests Node voisins PASS. Premier lancement navigateur refusé par le sandbox macOS (MachPort) : incident d'outillage, pas RED produit ; lancement isolé ensuite autorisé. WebKit non installé : NOT_AVAILABLE, aucune installation tentée. Ce test ne prouve pas le rendu natif Tauri/Safari, qui reste à confirmer ; aucune revue indépendante revendiquée.

Pas de nouveau bundle, de déploiement Pi, de modification de configuration ni de données. Rollback : retirer les seuls changements CSS et champ de fiche de ce patch.
