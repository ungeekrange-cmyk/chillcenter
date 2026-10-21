# ADR0035 — measured resume view / Vue des reprises mesurées

- Status: PROPOSED implementation detail under explicit user feature request.
- Author/owner `/root`; formal ADR review not performed; date2026-10-21.
- Implementation code/security reviews: APPROVED at `e54eb357`; see delivery record.
- Branch `codex/macos15-build`; complements ADR0014/0022, supersedes NONE.
- Authority: user requested “En cours” >=5%, thumbnail bars, detail percentage;
  rejected TMDB estimates and requested measuring duration at first playback.

## Context and choice

Personal state stores position, not duration. Catalogue sync intentionally ignores
position-only changes. Neither thousands of per-card requests nor a full catalogue
refresh on every progress event is acceptable for the Pi3.

Keep existing watched/lease rules. On accepted `Progress`, persist exact duration
transactionally in additive `cc_resume_measurement`, linked to the personal-state
composite key with delete/update cascade. No decoder/probe runs on the server.
Existing rows remain untouched and have no invented percentage until measurement.
No legacy API response field or alias is removed. Database schema3 remains
readable by older binaries; the extra table is unused by them. Backups include it;
legacy portable exports do not carry measurements (re-measure after import).

Add a profile-scoped batch read on the existing2026-10-14 API:
`GET /profiles/{profile}/playback-progress?known_revision=...`. Return at most30000
known resumes with exact duration, a boot-scoped opaque token, and lightweight
unchanged replies. Catalogue revision/cache is separate. Native HTTPS executes
outside UI mutex/single-flight; web uses the same allowlisted command and origin.
Clients cache only measured values per server/profile, poll at30s while visible,
refresh after playback/mutations and discard stale-context replies. A404 from
released0.7.2 disables only this feature, not the app; manual retry remains possible.

## UI and proof

“En cours” contains individual unwatched videos/episodes with position/duration
>=5%; unknown duration is excluded. Any started unwatched video with measured
duration gets a thumbnail bar, including below5%; detail displays a floored
integer percentage. A grouped show's arbitrary representative is not a show-wide
percentage. Series playback preserves its opaque series context for autoplay.
The mobile More sheet exposes the category without crowding the bottom rail.
FR/EN labels use existing locale machinery; no new theme or visual system.

Tests: measured-only/5% boundary/watched/profile scoping, late replies, unchanged
tokens, no per-card requests, artwork replacement retaining bars, API validation,
actual DB persistence/reopen and catalogue epoch isolation. Additive-table rollback
keeps original personal/catalogue data; do not delete measurements on rollback.
No Pi upgrade is implicit: this feature needs a later authorized server delivery.
