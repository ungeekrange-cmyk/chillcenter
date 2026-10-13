# Independent test execution review r2 — CC-UI-001

- **Review mode:** `EXECUTION_REVIEW`
- **Lane:** `DEV_STANDARD`
- **Candidate:** `codex/interface-server` at `d361eba6d1b1b24432a4cb793b07538c35b657d0`
- **Base:** `fe85cea00da8a7e1f35abf0a5f057907eb235b47`
- **Prior reviewed candidate:** `b8737bba65f010e349a8387201faf8d73cebe2b0` (`CHANGES_REQUESTED`)
- **Author / test author:** `/root`
- **Reviewer:** `/root/ui_test_review` (independent; no source/test edits)
- **Environment:** Apple Silicon `arm64`, macOS 27.0.1 (26A434), rustc 1.99.0 (b940084d7), cargo 1.99.0
- **Verdict:** `APPROVED`

Approval is limited to the corrected setup/profile/admin/catalog interface-server slice at the exact candidate SHA. The two r1 findings are closed by discriminating HTTP filter assertions and comprehensive negative profile/media namespace assertions. This is not Pi, playback, release-distribution, or G-04/full-product approval.

## Reviewer-executed evidence

Rust commands ran from `/private/tmp/chillcenter-interface.dbGN1v/repo` using `/bin/bash` non-login and:

`CARGO_HOME=/private/tmp/video-catalog-cargo RUSTUP_HOME=/private/tmp/video-catalog-rustup CARGO_TARGET_DIR=/private/tmp/chillcenter-ui-review-target PATH=/private/tmp/video-catalog-cargo/bin:/opt/homebrew/bin:/usr/bin:/bin`

| Exact command | Exit | Duration | Result | Skips/retries |
| --- | ---: | ---: | --- | --- |
| `git diff --check b8737bba65f010e349a8387201faf8d73cebe2b0..d361eba6d1b1b24432a4cb793b07538c35b657d0` | 0 | <1 s | Clean delta | N/A |
| `cargo test --locked -p chill-server --lib` | 0 | 6.63 s | 20 passed, 0 failed | 0 ignored; no retries/quarantine |
| `cargo test --locked -p chill-client -p chill-domain --lib` | 0 | 8.92 s | 28 passed, 0 failed (17 client + 11 domain) | 0 ignored; no retries/quarantine |
| `cargo test --locked --workspace --all-targets` | 0 | 9.12 s | 122 passed, 0 failed (64 catalog + 17 client + 11 domain + 10 player + 20 server) | 0 ignored; no retries/quarantine |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | 0 | 4.52 s | PASS | N/A |
| `/opt/homebrew/bin/node --check ui/connected/app.js` | 0 | 0.17 s | PASS | N/A |
| `/opt/homebrew/bin/node --check ui/connected/qa.js` | 0 | 0.03 s | PASS | N/A |
| `/bin/bash -n scripts/package-macos.sh` | 0 | <0.01 s | PASS | N/A |
| `/usr/bin/plutil -lint '/private/tmp/chillcenter-interface.dbGN1v/Chill Center.app/Contents/Info.plist'` | 0 | <1 s | `OK` | N/A |
| `/usr/bin/codesign --verify --deep --strict --verbose=2 '/private/tmp/chillcenter-interface.dbGN1v/Chill Center.app'` | 0 | <1 s | Valid on disk; satisfies designated requirement | N/A |

The inspected development bundle was 9.8 MiB with an arm64 Mach-O executable. Its enclosing target path is shared build output; the source working directory for the recorded build/package commands was the frozen candidate checkout, not the separate checkout that owns that target directory.

The all-targets player test emitted a macOS `kIOSurfaceMethodSetCoreVideoBridgedKeys` diagnostic and still passed its generated-media assertions without retry. It is historical player evidence, not proof that this UI slice's fake scan fixture decodes.

## Author-executed headed GUI and package evidence

The author built the exact clean source with `cargo build --locked --offline -p chillcenter-desktop -p chill-server` (exit 0), using `/private/tmp/chillcenter-implementation.6HSJzC/repo/target` only as the target directory. Server session `80130` ran:

`/private/tmp/chillcenter-implementation.6HSJzC/repo/target/debug/chill-server --config /private/tmp/chillcenter-ui-final.1G1tI9/server.json`

From the frozen candidate source directory, all three headed commands used the real Tauri DOM → Rust `ui_command` → HTTPS/verifier → server path:

| Literal command | Session / exit | Observed result |
| --- | --- | --- |
| `CHILL_UI_QA_URL=https://127.0.0.1:21443 CHILL_UI_QA_DATA=/private/tmp/chillcenter-ui-final.1G1tI9/client CHILL_UI_QA_ROOT=/private/tmp/chillcenter-ui-final.1G1tI9/videos /private/tmp/chillcenter-implementation.6HSJzC/repo/target/debug/chillcenter-desktop` | `97243` / 0 | `UI_QA_RESULT true connect-consent-profiles-library-scan-personal-state` |
| Same command plus `CHILL_UI_QA_MODE=reopen_delete` | `42996` / 0 | `UI_QA_RESULT true reopen_delete` |
| Same command plus `CHILL_UI_QA_MODE=renewal_notice` | immediate / 0 | `UI_QA_RESULT true renewal_notice` |

Before the third run only the isolated fixture `client.json` was changed: the remembered certificate expiry became `1` and one stored SPKI byte was flipped. This is a controlled expired-old-key simulation, not an actual 18-month wait/rotation. The run still exercised the current real certificate, HTTPS verifier, persisted trust event, and visible DOM banner across catalog/admin refresh.

The author also ran:

`CARGO_HOME=/private/tmp/video-catalog-cargo RUSTUP_HOME=/private/tmp/video-catalog-rustup CARGO_TARGET_DIR=/private/tmp/chillcenter-implementation.6HSJzC/repo/target CARGO_NET_OFFLINE=true PATH=/private/tmp/video-catalog-cargo/bin:/opt/homebrew/bin:/usr/bin:/bin bash scripts/package-macos.sh '/private/tmp/chillcenter-interface.dbGN1v/Chill Center.app'`

Result: exit 0 in 57.82 s; author-reported `plutil` and strict codesign verification were independently repeated above.

Fixture inspection after the GUI runs found `0700` client/server directories, `0600` client/SQLite/library files, one catalog row, only recreated profile `QA C`, and no personal-state row. `videos/Films/QA.2020.mp4` is 51 bytes and proves scanner/catalog wiring only.

## Assertion-quality and risk matrix

| Risk / behavior | Meaningful oracle | Result |
| --- | --- | --- |
| r1 `TEST-F-001`: query filters | 51 film + 2 series fixtures; exact film and one-episode matches; cross-category and nonmatching searches return empty; pagination independently remains 50+1 | PASS / finding closed |
| r1 `TEST-F-002`: namespace confusion/deletion | Valid media UUID as profile, valid profile UUID as media, random missing UUID, and deleted profile across catalog/watched/rating/preferences assert 404 plus safe typed code | PASS / finding closed |
| CAS | Current rename succeeds, stale rename returns 409; library config unit verifies failed stale write leaves prior value | PASS |
| Privacy/isolation | HTTP path is relative; profile B does not inherit A's watched/rating/preferences; client retains bounded safe API code and drops server detail | PASS |
| Failed profile switch | Failed prospective read and unavailable settings directory preserve displayed profile, in-memory last choice, and persisted last choice | PASS |
| Timed-out mutations | Both queued and already-started mutations time out and are proven absent after worker release | PASS |
| Scan responsiveness | A deliberately open 2,000-row WAL snapshot transaction coexists with a bounded interactive profile read | PASS |
| Scan/identity atomicity | Injected identity trigger failure preserves catalog contents, `last_scan`, and identity count | PASS |
| Certificate replacement visibility | Same-key renewal is not mislabeled; expired-key replacement creates a visible persistent notice in unit and headed GUI evidence | PASS for controlled simulation |
| Setup/profile/admin/catalog journey | Consent refuse/accept, root config, scan, CRUD/rename, personal state, reconnect, deletion cancellation/delete-last/recreate all have DOM state assertions | PASS |

The assertions validate returned status/code, persisted or absent state, inclusion/exclusion, and rollback outcomes; they are not tautological count-only checks. Temporary directories isolate test data. No retries, quarantines, ignored tests, or unexplained failures were observed.

## Limits / NOT_RUN

- No line, branch, or changed-code coverage instrument was supplied or run; percentages are not used as an approval basis.
- Invalid category and a search string longer than 500 characters are validated in source but do not have a dedicated executed boundary assertion. The shipped static UI constrains these values; this review does not elevate source inspection to executed proof.
- The renewal GUI case is a controlled stored-state mutation, not calendar passage, real certificate expiry, or annual production rotation.
- Accessibility audit, notarization/distribution, release QA hook absence at runtime, Linux/Pi/NAS/Wi-Fi/resource measurements, and a real NAS scan are `NOT_RUN`.
- Playback, media streaming, leases/progress, native Tauri player, import/export completion, AI, seasonal ambience, and global acceptance/G-04 remain outside this bounded slice.
- The fake `.mp4` is not video decode or playback evidence.
- `scripts/agent-context.py` remains absent as recorded in project context; no output was invented.

This report is immutable and bound to `d361eba6d1b1b24432a4cb793b07538c35b657d0`. Any source change makes this approval stale and requires a new exact-SHA review.
