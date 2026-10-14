# G-INTEGRATION-PI-001-r7-recovery

Canonical verdict: APPROVED

2026-10-14UTC. Distinct integration code/test `/root/pi_integration_gate` and security `/root/pi_install_security` APPROVED narrow BUG0042 recovery, no remote writes. Diagnostic `/root`, author `/root/pi_deployer`. Only this partial state authorized, no blanket resume.

- Source `126da0af167ee62588d92d90297f2dfc51962676`.
- Retained R7archiveSHA256 `ed485f99e14ab8953b7d007959ab9fc35a11a006d4d989260a703fe9be6ebee2`.
- BinarySHA256 `9a17ffe55e3d4128b9d483d170f5d2ede71382cef42d8637917c8f31e63b36a6`.
- RecoverySHA256 `dfa5a38ed808f5738fedf3a664bfcb873fb57978bc9c9954fc52644468538030`.
- RecoverytestSHA256 `00b8ad7568da9d9a37fb2502261e5b736602cf7e099fac6eb4fc11ec0ccce57a`.
- CWDtestSHA256 `57a4fb9c1778e5454df1b952f0283df1c83068b3b969aa4ad020ba088dd63db7`.
- SSHvalidatorSHA256 `d508cfdf964c5d68497f426413226cbca4d70abdca807dbb16cc058bad0521df`.
- Future-only first-installerSHA256 `6c4ae2fbc499d4d2d1b23ba8c73cdbd962a0bcaff8c095c7f8e6fb1851cf3669`, NOT used to replay partial install; R7manifest keeps original d3cd903f installer hash truthfully.

Exact partial state proven by root RO after r7: chillcenter997:983, groups983+984/nologin, service/config/binary/state absent; NAScheck fromhomeadminexit1 thenfrom/exit0. Recovery preserves account, does not repeat useradd/changegroups/deletions/remount. Before sudo writes, it compares all5usedstaged regular unique members byte-for-byte with known immutable R7archive and confirms source/binary. Privileged guard repeats exactpasswd/groups/absentpaths/unit/port and one exact CIFSRO; cd/thenNASenumeration underUIDservice. Copies only reviewed files, cmpconfig/unit, verifies systemd, enablesonlynewservice. Syntax/static/cwd checksPASS; unit/config/binary unchanged, source reviews still current.

Conditions: manual inspect canonical verdict and every relevant hash immediately before one invocation; exact shared socket /private/tmp/chillcenter-ssh.UAjtlb/control/admin192.168.1.42. Any state mismatch or partial failure stops, no blind retry; preserve account/state/diagnostics and NAS/other services. Postdeployment tests via distinct verifier before VERIFIED. This gate does not claim live service/TLS/NAS/API/performance PASS.
