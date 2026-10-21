# Local candidate — macOS15 and measured progress / Candidat local

Date: 2026-10-21. Branch: `codex/macos15-build`. Functional source:
`e54eb35726431bea2c01e6ad85d01e21da379de8`, base `e8d6f53`.
Subsequent documentation commits do not change packaged source bytes.

## Français

La cible macOS15+ et les fonctions de reprise ont été demandées explicitement.
Ce candidat **local, non publié** reste numéroté0.7.2/build12 pour les essais,
distinct du paquet public0.7.2/build11/macOS27+. Il faut une nouvelle version
avant une publication ou une distribution par le système de mise à jour.

- Entrées officielles Homebrew ARM64 Sequoia figées par SHA-256, isolées du
  Homebrew installé. Toutes les dépendances sont contrôlées avant signature.
- « En cours » : vidéos/épisodes non vus avec reprise d'au moins5% d'une durée
  réellement mesurée. Barres sur les vidéos commencées et pourcentage sur fiche.
- La durée est fournie par le lecteur au premier lancement, dans les événements
  Progress déjà validés par le bail ; persistance transactionnelle côté serveur.
  Aucune estimation TMDB ni analyse des vidéos sur le NAS par le serveur.
- Une reprise historique sans mesure conserve sa position ; pas de pourcentage
  inventé avant sa prochaine lecture. La synchronisation catalogue reste séparée.
- Mac et web partagent la présentation ; menu mobile Plus pour « En cours ».

**Pi/NAS, client web installé, réglages, applications normales et versions
publiées inchangés.** Le serveur domestique0.7.2 ne fournit pas le nouvel endpoint
additif. Il faudra une livraison serveur autorisée pour activer cette fonction
avec des données réelles ; un404/405 ne bloque pas le reste de l'application.

## English

User-approved macOS15+ target and exact resume progress. This is a **local,
unpublished**0.7.2/build12 test candidate, not the published0.7.2/build11/macOS27+
asset. Allocate a distinct release version before publication/automatic delivery.

The package uses hash-pinned official ARM64 Sequoia inputs, without replacing
installed Homebrew. In progress lists unwatched individual videos/episodes at
least5% into a measured duration; started videos get poster bars and a detail
percentage. The player reports exact duration on first playback; the server
persists it transactionally with accepted lease-validated Progress. Historical
resumes without measurement keep their position, with no invented percentage.
No TMDB estimate, NAS probe or server transcoding is introduced.

The Mac and responsive web UI use one profile-scoped batch read independent of
catalogue deltas. The home Pi/web/configuration/NAS and public releases were not
touched. The deployed0.7.2 server needs a later authorized upgrade for this new
additive endpoint. Older servers disable only this feature, not the application.

## Artifacts and exact provenance

Local output directory: `/private/tmp/chillcenter-macos15-delivery.4D1l0s`.

| Artifact | Identity |
| :--- | :--- |
| Final app | `Chill Center-reviewed.app`, Apple Silicon,0.7.2/build12, minimum15.0 |
| Final DMG | `Chill-Center-0.7.2-macos15-candidate-build12.dmg`,51,850,429bytes |
| DMG SHA-256 | `a4c42ea882db3ba848013d1a89cfca1e6f0025b3b1e8f0e9b502c6a2411c4ab6` |
| Desktop executable SHA-256 | `7095c4c4d9dff91ab8da1fa1d52834acc8a4e53efd0640f5c0f2c9626e521595` |
| Media input lock SHA-256 | `3fde480042da7cdaee227f984f8e85aae0d876cda06a5fa57febfe5df052cbd1` |
| Generated web test copy | `web-reviewed`, build `f0fa4e3c4300`,102files; not deployed |

The earlier `Chill Center.app` in that directory was built from8283667 before
the review correction; **do not use it as the final candidate**. Neither copy
overwrites an existing distribution. The DMG contains the final reviewed app
under the standard name `Chill Center.app` and a link to Applications.

Reproduction uses the reviewed source and:

```sh
CHILLCENTER_MEDIA_ROOT=/absolute/new/verified-media-inputs \
CARGO_TARGET_DIR=/absolute/build-cache \
bash scripts/package-macos.sh '/absolute/new/Chill Center-reviewed.app'
python3 scripts/package-macos-dmg.py '/absolute/new/Chill Center-reviewed.app' \
  /absolute/new/Chill-Center-candidate.dmg
```

The packaging script prepares inputs from the checked-in lock when absent,
verifies existing input receipts, and exclusively publishes new outputs.

## Executed proof / Preuves exécutées

| Command/control | Result and scope |
| :--- | :--- |
| `node --test ui/connected/*.test.js ui/connected/tests/*.test.js web/test/*.test.js` |334PASS |
| `node --test scripts/test-bilingual-docs.mjs` |25PASS |
| `python3 -m unittest discover -s scripts/helpers -p 'test_macos*.py'` |21PASS |
| Same Python discovery, `test_publish_macos_bundle.py` |3PASS |
| `cargo test -p chill-domain --lib` |75PASS, including persistence/reopen and exact-duration isolation |
| `cargo test -p chill-client --lib` |53PASS,1 explicit memory NFR ignored |
| `cargo test -p chill-server --lib` |148PASS,1 explicit NFR ignored; actual router/lease progress contract |
| `cargo fmt --all -- --check`, Bash syntax, `git diff --check` |PASS |
| Final release bundle build |PASS;50 relocated ARM64 dylibs from42 runtime formulas |
| All51 Mach-O deployment minima |Maximum15.0; no forged minimum, SDK version not confused with minimum |
| Final `codesign --verify --deep --strict`; libmpv load |PASS, client API2.5 on hostmacOS27.0.1 |
| DMG `hdiutil verify`, read-only mount inspection |PASS; Applications link, executable hash, minimum15.0 and strict signature checked; volume detached |
| Real headless Chromium, isolated HTTP API fixtures |PASS;5% threshold,42% detail, preserved search,390×844 mobile More navigation, no overflow/JS error;3 batch requests |

Browser evidence is in `progress-desktop.png`, `progress-detail.png`,
`progress-mobile.png` and `progress-browser.cjs` beside the artifacts. These use
fictitious media, not the Pi/NAS, and do not prove actual iPhone/native playback.

The first source2edaab5 build failed before output publication on an untyped
`None` body.8283667 fixed the Rust annotation. Independent code R1 then found
[BUG0131](../debug/BUG-0131-measured-progress-server-rollback.md); e54eb357 fixes
the stale cache and closes the security review's LOW symlink validation issue.
All earlier reports remain unchanged.

## Reviews, remaining gates and rollback

- [Code R1](../reviews/MACOS15-PROGRESS-code.md): CHANGES_REQUESTED, historical.
- [Code R2](../reviews/MACOS15-PROGRESS-code-R2.md): **APPROVED**, exact e54eb357.
- [Security R1](../reviews/MACOS15-PROGRESS-security.md): APPROVED with one LOW.
- [Security R2](../reviews/MACOS15-PROGRESS-security-R2.md): **APPROVED**, LOW remediated, exact e54eb357.
- Physical macOS15/26 execution: **NOT_RUN**. Static minima and loading on27 do
  not certify operation on those older machines. No normal app was launched.
- No new public release, tag, push, main merge, server deployment or web deployment.
- Ad-hoc signed, **not Apple-notarized**. Public codec distribution still needs
  the corresponding-source/license obligations checked before release; bundled
  notices and provenance are not by themselves a legal compliance verdict.
- [ADR0034](../architecture/decisions/ADR-0034-macos15-media-build.md) and
  [ADR0035](../architecture/decisions/ADR-0035-measured-resume-view.md) retain their
  proposed implementation-detail status; code/security approval is not ADR approval.

Rollback: keep using the retained published application; this task installed
nothing. The additive measurement table leaves legacy personal/catalogue rows
intact; an older server ignores it. Do not remove that table or reset profiles
to roll back. Client404/405 clears only its incompatible scoped observation
cache; transient errors preserve offline observations. Legacy portable exports
do not include measurements: remeasure after import.
