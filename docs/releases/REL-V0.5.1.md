# Chill Center Mac0.5.1 — décor jusqu'en haut

## Livré localement / lancé

Source **375ec6b35c86c4b9df686ddafabb30d36446e36c**, tag annoté local **v0.5.1**, application0.5.1/build6. Bundle : `/Users/ghillaumesoubigou/Documents/Dev/Perso/ChillCenter/artifacts/macos/Chill Center-v0.5.1.app`. ExécutableSHA256 **815c2a0c76e70c533fa909fcd6fc9d23fe72e8eb0ca4cd1b5e978ff783e35799**. Compilation offline/locked,48dylibs arm64, signature profonde ad hoc et chargement libmpvAPI2.5 PASS ; même helper de packaging/publication exclusive quev0.5. Lancement `open -n` réussi, PID36612 encore actif52s à10:27:13UTC le16octobre2026, chemin exact du nouveau bundle. La preuve reste packaging/démarrage et DOM synthétique ; appréciation visuelle native à faire par l'utilisateur.

Ancien tagv0.5 relu inchangé à1e16ee2058269b04e3bad7f57c2975eb07d77011 ; ancien bundle préservé. Branche canonique du correctif importée sans checkout, seul `.claude/` non suivi conservé. Aucun appel ni déploiement Pi, aucun changement web/API/NAS ou de configuration foyer. Aucun push. Le statut « à compléter » ci-dessous décrit le checkpoint de préparation, remplacé par cet état courant.

16octobre2026, correction demandée après la capture utilisateur. [BUG0074](../debug/BUG-0074-event-header-opaque-background.md) : décor déjà ancré top0, caché par le fond opaque du bandeau de recherche/profil. Correction produit : une seule règle events.css rend ce bandeau transparent pour Noël et Halloween, clair/sombre. Champ de recherche, voile, images, palettes, autres thèmes et lecteur conservés.

Source et bundle définitifs à compléter après construction. Application0.5.1/build6 ; métadonnées Cargo/Tauri/plist/lock synchronisées, API2026-10-14/serveur/web inchangés. Tag v0.5 et bundle précédent conservés sans modification. Pas de nouvelle revue indépendante revendiquée pour ce correctif local borné.

## Preuves

- RED avant correction : test du bandeau Noël sombre, réelrgb(21,27,24) au lieu de transparent.
- GREEN après : `CHILL_PREVIEW_NODE_MODULES=/private/tmp/marvin-cockpit.NmYyTL/package-manager/node_modules CHILL_V05_EVIDENCE=/private/tmp/chillcenter-v051-appearance-evidence node scripts/test-appearance-v05.cjs` PASS. Vraie interface connectée avec pont synthétique, recherche visible, transparence pour les quatre couples événement/thème, image top0/ancrage droit, retour au fond opaque hors événement ; captures inspectées. Les contrôles d'apparence existants proches passent aussi. Aucun test catalogue/lecteur/Pi/NAS supplémentaire.
- `git diff --check` PASS ; delta produithorsversion limité à une règle CSS. Aucun schéma, API, secret, configuration foyer ou bibliothèque ne change.

## Livraison et repli

Bundle correctif distinct à publier sous `artifacts/macos/Chill Center-v0.5.1.app`. Les bundles0.5 etf3 sont conservés. Lancement uniquement du nouveau bundle ; ne pas fermer ni tuer une ancienne instance sans demande humaine. Repli : quitter la nouvelle application et relancer v0.5, sans restauration de données. Signature/hash/lancement à confirmer après exécution réelle. La validation visuelle native reste humaine, pas déduite du test Chrome.
