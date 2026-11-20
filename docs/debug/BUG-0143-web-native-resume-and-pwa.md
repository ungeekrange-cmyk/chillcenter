# BUG0143 — Reprise native web et installation PWA

20 novembre 2026. Demande : lecture défaillante Chrome Android/Pixel 9 Pro,
Safari iPhone 17, Chrome/Vivaldi macOS ; films The Holiday et Projet dernière
chance. Livrer aussi une PWA installable.

## Causes et correction

Le lecteur attendait le remplissage de tout le cache, jusqu’à 90 secondes de
média, avant d’appeler Play. Il prépare désormais trois secondes, puis continue
le chargement en arrière-plan avec le même élément vidéo natif.

Reproduction isolée sur le vrai MKV HEVC/AAC : reprise demandée à 5000 s,
vidéo tamponnée seulement à partir de 5001,042 s et téléchargement continu.
Une petite valeur de cache arrière déclenchait `SourceBuffer.remove` au milieu
du GOP. MSE retire aussi ses images dépendantes jusqu’à l’image clé suivante.
Le MP4/MKV fournit maintenant les points d’accès aléatoire de chaque fragment ;
la purge vidéo s’arrête exactement avant un point connu et conserve le GOP
courant, y compris avec zéro cache arrière. Le seek invalide cet index local.
Les Cues MKV sont filtrés sur la piste vidéo : une cue audio/sous-titre ne
constitue pas un point de reprise vidéo. Le flux ManagedMediaSource de Safari
respecte son budget natif et ses évictions, avec une amorce bornée même si un
événement de reprise de streaming manque après un seek.

La PWA manquait de PNG 192/512, d’icône Apple, de métadonnées et d’un parcours
Installer. Le précache atomique demandait aussi les décorations facultatives
du pack de thèmes indépendant, qui peuvent être absentes sur le serveur.
Icônes dérivées du logo existant, manifeste d’identité stable, bouton localisé,
invite native sur geste utilisateur et aide Safari/Android/Mac sont ajoutés.
Le worker ne précache que la coque versionnée ; API, médias, thèmes indépendants
et gros codecs à la demande restent réseau. La navigation vérifie le nouvel
HTML réseau pour éviter de figer une PWA installée sur une ancienne version.
Un échec de worker/certificat est expliqué dans l’aide d’installation.

## Preuves et limites

Tests Node ciblés et navigateur synthétique : H.264/AAC 11,025 kHz MP4,
HEVC/AAC MKV avec GOP longs/B-frames, cache arrière nul, pause, reprise et sauts
hors tampon. Les deux fichiers NAS ont été lus par plages SSH bornées dans un
contexte de diagnostic local ; aucune acquisition de bail ni écriture de profil.
Chrome macOS décode les deux débuts, la reprise MP4 à 1200 s et, après correction,
la reprise HEVC à 5000 s (progression à 5002,13 s, aucune erreur native).
La valeur réduite du cache appartient au diagnostic, pas à une preuve du réglage
personnel de l’utilisateur. Ce défaut n’explique pas à lui seul chaque échec
sur chaque appareil signalé.

Contrôle PWA Chromium loopback : manifeste/icônes, vrai worker installé,
critères CDP hors restriction normale du contexte privé de test, navigation
réseau, invite/dismissal/erreur et aide iOS. Aucun test physique Pixel/iPhone,
ni Safari réel ou batterie revendiqué. Le décodeur reste celui du navigateur ;
un codec absent de l’OS ne devient pas lisible par installation de la PWA.
Aucun transcodage serveur, nouvelle dépendance, migration ou modification TLS.

Références : [suppression MSE](https://www.w3.org/TR/media-source-2/#sourcebuffer-coded-frame-removal),
[ManagedMediaSource](https://developer.mozilla.org/en-US/docs/Web/API/ManagedMediaSource),
[installation PWA](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Guides/Making_PWAs_installable).

## Vivaldi macOS en interface réelle

Après échec du lanceur Playwright (déconnexion avant newPage), contrôle CUA
du vrai Vivaldi sur une fixture loopback, sans média ni profil personnel :
H.264 et HEVC reprennent à4s, sautent à7,5s/12,5s/4s, puis progressent
avec de nouvelles images décodées. Résultats : H.2644,326s/51images,
HEVC4,324s/53images, décodeur navigateur, aucune erreur.
