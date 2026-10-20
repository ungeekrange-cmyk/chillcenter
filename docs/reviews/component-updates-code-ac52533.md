# Independent code operational delta review — component updates `ac52533`

- **Review type / reviewer instance:** code / `/root/v07_final_code`
- **Lane:** `FULL_ASSURANCE`, accelerated operational delta review
- **Parent reviewed SHA / verdict:** `3913267e98f82eeca5f7c627490fbdd019636177` / `APPROVED_SCOPED` in `component-updates-code-3913267.md`
- **New source SHA:** `ac5253328da3bd2cb6a65e9913c0bbe5b8d3d33b`
- **Independence:** reviewer did not author the source; source-non-mutating
- **Date:** 2026-10-20
- **Current verdict:** **APPROVED_SCOPED**

No P0, P1 or P2 correctness, architecture, compatibility or maintainability
finding remains in the exact source delta. The prior code approval is reusable
outside the four changed source/test files. The intermediate `90e7fcc` commit is
a documentation/evidence carrier; the behavioral changes are `1b6b2dc` and
`ac52533`.

## Source delta review

- `apps/desktop/src/update_bridge.rs:69-96,1001-1030` extracts the existing
  native AppKit confirmation and post-consent playback guard. The production
  order is unchanged: verified download, pre-dialog playback guard, SDK update
  restoration, consent, second playback guard, fresh manifest verification,
  receipt persistence, then installer mutation. Cancel, closed/dropped dialog
  and post-consent busy paths retain `UPDATE_CANCELLED` / `UPDATE_BUSY`; the
  helper remains crate-private and exposes no JS command or public trust input.
- `scripts/helpers/macos_minimum_version.py:11-55` parses deployment minima,
  not SDK versions, from every Mach-O the existing packager can emit: its single
  arm64 executable plus the complete flattened top-level dylib closure. It
  rejects a missing closure, path-like executable name, wrong platform,
  unrecognized load command, malformed version and failed `vtool` invocation.
  It starts from the configured floor, so it cannot lower it, and writes only
  after every inspection succeeds.
- `scripts/package-macos.sh:17,54-69` requires the platform `vtool`, derives the
  staged plist after the full Homebrew closure is copied, validates the plist,
  and only then signs the dylibs, executable and bundle. The metadata rewrite is
  therefore covered by the final bundle signature. Existing no-overwrite,
  arm64-only, relocation, load-test and forbidden-package-manager-link checks
  remain in force.
- The seven Python cases cover modern and legacy load commands, SDK/minimum
  distinction, wrong-platform and malformed fail-closed behavior, numeric
  ordering, maximum selection, configured-floor preservation, metadata
  preservation and no plist rewrite after inspection failure. The Rust unit
  fixes the preserved busy code/message contract. No runtime, dependency,
  lockfile, endpoint, key, installer privilege or public API changed, so the
  runtime-lifecycle review has no new selection to approve.

## Evidence

Reviewer-run or reviewer-inspected against exact
`ac5253328da3bd2cb6a65e9913c0bbe5b8d3d33b`:

- `git diff --check 3913267..ac52533` — PASS.
- `python3 scripts/helpers/test_macos_minimum_version.py` — PASS, 7/7.
- `cargo test -p chillcenter-desktop --locked --offline
  update_bridge::tests -- --test-threads=1` — PASS, 9/9.
- `cargo fmt --check` — PASS.
- `cargo clippy -p chillcenter-desktop --locked --offline --all-targets --
  -D warnings` — PASS.
- `bash -n scripts/package-macos.sh` — PASS.
- Exact-source package log `/private/tmp/chillcenter-v07-minos-ac52533.log` —
  release build completed; 48 arm64 dylibs from 41 formulae; all 49 packaged
  Mach-O files inspected; derived minimum `27.0`; plist lint, deep strict
  ad-hoc signature verification, local libmpv API 2.5 load and relocated-closure
  verification all PASS.
- Independent read-only checks of the resulting exact-source bundle confirmed
  `LSMinimumSystemVersion=27.0`, arm64 executable, 48 dylibs, a valid deep strict
  signature and no `/opt/homebrew`, `/usr/local` or `/opt/local` load path. Its
  deployment minima are 11.0 (one file), 13.0 (one), 26.0 (13) and 27.0 (34),
  independently confirming that `27.0` is the maximum requirement rather than
  the SDK value.

The broad Rust, Node, Linux VM/root-ledger and hostile-input evidence attached
to the approved parent remains behaviorally valid: neither Mac-only delta can
alter those paths. The targeted Rust and package proofs above cover the changed
behavior; independent test/security reviewers remain responsible for their
own exact-SHA verdicts.

## Limitations and required follow-up

This is a code verdict, not release approval. It does not claim the separately
owned genuine AppKit consent/install/restart journey, behavior on another Mac
or an older macOS release, notarization, GitHub publication, Pi/NAS access,
production installation, power-cut or disk-full evidence. The package floor is
derived from the exact Homebrew inputs used for each build; release evidence
must continue to record that resulting floor rather than reuse the source
configuration's `13.0` value or a previous build's result.

## Verdict

**APPROVED_SCOPED for code at exact source SHA
`ac5253328da3bd2cb6a65e9913c0bbe5b8d3d33b`.** No code correction is requested.
Any later source, packaging-helper or dependency-input change makes this verdict
stale; a report-only carrier commit does not.
