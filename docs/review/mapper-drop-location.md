# Element Mapper drop location

Dropping below the last source row could change an existing mapping. AppKit retained a proposed row/item from an earlier drag position, and both Mapper destinations trusted it. The original Windows handlers reject a null target; pinned Gong 4.0.0 hit testing confirms that the tested blank areas have no target.

The table and outline now convert `NSDraggingInfo.draggingLocation` into their own coordinates and hit-test the actual row in both validation and acceptance. Blank rows are rejected. The outline also checks its existing source-only policy during acceptance. Model assignment, serialization, group semantics and payload formats are unchanged.

Release `10010538021fec9f8a12f1be9185e37e8a002e09703518c193b6363da8654da1` and debug `66ad484a1d7792013c52a11baef339640cbf11c173f440abeef37c7b2ad7027d` are promoted to the standard local outputs. Only `ElementMapperSheet.swift` changed among 808 production inputs from release 97498. Both bundles pass audits; all copied bytes, permissions and signatures were independently checked. Previous 97498/8c129 bundles remain in `target/accepted-bundles-before-mapper-drop`.

The private full-app run passes 40 steps in 63 seconds: seven actual OS drags, valid table/outline leaf and group assignments, both blank-area rejections, physical Delete, save/reopen and native Cancel after a distinct unsaved change. The saved map's hash is identical before the unsaved change, after Cancel and after reopening. Both processes exit normally; input is released and the previous desktop and owner preferences are preserved. Setup, tree switching, save and readback use existing test hooks; this run does not claim physical file-panel navigation or normal Quit.

Separate unshown AppKit handler probes use the exact four production methods with minimal model/drag adapters. The candidate passes 11 checks; the baseline reproduces six wrong-target/blank-area failures. Real clip-view scrolling reaches 460 points and actual row 22. These probes establish coordinate handling, not additional OS drag sessions.

Retained evidence:

- `target/mapper-drag-foreground-04/native-semantic-audit.json`: original physical failures.
- `target/mapper-drop-location-fix/contract-and-scope.md`: original Windows/Gong contract and source provenance.
- `target/mapper-drag-foreground-05/result.json` and `independent-acceptance.json`: actual fixed-app run and independent acceptance.
- `target/mapper-drop-location-fix/handler-probe-run.json`: raw baseline/candidate handler results.
- `target/mapper-drop-integration/promotion.json` and `postcopy-verification.json`: standard outputs and preserved previous bundles.

The prior 97498 portable 164-step evidence remains bound to that executable. Its media/playback components are unchanged; the broad suite was not repeated for this Mapper-only fix. The refreshed 100105 portable archive passes transfer hashes, executable modes/signatures and bundle audits. Actual macOS 26 and the other external verification gates remain open.
