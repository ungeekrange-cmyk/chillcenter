# Téléchargements et lecture hors connexion

Périmètre fonctionnel approuvé le 21 novembre 2026 pour le client web et
l’application Mac. L’utilisateur a confirmé : « Oui je valide, y compris
la suppression après visionnage activée par défaut. » Complète le [périmètre existant](SCOPE.md).

| Situation | Comportement attendu |
| --- | --- |
| Fiche d’un film ou d’un épisode, d’une saison ou d’une série | L’action **Télécharger** prend la vidéo ou les épisodes disponibles de la sélection. Elle indique l’avancement et permet d’annuler un téléchargement. Une copie complète déjà présente est réutilisée. |
| Un premier fichier a été téléchargé | La rubrique **Téléchargements** apparaît sous les autres catégories. Les films et les séries sont à la racine. Ouvrir une série donne accès aux saisons et aux épisodes enregistrés. |
| Le serveur est coupé avant l’ouverture du client | L’utilisateur accède aux téléchargements de son profil sur cet appareil dès l’ouverture. Il peut les lire, chercher dans la vidéo et choisir parmi les pistes disponibles dans la copie. Les sous-titres externes disponibles au téléchargement accompagnent la vidéo. |
| Un fichier local a été supprimé en dehors de Chill Center | L’entrée disparaît lorsque la rubrique est ouverte ou quand l’appareil y revient. La taille du groupe et le total sont recalculés. Si le dernier épisode d’une série manque, la série disparaît de la rubrique. La rubrique disparaît quand il ne reste aucune copie. |
| L’utilisateur consulte l’espace de stockage | Chaque vidéo affiche sa taille. Les saisons et séries affichent la somme des épisodes enregistrés. La rubrique affiche le total des téléchargements et les capacités libre et totale du volume sur Mac, ou du stockage alloué au site sur le web, avec la mention « estimation du navigateur ». |
| Un téléchargement est interrompu ou le stockage est plein | Un message distingue le fichier incomplet d’une copie prête à lire. Les vidéos déjà enregistrées restent utilisables et la rubrique comptabilise les copies terminées. |
| L’utilisateur veut faire de la place | La rubrique offre une suppression manuelle par vidéo, saison ou série. L’option **Supprimer après visionnage** s’applique à la copie locale sélectionnée et est activée par défaut. L’utilisateur peut la désactiver par vidéo. Elle agit après une fin de lecture réellement visionnée. Un saut en fin ou un marquage manuel comme vu ne déclenche pas la suppression. |

Chaque navigateur et chaque Mac conserve ses propres copies. La suppression
libère ces copies locales et préserve les sources de la vidéothèque.
La progression de lecture reste disponible sur l’appareil pendant la coupure.
Une lecture hors connexion ne peut pas obtenir un bail du serveur. La reprise
et l’historique lors de la reconnexion devront être définis de façon à préserver
les visionnages plus récents faits ailleurs.

Pour le web, l’utilisateur ouvre d’abord Chill Center en ligne, à l’adresse
HTTPS reconnue par le navigateur. Il revient à cette même adresse pour ouvrir
la rubrique hors connexion. Le navigateur doit autoriser le stockage local et
le service worker. Si ses données de site sont effacées, les copies conservées
dans ce navigateur disparaissent. La disponibilité doit être contrôlée sur le
navigateur et le certificat réellement utilisés avant de déclarer ce parcours
utilisable sur le Pi.

[Architecture web et Mac proposée](../architecture/decisions/ADR-0046-offline-downloads.md).
