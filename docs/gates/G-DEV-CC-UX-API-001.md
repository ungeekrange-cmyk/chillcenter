# G-DEV-CC-UX-API-001 — intégration interface et API datée

Canonical verdict: APPROVED

2026-10-14, DEV_STANDARD. Source immuable : `9277ac4d51281b3e6d72475294b1e7822cb8e913`, branche isolée `codex/cinefolio-ui-rework`, base utilisateur `5665f60bc59ceb3b2a9caa1622af053af0645d47`. Le coordinateur `/root` consigne les verdicts ; il n'approuve pas son propre code.

- Code/ADR : `/root/ux_api_code_review`, [rapport exact](../reviews/ux-api-code-9277ac4.md), APPROVED.
- Sécurité : `/root/ux_api_security_review`, [rapport exact](../reviews/ux-api-security-9277ac4.md), APPROVED.
- Tests/intégration : `/root/ux_api_test_review`, [rapport exact](../reviews/ux-api-test-9277ac4.md), APPROVED.
- Vérifications indépendantes : 289/289 Rust, 47/47 Node, clippy `-D warnings`, contrat 66 routes/85 opérations et validateur OpenAPI 3.1 formel PASS. Pas une revendication de 133 critères physiquement validés.
- Lecteur Tauri réel, paire fraîche sans retry/skip : A minimal 27 057 ms, B représentatif 26 075 ms, watchdog externe120s ; toutes phases visibilité→reopen PASS, plein écran AppKit/safe-area, pistes, seek, vitesse/volume, inactivité/focus et 10 secondes réelles finales avant vu. Les échecs précédents sont conservés ; toute récidive réouvre immédiatement le bug.

Artefacts compilés depuis cette source, avant le carrier documentaire :

- Mac : 75 Mio, 48 dylibs ARM64 autonomes, signature ad-hoc et libmpv API2.5 PASS. SHA256 exécutable `c19a229be382597e644e0c5e6d12011ab3f7afe7c8ec4de73e0620f98a542c2c`. Non notarized.
- Linux ARM64 : cible glibc2.28, Rust1.99/cargo-zigbuild0.23.4/Zig0.17 ; SHA256 binaire `1eb24e97bcdf8f66440c1681af92d6202c151ebe336c2890ea42a7f8179ac181` ; archive `a202c94eaf616132eedb7fa8213d4e0c07638ea5d9c8ba44523974c22770552a`.
- OpenAPI embarqué : `d12c606e4d922c1a52f0fe6f12f72940b1498c442dce7ba43ac9f83f946778f4`.

Promotion sérialisée par intégrateur distinct, serveur avant client. Carrier limité aux rapports/documentation/manifestes : aucun changement des sources compilées, API ou entrées de déploiement. Préserver anciens binaires/bundles et données, sauvegarde privée offline sur le Pi avant remplacement, configuration/TLS/NAS inchangés. Rollback serveur et client coordonné.

Ce gate permet la livraison mais ne confirme pas encore son exécution. Un vérificateur distinct doit contrôler le Pi réellement mis à jour. TMDB/IA réels sans clé fournie, son/veille/multi-écran/HDR/RSSmax humains et nouvelle charge Pi/NAS/Wi-Fi restent NOT_RUN ; réserves historiques sur trois flux inchangées.
