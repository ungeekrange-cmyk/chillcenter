# G-INTEGRATION-PI-001

Historical verdict: APPROVED; current status: STALE_FOR_NEW_INSTALLER

Superseded by r3 following BUG-0040 and independent SEC-PI-002 correction. No service/account/system files were installed with this candidate.

2026-10-14 UTC. Independent integration reviewer `/root/pi_integration_gate` and operational security reviewer `/root/pi_install_security` both returned APPROVED, read-only. Coordinator records their verdicts, does not self-approve.

- Source: `126da0af167ee62588d92d90297f2dfc51962676`.
- Build checkout: `de5ac7b8250a9e4835656e2a3823f142f7db09c1`; functional source diff empty.
- Archive SHA256: `67645c79a1544cb9c00ff2408c6f89df12699b9f299fe749e6944aaa54739c2a`.
- Binary SHA256: `9a17ffe55e3d4128b9d483d170f5d2ede71382cef42d8637917c8f31e63b36a6`.
- Installer SHA256: `ef3bbfcb8e1762af3262b9f0c9136572512f2bb7b2d574f2bc7897217ed70a82`.
- Target: `admin@chillcenter.local`, Debian13.7 ARM64 Pi3B+, glibc2.41. NAS `/srv/volumes/video`, CIFS `//nas.local/Video`, RO, supplementary group984 explicitly approved by user.

Evidence: reviewers reproduced hashes/source equivalence, archive18 members unique root/no links/traversal/special files/secrets, package config/unit/lock exact, ELF64 PIE AArch64 glibc2.28. Integration reviewer ran4 packaging tests PASS; both bash syntax PASS; independent SSH preflight confirmed account/unit/files absent,8443free, NASmountRO, group984 and runtime tools present.

Mandatory conditions: deployer manually inspects this exact canonical verdict and archive/installer hashes (mitigates SEC-PI-001 LOW lexical gate check); invokes script with bash and --go only; transferred binary's readelf/ldd/--help must pass before privileged installation. No blind rerun after partial installation. Critical failure: stop/disable only new service and preserve state; NAS and other services unchanged. Post-installation TLS/API/NAS/Range/profiles/resources require a distinct verifier. This gate does not assert those runtime checks have passed.
