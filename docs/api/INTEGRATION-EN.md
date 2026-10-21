# Integrating a third-party client — Chill Center API 2026-10-14

[Français](INTEGRATION-FR.md) | **English**

The official Mac client and third-party clients communicate with the server
through a JSON API over HTTPS. Video bytes also use HTTPS, exclusively through
HTTP `Range`: the server never transcodes and clients never access the NAS.

The canonical contract is [openapi-2026-10-14.json](openapi-2026-10-14.json),
served by each instance at `GET /api/2026-10-14/openapi.json`. The version is a
**contract date**, not the date of the request. The base is:

```
https://nom-ou-ip-du-serveur:8443/api/2026-10-14
```

Since server0.7.1, `/api/v1` is retired and returns404; the dated base is the
only served API. Third-party integrations must migrate their request prefix.
Responses include `X-Chill-Api-Version: 2026-10-14`;
`GET /server` also advertises versions and capabilities actually served.
Clients must ignore unknown response fields, but must never send undocumented
fields in request bodies.

## Independent component updates (server 0.7.0 and later)

These are additive routes in the same dated API; no legacy alias is served.
`GET /admin/updates` reports actual installed server/web/theme versions, policies,
bounded operation receipts, availability, helper readiness and the server OS local
UTC offset. A status read does not contact GitHub or trigger installation.
An older server returns 404: disable only these controls, not the catalogue.

`PUT /admin/updates/policy` (also `POST /admin/updates/policy`) accepts exactly
`component`, `mode`, `hour`, `minute`, `expected_revision`. Components are
`server`, `web`, `themes`; modes are `manual`, `check`, `automatic`. Defaults
are automatic at 02:00 local server time. Read the policy revision first; a
concurrent change returns `UPDATE_STATE_CHANGED` and requires a fresh read.

`POST /admin/updates/check` and `POST /admin/updates/install` accept only
`{"component":"server"}` (or `web`/`themes`). A 200 is a queued/coalesced
operation receipt, **not** installation success: poll status for the outcome.
Since server 0.7.2, `check_available: true` is independent of `helper_available`.
Without a helper, manual discovery uses an unprivileged worker: fixed repository,
metadata/signatures only, no payload archive download or installation. At most
three workers (one per component), coalescing and a five-minute cache;
`cached: true` identifies a reused receipt. Signature/network errors and
incompatibility remain explicit results, never a false “up to date”. Status and
catalogue remain usable during checks. Discovery state is separate from install
receipts and the helper's anti-rollback ledger.
`UPDATE_HELPER_NOT_INSTALLED` concerns installation: a local administrator must
install the separate signed helper. Playback and jobs defer installation.
Scheduling is not active without that helper.
No route accepts an arbitrary URL, path, key, tag, shell command or forced
interruption. Mac checks and explicitly confirmed installation are device-local,
not server API operations. See the [integration contract](../updates/CONTRACT.md)
and [approved security decision](../architecture/decisions/ADR-0029-component-releases-updates.md).

## TLS connection and LAN scope

Chill Center targets a home LAN, without authentication. Anyone who can reach
the API can administer the catalogue and choose a profile. Do not expose it
to the Internet or put a public proxy in front of it.

On first connection, present the server address and certificate SPKI fingerprint
to the owner, then store the server identity. Certificates renew annually and
are valid for 18 months. Never use `curl -k`, `--insecure` or globally disable
TLS. For a command-line client, supply the approved public certificate:

```sh
BASE='https://chillcenter.local:8443/api/2026-10-14'
curl --fail-with-body --cacert ./chill-center-public.pem "$BASE/server"
```

There is no cross-origin CORS: a website or Swagger UI loaded from another
origin cannot call the server. The web client served by the instance is
accepted with `Sec-Fetch-Site: same-origin` and, if present, an `Origin`
matching the server origin exactly (validated scheme and host/port). A native
client or trusted local service sends no browser origin headers. Their absence
does not add authentication; do not forge them to bypass an origin error.

On the web client's optional HTTP listener, browsers may omit `Sec-Fetch-Site`;
any supplied `Origin` must still match the server. HTTPS-only routes remain
rejected on that listener. Examples in this guide use HTTPS.

## Errors and concurrent writes

Application errors use this format, without NAS paths or secrets:

```json
{"code":"PROFILE_CONFLICT","message":"Opération refusée ou temporairement indisponible.","request_id":"b02b03d5-5c84-4055-b7b5-9a9a7921f41f"}
```

Keep `request_id` for diagnostics. HTTP status matters: `400/422` invalid
request, `403` rejected origin/host, `404` missing identifier, `409` conflict
or active lease, `412` changed media/subtitle, `413` size limit, `416` invalid
video range, `429` worker saturation and `5xx` unavailability. Cancelled
requests or saturated database reads may also return `408`, `503` or stable
codes such as `BUSY`/`REQUEST_TIMEOUT`; retry only explicitly safe reads,
with bounded attempts.

Shared objects carrying `revision` use optimistic concurrency. After `409`,
reread the object, present the conflict and submit a mutation with the actual
revision read. Do not invent a global CAS rule: not every personal mutation
(`watched`, `rating`, preferences, playlist deletion) uses `expected_revision`.
OpenAPI defines required fields per operation. Requests are strict (unknown
fields rejected); responses are extensible (ignore additional fields).

## Profiles, catalogue and collections

The profile is always explicit in the URL. Sagas are shared across the home;
playlists belong to the URL's profile. `profile`, `media`, `playlist`,
export/import and TMDB job identifiers are UUIDs. A `saga` identifier is opaque
and the default `series` parameter is a URL-encoded name: do not convert them
to UUIDs.

Deleting a profile requires `x-chill-confirm-delete` with exactly the URL's
`{profile}` UUID, not `true` or a generic confirmation.

`PUT .../preferences` replaces preferences: send audio_language,
subtitle_language and autoplay_seconds together. An omitted field returns to
its historical default (English/English/10 seconds), not its previous value.
For its new 20 s default the Mac client sends `POST /profiles` with
`initial_preferences: {"audio_language":"en","subtitle_language":"en","autoplay_seconds":20}`:
creation and preferences are atomic. Without this optional field, older clients
retain their behaviour; existing profiles are not rewritten. An empty subtitle
language disables automatic preference selection; `autoplay_seconds: null`
disables automatic next-video playback. Allowed delays: 0, 5, 10, 15, 20, 30 s.

The server catalogue is paginated (`offset`, `limit`, maximum 100, default 48),
even though the official interface loads pages continuously. For a lightweight
integration, request only what is visible:

```sh
PROFILE='11111111-1111-4111-8111-111111111111'
curl --fail-with-body --cacert ./chill-center-public.pem \
  "$BASE/profiles/$PROFILE/catalog?offset=0&limit=48&category=film&available=true"
```

Responses contain `items`, `has_more` and `series_counters`. Each counter may
provide an opaque `series_key`: `tmdb:tv:<id>` for identified series,
`media:<stable_uuid>` for unidentified groups, or the title alone only when
there is a single matching group. Never merge groups solely by title:
same-name shows such as different Doctor Who versions remain distinct.
`Media.path` is only a relative catalogue path, not a URL or a storage access
right. Follow OpenAPI revisions and bodies exactly when editing metadata,
sagas or ordering.

### Local catalogue synchronization (desktop client)

The official client caches metadata and thumbnails privately on the device;
video streams remain exclusively server-supplied. Third-party clients may
use the same approach with these additive dated-API extensions.

`GET /profiles/{profile}/catalog/revision` returns only `{ "revision": "…" }`,
without reading metadata, creating a checkpoint or opening a transfer. It is
the same opaque shared/personal cursor used by `changes`: compare for equality,
never numerically. A change affecting only another profile does not invalidate
it. This GET suits browsers without the Mac's SQLite mirror: unchanged means
keep the current view; changed means reload through the existing mechanism.
It is neither a delta nor an atomic catalogue capture. On older servers, only
`404 ROUTE_NOT_FOUND` permits fallback to ordinary reloading; a profile/network
error does not mean “unchanged”. Compatible addition to2026-10-14;
historically absent from Pi source ffcc6349 at the15October check.

`POST /profiles/{profile}/catalog/changes` accepts `{ "known_revision": "…" }`.
If unchanged, it returns `{ "revision": "…", "unchanged": true }` without
materializing the catalogue. Otherwise, the server compares the known durable
checkpoint with a coherent capture and returns only additions/updates in
`items` and removed identifiers in `deleted_ids`. First connection, an unknown
cursor or an evicted cursor yields `reset: true`, with the complete catalogue
in `items`. Responses also contain sagas, playlists, series revisions,
`season_metadata` and `snapshot_id` for later pages. Items are
`{ "media": {…}, "revision": n }`.

`GET /profiles/{profile}/catalog/changes/{snapshot}?offset=…` reads immutable
pages of at most 100 items. Snapshots are server/profile-bound, retained for up
to 120 seconds, with at most three active transfers and 32 MiB serialized
globally. Single-page responses occupy no transfer slot; the last page releases
the transfer. If that final response is lost, `404 SYNC_SNAPSHOT_NOT_FOUND`
requires restarting the POST without publishing partial results. The revision
is an opaque durable cursor surviving restart, without process identifiers;
never compare it numerically. Official restore changes its generation to avoid
reusing old revisions. Checkpoints contain only IDs and SHA-256 signatures,
are deleted with their profile and limited to four per profile and 16 MiB
globally. Apply the delta in one local transaction; retain the old mirror on
failure. Historical `/catalog/snapshot` routes remain available and unchanged.

Legacy pages use
`GET /profiles/{profile}/catalog/snapshot/{snapshot_id}?offset=100`.
The legacy snapshot is immutable for two minutes and server/profile-bound.
Prefer `changes` to avoid resending unchanged items. A fourth active transfer
returns `429 SYNC_BUSY`; `413 SYNC_TOO_LARGE` means a limit was reached.
Playback positions, speeds and preferences are not mirrored and do not
invalidate a delta; watched status and ratings invalidate only their profile.

A concurrent scan during capture may produce `409 SYNC_SNAPSHOT_CHANGED`:
retry the `POST` with bounded attempts (the Mac uses at most three, 100 ms
apart), then retain the old cache if conflicts persist. Epochs are checked
before/after all reads; incoherent pages are not published under a new revision.

Auxiliary documents (`sagas`, `playlists`, `series_revisions`, `season_metadata`)
are complete only in the POST response. Later GET pages carry empty arrays or
objects in these fields: do not replace the initial document with them. The
Mac limits accumulated JSON across all pages, including documents, to 32 MiB.
Old complete checkpoints from the undelivered prototype are disposable cache
and cleared in favour of bounded signatures; catalogue/personal data is not erased.

The Mac stores metadata in private SQLite per server/profile, plus original
artwork and JPEG thumbnails. Grids, details, suggestions, series and lists read
locally; original images load progressively, one at a time, outside the player
lock. Artwork budget: 2 GiB per profile. Catalogue changes publish a coherent
updated copy; unchanged means only the revision check. Videos remain HTTPS
Range streams, never mirror data. Failed refresh retains the last complete
copy and marks it stale without network retries at each navigation; startup,
focus or explicit refresh permit another attempt. No mutation is replayed
offline. After a valid authoritative profile list, only caches for deleted
profiles on that server are purged; failures are reported and symlinks refused.

Archives containing playlist artwork use `data.json.format_version` 2;
the ZIP container/manifest stays at format 1. The new importer accepts documents
1 and 2. Old servers cannot import document 2: restore their earlier private
backup for a full rollback. Documents without the extension stay at format 1.

Paths remain relative. Snapshots expose no absolute NAS path, operational
database or other profile. Never reuse one server/profile's cache for another.
Historical catalogue routes remain unchanged.

A playlist may have custom artwork:
`PUT /profiles/{profile}/playlists/{playlist}/poster?expected_revision=n`,
JPEG/PNG/WebP up to 8 MiB, then
`GET /profiles/{profile}/playlists/{playlist}/artwork`. Optional `poster`
contains its URL. Without custom artwork, the client composes a tile from the
first nine members.

Series metadata can be identified/edited using TMDB scopes `series` and `season`.
`SeriesResponse.tmdb_id` identifies the selected show (same-name shows remain
distinct); `season_metadata` holds known metadata per season number.
`PUT /profiles/{profile}/series/{series}` returns `episodes`, `revision` and the
opaque `key` to reuse after renaming. Season searches use the show's `tmdb_id`.
Artwork may be imported without closing details using
`PUT /profiles/{profile}/media/{media}/series/poster` or
`PUT /profiles/{profile}/media/{media}/season/poster`, CAS via `expected_revision`.
Read season artwork through
`GET /profiles/{profile}/series/{tv}/seasons/{season}/artwork`.
Accepted formats remain JPEG/PNG/WebP, maximum 8 MiB.

Existing-client compatibility: series segments are **titles by default**, even
when beginning with `media:` or `tmdb:tv:`. Their historical aggregated view
remains in GET and PUT under the dated base; `?selector=title` equals
omitting the parameter. A title PUT applies common metadata to the entire view
with the same global CAS and rename-collision rejection as before. It changes
no TMDB ID, file or season assignment. When distinct shows share a title, GET
does not pretend the first show's identity/seasons are shared:
`tmdb_id: null`, `season_metadata: {}`. To isolate same-name shows, new clients
use catalogue keys `media:<uuid>` or `tmdb:tv:<id>` **with `?selector=key` in
GET and PUT**; prefixes never implicitly select that mode. Other `selector`
values return 400. A legacy title view does not merge persisted identities.
This optional 2026-10-14 addition needs no old-web-client update for its
historical operations. The new Mac bundle sends the selector explicitly;
11aa529 remains an older partial client, notably for new key-based edits.

## Measured progress — additive change in preparation

`GET /profiles/{profile}/playback-progress` reads profile resumes with a total
duration **measured by the player**, persisted on a lease-validated `Progress`
event. The server does not probe videos, transcode or infer duration from TMDB.
Legacy resumes without a measured duration are omitted until playback occurs.

The response contains `revision`, `unchanged` and, when changed, `items` (up to
30000), with `media_id`, `position_ms`, `duration_ms`, `watched`. Supply the
previous opaque token as `known_revision` to receive `unchanged: true` without
items when nothing changed. A restart invalidates the token. This read does not
change catalogue revisions; clients query this separate stream in the background,
at most once every30seconds during normal use. Cache it separately per
server/profile and never publish a late response from an old context.

“In progress” includes unwatched videos whose position is at least5% of their
measured duration. Thumbnail bars and detail percentages use that same duration;
no approximate percentages. Position does not prove content was watched after a
seek: the automatic watched rule is unchanged. Release0.7.2 lacks this route404;
disable only this feature and keep the remaining client compatible. No implicit
household Pi deployment is authorized by this documentation.

## Playback: lease, SSE and Range

Streaming requires acquiring the profile's lease first. Use a persistent
`device_id` UUID per device; the server allocates `session_id`. Normal sequence:

1. `POST /profiles/{profile}/playback/acquire` with `media_id`, `device_id`,
   `context`; the returned lease contains the server-assigned `session_id`.
2. Retain the lease (session, generation, media revision, expiry) and open
   `GET /media/{media}/stream` with `X-Chill-Session`, `X-Chill-Generation`
   and a single `Range`.
3. Renew with `heartbeat` about every 5 s; send `progress` with a strictly
   increasing sequence and millisecond values.
4. Listen to SSE `playback/events`. A pause event requires stopping locally,
   even if the player still has buffered bytes.
5. On stop or media change, send final progress then `release`. For a local
   pause retaining the buffer, the Mac keeps engine and lease, sends position
   and continues `heartbeat` without watched-time credit (ADR0022). Other
   clients may release while paused but must reacquire and reload; never
   reuse an expired/revoked lease.

`Progress.final_elapsed_ms` is real playback time in the final 10% of a video,
excluding pause, buffering and seeks. For durations of at least 100 s, the
server marks watched after 10 accumulated seconds. Speed ×2 does not double
that time. Allowed speeds: 0.5, 0.75, 1, 1.25, 1.5, 1.75, 2.

Functional amendment of 16 October 2026 (no DTO/URL change): for known durations
under 100 s, mark watched at the end after continuous coverage of the entire
final tenth within the same lease. Send ordered events before/during that
segment, including the last at `position_ms == duration_ms`, without fabricating
`final_elapsed_ms`. An EOF reported paused is accepted if the last interval
started playing. Seeking, rewinding, duration changes or advancing while
loading invalidate coverage; a stationary pause preserves it. Evidence is
bounded by position **and** actual monotonic elapsed time at the declared
speed; positional drift tolerance never supplies missing time. This does not
prove human attention. A new session/transfer/restart requires fresh coverage.
Personal counters remain real milliseconds and existing states are not
rewritten; web/Mac clients need no new fields. An older server retains the
original rule, so this exception requires a server update.

Same-profile transfer requires deliberate confirmation: propose, confirm,
listen for pause and the previous player's final ACK, then play only with the
new lease. Never try to extend an old lease during transfer.

Never request the whole video file. Send one range, retain `ETag` and send it
in `If-Match` on later reads. `206` is normal; `412` means changed media and
`416` an invalid range. The client buffer is local, not direct NAS access.

## 64-bit integers and a safe Python prototype

`media_revision` is an opaque unsigned 64-bit integer (`uint64`); preserve it
exactly in `Lease`, `PlaybackControl` and `Progress`. Python and Rust do so
natively. JavaScript clients must not use `JSON.parse`/`Number` for values
potentially above `2^53 - 1`: use a lossless JSON parser, retain the value as
`BigInt` or text and emit exactly the same **JSON number**. A JSON string is
not interchangeable without contract agreement, and `JSON.stringify` cannot
directly handle `BigInt`: use a suitable lossless serializer. Rounded values
are rejected with `LEASE_INVALID`.

This standard-library Python prototype calls no external provider. Replace
synthetic UUIDs and the approved public certificate; it acquires a lease,
requests a range, renews, then releases. TLS is never disabled.

```python
import json
import ssl
import urllib.request

base = "https://chillcenter.local:8443/api/2026-10-14"
profile = "11111111-1111-4111-8111-111111111111"
media = "22222222-2222-4222-8222-222222222222"
device = "33333333-3333-4333-8333-333333333333"
tls = ssl.create_default_context(cafile="./chill-center-public.pem")

def request(path, method="GET", payload=None, headers=None):
    data = None if payload is None else json.dumps(payload).encode("utf-8")
    request_headers = {"Accept": "application/json", **(headers or {})}
    if data is not None:
        request_headers["Content-Type"] = "application/json"
    req = urllib.request.Request(base + path, data=data, method=method, headers=request_headers)
    with urllib.request.urlopen(req, context=tls, timeout=20) as response:
        return response.status, {name.lower(): value for name, value in response.headers.items()}, response.read()

control = None
try:
    _, _, raw = request(f"/profiles/{profile}/playback/acquire", "POST", {
        "media_id": media, "device_id": device, "context": {"kind": "standalone", "id": None}
    })
    grant = json.loads(raw)             # Python preserves 64-bit integers.
    lease = grant["lease"]
    control = {key: lease[key] for key in ("session_id", "generation", "media_id", "media_revision")}
    status, headers, chunk = request(f"/media/{media}/stream", headers={
        "X-Chill-Session": control["session_id"],
        "X-Chill-Generation": str(control["generation"]),
        "Range": "bytes=0-65535"
    })
    assert status == 206 and headers["content-range"].startswith("bytes ")
    request(f"/profiles/{profile}/playback/heartbeat", "POST", control)
finally:
    if control is not None:
        request(f"/profiles/{profile}/playback/release", "POST", {"lease": control, "progress": None})
```

## Administration, TMDB, AI and archives

`/admin/*` endpoints share the unauthenticated LAN scope. The default catalogue
page cannot trigger scans; import/export and library configuration are also
administrative operations.

Send a TMDB key only through `PUT /admin/tmdb`, in `credential` marked
`writeOnly`. A v3 key is exactly 32 hexadecimal characters; a v4 token is
20–2,048 graphic characters after an optional `Bearer ` prefix. Leading and
trailing whitespace is accepted then trimmed, not part of the secret.
`profile_id` is required only when `auto_enrich` is `true`. The server never
returns the secret and excludes it from exports. Status reveals only
`configured`, credential type, source and automation state. Never put a v3
key or Bearer token in a URL, log, screenshot or error response.

Track TMDB jobs with `GET /admin/tmdb/jobs/{id}`; jobs can be cancelled/resumed.
Limits: 2 active jobs, 8 completed jobs retained for 15 minutes, 3,000 media
per worklist and 2 MiB of persistent state. Exceeding limits or a full queue
returns `413` or `429`; wait and reread status instead of submitting duplicates.
Show counters without assuming the internal media worklist is public.

Cancellation does not immediately admit a third worker: its reservation is
released only after actual completion, including any provider request bounded
to 15 s. History expiry is reconciled on next access/update or restart, with
no permanent timer. Expired history may return `404 TMDB_JOB_NOT_FOUND`.
Atomic writes may briefly occupy both old and new state files (2 MiB each).

AI does not run on this server. `GET /profiles/{profile}/ai/context` provides
a profile-scoped snapshot (maximum 3,000 entries; `watched`, `available`,
`category` or CSV `categories` filters). Send a plan to `ai/apply` only after
human confirmation and with both revisions supplied by the snapshot.

Exports are asynchronous, then downloadable as ZIP when ready. Import has
four stages: ZIP to preview, preview to profile/root mapping plan, confirmed
plan to application, or cancellation. Limits: 1 GiB compressed ZIP, 2 GiB
uncompressed, 20,000 entries. Videos and secrets are never imported/exported.
Plans are temporary and may expire; `409 IMPORT_PLAN_STALE` requires a new preview.

## Compatibility and limits

### Compatible additions of 16 October 2026

API base remains **2026-10-14**, the sole served version since server0.7.1. Additive capability
`multi_library_locations`: `GET /admin/library` provides
`additional_video_roots` alongside `video_root` and `revision`.
`PUT /admin/library` accepts this optional array: omission or `null` retains
additional locations; `[]` removes them. Older clients may therefore edit
the main root without deleting the others. Maximum 16 total locations,
absolute, distinct and non-overlapping. The server neither creates nor mounts
them. Revision checks and rejection during a scan remain unchanged.

A scan processes all locations sequentially under global budgets of 300 s,
64 MiB of results and 200,000 media. No new state is published if a source
fails. Existing main-root UUIDs do not change; additional roots use a reserved
internal indexing namespace stable across reordering. Clients always receive
opaque identifiers, never arbitrary playback paths. Removing a configured
location deletes no files. Legacy single-main-root archives remain importable;
archives with multi-location identifiers are rejected with
`IMPORT_MULTIPLE_LIBRARY_LOCATIONS_UNSUPPORTED` until explicit mapping is defined.

At launch, `503 MEDIA_FILE_MISSING` means the server confirmed that only the
leaf file is missing in an accessible directory, persisted the entry as
unavailable and published its revision through deltas. No lease is granted.
Show the status, cleanly close launch state and offer to locate/manage the
reference. Mount, parent-directory or permission errors do not prove this:
they remain `MEDIA_UNAVAILABLE` and must not arbitrarily downgrade catalogue
videos. Error codes are extensible; old clients still receive structured
errors without success-body changes.

The server promises no particular codec within MKV/MP4/AVI: decoding and
tracks are the client's responsibility. External subtitles are lease-protected
resources limited to 16 MiB. Uploaded artwork is JPEG/PNG/WebP, maximum 8 MiB.
Ordinary JSON calls are limited to 1 MiB.

Before updating a client, read `GET /server` and enable only advertised
capabilities. Before a household production integration, test Range errors,
lease expiry, resume after transfer, concurrent modification and TLS connection
with a renewed certificate.
