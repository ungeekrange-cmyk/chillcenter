# BUG0131 — measured progress survives a server rollback

- Date: 2026-10-21; priority P2; owner `/root`.
- Environment: local candidate `8283667`, shared Mac/web progress UI.
- Status: **FIXED / VERIFIED_LOCAL**, source `e54eb357`.
- Found independently by `/root/mac15_progress_code`; never deployed to the Pi.

## Evidence and correction

H1 confirmed: first load measured resumes from a compatible server; then return
404/405 on a forced refresh (older server after rollback). The response was
marked unsupported but its previous entries/revision and localStorage key were
retained. Bars and the In progress category could indefinitely show stale data.

On 404/405, only for the still-current server/profile and pending request, clear
entries and revision, remove that scoped cache key, then redraw. Keep the cache
on transient network/503 errors; an explicit retry can detect a later upgrade.
No server data, other profile or catalogue state is removed.

Regression: seed a successful measured response, render details, then test
404, 405 and 503 independently; verify cache/bar/detail/redraw behavior and
successful retry with the correct revision. Eight progress tests PASS. Code
and security R2 reviews APPROVED for exact source `e54eb357`.

FR : un ancien serveur ne doit pas laisser des pourcentages orphelins en cache.
Purge limitée au profil/serveur concerné sur 404/405 ; données hors ligne
conservées en cas d'erreur transitoire. Correction et tests locaux uniquement.
