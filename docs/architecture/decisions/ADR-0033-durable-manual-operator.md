# ADR0033 — Durable manual ownership / Suivi durable de l'opérateur

Date: 2026-10-21. Status: IMPLEMENTED_DETAIL, user-requested BUG0128 correction.
Author: `/root`. Supersedes: NONE; complements manual deployment, not ADR0029's
signed updater contract. No new remote API or privileged role.

## Context and decision / Contexte et décision

The0.7.2 incident proved that SSH observer completion is not remote transaction
completion. A backup can survive while a separate recovery incorrectly restarts
the service over its live data lock. A PID or transition journal is not mutation
authority.

Keep the root-only, payload-pinned Bash operator and its existing backup, TLS,
health and invariant guards. Add a stable private kernel lock inherited by all
descendants. A separate backup lease also protects the operator's own EXIT
recovery trap. Never unlink these inodes. Concurrent modes refuse before any
service action.
Retained/quarantined payload names use a transaction UUID instead of wall-clock
seconds, avoiding aliasing of rapid retries and preserving prior rollback pairs.

A Python3 standard-library launcher reserves that same open-file-description
lock before acknowledging a UUID chosen before SSH. It snapshots reviewed code,
detaches IO/session, and writes a private transcript plus fsynced atomic state
and actual terminal exit code. Supervisor loss is uncertainty, never inferred
success/failure. Linux boot/start identity prevents PID reuse. No automatic
cancellation/recovery, new installer, timer or configuration writer is added.

FR : verrou conservé par les descendants, reçu durable et journal privé,
incertitude explicite sans superviseur, récupération sérialisée. L'autorité
reste l'administrateur root déjà approuvé ; aucun NAS, client, API ou paquet
publié n'est modifié.

## Consequences and proof / Conséquences et preuves

Util-linux `flock` is a prerequisite, refused before stop if absent. Python3
already existed in the operator stack; no third-party package is added. Private
per-job evidence is retained, not silently pruned. A bounded orphan may delay
recovery; availability never warrants deleting the data lock. Actual health/data
verification remains independent of the receipt.

Real Linux lock/signal/uid and loopback SSH-disconnect tests complement service/
payload fixtures. Physical power loss and a household deployment are NOT_RUN.
See [the guide](../../operations/LEGACY-DEPLOYMENT-JOBS.md),
[BUG0128](../../debug/BUG-0128-operator-transport-and-recovery-overlap.md) and
independent reviews. Existing releases, tags and Pi are untouched.
