# Preview setup-window bounds — November 12, 2026

Preview F5 now has **24 +10 native assertions** covering the real restore, window-delegate and persistence paths with `UITestHarness` inactive. Both private processes exited0, in1.91seconds and1.04seconds. No production code or standard bundle changed.

The fixture compiles exact, unmodified copies of all10 `VixenPreview` source files alongside a narrow internal entry point, with optimization and the target's original Swift language settings. It links the matching optimized production VixenKit/Bridge/FFI objects from release `3484a1ad1f8f7456f908b8a6754d0f90bfaca705044ac36bf845cab3b96ec8dc`. This grants internal access without exposing private methods or altering the tested code. It is source-equivalent module evidence, not execution of the shipped Preview object or full SwiftUI window root.

The main run, `target/preview-bounds-native-02/result.json`, opens a freshly generated controller-free, shape-empty, background-free profile through `VixenCore.open(muted:true)` and `AppModel.adopt`. Actual Metal-backed designers open successfully in three separate Core/model lifetimes. With `VIXEN_UI_TEST` absent, `model.open` executes its normal `restoreBounds` path:

- Stored outer frame `[45,610,620,400]` restores exactly.
- Programmatic `NSWindow.setFrame` automatically sends a real AppKit move notification through the installed `DesignerWindowDelegate` to its original-delegate recorder. No explicit move fallback was needed. The real `model.save()` writes SetupLeft70, SetupTop90, SetupWidth620, SetupHeight400 while the window remains open.
- A directly supplied end-live-resize notification reaches the actual installed delegate and its original recorder. The next real save writes70,60,660,430. Independent XML parsing verifies all four fields.
- A new Core/model lifetime reads that saved profile and restores actual outer frame `[70,590,660,430]`.
- Stored offscreen coordinates `[-30000,-30000]` fail the recoverability check and retain the initial fallback frame `[90,550,580,420]`.

A separate **right-corner-only** run, `target/preview-bounds-native-right-corner-01/result.json`, executes only10 new assertions. Requested and actual outer frame both equal `[-1000,330,1100,680]`. Its top-left `[-1000,1010]` lies outside every working area, while `(right−1,top)=[99,1010]` lies inside the observed working area `[0,94,1920,956]`. AppKit preserves the requested frame; it does not clamp the restore. This larger raw host frame does not establish full SwiftUI layout behavior. The earlier24 assertions were not rerun or overwritten.

All windows stay unshown, transparent and non-key/non-main. A fail-closed ordering guard allows only native orderOut; it observes three closes in the main run and one in the supplementary run. The cursor-only guard is never invoked. No activation, global input, physical movement, Finder launch, remote device, controller, audio output or web listener is used. A web session is registered only for status inspection; `running=false` in every Core lifetime, with no Administration/service startup. Private Cocoa identity and `VIXEN_SETTINGS_DIR` isolate settings; the owner's preferences and foreground app remain unchanged. Both subprocesses are reaped.

The42-check report `target/preview-bounds-native-final-review.json` pins both results, all retained inputs, the exact10-source module copies, independent XML checks and unchanged standard bundles/frozen foreground batchv4. The earlier33-check report records the main run before the fixture gained its optional corner-only mode. Preparation01 failed only because the restricted Swift macro-plugin sandbox denied Observation expansion; no app launched. Native compilation and the two actual runs then succeeded.

Physical dragging/resizing, visible placement, a full SwiftUI-root layout, Finder/clean-user permissions, a multiscreen sweep and F71 live-preview persistence are separate scopes. No claim for those follows from this F5 verification.
