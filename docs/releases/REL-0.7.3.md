# 0.7.3 — published release / version publiée

Published / publiée : 2026-10-21. Documentation synchronized / synchronisée :
2026-10-22. Mac, server and web: **0.7.3**. Themes remain **0.7.1**;
API **2026-10-14**, database schema **3**, theme renderer **2** are unchanged.

## Français

- Cible Mac **macOS15+ / Apple Silicon**, avec bibliothèques officielles ARM64
  Sequoia figées, isolées du Homebrew installé ; rejet d'un minimum supérieur15.0.
- Catégorie **En cours** : vidéos/épisodes non vus avec une reprise d'au moins5%.
- Barres de progression sur les vignettes des vidéos commencées et pourcentage
  dans la fiche. La durée totale exacte est mesurée par le lecteur au premier
  lancement puis persistée avec les événements validés. Aucune estimation TMDB.
- Endpoint groupé par profil, additif dans l'API actuelle, indépendant des
  deltas catalogue. Mac et web adaptés, accès mobile dans Plus.
- Après rollback vers un serveur sans endpoint, purge du seul cache de mesures
  concerné ; une panne réseau transitoire conserve les données hors ligne.

**Paquets publiés** : [release v0.7.3](https://github.com/ungeekrange-cmyk/chillcenter/releases/tag/v0.7.3),
DMG Mac 0.7.3/build13, serveur et web 0.7.3 ; thèmes 0.7.1 conservés.
L'affichage mesuré nécessite le serveur 0.7.3. Aucun accès direct au Pi/NAS
ni changement de réglage dans cette publication ; une politique automatique
existante peut détecter et installer les nouveaux paquets.

[Télécharger le DMG Apple Silicon](https://github.com/ungeekrange-cmyk/chillcenter/releases/download/v0.7.3/Chill-Center-0.7.3-macos-arm64.dmg).
Les [sources correspondantes du Mac](https://github.com/ungeekrange-cmyk/chillcenter/releases/download/v0.7.3/Chill-Center-0.7.3-corresponding-sources.tar.gz)
incluent les 42 bibliothèques, licences, recettes et correctifs ; FFmpeg est
sous GPL-3.0-or-later. Les signatures et `SOURCE-ACCESS.md` accompagnent les paquets.

## English

- **macOS15+ / Apple Silicon** Mac target, with pinned official ARM64 Sequoia
  libraries isolated from installed Homebrew; reject deployment minima above15.0.
- **In progress**: unwatched videos/episodes with a saved position at least5% in.
- Poster bars for started videos and detail percentages. Exact total duration
  comes from the player on first playback and is persisted with accepted
  progress events. No TMDB duration estimate.
- Additive profile batch endpoint in the existing API, independent of catalogue
  deltas. Both clients adapted, with the mobile entry under More.
- An unsupported server rollback clears only the scoped measured cache;
  transient offline failures retain observations.

**Published packages**: [v0.7.3 release](https://github.com/ungeekrange-cmyk/chillcenter/releases/tag/v0.7.3),
Mac DMG 0.7.3/build13, server and web 0.7.3; themes remain 0.7.1.
Measured progress requires server 0.7.3. No direct Pi/NAS access or settings
change is part of publication; an existing automatic policy may detect and
install the new packages.

[Download the Apple Silicon DMG](https://github.com/ungeekrange-cmyk/chillcenter/releases/download/v0.7.3/Chill-Center-0.7.3-macos-arm64.dmg).
[Mac corresponding sources](https://github.com/ungeekrange-cmyk/chillcenter/releases/download/v0.7.3/Chill-Center-0.7.3-corresponding-sources.tar.gz)
include the 42 libraries, licenses, recipes and patches; FFmpeg is under
GPL-3.0-or-later. Signatures and `SOURCE-ACCESS.md` accompany the packages.

## Verification and limits / Vérifications et limites

Functional source `e54eb357` has independent code/security R2 approval;
[candidate build/test evidence](REL-MACOS15-PROGRESS-CANDIDATE.md) includes
334 UI/web,25 documentation,24 Python and276 Rust tests passing (2 NFR ignored),
51 Mach-O minima checks, libmpv load, strict signature, DMG and Chromium smoke.
Those are historical proofs of that candidate, separate from final 0.7.3 packages.

Final immutable package source:
`77eb17a5c40b61aedacdd69270589cb4b1143ad5`. Four annotated tags point to it:
`mac-v0.7.3`, `server-v0.7.3`, `web-v0.7.3`, `v0.7.3`. Mac/server/web reserve
sequence4; themes retain sequence2. Documentation corrections are subsequent
commits and do not move those tags or replace any published bytes.

Final release checks recorded by the independent publication gate: Mac10,
server14, web4 and umbrella25 assets compared to sealed local names, sizes and
SHA-256 digests; v0.7.3 is public, non-prerelease and Latest. Final Mac checks:
51 Mach-O minima, strict signature, libmpv load and read-only DMG mount PASS.
Actual Linux ARM64 VM smoke with the final server/web and strict HTTPS PASS.
macOS15/26 physical execution and iPhone tests remain **NOT_RUN**. Mac is ad-hoc
signed, not Apple-notarized. This documentation correction reruns only targeted
documentation checks, not playback tests or deployment.

Integration checks on0.7.3:33 documentation/version/release tests PASS;
`cargo metadata --locked --no-deps` PASS; server0.7.3 library tests148PASS/1NFR
ignored; generated web build reports0.7.3; formatting and diff checks PASS.
Added lines/new files were scanned for private keys, certificates, GitHub token
patterns and the household server address, with no findings. This heuristic
does not replace the independent implementation reviews cited above.

## Package SHA-256 / Empreintes des paquets

| Asset | SHA-256 |
| --- | --- |
| `Chill-Center-0.7.3-macos-arm64.dmg` | `a15d897cb687e148f0970fff2a552d578f30332c5c54a61a3a9bf4cc20dc2e55` |
| `Chill-Center-server-0.7.3-linux-aarch64.tar.gz` | `2986a43781c10159f4daba371106f859cf3adb7d7ddf0ab85e8a5ffb86cba576` |
| `Chill-Center-web-0.7.3-install.zip` | `ad67059ad16ee8170364aa679a75a61bca0944371c35f021159608e306600175` |
| `Chill-Center-0.7.3-corresponding-sources.tar.gz` | `e9303b1d51466104bba82da4098d53faa3f8c3affc9fe305be5a88323c0f63e6` |

Checksums identify bytes, not trust on their own. Verify the matching Minisign
signature with the pinned release key; see the [FR](../updates/GUIDE-FR.md) /
[EN](../updates/GUIDE-EN.md) installation guides. Previous releases stay available.
