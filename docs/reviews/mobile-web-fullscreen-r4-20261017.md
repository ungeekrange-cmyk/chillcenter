# Revue indépendante — Web mobile R4 (2026-10-17)

## Verdict

**APPROUVÉE DANS LE PÉRIMÈTRE.** Aucun bloqueur relevé dans la candidate
figée `fa13d447abd4c35e9ba1f14f85ef05eaa2b25070` (parent
`4f41a86170d60f51a8dc028c0405fa4c3485e869`). La portée reste exclusivement
le client Web : aucun changement du serveur, de l’API, du Pi/NAS, du client
Mac, ni nouvelle dépendance.

## Constats

- `web/src/mobile-playback.js` expire l’avertissement au bout de 2,5 s. Les
  rafraîchissements de média, langue, orientation ou taille ne recréent pas
  l’échéance ; une nouvelle tentative explicite seulement peut la recréer.
- La tentative reste dans le geste utilisateur et essaie, sans déclarer un
  succès fictif, l’API standard avec `navigationUI: "hide"`, les orthographes
  WebKit `webkitEnterFullscreen` / `webkitEnterFullScreen`, puis le mode de
  présentation WebKit. L’état affiché est celui réellement signalé par le
  navigateur.
- `web/src/mobile.css` emploie uniquement les `safe-area-inset-*` fournis par
  l’OS. L’image, les commandes, l’indication et le rail paysage tiennent
  compte des quatre bords ; les vérifications Chromium à insets synthétiques
  confirment l’absence de débordement gauche/droite/bas.
- `web/src/mobile-ui.js` corrige l’erreur d’arrondi qui pouvait transformer
  trois colonnes en deux pour une largeur CSS fractionnaire. La correction ne
  modifie pas les seuils fonctionnels 320 px / trois colonnes.

## Preuves exécutées

| Vérification | Résultat |
| --- | --- |
| `git diff --check 4f41a86` | PASS |
| `node --test web/test/mobile-playback.test.js web/test/mobile-ui.test.js` | 36/36 PASS |
| `node --test ui/connected/*.test.js ui/connected/tests/*.test.js web/test/*.test.js` | 315/315 PASS |
| `CHILL_PREVIEW_NODE_MODULES=… node scripts/test-web-mobile.cjs` | PASS : vues 320/390/430/736/844, bureau 1280, entrée/sortie plein écran Chromium, insets, rail paysage et largeur fractionnaire |
| `node scripts/test-bilingual-docs.mjs` | PASS après ajout de ce rapport |

## Archive vérifiée indépendamment

Archive contrôlée :
`/private/tmp/chillcenter-web-mobile-r4.Q9nvhr/package/chillcenter-web.tar`.

- SHA-256 : `c139dca29fe63c5bb9ed016b8ac7f180b83f8f01475ff379be8eb298dba80182`
  (6 687 744 octets) ; build `67773f7877d3` cohérent avec `web/build.json`.
- 88 entrées TAR : aucun chemin absolu ou traversant, aucun doublon, aucun
  lien/special file (seulement les six répertoires attendus et fichiers
  ordinaires).
- Extraction isolée réussie ; les 81 entrées de `SHA256SUMS` sont toutes
  valides et correspondent exactement à `web/dist` (plus le manifeste).
- Les hashes WASM archivés correspondent aux provenance files : audio
  `5075105bfdfcde924be0163a785a9d5fa0d2f1eb2c0d52aecf5ea571497d47d5`, MPEG-4
  `453fb7b171fde50d1306a57793ebac571459a7c324d28bd8b4f6c3db6b7bda53`.
  Les deux `BUILD-INFO.json` et avis LGPL sont présents.

## Limites de validation / suivi

- **RETEST_APPAREIL_SAFARI_REQUIS :** Chromium et les doubles WebKit testent
  les contrats asynchrones et les géométries, mais ne prouvent pas l’entrée en
  plein écran sur un iPhone physique. Tester Safari iPhone avec vidéo native,
  présentation WebKit, sortie et les deux orientations avec encoche.
- Les canvas ou navigateurs qui refusent le plein écran restent explicitement
  limités à l’écran disponible : aucune promesse de plein écran réel n’est
  faite. Un éventuel plein écran natif d’application est une étude de
  faisabilité distincte ; il n’est ni autorisé ni inclus dans R4.

## English summary

**APPROVED IN SCOPE.** The frozen Web-only candidate expires the Safari hint
once after 2.5 seconds, tries standard and WebKit fullscreen APIs from the
trusted gesture without inventing success, uses OS safe-area insets, and
stabilizes fractional-width three-column layouts. The targeted suite passed
36/36, the full Node suite 315/315, Chromium mobile QA passed, and the archive
hash, manifest, paths, file types, build provenance and WASM notices were
verified independently. Physical iPhone Safari remains a required device
retest; no native-app implementation is part of this release.

`totals: 0🔴 0🟡 0🔵 1❓ (physical Safari device retest)`
