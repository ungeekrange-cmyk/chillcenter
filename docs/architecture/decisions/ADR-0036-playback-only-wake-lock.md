# ADR-0036 — Maintien éveillé pendant la lecture effective

2026-11-06 · Implémenté sur `codex/playback-wake-lock`, base `bacee3c`.
Demande utilisateur : empêcher la veille sur web et macOS uniquement pendant
la lecture vidéo, jamais par la simple ouverture de l'app ni pendant une pause.
Complète ADR-0004 et ADR-0015 ; ne remplace aucun ADR.

macOS : assertion IOKit `PreventUserIdleDisplaySleep`, possédée par le lecteur,
créée seulement après chargement lorsque libmpv confirme `core-idle=no`, sans
pause, tampon en attente, fin, erreur, surface fermée ni bail expiré. La pulse
de sécurité existante (250 ms), indépendante du verrou réseau, observe aussi
la fin et le tampon. Pause, fermeture, rechargement et expiration libèrent
l'assertion ; son propriétaire la libère également à sa destruction.

Web : `navigator.wakeLock.request("screen")`, avec le même critère de lecture
effective et une page visible. Les événements média déclenchent la mise à jour
immédiate ; le status existant couvre aussi les moteurs sans événements DOM.
Pause/arrêt/transfert libèrent avant tout appel réseau. Une autorisation tardive
est libérée si sa génération ne correspond plus. Page cachée/quittée : libération ;
retour : nouvelle demande uniquement si la lecture continue. API absente ou refusée :
la lecture reste possible, sans boucle de demandes. Pas de vidéo artificielle,
simulation de mouvement utilisateur ni changement des paramètres système.

Limites : la politique système, une veille volontaire et la fermeture du capot
restent prioritaires. Le web exige un contexte sécurisé reconnu par le navigateur,
généralement HTTPS ; le listener HTTP LAN historique ne peut garantir cette fonction.
Aucun changement de TLS, permissions serveur, API, dépendance ou NAS.

Références primaires : [W3C Screen Wake Lock](https://www.w3.org/TR/screen-wake-lock/),
[assertion Apple](https://developer.apple.com/documentation/iokit/kiopmassertiontypepreventuseridledisplaysleep),
[propriétés mpv](https://mpv.io/manual/master/#properties).
Les signatures IOKit sont aussi vérifiées dans le SDK local `IOPMLib.h`.

Preuves et limites d'exécution : [BUG-0132](../../debug/BUG-0132-playback-idle-sleep.md).
