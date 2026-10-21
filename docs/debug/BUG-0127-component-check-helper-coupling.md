# BUG0127 — Component checks incorrectly require the installer

Date: 2026-10-21. Status: FIXED / VERIFIED (headless deployed scope). Branch:
`codex/update-check-controls`, base `6ba3970`. User reports web 0.7.0 and Mac
0.7.1 can only check the native Mac release. Related: ADR0029, component update
contract, sole API `2026-10-14`.

## Expected / observed

All supported component checks are usable independently of installation rights.
“Install now” is absent until a newer release is detected. Instead, the shared
UI unconditionally creates the install button and disables **check**, save and
install together when `helper_available` is false. The Pi deliberately has no
root updater installed. The API also rejects check intents in that case.

## Evidence and diagnosis

H1 confirmed by source tracing: `ui/connected/updates.js` gates all three buttons
on the root helper; `updates_api::intent` rejects both intent kinds when the
helper configuration is absent. Local Mac checks bypass that server path,
explaining the different behavior. No certificate/network bypass is warranted.

Regression added first: the new-version visibility unit test fails RED on the
old implementation (missing predicate). Shared DOM fixture now covers Mac and
web, no helper, checks for server/web/themes, no/equal/older/newer versions,
failed recheck, native consent and use of the catalogue during a held request.

## Correction

- Separate UI capabilities and hide installation unless a newer stable version
  is detected. Installation/scheduling remain disabled without a root helper.
- Add a bounded non-root manual metadata worker for legacy/no-helper servers.
  Existing signed-manifest verifier and fixed GitHub transport are reused.
  Discovery persists only UI results in a separate store; no install intent,
  payload download, root cache or anti-rollback acceptance is created.
- Keep five-minute coalescing, one worker per component, bounded HTTP and files,
  explicit errors/incompatibility, atomic result publication and background work.
- Add `check_available` compatibly; dated API version and closed request fields
  are unchanged. Translate feedback into French and English.
- Prepare Mac/server/web 0.7.2, sequence 3; unchanged themes stay 0.7.1/sequence 2.
  Existing signed tags/releases are immutable and were not edited.

## Verification and limitations

Exact source `a38d4b16616ebd6735fa4aab0b518e28d767079e`: server library
147 PASS / 1 historical RSS NFR ignored; Clippy all-targets PASS; Node web/update/
release/security 198 PASS; bilingual docs 23 PASS; actual shared Mac/web DOM PASS.
Independent security review **APPROVED**, no open delta findings:
[report](../reviews/BUG-0127-security.md). Signature/transport/state primitives
were additionally rechecked by that distinct reviewer.

Real HTTPS on an isolated loopback server with empty private data, fixture web
0.7.0 and renderer 1: GitHub's signed web 0.7.1 detected (`AVAILABLE`, exact source
and sequence 2); themes report `CHECK_INCOMPATIBLE`; server reports `NO_UPDATE`;
repeated web request returns the same cached receipt; installation stays 503
`UPDATE_HELPER_NOT_INSTALLED`. API identity/status remain responsive while checks
run. No payload/archive download was invoked. The owned QA server was stopped.

Mac 0.7.2/build 11 bundled from exact source, 48 arm64 libraries, strict ad-hoc
signature and libmpv load PASS, actual minimum macOS 27.0. DMG generated and
`hdiutil verify` PASS; SHA256
`1d80517bdf1f5e4e079c14bc7ad6398b83de6347fc582fccf204140416b364f5`.
Local candidate: `artifacts/macos/v0.7.2-a38d4b1/Chill-Center-0.7.2-macos-arm64.dmg`.
Web candidate built as 0.7.2/build `ad12a307191b`, 101 files. Native GUI actions,
Pi ARM binary packaging and publication are not inferred from these checks.

The first default-sandbox Chrome invocation failed before page load (SIGABRT /
EPERM); the isolated escalated browser run succeeds. A first web fixture reused
the old document through hash-only navigation; fresh-document navigation fixed
the oracle, not application code. Full Rust default-sandbox run hit one known
socket EPERM; escalated loopback-only rerun passes. The initial QA launch from its
data ancestor was refused by the workspace/data separation guard; relaunch from
the actual source checkout passes, without weakening that guard. No native
player test is needed here.

## Publication and deployed verification / Publication et vérification réelle

The user subsequently authorized publication and Pi server/web deployment.
v0.7.2 is public with the signed Mac DMG; exact server/web 0.7.2 artifacts are
installed after a fresh private backup. The distinct
[release verifier](../releases/REL-0.7.2-verification.md) approved strict-TLS
identity/catalogue, protected settings/data, backup and rollback readiness.
Real server/web/themes checks all reached `NO_UPDATE`, a repeated check was
cached, and installation without the helper remains explicitly refused (503).
No payload is downloaded by the metadata worker. Native Mac GUI is NOT_RUN;
install the released 0.7.2 DMG to obtain its shared-UI correction.

FR : publié et vérifié sur le Pi, avec sauvegarde et ancienne paire conservées.
Recherches réelles des trois composants fonctionnelles, installation toujours
séparée des droits de recherche. NAS, réglages, TLS et configuration de l'updater
inchangés. L'incident de supervision pendant la sauvegarde est distinct et reste
tracé en [BUG0128](BUG-0128-operator-transport-and-recovery-overlap.md), sans être
requalifié en succès de transport SSH. Aucun test du lecteur n'a été relancé.
