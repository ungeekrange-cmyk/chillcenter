# BUG-0153 — Le choix des sous-titres disparaît à la réouverture

26 novembre 2026 · macOS 0.7.13 et web 0.7.17.

Après fermeture de l’application ou de l’onglet, le lecteur revient aux
préférences générales du profil. Les choix par série de
[BUG-0152](BUG-0152-untagged-subtitles-between-episodes.md) vivaient dans
l’instance du lecteur. Une nouvelle instance démarrait avec une mémoire vide.

Le choix manuel de la piste et l’état « Désactivés » sont désormais enregistrés
dès la sélection. Ils sont relus au lancement d’une vidéo associée à la même
série, pour le même profil et le même serveur. La piste est recherchée
d’après sa description, même si son identifiant dans la vidéo a changé.

Le test web recrée le lecteur en réutilisant le stockage du site. Il
reproduit la perte avant correction et vérifie la piste, la désactivation et
l’isolation entre profils. Le test Mac recrée le lecteur avec le même dossier
local et vérifie les choix par série, profil et serveur.

Le contrôle Chromium local a recréé le navigateur avec le même profil de
test et confirmé la restauration d’une piste sans langue et du choix
« Désactivés ». Les suites ont validé 380 contrôles web et documentaires
et 50 tests macOS. Le test macOS optionnel qui contacte GitHub était ignoré.
