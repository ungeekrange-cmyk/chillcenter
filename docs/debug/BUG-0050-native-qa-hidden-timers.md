# Debug Session: BUG-NATIVE-QA-HIDDEN-TIMERS

- **Symptom**: exact source `db9455cf202a59be7ec17079ab3ad75cbcb07d44` passes the corrected overlay hit-test, enters `overlay inactivity`, then emits no later phase. Pair-A exec `99088` is killed by the external 120 s watchdog (`TERM`, exit 1); every phase trace through `elapsed_ms=2074` reports `document_hidden=true`.
- **Expected / Actual**: the overlay must hide after 2.5 s, pointer movement must reveal it, and a focused control must remain visible. Actual: the timer-sensitive step never wakes while the WKWebView page is hidden.
- **Environment, Branch, Commit**: clean source `/private/tmp/chillcenter-ui-rework.4oZp3D/repo`, branch `codex/cinefolio-ui-rework`, commit `db9455cf202a59be7ec17079ab3ad75cbcb07d44`; macOS debug Tauri; one synthetic MP4 plus English SRT; original fresh server `127.0.0.1:18550` and private data under `/private/tmp/chillcenter-native-pair.cwOqF1/`.
- **Base artifact**: root debug binaries from the exact source; all probes were built separately in `/private/tmp/chillcenter-player-diag-target` from detached `/private/tmp/chillcenter-player-diag-src`.
- **Debugger instance**: `/root/ux_native_diagnostic`.
- **Completed UTC**: `2026-10-14T17:16:03Z`.
- **Reproduction**: run `CHILL_UI_QA_MODE=player` with the minimal loopback fixture while the native QA document reports hidden. The corrected hit-test passes; the first `setTimeout`-dependent inactivity wait never completes before the external watchdog.
- **Regression evidence before correction**: pair-A exec `99088`, `TRACE ... overlay inactivity ... document_hidden=true`, then no phase for 120 s and watchdog termination. This is `TIMEOUT`, never PASS.
- **Diagnostic status**: `ROOT_CAUSE_CONFIRMED` on hypothesis 5; no sixth hypothesis run.

## Hypothesis ledger

| # | Falsifiable hypothesis and predicted observation | Minimal experiment | Evidence | Verdict |
|---|---|---|---|---|
| 1 | Tauri `WebviewWindow.show()` plus `set_focus()` is sufficient to make the hidden QA document visible; `document.hidden` should become false and the inactivity timer should complete. | Disposable host calls only its own window show/focus, tracing before/after; one player-A run. | Host returned `shown=true focused=true`, but `document.hidden` stayed true from boot through `overlay inactivity`; no timer completion before the owned process was terminated at about 60 s. | `INCONCLUSIVE` — attempted condition removal did not change the condition. |
| 2 | The executable is an inactive `NSApplication`; direct activation should change inactive/hidden to active/visible. | Disposable host observes `NSApplication.isActive`, invokes activation on its own application, and traces page visibility. | Before intervention the fresh process already reported `document_hidden=false` and `NSApplication.isActive=true`; activation was a no-op. The run later failed at native load because prior playback state had contaminated the reused server/profile, so it was excluded from timer causality. | `REFUTED` — the predicted inactive precondition was absent. |
| 3 | A clean active, visible, unoccluded application provides a non-hanging control baseline. | New DB/server `18552`; observe AppKit application/window state without changing it; stop immediately after the timer-sensitive stage. | `isActive=true`, application/window occlusion included `Visible`, window visible/key/not-miniaturized, `document_hidden=false`; overlay inactivity and focus timers completed in `5231 ms`. | `INCONCLUSIVE` for root cause, but establishes the visible control baseline. |
| 4 | `NSApplication.deactivate()` will create the hidden state; native reactivation after 5 s will resume the pending timer. | New DB/server `18553`; own-app deactivate, native 5 s wakeup/reactivation. | Deactivation did not change host or page state: application stayed active/visible and `document_hidden=false`; timers completed normally in `5237 ms`. | `INCONCLUSIVE` — the experiment did not create the suspected condition. |
| 5 | Explicitly hiding the own application reproduces the suspended-timer symptom; native unhide+activation alone removes it and releases the already-pending JS timer. | New DB/server `18554`; at boot call only `NSApplication.hide` on the QA app; independent native wakeup after 5 s calls `unhide` plus activation; trace Page Visibility events and timer completion. | Start: `document_hidden=false`. Hide caused `visibility-event document_hidden=true` at `31 ms`. All phases through `overlay inactivity` at `1712 ms` remained hidden. Native unhide/activate ran at 5 s; visibility event reported false at `5129 ms`; the pending inactivity/focus sequence completed at `5727 ms`; diagnostic exited 0. | `CONFIRMED`. |

## Confirmed cause and scope

The headed QA was allowed to start while its WKWebView document was hidden. The overlay-inactivity oracle depends on WebKit timers in both production `showPlayerControls()` and the QA polling/waits. In the hidden state observed in exec `99088`, those timers are suspended, so the external 120 s watchdog terminates a live native renderer before the JavaScript step can advance. H5 reproduced the hidden state and showed that changing only own-application visibility/activation releases the already-pending timer.

The historical session `52212` sample `/private/tmp/chillcenter-player-hang-33137.txt` showing AppKit framebuffer snapshot / Metal `glReadPixels` wait is therefore an observation of the render loop while the QA page is suspended; this cycle did **not** establish a GPU/Metal deadlock as the cause. No GPU, renderer, timeout, or product-timer change is justified.

## Causal correction handoff

- **Authorized minimal correction**: a distinct builder may change only the debug/native QA launch path so the host shows/unhides and activates its own Chill Center application/window, then begins timer-sensitive player QA only after the page is visibly active (`document.hidden === false`, using the visibility event/readiness gate). Retain the overlay inactivity, pointer and focused-control assertions.
- **Explicitly out of scope**: skipping assertions, globally disabling WebKit timer suspension, extending the watchdog, modifying production `player.js`/CSS, changing AppKit rendering, or treating pair B as passed.
- **Required red-to-green**: reproduce player A from a new DB/client/lease on the exact correction candidate; show initial visible readiness and progression beyond `overlay inactivity`; then complete the full A run before starting pair B. A diagnostic early exit is not full native QA proof.
- **Review impact**: native QA evidence for `db9455c` remains failed/timeout. Any correction creates a new exact candidate requiring current focused test/review evidence.

## Cleanup

All temporary probes were removed. The detached source is clean at `db9455c`; `ui/connected/player-qa.js` SHA-256 matches the frozen source: `aea5f26f4bb8b3f87f568146fe911c5e4833c60a640cb00145f3cbae7b7ff477`. The diagnostic binary was rebuilt from the restored baseline. Probe servers on ports `18552`–`18554` were stopped; no pair-B run was started.
