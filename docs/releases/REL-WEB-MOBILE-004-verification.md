# REL-WEB-MOBILE-004 — vérification post-déploiement / post-deployment verification

**Verdict : APPROVED_SCOPED**

**Vérificateur :** instance indépendante, lecture seule

**Horodatage de vérification (UTC) :** `2026-10-17T20:51:07Z`

**Portée :** assets web R4 uniquement ; aucune écriture profil/bail, aucun stream ou accès NAS, aucun redémarrage, aucune modification serveur/API/configuration/Mac.

## FR

### Identité et intégrité

- Source déployée : `fa13d447abd4c35e9ba1f14f85ef05eaa2b25070`; build servi : `67773f7877d3`.
- Archive conservée sur la cible : SHA-256 `c139dca29fe63c5bb9ed016b8ac7f180b83f8f01475ff379be8eb298dba80182`.
- Les 81 entrées du manifeste candidat valident l'arborescence installée ; 82 fichiers web sont présents.

### Smoke web, lecture seule

- HTTPS strict : page, `build.json`, API datée et alias v1 répondent `200`; le build servi est celui attendu.
- HTTP sur le port applicatif répond `200`. L'alias HTTP par défaut est vérifié séparément depuis le LAN par le coordinateur ; aucune conclusion n'est tirée ici depuis la cible.
- `mobile-playback.js` et `mobile.css` sont servis avec les bons types MIME. Le délai `2500` est présent dans le script servi ; les insets gauche/droite sont présents dans la feuille CSS servie.
- WASM audio et MPEG-4 sont servis `application/wasm`.
- `/.env` renvoie `404`; une requête API marquée cross-site renvoie `403`.

### Sauvegarde, rollback et invariants

- Sauvegarde privée : `/root/chillcenter-rollback/predeploy-mobile-r4-20261017T204951Z`, permissions `0700`.
- `web-before.tar` : SHA-256 `37216843cfa433d3c3982a0a77b9f815dd974af3c3143552765cb0accb7270f2`, 88 membres ; hash de manifeste transmis `8019e0efe658c9dbd6b1c93113d7cce61ec1e17d7aea5fac72252437dbd37d54`.
- Les sommes de sauvegarde valident l'ancien arbre conservé : rollback web disponible et vérifié.
- Service actif, PID `5420`, zéro redémarrage. Binaire, configuration, unité, certificat et SPKI correspondent au baseline protégé.
- Aucun warning/error/failure/panic dans le journal de service borné après bascule.

### Limites obligatoires

**DEVICE_RETEST_PENDING :** Safari physique, comportement plein écran par codec/API et vraies zones caméra ne sont pas validés par ces sondes. Les cutouts réels et le rendu du bandeau doivent être retestés sur appareil. Un canvas/Xvid dont Safari refuse le plein écran du panneau reste limité à l'écran disponible : aucun plein écran forcé ou faux succès n'est revendiqué.

## EN

### Identity and integrity

- Deployed source: `fa13d447abd4c35e9ba1f14f85ef05eaa2b25070`; served build: `67773f7877d3`.
- Retained target archive: SHA-256 `c139dca29fe63c5bb9ed016b8ac7f180b83f8f01475ff379be8eb298dba80182`.
- All 81 candidate-manifest entries validate the installed tree; 82 web files are present.

### Read-only web smoke tests

- Strict HTTPS: page, `build.json`, dated API, and v1 alias return `200`; the served build is the expected one.
- HTTP on the application port returns `200`. The default HTTP alias is independently checked from the LAN by the coordinator; no conclusion here is inferred from the target.
- `mobile-playback.js` and `mobile.css` are served with correct MIME types. The `2500` timeout is present in the served script and left/right insets are present in the served CSS.
- Audio and MPEG-4 WASM are served as `application/wasm`.
- `/.env` returns `404`; an API request marked cross-site returns `403`.

### Backup, rollback, and invariants

- Private backup: `/root/chillcenter-rollback/predeploy-mobile-r4-20261017T204951Z`, mode `0700`.
- `web-before.tar`: SHA-256 `37216843cfa433d3c3982a0a77b9f815dd974af3c3143552765cb0accb7270f2`, 88 members; supplied manifest hash `8019e0efe658c9dbd6b1c93113d7cce61ec1e17d7aea5fac72252437dbd37d54`.
- Backup checksums validate the retained old tree: web rollback is available and verified.
- Service active, PID `5420`, zero restarts. Binary, configuration, unit, certificate, and SPKI match the protected baseline.
- No warning/error/failure/panic was found in the bounded post-switch service journal.

### Mandatory limitations

**DEVICE_RETEST_PENDING:** physical Safari, fullscreen behaviour by codec/API, and actual camera cutouts are not validated by these probes. Real cutouts and notice rendering require device retest. A canvas/Xvid for which Safari rejects panel fullscreen remains limited to the available screen: no forced fullscreen or false success is claimed.
