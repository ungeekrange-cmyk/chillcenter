# REL-WEB-MOBILE-002 — vérification post-déploiement / post-deployment verification

**Verdict : APPROVED_SCOPED**

**Vérificateur :** instance indépendante, lecture seule

**Horodatage de vérification (UTC) :** `2026-10-17T18:08:14Z`

**Portée :** ressources web R2 seulement ; aucune écriture de profil ou de bail, aucun stream/accès NAS, aucun redémarrage, aucune modification serveur/API/configuration/Mac.

## FR

### Identité et contenu déployé

- Candidat vérifié : source `8b950334a17641aaa1a20e2129ccfe9a449b813a`, build servi `95ebee4fbe0a`.
- Archive retenue : SHA-256 `0da5d3962cbccabc966f71f1b9901aa61c4a2ef74508bb82ba1909694a1d7a50`, 6 628 864 octets ; 82 fichiers, manifeste de 81 entrées.
- Les 81 sommes du manifeste candidat valident l'arborescence web installée ; 82 fichiers sont présents. `build.json` servi confirme le build attendu.
- Les scripts `mse-engine.js` (MSE/MMS) et `mobile-playback.js` (retry) sont servis en `200`, JavaScript MIME correct.

### Sauvegarde et rollback

- Sauvegarde privée : `/root/chillcenter-rollback/predeploy-mobile-r2-20261017T180658Z`, permissions `0700`.
- `web-before.tar` : SHA-256 `6f34ad5f587ad0f24845027add7a23dd775d2becdc7e5ae968c0a61589bae8dd`, 88 membres ; manifeste de sauvegarde transmis `0abf26711c44fa553fc538b12548f24e932ebc7e4d21fec85c1aabe89c2670ad`.
- Les sommes du manifeste de sauvegarde valident l'ancien arbre retenu : rollback web disponible et vérifié.

### Smoke et sécurité, lecture seule

- HTTPS strict avec le certificat de confiance : page web, API datée et alias v1 répondent `200`.
- HTTP sur le port applicatif répond `200`. Le port HTTP par défaut est vérifié depuis le LAN séparément par le coordinateur ; aucune conclusion n'est tirée ici depuis une auto-sonde Pi.
- WASM est servi en `application/wasm`; JavaScript en `text/javascript; charset=utf-8`.
- `/.env` renvoie `404`; l'API avec marqueur cross-site renvoie `403`.
- La page servie contient `viewport-fit=cover`.

### Invariants

- Service actif, PID `5420`, zéro redémarrage.
- Binaire, configuration, unité, certificat et SPKI correspondent tous au baseline protégé fourni. Les valeurs sensibles ne sont pas dupliquées ici.
- Aucun warning/error/failure/panic dans le journal de service borné après la bascule.

### Limite obligatoire

**DEVICE_RETEST_PENDING :** le rendu et la lecture sur iPhone restent à retester physiquement, notamment le chemin `ManagedMediaSource`, le retry et AVI/MKV. La santé serveur et les ressources JavaScript servies ne prouvent pas le comportement WebKit natif.

## EN

### Deployed identity and content

- Verified candidate: source `8b950334a17641aaa1a20e2129ccfe9a449b813a`, served build `95ebee4fbe0a`.
- Retained archive: SHA-256 `0da5d3962cbccabc966f71f1b9901aa61c4a2ef74508bb82ba1909694a1d7a50`, 6,628,864 bytes; 82 files and an 81-entry manifest.
- All 81 candidate-manifest checksums validate the installed web tree; 82 files are present. Served `build.json` confirms the expected build.
- `mse-engine.js` (MSE/MMS) and `mobile-playback.js` (retry) are served with `200` and the correct JavaScript MIME type.

### Backup and rollback

- Private backup: `/root/chillcenter-rollback/predeploy-mobile-r2-20261017T180658Z`, mode `0700`.
- `web-before.tar`: SHA-256 `6f34ad5f587ad0f24845027add7a23dd775d2becdc7e5ae968c0a61589bae8dd`, 88 members; supplied backup-manifest hash `0abf26711c44fa553fc538b12548f24e932ebc7e4d21fec85c1aabe89c2670ad`.
- Backup-manifest checksums validate the retained old tree: web rollback is available and verified.

### Read-only smoke and security

- Strict HTTPS using the trusted certificate: web page, dated API, and v1 alias return `200`.
- HTTP on the application port returns `200`. The default HTTP port is checked separately from the LAN by the coordinator; no conclusion here is based on a Pi-local probe.
- WASM is served as `application/wasm`; JavaScript as `text/javascript; charset=utf-8`.
- `/.env` returns `404`; the API with a cross-site marker returns `403`.
- The served page contains `viewport-fit=cover`.

### Invariants

- Service active, PID `5420`, zero restarts.
- Binary, configuration, unit, certificate, and SPKI all match the supplied protected baseline. Sensitive values are not duplicated here.
- No warning/error/failure/panic was found in the bounded post-switch service journal.

### Mandatory limitation

**DEVICE_RETEST_PENDING:** physical iPhone rendering and playback remain to be retested, notably the `ManagedMediaSource`, retry, and AVI/MKV paths. Server health and served JavaScript do not prove native WebKit behaviour.
