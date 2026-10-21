# Second cleanup / Deuxième nettoyage — 2026-10-21

## Autorisation et protection / Authority and preservation

L'utilisateur a demandé de nettoyer les dossiers Chill Center inutilisés de
`/private/tmp`, explicitement le `target/debug` du worktree courant. Cette
autorisation est postérieure au premier audit, qui protégeait encore ce cache.
Sources, branches, historique Git commun, autres worktrees, sauvegardes,
livraisons, `target/release` et `artifacts/` sont conservés. Aucun fichier du Pi,
NAS ou client web installé n'a été supprimé par ce nettoyage local.

The user explicitly authorized unused Chill Center temporary-build cleanup,
including the current worktree's `target/debug`. Sources, Git history/branches,
other checkouts, backups and delivered releases remain intact. No remote files
were removed. Future debug builds will need to regenerate their outputs.

## Contrôles et actions / Checks and actions

- Exact worktree: `/private/tmp/chillcenter-fixes-web-parity`, source initially
  clean at `efac44d`; shared Git in
  `/private/tmp/chillcenter-native-ui.KZRHlE/repo/.git` preserved.
- No Cargo, rustc or Chill Center application/server process observed. `lsof
  -nP +D` returned no open files for the exact current debug tree and each of
  the eight external caches immediately before deletion. SSH master and an
  unrelated VM process were retained.
- Storage-check scanner on the exact roots: no errors or skipped mounts;
  hashing limited to 1 MiB per root. This is not a full duplicate-content audit.
- Native current-cache cleanup: `cargo clean --profile dev --target-dir
  /private/tmp/chillcenter-fixes-web-parity/target --frozen`, exit 0.
  Cargo reported 138871 removed file paths / 36.5 GiB logical output; this is
  **not** the independently measured free-space gain.
- Eight external roots contained only Cargo cache metadata/debug outputs, no
  Git/source/release tree. Each CACHEDIR.TAG signature was checked; exact paths
  below were removed, not a wildcard or shared-directory purge.

| Removed external cache | Allocated KiB before |
| --- | ---: |
| `/private/tmp/chillcenter-resource-audit-target` | 113196 |
| `/private/tmp/chillcenter-security-ac52533-target` | 2404744 |
| `/private/tmp/chillcenter-security-review-r2-target` | 113196 |
| `/private/tmp/chillcenter-security-review-target` | 621060 |
| `/private/tmp/chillcenter-v07-consent-proof-target` | 2452472 |
| `/private/tmp/chillcenter-v07-hostile-target` | 714108 |
| `/private/tmp/chillcenter-v07-mac-qa-build-target-2` | 1614676 |
| `/private/tmp/chillcenter-v071-mdns-target` | 101824 |

## Mesures / Measurements

| Measure | Before KiB | Immediately after KiB |
| --- | ---: | ---: |
| Current complete worktree (`du -sk`) | 37354248 | 4489520 |
| Current target | 36788916 | 3924164 |
| Current target/debug | 32864752 | absent |
| Current target/release | 2950468 | 2950468 |
| Current artifacts | 498696 | 498696 |
| Free filesystem space (`df -k`) | 84400000 | 124400000 |

Gain libre immédiatement observé : **40960000000 octets = 40,96 Go**
(38,15 Gio). Le worktree passe de 38,25 Go à 4,60 Go. Le gain ne s'ajoute pas
aux 15,47 Go du premier nettoyage dans les mesures ci-dessus : c'est une
opération distincte. Les liens physiques, clones/snapshots APFS et écritures
concurrentes expliquent qu'allocation, tailles Cargo et différence `df` ne
coïncident pas nécessairement. Une lecture ultérieure de `df` a encore varié ;
aucun gain ultérieur n'est attribué au nettoyage.

Immediately observed free-space increase: **40.96 GB**, measured on the same
filesystem. This is separate from the first cleanup's 15.47 GB, not a summed
claim. APFS and unrelated concurrent filesystem activity limit attribution.
Other temporary directories, the stopped test VM, all worktrees and final
signed/downloaded packages remain available; their usefulness was not disproved.
