# BUG0142 — Maintien éveillé web perdu pendant une lecture continue

20 novembre 2026. Signalement utilisateur : le client web semble laisser la
machine se mettre en veille pendant la lecture. Branche `codex/web-wake-recovery`,
base1244292. La reproduction physique sur l'appareil utilisateur reste distincte.

## Cause confirmée

Le client appelle déjà `navigator.wakeLock.request("screen")`. Cependant,
`ScreenWakeLock.setActive(true)` retourne immédiatement si la lecture était déjà
active. Après un événement `release` du navigateur, le gestionnaire efface son
sentinel mais ne redemande plus de verrou tant que l'état actif ne change pas.
Une demande refusée une première fois a le même problème. Un sentinel déjà
libéré avant la résolution de la promesse est aussi pris pour une protection.

Trois régressions ajoutées échouent sur la base : récupération après révocation,
refus temporaire puis reprise, sentinel déjà libéré. Ce constat ne permet pas
d'attribuer toute veille utilisateur à cette seule cause : HTTP LAN/API absente,
restriction du navigateur ou mode économie d'énergie sont d'autres conditions.

## Correction

La lecture effective visible relance les acquisitions manquantes depuis le status
existant. Trois tentatives au maximum par rafale, espacées de1s puis5s. Une
protection stable pendant30s réinitialise le budget de récupération ; les refus
et révocations immédiates répétés ne créent pas de boucle. Pause/arrêt/fin,
transfert, erreur et page cachée libèrent immédiatement. Les autorisations tardives
restent liées à leur génération et sont relâchées si devenues obsolètes.

Une API absente ou un refus persistant produit un message utile pendant la
lecture, en FR/EN, pour vérifier HTTPS et économie d'énergie. Aucune simulation
d'activité, vidéo cachée, modification de TLS, permissions ou réglages système.

## Vérifications

- 20 tests Wake Lock PASS, dont les trois régressions rouges puis vertes,
  refus bornés, révocations immédiates, vidéo longue, pause pendant backoff,
  absence de l'API et toutes les libérations antérieures.
- 384 tests JavaScript partagés/web PASS.
- `scripts/test-web-wake-browser.cjs` : vraie vidéo synthétique et vraie API
  Screen Wake Lock dans Chromium sur origine loopback sécurisée. Acquisition,
  révocation simulée par `sentinel.release()`, récupération sans pause, pause,
  reprise et arrêt PASS. Aucun profil familial ou média NAS utilisé.
- Pas de mesure d'extinction physique ni de test Safari/iPhone revendiqués.

Publication/installation : à consigner après livraison. Retour arrière logiciel
possible sans migration ou changement d'état personnel ; garder les reçus signés.
Voir [ADR0043](../architecture/decisions/ADR-0043-web-wake-lock-recovery.md).
