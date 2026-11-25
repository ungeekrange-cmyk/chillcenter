# ADR-0050 — Séances à deux, bilan, téléchargements et repères

24 novembre 2026 · Architecture validée par l’utilisateur (« Validé. Go ») · Client web, client Mac et serveur du foyer

Le périmètre approuvé couvre Ce soir à deux, le téléchargement de la suite,
le bilan de la bibliothèque et les repères d’intro et de récap. La cible reste
le Raspberry Pi qui sert les fichiers du NAS et conserve les métadonnées.

## Ce soir à deux

Le profil actif choisit le deuxième profil dans **Demander conseil** sur Mac
et dans **Ce soir à deux** sur le web. Le parcours **Surprends-moi** est encore
au stade de la proposition. Le mode duo utilisera son principe de trois
questions à deux ou trois réponses, dont une sur le temps disponible.
La grille affiche jusqu’à neuf films ou séries du catalogue. Pour une série,
le budget porte sur l’épisode suggéré. Les titres vus par les deux profils
sont écartés. Les titres vus par un seul restent possibles et sont
identifiés. Une durée inconnue est signalée et classée après les candidats
dont la durée correspond. Le classement privilégie le plus faible des deux
scores individuels, puis la moyenne. Les réponses et les genres, thèmes et
historiques déjà enregistrés suffisent. Le serveur calcule le résultat
localement et renvoie les suggestions et leurs raisons.

Depuis un résultat, **Regarder à deux** montre les deux profils et
la position de départ avant de lancer un seul lecteur. Le serveur associe
les deux profils à la même session et vérifie que les deux sont libres.
Si l’un d’eux utilise déjà un lecteur, on peut attendre la fin de sa séance
ou choisir un autre profil. La lecture d’un épisode suivant prolonge le duo
uniquement tant que le mode reste affiché dans le lecteur.

Les événements valides du lecteur mettent à jour les deux progressions dans
une même transaction. Pour la personne invitée, la position ne recule pas
et un titre déjà vu conserve ce statut. Les seuils actuels déterminent le
passage à « vu ». Les notes restent attachées au profil qui les saisit.
Le mode duo requiert une connexion au Pi et la fermeture de sa session
libère les deux profils. La lecture solo suivra le contrat actuel.
[Contrats de lecture et profils](../CONTRACTS.md).

## Télécharger la suite

Sur la fiche série, **Télécharger la suite** propose de garder **1** ou **2**
épisodes d’avance. L’option est désactivée au départ, propre au profil et à
l’appareil. Le premier téléchargement démarre dès l’activation. Ensuite,
la file se complète après un épisode marqué vu et au retour de la connexion.
Elle reprend la continuité déjà utilisée par **Lire** et **Épisode suivant**.
Un ordre incertain ou un épisode manquant interrompt la sélection à cet
endroit. Une copie prête ou déjà en file compte dans l’avance et reste
unique par épisode et version.

La file de transfert existante garde ses limites. Un plafond local, réglable
dans Téléchargements, s’applique aux copies automatiques. Quand ce plafond ou
l’espace disponible est atteint, la file attend et indique la raison. Les
éléments conservés manuellement et les téléchargements existants restent
disponibles. La préférence globale **Supprimer après visionnage** continue de
gérer les copies terminées, en respectant leur protection individuelle.
Le Mac peut remplir la file tant que l’application tourne. Sur le web, le
site reprend la file à l’ouverture ou au retour en ligne pendant qu’il est
ouvert, avec les prérequis HTTPS et stockage actuels.
[Téléchargements actuels](ADR-0046-offline-downloads.md) · [Suivi WebKit de la synchronisation périodique](https://bugs.webkit.org/show_bug.cgi?id=204117).

## Bilan de la bibliothèque

Une rubrique dans **Administration** regroupe quatre états du catalogue.

| Vue | Critère |
| --- | --- |
| Doublons probables | Identité TMDB commune pour un film, ou identité de série et numéros de saison et d’épisode communs. Les rapprochements d’empreintes existants restent disponibles pour les fichiers réapparus. |
| Épisodes manquants | Trous dans une saison numérotée, ou épisodes déjà diffusés confirmés par les métadonnées TMDB disponibles. Les épisodes spéciaux et les ordres ambigus ne rentrent pas dans le décompte automatique. |
| Fichiers indisponibles | État enregistré par le dernier scan, en précisant si la racine de stockage était elle-même indisponible. |
| Sagas à un seul titre | Sagas dont la liste de membres actuelle contient exactement un film. |

Le bilan travaille sur un instantané du catalogue et affiche la date du
dernier scan. L’ouverture de la page ne déclenche ni scan du NAS ni appel TMDB
en masse. Le clic mène à la fiche ou à l’outil de correction existant.
Avant une suppression de saga ou une modification de catalogue, l’aperçu
énumère les éléments et l’effet sur les historiques, là où ils sont concernés.
Le serveur revalide les révisions avant d’appliquer la sélection. Les fichiers
source restent intacts. Deux fichiers de même identité TMDB peuvent rester
dans le catalogue si l’utilisateur veut les garder.

## Passer l’intro et le récap

Le serveur lit les chapitres disponibles avec la même approche bornée que pour
la mesure de durée. Un repère exploitable contient un début et une fin.
Seuls les libellés explicites d’intro et de récap sont interprétés. Les
repères sont conservés par version de fichier et fournis au lecteur en
lecture en ligne et dans les métadonnées d’une copie hors connexion.
Le bouton **Passer l’intro** ou **Passer le récap** apparaît dans le lecteur
pendant l’intervalle et utilise le saut existant. Le temps sauté ne compte
pas comme du temps regardé.

Depuis **Modifier** sur une saison, on peut prévisualiser et enregistrer un
repère commun. Un réglage propre à l’épisode prévaut sur celui de la saison,
et les chapitres explicites du fichier servent en l’absence de réglage.
Un repère invalide pour la durée de l’épisode reste masqué. Si le fichier est
remplacé, les chapitres sont relus et les repères propres à l’ancienne version
doivent être reconfirmés. La première livraison utilise les chapitres
exploitables des conteneurs pris en charge et les repères manuels.
[Spécification des chapitres Matroska](https://www.rfc-editor.org/rfc/rfc9559.html) · [Segments Jellyfin](https://jellyfin.org/docs/general/server/metadata/media-segments/).

## Compatibilité et ordre de livraison

Les fonctionnalités serveur sont annoncées séparément aux clients. Les
clients plus anciens conservent leurs parcours actuels. Les copies locales
existantes restent lisibles après mise à jour. Les nouvelles écritures
utilisent l’identité du serveur, du profil ou de la série et les révisions
adaptées à chaque opération.

Le bilan peut être livré à part. Le téléchargement automatique vient ensuite
avec sa file locale. Les repères de lecture peuvent suivre indépendamment.
Le mode duo exige une vérification dédiée des baux des deux profils, du
passage à l’épisode suivant et de la progression avant sa mise en service.
