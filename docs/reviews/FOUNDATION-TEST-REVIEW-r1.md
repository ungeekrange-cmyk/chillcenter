# Independent Test Execution Review — bounded foundation

- **Review mode:** `EXECUTION_REVIEW`
- **Lane:** approved implementation, bounded foundation review; not full-application acceptance
- **Branch / frozen source:** `codex/implementation` / `9fb4cbc026b72ae0464328700db86ddeefa17f7c`
- **Parent:** `a06d95f29da099a7a93769420fa5ce62a6478581`
- **Reviewer instance:** `/root/foundation_test_review` (`agent-test-reviewer`)
- **Author/reviewer separation:** reviewer did not author or edit candidate source/tests
- **Environment:** macOS 27.0.1 (`26A434`), Apple Silicon `arm64`; rustc 1.99.0 (`b940084d7`), cargo 1.99.0 (`5f94df478`), mpv 0.41.0, FFmpeg 9.0.2
- **Verdict:** `CHANGES_REQUESTED`

The verdict is narrow: the 105-test workspace foundation is green and the current native renderer was proven, but two evidence/reproducibility gaps below prevent approval of the whole bounded candidate. This report does not approve code correctness, security, desktop/Tauri, all stories, G-04, Pi/NAS deployment, or a finished application.

## Reproducible evidence

All candidate commands used `/bin/bash`, `login:false`, workdir `/private/tmp/chillcenter-implementation.6HSJzC/repo`, and:

```text
CARGO_HOME=/private/tmp/video-catalog-cargo
RUSTUP_HOME=/private/tmp/video-catalog-rustup
PATH=/private/tmp/video-catalog-cargo/bin:/opt/homebrew/bin:/usr/bin:/bin
```

| Command | Exit | Duration | Result | Skips/retries | Evidence validity |
|---|---:|---:|---|---|---|
| `cargo test --offline --locked -p chill-catalog -p chill-domain -p chill-server -p chill-client -p chill-player` | 0 | 1.2 s observed | 105 passed: catalog 64, client 9, domain 10, player 7, server 15; 0 failed/ignored/measured/filtered; binary/doc-test targets contained 0 tests | None | Independent exact-SHA run. Player emitted expected local VideoToolbox/kIOSurface fallback diagnostics; test remained green and this is not hardware-acceleration proof. |
| `cargo check --workspace --all-targets --offline --locked` | 0 | 0.9 s observed (`Finished` 0.85 s) | Workspace members check | None | Exact SHA, but root `Cargo.toml` explicitly excludes `apps/desktop` and `native/macos-render-probe`; therefore this is not desktop/native-probe/full-app proof. |
| `cargo check --manifest-path native/macos-render-probe/Cargo.toml --offline --locked` in clean `git archive` of `9fb4cbc...` | nonzero | 0.4 s | Cargo refused because the committed probe lockfile requires update | None | Valid reproducibility failure; candidate was untouched. |
| Same native check with `--offline` after allowing lock refresh only in isolated `/private/tmp/chillcenter-native-review.3IpLXe` | 0 | 32.69 s build | Current probe, client, and player compile | None | Exact source, but regenerated temporary dependency lock, not the committed probe lock. |
| `cargo run --manifest-path native/macos-render-probe/Cargo.toml --offline --locked` in that isolated copy, outside managed AppKit sandbox | 0 | 29.60 s fresh build plus probe | `PROBE_OK`; 320×180 nonzero 172,058/distinct 256; 640×360 nonzero 669,272/distinct 256; seek/pause/resize position `Some(1.25)` | None | Current `9fb4cbc...` source and real AppKit/libmpv/OpenGL execution, but with the temporary refreshed lock described above. No candidate/NAS file was changed. |

No coverage tool was run; no percentage is claimed. No flaky failure or retry occurred.

The coordinator additionally supplied real-HTTPS evidence from unchanged server source at `a06d95f...`: persisted-library GET succeeded and an inactive raw TLS socket was closed at 5002 ms. The `9fb4cbc...` delta touches only `chill-client` and `chill-player`, so the server code is byte-unchanged. This is supporting evidence, not an independently rerun command in this review because no exact command/output artifact was supplied.

## Assertion and risk assessment

| Risk / boundary | Result | Meaningful assertion evidence | Limit |
|---|---|---|---|
| Server address/DNS/NAS root are configurable, not hardcoded | `PASS` | `address_and_mount_are_not_fixed` accepts `10.10.0.8:9443`, `media.home`, and `/mnt/other-nas/films`, then asserts JSON round-trip equality. These differ from example `192.168.1.42` and `/srv/volumes/video`. Public/wildcard listeners are negatively tested; another private IP `192.168.50.9:9443` is accepted. | Syntax/config proof only; no network or NAS mount contacted. |
| Library-root persistence independent of API test | `PASS` | Direct config test starts at `/mnt/initial`, writes `/mnt/new-nas`, reloads from disk and compares complete settings, rejects stale revision/root `/mnt/stale`, then persists `None`. API test independently asserts 200, `/mnt/custom-movies`, revision 1, on-disk `library.json`, and stale-update 409. | API unit route does not reconstruct the full HTTPS process; supplied real-process evidence is supporting only. |
| NAS may be unavailable during configuration | `PASS` for designed behavior | `LibrarySettings::update` performs syntactic path validation only; tests use nonexistent fixture roots and succeed, while missing-root catalog behavior is separately tested. | Actual reconnect, mount loss, catalogue preservation on Pi/NAS are `NOT_RUN`. |
| Ordered resume/domain remediation | `PASS` at store seam | Test asserts early/pause/seek resume persistence without watch credit; related profile isolation, reassociation, pagination, rating clear, playlist atomicity, and disabled unsafe import tests pass. | No HTTP lease/progress endpoint or full journey exists. |
| Strict HTTP range validation | `PASS` at pure client seam | Tests reject wrong unit, start/end, inverted/out-of-total/unknown ranges, mismatched `Content-Length`, short body, and extra body; exact valid bounds pass. | No real server media endpoint/E2E range stream exists. |
| libmpv ABI and renderer lifetime | `PASS` in this environment | Tests reject API 1.106, accept 2.x, and assert renderer free precedes core termination while renderer remains usable after dropping `Player`; installed libmpv was present, so the conditional test body executed. Current-source AppKit probe also passed. | The tests can silently return on hosts without installed libmpv; CI must report that precondition explicitly. Tauri `NSView` integration remains absent. |
| Stream resource bounds | `PARTIAL` | The 1 MiB allocation cap is asserted. Static inspection shows a zero-capacity channel, one worker per source, and `Drop` joining it. | No behavioral regression test proves one-worker/no-queue behavior, refusal while a read is active, or worker join after cancellation; see `TEST-F-001`. |
| Desktop application | `NOT_RUN` / excluded | `apps/desktop` contains a manifest/build script and `fn main() {}` only and is excluded from the workspace. | No Tauri config, UI, `.app`, desktop build, or app smoke; workspace green must not be presented as app proof. |
| Linux ARM64 / Raspberry Pi 3 / NAS / Wi-Fi / 2–3 profiles / RSS-CPU-throughput budgets | `NOT_RUN` | Pi is explicitly unavailable; no physical target or NAS was touched. | Required before the corresponding story/NFR claims or G-04, not for this local foundation unit result. |

## Findings and routing

| ID | Type | Evidence | Required action | Route |
|---|---|---|---|---|
| `TEST-F-001` | Missing high-risk regression coverage | `stream_callback_request_allocation_is_capped` proves only the length helper. None of the 7 player tests instantiate `HttpReadWorker` or assert its single-worker/zero-queue/drop-join behavior under an active/cancelled request. Static inspection is not an execution oracle for this prior resource-exhaustion risk. | Add a deterministic worker-lifecycle regression using a controlled blocking transport seam: prove at most one active request/buffer, a concurrent/repeated read cannot queue or spawn, cancellation returns, and drop joins after the bounded request completes. Do not use timing-only sleeps as the oracle. | Builder/test author, then exact-SHA test re-review. |
| `TEST-F-002` | Evidence reproducibility | In a clean archive of `9fb4cbc...`, the native probe fails immediately with `--offline --locked` because `native/macos-render-probe/Cargo.lock` requires seven dependency updates. The probe passes only after refreshing that lock in an isolated copy. | Refresh and commit the probe lockfile intentionally, then rerun the locked compile and real AppKit probe against the new frozen SHA. | Builder/integration owner, then exact-SHA test re-review. |

## Evidence limitations

- Uncommitted `docs/implementation/STATUS.md` and `FOUNDATION-EVIDENCE.md` are drafts, not candidate content; they still cite older SHA/count evidence (`be831395...`, 96 tests) and must not be used as exact evidence for `9fb4cbc...`/105 tests.
- The shared working tree showed an uncommitted refresh of `native/macos-render-probe/Cargo.lock` after the frozen-SHA checks. It is not part of `9fb4cbc...` and was not used to convert the committed-lock failure into a pass; a new committed SHA requires re-review.
- The historical `11fb917...` probe ledger was inspected but is not substituted for current-source proof; the reviewer reran the current source as recorded above.
- Configuration examples remain configurable examples. Neither `192.168.1.42` nor `/srv/volumes/video` was treated as a fixed deployment target.
- No source/test repair, deployment, real NAS access, or full-app claim was performed by this reviewer.
