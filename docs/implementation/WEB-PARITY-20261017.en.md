# Fixes 82–84 and web parity — 17 October 2026

**English** | [Français](WEB-PARITY-20261017.md)

## Scope

Branch `codex/fix-82-84-web-parity`, Mac base `3b2e28b`, web integration
`e6e23db`. Other worktrees, the GitHub v0.6 release, its artifacts and tag are
preserved. The candidate is **0.6.1 / build 8**, without a new GitHub tag or
publication. No Pi/NAS access or deployment is performed in this batch.

- BUG0082: explicit, contextual CAS recovery, drafts retained; no automatic
  mutation retry.
- BUG0083: proportionate, centered search SVG.
- BUG0084: fixed-center sync SVG honoring reduced motion.
- Web: shared Mac views/settings, filters, collections, separate same-name
  shows, TMDB/artwork tools, profiles, languages, identity and archives.
  **No web AI**, explicitly requested by the user. Related titles still use
  local metadata matching.
- IndexedDB mirror: existing deltas, atomic publication, bounded legacy
  fallback and invalidation after associations/artwork; no server API changes.

## Verification and limitations

Commands run:

```sh
node --test ui/connected/*.test.js ui/connected/tests/*.test.js web/test/*.test.js
node --test scripts/test-bilingual-docs.mjs scripts/test-public-site.mjs
node scripts/test-ui-recovery-icons.cjs
node scripts/build-web.mjs /absolute/path/to/owned/web/dist
node scripts/test-web-parity.cjs
```

Browser checks use Playwright/Chrome and a synthetic API, not the household
server: actual shared DOM, built web bridge, IndexedDB, browser artwork picker
and language persistence. They do not validate NAS playback, Safari/Firefox,
real download permissions or human Tauri rendering. Existing codec tests use
local fixtures; no new general player walkthrough is imposed on the user.

Independent review identified nonrepresentative episode selectors and an
unbounded snapshot fallback; both received fixes and regression tests before
handoff. An initial web walkthrough exposed incomplete TMDB invalidation,
fixed before the green run. Chrome sandbox launch failures and a language
fixture overwriting preferences on reload are not server defects.

## Artifacts and rollback

Final build results, hashes and verification counts are recorded in
[the candidate handoff](../releases/REL-WEB-PARITY-001.en.md).
The published v0.6 bundle remains untouched. Serve the web client over HTTPS
from the server, not `file://`; codec/fullscreen capabilities remain browser
dependent. No web/Pi deployment is performed here.

Rollback: use the retained v0.6 bundle and previous web distribution. No server
schema migration, cache deletion, scanning or video rewrite.
See [ADR0025](../architecture/decisions/ADR-0025-web-shared-ui-parity.md).
