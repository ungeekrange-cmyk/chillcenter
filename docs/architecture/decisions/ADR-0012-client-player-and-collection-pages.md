# ADR-0012 — présentation locale du lecteur et pages de collections

Date : 2026-10-14. Statut : IMPLEMENTED_DETAIL, demande utilisateur directe. Supersedes : NONE ; superseded_by : NONE. Complète ADR-0004 et ADR-0010 sans changer le moteur, le cache ou les contrats HTTPS.

## Décision

Le lancement manuel d'une vidéo utilise par défaut le plein écran natif. Un booléen `player_presentation.fullscreen_on_start`, conservé par Rust dans les réglages de ce Mac, permet de choisir le panneau droit intégral. Les anciennes configurations sans ce champ adoptent `true`. La commande IPC fermée `save_player_settings` n'est pas une route serveur et ne modifie aucun profil. Une erreur de sauvegarde est signalée ; aucun `localStorage`, préférence réseau ou dépendance supplémentaire.

Le mode fenêtre conserve la barre latérale mais masque la recherche et le contenu sous-jacent. La surface native libmpv occupe tout l'espace droit, sans ratio16:9 imposé au conteneur. Le moteur conserve l'image entière avec bandes noires. Ne pas peindre un fond DOM opaque sur la découpe transparente, qui cacherait la vidéo native. La flèche retour est à gauche et le titre à droite ; commandes et sélecteurs partagent le même traitement. Fondu d'opacité400ms, supprimé si réduction des animations demandée.

Sagas, listes et leur contenu sont des sections exclusives dans le panneau droit, avec retour et création/ajout par `+`. Le sélecteur d'ajout utilise des affiches et des filtres catégorie/vu ; la sélection survit aux recherches. Les sagas restent limitées aux films conformément au contrat existant ; les listes acceptent aussi épisodes, documentaires et spectacles. Les lectures et modifications utilisent les mêmes commandes Rust, révisions CAS et contextes de lecture qu'avant.

Grilles de catalogue, membres et sélecteur virtualisées ; affiches visibles seulement, requêtes d'images séquentielles, résultats périmés de profil/serveur/recherche ignorés, sélection limitée à1000 et chargement catalogue borné. L'API continue de fonctionner en lots internes de48, sans boutons de pagination.

## Conséquences et retour

Aucun nouveau binaire Pi, migration SQLite, modification NAS, service ou fournisseur externe nécessaire. La première entrée peut déclencher l'animation plein écran macOS ; Échap reste disponible. Un ancien client peut ignorer/perdre ce nouveau réglage lorsqu'il réenregistre ses réglages : le réappliquer ensuite. Retour à l'ancien bundle possible sans changer le serveur.

Le HTML `ui/connected/index.html` seul est un aperçu, pas un client web connecté. Le transport TLS, la confiance et le lecteur nécessitent le pont Rust de l'application Tauri. Le client web reste différé (F-02). La validation technique du lot et la validation visuelle de l'utilisateur restent distinctes.
