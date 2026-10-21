# Chill Center v0.7.1 — release notes

[Français](REL-V0.7.1.md) | **English**

Published on 21 October 2026: [v0.7.1 release](https://github.com/ungeekrange-cmyk/chillcenter/releases/tag/v0.7.1).
Immutable package source and target of all five tags:
`c603f29db2936ee40f7bf66484187b4e237d9bfd`. Manifests, provenance and
SHA256SUMS are signed; subsequent GitHub controls do not change these bytes.

[Download the Mac DMG](https://github.com/ungeekrange-cmyk/chillcenter/releases/download/v0.7.1/Chill-Center-0.7.1-macos-arm64.dmg).

- Mac: bounded four-second Bonjour/mDNS server discovery at launch. A healthy
  saved server keeps priority. Manual entry is behind “Can't find your server?”.
  Advertisements never approve a certificate: ordinary HTTPS trust still applies.
- Linux server: Bonjour advertising starts only after the HTTPS socket binds.
  Fresh installation allows AF_NETLINK only for interface enumeration, retaining
  the non-root user and empty capability sets.
- Sole API `/api/2026-10-14`; retired `/api/v1` routes return404. Stored old artwork
  references are normalized without clearing existing caches.
- Approved Easter/Valentine light/dark ambiences, petals and solid rabbit
  silhouettes. Western Gregorian Easter is recalculated annually, D−30;
  Valentine starts D−14. Seasons resume the next day. Renderer-2 packs remain
  declarative; renderer1 packs still load. Packs cannot execute code.
- Release-ledger hardening uses fixed component identifiers rather than a
  command-line-supplied regex. Linux test fixtures explicitly require TLS1.2+.
- Dependency maintenance: weekly Dependabot proposals, read-only CI and guards
  on actual shipping Mac/server dependency graphs. The medium
  `GHSA-wrw7-89jp-8q8g` advisory stays open for the unsupported native Linux
  desktop GTK chain; absent from both shipped graphs, no upstream fix claimed.
  [Assessment and follow-up](../security/DEPENDENCIES.en.md).

All four distribution components are version0.7.1. The new downloadable web
package includes the ambiences, but **the household's deployed web files are
not replaced** by the authorized server-only operation: web0.7.0 already uses
the dated API. NAS, application settings and TLS identity remain protected.

Mac: Apple Silicon DMG, macOS27+ required by bundled libraries, ad-hoc and
Minisign signatures, not Apple-notarized. Trusted local network only, no
authentication or server transcoding. Installation and macOS Local Network
consent remain human actions.

Detailed evidence: [delivery](REL-0.7.1-delivery.md),
[resources](REL-0.7.1-resource-final.md), [exact security review](REL-0.7.1-security-candidate.md),
[independent verification](REL-0.7.1-verification.md).
GitHub publication and Pi installation are separate operations; their actual
results are recorded separately, never inferred from a build or mocked test.
