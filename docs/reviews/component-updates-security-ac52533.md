# Operational security delta review — component updates `ac52533`

- **Review Type / Reviewer Instance:** security / `/root/v07_security_delta`
- **Parent Reviewed SHA / Verdict:** `3913267e98f82eeca5f7c627490fbdd019636177` / **APPROVED**, [independent report](component-updates-security-3913267.md)
- **New Source SHA:** `ac5253328da3bd2cb6a65e9913c0bbe5b8d3d33b`
- **Reviewed at:** `2026-10-20T08:34:45Z`
- **Lane:** production/material-risk, full assurance; accelerated operational delta
- **Current Verdict:** **APPROVED**

This reviewer did not author the reviewed design, source or remediation. The
review was read-only except for this report and disposable local test outputs.
No Pi, NAS, SSH, GitHub, cloud or production signing operation was performed.
The production private signing key was never read, printed or used.

## Source delta and security invariants

The only source changes after the approved parent are:

1. `1b6b2dc` extracts the existing native Mac confirmation into a shared Rust
   helper and adds a focused guard test. The dialog remains warning/OK-cancel;
   a closed callback channel still fails closed as cancellation; the playback
   guard still runs immediately after positive consent; and the exact signed
   manifest is still reverified before the acceptance receipt or executable
   mutation. No renderer command, feature flag, environment bypass or alternate
   production consent path was added.
2. `ac52533` adds the Mac minimum-OS packaging helper, seven tests and its
   invocation in `scripts/package-macos.sh`. `CFBundleExecutable` must be a
   basename, every packaged top-level media dylib and the executable are
   inspected with absolute `/usr/bin/vtool` through an argument vector, malformed
   or non-macOS load commands fail closed, and versions compare numerically.
   The configured floor is never lowered. The helper changes only the new staging
   plist before signing; `set -e` and the existing cleanup/publish flow prevent a
   failed inspection from publishing or overwriting a destination bundle.

The inspected prior 0.7 candidate contains 48 top-level dylibs, no nested files
or symlinks, plus the executable. It advertised macOS `13.0`, while all 49 Mach-O
files derive a required maximum of `27.0`. Invoking the new helper on a disposable
clone changed only that clone's plist to `27.0`; the original remained `13.0`.
The old packages therefore remain historical, non-releasable artifacts and must
not be published as the `ac52533` output.

No authentication, authorization, tenant, session, cookie, CSRF, redirect,
customer-data, network-exposure or logging boundary changed. `Cargo.lock`, JS
lockfiles, runtime pins, `updates/release.pub`, updater endpoints and URL
admission are unchanged. The source delta contains no URL or private-key marker.
The existing fixed-origin, signed-asset, no-renderer-URL/path/force and exact-key
tests remain green. Rust/Cargo `1.99.1`, Node `24.13.0` and dependencies are
unchanged from the approved parent. The eight formerly HIGH findings remain
closed; no new CRITICAL, HIGH, MEDIUM or LOW finding was identified.

## Reproducible evidence

| Tool / command | Result | Scope / limitation |
| :--- | :--- | :--- |
| Git 2.54.0 — `git rev-parse HEAD`; `git diff --check 3913267..ac52533`; source-path and lock/trust-root comparisons | PASS; exact SHA `ac5253328da3bd2cb6a65e9913c0bbe5b8d3d33b`; four changed source/test paths; no lockfile, runtime pin or `release.pub` delta | Pre-existing and concurrent uncommitted documentation/report changes were preserved and excluded. |
| Rust/Cargo 1.99.1 — `CARGO_TARGET_DIR=/private/tmp/chillcenter-security-ac52533-target cargo test --locked -p chillcenter-desktop update_bridge` | PASS: 9 passed, 28 filtered | Exact source SHA; includes consent busy/cancel boundary, embedded key/config and renderer non-injection. Minisign uses only a disposable synthetic test key; its key material is omitted here. No native UI interaction. |
| Rust/Cargo 1.99.1 — `cargo fmt --all -- --check` | PASS | Formatting only. |
| Python 3 — `python3 scripts/helpers/test_macos_minimum_version.py -v` | PASS: 7/7 | Covers SDK/minimum distinction, legacy load command, wrong platform/malformed input, numeric comparison, maximum selection, no lowering and no rewrite after failed inspection. |
| Bash — `/bin/bash -n scripts/package-macos.sh` | PASS | Syntax/static shell check only. |
| macOS `vtool`/`plutil` plus `/usr/bin/python3 scripts/helpers/macos_minimum_version.py '/private/tmp/chillcenter-security-ac52533-bundle.1l0gx2/Chill Center.app'` | PASS: 49 Mach-O files inspected; disposable plist `13.0` -> `27.0`; original stayed `13.0` | Uses the prior local candidate's packaged executable/libraries to exercise the new helper. It is not a clean rebuild or signed release artifact. |
| Changed-source marker scan over `apps/` and `scripts/` | PASS: no changed URL or private-key header/Minisign-secret marker | Pattern scan is not a full-history or entropy scan; trust-root immutability and focused runtime tests provide the relevant delta evidence. |

## Limitations / required follow-up

This verdict approves only the security delta at exact source SHA `ac52533`.
It does not approve an older archive, a tag, publication, notarization, native
Mac install/restart UX, Pi deployment or post-release behavior. Release tooling
must build a fresh immutable Mac artifact from this SHA, confirm the packaged
plist matches every bundled Mach-O before signing, keep the private key outside
Git/GitHub, verify downloaded hashes/signatures, and complete the distinct
packaging/test and release-verification gates. No additional security remediation
is required for this source delta.
