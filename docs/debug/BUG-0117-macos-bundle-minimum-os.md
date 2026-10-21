# BUG0117 — bundle advertises an older OS than its media libraries require

- Priority: P1, pre-publication compatibility; no affected v0.7 was delivered.
- Source correction: `ac5253328da3bd2cb6a65e9913c0bbe5b8d3d33b`.
- Status: FIXED / VERIFIED_PACKAGE, 20 October 2026; not yet delivered.

The Rust executable's deployment minimum was macOS 13, copied into Info.plist,
but the actual 48 bundled ARM64 media libraries had higher requirements. Of 49
Mach-O files, 34 required macOS 27, 13 required 26, one required 13 and one 11.
The public distributable therefore requires **macOS 27+**, not macOS 13+.

Packaging now inspects every executable/library with system `vtool`, takes the
maximum of their deployment minima and the configured floor, then writes the
new staging bundle's Info.plist **before signing**. Missing or malformed build
requirements fail the build; SDK version is not mistaken for deployment OS.
The README and bilingual installation guide state the actual distribution
requirement. No binary deployment minimum is forged or lowered.

Seven focused Python tests cover modern/legacy commands, numeric ordering,
SDK exclusion, wrong platform, preserved metadata, unchanged files on failure
and the configured floor. The new `ac52533` bundle passed independent all-library
inspection, strict code-sign verification and bundled libmpv loading; Info.plist
advertises 27.0. Any final rebuild must repeat those checks before publication.

Résumé : le bundle fourni exige macOS 27 à cause de ses dépendances embarquées.
Le packaging calcule et annonce désormais le vrai minimum, avant signature.

## 2026-10-21 — separate macOS15 candidate, not a published replacement

The user approved targeting macOS15+. Source `e54eb357` now packages pinned
official ARM64 Sequoia inputs, isolated from installed Homebrew, and refuses
any deployment minimum above15.0 before signing. It does not forge load commands
or replace the existing public0.7.2 asset. Candidate build12 is separate from
published build11; full candidate evidence belongs to
[the local delivery record](../releases/REL-MACOS15-PROGRESS-CANDIDATE.md).

FR : le candidat local utilise réellement des bibliothèques compatibles avec
un minimum15.0. Les essais physiques sur macOS15/26 restent **NOT_RUN** ; ne pas
les déduire du contrôle statique. Le DMG publié27+ n'a pas été modifié.
