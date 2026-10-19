# BUG0113 — Privileged component updater is not release-ready

- Priority: P1; status: OPEN / RELEASE_BLOCKED.
- Source candidate: `22a38ff50879837c32fc8d5ac7c7200f35c8eaa1`.
- Architecture: accepted ADR0029 R2; acceptance of the design is not approval
  of this implementation. No main merge, public 0.7 release or Pi installation.

The preparatory independent security review found concrete Linux-helper
implementation gaps despite passing local unit tests:

1. A stale activation-authorization file can outlive the updater process and
   authorize an uncommitted slot on restart.
2. A crash while writing the final content-addressed cache directory can poison
   subsequent checks; owned temporary receipt files lack safe recovery.
3. Bootstrap interpreter/PATH/Python environment is insufficiently isolated.
4. Offline backup child execution has neither bounded elapsed time nor bounded
   captured stdout/stderr while the server is stopped.
5. Archive staging/validation happens after service quiescence/stop instead of
   before the short final commit window.
6. Server unit filesystem restrictions prevent its privileged pre-start recovery
   from writing rollback changes under the component installation roots.
7. Extracted slot modes inherit the root helper's private umask and can prevent
   the non-root service from traversing/reading activated resources.
8. Failure/timeout orchestration does not guarantee prompt recovery and restart
   after a helper crash while the server is intentionally stopped.

Exact file/line references and final independent verdict are owned by the
security review report for this source. These are **not** accepted risks and
must not be papered over by retries, relaxed verification or an advertised
successful installation. No affected helper has been installed on the Pi.

Other findings were addressed on the integration branch: native Mac consent,
signed-byte revalidation at installation time, durable pre-install sequence
acceptance, bounded local cache, exact theme tree/calendar/PNG validation,
retained web resources and background writer quiescence. Those changes still
require exact-source independent review; they do not close this umbrella bug.

Required closure: minimal cause-specific corrections, interruption/fault tests
at every privileged transition, root/systemd integration in an isolated Linux
environment, independent code/test/security approval and immutable package
verification. Preserve current versions/backups; never test faults on the NAS.

Independent [code review](../reviews/component-updates-code-22a38ff.md), same SHA,
also records four P1 functional blockers: busy installations become terminal
failures rather than deferred work; interrupted Running operations are not
reconciled on restart; cached failed/running checks can claim availability; and
Mac/theme accepted releases lack an exact-identity retry path after failure.
Two additional P2 issues concern transient-error messaging and scheduling of
unsupported component ledgers. These require separate regression evidence too.
