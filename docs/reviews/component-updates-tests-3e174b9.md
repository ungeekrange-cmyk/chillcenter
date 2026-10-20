# Independent test operational delta — component updates `3e174b9`

- **Review mode / lane:** `EXECUTION_REVIEW` / `FULL_ASSURANCE`
- **Parent reviewed SHA / verdict:**
  `ac5253328da3bd2cb6a65e9913c0bbe5b8d3d33b` /
  `CHANGES_REQUESTED` in `component-updates-tests-ac52533.md`
- **Exact new source:** `codex/release-v0.7-fixes` /
  `3e174b9e6ed89e08f67e763c3b30028c15767080`
- **Source delta:** manual DMG packager and five focused tests only;
  application runtime, dependencies and updater logic unchanged
- **Native QA seam:**
  `7efb31c1e196c53cfa231dbcf043bf6ff1bc75ef`
- **Independent reviewer:** `/root/v07_final_tests`
- **Environment:** macOS 27.0.1 (26A434), arm64; Python 3.9.6;
  Rust/Cargo 1.99.1; Node 24.13.0
- **Verdict:** **APPROVED**

The only blocking item from the parent review is closed. On the fourth fresh
0.7.2 prompt after the successful 0.7.1 SDK replacement/restart, the user
explicitly selected **Cancel**. The exact production consent helper returned
the cancellation path, the QA process exited 0, the journal recorded its
machine PASS, only the manifest was requested, and the complete isolated app
tree remained identical to the pre-attempt reference. The three earlier
accepted callbacks remain preserved as adverse `FAIL` history.

The new source delta adds only a standard manual-install DMG generator and its
tests. The generated image passed checksum verification and a real read-only
mount. Its embedded app has a valid deep strict signature and is byte-for-byte
identical to the previously reviewed exact-runtime app. No test, source or
evidence in this review contacts the Pi, NAS, GitHub or a production install.

## Reproducible evidence

| Command / evidence | Exit | Duration | Result | Skips / retries | Scope and limit |
| :--- | :---: | :---: | :--- | :--- | :--- |
| `python3 scripts/helpers/test_macos_dmg.py -v` | 0 | 0.005 s test body | 5 passed, 0 failed | 0 | Independent exact-source rerun of all new DMG tests |
| `/private/tmp/chillcenter-v07-dmg-helper-tests.log`, SHA-256 `27ef650b…5acfd` | 0 reported | 1.475 s | 19 passed, 0 failed | 0 | Combined Linux/Mac packaging-helper regression; includes the same five DMG cases |
| `/private/tmp/chillcenter-v07-dmg-doc-tests.log`, SHA-256 `fb187811…494d` | 0 reported | 116.24 ms | 32 passed, 0 failed | 0 | FR/EN navigation, link/privacy and deterministic public-artifact checks |
| `hdiutil verify Chill-Center-0.7.0-macos-arm64.dmg` | 0 | not captured | image checksum **VALID** | 0 | Independent verification of the actual DMG, not a mocked command |
| Read-only mount inspection at `/Volumes/Chill Center` | 0 | not captured | root contents, symlink, instructions, metadata, signature and full app tree PASS | 0 | Existing real HFS attachment inspected; no launch or installation |
| `/private/tmp/chillcenter-qa-native-updates.YNFPqL/qa-native-updates.log`, SHA-256 `23d2f0de…67e3e` | 0 observed by coordinator | duration not captured | cancellation PASS after three preserved adverse FAIL attempts | 0 retries counted as passes | Real AppKit prompt and production helper; disposable QA app/key/loopback only |
| `/private/tmp/chillcenter-qa-native-updates.YNFPqL/network.count`, SHA-256 `5e164ad0…4b3e2` | N/A | N/A | six requests total; zero 0.7.2 archive requests | N/A | Append-only request oracle for the whole native journey |
| Parent exact-runtime suites | 0 reported | recorded in parent report | Rust 435 passed/0 failed/2 established NFR ignores; Node 359 passed/0 failed | 2 manual NFR ignores, not counted as passes | Reused because `ac52533..3e174b9` has zero application/runtime/dependency delta |

The committed and working-tree blobs of both new Python files are identical.
`git diff ac52533..3e174b9` contains exactly
`scripts/package-macos-dmg.py` and `scripts/helpers/test_macos_dmg.py`; a
runtime-path diff over `apps`, `crates`, `deploy`, manifests and lockfiles is
empty. The product and QA copies of `update_bridge.rs` remain byte-identical
(SHA-256 `e7ebc359…8d2`), so the now-completed native evidence exercises the
same consent logic present at the exact final source.

## Risk-to-evidence matrix

| Risk / requirement | Layer and oracle | Result | Assertion quality and evidence |
| :--- | :--- | :--- | :--- |
| `CUAC-TEST-001`: cancel a fresh later update without mutation | Native E2E: user clicks Cancel; helper returns `UPDATE_CANCELLED`; no archive GET; exact bundle unchanged | **CLOSED / PASS** | Journal line 14 is `PASS production native consent cancelled; no install or SDK download`. The ledger gained exactly one fourth `/second.json` GET and still has no 0.7.2 archive GET. Coordinator observed session 47805 exit 0. |
| Preserve the installed 0.7.1 QA app on cancel | Native E2E post-state: exact full-tree hash, marker, executable and signature | **PASS** | Independent post-check reproduced tree hash `385446ab…ab8f1`, exactly the pre-attempt reference; executable remains `bf3ef05b…ff690`; marker remains `qa native update fixture v0.7.1`; deep strict codesign passes. |
| Preserve adverse interaction history | Evidence integrity: failed attempts remain visible | **PASS** | Journal retains all three `FAIL second-release consent was accepted unexpectedly` lines before the final prompt/PASS. They were not reclassified as passing retries. |
| Create a usable standard manual installer | Packaging integration: real UDZO/HFS+ image verifies and mounts read-only | **PASS** | DMG size 46,881,214 bytes, SHA-256 `7c352bdb…47a5`; `hdiutil verify` is valid. Mount flags include `read-only`, `nodev` and `nosuid`. |
| Ship only the intended DMG root context | Mounted-artifact inspection: app, `/Applications` link and bilingual instructions only | **PASS** | Root contains exactly `Chill Center.app`, `Applications` and `Installation.txt`; link target is exactly `/Applications`. Instructions identify Apple Silicon, macOS 27.0+, drag-to-Applications flow, ad-hoc signature and absence of notarization. |
| Preserve the reviewed app inside the image | Real-boundary comparison: signature, metadata and complete-tree equality | **PASS** | Mounted app is 0.7.0/build 9/minimum macOS 27.0; deep strict codesign passes. `diff -rq` against `/private/tmp/chillcenter-v07-minos-ac52533/Chill Center.app` is empty; an independent deterministic digest is identical on both trees (`327c85cc…f06c`). |
| Never overwrite an existing or racing output | Unit/filesystem boundary: existing file, dangling symlink and late race remain intact | **PASS** | Tests inspect retained bytes/symlink and assert exclusive hard-link publication fails on an existing target; successful publication remains valid after the staging link is removed. |
| Reject wrong input identity and recursive destination | Unit boundary: QA identifier and output under the input app raise errors | **PASS** | Assertions execute `validate_paths` and require `ValueError`; normal identifier/version/minimum path passes. |

## Quality and limitations

- The new unit assertions validate observable filesystem state, not only mocked
  command strings. The successful packaging path is additionally exercised by
  an actual image, checksum verification, read-only mount, deep signature check
  and full-tree comparison.
- The packager verifies the source and copied app before image creation and
  publishes only after `hdiutil verify`. The focused tests do not inject each
  possible `codesign`/`hdiutil` process failure; source ordering and exclusive
  publication make this a non-blocking residual test limitation.
- One independent attempt to create a second attachment with the deprecated
  `hdiutil attach -mountpoint` interface returned `Périphérique non configuré`
  while the image was already attached. It is not counted as a pass or product
  failure. `hdiutil verify` succeeded independently, and the existing real
  read-only attachment supplied the mounted-artifact assertions.
- No coverage percentage was supplied; approval rests on branch, boundary and
  artifact assertions. The two inherited ignored Rust NFR harnesses remain
  ignored, explicitly outside this packaging/native-consent delta.
- The DMG is intentionally ad-hoc signed and not Apple-notarized; the mounted
  instructions disclose both facts. This review does not claim notarization,
  another-Mac/older-macOS compatibility, live GitHub download, or production
  installation.
- A loopback QA HTTP fixture and the read-only DMG attachment were still active
  when inspected. They are local, non-production evidence resources and may be
  stopped/ejected by their owning coordinator after evidence capture; this is
  cleanup, not a release-gate failure.

## Gate conclusion

`CUAC-TEST-001` is closed by the genuine final Cancel attempt and exact
no-download/no-mutation post-state. The new DMG delta has focused negative
filesystem tests plus a real verified, read-only mounted artifact whose app is
identical to the reviewed runtime candidate. The independent test gate for
exact source `3e174b9e6ed89e08f67e763c3b30028c15767080` is therefore
**APPROVED**.

This approval is test evidence, not an action to publish, tag, sign with a
production key, merge, or deploy. Those operations still require the distinct
exact-source code/security and release/integration gates. No Pi/NAS operation
is required or authorized by this report.
