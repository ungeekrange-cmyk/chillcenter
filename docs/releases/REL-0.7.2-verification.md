# REL-0.7.2 — independent release verification

**Status: APPROVED (public artifacts and deployed Pi server/web verified; stated scope limits apply).**

- **Environment:** GitHub release publication, then household Pi server/web.
- **Included work item:** BUG-0127.
- **Source artifact:** `a38d4b16616ebd6735fa4aab0b518e28d767079e`; integration carrier
  `5cf1bbe70f1dd4ee98d67ae691334377df265b73`.
- **Verifier:** independent `agent-release-verifier` instance, distinct from
  author, deployer, integration reviewer, and security/code/test reviewers.
- **Reviewed:** 2026-10-21 UTC (local immutable-artifact phase).

## Local immutable-artifact evidence

| Check | Expected | Observed | Result |
| --- | --- | --- | --- |
| v0.7.2 component tags | mac/server/web/umbrella peel to `a38d4b…` | All four local annotated tags peel to exactly `a38d4b…` | PASS |
| Component versions and source | mac/server/web `0.7.2`, source `a38d4b…` | Signed manifests contain those exact values | PASS |
| Delivery inventory | every `SHA256SUMS` entry matches | `shasum -a 256 -c SHA256SUMS`: 20/20 `OK` | PASS |
| Minisign trust | shared `release.pub` and all signed assets verify | `release.pub` SHA-256 `6a16e189…a05a8cc`; mac/server/web manifests, server installers, delivery provenance, checksum file, manifests and archives all verify | PASS |
| Themes preservation | v0.7.1 package and signature reused byte-for-byte | ZIP `22c3fbe4…dc1d47`, 9,064,104 bytes; signature `09c97503…72240`, 319 bytes; byte-identical to both retained v0.7.1 local public/delivery copies | PASS |
| Delivery DMG pair | v0.7.2 DMG and signature bound into delivery | DMG `1d80517b…b364f5`, 51,462,982 bytes; signature verified | PASS |

The local signed component assets to compare against GitHub are:

| Release | Assets expected from local signed inventory |
| --- | --- |
| `mac-v0.7.2` | `Chill-Center-0.7.2-macos-arm64.app.tar.gz`, its `.minisig`, `mac-manifest.json`, its `.minisig`, `release.pub` |
| `server-v0.7.2` | `chill-server`, `chill-update`, three units, three installers and their `.minisig` files, `server-manifest.json`, its `.minisig`, `release.pub` |
| `web-v0.7.2` | `Chill-Center-web-0.7.2.zip`, `web-manifest.json`, its `.minisig`, `release.pub` |
| `v0.7.2` | 22 delivery assets, including the Mac DMG/signature pair, server and web install archives/signatures, unchanged v0.7.1 themes pair, provenance, checksum inventory, all four manifests/signatures, and `release.pub` |

## GitHub draft inventory

The first two API calls failed with a connection refusal; after the release
state changed, one authorized read-only retry succeeded. At that point all
three component releases were drafts, not prereleases, and their entire remote
inventories matched the local signed inventories exactly by **asset name, byte
length, and GitHub SHA-256 digest**:

| Draft | Remote assets checked | Result |
| --- | ---: | --- |
| `mac-v0.7.2` | 7 (including the delivery DMG and its signature) | PASS |
| `server-v0.7.2` | 14 | PASS |
| `web-v0.7.2` | 4 | PASS |

After upload completion, the same mechanical comparison was re-run. All four
drafts remain `isDraft=true`, `isPrerelease=false`, and have complete matching
inventories:

| Draft | Assets compared | Result |
| --- | ---: | --- |
| `mac-v0.7.2` | 7, including the DMG/signature pair | PASS |
| `server-v0.7.2` | 14 | PASS |
| `web-v0.7.2` | 4 | PASS |
| `v0.7.2` | 22 | PASS |

For every listed asset, the comparison is exact on **name, byte length, and
GitHub-provided SHA-256 digest** against the local signed artifact inventory.
This approves the draft artifact gate only. It does not establish publication
or public-download proof.

## Public download proof

All four releases were subsequently observed as published, non-prerelease
releases: `mac-v0.7.2` at `2026-10-21T12:37:52Z`, `server-v0.7.2` at
`12:37:57Z`, `web-v0.7.2` at `12:38:00Z`, and umbrella `v0.7.2` at
`12:38:03Z`.

One fresh temporary download of the **22 umbrella assets** was made from the
public published release. Each downloaded file is byte-for-byte identical to
the local delivery file. In that independent public copy, `SHA256SUMS` reports
20/20 `OK`; Minisign validates `PROVENANCE.json`, `SHA256SUMS`, all four
component manifests, and all four delivery archives (including the Mac
DMG/signature pair). This opens the public-artifact gate for the authorized Pi
server/web operation only; it is not a claim that the Pi has been changed.

This verifier made no deployment, backup, configuration mutation, NAS access,
or GitHub mutation. Before the recovery failure, the Pi post-deployment checks
were `NOT_RUN`. The prepared post-deployment scope is only the declared
server/web pins, retained root-private backup and rollback evidence, protected
invariants, dated HTTPS API, and metadata-only manual update checks; it excludes
media playback, media/profile writes, and any NAS access.

## Pi deployment and recovery incident / Déploiement Pi et incident de récupération

**FR — verdict : `APPROVED`.** Un incident de transport pré-échange, dont le
code de sortie final n'a pas été observé, a
laissé un processus `chill-server --backup` actif en concurrence avec les
redémarrages systemd. Il a déclenché l'erreur « le service doit être arrêté ».
Le diagnostic indépendant en lecture seule confirme cette cause opérationnelle;
il ne s'agit pas d'une régression du binaire livré. Après achèvement naturel et
mise au repos du workflow propriétaire, la cible réellement active est saine : service
actif, binaire `cdde3ed0…b4cadea`, web `0.7.2` build `ad12a307191b` (101
fichiers), API datée disponible et catalogue répondant. Les empreintes et
permissions protégées (configuration, certificat, clé et drop-in réseau) sont
identiques au baseline. SQLite est intègre, schéma 3 et 2 496 médias.
Deux relevés consécutifs ont gardé le même PID et le même compteur de
redémarrages; le journal après le dernier démarrage réussi ne contient aucune
nouvelle erreur. Une comparaison canonique en lecture seule et compatible WAL
de 29 tables SQLite actuelles contre la sauvegarde a les mêmes tables, mêmes
comptages et la même empreinte.

La sauvegarde privée actuelle est valide : 1 924 entrées de manifeste ont été
recalculées sans écart; sa SQLite est intègre, schéma 3, 2 496 médias. La
préparation au rollback est aussi vérifiée : deux binaires anciens conservent
le SHA-256 `8cf14421…2491c`; l'arbre web `0.7.0`/`a1c12ba4a582` correspond
octet à octet à son ZIP conservé (99 fichiers, 0 écart). Le répertoire de
rollback est privé au superutilisateur. Le helper et son timer restent absents.

**EN — verdict: `APPROVED`.** A pre-swap transport incident, whose final exit
code was not observed, left an
active `chill-server --backup` process concurrent with systemd restarts and
caused the “service must be stopped” error. The independent read-only diagnosis
confirms an operational workflow race, not a regression in the released binary.
After natural completion and quiescence of the owner workflow, the actual
active target is healthy: running service, binary `cdde3ed0…b4cadea`, web
`0.7.2` build `ad12a307191b` (101 files), dated API and responsive catalogue.
Protected configuration/TLS/drop-in fingerprints and permissions match the
baseline. SQLite integrity is `ok`, schema is 3, and media count is 2,496.
Two consecutive samples retained the same PID and restart counter, and the
journal after the last successful start contains no new error. A WAL-aware,
read-only canonical comparison of all 29 current SQLite tables with the backup
has identical table sets, row counts, and fingerprint.

The current private backup validates all 1,924 manifest entries and its SQLite
also reports integrity `ok`, schema 3 and 2,496 media. Rollback readiness is
verified with two retained old-binary copies at SHA-256 `8cf14421…2491c` and a
retained 0.7.0 web tree byte-identical to its ZIP (99 files, zero mismatch).
The rollback directory is root-private. The helper service and timer remain
absent.

## Real critical-path smoke

Strict-TLS requests confirmed server `0.7.2`, API `2026-10-14`, and a responsive
catalogue page. The update status reports server/web `0.7.2`, themes `0.7.1`,
`check_available=true`, no helper, no mutation, not busy and not quiescing.
One authorized metadata-only check per server/web/themes completed with
`NO_UPDATE`; the repeated server check returned `cached=true`. The negative
install check returned HTTP 503 `UPDATE_HELPER_NOT_INSTALLED`, with no payload
or installation. No media, profile, NAS, configuration, installer, or playback
write was performed.

Native Mac GUI/network-consent and media-player journeys are **NOT_RUN** and
are not inferred from these headless Pi checks.

## Failure and rollback recommendation

- Rollback trigger met: **No** — current target health and retained rollback
  evidence pass. The interrupted-workflow incident is retained as operational
  evidence; it does not justify a rollback of the healthy current release.
- Recommendation: open the post-deployment verification gate. The work remains
  subject to the explicitly stated headless-scope limits above.
- Work-item state: **VERIFIED** — this independent `APPROVED` report opens the
  post-deployment gate; the release coordinator records that guarded transition.
