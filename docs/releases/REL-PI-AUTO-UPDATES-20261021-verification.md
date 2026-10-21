# Vérification indépendante — adoption legacy des mises à jour automatiques / Independent verification

- **Date:** 2026-10-21 (after activation)
- **Scope:** existing household Pi; server and web only, daily policy at 02:00 local time after the completed offline backup.
- **Reviewed source:** plan/template `50368f0`; signed runtime `a38d4b1`; report carrier `e9b8dfe`.
- **Verifier:** independent post-activation release verifier (read-only remote inspection; not the builder, deployer or security reviewer).
- **Verdict:** **APPROVED_SCOPED**.

## Evidence / Preuves

| Gate | Observed | Result |
| :--- | :--- | :--- |
| Exact signed runtime | Helper `b9d52238…d281`, service `a1874229…a52d`, timer `cc60e261…8b0b`, and adoption drop-in `ebfdf529…56db` match the approved pins. | PASS |
| Payload and slots | Running server uses the relative server slot and has the approved `cdde3ed0…cadea` payload. Server and web slots resolve to `0.7.2-3`; retained flat web and slot each contain 102 files and have the same complete-tree digest. Themes have no release/current payload. | PASS |
| Preservation/security | Protected baseline hashes pass. The original server config, vendor unit and `10`/`20` pre-existing drop-ins are byte-identical to the private pre-adoption archive. Effective service UID/GID, working directory, NAS read-only path/mount dependency, and AF_NETLINK/Bonjour allowance remain unchanged. Root configuration/trust roots have restrictive root ownership/modes. | PASS |
| Backup and data | Private receipt records the completed offline backup (`active/exited`, exit 0), 1,924 verified files and 29 logical tables. Before/after logical-table snapshots are byte-identical; an independent read-only transaction found all 29 snapshot table row counts unchanged. | PASS |
| Runtime smoke | Main service is active; timer is enabled/active and the update service is idle. Strict TLS with the root-owned CA copy succeeds for the dated updates API. The response reports server/web automatic at 02:00, themes manual, `busy=false`, `check_available=true`, and no active mutation. No current unit error-level journal entries were found. | PASS |
| First admission | Private helper inspection and explicit server/web checks record current `0.7.2`, sequence 3 and `no_update`; the bounded first tick completed without installation or archive replacement. The minute timer is only the engine wake-up; the next routine admission is 22 October at 02:00 Europe/Dublin. | PASS |
| Rollback readiness | Flat binary/web, pre-adoption archive, baseline slots, private backup and anti-rollback ledger are retained. Rollback remains the documented removal of only the new adoption drop-in while all updater work is quiescent; this verifier made no mutation. | PASS |

## Incidents and limits / Incidents et limites

- The first backup launcher failed closed because a blocking oneshot launch retained its own lock. Its evidence remains retained; the corrected `--no-block` job reached terminal success before any adoption mutation. See `BUG-0129`.
- The first attempted root ledger used schema 1. The helper rejected it before service/timer start; the refused file remains retained and the corrected schema-2 ledger preserves server/web highest sequence 3 with no pending transaction. This is not counted as a successful first attempt.
- Future installation of a higher version, rollback during an installation, power loss/network/disk failure, and physical web/PWA playback were **NOT_RUN**. A successful `NO_UPDATE` check does not prove those scenarios.

No NAS traversal, unrelated configuration change, helper check/tick, installation, restart, or remote write was performed by this verifier.
