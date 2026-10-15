# REL-LIBRARY-UX-SYNC-001 — Mac publié, Pi suspendu

2026-10-15, coordinateur `/root`, DEV_FAST corrections approuvées du foyer, sans nouvelle frontière cloud/auth/NAS/transcodage. Autorité : demandes de corriger/intégrer/bundler/déployer ; pas d’autorité supposée pour écraser un binaire Pi non attribué.

## Artefacts et périmètre

- Source `11aa529f2eced060156a3017456bf1d9634f8a56`, basea59f5e6, branchecodex/library-ux-sync-matching.
- Mac publié `artifacts/macos/Chill Center-11aa529.app`, exécutable SHA `d07bc889e077c1e2710caab01b66d9058941ee6ac11e5938e113667da31ec59d`. Signature ad-hoc/deep strict PASS,48dylibs ARM64 relocalisées, API libmpv2.5 chargée ; non notarisé. Anciennes versions et `.claude/` préservées, promotion ff-only, aucun push.
- Archive Linux publiée `artifacts/linux/chillcenter-server-11aa529-pi64.tar.gz`, SHA `3350b6a46a75910e069a536b57b95dacebbbd676a7c95d34b58a16ce30499f62`. Binaire ARM64 glibc2.28 SHA `de98776dd5d0d20c72b7770591c78375186026e283b73ae4c66fe21b82398eb0`, ELF vérifié. **Pas validé comme déployé sur Pi.**

BUG0064–0067 : réseau/publication miroir hors verrou UI, comparaison durable/delta serveur, fiches isolées/images actualisées sans fermeture, cinq étoiles, vignettes, matching admin, éditions/import série/saison, homonymes et bouton IA après sélection profil. API2026-10-14 additive/v1 conservé. IA réelle configurée/authentifiée et matching original/français automatique non prouvés par ce lot.

## Preuves

281 tests Rust des quatre bibliothèques concernées,41Node ciblés,23Desktop au checkpoint3bbb (bridge IA/artwork inchangés depuis), Clippy workspace/all-targets, contrat75chemins et navigateur synthétique PASS. Deux avertissements dead-code natifs existants en build release et un avertissement linker Zig non bloquants ; aucun nouveau test lecteur réel. Revues indépendantes exact11aa : UI APPROVED, séries APPROVED_SCOPED/ADR0015, synchronisation APPROVED_SCOPED/ADR0014 APPROVED ; refus historiques conservés. [Preuves](../implementation/LIBRARY-UX-SYNC-EVIDENCE.md).

## Pi et interruption

Première procédure arrêtée sans bascule prévue, service remis en ligne. Seconde refusée au préflight : fichier remplacé par un troisième hash. À15:57:04Z, processus/fichier SHA `a7ab7b07633c916c358f1656aa8f154b207185d5ac306c374d2a7a52b0c7d527`, PID9457 actif/running, HTTPS datée/v1 disponible, config/unité/SPKI inchangées. Provenance inconnue : ni11aa ni c610. Écritures suspendues, utilisateur interrogé sur un déploiement concurrent.

[BUG0068](../debug/BUG-0068-unattributed-pi-binary.md), [vérification indépendante BLOCKED](REL-LIBRARY-UX-SYNC-001-verification.md). Pas de smoke UiClient réel/delta, mesure RSS/latence Pi ou compatibilité du bundle avec ce serveur inconnu. La disponibilité HTTPS ne valide pas la release. Retour arrière/sauvegarde d’une future bascule à réétablir depuis la version réellement attribuée, sans écraser les archives. Aucune US globale déclarée terminée.
