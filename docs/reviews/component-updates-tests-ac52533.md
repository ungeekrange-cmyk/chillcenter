# Independent test execution delta — component updates `ac52533`

- **Review mode / lane:** `EXECUTION_REVIEW` / `FULL_ASSURANCE`
- **Exact product source:** `codex/release-v0.7-fixes` /
  `ac5253328da3bd2cb6a65e9913c0bbe5b8d3d33b`
- **Parent native verdict:** `CHANGES_REQUESTED` in
  `component-updates-tests-native-20261020.md`
- **Production consent-helper commit:**
  `1b6b2dc79ae7b680e26afab17a1de0c6cba41fed`
- **Feature-gated native QA seam:**
  `7efb31c1e196c53cfa231dbcf043bf6ff1bc75ef`
- **Independent reviewer:** `/root/v07_final_tests`
- **Environment:** macOS 27.0.1 (26A434), Apple Silicon; Rust/Cargo
  1.99.1; Node 24.13.0; Tauri 2.12.3; `tauri-plugin-updater` 2.14.0
- **Verdict:** **CHANGES_REQUESTED**

The exact-source regression, packaging and broad suites are green. The native
QA evidence also closes the two playback-busy branches left open by the parent
review: the production `UpdateHost::install` entry rejected an already-active
renderer before network or prompt, and the same crate-private production
consent helper rejected installation when rendering became active while a real
AppKit prompt was open and the user accepted.

The later-release journey remains incomplete in one exact respect. After the
successful signed 0.7.1 SDK replacement and real restart, a signed 0.7.2 offer
produced a fresh production-helper prompt three times. All three callbacks
reported acceptance, which the QA oracle correctly recorded as `FAIL` before
SDK download. This proves that consent is requested again and that unexpected
acceptance cannot mutate the app in this test mode. It does **not** execute the
agreed cancellation branch. The prior human cancellation used the earlier
harness, had no machine PASS line, and predates the shared-helper evidence; it
is not substituted for this missing check. No cause is assigned to the human
interaction or SDK while clarification is pending.

## Exact-source and packaging evidence

| Evidence / command | Result | Assertion quality and limit |
| :--- | :--- | :--- |
| `/private/tmp/chillcenter-v07-ac52533-rust.log`, SHA-256 `18d357bf…8ef35` | **PASS:** 435 passed, 0 failed, 2 ignored | Full exact-source Rust workspace. The two ignored cases are established manual NFR harnesses, not updater-functional passes. |
| `/private/tmp/chillcenter-v07-ac52533-node.log`, SHA-256 `f786ca08…ef4b4` | **PASS:** 359 passed, 0 failed, 0 skipped | Full Node regression. |
| `python3 scripts/helpers/test_macos_minimum_version.py` | **PASS:** 7/7 coordinator run and 7/7 independent reviewer rerun | Fourteen successful executions across two runs; covers SDK/minimum distinction, legacy command, malformed/wrong platform rejection, numeric maximum, configured-floor preservation and no rewrite after failure. |
| `cargo test -p chillcenter-desktop update_bridge::tests::macos_post_consent_guard_keeps_the_existing_busy_code -- --exact` | **PASS:** 1/1 independent rerun | Preserves the exact `UPDATE_BUSY` code/message for the post-consent guard; native race behavior is supplied separately below. |
| `/private/tmp/chillcenter-v07-minos-ac52533.log`, SHA-256 `1b829f09…4ec0` | **PASS:** exact-source release bundle built; 48 arm64 dylibs plus executable inspected; deep strict signature and relocated libmpv load passed | The resulting `Info.plist` advertises `LSMinimumSystemVersion=27.0`, equal to the independently observed maximum Mach-O requirement rather than the executable's `13.0` or SDK field. |
| Independent bundle inspection of `/private/tmp/chillcenter-v07-minos-ac52533/Chill Center.app` | **PASS** | Deep strict ad-hoc codesign verifies; all 49 Mach-O files have recognizable macOS minima; 48 relocated dylibs are present. This is local packaging evidence, not notarization or publication. |

The broad Linux VM/systemd, root-ledger, hostile-input and accepted-time proofs
approved in the parent reviews remain applicable. The source delta after the
last approved product SHA is confined to the consent-helper extraction/test and
Mac packaging floor helper/test. No Pi, NAS or production host was needed or
contacted for this delta.

## Native evidence inspected

| Required behavior | Result | Evidence and limitation |
| :--- | :--- | :--- |
| Busy before consent | **CLOSED** | QA mode `busy_before` called the real production `UpdateHost::install` with rendering true. It returned `UPDATE_BUSY`; the harness compared the complete isolated bundle tree and request counter before logging PASS. No manifest GET, prompt or mutation occurred. |
| Rendering becomes busy while consent is open | **CLOSED** | QA mode `busy_during` invoked the exact crate-private production helper, changed the shared `AtomicBool` false→true while the real AppKit prompt was open, and the user accepted. It returned `UPDATE_BUSY`; complete tree hash and request counter remained unchanged. |
| Signed 0.7.1 SDK install and restart | **CLOSED** | Real production-helper prompt accepted, Tauri SDK verified and replaced the isolated QA bundle, separate restart prompt accepted, and the replacement executable relaunched. Current bundle passes deep strict codesign, carries marker `qa native update fixture v0.7.1`, and executable SHA-256 is `bf3ef05b…ff690`. |
| Fresh later consent after restart | **PASS_SCOPED** | Three 0.7.2 launches each reached a fresh production-helper prompt. Thus consent was not cached or inherited from 0.7.1. |
| Later-release cancellation prevents mutation | **NOT_RUN / BLOCKING** | Each 0.7.2 callback returned acceptance. The harness logged three explicit `FAIL second-release consent was accepted unexpectedly` lines and stopped before SDK download. There is no `PASS production native consent cancelled` line. |

The exercised `apps/desktop/src/update_bridge.rs` is byte-identical between
product source `ac52533` and QA source `7efb31c` (SHA-256
`e7ebc359…8d2`); production `tauri.conf.json` and `Cargo.lock` are also
byte-identical. The QA-only delta is a feature-gated module/config using an
ephemeral private test key and fixed loopback endpoint; it adds no production
key or URL injection seam.

The final append-only QA journal is
`/private/tmp/chillcenter-qa-native-updates.YNFPqL/qa-native-updates.log`
(SHA-256 `e3696334…3c3f`). Its request ledger (SHA-256
`879e9547…2e63`) contains exactly one 0.7.1 manifest GET, one 0.7.1 archive
GET and three 0.7.2 manifest GETs. It contains no 0.7.2 archive GET. The
isolated bundle remains signed at 0.7.1 with pre-cancellation reference tree
hash `385446ab…ab8f1`. A repeated server-start attempt left an adverse bind
error in `loopback.log`, but independent `lsof` inspection showed that the
active listener's working directory was this exact `YNFPqL` root; the request
ledger and served files therefore remain attributable to this fixture.

## Remaining finding and route

| ID | Severity / type | Evidence | Required action | Route |
| :--- | :--- | :--- | :--- | :--- |
| `CUAC-TEST-001` | **HIGH — agreed native cancellation oracle not executed** | Three fresh 0.7.2 prompts ended in explicit adverse `FAIL` outcomes; no 0.7.2 archive was fetched and no cancellation PASS exists. | Launch the existing isolated candidate once in `second_cancel` mode, deliberately select **Annuler / Cancel**, and capture: `UPDATE_CANCELLED` PASS, exactly one additional `/second.json` GET, no 0.7.2 archive GET, unchanged tree/executable/marker hashes, and clean QA-process exit. Preserve all three adverse attempts. If the callback still reports acceptance after an unambiguous Cancel click, diagnose that native dialog behavior rather than weakening the oracle. No broad-suite rerun is required unless product source changes. | Native test owner, then fresh independent evidence check |

## Gate conclusion

Code and security reviewers approved the exact source, the exact-source suites
are green, packaging now advertises its real macOS floor, and the busy-before,
busy-during, signed install, restart and fresh-prompt behaviors have credible
native evidence. Nevertheless a missing check is not a pass: the agreed 0.7.2
cancel/no-mutation branch has not executed. The test gate for exact source
`ac5253328da3bd2cb6a65e9913c0bbe5b8d3d33b` therefore remains
**CHANGES_REQUESTED**, and public v0.7 publication remains blocked. No Pi
deployment, live GitHub publication, notarization or production install is
claimed.
