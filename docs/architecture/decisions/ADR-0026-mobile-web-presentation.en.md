# ADR0026 — Web-only smartphone presentation

[Français](ADR-0026-mobile-web-presentation.md) · **English**

- Date: 2026-10-17. Status: ACCEPTED; the user explicitly approved scope and
  concept R1 before integration.
- Extends [ADR0025](ADR-0025-web-shared-ui-parity.md), without API changes.
- Scope: web presentation files and generation of static web assets only.

The web build adds adapters after the common scripts, without editing
`ui/connected` or the Mac client. On smartphones, the sidebar becomes five
bottom destinations: Films, Series, Docs, Shows and More. More opens the full
collection, sagas, playlists and settings. Buttons forward to existing actions;
there is no second catalogue, collection or profile implementation.

The grid retains virtualization and height calculation: three columns by
default, two on very narrow screens to retain usable touch targets. The web
calculator adapts columns and height together; a CSS-only column override
would desynchronize virtual scrolling. Padding reserves navigation and browser
safe areas.

Playback requests real fullscreen directly in the Play gesture, before the
network response, then requests landscape locking if the browser permits it.
Denial does not stop playback: rotation guidance and a real retry are offered.
CSS rotation is never presented as native fullscreen. The WebKit fallback
applies only to a compatible actual video, not canvas rendering. Users can
exit fullscreen; late asynchronous replies are invalidated when closing.
Playback buffering and engines remain unchanged.

Synthetic browser tests do not prove fullscreen policy on a physical phone.
Safari/iPhone and Android still require device testing, especially over HTTP,
where some browser capabilities remain restricted. AI remains excluded from
the web. Existing languages, profiles, themes and filters are retained. No new
contract or server migration is introduced.

Delivery: private backup of the previous assets, replacement of the web tree
only, independent protected-hash and unchanged-PID verification. Rollback uses
the preserved previous assets, without service restart or NAS modification.
