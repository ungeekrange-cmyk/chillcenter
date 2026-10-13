# BUG-0004 — type de configuration TLS non reconnu par reqwest

- 2026-10-13 ; Mac ARM64, Rust 1.99.0, reqwest0.13.5/rustls0.23.45 ; base `fe85cea`, branche `codex/interface-server`.
- Attendu : après consentement dans la vraie webview, connexion HTTPS et liste des profils. Réel : échec de construction du transport, erreur générique affichée, aucun profil créé.
- Reproduction : application debug avec serveur loopback19443 et données temporaires ; `UI_QA_RESULT false approve consent`. Probe isolé `cargo run -p chill-client --example transport_probe -- https://localhost:19443` : sortie1, `TRANSPORT_BUILD_FAILED … builder error`.
- Hypothèse unique : `use_preconfigured_tls` reçoit `Arc<ClientConfig>` alors que reqwest downcast `ClientConfig` par valeur. Lecture primaire du code verrouillé `reqwest-0.13.5/src/async_impl/client.rs:2223`, mécanisme confirmé et panne reproduite avant correction. Aucun contournement cryptographique essayé.
- Correction : utiliser `tls_backend_preconfigured(tls.as_ref().clone())` pour les clients async et bloquant ; même verifier pin/SAN/période/signatures conservé. Test de construction ajouté, aucun HTTP ni confiance préalable requis pour ce test.
- Régressions : test unitaire de construction, probe red→green et nouveau parcours réel interface/HTTPS à consigner dans les preuves du lot. Ni le NAS ni les données utilisateur ne sont utilisés.
- Rollback : parent du candidat, données de fixture indépendantes. Limite historique : les tests mémoire du lecteur ne prouvaient pas la construction du transport HTTP.
