# ADR-0032 — Discovery interface access / Accès aux interfaces pour la découverte

- Status: **PROPOSED — USER APPROVAL REQUIRED**,2026-10-20.
- Owner:/root. Extends ADR0030; does not authorize deployment or unit changes.

## Evidence / Preuves

The current Pi service allows only AF_UNIX, AF_INET and AF_INET6. Linux
`getifaddrs`, used by the new discovery dependency, needs AF_NETLINK. Two isolated
transient probes under the existing service user and NoNewPrivileges produced:

| Allowed families | getifaddrs | errno |
| --- | --- | --- |
| Existing three families | -1 | 97 (EAFNOSUPPORT) |
| Same three plus AF_NETLINK | 0 | 0 |

The probes were collected automatically. The real Chill Center service was never
stopped/reconfigured; its old0.7.0 binary hash and PID remained unchanged. The
dependency silently turns interface-enumeration failure into an empty interface
list; successful registration alone therefore cannot prove LAN advertisement.

## Proposed decision / Décision proposée

Add **only AF_NETLINK** to the existing systemd address-family allowlist, through
a narrowly scoped reversible drop-in, after explicit user approval and backup.
Keep the same non-root user, empty capability set, NoNewPrivileges, read-only NAS,
all filesystem protection, ports, TLS material, application configuration and web
files. AF_NETLINK permits interface queries; it does not grant CAP_NET_ADMIN.
No new privileged helper, timer, Avahi configuration or persistent service.

L’utilisateur a demandé de préserver les réglages existants : cette exception au
cloisonnement du service doit être autorisée explicitement avant toute écriture
de l’unité. Aucun changement n’a encore été appliqué.

## Alternatives and rollback / Alternatives et retour arrière

- Leave the restriction: existing manual/saved connections work, but automatic
  discovery cannot be claimed functional on this Pi.
- Weaken all sandboxing or add capabilities: rejected.
- Replace/fork the discovery dependency or add a privileged discovery helper:
  larger code/security/resource footprint, not justified for this narrow need.

If approved, preserve exact prior unit bytes/hash, apply the one-family drop-in,
run daemon-reload and the separately approved binary upgrade. Verify effective
families, strict TLS/API, unchanged application settings/web/data and actual
Mac LAN discovery. Roll back both binary and the owned drop-in on failure;
never remove unrelated configuration. Physical GUI approval remains separate.
