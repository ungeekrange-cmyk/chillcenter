# Préparation GitHub Pages — 16 octobre 2026

État : **préparé localement, non publié**. Le site est une présentation statique, pas le client web/PWA et pas une passerelle vers le serveur du foyer. Aucun déploiement Pi ou Mac n’est nécessaire à ce lot.

## Frontière de publication

`scripts/build-public-site.mjs` assemble uniquement :

| Source | Destination publique |
| --- | --- |
| `site/index.html`, `api.html`, `404.html`, `styles.css`, `.nojekyll` | racine du site |
| `icons/branding/vector/default.svg` | `assets/logo.svg` |
| `docs/api/openapi-2026-10-14.json` | `api/openapi-2026-10-14.json` |
| `LICENSE` | `LICENSE.txt` |

Un manifeste SHA-256 est généré. Aucun autre fichier du dépôt, configuration, base, journal ou document de travail n’est copié. Les affiches du site sont des illustrations CSS fictives, pas des médias du foyer ni des images TMDB.

La construction refuse les liens symboliques dans les chemins inspectés, les liens physiques multiples sur les fichiers de sortie, les fichiers non gérés déjà présents dans la destination, les chemins personnels, les adresses réseau privées et certains formats de clés connus. Elle ne supprime rien récursivement et ne révèle pas les valeurs rejetées dans ses diagnostics. Le contrôle est un garde-fou, **pas un audit exhaustif de secrets ou de l’historique Git**. Les fichiers dérivés autorisés sont remplacés à chaque construction ; ne pas les éditer manuellement. Ne pas modifier les chemins de sortie en parallèle pendant une construction.

## Prévisualiser et vérifier

Depuis la racine :

```sh
node --test scripts/test-public-site.mjs
node scripts/build-public-site.mjs
```

Ouvrir `dist/public/index.html`. Les chemins relatifs permettent un hébergement sous `/chillcenter/`, sans domaine personnalisé ni serveur applicatif. Pour une prévisualisation HTTP locale facultative : `python3 -m http.server 8080 --bind 127.0.0.1 --directory dist/public` ; arrêter le processus après usage.

Le test navigateur facultatif `scripts/test-public-site-browser.cjs` attend un Playwright déjà disponible (`CHILL_PREVIEW_NODE_MODULES`), un navigateur (`CHILL_CHROME_PATH` facultatif) et un répertoire absolu `CHILL_PUBLIC_SITE_EVIDENCE`. Il contrôle les deux tons, plusieurs largeurs, les liens et l’absence de chargement distant. Aucune installation de dépendance du produit n’est ajoutée.

## Publication future : décision humaine requise

1. Faire valider le rendu et le contenu publics. Vérifier les droits du logo, les liens et l’OpenAPI exporté.
2. **Auditer séparément la visibilité du dépôt et tout son historique** avant de le rendre public. Des documents internes contiennent encore des chemins, adresses et éléments d’exploitation du foyer. Nettoyer le README ne retire rien de l’historique. Ne pas changer la visibilité et ne pas réécrire l’historique implicitement.
3. Choisir explicitement le dépôt/plan GitHub utilisé pour Pages. Un dépôt privé peut avoir un site public selon le plan et les réglages ; ne pas confondre visibilité du site et du code. À défaut d’un plan adéquat, envisager un dépôt de publication ne contenant que l’artefact relu, après approbation.
4. Intégrer les fichiers relus sur la branche par défaut, puis configurer **Settings → Pages → Source → GitHub Actions**. Cette opération n’a pas été effectuée. Définir les règles de protection de l’environnement `github-pages` et les approbateurs disponibles selon le plan.
5. Lancer manuellement le workflow `Prepare or publish public presentation`. Avec `publish=false` (défaut), il construit l’artefact sans déployer. Après approbation seulement, sélectionner `publish=true` sur la branche par défaut. Aucune action sur `push`, PR ou horaire n’est configurée.
6. Après publication, vérifier l’URL renvoyée par le job, la navigation sous `/chillcenter/`, l’API JSON et la page 404. L’URL de retour 404 est prévue pour le dépôt GitHub actuel : l’adapter si le propriétaire, le nom du dépôt ou le domaine change.

Le workflow ne contient pas de PAT : `contents: read` pour la construction ; `pages: write` et `id-token: write` uniquement pour le job de déploiement, avec environnement dédié. `configure-pages` n’active pas automatiquement Pages. Les actions officielles sont épinglées sur leurs SHA complets ; Node 24 LTS et Ubuntu 24.04 sont utilisés. Réévaluer ces versions lors d’une publication ultérieure.

Versions vérifiées le 16 octobre 2026 via les références officielles GitHub : checkout 7.0.1, setup-node 7.1.0, configure-pages 6.0.0, upload-pages-artifact 5.0.0, deploy-pages 5.0.1. Ce workflow reste à exécuter réellement sur GitHub après autorisation ; les tests locaux ne prouvent pas les droits ou la configuration de l’environnement distant.

Références : [workflows Pages officiels](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages), [offres et visibilité Pages](https://docs.github.com/en/pages/getting-started-with-github-pages/about-github-pages), [cycle Node.js](https://github.com/nodejs/Release).

## Revenir en arrière

Avant publication : abandonner cette branche ou revenir au commit précédent, sans toucher aux versions livrées. Après publication approuvée : reconstruire l’artefact d’un commit public antérieur relu et le redéployer manuellement ; désactiver Pages depuis les réglages si nécessaire. Ne pas supprimer les données ou les historiques du projet pour retirer le site.
