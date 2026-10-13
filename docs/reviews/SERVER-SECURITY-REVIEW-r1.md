# Independent Security Review — server configuration slice

- **Reviewed source**: `5115afde12e73c6c97bd49104e7cd19ac111fdb7`
- **Base**: `c8d2a4c1a3076cbc6065521e11126c153232337a`
- **Reviewed at**: `2026-10-13T16:18:37Z`
- **Reviewer instance**: `/root/server_security`
- **Mode / scope**: independent read-only review; `crates/chill-server/{config,tls,api,main}.rs`, server/workspace manifests, exact lockfile, and deployment example. Profile-store remediation, native client trust/Range code, video/import/lease endpoints, Pi/NAS deployment, and the remaining stories are excluded.
- **Scoped verdict**: `CHANGES_REQUESTED`

The household-LAN/no-auth choice and the post-expiry key-change exception are user-accepted requirements (`F-18`, `F-19`, `ADR-0005`) and are not reopened here. They make preservation of the LAN boundary and TLS invariants especially important.

## Findings

### `SRV-SEC-001` — HIGH — wildcard listener defeats the only access boundary

- **Evidence**: `deploy/server.example.json:2` recommends `0.0.0.0:8443`; `config.rs:42-71` accepts unspecified addresses; `main.rs:65-66` binds them directly. Mutating unauthenticated routes are registered at `api.rs:88-103`. The Host check at `api.rs:117-139` validates a client-controlled header, not the receiving interface or source network.
- **Preconditions / abuse path**: if the Pi acquires any unintended reachable interface (Ethernet, VPN, container bridge, later router forwarding, or an operator error), a remote peer can ignore the self-signed-certificate trust UI, connect to that interface, send an allowed `Host` such as the configured LAN IP, and invoke profile/library writes. No credential or browser Origin is required for a native HTTP client.
- **Asset / impact**: library configuration, profile data, and service availability; the LAN-only trust boundary is broadened silently.
- **Required remediation / retest**: reject unspecified listen addresses in the unauthenticated mode and make the example use the explicit LAN address, or supply and verify an equivalent interface/firewall allowlist as part of the deployment contract. Add a test that `0.0.0.0` and `[::]` are refused absent an explicitly approved compensating boundary, plus a real smoke proving the service is unreachable through an unintended interface.
- **Blocking**: yes.

### `SRV-SEC-002` — MEDIUM — no connection, handshake, or request-time resource bounds

- **Evidence**: `api.rs:101` limits decoded request bodies to 1 MiB and `api.rs:31,62-68` bounds the SQLite queue, but `main.rs:65-68` serves connections without a TLS-handshake timeout, header/body deadline, global connection cap, or per-peer concurrency control.
- **Preconditions / abuse path**: any reachable LAN peer can hold many TLS/HTTP connections or drip request bytes, consuming tasks, sockets, and memory on the 1 GiB Pi before the JSON body limit or database queue applies.
- **Asset / impact**: server availability for all household clients.
- **Required remediation / retest**: add bounded concurrent accepts and explicit TLS/header/body/request deadlines appropriate to streaming endpoints; prove idle/slow clients are evicted while ordinary HTTPS and future media streams remain functional.
- **Blocking**: no by severity, but must be owned and resolved before this slice is approved.

### `SRV-SEC-003` — MEDIUM — `data_dir` ownership and mode are trusted, not enforced

- **Evidence**: `main.rs:26-32` creates and uses an arbitrary absolute `data_dir` without checking symlinks, owner, or group/world-write/read modes. The TLS child directory and key are correctly forced to `0700`/`0600` (`tls.rs:39-43,69-70`), and staged private writes use `0600` (`config.rs:145-159`), but an unsafe writable parent permits replacement/renaming of those children; the SQLite file also inherits filesystem/umask behavior.
- **Preconditions / abuse path**: an operator selects or pre-creates a group/world-writable directory, or installation creates one with unsafe ownership. Another local account can replace configuration/database/TLS paths; a merely world-readable SQLite file can disclose profile/preferences data.
- **Asset / impact**: persistent configuration and profile integrity/confidentiality; TLS key/certificate integrity on a multi-user host.
- **Required remediation / retest**: create or validate a service-owned non-symlink directory with restrictive mode (normally `0700`), fail closed on unsafe ownership/modes, and verify restrictive SQLite/config/TLS artifact modes. Cover unsafe pre-existing parent and symlink cases.
- **Blocking**: no by severity, but must be owned and resolved before approval.

### `SRV-SEC-004` — MEDIUM — certificate reuse trusts schedule metadata and startup time

- **Evidence**: `tls.rs:47-76` reuses any existing certificate when the persisted `renew_at` is later than the supplied clock and names match; it does not parse and cross-check the certificate's SANs, SPKI/key match, `not_before`, or `not_after`. `main.rs:28` performs this decision once at startup; `main.rs:39-45` waits roughly 24 hours for the next check after the initial interval tick is consumed.
- **Preconditions / abuse path**: the Pi boots after power loss before reliable time synchronization, then its clock jumps forward after startup; or the schedule/certificate is partially restored/corrupted. An actually expired or not-yet-valid certificate can continue to be served, and official clients must reject it under ADR-0005.
- **Asset / impact**: HTTPS availability and correctness of the accepted calendar/renewal contract.
- **Required remediation / retest**: require time-sync readiness or a defensible clock sanity gate before serving; parse and validate the persisted certificate against key, SANs, and actual validity at every startup; re-evaluate promptly after clock synchronization. Test cold boot with stale past/future clocks and mismatched/restored artifacts. Preserve the current stable-key behavior and refusal to silently replace a missing established key.
- **Blocking**: no by severity, but must be owned and resolved before approval.

### `SRV-SEC-005` — LOW — fsync failure can report failure after the config was committed

- **Evidence**: `config.rs:159` renames the staged file into place, then `config.rs:160-162` can return an error if parent-directory fsync fails. `LibrarySettings::update` only updates memory after the helper returns success (`config.rs:123-125`). Thus the API can report failure and retain the old in-memory revision even though disk already contains the new revision.
- **Preconditions / abuse path**: a local filesystem or I/O fault rejects directory fsync after rename.
- **Asset / impact**: library configuration consistency and revision-based conflict handling; behavior can change unexpectedly after restart.
- **Required remediation / retest**: represent the post-rename state as committed (with durability failure separately surfaced), or reload/reconcile disk state before returning. Add fault-injection coverage for failure before and after rename.
- **Blocking**: no by severity; ownership is required.

## Controls verified in this slice

- Browser-origin requests are rejected and Host/HTTP2 authority is allowlisted (`api.rs:106-140`); no CORS, cookie, session, redirect, or CSRF token surface exists. This does not authenticate native clients, as intended by F-18.
- JSON bodies are capped at 1 MiB, unknown DTO fields are rejected, the database work queue is bounded, and public errors are generic with opaque request IDs.
- Certificate SAN input is syntactically bounded, issuance uses configured DNS/IP SANs, renewal keeps the service key, missing established keys fail closed, and private-key writes/modes are restrictive. Logs expose only coarse error codes/messages and no key, body, profile name, or absolute configured path.
- No credential/private-key/certificate material was found by the filename-only exact-tree secret-pattern scan. The exact lock contains registry checksums and no Git-sourced packages. Security-critical locked versions include `rustls 0.23.45` and `rustls-webpki 0.103.15`; RustSec marks `0.23.45` as patched for `RUSTSEC-2026-0285` and `rustls-webpki >=0.103.10` as patched for `RUSTSEC-2026-0049`.

## Evidence and limitations

| Tool / source | Exact action | Result | Limitation |
| --- | --- | --- | --- |
| Git | `git rev-parse 5115afd`; `git diff c8d2a4c..5115afde -- ...`; `git show 5115afde:<path>` | Frozen SHA resolved to `5115afde12e73c6c97bd49104e7cd19ac111fdb7`; reviewed exact blobs | Shared checkout advanced afterward; no later source is certified by this report |
| `rg` / Git grep | Static searches for secrets, IP literals, logs, unsafe/filesystem, Origin/Host, cookie/session/redirect constructs | No committed secret material; literals limited to loopback/tests/docs/deployment example; logging is redacted | Pattern scan is not entropy-based secret detection |
| Lockfile inspection | Exact `5115afde:Cargo.lock`, direct/transitive TLS versions, source/checksum fields | 508 packages; 501 registry sources/checksums; no Git sources | Comprehensive `cargo audit` was not runnable because `cargo`/`rustc` were unavailable in the mandated non-login shell |
| RustSec, 2026-10-13 | `RUSTSEC-2026-0285`, `RUSTSEC-2026-0049` version checks | Exact TLS versions are at/above patched bounds | Targeted advisory check only, not a full dependency audit |
| Supplied execution evidence | 11 server tests including HTTP/2 authority; local loopback HTTPS smoke on port 18443 using temporary DB/cert | Reported passing by coordinator | Not independently rerun here; no Pi, NAS, firewall/interface, slow-client, or power-loss/time-sync evidence |

This verdict is only for the server configuration slice at the exact SHA above. It does not approve the full candidate, all planned endpoints/stories, the later combined foundation SHA, deployment, or the accepted TLS-expiry risk beyond confirming that this server code does not silently rotate an established key.
