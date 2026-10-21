# BUG0124 — Signed expiry fixture equals the installed version

Date2026-10-21, author/root, status **FIXED_TEST / regression PASS**.

Full Rust suite failed the signed expiry-replay test with `downgrade or reinstall
refused`. Its hardcoded Mac0.7.1 candidate now equaled the real build. Production
correctly refused reinstall before this fixture could exercise expiry recovery.

Confirmed cause: fixture version drift. Derive candidate from package version
plus one patch, including tag/asset URL. Signature/expiry/substitution guards
unchanged. Full workspace rerun PASS; initial sandbox socket denials are tooling
observations, not player regressions. Aucun changement du lecteur ni du Pi.
