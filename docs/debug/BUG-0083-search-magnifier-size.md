# BUG-0083 — Loupe de recherche trop petite

## 17 octobre 2026 — FIXED / VERIFIED_LOCAL

Cause reproduite : le caractère typographique ⌕ n'avait ni taille ni boîte
graphique maîtrisée. Remplacement par un SVG accessible décoratif de 24×24,
proportionné au champ sans modifier son label ni son focus.
`scripts/test-ui-recovery-icons.cjs` mesure taille et centrage dans le vrai DOM,
en clair/sombre aux largeurs 760, 1180 et 1600 px : PASS après échec témoin avant
correction. Rendu humain Tauri/échelles Retina restant à confirmer.
Le signalement initial reste historique.

**OPEN — non bloquant, signalé et illustré, non corrigé dans ce lot.** 16 octobre 2026, client Mac, version et facteur d’échelle exacts à relever.

La loupe à gauche du champ de recherche apparaît nettement trop petite par rapport à la hauteur du champ et aux autres icônes. Attendu : pictogramme lisible, centré verticalement, proportionné au champ et cohérent avec les contrôles voisins.

La capture utilisateur confirme l’écart visuel ; cause non diagnostiquée. À vérifier en clair/sombre et aux échelles habituelles du Mac. Une future correction doit préserver le label accessible, le focus clavier et la largeur utile de saisie. Aucun changement produit ni nouvelle validation annoncés ici.
