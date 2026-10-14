# BUG-0031 — Durabilité et publication des réglages

2026-10-14, reprend les LOW UISEC-002 / SRV-SEC-005 de la revue interface r3, sans effacer leur historique. Client : succès après rename sans fsync du parent. Serveur : rename réussi puis fsync du parent échoué renvoyait une erreur en laissant l'ancienne racine/révision en mémoire malgré le nouveau fichier.

Correction : parent fsync client avant succès ; échec après publication signalé LOCAL_DURABILITY_UNKNOWN et invitation à rouvrir pour vérifier, aucune promesse d'annulation du rename. Serveur : résultat CONFIG_DURABILITY_UNKNOWN distinct, mémoire réconciliée avec la racine/révision publiée, anciens baux invalidés même sur ce résultat ; HTTP503 explicite et recharge demandée. Avant rename, erreur conserve l'état précédent. Aucun secret, migration ou accès NAS ajouté.

Régressions injectent l'échec de synchronisation du répertoire à la même couture utilisée en production, après véritable écriture/fsync/rename. Tests `rename_publication_and_directory_durability_are_distinguished` (serveur) et `settings_directory_sync_failure_is_explicit_and_reopen_reads_published_state` (client) PASS avec Cargo offline/locked. Contrôles de contenu/reopen/CAS et absence de stage connu ; échec avant publication n'altère ni fichier ni mémoire. Perte de courant réelle NOT_RUN, revue finale indépendante requise.
