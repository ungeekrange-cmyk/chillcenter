# BUG-0001 — fournisseur cryptographique ambigu au démarrage

- Date : 2026-10-13. Environnement : développement Mac ARM64, Rust 1.99.0, rustls 0.23.45, Axum Server 0.8.0. Aucun Pi/NAS concerné.
- Branche : `codex/implementation`, base `c8d2a4c` ; fichiers serveur nouveaux avant commit.
- Attendu : démarrage HTTPS local ; réel : panic `Could not automatically determine the process-level CryptoProvider` avant ouverture du port.
- Reproduction : `cargo run --offline -p chill-server -- --config /private/tmp/chillcenter-server-smoke.cSmWkb/config.json` ; sortie 101. Les tests de routeur sans vrai listener n'exerçaient pas cette initialisation.
- H1 : les features unifiées activent AWS-LC et Ring simultanément. Expérience : `cargo tree --offline -p chill-server -e features -i rustls`. Résultat : les deux apparaissent, AWS-LC via reqwest/Axum, Ring via la déclaration serveur. Cause confirmée par le message exact et le graphe ; aucune deuxième hypothèse.
- Correction : retirer le fournisseur Ring serveur non utilisé et sélectionner explicitement AWS-LC avant chargement rustls. Le client utilise son propre fournisseur ; la réunion des features du workspace ne doit pas reintroduire le panic.
- Régression ajoutée : chargement réel de `RustlsConfig` depuis une clé et un certificat générés. Re-test/smoke HTTPS à consigner dans les preuves de livraison.
- Rollback : revenir au commit parent du candidat ; fichiers vidéos et base utilisateur non touchés. Aucun contournement de validation TLS, aucune clé enregistrée dans Git/log.
- Référence primaire : [rustls CryptoProvider](https://docs.rs/rustls/latest/rustls/crypto/struct.CryptoProvider.html).
