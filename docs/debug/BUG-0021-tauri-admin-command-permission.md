# BUG-0021 — Permission Tauri manquante pour les commandes administration et image

2026-10-14 · auteur `/root/admin_contract_fix` · état : FIXED_PENDING_INDEPENDENT_REVIEW.

## Symptôme et reproduction

Environnement : macOS arm64, Tauri 2.12.3, branche `codex/admin-contract-fix`; fixture QA privée isolée et transport HTTPS local épinglé. Aucun NAS, sélecteur macOS réel, archive ni donnée du foyer n'a été utilisé.

Le parcours QA natif d'export en arrière-plan a produit `UI_QA_RESULT false native export background/pinned file save: "admin_command not allowed. Command not found"`. `admin_command` est déjà enregistré dans `tauri::generate_handler!`, mais l'invocation était refusée avant le gestionnaire Rust.

## Hypothèse et cause confirmée

H1, confirmée par lecture de la configuration : `apps/desktop/build.rs` déclarait seulement `ui_command`, `player_command` et `qa_result` dans l'AppManifest; `capabilities/main.json` n'accordait que les permissions `allow-*` correspondantes. Les handlers `admin_command` et `artwork_command` enregistrés dans `main.rs` ne produisaient donc pas leurs permissions de commande, et n'étaient pas autorisés dans la fenêtre locale `main`.

## Correction et régression

L'AppManifest déclare maintenant `artwork_command` et `admin_command`; la capacité `main` ajoute exactement `allow-artwork-command` et `allow-admin-command`. La portée reste la fenêtre locale `main`. Aucun plugin dialog, filesystem, réseau ou shell n'est ajouté : les sélecteurs d'archive/image restent initiés par Rust, et JavaScript ne fournit ni chemin ni URL.

Vérification exécutée sur le candidat : `cargo check --offline --locked -p chillcenter-desktop` passe avec le cache et target isolés. La génération Tauri de ce build accepte les noms de permissions ajoutés; un garde structurel confirme les deux commandes, les deux `allow-*` et l'absence de permission plugin filesystem/shell/network/dialog. Le parcours QA emploie une seam de fichier privé pour prouver l'IPC et le flux HTTPS; il ne constitue pas une preuve du sélecteur macOS réel.

## Retour et portée

Retour logiciel : revenir au commit parent du candidat retire ces deux autorisations de commande uniquement. Exigences concernées : F-17 et la frontière Webview → Rust de `docs/architecture/CONTRACTS.md`. Cette note ne constitue pas une approbation indépendante ni une certification du dialogue système.
