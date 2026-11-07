# Accueil, collections, mises à jour et curseur — preuve du candidat

2026-11-07 · branche `codex/updates-collection-management`.
Base `daba984b7e4de0c132f6cdd2b3a17e9cb9b7fa83`.
Source finale vérifiée : `a0a47ca0d59b26e83f451581bea72dbd4f2362f8`.
Ce document est ajouté après cette source et ne change pas le runtime.

## Résultat et revue

Accueil remplace Toute la collection comme destination initiale ; trois rangées
horizontales au maximum, vingt entités chacune, séries regroupées. Reprendre
considère les saisons commencées mais inachevées ; les suggestions sont locales
et liées aux cinq dernières entités vues datées du profil. Ancien historique
sans date conservé sans date inventée. Recherche/catégories restent disponibles.
Jaquettes : membre, TMDB ou import avec confirmation explicite. Gestion visible,
suppression nommée conservant les vidéos. Curseur synchronisé avec les commandes.
Helper : résumé UI publié0640 malgré UMask0077, aucun autre droit élargi.

Revue indépendante de code `/root/final_review` : APPROVED pour le premier lot
`2b46d23` contre `daba984`, puis APPROVED pour Accueil/curseur `a0a47ca` (incluant
`6daab89`) contre `f3f9b7b`. Les observations sur profils, cache, jetons, cartes
obsolètes, largeur mobile, métadonnées de série et saisons inconnues sont closes.
Le coordinateur consigne ce verdict ; le reviewer n'a pas édité les sources.

## Vérifications exécutées

- `node --test ui/connected/tests/*.test.js ui/connected/*.test.js web/test/*.test.js scripts/test-bilingual-docs.mjs scripts/test-update-helper-hardening.cjs` : **396 passent**, aucun échec.
- `cargo test --locked --offline -p chill-domain -p chill-catalog -p chill-client -p chill-server -p chill-updates --lib --bins` : **397 passent**, deux tests manuels préexistants ignorés. Cache compatible partagé réutilisé ; fixtures HTTP sur localhost autorisées.
- `cargo fmt --all --check` et Clippy strict `--all-targets -- -D warnings` sur domaine/catalogue/client/serveur/helper/desktop : PASS.
- `node scripts/validate-api-contract.mjs` : **86 chemins**, PASS.
- Construction web : **105 fichiers**, build **6c4a494bf40e**. Nouvelle sortie de test bornée17Mio, dans l'espace existant du lot ; release0.7.4 conservée séparément.

Essais manuels dans le navigateur, données synthétiques : gestion/renommage et
suppression de groupe de test, copie de jaquette et recherche TMDB simulée,
confirmation explicite ; Accueil initial, rangées et flèches horizontales, passage
vers Films puis retour au logo Accueil mobile, suggestions et badges de reprise.
Rendus1440×1000,375×812 et contrôles membres361×812 vérifiés. Captures hors Git :
`home-desktop-final.jpg`, `home-mobile-final.jpg`,
`collection-cover-desktop.jpg`, `collection-mobile-controls-final.jpg` dans
l'espace temporaire existant du lot. La fixture finale intègre les feuilles CSS
exactes du projet pour éviter des téléchargements CSS locaux incomplets ; aucune
modification d'inline style n'est ajoutée à l'application. Serveurs QA arrêtés,
aucun cache ni livrable purgé.

## Réalité déployée et limites

Aucun nouveau paquet publié, fusion main, installation de binaire ou migration de
la base familiale dans ce lot. Versions source inchangées ; elles ne prouvent pas
une livraison. La seule réparation réelle du serveur est documentée BUG-0134 :
mode du résumé de disponibilité rétabli, API web installé0.7.2/disponible0.7.4
relue. Le helper installé reste ancien et peut recréer0600 avant sa mise à jour.
Pas de test TMDB réel utilisant la clé familiale ni de nouveau binaire Mac lancé.
Le navigateur teste l'interface partagée ; les transports natifs sont testés et
compilés. Le masquage natif du curseur repose aussi sur la revue du hit-test.

Accueil est borné à10000 vidéos ; erreur explicite au-delà. Les archives portables
conservent le nouvel horodatage mais omettent toujours les durées mesurées selon
ADR-0035 : reprise Home éventuellement absente jusqu'à relecture. Les sauvegardes
SQLite complètes conservent les mesures. Les anciens importeurs stricts refusent
une archive contenant le nouveau champ de date ; anciens exports restent lisibles.

Rollback : versions précédentes, sans suppression de la colonne nullable ajoutée.
Le trigger empêche une ancienne écriture watched de conserver une date devenue
fausse. Aucun changement de secret, TLS, IAM ou exposition réseau. Les sources,
worktrees et diagnostics privés sont préservés ; aucun nettoyage implicite.
