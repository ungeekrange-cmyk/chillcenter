# ADR0027 — Content-first mobile UI and iPhone playback

[Français](ADR-0027-mobile-content-and-iphone-playback.md) · **English**

- Date: 2026-10-17. ACCEPTED_BY_USER_CHANGE_REQUEST.
- Supersedes only always-visible mobile search and bottom navigation in phone
  landscape from [ADR0026](ADR-0026-mobile-web-presentation.en.md). Mac/web,
  API and server separation guarantees remain unchanged.

Mobile catalogues open directly on posters. A magnifier expands search and
filters; changing destination folds them without clearing selections. Phone
landscape uses a left navigation rail. Season/show tools move after episodes
into collapsed sections. Desktop and collection-add pickers retain their behavior.

The web engine keeps MediaSource when available and accepts ManagedMediaSource
as a fallback. Remote playback must be disabled before attaching this fallback,
because no alternative AirPlay source is provided. There is no new decoder,
dependency, migration or server route.

Autoplay refusal is no longer ignored: a fresh touch resumes the already
prepared element without reopening its stream. The retry shares the normal
control's lease guard and cannot undo a server-ordered pause. Video-native
fullscreen is requested directly in the gesture when panel fullscreen is
unavailable; canvas rendering cannot use this fallback. Rotation guidance is
not shown when the phone is already in landscape.

Chromium validation using real synthetic containers is not treated as proof of
native Safari compatibility. The two reported videos still require physical
device validation, particularly MP3 audio inside AVI. Previous static assets
are retained for a web-only rollback.
