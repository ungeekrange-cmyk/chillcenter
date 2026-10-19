# Theme packs

[Français](README.md) | **English**

Status: declarative embedded pack `themes/default` (format/renderer 1, version
0.7.0). The loader validates the manifest before use and falls back to the
embedded copy when an external pack is missing or invalid. Independent delivery
is specified in [ADR-0029](../docs/architecture/decisions/ADR-0029-component-releases-updates.md).

## Proposed calendar and precedence

Inclusive dates, evaluated at local client midnight in the Northern Hemisphere.
The visual calendar's timezone is separate from the server update timer's zone.

| Ambience | First day | Last day | Automatic priority |
| --- | --- | --- | --- |
| Spring | March 1 | May 31 | Season |
| Summer | June 1 | August 31 | Season |
| Autumn | September 1 | November 30 | Season |
| Winter | December 1 | Last day of February | Season |
| Halloween | October 1 | October 31 | Event overrides season |
| Christmas | November 25 | December 25 | Event overrides season |

These preserve the approved F-21 calendar. The season resumes the day after an
event ends. Disabled ambience preserves light/dark mode; a persistent manual
choice overrides the calendar. Future overlapping events require distinct
explicit numeric priorities; ambiguous ties invalidate a pack.

Each ambience supplies light and dark variants. System mode selects the variant,
not the ambience priority. Decorations must never cover video. Effects respect
reduced motion, visibility and small screens. Event image subjects stay anchored
to the right.

## Loaded format

Each independently versioned pack will contain a declarative JSON manifest
(pack version, compatible format/renderer, identifiers, dates, priorities,
palettes, resources and SHA-256 hashes), images and logos. No JavaScript, HTML,
absolute paths, remote URLs, arbitrary CSS or installation scripts are allowed.
Historical SVGs remain in `ui/connected/branding`; the pack contains only their
approved PNG exports, checked with a size and SHA-256. Rendering and animation
code remain in the application.

The GitHub `themes-v<version>` release will distribute a signed pack independently
of binaries. Verify, stage in a new directory, then switch atomically; retain the
previous pack and a bundled fallback. Reject incompatible formats without
breaking display. Mac and web server maintain their own installed copies. Pack
updates require no client/server rebuild; new effects outside the renderer
contract may still require an application update.

Private keys and local certificates stay outside Git. Only a release-verification
public key may be embedded after explicit approval.
