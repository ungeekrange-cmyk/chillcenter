# Revue indépendante / Independent review — client web mobile

Date: 2026-10-17. Base: `085f731248a3f2099917c81b329a866c09d63727`.
Candidate reviewed before commit: build `b1722f00c0a6`, with source hashes
`mobile-ui.js` `87871c8f…9acf`, `mobile.css` `df999c4e…c78cd`,
`mobile-playback.js` `2d64fd6a…7d88`, `player-web.js` `25cdc709…7e02`.

## Français

**APPROVED_SCOPED** — aucun défaut bloquant ou majeur restant dans la portée
du client web responsive. Le candidat conserve le client Mac et le serveur hors
du diff, fournit la navigation mobile à cinq destinations avec le panneau Plus,
des actions tactiles d'au moins 44px, une grille virtualisée de deux colonnes à
320px puis trois colonnes, les parcours sagas/listes/configuration et les libellés
français/anglais. Le bureau retrouve sa barre latérale.

La lecture demande le vrai plein écran depuis le geste utilisateur, ne prétend
pas réussir si le navigateur le refuse, puis tente le verrouillage paysage. La
fermeture nettoie un plein écran accordé tardivement, sans réinitialiser le
lecteur, le tampon ou le cache. La navigation revient après fermeture du lecteur,
même après un redimensionnement. Les décors Noël/Halloween n'héritent plus de
l'espace de la barre latérale masquée et restent absents au-dessus du lecteur.

Les écarts trouvés pendant la revue ont été corrigés avant ce verdict : cibles
tactiles trop petites et actions de collections dépendantes du survol ; profil
mal placé ; verrouillage d'orientation déclenché sur bureau ; course d'un plein
écran accordé après fermeture ; seuil de densité calculé sur la largeur du
contenu plutôt que celle de l'écran ; réservation de 205px du décor événementiel ;
barre mobile restant masquée après certains cycles du lecteur.

Preuves indépendantes : `git diff --check` PASS ; 147/147 tests web PASS ;
18/18 tests mobiles ciblés PASS ; contrôles syntaxiques JavaScript et shell PASS ;
QA Chromium du build PASS à 320/390/430/736/844px et 1280px, navigation,
filtres, position de défilement, collections non vides, configuration, FR/EN,
thèmes, entrée/sortie du vrai plein écran et retour de navigation inclus.
L'opérateur statique inchangé (`b0baea0b…3996`) passe `bash -n` et `--self-test`.

Limites : aucun appareil iOS/Safari ou Android physique, flux vidéo réel,
verrouillage paysage réel ni Pi n'a été utilisé par cette revue. L'orientation
refusée est testée avec la politique navigateur simulée et son repli honnête.
Le verdict ne couvre pas encore l'archive de déploiement, qui doit être figée et
contrôlée par son SHA256 et son manifeste avant installation.

## English

**APPROVED_SCOPED** — no blocking or major finding remains in the responsive
web-client scope. The candidate leaves the Mac client and server outside the
diff, provides the five-destination mobile navigation and More sheet, touch
targets of at least 44px, a virtualized two-column grid at 320px and three
columns above it, working saga/playlist/settings routes, and French/English
labels. The wide layout restores the existing sidebar.

Playback requests actual fullscreen directly from the trusted user gesture,
does not claim success after browser refusal, and then attempts a landscape
lock. Closing cleans up a fullscreen grant that resolves late without resetting
the player, buffer, or cache. Mobile navigation returns after player close even
when playback included a resize. Christmas/Halloween decoration no longer
reserves the hidden desktop sidebar and remains disabled over playback.

Review-discovered defects were fixed before this verdict: undersized touch
targets and collection actions dependent on hover; misplaced profile; desktop
orientation lock; late fullscreen grant race; density threshold based on
content rather than viewport width; a stale 205px event-art reservation; and
mobile navigation remaining hidden after some player lifecycles.

Independent evidence: `git diff --check` PASS; 147/147 web tests PASS; 18/18
targeted mobile tests PASS; JavaScript and shell syntax checks PASS; built-client
Chromium QA PASS at 320/390/430/736/844px and 1280px, including navigation,
filters, retained scroll position, non-empty collections, settings, FR/EN,
themes, actual fullscreen entry/exit, and navigation restoration. The unchanged
static-assets operator (`b0baea0b…3996`) passes `bash -n` and `--self-test`.

Limits: this review used no physical iOS/Safari or Android device, real video
stream, real landscape lock, or Pi. Orientation refusal is covered with a
mocked browser policy and the honest fallback. This verdict does not yet cover
the deployment archive; its frozen SHA256 and manifest require separate
verification before installation.

## Annexe de livraison / Delivery appendix

La limitation ci-dessus sur l'archive est levée pour le paquet figé suivant :
commit `98ee3e37e8b19c0d2e8e92991b3fb917eb0a13b8`, archive
`chillcenter-web.tar`, SHA256
`281cf6eb6babca616004f425ac9e13332f38ef9a3af59f7793398640c81cc0ed`,
6 616 576octets, build `b1722f00c0a6`. **APPROVED_SCOPED** pour une
installation des seuls fichiers web avec l'opérateur
`b0baea0b2a572b65c9a79cd792276143b45092dc95523eb47815b8742c743996`.

L'archive contient exactement82fichiers réguliers et6répertoires, tous sous
`web/`, sans lien, nœud spécial, chemin absolu, remontée ou doublon. Les81lignes
de `SHA256SUMS` couvrent exactement tous les fichiers sauf le manifeste ; chaque
empreinte est valide. L'arborescence hors manifeste est identique au build web
revu. La taille reste sous la limite de16Mio de l'opérateur.

The archive limitation above is cleared for this exact frozen package: commit
`98ee3e37e8b19c0d2e8e92991b3fb917eb0a13b8`, `chillcenter-web.tar`, SHA256
`281cf6eb6babca616004f425ac9e13332f38ef9a3af59f7793398640c81cc0ed`,
6,616,576bytes, build `b1722f00c0a6`. It is **APPROVED_SCOPED** for a
web-assets-only install with operator
`b0baea0b2a572b65c9a79cd792276143b45092dc95523eb47815b8742c743996`.

The archive contains exactly82regular files and6directories, all below
`web/`, with no link, special node, absolute path, traversal, or duplicate.
The81-line `SHA256SUMS` manifest covers every file except itself and all hashes
validate. The non-manifest tree is byte-identical to the reviewed web build.
The archive remains below the operator's16MiB limit.
