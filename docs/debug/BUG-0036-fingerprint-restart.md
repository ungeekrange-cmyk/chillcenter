# BUG-0036 — Empreinte interrompue bloquée après redémarrage

2026-10-14, audit final root. Hypothèse confirmée : les états SQLite queued/running survivent à la mort du processus ; le pump les exclut et la demande complète retourne FINGERPRINT_BUSY indéfiniment. Test serveur RED : une ligne running persistée reste exclue après ouverture du nouvel acteur.

Correction au démarrage exclusif du serveur uniquement : queued/running deviennent failed avant de publier l'acteur SQLite. Aucune reprise automatique du full hash coûteux ; un scan peut refaire la preuve partielle, un utilisateur peut redemander la preuve complète. Store::open ordinaire, lecture/export et snapshot n'invalident jamais un worker actif. Aucune note, UUID, playlist ou preuve complète terminée n'est supprimée. Le verrou de données du service est acquis avant l'acteur.

Régression startup_recovers_interrupted_fingerprints_without_generic_open_mutating_jobs : GREEN PASS après le RED ; ouverture générique sans reset vérifiée dans le même test. Kill/panne électrique réelle du Pi NOT_RUN. Pas de nouveau schéma ; rollback logiciel avec sauvegarde pré-migration habituelle.
