# 0.7.8 — Reprendre / Continue watching

## Français

Publié et installé le9novembre2026 : Mac0.7.8/build18/séquence9,
serveur/web0.7.7/séquence8. Thèmes0.7.1 inchangés.
Source immuable `70c8292adeea820155fe47fc92e8ba5a69e8ec70` fusionnée sur main ; documents complétés ensuite.
`v0.7.8` Latest, tags `mac-v0.7.8`, `server-v0.7.7`, `web-v0.7.7`.
51 assets/35 fichiers uniques retéléchargés, empreintes et signatures vérifiées.

Reprendre affiche le pourcentage réel, les épisodes vus/total de la saison,
et écarte les essais de films sous5 % sans effacer les positions. La coche
marque le film ou uniquement la saison affichée ; après sauvegarde réussie,
fondu180ms puis glissement300ms. Échec : carte conservée. Focus clavier sur
le voisin visible, même en fin de rangée défilée ; mouvements réduits respectés.

Installation Pi via le helper normal, sauvegarde complète préinstallation,
santé TLS épinglée, identité serveur et configurations/helper/unités conservés.
Serveur et timer actifs, slots serveur/web0.7.7-8 et séquences8 vérifiés.
Fichiers web servis identiques au paquet signé. Installation Mac depuis
le mécanisme de mise à jour puis relance ; version/build/signature et GUI vérifiés.
Après la maintenance Pi : reconnexion au serveur déjà épinglé et rechargement
du catalogue en lecture seule.
L’API et la GUI confirment la reprise, sa progression et les épisodes vus/total.
Aucune nouvelle lecture lancée, aucun marquage réel de saison pour un test.
Positions, notes, profils et fichiers NAS conservés.

## English

Published and installed on9November2026: Mac0.7.8/build18/sequence9,
server/web0.7.7/sequence8; themes0.7.1 unchanged. Immutable source `70c8292adeea820155fe47fc92e8ba5a69e8ec70`
is on main. All51 public assets/35 unique files were downloaded and verified.
Continue watching shows real progress, watched/total season episodes and
excludes film trials below5 % without deleting their positions. The checkbox
marks the film or displayed season; a successful save fades the card and slides
its neighbors left. Failure keeps the card. Visible adjacent keyboard focus and
reduced motion are preserved, including scrolled middle/last removals.
The Pi signed helper and Mac updater installed the release, with backups,
pinned TLS health, served-byte verification and GUI checks. Personal state and
NAS files are preserved. Real household seasons were not marked just for a test.

## Validation / Evidence

Exact source: `70c8292adeea820155fe47fc92e8ba5a69e8ec70`. Independent source and artifact reviews APPROVED.
Rust: 347 tests; native SDK install/tamper/restart:3;
JavaScript377; bilingual docs27; strict core Clippy reused from unchanged fe28;
Chromium desktop/mobile/reduced motion and existing web/mobile smoke PASS.
GitHub CI runs 42630229302 and 42630229961 succeeded on the exact source.
Mac:51 arm64 Mach-O,50 relocated dylibs, macOS15 minimum, ad-hoc signature,
not Apple notarized. Physical macOS15/26 and iPhone Safari tests not claimed.
Linux AArch64: GLIBC2.28 maximum; updater helper binary unchanged.
Corresponding sources:1165 tracked hashes,42 runtime formulas,54 upstream
inputs,248 internal hashes, all verified. No schema migration or server transcoding.

## Nettoyage et retour arrière / Cleanup and rollback

Nettoyage des caches et intermédiaires propres à cette livraison, anciens
binaires locaux récupérables depuis GitHub et copies QA inactives. Gain d’espace
libre observé : 9721892864 octets ; mesure APFS, distincte des tailles logiques.
Sources, quatre worktrees, historiques et sauvegardes personnelles conservés.
Paquets canoniques actuels, signatures, sources correspondantes et reçus gardés.
Aucune suppression de données, sauvegardes ou slots de rollback du Pi.
Retour arrière logiciel administrateur depuis les paquets signés et sauvegardes
conservés, selon les reçus anti-rollback ; ne pas abaisser les séquences à la main.

[BUG0141](../debug/BUG-0141-home-progress-and-watched-controls.md) ·
[ADR0042](../architecture/decisions/ADR-0042-home-progress-controls.md)
