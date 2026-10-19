# ADR-0029 — Independent components, theme packs and verified updates

- Status: **PROPOSED**; no updater, installer, trust key or release published.
- Owner / author instance: `/root` / `/root`.
- Reviewer instance: PENDING.
- Date: 2026-10-19.
- Branch / base: `codex/component-updates-design` / `6cdb649b8c834c4173afd320da2c8da64867a84e`.
- Revision: R2; reviewed commit/time: PENDING.
- Supersedes / superseded by: NONE / NONE. Extends packaging and appearance
  architecture without changing existing API or historical release evidence.
- Related items: user request 2026-10-19; F-01/F-18/F-20/F-21;
  [theme contract](../../../themes/README.md), API date-versioning ADR-0011,
  shared UI ADR-0025, existing Mac v0.6 and web mobile R4 delivery.

## Context / Contexte

The user requests independent theme delivery, prominent Mac/web/server downloads
and installers, visible component versions, GitHub update checks and configurable
manual/automatic updates. Mac installation always requires “Install now”. Server
default: check at 02:00 local time and update afterwards. Repository visibility
was verified PUBLIC, with main default, on 2026-10-19; only Mac v0.6 is currently
published. No public-visibility change is part of this proposal.

Current source has Mac 0.6.1, internal workspace/server 0.1.0, web content build
67773f7877d3, embedded CSS/assets and duplicate Mac/web calendar resolution.
Existing Linux and web packages/operators are not generic public installers.
No production update trust root or automatic installer currently exists.

## Options considered / Options examinées

| Option | Benefits | Costs and risks | Reversibility |
| --- | --- | --- | --- |
| Manual release downloads only | Reuses current GitHub releases | Does not meet scheduled automatic server updates | Easy |
| HTTPS plus unsigned hashes | Simple integrity against transfer damage | Repository/manifest replacement permits untrusted code execution; rejected | Medium |
| Signed component releases, separate privileged installation | Meets request; tampering detectable; bounded privileges and rollback | Signing-key custody, recovery and installer review required | Retain old installed version |

## Proposed decision / Décision proposée

### Four independent release streams

Mac, server, web, themes each have a SemVer, source commit, content hash and
compatibility constraints. Proposed first coordinated release: 0.7.0 for each;
later versions advance independently. Tags `mac-vX.Y.Z`, `server-vX.Y.Z`,
`web-vX.Y.Z`, `themes-vX.Y.Z`. Never move existing v0.6/tag or replace its bundle.
Do not use global `/releases/latest`: it may describe another component. Resolve
only stable, non-draft releases for the requested component; never auto-downgrade.

README FR/EN prominently links actual assets only after successful publication:
Apple Silicon Mac installer/archive, Linux ARM64/ARMv7 server packages, web
archive, theme pack, install instructions, manifests/signatures and licences.
Server ELF targets and platform dependencies must be verified. No paid Apple
signing identity or notarization is assumed; existing ad hoc limits stay explicit.

Publication is a manual maintainer operation, not a new automatic GitHub workflow.
Build from an exact clean committed source, run relevant tests and independent
security review, create private local staging, generate provenance/hashes, sign
locally, then publish only the reviewed assets using the maintainer's existing
GitHub CLI authentication to this repository. No new PAT, Actions signing secret,
push-triggered workflow or unreviewed CI execution is provisioned. The requested
new public assets are authorized by this user request, but key provisioning and
the implementation gate remain pending. No old release, tag, asset or backup
is deleted/replaced. Publication refuses asset-name collisions, records source
and artifact hashes, then downloads the published assets to verify hashes and
signatures. Failed uploads leave an explicitly incomplete draft, not an advertised
installable release. A stable signed release cannot be edited in place; corrections
get a new version. Local build tools and dependency pins remain part of provenance.

The shared signed manifest schema v1 has mandatory fields: `schema_version`,
`component` (mac/server/web/themes enum), SemVer `version`, increasing integer
`release_sequence`, `release_tag`, exact `source_commit`, `issued_at`,
`expires_at`, `key_id`, `compatibility`, and `assets`. Each asset binds `target`,
fixed-repository URL, byte length and SHA-256; the detached signature authenticates
the exact manifest bytes and thus these payload hashes. Mac additionally has the
signature required by the maintained Tauri updater. Parsers reject duplicate JSON
keys, unsupported required schema fields, malformed values, expired metadata and
unknown key IDs. Expired metadata keeps current software running and reports a
maintainer refresh requirement; it never disables verification. Save the highest
accepted release sequence per component to prevent replay/downgrade; rollback
is an explicit local recovery operation, not an automatic lower-version download.

`compatibility` specifies OS/architecture, supported API versions, database read/
write and rollback schema ranges for server, required server/API range for clients,
and theme format/renderer range for themes. Version display is not a substitute
for these checks. The installer checks installed and proposed versions together
before mutating anything. A compatible server update precedes a dependent web
update; Mac remains explicitly confirmed, and themes follow a compatible renderer.
If an existing client/web would become unsupported, or rollback cannot restore a
compatible combination, refuse the plan with an actionable status. A component
update never silently upgrades another component: required prerequisites are
shown and separately admitted by their configured policies. API aliases continue
to serve older clients. Each separate update has its own atomic switch and smoke
test; schema-incompatible server transitions require a reviewed manual migration,
not an automatic cross-component transaction.

Public installers are inspectable files downloaded and verified before execution,
not `curl | sudo sh`. Generic Linux setup prompts for/configures library paths
and listening addresses, refuses overwrites, leaves NAS mounts/credentials alone,
and creates a non-root service. Web installer replaces only reviewed web assets;
it never overwrites binary/config/TLS/database/NAS. No real LAN defaults in Git.

### Settings and API

Settings shows installed Mac or web version, server version, theme pack version,
build/source, API version, available updates, check/install results and next
scheduled attempt. Server version is its reported running version, not inferred
from client version. Older servers show unsupported update controls, not a
connection failure. API additions preserve `/api/2026-10-14` and `/api/v1`.

Checks/downloads run asynchronously without global UI locks or catalog reset.
Policies distinguish manual check, automatic check with manual install, and
automatic installation where permitted. Mac always requires an explicit
“Installer maintenant / Install now” action, including restart consent.
No background Mac daemon is added: when open it checks at the selected time;
a missed check can run on next launch, without installation.

### Scheduling and availability

Server default policy is automatic, 02:00 in the server OS local timezone;
settings displays that timezone and the next attempt. The timezone is not changed.
A bounded systemd scheduler reevaluates local date/time to handle DST gaps and
repeats, missed checks run on service startup, and an attempt is recorded once
per local day. Manual “check/update now” remains available; scheduling can be
disabled or changed. Network/rate-limit failures keep the running version and
use bounded retry; no busy-loop or repeated install after a failure.

Download and verify in background. Automatic install waits until no active
playback or conflicting scan/import/TMDB write job; it queues rather than
interrupts. A manual forced restart requires a separate explicit warning and
confirmation. New sessions are temporarily prevented only during the short
final backup/swap window, closing the idle-check race. Abort safely before swap
if work cannot drain. No long-lived database lock during network I/O.

Web/theme updates have separate policies and versions, coordinated with
server/API compatibility. Retain old content-hashed assets until old clients
can finish. A browser does not become a privileged installer: it requests a
bounded server-side action and activates new UI when idle/reloaded, never
halfway through playback. A theme reload needs no application reinstall.

### Trust boundary — approval required

Use maintained Tauri updater verification for Mac where compatible with the
existing custom bundle; reuse the same established detached-signature format
for Linux/web/theme manifests and payloads, not bespoke cryptography. The exact
compatible dependency versions must be verified/pinned during implementation.
Validate component, target, monotonic version, byte count, digest, signature,
API/schema/renderer constraints before staging. Accept only fixed repository
release endpoints and verified asset destinations; reject arbitrary URLs,
redirect targets, paths, commands, archives with traversal/symlinks and zip bombs.

A release verification public key is embedded/pinned. Private signing material
is never committed, logged, exported or sent in chat. Proposed custody: maintainer
local private file outside Git, protected permissions and an offline backup;
no GitHub signing secret is created implicitly. Key creation/provisioning and
public-key inclusion wait for explicit user approval. Rotation requires a
cross-signed transition or explicit reinstallation; loss does not permit bypass.
Signatures are distinct from TLS certificates and Apple code signing.

The unauthenticated LAN API is not granted arbitrary root execution. Server
stays non-root and writes bounded update intents/status only. A separately
installed root-owned updater validates fixed paths, key and release identities;
it does not execute service-owned scripts or shell fragments. Local installation
authorizes this helper; no general sudo permission or writable root executable.
Because LAN administration is unauthenticated, household users may request a
valid signed update or change allowed scheduling; document denial-of-service
risk. Never expose the API publicly.

Allowed LAN request fields are bounded enums/values only: component, check or
install operation, automatic/check-only/manual policy, local hour/minute and
expected policy revision. There is no URL, repository, key, command, path, arbitrary
tag, downgrade or credential field. Repository/key/install-root and safety limits
are root-owned local configuration, not editable by clients. Server enforces policy
revision compare-and-swap, validates times and returns the current policy on a
conflict. Updating a schedule does not reset the daily-attempt ledger or trigger
an immediate install. Equal requests return the existing operation ID/status.

There is one global install operation and at most one pending intent per component;
duplicate checks coalesce. At most one new outbound check per five minutes per
component, cached status available meanwhile, and no unbounded queue/log growth.
Admission persists across API restarts; fixed-duration retries are capped and
cannot reset on repeated LAN requests. No-update requests cause no restart.
Root helper verifies intent structure, signed newest eligible target and policy
again; the non-root service cannot submit executable instructions. Manual client
installation still waits when busy. Forced interruption is a local administrator
command with explicit confirmation, not an unauthenticated browser/API bypass;
Settings explains how to proceed if immediate interruption is really required.
This limits but does not eliminate LAN denial-of-service: household members can
disable automatic checks/change their time within allowed policy, consistent
with existing unauthenticated household configuration. This risk and the local-only
force behavior form part of the user approval requested for R2.

### Themes

Independent declarative pack under `themes/`; dates/priorities, variants,
resource safety, fallbacks and renderer compatibility are specified in its
FR/EN README. No free-form executable CSS/JS downloaded as a theme. Both Mac
and web consume the same validated calendar/pack contract, with a bundled
fallback and previous installed pack retained.

## Consequences, migration and rollback / Conséquences

Positive: small independent updates, low-RAM streaming downloads, discoverable
versions, no server transcoding and no video copying. Costs: one signing-key
lifecycle, staging disk space, separate privileged helper and dual web assets.
No new cloud service or paid Apple account assumed; public release/CI quotas
and artifact sizes must be checked before adding automated build workflows.

Bootstrap requires one explicit install of the updater-capable components.
Existing clients continue to use existing API. Source migration preserves profile
preferences and uses bundled themes on invalid/missing external packs.
Before server swap, save private consistent database/config/TLS state and retain
the previous binary. Perform bounded health/API/version smoke; roll back on
failure. Automatic updates must refuse schema changes that cannot safely run
with the previous binary, rather than claim a binary-only rollback is sufficient.
Never restore over NAS or delete user data. Retention/purge policy is a separate
choice; do not silently delete delivered versions or private backups.

## Verification plan / Plan de vérification

Not executed yet: calendar boundaries/precedence/DST/leap years, malformed packs,
version compatibility and no downgrade, forged/missing signatures, redirect and
archive attacks, interrupted download and disk-full handling, duplicate scheduler
triggers, UI responsiveness, running-version display with old-server fallback,
Mac mandatory install confirmation, busy playback/job deferral and race closure,
backup/swap/rollback simulations, old web assets, final packages/licenses.
Independent code, test and security review must cover the frozen implementation.
Real Pi installation, restart, new helper and scheduling need a separately
confirmed deployment scope; this design performs none.

## Approval / Validation

- Independent ADR review: PENDING.
- User gate: REQUIRED; currently PENDING. New downloaded-code/privilege trust
  boundary must be approved before implementation/provisioning.
- This document proposes behavior, not completed functionality or test evidence.
- Sources checked 2026-10-19: [Tauri updater](https://tauri.app/plugin/updater/)
  (mandatory update signatures) and
  [systemd timer documentation](https://www.freedesktop.org/software/systemd/man/260/systemd.timer.html)
  (calendar timers, clock synchronization and persistence).

### Résumé français soumis à validation

Quatre versions indépendantes, futures releases 0.7.0 et téléchargement visible.
Packs visuels déclaratifs, sans code actif, calendrier actuel conservé. Contrôles
asynchrones ; Mac toujours « Installer maintenant ». Serveur vérifié à 02:00
heure locale, puis installé quand lectures et travaux sont terminés ; forçage
manuel après avertissement. Releases signées, clé privée locale hors Git et
clé publique embarquée, installateur privilégié distinct du service non-root.
Sauvegarde et retour arrière vérifiés ; aucun changement Pi immédiat.
Publication manuelle par le mainteneur, aucune clé de signature dans GitHub.
Un manifeste signé lie versions/empreintes/compatibilités ; toute mise à jour
qui casserait un client est refusée. Demandes LAN regroupées et bornées, aucun
chemin/commande/URL arbitraire. Le forçage interrompant une lecture reste une
commande locale d'administrateur, non un droit ajouté à l'API sans authentification.
