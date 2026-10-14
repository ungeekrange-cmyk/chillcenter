# G-POST-CC-UX-API-001 — bascule Pi et connexion client

Canonical verdict: APPROVED

2026-10-14, DEV_STANDARD. Le coordinateur consigne le verdict indépendant, sans auto-approbation. Source compilée immuable `9277ac4d51281b3e6d72475294b1e7822cb8e913`, [gate logiciel](G-DEV-CC-UX-API-001.md) et [revue opérateur V4](../reviews/pi-update-operator-9277ac4.md) APPROVED.

Autorité humaine renouvelée : arrêt du service, sauvegarde privée surPi, installation du seul nouveau binaire, redémarrage, NAS/configuration inchangés, puis connexion client. Déployeur distinct `/root/ux_api_delivery`, vérificateur distinct `/root/ux_api_post_verify`.

- Déployé19:12:40UTC, PID5145, binaire et `/proc/5145/exe` SHA256 `1eb24e97bcdf8f66440c1681af92d6202c151ebe336c2890ea42a7f8179ac181`, processus stable/NRestarts0/warnings0.
- Vérifié19:15:04→19:16:30UTC : [rapport indépendant APPROVED](../releases/REL-UX-API-001-pi-verification.md), TLS strict/SPKI conservé, API datée200, aliasv1/version1, OpenAPIhash exact, identité/confinement/config/unité/NASCIFSRO conservés.
- Sauvegarde privée fraîche `62584da8-ce12-45b6-9e1a-50e6accb5767`, parents0700, manifeste/tailles cohérents, SQLitequick_check=ok. Clé privée ni lue ni copiée par le vérificateur. R7 vérifié préservé pour rollback binaire seul ; aucun déclencheur de rollback.
- Connexion via le même `UiClient` Rust que Tauri surPi réel, catalogue48/has_more et réouverture PASS, propre dossier temporaire frais du vérificateur,0,99s/exit0, sans retry/skip/mutation serveur. Ce n'est pas un clic dans l'application Mac de l'utilisateur.

Le refus initial d'exécution et la première tentative opérateur échouée restent historiques dans les rapports et [BUG0051](../debug/BUG-0051-pi-update-output-quoting.md). Aucun fournisseur réel, nouvelle vidéo/RangeNAS, nouveau benchmarkWi-Fi/2–3flux ou son/veille/HDR/multi-écran/RSSmax humain validé. Les réserves des stories ne deviennent pas des PASS par ce gate de connexion. Pas de push/main/migration/reset ni de modification des artefacts compilés dans le carrier documentaire.
