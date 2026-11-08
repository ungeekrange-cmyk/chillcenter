# BUG0139 — Sauvegarde saine interrompue avant une mise à jour du Pi

8 novembre 2026, observé pendant l'installation explicitement autorisée du
serveur et du web sur le Pi. Serveur actuel 0.7.2, helper signé du serveur 0.7.4.

Le premier apply a attendu la quiescence sans forcer ; le serveur était occupé.
Après fermeture du Mac et expiration du bail restant, busy=false, mutations=0.
Le nouvel apply normal a arrêté le service et lancé la sauvegarde hors ligne,
puis a rendu `offline server backup exceeded time or output bound` à 120 s.
L'ancien serveur est revenu actif, le slot est resté 0.7.2, aucune donnée écrasée.

Preuve de progression : sauvegarde partielle sans manifeste final, 487 fichiers,
30 934 457 octets ; les affiches actuelles occupent environ 81 MiB. Le binaire
écrit et fsync chaque fichier pour la durabilité. La limite de deux minutes
interrompt donc une sauvegarde qui progresse sur ce stockage lent.

Correction ciblée : plafond de la sauvegarde hors ligne porté à 15 minutes ;
unité du helper à 30 minutes pour laisser le temps aux autres étapes et à la
récupération. TimeoutStopSec=150s reste inchangé. Aucun changement de groupes,
capacités, chemins accessibles, signatures, reçus, expiration, quiescence ou
protection anti-retour arrière. Les limites de sortie, la supervision du groupe
de processus et le refus d'une sauvegarde incomplète restent identiques.

Le correctif est préparé pour le serveur 0.7.5 et son helper. Le remplacement
administrateur du helper et de l'unité est nécessaire sur cette installation ;
ne pas réutiliser l'installateur réservé à une installation initiale.
Retour arrière : ancien helper et unité sauvegardés, ancien slot serveur conservé.
Les sauvegardes partielles sont préservées comme preuves, sans aucune purge.
