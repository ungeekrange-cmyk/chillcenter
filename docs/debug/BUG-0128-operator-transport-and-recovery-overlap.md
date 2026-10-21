# BUG0128 — Long-running operator receipt and recovery overlap

Date: 2026-10-21. Priority: P1 before reusing the legacy deployment operator.
Status: OPEN (operational orchestration; released application code unaffected).

## Observation / Observation

During the authorized 0.7.2 Pi update, the local SSH execution returned/yielded
without a terminal stdout receipt or observable exit code. The remote offline
backup was still running. A separate `--recover` was invoked prematurely while
that original transaction remained in flight. The old service temporarily
auto-restarted and refused startup because the backup held its exclusive data
lock. Neither absent output nor an intermediate old-version hash established
that the remote operation had ended.

La sortie SSH absente a été interprétée trop tôt comme une interruption. Une
récupération a chevauché la sauvegarde toujours active ; le verrou de données a
alors correctement empêché le démarrage concurrent du service. L’ancienne paire
observée pendant cette fenêtre n’était pas un état final de restauration.

## Evidence and outcome / Preuves et résultat

- H1 confirmed independently: an actual `chill-server --backup` process and
  application startup error “Le service doit être arrêté avant cette opération”.
- The original transaction subsequently installed the exact authorized 0.7.2
  server/web pair. A later root read-only probe found the new server hash while
  a stale deployer receipt still described the old pair; that receipt was
  explicitly corrected, not treated as proof of restoration.
- No lock file was removed, no backup process killed, no second deployment
  launched, and no NAS/settings/TLS change authorized or performed.
- The final live state, backup and protected invariants require the distinct
  [release verification](../releases/REL-0.7.2-verification.md), not a reconstructed
  successful SSH exit code. Transport exit status remains **NOT_OBSERVED**.

## Required follow-up / Suite nécessaire

Before another use, give the remote transaction a durable private job identity,
transcript and terminal result that can be reattached/polled after a transport
yield/disconnect. Serialize deploy/recover with demonstrable ownership of the
operation, refusing recovery while its live transaction/backup is running.
Distinguish tool-cell completion, SSH completion and remote transaction
completion. Test a real disconnect during backup, and concurrent recovery, on
an isolated fixture before any new production operation. Never use removal of
the data lock or a server restart to bypass this protection.

Avant réutilisation : reçu durable, suivi du travail distant, exclusion mutuelle
déploiement/récupération et tests d’interruption réels. La mise à jour applicative
0.7.2 n’est pas la cause de cet incident ; aucun correctif d’orchestration ni
nouveau déploiement n’est implicitement revendiqué dans ce lot.
