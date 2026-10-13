# Native render probe diagnostic ledger

- Symptom: `NSOpenGLPixelFormat` returned `None` in the managed command sandbox.
- Expected: a main-thread AppKit OpenGL context renders generated video through libmpv.
- Environment: macOS Apple Silicon, branch `codex/native-player`, base `4c066a56f739e257bca39cd90fe4c9af019bd29a`, mpv 0.41.0.
- Reproduction: `cargo run --manifest-path native/macos-render-probe/Cargo.toml --offline`.
- Status: `ROOT_CAUSE_CONFIRMED`.

| # | Hypothesis | Minimal experiment | Result | Verdict |
| --- | --- | --- | --- | --- |
| 1 | The host rejects only accelerated OpenGL 3.2 attributes. | Remove acceleration/profile attributes; retain the same sandbox and constructor. | Identical pixel-format rejection. | `REFUTED` |
| 2 | A non-bundled process must initialize shared `NSApplication` first. | Initialize it on the verified main thread; retain sandbox and attributes. | Identical pixel-format rejection. | `REFUTED` |
| 3 | The managed sandbox denies the WindowServer/GPU service required for pixel-format selection. | Run the exact compiled binary outside the sandbox, changing no code or attributes. | Both 320×180 and 640×360 FBOs rendered nonblank readbacks; pause/seek succeeded. | `CONFIRMED` |

No production correction is required. The probe must be executed in the logged-in local GUI session. Temporary attribute and `NSApplication` experiments were removed. This does not prove the future Tauri `NSView` surface integration.
