# BUG-0070 — client web : le lecteur affiche le titre de la vidéo précédente pendant l'ouverture de la suivante

- Statut : **corrigé** le 15 octobre 2026 (branche `codex/web-pwa`). Nature : **bug applicatif** (client web).
- Numérotation : BUG-0070 pour ne pas entrer en collision avec la suite BUG-0052…0063 de la branche du client Mac.

## Symptômes

Lancer une vidéo, revenir au catalogue, en choisir une autre : pendant le chargement, le panneau du lecteur affiche encore le **titre de la première vidéo**, avec « Mise en mémoire tampon… » ou « Chargement… ». Les contrôles restent inactifs jusqu'à la fin de l'ouverture.

## Environnement

Client web (PWA) sur le Pi, navigateur de bureau ; film MKV lourd (ouverture de l'index du conteneur sur le réseau, plusieurs secondes).

## Hypothèses

1. Le titre est mal mis à jour par l'interface partagée (`ui/connected/player.js`).
2. Le lecteur web ne répond qu'après l'ouverture complète du fichier, donc l'interface garde l'ancien état affiché.

## Preuves

- `chillPlay` (interface partagée) met la note à « Chargement de la vidéo… » mais **ne touche pas `#player-title`** ; le titre n'est réécrit qu'au premier rendu issu de la réponse à `start`.
- `WebPlayer.start()` / `begin()` attendait `engine.open()` (démuxage, premiers fragments) **avant de répondre** : tout le temps d'ouverture, le panneau gardait l'ancien titre. Sur le client Mac l'ouverture native répond presque tout de suite, d'où l'absence du symptôme.
- Les contrôles pendant l'ouverture agissaient sur un moteur pas encore ouvert.

## Correction

`begin()` répond **immédiatement** avec une vue « chargement » (`native.loaded=false`) portant le titre de la **nouvelle** vidéo (issu de `playback-info`, déjà reçu), et ouvre le fichier **en arrière-plan** (`finishOpening`). Un échec est livré par le `status` suivant (`closed(error)`, une seule fois, bail libéré) ; `stop()` ou un nouveau `start` pendant l'ouverture l'**annule** (moteur fermé, jamais promu en moteur vivant) ; `seek`, `pause`, vitesse pendant l'ouverture ne ferment plus le panneau. L'interface partagée n'est pas modifiée (aucun conflit avec la branche Mac).

## Régression

`web/test/player.test.js` : « start answers at once with the NEW title while the file is still opening », « a failed open is reported by the next status poll, and the lease is released », « leaving the player (or starting another video) during the open cancels it » ; les tests existants passent avec un banc qui attend l'état stabilisé. 54 tests web, 0 échec. **Non vérifié** : en navigateur réel sur le Pi (à faire après déploiement).
