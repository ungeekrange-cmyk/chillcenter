# Client web 0.7.9 — lecture native et PWA

## Français

Publié et installé sur le Pi le20novembre2026 : web0.7.9/séquence10.
Source immuable `6470bbd0ee77983596d9a6f1827288da1646d68e`, tag `web-v0.7.9`.
Six assets publics, tailles/SHA-256/signatures et tag vérifiés ; revue artefacts
ARTIFACT_APPROVED_SCOPED. CodeQL JavaScript exact réussi.
Mac0.7.8, serveur0.7.7 et thèmes0.7.1 restent compatibles.

Démarrage après une courte amorce puis chargement en arrière-plan. La purge
conserve les images clés nécessaires au GOP courant, même sans cache arrière.
Reprises MKV indexées sur la vidéo ; gestion du budget ManagedMediaSource.
Le navigateur/OS décode H.264/HEVC quand il les prend en charge ; aucun
transcodage serveur ni nouvelle dépendance.

PWA : icônes PNG/Apple/maskable, bouton Installer localisé et aide par appareil.
Coque seule en cache, navigation réseau pour charger les mises à jour, vidéos
et API toujours sur le serveur. HTTPS de confiance requis pour installer.
[Installation PWA FR](../updates/PWA-FR.md).

389 tests unitaires JavaScript,28 contrôles documentaires, Chrome natif avec
HEVC réel, PWA/worker et régression du lecteur mobile synthétique PASS.
Revue indépendante APPROVED_SCOPED. Les vrais fichiers The Holiday et Projet
dernière chance ont aussi été lus par plages bornées via SSH dans Chrome,
sans bail ni écriture de profil. Reprise HEVC5000s réparée et réellement avancée.
Pas de preuve physique Safari/iPhone, Pixel/Android ou de consommation batterie.
Vivaldi macOS vérifié ensuite par CUA dans sa vraie fenêtre : reprises et
sauts H.264/HEVC avec de nouvelles images décodées, aucune erreur. Son lanceur
Playwright se déconnecte avant la création de page ; ce résultat n’est pas utilisé. La confiance du certificat familial pour un worker reste distincte
du test PWA sur loopback. Aucun fichier NAS ou état personnel modifié.

Retour arrière : garder les slots et paquets signés précédents ; les contrôles
anti-rollback exigent le parcours administrateur prévu. Aucune migration.

## English

Published and installed on the Pi on20November2026: web0.7.9/sequence10,
immutable source `6470bbd0ee77983596d9a6f1827288da1646d68e`, tag `web-v0.7.9`.
All six public assets, hashes/signatures/tag verified; artifact review approved,
exact JavaScript CodeQL passed. Compatible with
Mac0.7.8, server0.7.7 and themes0.7.1.

Playback starts after a short prime and continues buffering in the background.
Video eviction retains the current GOP's random-access point, including zero
back cache. MKV seeks use video cues; Safari managed buffers follow native
streaming signals. Browser/OS native decoding remains in charge, with no server
transcoding or new dependency.

Installable PWA icons, Apple metadata, localized Install action and device help.
Only the shell is cached; navigation checks the current server HTML. API/media
remain online and installation requires trusted HTTPS.
[PWA installation EN](../updates/PWA-EN.md).

389 JS unit tests,28 documentation checks, real Chrome HEVC/native resume and
seek, PWA/worker and synthetic mobile playback regression passed. Independent
review APPROVED_SCOPED. Bounded read-only SSH ranges also verified the two
reported household files in an isolated Chrome controller without a playback
lease or personal-state writes. Physical Safari/iPhone and Pixel/Android are
not validated. Vivaldi macOS subsequently passed real-UI CUA checks on isolated H.264/HEVC
fixtures: resume/seeks advanced and decoded new frames with no errors. Its
Playwright launcher disconnects before page creation; that is not counted as proof. Loopback PWA
tests do not establish trust in the household TLS certificate. Existing rollback
slots/backups retained; no data migration or NAS modification.

[BUG0143](../debug/BUG-0143-web-native-resume-and-pwa.md).

## Vérification installée / Installed verification

Installation par l’API normale du helper signé, TLS et SPKI existants épinglés.
91 ressources servies correspondent au ZIP publié ; pack de thèmes indépendant
conservé. État personnel de lecture inchangé pendant l’installation : True.
Aucun bail/écriture QA sur le foyer. Le paquet contient112 fichiers physiques
(111 déclarés plus build.json). La confiance du certificat pour une installation
PWA sur chaque appareil reste à établir ; aucun réglage TLS système modifié.

Installed through the normal signed helper API using the existing TLS/SPKI trust.
91 served resources match the public ZIP. Independent themes retained. No QA
household lease/write. Physical mobile browser and certificate trust validation
remain device-specific; no OS trust changes were made.
