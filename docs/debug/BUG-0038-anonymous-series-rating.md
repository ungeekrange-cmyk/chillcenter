# BUG-0038 — Notes de séries sans nom

14 octobre2026, revue code indépendante ded8b297. Comparaison Option series=None sélectionnait toutes les vidéos de catégorie serie sans nom, même de titres différents. US-013-AC-02/03 : noter une série pouvait changer une autre. Test domaine RED : B reçoit4 alors que seul A doit être noté.

Correction minimale : clé effective series.unwrap_or(title), comme la navigation et le catalogue hérités ; scope season conserve son filtre saison. Transaction/personnalisation inchangées. Test séries anonymes A saisons1/2, B saison1, autre profil : série puis suppression de note saison et note B, aucune contamination. GREEN PASS ciblé. Aucune donnée foyer modifiée ; aucune correction rétroactive supposée des bases où une note aurait déjà été propagée.
