# Native colour-session continuation audit

This review found correctness defects behind the remaining colour/gradient UI checks. It does not treat existing backend or synthetic-input checks as proof of WindowServer drag/drop, physical system colour/font selection, or normal Open/Save-panel behavior.

## Source contracts and fixes

| Trigger | Original behavior | Retained change |
| --- | --- | --- |
| New Color, then close the white picker without changing it | `ColorPanelSession.finish()` suppressed the only callback, so no colour was added. Changing and returning to white had the same result. | Creation opts into `notifyUnchanged`; ordinary edits retain no-op suppression. `LibraryPanels.newColor()` and collection Add use it. |
| Choose a pending colour on gradient stop A, then select B | The delayed callback used `session.setSelectedColor`, which now targeted B. | Finish before native bar selection and before location, Delete, preset load/save, unlink, library edit, OK and Cancel. |
| Change a colour, return to the original, then close | Cleanup was inside a callback suppressed for a no-op. Gradient, effect-row and Marks live overlays remained installed. | A separate `onEnd` always clears temporary presentation, independently of the changed-value callback. |
| Begin adding a colour to collection A, then choose B | Add's callback appended to the current collection B. | Finish before collection selection, new/delete, import/export and final acceptance/cancellation. |
| Add a pending colour, then click Add again | The second call captured the old `lastColor` before beginning committed the first pick. | Finish the previous session before reading `editor.values().lastColor`. |

The intended contracts are effect-editor F103 (commit when the target changes, one edit per session) and sequencer-shell F343/F365/F368. The upstream `Form_ColorLibrary.AddNewColor` and `ColorCollectionLibrary_Form.AddColorToCollection` accept unchanged colours on OK. Their modal picker prevents retargeting while pending. These changes preserve the documented nonmodal Mac design while keeping its target ownership and no-op Undo behavior.

`ColorPanelSession.finish()` clears its active callbacks and pending-commit registration before invoking clients, captures the chosen value before callbacks can reenter, and leaves repeated `finish()` calls inert. Cleanup runs before the changed-value callback; it does not create an edit. All current live-preview clients were inspected and migrated to `onEnd`.

## Verification

The standalone callback driver links the actual cached debug VixenKit object, generated bridge and matching Rust static library. It calls real `GradientBarNSView.mouseDown`, `mouseUp` and both Delete-key paths with local AppKit events. Each gradient starts with green at 0 and blue at 1. No implementation algorithm is copied into the assertions.

- `target/color-session-native-baseline-02/result.json`: 13 checks, ten expected original failures, including wrong-stop retargeting, both Delete ordering paths and stale reverted preview.
- `target/color-session-native-fixed-02/result.json`: all 13 pass; initial stops, selected swatch, callback counts and commit-time stop arrays are recorded.
- `target/color-sheet-controls-baseline-05/result.json`: all 64 harness steps execute successfully; 12 assertions record ten expected original failures. They include unchanged/reverted/repeated Add, exact collection A/B retargeting, pending colour missing from a saved preset, and actual effect-row/Marks stale previews. The original mark revision and the ordinary effect commit count remain unchanged when reverted.
- `target/color-sheet-controls-fixed-05/result.json`: all 64 harness steps and all 12 assertions pass. Collection A gains the pending red while B remains blue; repeated Add produces green/red/red; effect and Marks reverted overlays clear without a commit or revision change; the saved gradient contains red at 0 and its original blue at 1. Its 22 exact linked inputs are frozen under `target/color-sheet-controls-fixed-05/input-snapshot`; its manifest retains both original compiler paths/hashes and the verified snapshot mapping.

The sheet driver mounts production `GradientEditorDialog` and `SequencerSheetView`. Add, Load Preset, Save Preset and Unlink buttons use the existing native harness. SwiftUI's menu Picker could not open offscreen; the collection retarget regression therefore obtains the actual `Binding<String>` from the compiled view body and calls that production setter directly. It does not copy the `finish`/`select` implementation and does not establish native menu acceptance. Load Preset checks commit timing at selector entry and cancellation; preset selection and physical library-tile gestures remain separate. Earlier attempts retain their failures and are not acceptance evidence.

Each driver has a separate ad-hoc-signed bundle identifier, a copied controller-free profile where applicable, and muted output. It never posts system input or activates an application. Exact linked-object/FFI/source hashes and compiler arguments are in each `prepared.json`; completed reports recheck those inputs. Baselines are historical cached objects, not reconstructed source. The accepted release bundle is not modified by these tests.

The combined scoped result is **25 passing assertions**, including **64 successful sheet-harness steps**, with before/after evidence. The callback binary predates the final repeated-Add fix, which changes only SequencerSheets and is covered by the later sheet suite. The final Swift incremental build completed in 2.82 seconds; its existing `StripOverflowLayout` Sendable warning is retained in `target/color-session-native-review/incremental-build-elevated.log`. The initial restricted compile could not write the compiler's existing user cache; the permitted local-cache compile succeeded without downloads.

Exact provenance and current-source scope are consolidated in `target/native-color-session-review.json`. Shared `target/build-main/swift` contains the incremental build but may be advanced by the independent library re-entry work. The frozen callback and sheet inputs remain sufficient to inspect these exact results. Library re-entry edits are not attributed to these colour tests. Standard debug/release bundle integration is a parent-task follow-up.

## Concrete next checks

The initial audit identified a separate palette target-ownership concern: `LibraryPanels.editColor(at:)` captured an array index while palette operations could move/remove it. The focused follow-up below now reproduces and fixes that concern. Its evidence is separate from the gradient checks.

Physical system colour/font controls, library tile double-click input and actual minimum-OS execution remain separate checks. The normal Open/Save work and library WindowServer drag acceptance are owned by the parallel review tasks; this report makes no claim about their outcomes.

## Follow-up: palette target ownership

The focused palette probe now calls the actual `SequencerModel` methods, shared colour session and real Rust palette persistence using an isolated, muted profile. Its first baseline records 13 failing assertions among 15 cases. For example, deleting the edited blue from green/blue/red initially leaves green/red, but closing the pending yellow edit changes the surviving red to yellow. Exported VFC data retains blue instead of pending yellow. Pending New cyan followed by Import white reverses the requested append order, and opening Edit again displays the stale blue start value.

The minimal retained fix finishes before the `.color` branches construct snapshots for removal, either reorder path, insertion, import and export. Edit finishes before reading its initial colour. Removal rereads selected values by their unchanged position IDs after finishing, then still uses upstream's first-value removal; flushing alone would leave the caller's old `LibraryEntry.argb` values stale. `setPalette` deliberately remains a pure writer: flushing there would occur after its caller had already constructed a stale snapshot and could overwrite the commit or recurse through it.

The parent review found one further captured snapshot: `LibraryCellView.mouseDown` selects an item, then `mouseDragged` obtains a payload through a closure holding an older `LibraryEntry`. Finishing only in the receiving mutation occurs too late to correct that payload. The colour branch of `dragValue(for:link:)` now finishes first and rereads the current colour by the entry's position ID. Curve and gradient payload paths are unchanged.

Only the top `SequencerModel` extension in `LibraryPanels.swift` belongs to this fix. The parallel lower-view drag tracker and native visibility changes are separate. Before/after prefix snapshots and the exact diff are in `target/palette-sessions-source-audit`.

- `target/palette-sessions-baseline-02/result.json`: clean exit, 15 assertions, 13 expected failures.
- `target/palette-sessions-fixed-01/result.json`: all 15 pass before the additional payload audit.
- `target/palette-sessions-payload-baseline-01/result.json`: 16 assertions; only the captured payload fails, returning old blue instead of pending yellow.
- `target/palette-sessions-fixed-02/result.json`: **all 16 pass**, including correct palette persistence, duplicate removal, import/export contents, no-op controls and committed drag payload.

All four accepted processes exit successfully, remain inactive at their recorded checkpoints, use no system input and preserve the source fixture. Their executable and linked inputs are frozen and verified. The first diagnostic attempt wrote its complete results, then timed out in an unrelated document-close path; it is not an accepted run. The corrected driver exits its private callback-only process directly and makes no normal-quit claim. Exact source/provenance review: `target/native-palette-session-review.json`. The final incremental Swift build passes in 2.34 seconds; standard bundle integration remains separate.
