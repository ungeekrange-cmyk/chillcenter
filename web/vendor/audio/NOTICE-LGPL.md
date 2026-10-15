# Transcodeur audio (AC-3, E-AC-3, DTS, TrueHD vers AAC) — avis de licence

`audio-transcoder.wasm` et `audio-transcoder.js` sont une compilation WebAssembly de **FFmpeg 9.0.2** (bibliothèques `libavcodec`, `libswresample` et `libavutil` ; décodeurs `ac3`, `eac3`, `dca`, `truehd`, `mlp` ; encodeur `aac` natif), distribuée sous **LGPL v2.1 ou ultérieure**. La compilation n'utilise ni `--enable-gpl` ni `--enable-nonfree` ni bibliothèque externe.

- Sources exactes : <https://ffmpeg.org/releases/ffmpeg-9.0.2.tar.xz>, SHA-256 `8c3850283eb25fa026482078a04051e0be17347b09ef81a0849bec15a96e002e` (valeur identique à celle épinglée par la formule Homebrew de FFmpeg 9.0.2).
- Ligne `configure`, version d'Emscripten et sommes de contrôle des binaires : [`BUILD-INFO.json`](BUILD-INFO.json).
- Reconstruction : [`web/decoder/audio/build-in-docker.sh`](../../decoder/audio/build-in-docker.sh) (image `emscripten/emsdk` épinglée par digest, sans réseau dans le conteneur). Le code propre au projet (enveloppe C `wrapper.c`, couche JS) se trouve dans `web/decoder/audio/` et `web/src/`.
- Conformément à la LGPL, vous pouvez remplacer ce module par une version recompilée à partir des mêmes sources : le chargeur n'exige que les fonctions exportées listées dans `build.sh`.
- Usage : le navigateur du visiteur convertit localement la piste audio en AAC ; le serveur ne transcode jamais.

Dolby, Dolby Digital et Dolby Digital Plus sont des marques de Dolby Laboratories ; DTS est une marque de DTS, Inc. ; FFmpeg est une marque de Fabrice Bellard. Ce projet n'est affilié à aucun d'eux ni approuvé par eux.
