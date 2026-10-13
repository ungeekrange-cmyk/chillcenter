# BUG-0003 — fixture déplacée lors de l'extraction du catalogue

- 2026-10-13, Mac ARM64/Rust 1.99.0, candidat `42ccc80`.
- Symptôme : `cargo test -p chill-catalog` ne compile pas, deux `include_bytes!("../icons/icon.png")` des tests référencent l'ancien emplacement de la crate monolithique.
- Hypothèse unique confirmée : fixture conservée à la racine `icons/icon.png`, crate extraite sous `crates/chill-catalog` ; le compilateur indique l'emplacement correct. Aucun défaut de lecture média impliqué.
- Correction `be83139` : seuls les deux chemins des tests deviennent `../../../icons/icon.png`, asset inchangé.
- Régression : 64 tests catalogue et suite totale 96 tests passent ; sortie 0. Statut VERIFIED. Rollback : parent du commit, aucune donnée utilisateur touchée.
