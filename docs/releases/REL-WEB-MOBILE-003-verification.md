# REL-WEB-MOBILE-003 — vérification post-déploiement / post-deployment verification

**Verdict : APPROVED_SCOPED**

**Vérificateur :** instance indépendante, lecture seule

**Horodatage de vérification (UTC) :** `2026-10-17T19:31:50Z`

**Portée :** assets web R3 seulement ; aucune écriture de profil/bail, aucun stream, accès NAS, redémarrage ou changement de serveur/API/configuration/Mac.

## FR

### Identité et intégrité de la livraison

- Source déployée : `acd7aebfc3c9b902eb7b4b414a14750d86017e08`; build servi : `a15795b5c4cd`.
- Archive encore retenue sur la cible : SHA-256 `34a1663eda23e972ecef6e8b68ac7641432465f8d751eeaea0de8bffe9316bda`.
- Les 81 sommes du manifeste candidat sont toutes valides contre l'arborescence installée ; 82 fichiers web sont présents.
- Le WASM audio installé a le SHA-256 attendu `5075105bfdfcde924be0163a785a9d5fa0d2f1eb2c0d52aecf5ea571497d47d5`.

### Ressources et smoke tests, lecture seule

- HTTPS strict avec le certificat de confiance : page web, `build.json`, API datée et alias v1 répondent `200`; `build.json` annonce le build attendu.
- HTTP sur le port applicatif répond `200`. Aucune conclusion sur un alias HTTP par défaut n'est déduite d'une auto-sonde Pi.
- `mse-engine.js`, `mobile-playback.js` et le transcodeur audio sont servis `200`; WASM est `application/wasm` et JavaScript `text/javascript; charset=utf-8`.
- `/.env` renvoie `404`; une requête API avec marqueur cross-site renvoie `403`.
- Le document servi contient `viewport-fit=cover`.

### Sauvegarde, rollback et invariants

- Sauvegarde privée : `/root/chillcenter-rollback/predeploy-mobile-r3-20261017T193045Z`, permissions `0700`.
- `web-before.tar` : SHA-256 `2f3afc29058d5ab91a428d22a6bbae9b57b86073ed0289963bfe95fc411bf7cc`, 88 membres. Le hash de manifeste transmis est `47cc29dd349854baefca0d95e70f3ee0b8156019ddbb9eafd8aae1a6cfe84c1b`.
- Les sommes de sauvegarde valident l'ancien arbre retenu ; le rollback web est disponible et vérifié.
- Service actif, PID `5420`, zéro redémarrage. Binaire, configuration, unité, certificat et SPKI correspondent au baseline protégé fourni.
- Aucun warning/error/failure/panic trouvé dans le journal borné après le déploiement.

### Limites obligatoires

**DEVICE_RETEST_PENDING :** aucune validation physique iPhone/Safari/Vivaldi, `ManagedMediaSource`, son AVI/MKV, seek, pause ou plein écran natif n'est revendiquée. Un Xvid rendu sur canvas ne peut pas obtenir le plein écran vidéo natif lorsque Safari refuse le plein écran du panneau ; cette limitation reste explicitement attendue, sans faux état de succès.

## EN

### Release identity and integrity

- Deployed source: `acd7aebfc3c9b902eb7b4b414a14750d86017e08`; served build: `a15795b5c4cd`.
- Retained archive on the target: SHA-256 `34a1663eda23e972ecef6e8b68ac7641432465f8d751eeaea0de8bffe9316bda`.
- All 81 candidate-manifest checksums validate the installed tree; 82 web files are present.
- Installed audio WASM has the expected SHA-256 `5075105bfdfcde924be0163a785a9d5fa0d2f1eb2c0d52aecf5ea571497d47d5`.

### Read-only resources and smoke tests

- Strict HTTPS using the trusted certificate: web page, `build.json`, dated API and v1 alias return `200`; `build.json` reports the expected build.
- HTTP on the application port returns `200`. No conclusion about a default HTTP alias is inferred from a Pi-local probe.
- `mse-engine.js`, `mobile-playback.js`, and the audio transcoder are served with `200`; WASM is `application/wasm` and JavaScript is `text/javascript; charset=utf-8`.
- `/.env` returns `404`; an API request marked cross-site returns `403`.
- The served document contains `viewport-fit=cover`.

### Backup, rollback, and invariants

- Private backup: `/root/chillcenter-rollback/predeploy-mobile-r3-20261017T193045Z`, mode `0700`.
- `web-before.tar`: SHA-256 `2f3afc29058d5ab91a428d22a6bbae9b57b86073ed0289963bfe95fc411bf7cc`, 88 members. The supplied manifest hash is `47cc29dd349854baefca0d95e70f3ee0b8156019ddbb9eafd8aae1a6cfe84c1b`.
- Backup checksums validate the retained old tree; web rollback is available and verified.
- Service active, PID `5420`, zero restarts. Binary, configuration, unit, certificate, and SPKI match the supplied protected baseline.
- No warning/error/failure/panic was found in the bounded post-deployment journal.

### Mandatory limitations

**DEVICE_RETEST_PENDING:** no physical iPhone/Safari/Vivaldi validation, `ManagedMediaSource`, AVI/MKV audio, seeking, pause, or native fullscreen is claimed. A canvas-rendered Xvid cannot obtain native video fullscreen when Safari rejects panel fullscreen; that limitation remains explicit and no false-success state is claimed.
