# ADR0042 — Durées réelles et marquage de la saison dans Reprendre

9 novembre2026, demande utilisateur explicite ; implémentation livrée dans Mac0.7.8 et serveur/web0.7.7.
Remplace le critère position>0 seul de l’ADR0041 lorsque la durée réelle est
disponible. Complète ADR0038 sans changer ses trois rangées de20 ni leur portée
par profil. Le choix utilisateur pour la coche d’une série est la saison affichée.

## Durée et sélection

La durée d’un fichier peut être lue dans son en-tête MP4/MOV (`mvhd`), Matroska
(`Info/Duration`, `TimestampScale`) ou AVI (`avih`, OpenDML). Lecture de faits
du conteneur, sans décodage, transcodage, outil externe ou durée TMDB supposée.
Références primaires : [QTFF](https://developer.apple.com/documentation/quicktime-file-format/movie_header_atom),
[RFC9559](https://www.rfc-editor.org/rfc/rfc9559.html),
[AVIMAINHEADER](https://learn.microsoft.com/en-us/previous-versions/ms779632%28v%3Dvs.85%29).

Le serveur enrichit la projection Home après sa lecture SQL ; le domaine ne lit
aucun fichier. Au maximum20 candidats,64Kio/2048 lectures par fichier,200ms de
budget de parcours par fichier et2s par lot. Aucun parcours de bibliothèque.
Un seul lot de métadonnées simultané par service, partagé avec un des quatre
permis d’I/O existants. Un syscall bloqué conserve ses permis après le timeout
HTTP5s : aucun remplacement illimité, trois permis restent disponibles aux flux.
En cas d’indisponibilité, la projection sans pourcentage reste utilisable.

Ouverture descriptor-relative `open_indexed`, uniquement depuis les chemins
indexés sous les racines configurées ; pas de symlink, chemin client ou URL libre.
Contrôle de version avant/après lecture et de racine/chemin avant publication.
Cache mémoire plafonné à512 entrées, y compris échecs ; clé chemin + device,
inode, taille, mtime et ctime. Les durées sont communes, sans donnée de profil.

Une vidéo disponible/non vue a une reprise significative à partir de5 % et
avant la fin, si sa durée est connue. Une reprise héritée sans durée reste
accessible ; l’interface annonce la position et l’absence de durée. Aucun
historique n’est réécrit par la lecture Home. Les durées du conteneur ne sont
jamais enregistrées comme événements de lecture validés par bail. Les règles
de marquage automatique F-08 restent inchangées. Pas de migration de schéma.

## Saison, coche et animation

`episode_counts:{watched,total}` compte tous les membres de la saison proposée,
y compris indisponibles ; saison inconnue distincte, même règle de regroupement.
Le pourcentage de la série décrit ses épisodes terminés, pas la position d’un
épisode représentatif. Le pourcentage du film utilise sa position/durée réelle.

Le PUT historique `media/{id}/watched` accepte aussi `scope:season|series` avec
`expected_revision` obligatoire (révision commune `watch_revision` de Home).
L’épisode référencé détermine la saison exacte. Le domaine vérifie la révision,
sélectionne au plus1000 épisodes et applique le tout dans une transaction.
Métadonnées modifiées :409, aucun marquage partiel. Position, notes et dates des
épisodes déjà vus sont conservées. La portée vidéo historique reste compatible.
Le client expose la portée saison demandée ; la portée série n’est pas utilisée
par la coche de Reprendre. Les opérations sont liées au serveur/profil capturés.

Le bouton ne retire rien avant le succès serveur. Puis fondu180ms et translation
des cartes suivantes300ms ; l’état en erreur reste visible et réessayable.
Les lectures d’Accueil sont différées durant une mutation, les cartes conservées
par identité et les réponses d’anciens contextes rejetées. Focus sans défilement,
marge de snap cohérente et mouvements réduits sans animation.

Rollback logiciel possible sans réécriture des données ; le marquage manuel reste
un état utilisateur réversible depuis les fiches. Publication, installations et mesure du déploiement restent des opérations distinctes de la validation technique.

Livraison et installations vérifiées : [REL-0.7.8](../../releases/REL-0.7.8.md). Le SHA des exécutables, les signatures et les preuves sont liés à la source immuable ; les documents de livraison sont un commit ultérieur, sans déplacement des tags.
