# ADR0045 — Position d’une nouvelle lecture

20 novembre 2026. État : IMPLEMENTED_LOCAL, revue finale en cours.
Complète ADR0042/0044 ; aucun changement API ni schéma.

La position durable conserve l’historique de visionnage. Une action Lecture
fraîche utilise une copie normalisée : zéro pour un média vu ou arrivé à sa fin
mesurée à250ms près. Les autres positions partielles sont conservées. La lecture
de durée est ciblée sur la clé profil/média, sans parcourir le catalogue/profil.
Un transfert de la même vidéo continue la lecture courante ; une cible différente
reçoit la position de nouvelle lecture. Aucun reset de base pour satisfaire l’UI.
Le lecteur web vérifie aussi l’EOF à partir de la durée de son conteneur, comme
le lecteur natif Mac existant, et gère un grant watched d’un ancien serveur.

Sous-titres : compléter la langue préférée par l’unique piste texte sans langue
si la préférence est active. Garder `und`, sans annoncer une langue inventée.
Ne jamais forcer Off ou choisir parmi plusieurs pistes ambiguës.

Conséquences : web et Mac partagent la règle serveur sans nouveau binaire Mac.
Tests domaine/API isolés, navigateur réel et revue du transfert nécessaires ;
aucun test ne modifie l’état personnel réel du foyer.
