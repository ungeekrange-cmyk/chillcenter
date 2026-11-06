# BUG-0132 — Veille pendant la lecture vidéo

2026-11-06 · Branche `codex/playback-wake-lock`, base `bacee3c` · Correctif local.

Attendu : maintenir l'écran et la machine éveillés uniquement pendant la lecture
effective sur web/macOS. Pause, arrêt, fin, erreur, transfert et bail perdu doivent
relâcher le maintien éveillé, même si une requête réseau est encore en attente.

Constat source : aucun gestionnaire Wake Lock dans le lecteur web ; le rendu
macOS `vo=libmpv` n'installe pas d'assertion IOKit liée au cycle du lecteur.
L'option mpv screensaver dépend du backend de sortie et ne suffit pas à prouver
le comportement de cette surface OpenGL embarquée.

Hypothèse unique confirmée par inspection : absence de gestion explicite du
maintien éveillé dans les deux clients. Pas de changement des délais de veille OS.
Reproduction humaine de l'extinction après inactivité : NOT_RUN ; le signalement
utilisateur et la lacune source ne sont pas présentés comme cette mesure physique.

Correction : [ADR-0036](../architecture/decisions/ADR-0036-playback-only-wake-lock.md).
Les tests couvrent les états non actifs, les autorisations tardives après pause,
pause/reprise rapide, visibilité, fermeture de page, refus et absence de l'API,
et la libération avant les requêtes réseau retardées. Test IOKit réel : assertion
retrouvée dans le système pendant son acquisition, absente après pause/destruction.
Test libmpv réel avec vidéo/audio synthétiques : état initial inactif, lecture,
pause, reprise et EOF. Résultats finaux consignés après exécution ci-dessous.

Rollback : revenir sur le commit de ce correctif ; aucune migration ni donnée
persistante modifiée. Aucun paquet, déploiement Pi/web, installation Mac ou
publication GitHub n'est réalisé par ce lot. Pas de garantie sur HTTP LAN.

Espace de travail : `/private/tmp/chillcenter-playback-wake.BEMWYx/repo`, propriétaire
de cette tâche ; nouveau car aucun worktree existant propre compatible n'était
disponible. Conservé pour la livraison et ses preuves. Cache Cargo existant réutilisé
dans `/private/tmp/chillcenter-fixes-web-parity/target`, sans nouvel environnement
ni téléchargement. Les tests temporaires restent bornés au lot ; aucune purge.

## Vérification finale

- `node --test web/test/*.test.js ui/connected/*.test.js ui/connected/tests/*.test.js` :
  **347 PASS**, dont 13 régressions Wake Lock.
- `cargo check --locked --offline -p chillcenter-desktop --target-dir /private/tmp/chillcenter-fixes-web-parity/target` : PASS.
- `cargo test --locked --offline -p chillcenter-desktop -p chill-player --target-dir /private/tmp/chillcenter-fixes-web-parity/target` :
  **60 PASS** (20 lecteur, 40 desktop), dont IOKit réel et libmpv réel.
  Premier passage sandbox : 18 tests lecteur PASS, deux tests HTTP loopback
  existants refusés par le sandbox (`bind`, EPERM). Reprise hors sandbox :
  tous PASS ; aucune correction applicative pour contourner cette restriction.
- `cargo clippy --locked --offline -p chillcenter-desktop -p chill-player --target-dir /private/tmp/chillcenter-fixes-web-parity/target -- -D warnings` : PASS.
- `cargo fmt --all -- --check`, `git diff --check` : PASS.
- `node scripts/build-web.mjs ../web-build` : PASS, 102 fichiers,
  build `6073f1f890c6`.
- `git fetch origin` : base `origin/main` confirmée `bacee3c`, aucun écart
  avec la base locale avant ce correctif.

Contrôle ciblé du diff : aucune dépendance, authentification, autorisation,
redirection ou journalisation sensible ajoutée ; pas de permission serveur
élargie. Vérifications effectuées par l'auteur, sans revue indépendante.
Pas de test GUI assemblé, navigateur réel, attente physique de veille ni
test matériel macOS15/26 revendiqué. Statut **FIXED / VERIFIED_SCOPED**, local.

Intermédiaires conservés : `../web-build` (17 Mio), `../playback-wake-tests`
(1 Mio), journaux de ce lot ; supprimables après intégration sur autorisation
ciblée. Cache Cargo existant réutilisé : 4,1 Gio après vérification ; aucune
suppression effectuée, sources, historique et livrables conservés.
