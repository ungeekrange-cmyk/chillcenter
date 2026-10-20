# Vérification indépendante après publication — Chill Center v0.7

**Verdict / Verdict: `APPROVED`**

La publication GitHub publique v0.7 est vérifiée pour les octets effectivement
téléchargés. Les cinq releases sont stables, les tags et `main` pointent vers
le carrier exact, les signatures Minisign et les sommes sont valides, et les
archives ont la structure attendue. Cette approbation ne revendique aucun test
Pi, NAS, SSH, Safari, installation, lancement GUI ou lecture vidéo.

The public GitHub v0.7 publication is verified against independently downloaded
bytes. All five releases are stable, tags and `main` resolve to the exact carrier,
Minisign signatures and checksums are valid, and archive layouts match the
release contract. This approval makes no Pi, NAS, SSH, Safari, installation,
GUI-launch, or playback claim.

- **Lane / mode:** `FULL_ASSURANCE`; test evidence audit in `INTEGRATION_REVIEW`
  mode
- **Public repository:** `ungeekrange-cmyk/chillcenter`, `PUBLIC`, default branch
  `main`
- **Reviewed source:** runtime/packaging `3e174b9e6ed89e08f67e763c3b30028c15767080`;
  published carrier `3a043334c5c3c676251ab34bf8bab2e4bfb8df4f`
- **Carrier classification:** `3e174b9..3a043334` contains only `README*.md`
  and `docs/**`; `git diff --check` passed and no runtime/packaging path changed
- **Publisher instance:** `/root`
- **Independent verifier instance:** `/root/v07_publication_verifier`
- **Reviewed at:** `2026-10-20T18:32:23Z`
- **Verifier environment:** macOS 27.0.1 (26A434), arm64; Git 2.54.0;
  GitHub CLI 2.97.0; Minisign 0.12; Python 3.9.6; Node 24.13.0;
  Rust 1.99.1
- **Fresh download root:**
  `/private/tmp/chillcenter-v07-public-verify.ZPodKg`

## Public state / État public

| Release | GitHub ID | Published UTC | Assets | State | Tag target |
| :--- | ---: | :--- | ---: | :--- | :--- |
| `mac-v0.7.0` | 418723980 | 2026-10-20T18:25:26Z | 5 | stable, public | `3a043334c5c3c676251ab34bf8bab2e4bfb8df4f` |
| `server-v0.7.0` | 418724262 | 2026-10-20T18:25:29Z | 14 | stable, public | `3a043334c5c3c676251ab34bf8bab2e4bfb8df4f` |
| `web-v0.7.0` | 418724452 | 2026-10-20T18:25:30Z | 4 | stable, public | `3a043334c5c3c676251ab34bf8bab2e4bfb8df4f` |
| `themes-v0.7.0` | 418724554 | 2026-10-20T18:25:31Z | 4 | stable, public | `3a043334c5c3c676251ab34bf8bab2e4bfb8df4f` |
| `v0.7` | 418725680 | 2026-10-20T18:25:33Z | 22 | stable, public, latest | `3a043334c5c3c676251ab34bf8bab2e4bfb8df4f` |

`main` resolves to the same `3a043334` commit. Release `v0.6` remains public and
stable as GitHub release 415696382, published `2026-10-17T09:17:36Z`, with its
two original uploaded assets; its tag still resolves to
`69da60869506f035137690a0a979850545409e5c`.

## Public byte evidence / Preuves des octets publics

Every file below was downloaded from the public release, not copied from the
publisher's assembly directory. The production public key is byte-identical in
all streams, has key ID `18FD790B48C202F8`, and has SHA-256
`6a16e1896992dcb6e55486b8b325bcbc45a07a4ca422c0f81c664a372a05a8cc`.
The production private key was neither read nor used by this verifier.

| Public artifact | SHA-256 | Verification |
| :--- | :--- | :--- |
| `Chill-Center-0.7.0-macos-arm64.dmg` | `7efd143d832b4882c1fb97d79fd9b234e04c863440151c0dabbc4ab22a2ded75` | signed outer artifact, signed sums and provenance |
| `Chill-Center-0.7.0-macos-arm64.app.tar.gz` | `da9ee463a8397aa6a07237127674582e6b811fd3f21d94e13d3dfb8e697cbd66` | Mac manifest and Tauri/Minisign signature |
| `Chill-Center-server-0.7.0-linux-aarch64.tar.gz` | `db0e3ca40a38d6f29ee8a617983081750dff51c2e57f3beee108a2aabac560a8` | signed outer artifact, signed sums and provenance |
| `Chill-Center-web-0.7.0-install.zip` | `c39dcd710bf3e067b8689aa25d9297dc09836580f8bbdcba67c28e3b4905edac` | signed outer artifact, signed sums and provenance |
| `Chill-Center-web-0.7.0.zip` | `0fa1aabf20d9f84b3e2722b0af7017dd09de9503da4524140a19f593072d7b57` | Web component manifest |
| `Chill-Center-themes-0.7.0-install.zip` | `faa91320afae4fbbd10ffb37214daf0a6d0acc56c4b350f9bd7524fa00d3d024` | signed outer artifact, signed sums and provenance |
| `Chill-Center-themes-0.7.0.zip` | `d0888380d1799f0c3b529bae0a735c5910c9b52d74bb473f64929622b9aa479c` | Themes component manifest |
| `PROVENANCE.json` | `a84cc76a35b1ddcb75591a7e8850e7044ea03074a7ae74aec7416b5ffc194926` | Minisign verified; source/versions/build/DMG bound |
| `SHA256SUMS` | `6b1c9a734adc1a3c1453337635babeaf516666468b83165c7d99505800ee0a1d` | Minisign verified; every listed file passed `shasum -c` |

Signed manifest hashes are Mac
`142856bfd97cecabb9d2c22263102f0c3c56abae9dc06aec7edc7b4fa603b578`,
server `447ea153849ef7239895f882ccce062102b1779734f1fb3f281fbafe62c68f28`,
web `7993c849b02c886df2d529369c266c46053aa8f5cedbed1770d2f3099703547d`,
and themes `36e2cada29ca9f8852351c15243d950377d8ab6c300f122c7056c085180fe912`.
Each declares schema 1, version 0.7.0, sequence 1, API 2026.10.14,
production key ID `18FD790B48C202F8`, and source `3a043334c5c3c676251ab34bf8bab2e4bfb8df4f`.
The base64 `tauri_signature` in the public Mac manifest decodes byte-for-byte to
the downloaded TAR `.minisig` and verifies that TAR.

## Artifact and boundary checks / Contrôles des paquets

| Check | Observed result | Result |
| :--- | :--- | :---: |
| Server complete installer | One bounded top-level directory; executable server/updater/scripts, three systemd units, signed server manifest, public key, FR/EN guides and licence present; contained bytes equal component-stream bytes | PASS |
| Web complete installer | Signed installer/server/web manifests present; inner component ZIP equals the component stream; build `a1c12ba4a582`, 98 declared payload files plus `build.json` | PASS |
| Themes complete installer | Signed installer/server/themes manifests present; inner ZIP equals the component stream; format/renderer 1 and all 15 declared assets exactly match the archive's file set, sizes and hashes | PASS |
| Archive safety | No absolute/traversal path, duplicate entry, `__MACOSX`, or AppleDouble entry in the inspected TAR/ZIP inventories; Mac TAR has exactly one top-level `.app` | PASS |
| DMG checksum/mount | `hdiutil verify` valid; independently mounted `/dev/disk5s1` at a verifier-owned path with `read-only,nodev,nosuid,noowners,nobrowse` | PASS |
| DMG root | Exactly `Chill Center.app`, `Applications -> /Applications`, and bilingual `Installation.txt` | PASS |
| Mac identity | Deep strict ad-hoc `codesign` valid; version 0.7.0, build 9, minimum macOS 27.0, 48 arm64 dylibs | PASS |
| DMG versus signed TAR | `diff -qr` empty and the complete regular-file/symlink hash inventories are identical; tree inventory SHA-256 `743d2fd4c45456a1c4614ba70917ee187ed65a4c2235ade3db85a31514374002` | PASS |
| Source/libmpv binding | Build-script output records `CHILL_SOURCE_COMMIT=3a043334c5c3c676251ab34bf8bab2e4bfb8df4f`; production build log compiles that worktree; early non-GUI `--verify-bundle` reports bundled libmpv client API 2.5 | PASS |
| Cleanup | Verifier-owned `/dev/disk5` ejected; verifier mountpoint removed. The user's existing `/Volumes/Chill Center` candidate was not touched | PASS |

Core exact commands, all exit 0 on the accepted fresh downloads:

```text
gh release download mac-v0.7.0 --repo ungeekrange-cmyk/chillcenter --dir /private/tmp/chillcenter-v07-public-verify.ZPodKg/mac
gh release download server-v0.7.0 --repo ungeekrange-cmyk/chillcenter --dir /private/tmp/chillcenter-v07-public-verify.ZPodKg/server
gh release download web-v0.7.0 --repo ungeekrange-cmyk/chillcenter --dir /private/tmp/chillcenter-v07-public-verify.ZPodKg/web
gh release download themes-v0.7.0 --repo ungeekrange-cmyk/chillcenter --dir /private/tmp/chillcenter-v07-public-verify.ZPodKg/themes
gh release download v0.7 --repo ungeekrange-cmyk/chillcenter --dir /private/tmp/chillcenter-v07-public-verify.ZPodKg/v0.7-complete
cd /private/tmp/chillcenter-v07-public-verify.ZPodKg/v0.7-complete && shasum -a 256 -c SHA256SUMS
minisign -Vm PROVENANCE.json -x PROVENANCE.json.minisig -p release.pub
minisign -Vm SHA256SUMS -x SHA256SUMS.minisig -p release.pub
codesign --verify --deep --strict --verbose=2 '/private/tmp/chillcenter-v07-public-verify.ZPodKg/extracted/mac/Chill Center.app'
'/private/tmp/chillcenter-v07-public-verify.ZPodKg/extracted/mac/Chill Center.app/Contents/MacOS/chillcenter-desktop' --verify-bundle
hdiutil verify /private/tmp/chillcenter-v07-public-verify.ZPodKg/v0.7-complete/Chill-Center-0.7.0-macos-arm64.dmg
diskutil image attach --readOnly --nobrowse --mountPoint /private/tmp/chillcenter-v07-public-verify.ZPodKg/dmg-mount /private/tmp/chillcenter-v07-public-verify.ZPodKg/v0.7-complete/Chill-Center-0.7.0-macos-arm64.dmg
diff -qr '/private/tmp/chillcenter-v07-public-verify.ZPodKg/extracted/mac/Chill Center.app' '/private/tmp/chillcenter-v07-public-verify.ZPodKg/dmg-mount/Chill Center.app'
diskutil eject /dev/disk5
```

| Evidence execution | Exit | Duration | Passed / failed / skipped | Retry status |
| :--- | ---: | ---: | :--- | :--- |
| Public metadata completion (`mac-v0.7.0`, `main`, `v0.6`) | 0 | 3.2 s | 3/0/0 queries | none |
| Fresh umbrella download, 22 assets | 0 | 28.9 s | 22/0/0 files completed | accepted retry after rejected partial transfer |
| Signed sums, manifests, component bytes and Tauri signature | 0 | 0.9 s | all declared files/0/0 | none |
| Server/web/themes/Mac archive layout and content assertions | 0 | 1.4 s | all selected boundaries/0/0 | two shell-wrapper mistakes corrected; no product assertion retried as a pass |
| Mac TAR metadata, source-build binding, libmpv CLI and DMG checksum | 0 | 2.0 s | all selected boundaries/0/0 | none |
| DMG attach, mounted-byte inspection and eject | 0 | 14.9 s | attach + assertions + eject/0/0 | none |

The first combined GitHub transfer encountered a transient API connection
refusal and left an incomplete umbrella download. Those partial bytes were
explicitly rejected as evidence. A second download into the fresh
`v0.7-complete` directory completed all 22 assets in 28.9 seconds and passed the
signed byte checks above; no retry is relabelled as a pass.

## Integration/test evidence audit

The meaningful assertions and exact-source reports were inspected, not merely
their headline counts. The runtime/packaging candidate `3e174b9` has independent
code, security and test approval. Evidence records 435 Rust passes, zero
failures and two pre-existing ignored NFR harnesses; 359 Node passes; and 19
Python packaging passes. The two ignored NFR checks remain ignored and are not
counted as passes.

The real native closeout validates busy admission before network access, signed
replacement and restart, a later native consent prompt, cancellation with only
the manifest GET, no 0.7.2 package GET, and unchanged application tree,
executable, marker and signature. The three earlier unexpected acceptances
remain adverse evidence rather than being reclassified. See
[`component-updates-tests-3e174b9.md`](../reviews/component-updates-tests-3e174b9.md)
and [`component-updates-tests-native-20261020.md`](../reviews/component-updates-tests-native-20261020.md).
Because `3e174b9..3a043334` is documentation/evidence only, the reviewed runtime
suite remains applicable; no broad suite was rerun during public-byte
verification.

**Integration-review verdict: `APPROVED`.** The exact combined public candidate,
source gates, signed compatibility metadata and packaging boundaries are
reproducible and current. No missing/skipped check is treated as a pass.

## Release gate, limitations and correction policy

| Release-verification area | Result |
| :--- | :--- |
| Version/configuration | PASS: public tags, carrier, versions, sequence, API and compatibility metadata match |
| Critical publication smoke | PASS: fresh public downloads, signature verification, archive inspection and DMG read-only mount |
| Logs/metrics/traces/alarms | N/A for this static GitHub distribution; GitHub uploaded state/digests and downloadability were checked. No deployed service observability is claimed |
| Migration/backward compatibility | PASS_SCOPED: v0.6 is preserved; API 2026.10.14 and database schema 3 bounds match. No migration was executed; the initial server installer still refuses an existing installation |
| Security/access sanity | PASS_SCOPED: intended public visibility, fixed public trust key, signed metadata/assets, safe archive paths and no private-key access |

No rollback trigger is met. If a defect is later confirmed, publish a new
corrective component version and monotonically higher release sequence (for
example `*-v0.7.1` and a new umbrella release), then make that new release the
current recommendation. Do **not** replace, retag, delete or overwrite the
stable v0.7 tags or assets, and do not bypass signed anti-rollback controls.

Not run and not claimed: Pi/SSH/NAS contact, deployed web/server checks, physical
Safari, GUI launch, media playback, Developer ID/notarization/Gatekeeper on
another Mac, Finder drag/drop, an actual application installation, power-loss,
disk-full, or live service logs/metrics/traces. The only application executable
invocation was the documented early `--verify-bundle` CLI path, which exits
before GUI, configuration or network startup.
