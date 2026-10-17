# BUG0111 — Vignettes et plein écran web mobile ; recherche à vérifier

2026-10-17. Demande utilisateur : vignettes mobiles masquées par les contrôles,
plein écran inopérant, recherche paraissant annulée au changement de rubrique.
Base : `dc74ac0` ; branche `codex/web-iphone-mp3-r3`.

## Mécanismes confirmés

- Les règles CSS mobiles rendaient les actions lecture/vu/ajout toujours
  visibles sur les affiches. Elles sont masquées pour les cartes du catalogue
  mobile uniquement ; toucher la carte ouvre sa fiche. Actions desktop et
  actions dans la fiche conservées.
- La commande plein écran appelait seulement `panel.requestFullscreen`, sans
  le repli vidéo iPhone. La file de commandes partagée pouvait aussi attendre
  une opération avant l'appel exigeant un geste utilisateur. Le bouton mobile
  appelle désormais l'adaptateur directement dans le geste ; l'état reste
  l'état réel du navigateur, pas une classe CSS simulant un succès.
- Pour le Xvid rendu sur canvas, le plein écran vidéo natif ne s'applique pas.
  Si le navigateur n'autorise pas le plein écran d'un panneau, une explication
  remplace la guidance de rotation trompeuse ; lecture dans l'écran disponible.

## Recherche

Le handler partagé conserve déjà le champ de recherche au changement de
catégorie. Le test du build réel confirme aussi les résultats : saisie puis
clic avant expiration du debounce, passage Films/Séries/Documentaires/Toute
la collection, champ conservé et respectivement 1/0/0/1 cartes pour la fixture.
Le signalement d'annulation n'est pas reproduit sur ce candidat ; le garde
spéculatif initial a été retiré. Régression E2E ajoutée, pas de faux correctif.

## Statut et preuves

Implémentation locale et tests ciblés ; trois QA du build complet PASS
(mobile, desktop, lecture MP3 refusé nativement). Vrai plein écran Chromium,
API native iPhone simulée pour l'ordre synchrone et la confirmation d'état.
Une première exécution UI a expiré au redimensionnement ; le build régénéré
passe le même scénario sans modification du virtualiseur. L'échec intermédiaire
de l'ancien mock plein écran (déjà actif avant demande) a été corrigé dans le test.
Retest physique iPhone requis. Aucune modification Mac, serveur ou API.
Décision : [ADR0028](../architecture/decisions/ADR-0028-web-mp3-fallback.md).
