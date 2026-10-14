# Pi / NAS — Preuves médias réelles

14 octobre 2026. Source serveur `126da0a`, binaire ARM64 `9a17ffe…36a6`, composition déployée R7 + overlay WorkingDirectory ; détails et hashes complets dans [REL-PI-001](REL-PI-001.md). Cette fiche distingue les succès exécutés des mesures encore en cours.

## Bibliothèque et transport

Le scan récursif a réellement trouvé **2 496 vidéos en 10,14 s** : 1 944 MKV, 369 AVI et 183 MP4. Le compte serveur lit le CIFS `//192.168.1.92/Video` sur `/srv/volumes/video`, groupe984 explicitement approuvé, montage inchangé en lecture seule. Le Mac ne monte ni n'ouvre le NAS : vidéo, sous-titres et catalogue passent par HTTPS serveur. Aucun import de métadonnées Cinéfolio ni appel payant n'a été lancé.

Trois profils `QA Pi 9be1667e {1,2,3}` sont créés pour ces tests, sans profil personnel. TLS du harnais Rust est épinglé au SPKI public obtenu via la session SSH strictement vérifiée ; le diagnostic FFprobe client utilise la même feuille publique comme CA, `tls_verify=1`, `verifyhost=192.168.1.42`, aucun `-k`. Les capabilities de bail, noms, chemins et textes de sous-titres ne sont pas journalisés. FFprobe fonctionne **sur le Mac uniquement**, pas sur le Pi. [Options TLS primaires FFmpeg](https://ffmpeg.org/ffmpeg-protocols.html#tls).

La première sonde prepare a retourné404 après le scan : erreur de son appel qui plaçait ?query dans le path, pas du catalogue. Le GET correctement construit a retourné100éléments/has_moretrue. [BUG0044](../debug/BUG-0044-pi-probe-catalog-query.md) est corrigé dans le harnais par options de query séparées, testunitaire/buildrelease/clippyPASS, SHA sourceeee307ad673b5df79b2a9cf91699913a328b02bef30edd9a2c108a9dc1b1696d. Le diagnostic FFprobe initial a aussi omis les en-têtes de bail (400 normal, fail-closed) ; corrigé avec X-Chill-Session/X-Chill-Generation exigés, aucune politique serveur affaiblie.

## Premiers décodages, 10 s chacun — PASS

| Échantillon opaque | Format / codec réel | Durée | Débit moyen calculé | Observation |
| --- | --- | --- | --- | --- |
| 0e6bfa18… | AVI, MPEG-4 part2 | 4545,44 s | 1,296 Mb/s | Décodage logiciel, 1audio, 0subtitle ; chargé en4,33s, 0buffer sur101samples |
| aefaf454… | MKV, H.264, 1920×808 | 5095,089 s | 3,049 Mb/s | VideoToolbox-copy, 1audioAC3/fre ; chargé en0,43s, 0buffer sur100samples |
| a999ab9b… | MP4, H.264, 1920×1024 | 6127,104 s | 2,638 Mb/s | 1audioAAC/und ; chargé en3,30s, 0buffer sur99samples, hw non confirmé àload |

Les trois parcours réels ont accepté les sept vitesses, seek45s, heartbeat/progression, reprise serveur identique avec un second identifiant d'appareil, et refus du bail libéré. Position finale AVI54,24s/MKV54,713s/MP454,637917s ; `PLAYBACK_PASS` et exit0 pour chaque processus. Le second appareil est simulé par un nouvel UUID sur le même Mac, pas deux Mac physiques. Ces essais headless (`vo=null/ao=null`) prouvent transport et décodage, pas l'écoute humaine ou l'image AppKit réelle de ces films.

Échantillon supplémentaire identifié pour la troisième lecture : `6bc36d26…`, MKV H.264 **1280×720**, audioAC3/fre et sous-titreASS/fre intégré. Les dimensions ci-dessus sont obtenues par FFprobe client sur les flux réels, pas inférées des seuls noms de fichiers.

## Sous-titres — preuves distinctes

Le MP4a999ab9b a un sous-titre externe trouvé via API : téléchargementHTTPS strictPASS,117514octets UTF-8 valides, SHA256dea0f3819c8a2a4da0980bd811fb02776a82fea4a3986473d00d31aa2d165e1c ; FFprobe sur ces octets en stdin identifie `subrip/subtitle`. Aucun texte affiché. L'ASS intégré du MKV720p est identifié dans le flux ; sa sélection native sera observée lors de la troisième lecture. Le rendu, le timing et le changement de langues dans l'interface réelle restent à valider, sans les confondre avec ces tests de transport/parsing.

## Charge longue — IN_PROGRESS

Deux clients headless réels MKV1920×808 et MP41920×1024, profilsQA2/QA3, démarrés autour de11:28:39UTC pour1830secondes chacun, afin d'obtenir au moins30min de recouvrement malgré les secondes de chargement. Puis trois lecteurs avec le MKV720p sont prévus. Chaque flux rapporte sa progression et buffering ; la sonde Pi mesure toutes30s les jiffies de processus, RSS, mémoire système disponible, swap, température et octetswlan0. Le CPU est en % d'un cœur et par intervalle, pas le pcpu moyen depuis démarrage. Le débit interfaceinclut CIFS et autres services ; le débit moyen encodé est séparé.

Premiers10min : PID3016 stable, RSS≈23,4–23,7MiB, CPU≈6–12% d'un cœur dans les premières fenêtres, disponible≈616–621MiB, swap0,temp≈47°C. MKV23samplesbuffer sur5549, MP40sur5539 ; ce n'est pas zéro buffering garanti ni une endurance déjà terminée. Compter des samples100ms est une estimation du temps de buffering, pas une mesure précise de chaque interruption. Aucun PASS30min/3flux ne sera enregistré avant les fins réelles et les summaries.

Restes explicites : mesure3flux, repos avec3000fiches (dataset réel2496), RSS webview/son/rendu pleinécran/veille, HEVC réel/multiplesaudio/bilinguisme/enchaînementNAS, reboot/panneNAS/restore physique et rotation annuelle réelle. Les fixtures logicielles précédentes ne deviennent pas des preuves physiques. Aucun reboot/démontageNAS/coupure ou suppression des vidéos n'est autorisé implicitement par ce test.
