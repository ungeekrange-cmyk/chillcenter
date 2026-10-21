# ADR0034 — macOS 15+ media build / Chaîne multimédia macOS15+

- Status: PROPOSED implementation detail; user explicitly approved macOS15+.
- Date:2026-10-21; author/owner `/root`; independent reviewer PENDING.
- Branch `codex/macos15-build`, base `e8d6f53`; supersedes NONE; complements ADR0004.
- Mode: scoped local build, no public release, Pi, web deployment or data migration.

## Context / Contexte

Published0.7.2 has a Rust13 executable but34 bundled libraries require27
([BUG0117](../../debug/BUG-0117-macos-bundle-minimum-os.md)). User approved the
target15+ (15Sequoia,26Tahoe,27). This does not prove execution on older systems.
Changing only plist/Mach-O minima would be a false compatibility claim.

## Options and decision / Options et décision

- Keep installed Homebrew inputs: rejected as default, automatically raises OS.
- Rebuild the complete codec chain from source: possible, larger build/maintenance.
- **Pinned official Homebrew ARM64 Sequoia bottles**, isolated from installed
  Homebrew: selected. Same provider/toolchain family, no local brew replacement.

Lock exact versions, SHA-256, official content-addressed URLs and license/source
metadata in `deploy/macos/media-sequoia.lock.json`. Download over HTTPS; verify
hashes before bounded extraction of dylibs/notices only. Never execute a recipe
or bottle binary during extraction. Reject traversal, special files, duplicate
names and escaping links. Resolve the transitive closure exclusively inside the
isolated Cellar; never fall back to newer installed libraries. Preserve notices.
Rust deployment target, Tauri and source plist agree on15.0. Inspect every Mach-O
before signing; reject any deployment minimum >15.0. SDK27 is not a minimum.
Keep ad hoc signing and absence of Apple notarization explicit.

## Consequences, proof, rollback

No new engine, API, signing key or paid service. Build inputs require network and
space; isolated input receipt detects changed files on reuse. Local Homebrew and
published releases remain untouched. New candidate bundle/DMG use new paths;
rollback is the retained old application, not edits to a signed public asset.
Tests cover extraction/resolution/cache tampering, numeric OS bounds, exclusive
publication and signing/load. All-library51-Mach-O probe at15.0 and libmpv2.5 load
on currentmacOS27 PASS; final bundle and physical15/26 execution remain separate
gates. Physical old-OS tests may be NOT_RUN, never inferred from vtool alone.

Primary sources checked2026-10-21: [Homebrew mpv](https://formulae.brew.sh/formula/mpv),
[bottle format](https://docs.brew.sh/Bottles). Review/source/build receipts belong
to the exact delivery candidate, not this design's initial observation.

FR : cible validée, pas compatibilité physique certifiée. Bibliothèques Sequoia
figées et isolées; refus au-delà15, aucune falsification de minimum, retour
possible au bundle publié inchangé. Publication d'une nouvelle version distincte
nécessaire avant distribution automatique; aucune release réécrite ici.
