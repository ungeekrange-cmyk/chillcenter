# Pi — automatic server/web updates / Mises à jour automatiques serveur/web

2026-10-21. **Completed / Terminé**, independently verified **APPROVED_SCOPED**.
This record covers the explicit household Pi authorization, not a new release.

## Résultat

Le serveur et le client web restent **0.7.2**, avec recherche et installation
automatiques à **02:00, heure locale du Pi (Europe/Dublin)**. Prochain passage
quotidien : **22 octobre 2026 à02:00 locale**. Le premier contrôle réel GitHub
et le premier tick ont conclu `no_update` : aucun téléchargement d'archive ni
remplacement de version. Le réveil systemd chaque minute sert au moteur de
planification; il ne provoque pas une recherche GitHub chaque minute.

Avant activation : sauvegarde privée hors ligne terminée, manifeste et empreintes
des **1924 fichiers** vérifiés; contenu logique des **29 tables** identique avant
et après adoption. Les mêmes octets signés ont été copiés dans des slots
versionnés, en conservant binaire/web plats et unité d'origine pour retour arrière.
Les réglages d'origine, NAS, certificats, comptes/groupes, protections systemd,
Bonjour et cron de maintenance OS dimanche03:00 sont préservés.
Thèmes **0.7.1** conservés; leur installation automatique n'est pas activée.

Les liens README FR/EN étaient restés en0.7.1 malgré les paquets0.7.2 :
correction publiée sur main dans `7644d1d`, avec test de cohérence par composant.
Aucun paquet, tag ou release existant n'a été réécrit; aucun bundle Mac reconstruit.

## Immutable inputs and verification

- Application runtime: existing signed v0.7.2, source `a38d4b1`.
- Adoption plan and optional drop-in: `50368f0`; independent security approval
  recorded in `e9b8dfe` and [the security review](../reviews/AUTO-UPDATES-legacy-security.md).
- Server raw SHA-256: `cdde3ed064b79258ac17de3734c6750dd844e0f150bfcad8652f9d7e9b4cadea`.
- Helper raw SHA-256: `b9d522381e5afe93ffdfb3beefecb5de1c0455df48dcbe0b7581ebbc99dfd281`.
- Drop-in SHA-256: `ebfdf529226ba5c2a9f89a4f1f5c79f3c0014cb0801634b9ff9dc98b90ca56db`.
- Full signed unit/archive pins and procedure:
  [bilingual operations guide](../operations/ENABLE-LEGACY-AUTO-UPDATES.md).

| Check | Result |
| --- | --- |
| `node --test scripts/test-legacy-update-slot-dropin.mjs scripts/test-bilingual-docs.mjs` | 25 PASS |
| Independent security review: `cargo test -p chill-updates --locked` | 26 PASS |
| Real `systemd-analyze verify` for application/update service/timer | PASS |
| Signed helper `inspect`, explicit server/web `check`, initial `tick` | PASS; current0.7.2/sequence3, no install |
| Strict TLS dated API, active service, enabled/active persistent timer | PASS |
| Original protected hashes; all102 flat/slot web files;29 logical tables | Unchanged |
| Independent read-only live verification | [APPROVED_SCOPED](REL-PI-AUTO-UPDATES-20261021-verification.md) |

The root helper uses the existing public release key and strict HTTPS health
trust. The new schema-2 anti-rollback ledger starts at the independently verified
server/web sequence3 with no pending transaction. No existing acceptance floor
was lowered. Policy/config state uses schema1; these schemas are distinct.
The signed helper's internal CLI version0.1.0 is not the application version.

## Private receipts, incidents and recovery

Root-private evidence: `/var/backups/chillcenter-auto-updates.N2swYLcW`.
Verified application backup UUID: `203657ec-b613-403c-bf8e-ff51a71cec59` under
the existing data backup directory. Private config, certificate keys, household
address, credentials and database contents are not published here.

- [BUG0129](../debug/BUG-0129-oneshot-backup-launch-wait.md): first oneshot
  launcher held the lock while waiting for its child; failed before backup copy.
  Detached `--no-block` retry reached terminal exit0 and passed full verification
  before adoption. Failed receipt remains retained.
- [BUG0130](../debug/BUG-0130-adoption-trust-schema.md): helper rejected the new
  ledger's wrong schema1 before service/timer activation. Only the schema tag
  was atomically corrected to2; sequence floors remained3. Refused input retained.

Rollback requires quiescent updater/backup jobs, disabling scheduling and removing
only the new `40-legacy-update-slots.conf` drop-in, then daemon-reload/start of
the retained flat binary and original unit. Retain all evidence, slots, backup
and root acceptance ledger; never reset trust to permit replay. Follow the guide
before any operation; no rollback was necessary after successful activation.

## English summary and limits

Automatic **server and web** updates are enabled at **02:00 Pi local time**, after
a verified coherent backup. Existing signed0.7.2 payloads remain installed;
initial real checks found no newer version. NAS and unrelated settings are
unchanged. The original flat installation and private backup remain available
for recovery. Themes remain0.7.1 and are excluded from automatic installation.
README download links now point to the independently versioned current packages.

Future installation of a higher version, failure during installation, rollback
during an update, power/network/disk failure and physical web/PWA playback were
**NOT_RUN** here. Successful `no_update` and timer checks do not prove those
scenarios. The earlier BUG0128 isolated interruption evidence remains separate.
