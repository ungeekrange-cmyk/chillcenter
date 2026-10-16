# G-INTEGRATION-PI-001-r6

Canonical verdict: APPROVED

2026-10-14UTC. Independent code/test integration reviewer `/root/pi_integration_gate` and security `/root/pi_install_security` both APPROVED r6. Author `/root/pi_deployer`, coordinator records independent verdicts. Supersedes all older installer gates; product source/binary unchanged.

- Source `126da0af167ee62588d92d90297f2dfc51962676`, buildcheckoutde5ac7b (functional sources identical at build).
- Archive SHA256 `b5c20d971dde72b94944817c698201a72a7db276cddbbc5298783cd7f27bd1ea`.
- Binary SHA256 `9a17ffe55e3d4128b9d483d170f5d2ede71382cef42d8637917c8f31e63b36a6`.
- Installer SHA256 `f6acfeac4ae945bc5a1578c1c055dcd392e43bef79a9ae4275671d0d7b631d23`.
- SSH validator SHA256 `d508cfdf964c5d68497f426413226cbca4d70abdca807dbb16cc058bad0521df`.
- SSH regression SHA256 `b16afcbc953a993b1af236188f4e745e04e09a5244a7abaa495bbca2e3579439`.
- NAS regression SHA256 `c842677a8e2b08960f5431d93cce8b81f79627dd5ede93029a6df3911a69e35d`.

Evidence: hashes/archive payloads reproduced; only manifest operator-tool hashes changed across later revisions, package safe18 members, binary/unit/config/lock unchanged. Bash/Python syntax and NAS/socket regressions PASS; helper rejects `%`/`$` expansion, non-private/symlink/mismatched-owner parent/socket. Real socket `/private/tmp/chillcenter-ssh.UAjtlb/control` validated user-owned0700 parent/0600socket. Root actual `ssh -O check` masterPID31195 and RO shared-session inventory PASS: service/account/config/state/r3staging absent. User explicitly established this session in Terminal.

Mandatory deployment conditions: inspect exact canonical verdict AND archive/installer/helper/test hashes immediately before invocation, mitigates LOW lexical gate and helper external dependency; use this exact shared socket/adminchillcenter.local only, StrictHostKeyChecking, no direct agent retry. Direct bash --go, transferred ELF/readelf/ldd/--help before useradd/system paths; approved group984, NAS RO, bindLAN8443, no other services/mounts changed. If partial failure, stop/preserve/diagnose, not blind rerun. Post-deployment verification by distinct `/root/pi_release_verifier` mandatory; no hardware performance or all133AC PASS declared by this gate.
