# Independent Security Delta Review R2 — server slice

- **Prior reviewed source**: `5115afde12e73c6c97bd49104e7cd19ac111fdb7`
- **Reviewed source**: `a06d95f29da099a7a93769420fa5ce62a6478581`
- **Reviewed at**: `2026-10-13T16:31:47Z`
- **Reviewer instance**: `/root/server_security`
- **Scope**: read-only delta review of `crates/chill-server`, server/workspace manifests and lockfile, and `deploy/server.example.json`.
- **Scoped verdict**: `APPROVED`

Approval is limited to this server configuration/API foundation. It does not approve profile-store semantics, native Mac transport/probe, video/Range/import/lease endpoints, desktop workspace readiness, Pi/NAS deployment, all stories, or the full application. The accepted household-LAN/no-auth and post-expiry TLS exception remain governed by F-18/F-19 and ADR-0005.

## Prior finding disposition

| Finding | Delta evidence | Disposition |
| --- | --- | --- |
| `SRV-SEC-001` HIGH — wildcard listener broadens the LAN boundary | `config.rs:42-85,283-301` rejects wildcard/public binds; `connection.rs:37-55` rejects non-local peers before TLS; `deploy/server.example.json:2` uses the explicit LAN IP. Host/Origin checks remain at `api.rs:106-138`. | `RESOLVED` |
| `SRV-SEC-002` MEDIUM — unbounded sockets/handshake/request time | `connection.rs:16-55` holds one of 16 permits for each live socket; `main.rs:69-85` sets a 5 s TLS handshake, 5 s HTTP/1 header limit with no keepalive, and eight HTTP/2 streams with ping timeout; `api.rs:139-143` caps current JSON routes at 30 s. | `RESOLVED` for current JSON-only routes. Future media streams still require per-read/idle limits rather than this total deadline. |
| `SRV-SEC-003` MEDIUM — unsafe `data_dir` ownership/mode | `config.rs:87-125` rejects symlink components and unsafe parents/final ownership, then applies `0700`; `main.rs:27-35` sets `umask(077)` before TLS/SQLite creation. `config.rs:303-315` covers symlink and unsafe-mode rejection. | `RESOLVED` |
| `SRV-SEC-004` MEDIUM — certificate reuse trusts schedule/clock | `tls.rs:37-63` gates uninitialized/backward clocks; `tls.rs:80-84,121-169` checks validity, self-signature, SPKI/key equality, SAN set, and scheduled validity; `main.rs:43-66` rechecks every 60 s; `tls.rs:191-207` covers cold/backward clock and corrupted restore while preserving key/server identity. | `RESOLVED` for this code slice. First-ever issuance still relies on the architecture's reliable-time deployment invariant; Pi/systemd time-sync is not proven here. |
| `SRV-SEC-005` LOW — post-rename fsync failure can leave disk/memory revisions divergent | Logic is unchanged at `config.rs:175-178,197-220`. The coordinator records it as owned and not fixed. | `OPEN`, nonblocking LOW; retain until fault-injection/reconciliation work is completed. |

## Delta security checks

- No new authentication, tenant, cookie, session, redirect, CORS, or cloud/IAM boundary was introduced. Profiles remain selectors rather than confidentiality tenants under the accepted no-auth scope.
- Rejecting public/wildcard local binds plus the pre-TLS peer check prevents a client from bypassing the LAN boundary merely by forging the allowed Host header. Router forwarding/reverse proxies and firewall policy remain deployment concerns and are not approved here.
- Socket permits are carried by the wrapped I/O object until disconnect, so they cover both TLS handshakes and HTTP connection lifetime. The explicit 5 s TLS value is stricter than the supplied local axum-server 0.8 source default of 10 s.
- The TLS parser reads only the service-owned persisted certificate and caps it at 64 KiB before parsing. A malformed/mismatched certificate is regenerated with the established key and server ID; a missing established key still fails closed.
- `x509-parser 0.18.1` and `hyper-util 0.1.21` are locked with registry checksums; no Git-sourced package or committed secret/key/certificate material was found in the exact reviewed tree. Targeted RustSec search found no advisory for `x509-parser`; locked `rustls 0.23.45` and `rustls-webpki 0.103.15` remain at/above the previously checked patched bounds.
- Errors and renewal logs remain redacted; no request body, profile name, configured path, or key is logged by the reviewed slice.

## Evidence and limitations

| Evidence | Result | Limitation |
| --- | --- | --- |
| `git diff --check 5115afde..a06d95f2` and exact `git show a06d95f2:<path>` review | Clean delta; frozen full SHA verified | Shared checkout contains unrelated documentation work, so only exact commit blobs were assessed |
| Exact-tree secret pattern scan | No matching committed key/certificate/credential files | Pattern scan is not entropy-based detection |
| Exact `Cargo.lock` inspection | 240 packages, 234 registry sources/checksums, no Git sources | Comprehensive `cargo audit` was unavailable because `cargo`/`rustc` were absent from the mandated non-login shell |
| Coordinator-provided execution evidence | 15 server tests pass at this frozen source | Not independently rerun; no real slow-client saturation, multi-interface, Pi, time-sync, filesystem-fault, or NAS test |
| Local HTTPS probe referenced by coordinator | Proven separately at `11fb917a11a40c0d31674f12690500b78e03aa7d` | Different SHA and therefore contextual only, not execution proof for `a06d95f2` |

The remaining LOW atomic-write reconciliation must stay owned and visible, but no open HIGH or CRITICAL issue remains in this bounded server slice. A source change makes this approval stale.
