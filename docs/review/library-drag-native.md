# Library drag continuation — November 11, 2026

Status: the focused native foreground plan passes all 29 steps and exact persistence checks for multi-selection reorder, empty-space move and toolbar-to-panel copy/name/insert. The viewport-boundary re-entry fix passes 58 actual destination-handler checks. A separate real DockPane proof also exposed and verified the fix for hidden Effect Editor native drop targets. The combined candidate also passes the real foreground reorder/copy and re-entry boundary plans. Standard debug `001c6d37…` and accepted release `fd5c974c…` are unchanged. Do not mark all library gesture parity rows complete.

## Source defects and changes

`LibraryPanel.press` used to replace the selection on every plain mouse-down. A drag begun over one of two selected entries therefore moved just that entry. Panels now retain an existing selection through drag initiation; a plain click without a drag collapses it on mouse-up. The release uses the original press modifiers, so releasing Command before the mouse does not convert a Command toggle into an exclusive click. Toolbar mouse-down selection behavior is unchanged.

The panel drag payload now comes from the first selected entry in list order and refuses an empty selection. This follows `Form_CurveLibrary.cs:346–357` and `Form_GradientLibrary.cs:343–354`; the original list supplies the first selected value and moves every selected item at `Form_CurveLibrary.cs:417–430`. Panel double-click editing now requires exactly one selected item, as each original library form does.

Cell and empty-space drop destinations formerly treated every same-kind library cell as their own list. Thus a toolbar curve dropped into a panel took its move branch and could move an unrelated selection instead of asking for a name. Panel destinations now require the source's shared `LibraryDropTracker` identity. Toolbar cells have no panel tracker, so their value-only payload follows the normal copy/name/insert path. The original toolbar supplies a value without the list's `ListViewItem` marker (`TimedSequenceEditorForm_Toolstrip.cs:647–670`); the form distinguishes that copy path in `Form_CurveLibrary.cs:361–415`. Strip destination behavior is preserved. Cross-window library drops are outside this focused acceptance.

## Prepared comparison

The controller-free generated profile contains Curve A/B/C/D with constant values 10/20/30/40 and an empty two-second sequence. The driver sends actual WindowServer CG mouse events; it does not call library mutation or drag/drop callbacks. Read-only live ModuleStore snapshots and the Rust inspector check persistence while the app remains alive. Before/after transport state must remain stopped.

The helper verifies the owned PID/executable and private bundle identifier, existing AX/event permissions, actual key/main window, exact window size, and WindowServer ownership of each pointer position. It stops on focus loss or an unexpected authorization surface, restores the pointer/prior app when safe, and has bounded process deadlines. All profiles, Cocoa preference domains, Rust settings and logs are private; tests remain muted.

The reviewed raster `target/library-drag-layout/ui/ready.png` is 2200×1560 pixels for a 1100×780-point **whole window**, including title bar. `UITestHarness.snapshot` captures `contentView.superview`, and `sequencer.resize` sets the whole window frame. The driver uses these observed cell/toolbar coordinates only after matching that same geometry. This is not an AX label-derived cell locator.

Prepared cases:

1. Command-select C and D; drag C onto A. Expect C/D/A/B.
2. Select C and D again; drag onto empty panel space. Expect A/B/C/D.
3. Drag toolbar A onto panel B. Expect a real `Curve name?` sheet, then A/Imported Curve/B/C/D with values 10/10/20/30/40.

The baseline must demonstrate a positive actual reorder before its failure is treated as evidence of the selection defect. A failed or unrouted pointer run is not a reproduced product failure. The candidate must pass all exact orders, values, link/current flags, live-file checkpoints and process cleanup.

Artifacts:

- `target/library-drag-preparation.json`: source hash and unchanged standard bundle proofs.
- `target/library-drag-source-review/LibraryPanels.before.swift`: frozen prior source.
- `target/private-app-library-drag-debug-baseline/private-app.json`: source debug `001c6d37…`, unique-domain clone `f0996032…`.
- `target/private-app-library-drag-candidate-final/private-app.json`: candidate source `8c8f350f…`, unique-domain clone `3e7c6289…`.
- `target/library-drag-candidate-final-build.log`: complete build succeeds; existing unrelated compiler warnings remain.
- `target/private-app-library-drag-candidate-final/bundle-audit.json`: strict signatures, 222 resources and four native binaries pass.
- `target/library-drag-debug-baseline/prepared.json` and `target/library-drag-candidate-final/prepared.json`: initial 29-step plans; the original helper has the startup race described below. Do not use those plans for another run.

The replacement baseline is prepared at `target/library-drag-debug-baseline-v2`. After the drag diagnostic is ready and a fresh desktop interval is coordinated, its command is:

```sh
python3 scripts/test_library_drag_foreground.py --app target/private-app-library-drag-debug-baseline/Vixen.app --helper target/foreground-library-drag-v2 --out target/library-drag-debug-baseline-v2 --run-prepared
```

Expect the baseline acceptance assertion to fail; inspect its actual records before continuing. Each run has a 75-second outer limit and normally needs around 40 seconds. Never launch both helpers together. Existing `target/library-drag-baseline` uses the older release baseline and older step plan.

## First foreground interval

The user supplied a fresh three-minute idle interval, shared sequentially with the Layer test. Library work began only after the Layer app/helper exited. No foreground activity was continued beyond that coordinated interval.

`target/library-drag-debug-baseline/result.json`: the helper sampled `NSRunningApplication` before direct process launch had registered with Launch Services, failed its executable-identity precondition and sent no input. The private app later completed its 29 harness steps and exited normally. This is a driver startup failure, not a reproduced product defect. The helper now retries that registration lookup for at most ten seconds before checking identity or activating anything; `target/foreground-library-drag-v2` contains the fix.

`target/library-drag-candidate-v2/result.json`: candidate `3e7c6289…` completed all 29 harness steps with stopped transport, unchanged source fixture, correct actual key/main ownership throughout, and successful app/helper exit plus restoration of prior app PID1335. `selected-two.png` visibly shows C and D highlighted, proving the initial clicks reached the intended cells. `moved-left.png` still shows that selection. However, no persisted order changed and no toolbar naming sheet appeared. The strict acceptance correctly failed with `No positive system drag routing was observed`. The helper's own `passed` field means its dispatch/ownership lifecycle completed; it does **not** override the outer acceptance failure.

The next check must inspect actual drag-session entry/drop delivery before interpreting the unchanged order. Merely repeating this same pointer plan would add no evidence. No production gesture acceptance or 10-point boundary claim follows from this run.

## Prepared route diagnostic

The library helper's CG construction, left-button and click-state fields, and event-posting tap match the successful Layer and Export Controllers helpers. A new `LibraryDragTrace` in `LibraryPanels.swift` records mouse-down/drag/up, absent payloads, session start/return, cell/empty-space entry and drop, source identity and the distance gate. It is inert unless both the UI harness and `VIXEN_LIBRARY_DRAG_TRACE=1` are present, and stops after 200 records. These are observations only; the same native drag path still performs every edit.

Private diagnostic app `12a562c7…0193fd` (source `8b3c0d7f…c0f9`) builds and passes its bundle audit. Its unique domain and provenance are in `target/private-app-library-drag-diagnostic/private-app.json`; its 29-step plan is `target/library-drag-diagnostic/prepared.json`. It was subsequently run during a fresh 45-second desktop interval:

```sh
python3 scripts/test_library_drag_foreground.py --app target/private-app-library-drag-diagnostic/Vixen.app --helper target/foreground-library-drag-v2 --out target/library-drag-diagnostic --run-prepared
```

The bounded `library-drag` lines in that run's `process.log` show mouse-down, mouse-up, mouse-dragged, payload creation, session start and session return on the Curve cells. The next drag-enter callbacks identify **Gradient** cells and their empty-space catcher at the same coordinates. Thus the run rules out missing mouse-drag delivery, absent payload and the proposed need for a blocking mouse tracking loop. It establishes hidden native destinations as an actual routing defect. The strict acceptance still fails, and the previous source-only candidate and failed runs remain unchanged.

## Hidden native destination fix

`SequencerContent.swift:251–258` retains dock panels in a ZStack using zero opacity and SwiftUI hit-testing suppression. Those modifiers left inactive panels' embedded NSViews registered as AppKit drag destinations. Both `LibraryCell` and `LibraryDropCatcher` now consume the existing `panelIsShowing` environment value, hide their native NSViews and unregister dragged types while inactive; switching back restores the three value types. Defensive callbacks also reject hidden views. All three library kinds use the same path, and strip views retain the environment's default visible state. Panel state remains alive.

Separately, LibraryCell now passes its saved mouse-down event to `beginDraggingSession`, matching the documented API and the existing CurveStrip/GradientStrip/ColorSwatch implementations. The original trace already showed session startup, so this API correction is not presented as the cause of the hidden-destination failure. Apple documents the initial event in [beginDraggingSession](https://developer.apple.com/documentation/appkit/nsview/begindraggingsession(with:event:source:)); [separate mouse-down/drag/up handlers](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/EventOverview/HandlingMouseEvents/HandlingMouseEvents.html) are supported without blocking the event loop.

Candidate `a5557526…ff12` (source `5ef0b651…5968`) in `target/private-app-library-drag-visibility` builds and passes the bundle audit. It includes the concurrently coordinated ColorPanelSession changes and the explicit unchanged-white creation callback in `newColor`; those have their own verification scope. `target/library-visibility-check/visibility-result.json` passes 24 native harness steps and 60 native destination-state assertions across Curve → Color → Gradient → Curve: the active tab's four cells and empty catcher are visible with three registered types, while the other ten views are hidden with zero registered types. This uses a read-only view-tree probe, not synthetic drag callbacks. The generated fixture remains unchanged and transport stays stopped.

The foreground plan `target/library-drag-visibility-foreground/prepared.json` ran during a fresh coordinated desktop interval. Its 29 harness steps passed, and live ModuleStore checkpoints prove C/D/A/B after dragging the two selected entries onto A, then A/B/C/D after moving that selection to empty space. The native trace identifies visible Curve destinations with `own=true`, passing distance checks and accepted operations. This establishes positive actual drag routing and verifies both reorder cases; all prior original fixture and foreground/exit safeguards passed.

Toolbar A then entered the Curve B destination with `own=false`, passed the distance gate and produced an accepted drop plus the actual `Curve name?` sheet (`toolbarNamingPromptObserved=true`). However, the helper set AXValue and immediately pressed OK; the final screenshot shows the original empty-name validation alert. That AX text operation did not commit the SwiftUI-bound field before submission, so no new item was created. The combined acceptance correctly fails at the toolbar insertion assertion. This is distinct from the previously unrouted drag problem.

Helper `target/foreground-library-drag-v3` now explicitly focuses the verified name field, posts actual CG Unicode key events, requires AX text readback and an editing settle before pressing the existing OK button. No production source changed for this helper correction. A fresh 29-step plan using the same candidate is prepared at `target/library-drag-visibility-foreground-v3`, normally around 30 seconds:

```sh
python3 scripts/test_library_drag_foreground.py --app target/private-app-library-drag-visibility/Vixen.app --helper target/foreground-library-drag-v3 --out target/library-drag-visibility-foreground-v3 --run-prepared
```

The exact drop threshold still needs its own boundary check. In particular, upstream resets its reference point on re-entering the whole list; the current shared tracker retains the first entry for a drag sequence so transitions between child cells do not reset it. Leaving the entire panel and re-entering is an additional unverified edge case. Hidden native controls in other dock panels are likewise outside the three-library visibility proof.

The exact 10-point drop boundary, color/gradient pointer coverage, external/editor payload routing, panel-to-toolbar/cross-window behavior, click-collapse and double-click gestures still need separate evidence. The unchanged Rust library move algorithms retain their prior tests; no broad Rust suite was repeated for this Swift-only change.


## Completed toolbar foreground acceptance

The v3 run at `target/library-drag-visibility-foreground-v3` repeated both successful reorder cases, but its trace has no toolbar mouse-down or drag session, so it never reached the new name-entry code. This is an unrouted helper attempt, not evidence that the copy handler regressed. The tiny toolbar cell was vulnerable to input arriving before pointer/window positioning had settled.

Helper v4 checks the actual cursor point after a mouse-move settle and requires WindowServer/AX window frames to agree before clicking or dragging. It then uses v3's focused CG Unicode name input with AX readback. Root ran `target/library-drag-visibility-foreground-v4` during a fresh coordinated desktop interval using unchanged candidate `a5557526…ff12`. All 29 native harness steps, foreground safeguards and outer acceptance passed:

- C/D move together onto A: C/D/A/B, verified in live ModuleStore.
- C/D move to empty space: A/B/C/D, verified in live ModuleStore.
- Toolbar A enters the copy route, opens the actual name sheet, accepts `Imported Curve`, and persists A/Imported Curve/B/C/D with values 10/10/20/30/40, empty links and current flags.

The original fixture remained unchanged, transport stayed stopped, app/helper exited, and the previous Chrome process was restored. This verifies these specific real WindowServer interactions. It does not add color/gradient, external/editor, cross-window, click-collapse or double-click coverage. Earlier failed artifacts remain intact.

## Exact handler boundary and re-entry evidence

`scripts/check_library_drop_handlers.swift` links the actual compiled `LibraryCellView`, `LibraryDropCatcher.CatcherView` and `LibraryDropTracker`, supplying only `NSDraggingInfo` metadata. It contains no replacement acceptance/move implementation. It creates no window or profile and posts no input. Its scope is actual destination-handler behavior, not WindowServer session routing.

`target/library-drop-handlers-before-v2/result.json` records 54/56 passing checks and exactly two re-entry failures. Both destinations reject ±10-point per-axis changes, including diagonal (10,10), and accept ±10.001. Toolbar source uses the copy callback; hidden cells and hidden ancestors reject; child transitions retain the list-entry reference; a new drag sequence resets it. Leaving the entire list and re-entering the same sequence does not reset: a drop only five points after re-entry is wrongly accepted against the original entry. Upstream `Form_CurveLibrary.cs:361–364` assigns the entry point each time its whole ListView receives DragEnter; the strict boundary is at line 395.

The approved minimal fix adds a weak reference from the shared tracker to the existing ScaleCatcher behind the ScrollView. That native view supplies the actual visible viewport bounds. Cell and empty-catcher `draggingExited` handlers reset the sequence only when the supplied window point is outside that viewport; moving between children inside the viewport keeps the original reference. The foreground-accepted `a555` candidate is preserved while a separate private candidate is built. Candidate handler and native viewport-wiring verification remain pending here.


## Verified viewport conversion and hidden Effect Editor controls

The final same-handler before/after comparison uses real unshown NSWindows, a viewport offset by (50,60), and explicit `draggingExited` delivery for cell→cell, cell→empty and empty→cell transitions. The old compiled objects in `target/library-drop-handlers-viewport-before` pass 56/58, with exactly the two whole-list re-entry failures. `target/library-drop-handlers-viewport-after` and `target/library-drop-handlers-combined` each pass **58/58**. Both whole-list cases now reject a five-point drop after re-entry; all inside-viewport child transitions preserve the original list reference, and all strict ±10/±10.001 cases retain their expected results. The before/after driver source is identical; a compile flag only wires the candidate's new viewport property, which the old tracker lacks. Inputs are pinned or hashed at link and the test uses no copied acceptance implementation.

The first real-window attempt caught an additional geometry detail: a non-clipping native view's `visibleRect` can extend beyond its own bounds, here to the whole window. Therefore the tracker uses `bounds.intersection(visibleRect)`, not `visibleRect` alone. The failed attempt is retained at `target/library-drop-handlers-window-after`. The actual DockPane probe independently observes the library's existing ScaleCatcher viewport at {{0,47},{400,576}}, while its unbounded visible rectangle covers {{0,0},{400,732}}. This supports using the viewport bounds and confirms that the production SwiftUI wiring supplies the expected native reference.

`scripts/check_library_dock_visibility.swift` mounts the **actual** production DockPane in an offscreen, non-activating private app. It creates one private controller-free element, selects a Pulse effect and then a Set Level effect, and switches Effect Editor→Curve Library→Effect Editor for each. It calls no drag handlers and sends no desktop input. `target/library-dock-visibility-before-v2/verification.json` demonstrates the defect in all three existing production control classes: Pulse's CurveStripNSView and GradientStripNSView, and Set Level's ColorSwatchNSView remain `hidden=false, registered=3` while Curve Library is the selected tab, at overlapping coordinates.

The existing `panelIsShowing` environment key and visibility method now live in VixenKit so all five ValueDragView subclasses use the same inactive-panel behavior. CurveStrip, GradientStrip and ColorSwatch consume the value alongside the two library destinations; ValueDragView's base handlers also reject hidden views/ancestors. The default remains visible outside retained dock tabs. No ColorPanelSession behavior was changed by this work. `target/library-dock-visibility-after/verification.json` passes all **9 native editor-view assertions**: active controls are visible/registered, inactive controls are hidden/unregistered, and switching back restores registration. Each retained library cell also has its native viewport reference. Both native probes preserve the foreground process and leave the original fixture unchanged.

The combined private candidate is `target/private-app-library-drag-shared-visibility/Vixen.app`, executable `6f2f5da590bb44ccbba08aa2a499a5697a5609bda26afb9279782a3b4c580e22`, source `ed5c303fd2a51b9e614fd41615022be541c871b40839da395b9f8f829fe9f66e`. It includes the separately verified palette session-ownership fixes in LibraryPanels' model extension. Build log: `target/library-drag-shared-visibility-build.log`; bundle audit: `target/private-app-library-drag-shared-visibility/bundle-audit.json` (passed). The previous foreground-accepted `a555` candidate and standard debug/release bundles remain intact.

Two fresh foreground plans are prepared with helper `target/foreground-library-drag-v5`:

- `target/library-drag-shared-visibility-foreground`: repeats the 29-step selection/reorder/toolbar copy proof against the combined candidate.
- `target/library-drag-shared-visibility-edges`: 26-step plan with actual CG drags that leave the list, re-enter empty space, then move +5, +10 and +11 points. Acceptance requires actual native initial-cell/re-entry/drop coordinates, rejected/rejected/accepted distance gates, unchanged A/B/C/D for the first two and B/C/D/A for the last, live persistence, stopped transport and exact values.

These two plans are not yet claimed as run here. The root coordinates them under the user's hour-long unattended desktop authorization; all native background processes above have exited. Exact negative-axis and diagonal thresholds have handler evidence only; color/gradient pointers, external/editor drop routing, cross-window behavior and click-collapse/double-click gestures remain outside this focused acceptance.


## Combined candidate foreground completion

Root ran the frozen `6f2f5da5…580e22` candidate with helper v6 after the host display changed from2x to1x backing scale. The first v5 attempt rejected the 1100×780 raster before input; root inspected that raster and verified the same1100×780-point layout. Helper v6 accepts either observed backing scale with matching aspect/point geometry and tighter ownership/time limits. That rejected attempt remains a helper calibration failure, not product evidence.

- `target/library-drag-shared-visibility-foreground-v6/result.json`: all29 steps and exact normal reorder/toolbar copy persistence acceptance pass again.
- `target/library-drag-shared-visibility-edges-v6/result.json`: all26 steps pass. Each actual native session first enters its cell around(876,580), leaves the viewport, then re-enters empty space at(875.75,430). Drops at x880.75 and885.75 (+5 and+10) are rejected and preserve A/B/C/D. The drop at x886.75 (+11) is accepted and persists B/C/D/A with exact values.

Both runs preserve the original fixture and stopped transport, exit their app/helper processes, and restore the preceding app. This closes the focused positive-x native threshold and whole-list re-entry checks. The remaining negative-axis/diagonal boundaries have direct-handler evidence. The independently discovered panel→toolbar ownership issue is being handled separately; the frozen6f candidate does not include that later fix or the later palette captured-payload fix.

## Actual panel-to-toolbar release route — November 12, 2026 UTC

`target/library-panel-to-toolbar-release/result.json` passes all **29 native steps** with three actual OS drags on private release clone `d400d8a09c1cd33a24cd1a21729b1cd630f79a696d7401a11d0c79f2b74b4d1c` (source release `689481b8a4f15a9543cd84e97dd1e374be87ef64499ec9cc4acad3ac008db8f0`). The first two drags preserve multi-selection reordering and the move to empty space. The last drag now goes from panel Curve A onto toolbar Curve B: the native naming sheet appears, actual focused Unicode keys enter `Imported Curve`, and the saved order is A/B/C/D/Imported with levels 10/20/30/40/10. This proves a copy appended to the toolbar library rather than an internal reorder. The helper verified foreground, key/main ownership and settled pointer coordinates, then restored the previous app. The private app exited normally through its test harness; this is not normal Quit-menu evidence.

The runner/helper add an explicit `--panel-to-toolbar` mode; their original directions remain unchanged. This build includes symmetric panel/toolbar tracker routing but precedes the separately identified cross-sequencer-model toolbar ownership fix. It does not establish cross-window physical dragging.
