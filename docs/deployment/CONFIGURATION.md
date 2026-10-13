# Configuration modifiable

L'adresse `192.168.1.42` et la racine `/srv/volumes/video` indiquées dans ce chat sont les valeurs prévues pour le foyer, **pas des constantes du logiciel**.

Le fichier [server.example.json](../../deploy/server.example.json) est un exemple à adapter avant installation. `listen` choisit l'adresse/port d'écoute, `tls_names` les IP/DNS contenus dans le certificat HTTPS, `data_dir` le stockage local de la base et de la configuration. Changer l'écoute ou les noms HTTPS demande un redémarrage du service. Une modification des noms renouvelle le certificat avec la même clé ; les clients continuent à vérifier le nom utilisé.

`video_root` sert uniquement à initialiser la bibliothèque. Les modifications effectuées dans Administration sont conservées dans `data_dir/library.json` avec une révision anti-écrasement concurrent. Ce fichier appartient au compte du service, sans nécessiter de rendre `/etc` modifiable. Un montage temporairement indisponible reste configurable et ne provoque jamais une purge automatique. Retirer la racine laisse le catalogue existant intact.

Le client choisit indépendamment l'adresse HTTPS du serveur (DNS ou IP, port modifiable). Un changement de serveur implique son propre consentement TLS et son propre dernier profil ; aucune adresse NAS n'est nécessaire au client.

État : contrats de configuration et persistance implémentés/testables ; interface d'administration et raccordement au service encore en cours. Aucun déploiement sur le Pi n'a été effectué.
