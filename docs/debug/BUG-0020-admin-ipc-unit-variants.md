# BUG-0020 — Les commandes IPC unitaires acceptaient des champs inconnus

2026-10-14 · auteur `/root/admin_contract_fix` · état : FIXED_PENDING_INDEPENDENT_REVIEW.

## Symptôme et reproduction

Environnement : macOS arm64, Rust 1.99.0, branche `codex/admin-contract-fix`, base `c4b50fa4d4f22127d1279d00e4d03667127577b5`; aucun NAS, fichier vidéo ou donnée du foyer n'a été utilisé.

Le contrat IPC fermé attend que les commandes d'administration n'acceptent que leurs champs déclarés. La régression ciblée suivante était RED sur la base :

```text
CARGO_HOME=/private/tmp/video-catalog-cargo RUSTUP_HOME=/private/tmp/video-catalog-rustup CARGO_TARGET_DIR=/private/tmp/chillcenter-admin-contract-target PATH=/private/tmp/video-catalog-cargo/bin:/opt/homebrew/bin:/usr/bin:/bin cargo test --offline --locked -p chillcenter-desktop commands_reject_paths_urls_and_unknown_fields
```

Résultat réel : `{"action":"import","path":"/tmp/file"}` était désérialisé avec succès malgré `deny_unknown_fields`; le test a échoué sur cette assertion. Aucun chemin n'a été ouvert, lu, écrit ou utilisé par cette reproduction.

## Hypothèse et cause confirmée

1. Hypothèse confirmée par la reproduction : avec serde, les variants unitaires (`Import`, `Export`) d'un enum interne taggé ignorent les clés supplémentaires; `deny_unknown_fields` ne ferme donc pas leur forme JSON.

## Correction

`AdminCommand::Import` et `AdminCommand::Export` sont devenus des variants struct vides (`Import {}`, `Export {}`). La représentation d'appel valide reste inchangée : `{"action":"import"}` et `{"action":"export"}`. Les motifs de `main.rs` ont été adaptés sans modifier les autres commandes ni le comportement de sélection de fichier.

La régression couvre désormais le champ inconnu `path` sur import, `url` sur export et save, ainsi que la désérialisation des deux appels valides. GREEN : la commande ciblée ci-dessus passe (1/1). `cargo check --offline --locked -p chillcenter-desktop` passe également sur le candidat, avec le même environnement isolé.

## Contrat, retour et limites

Contrat proposé au coordinateur : les actions IPC `import` et `export` sont des objets fermés ne contenant que `action`; toute clé supplémentaire est rejetée avant traitement. Les JSON valides existants restent identiques.

Retour logiciel : revenir au commit parent du candidat retire uniquement ce durcissement de désérialisation. Ce document ne constitue pas une approbation indépendante ni une affirmation de sécurité générale; il décrit uniquement le comportement de désérialisation testé.

Exigences concernées : F-17, et la contrainte d'administration sans navigation arbitraire de `docs/architecture/CONTRACTS.md`. ADR concerné : ADR-0009.
