# Installer Chill Center comme application web

**Français** | [English](PWA-EN.md)

Ouvrir le client depuis l’appareil à installer, sur le même réseau que le Pi.
L’adresse HTTP habituelle (`http://<adresse-du-Pi>`, port configuré, souvent 80) reste utilisable comme
site. Pour une PWA installable et le maintien éveillé, utiliser l’accès HTTPS
existant : `https://<adresse-du-Pi>:<port-HTTPS>` (8443 dans l’installation testée). Le navigateur doit faire confiance au
certificat du serveur. Une simple exception de page peut ne pas suffire pour
le service worker ; vérifier le certificat et établir sa confiance sur l’appareil.
Ne pas désactiver la vérification TLS du navigateur.

- Chrome Android : menu du navigateur ⋮ → **Installer l’application**, lorsque proposé.
- Safari iPhone/iPad : **Partager → Ajouter à l’écran d’accueil**.
- Chrome/Vivaldi Mac : menu d’installation du site dans le navigateur.
- Safari Mac : **Fichier → Ajouter au Dock**.

L’application s’ouvre dans sa propre fenêtre. Le Pi et les vidéos restent
nécessaires : les films, les réponses API et les historiques ne sont pas
stockés par le cache hors ligne. Les profils/préférences HTTP et HTTPS utilisent
des stockages de navigateur distincts ; choisir son profil sur la nouvelle origine.

Après une mise à jour, rouvrir/recharger l’application pour charger le nouveau
client. Aucune relance automatique n’interrompt une vidéo en cours. Une PWA
utilise les codecs et le lecteur du navigateur/OS : l’installation ne remplace
pas un codec vidéo manquant. Le client web ne propose plus de bouton « Installer l’app » à partir de 0.7.10 ;
les éventuelles actions d’installation appartiennent au navigateur.

Les tests actuels couvrent Chromium macOS et des fixtures mobiles synthétiques.
Une validation physique Safari iPhone et Chrome Pixel reste nécessaire.
