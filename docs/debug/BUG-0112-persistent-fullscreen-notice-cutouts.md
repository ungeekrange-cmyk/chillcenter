# BUG0112 — Bandeau plein écran persistant et zones caméra mobiles

2026-10-17. Base `4f41a86`, branche `codex/web-mobile-fullscreen-r4`.
Retour humain Safari : bandeau plein écran impossible toujours visible ; demande
de disparition rapide, d'une autre tentative plein écran et de protection des
encoches/Face ID/poinçons. Le signalement ne prouve pas qu'une API manquante peut
être forcée ; aucune nouvelle application native n'est demandée à implémenter.

## Causes et correction

Le bandeau n'avait pas de durée limite, et `refreshMedia()` le maintenait affiché
à chaque état du lecteur. Délai unique **2 500 ms**, jamais renouvelé par les
états, changements d'orientation ou de langue ; une nouvelle tentative explicite
peut le réafficher brièvement. Fermeture du lecteur : timer annulé.

Autre chemin WebKit : `webkitSetPresentationMode("fullscreen")` si disponible
et accepté, lorsque l'entrée vidéo classique échoue ; variantes historiques
des noms supportées. L'API standard reçoit `navigationUI: "hide"`. Les appels
restent dans le geste utilisateur et seul l'état réel vaut succès. Aucune boucle
de forcing, aucun faux plein écran CSS, aucune demande audio/vidéo supplémentaire.
Un canvas Xvid peut rester refusé en plein écran natif par Safari.

Les marges caméra reposent sur les quatre `safe-area-inset-*` de l'OS, pas sur
une détection de modèle : vidéo, commandes, fiches et rail paysage hors des zones
sensibles, fond noir jusqu'au bord. `viewport-fit=cover` existant conservé.

Pendant la QA, densité de grille instable à 430 px : mécanisme reproduit à une
largeur utile de `366.03125`, quotient théorique 3 mais flottant
`2.9999999999999996`, donc deux colonnes. Tolérance numérique de quelques ULP
dans l'adaptateur web seulement ; régression sur des milliers de largeurs
fractionnaires, aucune modification du virtualiseur commun Mac.

## Preuves et limites

Tests déterministes du timer, poll/resize/langue après expiration, reprise d'une
tentative, fermeture ; API WebKit simulée et confirmation d'état ; géométrie
Chromium avec insets gauche/droite simulés et bandeau réellement expiré.
Ces preuves ne sont pas un essai physique iPhone ni une garantie de plein écran
Safari pour tous les codecs. Aucun transcodage serveur, changement Mac/API/NAS,
nouvelle dépendance ou modification du moteur audio.

Sources primaires : [WebKit, safe areas](https://webkit.org/blog/7929/designing-websites-for-iphone-x/),
[Apple, mode de présentation](https://developer.apple.com/documentation/webkitjs/htmlvideoelement/1631224-webkitsetpresentationmode).
