# Independent code review — Chill Center interface/server R3

## Review identity

- Candidate: `c2a9d72d634cc7282eb8a20ca9fa138ed92e2fec`
- Narrow correction base: `d361eba6d1b1b24432a4cb793b07538c35b657d0`
- Original slice base: `fe85cea00da8a7e1f35abf0a5f057907eb235b47`
- Repository: `/private/tmp/chillcenter-interface.dbGN1v/repo`
- Mode: independent, source-read-only delta review; only this report was written
- Reused evidence: R1 and R2 findings, source inspection and exact-SHA tests remain valid except where superseded by this six-file correction

## Verdict

**APPROVED** for the bounded Chill Center interface/server slice at exact candidate SHA `c2a9d72d634cc7282eb8a20ca9fa138ed92e2fec`.

No actionable correctness, compatibility, security-boundary, or maintainability findings remain in the reviewed delta. The R2 MEDIUM typed-error finding is closed, so all five findings from the initial review are now closed.

## R2 finding closure

`Database::get` now maps only `rusqlite::Error::QueryReturnedNoRows` to the missing-media sentinel. Other SQLite failures use the safe database-error sentinel, and malformed stored JSON is treated as corrupt storage rather than a client value or missing namespace (`crates/chill-catalog/src/database.rs:69-79`). The established typed adapter then maps this to 503 `DATABASE_OFFLINE`; it does not expose SQL, paths, row contents, or provider detail.

The regression coverage is proportional and reaches both relevant boundaries:

- Catalogue tests distinguish a genuinely absent row, malformed JSON, and a failed query after the temporary test table is removed (`crates/chill-catalog/src/database.rs:969-987`).
- The real rating HTTP route is exercised after the personal/profile namespace assertions and returns 503 `DATABASE_OFFLINE`, not 404 `MEDIA_NOT_FOUND`, for a catalogue read failure (`crates/chill-server/src/api.rs:837-854`).

The direct `rusqlite` server dependency is test-only and reuses the already locked workspace crate; it adds no runtime dependency or provider.

## Additional delta evidence

- The new failed-initial-approval regression proves provisional trust is removed from memory, is not persisted to `client.json`, leaves no connected state, and creates no remembered server after the follow-on connection fails (`crates/chill-client/src/ui.rs:773-803`). This confirms the already reviewed rollback flow without adding a production test seam.
- `docs/debug/BUG-0008-api-read-errors.md` accurately records the hypothesis, affected route, safe correction, test-only destructive fixture, and bounded rollback status.
- `git rev-parse HEAD` returned the exact candidate SHA and `git status --short` was empty.
- `git merge-base d361eba6d1b1b24432a4cb793b07538c35b657d0 HEAD` returned the R2 SHA; the delta is one commit and six files.
- `git diff --check d361eba6d1b1b24432a4cb793b07538c35b657d0..c2a9d72d634cc7282eb8a20ca9fa138ed92e2fec` passed.
- Independent `cargo test --workspace --locked --offline` passed in a separate target: **124 tests, 0 failures**, plus all doc tests.
- Independent `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` passed in the separate target.

## Prior findings at final SHA

1. Profile-switch view/persistence rollback — **CLOSED**.
2. Queued and started mutation cancellation/deadline enforcement — **CLOSED**.
3. Typed safe API/client errors, including catalogue rating read failures — **CLOSED**.
4. Visible, connection-persistent expired-key replacement notice — **CLOSED**.
5. Atomic catalogue, scan-marker, and stable-identity snapshot — **CLOSED**.

## Approval boundary

This approval covers only the reviewed interface/server slice: typed Tauri bridge and connected UI, HTTPS JSON transport/trust/profile state, profile/catalogue APIs and personal-state mutations, pagination/rename CAS, scan worker and SQLite coordination, plus the development-only macOS bundle files. It does not approve player integration, leases/streaming/progress, mutable import, TMDB/provider workflows, AI, final libmpv packaging, Pi deployment, production release, or the full product; their documented `NOT_READY` status remains unchanged.
