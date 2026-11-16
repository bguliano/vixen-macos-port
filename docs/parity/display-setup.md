# Display Setup parity

Status of every entry in `docs/spec/display-setup.md`, the register of deliberate changes, and every upstream control of this area's forms.

**Status.** **done**: built. **partial**: built, with what's missing named. **elsewhere**: this window opens it and another area builds it. **omitted**: deliberately not built, with the reason.

**Paths.** `rs/` is `crates/vixen-session/src/setup/`. `FFI` is `crates/vixen-ffi/src/setup.rs` (and `setup/wire.rs`, `setup/fixture.rs`). `Setup/` is `app/Sources/VixenSetup/`, `Win/` is `app/Sources/VixenSetup/Window/`. `Kit/` is `app/Sources/VixenKit/`.

**How things were checked.** "Rust" means a unit test in the named file (`cargo test -p vixen-session --lib setup::`). "Harness" means driven through `VIXEN_UI_TEST` against a copy of the owner's show (`target/show-setup`), with snapshots read and the hooks in `Win/SetupTestHooks.swift` reporting state; real mouse and keyboard can't reach a test window, so clicks on the canvas go through the canvas's own event handlers and tree selections through `OutlineHandle`. "Code" means built and compiled but not run.

## Features

| F | Feature | Status | Where | Checked, and what's different |
|---|---|---|---|---|
| F1 | Open Display Setup | done | `Setup/SetupScene.swift` (`WindowID.displaySetup`), `Win/SetupModel.windowOpened`, `rs/mod.rs` `SetupSession::open` | Harness. Opening calls `AppModel.beginSystemConfigSession()`; the main window's buttons and menus are Administration's to disable |
| F2 | Three-column layout | done | `Win/SetupRootView.swift` (`HSplitView`, `SplitViewStart` sets 25/50/25 once) | Harness snapshot |
| F3 | Patching view switch | done | `Win/SetupRootView.PatchingPanel`, `GraphCanvasController.activeChanged` | Harness. Both views stay alive; an inactive view hears nothing and catches up when shown |
| F4 | Cross-panel event wiring | done | `Win/SetupModel` `elementsChanged`, `patchingUpdated`, `controllersChanged` | Harness. One refresh per gesture, not per row |
| F5 | Cross-panel selection commands | done | `rs/patching.rs` `find_outputs`, `find_elements`; `SetupModel.findOutputs`, `findElements` | Harness on the real show |
| F6 | OK | done | `rs/commit.rs` `SetupSession::commit`, `AppCore::apply_display`; `SetupModel.ok` through `AppModel.endSystemConfigSession(.performed)` | Rust and harness: a clean OK writes nothing; an edited OK rewrote SystemConfig.xml with only the edits and the built-in tags, and ModuleStore.xml with only three unused `xmlns:a` declarations on empty elements dropped (the vixen-io writer's way); reopening showed the saved state |
| F7 | Cancel, and closing the window | done | `SetupModel.cancel`, `windowShouldClose`, `SetupWindowAccess.CloseDelegate` | Harness for Cancel and Esc. Close box with a dirty session: code |
| F8 | Help | done | `SetupModel.help` (`https://www.vixenlights.com/docs/usage/display-setup/`) | Code; not run, so no browser opened in tests |
| F150 | Escape while something is in progress | done | `GraphCanvasView.cancelGesture`, `cancelOperation`, `performKeyEquivalent` | Harness: Esc through the window during a frame, move and connect drag cancels only the drag; Esc with nothing going on is Cancel. Tree drags use the system drag loop, which takes Esc itself (code) |
| F9 | "Add:" type picker | done | `Win/SetupPanels.ElementsPanel`, `rs/templates.rs` `ALL_TEMPLATES` | Harness |
| F10 | Add Elements (+) | done | `SetupModel.addFromTemplate` | Harness: Single Item and Arch |
| F11 | Element tree | done | `Kit/OutlineView.swift`, `ElementTreeProvider`, `rs/elements.rs` `element_rows` | Harness |
| F12 | Delete Elements button | done | `SetupModel.deleteElements`, `rs/elements.rs` `delete_rows` | Harness |
| F13 | Rename Elements button | done | `SetupModel.renameElements` | Harness |
| F14 | Find outputs button | done | `SetupModel.findOutputs` | Harness |
| F15 | Configure row | done | `SetupModel.runHelper`; the helpers are the dialogs area's | Harness: placeholder without the dialogs area, the real helper sheet with it |
| F16 | Properties list | done | `ElementsPanel` (buttons under the list: Add, Delete, Configure) | Harness snapshot |
| F17 | Add Property | done | `SetupModel.addProperty`, `rs/properties.rs` `add_property` | Harness. Sheet titled "Add Property" (register) |
| F18 | Delete Property | done | `SetupModel.deleteProperties`, `rs/properties.rs` `remove_properties` | Harness: "Remove Property?" |
| F19 | Configure Property | elsewhere | `SetupModel.configureProperty` → `DialogEntryPoint` | Harness: placeholder without the dialogs area, the real Color sheet with it |
| F20 | Enable rules, elements panel | done | `rs/elements.rs` `TreeMenuState`, `SetupModel` counts | Harness snapshots |
| F21 | Rows and hierarchy | done | `rs/elements.rs`, `rs/tree.rs` | Rust, harness |
| F22 | Status icon per row | done | `rs/mod.rs` `RowIcon`, `try_element_icons` | Harness: green, folder |
| F23 | Lazy children | done | `OutlineView` `outlineChildrenAsync`, `shouldExpandItem` | Harness, and a 20,000-child group (synthetic) |
| F24 | Expansion memory | done | `OutlineView` boxes keep expansion across reloads | Harness |
| F25 | Tag colour dots | done | `OutlineRow.trailingDots`, `rs/tags.rs` | Harness snapshot |
| F26 | Selection | done | `OutlineView`, `SetupModel.elementSelectionChanged` | Harness |
| F27 | Selecting from code | done | `OutlineHandle.select(_:expanding:)`, `rs/elements.rs` `reveal_nodes` | Harness |
| F28 | Keyboard: Delete | done | `TreeOutlineView.keyDown` → `onCommand(.delete)` | Code (the command itself: harness) |
| F29 | Keyboard: Copy, Paste, Paste as New | done | `TreeOutlineView` `copy:`, `paste:`, ⇧⌘V | Code (the commands: harness) |
| F30 | Keyboard navigation | done | `NSOutlineView` | Native |
| F31 | Find elements | done | `SetupModel.findElements` | Harness |
| F149 | Double-click on an element row | done | `OutlineView.Coordinator.doubleClicked` toggles a row with children when no handler is set | Code |
| F32 | Enable and visibility table | done | `rs/elements.rs` `tree_menu_state`, `SetupModel.elementMenu` | Harness menu dumps against the table |
| F33 | Cut | done | always disabled | Harness menu dump (register) |
| F34 | Copy | done | `rs/elements.rs` `copy_rows` | Harness |
| F35 | Paste: links | done | `rs/elements.rs` `paste_links`, `live_clipboard` | Rust: a node deleted since the copy is skipped, and with nothing left the items are off and a keyboard paste beeps (register) |
| F36 | `CanPaste` rule | done | `rs/tree.rs` `can_paste` | Rust, harness (Paste as New onto the copied node refused) |
| F37 | Paste as New | done | `rs/elements.rs` `paste_as_new`, `duplicate_subtree` | Rust, harness ("Plug 15 - 2") |
| F38 | Copy properties | done | `rs/properties.rs` `copy_properties` | Harness |
| F39 | Paste properties | done | `rs/properties.rs` `paste_properties` | Rust |
| F40 | Add | done | `SetupModel.addElement` | Harness, with the convert prompt. Declining it cancels (register) |
| F41 | Add Multiple | done | `rs/elements.rs` `add_elements` (one batch) | Harness: NewName-1 to 5. The F43 prompt comes once first (register) |
| F42 | Delete | done | `rs/elements.rs` `delete_rows` | Harness: an emptied group becomes a leaf with a new element |
| F43 | "Loses patches" prompt | done | `rs/elements.rs` `would_lose_patches`, `SetupModel.checkLosesPatches` | Harness |
| F44 | Row deletion semantics | done | `rs/elements.rs` `delete_rows`, `rs/tree.rs` `remove_node` | Rust: group and child together leave nothing; elements no node refers to stay in the file (register) |
| F45 | Create Group | done | `rs/elements.rs` `create_group` | Harness. Blank prompt title, as upstream's one-argument `TextDialog` |
| F46 | Rename (context) | done | `SetupModel.renameElements` | Harness |
| F47 | Find/Replace | done | `VixenKit/SubstitutionRenamerSheet.swift`, `rs/naming.rs` `replace_patterns` | Harness: sheet opens; refreshes others (register) |
| F48 | Collapse All | done | `OutlineHandle.collapseAll` | Harness |
| F49 | Rename rules | done | `rs/elements.rs` `rename_rows`, `rs/tree.rs` `uniquify` | Rust: `[A, B]` to `[B, C]` gives `B - 2`, `C`; "Root" gives "Root - 2" |
| F50 | Reverse Elements | done | `rs/tree.rs` `reverse_rows` | Harness |
| F51 | Sort | done | `rs/tree.rs` `sort_children` | Harness |
| F52 | Export Wire Diagram | done | `VixenKit/WireDiagramSheet.swift`, `rs/wire_diagram.rs` | Harness: SVG written; alert for a group without Location |
| F53 | Export Element Tree | done | `rs/export.rs` (`.v3e`) | Harness: file written |
| F54 | Tags submenu | done | `SetupModel.elementMenu` tag items | Harness |
| F55 | Toggle a tag | done | `rs/tags.rs` `toggle_tag` | Harness |
| F56 | Manage Tags | done | `Setup/Sheets/ManageTagColorsSheet.swift` | Harness: sheet opens |
| F57 | Drag: moving | done | `OutlineView` drag and drop, `rs/elements.rs` `drop_rows` | Harness through the same call a drop ends in; real mouse drags not run |
| F58 | Drag: copying (linking) | done | as F57, ⌥ held | Harness, as F57 |
| F59 | Drag validity | done | `rs/elements.rs` `can_drop_rows` | Rust, harness (a group onto its own child refused) |
| F60 | Selection after Paste, Sort, Reverse | done | `EditResult.select`, `SetupModel.apply` | Rust |
| F61 | Patch points | done | `rs/flow.rs` `patch_points`, `count_patch_points` | Rust, and the real show's counts (3,367 outputs, all patched) |
| F62 | "Selected Elements" counts | done | `rs/patching.rs` `element_stats` | Harness |
| F63 | Unpatch Elements | done | `rs/patching.rs` `unpatch_elements_preview`, `unpatch_elements` | Harness, and saved |
| F64 | Reverse Element Order | done | `rs/patching.rs` `PatchOptions` | Harness |
| F65 | "Selected Controllers" counts | done | `rs/patching.rs` `controller_stats` | Harness, including outputs a collapse hid (F81) |
| F66 | Unpatch Controllers | done | `rs/patching.rs` `unpatch_outputs` | Harness |
| F67 | Patching Options group | done | `Win/SetupPanels.SimplePatchingView` | Harness snapshot. Only the element reverse box has a tooltip |
| F68 | Summary and warnings | done | `rs/patching.rs` `patch_summary`, `order_check` | Harness: "This will patch 0 element patch points to 0 controller outputs." with nothing selected |
| F69 | Patch Elements | done | `rs/patching.rs` `patch_elements`, `SetupModel.patchElements` | Rust, harness ("Patched 3 ...") |
| F70 | View state lifetime | done | `SetupModel.resetForNewWindow` | Harness: reopening starts from the saved system, clean |
| F71 | Mouse selection | done | `NSOutlineView` | Native (register) |
| F72 | `CanReverseElements` | done | `rs/tree.rs` `can_reverse` | Rust |
| F73 | Keyboard navigation | done | `NSOutlineView`, `OutlineView.noteKeyDown` keeps the range anchor | Native (register) |
| F74 | Drag visuals and hit zones | done | `NSOutlineView` drop indicators | Code. Primary button only; the unused Move Here / Copy Here menu is omitted (see controls) |
| F75 | Drop refusals | done | `rs/elements.rs` `can_drop_rows` | Rust: a true ancestor test (register) |
| F153 | Dragging between the trees | done | `OutlineView.validateDrop` answers only the drag its own tree started | Code |
| F76 | Add row | done | `ControllersPanel` | Harness |
| F77 | Add Controller | done | `SetupModel.addController`, `rs/controllers.rs` `add_controller` | Harness: name and count prompts, only the new controller selected. It starts at OK, when the engine runs it |
| F78 | Move up / down | done | `rs/controllers.rs` `move_controllers` | Harness (register) |
| F79 | Drag to reorder | done | `rs/controllers.rs` `drop_controllers` | Harness through the drop call (register: no device restarts) |
| F80 | Controller tree rows | done | `rs/controllers.rs` `controller_rows`, pages over 5,000 outputs | Harness; pages on the synthetic display |
| F81 | Logical controller selection | done | `Kit/OutlineView.swift` `keepsHiddenSelection`, `SetupModel.controllerHidden` | Harness: outputs stay selected and counted while collapsed, come back highlighted, and the menu is off with nothing highlighted |
| F82 | Double-click | done | `ControllersPanel` `onDoubleClick` → `configureController` | Code |
| F83 | Keyboard | done | `onCommand(.delete)` only | Code |
| F84 | Find elements button | done | `SetupModel.findElements` | Harness |
| F85 | Read-outs | done | `rs/controllers.rs` `controller_panel_state` | Harness: "Generic Serial", 16 |
| F86 | Buttons | done | `ControllersPanel` | Harness snapshot |
| F87 | Configure Controller | elsewhere | `SetupModel.configureController` → `DialogEntryPoint` | Harness with the dialogs area: the Generic Serial sheet opens on the working copy |
| F88 | Channel Count | done | `SetupModel.channelCount`, `rs/controllers.rs` `set_output_count` | Harness: bounds 0 to no limit |
| F89 | Rename Controller | done | `SetupModel.renameController` | Harness |
| F90 | Delete Controller(s) | done | `SetupModel.deleteControllers`, `rs/controllers.rs` `delete_controllers` | Harness: selection cleared, counts zero (register) |
| F91 | Start / Stop | done | `rs/controllers.rs` `start_controllers`, `stop_controllers`; `playback.rs` `set_controller_running` | Harness: icon and menu state. Hardware is only touched when the app isn't muted; Cancel restores what was running |
| F92 | Menu with a controller selected | done | `SetupModel.controllerMenu`, `rs/controllers.rs` `controller_menu_state` | Harness: singular and plural |
| F93 | Menu with only outputs | done | as F92 | Harness |
| F94 | Insert Channels | done | `rs/controllers.rs` `insert_outputs` | Harness |
| F95 | Remove Channels | done | `rs/controllers.rs` `remove_selected_outputs`, `SetupModel.removeChannels` | Harness: the same index stays selected, an index past the end goes |
| F96 | Unpatch Channel(s) / Controller(s) | done | `rs/patching.rs` `unpatch_outputs` | Harness |
| F97 | Find Patched Element(s) | done | `SetupModel.findElements` | Harness |
| F98 | Other context items | done | `SetupModel.controllerMenu` | Harness menu dumps |
| F99 | Template list | done | `rs/templates.rs` `ALL_TEMPLATES` | Harness |
| F100 | Shared add flow | done | `SetupModel.addFromTemplate`, `Setup/Sheets/TemplateSheet.swift` | Harness: Arch with the dimming prompt and the colour helper |
| F101 | Uniquifying in templates | done | `rs/templates.rs` `uniquify_in` | Rust |
| F102 | Single Item | done | `rs/templates.rs` | Rust, harness |
| F103 | Numbered Group, Arch, Candy Cane, Star | done | `rs/templates.rs` | Rust; Arch by harness |
| F104 | Starburst | done | `rs/templates.rs` | Rust |
| F105 | Icicles | done | `rs/templates.rs` `parse_icicle_pattern`, `TemplateSheet` | Rust (register: pasted text validated) |
| F106 | LipSync | done | `rs/templates.rs` | Rust |
| F107 | Megatree | done | `rs/templates.rs` | Rust. Order logic ignores Pixel Tree, as upstream |
| F108 | Pixel Grid | done | `rs/templates.rs` | Rust |
| F109 | `AddPatchingOrder` | done | `rs/templates.rs` `add_patching_order(_zigzag)` | Rust |
| F110 | Intelligent Fixture | done | `rs/fixture.rs`, `Setup/FixtureWizard/*`, `SetupModel.addFixture` | Rust (64 tests); the pages by the worker's harness; harness here: the wizard opens from the Add list, its close box cancels and leaves the window as it was, and closing Display Setup closes it. Per-run result (register) |
| F111 | Graphical view layout | done | `Win/GraphCanvas.swift` `GraphicalPatchingView` | Harness snapshot |
| F112 | What it displays | done | `rs/graph.rs` `graph_scene`, `displayed_filters` | Rust, harness. Each filter once (register) |
| F113 | Shape appearance | done | `GraphCanvasView.draw`, `ShapeStyle` | Harness snapshots. Scene from the model each time (register) |
| F114 | Ports | done | `rs/graph.rs` `port_fraction` | Rust, harness |
| F115 | Layout | done | `rs/graph.rs` `layout_filters` | Rust, harness. Depth computed at each layout (register). Between layouts filters stay where they are, and a filter that lost its source stays until the next one. Free-floating filters go under connected ones when both are shown, where upstream's top grid would cover them (addition) |
| F116 | Lines | done | `rs/graph.rs` `graph_lines` | Rust, harness: left and right stubs. All leaves get stubs (register) |
| F117 | Selecting | done | `GraphCanvasView.select`, frame select | Harness: click, ⌘ and ⇧ toggle, empty click clears, frame, ⌘A (root shapes and lines, as NShape), right click keeps the selection, a new filter or line is selected |
| F118 | Moving filters | done | `GraphCanvasView.beginMove`, `commitMove`; `GraphCanvasController.placeFilters` (undo) | Harness: drag, Esc, undo. A drag that starts on an element frames instead |
| F119 | Patching by drag | done | `GraphCanvasView.beginConnect`, `connectTarget`; `rs/graph.rs` `graph_connect_candidate`, `graph_connect` | Harness: left drag from a port, right drag and Control drag from the body (first free port), refusals (itself, a loop, an element), the new line selected, no relayout (register) |
| F120 | Delete | done | `GraphCanvasView.deleteSelection`, `rs/graph.rs` `graph_delete` | Harness: line, filter, Delete Filter button, left stub cuts its patch, right stub beeps (register) |
| F121 | Add Filter | done | `GraphCanvasController.addFilter`, `rs/graph.rs` `graph_add_filter` | Harness: sorted list titled "Add Filter", new filter selected and kept across selection changes (register) |
| F122 | Copy, Cut, Paste filters | done | `GraphCanvasView` `copy:`, `cut:`, `paste:`; `rs/graph.rs` `graph_paste_filters` | Harness through the actions the Edit menu sends: paste centred at the pointer or the last click, last one selected |
| F123 | Context menu | done | `GraphCanvasView.contextMenu` | Harness: items and their enabling, Paste Filter (Multiple) with its prompt (register) |
| F124 | Double-click | done | `GraphCanvasView.doubleClick`, `rs/graph.rs` `graph_toggle_collapsed` | Harness: group collapses to one box with its lines gathered at the edge, expands, nothing when not selected; a filter opens its dialog, the real Color Breakdown sheet with the dialogs area (register) |
| F125 | Zoom | done | `GraphCanvasController` `zoom`, `zoomToFit`; `rs/graph.rs` `zoom_step`, `zoom_to_fit` | Rust, harness: 100, 110, 121, 108, fit (register). Pinch and ⌘+/⌘-: code |
| F126 | Unpatch while Graphical shows | done | `SetupModel.unpatchControllers` | Code path shared with Simple (register) |
| F127 | Incidental NShape behaviour | omitted | | Port-drag resize and input-port drags do nothing (register) |
| F151 | Cursor feedback | done | `GraphCanvasView.updateCursor` | Harness: open hand over a filter, crosshair over an output port, arrow elsewhere, closed hand while moving, pointing hand while connecting |
| F152 | Auto-scroll while dragging | done | `autoscroll(with:)` in the drag handlers | Code |
| F128 | Wire diagram options | done | `VixenKit/WireDiagramSheet.swift` | Worker's harness. Cancel left of OK |
| F129 | Node order and labels | done | `rs/wire_diagram.rs` | Rust (48 tests) |
| F130 | Drawing | done | `rs/wire_diagram.rs` | Rust; label placement compared with a literal port on five datasets |
| F131 | Result | done | `SetupModel.exportWireDiagram` | Harness: file written; alert when a leaf has no Location (register) |
| F132 | Manage Tag Colors | done | `Setup/Sheets/ManageTagColorsSheet.swift` | Harness: opens. Real colour panel pick not run |
| F133 | Name generator | done | `VixenKit/NameGeneratorSheet.swift` | Harness: opens; worker's harness for its controls. The hidden template label and combo aren't shown |
| F134 | Template and rule list | done | as F133 | Fixed rule order: Sequential Letters, List of Letters, Sequential Numbers, List of Words |
| F135 | Generation algorithm | done | `rs/naming.rs` `generate_names` | Rust |
| F136 | Rule types | done | `rs/naming.rs` | Rust: "a,,b" is three words, "" one empty word |
| F137 | Find / Replace Name | done | `VixenKit/SubstitutionRenamerSheet.swift` | Worker's harness. U and D off with nothing selected (register) |
| F138 | Naming rules | done | `rs/tree.rs` `uniquify` | Rust (register: unchanged names stay) |
| F139 | Wizard window | done | `Setup/SetupScene.swift` (`SetupWindows.fixtureWizard`), `Setup/FixtureWizard/*` | Native wizard 58 steps and curve/save 47 steps pass. Finish on Summary only; revisits preserve answers while their dependencies remain compatible, as the audited specialized-editor spec requires |
| F140 | Decorative moving heads | done | `Kit/MovingHeadView.swift`, `vixen-render/fixture.rs`, `FixtureWizardView.swift` | Shared Metal geometry and local camera, pan 35/315 and page colors. Owner chose OpenGL +180° bottom tilt consistently in live and editor views. Native snapshots inspected |
| F141 | Select Profile | done | `rs/fixture.rs`, `FixtureWizardPages.swift` | Rust, worker's harness |
| F142 | Edit Functions, Edit Channels | done | `VixenDialogs/FixtureEditor.swift`, shared `rs/fixture_editor.rs`, `FFI/setup/fixture.rs` | Shared profile/function/channel editors, validation blocks invalid navigation; native wizard 58 steps pass. See specialized-editors F109–F123 |
| F143 | Color Support | done | as F141 | Rust |
| F144 | Automation | done | as F141 | Rust (register: disabled means false) |
| F145 | Dimming Curves | done | `FixtureDimmingCurvesPage`, `Kit/CurveEditorDialog.swift`, `FFI/setup/fixture.rs` | Shared editor accepts a CurveValue and retains the previous draft on Cancel. Native 47-step run verified Cancel/reopen, Reverse/Return/reopen and generated filter XML `(0,100),(100,0)`. Backend generation tests cover per-color copies |
| F146 | Grouping | done | as F141 | Rust |
| F147 | Summary | done | as F141 | Rust |
| F148 | Shared prompts | done | `Setup/Sheets/PromptSheets.swift`, `SetupModel.promptText`, `promptNumber`, `promptList` | Harness: the prompts' titles, text and bounds as listed above |

## Register of deliberate changes

Every row of the spec's section N is built as the Mac column says: F7, F150, F17/F121/F148, F44, F77, F90, F112, F113/F115, F120, F125, F137, F144, F33, F35/F37, F40, F41, F47/F51, F57, F71, F73, F75, F153, F78/F79, F105, F110, F116, F118, F119, F121/F122, F123, F124, F126, F127, F131, F138. F140's initial omission is superseded by the shared Metal fixture rendering. Where each lives is in the table above.

Further differences, for the owner to veto:

- **Clean OK writes nothing.** With no edits, OK leaves both files byte-identical and makes no backups (upstream rewrites them).
- **Filters stay put between layouts (F115, F119, F120).** A connect, a delete or a filter's setup doesn't move any filter; a filter whose line was deleted stays on the canvas until the next element selection change, element edit or resize, so it can be reconnected.
- **Free-floating filters under connected ones (F115).** Upstream's top grid would cover connected filters; it only shows both through its depth bug.
- **Paste centred at the pointer (F122)**, as NShape places a shape by its centre.
- **Group Color and Location data (data section).** At OK they go from the store and the group's property takes its default values, which is what upstream shows after the next load.
- **Elements no node refers to (F44)** are kept at OK, as upstream keeps them, so their ids stay.
- **Row ids** in the trees are arena paths such as `12/345`, not GUIDs.
- **Controller run state**: when the engine isn't running or the app is muted, controllers count as running unless on the disabled list, and Start/Stop only changes the list. Cancel restarts or stops only what this session changed.
- **Property edits** go into the working copy and reach the system at OK, not at once.
- **Unpatch prompt** has no close box, so it can't be dismissed as "Yes" (F63, F66).

## Controls

Every control of the forms in `docs/reference/forms` for this area. Layout containers (`TableLayoutPanel`, `FlowLayoutPanel`, `Panel`, divider labels, `components`, `toolTip1`, sizes) are SwiftUI stacks, separators and `.help` tooltips, and aren't listed one by one.

| Form | Control | Mac |
|---|---|---|
| DisplaySetup | `elementLabel` "Elements", `patchingHeaderLabel` "Patching", `controllersHeaderLabel` "Controllers" | `SetupPanelHeader` in each column |
| DisplaySetup | `radioButtonPatchingSimple`, `radioButtonPatchingGraphical` | Radio group in `PatchingPanel` |
| DisplaySetup | `buttonOk`, `buttonCancel`, `buttonHelp` | Bottom of the controllers column; OK default, Cancel on Esc |
| SetupElementsTree | `label1` "Add:", `comboBoxNewItemType`, `buttonAddTemplate` | Add row in `ElementsPanel` |
| SetupElementsTree | `elementTree` | `OutlineView` with `ElementTreeProvider` |
| SetupElementsTree | `buttonDeleteElements`, `buttonRenameElements`, `buttonSelectDestinationOutputs` | Icon buttons under the tree |
| SetupElementsTree | `groupBoxSelectedItems`, `label2` "Configure:", `comboBoxSetupHelperType`, `buttonRunHelperSetup` | "Selected Item(s):" group |
| SetupElementsTree | `label3` "Properties:", `listViewProperties`, `columnHeader2`, `listViewItem1`-`4` (Designer samples, never shown) | Properties list |
| SetupElementsTree | `buttonAddProperty`, `buttonRemoveProperty`, `buttonConfigureProperty` | Under the list, in that order |
| ElementTree | `cutNodesToolStripMenuItem` | "Cut", always off (F33) |
| ElementTree | `copyNodesToolStripMenuItem`, `pasteNodesToolStripMenuItem`, `pasteAsNewToolStripMenuItem` | Menu items, and ⌘C, ⌘V, ⇧⌘V |
| ElementTree | `nodePropertiesToolStripMenuItem` > `copyPropertiesToolStripMenuItem`, `pastePropertiesToolStripMenuItem` | "Item properties" submenu |
| ElementTree | `addNewNodeToolStripMenuItem`, `addMultipleNewNodesToolStripMenuItem`, `deleteNodesToolStripMenuItem`, `createGroupWithNodesToolStripMenuItem`, `renameNodesToolStripMenuItem`, `patternRenameToolStripMenuItem`, `collapseAllToolStripMenuItem`, `reverseElementsToolStripMenuItem`, `sortToolStripMenuItem` | Menu items of the same names |
| ElementTree | `exportWireDiagramToolStripMenuItem` > `frontWireDiagramToolStripMenuItem`, `backWireDiagramToolStripMenuItem`; `exportElementTreeToolStripMenuItem` | Submenu and item |
| ElementTree | `tagsToolStripMenuItem` | "Tags" submenu, with Manage Tags... |
| ElementTree | `contextMenuStripDragging`, `moveHereToolStripMenuItem`, `copyHereToolStripMenuItem` | Omitted: upstream never shows this menu (F74); ⌥ chooses copy |
| ElementTree | `treeview`, `treeIconsImageList`, separators | `TreeOutlineView`, SF Symbols and coloured balls |
| SetupPatchingSimple | `groupBoxElements` and its labels: Total Patch Points, Connected, Unconnected, Items, Groups, Elements, Filters with their counts | "Selected Elements" box |
| SetupPatchingSimple | `buttonUnpatchElements`, `checkBoxReverseElementOrder` | Same box and Patching Options |
| SetupPatchingSimple | `groupBoxControllers` and its labels: Outputs, Patched, Unpatched, Controllers, First output, Last output with their values | "Selected Controllers" box |
| SetupPatchingSimple | `buttonUnpatchControllers`, `checkBoxReverseOutputOrder` | Same box and Patching Options |
| SetupPatchingSimple | `groupBoxPatching`, `groupBoxElementOptions`, `radioButtonUnconnectedPatchPointsOnly`, `radioButtonAllAvailablePatchPoints`, `groupBoxOutputOptions`, `radioButtonUnpatchedOutputsOnly`, `radioButtonAllOutputs` | "Patching Options" |
| SetupPatchingSimple | `labelPatchSummary`, `labelPatchWarning`, `buttonDoPatching` | Summary line, yellow warning, "Patch Elements" |
| SetupPatchingGraphical | `diagramDisplay`, `diagramSetController`, `project`, `cachedRepository`, `roleBasedSecurityManager1` | `GraphCanvasView` in `GraphScrollView`; the NShape plumbing has no counterpart |
| SetupPatchingGraphical | `buttonAddFilter`, `buttonDeleteFilter`, `buttonZoomIn`, `buttonZoomOut`, `buttonZoomFit` | Button row under the canvas |
| SetupPatchingGraphical | `diagramContextMenuStrip`, `copyFilterToolStripMenuItem`, `pasteFilterToolStripMenuItem`, `pasteFilterMultipleToolStripMenuItem` | Canvas context menu (F123) |
| SetupControllersSimple | `label5` "Add:", `comboBoxNewControllerType`, `buttonAddController`, `buttonRaiseControllerOrdering`, `buttonLowerControllerOrdering` | Add row |
| SetupControllersSimple | `controllerTree` | `OutlineView` with `ControllerTreeProvider` |
| SetupControllersSimple | `buttonSelectSourceElements` | Find button under the tree |
| SetupControllersSimple | `groupBoxSelectedController`, `label1` "Type:", `labelControllerType`, `label2` "Outputs:", `labelOutputCount` | "Selected Controller" group |
| SetupControllersSimple | `buttonConfigureController`, `buttonNumberChannelsController`, `buttonRenameController`, `buttonDeleteController`, `buttonStartController`, `buttonStopController` | Icon buttons in that order |
| ControllerTree | `configureToolStripMenuItem`, `channelCountToolStripMenuItem`, `renameToolStripMenuItem`, `deleteToolStripMenuItem`, `startControllerToolStripMenuItem`, `stopControllerToolStripMenuItem`, `unpatchControllerToolStripMenuItem` | Controller menu |
| ControllerTree | `insertChannelsToolStripMenuItem`, `removeChannelsToolStripMenuItem`, `unpatchChannelsToolStripMenuItem`, `findPatchedChannelsToolStripMenuItem` | Output menu, singular and plural titles |
| ControllerTree | `treeview`, `treeIconsImageList`, `toolStripSeparator` | `TreeOutlineView`, icons, separator |
| ListSelectDialog | `listBoxItems`, `buttonOk`, `buttonCancel` | `ListSelectSheet` (OK off until a row is chosen; double-click or Return accepts) |
| NumberDialog | `labelPrompt`, `numericUpDownChooser`, `button1` OK, `button2` Cancel | `NumberPromptSheet` |
| TextDialog | `labelPrompt`, `textBoxResponse`, `buttonOk`, `buttonCancel` | `TextPromptSheet` |
| Templates, naming editors, MessageBoxForm, tag manager, wire diagram, fixture wizard | all controls | Mapped in the sheets', wire diagram's and wizard's own sections of this file's sources: `Setup/Sheets/*` (control map in each sheet's header comment), `VixenKit/WireDiagramSheet.swift`, `Setup/FixtureWizard/*`. `NameGenerator`'s hidden `label2` and `comboBoxTemplates` aren't shown, as upstream never shows them |

## Dialog entry points

The property, filter and controller dialogs are `docs/spec/property-filter-controller-setup.md`'s. Every one goes through `Win/DialogEntryPoints.swift` `DialogEntryPoint` with a `SetupDialogRequest`; with the `dialogs` area in the build it shows the real sheet working on this window's working copy (`DisplaySetupSession.dialogContext()`), otherwise a placeholder naming it.

| Request | Opened from |
|---|---|
| `.controller` | Configure button, controller menu Configure, double-click on a controller row (`SetupModel.configureController`) |
| `.filter` | Double-click on a selected filter shape (`GraphCanvasController.configureFilter`) |
| `.property` | Configure Property button and a double-click in the properties list (`SetupModel.configureProperty`) |
| `.helper` | Configure row (`SetupModel.runHelper`); after a template, Dimming Curve and Color Handling (`addFromTemplate`); Patching Order when an order conflicts (`patchElements`) |
| Wizard placeholders | Fixture Function Editor, Fixture Channel Editor, Curve Editor (`Setup/FixtureWizard`) |

### Shared tool sheets — 2026-11-11

NameGeneratorSheet, SubstitutionRenamerSheet, NamePreview, SheetControls and WireDiagramSheet moved into VixenKit so the Custom Prop Editor uses the same native controls. Their setup-prefixed FFI entrypoints remain unchanged, now in always-built `vixen-ffi/naming.rs` and `wire_diagram.rs`. Combined Setup/Tools build passes; existing naming15 and wire48 tests pass (one pre-existing wire performance test ignored). Native Setup rename/find-replace smoke passed16 steps, returning12 expected names and nil after Cancel, with inspected screenshots under `target/resume-ui/shared-naming/out`.
