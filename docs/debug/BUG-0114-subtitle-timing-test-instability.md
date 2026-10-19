# BUG0114 — Existing subtitle timing test is unstable during update verification

- Priority: P2 test reliability; status: OPEN / CAUSE_UNCONFIRMED.
- Environment: macOS Apple Silicon, Rust 1.99.0, installed libmpv/FFmpeg.
- No player production source change in the component-update branch.

Observed test: `tests::subtitle_changes_keep_the_current_playing_or_paused_state`
in `crates/chill-player/src/lib.rs`. It asserts more than 0.1 s advancement after
an exact 250 ms sleep on a 10 fps synthetic fixture. The pause-property assertion
passes; the subsequent video-time assertion sometimes fails.

Evidence during this turn:

- Initial workspace run passed.
- A later workspace run failed at the advancement assertion.
- A targeted rerun while builds were active failed at the same assertion.
- Direct execution of the existing test binary after compilation passed in 1.43 s.

Hypothesis ledger: H1, scheduling/decode-start delay under concurrent build load
can exceed this test's short observation budget. Prediction: quiet execution
passes. The quiet run supports the hypothesis but does not prove causality;
there was no controlled paired load experiment. Verdict: INCONCLUSIVE.

No test threshold, skip or player behavior was changed to obtain a pass. This
record does not establish a user-visible subtitle regression and does not claim
the test is fixed. Preserve failed attempts in the final verification evidence.
