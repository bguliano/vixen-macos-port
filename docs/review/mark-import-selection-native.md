# Imported mark selection: native F35 verification

The audited release `0d7edc262607ad317134ab6f2fc03e8730d72151d7d6b0b74342d0b6331f992d` passes **94 native steps** on a generated controller-free profile. No production code or app hooks changed.

- `target/mark-import-native/accepted-subset/result.json`:64 steps. The actual Mark Collections Import toolbar action opens Import Type Selection; Vixen3 is selected and the existing queued file-result hook supplies the generated `.v3m`. The first row starts selected and all three boxes checked. CG-backed posted Space unchecks only the selected row. Native table selection and further Space presses uncheck all three; OK is visibly disabled, and Return leaves the sheet open with no collections or dirty state. Selecting Middle then Zebra enables OK, and Return accepts. Saved XML contains Zebra then Middle in source order, with the original collection IDs and exact mark start times, durations and text. Alpha is absent. Playback remains stopped while Space operates in the modal sheet.
- `target/mark-import-native/cancelled/result.json`:30 steps in a separate process reopening the saved profile. A posted Space visibly changes the first checkbox, then Escape cancels. The complete rows and dirty state remain identical; the sequence, source `.v3m` and release executable remain byte-identical by SHA256. Playback remains stopped.

The source file contains three independent collections in deliberately nonalphabetic order: Zebra Source, Alpha Source, Middle Source, with marks at1/2/3seconds,450ms duration and distinct labels. `crates/vixen-session/examples/mark_import_fixture.rs` uses the real `.v3m` writer/reader and validates the persisted fields before launch. Individual mark IDs are not serialized by this format, so fixture validation correctly compares collection IDs and mark values, not regenerated mark IDs.

`scripts/test_mark_import_selection_ui.py` asserts native action results, initial row highlight, each checkbox's visible checkmark pixels, enabled/disabled OK text, unchanged state after disabled Return, transport state, saved XML and Cancel hashes. Checkmark crops exclude row names; screenshots are normalized from the observed1×/2× backing scales. The final all-unchecked/Return snapshot was visually inspected. The script uses local Python and Pillow; no downloads or external services are involved.

The native `selectRow` action calls the real `NSTableView.selectRowIndexes`; it does not assign the imported-selection model. Physical pointer selection, system file-panel navigation and general foreground focus are not claimed. The app stays muted/offscreen and uses only generated data.

Earlier attempts are retained and excluded from the94 count. `probe` matched the word Import in the sequence title through OCR and never reached the dialog. `acceptance` used a toolbar coordinate from the other observed dock width and missed entry. `acceptance-final` completed the first subset/save path but failed its second Import entry after the offscreen parent window changed size; the final checks therefore use independent processes. `probe-toolbar` is a separate30-step geometry/Space/Cancel probe. None of these setup failures is classified as a production selection defect.

To reproduce with fresh private output folders:

```sh
source ./env.sh
CARGO_TARGET_DIR=target/vixen-session cargo build --offline -j2 -p vixen-session --example mark_import_fixture
target/vixen-session/debug/examples/mark_import_fixture target/f35-private/profile
python3 scripts/test_mark_import_selection_ui.py --profile target/f35-private/profile --out target/f35-private/accept --mode accept
python3 scripts/test_mark_import_selection_ui.py --profile target/f35-private/profile --out target/f35-private/cancel --mode cancel
```
