# Décodeur MPEG-4 Part 2 (Xvid / DivX) — avis de licence

`mpeg4-decoder.wasm` et `mpeg4-decoder.js` sont une compilation WebAssembly de **FFmpeg 9.0.2** (bibliothèques `libavcodec` et `libavutil`, décodeur `mpeg4` uniquement), distribuée sous **LGPL v2.1 ou ultérieure**. La compilation n'utilise ni `--enable-gpl` ni `--enable-nonfree` ni bibliothèque externe.

- Sources exactes : <https://ffmpeg.org/releases/ffmpeg-9.0.2.tar.xz>, SHA-256 `8c3850283eb25fa026482078a04051e0be17347b09ef81a0849bec15a96e002e` (valeur identique à celle épinglée par la formule Homebrew de FFmpeg 9.0.2).
- Ligne `configure`, version d'Emscripten et sommes de contrôle des binaires : [`BUILD-INFO.json`](BUILD-INFO.json).
- Reconstruction : [`web/decoder/mpeg4/build-in-docker.sh`](../../decoder/mpeg4/build-in-docker.sh) (image `emscripten/emsdk` épinglée par digest, sans réseau dans le conteneur). Le code propre au projet (enveloppe C `wrapper.c`, couche JS) se trouve dans `web/decoder/mpeg4/` et `web/src/`.
- Conformément à la LGPL, vous pouvez remplacer ce module par une version recompilée à partir des mêmes sources : le chargeur n'exige que les fonctions exportées listées dans `build.sh`.

FFmpeg est une marque de Fabrice Bellard ; ce projet n'est ni affilié à FFmpeg ni approuvé par lui.
