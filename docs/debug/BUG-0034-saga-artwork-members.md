# BUG-0034 — Affiche de saga et membres sur le même slot

2026-10-14, source41593db/root. Cause confirmée : openCollection déclenchait loadSagaArtwork sans await, puis collectionPage appelait run pendant que l'affiche détenait le slot UI. Ces lectures étaient sautées, alors que collectionLoaded avançait. Une saga illustrée pouvait donc présenter une affiche sans ses membres.

RED déterministe : `node --test ui/connected/tests/saga-loading.test.js`, vraie fonction openCollection extraite du fichier livré, promesse d'affiche contrôlée, première lecture de membre effectuée alors que le slot est occupé. Correction : attendre la lecture d'affiche avant les lectures paginées des membres ; erreur d'affiche non fatale ne bloque pas la collection. Aucun replay de mutation et aucun chargement de vidéo.

GREEN : test du même contrôleur PASS, inclus dans les23 tests Node exécutés. Le libellé TMDB est désormais réservé à une fiche possédant tmdb_id, sans attribuer ce fournisseur à toute image importée non marquée personnelle. Revue finale requise.
