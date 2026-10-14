# REL-UX-API-001 — interface Tauri et API 2026-10-14

Date2026-10-14. État : `LOCAL_DELIVERED / PI_DEPLOYMENT_BLOCKED`.

[Vérification indépendante](REL-UX-API-001-verification.md) : APPROVED_LOCAL / BLOCKED_PI, verdict global BLOCKED. Bundle Mac/source/manifestes/archive/TLS ancien et confinement vérifiés ; Pi toujours R7 actif, binaire nouveau seulement staged. Le nouveau parcours catalogue/RangeNAS n'est pas exécuté. Le checksum annexe contient le bon hash mais un ancien nom d'archive ; correction de cette seule métadonnée en cours, sans altérer l'archive ou son hash.

Source approuvée `9277ac4d51281b3e6d72475294b1e7822cb8e913`, carrier documentaire `d25744102c9b2ff3fe69669c482bd561b2b5dff4`. Déployeur distinct `/root/ux_api_delivery`. Promotion locale fast-forward uniquement sur `codex/player-streaming`, ancien HEAD5665 préservé dans l'historique ; aucun push ni écriture main. [Triple revue](../gates/G-DEV-CC-UX-API-001.md) APPROVED, pas auto-approbation du coordinateur.

## Livraison locale réalisée

- Interface CC-UX-002 raccordée, catalogue continu, navigation séries et collections, administration/TMDB/conseil et lecteur natif ; 289Rust/47Node/clippy, OpenAPI formel et deux vrais parcours natifs frais sans retry PASS.
- API canonique `/api/2026-10-14`, alias ancien `/api/v1`, [guide tiers](../api/INTEGRATION-FR.md), [OpenAPI3.1](../api/openapi-2026-10-14.json), 66routes/85opérations. L'alias conserve `api_version:1`, la base datée annonce la date. TLS/Range/SSE, concurrence, UUID/unités64bits et limites documentés.
- Nouveau bundle `artifacts/macos/Chill Center-9277ac4.app`,75Mio/48dylibsARM64, autonome libmpv/FFmpeg, signaturead-hoc. Exécutable SHA256 `c19a229be382597e644e0c5e6d12011ab3f7afe7c8ec4de73e0620f98a542c2c`. Ancien624 conservé ; pas d'installation /Applications ni migration AppSupport.
- Archive `artifacts/linux/chillcenter-server-9277ac4-pi64.tar.gz`, SHA256 `a202c94eaf616132eedb7fa8213d4e0c07638ea5d9c8ba44523974c22770552a`, binaire `1eb24e97bcdf8f66440c1681af92d6202c151ebe336c2890ea42a7f8179ac181`. Manifeste fidèlecargo-zigbuild/cibleglibc2.28/source9277 ; licence/docsAPI, sans cléTLS/config privée.

## Pi — bascule non réalisée

Le transfert du binaire vers `/var/tmp/chill-server-9277ac4.new` a réussi, hash vérifié. La protection d'exécution a ensuite **refusé** l'opération stop/sauvegarde/remplacement/restart et demande une nouvelle confirmation humaine explicite/informée. Aucun contournement ou rerun : service R7 toujours actif, ancien hash `9a17ffe55e3d4128b9d483d170f5d2ede71382cef42d8637917c8f31e63b36a6`, PID3016 au préflight. Ni configuration/unité/TLS/NAS ni profil foyer/données n'ont été modifiés par cette bascule non exécutée.

Le nouveau client exige le contrat daté : **ne pas le connecter au Pi avant sa mise à jour**. L'ancienne application continue de parler à l'ancien serveur. Le gate logiciel APPROVED n'est pas une preuve de déploiement Pi ni un gate post-déploiement.

## Actions soumises à confirmation

1. Vérifier version installée, connexion partagée stricte et absence de connexions/lecture actives.
2. Arrêter brièvement Chill Center ; sauvegarde privée offline sous UIDchillcenter, cwd`/usr/lib/chillcenter`, commande revue `--backup --config /etc/chillcenter/server.json`. Sauvegarde contenant des secrets gardée sur le Pi, jamais copiée/affichée.
3. Préserver R7 à `/root/chillcenter-rollback/chill-server-r7-9a17ffe55e3d`, puis remplacer atomiquement le seul binaire par le stage vérifié et redémarrer. Aucun changement NAS/config/unité/groupe/TLS/exposition réseau.
4. Vérification distincte TLS strict/version/date+v1/OpenAPIhash/service/identité/confinement, puis catalogue et RangeNAS via profil QA exclusivement temporaire et explicitement nettoyé. Le profil foyer existant doit rester intact.

Déclencheurs rollback : hash erroné, sauvegarde échouée, service/start/API critique en échec, régression de confinement. Préserver les preuves ; restaurer seulement l'ancien binaire et revenir à l'ancien client, sans reset de base ou NAS. Rollback opérationnel nécessitant les droits d'exécution applicables, puis vérification de restauration indépendante.

## Réserves maintenues

Pas d'appel réel TMDB/IA sans clé/utilisation explicitement fournie, pas de nouvelle mesure RSS du catalogue3000 dans la webview ou sous charge Pi, pas de validation humaine du son/veille/multi-écran/HDR. Les résultats physiques antérieurs deuxDune qualifiés et échecDune2àtroisflux ne sont pas améliorés par cette remise : [preuves historiques](REL-PI-001-media-evidence.md). API datée et catalogue/Range réels sur le nouveau Pi restent NOT_RUN jusqu'à bascule autorisée ; aucun133ACtousPASS affirmé.

Sources/exécutions : [preuves détaillées](../implementation/UX-API-EVIDENCE.md), [BUG0049](../debug/BUG-0049-native-qa-hidden-controls.md), [BUG0050](../debug/BUG-0050-native-qa-hidden-timers.md). Les rapports défavorables restent historiques, jamais remplacés par les derniers APPROVED.
