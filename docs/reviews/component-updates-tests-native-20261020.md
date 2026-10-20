# Independent native test execution delta — component updates `3913267`

- **Review mode / lane:** `EXECUTION_REVIEW` / `FULL_ASSURANCE`
- **Reviewed product source:** `codex/release-v0.7-fixes` /
  `3913267e98f82eeca5f7c627490fbdd019636177`
- **Documentation carrier:** `90e7fcc6b553b7caefd6135a543461b4387d6b70`
  (`3913267..90e7fcc` changes documentation/evidence only; no product, unit,
  script, dependency, UI, theme or update-key source delta)
- **Parent verdict:** `CHANGES_REQUESTED` in
  `component-updates-tests-3913267.md`, with only `CU391-TEST-001` open
- **Native QA seam:** `54dd9689a99657bba9a189366a8a29830cd99509`
  (`qa-native-updates` debug-only feature and local loopback/test-key harness)
- **Independent reviewer:** `/root/v07_final_tests`
- **Environment:** macOS 27.0.1 (26A434), Apple Silicon; Tauri 2.12.3;
  `tauri-plugin-updater` 2.14.0; Minisign 0.12
- **Verdict:** **CHANGES_REQUESTED**

The new evidence materially closes the native SDK/package part of the prior
finding. A real AppKit prompt was cancelled by the user without replacing the
isolated app; malformed signature bytes were rejected; a correctly signed but
AppleDouble-contaminated archive failed installation while preserving the old
executable; the corrected signed archive was verified and installed by the
real Tauri updater SDK; the user accepted the separate restart prompt; and the
replacement executable actually relaunched and detected its marker.

The remaining release gap is narrower but still material: this feature-gated
QA path calls the updater SDK directly. It does not execute the production
`UpdateHost::install` orchestration, and therefore does not behaviorally assert
the playback-busy guard either before the prompt or when playback starts while
the prompt is open. It also does not offer a later signed release after restart
to prove a fresh install consent. Those checks were explicit parts of the
previous HIGH finding. Their source guards and closed renderer DTO are useful
code/unit evidence, but missing runtime assertions are not passes.

## Evidence inspected

| Evidence / check | Exit / result | Assertion quality and limit |
| :--- | :--- | :--- |
| User confirmation on 20 October 2026: clicked **Annuler** on the first real AppKit install prompt | **PASS_HUMAN_SCOPED** | The process ended, no update marker existed and the coordinator recorded the executable SHA-256 unchanged at `5fc70968…526c8f`. The QA journal contains only `PROMPT native consent queued` for this attempt, not `PASS native consent cancelled`; this report relies on the explicit human observation plus post-state and does not invent a machine cancellation event. |
| `/private/tmp/chillcenter-qa-native-updates.LiX6KA/qa-native-updates.log`, SHA-256 `402684cf…40c5d` | Lines 1–7 preserved; bad-package PASS, AppleDouble FAIL, valid install PASS, restarted executable PASS | Append-only outcome log from the isolated bundle. Durations were not captured. |
| Real invalid-signature package path | **PASS** | `tauri-plugin-updater::Update::download` rejected the deliberately invalid signature and the QA harness compared the complete pre/post bundle tree hash before logging `PASS bad-package rejected; QA bundle hash preserved`. |
| Signed AppleDouble archive `/private/tmp/chillcenter-qa-native-updates.LiX6KA/appledouble-failure.app.tar.gz`, SHA-256 `2773c27f…db94` | Expected install failure | Python raw-member inspection shows 474 members, 237 AppleDouble entries and two apparent app roots. The SDK failed unpacking `._Chill Center QA.app`; the coordinator recorded the old executable SHA-256 still `5fc70968…526c8f`. This is retained adverse evidence, not a passing installer attempt. |
| Corrected signed archive `/private/tmp/chillcenter-qa-native-updates.LiX6KA/Chill-Center-QA_0.7.1_aarch64.app.tar.gz`, SHA-256 `ef40f41d…2f092` | **PASS** | Raw `tarfile` inspection: 237 members, exactly one `Chill Center QA.app` root, zero `._` members and marker present. Minisign verification with the ephemeral QA public key passed. `54dd968` changes only the QA archive command (`COPYFILE_DISABLE=1`, no xattrs/ACLs/fflags) plus this raw-member regression check; the production packaging path already used `COPYFILE_DISABLE`. |
| Real install/restart, same QA journal lines 5–7 | **PASS** | The user clicked the actual native **Installer maintenant** and **Redémarrer** prompts. SDK verification/replacement completed, then a new process appended `PASS restarted actual QA executable after isolated replacement`. Reported launch session exited 0. |
| Installed isolated QA bundle | **PASS** | Current executable SHA-256 `d8fc9de7…f7eb5` exactly equals the staged executable; arm64 Mach-O; deep strict ad-hoc codesign verification passes; installed marker SHA-256 `867cb922…6509` contains `qa native update fixture v0.7.1`. |
| QA isolation/key separation | **PASS** | Root is user-owned mode `0700`; ephemeral private test key is mode `0600`; endpoint allowlist is fixed to `127.0.0.1:18765`; QA and production public-key bytes differ (`f4f60ed8…5e4e` versus `6a16e189…a8cc`). No GitHub, Pi, NAS or installed production app was contacted. |
| Product/QA source equivalence for the exercised updater | **PASS_SCOPED** | `apps/desktop/src/update_bridge.rs`, `apps/desktop/tauri.conf.json` and `Cargo.lock` have identical Git blobs at `3913267` and `54dd968`. The only linked library delta is a functionally neutral borrow cleanup in `chill-updates/install.rs`; normal `main.rs` is wrapped by the feature-only QA branch. The QA dialog/orchestration module itself is not the production `UpdateHost::install` path. |
| Prior broad evidence | unchanged | 8/8 exact-source desktop updater tests, 433 Rust PASS/2 established NFR ignores, 359 Node PASS, exact-source desktop Clippy, Linux VM/systemd and root-ledger proofs remain valid because neither the carrier nor QA archive correction changes product source. |

## `CU391-TEST-001` closure matrix

| Required behavior | Result | Evidence / limitation |
| :--- | :--- | :--- |
| Native cancel; no installation without the click | **PASS_HUMAN_SCOPED** | Explicit user click on **Annuler**, process ended, no marker, unchanged executable hash. No machine `PASS cancelled` line was emitted, so only the human/post-state claim is made. |
| Native accept and real SDK signature/package verification | **PASS** | Real AppKit acceptance plus bad signature, signed failure archive and signed valid archive through Tauri updater SDK. |
| Failed installation preserves current app | **PASS_SCOPED** | AppleDouble failure is genuine and the executable hash remained exact. Full-tree post-failure hash was not captured independently. |
| Successful replacement and restart consent | **PASS** | Separate actual restart prompt accepted; installed executable equals stage; new executable relaunched and marker path executed. |
| Busy before consent | **NOT_RUN** | Production source checks `rendering` at entry, before SDK construction and again after download, but the native QA feature has no player/rendering state and does not call this function. |
| Playback becomes busy while consent dialog is open | **NOT_RUN** | Production source rechecks `rendering` immediately after `receive.await`; no behavioral test toggles it during a real prompt and proves zero mutation. |
| Fresh consent for a later update after restart | **NOT_RUN** | The second observed prompt was restart consent. On relaunch the QA marker short-circuits to PASS; no newer signed fixture was offered. Renderer-supplied consent remains rejected by the exact-source unit test, but that does not execute a second native install prompt. |

## Finding and routing

| ID | Severity / type | Evidence | Required action | Route |
| :--- | :--- | :--- | :--- | :--- |
| `CU391-NATIVE-001` | **HIGH — production orchestration branches remain unevidenced** | The genuine SDK/AppKit seam bypasses `UpdateHost::install`; busy-before, busy-during-dialog and later fresh-consent rows above are `NOT_RUN`. | Add a test seam around the actual production Mac install orchestration, or execute an equivalent isolated native candidate journey: (1) rendering already true rejects before prompt/download and preserves the bundle; (2) rendering changes false→true while the consent dialog is open, then acceptance returns `UPDATE_BUSY` with no install; (3) after restart, offer a newer signed fixture and prove a new install consent is required before mutation. Record exact source/bundle hashes, outcomes and post-state. | Builder/test author and native test owner, then fresh independent test delta review |

## Gate conclusion

The evidence is credible and valuable: native AppKit interaction, SDK signature
rejection, failure preservation, signed replacement and actual restart are no
longer missing. However `CU391-TEST-001` is only **PARTIALLY CLOSED** because
three explicitly required production-orchestration branches remain `NOT_RUN`.
The exact-source test gate therefore remains **CHANGES_REQUESTED** for public
v0.7 publication. This verdict does not require Pi/NAS or live GitHub testing,
and it does not invalidate the existing Linux, Rust, Node or security approvals.
