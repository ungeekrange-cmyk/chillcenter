# BUG0129 — oneshot backup launch waits on its own lock

- Date:2026-10-21; operator `/root`, legacy updater adoption.
- Environment: household Debian13 Pi; no product source/dependency change.
- Branch: `codex/pi-automatic-updates`, plan50368f0; published runtime0.7.2.
- Status: **VERIFIED_OPERATIONAL**, corrected durable job completed.

## Symptom and bounded hypothesis / Symptôme et hypothèse

The first uniquely named backup unit failed with exit1 after its lock wait.
The transcript remained empty, so no application backup had started; the server
was already safely stopped and the maintenance marker retained. No component
slot, helper config, trust ledger or adoption drop-in had been installed.

H1: `systemd-run` without `--no-block` waits for oneshot startup completion;
the launching shell therefore retains the same lock the backup child needs.
The child uses a bounded30-second `flock` wait and fails closed rather than
starting concurrently. Confirmed by the first unit's terminal failure, empty
log, same-lock ownership and the local `systemd-run --help` semantics.

## Correction and evidence / Correction et preuves

Retain the failed unit/receipt. Verify it is terminal, server inactive, transcript
empty and lock now available; launch a **new** named unit with `--no-block`.
The launching shell then exits/releases its lock; PID1 owns the backup, private
log and completion independently of SSH. `RemainAfterExit=yes` keeps its terminal
success inspectable. Do not restart or recover while the new unit is activating.

The corrected launch returned immediately and the new unit entered
`activating/start` with a live lock owner. An intermediate `Result=success` or
status0 is **not** accepted as completion. The corrected job then reached
`active/exited`, PID0, `ExecMainCode=1` and status0. Receipt
`203657ec-b613-403c-bf8e-ff51a71cec59` was verified:1924 file hashes and29 SQLite
tables identical to the stopped original. No adoption step ran before that proof.

FR : incident de commande opérateur, pas un défaut du lecteur/serveur. Premier
échec conservé; relance non bloquante sous un identifiant neuf. Aucun faux
succès ni récupération concurrente. Retour possible au binaire plat conservé
après preuve que la sauvegarde est terminée; ne pas effacer de verrou actif.
