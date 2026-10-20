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

The final Rust transcript copy likewise omits its redundant terminal blank
line; the original `/private/tmp/chillcenter-v07-ac52533-rust.log` remains
unchanged and its original hash is recorded by the independent reviewer.

Final 20 October evidence supplements the historical transcripts above:

- `rust-final-ac52533.txt`: 435 PASS, zero failed, two existing NFR ignores.
- `node-final-ac52533.txt`: 359 PASS, zero failed/skipped.
- `native-consent-20261020.txt`: busy-before/busy-during PASS, genuine signed
  installation/restart PASS, three adverse acceptance attempts preserved and
  final production-helper cancellation PASS.
- `native-loopback-20261020.txt`: only one 0.7.1 payload and four 0.7.2 metadata
  requests; no 0.7.2 payload download.

Native closure is independently approved in
[the final test review](../../../reviews/component-updates-tests-3e174b9.md).
The isolated QA feature remains on a separate branch and is absent from release
builds; private test keys are never included in this evidence or packages.
