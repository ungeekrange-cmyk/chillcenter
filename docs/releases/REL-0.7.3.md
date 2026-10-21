# 0.7.3 — source on main / sources sur main

Date: 2026-10-21. Mac, server and web: **0.7.3**. Themes remain **0.7.1**;
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

**Publication des sources uniquement** : aucun tag/release/paquet0.7.3 créé,
aucun Pi/NAS/réglage modifié. Les téléchargements publiés restent0.7.2,
thèmes0.7.1. L'affichage mesuré nécessite le futur serveur0.7.3 ; il ne devient
pas disponible au foyer par un simple push Git. Le DMG local de test antérieur
reste identifié0.7.2/build12 et n'est ni renommé ni présenté comme0.7.3.

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

**Source publication only**: no0.7.3 tag, release or package is created. No Pi,
NAS or configuration is modified. Published downloads stay0.7.2, themes0.7.1.
Measured progress needs a later server0.7.3 delivery, not merely a Git push.
The earlier local test DMG remains0.7.2/build12, never relabelled as0.7.3.

## Verification and limits / Vérifications et limites

Functional source `e54eb357` has independent code/security R2 approval;
[candidate build/test evidence](REL-MACOS15-PROGRESS-CANDIDATE.md) includes
334 UI/web,25 documentation,24 Python and276 Rust tests passing (2 NFR ignored),
51 Mach-O minima checks, libmpv load, strict signature, DMG and Chromium smoke.
Those are proofs of that exact candidate, not rebuilt0.7.3 packages.

This integration changes only version metadata, README, documentation and their
checks after that reviewed implementation. macOS15/26 physical execution is
**NOT_RUN**, not inferred from static minima. A later release must build/sign
distinct packages from the new version and rerun its artifact/release gates.

Integration checks on0.7.3:33 documentation/version/release tests PASS;
`cargo metadata --locked --no-deps` PASS; server0.7.3 library tests148PASS/1NFR
ignored; generated web build reports0.7.3; formatting and diff checks PASS.
Added lines/new files were scanned for private keys, certificates, GitHub token
patterns and the household server address, with no findings. This heuristic
does not replace the independent implementation reviews cited above.
