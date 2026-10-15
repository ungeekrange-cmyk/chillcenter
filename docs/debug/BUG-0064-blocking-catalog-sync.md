# BUG-0064 — Contrôle serveur bloquant et catalogue effacé

Auteur `/root`, 2026-10-15 ; base livrée c610ea9, candidat `codex/library-ux-sync-matching`.

Cause confirmée : `run` global, transfert sous verrou UiClient, `loadPage` vidant sa grille avant réception. Révision antérieure avec nonce de processus, transfert complet si modification.

Correction ADR-0014 : travail HTTPS détaché, delta et curseur persistants, publication atomique puis remplacement DOM. Les générations de mutation invalident les résultats périmés. Repli 404 vers ancien serveur.

Preuves locales : Node chargement atomique/contexte ; Rust delta/checkpoints/CAS ; `scripts/test-library-ux.cjs` conserve les affiches et utilise la recherche avec une synchro volontairement suspendue. Pi/NAS et nouvelle vérification Tauri : NOT_RUN à ce stade ; ne pas confondre avec c610ea9 livré.
