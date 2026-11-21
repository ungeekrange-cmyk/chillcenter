# Téléchargements — scénarios Web et Mac

21 novembre 2026 · Scénarios soumis à validation. Ils couvrent le
[périmètre fonctionnel approuvé](../discovery/OFFLINE-DOWNLOADS.md),
[l’architecture validée](../architecture/decisions/ADR-0046-offline-downloads.md)
et [le réglage global](../architecture/decisions/ADR-0047-download-auto-delete-preference.md).
Sauf mention contraire, chaque scénario vaut pour les deux clients.

## DL-01 — Choisir ce que j’emporte

**Étant donné** que je suis en ligne, sur la fiche d’un film, d’un épisode,
d’une saison ou d’une série, **quand** je choisis **Télécharger**, **alors**
le client met en file la vidéo ou les épisodes disponibles de la sélection.
Pour une saison ou une série, il présente avant le lancement le nombre
d’épisodes et la place à prévoir. Il signale les épisodes indisponibles.
Une copie complète de la même version est réutilisée.

Sur le web, si la préparation hors ligne est impossible, l’action indique
le prérequis manquant que le navigateur permet d’identifier. Avec un ancien
serveur, les copies locales déjà prêtes restent accessibles depuis
Téléchargements.

## DL-02 — Suivre, annuler ou reprendre un téléchargement

**Étant donné** qu’une série est en cours de téléchargement, **quand** je
lance une autre vidéo ou que je navigue dans le catalogue, **alors** je
peux continuer à utiliser le client et consulter l’avancement de chaque
épisode. Si je quitte, annule, perds la connexion ou manque d’espace, les
épisodes terminés restent lisibles. La fiche distingue les téléchargements
interrompus des copies prêtes et permet de reprendre un échec.

À la reprise, le client poursuit la même version si le serveur la confirme.
Si le fichier source a changé, le client recommence cet élément. Deux onglets
qui demandent le même épisode ne créent qu’une copie. Aucune copie incomplète
n’apparaît comme prête à lire.

## DL-03 — Retrouver mes vidéos et mesurer l’espace utilisé

**Étant donné** qu’au moins une copie de mon profil est prête, **quand**
j’ouvre **Téléchargements** sous les autres catégories, **alors** la racine
montre mes films et mes séries. En ouvrant une série, je retrouve ses saisons
et les épisodes enregistrés. Une série ne prend qu’une place à la racine.

Le client indique la taille de chaque vidéo ou épisode, celle des copies
conservées par saison et par série, ainsi que le total de la rubrique. Sur
Mac, il indique l’espace libre et la taille totale du volume. Sur le web, il
affiche le quota et la place restante estimés pour le site. Une mesure
indisponible est signalée. Les téléchargements d’un autre profil ne
s’affichent pas dans ma liste.

## DL-04 — Ouvrir Chill Center et lire avec le Pi éteint

**Étant donné** que j’ai téléchargé des vidéos et préparé le client web en
HTTPS reconnu, **quand** je rouvre l’adresse dans le même navigateur ou
que je relance l’app Mac alors que le Pi est indisponible, **alors** j’accède
à mes téléchargements et aux profils déjà connus sur ce client. Je peux
lancer une vidéo, reprendre sa position, avancer ou reculer et choisir
les pistes audio et de sous-titres disponibles dans la copie.

Un MP4, un MKV avec sous-titres intégrés et une copie avec sous-titre externe
sont des parcours de référence. Sur le web, ce parcours inclut une vidéo
qui utilise un codec chargé à la demande. Si la connexion tombe pendant
une lecture lancée depuis Téléchargements, les octets locaux restent
accessibles. En ligne, un conflit de lecture propose le transfert habituel.

## DL-05 — Retrouver ma progression à la reconnexion

**Étant donné** que j’ai avancé hors ligne, **quand** le même serveur redevient
disponible pour le même profil, **alors** la progression se synchronise si
la révision de référence est toujours actuelle et qu’aucun bail actif ne
s’y oppose. Réessayer la même synchronisation laisse le même résultat.

Si un autre appareil a avancé entre-temps, sa progression serveur est
conservée et ma reprise locale reste accessible dans Téléchargements. Une
adresse réutilisée par un autre serveur ne mélange pas les copies ou les
progressions. Une vidéo ou un profil supprimé ne réapparaît pas sur le serveur
à la suite de cette synchronisation.

## DL-06 — Tenir la liste à jour après un effacement externe

**Étant donné** qu’un fichier local est supprimé via le Finder ou que le
navigateur retire la copie de son stockage, **quand** je consulte de nouveau
la rubrique, **alors** la copie manquante disparaît et les tailles sont
recalculées. Si je supprime le dernier épisode d’une série, la série
disparaît de la racine. Quand il ne reste aucune copie prête dans le profil,
la rubrique disparaît.

Si le navigateur efface toutes les données du site, je peux constater
la liste vide à la prochaine ouverture en ligne. Une erreur d’accès
au fichier est signalée comme telle afin que la copie puisse rester
récupérable.

## DL-07 — Régler une fois la suppression automatique

**Étant donné** que la suppression automatique est activée par défaut,
**quand** je la désactive dans **Téléchargements**, **alors**
**Configuration → Téléchargements** affiche le même état. Si je la réactive
dans Configuration, la rubrique reflète le changement. Le réglage se
conserve après fermeture et se modifie hors connexion. Dans le navigateur,
les onglets du même profil voient la valeur mise à jour.

Le réglage concerne les copies déjà présentes et les suivantes, pour le
profil sur ce client. Si aucune copie ne reste, Configuration permet
toujours de le modifier. Lorsque le réglage est activé, **Conserver cette
vidéo** permet de garder une copie précise. Désactiver puis réactiver
le réglage préserve ces choix individuels.

## DL-08 — Récupérer la place après un visionnage

**Étant donné** que la suppression automatique est activée et que je n’ai
pas choisi de conserver cette copie, **quand** je termine la vidéo après
avoir satisfait la règle de visionnage réel de sa fin, **alors** le client
enregistre d’abord ma progression, puis efface la copie et ses sous-titres
lorsqu’aucun lecteur ne l’utilise. Le total de la rubrique baisse. Pour les
vidéos courtes, la règle de visionnage de la fin déjà prévue s’applique.

Un saut à la fin, un arrêt, un marquage manuel ou le téléchargement d’une
vidéo déjà vue conserve la copie. Réactiver l’option conserve aussi les
copies terminées précédemment. Si je désactive le réglage avant l’effacement
en attente, la copie reste disponible. En cas d’échec de suppression, elle
reste visible avec une action de réessai. La vidéothèque et les copies des
autres appareils restent accessibles.

## DL-09 — Effacer des copies à la demande

**Étant donné** que je manque de place, **quand** je supprime le
téléchargement d’un film, d’un épisode, d’une saison ou d’une série,
**alors** seules les copies locales sélectionnées sont effacées. Avant
une suppression groupée, la confirmation indique les épisodes concernés et
la place à récupérer. L’action fonctionne hors connexion et quel que soit
le réglage de suppression après visionnage.

## Critères de remise

Démontrer DL-01 à DL-09 sur une bibliothèque de test. Vérifier un démarrage
à froid sans serveur dans le navigateur et dans l’app Mac, une suppression
externe et la persistance du réglage global. Couvrir Safari sur iPhone,
Chrome sur Android et un navigateur de bureau avec HTTPS reconnu. Ce
document décrit les vérifications à réaliser avant livraison.
