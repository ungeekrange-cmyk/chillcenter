# BUG0127 — Component checks incorrectly require the installer

Date: 2026-10-21. Status: FIXED / verification in progress. Branch:
`codex/update-check-controls`, base `6ba3970`. User reports web 0.7.0 and Mac
0.7.1 can only check the native Mac release. Related: ADR0029, component update
contract, sole API `2026-10-14`.

## Expected / observed

All supported component checks are usable independently of installation rights.
“Install now” is absent until a newer release is detected. Instead, the shared
UI unconditionally creates the install button and disables **check**, save and
install together when `helper_available` is false. The Pi deliberately has no
root updater installed. The API also rejects check intents in that case.

## Evidence and diagnosis

H1 confirmed by source tracing: `ui/connected/updates.js` gates all three buttons
on the root helper; `updates_api::intent` rejects both intent kinds when the
helper configuration is absent. Local Mac checks bypass that server path,
explaining the different behavior. No certificate/network bypass is warranted.

Regression added first: the new-version visibility unit test fails RED on the
old implementation (missing predicate). Shared DOM fixture now covers Mac and
web, no helper, checks for server/web/themes, no/equal/older/newer versions,
failed recheck, native consent and use of the catalogue during a held request.

## Correction

- Separate UI capabilities and hide installation unless a newer stable version
  is detected. Installation/scheduling remain disabled without a root helper.
- Add a bounded non-root manual metadata worker for legacy/no-helper servers.
  Existing signed-manifest verifier and fixed GitHub transport are reused.
  Discovery persists only UI results in a separate store; no install intent,
  payload download, root cache or anti-rollback acceptance is created.
- Keep five-minute coalescing, one worker per component, bounded HTTP and files,
  explicit errors/incompatibility, atomic result publication and background work.
- Add `check_available` compatibly; dated API version and closed request fields
  are unchanged. Translate feedback into French and English.
- Prepare Mac/server/web 0.7.2, sequence 3; unchanged themes stay 0.7.1/sequence 2.
  Existing signed tags/releases are immutable and were not edited.

## Verification and limitations

Unit and actual browser DOM checks pass in the working candidate; exact final
commands/results and source SHA will be recorded after the full targeted gate.
The first default-sandbox Chrome invocation failed before page load (SIGABRT /
EPERM); the isolated escalated browser run succeeds. A first web fixture reused
the old document through hash-only navigation; fresh-document navigation fixed
the oracle, not application code. No native player test is needed here.

No Pi deployment or GitHub publication is part of this correction so far.
The installed web 0.7.0/Mac 0.7.1 require their corrected builds and the new
server capability to exercise this path. Rollback: restore previous client/web
and server binaries; optional check metadata is ignored by older versions.
NAS, settings, TLS identity and privileged updater configuration are untouched.
