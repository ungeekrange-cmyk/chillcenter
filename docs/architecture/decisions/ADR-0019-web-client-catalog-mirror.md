# ADR-0019 — Miroir local du catalogue côté navigateur (IndexedDB) et synchronisation en arrière-plan

- Status : `PROPOSED` ; date : `2026-10-15`. Passe à `ACCEPTED` après revue indépendante des octets déployés et déploiement vérifié.
- Decision Owner / Author Instance : `/root` ; Reviewer Instance : à désigner (≠ auteur).
- Branche : `codex/web-pwa`. Supersedes / Superseded By : `NONE` / `NONE`. S'appuie sur ADR-0014 (client web), ADR-0013 (miroir du client Mac, même contrat serveur), ADR-0011 (API datée).
- Contrat d'API : **inchangé, `2026-10-14`** (alias `v1`). Le client web n'ajoute aucune route et n'exige aucune version plus récente ; les routes d'instantané utilisées figurent dans l'OpenAPI publié (`docs/api/openapi-2026-10-14.json`) et dans `docs/api/INTEGRATION-FR.md`.

## Contexte

Demande utilisateur (15 octobre 2026) : le chargement des données est trop long et empêche de naviguer ; garder une empreinte locale côté client pour ne pas tout recharger, ne synchroniser que la différence, en arrière-plan, avec un coût de ressources réduit ; viser la fluidité.

Mesures réelles sur le Pi (2 496 titres, 15 octobre 2026) :

| Appel | Durée |
| --- | --- |
| page de catalogue (48 titres, `grouped`) | 1,1 à 2,0 s |
| `sagas` / `series/{nom}` / `related` | 1,8 s / 1,7 s / 1,1 s |
| instantané complet (POST + 25 pages de 100) | ≈ 2,3 s + 25 × 50 ms, ~3,5 Mo |
| vérification « rien n'a changé » (`known_revision`) | **15 ms** |
| `profiles`, `admin/library`, fiche, affiche | 10 à 30 ms |

Causes : (1) `loadPage` de l'interface partagée enchaîne **toutes** les pages (52 pages de 48 pour 2 496 titres, ≈ 1 s chacune, soit plus d'une minute) ; (2) le serveur n'a qu'un accès base (`DatabaseActor`) : chaque requête lourde fait attendre toutes les autres, y compris des appels légers ; (3) `run()` de l'interface désactive tous les boutons pendant une commande.

## Décision

1. **Magasin local** (`catalog-store.js`) : IndexedDB `chill-center-web` (fonctionne en HTTP clair, contrairement à Cache Storage et au Service Worker), un miroir par `serveur|profil`, mises à jour en une transaction ; repli mémoire (navigation privée, IndexedDB refusé). Aucune donnée n'est écrite hors du navigateur de l'utilisateur.
2. **Moteur de requêtes local** (`catalog-query.js`) : port de `Store::browse_catalog` (filtres, recherche, séries groupées, tris, compteurs). Les égalités se départagent par le **chemin relatif du fichier** (identifiant « legacy » du serveur, constaté sur données réelles). Les listes triées sont mémoïsées par jeu de filtres (l'interface lit 50+ pages de suite). **Validation différentielle** sur le Pi : 66 requêtes (tous tris, catégories, filtres, recherche avec caractères spéciaux, séries groupées ou non, pages profondes) donnent exactement les mêmes éléments, le même ordre, les mêmes compteurs que le serveur (`scripts/catalog-differential.mjs`, 0 écart).
3. **Synchronisation** (`catalog-sync.js`) par les routes d'instantané **déjà publiées** : `POST …/catalog/snapshot {known_revision}` (réponse `unchanged` en 15 ms) puis pages de 100 si la révision a changé. Seule la **différence** est écrite localement (contenu comparé, car l'état personnel `watched`/`rating` est dans la fiche) et annoncée à l'interface. Reprises : 409 `SYNC_SNAPSHOT_CHANGED` et 429 (bornées), 404/410 sur une page (instantané expiré : un seul redémarrage), instantané incomplet refusé, commit en échec = ancienne copie conservée. Un serveur sans ces routes (`ROUTE_NOT_FOUND`) désactive le miroir pour la connexion : tout reste servi par l'API.
4. **Déclenchement** : à l'ouverture d'un profil (copie locale servie tout de suite, vérification ensuite), retour de l'onglet, minuterie lente (3 min) seulement quand l'onglet est visible, 0,8 s après toute écriture (marquer vu, noter, éditer, sagas, listes), à la fin d'un scan. `refresh` répond aussitôt, la vérification se fait en tâche de fond ; quand elle trouve des changements, un événement `chill:catalog-updated` rafraîchit discrètement la page (jamais pendant une lecture, jamais pendant une commande, message plutôt que saut si l'utilisateur a fait défiler la page).
5. **Lectures locales** : catalogue, séries, sagas, playlists, fiche média. `related_media` reste côté serveur (liste secondaire). Les écritures vont toujours au serveur ; `watched` et `rating` sont répercutés dans le miroir sans attendre.
6. **Affiches** (`artwork-cache.js`) : mémoire (URL d'objet, 400 au plus), IndexedDB (blobs, 150 Mio au plus, éviction des moins récemment utilisés), puis réseau limité à **4 téléchargements simultanés**, le plus récemment demandé d'abord ; valides tant que la révision de la fiche ne change pas. Pas de recompression (les affiches font ≈ 90 Ko).
7. **Versions des fichiers** : chaque script, feuille de style et Worker est appelé avec `?v=<identifiant de build>` (le serveur ignore la requête). Sans Service Worker (HTTP clair), un navigateur pouvait mélanger anciens et nouveaux fichiers pendant une heure après une mise à jour (cache `max-age=3600`) ; constaté en test réel.

## Mesures (serveur réel sur le Mac, 2 443 titres, navigateur en HTTP clair)

| | Avant | Après |
| --- | --- | --- |
| première visite, 1 843 cartes affichées | ≈ 1 min sur le Pi (52 requêtes lourdes) | 1,26 s (le premier lot en 0,12 s), synchro complète en parallèle |
| visite suivante | idem | **22 ms**, 0 requête de page, 1 vérification |
| filtre, tri, recherche, page 20 | 1 à 2 s | **< 10 ms** |
| 39 pages de suite | ≈ 40 s | ≈ 0 ms (tri mémoïsé) |
| fichier ajouté puis scan | rechargement manuel | affichage mis à jour en 1,4 s sans rechargement |

## Conséquences et limites

- Mémoire : ≈ 3,5 Mo de JSON par profil pour 2 500 titres, plafond serveur de l'instantané 10 000 titres / 32 Mio.
- Le premier chargement d'un profil sur un navigateur neuf télécharge l'instantané complet (≈ 3,5 Mo) ; ensuite seule une vérification de 150 octets.
- Divergence assumée avec le miroir du Mac : pas de mode « hors connexion » (la connexion échoue encore si le serveur est injoignable) ; l'ADR-0013 reste la référence côté Mac.
- `related_media` (1,1 s) et les actions d'écriture dépendent encore du serveur.
- Aucun changement de contrat, de route, de binaire serveur ni de configuration : déploiement = nouvelle archive web seule.
