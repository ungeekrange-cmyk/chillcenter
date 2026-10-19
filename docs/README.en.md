# Chill Center documentation

[Français](README.md) | **English**

Bilingual reference documentation for the v0.6 Mac client and API 2026-10-14.
Each document pair carries the same guarantees, identifiers, limits and
caveats; examples do not contain home configuration. The OpenAPI contract
remains a single language-independent specification. A documentation change
does not imply a new bundle or deployment.

| Topic | Français | English |
| --- | --- | --- |
| Overview and building | [README](../README.md) | [README](../README.en.md) |
| Current architecture summary | [Architecture](architecture/OVERVIEW.md) | [Architecture](architecture/OVERVIEW.en.md) |
| Third-party API integration | [Guide API](api/INTEGRATION-FR.md) | [API guide](api/INTEGRATION-EN.md) |
| Independent versions and updates, candidate | [Guide](updates/GUIDE-FR.md) | [Guide](updates/GUIDE-EN.md) |
| Theme packs and calendar | [Thèmes](../themes/README.md) | [Themes](../themes/README.en.md) |
| v0.6 delivery and known limitations | [Release](releases/REL-V0.6.md) | [Release](releases/REL-V0.6.en.md) |
| Next candidate: fixes 82–84 and web parity (no AI) | [Rapport](implementation/WEB-PARITY-20261017.md) | [Report](implementation/WEB-PARITY-20261017.en.md) |
| Web-assets-only Pi deployment | [Livraison](releases/REL-WEB-PARITY-001-web-deployment.md) | [Delivery](releases/REL-WEB-PARITY-001-web-deployment.en.md) |
| Smartphone web client delivery | [Livraison mobile](releases/REL-WEB-MOBILE-001.md) | [Mobile delivery](releases/REL-WEB-MOBILE-001.en.md) |
| Mobile R2 and iPhone playback fixes | [Livraison R2](releases/REL-WEB-MOBILE-002.md) | [R2 delivery](releases/REL-WEB-MOBILE-002.en.md) |
| AVI MP3 and mobile controls R3 | [Livraison R3](releases/REL-WEB-MOBILE-003.md) | [R3 delivery](releases/REL-WEB-MOBILE-003.en.md) |
| Fullscreen notice and camera insets R4 | [Livraison R4](releases/REL-WEB-MOBILE-004.md) | [R4 delivery](releases/REL-WEB-MOBILE-004.en.md) |
| Browser-side AVI MP3 and mobile fullscreen | [ADR0028](architecture/decisions/ADR-0028-web-mp3-fallback.md) | [ADR0028](architecture/decisions/ADR-0028-web-mp3-fallback.en.md) |
| Content-first mobile and iPhone playback decision | [ADR0027](architecture/decisions/ADR-0027-mobile-content-and-iphone-playback.md) | [ADR0027](architecture/decisions/ADR-0027-mobile-content-and-iphone-playback.en.md) |
| Smartphone adapter decision | [ADR0026](architecture/decisions/ADR-0026-mobile-web-presentation.md) | [ADR0026](architecture/decisions/ADR-0026-mobile-web-presentation.en.md) |
| Contributing | [Contribuer](../CONTRIBUTING.md) | [Contributing](../CONTRIBUTING.en.md) |
| Security | [Sécurité](../SECURITY.md) | [Security](../SECURITY.en.md) |
| Public repository preparation | [GitHub](publication/GITHUB-REPOSITORY.md) | [GitHub](publication/GITHUB-REPOSITORY.en.md) |
| Optional GitHub Pages | [Pages](publication/GITHUB-PAGES.md) | [Pages](publication/GITHUB-PAGES.en.md) |

## Traceability and maintenance

The [chronological context](PROJECT_CONTEXT.md),
[accepted decisions](architecture/decisions/INDEX.md),
[bugs](debug/INDEX.md), reviews and dated evidence remain in their original
language. They are not retroactively translated or rewritten: their status
and approvals remain tied to the exact cited sources. The v0.6 release notes
summarize delivery without claiming every limitation has been resolved.

When changing a reference document, update both languages in the same change.
Do not translate contract identifiers, field names, commands, hashes or example
values. Check links, limits and caveats in both versions:

```sh
node --test scripts/test-bilingual-docs.mjs
git diff --check
```

This documentation supplement follows the immutable `v0.6` tag; it does not
move the tag and retains the signed bundle built from `bac22ca`.
