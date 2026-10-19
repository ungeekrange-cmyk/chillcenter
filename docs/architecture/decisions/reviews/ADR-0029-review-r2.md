# Independent ADR Review

- **ADR**: `ADR-0029` R2
- **Branch / Base Commit / Reviewed Commit**: `codex/component-updates-design` /
  `6cdb649b8c834c4173afd320da2c8da64867a84e` /
  `ac85794541a59b011be11ebbe839c3e30a293ea7`
- **Artifact Hash / Reviewed At UTC**:
  `f825a8f688ec326358d552d8f1155b5cb080d44730a1dc7dbf79e56ebfcedcd7` /
  `2026-10-19T19:39:02Z`
- **Author Instance**: `/root`
- **Reviewer Instance**: `/root/web_r3_review`
- **Verdict**: `APPROVED` (required user gate remains pending)

## Checklist

- [x] The problem, evidence and decision drivers are clear.
- [x] Manual downloads, unsigned hashes and signed component releases are fairly evaluated.
- [x] The revision remains compatible with ADR-0011's dated API/`v1`, ADR-0025's platform split and F-20/F-21's local theme calendar.
- [x] Security/privacy/cost/operability/testability/rollback are addressed without claiming implementation or deployment.
- [x] Publication is a manual maintainer operation, with no new Actions workflow, PAT, signing secret, release overwrite or implicit public execution.
- [x] Manifest v1 defines signed identity, sequence, expiry, asset, platform, API/schema and renderer compatibility fields; prerequisite order and refusal semantics are explicit.
- [x] LAN update actions are bounded typed intents with persisted CAS, deduplication, rate limiting, queue limits and a root-owned immutable trust/install configuration. Forced interruption is local-only.
- [x] A required user approval is explicit; the ADR remains `PROPOSED` and historical releases are not superseded.

## Resolution of R1 findings

| Prior finding | Resolution evidence |
| :--- | :--- |
| `ADR-F-001` | Lines 54–66 set manual maintainer publication, reviewed immutable assets, existing GitHub CLI authentication only, no CI secrets/workflows, post-upload verification and no overwrite. |
| `ADR-F-002` | Lines 68–93 define signed manifest v1, parser refusal, replay prevention, compatibility fields, update order, old-client protection and manual migration boundary. |
| `ADR-F-003` | Lines 167–189 constrain LAN request fields, policy CAS, daily-ledger preservation, operation coalescing, five-minute admission, persisted limits and local-only force. |

## Review observations and gates

- Theme FR/EN documentation remains aligned: Halloween is 1–31 October and
  Christmas 25 November–25 December; the season resumes the following day.
- R2 deliberately does not create a signing key, privileged helper, installer,
  GitHub workflow, release asset, client/runtime change, Pi deployment or public
  visibility change. No runtime test was run for this design-only review.
- **User approval is still required before implementation or provisioning.** It
  must cover embedded verification-key bootstrap/custody and rotation, signed
  downloaded-code installation through a root-owned helper, automatic server
  checks/updates at 02:00 local time, and the residual accepted LAN availability
  risk (household users may alter allowed scheduling; only local administration
  may force interruption).
- After that approval, freeze the implementation separately and perform code,
  security, package and deployment reviews; this ADR approval does not approve
  a deployment.
