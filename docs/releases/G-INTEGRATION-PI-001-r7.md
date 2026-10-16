# G-INTEGRATION-PI-001-r7

Canonical verdict: APPROVED

2026-10-14UTC. Independent integration code/test `/root/pi_integration_gate` and security `/root/pi_install_security` APPROVED r7, read-only; coordinator records verdicts, not self-approval. Supersedes earlier script gates. Product126/binary unchanged.

- Source: `126da0af167ee62588d92d90297f2dfc51962676`.
- ArchiveSHA256: `ed485f99e14ab8953b7d007959ab9fc35a11a006d4d989260a703fe9be6ebee2`.
- BinarySHA256: `9a17ffe55e3d4128b9d483d170f5d2ede71382cef42d8637917c8f31e63b36a6`.
- InstallerSHA256: `d3cd903f776efdac7b732fb28e3ee01a9b3911650b7799f9fe6ded79d01d97de`.
- SSHvalidatorSHA256: `d508cfdf964c5d68497f426413226cbca4d70abdca807dbb16cc058bad0521df`.
- SSHtestSHA256: `b16afcbc953a993b1af236188f4e745e04e09a5244a7abaa495bbca2e3579439`.
- NAStestSHA256: `c842677a8e2b08960f5431d93cce8b81f79627dd5ede93029a6df3911a69e35d`.
- CLItestSHA256: `1aadb46f791138590c1313ee69b0545e2d51e79633152fe4c8c18d6f73aa7072`.

Evidence: archive safe18members/unique root/payload unchanged, r6/r7 only manifest-installer-hash differs. Bashsyntax/regressionsPASS. BUG0041 confirms exact reviewedCLI usage5lines exit1 onrealPi, ELF64/ldd and binaryexecutionPASS; r7 explicitly checks that exact status+output rather than mask failures. Exit0/signal/error/wrongusage rejected. All SSH/NAS protections of APPROVEDr6 preserved. Realmaster user-created `/private/tmp/chillcenter-ssh.UAjtlb/control`, parent0700/socket0600, strictknownhosts and pristine privileged installation paths confirmed. Only private r6stage exists, preserved for diagnosis.

Before invocation deployer must inspect canonical verdict and all operator hashes; use exact approved socket/adminchillcenter.local, direct bash --go and new immutable r7stage, no blind reuse/overwrite of r6. ELF/ldd/runtimecontract before useradd/system paths. Supplementarygroup984 userapproved, service nonroot/LAN8443, NAS/other services unchanged. Critical partial failure stop/preserve/diagnose, not rerun. Distinct postdeployment verifier required. No live service, scan/decode/perf or all133AC PASS claimed here.
