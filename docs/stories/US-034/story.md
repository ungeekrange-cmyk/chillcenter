# US034 — Retrouver et modifier les réglages

Complément demandé explicitement le16octobre2026. Trace F14/F18, [lot Configuration](../../implementation/CONFIGURATION-MATCHING-20261016.md). Pas une nouvelle phase de découverte ni une approbation supposée de future publication.

En tant que membre du foyer, je veux retrouver les réglages dans un panneau Configuration clair, pour modifier rapidement la bibliothèque, mes préférences et les outils communs.

- **US034-AC01** : menu et titre « Configuration », vue « Tout » montrant tous les réglages, et neuf catégories dédiées au plus dans une colonne gauche.
- **US034-AC02** : recherche globale normalisée (casse/accents/ponctuation) sur noms et libellés des réglages, pas sur valeurs des secrets ; une saisie revient à « Tout », aucun résultat explicitement signalé.
- **US034-AC03** : chaque rubrique est une carte pleine largeur de la colonne de contenu ; aucune carte ou formulaire perdu lors d'un filtre, ajout dynamique ou changement de catégorie. Navigation responsive et clavier.
- **US034-AC04** : « Bibliothèque » et « Localisation de la bibliothèque », ajout/retrait d'emplacements, enregistrement explicite ; ancien message NAS superflu retiré. Les conséquences communes/personnelles et l'absence d'authentification demeurent indiquées.

Les IDs techniques admin existants et routes historiques restent conservés pour compatibilité. Pi et client web ne sont pas supposés déployés par ce document. Tests et preuve humaine restent distincts.
