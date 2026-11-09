# 0.7.8 — Reprendre : progression et coche / Continue watching progress

## Français

Candidat préparé le9novembre2026, non encore publié/installé. Mac0.7.8/build18/
séquence9, serveur/web0.7.7/séquence8 ; thèmes0.7.1 inchangés.

Reprendre affiche le pourcentage réel depuis la durée du fichier, sans durée
TMDB supposée ni lecture vidéo. Les essais sous5 % sont écartés sans effacer
leur position. Les séries indiquent les épisodes vus sur le total de la saison
affichée. La coche marque le film ou uniquement cette saison comme vue ; après
enregistrement réussi, fondu puis glissement des cartes. Échec : carte conservée.
Les barres restent visibles après chargement des jaquettes. Profil, positions,
notes et autres saisons sont conservés ; aucun changement de schéma ou de NAS.

## English

Candidate prepared on9November2026, not yet published/installed. Mac0.7.8/build18/
sequence9, server/web0.7.7/sequence8; themes0.7.1 unchanged.

Continue watching shows real progress from file headers without guessed TMDB
durations or video playback. Trials below5 % are excluded without deleting
their saved positions. Shows display watched/total episodes for the selected
season. The checkbox marks the film or that season watched; after a successful
save, the card fades and its neighbors slide left. Failed saves keep the card.
Progress bars survive poster loading. Profiles, positions, ratings and other
seasons are preserved, with no schema migration or NAS changes.

Initial functional base fe28a777bfd5229347f9228ef91796ba92112585 passed302 Rust,
377 JavaScript and27 documentation tests, strict Clippy, Mac compilation and
real-file header verification. The unchanged core evidence is retained.
Focus correction 1dc912a6856bb3975b51219329598e8e3dd43df2 passed377 JavaScript,
27 documentation tests and Chromium desktop/mobile/reduced-motion journeys,
including scrolled middle/last removal and revision-driven focus restoration.
The final clean build commit is recorded in the signed PROVENANCE.json and
component manifests; this release record is completed after publication.
Independent release review and exact-version native validation are separate.

[BUG0141](../debug/BUG-0141-home-progress-and-watched-controls.md) ·
[ADR0042](../architecture/decisions/ADR-0042-home-progress-controls.md)
