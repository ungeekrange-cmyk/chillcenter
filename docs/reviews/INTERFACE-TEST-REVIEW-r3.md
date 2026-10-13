# Operational delta test review r3 — CC-UI-001

- **Review type / reviewer:** `EXECUTION_REVIEW` / `/root/ui_test_review`
- **Lane:** `DEV_STANDARD`
- **Parent reviewed SHA / verdict:** `d361eba6d1b1b24432a4cb793b07538c35b657d0` / `APPROVED` in `/private/tmp/chillcenter-interface-test-review-r2.md`
- **New source SHA:** `c2a9d72d634cc7282eb8a20ca9fa138ed92e2fec`
- **Branch / base:** `codex/interface-server` / `fe85cea00da8a7e1f35abf0a5f057907eb235b47`
- **Author / test author:** `/root`
- **Reviewer independence:** no production or test authorship; report-only
- **Environment:** Apple Silicon `arm64`, macOS 27.0.1 (26A434), rustc 1.99.0 (b940084d7), cargo 1.99.0
- **Current verdict:** `APPROVED`

## Source delta and prior-evidence validity

The delta from `d361eba` changes catalog `Database::get` so only `QueryReturnedNoRows` maps to media-not-found; other SQLite failures and corrupt JSON map to database failure/HTTP 503. It adds storage/API regressions, a failed-initial-approval trust rollback regression, one existing `rusqlite` server dev-dependency lock entry, and BUG documentation. Production desktop code, frontend assets, packaging script, API success behavior, and approved r2 contracts are unchanged.

R2 evidence remains valid for unaffected setup/profile/admin/catalog behavior. Exact-SHA focused tests, the full suite, strict lint, build, all three headed GUI journeys, and packaging were nevertheless rerun for `c2a9d72`.

## Reviewer-executed evidence

Commands ran from `/private/tmp/chillcenter-interface.dbGN1v/repo` with `/bin/bash` non-login and `CARGO_HOME=/private/tmp/video-catalog-cargo RUSTUP_HOME=/private/tmp/video-catalog-rustup CARGO_TARGET_DIR=/private/tmp/chillcenter-ui-review-target PATH=/private/tmp/video-catalog-cargo/bin:/opt/homebrew/bin:/usr/bin:/bin` unless shown otherwise.

| Exact command | Exit | Duration | Result | Skips/retries |
| --- | ---: | ---: | --- | --- |
| `git diff --check d361eba6d1b1b24432a4cb793b07538c35b657d0..c2a9d72d634cc7282eb8a20ca9fa138ed92e2fec` | 0 | <1 s | Clean narrow delta | N/A |
| `cargo test --locked -p chill-catalog missing_media_is_distinct_from_failed_or_corrupt_catalogue_reads` | 0 | 23.19 s cold | 1 passed, 0 failed, 64 filtered | 0 ignored; no retry |
| `cargo test --locked -p chill-client failed_initial_connection_discards_provisional_trust_and_never_persists_it` | 0 | 2.86 s | 1 passed, 0 failed, 17 filtered | 0 ignored; no retry |
| `cargo test --locked -p chill-server http_catalog_queries_are_paged_and_personal_state_is_isolated` | 0 | 10.48 s | 1 passed, 0 failed, 19 filtered | 0 ignored; no retry |
| `cargo test --locked --workspace --all-targets` | 0 | 10.76 s | 124 passed, 0 failed (65 catalog + 18 client + 11 domain + 10 player + 20 server) | 0 ignored; no retry/quarantine |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | 0 | 3.97 s | PASS | N/A |
| `CARGO_NET_OFFLINE=true cargo build --locked --offline -p chillcenter-desktop -p chill-server` | 0 | 34.49 s | PASS | N/A |
| `/usr/bin/plutil -lint '/private/tmp/chillcenter-interface.dbGN1v/Chill Center-r3.app/Contents/Info.plist'` | 0 | <1 s | `OK` | N/A |
| `/usr/bin/codesign --verify --deep --strict --verbose=2 '/private/tmp/chillcenter-interface.dbGN1v/Chill Center-r3.app'` | 0 | <1 s | Valid; designated requirement satisfied | N/A |
| `/usr/bin/shasum -a 256 '…/Chill Center-r3.app/Contents/MacOS/chillcenter-desktop'` | 0 | <1 s | `6aad62ca783248d365f84c7df8322de44f0b2d36bfce35e49ebae0617b0f1102` | N/A |

The bundle is 9.8 MiB and its executable is arm64 Mach-O. The all-targets player test emitted the same macOS/VideoToolbox fallback diagnostics as r2 and passed without retry; this does not expand approval into UI playback.

## Assertion quality

| Delta risk | Evidence and oracle | Result |
| --- | --- | --- |
| Missing media versus failed read | Storage test separately asserts absent ID → media-not-found, corrupt JSON → generic database error, and dropped `media` table → generic database error | PASS |
| HTTP classification | After earlier successful personal-state and namespace assertions, the test drops only the temporary fixture's `media` table; rating then must return 503 with safe `DATABASE_OFFLINE`, not 404 `MEDIA_NOT_FOUND` | PASS |
| Failed initial approval | An isolated pending certificate is approved toward an unreachable endpoint; failure must leave no connection, restore the exact in-memory trust store, preserve the reopened persisted trust store, and keep server mappings empty | PASS |
| Regression scope | Full 124-test all-targets suite plus strict clippy and locked offline desktop/server build | PASS |

These tests assert distinct externally meaningful outcomes and post-failure state; they are not mock-only status checks. Temporary data is isolated. No source/test modifications, skipped critical checks, retries, quarantines, or flakiness occurred during review.

## Exact-SHA headed GUI and package evidence

The author ran a fresh fixture `/private/tmp/chillcenter-ui-delivery.9F69kK` with server session `66432`:

`/private/tmp/chillcenter-implementation.6HSJzC/repo/target/debug/chill-server --config /private/tmp/chillcenter-ui-delivery.9F69kK/server.json`

| Literal Tauri command | Session / exit | Result |
| --- | --- | --- |
| `CHILL_UI_QA_URL=https://127.0.0.1:22443 CHILL_UI_QA_DATA=/private/tmp/chillcenter-ui-delivery.9F69kK/client CHILL_UI_QA_ROOT=/private/tmp/chillcenter-ui-delivery.9F69kK/videos /private/tmp/chillcenter-implementation.6HSJzC/repo/target/debug/chillcenter-desktop` | `68416` / 0 | `UI_QA_RESULT true connect-consent-profiles-library-scan-personal-state` |
| Same plus `CHILL_UI_QA_MODE=reopen_delete` | `74166` / 0 | `UI_QA_RESULT true reopen_delete` |
| Same plus `CHILL_UI_QA_MODE=renewal_notice` | `81227` / 0 | `UI_QA_RESULT true renewal_notice` |

Before the third mode, only the isolated remembered pin was changed (expiry `1`, one SPKI byte flipped), as in r2. This remains a controlled expired-key simulation rather than real calendar/certificate rotation.

The author packaged the exact SHA with:

`CARGO_HOME=/private/tmp/video-catalog-cargo RUSTUP_HOME=/private/tmp/video-catalog-rustup CARGO_TARGET_DIR=/private/tmp/chillcenter-implementation.6HSJzC/repo/target CARGO_NET_OFFLINE=true PATH=/private/tmp/video-catalog-cargo/bin:/opt/homebrew/bin:/usr/bin:/bin bash scripts/package-macos.sh '/private/tmp/chillcenter-interface.dbGN1v/Chill Center-r3.app'`

Result: exit 0 in 2m48s, `plutil`/strict codesign valid. Independent verification above confirms the bundle and hash. Desktop bytes matching r2 are expected: the new client change is under `cfg(test)`, while the production catalog correction is server-side.

Fixture inspection found client/server directories `0700`, state files `0600`, one media row, only recreated `QA C`, no personal-state row, and a 51-byte fake `.mp4`. It proves scanner/catalog wiring, not decoding.

## Limitations / NOT_RUN

- R2 limitations remain: no coverage instrumentation; invalid category and >500-character query lack dedicated execution; accessibility/notarization/distribution, real expiry/annual rotation, Pi/NAS/Wi-Fi/resources, and real NAS scan are not run.
- No physical disk failure or corrupted household database was induced. SQLite/JSON failures use disposable fixtures and a dropped temporary table.
- Playback, streaming, leases/progress, native Tauri player, import/export completion, AI, seasonal ambience, and G-04/full-product acceptance remain outside this slice.
- The fake `.mp4` is scanner-only evidence.

This accelerated approval is immutable and bound to source SHA `c2a9d72d634cc7282eb8a20ca9fa138ed92e2fec`. Any source change makes it stale.
