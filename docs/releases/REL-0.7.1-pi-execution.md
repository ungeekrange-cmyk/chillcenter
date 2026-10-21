# Chill Center 0.7.1 — Pi execution record

**State: `VERIFIED_SCOPED`** — by the distinct verifier’s
[`APPROVED_SCOPED` report](REL-0.7.1-verification.md), not by this deployer.

## Scope

This release replaces only the server binary after a fresh private backup. The
installed web tree, configuration, TLS material, data, NAS and service identity
remain invariants. No updater helper, timer, theme release, or other service
configuration is in scope.

The one authorized systemd exception is the reversible, operator-owned drop-in
which adds only `AF_NETLINK` to the existing address-family set for discovery
interface enumeration. It is applied before the binary operator captures its
invariants; no other sandbox relaxation is permitted.

## Historical pre-execution gates

The following template gates were satisfied before the recorded operation; they
remain historical evidence, not new deployer approvals.

- [x] An explicit human `APPROVED` instruction named the exact final artifact,
  its verified pin, and the exact deployment envelope/target.
- [x] A distinct security review approved the frozen final source and artifact;
  there is no blocking finding.
- [x] The release candidate evidence bound the supplied binary to the frozen
  source revision and the operator receives matching old/new hash pins.
- [x] The local reviewed-operator hashes matched the release evidence.
- [x] The private rollback location was root-owned `0700`, with no incomplete
  transition that requires recovery.
- [x] Immediate read-only baseline was unchanged: active service, expected old
  binary, non-root service user, no capabilities change, exactly the original
  three families, no existing owned discovery drop-in, and preserved protected
  config/data/TLS/web invariants.
- [x] Binary-operator `preflight` passed with the private operator-supplied
  SAN-matching HTTPS URL, certificate, paths and exact pins. The quiet gate
  found no active scan/work, lease, job, or client socket.

Private target details, hashes, paths, certificates, transcripts and baseline
values are retained only in the private execution material; they do not belong
in this tracked record.

## Ordered procedure after all gates are current

1. Revalidate the above baseline without NAS traversal.
2. Run the reviewed family operator `apply` against the private rollback
   directory. It must snapshot the exact prior unit/family state, install its
   one owned file, reload systemd, and prove the exact original family set plus
   `AF_NETLINK`.
3. Reconfirm service user/capabilities and run binary-operator `preflight` with
   the same private pins and HTTPS/TLS arguments.
4. Run binary-operator `deploy` with precisely those arguments. It owns the
   staging journal, final quiet gate, maintenance admission, backup, swap,
   dated-API version proof, legacy-route rejection, and persistence invariants.
5. Record outcome metadata without private values, then hand off to a distinct
   release verifier. Local fixtures and mocks do not prove live Pi discovery,
   LAN browse, or post-deployment smoke.

## Failure and rollback

Any failed guard, invariant drift, TLS failure, unexpected family set, failed
backup, or incomplete journal stops the release. Preserve journals and flags.

For a failed binary deployment, run the binary operator's owned `recover` with
the same pins until it proves recovery. Only then run the family operator's
owned `rollback`: it may remove only the exact drop-in it created, reload,
restart, and prove the original unit bytes and original family set. Do not
remove a changed drop-in or any unrelated file. A distinct verifier must prove
the restored state before it is reported as rolled back.

## Evidence status

- Reviewed local binary and family-operator tests: `PASS` (31 tests in this
  checkout).
- Exact source, code/test/security approvals, and artifact pin were supplied by
  the release coordinator before execution; their exact references remain in
  the candidate evidence.
- Remote baseline and strict operator preflight: `PASS`; no NAS walk.
- Family operator `apply`: `PASS`; effective address families were exactly the
  original set plus `AF_NETLINK`, with service identity and capabilities
  unchanged.
- Binary operator `deploy`: `PASS` (return code `0`) at `2026-10-21T10:27:05Z`.
  It created fresh private backup receipt
  `33de4f70-102a-42f3-afa3-81d607b35d95` before the binary swap.
- Deployment sanity: active service, pinned server artifact, owned drop-in,
  binary journal `complete`, and family journal `applied`.
- Web, configuration, TLS, data, NAS, helper, timer and themes: `NOT_MODIFIED`
  by this operation.
- Independent post-deployment verification:
  [`APPROVED_SCOPED`](REL-0.7.1-verification.md). It independently confirmed
  the active artifact, protected data/web invariants, fresh backup, strict API
  and TLS behavior, and real headless mDNS discovery. Its separate check of
  fresh public downloads is also `APPROVED`.
- Mac GUI and local-network-consent verification: `NOT_RUN`; no GUI conclusion
  is inferred from the headless mDNS probe.
