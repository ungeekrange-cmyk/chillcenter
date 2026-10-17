# Revue indépendante / Independent review — web mobile R3

Date : 2026-10-17. Base : `dc74ac0`. Candidat figé :
`acd7aebfc3c9b902eb7b4b414a14750d86017e08` (`fix(web): support AVI MP3
fallback and mobile playback controls`). Cette revue est indépendante des
auteurs ; elle ne couvre ni un déploiement ni un essai sur le NAS.

## Français

**APPROVED_SCOPED** — aucun défaut bloquant ou majeur n'a été trouvé dans le
lot figé. Le diff fonctionnel reste dans le client web et ses tests/preuves ;
aucun chemin du client Mac, du binaire serveur Rust, des contrats API, de la
configuration ou du NAS n'est modifié.

Le repli AVI MP3 garde le chemin MSE natif lorsqu'il est annoncé disponible et,
sinon, reconstruit uniquement l'audio en AAC-LC dans le Worker FFmpeg/WASM du
navigateur. La reconstruction assemble les trames MP3 coupées ou concaténées,
conserve la chronologie, écarte les résultats devenus obsolètes après une
recherche/fermeture et ferme le Worker. Les contrôles mobiles ne couvrent plus
les affiches du catalogue ; la recherche reste appliquée lors d'un changement de
rubrique. Le plein écran est demandé dans le geste tactile et n'est déclaré
actif qu'après confirmation par le navigateur.

Deux défauts découverts pendant cette revue ont été corrigés avant le gel :

- le fragment AAC adressait toutes ses trames à l'offset zéro, ce qui faisait
  échouer la conversion AVI MP3 avec `RangeError` ;
- la génération AVI n'était pas initialisée et le tampon MP3 partiel survivait
  à une recherche, ce qui pouvait arrêter ou contaminer l'audio après un seek.

Preuves exécutées indépendamment sur ce checkout :

- `git diff --check dc74ac0` : PASS ;
- `node --test ui/connected/*.test.js ui/connected/tests/*.test.js web/test/*.test.js` : **311/311 PASS** ;
- `node --test scripts/test-bilingual-docs.mjs` : **18/18 PASS** ;
- QA Chromium du build : mobile, parité bureau et lecture synthétique : PASS.
  La dernière génère un MKV H.264/AAC et un AVI Xvid/MP3, force le repli local
  MP3→AAC et ne contacte ni Pi, ni NAS, ni API réelle.

Archive contrôlée indépendamment :
`/private/tmp/chillcenter-web-mobile-r3.GPDLb7/package/chillcenter-web.tar`,
SHA-256 `34a1663eda23e972ecef6e8b68ac7641432465f8d751eeaea0de8bffe9316bda`,
6 683 648 octets, build `a15795b5c4cd`. Ses 88 membres sont 82 fichiers
réguliers et 6 répertoires sous `web/`, sans lien, nœud spécial, doublon, chemin
absolu ni remontée. Les 81 entrées de `web/SHA256SUMS` couvrent exactement les
81 fichiers hors manifeste et chaque empreinte est valide. Le SHA du WASM
(`5075105b…d47d5`) est identique à `BUILD-INFO.json` ; l'avis LGPL cite MP3.

Limites à conserver : cette preuve ne valide pas Safari/Vivaldi iOS réel,
ManagedMediaSource sur appareil, les deux vidéos du foyer, la consommation
réelle sur iPhone ou tous les AVI. Sur iPhone, un élément vidéo qui n'existe pas
encore au premier geste exige un second toucher visible ; un Xvid rendu sur
canvas ne peut pas entrer en plein écran vidéo natif si le navigateur refuse le
plein écran du panneau. Ce comportement est explicite, sans faux succès.

## English

**APPROVED_SCOPED** — no blocking or major defect was found in the frozen
candidate. Functional changes remain in the web client and its evidence; no Mac
client, Rust server binary, API contract, configuration, or NAS path changed.

The AVI MP3 fallback retains native MSE when advertised, otherwise rebuilds
audio only as AAC-LC in the browser FFmpeg/WASM Worker. It reassembles split or
concatenated MP3 frames, preserves the timeline, rejects stale seek/close work,
and closes the Worker. Mobile catalogue posters are unobscured, category changes
retain search, and fullscreen is requested in the touch gesture and reported
only from actual browser state.

Review findings fixed before freeze: AAC fragment offsets all pointed at entry
zero (causing a `RangeError`), and AVI generation/partial MP3 state was unsafe
across seek. Independent evidence executed on this checkout: 311/311 combined
Node tests, 18/18 bilingual-document tests, clean `git diff --check dc74ac0`,
and all three built Chromium QA runs (mobile, desktop parity, synthetic MKV and
Xvid/MP3 AVI playback). The synthetic test forces local MP3-to-AAC fallback and
does not contact the Pi, NAS, or live API.

The reviewed archive is
`/private/tmp/chillcenter-web-mobile-r3.GPDLb7/package/chillcenter-web.tar`,
SHA-256 `34a1663eda23e972ecef6e8b68ac7641432465f8d751eeaea0de8bffe9316bda`,
6,683,648 bytes, build `a15795b5c4cd`. It has 88 members: 82 regular files and
6 directories under `web/`, with no links, special nodes, duplicate, absolute,
or traversal paths. All 81 manifest hashes validate and cover precisely all
non-manifest files. The WASM SHA matches `BUILD-INFO.json`; the LGPL notice
correctly names MP3.

This is not a physical iPhone/Safari/Vivaldi validation, proof for household
files, a battery measurement, or a claim to support every AVI. A video element
not yet available during the first iPhone gesture needs the visible second tap;
a canvas Xvid path cannot use native video fullscreen when panel fullscreen is
unavailable. The implementation states these limitations instead of reporting
false fullscreen success.
