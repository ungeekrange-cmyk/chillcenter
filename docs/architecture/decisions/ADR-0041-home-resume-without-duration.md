# ADR0041 — Reprises héritées dans l’Accueil et navigation unique

8 novembre 2026. Demande utilisateur explicite : « En cours » fait doublon avec
l’Accueil ; un film commencé manque dans ce dernier. Implémentation locale,
publication et déploiement distincts.

Remplace uniquement la catégorie de navigation de l’ADR0035 et l’exclusion des
reprises sans durée décrite dans l’ADR0038. Ces décisions historiques restent
conservées. La progression mesurée et ses garanties de précision sont maintenues.

La rangée Reprendre est l’unique accès aux lectures en cours. Elle contient les
films/vidéos disponibles, non vus et ayant une position persistée strictement
positive, y compris après import ou mise à niveau depuis un ancien serveur.
Le champ `progress` conserve sa forme actuelle et vaut null sans durée mesurée.
Une durée connue autorise le pourcentage ; une position connue suffit à proposer
la reprise. Ni TMDB ni une approximation ne fournissent une durée manquante.

Les séries restent regroupées par saison commencée non terminée ; les rangées
restent plafonnées à20 éléments et propres au profil. Aucun changement de
marquage vu, de transport, de confiance TLS, de schéma ou de lecture NAS.
La route compacte de progression mesurée continue à omettre les anciennes
reprises sans durée : elle sert les pourcentages, indépendamment de l’éligibilité
à l’Accueil.

La navigation Mac/web et le menu mobile retirent En cours. Les catégories,
recherche, fiches, barres mesurées et contextes d’enchaînement restent disponibles.
Rollback du code sans migration inverse ; aucune donnée n’est réécrite.
