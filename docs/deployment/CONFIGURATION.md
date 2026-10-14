# Configuration modifiable

L'adresse `192.168.1.42` et la racine `/srv/volumes/video` indiquées dans ce chat sont les valeurs prévues pour le foyer, **pas des constantes du logiciel**.

Le fichier [server.example.json](../../deploy/server.example.json) est un exemple à adapter avant installation. `listen` choisit une adresse LAN explicite/loopback et le port d'écoute, `tls_names` les IP/DNS contenus dans le certificat HTTPS, `data_dir` le stockage local de la base et de la configuration. Sans authentification, les écoutes wildcard (`0.0.0.0`, `::`) et les adresses publiques sont refusées. Changer l'écoute ou les noms HTTPS demande un redémarrage du service. Une modification des noms renouvelle le certificat avec la même clé ; les clients continuent à vérifier le nom utilisé.

`video_root` sert uniquement à initialiser la bibliothèque. Les modifications effectuées dans Administration sont conservées dans `data_dir/library.json` avec une révision anti-écrasement concurrent. Ce fichier appartient au compte du service, sans nécessiter de rendre `/etc` modifiable. Un montage temporairement indisponible reste configurable et ne provoque jamais une purge automatique. Retirer la racine laisse le catalogue existant intact.

Le client choisit indépendamment l'adresse HTTPS du serveur (DNS ou IP, port modifiable). Un changement de serveur implique son propre consentement TLS et son propre dernier profil ; aucune adresse NAS n'est nécessaire au client.

Utiliser un `data_dir` appartenant au compte du service, sans liens symboliques ni parent modifiable par d'autres comptes (hors répertoire temporaire sticky pour les tests). Il est protégé en `0700`, les fichiers privés sont créés en `0600` ; ne pas stocker la base sur le NAS. Une horloge Linux synchronisée avant démarrage reste nécessaire ; un cold boot à 1970 ou un retour en arrière avant le certificat est refusé. Les certificats existants sont vérifiés contre la clé/SAN/calendrier, avec recontrôle chaque minute après un saut NTP.

État : configuration persistante raccordée au service et à l'interface Mac, testée par HTTPS local (IP/port distincts de ceux du Pi). API `GET/PUT /api/v1/admin/library` ; mise à jour JSON `{ "video_root": "/montage/choisi", "expected_revision": 0 }`. Une révision périmée produit `409 CONFIG_CONFLICT` ; modifier la racine pendant un scan produit `409 SCAN_ACTIVE`. Aucun déploiement sur le Pi n'a été effectué.

Dans l'application Mac : choisir le DNS/IP et le port, vérifier l'empreinte TLS affichée puis accepter ou refuser ; choisir/créer un profil. Administration permet de saisir le chemin **Linux** du montage et de lancer le scan. Le scan n'est pas accessible depuis la barre du catalogue ; Actualiser recharge uniquement ses données. Les préférences audio/sous-titres/enchaînement appartiennent au profil ; le thème automatique/clair/sombre appartient au Mac. Aucun plugin de webview ne lit le NAS ou le disque média.

TMDB facultatif : le serveur lit `CHILL_TMDB_TOKEN` dans son environnement. Ne pas mettre ce secret dans Git, le JSON, l'archive portable ou le client. Pour systemd, l'opérateur peut configurer un EnvironmentFile local protégé avec un drop-in, puis redémarrer explicitement le service ; l'application signale si TMDB n'est pas configuré et permet toujours les corrections manuelles. Aucun token réel configuré pendant les essais locaux.

IA facultative : Administration → Organiser avec l'IA → CLI utilisé sur ce Mac. Choisir Codex CLI ou Claude Code, chemin local et connexion effectuée séparément dans Terminal. Les demandes externes sont explicites avec aperçu/CAS ; aucun abonnement ou appel payant créé par l'installation. Thème et ambiances sont propres à ce Mac ; audio/sous-titres/enchaînement au profil.
