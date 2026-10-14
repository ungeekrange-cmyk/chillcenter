# ADR-0014 — Évaluation d'un décodeur MPEG-4 ASP (Xvid) en WebAssembly pour l'AVI web

- Status : `PROPOSED` — direction **B** demandée par l'utilisateur (voir l'historique) ; **aucun décodeur adopté, rien téléchargé ni installé**. Date : `2026-10-14`.
- Decision Owner / Author Instance : `/root` ; Reviewer Instance : à désigner (≠ auteur).
- Branche : `codex/web-pwa`. Supersedes / Superseded By : `NONE` / `NONE`. Complète ADR-0013 (qui réserve le WASM à un ADR dédié).
- Related Items : ADR-0012/0013, [preuve Pi](../../implementation/WEB-PI-READONLY-EVIDENCE.md), F-22.

## Historique de la décision utilisateur (14 octobre 2026)

1. Choix de l'**option A** (« message explicite + renvoi vers le client Mac, spike plus tard si besoin »).
2. **Aussitôt remplacé** par : « force le max sans client mac. Le but est même un client léger multi plateforme ». Le client web ne doit donc **pas** dépendre du client Mac, y compris pour l'AVI : l'option A, dont le repli est le client Mac, n'est **plus retenue** ; la direction est l'**option B** (décodeur MPEG-4 ASP en WASM + second moteur de lecture), à réaliser par étapes.
3. La condition « rien n'est téléchargé avant accord » reste en vigueur : les parties réalisables sans téléchargement (démuxeur AVI en JS pur, moteur canvas, interface de décodeur, tests avec décodeur factice) sont faites en premier ; la compilation du décodeur réel nécessite l'outillage listé en bas de ce document et un accord explicite.

## Contexte

Essai en lecture seule sur le Pi (14 octobre 2026) : le catalogue compte 369 AVI sur 2 496 titres (**15 %**). Les 5 AVI échantillonnés sont en **Xvid (MPEG-4 Part 2, « ASP »)**. Aucun navigateur ne le décode dans `<video>`/MediaSource, et rien ne laisse penser que WebCodecs le fasse (la recherche n'a trouvé aucune source officielle l'attestant ; à vérifier au runtime par `VideoDecoder.isConfigSupported()`). Un relevé des en-têtes des 369 AVI (lecture seule, voir ci-dessous) a ensuite précisé résolution et audio. Contrainte utilisateur : technologies **stables et légères**.

## Ce qu'il faudrait réellement (trois pièces, pas une)

1. **Démuxeur AVI (RIFF)** : format simple (`hdrl`/`movi`/`idx1`) ; écriture en **JS pur dans le dépôt**, sans dépendance, comme MP4/MKV. Coût estimé : ~250–400 lignes + tests (estimation, pas une mesure).
2. **Décodeur MPEG-4 ASP** : seul morceau qui nécessite du WebAssembly. Options étudiées ci-dessous.
3. **Chemin de rendu différent** : un décodeur logiciel produit des images brutes, qu'on ne peut pas donner à MediaSource. Il faut un second moteur : décodage dans un Worker, dessin sur `<canvas>` (2D ou WebGL), audio par WebAudio ou par MSE audio seul, **horloge A/V, pause, vitesse, saut, plein écran et sous-titres refaits** par ce moteur. C'est la vraie charge de travail, supérieure à la taille du décodeur. Alternative écartée : décoder en WASM puis réencoder en H.264 avec WebCodecs pour réutiliser MSE (perte de qualité, CPU/GPU doublés, latence).

## Options pour le décodeur

| Option | Taille (état des sources) | Licence | Stabilité / maintenance | Verdict |
| --- | --- | --- | --- | --- |
| **FFmpeg `mpeg4` natif via une variante *libav.js* personnalisée** | Builds « assez complets » de libav.js : ~1,5 à 3 Mio ; un build vide ~178 Kio ; glue JS jusqu'à ~408 Kio, très compressible. Une variante réduite (décodeur `mpeg4` + démuxeur facultatif) devrait tenir bien en dessous des builds complets : **estimation, non mesurée** | FFmpeg : LGPL v2.1+ tant que `--enable-gpl` n'est pas utilisé ; libav.js lui-même : BSD 0-clause. Obligation : fournir les sources exactes et la ligne `configure` avec le binaire | Les variantes publiées de libav.js n'incluent **ni décodeur MPEG-4 Part 2 ni démuxeur AVI** (d'après la page du projet) : il faut **compiler une variante** (`make build-<variante>`, outillage Emscripten) ; une fois le `.wasm` compilé, on le **fige dans le dépôt** (pas de mise à jour suivie) | **Seule voie réaliste**, mais avec un coût de build et de conformité LGPL |
| *libxvidcore* (décodeur Xvid d'origine) compilé en WASM | Petit | **GPL v2** : contaminerait l'application complète | — | **Écartée** |
| *ffmpeg.wasm* (`@ffmpeg/core`) | ~25–31 Mio (sources non officielles) | MIT pour le wrapper ; le cœur suit les licences de FFmpeg et des bibliothèques liées | Pensé pour la conversion de fichiers, pas la lecture en flux ; multi-thread = isolation inter-origine (COOP/COEP) | **Écartée** : lourd, hors critère « léger » |
| Décodeur MPEG-4 ASP écrit à la main en JS/Rust→WASM | Très petit | Propre au projet | Plusieurs milliers de lignes (VLC, prédiction DC/AC, compensation de mouvement quart de pixel, GMC, B-frames) ; risque élevé de défauts | **Écartée** |

## Performance

Mesure de référence **native**, sur ce Mac (Apple Silicon), FFmpeg `mpeg4`, **1 thread**, contenu synthétique `testsrc2` à 1,5 Mb/s, 24 i/s, 30 s : 640×272 → ×319 temps réel, 720×400 → ×203, 1280×720 → ×109. Ce n'est **pas** une mesure WASM ni sur un appareil de lecture réel ; un film réel coûte davantage qu'une mire.

Hypothèse de travail (non mesurée) : WASM sans threads ralentit un décodage déjà optimisé en assembleur d'un facteur de l'ordre de 3 à 10. À 720×400 cela laisse une marge confortable sur un Mac ou un PC récents ; elle se réduit sur tablette/téléphone, et le **rendu des images** (conversion YUV→RGB, dessin) s'ajoute. Les ordres de grandeur trouvés pour ffmpeg.wasm (« 10 à 100× plus lent » dans un guide non officiel, sans méthode) montrent qu'on ne peut pas conclure sans mesure sur les appareils cibles.

Contraintes WASM : mémoire plafonnée à 1 Gio ; threads possibles seulement avec SharedArrayBuffer et en-têtes d'isolation (COOP/COEP) qui compliquent le service depuis le Pi : à éviter, donc **un seul thread** et un décodage dans un Worker.

## Relevé des 369 AVI (Pi, lecture seule, 14 octobre 2026)

En-tête RIFF uniquement (64 Kio par fichier), un bail volatil acquis puis libéré par fichier, aucune progression, aucun titre relevé. **368 fichiers lus sur 369** (1 non lisible par l'analyseur : à examiner).

| Mesure | Résultat |
| --- | --- |
| Codec vidéo | **Xvid 354**, **DX50 (DivX 5) 13** — soit 367 en MPEG-4 ASP ; 1 en H.264 |
| Largeur d'image | < 600 px : 183 ; ≈ 640 px : 168 ; ≈ 720 px : 15 ; 1 en 720p ; 1 vers 960 px — **99 % en définition standard (≤ ≈ 720 px de large)** |
| Audio (première piste) | **MP3 stéréo : 364** ; AC-3 : 4 (dont 3 en 5.1) |
| Pistes audio | 1 seule piste audio dans 368 fichiers |

Conséquences :

- **Performance** : le contenu est en SD, donc nettement moins coûteux que les cas 720p/1080p de la mesure de référence (natif : ×200 à ×300 temps réel à 640–720 px). Même avec un facteur de ralentissement WASM pessimiste de 10×, la marge théorique reste de l'ordre de ×20 sur un Mac ; **non mesuré en WASM ni sur tablette**. Le risque de fluidité est faible pour ce catalogue.
- **Audio** : le MP3 est décodé par tous les navigateurs ; aucun décodeur audio WASM n'est nécessaire pour 364 fichiers sur 368. Les 4 AC-3 retombent dans la limite générale déjà acceptée (dépend du navigateur).
- **Un seul décodeur** (MPEG-4 ASP, tout le catalogue AVI sauf 1 fichier) suffit : le périmètre du WASM est étroit et bien défini.

## Options de décision (pour l'utilisateur)

| Option | Coût | Effet |
| --- | --- | --- |
| **A. Ne pas lire l'AVI dans le client web** : message explicite, renvoi vers le client Mac (libmpv) | ≈ 0 ; déjà le comportement actuel | 15 % du catalogue non lisible en web ; aucun WASM, rien à maintenir |
| **B. Spike borné** : variante libav.js `mpeg4` figée dans le dépôt + démuxeur AVI JS + moteur canvas/WebAudio | Élevé (second moteur de lecture, build Emscripten, conformité LGPL, tests sur appareils) | Couvre les Xvid ; reste la question audio (MP3 ok, AC-3 dépend du navigateur) |
| C. Conversion hors projet des AVI (en H.264/MKV) | Hors périmètre : le NAS est en lecture seule et l'import ne modifie aucune vidéo | Non retenue |

## Recommandation (mise à jour après le relevé)

Le relevé améliore le dossier de B : **99 % des AVI sont en SD et 99 % ont de l'audio MP3**, donc un seul décodeur vidéo étroit (MPEG-4 ASP) et aucun décodeur audio. La charge CPU n'est plus le risque principal. Le coût qui reste est **structurel** : un second moteur de lecture (canvas, horloge A/V, WebAudio, sous-titres) et la compilation figée d'une variante FFmpeg (outillage Emscripten, sources LGPL jointes).

- **Si les AVI comptent pour l'usage (15 % du catalogue)** : B devient défendable comme **spike borné**, limité aux AVI ≤ 720 px de large, avec critères d'arrêt chiffrés (images/s soutenues sur Mac et sur l'appareil le plus faible visé, mémoire, taille du bundle < 1 Mio, aucune dépendance suivie : `.wasm` figé dans le dépôt avec ses sources et sa ligne `configure`).
- **Sinon** : A (message explicite + client Mac) reste valable et gratuit.

Décision utilisateur requise. Rien n'est téléchargé ni installé tant qu'elle n'est pas prise.

## Réalisé sans aucun téléchargement (14 octobre 2026, branche `codex/web-pwa`)

| Pièce | État | Vérification |
| --- | --- | --- |
| Démuxeur AVI en JS pur (`web/src/avi.js`) : `idx1`, OpenDML (non testé faute de fichier > 1 Gio), temps vidéo par position d'image, audio CBR/VBR, MP3 | Fait | 3 tests ffmpeg + **8 AVI réels du Pi ouverts sur 8** (Xvid 432×336 à 640×480, 23,98 et 25 i/s, dts monotones, saut sur keyframe) |
| Chemin vidéo logiciel (`web/src/soft-video.js`) : échantillons bruts → décodeur enfichable → canvas, cadencé par l'horloge audio, file bornée, saut, reset, vidage final, conversion I420→RGBA | Fait | 7 tests avec décodeur factice ; **dans le navigateur** avec un vrai AVI + MP3 : écart image/son ≤ 30 ms en lecture, après saut (même hors tampon), en pause et à ×2 ; fin de vidéo détectée |
| Audio MP3 en flux brut `audio/mpeg` quand `audio/mp4` ne l'accepte pas (constaté sur ce Chromium) + **resynchronisation sur frontière de trame** (des AVI du catalogue découpent les trames entre chunks : première synchro à l'octet 84 sur un fichier réel) | Fait | 2 tests MP3 ; navigateur OK |
| Interface de décodeur `ChillSoft.decoders.register({match, create})` ; message clair tant qu'aucun décodeur n'est installé | Fait | Test : « Le décodeur logiciel Xvid (MPEG-4) n'est pas installé… » |

**Manque encore : le décodeur MPEG-4 ASP lui-même** (l'unique pièce WebAssembly). Tout le reste du chemin AVI est prêt et éprouvé avec un décodeur factice.

## Téléchargements nécessaires pour le décodeur réel — soumis à ton accord, rien n'est téléchargé

Choix technique, cohérent avec « léger et stable » : **pas de libav.js ni ffmpeg.wasm**. On compile un module minimal à partir des sources de FFmpeg, avec notre propre enveloppe C d'une centaine de lignes (ouverture du décodeur `mpeg4`, envoi d'un paquet, récupération d'une image I420), puis on **fige le `.wasm` dans le dépôt** (aucune dépendance suivie à l'exécution).

| Élément à télécharger | Source | Taille (estimation à confirmer avant téléchargement) | Raison |
| --- | --- | --- | --- |
| Archive des sources de FFmpeg, une version précise (par ex. une 7.x stable) | ffmpeg.org (somme SHA-256 publiée à vérifier) | ≈ 10–12 Mio compressée | Seul le décodeur `mpeg4` et son parseur y seront compilés |
| Chaîne Emscripten : image Docker `emscripten/emsdk` à tag figé (Docker est déjà installé ici) ou emsdk | Docker Hub / dépôt Emscripten | de l'ordre de 1 à 2 Gio (outillage de build, jamais livré) | Compiler en WebAssembly |

Aucun autre élément. Configuration visée : `--disable-everything --enable-decoder=mpeg4 --enable-parser=mpeg4video`, **sans** `--enable-gpl` ni `--enable-nonfree` ; module attendu de l'ordre de quelques centaines de Kio (**estimation**, à mesurer). Livrables dans le dépôt : `.wasm` + glue JS (< 20 Kio estimé), le **numéro de version exact de FFmpeg, la somme SHA-256 de l'archive, la ligne `configure`** et l'avis de licence LGPL ; l'obligation LGPL de fournir les sources est remplie par ces références et l'archive d'origine.

Critères d'arrêt du spike (chiffrés, SD uniquement ≤ 720 px de large) : ≥ 30 images/s soutenues en décodage seul à 640×480 sur le Mac et sur l'appareil le plus faible visé ; `.wasm` < 1 Mio ; aucune erreur de désynchronisation sur 10 min de lecture ; mémoire < 150 Mio. Au-delà de ces critères, l'option est abandonnée et ADR-0014 est remplacé.

## À vérifier avant tout engagement (non fait)

- Taille réelle du module `mpeg4` minimal et débit mesuré en WASM (build local : voir la liste de téléchargements ci-dessus).
- Licence de chaque composant lié dans la variante, ligne `configure` sans `--enable-gpl` ni `--enable-nonfree`.
- `VideoDecoder.isConfigSupported()` pour MPEG-4 ASP sur les navigateurs cibles.
- ~~Résolutions et codecs audio réels des AVI du catalogue~~ : **fait** (voir le relevé) ; reste 1 AVI illisible à examiner.

## Sources consultées (lecture seule, aucun téléchargement de binaire)

- [libav.js](https://github.com/Yahweasel/libav.js) : variantes, tailles approximatives, licences, construction de variantes personnalisées.
- [FFmpeg — aspects légaux](https://ffmpeg.org/legal.html) : LGPL/GPL, `--enable-gpl`, obligations de distribution.
- [MDN — WebCodecs API](https://developer.mozilla.org/docs/Web/API/WebCodecs_API) et [Can I use — WebCodecs](https://caniuse.com/webcodecs) : codecs et support ; MPEG-4 Part 2 non mentionné.
- Dépôts et guides tiers sur ffmpeg.wasm (taille ≈ 25–31 Mio, COOP/COEP pour le multi-thread) : informations **non officielles**, à confirmer avant usage.
