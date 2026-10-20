# ADR-0030 — Bounded LAN discovery / Découverte locale bornée

- Status: user-approved scope, source independent review pending.
- Date:2026-10-20. Owner:/root. Release:Mac/server0.7.1; web/themes remain0.7.0.
- Extends ADR-0005 (HTTPS), ADR-0011 (dated API), ADR-0029 (independent versions).

## Context / Contexte

The Mac client should find a household server without entering IP/port. Manual
input must initially be hidden behind “Vous ne trouvez pas votre serveur ?” /
“Can’t find your server?”. The existing healthy saved connection takes priority.
Le serveur et le client Mac évoluent ; aucun fichier du client web, réglage,
certificat, compte, montage ou fichier NAS n’est modifié.

## Decision / Décision

Use mDNS/DNS-SD `_chillcenter._tcp.local.`. The server announces only its stable
UUID, protocol=https, API2026-10-14, component version and configured HTTPS port.
No media path, profile name or credential is announced. Advertise LAN interface
addresses for a wildcard listener in the reusable discovery helper, or the
concrete LAN bind address. Current server configuration deliberately permits
only concrete LAN listeners; this change does not broaden that policy. Public and
loopback addresses are excluded. If multicast is unavailable, the server remains
usable through ordinary HTTPS and the client offers retry/manual entry.

Browse once at launch, off the main UI/client command lock, bounded to four
seconds (hard cap five),16servers and8addresses/server. No subnet/port scan or
background polling. Explicit retry is permitted. One candidate may enter the
ordinary connection flow; multiple candidates require a choice. Late results
must not replace manual input, a saved healthy connection or active playback.

Discovery is **not trust**: unauthenticated LAN records are only hints. Existing
HTTPS hostname/SAN, SPKI approval/pinning and actual API server UUID verification
remain mandatory and unchanged. A spoofed announcement cannot approve a key.
An address change not covered by the configured certificate remains an ordinary
TLS failure requiring operator certificate configuration, not a discovery bypass.

Mac-only native initialization injects the connection UX; shared web source and
deployed web bytes are unchanged. The new native command accepts only `browse`
with no arbitrary URL/hostname argument. Service advertisement failure is nonfatal.

## Alternatives / Options

- Port/subnet scan: rejected, unnecessary traffic and delay on Wi-Fi/Pi.
- Fixed default IP: rejected, brittle and not committed to the repository.
- Browser discovery: out of scope; requires different platform capabilities.
- Trust mDNS UUID or automatically approve certificates: rejected.

## Verification / Vérification

Required: bounded parser/command tests, wildcard/concrete listener tests, native
UI hidden fallback/retry/multiple/stale/saved-session tests, compiled Mac/server
checks and actual LAN announcement/browse. Preserve strict HTTPS trust and dated
API routes (ADR0031 separately retires the old alias). Before Pi swap: private consistent backup, exact old/new binary
hashes, idle gate and rollback. After: running version0.7.1, web tree unchanged,
settings/TLS/unit and catalogue semantic state preserved. GUI/OS permission
limitations must be reported, never described as passing tests.

## Consequences and rollback / Conséquences et retour arrière

Adds one bounded mDNS daemon/server and one short-lived browse/client launch.
Multicast must be permitted on the LAN; routed/VPN/isolated Wi-Fi may need manual
entry. IPv6 link-local authorities needing an interface scope are not offered.
No API route, database schema or data migration. Restore the preserved0.7.0
server binary and prior Mac bundle to roll back; retain the unchanged web tree.

The separately requested Easter/Valentine concepts remain proposals only under
`docs/design/concepts/2026-10-20/`; no calendar/theme integration is authorized
by this discovery decision.
