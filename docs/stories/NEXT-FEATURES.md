# Chill Center — Scénarios des prochaines fonctionnalités

25 novembre 2026 · Scénarios validés par l’utilisateur (« On les validé. Next ») · Web et Mac

Les parcours s’appuient sur [l’architecture validée](../architecture/decisions/ADR-0050-shared-viewing-library-and-playback.md).
Ils s’appliquent aux deux clients, avec les différences d’exécution indiquées
pour les téléchargements.

## Ce soir à deux

### DUO-01 — Choisir quoi regarder

Je choisis un deuxième profil et réponds à trois questions, dont une sur
le temps dont on dispose. Selon le contexte et les premières réponses,
le parcours peut demander si on a envie d’un film ou d’une série, ou de
retrouver quelque chose de familier ou de découvrir autre chose.
Les questions restent adaptées aux choix précédents sur deux passages
successifs, avec deux ou trois réponses proposées à chaque fois.

Je reçois une grille de neuf propositions au maximum, avec une raison courte
et un accès à chaque fiche. Avec 45 minutes, un épisode de 39 minutes peut
être proposé et un film de 95 minutes est écarté. Les titres vus par les deux
profils sont écartés. Ceux vus par une seule personne portent une indication.
Une durée inconnue est indiquée et vient après les durées confirmées qui
correspondent. S’il reste quatre candidats, la grille affiche quatre cartes.

### DUO-02 — Regarder sur un écran et retrouver deux progressions

Je choisis **Regarder à deux**. Je vois les noms des deux profils, l’épisode
le cas échéant et le point de départ avant de lancer le lecteur. Si l’invité
était à 32 minutes et que la séance part de 20 minutes, sa reprise reste à
32 minutes tant que la séance ne dépasse pas ce point. Une fois la séance
arrivée à 38 minutes, les deux profils retrouvent cette position en reprise.
Les règles de visionnage s’appliquent aux deux profils. Un titre déjà marqué
vu garde cet état et chacun retrouve sa propre note.

Si on enchaîne sur l’épisode suivant avec le mode duo actif, le lecteur
indique les deux profils et poursuit l’enregistrement pour chacun.
Fermer le lecteur clôt la séance commune. La prochaine lecture solo agit
sur le profil sélectionné.

### DUO-03 — Gérer un profil occupé et une coupure

Si le second profil regarde déjà une vidéo, **Regarder à deux** indique
le conflit et permet de revenir au choix des profils. La lecture en cours
continue sur l’autre appareil. Une coupure réseau met fin à la séance selon
les règles de connexion du lecteur. Les deux profils retrouvent la dernière
progression enregistrée à la reconnexion.

Pour la recette, simuler deux lancements concurrents pour le même invité,
un événement renvoyé à l’identique, puis un événement arrivé après la
fermeture. Une seule séance obtient les deux profils. Un même événement
compte une fois et une séance close n’accepte plus de progression.

## Télécharger la suite

### SUITE-01 — Préparer deux épisodes et remplacer ceux que je termine

Sur la fiche d’une série, j’active **Télécharger la suite** et choisis
**2 épisodes**. Les deux premiers épisodes à regarder selon la continuité
de la série se mettent en file. Si l’un d’eux est déjà téléchargé, sa copie
compte comme le premier épisode d’avance.

Après avoir terminé le premier des deux, la file ajoute le suivant dans
l’ordre. Avec **Supprimer après visionnage** activé, sa copie est supprimée
une fois la lecture terminée et le fichier libéré. Si je l’ai protégée via
**Conserver cette vidéo**, elle reste lisible. L’ordre peut traverser deux
saisons lorsque la suite est connue. Si E03 est manquant entre E02 et E04,
la file indique où elle s’arrête.

### SUITE-02 — Reprendre après une coupure

Je termine un épisode téléchargé pendant que le serveur est inaccessible.
Je peux lire les copies prêtes. Au retour de la connexion, le client
synchronise la progression puis complète l’avance à partir de l’état
retenu par les règles de synchronisation existantes.

Sur Mac, la file peut repartir avec l’application ouverte en arrière-plan.
Sur le web, elle reprend si la page est ouverte ou à sa prochaine ouverture.
Deux onglets ouverts sur le même profil sur la même origine ne téléchargent
qu’une copie du même épisode. Un autre profil conserve ses propres réglages
et sa propre file.

### SUITE-03 — Atteindre le plafond puis libérer de la place

J’ai fixé un plafond pour les téléchargements automatiques. Si le prochain
épisode dépasse la place restante ou si le stockage est plein, la file attend
et explique la raison. Je peux libérer de l’espace ou relever le plafond.
La file reprend lorsqu’il y a assez de place. Mes téléchargements manuels
et les épisodes protégés restent disponibles. Si je désactive **Télécharger
la suite**, les copies prêtes restent lisibles et la série cesse de programmer
de nouveaux épisodes.

## Bilan de la bibliothèque

### BILAN-01 — Comprendre les signalements

Dans **Administration → Bilan de la bibliothèque**, je vois la date du
dernier scan et les quatre vues prévues. Deux films identifiés sous la même
fiche TMDB sont signalés comme doublon probable. Deux séries homonymes
identifiées comme distinctes ne sont pas fusionnées. Une saison avec E01
et E03 signale E02. Un spécial en saison 0 ou un épisode TMDB dont la date
de diffusion est à venir n’est pas compté comme manquant.

Quand le NAS était inaccessible au dernier scan, la vue indique l’état
de la racine. L’ouverture de la page s’appuie sur l’état déjà enregistré.
Je peux ouvrir chaque fiche et conserver les deux entrées d’un doublon
probable si elles correspondent à deux copies que je veux garder.

### BILAN-02 — Vérifier une correction avant de l’appliquer

Je sélectionne une saga à un seul film. L’aperçu montre le nom de la saga
et son membre. Après confirmation, la saga disparaît, le film reste au
catalogue et son historique reste accessible. Si la saga a reçu un autre
film depuis l’aperçu, la confirmation échoue et la page me montre son état
actuel. Lorsque je passe par l’outil existant pour oublier une référence
indisponible, l’aperçu précise l’effet sur les historiques.
Annuler l’aperçu laisse la sélection intacte.

## Passer l’intro et le récap

### REPERE-01 — Sauter le bon segment

Un épisode contient un chapitre **Récap** de 00:05 à 00:40 et un chapitre
**Intro** de 01:12 à 02:04. Le lecteur montre chaque bouton sur l’intervalle
correspondant. Cliquer sur **Passer l’intro** avance à 02:04. Faire un retour
à l’intérieur du segment rend de nouveau le bouton disponible. La partie
sautée ne compte pas comme temps regardé. Le bouton apparaît aussi à la
reprise si la position enregistrée se trouve à l’intérieur du segment.

Un chapitre intitulé **Chapitre 1** ne déclenche aucun bouton. Les repères
d’une copie téléchargée restent disponibles lorsque le Pi est éteint.

### REPERE-02 — Ajuster un repère et gérer un fichier remplacé

Dans **Modifier** sur une saison, je peux choisir et prévisualiser le début
et la fin d’un repère d’intro ou de récap sur un épisode. Le réglage saison
s’applique à ses épisodes. Si je règle un épisode à part, il utilise ses
propres bornes. Retirer le réglage de l’épisode revient au repère de la
saison ou, à défaut, au chapitre explicite du fichier.

Une fin avant le début est refusée. Un repère de saison qui dépasse
la durée d’un épisode reste masqué pour cet épisode. Si un nouveau fichier
remplace la vidéo, le repère manuel propre à l’ancienne version est à
reconfirmer et le lecteur peut utiliser les chapitres valides du nouveau
fichier.

## Vérifications de livraison

Les parcours de lecture seront vérifiés sur le Mac et dans Chrome et Safari.
La recette mobile couvre Safari sur iPhone et Chrome sur Android, y compris la
réouverture après une coupure et les boutons du lecteur. Chaque lot doit
également vérifier qu’un client plus ancien garde ses actions habituelles,
qu’un client récent ouvre ses copies existantes avec un serveur plus ancien
et que le Pi ne lance ni scan global à l’ouverture du bilan ni transfert
au-delà de la file déjà bornée. Ces vérifications restent à exécuter.
