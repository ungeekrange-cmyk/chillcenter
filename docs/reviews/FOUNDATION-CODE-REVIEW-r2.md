# Chill Center foundation code delta review — R2

- **Verdict:** `CHANGES_REQUESTED`
- **Candidate:** `9fb4cbc026b72ae0464328700db86ddeefa17f7c`
- **Prior reviewed candidate:** `be83139559144dec1fd7e55be8b900ab1c1d6036`
- **Scope:** bounded re-evaluation of the prior foundation correctness blockers. This does not review or approve all 33 stories, the unfinished desktop application, the deferred progress/export HTTP features, or the separately reviewed server-security delta.

## Exact-candidate evidence

- `git rev-parse HEAD` resolved to the candidate SHA. The source was frozen; unrelated documentation/review files were dirty and excluded from candidate evidence.
- `cargo test -p chill-domain -p chill-client -p chill-player --locked --offline` passed: domain 10, client 9, player 7, plus doc tests. This is reviewer sanity evidence, not a substitute for independent test certification.
- `cargo check --workspace --all-targets --locked --offline` passed. `Cargo.toml:1-4` now explicitly limits the buildable foundation workspace to the five crates and excludes `apps/desktop` and the standalone native probe.
- `git diff --check` reported only a blank line at EOF in the excluded `native/macos-render-probe/Cargo.toml`; no reviewed foundation-source whitespace error was reported.

## Historical finding disposition

| Prior finding | Disposition at `9fb4cbc` | Evidence |
| --- | --- | --- |
| F-001 progress rejected normal resume/pause/seek/stop | **CLOSED** | `crates/chill-domain/src/store.rs:452-500` now persists every ordered lease-validated position and admits watched credit only for uninterrupted final-tenth playback. Regression coverage is at `:1161-1211`. The lease arbiter and HTTP progress endpoint remain absent, so this is a correct internal seam, not a completed capability. |
| F-002 renderer could outlive core/library | **CLOSED** | `Player` and `OpenGlRenderer` share `Rc<PlayerCore>` (`crates/chill-player/src/lib.rs:436-446,724-730,797-836`). Renderer drop frees the render context before releasing the final core reference; the core then terminates mpv while its `Library` is still owned. Drop-order coverage is present at `:1148-1222`. |
| F-003 accepted API did not match OpenGL FFI layout | **CLOSED** | The two-field API-2.x layout is documented at `crates/chill-player/src/lib.rs:380-386`, and `supported_client_api` rejects every major version below 2 (`:976-978`). |
| F-004 workspace could not build because desktop skeleton was included | **CLOSED FOR FOUNDATION SCOPE** | `Cargo.toml:1-4` excludes the unfinished desktop and native probe; the locked offline all-target foundation check passes. This is not evidence that the desktop app builds or runs. |
| F-005 misaligned partial responses were accepted | **CLOSED** | `crates/chill-client/src/lib.rs:640-659,683-778` requires a parseable `bytes` Content-Range, matching start, bounded end, coherent total, exact optional Content-Length, and exact observed body length. |
| F-008 unbounded detached thread per media read | **RESOURCE ASPECT CLOSED; NEW R2-F-001 OPEN** | `crates/chill-player/src/lib.rs:208-278` creates one joined worker per open stream, and `:280-331` caps each read at 1 MiB with no queued request. The prior thread/memory accumulation is removed, but its rendezvous dispatch has a correctness race below. |

F-006 (typed API errors/profile expected revision) and F-007 (post-allocation compatibility JSON export cap) remain explicitly `NOT_READY` follow-ons. Neither progress nor export is exposed by the current server router, so no completion or HTTP readiness is inferred.

## New finding

### R2-F-001 — MEDIUM — Idle zero-capacity worker can spuriously reject a read as busy

`HttpReadWorker::start` creates its command channel with `mpsc::sync_channel(0)` (`crates/chill-player/src/lib.rs:229-232`), while `read_at` dispatches with `try_send` and maps `Full` to “une lecture HTTP précédente est encore active” (`:293-310`). For a zero-capacity rendezvous channel, `try_send` fails not only when work is active but whenever the receiver is not already waiting. Immediately after the separate ready rendezvous, and briefly between worker loop iterations, the idle worker may not yet have entered `receiver.recv()`. A legitimate first or subsequent read can therefore fail solely from scheduling and may abort playback. Rust's standard-library contract explicitly says `try_send` returns immediately when “no receiver is waiting”: <https://doc.rust-lang.org/std/sync/mpsc/struct.SyncSender.html#method.try_send>.

**Required:** preserve one-in-flight backpressure without depending on receiver scheduling—for example, use a capacity-one command channel plus explicit in-flight state, or perform a cancellation-aware send/ack handshake. Add a deterministic test that dispatches immediately after worker startup and repeatedly at the completion boundary.

## Scoped verdict and readiness boundary

The prior HIGH blockers are closed, and the candidate is a buildable crate foundation. The native transport/player foundation nevertheless remains `CHANGES_REQUESTED` for R2-F-001 under the code-review rule that discovered correctness issues return to the builder. F-006/F-007 remain deferred and unexposed, import remains disabled, and the desktop/full product remains `NOT_READY`. No security or complete test verdict is inferred.

No source changes were authored by this reviewer; the historical R1 report remains intact.
