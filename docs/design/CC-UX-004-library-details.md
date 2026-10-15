# CC-UX-004 — Catalogue et fiches

Demande utilisateur 15 octobre 2026, auteur `/root`. Ajustements du design déjà approuvé, pas de remplacement des ambiances ni des assets.

Référence générale : hiérarchie « affiches → fiche → série/saison/épisode » et actions contextuelles de Plex ([fiche](https://support.plex.tv/articles/202462186-viewing-item-details/), [édition](https://support.plex.tv/articles/201272763-edit-details/)). Ces références n’impliquent aucune copie d’assets ou dépendance Plex.

- Affiche : lecture au centre, vu/non vu en bas à gauche, ajout liste/saga en bas à droite ; autre clic ouvre la fiche. Note existante sous l’affiche, jamais éditable sur celle-ci.
- Fiche : cinq étoiles immédiates, synopsis/affiche liés à l’identité ouverte, fermeture extérieure et position du catalogue conservée. Import/matching actualise sans fermer.
- Série/saison : recherche TMDB et import d’image à leur niveau. La saison dépend d’une série TV choisie. Identité opaque différente du nom affiché, homonymes séparés par source/identifiant TMDB.
- Administration : file de matching sur demande, emplacement relatif au survol, choix explicite. Le fournisseur et la synchro ne bloquent pas la navigation.
- Synchronisation : icône discrète en haut à droite ; aucune disparition de la grille avant publication du résultat complet. Catalogue toujours continu et virtualisé.

Preuve locale : `scripts/test-library-ux.cjs` avec passerelle fictive et navigateur jetable ; ce n’est ni une preuve Tauri/native de ce lot ni une mesure sur le Pi. Les essais lecteur ne sont pas nécessaires à ces changements de présentation.
