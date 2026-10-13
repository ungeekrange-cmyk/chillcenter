# Independent test execution review — CC-UI-001

- **Review mode:** `EXECUTION_REVIEW`
- **Lane:** `DEV_STANDARD`
- **Candidate:** branch `codex/interface-server`, source SHA `b8737bba65f010e349a8387201faf8d73cebe2b0`
- **Base:** `fe85cea00da8a7e1f35abf0a5f057907eb235b47`
- **Test author / delivery owner:** `/root`
- **Reviewer:** `/root/ui_test_review` (independent; no source or test edits)
- **Environment:** Apple Silicon `arm64`, macOS 27.0.1 (26A434), rustc 1.99.0 (b940084d7), cargo 1.99.0
- **Verdict:** `CHANGES_REQUESTED`

The bounded setup/profile/admin/catalog slice builds cleanly and its 114-test workspace regression suite passes. The two headed debug-WebView journeys also passed on the exact SHA. Approval is withheld because the current filter test oracle cannot detect ignored `q`/`category` parameters, and negative namespace/unknown-media behavior is not exercised at the new HTTP boundary.

## Reproducible reviewer evidence

All Rust commands used `CARGO_HOME=/private/tmp/video-catalog-cargo`, `RUSTUP_HOME=/private/tmp/video-catalog-rustup`, `CARGO_TARGET_DIR=/private/tmp/chillcenter-ui-review-target`, and `PATH=/private/tmp/video-catalog-cargo/bin:/opt/homebrew/bin:/usr/bin:/bin` from `/private/tmp/chillcenter-interface.dbGN1v/repo` with `/bin/bash`, non-login.

| Command | Exit | Duration | Result | Skips/retries |
| --- | ---: | ---: | --- | --- |
| `git diff --check fe85cea..b8737bba` | 0 | <1 s | Clean | N/A |
| `cargo test --locked -p chill-client -p chill-domain -p chill-server --lib` | 0 | 68.34 s cold build | 40 passed, 0 failed | 0 ignored; no retries/quarantine |
| `cargo test --locked --workspace --all-targets` | 0 | 66.49 s | 114 passed, 0 failed (64 catalog, 13 client, 10 domain, 10 player, 17 server; zero-test example/binaries also executed) | 0 ignored; no retries/quarantine |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | 0 | 54.23 s | PASS | N/A |
| `/opt/homebrew/bin/node --check ui/connected/app.js` | 0 | 0.33 s | PASS | N/A |
| `/opt/homebrew/bin/node --check ui/connected/qa.js` | 0 | 0.03 s | PASS | N/A |
| `stat -f '%Sp %N'` on the fresh QA client/server directories and state files | 0 | <1 s | Client/server directories `0700`; `client.json`, SQLite and `library.json` `0600` | N/A |
| `sqlite3 …/server/catalog.sqlite 'select … from cc_profiles; select … from cc_profile_state; select count(*) from media;'` after the second journey | 0 | <1 s | Only recreated profile `QA C`; no personal-state row; one catalog row | N/A |

The all-targets player test emitted VideoToolbox fallback diagnostics but passed its real generated-media stream/seek/track assertions; this is neither a retry nor evidence for the UI slice's fake scan fixture.

## Author-supplied headed GUI evidence

The delivery owner reported two exact-`b8737bb` runs against fresh fixture `/private/tmp/chillcenter-ui-proof.FXWHAX` and loopback HTTPS port 20443:

| Session / mode | Observed output | Exit | Scope |
| --- | --- | ---: | --- |
| `9968`, initial journey | `UI_QA_RESULT true connect-consent-profiles-library-scan-personal-state` | 0 | Real headed Tauri DOM → allowlisted Rust command → pinned HTTPS server: refuse/accept consent, root config, scan, profile CRUD/rename, watched/rating/preferences isolation |
| `18931`, `reopen_delete` | `UI_QA_RESULT true reopen_delete` | 0 | Reopen/last-profile restoration, cancel deletion, delete other and last profile, recreate/select profile |

Source inspection confirms `qa.js` is injected by the debug Tauri host under `cfg(debug_assertions)` and drives the production DOM/Tauri invoke seam; it is not a browser-side transport mock. The owner identified executable `/private/tmp/chillcenter-implementation.6HSJzC/repo/target/debug/chillcenter-desktop` with `CHILL_UI_QA_URL=https://127.0.0.1:20443`, `CHILL_UI_QA_DATA=/private/tmp/chillcenter-ui-proof.FXWHAX/client`, `CHILL_UI_QA_ROOT=/private/tmp/chillcenter-ui-proof.FXWHAX/videos`, empty first mode, and `CHILL_UI_QA_MODE=reopen_delete` for the second run. The complete historic wrapper command lines and durations were unavailable. Therefore the GUI evidence is accepted as observed author evidence, not claimed independently reproducible from a complete literal command in this handoff.

The scan fixture `Film.2020.mp4` is 42 bytes. It proves discovery/catalog flow only, not decoding, playback, media streaming, leases, progress, or native-player integration.

## Assertion assessment

| Risk / contract | Evidence and oracle | Result |
| --- | --- | --- |
| HTTPS-only editable server address; no arbitrary URL/path | `addresses_are_editable_but_not_arbitrary_urls`, `endpoint_rejects_http_and_path_escape`, headed consent journey | PASS |
| Consent/profile selection cannot be forged | `profile_selection_and_consent_are_not_caller_forgeable` plus headed refuse/accept flow | PASS |
| Closed opaque-ID/request seam | `opaque_ids_cannot_inject_paths_or_headers`; `UiCommand` is a deny-unknown-fields tagged enum | PASS within tested unit seam |
| Profile/library CAS | API tests assert current revision succeeds and stale revision returns 409; config unit also asserts stale write leaves the prior root | PASS |
| Missing profile returns 404 | API test asserts `/profiles/missing/preferences` returns 404 | PASS only for this route; broader namespace cases are missing |
| Pagination | 51 rows produce 50 items with `has_more=true`, then one item with `has_more=false` at offset 50 | PASS |
| Catalog path privacy | HTTP assertion requires returned path not start with `/`; legacy export test excludes `/nas/` | PASS |
| Personal watched/rating/preferences isolation | HTTP test asserts watched remains false for profile B; headed journey observes A's watched/rating/preferences and B's defaults | PASS |
| Device-local theme and last profile | Theme reopen unit plus headed reopen journey | PASS |
| Query filtering | Existing HTTP data all have category `film` and titles containing `Film`; assertions only count pages. Domain test likewise uses all-film data and does not assert title exclusion. | **NOT PROVED** |
| Scan success | Headed subprocess scan discovers one fake `.mp4` and refreshes the catalog | PASS for success path only |

## Findings

### TEST-F-001 — catalog filter assertions are non-discriminating

`api::tests::http_catalog_queries_are_paged_and_personal_state_is_isolated` sends `category=film&q=Film`, but all 51 fixtures match both predicates and the oracle checks only page size/`has_more`. `store::tests::catalogue_page_is_bounded_and_filters_in_sqlite` also uses only film rows and does not exercise a title filter. Both tests would pass if the implementation ignored `category` and `q`, while the change card claims a filtered catalog.

**Required action:** the builder/test author should add mixed categories and matching/nonmatching titles, then assert inclusion and exclusion at the HTTP boundary; cover invalid category and the documented 500-character query bound. The reviewer must not author or repair these tests.

### TEST-F-002 — negative profile/media namespace coverage is incomplete

The only new HTTP 404 assertion is a missing profile on `/preferences`. There is no executed negative evidence for missing profiles on catalog/watched/rating, nor for an unknown media UUID on watched/rating, including response code/body and no-state-change postconditions. This matters to the UI's stale-profile/element refresh behavior and to the change card's `HTTP … 404` claim.

**Required action:** define the expected status for unknown media (404 if it is an absent resource; otherwise document the contract) and add HTTP-boundary tests for the negative profile/media namespace cases with no-write assertions. Route to the builder/test author.

## Limits and checks not run

- No line/branch/changed-code coverage instrumentation exists in the supplied evidence; no percentage is treated as a pass.
- Accessibility audit, release-bundle QA, Linux/Pi/NAS/Wi-Fi/resource measurements, real video decode/playback, leases/progress, import/export, AI, and seasonal ambience are `NOT_RUN` and outside this bounded slice.
- Scan concurrency/root-freeze/failure-retains-catalog HTTP cases were not independently executed; the headed run covers only one successful scan. Existing lower-layer tests cover capped subprocess output and empty-scan preservation, not the new route's concurrent conflict behavior.
- A supplementary reviewer curl probe to port 20443 was `NOT_RUN`: the review sandbox could see an existing host listener but could not connect; an attempted local `cargo run --locked -p chill-server --bin chill-server -- --config /private/tmp/chillcenter-ui-proof.FXWHAX/server.json` exited 1 because the port was already occupied. This tooling observation does not contradict the owner's successful host GUI runs.
- `scripts/agent-context.py` is absent in this repository, as already recorded in `docs/PROJECT_CONTEXT.md`; no context output was fabricated.

This report is frozen to source SHA `b8737bba65f010e349a8387201faf8d73cebe2b0`. Any remediation commit requires a new exact-SHA execution review; this report must not be relabeled current.
