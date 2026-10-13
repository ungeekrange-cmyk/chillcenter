# Independent domain code review

- Reviewer instance: `/root/domain_review`
- Date: 2026-10-13
- Baseline: `181f2c295be4a439720ac8b0e7e12a733569edee`
- Candidate: `f064f4c02e575fb4787ab929af4e6b2dafce8a51`
- Scope: `crates/chill-domain/src/store.rs`, `crates/chill-domain/src/types.rs`
- Verdict: `CHANGES_REQUESTED`

## Delta and reused evidence

`f064f4c` is a direct child of `181f2c2`. The scoped delta adds `store.rs` (1,023 lines) and changes only derives/formatting in `types.rs` (+26/-6); no prior exact-SHA code review was supplied. Reused current accepted evidence: `docs/architecture/CONTRACTS.md` (`CC-DOC-001`), ADR-0002, ADR-0006, and approved profile/state/import stories US-002/003/004/013/014/021/022/027/028. No security or test verdict is claimed.

Read-only/static evidence: `git diff --check` passed. Diagnostic `cargo test -p chill-domain --offline` was run against a separate `/private/tmp` copy because the source has no lockfile; 6 tests passed. This is not test-review certification.

## Findings

1. **HIGH — Progress accepts stale/replayed and unqualified watch credit.** `store.rs:45-52` persists none of the contract-required last session/sequence state, while `store.rs:296-325` accepts only profile/media/position/elapsed/duration and blindly adds caller-supplied `final_elapsed_ms`. It cannot reject an old device after a newer session, duplicate sequence, inactive generation, seek/buffering time, disallowed speed, or elapsed time outside the final tenth. This violates `CONTRACTS.md:13,54-56`, US-021-AC-04, and US-022-AC-02/03. **Required:** make the state write consume validated session/generation/sequence/playback facts, atomically compare/store ordering state, require the active media revision/lease, and validate bounded final-tenth elapsed before updating position/counter.

2. **HIGH — “Stable” media IDs change when a file is renamed or moved.** `store.rs:215-219,720-725` assigns a fresh UUID from the current legacy ID/path and has no reassociation path. Because the scanner's legacy ID is the relative path, a rename creates a new UUID while history/playlists remain attached to the unavailable old identity. This violates ADR-0002:20-26 and F-16. **Required:** preserve UUID independently of location and implement the accepted explicit root/reassociation workflow (reliable identity/full proof or human confirmation); update location without replacing identity.

3. **HIGH — Import silently drops all exported common metadata and saga data.** Export includes scrubbed common media data at `store.rs:515-525`, but import reads only `id` and `relative_path` at `store.rs:573-595`; the `data` object is never validated or applied. US-028-AC-01 requires imported common metadata/sagas, while personal data goes to the selected profile. **Required:** include common catalogue changes in the prepared plan, validate revisions/conflicts, and apply them atomically with the personal import while never mutating source files/videos.

4. **HIGH — Import overwrites newer destination state and permits implicit profile merging instead of previewing conflicts.** `store.rs:599-619` only checks that every source profile has some destination; it does not require an injective mapping or compare provenance/revisions. The upserts at `store.rs:624-645` unconditionally replace preferences and state, and playlist conflicts are first discovered during the write at `store.rs:647-676`. `cc_imports` stores only `export_id` (`store.rs:75`), so there is no state-level provenance to decide “destination newer.” This violates US-028-AC-03 and ADR-0006:20. **Required:** produce a complete pre-apply conflict plan, reject/explicitly resolve many-to-one profile maps, retain source/revision provenance, and apply only the confirmed plan in one transaction.

5. **HIGH — Export is neither a coherent snapshot nor bounded/streamed.** `store.rs:515-526,771-793` reads catalogue, profiles, states, and playlists in separate statements across two SQLite connections; concurrent writes can therefore produce a mixed-time archive. It also materializes the complete catalogue and personal state as `Vec<Value>`/one JSON `Value`, contrary to the Pi memory constraint and the mandated streaming export (`CONTRACTS.md:67-68`, US-027-AC-03). **Required:** coordinate a consistent database snapshot and stream the manifest/data to staging/output with enforced size/error handling; never label a partial result restorable.

6. **MEDIUM — Catalogue access ignores the 100-item/page database-filtering contract.** `store.rs:193-200` calls `Database::list()` and returns the entire catalogue; series rating at `store.rs:257-265` also scans the entire profile view. This violates `CONTRACTS.md:28,62` and is material on the 1 GiB Pi. **Required:** expose bounded page/filter queries and perform grouping/filtering in SQLite; reserve full iteration for explicitly streamed administration jobs.

7. **MEDIUM — Removing a rating cannot clear it.** `set_rating(..., None, ...)` passes the requested clear at `store.rs:237-270`, but `upsert_state` uses `rating.flatten().or(old.rating)` at `store.rs:837-848`, restoring the old value. This fails US-013-AC-01. **Required:** distinguish “unchanged” from “set NULL” and persist NULL for an explicit removal, including series/season scope.

8. **MEDIUM — Playlist member removal is not atomic and does not advance the playlist revision.** `store.rs:429-444` autocommits the delete, then `store.rs:764-770` renumbers in a separate transaction; a normalization failure leaves a partial operation, and neither path increments `cc_playlists.revision`. This violates the atomic ordered/revisioned playlist contract (`CONTRACTS.md:16,20`). **Required:** delete, renumber safely, and bump revision in one immediate transaction with expected-revision/conflict handling.

## Scoped assessment

Profile-keyed FKs and ownership checks generally isolate personal rows, profile deletion leaves the common catalogue intact, and successful personal import writes plus its `export_id` are transactionally committed together. Those positives do not offset the contract failures above. `types.rs` introduces no additional behavioral finding in this delta.

**Routing:** return to the domain builder for localized/structural corrections; any new source SHA makes this verdict stale and requires a current review delta.
