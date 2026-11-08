# Correctif de mise à jour Mac — 8 novembre 2026

Branche `codex/mac-updater-one-click`, base `978a4e9`. Demande utilisateur
explicite ; [ADR0039](../architecture/decisions/ADR-0039-mac-one-click-updates.md)
et [BUG0138](../debug/BUG-0138-mac-update-orphan-check.md).

Réutilisation du worktree `/private/tmp/chillcenter-playback-wake.BEMWYx/repo`,
du cache Cargo `/private/tmp/chillcenter-fixes-web-parity/target` et du Playwright
déjà présent dans le cache npm. Pas de nouvel environnement ni de purge.
Les copies QA générées `mac-update-sdk.*` sous le dossier de tâche servent
uniquement au remplacement natif isolé et sont conservées comme preuves.

La publication courante reste Mac/web 0.7.5 et serveur 0.7.4. Ce correctif n'est
pas encore empaqueté/publié et n'est pas installé dans `/Applications`.

## Contrôles

Contrôles exécutés :

- Suite Mac : 43 tests réussis ; les trois parcours opt-in sont exécutés séparément.
- JavaScript partagé/web : 369 tests réussis.
- Documentation FR/EN et composant de mise à jour : 35 contrôles réussis.
- Clippy Mac `--all-targets -- -D warnings` et `cargo fmt --all -- --check` passent.
- Chromium, composant Mac : démarrage et réponse serveur maintenus en attente,
  polling actif, affichage immédiat, aucune politique Mac, une installation
  pour les clics badge/carte, erreur persistante, reprise et badge périmé caché.
- Chromium, Configuration réelle : géométrie horizontale de l’icône près de
  Configuration, politiques des autres composants, navigation pendant une
  demande en attente, repli d’un serveur ancien. Données et IPC synthétiques.
- Test opt-in réseau : vraie admission détachée après récupération, résultat
  GitHub NO_UPDATE en environ 3,7 secondes ; aucune donnée familiale utilisée.
- Test opt-in SDK : paquet altéré refusé et arbre de bundle strictement conservé ;
  installation réelle du paquet public signé 0.7.5 dans une copie de 0.7.4 ;
  codesign profond strict et lancement du nouveau binaire/libmpv réussis.
  Preuve préservée : `mac-update-sdk.fVOySO/result.json` sous le dossier de tâche.
- Test opt-in de relance : copie du harness dans le bundle jetable, remplacement
  par le paquet signé, appel réel à `app.restart()`, nouveau PID observé sur le
  chemin exact du bundle remplacé et code de sortie 0 confirmé par kqueue
  NOTE_EXITSTATUS. Signature profonde toujours valide.
  Preuve : `mac-update-restart.DnUwEv/result.json` sous le dossier de tâche.

Le harness SDK opt-in `apps/desktop/tests/mac_updater_install.rs` utilise les API publiques
du SDK et le paquet Mac 0.7.5 déjà publié. Il remplace une copie QA de 0.7.4,
vérifie la conservation du bundle après corruption du paquet, puis exécute le
nouveau Mach-O avec `--verify-bundle`, qui sort avant l'ouverture des réglages.
Le parcours de relance démarre le harness lui-même depuis la copie `.app` afin
que `app.restart()` vise son vrai chemin après remplacement. L'Env géré par
Tauri est fixé avant construction à l'unique argument `--verify-bundle` ;
le binaire public relancé sort donc avant toute initialisation de données.
Le runtime mock permet le remplacement et la relance natifs sans interface
familiale. Les tests Chromium couvrent séparément l'interface et ses commandes ;
aucun succès de lecture familiale ou de déploiement utilisateur n'est revendiqué.

Commandes reproductibles depuis le worktree, avec `CARGO_TARGET_DIR` pointant
sur le cache réutilisé et `CHILL_PREVIEW_NODE_MODULES` sur le Playwright existant :

```sh
cargo test --locked --offline -p chillcenter-desktop
cargo test --locked --offline -p chillcenter-desktop live_check_recovers_restart -- --ignored --nocapture
CHILL_MAC_UPDATE_QA_ROOT=/private/tmp/chillcenter-playback-wake.BEMWYx cargo test --locked --offline -p chillcenter-desktop --test mac_updater_install -- --ignored --nocapture
node scripts/test-component-settings.cjs
node scripts/test-mac-updates.cjs
```

Le premier essai du harness SDK avait une configuration mock nulle et a été
refusé à l'initialisation du plugin, avant toute mutation. Il a été corrigé pour
charger la configuration publique exacte de l'application ; le parcours complet
est ensuite passé. Cette observation d'outillage n'est pas la cause du bug installé.
Les copies d'essai et le prélèvement de pile sont conservés sous le dossier de
tâche ; aucune source, livraison, donnée personnelle ni ancien paquet supprimé.
