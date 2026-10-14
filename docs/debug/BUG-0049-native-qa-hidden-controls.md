# Debug Session: BUG-UI-NATIVE-HITTEST

- **Symptom**: fresh native QA at exact source `0a7e8f300d0e1f1b0723cf91d6aade15412027f5` exits 1 with `UI_QA_RESULT false native overlay hit-test: overlay DOM hit-test failed` (session `22914`).
- **Expected / Actual**: the QA intends to prove the visible pause control receives the DOM hit at its centre; actual timing can let the intentional 2.5 s inactivity state hide the controls first, so the centre resolves to `#native-video`.
- **Environment, Branch, Commit**: clean detached diagnostic clone of `/private/tmp/chillcenter-ui-rework.4oZp3D/repo`, branch source `codex/cinefolio-ui-rework`, commit `0a7e8f300d0e1f1b0723cf91d6aade15412027f5`; macOS debug Tauri host; fixture-only HTTPS server `127.0.0.1:18549`; fresh private client directories under `/private/tmp/chillcenter-player-final-qa.AnzUV0/`.
- **Base artifact**: `/private/tmp/chillcenter-root-target/debug/chillcenter-desktop`; diagnostic binary was built separately in `/private/tmp/chillcenter-player-diag-target`.
- **Debugger instance**: `/root/ux_native_diagnostic`.
- **Started / completed UTC**: `2026-10-14T16:42Z` / `2026-10-14T16:54:33Z`.
- **Reproduction**: run `CHILL_UI_QA_MODE=player` with `CHILL_UI_QA_URL=https://127.0.0.1:18549`, the fresh fixture root, and fresh client data; failure is at `ui/connected/player-qa.js` immediately after native loaded/nonblank/audio/subtitle and native z-order/pointer flags pass.
- **Regression test before correction**: `ui/connected/player-qa.js`, assertion using `document.elementFromPoint()` at the centre of `#player-pause`; session `22914` is RED.
- **Diagnostic status**: `ROOT_CAUSE_CONFIRMED`.

## Hypothesis ledger

| # | Clean baseline | Falsifiable hypothesis | Predicted observation | Minimal experiment | Evidence | Verdict | Restored |
|---|---|---|---|---|---|---|---|
| 1 | Clean exact `0a7e8f3`; no source/product edit | The QA oracle races the intentional 2.5 s control auto-hide; this is not evidence of lost native/webview z-order. | After idle >2.5 s: `controls-hidden`, overlay `pointer-events:none`, centre hit is `#native-video`; calling existing `showPlayerControls()` alone removes the class and makes the same point hit the button/descendant. A run reaching the check before 2.5 s can pass. | In a detached disposable copy only, trace computed state before/after `showPlayerControls()`, early-exit after this check; final causal run adds only 3 s idle to make the condition deterministic. | Natural failing trace: `idle_ms=2750`, `controls-hidden`, `pointer-events=none`, `top_id=native-video`, `contains=false`. Natural fast trace: `idle_ms=2221`, no hidden class, `pointer-events=auto`, top `svg`, `contains=true`. Causal run: before `idle_ms=3868`, hidden/none/native-video/false; after existing `showPlayerControls()`: `idle_ms=6`, class removed/auto/svg/true; exit 0 `diagnostic-dom-hit-auto-hide-causality`. | `CONFIRMED` | Yes. Probe removed; detached clone and frozen source now have identical `player-qa.js` SHA-256 `2b4f1169ddc7ff200ce9c672fbd7c16740a06dfd41fffdc4aa653e58ade13135`; diagnostic binary rebuilt from restored source. |

The first probe used a double `requestAnimationFrame` and stalled because an inactive QA window did not schedule it. That instrumentation limitation was removed; it is not a product/server hypothesis and did not increment the ledger.

## Confirmed cause and correction handoff

- **Cause**: `window.chillPlay()` starts the inactivity clock through `showPlayerControls()`. The QA then waits for native load, nonblank framebuffer, tracks, and native z-order. When those waits cross 2.5 s, production correctly applies `.controls-hidden`; CSS sets `#player-overlay` to `pointer-events:none`. The oracle nevertheless hit-tests the now-hidden pause button. This is timing-dependent test-oracle failure, not a demonstrated AppKit z-order defect.
- **Minimal authorized correction**: a distinct builder may change only the player QA oracle so it explicitly calls `showPlayerControls()` immediately before measuring/hit-testing `#player-pause` (and may assert the hidden class is absent). Preserve the later explicit `overlay inactivity` assertions unchanged. No `player.js`, CSS, AppKit, renderer, or timeout change is derived from this cause.
- **Out of scope**: speculative production UI/native changes, longer global waits, renderer changes, or weakening/removing the DOM hit-test.
- **Review impact**: the exact-SHA native QA/review evidence for `0a7e8f3` remains RED/STALE until a distinct builder supplies a causal delta and the corrected native QA passes on the exact new candidate.

## Separate unresolved timeout

The earlier session `52212` / PID `33137` remains `TIMEOUT`, not PASS. Its sample `/private/tmp/chillcenter-player-hang-33137.txt` places the main thread in AppKit framebuffer snapshot / `glReadPixels` Metal wait, but no causal condition/removal experiment was run. H1 above explains only the fresh `overlay DOM hit-test failed`. Per the confirmed-cause stop rule, this debugger did not start a second hypothesis. Diagnose the historical hang as a separate frozen incident/cycle with phase traces and a bounded reproduction; do not infer that the QA-oracle correction resolves it.
