# Independent native library visibility review

The shared dock visibility and library re-entry changes pass source review. Current shared source bytes match the five recorded snapshots; the lower LibraryPanels region matches the exact combined handler fixture (SHA-256 `c2759ade44e60a915eee933bc386f8f73e66656ca44afe7c9323122553c2cdfb`). The later palette changes are outside that region. Shared snapshots were explicitly captured after the tests, so this comparison does not claim an independent earlier-time snapshot.

The single shared environment key defaults to visible. DockPane applies the same tab selection to SwiftUI visibility and each native value destination; hidden controls unregister and drop handlers reject hidden ancestors. Standalone and floating editors remain visible by default. The viewport conversion and bounds/visibleRect intersection preserve entry position across child destinations and reset it on whole-panel exit.

Existing evidence supports these changes: `target/library-drop-handlers-combined/result.json` contains 58 passing checks against actual compiled native destination handlers; `target/library-dock-visibility-after/verification.json` contains 9 passing observations of actual DockPane controls across tab switches. The former supplies dragging metadata and the latter uses an offscreen view tree; neither is a WindowServer drag-session acceptance test. No new application was launched during this review.

One adjacent P2 issue remains in `LibraryPanels.swift:479`: `isOwnList` only compares tracker identity when the destination has a tracker. A library-panel cell dragged onto a toolbar cell has a non-nil source tracker and nil destination tracker, so it takes the internal reorder callback. F170 and the upstream toolstrip handlers instead require the external copy/name branch (or rejection for a linked curve/gradient). The current 58 checks cover toolbar-to-panel but not the reverse route. A focused actual-handler check should cover color/unlinked copies, linked rejection, and preservation of same-toolbar reorder before a minimal correction. This issue predates the reviewed visibility/re-entry change; no production edit was made by this audit.

Machine-readable evidence, exact source hashes, and references are in `target/native-library-visibility-independent-review.json`.

## Focused routing follow-up

The adjacent panel-to-toolbar issue is now fixed. A compiled baseline reproduced exactly five failures: normal color/curve/gradient values invoked reorder, and linked curve/gradient values bypassed rejection. The actual production acceptance callback was used on an unopened model. Symmetric tracker identity now separates panel and toolbar sources. The fixed fixture passes all 68 checks: the original 58, all five corrected panel-to-toolbar routes, and five preserved same-toolbar reorder cases (including linked values). Both processes exited successfully without changing foreground focus. The normal naming dialog is not exercised by this callback test.

Both fixtures retain signed executable, source snapshot, runner snapshot and 65 hashed linked-input copies. `target/native-library-drop-routing-review.json` records exact provenance; `target/library-drop-routing-change.diff` contains the one-condition production fix and comment. The top palette extension is byte-identical before and after. The debug cache rebuilt successfully in 2.28 seconds; standard app bundles were not modified by this follow-up.

## Cross-sequencer ownership follow-up

Shared library storage does not make a drag between different sequencer windows an internal reorder. Each native sequencer has a separate model; upstream's `_itemMove` is an instance field only enabled in the source form. A destination form therefore uses copy/name or rejects linked values. The prior nil/nil toolbar tracker comparison incorrectly bypassed that distinction.

The focused baseline reproduced five cross-model failures while preserving the prior 68 checks. A weak `libraryModel`, assigned by the actual LibraryCell representable, now participates in source/destination identity. The fixed run passes all 78 checks. Ten new cases mount actual production representables in separate never-visible native windows: five cross-model copy/reject cases, and five same-model floating-toolbar reorder cases. These directly cover `updateNSView` owner wiring. No profile, core, visible window, global input, or naming dialog was used. The empty toolbar catcher already has no internal-move callback; panel catchers have unique trackers, so neither needed a change.

Exact snapshots, signatures and 65 frozen inputs per fixture are recorded in `target/native-library-owner-review.json`. The debug cache rebuilt in 2.33 seconds. Parent task owns complete bundle builds and physical drag acceptance.

## Physical panel-to-toolbar result reviewed independently

`target/library-panel-to-toolbar-release/result.json` passes independent audit: all 29 harness steps, actual OS drag from the panel into toolbar cell 1, native curve-name entry, live append checkpoint and independently parsed persisted points are consistent. The copied Curve A remains a detached value with flat level 10, appended after the original four curves. Input ownership checks, stopped transport, exit, restoration, source-fixture hashes and private app signature also pass.

This physical run used private executable `d400d8a09c1cd33a24cd1a21729b1cd630f79a696d7401a11d0c79f2b74b4d1c`, derived from release `689481b8a4f15a9543cd84e97dd1e374be87ef64499ec9cc4acad3ac008db8f0`, before the later cross-model owner fix. It therefore proves the reverse route for that release, not final integrated or cross-window acceptance. The exact helper, results, inputs and current reviewed source snapshots are recorded in `target/native-library-physical-routing-independent-review.json`. Helper source snapshots were captured after execution and are labelled accordingly.
