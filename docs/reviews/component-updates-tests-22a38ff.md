# Independent test execution review — component updates `22a38ff`

- **Review mode:** `EXECUTION_REVIEW`
- **Branch / frozen source:** `codex/component-updates-design` /
  `22a38ff50879837c32fc8d5ac7c7200f35c8eaa1`
- **Baseline:** `ac85794541a59b011be11ebbe839c3e30a293ea7`
- **Authors / integrator:** `/root/component_update_engine`, `/root`
- **Independent reviewer:** `/root/component_updates_tests`
- **Reviewed at:** `2026-10-19T20:42:46Z`
- **Environment:** macOS 27.0.1 (26A434), Apple Silicon; Rust/Cargo 1.99.0;
  Node 24.13.0; Google Chrome 155.0.8059.82
- **Verdict:** **CHANGES_REQUESTED — not ready for merge, release, publication or deployment**

The focused checks are green and several assertions are meaningful, especially
theme confinement, immutable web assets, closed API bodies, CAS conflicts and
rootless rollback. They do not prove the privileged Linux updater, production
manifest pipeline, native Mac confirmation, real GitHub transport, systemd
recovery or Pi behavior. The independent security review also reports eight open
HIGH defects whose affected paths have no adequate execution evidence. Passing
unit and synthetic-browser checks cannot override those release blockers.

## Reproducible evidence

Commands below were run against the frozen SHA by this reviewer unless marked
`coordinator evidence`. No network, Pi, NAS, cloud, release or system mutation
was used.

| Command | Exit | Duration | Result | Skips/retries | Evidence scope |
| :--- | :---: | :---: | :--- | :--- | :--- |
| `cargo test -p chill-updates --locked --offline` | 0 | 5.16 s | 13 passed (12 library + 1 shell integration) | 0 | Focused updater unit/rootless simulations |
| `cargo test -p chill-server component_update_routes_are_additive_versioned_and_closed --locked --offline` | 0 | 15.62 s | 1 passed | 143 filtered | Dated API + v1 alias, defaults, closed DTO, policy CAS |
| `cargo test -p chill-server web_static::tests --locked --offline` | 0 | 0.50 s | 6 passed | 138 filtered | Retained web/theme asset and symlink boundaries |
| `node --test scripts/test-component-release.mjs ui/connected/updates.test.js web/test/bridge.test.js` | 0 | 0.21 s | 26 passed | 0 | Manifest generator, bounded UI policy, web bridge plus inherited bridge regression |
| `node ui/connected/theme-pack.test.js` | 0 | 0.04 s | script PASS | N/A | Calendar boundaries, strict theme metadata, fallback |
| `CHILL_PREVIEW_NODE_MODULES=/private/tmp/marvin-cockpit.NmYyTL/package-manager/node_modules node scripts/test-component-settings.cjs` | 1 then 0 | 0.81 s then 1.98 s | PASS outside sandbox | one environment retry | Real shared DOM/UI with synthetic Tauri bridge; first Chrome launch aborted under sandbox (`SIGABRT`/`EPERM`), not a candidate assertion failure |
| `cargo test --locked --offline --workspace --all-targets -- --test-threads=1` (`coordinator evidence`) | 0 | not captured | latest serialized full workspace PASS | 2 ignored; earlier separate runs include one unstable player assertion | Full Rust regression; ignored RSS harnesses are not passes |
| all UI/web/component-release/bilingual/public-site Node checks (`coordinator evidence`; exact aggregate invocation/duration not captured) | 0 | not captured | 357/357 passed | 0 reported | Broad Node regression, including 22 bilingual and 9 public-site checks |

The two ignored Rust checks are the deliberately isolated NFR harnesses
`benchmark_catalog_publication_memory` and `sync_memory_10k_production_route`.
They are unrelated to the update delta, but remain `NOT_RUN`, not passes.

The earlier full-workspace history is preserved in
[`BUG-0114`](../debug/BUG-0114-subtitle-timing-test-instability.md): the initial
run passed, a later run and a load-concurrent targeted run failed
`subtitle_changes_keep_the_current_playing_or_paused_state`, and a direct quiet
binary run passed in 1.43 s. No player source or threshold was changed. The
scheduling/decode-load hypothesis remains inconclusive; the latest full pass
does not erase the flake.

## Risk-to-evidence matrix

| Risk / contract | Evidence and oracle | Result | Assertion quality / limit |
| :--- | :--- | :--- | :--- |
| Four independently versioned/signed streams | Manifest-generator tests bind component tag, source, sequence and compatibility; Rust verifies a public synthetic Minisign vector | **PARTIAL** | Good generator assertions, but no test drives `parse_and_verify_manifest` end-to-end for mac/server/web/themes or its rejection matrix. The vector calls Minisign directly in legacy fixture mode and does not prove the production parser path. |
| Declarative independent themes | Rust exact-tree/hash/PNG validation; desktop fallback/path tests; JS calendar/fallback checks; server retained-hash/static-path tests | **PASS_SCOPED** | Strong negative assertions for traversal, undeclared files, active content and invalid dates. No installed signed theme update or real native/browser reload was exercised. |
| Bounded LAN intents and compatibility API | Dated/v1 route test, closed request DTO tests, web bridge exact-body test, one persisted CAS/coalescing test | **PARTIAL** | Exact request bodies and stale CAS are meaningful. Five-minute rate limit, global install slot, transition races, terminal-history bound, corrupt persistence and restart recovery are untested. |
| 02:00 local scheduler | Default 02:00 asserted through API/UI; unit tests cover one UTC missed-time date and bounded retry | **PARTIAL** | No DST gap/repeat, non-UTC local offset, clock correction, systemd persistence or real helper tick execution. |
| Mac always confirms natively | UI omits automatic mode; renderer DTO rejects `confirmed`, URL and force; source contains `app.dialog()`; Chrome UI passes | **NOT_RUN at native boundary** | The browser uses a synthetic bridge and the Node assertion is source-text inspection. No Tauri/AppKit dialog cancel/accept, updater signature, install/restart or playback-race journey ran. |
| Privileged Linux install/rollback/recovery | Rootless health-failure rollback and injected post-switch panic pass; shell syntax/help/static substring test passes | **NOT_RUN at required boundary** | `bash -n`, `--help` and source substrings do not prove root ownership, modes, non-root readability, systemd sandboxing, backup bounds, kill recovery or safe bootstrap environment. |
| Regression | Latest serialized Rust workspace and 357 Node checks pass | **PASS with open flake** | Broad evidence is useful; two NFR harnesses are ignored and BUG0114 remains inconclusive. |

## Assertion-quality assessment

Strong examples include exact symlink refusal and old-content selection in
`web_static`, exact policy-revision conflict at both API aliases, response-body
stripping in the web bridge, theme hash/tree/PNG failures, and rollback restoring
the prior `current` link after a failed health check. These have observable
behavioral oracles rather than mere execution.

The principal weak assertions are security-critical: the installer integration
test only parses shell/help and searches source strings
(`crates/chill-updates/tests/installers.rs:8-46`); the trust test validates an
in-memory structure but never opens or transitions `AntiRollbackStore`
(`crates/chill-updates/src/trust.rs:320-342`); the signature tests do not call
the production manifest entry point (`manifest.rs:161-186,453-481`); and the Mac
consent checks inspect DTO/source structure while the real dialog and installer
remain unexecuted (`apps/desktop/src/update_bridge.rs:444-510,695-727`,
`scripts/test-component-settings.cjs:1-61`).

## Findings and routing

| ID | Severity / type | Evidence | Required action | Route |
| :--- | :--- | :--- | :--- | :--- |
| `CU-TEST-001` | **HIGH — missing production trust-path coverage** | Contract requires parser/signature/URL/archive attacks and bounded persisted state (`docs/updates/CONTRACT.md:43-45`), while 13 updater tests omit an end-to-end `parse_and_verify_manifest` fixture for every component, `AntiRollbackStore` transitions, redirect/download bounds, ZIP/raw attack cases, rate/global-slot/history bounds and corrupt/symlink persistence. | Add clearly synthetic pre-generated signed fixtures and negative matrices that drive the actual production seams; cover tamper, expiry, component/target/key/tag substitution, compatibility, replay/downgrade, byte/hash mismatch, archive limits, trust persistence and intent transitions. Do not use a production private key. | Builder/test author, then fresh independent test + security review |
| `CU-TEST-002` | **HIGH — missing privileged Linux/systemd and fault evidence** | The current shell check is static only. Real root/systemd/Pi/public-GitHub installation is explicitly `NOT_RUN`. The exact-SHA security report records eight HIGH defects in activation authorization, cache publication, bootstrap environment, backup bounds, pre-quiescence staging, recovery sandbox, slot modes and failure orchestration. | Correct the eight defects first. Then run isolated Linux root/systemd integration as the real service UID/GID, hostile-environment bootstrap tests, maximum-archive/outage measurements, disk/error injection and TERM/KILL/power-loss simulations at each marker/swap/start point. Prove old healthy service recovery. Pi and published GitHub smoke remain separate explicitly authorized gates. | Builder + test author; integration manager; security reviewer |
| `CU-TEST-003` | **HIGH — missing native Mac confirmation/install journey** | The Chrome harness declares a synthetic bridge; Rust/Node tests only establish that renderer fields cannot self-confirm and that dialog source exists. | Execute a real Tauri/AppKit candidate journey: cancel and accept dialog, no install without click, signed metadata/package rejection, busy playback before/after consent, installer failure preserving current app, and restart consent. Record bundle/source hashes and visible results. | Builder/test author, then independent native execution review |
| `CU-TEST-004` | **MEDIUM — open regression flake** | [`BUG-0114`](../debug/BUG-0114-subtitle-timing-test-instability.md) records pass/fail/pass behavior; the load hypothesis is not confirmed and no fix exists. | Continue one-hypothesis-at-a-time diagnosis with a controlled quiet-versus-load experiment. Do not relax or skip the assertion merely to obtain green. | Debugger; then builder/test author if cause is confirmed |

## Gate conclusion

The candidate has credible scoped proof for declarative theme safety, retained
web assets, closed LAN DTOs, version display and nonblocking synthetic settings.
It does **not** have genuine evidence for the highest-risk executable boundaries.
`CHANGES_REQUESTED` is therefore required. The exact source must be corrected,
re-frozen and re-reviewed; unit-suite success, a private web build, or a Mac
package build cannot be presented as release, public-install or Pi readiness.
