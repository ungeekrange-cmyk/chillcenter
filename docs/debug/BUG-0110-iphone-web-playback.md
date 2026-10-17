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
