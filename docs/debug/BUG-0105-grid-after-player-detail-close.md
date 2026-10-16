# BUG-0105 — vignettes étirées après lecteur puis fermeture de fiche

16 octobre 2026, auteur `/root`, base `0c49a8c`, branche `codex/playback-locale`.
Statut : FIXED / VERIFIED_LOCAL, rendu humain Tauri restant.

La capture utilisateur montre des affiches devenues des bandes pleine largeur.
Le catalogue reste `hidden=false` mais le CSS `body.player-open` le retire du layout.
Un rafraîchissement peut alors publier une fenêtre virtualisée mesurée à largeur zéro,
avec une seule colonne. La fermeture du lecteur ne l’invalidait pas systématiquement.

Correction : ne rien mesurer/publier pendant ce masquage ou à largeur nulle. Invalider
la clé de layout et reprogrammer le rendu après sortie du lecteur et fermeture de fiche.
Même garde pour les membres de saga/playlist. Pas de rechargement réseau des affiches,
de suppression du miroir ou de remplacement systématique des cartes conservées.

Preuve RED/GREEN : `scripts/test-playback-layout-language.cjs`, pont synthétique et
vrai DOM Chromium. Sur la base : largeur d’affiche 1089,8px, assertion de colonnes en
échec. Sur le candidat : portrait 2/3, colonnes normales après lecture → fiche → fermeture,
trois langues système. `scripts/test-catalog-virtual-artwork.cjs` : chevauchement des
fenêtres et images décodées conservés sans fetch répété, PASS.

Lié au symptôme historique BUG0080, mais nouveau scénario précis ; aucune validation
humaine native déduite du test Chromium. Rollback : ancien bundle conservé.
