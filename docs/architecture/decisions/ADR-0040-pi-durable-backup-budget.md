# ADR0040 — Budget de sauvegarde durable sur le Pi

8 novembre 2026. IMPLEMENTED_DETAIL pour terminer la mise à jour autorisée.
Complète ADR0029/ADR0033 sans remplacer les règles de signature ou de sauvegarde.

Une sauvegarde réelle a produit 487 fichiers / 30,9 Mo en 120 secondes, puis a
été correctement interrompue. Le catalogue d'affiches actuel représente environ
81 Mio et plusieurs milliers de fichiers avec fsync individuel. Ce résultat
prouve un budget trop court sur ce stockage, sans prouver un blocage du processus.

La sauvegarde hors ligne dispose désormais de 15 minutes au maximum. L'unité du
helper autorise 30 minutes pour le contrôle, les téléchargements, l'arrêt, la
sauvegarde, sa validation et l'activation. Le délai d'arrêt/récupération reste
150 secondes. Les autres requêtes gardent leurs délais existants.

Le processus de sauvegarde conserve ses UID/GID sans privilèges, ses limites de
sortie et son groupe supervisé. Une sauvegarde incomplète ne permet jamais
l'activation. Ni signatures, ni schéma, ni secret, ni capacité système, ni chemin
accessible ne changent. L'installation garde sa quiescence non forcée et son
retour arrière automatique. Le coût est une interruption potentiellement plus
longue sur un Pi lent ; elle remplace des échecs répétés sans réduire la durabilité.

Le correctif est préparé dans le serveur/helper 0.7.5. Un administrateur remplace
le helper et son unité signés avant de lancer l'installation normale. Les octets
publiés de 0.7.4 et les sources Mac 0.7.6 restent immuables.
Voir [BUG0139](../../debug/BUG-0139-pi-offline-backup-budget.md).
