# BUG-0071 — client web : pas de son sur les films en AC-3 / E-AC-3 (Baby Driver, Deadpool & Wolverine)

- Statut : **corrigé** le 15 octobre 2026 (branche `codex/web-pwa`), vérifié en navigateur réel sur un film de test ; **à vérifier sur les deux films réels après déploiement**. Nature : **limite du client web** (codec non lisible par MSE), pas un défaut du serveur ni des fichiers.
- Numérotation : BUG-0071 (suite de BUG-0070, hors de la plage de la branche du client Mac).

## Symptômes

Lecture d'un film : l'image passe, aucun son. Message : « Audio non pris en charge par ce navigateur : A_AC3. Lecture sans son. » Signalé pour Baby Driver et Deadpool & Wolverine ; « un certain nombre » d'autres.

## Environnement

Client web sur le Pi ; navigateur Chromium (application) ; MKV du NAS.

## Hypothèses et preuves

1. *Fichiers sans piste audio ?* Faux : lecture des pistes par le démuxeur sur le Pi (volatile, sans progression) : Baby Driver = 2 pistes `A_AC3` (fr et sans langue), Deadpool & Wolverine = 1 piste `A_EAC3` (en).
2. *Le navigateur ne sait pas lire ces codecs ?* Vrai : `MediaSource.isTypeSupported` donne `false` pour `ac-3`, `ec-3`, `dtsc` (navigateur de l'application) ; Chrome n'a en général pas le décodeur, Safari l'a.
3. *Étendue :* sur 12 MKV réels échantillonnés, 6 portent de l'AC-3 et 1 de l'E-AC-3 (≈ 58 %).
4. *Reproduction locale :* film de test à deux pistes AC-3 sur un serveur réel, navigateur de l'application : exactement le message ci-dessus, `tracks: []`.

## Correction

Conversion locale en AAC par un transcodeur WebAssembly (FFmpeg LGPL) : [ADR-0020](../architecture/decisions/ADR-0020-web-audio-transcode-to-aac.md). Les pistes sont déclarées comme AAC ordinaire ; le serveur ne transcode rien.

## Régression

`web/test/audio-transcode.test.js` (9 tests, vrai module + décodeur indépendant), fixtures `web/test/fixtures/*.mkv` régénérables par `scripts/make-audio-fixtures.sh`. Test réel dans le navigateur : voir l'ADR. **Non vérifié** : lecture des deux films réels (en attente du déploiement), MP4/AVI à audio AC-3, charge CPU sur téléphone.

## Note d'outillage (constat annexe, corrigé)

Pendant ce test, le navigateur a servi un ancien `mkv.js` : les fichiers statiques étaient mis en cache une heure sans version dans l'URL. Chaque script, feuille de style et Worker porte désormais `?v=<build>` (voir ADR-0019, décision 7).
