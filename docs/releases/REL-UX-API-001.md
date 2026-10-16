# REL-UX-API-001 — interface Tauri et API 2026-10-14

Date2026-10-14. État : `LOCAL_DELIVERED / PI_DEPLOYED_VERIFIED`. [Vérification post-bascule indépendante](REL-UX-API-001-pi-verification.md) : **APPROVED** pour le périmètre DEV connexion/catalogue décrit, pas pour les exclusions matérielles/fournisseurs.

[Vérification historique avant bascule](REL-UX-API-001-verification.md) : APPROVED_LOCAL / BLOCKED_PI, conservée sans requalifier ses résultats. La confirmation humaine a depuis autorisé l'arrêt, la sauvegarde privée, le remplacement du seul binaire et le redémarrage. Le checksum annexe a été corrigé, `shasum -c` PASS, sans modifier l'archive ; l'ancien sidecar est conservé.

Source approuvée `9277ac4d51281b3e6d72475294b1e7822cb8e913`, carrier documentaire `d25744102c9b2ff3fe69669c482bd561b2b5dff4`. Déployeur distinct `/root/ux_api_delivery`. Promotion locale fast-forward uniquement sur `codex/player-streaming`, ancien HEAD5665 préservé dans l'historique ; aucun push ni écriture main. [Triple revue](../gates/G-DEV-CC-UX-API-001.md) APPROVED, pas auto-approbation du coordinateur.

## Livraison locale réalisée

- Interface CC-UX-002 raccordée, catalogue continu, navigation séries et collections, administration/TMDB/conseil et lecteur natif ; 289Rust/47Node/clippy, OpenAPI formel et deux vrais parcours natifs frais sans retry PASS.
- API canonique `/api/2026-10-14`, alias ancien `/api/v1`, [guide tiers](../api/INTEGRATION-FR.md), [OpenAPI3.1](../api/openapi-2026-10-14.json), 66routes/85opérations. L'alias conserve `api_version:1`, la base datée annonce la date. TLS/Range/SSE, concurrence, UUID/unités64bits et limites documentés.
- Nouveau bundle `artifacts/macos/Chill Center-9277ac4.app`,75Mio/48dylibsARM64, autonome libmpv/FFmpeg, signaturead-hoc. Exécutable SHA256 `c19a229be382597e644e0c5e6d12011ab3f7afe7c8ec4de73e0620f98a542c2c`. Ancien624 conservé ; pas d'installation /Applications ni migration AppSupport.
- Archive `artifacts/linux/chillcenter-server-9277ac4-pi64.tar.gz`, SHA256 `a202c94eaf616132eedb7fa8213d4e0c07638ea5d9c8ba44523974c22770552a`, binaire `1eb24e97bcdf8f66440c1681af92d6202c151ebe336c2890ea42a7f8179ac181`. Manifeste fidèlecargo-zigbuild/cibleglibc2.28/source9277 ; licence/docsAPI, sans cléTLS/config privée.

## Pi — bascule réalisée

Le premier refus d'exécution a été respecté. Après confirmation humaine renouvelée, une tentative a créé la sauvegarde privée mais échoué dans le quoting du parseur opérateur avant remplacement ; R7 a été immédiatement redémarré. [BUG0051](../debug/BUG-0051-pi-update-output-quoting.md) et [vérification de restauration](REL-UX-API-001-r7-restoration-verification.md) conservent cet échec et sa preuve, sans le présenter comme une bascule.

La commande V4 exacte `55c6be484d50a8065115ae4b1bf867377c24b255640c033e5fc2faf6f3289587` a reçu une [revue indépendante APPROVED](../reviews/pi-update-operator-9277ac4.md), puis a été exécutée par le déployeur distinct. À `2026-10-14T19:12:40Z`, Pi actif PID5145/NRestarts0, binaire `1eb24e97bcdf8f66440c1681af92d6202c151ebe336c2890ea42a7f8179ac181`. [Rapport opératoire](REL-UX-API-001-pi-deployment.md). Configuration, unité et SPKI public inchangés par comparaison pré/post ; NAS, identités et profils non modifiés.

Sauvegarde privée fraîche : `/var/lib/chillcenter/data/backups/62584da8-ce12-45b6-9e1a-50e6accb5767`, sur Pi uniquement, racine0700 UID997:GID983. R7 préservé à `/root/chillcenter-rollback/chill-server-r7-9a17ffe55e3d`, hash `9a17ffe55e3d4128b9d483d170f5d2ede71382cef42d8637917c8f31e63b36a6`. Ni copie de secrets vers le Mac ni restauration de données.

Sonde réelle depuis le Mac utilisant le même `UiClient` Rust que Tauri, réglages privés temporaires, aucun accès AppSupport réel ni écriture serveur : avant bascule HTTP404 ; après bascule `CLIENT_CONNECTED api=2026-10-14 profiles=1 tls_pin=verified`, `CLIENT_CATALOGUE_READ items=48 has_more=true`, `CLIENT_REOPEN_CONNECTED`, exit0. Ce n'est pas un clic dans la fenêtre native de l'utilisateur ni une nouvelle lecture vidéo/NAS. Le nouveau bundle peut désormais se connecter à `https://chillcenter.local:8443`.

Vérificateur distinct `/root/ux_api_post_verify`,19:15:04→19:16:30UTC : fichier et `/proc/5145/exe` exacts, PID stable/NRestarts0/warnings0, confinement/identité et NAS conservés ; sauvegarde privée/manifeste/tailles/SQLite `quick_check=ok`, TLS strict sans `-k`, API datée200, aliasv1/version1 et OpenAPI embarqué exacts. Sa propre sonde avec un nouveau dossier client privé termine connexion/catalogue48/reconnexion en0,99s/exit0 sans retry. [Rapport indépendant APPROVED](REL-UX-API-001-pi-verification.md), [gate post-déploiement](../gates/G-POST-CC-UX-API-001.md). Aucun déclencheur de rollback observé.

## Périmètre autorisé et exécuté

1. Vérifier version installée, connexion partagée stricte et absence de connexions/lecture actives.
2. Arrêter brièvement Chill Center ; sauvegarde privée offline sous UIDchillcenter, cwd`/usr/lib/chillcenter`, commande revue `--backup --config /etc/chillcenter/server.json`. Sauvegarde contenant des secrets gardée sur le Pi, jamais copiée/affichée.
3. Préserver R7 à `/root/chillcenter-rollback/chill-server-r7-9a17ffe55e3d`, puis remplacer atomiquement le seul binaire par le stage vérifié et redémarrer. Aucun changement NAS/config/unité/groupe/TLS/exposition réseau.
4. Vérification distincte TLS strict/version/date+v1/OpenAPIhash/service/identité/confinement, sauvegarde et connexion/catalogue/reconnexion client en lecture seule. Aucun profil QA, bail ou autre donnée du foyer créé. Pas de nouveau test Range/lecture NAS sous cette autorisation de connexion.

Déclencheurs rollback : hash erroné, sauvegarde échouée, service/start/API critique en échec, régression de confinement. Préserver les preuves ; restaurer seulement l'ancien binaire et revenir à l'ancien client, sans reset de base ou NAS. Rollback opérationnel nécessitant les droits d'exécution applicables, puis vérification de restauration indépendante.

## Réserves maintenues

Pas d'appel réel TMDB/IA sans clé/utilisation explicitement fournie, pas de nouvelle mesure RSS du catalogue3000 dans la webview ou sous charge Pi, pas de validation humaine du son/veille/multi-écran/HDR. Les résultats physiques antérieurs deuxDune qualifiés et échecDune2àtroisflux ne sont pas améliorés par cette remise : [preuves historiques](REL-PI-001-media-evidence.md). Catalogue client réel après bascule PASS ; nouveau Range/lecture NAS et nouvelle charge multi-vidéos NOT_RUN. Aucun133ACtousPASS affirmé.

Sources/exécutions : [preuves détaillées](../implementation/UX-API-EVIDENCE.md), [BUG0049](../debug/BUG-0049-native-qa-hidden-controls.md), [BUG0050](../debug/BUG-0050-native-qa-hidden-timers.md). Les rapports défavorables restent historiques, jamais remplacés par les derniers APPROVED.
