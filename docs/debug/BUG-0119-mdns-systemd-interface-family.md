# BUG-0119 — Linux discovery blocked by service sandbox

- Status: **OPEN / USER_PERMISSION_REQUIRED**,2026-10-20.
- Category: environment compatibility discovered before deployment; no Pi outage.
- Component: proposed server0.7.1, Linux existing systemd sandbox.

`mdns-sd`/`if-addrs` enumerate interfaces through Linux `getifaddrs`. The existing
service restricts address families to AF_UNIX/AF_INET/AF_INET6. A transient
non-root probe with precisely these families returned-1/errno97; the same probe
with AF_NETLINK added returned0/errno0. No NAS access, persistent unit/config
change or Chill Center restart occurred. Both transient probes were collected.

Inference supported by source: the mDNS dependency uses
`get_if_addrs().unwrap_or_default()`, so the denied enumeration becomes no usable
interface. Registering a service without error does not prove announcement.

Proposed remediation and rollback: [ADR0032](../architecture/decisions/ADR-0032-discovery-interface-enumeration.md).
It exceeds the user's “existing settings unchanged” boundary, so installation is
paused before stopping the service. Current server remains0.7.0, existing web
unchanged. Actual LAN discovery and GUI verification remain NOT_RUN.
