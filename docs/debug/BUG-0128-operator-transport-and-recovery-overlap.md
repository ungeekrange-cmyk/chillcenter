# BUG0128 — Long-running operator receipt and recovery overlap

Date: 2026-10-21. Priority: P1 before reusing the legacy deployment operator.
Status: FIXED / VERIFIED_ISOLATED (operational orchestration; released application code unaffected).

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

## Corrective work / Correctif demandé ensuite

The user subsequently explicitly requested this correction without Pi access or
published-version mutation. Branch `codex/bug0128-durable-operator`, base `0122c22`.
The original incident and unobserved transport exit above remain historical.

- Kernel lock shared by every operator mode and inherited by its full process
  tree; separate backup lease prevents its own EXIT recovery over an orphan.
- Root-private detached UUID job, code snapshots, transcript and atomic fsynced
  terminal receipt; metadata status does not infer success after observer loss.
- Retained/quarantined payloads use a transaction UUID, not a one-second
  timestamp. A frozen-clock regression exposed collision with a previous
  rollback tree; RED `recovery_verification_failed`, GREEN exact prior pair.
- Same root authority, explicit pins, backup, TLS/invariant guards; no installer,
  automatic cleanup, process-kill API, configuration change or new dependency.
- RED: old recovery returned0 while another owner held the operation lock.
  GREEN: rejects42 before any service action. Real Linux signal/orphan/partial
  swap recovery and loopback SSH-disconnect proofs accompany the regression.
- Initial Linux fixture failures were tooling (`unzip` absent) and a mock JSON
  escaping difference, corrected in the isolated fixture without weakening
  production checks. A closed-IO orphan fixture explicitly exercises the backup
  lease instead of merely waiting for inherited stdout EOF.

Final local matrix:20 Linux tests PASS (10 legacy/regression,10 real process/
resilience), real loopback SSH disconnect/reattach PASS. Bash/Python syntax PASS.
Tests use disposable local fixtures, not the Pi, NAS or full real-systemd stack.

Exact source `a6f0fc0c0c98ee1d4a4ecfb2236177fa157a2415`: independent
[code](../reviews/BUG-0128-code.md), [tests](../reviews/BUG-0128-tests.md) and
[security](../reviews/BUG-0128-security.md) reviews **APPROVED**, no open findings.
The follow-up carrier changes documentation only. This closes the isolated
correction, not a deployment or physical power-loss validation.

FR : correctif local seulement, aucune connexion Pi, aucun NAS, tag, release ou
paquet publié modifié. [ADR0033](../architecture/decisions/ADR-0033-durable-manual-operator.md)
et [guide FR/EN](../operations/LEGACY-DEPLOYMENT-JOBS.md) expliquent les gardes,
reconnexion, code réel et limites. Les revues indépendantes restent distinctes.
