# Chill Center Mac0.5.1 — décor jusqu'en haut

16octobre2026, correction demandée après la capture utilisateur. [BUG0074](../debug/BUG-0074-event-header-opaque-background.md) : décor déjà ancré top0, caché par le fond opaque du bandeau de recherche/profil. Correction produit : une seule règle events.css rend ce bandeau transparent pour Noël et Halloween, clair/sombre. Champ de recherche, voile, images, palettes, autres thèmes et lecteur conservés.

Source et bundle définitifs à compléter après construction. Application0.5.1/build6 ; métadonnées Cargo/Tauri/plist/lock synchronisées, API2026-10-14/serveur/web inchangés. Tag v0.5 et bundle précédent conservés sans modification. Pas de nouvelle revue indépendante revendiquée pour ce correctif local borné.

## Preuves

- RED avant correction : test du bandeau Noël sombre, réelrgb(21,27,24) au lieu de transparent.
- GREEN après : `CHILL_PREVIEW_NODE_MODULES=/private/tmp/marvin-cockpit.NmYyTL/package-manager/node_modules CHILL_V05_EVIDENCE=/private/tmp/chillcenter-v051-appearance-evidence node scripts/test-appearance-v05.cjs` PASS. Vraie interface connectée avec pont synthétique, recherche visible, transparence pour les quatre couples événement/thème, image top0/ancrage droit, retour au fond opaque hors événement ; captures inspectées. Les contrôles d'apparence existants proches passent aussi. Aucun test catalogue/lecteur/Pi/NAS supplémentaire.
- `git diff --check` PASS ; delta produithorsversion limité à une règle CSS. Aucun schéma, API, secret, configuration foyer ou bibliothèque ne change.

## Livraison et repli

Bundle correctif distinct à publier sous `artifacts/macos/Chill Center-v0.5.1.app`. Les bundles0.5 etf3 sont conservés. Lancement uniquement du nouveau bundle ; ne pas fermer ni tuer une ancienne instance sans demande humaine. Repli : quitter la nouvelle application et relancer v0.5, sans restauration de données. Signature/hash/lancement à confirmer après exécution réelle. La validation visuelle native reste humaine, pas déduite du test Chrome.
