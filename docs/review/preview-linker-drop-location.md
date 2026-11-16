# Preview linker actual-pointer drop fix

The linker now assigns a dragged element to the row under the pointer and rejects blank space. Both validation and acceptance independently compute `tableView.row(at:)`, require a current valid index and require an element payload. This matches the original Windows linker’s `PointToClient` / `GetItemAt` behavior. Only the two assignment handlers in `app/Sources/VixenPreview/DesignerDialogs.swift` changed; insertion/reordering tables are outside this fix.

The old code trusted AppKit’s proposed row. A stale proposal could target the wrong row, and a negative accepted row could reach the model closure’s `UInt32(row)` conversion. The focused probe records that negative callback; it does not execute or claim an observed production crash.

## Evidence

- `target/preview-linker-drop-fix-02/result.json`: ten baseline failures become ten candidate passes using actual unshown `NSTableView` hit testing and scrolling. Cases cover stale/negative/out-of-range proposals, blank space, invalid payload, scrolling, pointer movement after validation and row-count shrink. No global input or visible window was used.
- `target/preview-linker-drop-fix-02/independent-source-result-review.json`: independent source, upstream contract and result review passes nine checks. The unsuccessful restricted attempt01 is retained and excluded.
- `target/preview-linker-integration/build-validation.json`: complete release and debug builds and both bundle audits pass; all 808 frozen production inputs match. The only change from the cursor candidate is `DesignerDialogs.swift`.
- New release SHA: `29edeac86a51b72dad29be04694a93d31d12a4113d10ab649b3cee7e146ab9ac`.
- New debug SHA: `27f38b28057b1fc640084be579f37cd8b34beef9e8ec03bdbb1fe2edc020d515`.

The build also contains the independently reviewed Curve/Gradient cursor repair. Rust and Cargo inputs are unchanged from the accepted 3,533-test candidate; that broad suite was not repeated.

## Native acceptance still pending

`target/preview-linker-physical-02/prepared.json` prepares two real drags, save/reopen, a distinct double-click assignment, and Cancel on private inputs. All 50 initial links point to FixtureTwo; a valid FixtureOne drop must change all 50, a blank FixtureTwo drop must leave them unchanged, and cancelling a later distinct FixtureTwo edit must retain saved FixtureOne. Exact unrelated XML/model preservation is required. Preparation is not execution.

This candidate is not promoted. The standard accepted builds remain the export-name/float-roundtrip release `a1f84fc3…` and debug `6f01171c…`. Physical linker acceptance and actual cursor appearance remain required before promoting these changes.
