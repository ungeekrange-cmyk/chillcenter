# Independent code delta review — component updates `3913267`

- **Review type / lane:** code correctness, `FULL_ASSURANCE` operational delta review
- **Branch / exact source:** `codex/release-v0.7-fixes` / `3913267e98f82eeca5f7c627490fbdd019636177`
- **Parent reviewed SHA / verdict:** `6041f4dc8acb6e14ea59bcb4e8ce6cec5d9edcaa` / `APPROVED_SCOPED` in `component-updates-code-6041f4d.md`
- **Independent reviewer:** `/root/v07_final_code` (not an author; source-non-mutating)
- **Date:** 2026-10-19
- **Verdict:** **APPROVED_SCOPED**

No P0, P1 or P2 correctness, architecture, compatibility or maintainability
finding remains in `6041f4d..3913267`. The delta fixes the observed native
startup failure without opening a configurable release-key or endpoint path.
The previous exact-source code approval is otherwise reusable because no Linux,
server, web, theme, updater state-machine or installation behavior changed.

## Delta review

The exact delta is two files: one required bootstrap configuration value and one
focused regression test.

- `tauri-plugin-updater 2.14.0` declares `Config.pubkey` as a mandatory
  deserialization field. The plugin config is deserialized before
  `Builder::pubkey` can override it, matching the real pre-window startup abort.
- `apps/desktop/tauri.conf.json:15` now supplies the base64 encoding of the
  existing public `updates/release.pub`. It adds no endpoint, URL, private key,
  downgrade flag or insecure transport/certificate/hostname option.
- The production plugin builder remains bound to the compile-time `PUBLIC_KEY`
  constant (`apps/desktop/src/update_bridge.rs:1228-1232`). The configuration
  value is therefore a bootstrap prerequisite, not a second runtime-selectable
  trust source.
- The new test deserializes the real embedded JSON into the real SDK `Config`,
  decodes the key and compares it byte-for-byte with `PUBLIC_KEY`. It also
  requires signed versions, empty endpoints, no downgrade, and all dangerous
  transport/TLS flags false (`apps/desktop/src/update_bridge.rs:1248-1266`). This
  makes future key rotation fail closed unless both compile-time and packaged
  configuration pins are updated together.

No dependency, lockfile, runtime version or public API changed. The exact-source
security review `component-updates-security-3913267.md` independently reports
`APPROVED` with no open finding. The exact-source test review remains
`CHANGES_REQUESTED` solely for the still-unexecuted genuine AppKit install
journey; that missing release proof is not an observed source defect and is not
reclassified by this code review.

## Evidence

Reviewer-run against exact `3913267e98f82eeca5f7c627490fbdd019636177`:

- `git diff --check 6041f4d..3913267` — PASS.
- `cargo fmt --check` — PASS.
- Direct decode of `plugins.updater.pubkey` versus `updates/release.pub` — exact
  byte equality, 113/113 bytes.
- `cargo test -p chillcenter-desktop --locked --offline
  update_bridge::tests -- --test-threads=1` — PASS 8/8, including SDK config
  deserialization and the existing ephemeral-Minisign recovery cases.
- `cargo clippy -p chillcenter-desktop --locked --offline --all-targets --
  -D warnings` — PASS.

The broad Rust, Node, Linux VM/root-ledger, helper-hardening and recovery evidence
listed in `component-updates-code-6041f4d.md` remains valid: this Mac-only config
and unit-test delta cannot change those code paths. The adverse native launch at
the parent SHA remains useful causal evidence rather than being erased.

## Residual proof limits

No rebuilt exact-source native app has yet completed the real AppKit
cancel/accept/install/failure/restart journey. The focused regression proves the
specific pre-builder configuration failure is closed, but not the downstream
GUI and installer flow. No Pi/NAS, production host, GitHub publication,
power-cut or disk-full action is claimed. These remain test/release gates, not
code findings.

## Verdict and routing

**APPROVED_SCOPED for code at exact SHA
`3913267e98f82eeca5f7c627490fbdd019636177`.** No code correction is requested.
Publication remains blocked by the current independent test verdict until the
exact-source native journey succeeds and is reviewed. Any later source or
packaged-config change makes this code verdict stale; a report-only carrier
commit does not.
