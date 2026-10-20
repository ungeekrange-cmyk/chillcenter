# BUG0116 — macOS QA update archive contains AppleDouble entries

- Priority: P2, QA fixture packaging; not a delivered production artifact.
- Status: FIXED / VERIFIED_NATIVE, 20 October 2026.
- QA correction: `54dd9689a99657bba9a189366a8a29830cd99509` (separate QA branch).

The genuine Tauri SDK download verified its test signature but could not unpack
`._Chill Center QA.app`. macOS tar had inserted AppleDouble metadata. Its normal
listing hid those entries; Python `tarfile` exposed 474 members, including 237
AppleDouble entries. The production archive already disabled copyfile metadata;
this was a defect in the isolated QA fixture generator, not a reason to relax
the updater's archive validation.

The QA generator now sets `COPYFILE_DISABLE=1`, disables xattrs/ACLs/flags and
checks raw members for a single app root and no `._` components. The corrected
237-member test archive was installed and the real QA executable restarted.
The rejected package and failed log remain in the private local QA evidence.
Only ephemeral test keys were used. No normal application or Pi was changed.

Résumé : métadonnées macOS indésirables dans l'archive QA, corrigées dans le
générateur et vérifiées par une vraie installation/redémarrage. Le rejet initial
est conservé ; aucun assouplissement de sécurité du produit.
