# Independent code review — Chill Center interface/server R2

## Review identity

- Candidate: `d361eba6d1b1b24432a4cb793b07538c35b657d0`
- Prior reviewed candidate: `b8737bba65f010e349a8387201faf8d73cebe2b0`
- Base: `fe85cea00da8a7e1f35abf0a5f057907eb235b47`
- Repository: `/private/tmp/chillcenter-interface.dbGN1v/repo`
- Mode: independent, source-read-only review; only this report was written
- Scope: typed Tauri bridge and connected UI, client JSON transport/trust/profile state, profile and catalogue APIs, pagination/personal state/rename CAS, scanner worker and SQLite coordination, and the development-only macOS bundle additions
- Explicitly excluded from approval: player integration, leases/streaming/progress, mutable import, TMDB/provider flows, AI, final libmpv packaging, Pi deployment and the full product

## Verdict

**CHANGES_REQUESTED** for this bounded slice at the exact candidate SHA.

Four of the five findings from the initial `b8737bb` review are closed, and most of the typed-error finding is corrected. One current HTTP path still misclassifies database read failures as a missing media namespace, so the typed error boundary is not yet complete.

## Finding

### MEDIUM — The rating path reports database read failures as `MEDIA_NOT_FOUND`

`Database::get` maps every `rusqlite::Connection::query_row` error to the same missing-media string, rather than limiting that conversion to `rusqlite::Error::QueryReturnedNoRows` (`crates/chill-catalog/src/database.rs:69-74`). The current rating endpoint reaches this legacy helper through `Store::set_rating` -> `Store::media` -> `catalogue.get` (`crates/chill-domain/src/store.rs:420`, `crates/chill-domain/src/store.rs:295-299`, `crates/chill-server/src/catalog_api.rs:82-90`). The safe boundary then maps that string to `StoreFailure::MediaNotFound` (`crates/chill-domain/src/lib.rs:18-29`) and HTTP 404 `MEDIA_NOT_FOUND` (`crates/chill-server/src/api.rs:129-140`). The desktop consequently tells the user that the item disappeared and applies 404 refresh recovery (`crates/chill-client/src/ui.rs:101-107`, `ui/connected/app.js:7-9`) even when the actual cause is SQLite I/O, corruption, schema, or another read failure that should be `DATABASE_OFFLINE`/503.

Impact: a recoverable storage failure is presented as a permanent namespace deletion. This defeats the new operation-specific error contract on a user-visible personal-state mutation and can cause misleading refresh/retry behavior.

Action: make `Database::get` return the missing-media sentinel only for `QueryReturnedNoRows`; route every other `rusqlite` error through the database-error sentinel. Add a regression that forces a non-`QueryReturnedNoRows` catalogue read failure and proves the rating API returns 503 `DATABASE_OFFLINE`, while an actually absent stable/media mapping remains 404 `MEDIA_NOT_FOUND`.

## Prior finding disposition

1. **Profile selection namespace atomicity — CLOSED.** The selected profile is staged, its prospective view is fetched before persistence, and both active profile and remembered choice are restored on view or save failure (`crates/chill-client/src/ui.rs:478-511`, `584-599`). The regression covers request and persistence failure (`crates/chill-client/src/ui.rs:812-855`).
2. **Queued/started mutations committing after timeout — CLOSED.** Actor jobs have a ten-second budget, closed queued receivers are skipped, and cancellation/deadline probes are installed as SQLite progress and pre-commit hooks on both connections (`crates/chill-server/src/api.rs:72-115`, `crates/chill-domain/src/store.rs:311-337`, `crates/chill-catalog/src/database.rs:862-885`). Regression coverage exercises both queued and already-started mutations (`crates/chill-server/src/api.rs:385-437`).
3. **Typed API/client error state — PARTIALLY CLOSED; finding above remains.** Safe bounded codes are now parsed and mapped without exposing server detail, and the added tests distinguish missing profile/media namespaces. The retained `Database::get` mapping leaves the rating read path incorrect.
4. **Expired-key replacement not visibly signaled — CLOSED.** A replacement event becomes a connection-scoped notice included in every connected view and rendered independently from transient messages (`crates/chill-client/src/ui.rs:460-475`, `725-728`, `ui/connected/app.js:11-13`, `ui/connected/index.html:4`). Unit and real-UI QA assertions distinguish same-key renewal and retain the banner across catalogue/admin refreshes.
5. **Catalogue and stable identities committed separately — CLOSED.** `snapshot_with` writes media, `last_scan`, and identities on one SQLite transaction and commits once (`crates/chill-catalog/src/database.rs:821-859`, `crates/chill-domain/src/store.rs:307-309`, `967-971`). Trigger fault injection proves rollback of catalogue, scan marker, and identities (`crates/chill-domain/src/store.rs:1107-1149`).

## Additional bounded review evidence

- Exact identity and cleanliness: `git rev-parse HEAD` returned the candidate SHA; `git status --short` was empty.
- Delta hygiene: `git diff --check b8737bba65f010e349a8387201faf8d73cebe2b0..d361eba6d1b1b24432a4cb793b07538c35b657d0` passed.
- `cargo test --workspace --locked --offline` passed in an isolated target: 122 unit tests, 0 failures, plus all doc tests (64 catalog, 17 client, 11 domain, 10 player, 20 server). This includes queued/started cancellation, 2,000-row locked snapshot responsiveness, identity fault rollback, profile-switch request/save rollback, filter discrimination, namespace isolation, and visible renewal notice tests.
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` passed in the isolated target.
- The scan worker is bounded to one active job, has capped output and a 300-second discovery-process budget, holds the configured root stable against concurrent configuration changes, and performs snapshot persistence on a separate blocking writer connection (`crates/chill-server/src/catalog_api.rs:102-175`). WAL readers remain demonstrably responsive during the held scan transaction.
- The Tauri bridge remains one typed, single-flight command surface; JavaScript cannot supply arbitrary paths or headers (`apps/desktop/src/main.rs:6-40`, `crates/chill-client/src/ui.rs:17-71`).
- `apps/desktop/Info.plist` and `scripts/package-macos.sh` are accurately labeled and constrained as Apple Silicon development-only packaging. They do not claim notarization, installer readiness, or final player/libmpv delivery.

## Approval boundary

This verdict is only for the frozen interface/server slice. It does not alter the documented `NOT_READY` status of player/leases/import/TMDB/AI work and is not a production, Pi, full-product, or final macOS distribution approval.
