# Export filenames and dragged numeric values — November 12, 2026

Two defects were reproduced during native verification on the accepted
`945941c6…` build. Their fixes are in isolated release `a1f84fc3…` and debug
`6f01171c…` builds. Both are now promoted locally; previous standard builds are preserved.
Promotion evidence: `target/save-name-float-integration/promotion.json`.

## Export default filename

The normal Export sheet offered `Panel Source Café.fseq.fseq` even though the
Rust request and `NSSavePanel.nameFieldStringValue` getter each contained only
one extension. A shown native A/B probe reproduced both the visible Save As
value and the proposed URL: the original property order doubled the suffix;
assigning the name after the content-type and extension options produced one
suffix. No file was saved by this probe.

`ExportSequenceSheet.swift` now assigns the offered name after those options.
The original Windows contract also offers the sequence stem plus one format
extension and then uses the selected path. Independent review verified the
minimal source change and the A/B evidence.

Evidence: `target/save-panel-shown-probe-04/acceptance.json` and
`independent-acceptance-raster-return.json`. Its raw wrapper failure remains:
the consumer incorrectly expected `NSModalResponseStop` (-1000) for Cancel;
the SDK defines `NSModalResponseCancel` as 0. All other consumer assertions were
reused unchanged. Probe03's earlier preflight failure was a CGFloat/Double
representation mismatch, not an established Accessibility denial; probe04
recorded Accessibility trust as true.

Actual integrated saving now passes in Export06: all three offered names have one suffix, Cancel preserves the sentinel, and explicit Unicode Save plus untouched-default Save produce two structurally exact FSEQ files. The first output remains byte-identical through the second Save. Normal Close/Quit and cleanup pass.

The raw wrapper failed only because FSEQ block pairs were tuples in memory and lists after JSON reload. `target/sequence-export-physical-06/acceptance.json` preserves that failure and reconciles only this representation under 14 strict checks and six tamper controls. The parser now returns lists; unchanged full acceptance consumes the retained artifacts and five further tamper controls reject. No native rerun or product change was needed (`target/sequence-export-checkpoint-fix/result.json`).

## Dragged Gradient and Curve precision

Gradient03 performed a real drag and naming, but its saved Gradient differed
in one XYZ Y coordinate: `10.637748203969881` became
`10.63774820396988`, one f64 ULP. Its strict saved-state checkpoint correctly
stopped before the second Cancel route. The original sequence and owner
preferences remained unchanged; helper cleanup completed.

The actual `DragPayload` JSON decoder reproduced this loss with the default
serde_json parser. Enabling the existing `float_roundtrip` feature preserves
the exact bits. `Cargo.toml` now enables that feature at the unchanged locked
serde_json version; `Cargo.lock` is unchanged. The regression tests cover
Gradient XYZ, stop/alpha values and metadata, plus Curve coordinate bits and
metadata. Original in-process object copies preserve these values, so no
numeric tolerance was introduced.

Evidence: `target/drag-float-roundtrip-fix/result.json` and
`independent-source-review.json`; baseline cases fail 2/2, isolated fixed cases
pass 4/4, and the actual workspace integration target passes 2/2. Gradient03's
raw failure and precise value diff remain in its directory. Gradient06 now passes physical copy and the second naming Cancel: two real drags, 24 native observations, all 24 numeric fields preserved as exact source text and IEEE f64 bits, unchanged source/library entries and normal Quit. Independent acceptance passes 17 checks in `target/editor-family-gradient-physical-06/independent-acceptance.json`. Gradient05's early registration guard failure is retained; it posted no input and does not establish user interference.

## Integration and test-helper recovery

Both bundles pass 222-resource/four-executable audits. The full offline Rust
workspace run exits 0: 3,533 passed, zero failed, 10 ignored across 105 reported
suites. The ignored cases remain separately scoped hardware, performance,
background-app and fixture-generator checks. All 808 production inputs match
the captured build manifest; only Cargo.toml and ExportSequenceSheet.swift
changed from the accepted build. Owner settings/preferences remain unchanged.
See `target/save-name-float-integration/build-and-workspace-validation.json`.

Color editor-to-library copying separately passed on the accepted build:
one real drag, exact inserted color/order, saved state and normal cleanup.
`target/editor-family-color-physical-03/acceptance.json` records its scope.
Five Prop graph model tests also pass across nine variants, with independent
original-source review, exact persistence and Undo/Redo checks; physical
cross-window rejection is still separate (`target/prop-graph-contract-review`).

Export05 stopped before export because WindowServer had not yet acknowledged
a posted Shift key-up; the helper mistook that delayed state for foreign input
and withheld the remaining Command release. Root released only its recorded
Command key and independently verified clear input and no remaining private
processes (`target/export05-input-recovery/acceptance.json`). The raw failed
run remains unchanged. Export and Package helpers now track pending key-up acknowledgements and
block new input until release is observed. Their control tests are not a claim
that the new native workflows have already passed.

The resumed ten-minute foreground grant ended at 2026-11-13 03:32:54 UTC. New native launches require fresh desktop availability. Windows SSH is also not yet verified: antivirus blocked
the user's setup attempt, and both confirmed LAN and Tailscale port22 checks
timed out afterward. No antivirus configuration was changed. Saved setup logs
are needed to establish which Windows setup stages completed.
