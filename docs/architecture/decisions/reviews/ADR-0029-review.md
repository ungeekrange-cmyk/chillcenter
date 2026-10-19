# Independent ADR Review

- **ADR**: `ADR-0029` R1
- **Branch / Base Commit / Reviewed Commit**: `codex/component-updates-design` /
  `6cdb649b8c834c4173afd320da2c8da64867a84e` /
  `ac85794541a59b011be11ebbe839c3e30a293ea7`
- **Artifact Hash / Reviewed At UTC**:
  `a2a9dc05d58066749e748c19102778be0095eaffb56a9fe83474a3e040b677a6` /
  `2026-10-19T19:39:02Z`
- **Author Instance**: `/root`
- **Reviewer Instance**: `/root/web_r3_review`
- **Verdict**: `CHANGES_REQUESTED`

## Checklist

- [x] The problem, existing delivery gap and trust-boundary drivers are evidenced.
- [x] Three viable options, including manual-download status quo, are evaluated.
- [x] The proposal preserves ADR-0011's dated API/`v1` compatibility and ADR-0025's web/Mac separation.
- [x] Security, privacy, costs, operability, testing, staging, rollback and the Pi/NAS non-interference boundary are addressed at a design level.
- [ ] The release-automation policy and machine-enforced component compatibility contract are sufficiently decided.
- [x] A required user approval is identified; the ADR remains `PROPOSED`.
- [x] It does not supersede accepted historical delivery evidence.

## Findings and required changes

| ID | Evidence | Risk | Required change |
| :--- | :--- | :--- | :--- |
| `ADR-F-001` | The ADR asks for GitHub manual/automatic checks, but says only that CI quotas must be checked “before adding automated build workflows.” It never assigns publication authority, required blocking checks, token permissions, artifact provenance/retention, or whether release publication is manual versus workflow-triggered. | A later workflow can accidentally publish unsigned/unreviewed artifacts, use an overly broad GitHub token, or leave “manual/automatic” behavior inconsistent with the installer trust model. | Add a normative release-workflow policy: trigger and approval gate, the exact checks that block each component publication, least-privilege permissions, artifact/signature provenance and retention, and an explicit statement that no workflow publishes until that design plus the user trust-key gate are approved. |
| `ADR-F-002` | The ADR requires clients to validate API/schema/renderer constraints, but defines neither the manifest compatibility fields nor an update-order rule for independently versioned Mac/server/web/theme components. | Compatibility can be claimed but not computed or tested; an independently published component may be staged against an incompatible server/API or renderer. | Define a signed manifest compatibility schema (at minimum component, target, version, source/content digest, API date aliases/minimum server, theme-pack format/renderer range, and downgrade/channel policy), a server-first compatibility/update order, and deterministic refusal/error behavior. Include matrix/fixture tests in the verification plan. |
| `ADR-F-003` | The ADR correctly notes that unauthenticated LAN users can change allowed scheduling and request updates, but does not define the operational guard against repeated manual update intents while an existing check/download/install is in progress. | A household LAN peer can create persistent update churn or prevent the 02:00 policy from completing, beyond the acknowledged generic denial-of-service risk. | Specify one persisted, bounded update state machine per server (idempotency/coalescing for manual actions, cooldown and audit-safe status), and state which scheduling settings remain mutable without authentication. Keep it explicitly within the accepted F-18 LAN risk rather than implying an authorization boundary. |

## Positive review observations

- The design does not invent a native mobile app, a cloud service, a public-visibility change, a Pi deployment, or a signing key. These remain outside this proposal.
- The separate root-owned helper, fixed verified paths, non-root server, non-executable theme format, signature-before-staging rule, old-asset retention and explicit Mac install action are sound boundaries to retain.
- `themes/README.md` and `themes/README.en.md` agree with F-21 and US-032: Halloween is 1–31 October, Christmas is 25 November–25 December, and the season resumes the following day. Their client-local calendar is correctly distinct from the server's local-time scheduler.
- The stated verification plan honestly marks runtime, Pi, installer, signer, scheduler and device work as not executed. No runtime test was run for this documentation-only review.

## Approval state

The ADR must remain **PROPOSED**. Address `ADR-F-001` through `ADR-F-003`, freeze
a new revision/hash, and obtain a fresh independent review. Even if those changes
are accepted, the explicit user approval remains required before key generation,
key provisioning, downloaded-code installation, privileged helper creation,
automatic publication or deployment.
