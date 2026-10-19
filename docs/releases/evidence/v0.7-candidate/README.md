# Candidate v0.7 execution evidence

These are local test/build transcripts, not production or Pi execution.
Runtime source reviewed: `3913267`; the broad parent suite and exact delta
are distinguished in the [independent test review](../../../reviews/component-updates-tests-3913267.md).

- `rust-workspace.txt`: 433 PASS, two pre-existing ignored NFR harnesses.
- `desktop-updates.txt`: exact native delta, eight PASS.
- `node-tests.txt`: 359 PASS; `settings-browser.txt`: synthetic bridge only.
- `clippy.txt`: workspace/all-targets strict lint PASS.
- `linux-systemd.txt`: disposable Ubuntu VM, actual root helper/server/units,
  signed test packages, Busy/retry, TERM/KILL, rollback and exact expired retry.
- `linux-root-ledger.txt`: two actual Linux root tests PASS.
- `linux-final-units.txt`: healthy old server and final 150-second rescue budget.
- `macos-package.txt`: standalone candidate build, 48 relocated arm64 libraries,
  ad-hoc strict signature and bundled libmpv loading.

Three transcript copies have only redundant terminal blank lines trimmed for
Git whitespace checks. Original files and hashes cited by reviewers remain at
the local paths named in their reports. No production private key, service
certificate or household server address is included. Synthetic public keys and
test-only key filenames are not production credentials.

Native AppKit consent/install/restart is **not yet passed**. The isolated QA
feature remains on a separate test branch and is absent from release builds.
