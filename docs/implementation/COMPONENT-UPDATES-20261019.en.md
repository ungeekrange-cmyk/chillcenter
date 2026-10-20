# Independent components — 19 October 2026 candidate

[Français](COMPONENT-UPDATES-20261019.md) | **English**

Frozen source: `22a38ff50879837c32fc8d5ac7c7200f35c8eaa1`, branch
`codex/component-updates-design`, public base `ac85794`.
**RELEASE_BLOCKED: no main merge, 0.7 release or Pi installation.**

The human approved ADR0029 R2 with “oui je valide”, recorded in the
[gate](../gates/G-ADR-0029-R2.md). Code and test review cannot replace security
review or missing system-level evidence.

## Implemented on the branch, not delivered

- Initial source version0.7.0 for Mac/server/web/themes. Since the0.7.1 delivery,
  only the dated2026-10-14 API is served; the old alias is retired (ADR0031).
- Declarative packs in `themes/default`, FR/EN dates/priorities, validated
  resources and embedded fallback. No JS/CSS/HTML in themes.
- Settings card independent of the catalogue lock, actual installed versions,
  old-server support with only unsupported controls disabled.
- Fixed-repository GitHub discovery, signed manifests, compatibility, monotonic
  replay protection; native confirmation before Mac installation.
- Manual/check/automatic policies, local time and default server 02:00.
- Bounded LAN intents, separate helper, release slots and rollback simulations.
  **Its privileged Linux path is not approved for delivery.**
- Inspectable installers and local release preparation/signing. Private key stays
  outside Git; only its public key is embedded.
- Additive FR/EN API documentation/OpenAPI and bilingual user guide.

## Local evidence

Commands against the frozen source, Apple Silicon Mac / Rust 1.99.0:

```sh
cargo test --locked --offline --workspace --all-targets -- --test-threads=1
cargo clippy --locked --offline -p chill-updates -p chill-server -p chillcenter-desktop --all-targets -- -D warnings
cargo fmt --check --all
node --test ui/connected/*.test.js ui/connected/tests/*.test.js web/test/*.test.js scripts/test-component-release.mjs scripts/test-bilingual-docs.mjs scripts/test-public-site.mjs
```

Results: serialized Rust suite PASS (two existing ignored tests), Clippy/format
PASS, **357/357 JavaScript/documentation tests PASS**. Engine: 12 unit tests and
1 shell-syntax/bootstrap-help integration test; desktop: 31 unit tests. Shell
syntax does not prove root/systemd operation.

Isolated Chromium QA `scripts/test-component-settings.cjs` PASS: versions, policy
CAS, preserved active settings draft, catalogue use during slow IPC and old-server
fallback. Synthetic bridge: not native Tauri confirmation or installation proof.
Four event-appearance variants also passed QA. Web build: **6dd3d04c9203**, 98 files,
not deployed.

An **internal local-review-only** Mac bundle was built: 0.7.0, build 9,
`Chill-Center-v0.7.0-review.app`. Ad-hoc signing/strict verification PASS,
48 relocated arm64 libraries and bundled libmpv load PASS. Signed executable
SHA-256: `aacf3689d8b381e5f00eb94a4edcb5f27b8c2bb2e9b91f618fe6765508ef36cc`.
It does not replace the delivered bundle and is neither published nor approved
for delivery.

Two earlier failures of the existing subtitle timing test remain recorded in
[BUG0114](../debug/BUG-0114-subtitle-timing-test-instability.md). Its isolated and
serialized-suite passes do not constitute a fix.

Added-line scan from `ac85794` to the frozen source found no private-key or
certificate header, GitHub/API-shaped credential or household private address.
This focused scan does not certify the entire history or unknown secret formats.

## Publication block and continuation

[BUG0113](../debug/BUG-0113-component-updater-release-gate.md) and the
[security review](../reviews/component-updates-security-22a38ff.md) record eight
HIGH findings: stale authorization, crash-poisoned cache, bootstrap environment,
unbounded backup, preparation during downtime, systemd restrictions incompatible
with recovery, slot permissions and restart after helper failure.
The [code review](../reviews/component-updates-code-22a38ff.md) additionally
requests four P1 retry/deferral/state corrections and two P2 corrections. The
[test review](../reviews/component-updates-tests-22a38ff.md) confirms focused
checks but requests the missing execution evidence. All three verdicts are
**CHANGES_REQUESTED** on the same source. Corrections,
independent review and Linux interruption tests are required before merge or
publication. Recheck the toolchain patch before delivery builds too.

Not executed: root/systemd install, real SIGKILL boundaries, disk full, Pi/NAS,
actual installed Mac bundle update, publication and signed-artifact download
verification. No CI secret, new network exposure, production server or NAS change.

Previously delivered versions remain unchanged and usable. No operational rollback
is needed: nothing was deployed. Do not enable the candidate helper to bypass
the gate. Continue fixes on an isolated branch without rewriting old tags/reports.
