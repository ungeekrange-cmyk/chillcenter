# G-INTEGRATION-PI-001-r3

Canonical verdict: APPROVED

2026-10-14UTC. Independent integration code/test reviewer `/root/pi_integration_gate` and operational security reviewer `/root/pi_install_security` returned APPROVED r3, read-only. Coordinator records their verdicts, not self-approval. Supersedes r1 false-rejection BUG-0040 and r2 BLOCKED SEC-PI-002; neither installed anything on the Pi.

- Functional source: `126da0af167ee62588d92d90297f2dfc51962676`.
- Build checkout: `de5ac7b8250a9e4835656e2a3823f142f7db09c1`; functional build sources identical.
- Archive SHA256: `6a2278b1c3a0b69200cfd2d476a60145fabf4c0f182cdf3e9e7d536aae7bd6db`.
- Binary SHA256: `9a17ffe55e3d4128b9d483d170f5d2ede71382cef42d8637917c8f31e63b36a6`.
- Installer SHA256: `3bda4aa4d392924ae93cdd20b5726a97aa669206bc06f2cff3b31efed0e13629`.
- Regression SHA256: `c842677a8e2b08960f5431d93cce8b81f79627dd5ede93029a6df3911a69e35d`.
- Authorized target: admin@chillcenter.local Debian13.7aarch64 Pi3B+, group984 marvin-admin approved by user; CIFS //nas.local/Video RO at /srv/volumes/video, LANbind8443.

Review evidence: reproduced hashes; unique safe18-member archive; r2/r3 only manifest-script-hash differs; binary/unit/config/lock unchanged; shell syntax and regressions PASS. Oldregex fails roinitial; newguard has2 positive/7 negative fixtures and passes realPiROmetadata. Both preflight and privileged guards demand one exact CIFS record with ro token and no rw. SEC-PI-002 resolved; SEC-PI-001 LOW lexical gate mitigated by deployer inspecting canonical verdict + exact hashes.

Conditions: invoke bash --go only; remote transferred ELF/ldd/--help must pass before privileged writes. No blind rerun after partial installation; on critical failure stop/disable only chillcenter and preserve all local data and NAS. Distinct post-deployment verifier required for TLS/API/NAS/Range/profiles/metrics. This gate does not assert runtime service or endurance PASS.
