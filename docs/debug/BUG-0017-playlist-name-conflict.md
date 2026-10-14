# BUG-0017 — Duplicate playlist rename incorrectly reports storage unavailable

Status: ROOT_CAUSE_CONFIRMED. Backend source: 064c5d4; Tauri regression repeats on
f7e634a plus the pending transfer UI. Environment: isolated localhost HTTPS fixture,
macOS Apple Silicon, no household data.

Observed: catalog QA passed playlist create/add/order then timed out on rename with
“Serveur occupé ou base indisponible”. Repeated fixture used the same destination name.

H1 (only hypothesis, CONFIRMED): the requested name is already used for this profile.
The SQLite UNIQUE(profile_id,name) constraint in `cc_playlists` is returned via db_error
by `rename_playlist_at_revision`, and therefore mapped to DATABASE_OFFLINE rather than a name conflict.
Read-only listing shows an existing “Playlist renommée”; a certificate-verified direct
PUT with revision3 and that name returns DATABASE_OFFLINE. Follow-up GET proves old name, order and
revision3 are intact. An ordinary new playlist creation succeeds, disproving general
storage unavailability within this experiment.

Authorized causal fix: detect/name the duplicate within the same CAS transaction and
map it to a conflict/recovery message, preserving rollback. Distinct builder to add
same-profile duplicate failure + different-profile same-name success regressions.
QA fixture rename must be unique on repeat; this correction is not evidence that
production should allow duplicate playlist names. Re-run actual Tauri workflow.
