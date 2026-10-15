# Revue indépendante finale — interface bibliothèque

- Mode : `EXECUTION_REVIEW` (`DEV_STANDARD`)
- Reviewer : `/root/review_library_ui`
- Base de la révision : `f544684b002ad8c1bf5d31f98006da573f576019`
- Source figée examinée : `3bbb87db5a3331f8f07d12ab1404afd8a5d86ee7`
- Périmètre : `ui/connected`, pont d'import d'affiche utilisé par cette UI et preuves synthétiques de `scripts/test-library-ux.cjs`
- Verdict : **APPROVED**

## Conclusion

Aucun défaut P1/P2 actionnable n'a été retrouvé dans le périmètre UI demandé. Les quatre écarts de la première revue sont corrigés : une affiche personnalisée reste visible au niveau série, les mutations de matching et du sélecteur attendent une synchronisation fraîche même lorsqu'une ancienne synchronisation est écartée, le sélecteur protège le contexte profil/dialogue, et les imports série/saison actualisent la vue ouverte en conservant la saison et le défilement.

La régression de disponibilité du bouton « Organiser avec l'IA » est aussi corrigée au bon niveau : `run()` restaure d'abord l'état transitoire des contrôles, puis rend l'état métier actualisé. Le navigateur synthétique charge les vrais scripts `app.js` et `ai.js`, clique le bouton, obtient la boîte de dialogue et constate uniquement l'appel local `settings`, sans démarrer de fournisseur.

## Preuves indépendantes

### Tests Node ciblés

Commande exécutée sur le SHA ci-dessus :

```text
/usr/local/bin/node --test \
  ui/connected/admin-navigation.dom.test.js \
  ui/connected/ai.dom.test.js \
  ui/connected/ai.test.js \
  ui/connected/cache.dom.test.js \
  ui/connected/catalog-loading.dom.test.js \
  ui/connected/catalog-window.test.js \
  ui/connected/identity.test.js \
  ui/connected/run-availability.dom.test.js \
  ui/connected/sync.dom.test.js \
  ui/connected/tmdb.dom.test.js \
  ui/connected/tests/*.test.js
```

Résultat : **50 PASS, 0 échec, 0 ignoré**. Les assertions significatives couvrent notamment : restauration de la disponibilité IA après `run()`, conservation des désactivations métier, enchaînement ancienne synchronisation écartée puis nouveau delta, nettoyage immédiat lors d'un changement de profil, absence de contenu périmé dans une fiche, cinq étoiles, contexte du sélecteur et invalidation des réponses tardives.

### Parcours Chromium synthétique

Commande :

```text
NODE_PATH=/Users/ghillaumesoubigou/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules \
  /usr/local/bin/node scripts/test-library-ux.cjs
```

Résultat hors sandbox macOS : **PASS**. Le premier lancement dans le bac à sable a été refusé par `MachPortRendezvous` ; la relance autorisée du même script a réussi. Le parcours vérifie les vrais gestionnaires UI pour :

- fiche suivante sans affiche/résumé de la fiche précédente ;
- note immédiate sur cinq étoiles et fermeture par le fond de boîte de dialogue ;
- import d'image et validation TMDB sans fermer la fiche ;
- recherche et catalogue utilisables pendant une synchronisation suspendue ;
- arrêt du matching avec réponse tardive et validation pendant une synchronisation concurrente ;
- publication fraîche d'un ajout à une liste ;
- import d'affiche série personnalisée, import saison, réouverture de la saison et chargement paresseux de sa nouvelle affiche ;
- bouton IA activé après actualisation, ouverture des réglages et absence de démarrage fournisseur.

`git diff --check f544684..3bbb87d` : **PASS**.

## Lecture du code et qualité des preuves

- `syncAfterMutation()` attend une synchronisation déjà en vol avant d'en demander explicitement une nouvelle ; les tests contrôlent deux appels et le chargement du nouveau miroir.
- Les callbacks matching/sélecteur combinent génération, serveur/profil et présence du dialogue ou de la ligne avant d'appliquer une réponse tardive.
- L'affiche commune série sélectionne désormais toute affiche publiée, y compris personnalisée. Les cibles natives `series` et `season` sont distinguées par le parcours synthétique : une mauvaise cible ferait échouer les assertions d'affiche personnalisée ou de métadonnée saison.
- Les deux tests unitaires de `run()` prouvent les deux directions de la règle : un contrôle nouvellement autorisé est réactivé, tandis qu'un contrôle que le nouvel état doit interdire reste désactivé.
- Le scénario navigateur exerce les scripts livrés ensemble et échoue sur toute erreur de page non traitée ; il complète donc utilement les tests VM plus isolés.

## Limites non bloquantes

Cette approbation est limitée à l'interface et aux contrats simulés. Elle ne constitue pas une validation avec un catalogue réel, un fournisseur TMDB/IA réel, le sélecteur de fichiers natif, le réseau, le Pi/NAS, ni le bundle Tauri signé. L'acceptation visuelle et gestuelle finale par l'utilisateur dans ce bundle reste nécessaire. Les tests du lecteur ont volontairement été exclus de cette révision, conformément au périmètre demandé ; aucun jugement nouveau n'est porté ici sur le lecteur.

Les preuves Rust globales annoncées dans le dossier d'implémentation n'ont pas été réexécutées dans cette revue UI. Le rapport est un artefact de revue ajouté après le gel ; il ne fait pas partie du SHA source examiné.

## Addendum — connexion manuelle et approbation du certificat

- Base approuvée : `3bbb87db5a3331f8f07d12ab1404afd8a5d86ee7`
- Source figée réexaminée : `11aa529f2eced060156a3017456bf1d9634f8a56`
- Périmètre du delta UI : `ui/connected/app.js` et `ui/connected/sync.dom.test.js`
- Verdict du delta : **APPROVED**

La connexion manuelle et l'approbation initiale du certificat chargent maintenant d'abord le miroir local avec `await loadPage()`, puis lancent la vérification réseau avec `void syncCatalog()`. Le catalogue utilisable n'attend donc pas la fin du réseau, tandis que le cache est tout de même réchauffé après ces deux parcours d'entrée. Les parcours sans profil actif restent inchangés.

Preuves exécutées indépendamment sur le nouveau SHA :

```text
/usr/local/bin/node --test ui/connected/sync.dom.test.js
```

Résultat : **3 PASS, 0 échec, 0 ignoré**. Le nouveau test exerce séparément `connect` et `approve`, laisse leur promesse de synchronisation volontairement non résolue, puis vérifie que chaque gestionnaire a déjà terminé après un chargement local et a déclenché exactement une vérification. Une régression réintroduisant `await syncCatalog()` suspendrait ce test.

`/usr/local/bin/node --check ui/connected/app.js` et `git diff --check 3bbb87d..11aa529` : **PASS**. Aucun test navigateur ou lecteur n'a été répété : le delta ne modifie ni le DOM rendu, ni le lecteur, ni les parcours visuels précédemment approuvés. Les limites d'acceptation réelles énoncées plus haut restent applicables.
