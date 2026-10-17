# ADR0028 — MP3 des AVI adapté côté navigateur

**Français** · [English](ADR-0028-web-mp3-fallback.en.md)

2026-10-17. Statut : ACCEPTED_BY_USER_CHANGE_REQUEST ; implémentation et preuves
dans la livraison R3. Complète [ADR0020 audio web](ADR-0020-web-audio-transcode-to-aac.md)
et [ADR0027](ADR-0027-mobile-content-and-iphone-playback.md), sans modifier leurs
preuves historiques. Auteur : `/root` ; revue indépendante distincte.

## Décision

Pour les AVI MP3, conserver le chemin natif quand MediaSource/ManagedMediaSource
accepte le MP3 dans MP4 ou `audio/mpeg`. Sinon, convertir uniquement l'audio en
AAC-LC stéréo dans le Worker FFmpeg WebAssembly existant. Le décodage Xvid reste
sur le client. Ni accès direct au NAS, ni transcodage serveur, ni changement
d'API, du client Mac ou de dépendance JavaScript.

Le module FFmpeg 9.0.2 est reconstruit avec le décodeur MP3, même source et image
Emscripten épinglées ; compilation sans réseau, sans GPL/nonfree, provenance et
licence dans `web/vendor/audio/`. Chargement à la demande et limite mémoire
existante conservés. La conversion consomme du CPU sur le téléphone ; ce lot
ne prétend pas mesurer sa batterie ou garantir tous les AVI possibles.

## Présentation et limites de plateforme

Le bouton plein écran web mobile appelle le navigateur dans le geste tactile,
avant toute file de commandes réseau. Utiliser le plein écran du panneau si
disponible, sinon celui de l'élément vidéo iPhone. Ne déclarer le plein écran
actif qu'à partir de son état réel. Un canvas Xvid sans API plein écran du
panneau ne peut pas utiliser l'API réservée aux vidéos : afficher cette limite
et conserver la vidéo dans toute la zone disponible, sans écran noir natif.

Références : [Apple, vidéo Safari](https://developer.apple.com/documentation/webkit/delivering-video-content-for-safari),
[WebKit, plein écran non vidéo iPhone](https://bugs.webkit.org/show_bug.cgi?id=240312).

## Validation et retour arrière

Exiger le vrai WASM, des AVI MP3 synthétiques, une comparaison audio indépendante,
des tests de cycle de vie et de lecture navigateur avec refus MP3 simulé.
Ces tests ne remplacent pas un essai physique iPhone/Safari. Revenir à l'archive
web précédente n'exige aucune migration de données ni changement serveur.
