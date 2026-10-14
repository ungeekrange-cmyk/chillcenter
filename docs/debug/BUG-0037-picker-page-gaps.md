# BUG-0037 — Sélecteur paginé avec trous

14 octobre2026, revue code indépendante du candidat ded8b297. API48/page, déplacement du sélecteur50 : médias48/49 puis98/99 invisibles lors d'ajout playlist/saga (US-014-AC-02/05). Test du vrai contrôleur extrait,103médias : RED montre les4IDs absents. Correction : réutiliser CATALOG_PAGE_SIZE de l'application pour avancer/reculer, même contrat fermé Rust. Le test vérifie toutes les lignes, offsets0/48/96/48/0 et sélection conservée. GREEN PASS, inclus dans24 tests Node ; aucun média/ordre personnel déplacé par le correctif.
