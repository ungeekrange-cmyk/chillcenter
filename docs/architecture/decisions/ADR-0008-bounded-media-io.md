# ADR-0008 — Mise en œuvre bornée des flux médias

2026-10-13 · IMPLEMENTED_DETAIL · auteur /root · CC-PLAY-001. Complément d'implémentation des ADR-0003/0004 acceptés ; `supersedes: NONE`. Aucune nouvelle fonction produit, aucun remplacement du protocole de transfert accepté. Ce complément n'est pas présenté comme une nouvelle validation utilisateur.

Le NAS peut bloquer dans un appel noyau : une ouverture ou un read sur l'actor SQLite ou les deux workers async ferait perdre la réactivité. Décision : quatre jobs fichiers au plus sur workers bloquants séparés, délai de réponse5s et conservation du permis jusqu'au retour de l'appel réel, même après annulation. Huit corps de flux maximum, blocs64Kio, backpressure HTTP. Les nombres sont des plafonds initiaux, pas des mesures Pi ; trois profils actifs maximum correspond à la cible familiale.

Ouverture en lecture seule par `openat`, parent après parent, `O_NOFOLLOW/O_DIRECTORY`, fichier final régulier. Aucun chemin libre envoyé par le client. Bail et ETag liés à la racine/révision et aux métadonnées dev/inode/taille/mtime/ctime ; métadonnées contrôlées avant/après chaque bloc. L'ETag est un identifiant de cohérence, pas une preuve cryptographique d'identité ni une autorisation. Le changement de racine et la publication d'un bail partagent l'ordre de verrouillage racine → registre ; suppression de profil et progression passent par l'actor.

Conséquences : pas de transcodage, mémoire de transport bornée ; NAS indisponible refuse la lecture sans créer une succession infinie de workers bloqués. Limites : un syscall noyau ne peut pas être interrompu par Tokio, ses quatre slots restent occupés jusqu'au retour. Des octets déjà livrés ne sont pas récupérables ; le client final devra appliquer l'échéance locale même avec son tampon. Aucun DRM ou auth ajouté. Les tests locaux ne prouvent ni Wi-Fi/NAS réel, ni capacité1080p du Pi.

Preuves : tests POSIX et Range, profils/replays/expiration, révocation entre blocs, fixture HTTPS/libmpv. Détails dans PLAYBACK-PLAN et PLAYBACK-EVIDENCE. Retour : ancien serveur dans une branche neuve, tous baux volatils oubliés ; aucune migration destructive.
