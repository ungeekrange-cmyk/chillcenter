# ADR-0013 — Moteur de lecture web : standards seulement, zéro dépendance

- Status : `PROPOSED` ; date : `2026-10-14`. Complète ADR-0012 (point 3) sans le remplacer.
- Decision Owner / Author Instance : `/root` ; Reviewer Instance : à désigner (≠ auteur).
- Branche : `codex/web-pwa`. Supersedes / Superseded By : `NONE` / `NONE`.
- Related Items : ADR-0008 (I/O bornées), ADR-0010 (cache), ADR-0012.

## Contexte

Demande utilisateur (14 octobre 2026) : technologies **le moins sujettes aux changements de version** et **les plus légères possible**. ADR-0012 laissait ouverte la voie WASM (ffmpeg/libav.js) ou une bibliothèque de démuxage (mp4box.js).

## Options

| Option | Légèreté | Stabilité | Couverture |
| --- | --- | --- | --- |
| libav.js / ffmpeg.wasm | plusieurs Mo, licences LGPL/GPL | dépendance fortement versionnée, builds fragiles | tous formats, décodage logiciel coûteux |
| mp4box.js (ou autre bibliothèque) | ~100 Kio | dépendance externe à épingler et suivre | MP4 seulement |
| **Code propre en JS pur + API navigateur standard** | quelques dizaines de Kio, aucun téléchargement | seules évoluent les API W3C (`fetch`, MediaSource), stables et rétrocompatibles | MP4 d'abord, puis MKV ; AVI après mesure |

## Décision

1. **Aucune bibliothèque tierce dans le client web** : ni npm, ni CDN, ni WASM. Le seul « runtime » est le navigateur : `fetch` avec `Range`, `MediaSource`/`SourceBuffer`, `HTMLVideoElement`.
2. **Le navigateur décode** (matériel quand disponible). Le code propre ne fait que **démuxer et remuxer en fMP4** (réécriture de boîtes, aucun décodage), exécuté chez le client. Le serveur reste un simple lecteur d'octets.
3. Démuxeurs écrits dans le dépôt, testés contre des fichiers générés par ffmpeg (outil de test uniquement, jamais livré) : **MP4/MOV** et **Matroska/WebM** (EBML) sont implémentés ; AVI reste à décider. AVI : seulement si la bibliothèque réelle en contient assez pour justifier un démuxeur (format ancien, codecs souvent MPEG-4 ASP non décodables par navigateur) ; sinon message explicite.
4. **Limite assumée** : un codec que le navigateur ne décode pas (AC-3/E-AC-3/DTS sur plusieurs navigateurs, MPEG-4 ASP/DivX, parfois HEVC) n'est pas lu. Le moteur choisit une autre piste audio décodable quand elle existe, sinon affiche le codec en cause. Aucun transcodage serveur ; un éventuel décodage WASM d'une seule piste audio fera l'objet d'un ADR ultérieur, après mesure du besoin réel.
5. Les sous-titres texte sont rendus par le client (`<track>`/cues) à partir de ressources déjà prévues par l'API ; le format ASS complet n'est pas promis.

## Conséquences

- Pas de mise à jour de dépendance à suivre ; surface réduite à auditer ; bundle de quelques dizaines de Kio servis par le Pi.
- Couverture inférieure à libmpv : parité F-22 **non acquise** tant que les formats/codecs réels de la bibliothèque ne sont pas inventoriés. À mesurer sur le catalogue (codecs réellement présents) avant d'élargir.
- Code de démuxage à maintenir par le projet : risque de bugs sur des fichiers atypiques (edit lists, fragments, tailles 64 bits) ; mitigé par tests sur fichiers variés et message d'erreur explicite en cas d'échec d'analyse.
- Rollback : le moteur est derrière l'interface `Engine` ; le moteur « indisponible » reste le repli.

## Validation

Tests Node sur fichiers MP4 et MKV générés (H.264/AAC avec B-frames, VP9/Opus), lecture réelle dans le navigateur intégré (MP4 VP9/Opus, MKV VP9/Opus, MKV H.264/AAC avec B-frames), à compléter par un essai sur Mac/Safari/Chrome avec de vrais fichiers du NAS après autorisation. Revue indépendante non faite.

## Constat ultérieur (14 octobre 2026)

Le MP3 (`mp4a.6B`) **n'est pas accepté dans `audio/mp4`** par le Chromium du navigateur intégré, alors que `audio/mpeg` (flux MPEG brut) l'est. Le moteur retombe donc sur un flux brut pour le MP3 (MP4, MKV, AVI), avec resynchronisation sur frontière de trame. Cette adaptation reste en JS pur, sans dépendance. Le support dépend du navigateur : à contrôler sur Safari/Firefox/Chrome cibles.
