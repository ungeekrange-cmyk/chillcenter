# BUG-0048 — File/historique TMDB non bornés

2026-10-14, coordinatrice `/root`, candidat fautif258db0ad27f05fafb12bd3592013ef300ead93fd. Statut `VERIFIED_CODE` sur50bbdf800a6c46c67ddd3cce74d8358286cd8eb6, correction par `/root/native_player_builder`, revue sécurité distincte `/root/ux_api_security_review` APPROVED ferme findingHIGH SEC-258-01. La relecture du candidat final complet et les mesures Pi restent distinctes.

Cause statique démontrée : les POST/retry ajoutent chacun une entrée persistée avec sa liste de médias et une tâche qui attend un mutex unique ; chaque compteur réécrit toute la map. Aucun plafond/pruning du nombre de travaux ; le chemin implicite/auto parcourt également au-delà de3000. Un client LAN peut donc épuiser la mémoire, les tâches ou le disque du Pi, malgré la sérialisation des requêtes au fournisseur. Le LAN sans authentification est accepté, pas une croissance non bornée.

Correction autorisée : deux réservations de tâches maximum, conservées jusqu'au retour réel du worker même après annulation ; historique8terminaux/15min, état sérialisé≤2Mio, lectures au démarrage bornées, maximum3000médias explicite/implicite/auto. Saturation429, dépassement413, pas de troncature silencieuse ou nouvelle tâche illimitée. Tests saturation/annulation/retry/restart/retention/limites avant une nouvelle revue exactSHA. La mesure RSS physique n'est pas déduite des tests.

Preuves exécutées :26TMDB ciblés PASS puis97serveur/289workspacePASS etclippy ; le reviewer sécurité reproduit26TMDB,40Node,contrat66. Pruning durable et limites au démarrage/réservation avant énumération/cancellation ne rend pas le slot couverts. TTL réconcilié au prochain accès, pas suppression forcée pendant l'inactivité ; mémoire/filebornées même au repos. Publication atomique peut brièvement conserver ancien+nouveau, chacun≤2Mio. Aucun code ajouté en modifiant ce statut.

Le candidat fautif n'a pas été promu ni déployé. Le serveurPi126da0a et ses2496vidéos restent conservés. Retour du correctif : binaire/client coordonnés, données/tokens privés préservés, pas de mutation du NAS. La clôture dépendra des preuves et revues finales, pas de ce plan.
