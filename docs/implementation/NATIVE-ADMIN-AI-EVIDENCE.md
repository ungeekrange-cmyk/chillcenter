# Administration et IA natives — preuves intermédiaires

2026-10-14 ; base intégrée `02f57d5`, application en cours d'implémentation. Ce rapport ne vaut ni revue indépendante, ni validation des133 AC, ni remise G-04.

## Archives

Vrai processus Tauri debug → commandes Rust allowlistées → serveur HTTPS épinglé `127.0.0.1:18546` → fichiers synthétiques uniquement. Résultat exécuté : `UI_QA_RESULT true native-Tauri-pinned-ZIP-export-preview-map-confirm-profile-invalidation`. Export asynchrone, téléchargement natif et publication locale sans écrasement ; aperçu et plan sans mutation ; correspondances injectives explicites ; import confirmé ; changement de profil invalide le plan. Le chemin de sélection est injecté uniquement dans le host debug de QA ; aucune prétention de test du dialogue système réel.

BUG-0022 : la correction de `Content-Length` a maintenant passé ce parcours réel en plus du test de route. Les codes d'erreur d'archives exposés sont des classes bornées, jamais des détails du transport.

## IA

Le host Mac conserve des réglages CLI locaux privés (`0600`, écriture atomique et fsync parent). Aucune clé ou configuration de fournisseur sur le Pi. CLI Codex/Claude installé et connecté par l'utilisateur ; aucun appel fournisseur réel effectué pour ces preuves. Bouton secondaire réduit, aucune dictée. Métadonnées du périmètre sélectionné et état du profil actif seulement ; alias temporaires avant envoi, jamais chemins/vidéos/affiches/autres profils. Une recherche ne peut produire de modifications. La proposition reste privée au serveur/profil courant, limitée, annulable avant confirmation ; confirmation ne promet pas un rollback après transmission.

180s bornent l'ensemble de la tâche native, y compris collecte et écriture stdin ; sortie fournisseur 2MiB, demande8000 caractères, contexte3000 médias/1MiB, application1MiB, un travail à la fois, proposition15min. La suppression du cache temporaire est limitée au dossier vide créé par le host : des fichiers inattendus du CLI ne sont jamais effacés récursivement.

Résultat exécuté : `UI_QA_RESULT true native-Tauri-AI-synthetic-preview-confirm-typed-apply-search-profile-isolation`, avec `scripts/fixtures/ai-cli.sh` exclusivement (aucun réseau). Parcours : réglages, proposition sans mutation, refus d'application non confirmée, application commune + note personnelle, recherche sans changement, changement de profil/refus d'ancien job et non-fuite de note. Restauration du titre de la fixture dans le finally du harnais ; profils QA restent synthétiques.

Commandes exécutées : build desktop offline/locked ; trois tests `ai_bridge::tests` PASS ; cinq `chill-domain ai::tests` PASS ; syntaxe JS et `node --test ui/connected/ai.test.js` PASS. Avant le delta IA : suites domaine32/client26/serveur57/player11/desktop9 PASS ; ces chiffres ne sont pas ceux de la future suite finale.

## Limites

NOT_RUN : fournisseur réel/connectivité/quotas/sign-in ; sélection OS d'archive/image ; migrations du foyer, matériel Pi/NAS/Wi-Fi. Les tests synthétiques ne démontrent pas la qualité des réponses d'un fournisseur ni ses garanties de confidentialité. Les CLI installés vérifiés via aide uniquement : Codex0.159.2, Claude2.1.285. Références primaires des commandes : [Codex CLI](https://learn.chatgpt.com/docs/cli/reference), [Claude CLI](https://code.claude.com/docs/en/cli-reference). Aucun fournisseur TypeSafe ajouté.
