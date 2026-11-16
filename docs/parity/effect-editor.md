# Effect editor parity

> **For the specialized editors worker (started 2026-11-09):** the Edit Polygons button calls `EffectEditorHooks.editPolygons` with a `PolygonEditRequest` (F37). Set that hook from `app/Sources/VixenApp/VixenApp.swift` under `#if canImport(VixenTools) && canImport(VixenEffectEditor)`, rather than editing VixenEffectEditor. Re-read VixenApp.swift right before editing it.

Status of every entry in `docs/spec/effect-editor.md`. This table is how passes hand over to each other.

Status values: **done** (with where it lives), **partial** (with what is missing), **todo**, **omitted** (with why).

Paths: `EE/` is `crates/vixen-session/src/effect_editor/`. `Kit` is `crates/vixen-ffi/src/kit.rs`, always built, because VixenKit is compiled in every build and can't call a feature-gated module. `FFI` is `crates/vixen-ffi/src/effecteditor.rs`. `UI/` is `app/Sources/VixenEffectEditor/`. `VK/` is `app/Sources/VixenKit/`.

| F | Feature | Status | Where, and what is missing |
|---|---|---|---|
| F1 | Effect Editor panel | done | `UI/EffectEditorPane.swift`: preview bar, header, one scroll view, information box at the end. Sequencer embeds `EffectEditorPane` |
| F2 | Showing the selection | done | `EffectEditorModel.selectionChanged`: 200 ms restart, identical selection does nothing. `EE/panel.rs panel_model` |
| F3 | Nothing selected | done | `panel_model` empty state, `UI/InformationBox` |
| F4 | Too many effects selected | done | `panel_model` too-many state; constant `DEFAULT_MULTI_EDIT_LIMIT` |
| F5 | Effect Preview switch | done | `UI/EffectEditorPane.previewBar`, `EffectEditorModel.previewOn`, `FFI preview_set_on` |
| F6 | Effect Preview playback | done | `EE/preview.rs EffectPreview`: loop on its own thread, schedule of copies, nothing cleared between loops. The engine is shared by taking turns (`crates/vixen-session/src/playback.rs` `begin_effect_preview`, `update_effect_preview`, `end_effect_preview`): real playback takes it from a running preview by itself, the windows see a stopped transport while a preview plays, and `FFI preview_poll` (every 250 ms from the panel) brings the preview back when the engine is free. Export calls `EffectEditorModel.suspendPreview()`/`resumePreview()`, or `VixenCore.holdEffectPreview(held:)` with no panel to hand. Stopping never waits (test 63). Checked in the app: on, Play takes it, Stop brings it back, off clears it |
| F7 | Header bar | done | `UI/HeaderBar` |
| F8 | Save as Default | done | `UI/HeaderBar`, `crates/vixen-io/src/effect_defaults.rs`, `crates/vixen-xml/src/nbfx.rs` (binary XML, text records when writing). Original managed C# contracts verify both directions for the store, three nested models and a66,001-byte opaque entry, including fractional UTC ticks; see `tools/golden/defaults/README.md`. Actual Windows application loading remains unverified. `EE/defaults.rs AppCore.effect_defaults`; new effects use `AppCore.new_effect_params`. Tools > Effect Defaults uses the same store |
| F9 | Reset to Built-in | done | `EffectEditorModel.resetDefault`, `FFI reset_default` |
| F10 | Help link | done | `UI/HeaderBar`, `UI/InformationBox`: `Link`, F11 |
| F11 | Information box | done | `UI/InformationBox`; empty text has no box |
| F12 | Scrolling | done | One `ScrollView`, no nested scrolling; collection lists are plain stacks |
| F13 | Category sections | done | `UI/SectionView`, `EditorSection.expanded` follows the name |
| F14 | Order | done | `visible_params_for`; `EE/panel.rs merged_params` for a mixed selection |
| F15 | Row layout: single value | done | `UI/RowView`, `NameLabel`, `labelWidth` measured over all rows |
| F16 | Row layout: collection | done | `UI/RowView` collection branch |
| F17 | Tooltips | done | `NameLabel` `.help`; `tooltip_of` |
| F18 | Visibility changes while editing | done | `EffectEditorModel.reload` diffs by `RowKey`; rows that stay keep their views |
| F19 | Choice lists refresh | done | Choices come with every `panel`; a 250 ms check of `edit_revision` picks up other windows |
| F20 | Filter box: not shown | omitted | Hidden upstream: not user visible (spec F20) |
| F21 | Which editor a row gets | done | `EE/panel.rs row_control`, kind first |
| F22 | Check box (`Bool`, no labels) | done | `UI/RowEditor` `.check` |
| F23 | Yes/No style choice (`Bool` with `bool_labels` and `Editor::Selection`) | done | `row_control` Bool with labels or Selection gives `Choice` |
| F24 | Integer field (`Int` with `Editor::Default` or `Number`) | done | `VK/ValueControls.swift DragNumberField`, `EE/numeric.rs`, Kit `NumberStripGesture`; gesture numbered for one undo step |
| F25 | Decimal field (`Double` with `Editor::Default`) | done | `DoubleField`, same control |
| F26 | Unsigned integers (`Long` / `Other("ulong")` with `Editor::Number`, and byte and ushort as `Int`) | done | `int_bounds`: `Long` is uint, `Other("ulong")` is u64 kept as raw text |
| F27 | Choice from a computed list (`Editor::Selection`) | done | `choice_control`, `UI/ChoicePicker`; blank when nothing matches |
| F28 | Depth choices | done | `EE/panel.rs merged_choices`: smallest depth among the selection's effects with a target; Fire and Wipe list `1..depth-1` and keep First's value when mixed, the `TargetElementDepthConverter` rows go blank; First's converter decides in a mix (tests 49, 50) |
| F29 | Enum drop-down (`Enum`) | done | `selection_items` leaves out `TilePictures.None`; a stored one shows blank |
| F30 | Label (read-only) | done | `RowControl::Label`; Video `VideoLength` refreshes on real file selection and `CacheSize` on completed rendering. Eleven session cache/metadata tests pass, including unchanged-selection revision, save/reopen, multi-selection and Undo generation guards; see `docs/review/video-metadata.md` and `docs/review/video-cache.md` for the separate native evidence |
| F31 | Text field (`Str`, `Guid` and other values with `Editor::Default`) | done | `UI/CommitTextField`, `text_to_value` |
| F32 | Sliders (`Int` + `Editor::Slider`, `Double` + `Editor::SliderDouble`, `Double` + `Editor::Level`, `Double`/`Percentage` + `Editor::Percent`) | done | `VK/ValueControls.swift ValueSlider`, `EE/numeric.rs`: 300 ms idle commit, one undo step per gesture, snap in Rust. Takes the focus when pressed, as upstream's does, then arrows ± tick, Page Up and Down ± 5, Home and End to the ends. Checked in the sequencer window: a click and twenty Lefts take Intensity from 100 to 80 as one step, "Undo 1 Effect(s) Intensity modified" |
| F33 | Display transforms | done | `row_control` and `stored_value` use `Display`; a write that stores the same value is `NoChange` |
| F34 | File path | done | `UI/PathField`, filters from `FFI path_filter`, `EE/panel.rs adopted_path` copies into the effect folder |
| F35 | Font | done | `VK/FontPicker.swift FontPanelSession`, `UI/FontField`, `font_display_text` |
| F36 | Single colour (`Color`, `Rgb`) | done | `VK/ColorControls.swift ColorSwatch`, `ColorPanelSession`, `DiscreteColorPicker.pickOne`, `UI/ColorField` |
| F37 | Polygons (`Other("PolygonContainer")`) | done | Real Polygon editor is connected to the existing Morph property entry and uses a draft transaction, single undo and preserved IDs/settings. Backend12 cases; native sample44, actual-effect42, fresh reload10 and gesture87 steps pass. Remaining minor polygon UI details are tracked in specialized-editor parity. |
| F38 | The curve strip: look | done | `VK/CurveStripView.swift`; `curve_strip_points`. SF Symbol badge in place of `librarylink.png` |
| F39 | Curve strip: move a point | done | `EE/curve_strip.rs`, Kit `CurveStripGesture`. November12 actual optimized-handler F39/F46 run passes71 checks, including strict2-point arming, actual point tooltip text/rounding, distinct moves, deferred callback and inert repeated release. Supplied local events do not establish physical pointer or visible-tooltip appearance; see `docs/review/inline-strip-native.md`. F42 handle tooltip evidence is separate. |
| F40 | Curve strip: add a point | done | `curve_mouse_down`, `mouse_up`; pinned by tests including the quirk |
| F41 | Curve strip: remove a point | done | `mouse_up`; refuses below 3 points |
| F42 | Curve strip: set a flat level | done | `CurveStripNSView` level bar and handle; `curve_level_from_handle`. One difference, for the owner's veto: the bar shows as soon as ⇧ is pressed over the strip, where upstream waits for the pointer to move. November12 actual optimized handler probe passes37 checks: strict >2-point travel, immediate updates on each qualifying move (documented FIX), deferred single commit, actual tooltip text/rounding and cleanup. Missing tooltip-on-press was reproduced and fixed; full builds/startup pass. Offscreen supplied mouse events do not prove physical pointer routing or visible tooltip appearance; see `docs/review/curve-handle-native.md` |
| F43 | Curve strip: invert, reverse | done | `mouse_up`; double-click and Return open F70 |
| F44 | The gradient strip: look | done | `VK/GradientStripView.swift`; `gradient_handles`, `gradient_strip_bands` |
| F45 | Gradient strip: add a stop | done | `gradient_add_stop`, checked by harness |
| F46 | Gradient strip: move a stop | done | `gradient_move_handle`, every accepted move after arming applies (FIX). The71-check optimized F39/F46 probe verifies strict2-point arming, repeated/shared-stop moves, complete data preservation, banker's center rounding, current Mac drag-tooltip text and one deferred commit. Original gradient `SliderPoint` uses defaultfalse for `showToolTip`; no original press-tooltip parity or missing press feature is claimed. Physical gestures/visible appearance remain open; see `docs/review/inline-strip-native.md`. |
| F47 | Gradient strip: delete a stop | done | `gradient_delete_handle`, ⌥-click and Delete |
| F48 | Gradient strip: edit a stop's colour | done | `VK/ColorControls.swift GradientStopColorEdit` |
| F49 | Gradient strip: reverse | done | `gradient_reverse`; ⇧⌥ on the strip |
| F50 | Gradient strip: open dialog | done | `GradientStripNSView.mouseDown` and `keyDown`; `VK/GradientEditorDialog` |
| F51 | Discrete awareness | done | `row_control`: `in_effect` false for rows of a list |
| F52 | Collection row kinds | done | `UI/CollectionEditors.swift`, `EE/panel.rs list_style`: every list kind observed; Fixture target workflow now passes 77 native steps, including named/colored headers, index choices and conditional Range/RGB controls (`target/fixture-authoring-ui-retry/native/result.json`). |
| F53 | Gradient and level pair rows | done | `UI/CollectionEditor` `.gradientLevels`, `list_with_item`; an unchanged pair is `NoChange`. Return opens the curve dialog from the curve half only, as upstream (`GradientStrip(opensOnReturn: false)` on the gradient half) |
| F54 | String lists | done | `CommitTextField.onFocus`, first row selected on load (F54's quirk kept, FIX applied) |
| F55 | Record lists (sub-rows) | done | `record_list_control`, `list_with_item`; whole list through `set_param`. Whirl's Meteor in `whirlpool/mod.rs`, State's colour in `state.rs`, both with `param_changed_ctx` |
| F56 | Add | done | `list_add`: empties per F56, Morph no add when empty, State's new row through `param_changed_ctx`; Fixture rows copy the selected capability's constructor fields. All 44 panel tests and 77 native Fixture workflow steps pass, including real picker/Add, Undo/Redo and save/reopen; see `docs/review/fixture-function-authoring.md`. |
| F57 | Remove | done | `can_remove`, `list_remove`, `list_style.min_items` |
| F58 | Selected row | done | `EffectEditorModel.selectedListRow`, `watchPresses`: a press outside the row's list clears it, the list's own buttons keep it, and a press on a strip, swatch or box selects (FIX) |
| F59 | Up and Down | done | `can_move`, `list_move`; `valid_move` pinned by upstream's test cases |
| F60 | What a multi-selection shows | done | `merged_params`, `top_row` |
| F61 | Merged metadata | done | `merged_params`, `merged_editor` |
| F62 | Setting a mixed value | done | `apply_to_all`, `commit`; tests |
| F63 | Undo | done | `PanelEdits.record`, `EffectEditorModel.undoRecorded`; `onUndoRecorded` hook for the sequencer |
| F64 | After a commit | done | `apply_to_all` starts renders off the main thread; `reload` |
| F65 | Selection of targets or marks changes while showing | done | `checkForOutsideEdits` |
| F66 | Keyboard and focus | done | Tab, Return and Escape in fields; commit on leaving. `VK/EditorKeys.swift`: strips, swatches and sliders declare the keys they use (`EditorKeyTaking`), and the sequencer's key monitor (`app/Sources/VixenSequencer/SequencerModel.swift handleKey`, one line) leaves those, text boxes, and pop-up and check box keys alone; everything else goes to the timeline's shortcuts as before. Checked in the sequencer window with the `editorTakesKey` hook: Left goes to the timeline until the slider is pressed, then Left and Home stay with it while `s` and Delete go on |
| F67 | Dragging a value out | done | `VK/KitSupport.swift ValueDragView` (`dragValue`, `startValueDrag`), `VixenDragValue`: private pasteboard types per kind and `NSColor` on colours; a press F39 captures, or a double-click, doesn't drag. Checked with the harness's `valueDrag` hook (`VK/ValueDragTesting.swift`), which takes what the view under one point would drag and offers it to the view under another through `NSDraggingDestination`: curve strip, gradient list item and swatch payloads all arrive. Only the window server's drag session itself isn't driven |
| F68 | Dropping a value in | done | `EE/panel.rs can_drop`, `drop_value` (the table, the discrete rule and the FIX refusing a gradient on a colour row, tested in Rust including test 35); `ValueDragView` accepts by asking Rust. Checked in the app with `valueDrag`: a curve onto a curve drops and commits one step ("1 Effect(s) Pulse Intensity modified", render follows, undo restores), a colour onto a curve and a curve onto a swatch are refused, a gradient list item onto another drops (`FFFF0000` lands in item 2), a gradient onto a curve is refused |
| F69 | Drags from the libraries and toolbars | done | `EE/drag.rs`, Kit `library_curve_drag_text`, `library_gradient_drag_text`: a copy, not current, named only when linking, a blank name unlinked (F107). The sources are the sequencer worker's library cells and toolbar strips; their drops land on this area's `ValueDragView`s, checked as F68 |
| F70 | Opening | done | `VK/CurveEditorDialog.swift CurveEditorDialog.present`, 300 ms guard |
| F71 | Three modes, from the curve's state | done | `EE/dialogs.rs CurveEditSession.mode` |
| F72 | The graph | done | `CurveEditSession.press`, `drag`, `release`, `cancel_drag`; `CurveGraphNSView`; harness. Review pass: ⌘⌥ away from a point adds one and takes it straight back out (KEEP, test `command_option_away_from_a_point_adds_then_removes`); it only added |
| F73 | Selected Point: X, Y, Update | done | `update_coordinates`, picks follow the sort |
| F74 | Reverse Curve, Invert Curve | done | `reverse`, `invert` |
| F75 | f(x) Curve | done | `EE/function.rs`, sheet in `CurveEditorDialog`. Unchanged original methods + pinned NCalc2 2.1.0: 4,282 cases / 8,650 points pass; exact Decimal arithmetic and retained partial points corrected. Diagnostic wording/locales remain outside the oracle; see `tools/golden/ncalc/README.md` |
| F76 | Draw Curve | done | `draw_begin`, `draw_move`, `set_draw_interval`; balloons as popovers |
| F77 | Load Curve | done | `load_from_library`, `VK/LibrarySelectors.swift` |
| F78 | Library group | done | `askForName`, `unlink`, `editLibraryEntry` |
| F79 | Curve Library selector | done | `VK/LibrarySelectors.swift CurveLibrarySelector` |
| F80 | Curve Library window: layout | done | Sequencer worker: `app/Sources/VixenSequencer/LibraryPanels.swift`, `crates/vixen-session/src/sequencer/libraries.rs`, on this area's `VixenCore.library_*`, dialogs and thumbnails (`docs/parity/sequencer-shell.md` F351) |
| F81 | Curve Library window: New, Edit, Delete | done | Sequencer worker: `app/Sources/VixenSequencer/LibraryPanels.swift`, `crates/vixen-session/src/sequencer/libraries.rs`; the delete text matches. Name loops and editor are VixenKit's `CurveEditorDialog.askForName`, `editLibraryEntry` |
| F82 | Curve Library window: reorder and add by drop | partial | `app/Sources/VixenSequencer/LibraryPanels.swift` now uses `libraryInsertCurve` at the pointer index and `libraryMoveCurves` for selection reorder; `LibraryDropTracker` enforces more than10 pt on either axis. The earlier missing-implementation note was stale. Backend/hook reorder verified; physical native drag and10 pt gesture remain unverified as sequencer F169–170 records |
| F83 | Dragging out: Maintain Library Link | done | Sequencer worker: `app/Sources/VixenSequencer/LibraryPanels.swift`, `crates/vixen-session/src/sequencer/libraries.rs`, "Maintain Library Link" saved as `curveLinkCurves`; payload from Kit `library_curve_drag_text` (blank name stays unlinked, F107) |
| F84 | Export and Import | done | Sequencer worker: `app/Sources/VixenSequencer/LibraryPanels.swift`, `crates/vixen-session/src/sequencer/libraries.rs`, `.vcl` with " 2", " 3" names; texts match |
| F85 | Thumbnail scale | done | Sequencer worker: `app/Sources/VixenSequencer/LibraryPanels.swift`, `crates/vixen-session/src/sequencer/libraries.rs`, ×1.03 per ⌘-scroll step, `Form_CurveLibrary/CurveLibraryImageScale` and `TextScale` |
| F86 | Editing a library curve | done | `AppCore.add_library_curve`, `session.libraries_changed`, `save_libraries` |
| F87 | Opening | done | `VK/GradientEditorDialog.swift` |
| F88 | Gradient bar and stops | done | `EE/dialogs.rs GradientEditSession`, `GradientBarNSView`; press, add, move, diamond, cursors |
| F89 | Selected Color Point row | done | `VK/GradientStopControls.swift`: `ColorLabelBox` (current over previous with `RRGGBB` text, black or white by upstream's contrast rule, border only when disabled; the colour panel's live colour shows on top) and `LocationUpDown` (digits-only field applying on Return or leaving, clamped, unparseable reverts; `NSStepper` by 1 with repeat; a chevron drops a vertical tracker in a popover, each value applying as it moves; setting it from outside raises nothing). Screen sampling is the colour panel's eyedropper |
| F90 | Editing a stop's colour | done | `editColor` in `GradientEditorContent` |
| F91 | Discrete display | done | `gradient_strip_bands` |
| F92 | Library state | done | `GradientEditSession.mode` |
| F93 | Library group actions | done | `askForName`, `GradientLibrarySelector`, `editLibraryEntry` |
| F94 | Gradient data rules to keep | done | `gradient_strip.rs`, `gradient_same_value`; operations clamp positions |
| F95 | Color Gradient Library selector | done | `VK/LibrarySelectors.swift GradientLibrarySelector` |
| F96 | Gradient Library window | partial | `app/Sources/VixenSequencer/LibraryPanels.swift` uses the correct upstream gradient import/export messages, target-index insertion and `libraryMoveGradients`. Earlier text/missing-implementation notes were stale. Physical native drag and10 pt gesture remain unverified as sequencer F169–170 records |
| F97 | Editing a library gradient | done | `editLibraryEntry` with no discrete limit |
| F98 | Color Library window | done | Sequencer worker: `LibraryPanel` in `app/Sources/VixenSequencer/LibraryPanels.swift`, palette and `.vfc` in `crates/vixen-session/src/sequencer/libraries.rs` (`docs/parity/sequencer-shell.md` F341 to F350) |
| F99 | Multiple Discrete Color Picker | done | `VK/ColorControls.swift MultipleDiscreteColorPicker` |
| F100 | Single Discrete Color Picker | done | `SingleDiscreteColorPicker` |
| F101 | Picker window size | done | `EE/dialogs.rs discrete_picker_size`; sheets use it |
| F102 | Which colours are "discrete" | done | `union_of_valid_colors`, State rows use the selection's |
| F103 | Vixen's Color Picker | done | `ColorPanelSession`: system panel, Quick Picks accessory, one commit per session |
| F104 | `Vixen.Common/WpfPropertyGrid` | omitted | Not the Effect Editor: Custom Prop Editor and Timing Track windows have their own specs |
| F105 | Choice lists in a multi-selection | done | `EE/panel.rs merged_choices`: union in first-seen order over the effects of First's type for the effect-list converters (every `Str` choice row but PhonemeMapping), minimum depth, boolean words falling back to True and False, First's list otherwise; the index is into the merged list. `FFI EffectEditorApi.panel`: LipSync's phoneme map names, from `vixen_io::lip_sync_map_names` (tests 49 to 53) |
| F106 | Custom State rows | done | `crates/fx-audio-state/src/state.rs` `CustomStateItems` case and `custom_item_valid_colors`; `list_style` inline row |
| F107 | Name prompt | done | `NamePromptSheet` keeps the text between rounds |
| F108 | Message boxes in this area | done | `askInSheet` with verbs |

## Forms in `docs/reference/forms/_index.json` for this area

| Form | Status |
|---|---|
| `FormEffectEditor` | `UI/EffectEditorPane.swift` |
| `CurveEditor` | `VK/CurveEditorDialog.swift` |
| `FunctionGenerator` | `VK/CurveEditorDialog.swift FunctionGeneratorSheet` |
| `CurveLibrarySelector` | `VK/LibrarySelectors.swift` |
| `ColorGradientEditor`, `GradientEditPanel` | `VK/GradientEditorDialog.swift` |
| `ColorGradientLibrarySelector` | `VK/LibrarySelectors.swift` |
| `Form_CurveLibrary`, `Form_GradientLibrary`, `Form_ColorLibrary` | Sequencer worker's panels (`app/Sources/VixenSequencer/LibraryPanels.swift`); they open the editors here. See F80 to F85, F96, F98 |
| `TextDialog` (name prompts, F107) | `VK/KitSupport.swift NamePromptSheet`, through `CurveEditorDialog.askForName` and `GradientEditorDialog.askForName` |
| `DimmingCurveHelper` | Not this area: the Dimming Curve filter's dialog, in `docs/parity/property-filter-controller-setup.md`. Its curve can open `CurveEditorDialog.present` |
| `MultipleDiscreteColorPickerView`, `SingleDiscreteColorPickerView`, `ColorPicker` (WPF and Common, not in the index) | `VK/ColorControls.swift` (F99 to F103) |

## Notes for the next pass

- **For the sequencer window** (`app/Sources/VixenSequencer`): `EffectEditorPane(session:selectedEffects:onUndoRecorded:)` or `EffectEditorPane(model:selectedEffects:)` with an `EffectEditorModel` it keeps. Real playback needs nothing: it takes the engine from the Effect Preview by itself, the transport the window sees stays its own, and the preview comes back after it. Export should call `model.suspendPreview()` and `model.resumePreview()` around the dialog, or `VixenCore.holdEffectPreview(held:)` with no panel at hand (F37 of `docs/spec/sequencer-shell.md`). The key monitor in `SequencerModel.handleKey` asks `EditorKeys.focusedEditorTakes(_:)` (one line added there this pass).
- **For the library panels**: `VixenCore.libraryInsertCurve(name:curve:index:)`, `libraryInsertGradient`, `libraryMoveCurves(selected:target:)`, `libraryMoveGradients` give F82's drop and reorder with upstream's semantics; `LibraryPanels.swift` now uses them, and the F96 texts are corrected. Physical drag verification remains open in F82/F96.
- **For the polygon editor**: `EffectEditorHooks.editPolygons`, see F37.
- Shared controls in VixenKit: `CurveStrip`, `GradientStrip`, `ColorSwatch`, `DragNumberField`, `ValueSlider`, `CurveThumbnail`, `GradientThumbnail`, `CurveEditorDialog`, `GradientEditorDialog`, `CurveLibrarySelector`, `GradientLibrarySelector`, `DiscreteColorPicker`, `ColorPanelSession`, `FontPanelSession`, `KitSheet`, `VixenDragValue`, `ColorLabelBox`, `LocationUpDown`, `EditorKeys`/`EditorKeyTaking`. `crates/vixen-ffi/src/kit.rs` is their Rust side. `app/Sources/VixenDialogs/FilterSheets.swift CurveEditorSheet` can call `CurveEditorDialog.present`.
- Harness hooks added this pass: `editorTakesKey` (what the focused control keeps from the sequencer's shortcuts; the `key` step goes straight to a window and never meets an app-wide monitor) and `valueDrag` (a drag from one point to another without the window server's session). The test window's menu has Play Sequence, Stop Sequence and Note Preview, and Dump Panel prints gradient list colours. The test window's `api` was a second panel with its own preview: it's the panel's own now.
- Checked in the running app this pass: all 16 of the owner's effects (snapshots and dumps), the gradient dialog's Selected Stop row, the Effect Preview hand-off with real playback, the panel inside the real sequencer window (selection, slider focus and keys, undo naming), drags between strips, swatches and list items with renders following, and undo.
- Original checkpoint limitation: WindowServer drags, library-selector double-click and native Color/Font panels were not driven end to end at that point. Later physical Curve/Gradient library scopes and actual native Color/Font ownership/close checks are recorded in [foreground evidence](../review/foreground-resume-20261112.md). Color-library reorder/copy and the remaining selector/editor gestures stay open; the earlier test-set results do not establish physical coverage.
- Snapshots of colours read differently between runs because of the window's colour space; Dump Panel's numbers are the ones to trust.
- `crates/vixen-xml/src/nbfx.rs` writes every value as text and reads all typed records. Files from the Windows app would prove it: none exist in the owner's profile.
- **NCalc verification, 2026-11-11:** the exact pinned package is now available. The original handlers generate the 4,282-case default oracle in `tests/golden/ncalc.json`. Parameter case, numeric types including Decimal, exceptional math and partial-point retention were corrected. See `tools/golden/ncalc/README.md` for provenance and limits.
- **Independent review, 2026-11-09.** Three behaviours marked done differed from the C# and are corrected in the rows above (F39 and F46 tooltip rounding, F42 handle travel, F72 ⌘⌥ press). About 40 behaviours were read against the C# line by line; the rest matched. Driven in the app this pass: the 16 owner effects and 14 others (snapshots and dumps against their attributes), Chase with Spin multi-edit (merged rows, ⇧⌘ invert on both, "2 Effect(s) Pulse Intensity modified", one undo), a merged check box revealing its dependent row, the 76-effect limit text, and Alternating's and Dissolve's lists (Add, select, Up, Down, Remove refused at the minimum, undo).
- Historical limits of that review: F42 travel tolerance and the strip tooltips were not driven then. November12 optimized actual-handler probes now cover F42 in37 checks and F39/F46 in71 checks, with transparent offscreen tooltip text/lifecycle evidence; physical pointer routing and visible appearance remain open. `function.rs` versus NCalc is superseded by the November11 oracle above.
- Reported, not changed: Morph's "Repeat Skip" opens at 0 on a slider whose range starts at 1. That is the table's default and `scripts/check_effect_metadata.py` has no finding for it, so it is upstream's own.
- Harness: `click` matches visible text by substring ("Up" lands on "Group Level"), so use `mouse` coordinates for a list's buttons. The test menu's "Add Next Of Same Type" adds the same effect each time; "Add To Selection" is the way to build a real multi-selection.
