# Chill Center foundation code review

- **Verdict:** `CHANGES_REQUESTED`
- **Candidate:** `be83139559144dec1fd7e55be8b900ab1c1d6036`
- **Baseline:** `181f2c295be4a439720ac8b0e7e12a733569edee`
- **Scope:** correctness/compatibility/maintainability review of the domain delta plus the server configuration/API/TLS and native client/player foundation. This is not a security or test-evidence certification, and it does not approve the unfinished desktop application.

## Evidence and delta disposition

Review used the committed Git objects at the exact SHAs above; unrelated uncommitted documentation/server edits appeared after the freeze and were not treated as candidate content. `git diff --check <base> <candidate>` passed. The coordinator's exact-candidate result of 96 passing targeted tests (catalog 64, client 8, domain 9, player 4, server 11, plus doc tests) is reused as reported evidence, not independently certified here. An independent `cargo check --workspace --all-targets --locked --offline` reached the desktop build script and failed because `apps/desktop/tauri.conf.json` is absent (F-004).

The earlier domain review's stable-identity, pagination, rating-clear and atomic playlist-removal findings are closed at the domain seam: explicit administrator reassociation is transactional (`store.rs:298-348`), the SQL page is capped at 100 (`store.rs:226-277`), clearing a rating now distinguishes `Some(None)` from no update (`store.rs:958-973`), and removal/reindex/revision commit together (`store.rs:604-639`). Unsafe import application is now deliberately disabled (`store.rs:740-775`), so the prior provenance/implicit-merge defects are closed by feature exclusion, not by a completed import. Import remains `NOT_READY`. Export is improved to reserve the SQLite writer and preserve profile/common records, but F-007 remains.

## Findings

### F-001 — HIGH — Validated progress rejects valid resume events

`crates/chill-domain/src/store.rs:452-460` rejects every event when playback is paused, buffering, seeking, or before 90% of the duration. Those flags should stop only the final-tenth watched-time accumulation; they do not make the resume position invalid. `docs/architecture/CONTRACTS.md:54-56` and `ARCHITECTURE.md:55-59` require periodic persistence throughout playback and immediate position persistence after pause, seek, and stop, including a deliberate position near zero. As written, ordinary resume cannot be persisted until the final tenth, and final pause/stop acknowledgements are rejected.

**Required:** separate resume-position validation/persistence from eligibility for adding `final_elapsed_ms`; accept ordered events for the active lease throughout the media and require the watched increment to be zero when paused/buffering/seeking/outside the final tenth. Add cases for early playback, pause, seek, stop-near-zero, and final-tenth accumulation.

The old unqualified `save_progress` now fails closed (`store.rs:424-440`), which is correct. However, no lease arbiter or HTTP progress endpoint is present in `crates/chill-server/src/api.rs:88-104`; therefore progress is not exposed unsafely, but the capability is not implemented.

### F-002 — HIGH — Renderer can outlive the mpv core and loaded library

`Player::create_opengl_renderer(&self)` returns an independent `OpenGlRenderer` containing raw context/function pointers (`crates/chill-player/src/lib.rs:591-645`). `Player::drop` destroys the mpv core and then its `Library` field is unloaded (`:674-684`), while a surviving renderer later calls `mpv_render_context_free` through that pointer (`:690-728`). mpv's render API requires the render context to be freed before the core; this ordering can yield undefined behavior and a call through an unloaded library.

**Required:** encode the ownership/lifetime so a renderer cannot outlive its `Player`/library, or give both a shared owner that frees the render context before terminating the core and unloads the library last. Add a compile-time or runtime drop-order regression test. Primary contract: <https://github.com/mpv-player/mpv/blob/master/include/mpv/render.h#L106-L120>.

### F-003 — HIGH — Declared libmpv compatibility does not match the FFI struct layout

`MpvOpenGlInitParams` has only two fields (`crates/chill-player/src/lib.rs:313-317`), but `Player::new` accepts client API 1.106 and newer (`:381-385`). In libmpv 1.x this struct still contains the trailing `extra_exts` field; it was removed only at client API 2.0. A supported 1.x library can therefore read past the Rust allocation during renderer creation.

**Required:** use generated/versioned bindings or provide the compatible trailing null field for the accepted 1.x layout (and prove the 2.x call remains valid), otherwise raise the minimum to API 2.0. Add ABI-version fixture coverage. Primary change record: <https://github.com/mpv-player/mpv/blob/master/DOCS/client-api-changes.rst#L45-L50>.

### F-004 — HIGH — The exact candidate cannot build all workspace targets

The workspace includes `apps/desktop` (`Cargo.toml:1-3`), whose manifest invokes `build.rs` (`apps/desktop/Cargo.toml:1-6`) and whose build script calls `tauri_build::build()`. The exact-candidate all-target check fails because `apps/desktop/tauri.conf.json` does not exist; `apps/desktop/src/main.rs:1` is also only `fn main() {}`.

**Required:** add a valid desktop configuration and entry-point wiring, or explicitly exclude the placeholder app from the buildable foundation workspace until that slice is ready. Re-run the locked all-target workspace check. This finding also prevents any full-application approval independently of the not-yet-run AppKit/native enabler proof.

### F-005 — MEDIUM — Range reads accept a misaligned partial response

`crates/chill-client/src/lib.rs:610-655` accepts any `206` and copies its body into the caller's requested offset. `parse_content_range_total` (`:674-675`) extracts only the total and never verifies the `bytes` unit or returned start/end against the requested range. A server/proxy returning a different partial range will silently feed the wrong bytes to libmpv.

**Required:** strictly parse `Content-Range`, require its start to equal the request start, require a coherent end/body length/total, and reject absent or malformed headers. Cover misaligned, truncated, oversized, and `416` responses.

### F-006 — MEDIUM — Exposed profile writes lose API conflict/not-found semantics

The API actor maps every store failure to `400 INVALID_OPERATION` (`crates/chill-server/src/api.rs:56-68`), although `CONTRACTS.md:36` requires `404` for missing IDs, `409` for revision conflicts, and distinct unavailability. In addition, rename accepts only a name (`api.rs:195-224`); although `Profile` exposes a revision, `Store::rename_profile` performs an unconditional update (`crates/chill-domain/src/store.rs:141-155`). Two clients can silently overwrite one another.

**Required:** use typed domain errors and map them to the contracted HTTP statuses/codes; require `expected_revision` for rename and update with `WHERE id=? AND revision=?`, returning `409` on stale revisions.

### F-007 — MEDIUM — The 1 MiB export guard is post-allocation

`crates/chill-domain/src/store.rs:710-737` first loads all media, identities, sagas, profiles and personal state into vectors/a JSON tree, then serializes a second complete buffer only to test its length (`:721-730`). A large library can exhaust memory well before the nominal 1 MiB rejection. This does not meet the streaming export bound in `CONTRACTS.md:67-68`.

**Required:** keep this helper explicitly test-only/small-data or enforce pre-materialization row/byte bounds; the production export must stream a coherent snapshot to bounded staging/output rather than constructing the archive in RAM.

### F-008 — MEDIUM — Media reads create an unbounded detached thread per callback

`crates/chill-player/src/lib.rs:214-264` describes a bounded worker, but calls `thread::spawn` for every `read_at` and allocates the full request buffer in that thread. Cancellation returns while the request thread remains alive until the transport timeout. Repeated cancellation/seeks can accumulate threads and memory.

**Required:** use a fixed-size worker/executor with bounded queue/backpressure and cooperative request cancellation where possible; cap requested read size and join/drain outstanding work during source teardown.

## Scoped verdict and readiness boundary

- **Domain:** `CHANGES_REQUESTED` because F-001 and F-007 remain. Profile isolation, explicit stable reassociation, SQL pagination, rating clear, and playlist removal are acceptable at their current internal seams. Import is safely disabled and remains `NOT_READY`; no import/provenance/idempotence completion is claimed.
- **Server config/TLS foundation:** no blocking correctness defect found in the static configuration persistence or certificate schedule in this scope. The exposed API still requires F-006, and playback leases/progress/catalog/media routes are absent and must not be advertised as complete.
- **Native transport/player:** `CHANGES_REQUESTED` because F-002, F-003, F-005, and F-008 affect correctness/compatibility. The independent AppKit/native-enabler proof is `NOT_RUN` and remains a readiness requirement.
- **Integrated/full app:** `NOT_READY`; F-004 prevents a workspace-wide build, the desktop entry point is a placeholder, and required playback/catalog/import/export surfaces are incomplete.

No source fixes were authored by this reviewer.
