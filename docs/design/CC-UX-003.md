# CC-UX-003 — retours lecteur, collections et recherche

14 octobre2026. Origine : corrections directes de l'utilisateur après essai Mac. Implémenté ; validation visuelle humaine du nouveau bundle en attente. Ce document complète CC-UX-002, sans réécrire son historique.

- Administration → Affichage du lecteur : plein écran initial activé par défaut, choix mémorisé sur ce Mac. Désactivé : toute la zone droite, du haut au bas et de la barre latérale au bord de fenêtre ; image conservée avec bandes noires.
- Flèche retour à gauche, titre à droite. Commandes vitesse/audio/sous-titres assorties aux boutons. Masquage et apparition par fondu400ms ; réduction des animations respectée. Échap quitte le plein écran ; retour arrête la lecture et retrouve la bibliothèque.
- Sagas et listes : pages dans le panneau droit, couvertures et noms sous les vignettes. Une collection ouvre une page de membres, avec retour et ajout `+`. Le `+` de la page racine révèle le formulaire de création. Gestion existante conservée dans un volet repliable.
- Ajout : page de recherche à vignettes, filtres films/séries-épisodes/documentaires/spectacles et vu/nonvu. Sélection persistante entre filtres, affiches visibles chargées par le transport Rust. Les sagas gardent le contrat films seulement. La recherche générale conserve sa grille et ses filtres de catégorie.
- Aucun bouton de pagination. Lots API internes et grilles virtualisées pour limiter DOM/images ; pas de nouveau framework ou fournisseur.

Le fichier HTML ouvert seul dans un navigateur est un aperçu non connecté : pas de pont Tauri/Rust, donc aucune connexion serveur ou lecture. Ce n'est ni une version web livrée ni une tâche de test demandée à l'utilisateur. Pour l'essai réel, ouvrir le bundle Mac indiqué dans [la release](../releases/REL-MAC-UI-PANELS-001.md).
