# 0.7.9 — Téléchargements hors connexion / Offline downloads

## Français

Chill Center permet de télécharger un film, un épisode, une saison ou une série sur le web et sur Mac. La rubrique **Téléchargements** affiche les copies et l’espace utilisé. Les copies s’ouvrent avec leurs sous-titres quand le serveur est arrêté. Le réglage **Supprimer après visionnage** est activé par défaut et peut être modifié dans la rubrique ou dans Configuration.

Cette livraison associe le serveur 0.7.9, le client web 0.7.13 et macOS 0.7.9 (build 19). Le web requiert une adresse HTTPS reconnue pour les téléchargements. Le Mac cible Apple Silicon et macOS 15 ou ultérieur. Rechargez le client web après installation. Les tests sur iPhone et Android restent à faire.

## English

Chill Center can download a movie, episode, season or series on the web and Mac. **Downloads** lists local copies and their storage usage. Play them with subtitles while the server is offline. **Delete after watching** defaults to on and can be changed in Downloads or Settings.

This release pairs server 0.7.9, web 0.7.13 and macOS 0.7.9 (build 19). Web downloads require a trusted HTTPS address. The Mac app targets Apple Silicon with macOS 15 or later. Reload the web client after installation. Physical iPhone and Android testing is pending.

[Exact source](https://github.com/ungeekrange-cmyk/chillcenter/tree/8e03e97bf67299b386c244c7ddccc926c55d111e)

## Distribution

[macOS 0.7.9](https://github.com/ungeekrange-cmyk/chillcenter/releases/tag/mac-v0.7.9) · [Serveur 0.7.9](https://github.com/ungeekrange-cmyk/chillcenter/releases/tag/server-v0.7.9) · [Web 0.7.13](https://github.com/ungeekrange-cmyk/chillcenter/releases/tag/web-v0.7.13).

## Vérification / Verification

GitHub Latest : `v0.7.9`. Quatre releases publiques, 53 fichiers contrôlés
par taille et SHA-256. Les manifestes Minisign, le DMG et les archives
d’installation accompagnent les paquets. L’archive des sources correspondantes
contient le commit exact et les entrées amont épinglées du lecteur.

GitHub Latest is `v0.7.9`. The four public releases contain 53 assets checked
by size and SHA-256. Minisign receipts and the exact corresponding sources are
published alongside the installers.

Le Mac a été installé depuis l’archive signée 0.7.9, build 19. La signature du
bundle, le chargement de libmpv et la relance de l’app ont été vérifiés.
Le précédent bundle a été conservé pour le retour arrière.
The signed Mac archive was installed and the bundle signature, libmpv loading
and application relaunch were verified.

Sur des vidéos synthétiques, Chromium et le client natif Mac ont relu une
copie avec sous-titres après arrêt du serveur et effacé le fichier à la fin.
Le serveur ARM64 conserve un minimum glibc 2.28. Les scénarios mobiles
physiques restent à exécuter.
