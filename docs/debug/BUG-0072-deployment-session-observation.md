# BUG-0072 — Fin de commande distante mal observée pendant la sauvegarde

2026-10-15, livraison serveur seul f3fdac5, opérateur approuvé SHA62081da7. Défaut de surveillance opérationnelle, pas du serveur ou des vidéos.

Symptôme : après lancement SSH de l'opérateur, son stdout/code terminal n'a pas été conservé. Le déployeur a interprété le silence comme un arrêt et signalé un échec. Il a tenté un redémarrage de l'ancien service alors que `chill-server --backup` était encore en cours ; le nouveau processus a refusé le verrou de données, comme prévu. Le déployeur a ensuite arrêté cette boucle de redémarrage, sans tuer l'opérateur ni la sauvegarde.

H1 — sauvegarde encore active tenant DataLock : CONFIRMED. Le déployeur a identifié le parent opérateur2834/2836 et le backup2903 tenant `.chillcenter-data.lock`. Le message « Le service doit être arrêté avant cette opération » correspond au refus flock de `operations::DataLock::acquire`, non à un fichier de verrou à supprimer. Aucun effacement de verrou, récupération de base ou relance de l'opérateur n'a été fait.

L'opérateur a ensuite terminé ses étapes : à22:14:57Z, contrôle indépendant du coordinateur active/running PID3117/NRestarts0, fichier installé SHA d77e9e1669287a6d10f279c17f33d7f776efe7e194acc01cd542bcea33511c90. Le verrou appartient alors uniquement au serveur actif. Le déployeur a observé copie rollbackffcc, sauvegarde5dd8df08-adcb-4d09-a6c1-2eb549c0c414 et empreintes web/config/unité/TLS/montage identiques. Cela ne reconstitue pas le code terminal perdu ; la vérification indépendante de ces éléments reste obligatoire.

Correctif de procédure : conserver explicitement le résultat complet de chaque appel (session_id/cell_id/output/exit_code), poursuivre la session/cellule tant qu'elle est active et vérifier le processus opérateur avant toute récupération manuelle. Absence de stdout n'est pas sortie terminale. Ne jamais démarrer le service pendant une sauvegarde hors ligne, ni supprimer un verrou vivant. Une cause propre au transport/outillage n'est pas démontrée ici ; ne pas la présenter comme un bug de celui-ci.

Aucun changement produit/opérateur supplémentaire ni nouvel essai de déploiement pour masquer l'incident. Les verdicts et preuves source restent inchangés ; le rapport post-déploiement doit distinguer l'état finalement constaté de l'exécution non journalisée de bout en bout.
