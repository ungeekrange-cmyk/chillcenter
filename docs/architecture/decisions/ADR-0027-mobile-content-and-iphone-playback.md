# ADR0027 — Contenu mobile prioritaire et lecteur iPhone

**Français** · [English](ADR-0027-mobile-content-and-iphone-playback.en.md)

- Date : 2026-10-17. ACCEPTED_BY_USER_CHANGE_REQUEST.
- Remplace uniquement la recherche mobile toujours visible et la navigation
  basse en paysage de l'[ADR0026](ADR-0026-mobile-web-presentation.md).
  Conserve les garanties de séparation Mac/web, API et serveur.

Le catalogue mobile s'ouvre directement sur les vignettes. Une loupe déploie
recherche et filtres ; le passage à une autre rubrique replie ces outils sans
effacer les choix. En paysage téléphone, les catégories occupent un rail gauche.
Les outils de saison/série sont déplacés après les épisodes dans des sections
repliées. Le desktop et le sélecteur d'ajout conservent leurs comportements.

Le moteur web conserve MediaSource quand il existe et accepte ManagedMediaSource
en repli. Pour ce dernier chemin, désactiver la lecture distante avant attachement
est nécessaire, aucune source AirPlay alternative n'étant fournie. Pas de
nouveau décodeur, dépendance, migration ni route serveur.

Le refus de lecture automatique n'est plus ignoré : un geste tactile reprend
l'élément déjà préparé, sans rouvrir son flux. Cette reprise partage la garde
de bail du contrôle normal et ne peut annuler une pause ordonnée par le serveur.
Le plein écran natif vidéo est demandé directement dans le geste si l'API de
plein écran du panneau est absente ; un rendu canvas ne peut emprunter ce repli.
La guidance de rotation n'est pas affichée quand le téléphone est déjà en paysage.

La validation Chromium avec conteneurs réels synthétiques n'est pas convertie
en preuve de compatibilité Safari native. La validation physique des codecs
des deux vidéos signalées reste nécessaire, notamment le MP3 en AVI. Les
anciens assets sont conservés lors de la livraison pour rollback web seul.
