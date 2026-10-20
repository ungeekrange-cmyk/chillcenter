# Independent code operational delta review — v0.7 Mac DMG `3e174b9`

- **Review type / reviewer instance:** code / `/root/v07_final_code`
- **Lane:** `FULL_ASSURANCE`, accelerated operational delta review
- **Parent reviewed SHA / verdict:** `ac5253328da3bd2cb6a65e9913c0bbe5b8d3d33b` / `APPROVED_SCOPED` in `component-updates-code-ac52533.md`
- **New source SHA:** `3e174b9e6ed89e08f67e763c3b30028c15767080`
- **Source delta:** `scripts/package-macos-dmg.py` and `scripts/helpers/test_macos_dmg.py` only
- **Independence:** reviewer did not author the source; source-non-mutating
- **Date:** 2026-10-20
- **Current verdict:** **APPROVED_SCOPED**

No P0, P1 or P2 correctness, compatibility, architecture or maintainability
finding remains in `ac52533..3e174b9`. The previously approved runtime, updater,
Mac application bundle and trust paths are unchanged; this delta adds only
manual-install media generation and its focused tests.

## Delta review

- `validate_paths` requires absolute `.app`/`.dmg` paths, a real non-symlink app
  directory, a previously absent destination, output outside the input bundle,
  the production bundle identifier and present numeric version/minimum-OS
  metadata. Existing files and dangling links are refused without mutation.
- The source app passes deep strict code-sign verification before copying. The
  copy preserves symlinks and is verified again, so a partial or signature-
  altering copy cannot reach image creation. The DMG contains only the verified
  `Chill Center.app`, `/Applications` symlink and bilingual plain-text manual.
- `/usr/bin/hdiutil create` explicitly selects compressed read-only `UDZO` and
  HFS+, followed by `hdiutil verify`. The helper neither launches nor installs
  the app and does not touch the Tauri `.app.tar.gz` updater payload.
- Staging is created inside the destination parent. `os.link(stage,
  destination)` is therefore a same-volume, atomic, exclusive publication: it
  cannot replace a target created after preflight, and removal of the temporary
  staging link leaves the published inode intact. Failure before that link
  leaves no destination.
- The five unit tests cover normal validation, preservation of existing files
  and dangling links, recursive/QA identity rejection, the racing-destination
  case and survival after staging cleanup. No runtime, dependency, lockfile,
  endpoint, release key or updater behavior changed; runtime-lifecycle approval
  therefore remains reusable.

## Evidence

Reviewer-run against exact source
`3e174b9e6ed89e08f67e763c3b30028c15767080`:

- `git diff --check ac52533..3e174b9` — PASS; exactly the two stated files.
- `python3 scripts/helpers/test_macos_dmg.py -v` — PASS, 5/5.
- `python3 -m unittest discover -s scripts/helpers -p 'test_*.py'` — PASS,
  19/19 packaging-helper tests.
- Candidate
  `/private/tmp/chillcenter-v07-dmg-ac52533/Chill-Center-0.7.0-macos-arm64.dmg`
  is 46,881,214 bytes with SHA-256
  `7c352bdb708b5d53a6f8a332e7ff33a77c54017043c90a0427d28e64902c47a5`;
  reviewer-run `hdiutil verify` reports a valid image checksum.
- A temporary reviewer-run `-readonly -nobrowse` mount exposed exactly the app,
  `Applications -> /Applications`, and `Installation.txt`. Mount flags included
  `read-only`, `nodev`, `nosuid`, `noowners` and `nobrowse`. Deep strict code-sign
  verification passed for the mounted app; `diff -qr` against
  `/private/tmp/chillcenter-v07-minos-ac52533/Chill Center.app` reported no
  difference. The volume was then ejected and the empty mount directory removed.

The binary inside the DMG is intentionally the already reviewed `ac52533` app;
`3e174b9` changes only the external packaging tool and tests. This preserves the
runtime evidence while binding the new manual-install artifact to the reviewed
packager delta.

## Limitations and routing

This code verdict does not claim notarization, Developer ID signing, Gatekeeper
behavior on another Mac, a Finder drag/drop performed by a human, GitHub upload,
or completion of the separately owned native AppKit cancellation/test gate. It
does not authorize or claim Pi, SSH, NAS or production-host contact. Publication
must use the exact verified DMG bytes/hash above and remain subject to the
independent exact-source test, security and release-verification gates.

## Verdict

**APPROVED_SCOPED for code at exact source SHA
`3e174b9e6ed89e08f67e763c3b30028c15767080`.** No code correction is requested.
Any later source, packager or DMG-byte change makes this verdict stale; a
report-only carrier commit does not.
