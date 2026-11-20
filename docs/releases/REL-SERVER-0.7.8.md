# Serveur 0.7.8 — relance et sous-titres

## Français

Publié et installé sur le Pi le 20 novembre 2026. Tag `server-v0.7.8`,
séquence9, source immuable `c0b01291b3bf1ca6bb8f97ffdb60be401f9315ba`. 16 assets publics
retéléchargés sans authentification, tailles/SHA256/signatures vérifiés.
Revues source/artefacts indépendantes APPROVED_SCOPED.

Une nouvelle action Lecture repart de zéro pour les vidéos marquées vues ou arrivées à la fin mesurée (250ms). La copie de départ est indépendante de la position durable : historique, note, compteur et statut vu conservés. Les reprises partielles restent à leur position. Un transfert de la vidéo en cours continue ; une nouvelle cible vue/terminée repart de zéro. La décision est atomique, et le cache borné préserve le token courant.

87 tests domaine +164 serveur PASS (1 ignoré préexistant),341 JS,28 docs et
Clippy strict PASS. ARM64/glibc2.28 reconstruit et ELF vérifié. Chrome isolé sur
Avatar S01E01 : piste native showing et cue active56.39–59.227s ; Projet dernière
chance : ancienne position9382.421s, relance au début puis avance2.58s. Aucune
lecture de test avec bail du foyer ni écriture personnelle. Mac0.7.8 compatible
par le contrat serveur ; lecture réelle dans l’application Mac et téléphones
physiques iPhone/Pixel non testés. CodeQL du SHA final réussi pour Rust, JavaScript, Python, C/C++ et Actions.

Helper signé : serveur0.7.8/séquence9 installé par la récupération root officielle
`apply server --retry-accepted`, après un échec de sauvegarde dans le service
systemd restreint. Sauvegarde complète maintenue, aucune force ni modification
des restrictions. Web0.7.11/séquence12 installé via l’API normale.
90 ressources HTTP web correspondent au payload signé. Progression
personnelle inchangée, thèmes/identité/TLS-SPKI conservés.
API2026-10-14 et schéma3 inchangés ; aucun transcodage serveur, changement de
certificat, réseau, PWA ou fichiers NAS. Mac0.7.8 et thèmes0.7.1 conservés.

Retour arrière : anciens slots et sauvegarde complète gardés ; procédure
administrateur officielle, sans déplacer manuellement les liens ni abaisser les
séquences. [BUG0145](../debug/BUG-0145-replay-and-untagged-subtitles.md).

## English

Published and installed on the Pi on20November2026. Source `c0b01291b3bf1ca6bb8f97ffdb60be401f9315ba`,
`server-v0.7.8`/sequence9; 16 signed public assets independently verified.

Fresh playback restarts watched or measured-completed videos within250ms of EOF. Only the returned start state changes; saved history, position, rating and watched status remain intact. Partial resumes and same-media transfers continue; different watched/completed targets restart. Transfer decisions are atomic and the bounded cache keeps the current token.

251 Rust tests,341 JS,28 docs and strict Clippy passed. Real-file Chrome tests
used bounded isolated SSH reads without household playback leases or writes.
Physical iPhone/Pixel and actual Mac-app playback were not tested. Existing
Mac0.7.8 is compatible. Signed helper/root recovery for server, normal API for web and payload hashes verified;
API/schema, identity, TLS trust, themes and NAS data preserved. Previous slots
and full backup remain available through the official recovery procedure.

Limite d’installation observée : le précontrôle automatique de sauvegarde serveur
reste à corriger ; cette installation a utilisé la récupération root officielle
sans force, avec sauvegarde complète. Les protections systemd sont conservées.
[BUG0146](../debug/BUG-0146-updater-offline-backup-sandbox.md).

Known installation limitation: the automatic server-backup preflight still needs
a durable correction. This installation used official local root recovery,
without force and with a full backup; systemd protections were preserved.
