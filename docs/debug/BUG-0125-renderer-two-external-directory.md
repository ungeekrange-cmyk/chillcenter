# BUG0125 — Renderer2 external theme directory rejected

Date2026-10-21, author/root, reported independently by security reviewer,
status **FIXED_SOURCE / external-directory regression PASS**.

Candidate1cf7b42 accepted the new embedded theme but its external directory
walker omitted `assets/decorations`. A valid signed renderer2 pack would fail
the native install health check. The bundled-only test could not reveal it.
Both walkers also counted only128assets+4entries, not manifest+four directories.

Confirmed static cause. Permit only the exact new decorations directory;
adjust cap to133entries and reject unexpected directories in the shared validator.
The new actual-directory test writes the full pack with a distinct fixture
version, checks both validator and native health, loads the installed release
symlink and proves it did not fall back. An undeclared directory remains refused.
Four native theme tests PASS. No signature/path/PNG decoding control relaxed.

FR : défaut découvert avant publication ; premier candidat non livré. Test sur
le vrai dossier installé et son lien de release, pas sur le seul pack embarqué.
