# BUG0123 — Native theme background exceeds CSS substitution limit

Date2026-10-21, author/root, status **FIXED_SOURCE / regression PASS**.
Scope: native theme renderer, not deployed web or media playback.

Integrated browser proof failed: `data-theme-background=true`, but background
was `none` for Easter. Multi-megabyte native base64 PNGs exceeded the CSS custom
property substitution limit. Existing Christmas/Halloween file fallbacks hid
the same defect. Confirmed cause, not calendar/absent file.

`460fa21` converts strictly validated native PNGs to locally minted, bounded
Blob URLs; `8c31f9c` fixes the first correction's own-URL rejection. Only live
issued URLs pass, reload/error revokes them, cap12MiB. No arbitrary external
blob, SVG or remote CSS. CSP adds blob only for images, never scripts/connect.

The final integrated regression hashes rendered Blob bytes against this event's
receipt for four events/two tones and removes all backdrop pixels during player
cutout. Screenshots actually inspected, rabbit anatomy/64×51.2 scale preserved.
Les premiers échecs restent des échecs ; preuve finale sur le vrai chemin
load/apply de l'interface, pas uniquement sur une fonction isolée.
