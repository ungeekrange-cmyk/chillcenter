# BUG-0016 — Saga mutations cross the stable/legacy identity boundary incorrectly

Status: ROOT_CAUSE_CONFIRMED. Diagnostic source: d745c4a (backend c814ae4).
Environment: macOS Apple Silicon, actual Tauri UI, isolated HTTPS server localhost:18546,
synthetic MP4/AVI fixtures; no NAS or household data accessed.

Reproduction: run `CHILL_UI_QA_MODE=catalog` with the isolated fixture. Create a saga,
then add the stable media UUIDs returned by the profile catalogue.
Actual: HTTP404, “Cet élément n'existe plus”; expected: films added at expected revision.
Raw result: `UI_QA_RESULT false saga commune UI: Cet élément n'existe plus. Rechargez le catalogue.`

H1 (only hypothesis, CONFIRMED): the new saga Store methods pass public stable UUIDs
directly to the legacy catalogue, whose media primary keys are relative filenames.
`Store::saga_members_at_revision` forwards unchanged IDs; `Database::saga_members_at_revision`
calls `self.get(media)` on the legacy media table. Read-only fixture SQLite experiment:
two identity mappings, zero media rows whose key equals stable UUID, two media rows whose
key equals mapped legacy ID. Saga membership remains empty after the failed transaction.
The inverse problem also exists in saga DTO/order: membership IDs are returned as legacy IDs.

Causal correction authorized: convert stable → legacy on member/order input and legacy →
stable on saga DTO output at the Store boundary, keeping conversions and expected revisions
consistent. Do not change catalogue keys, personal identities, files or unrelated APIs.
Distinct builder handoff required; regression must cover actual scanned identities and
membership/removal/order roundtrip plus stale revisions. Re-run the failing Tauri journey.
Prior reviews are not approvals for this corrected candidate.
