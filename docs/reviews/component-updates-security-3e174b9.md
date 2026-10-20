# Operational security delta review — Mac DMG `3e174b9`

- **Review Type / Reviewer Instance:** security / `/root/v07_security_delta`
- **Parent Reviewed SHA / Verdict:** `ac5253328da3bd2cb6a65e9913c0bbe5b8d3d33b` / **APPROVED**, [independent report](component-updates-security-ac52533.md)
- **New Source SHA:** `3e174b9e6ed89e08f67e763c3b30028c15767080`
- **Source Delta Scope:** `scripts/package-macos-dmg.py` and `scripts/helpers/test_macos_dmg.py` only
- **Test Evidence Validity:** parent runtime/updater evidence remains valid; focused DMG checks added below
- **Reviewed at:** `2026-10-20T18:11:40Z`
- **Lane:** production/material-risk, full assurance; accelerated operational delta
- **Current Verdict:** **APPROVED**

This reviewer did not author the reviewed design, implementation or tests. The
candidate source and production state were not modified. No Pi, NAS, SSH,
GitHub, cloud, application installation/launch or production signing operation
was performed. The production private signing key was never read, printed or
used. Concurrent uncommitted documentation and reports were preserved.

## Threat-boundary review

The new code creates passive manual-install media from an already built Mac
application. Protected assets are the exact reviewed app bytes, the absent DMG
destination, release provenance and the user's existing Applications directory.
Relevant abuse cases are path/command injection, overwrite or TOCTOU replacement
of a user-owned destination, mutation during copying, active/privileged behavior
inside the packager, and accidental substitution of the signed internal updater
or release trust root.

The delta preserves the boundary:

- Inputs and output must be absolute `.app`/`.dmg` paths. The input must be a
  real non-symlink directory with the normal production bundle identifier; QA
  identity and output recursively inside the app are rejected. Existing files
  and dangling destination links are refused without modification.
- All external commands use fixed absolute macOS tools and argument vectors,
  never a shell. Plist strings interpolated into the plain-text manual are
  restricted to non-empty digits and dots. No URL, command, credential, key,
  executable script or renderer-provided value enters the image.
- Deep strict code-sign verification runs before copying and again against the
  staged copy. Image creation uses fixed `UDZO` read-only format, HFS+, a fixed
  volume name and a fixed `/Applications` shortcut, followed by image checksum
  verification. The packager does not install, launch, resign or overwrite an
  application and adds no autorun behavior.
- The temporary directory is on the destination volume. Final `os.link` is an
  atomic exclusive publication: a target created after preflight makes the
  operation fail rather than replace it, while removal of the staging link does
  not remove the published inode. Failure before publication leaves no DMG at
  the requested destination.
- The internal Tauri `.app.tar.gz` updater, public trust key, signed-manifest
  verification, endpoints, runtime application, LAN API and privileged Linux
  helper are unchanged. Lockfiles, runtime pins and third-party dependencies
  are unchanged; the helper uses only the existing Python standard-library
  build-tool boundary. No authentication, authorization, tenant, session,
  cookie, CSRF, redirect, PII/customer-data, network or cloud boundary changed.

Ad-hoc `codesign` verification proves internal integrity, not source provenance.
That is intentionally supplied by the existing release gate: exact reviewed app
source/hash, exact DMG hash, manifest signature and post-upload verification.
The recorded local DMG is bound to app source `ac52533`; this packager does not
replace those release controls. No CRITICAL, HIGH, MEDIUM or LOW finding was
identified in the source delta.

## Reproducible evidence

| Tool / command | Result | Scope / limitation |
| :--- | :--- | :--- |
| Git 2.54.0 — `git rev-parse HEAD`; `git diff --check ac52533..3e174b9`; name/status, trust-root and lockfile comparisons | PASS; exact SHA `3e174b9e6ed89e08f67e763c3b30028c15767080`; exactly two added files; no key/config/lock/runtime delta | Dirty root-owned documentation/review work was excluded and preserved. |
| Python 3.9.6 — `PYTHONDONTWRITEBYTECODE=1 python3 scripts/helpers/test_macos_dmg.py -v` | PASS: 5/5 | Covers valid input, existing file/dangling link preservation, QA/recursive-output refusal, racing target refusal and hard-link survival after staging cleanup. |
| Python 3.9.6 — `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/helpers -p 'test_*.py' -v` | PASS: 19/19 | Packaging-helper regression suite; no application runtime execution. |
| Python AST parse and changed-source URL/private-key marker scan | PASS | Static syntax/pattern evidence only; no full-history entropy scan. |
| `/usr/bin/codesign --verify --deep --strict --verbose=2 '/private/tmp/chillcenter-v07-minos-ac52533/Chill Center.app'`; plist inspection | PASS; production bundle ID, version `0.7.0`, build `9`, minimum macOS `27.0`, 48 framework files and no framework symlinks | Verifies the local input bundle's internal signature/inventory, not Apple notarization or source authenticity by itself. Detailed library paths are omitted from this report. |
| `shasum -a 256` and `hdiutil verify` on `/private/tmp/chillcenter-v07-dmg-ac52533/Chill-Center-0.7.0-macos-arm64.dmg` | PASS; 46,881,214 bytes; SHA-256 `7c352bdb708b5d53a6f8a332e7ff33a77c54017043c90a0427d28e64902c47a5`; valid image checksum | Exact existing local candidate; not yet production-signed or uploaded. |
| Reviewer end-to-end `package-macos-dmg.py` invocation and direct trivial `hdiutil create` control probe | `NOT_RUN` to completion: both image-creation attempts fail with `Périphérique non configuré` because DiskManagement is unavailable in this reviewer sandbox | The control probe reproduces the environment/tool failure independently of packager inputs. No destination DMG was published. Existing successful build/mount evidence remains separately recorded in `docs/PROJECT_CONTEXT.md` and the independent code report. |

## Limitations / required follow-up

This verdict approves only the security delta at exact source SHA `3e174b9`.
It does not approve notarization, Developer ID identity, Gatekeeper behavior on
another Mac, a Finder drag/drop, a tag, GitHub publication or post-publication
verification. The DMG remains an ad-hoc-signed manual installer, as already
disclosed by the accepted release design and embedded manual. Release tooling
must use the exact verified DMG bytes/hash above or regenerate and re-review the
new bytes; bind them to the reviewed app/source in the signed manifest; keep the
private key outside Git/GitHub; and verify downloaded hashes/signatures after
publication. Any change to source, app bytes or DMG bytes makes the corresponding
evidence stale. No additional security remediation is required for this delta.
