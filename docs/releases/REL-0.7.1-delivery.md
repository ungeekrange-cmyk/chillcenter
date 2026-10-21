# Chill Center0.7.1 — delivery status / état de livraison

**Status 2026-10-21: published v0.7.1; Pi server and public downloads independently verified.**

Immutable package source: `c603f29db2936ee40f7bf66484187b4e237d9bfd`.
Code/security/test reviews independently approve this exact source. Mac, server,
web and themes distribution versions are 0.7.1; renderer 2 includes the approved
Easter/Valentine ambiences. The earlier fdb80b5 Mac DMG below is superseded.
Independent [artifact and Pi verification](REL-0.7.1-verification.md) passed.

- Final DMG SHA256:
  `5b281dd9c4847c2eac5f9505da64b5e538e06305a76f2d5230cfd265aee83a14`.
  Apple Silicon, macOS 27+, ad-hoc signed, not Apple-notarized. Real read-only
  mount, exact app tree and strict signature checked.
- Server SHA256:
  `8cf14421630cfd552e3af4ed340de107c0570046cc5ecb2784936ca8c552491c`.
  ARM64, glibc 2.28+. Installed on the Pi after a fresh private backup and the
  explicitly approved single AF_NETLINK exception. Strict TLS, dated API,
  data/settings invariants, unchanged web and actual headless mDNS passed.
  [Execution evidence and rollback](REL-0.7.1-pi-execution.md).
- Downloadable web build `e496ef4b7ca6` is 0.7.1, but household deployed web
  remains **0.7.0**, compatible and preserved. No NAS/config/TLS change; legacy
  root updater/timer was not activated. GUI Mac/Local Network consent NOT_RUN.
- Manifests and sequence 2 assets are signed by existing key
  `18FD790B48C202F8`; provenance/SHA256SUMS and all manual wrappers verified.
  Public download gate **APPROVED**: all 22 umbrella assets downloaded into a
  fresh directory, byte-identical to signed local references; signatures,
  SHA256SUMS, provenance, exact tags and component inventories verified.

Five releases are now stable and public, with umbrella **v0.7.1 Latest**,
published at `2026-10-21T10:45:54Z`. Component inventories are Mac 7, server 14,
web 4, themes 4; umbrella 22 assets. Before publication, every remote asset
state/size/SHA256 matched signed local bytes. All annotated tags peel to the
immutable package source above. Existing 0.7/0.6 releases were preserved;
repository visibility PUBLIC and default branch main are unchanged.

Security process follow-up `61da27f` changes only CI/tooling/docs, not shipping
inputs or Cargo.lock. Independent review approved it; hosted dependency run
39586395359 passed. The real medium glib advisory remains open, absent from
the actual Mac/server graphs. Five later CodeQL alerts remain open by explicit
user choice. CodeQL follow-up39586394861 also completed successfully in all five
languages; no additional open alert was observed. See
[GitHub controls](../security/GITHUB-SECURITY.md).

## Historical 21 October preparation checkpoint

Before final source freeze and controlled deployment, the candidate was still
under review and the original server remained untouched. The later evidence
above supersedes that preparation state, not the immutable historical hashes.

## Historical build snapshot —20October / Historique du20octobre

The remainder records the prior gate and immutable artifacts; it is retained,
not silently requalified as the final theme-integrated source.

## Ready / Prêt

- Runtime source:`fdb80b502db45d10c5384104912f5f246678c32a`; current delivery and
  deployment-operator gate:`aba640284cd1dcbb27bfb4d0c1bec2b9deef3c82`.
  Runtime `apps/`, `crates/`, shared UI and metadata have no subsequent delta.
- Mac/server version0.7.1; web/themes remain0.7.0. No web production file changed.
  The web's existing dated requests are compatible;182tests pass.
- Sole served API in the **new binary**:`2026-10-14`; old v1 routes return404.
  Current Pi0.7.0 still serves its old aliases until installation occurs.
- macOS Apple Silicon DMG:
  `artifacts/macos/v0.7.1-fdb80b5/Chill-Center-0.7.1-apple-silicon.dmg`,
  SHA256 `a752700170d40b137d74597beb073f29bb4f5aebcd157c6f9ee244c9ac908bfd`.
  Executable SHA256 `aba3ba3c9e2b0d64cadf601b88b2b9f37c0686269b3a63cd8984864802ad02cc`.
  Ad-hoc signed, not notarized; actual bundled libraries require macOS27.0+.
  Verified48relocated arm64 libraries, load-test, deep/strict signatures,
  read-only DMG mount,0.7.1 plist, Bonjour declaration and exact mounted binary.
- ARM64 Linux/glibc2.28 binary:
  `artifacts/linux/v0.7.1-fdb80b5/chill-server`,
  SHA256 `8cf14421630cfd552e3af4ed340de107c0570046cc5ecb2784936ca8c552491c`.
  Built with Rust/Cargo1.99.1, cargo-zigbuild0.23.4, Zig0.17.0. ELF validation
  passes; private staged execution on Pi reports `chill-server0.7.1`.

## Proof / Preuves

Server143PASS (one manual RSS check ignored); domain74PASS; client52PASS
(one benchmark ignored); discovery5PASS; desktop IPC1PASS; native discovery
DOM7PASS; web182PASS; calendar preview4PASS; bilingual docs23PASS;
deployment operator21PASS plus independent adversarial probes; OpenAPI80paths.
Code/security/test R4 reviews are APPROVED on exact `aba6402`.
Power-loss, real GUI/OS Local Network consent and actual LAN discovery are not
proved by these local/mocked tests.

## Pi unchanged / Pi inchangé

Independent fresh baseline: server0.7.0 binary SHA256
`68d526cd3f3af7cee9b07b06c62f46260656dda7cccc4c12994dee85431771e5`,
PID107443, active, zero restarts, RSS46360KiB; schema3,29tables,2496media.
Web0.7.0/build`a1c12ba4a582`,99installed files, unchanged.
Configuration/TLS/unit fingerprints are private. New bytes are staged privately;
read-only deployment preflight passed. No stop/swap, new data backup, NAS write,
web replacement, configuration edit or helper/timer activation has occurred.
The previous0.7 backup remains preserved; a fresh backup is still mandatory.

The final environment check exposed [BUG0119](../debug/BUG-0119-mdns-systemd-interface-family.md):
existing systemd family restrictions deny Linux interface enumeration.
Equivalent transient probes yielded errno97 without AF_NETLINK and success with
it. Actual unit and old binary stayed unchanged. [ADR0032](../architecture/decisions/ADR-0032-discovery-interface-enumeration.md)
proposes one reversible AF_NETLINK exception, requiring explicit user approval.
No silent sandbox/config relaxation is permitted.

## Themes / Ambiances

Easter and Valentine backgrounds approved. Sidebar rabbits/petals are proposals
in `docs/design/concepts/2026-10-20/events.html`, with light/dark modes and a
year selector. Easter uses annually computed Western Gregorian Sunday,J−30;
Valentine Jan31–Feb14,J−14; seasonal return the next day. No runtime theme pack
or web app changed. Character comparison and reduced-motion behavior are
documented in the concept README; physical GUI approval remains separate.

## Next permitted step / Prochaine étape autorisée

Ask for the single sandbox exception. If approved: preserve original unit,
apply only the reviewed exception, make the fresh private consistent backup,
install the pinned binary, verify strict TLS/API/web/data and real Mac browse,
then independent post-verification. Preserve old binary/unit for rollback.
If declined: keep current service and manual/saved connections; do not claim
automatic discovery works on this restricted Pi.
