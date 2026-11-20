# Client web0.7.8 — maintien éveillé / playback wake lock

## Français

Publié et installé sur le Pi le20novembre2026. Source immuable `bb4b6179e8210fad65166425fb9da4106a1ae8d9`,
tag `web-v0.7.8`, séquence9. Six assets publics retéléchargés ; tailles, SHA-256,
signatures et tag vérifiés. Mac0.7.8/build18, serveur0.7.7 et thèmes0.7.1 inchangés.
La release globale `v0.7.8` historique reste immuable ; le paquet web courant est
celui de `web-v0.7.8`.

Le lecteur redemande le verrou d’écran s’il est retiré pendant une lecture
continue. Essais espacés et bornés, puis indication d’un refus persistant.
Une protection stable permet de récupérer de nouvelles révocations plus tard
dans une longue vidéo. Pause, arrêt, fin, erreur et page cachée libèrent le
verrou ; une autorisation tardive est relâchée si la lecture a changé.

384 tests JS dont20 Wake Lock,27 tests docs et vraie API Chromium avec vidéo
synthétique PASS. Révocation simulée via le sentinel, sans interrompre la vidéo ;
pause/reprise/arrêt réels vérifiés. Revue source indépendante APPROVED_SCOPED.
CodeQL JavaScript réussi sur la source exacte. Aucune modification Rust ou
nouvelle dépendance. Aucune extinction physique ni Safari/iPhone revendiqués.

Installation via API normale du helper existant, certificat et SPKI épinglés
réutilisés depuis la confiance Mac. Opération terminale `installed`, manifeste
signé identique à celui publié,106 fichiers installés et85 ressources HTTP
vérifiés contre le ZIP. Thèmes indépendants inchangés.
Aucune vidéo NAS lancée ni modification de profil/notes/reprises par le test.
Les fichiers actuels et reçus sont conservés ; aucun nettoyage ou effacement
des sauvegardes/anciens slots Pi. Retour arrière administrateur depuis les
paquets signés et reçus anti-rollback, sans migration de données.

Recharger l’onglet ou rouvrir la PWA pour charger la nouvelle version. Le
navigateur doit reconnaître un contexte sécurisé (HTTPS ou loopback fiable).
Son refus et les politiques d’énergie restent prioritaires ; aucune simulation
d’activité, vidéo cachée ni modification des paramètres système.

## English

Published and installed on the Pi on20November2026: web0.7.8/sequence9,
immutable source `bb4b6179e8210fad65166425fb9da4106a1ae8d9`, tag `web-v0.7.8`. All six public assets and
signatures verified. Mac0.7.8/build18, server0.7.7 and themes0.7.1 are unchanged.
The historical global `v0.7.8` release was not rewritten.

The player recovers screen wake locks revoked during continuous playback, with
bounded backoff and a visible persistent-failure message. Stable grants renew
the recovery budget for long videos. Pause, stop, end, errors and hidden pages
release it; obsolete grants never survive a changed playback state.
384 JS tests including20 wake cases,27 docs and a real Chromium video/wake-lock
journey passed. JavaScript CodeQL passed; source independently reviewed.
Physical idle-sleep and Safari/iPhone execution were not tested.

Installed via the existing signed LAN updater with the previously approved Mac
certificate/SPKI pin. Terminal installed receipt and 85 served files match the
published payload. No household video or personal-state write was used for QA.
Existing backups/rollback slots retained. Reload existing tabs after updating;
recognized HTTPS/browser support and OS power policies still apply.

[BUG0142](../debug/BUG-0142-web-wake-lock-recovery.md) ·
[ADR0043](../architecture/decisions/ADR-0043-web-wake-lock-recovery.md)

## Accès HTTP / HTTP access

Le cas utilisateur confirmé est un accès HTTP par IP locale depuis Chrome/Vivaldi sur macOS. L’API est absente dans ce contexte : utiliser l’accès HTTPS existant, reconnu par le navigateur. Son certificat local peut demander une validation manuelle par l’utilisateur. Aucune validation de certificat navigateur n’a été contournée.

The reported setup uses a LAN IP over HTTP in Chrome/Vivaldi on macOS. Screen Wake Lock is unavailable in that context. Use the existing HTTPS endpoint trusted by the browser; its local certificate may require manual user approval. No browser certificate warning was bypassed.
