# BUG-0121 — Late discovery overrode explicit saved-server selection

- Status: **FIXED_SOURCE / DOM_VERIFIED**,2026-10-20; real GUI NOT_RUN.
- Found by independent review of `ad64644`, before delivery.

Saved-server buttons connect without submitting the manual form. Initial
cancellation only watched that form and disconnect/reject buttons, so a late
discovery result could override the selected server while its connection was
pending or failed.

Correction in `fdb80b5`: capture saved-server button intent, invalidate the
discovery generation before the ordinary connection handler. Focused DOM
regression proves a late single-server result cannot submit another connection.
Native discovery DOM suite7/7 passes; independent review closes the finding.
TLS trust checks are unchanged. No physical WebKit race proof is claimed.
