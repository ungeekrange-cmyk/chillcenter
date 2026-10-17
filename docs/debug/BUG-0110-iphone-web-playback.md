# BUG0110 — Lecteur web iPhone : moteur absent et démarrage silencieux

2026-10-17, utilisateur : iPhone, Safari et Vivaldi, HTTP et HTTPS ; échec sur
un AVI et un MKV. Version iOS et codecs exacts des fichiers non relevés.
Statut : FIXED_IMPLEMENTATION / VERIFIED_SYNTHETIC / DEVICE_RETEST_PENDING.

Mécanismes reproduits : le registre ne sélectionnait que `MediaSource`, pas
`ManagedMediaSource` seul ; `play()` ignorait tous les rejets. La guidance
de rotation apparaissait aussi lorsque le téléphone était déjà en paysage.

Correction : repli ManagedMediaSource avec lecture distante désactivée avant
attachement ; chemin MediaSource historique conservé. Les refus de démarrage
et interruptions courantes proposent un geste tactile sur le même élément
tamponné. Pauses, fermetures et tentatives anciennes invalident les rejets
tardifs. Une pause serveur ou un bail invalide bloque aussi cette reprise,
avant même la fin de l'ACK réseau. Aucun transcodage serveur ni changement API.

Preuves : tests unitaires moteur/reprise, ACK différé et bail invalide ;
`scripts/test-web-mobile-playback.cjs` décode et fait avancer de vrais clips
synthétiques MKV H.264/AAC et AVI Xvid/MP3 dans Chromium. Le constructeur MSE
y est exposé sous le nom ManagedMediaSource pour tester le câblage : ce n'est
pas une exécution de l'API Apple native. Le refus initial est volontairement
simulé ; la reprise ne réacquiert pas le bail ni ne recrée l'élément vidéo.

Limites ouvertes : retester les deux vidéos sur l'iPhone. Leur conteneur seul
ne prouve pas la compatibilité des codecs. Si Safari refuse MP3 via MSE/MMS,
l'AVI Xvid/MP3 peut encore échouer : le transcodeur audio existant ne convertit
pas MP3 en AAC. Aucune prise en charge de ce cas n'est prétendue ici.

Référence primaire : [WebKit — Safari 17.1 et Managed Media Source](https://webkit.org/blog/14735/webkit-features-in-safari-17-1/).

## Complément R3 — 17 octobre 2026

Demande explicite de résoudre aussi le MP3 des AVI, sans transcodage serveur.
Le Worker FFmpeg existant comprend maintenant le décodeur MP3. Si MSE/MMS
refuse le MP3 dans MP4 et `audio/mpeg`, le démuxeur AVI fournit de l'AAC-LC
stéréo converti localement ; sinon le chemin MP3 natif reste inchangé.
Le WASM reconstruit mesure 1 011 837 octets (+44 821), source FFmpeg et image
Emscripten épinglées inchangées. Pas de dépendance JavaScript supplémentaire.

Une revue a détecté avant livraison un index de paquets AAC erroné, puis des
gardes de génération et une purge de trame partielle manquantes au seek : ces
défauts du candidat sont corrigés et couverts. Les fragments AVI sont réassemblés
en trames MP3 complètes, les réponses tardives après fermeture/seek sont rejetées.
Le vrai WASM produit de l'AAC comparé au décodage FFmpeg indépendant (corrélation
>0,97, durée complète à la marge de padding près). Le navigateur lit un AVI
Xvid/MP3 synthétique même quand les capacités MP3 natives sont désactivées.

Cette réserve MP3 du checkpoint R2 est donc levée au niveau implémentation/test
synthétique. **DEVICE_RETEST_PENDING** reste valable pour Safari physique ;
aucune mesure de CPU/batterie iPhone ni validation des deux fichiers NAS réels.
Voir [ADR0028](../architecture/decisions/ADR-0028-web-mp3-fallback.md) et
[BUG0111](BUG-0111-mobile-posters-fullscreen-search.md) pour le plein écran.
