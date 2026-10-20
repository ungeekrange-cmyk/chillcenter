# BUG-0120 — Discovery announcement preceded HTTPS bind

- Status: **FIXED_SOURCE / LOCAL_REVIEWED**,2026-10-20; Pi/LAN NOT_RUN.
- Found by independent code review at `ad64644`, before delivery.

The initial candidate called `Advertiser::start` before polling an axum server
whose bind was deferred. A client could discover a server not yet listening.
Evidence is source-order review, not a claimed live LAN reproduction.

Correction: explicitly bind a nonblocking TCP listener before advertising and use
`axum_server::from_tcp`. Original fix `fd70855` is preserved on its own branch;
identical main bytes are in `7c5f3e0` and runtime `fdb80b5`. Independent R2–R4
review closes the finding; server compile and143tests pass. Real startup/LAN
interoperation is still required after the permitted deployment.
