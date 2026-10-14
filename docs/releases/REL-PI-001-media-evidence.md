# Pi / NAS — Preuves médias réelles

14 octobre 2026. Source serveur `126da0a`, binaire ARM64 `9a17ffe…36a6`, composition déployée R7 + overlay WorkingDirectory ; détails et hashes complets dans [REL-PI-001](REL-PI-001.md). Essais terminés, limites et échecs conservés ; les résultats headless ne prouvent pas les parcours humains.

## Bibliothèque et transport

[Vérificationfinaleindépendante](REL-PI-001-final-media-verification.md) : runtimeAPPROVED_SCOPED, charge3FAILED, exact2DuneQUALIFIED_PASSavecbuffering ; profilsQA retirés/API[], SQLmedia2496, NASRO/serviceactiveconfirmés sansécriture parlevérificateur. Ce rapport ne transforme pas les contrôlesheadless en testsgraphiques/humains.

Le scan récursif a réellement trouvé **2 496 vidéos en 10,14 s** : 1 944 MKV, 369 AVI et 183 MP4. Le compte serveur lit le CIFS `//192.168.1.92/Video` sur `/srv/volumes/video`, groupe984 explicitement approuvé, montage inchangé en lecture seule. Le Mac ne monte ni n'ouvre le NAS : vidéo, sous-titres et catalogue passent par HTTPS serveur. Aucun import de métadonnées Cinéfolio ni appel payant n'a été lancé.

Trois profils `QA Pi 9be1667e {1,2,3}` ont été créés pour ces tests, sans profil personnel ; **supprimés à12:52UTC aprèsfinstouteslessondes**, UUID+nomsexacts vérifiés etconfirmationAPIexigée, aucunprofilhorscetterun touché. Relecturefinale0profils, bibliothèqueetvidéosconservées ; pas demigrationCinéfolioeffectuée. TLS du harnais Rust est épinglé au SPKI public obtenu via la session SSH strictement vérifiée ; le diagnostic FFprobe client utilise la même feuille publique comme CA, `tls_verify=1`, `verifyhost=192.168.1.42`, aucun `-k`. Les capabilities de bail, noms, chemins et textes de sous-titres ne sont pas journalisés. FFprobe fonctionne **sur le Mac uniquement**, pas sur le Pi. [Options TLS primaires FFmpeg](https://ffmpeg.org/ffmpeg-protocols.html#tls).

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

## Charge longue initiale — COMPLETE, deux flux

Deux clients headless réels MKV1920×808 et MP41920×1024, profilsQA2/QA3, démarrés autour de11:28:39UTC pour1830secondes chacun, afin d'obtenir au moins30min de recouvrement malgré les secondes de chargement. Puis trois lecteurs avec le MKV720p sont prévus. Chaque flux rapporte sa progression et buffering ; la sonde Pi mesure toutes30s les jiffies de processus, RSS, mémoire système disponible, swap, température et octetswlan0. Le CPU est en % d'un cœur et par intervalle, pas le pcpu moyen depuis démarrage. Le débit interfaceinclut CIFS et autres services ; le débit moyen encodé est séparé.

Fin réelle : les deux sondes ont rendu PLAYBACK_PASS et exit0 après1830,098/1830,089 s. MKV : position1850,13 s depuis45 s,219samplesbuffer sur17114 ; MP4 :1874,498 s depuis45 s,0sur17147. Le compteur100ms estime les occurrences de buffering, pas la durée exacte ni leur cause. Le MKV a une progression inférieure au temps mural d'environ25 s : **ne pas annoncer zéro interruption** malgré le critère de progression>80% et la reprise/grant révoqué PASS. Ces deux échantillons ne sont pas Dune.

Sonde ressources :61échantillons/1800,07 s ; PID3016 inchangé, RSSmax23,73 Mio, disponiblemin613,45 Mio, swapmax0,tempmax47,774°C. CPUmoyen9,36% d'un cœur, max17,11%, débitswlan0maxRX26,039/TX10,885Mbit/s. La dernière fenêtre inclut la fin des lecteurs et ne prouve pas17,11% en lecture simultanée continue. Ces débits concernent l'interface entière, pas uniquement Chill Center.

Demande additionnelle : essai Dune2021/Dune2024 réellement présents sur le NAS et cible de tampon60 s avant/environ30 s arrière. [ADR-0010](../architecture/decisions/ADR-0010-client-playback-cache.md). Dune1MP4id9cc99991-1e29-4b9e-b5d2-e1c381448ac2/H2641920×768/AAC, un sous-titre externe ; Dune2MKViddbecd05a-8407-4878-b020-c22e212bd108/H2641920×800/EAC3,11sous-titresSRT intégrés. Dimensions/pistes vérifiées par FFprobe **Mac uniquement**, TLSCA+hostname strict, baux libérés. Le troisième échantillon1280×720 possède une pisteASS.

## Trois flux avec les deux Dune — FAIL pour la capacité globale

Source lecteur fixe27bd5b24b101ac7028bbd3ad8a2f81159bd40e03, executableharnaisSHA2564c42475a0b16c8a5c51429c033e287217e73c1399a3a4fb83ba8f2669654fcdd. Cacheyes/60s/plafonds128/64Mio/no donation, pas simples réglagescacheauto. Trois processusMac réellement simultanés, profilsQA2/3/1, débutvers12:07:30UTC,1830s chacun ; mesuresPi12:08:xx–12:38:xxUTC. DécodageVideoToolbox-copy sur les trois, sortiesheadless ; choix des11SRT deDune2 et de l'ASS720p accepté, pas preuve d'écoute/rendu.

| Média | Durée / débit moyen du fichier | Résultat réel |
| --- | --- | --- |
| Dune2021MP4 | 8936,16s /2,639Mbit/s /2947857762octets | exit0/PLAYBACK_PASS,1830,153s,position1872,40depuis45s,0buffer sur16821samples ; avanceindicative≈60s |
| Dune2024MKV | 10016,215s /6,836Mbit/s /8558400039octets | **exit1**, progression finale<80%du temps réel. Derniercheckpoint1814s :position1412,995s,4102samplesbuffer sur16757, avanceindicative0s. PasDECODE_COMPLETE/PASS/reprisenouvelUUID à inventer ; bail relâché sur erreur par le harnais |
| MKV720p1280×720 | 4612,352s /4,070Mbit/s /2346485058octets | exit0/PLAYBACK_PASS,1830,084s,position1819,067depuis45s,323buffer sur16892samples surtout début ; avanceindicative≈60s une foisremplie |

Les seuils de progression ne prouvent pas l'absence d'interruption : le720p PASS headless a quand même des stalls. Dune2 a subi plusieurs minutes d'interruption. La cible troisflux1080p/720p **n'est pas validée sur ce Wi-Fi/NAS** ; aucune cause unique démontrée, pas de correction spéculative ou transcodage ajouté. Le Mac ne monte pas le NAS.

SondePi61samples/1800,06s : PID3016/NRestarts0, RSSmax24,23Mio, CPUmoyen20,50% d'un cœur/max22,95%, disponiblemin608,95Mio,swapmax0,tempmax49,388°C,RXwlanmax33,248/TX15,864Mbit/s. La mémoire/CPU observés ne sont pas saturés, mais cela ne prouve pas la cause du buffering. Dernière fenêtre comprend la fin des lecteurs. MesureMac ponctuelle à~12:20UTC :RSS63,31/60,44/55,63Mio pourDune1/Dune2/720p, **pas des pics**, sanswebview/son/rendu. Budgetmax768Mio du client configurable non essayé.

Événements expurgés de capabilities : [Dune1](evidence/2026-10-14-three-dune1.log), [Dune2](evidence/2026-10-14-three-dune2.log), [720p](evidence/2026-10-14-three-third720.log), [ressources](evidence/2026-10-14-three-dune_metrics.log). Le compteur des samples≈100ms n'est pas une mesure précise de durée d'interruption. `demuxer-cache-duration` indicatifpeufiable, pas garantie60s ni preuve30sretoursansrefetch.

## Exactement deux Dune — COMPLETE,10min supplémentaires

Après fins des troislecteurs et de leursonde, deuxnouveauxprocessusclient source624f2447b5c0ede6b4a053ea3fa7b82db85c0121, harnaisSHA256ca566b9b7c89f5561c7e00ca4eea1757109d56c1fc5d4b5b70e0b9fa6a0cbb80, défaut128/64Mioet60s, profilsQA2/3. **Les deux ont fini PLAYBACK_PASS/exit0**, repriseparnouvelUUID etrefusgrantlibéréPASS. Durée600s chacun, seeks45s, métriquesPi600s. Le lecteur standalone emploie les défauts, **pas le formulaire graphique ou des valeurs personnalisées**. Cet essai10min ne devient pas30min ni une comparaison contrôlée avec l'anciencache20s.

- Dune1 :600,075s,position643,84sdepuis45s,0samplesbuffer/5514, avanceindicative≈60s.
- Dune2 :600,166s,position623,248sdepuis45s,184samplesbuffer/5536 surtoutdébut ; déficitdeprogression≈21,75s parrapportà600s, pasduréeexacted'interruption. Dernièreavanceindicative8,592s, jamaisannoncer60sgarantiesousceflux mêmeavec128Mio. La lecture se stabilise ensuite maiscet essain'est pas « zérointerruption ».
- Pi21mesures/600,06s : RSSmax24,36Mio,CPUmoyen16,16% d'un cœur/max17,43%,disponiblemin610,68Mio,swap0,tempmax49,388°C,RX26,347/TX12,561Mbit/s ; PID3016inchangé. Valeurswlan0globales, moyenneencodée/picsfichierdistincts, dernièresfenêtresincluentfin.

Logs sanscapabilities : [Dune1](evidence/2026-10-14-pair_dune1.log), [Dune2](evidence/2026-10-14-pair_dune2.log), [ressources](evidence/2026-10-14-pair_metrics.log). Pasdesmomentsd'usagehumain ni30minparfichier ; pasdetestdefaibleWi-Fi artificielouofflineavecbailexpiré.

Restes explicites : **capacitéfluide3flux nonvalidée/àdiagnostiquer**, repos avec3000fiches (dataset réel2496), RSS webview/son/rendu pleinécran/veille, HEVC réel/multiplesaudio/bilinguisme/enchaînementNAS, reboot/panneNAS/restore physique et rotation annuelle réelle. Les fixtures logicielles précédentes ne deviennent pas des preuves physiques. Aucun reboot/démontageNAS/coupure ou suppression des vidéos n'est autorisé implicitement par ce test. RetesteravecEthernet seraituneexpériencepertinente, pasuneaméliorationdéjàprouvée ouunchangementréseauautoriséimplicitement.
