# Vérification indépendante du déploiement Pi — Chill Center v0.7

**Verdict / Verdict: `APPROVED_SCOPED`**

Le déploiement Pi v0.7 est approuvé dans le périmètre vérifié. L’exécutable
actif, les octets web installés, les routes servies en TLS strict, les deux
préfixes API, la base sémantique, la configuration, les préférences, le
certificat et la sauvegarde correspondent aux preuves avant/après attendues.
Le service est resté stable après le second déploiement. Le premier essai
échoué et son retour arrière vérifié sont conservés ci-dessous comme preuve
défavorable, et non requalifiés en succès.

The Pi v0.7 deployment is approved for the verified scope. The active binary,
installed web bytes, strictly validated TLS routes, both API prefixes, semantic
database state, configuration, preferences, certificate, and backup match the
expected before/after evidence. The service remained stable after the second
deployment. The failed first attempt and its independently verified rollback
remain adverse evidence below and are not relabelled as a pass.

- **Lane / mode:** `FULL_ASSURANCE`; test-evidence audit in
  `EXECUTION_REVIEW` mode
- **Immutable release source:**
  `3a043334c5c3c676251ab34bf8bab2e4bfb8df4f`
- **Successful deployment operator:**
  `815b523a383e3d0a61e1b116f27495e1ca2584d0`, SHA-256
  `4dd8451281a6a3037861fab15eccaff8623273b558ae167dadbad51e02ed6cfb`
- **Documentation carrier during verification:**
  `213e59f095f452e9e300a9fee9087289ef556539`
- **Deployment owner:** `/root`
- **Independent verifier:** `/root/pi_v07_post_verifier`
- **Final sample:** `2026-10-20T20:37:32Z`
- **Private sanitized baseline:**
  `/private/tmp/chillcenter-pi-v07-baseline-20261020T200955Z.log`, mode `0600`,
  final SHA-256
  `f4d9d3cd21d47f1cc82bcd71e7885f227b5166fc77de7a5d30a90feba49520f4`

No credential, token value, profile identifier/name, catalogue item, media path,
NAS address, or TLS private-key byte is included in this record. Secret-bearing
values were compared only through the private baseline.

## Séquence réelle / Actual deployment sequence

| Event | Evidence | Result |
| :--- | :--- | :---: |
| Baseline | Read-only capture at `2026-10-20T20:09:55Z`; old binary, web, unit, config, TLS, persistent files, schema and 29 privacy-preserving table digests recorded | PASS |
| First attempt | Operator `24f62ac2f4da95fd51bf01f81ab934b35762f888`, started `20:18:45Z`; new server/web became healthy, then a persistence guard incorrectly included runtime systemd PID/start-time properties and exited 1 | FAIL, retained |
| Automatic recovery | Recovery reported `PASS`; independent checks found the old server SHA, old web build, unit/config/TLS/token state and all 29 semantic database digests restored exactly; service stable with zero restart | PASS |
| Operator correction | Guard narrowed to static unit/config state in `815b523`; its fault fixtures and separate security review passed before retry | PASS |
| Second attempt | Started `20:26:34Z`; service started at `20:29:16Z`; operator exited 0 with `deployment=PASS` | PASS |
| Independent post-check | Exact runtime/web/API/data/backup/security and stability checks below | PASS |

The first attempt created a coherent private backup at
`/var/lib/chillcenter/data/backups/6554e936-1886-4c55-b80a-b33d1735112f`
before restoring the old pair. Its manifest contained the exact closed file set,
had zero hash mismatches, excluded live WAL/SHM files, and its SQLite copy passed
`quick_check` plus all 29 semantic digests.

An earlier TLS preflight also briefly created and removed an unintended
temporary root/run URL file. It left no durable residue and changed neither the
service, configuration nor NAS, but this report does not claim that the whole
operation made zero Pi writes.

## Exécutable, service et état persistant / Runtime, service and persistent state

| Check | Observed result | Result |
| :--- | :--- | :---: |
| Active executable | `/proc/107443/exe` resolves to `/usr/lib/chillcenter/chill-server`; active and installed SHA-256 both `68d526cd3f3af7cee9b07b06c62f46260656dda7cccc4c12994dee85431771e5`; CLI and API report `0.7.0` | PASS |
| Platform compatibility | `aarch64`, Linux `6.18.50+rpt-rpi-v8`, glibc `2.41` | PASS |
| Service stability | `active/running`, PID `107443`, `NRestarts=0`; final sample retained the same PID; zero `warning`-or-higher journal entries since the second attempt | PASS |
| Service confinement | Runs as `chillcenter`; writable path remains limited to the private data root; NAS remains read-only with `nosuid,nodev,noexec`; `PrivateTmp`, `PrivateDevices`, `ProtectHome`, `ProtectSystem` and `NoNewPrivileges` remain effective | PASS |
| Static configuration | Server config SHA-256 remains `d6167cb1624b6d5888473af76d5470102afedce25b2e29ddbaa032730cd0ceb1`; effective unit, vendor unit and drop-in hashes exactly match the baseline | PASS |
| TLS and private settings | Certificate, key and TMDB-token fingerprints match the private baseline; certificate validity, target SAN and SPKI remain valid; token/key bytes were never printed | PASS |
| Other persistent settings | Sync generation and `{}` TMDB job content match; `library.json` remains absent as before; normal job-file restart mtime change is not treated as content change | PASS |
| Database | Schema `3`, `quick_check=ok`; all 29 table counts/digests match before/after, including profiles, normalized preferences, personal state, collections and revisions | PASS |
| Catalogue invariant | `2,496` media rows, correcting the earlier informal `2,497` estimate; API revision digest remains `4f213205a134ba9cc14b2c45f4a09c227de913b50518dea1bfdfd4d0a3b77147` | PASS |
| NAS boundary | Only mount-policy and service-account read/execute metadata were inspected; no NAS walk, scan, enrichment or write occurred | PASS_SCOPED |

## Web, PWA, thèmes et TLS / Web, PWA, themes and TLS

| Check | Observed result | Result |
| :--- | :--- | :---: |
| Installed tree | `build.json` declares version `0.7.0`, build `a1c12ba4a582` and 98 payload files; 99 regular files including `build.json`; exact full-tree digest `2cf12b9151221d903d312386d47a7b0c6db475ceb345f95d33685fb8c6b484f7` | PASS |
| Served-byte inventory | Strict-TLS GET reproduced size and SHA-256 for all 97 routable files; served inventory digest `e5639fb9e2885817e615d7386f8fbf77bb62c46951ac47698d8edd55c79b6261` | PASS |
| Closed MIME boundary | The remaining two declared files are `vendor/audio/NOTICE-LGPL.md` and `vendor/mpeg4/NOTICE-LGPL.md`; both are installed and hash-verified but intentionally return 404 because `.md` is outside the closed MIME allowlist | PASS, expected deny |
| Headers and MIME | Exact CSP on HTML; `nosniff` and `no-referrer` on every routable file; correct HTML, JS, CSS, JSON, web-manifest, WASM, SVG and PNG types | PASS |
| Cache and guards | `index.html`, `sw.js`, `manifest.webmanifest`, `build.json` and `themes/manifest.json` are `no-cache`; `.env` and traversal return 404, hostile browser origins 403 `BROWSER_ORIGIN_REFUSED`, static POST 405 | PASS |
| PWA | Manifest, icons, standalone start URL, service worker and build-pinned index/service-worker references are present and coherent | PASS_SCOPED |
| Themes | Fallback theme version `0.7.0`; all 15 declared assets match exact size and SHA-256 | PASS |
| External TLS | Independent client requests on ports `443` and `8443` both returned server `0.7.0`, API `2026-10-14`, web `0.7.0/a1c12ba4a582` using the configured CA certificate; no `-k`, insecure mode or validation bypass | PASS |

## API et état des mises à jour / API and update state

| Check | Observed result | Result |
| :--- | :--- | :---: |
| Versioned server contract | `/api/2026-10-14` reports API `2026-10-14`; `/api/v1` reports alias `1`; both advertise the dated API and `v1`, return server `0.7.0` and header `X-Chill-Api-Version: 2026-10-14` | PASS |
| Profiles and revision | Both prefixes return exactly one unchanged profile; profile bodies and revision values agree; revision response contains only `revision` and its private value digest matches baseline | PASS |
| Catalogue | Dated API returned 2,496 unique items, zero duplicates, in 25 bounded pages of at most 100; every response has `items`, `has_more`, and object-valued `series_counters`; first `v1` page is byte-semantically equal | PASS |
| Facets | Category, year and genre values derived privately from returned items each produced a non-empty page whose every item satisfies the facet; dated and `v1` results agree | PASS |
| OpenAPI | Both prefixes serve OpenAPI `3.1.0`, 80 paths, exact SHA-256 `881fe77c0f8d93ef57c7bcdc5f55c6f400d7f5af225c787771a752cc285b7a44` | PASS |
| Background work | `GET /admin/scan` reports `running=false`; no scan, TMDB job, enrichment, playback lease or mutation was started | PASS |
| Update status | The one authorized `GET /admin/updates` reports installed server/web/themes `0.7.0`, web build `a1c12ba4a582`, API `2026-10-14`, `busy=false`, `quiescing=false`, zero active mutations/operations and no available update | PASS_SCOPED |
| Update helper boundary | `helper_available=false`; `/etc/chillcenter/updates.json`, helper unit and timer remain absent by design for this lot. No check/install/policy request was sent | EXPECTED LIMITATION |

The update-status GET initialized normal local state because no updater state
previously existed. It created only the private `updates/` directory (`0700`,
service account) and an empty `state.lock` (`0600`, SHA-256 of the empty file);
it did not create an available update, operation or policy override. This
authorized side effect is recorded rather than hidden.

## Sauvegarde et retour arrière / Backup and rollback readiness

The successful attempt created
`/var/lib/chillcenter/data/backups/4cb8e869-7990-4e35-8311-b2ded8d4bd74`
with private mode/ownership. Its format-1/schema-3 manifest SHA-256 is
`3d760181dcbd50437e4a629c0e9798315978154d0fba7655a5b3d44a20ea3a4a`:
all 1,924 declared entries match the exact closed set, size and hash; the only
additional regular file is the manifest itself. WAL/SHM files are excluded,
config/certificate/key fingerprints match baseline, SQLite `quick_check` is
clean, and all 29 backup-table digests equal the live baseline.

Rollback material is retained at:

- old binary:
  `/root/chillcenter-rollback/chill-server-20261020T202641Z`, exact old SHA-256
  `6fecabc69258e372d71730a23451f5ff47565f60cbbc352552c12d8ac39424ea`;
- old web:
  `/root/chillcenter-rollback/web-20261020T202641Z`, build `67773f7877d3`,
  82 files, exact prior-tree digest
  `64848e85ee19337fc9a77c6f47769c068a1bf517dd5750c01ba9e9ab5ea4cc8a`;
- successful pre/post fingerprint:
  `/root/chillcenter-rollback/pre-v07.JJY3Uq`, identical before/after static
  digest `49d0a19769e1d0c03ebbd89d665878c0ad7bc98443b3b85eab8c823a0bb96432`.

The rollback root is private (`0700`, root-owned), the backup and old pair are
coherent, and the final free-space sample was about 8.44 GB. No rollback trigger
is met.

## Revue des preuves et limites / Evidence review and limits

The verifier inspected meaningful assertions and exact observed values, not
only headline pass counts. The successful operator is distinct from this
verifier; its corrected failure paths had separate review and seven focused
fixtures before use. Existing public runtime/integration approvals remain
applicable because the later carrier is documentation-only. No broad suite was
rerun, and no skipped or missing check is counted as a pass.

Not run and not claimed:

- physical Safari use, PWA installation, background/offline browser behavior;
- playback acquisition, media `Range`, full player loop or persistent playback
  session;
- NAS content traversal, new scan, TMDB enrichment or write;
- Mac installation/change, GitHub asset/tag change, power-loss or disk-full
  recovery;
- automatic component update execution, because the separate root helper,
  timer and configuration are intentionally not installed in this lot.

**Execution-review verdict: `APPROVED`.** The direct deployed evidence is
current, exact-source-bound and internally consistent. **Release-verification
verdict: `APPROVED_SCOPED`.** The scope limitation is the explicitly untested
physical browser/playback experience and unavailable update helper, not a known
critical product defect.
