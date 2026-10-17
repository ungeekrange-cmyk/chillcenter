# ADR-0020 — Audio AC-3 / E-AC-3 / DTS / TrueHD converti en AAC dans le navigateur (WebAssembly)

- Status : `PROPOSED` ; date : `2026-10-15`. Passe à `ACCEPTED` après revue indépendante des octets déployés et déploiement vérifié.
- Decision Owner / Author Instance : `/root` ; Reviewer Instance : à désigner (≠ auteur).
- Branche : `codex/web-pwa`. Supersedes / Superseded By : `NONE` / `NONE`. Complète ADR-0015 (moteur de lecture web) et ADR-0016 (même schéma que le décodeur MPEG-4). Contrat d'API inchangé (`2026-10-14`).
- Contrainte du projet respectée : **le serveur ne transcode rien** ; la conversion se fait sur l'appareil qui regarde.

## Contexte

Symptôme (15 octobre 2026) : pas de son sur plusieurs films (Baby Driver, Deadpool & Wolverine). Constat sur le Pi : ces MKV n'ont que des pistes **AC-3** (Baby Driver : deux) ou **E-AC-3** (Deadpool & Wolverine) ; le démuxeur les reconnaît sans pouvoir les déclarer à MSE (`mime = null`), donc « lecture sans son ». Échantillon réel de 12 MKV : 6 avec AC-3 et 1 avec E-AC-3. Le navigateur intégré à l'application (Chromium) répond `isTypeSupported = false` pour `ac-3`, `ec-3` et `dtsc` dans MSE ; Safari les lit, la plupart des Chrome non.

## Options

| Option | Pour | Contre |
| --- | --- | --- |
| A. Déclarer `ac-3` / `ec-3` à MSE | aucune conversion, aucun CPU | ne marche que là où le navigateur a le décodeur (Safari) ; comportement différent selon l'appareil |
| B. Transcoder côté serveur | simple pour le client | interdit par le cadrage du projet (aucun transcodage serveur), charge sur un Pi 3B+ |
| **C. Décoder en WebAssembly et ré-encoder en AAC-LC dans un Worker, juste avant l'emballage en fragments MP4** | identique partout, le moteur MSE ne voit qu'une piste AAC ordinaire (pistes, saut, vitesse, volume inchangés) | CPU client (quelques % d'un cœur), ≈ 1 Mo de WebAssembly, retard de décodage AC-3 de 5 ms conservé |
| D. Décoder en PCM et jouer par WebAudio | pas d'encodeur | synchronisation vidéo/audio à réinventer (vitesse, saut, pause) |

## Décision

Option **C**, sur le modèle du décodeur MPEG-4 (ADR-0016).

1. **Module** `web/vendor/audio/audio-transcoder.wasm` (967 Ko, ≈ 400 Ko compressé) : FFmpeg **9.0.2** (même source figée, SHA-256 `8c385028…002e`), décodeurs `ac3`, `eac3`, `dca`, `truehd`, `mlp`, encodeur `aac` natif, `libswresample` ; **LGPL**, aucune option `--enable-gpl` ni `--enable-nonfree`, compilé dans l'image `emscripten/emsdk` épinglée par digest, sans réseau. Avis : `web/vendor/audio/NOTICE-LGPL.md`.
2. **Piste déclarée comme AAC-LC stéréo** au même taux d'échantillonnage (`AudioSpecificConfig` calculée, 2 octets), étiquetée dans l'interface « AC-3 (converti en AAC) ». Pistes multiples, langues, sélection et changement à chaud : inchangés.
3. **Conversion dans le démuxeur** (`MkvSource.transcodeEntries`), par lots de trames, dans un **Worker** ; l'horodatage tient compte du retard de l'encodeur (`initial_padding` = 1024 échantillons : la trame de pré-roulement avant l'instant 0 est abandonnée) ; remise à zéro à chaque saut ; purge en fin de fichier ; trame endommagée sautée sans arrêt.
4. **Mixage** : stéréo, matrice standard de `libswresample` (centre et ambiances à −3 dB, basses exclues) plus **écrêteur doux** (genou de 0,9 à 1,0) pour que les pics sommés ne saturent pas ; 192 kb/s. Le mixage natif du décodeur AC-3 (normalisé) est plus bas de ≈ 6 dB (dialogues très faibles) : écarté.
5. **Chargement** : le `.wasm` n'est téléchargé que si une piste à convertir est lue.

## Validation (exécutée)

- 9 tests Node avec le vrai module : AC-3 5.1, E-AC-3 5.1, AC-3 stéréo 44,1 kHz, DTS stéréo décodés par un **décodeur indépendant** (FFmpeg système, outil de test seulement) puis comparés au décodage du fichier d'origine : corrélation > 0,97, alignement temporel ≤ 64 échantillons (AC-3/E-AC-3 : décalage de 256 échantillons = 5,3 ms, retard de transformée propre au format que la référence retire), niveau à ±2 dB, aucun échantillon hors pleine échelle ; première trame à 0 ; paquet corrompu ; transcodeur absent = erreur explicite (jamais un silence muet) ; `AudioSpecificConfig` identique à celle de l'encodeur.
- **Navigateur réel** (HTTP clair, serveur réel, film MKV à deux pistes AC-3) : démarrage en 1,3 s, son mesuré à la sortie de l'élément vidéo (550 Hz piste anglaise), changement vers la piste française (328 Hz), saut à 20 s : son maintenu, aucune erreur.

## Limites connues

- MP4 / AVI contenant de l'AC-3 ne sont pas encore convertis (seuls les MKV, cas des films concernés) ; ils restent « sans son » avec un message explicite.
- Pas de sortie 5.1 : stéréo seulement (un mixage 5.1 vers un amplificateur nécessiterait une option par appareil).
- Pas de mesure de charge sur téléphone ni de DTS-HD MA / TrueHD réels (fichiers de test stéréo / 5.1 AC-3 seulement).
- Retard de 5,3 ms propre à l'AC-3 conservé (très en dessous du seuil de perception, ≈ 40 ms).
