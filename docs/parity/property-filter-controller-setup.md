# Property, filter and controller setup parity

Every entry in `docs/spec/property-filter-controller-setup.md`, and below it every control in the `docs/reference/forms` files for these dialogs.

Paths: `L/` is `crates/vixen-session/src/dialogs/`, `FFI/` is `crates/vixen-ffi/src/dialogs/`, `UI/` is `app/Sources/VixenDialogs/`, `O/` is `crates/vixen-output/src/`.

"Checked" means the sheet was opened and used in the running app against a copy of the owner's real show, and `SystemConfig.xml` and `ModuleStore.xml` were saved and compared with a save that had no dialog in it. "Checked in XML" means that comparison showed the change and nothing else. "Tested" means a Rust unit test.

## Features

| F | Feature | Status | Where, and what is missing |
|---|---|---|---|
| F1 | Configure a controller | done | `UI/EntryPoints.swift` `ControllerSetupSheet`, routed by `L/controllers.rs` `controller_setup_kind`. A module with no setup gets nothing to show. Display Setup's cog opens it: run in the combined build with Generic Serial |
| F2 | Configure a filter | done | `UI/EntryPoints.swift` `FilterSetupSheet`, routed by `L/filters.rs` `filter_setup_kind`. The double-click is Display Setup's (display-setup F124). Output counts: `L/setup_world.rs` `set_filter_config` drops what was on a removed output and bumps the graph |
| F3 | Configure a property | done | `UI/EntryPoints.swift` `PropertySetupSheet`, routed by `L/properties.rs` `property_setup_kind`. Color runs the helper over the selection; others apply then copy with `apply_to_selection`, Grid included (FIX). Intelligent Fixture stays with the fixture editor's spec: `PropertySetupKind` says so and Display Setup routes it. Run from Display Setup with Orientation |
| F4 | Run a setup helper | done | `UI/EntryPoints.swift` `SetupHelperSheet`; the fixed alphabetical list is `L/properties.rs` `SETUP_HELPERS`, through `context.setupHelperNames()`. The combo and button are Display Setup's. Run from it with Color Handling |
| F5 | Configure a layer | done | `UI/EntryPoints.swift` `LayerFilterSetupSheet`; `L/layers.rs` `layer_setup_kind`; `FFI/layers.rs` `layer_filter_kind`. The Layer Editor button is the sequencer's |
| F6 | Other routes to the helpers | done | Entry points are ready: `SetupHelperSheet(helper:nodes:model:)` for the sequencer's face mapping; `simple: true` for F238 after a template; `colorType:` for F151's preset; `silent: true` for the importer. The alert before F238, and calling these, belong to their callers |
| F7 | Serial Port Config window | done | `UI/SerialPortConfigSheet.swift`; `L/serial.rs` |
| F8 | Port scan on open | done | `UI/SerialPortConfigSheet.swift` `probe` off the main thread, never when muted; Refresh button; `L/serial.rs` `probe_serial_ports` skips Bluetooth and this app's own ports. Real probing not run: tests open no ports |
| F9 | Current Configured Port label | done | `UI/SerialPortConfigSheet.swift`; `L/serial.rs` `serial_config_model` |
| F10 | Port Details group | done | `UI/SerialPortConfigSheet.swift`, with "No serial ports found" and OK off |
| F11 | Default values | done | `L/serial.rs` `serial_config_model` |
| F12 | Port name | done | `L/serial.rs` `serial_config_model`: configured port preselected; absent, including `COM9`, selects nothing and requires a choice |
| F13 | Port name value rule | done | `L/serial.rs` `SerialPortChoice`: the path and the label are separate, so there is nothing to strip |
| F14 | Baud rate | done | `vixen_output::serial::STANDARD_BAUD_RATES`; a saved rate not in the list shows blank and fails validation |
| F15 | Parity label and list | done | `L/serial.rs` `parity_choices` |
| F16 | Mark and Space on macOS | done | Disabled with the help text; a saved one shows selected with a note. `L/serial.rs`, `UI/SerialPortConfigSheet.swift` |
| F17 | Data bits | done | Picker 5 to 8; Rust still validates imported data |
| F18 | Stop bits | done | `L/serial.rs` `stop_bits_choices` |
| F19 | Control order and region | done | `UI/SerialPortConfigSheet.swift` |
| F20 | OK validation | done | `L/serial.rs` `validate_serial_config`; alert text checked in the app |
| F21 | In-use warning | done | `UI/SerialPortConfigSheet.swift`, "Use Port" and "Choose Another" |
| F22 | OK result | done | `SerialPortConfigSheet` returns settings to its caller |
| F23 | Cancel | done | Checked in the app |
| F24 | Default and cancel keys | done | `UI/Common.swift` `SheetButtons` |
| F25 | Owner-drawn combos | omitted | Theme painting only, no behaviour. Native controls |
| F26 | Generic Serial window | done | `UI/ControllerSheets.swift` `GenericSerialSheet` |
| F27 | Serial Port Settings group | done | Same; opens F7 as a nested sheet |
| F28 | Settings text | done | `L/serial.rs` `serial_settings_label`, "Not Set" handled by the caller. `COM9: 9600, None, 8, One` checked in the app against the real show |
| F29 | Packet Data group | done | `UI/ControllerSheets.swift` |
| F30 | Header checkbox | done | Same |
| F31 | Header field | done | Same; `L/controllers.rs` `generic_serial_apply`. Unchecked stores empty. ASCII with `?`: `vixen_output::dotnet::ascii_bytes`, tested |
| F32 | Footer checkbox | done | Same |
| F33 | Footer field | done | Same |
| F34 | OK button | done | `generic_serial_apply`. Checked through the real show's XML: only `Header` and `Footer` changed, `COM9` stayed |
| F35 | Cancel button | done | Checked: files same as the baseline save |
| F36 | Open failure status | done | `L/controllers.rs` `controller_configuration_problem`, in `context.controllers()` as each row's `status`, for Display Setup to show. Retry in `vixen_output::serial::RetryTimer`, Stop cancels |
| F37 | Data sent, for reference | done | `O/modules/generic_serial.rs`, one writer per controller in the runner. FIX (performance hazard): `serial::frame_would_lag` skips a frame while more than one is queued, using the port's `bytes_to_write`, which is `TIOCOUTQ`. No real port was opened to confirm the timing |
| F38 | Dummy Lighting window | done | `UI/ControllerSheets.swift` `DummyLightingSheet`; reads from the data, needs no running controller |
| F39 | Window Title field | done | Same. Blank round-trips, tested |
| F40 | Render style group | done | Same |
| F41 | Radio monochrome | done | Same |
| F42 | Radio 3 inputs | done | Same |
| F43 | Radio every input | done | Same; XML checked in the app |
| F44 | OK | done | `L/controllers.rs` `dummy_apply`; `L/store.rs` `follow_changes` calls `graph.set_controller_config`, which rebuilds the policy without a restart. `dummy_data_policy` tested |
| F45 | Cancel | done | Checked |
| F46 | Output window on Start | done | `UI/ControllerMonitors.swift` `ControllerMonitorScene`, opened when Rust reports a monitor; closes on Stop; closing by hand leaves it running. Checked in the app |
| F47 | Output window grid | done | `L/controllers.rs` `dummy_grid_layout`, with `ceil` and reflow; `DummyMonitorNSView`. 3351 outputs checked |
| F48 | Box colours | done | `L/controllers.rs` `dummy_frame_rgba`, tested |
| F49 | FPS readout | done | `format_fps`, `fps_from_counts`, red text in a `CATextLayer`; 100 fps seen in the app |
| F50 | State reset | done | `O/modules/dummy.rs` on start |
| F51 | E1.31 window | done | `UI/E131Sheet.swift`; `FFI/e131.rs` `E131Session` holds the controller paused while open (`runner.set_paused`). Segmented control in place of tabs (see the end of this file) |
| F52 | Destination label | done | `.title3` |
| F53 | Destination picker | done | `L/e131.rs` `e131_destination_list`, including `Multicast UNKNOWN`, and the primary interface as default. Windows GUID path tested in Rust |
| F54 | Add unicast button | done | `E131Sheet`; ends the grid's edit first; `FFI/e131.rs` `add_unicast`. Checked in the app |
| F55 | Remove unicast button | done | `FFI/e131.rs` `remove_selected_unicast`, with both messages in Rust |
| F56 | New Unicast window | done | `UI/E131Sheet.swift` `UnicastSheet` |
| F57 | Radio IP Address | done | Same; `L/e131.rs` `unicast_ip_is_valid` |
| F58 | Radio Network Name | done | Same; `unicast_name_is_valid` |
| F59 | OK and Cancel on F56 | done | Same |
| F60 | Unicast resolution, for reference | done | `O/modules/e131.rs` `last_ipv4` |
| F61 | Universe grid | done | `UI/E131Sheet.swift` `UniverseTable`, `NSTableView`; rows selected where upstream selects cells |
| F62 | Start column | done | Read-only labels aren't validated, and automatic starts may pass 99999. `L/e131.rs` |
| F63 | On column | done | `UniverseTable`, `set_active` |
| F64 | Universe column | done | `L/e131.rs` validate cell, exact text tested |
| F65 | Size column | done | Same; alert checked in the app |
| F66 | Cell input and validation | done | `DigitsFormatter`, `NSSound.beep()`; alert "Cell Validation" with the edit staying open. Cancel always works; OK and F54 commit the open editor through `UniverseGridHandle`. Checked |
| F67 | Header click (sort) | done | `KeyTableHeader`, `L/e131.rs` sort, numeric. Rust tested; the header click itself wasn't driven, since a test window never takes a first click on a table header |
| F68 | Delete key | done | `KeyTable.onDelete`. The `.` quirk isn't ported. Key not driven in the app |
| F69 | Add universe button | done | `FFI/e131.rs` `add_universe`, limit message, computed manual start, row selected. Checked |
| F70 | Add multiple universes | done | `add_universes` in one pass, `NumberPromptSheet`. Checked |
| F71 | Delete universe button | done | On with a row selected and more than one row. Checked |
| F72 | Delete all universes | done | "Delete All", "Cancel". Checked |
| F73 | Strip icons | done | SF Symbols, `.help` and accessibility labels |
| F74 | Manually manage start values | done | `L/e131.rs` `E131Options`, `AutoPopulate` checked in XML |
| F75 | Display ALL warnings | done | `Warnings` checked in XML |
| F76 | Gather statistics | done | `Statistics` checked in XML |
| F77 | Transmit blind data | done | `Blind` checked in XML |
| F78 | sACN priority | done | `NumberField`, 0 to 200 |
| F79 | Max Repeat Count | done | `countRow`: two characters, beep and focus stay. OK also refuses, and returns to the box |
| F80 | Max Suppress Count | done | Same |
| F81 | Repeat and suppress, for reference | done | `O/modules/e131.rs` `send_this_frame`, with the spec's table as a test |
| F82 | Cancel | done | Checked |
| F83 | OK: channel count | done | `L/e131.rs` `e131_ok_steps`, "Save Anyway", "Go Back". Checked |
| F84 | OK: overlap | done | Same; "Review Settings", "Continue". FIX. Rust tested |
| F85 | OK: duplicate universes | done | Same. Checked |
| F86 | OK: other controllers | done | Same; `FFI/e131.rs` gathers running peers from the world. Rust tested |
| F87 | Order of checks | done | `E131Sheet.nextStep` |
| F88 | OK result | done | `L/e131.rs` `e131_apply`, `e131_output_names`; deprecated per-universe members written. Restarts only if running: `L/store.rs` `follow_changes`. Names checked in `SystemConfig.xml` |
| F89 | Output names on count change | done | `O/modules/e131.rs` `output_names`, one-based. FIX |
| F90 | Old-version import | done | `O/modules/e131.rs` `start`, once per run, posted as a `ControllerMessage`; guarded on empty |
| F91 | Startup warnings | done | Same; FIX: no IPv4 on the interface means that universe sends nothing and start isn't clean. Shown by `ControllerMonitorScene`'s panel. Not run: no such interface here |
| F92 | Session statistics | done | `O/modules/e131.rs` `stats_text`, per-update timing, total ms; queued, never blocks. Rust tested |
| F93 | Scrolling message box | done | `UI/Common.swift` `ScrollingMessageSheet` |
| F94 | Unreachable upstream | omitted | Not wired upstream: `OkButtonClick`, the empty row menu, `DestinationContextMenuStripOpening`, `mainMenu`, `mIHelp`, `AboutBox`, `Help.ShowSysClick`, and the empty handlers |
| F95 | DDP window | done | `UI/ControllerSheets.swift` `DdpSheet`; prior run state kept both ways through the world |
| F96 | Mode radios and field | done | `L/controllers.rs` `ddp_draft` |
| F97 | OK | done | `ddp_can_apply` takes dotted quads only; `ddp_apply`; resolves off the main thread with `ProgressView`. Muted apps stay off the network, so to them every name is unknown. One real lookup of a made-up name happened while testing, before that guard |
| F98 | Cancel | done | Checked |
| F99 | Resolve on connect, for reference | done | `O/modules/ddp.rs` |
| F100 | Renard: Serial Port Config | done | `L/controllers.rs` `renard_apply`, `serial_only_settings`; `FFI/controllers.rs`. Write timeout logged, frame dropped: `O/modules/renard.rs` |
| F101 | Renard SetupDialog | omitted | Never instantiated upstream |
| F102 | DMX Pro: Serial Port Config | done | `L/controllers.rs` `dmx_pro_apply`, built from the data (FIX) |
| F103 | Output limit, for reference | done | `O/modules/dmx_usb_pro.rs` |
| F104 | OpenDMX window | done | `UI/ControllerSheets.swift` `OpenDmxSheet` |
| F105 | Device combo | done | `O/modules/open_dmx.rs` `list_devices_from`, `L/controllers.rs` `open_dmx_selection`, `open_dmx_device_label` |
| F106 | OK | done | `open_dmx_apply`; restarts only if running; start failures posted as alerts by the module |
| F107 | Cancel | done | Checked |
| F108 | Start fallback, for reference | done | `O/modules/open_dmx.rs`: a writer thread over the system serial driver at 250000 8N2, with a break, and a restart resumes. Written this pass. No adapter here, so never run against one |
| F109 | Elexol window | done | `UI/ControllerSheets.swift` `ElexolSheet` |
| F110 | IP Address | done | `L/controllers.rs` `elexol_draft` |
| F111 | Port | done | Same |
| F112 | Test button | done | `FFI/controllers.rs` `test_elexol`, off the main thread, only on click, and refused when muted. Alert title "Test Connection". Not run against hardware; the muted path was clicked in the app |
| F113 | Minium 'ON' Intensity | done | `IntSlider` with ticks every 16; `elexol_draft` shows the saved value; `O/modules/elexol.rs` always assigns once configured |
| F114 | OK | done | `elexol_apply`, "Invalid IP Address.", "Invalid Port."; XML checked. Connects only if running |
| F115 | Cancel | done | Checked |
| F116 | RDS window | done | `UI/ControllerSheets.swift` `RdsSheet`, staged |
| F117 | Hardware radios | done | `L/controllers.rs` `rds_set_hardware`, `apply_radio`: the old radio's handler runs first, and `ConnectionMode` follows upstream, including the forced 0 at open with HTTP(S) or V-FMT212R saved. A saved MRDS192 fires no handler, since its radio is already the checked one, so its `ConnectionMode` stays as saved (fixed in review, tested) |
| F118 | Field rules per hardware | done | `rds_field_rules`, run at open (FIX). Checked for V-FMT212R in the app |
| F119 | Serial Port | done | Path in `PortName`, restored from it; `PortNumber` round-trips |
| F120 | URL | done | `RdsSheet` |
| F121 | Macro hint | done | Same |
| F122 | Requires Authentication | done | Same |
| F123 | UserName and Password | done | `SecureField` for the password; limit of 8 |
| F124 | Test Interface group | done | `FFI/controllers.rs` `send_rds_test`, on the draft, real result; refused when muted. Muted path clicked in the app |
| F125 | Status strip | done | `RdsSheet` |
| F126 | Picture | done | `app/Resources/dialogs/rds-logo.png`, tinted, since its black ink would disappear on a dark sheet |
| F127 | Options group | done | `RdsSheet` |
| F128 | Hidden group | done | Not shown; `L/controllers.rs` `rds_apply` leaves `HideLaunchedWindows` alone |
| F129 | Save | done | Checked in XML |
| F130 | Runtime, for reference | done | `O/modules/rds.rs` and system-libcurl HTTP(S) bridge. All 301 output tests pass, including private TLS trust/hostname checks, Basic/Digest, credential-safe redirects, cookies, bounded responses and Stop/join/restart. Real encoders remain untested. See `docs/review/rds-https.md` |
| F131 | Launcher window | done | `UI/ControllerSheets.swift` `LauncherSheet`, staged, Save and Cancel |
| F132 | Hide Launched Windows | done | Same; XML checked |
| F133 | Launch, for reference | done | `O/modules/launcher.rs` |
| F134 | Debug window | done | `UI/ControllerMonitors.swift` `DebugTextView`: read-only `NSTextView`, refreshed by a display link; `O/modules/debug.rs` `format_commands`. Checked |
| F135 | Color Breakdown window | done | `UI/FilterSheets.swift` `ColorBreakdownSheet` |
| F136 | Instructions | done | Same, as one wrapped text |
| F137 | Template picker | done | `L/filters.rs` `COLOR_BREAKDOWN_TEMPLATES` |
| F138 | Apply Template | done | `L/filters.rs`, table tested; applied in the app |
| F139 | Colour list | done | `ScrollView` of rows |
| F140 | Row colour swatch | done | `ArgbColorPicker`; `color_for_item` stores pure colours as known ones (FIX). Picking through the system panel not driven |
| F141 | Row name | done | XML checked |
| F142 | Row Delete | done | XML checked; the removed output's patch lost its source |
| F143 | Add Color | done | XML checked: White, "New Color" |
| F144 | Mix checkbox | done | Staged; `color_breakdown_mix_note`. XML checked |
| F145 | 16 Bit checkbox | done | Staged. XML checked |
| F146 | OK | done | `color_breakdown_apply`. A no-op OK on the real filter left both files byte-identical to the baseline save |
| F147 | Cancel | done | Checked |
| F148 | Coarse Fine window | done | `UI/FilterSheets.swift` `CoarseFineSheet` |
| F149 | Fields | done | `IntSlider` with its tooltip; `coarse_fine_issues`. XML checked |
| F150 | Apply rules | done | `applyLive`, which swaps atomically through `graph.set_filter_config`. Cancel restores: checked in XML. Each slider change goes through the world |
| F151 | Color Configuration window | done | `UI/PropertySheets.swift` `ColorHelperSheet` |
| F152 | Radio single | done | Same; OK off until one is chosen |
| F153 | Single swatch | done | `ArgbColorPicker`, RoyalBlue |
| F154 | Radio multiple | done | Same |
| F155 | Colors picker | done | `L/color.rs` `color_helper_model`: a set that's gone stays on the first; no name selects nothing, and OK stays off |
| F156 | Edit Colors | done | Reload keeps the picked set when it still exists |
| F157 | Radio full | done | Same |
| F158 | Color Order picker | done | `L/color.rs` `COLOR_ORDERS`, opens on RGB as upstream |
| F159 | OK: properties | done | `L/color.rs` `color_helper_apply`, "Overwrite", "Skip"; real show checked |
| F160 | OK: filters | done | Same, `L/flow.rs` `leaf_outputs_or_filters`; off the main thread with a progress indicator |
| F161 | Result report | done | `color_report_text`; text checked in the app. Silent mode: `silent: true`, with the overwrite questions kept; not run in the app |
| F162 | Color Sets window | done | `UI/PropertySheets.swift` `ColorSetsSheet` |
| F163 | Defined list | done | Remove off until a set is selected. Switching drops edits, as upstream |
| F164 | Add set | done | `L/color.rs` `color_set_add`; `TextPromptSheet`, OK off when empty |
| F165 | Remove set | done | `color_set_remove`; sets resolve by name |
| F166 | Name field | done | `color_set_button_title` |
| F167 | Colours grid | done | Scrolling grid of `ArgbColorPicker`s; add button bottom right. No removing a colour, as upstream |
| F168 | Update or Make New button | done | `color_set_save`, "Name Requred" |
| F169 | Closing | done | "Save", "Don't Save", "Cancel". Checked; writes hold even when the caller cancels |
| F170 | Patching Order window | done | `UI/OrderSheet.swift`; `FFI/elements.rs` `OrderSession` |
| F171 | Order list | done | `L/order.rs` `order_helper_rows`, `NSTableView` |
| F172 | Drag to reorder | done | `UI/OrderSheet.swift` `OrderTable` row drag, `order_move`. Rust tested; a drag wasn't driven |
| F173 | Select all | done | `NSTableView`'s ⌘A |
| F174 | Context menu | done | Two or more selected. Checked |
| F175 | Reverse | done | `order_reverse`. XML checked |
| F176 | Zig Zag | done | `order_zigzag`, `order_zigzag_bounds`, `NumberPromptSheet`. Prompt checked; upstream's table tested |
| F177 | OK | done | `order_helper_apply`. XML checked |
| F178 | Order Setup window | done | `UI/PropertySheets.swift` `OrderValueSheet`, to Int32.max. XML checked |
| F179 | Orientation window | done | `OrientationSheet`. XML checked |
| F180 | Helper behaviour | done | `L/properties.rs` `orientation_helper_apply`, on the nodes themselves. Opens on Vertical: checked |
| F181 | Location window | done | `LocationSheet`. XML checked |
| F182 | Fields | done | `location_values`, `clamp_location` |
| F183 | Grid window | done | `GridSheet`, correct at open, values clamped. XML checked |
| F184 | Face window | done | `UI/FaceSheet.swift`. Segmented control in place of tabs |
| F185 | Mouth tab | done | `FaceTable`; `FaceHeaderCell` draws 48 x 48 above the text; 50 wide, Color 90, name to its content. PNGs in `app/Resources/dialogs/phonemes` |
| F186 | Outlines/Eyes tab | done | Same |
| F187 | Check a cell | done | `L/face.rs` `face_toggle`; click checked, ⌘ and ⇧ in `FaceTable` |
| F188 | Color cell | done | Double-click checked; by node id; `DiscreteColorSheet` |
| F189 | Colour validation | done | `L/face.rs` `face_helper_model`, `face_helper_apply` |
| F190 | OK | done | `face_helper_apply`: `Mouth` computed, node ids; XML checked. The item name `KeyValueOfFaceComponentbooleanIiw0dy87` is computed from .NET's rule, which reproduces the three names in the real files; no Windows Face sample exists to check against |
| F191 | Cancel | done | Same path as the others |
| F192 | Sort and row headers | done | None, no new-row line |
| F193 | State Mapping entry | done | `L/state.rs` `state_helper_begin`, `state_helper_commit`; `UI/EntryPoints.swift` shows the alert |
| F194 | State Mapper window | done | `UI/StateMapperSheet.swift`; `FFI/state.rs` `StateSession`. Checked |
| F195 | Definition picker | done | Snaps back on blocking errors, `StateDraft` |
| F196 | Add, Delete, ... menu | done | Add checked; rest Rust tested |
| F197 | Description | done | `TextEditor` |
| F198 | Preview toggle | done | `L/state_preview.rs`, muted in tests; toggle clicked in the app |
| F199 | State Items grid | done | `Table`; header sort becomes the stored order |
| F200 | Color cell | done | Hex on colour; discrete or free picker |
| F201 | Add | done | `state_common_discrete_color`. Checked |
| F202 | Copy | done | Checked, `State Item - 1 - 1` in XML |
| F203 | Remove | done | Texts in `StateMapperSheet`; Rust tested |
| F204 | Move up and down | done | Arrows with the tooltips |
| F205 | Assigned Elements pane | done | `NSOutlineView`; group disables below it; Count of 16 checked; stored ids in XML |
| F206 | Name prompt | done | `TextPromptSheet`. Checked |
| F207 | Name prompt validation | done | `L/state.rs` `state_normalize_name`, `state_name_warning`, under the field |
| F208 | Mapper validation | done | `StateDraft::issues` |
| F209 | OK | done | `state_helper_commit`, stable ids. XML checked |
| F210 | Cancel | done | Draft discarded; the property attaches only on OK. Tested |
| F211 | Naming rule | done | `state_next_name` |
| F212 | Chroma Key window | done | `UI/LayerSheets.swift` `ChromaKeySheet`; `L/layers.rs` |
| F213 | Color | done | `ArgbColorPicker`, tooltip; `chroma_key_from_dialog` derives saturation and hue |
| F214 | Hue tolerance | done | `IntSlider` 0 to 180, ticks 20 |
| F215 | Saturation tolerance | done | `IntSlider` 0 to 99, ticks 10; correct slider (FIX) |
| F216 | Brightness Range | done | `BrightnessRows`, `clamp_limits`, `limit_to_percent`; snapping checked in the app. OK writes both limits as shown / 100 even when no slider moved, so a stored 0.29 comes back 0.28 as upstream (fixed in review: `L/layers.rs` `chroma_key_apply`, tested) |
| F217 | Zero brightness checkbox | done | With the tooltip; the flag comes through (FIX) |
| F218 | OK and Cancel | done | Checked. Reaching the sequence's data goes through `FFI/layers.rs` `apply_chroma_key`, tested at the logic level only: no sequence was opened |
| F219 | Luma Key window | done | `LumaKeySheet`, no tooltips. OK writes both limits as shown / 100: `L/layers.rs` `luma_key_apply` (fixed in review, tested) |
| F220 | Mask And Fill window | done | `MaskFillSheet` |
| F221 | Exclude zero values | done | Same |
| F222 | Require Mixing Partner | done | Same; both flags copied |
| F223 | OK and Cancel | done | Same |
| F224 | Color Change | done | `L/layers.rs` `layer_setup_kind` says no setup |
| F225 | Highest Value | done | Same |
| F226 | Intensity Overlay | done | Same |
| F227 | Mask | done | Same |
| F228 | Multiply Color | done | Same |
| F229 | Proportional Mix | done | Same |
| F230 | Tagged Filter window | done | `UI/FilterSheets.swift` `TagFilterSheet`. XML checked |
| F231 | Tag field | done | `tag_filter_can_apply`; display name follows, and the graph bumps so Display Setup can refresh the title |
| F232 | Dimming Filter window | done | Same. XML checked |
| F233 | Color Wheel Filter window | done | Same. XML checked; `ColorWheelData` round-trips |
| F234 | Shutter Filter window | done | Same; OK off for 300: checked |
| F235 | Shutter value rules | done | `validate_index_value`; values on only when checked; `.help("0-255")` |
| F236 | Prism Filter window | done | Same; both values saved (FIX). XML checked |
| F237 | Dimming Curve Configure | done | `DimmingCurveFilterSheet` opens VixenKit's Curve Editor through a new public `CurveEditorView`; `L/filters.rs` `dimming_curve_apply`. XML checked after Invert Curve |
| F238 | Dimming Curve window | done | `UI/PropertySheets.swift` `DimmingHelperSheet` |
| F239 | Instructions | done | Same |
| F240 | Setup Dimming Curve button | done | Same; OK starts off, then on. Checked |
| F241 | Existing filter question | done | Same |
| F242 | Radios | done | Same; simple mode disables all but Insert |
| F243 | Help | done | `L/dimming.rs` `PATCHING_HELP_URL`. Not clicked in tests: it opens a browser |
| F244 | OK result | done | `L/dimming.rs` `dimming_helper_apply`, counts as upstream, own curve each. Real show: 16 created, 16 configured, patches added |
| F245 | Cancel | done | Checked |

## Choices made while building these

- **Module and file names.** The brief puts the area in `VixenDialogs`, `crates/vixen-ffi/src/dialogs.rs` and `crates/vixen-session/src/dialogs/`. The spec's `ModuleSetup` and `setup_dialogs.rs` name the same things.
- **Prompts.** `NumberPromptSheet`, `TextPromptSheet`, `ScrollingMessageSheet` and `DiscreteColorSheet` are in `UI/Common.swift`, public, instead of `VixenKit`.
- **Tabs.** E1.31 and Face use a segmented control. `TabView` in a sheet drew a broken bar here.
- **`ControllerMonitorScene`** is included by `VixenSetup/SetupScene.swift`, one guarded line added to that file, so the Dummy and Debug windows and the E1.31 messages appear in the whole app.
- **Colour sets' add button** adds a value white, where upstream adds the known `Color.White`. They look the same; the only visible difference is that an item made from it is named `ffffffff` instead of `White`.
- **Curve editor.** F237 and F240 use `VixenKit`'s Curve Editor through `CurveEditorView`, a small public view added to `CurveEditorDialog.swift`.
- **Grid, Face and State stay `PropertyData::Unknown`.** Their XML is edited in `L/`, keeping unknown members, without changing `vixen-core`.
- **Open DMX runtime.** The spec lists transmit "for reference". It had reported unsupported, so a writer over the system serial driver was added.
- **Performance hazards taken up.** Generic Serial skips frames while more than one is queued; Elexol connects once; a controller's restart from a dialog on the open profile runs off the caller's thread.
- **`VIXEN_MUTE`.** A muted app refuses Elexol Test, RDS Send and DDP's name lookup in Rust, and doesn't probe ports. `UITestHarness` gained `popup` and `editCell` steps for driving these sheets.

## Not run

- Any real serial port, network send or DNS lookup in a passing test, by the brief. Choosing a port in the picker: this Mac lists none once `cu.Bluetooth-Incoming-Port` and `cu.debug-console` are hidden, so the path through a chosen port is tested in Rust only.
- A click on a table header, a drag in the Patching Order list, the Delete key in the grid, and the system colour panel: a test window never takes these. Their logic is tested in Rust.
- Inside Display Setup, Generic Serial, the Orientation property and the Color Handling helper were run in the combined build through the window's OK to disk, and the window's Cancel left both files byte-identical. The other dialogs were run from the test window. A filter's double-click in graphical patching wasn't driven; `L/setup_world.rs` tests filters on the working copy.
- The Dummy and Debug windows were run from the test window, which starts those two controllers whatever the mute says. From Display Setup's Start button they haven't run, since a muted app starts nothing.
- Layer dialogs writing into an open sequence: they ran from the test window, which returns their result to a closure.

## Controls in `docs/reference/forms`

Layout-only entries (`AutoScaleDimensions`, `ClientSize`, `Margin`, `Padding`, `MinimumSize`, `MaximumSize`, `Size`, `components`, grid cell styles) are left out: sheets size to content.

### Controls/MessageBoxForm (Select more marks)

Lives in: `.dialogAlert` in `UI/Common.swift`, with verbs on the buttons.

| Control | Type | Text | Status |
|---|---|---|---|
| buttonOk | Button | OK | done |
| buttonCancel | Button | CANCEL | done |
| buttonNo | Button | NO | done |
| messageIcon | PictureBox |  | omitted: the system alert has its own icon |
| flowLayoutPanel1 | FlowLayoutPanel |  | done |
| tableLayoutPanel1 | TableLayoutPanel |  | done |
| txtMessage | RichTextBox |  | done: alert text |

### Controls/NumberDialog

Lives in: `UI/Common.swift` `NumberPromptSheet`.

| Control | Type | Text | Status |
|---|---|---|---|
| button2 | Button | Cancel | done |
| button1 | Button | OK | done |
| labelPrompt | Label | [] | done |
| numericUpDownChooser | NumericUpDown |  | done |

### Controls/SerialPortConfig (Serial Port Config)

Lives in: `UI/SerialPortConfigSheet.swift`.

| Control | Type | Text | Status |
|---|---|---|---|
| groupBox | GroupBox | Port Details | done |
| comboBoxBaudRate | ComboBox |  | done |
| comboBoxStopBits | ComboBox |  | done |
| comboBoxParity | ComboBox |  | done |
| label2 | Label | Baud rate | done |
| label5 | Label | Parity | done |
| label3 | Label | Data bits | done |
| label4 | Label | Stop bits | done |
| textBoxDataBits | TextBox |  | done: a picker of 5 to 8 (F17) |
| label1 | Label | Port name | done |
| comboBoxPortName | ComboBox |  | done |
| buttonOK | Button | OK | done |
| buttonCancel | Button | Cancel | done |
| currentConfiguredPortLabel | Label | Current Configured Port: | done |
| configuredPortValueLabel | Label | portname | done: the value beside the label |

### Controls/TextDialog

Lives in: `UI/Common.swift` `TextPromptSheet`.

| Control | Type | Text | Status |
|---|---|---|---|
| labelPrompt | Label | [] | done |
| textBoxResponse | TextBox |  | done |
| buttonOk | Button | OK | done |
| buttonCancel | Button | Cancel | done |

### Controller/DDP/DDPSetup (DDP Configuration)

Lives in: `UI/ControllerSheets.swift` `DdpSheet`.

| Control | Type | Text | Status |
|---|---|---|---|
| buttonCancel | Button | Cancel | done |
| buttonOK | Button | OK | done |
| groupBox1 | GroupBox | Configuration | done |
| radioButtonIPAddress | RadioButton | IP Address | done |
| radioButtonHostName | RadioButton | Host Name | done |
| label1 | Label | IP Address: | done |
| textBoxIPAddress | TextBox |  | done |
| textBoxHostName | TextBox | myhost.local | done |

### Controller/DebugController/DebugControllerOutputForm (Debug Controller Output)

Lives in: `UI/ControllerMonitors.swift` `DebugMonitorView`.

| Control | Type | Text | Status |
|---|---|---|---|
| textBoxOutput | TextBox |  | done |
| chkVerbose | CheckBox | Verbose | done |

### Controller/DummyLighting/DummyLightingOutputForm (Dummy Lighting Output)

Lives in: `UI/ControllerMonitors.swift` `DummyMonitorNSView`.

### Controller/DummyLighting/DummyLightingSetup (Dummy Lighting Configuration)

Lives in: `UI/ControllerSheets.swift` `DummyLightingSheet`.

| Control | Type | Text | Status |
|---|---|---|---|
| buttonOK | Button | OK | done |
| buttonCancel | Button | Cancel | done |
| radioButtonMonochrome | RadioButton | Set up as monochrome channels | done |
| radioButtonMultiRGB | RadioButton |  | done |
| radioButtonSingleRGB | RadioButton | Set up as RGB channels, with every input / mapping to a f... | done |
| textBoxWindowTitle | TextBox |  | done |
| label1 | Label | Window Title: | done |

### Controller/E131/AboutBox (AboutBox)

Lives in: omitted: unreachable upstream (F94).

| Control | Type | Text | Status |
|---|---|---|---|
| tableLayoutPanel | TableLayoutPanel |  | omitted: see above |
| logoPictureBox | PictureBox |  | omitted: see above |
| labelProductName | Label | Product Name | omitted: see above |
| labelVersion | Label | Version | omitted: see above |
| textBoxDescription | TextBox | Description | omitted: see above |
| okButton | Button | &OK | omitted: see above |

### Controller/E131/SetupForm (Streaming ACN (E1.31) Configuration)

Lives in: `UI/E131Sheet.swift`.

| Control | Type | Text | Status |
|---|---|---|---|
| rowManipulationContextMenuStrip | ContextMenuStrip |  | omitted: has no items upstream (F94) |
| mIHelp | ToolStripMenuItem | &Help | omitted: never added to the form (F94) |
| mainMenu | MenuStrip |  | omitted: never added to the form (F94) |
| warningsCheckBox | CheckBox | Display ALL Warnings/Errors and wait For OK | done |
| statisticsCheckBox | CheckBox | Gather statistics and display at end of session | done |
| eventRepeatCountTextBox | TextBox | 0 | done |
| label | Label | Max Repeat Count /  / Set to 0 to send all events (even 0... | done |
| okButton | Button | OK | done |
| cancelButton | Button | Cancel | done |
| eventSuppressCountTextBox | TextBox | 0 | done |
| label1 | Label | Max Suppress Count /  / Only used if Max Repeat Count / i... | done |
| autoPopulateStart | CheckBox | Manually manage start values. | done |
| btnDeleteUnicast | Button |  | done |
| btnAddUnicast | Button |  | done |
| btnDeleteUniverse | Button |  | done |
| btnAddMultipleUniverses | Button |  | done |
| btnAddUniverse | Button |  | done |
| lblDestination | Label | Destination | done |
| comboDestination | ComboBox |  | done |
| lblPriority | Label | sACN priority. | done |
| numericPriority | NumericUpDown |  | done |
| chkBoxTransmitBlind | CheckBox | Transmit blind data. | done |
| tabControlEX1 | TabControl |  | done: segmented control |
| tabPageEX1 | TabPage | Universes | done: segment "Universes" |
| btnDeleteAllUniverses | Button |  | done |
| univDGVN | DataGridViewNumbered |  | done |
| startColumn | DataGridViewTextBoxColumn |  | done |
| activeColumn | DataGridViewCheckBoxColumn |  | done |
| universeColumn | DataGridViewTextBoxColumn |  | done |
| sizeColumn | DataGridViewTextBoxColumn |  | done |
| tabPageEX2 | TabPage | Advanced Options | done: segment "Advanced Options" |

### Controller/E131/UnicastForm (New Unicast Destination)

Lives in: `UI/E131Sheet.swift` `UnicastSheet`.

| Control | Type | Text | Status |
|---|---|---|---|
| okButton | Button | &OK | done |
| cancelButton | Button | &Cancel | done |
| ipRadio | RadioButton | IP Address | done |
| networkNameRadio | RadioButton | Network Name | done |
| networkNameTextBox | TextBox | myComputer | done |
| ipTextBox | IpTextBox | 127.0.0.1 | done: a validated text field in place of `SysIPAddress32` |

### Controller/ElexolEtherIO/SetupDialog (Elexol USB I/O 24 Configuration)

Lives in: `UI/ControllerSheets.swift` `ElexolSheet`.

| Control | Type | Text | Status |
|---|---|---|---|
| ethernetSettingsGroupBox | GroupBox | Ethernet Settings | done |
| portTextBox | TextBox | 2424 | done |
| portLabel | Label | Port | done |
| testButton | Button | Test | done |
| ipAddressTextBox | TextBox | 10.10.10.10 | done |
| ipaddressLabel | Label | IP Address | done |
| groupBox1 | GroupBox | Advanced | done |
| minOnIntensityLabel | Label | Minium 'ON' Intensity (0-255): | done |
| minIntensityLabel | Label | 1 | done |
| sliderMinIntensityTrackBar | TrackBar |  | done |
| okButton | Button | OK | done |
| cancelButton | Button | Cancel | done |

### Controller/GenericSerial/SetupDialog (Generic Serial Configuration)

Lives in: `UI/ControllerSheets.swift` `GenericSerialSheet`.

| Control | Type | Text | Status |
|---|---|---|---|
| btnCancel | Button | Cancel | done |
| btnOkay | Button | OK | done |
| gbPacketData | GroupBox | Packet Data | done |
| tbFooter | TextBox |  | done |
| cbFooter | CheckBox | Send a text footer | done |
| tbHeader | TextBox |  | done |
| cbHeader | CheckBox | Send a text header | done |
| btnPortSetup | Button | Setup/Change Serial Port | done |
| groupBox1 | GroupBox | Serial Port Settings | done |
| lblSettings | Label | COM1: 115200, Space, 8, OnePointFive | done |
| lblSettingsLbl | Label | Currently: | done |

### Controller/LauncherController/SetupForm (Launcher Controller Configuration)

Lives in: `UI/ControllerSheets.swift` `LauncherSheet`.

| Control | Type | Text | Status |
|---|---|---|---|
| button1 | Button | Save | done |
| chkHideLaunchedWindows | CheckBox | Hide Launched Windows | done |

### Controller/OpenDMX/SetupDialog (OpenDMX Setup)

Lives in: `UI/ControllerSheets.swift` `OpenDmxSheet`.

| Control | Type | Text | Status |
|---|---|---|---|
| panel1 | Panel |  | layout only |
| buttonOK | Button | OK | done |
| buttonCancel | Button | Cancel | done |
| cmbDeviceList | ComboBox |  | done |

### Controller/RDSController/SetupForm (Launcher Commands and RDS Configuration)

Lives in: `UI/ControllerSheets.swift` `RdsSheet`.

| Control | Type | Text | Status |
|---|---|---|---|
| groupBox3 | GroupBox | Options | done |
| chkSlow | CheckBox | Slow | done |
| chkBiDirectional | CheckBox | BiDirectional | done |
| groupBox4 | GroupBox | RDS Hardware | done |
| label1 | Label | Serial Port | done |
| cboPortName | ComboBox |  | done |
| chkRequiresAuthentication | CheckBox | HTTP(s) Requires Authentication | done |
| lblPassword | Label | Password | done |
| lblUserName | Label | UserName | done |
| txtHttpPassword | TextBox |  | done: `SecureField` (F123) |
| txtHttpUsername | TextBox |  | done |
| txtUrl | TextBox |  | done |
| label3 | Label | {text}, {time} | done |
| label2 | Label | Macro Fields: | done |
| lblUrl | Label | URL | done |
| radioHttp | RadioButton | HTTP(S) | done |
| radioVFMT212R | RadioButton | V-FMT212R | done |
| radioMRDS1322 | RadioButton | MRDS1322 | done |
| radioMRDS192 | RadioButton | MRDS192 | done |
| groupBox2 | GroupBox | Test Interface | done |
| btnTX | Button | Send | done |
| txtPSInterface | TextBox |  | done |
| pictureBox1 | PictureBox |  | done |
| statusStrip1 | StatusStrip | statusStrip1 | done: the strip along the bottom |
| StatusLbl1 | ToolStripStatusLabel | DIYC | done |
| button1 | Button | Save | done |
| groupBox1 | GroupBox | Launcher Options | omitted: hidden upstream (F128); the data is kept |
| chkHideLaunchedWindows | CheckBox | Hide Launched Windows | omitted: hidden upstream (F128); the data is kept |
| label4 | Label | Macro value {text} | done |

### Controller/Renard/SetupDialog (Renard Configuration)

Lives in: omitted: never instantiated upstream (F101). Renard opens `SerialPortConfigSheet` directly.

| Control | Type | Text | Status |
|---|---|---|---|
| groupBox1 | GroupBox | Setup | omitted: see above |
| buttonPortSetup | Button | Port Setup | omitted: see above |
| comboBoxProtocolVersion | ComboBox |  | omitted: see above |
| label1 | Label | Protocol version: | omitted: see above |
| buttonOK | Button | OK | omitted: see above |
| buttonCancel | Button | Cancel | omitted: see above |

### LayerMixingFilter/ChromaKey/ChromaKeySetup (Chroma Key Configuration)

Lives in: `UI/LayerSheets.swift` `ChromaKeySheet`.

| Control | Type | Text | Status |
|---|---|---|---|
| btnOk | Button | OK | done |
| btnCancel | Button | Cancel | done |
| numLowerLimit | NumericTextBox | 0 | done |
| numUpperLimit | NumericTextBox | 0 | done |
| trkLowerLimit | TrackBar |  | done: `IntSlider` |
| trkUpperLimit | TrackBar |  | done: `IntSlider` |
| label1 | Label | Lower | done |
| label2 | Label | Upper | done |
| label3 | Label | Brightness Range | done |
| label4 | Label | Color | done |
| colorPanel1 | ColorPanel |  | done: `ArgbColorPicker` |
| label5 | Label | Tolerance | done |
| trkHueTolerance | TrackBar |  | done: `IntSlider` |
| trkSaturationTolerance | TrackBar |  | done: `IntSlider` |
| label6 | Label | Hue | done |
| label7 | Label | Saturation | done |
| toolTip | ToolTip |  | done: `.help` |
| chkTransparentOnZeroBrightness | CheckBox | Treat Zero Brightness as Transparent. | done |

### LayerMixingFilter/LumaKey/LumaKeySetup (Luma Key Configuration)

Lives in: `UI/LayerSheets.swift` `LumaKeySheet`.

| Control | Type | Text | Status |
|---|---|---|---|
| btnOk | Button | OK | done |
| btnCancel | Button | Cancel | done |
| numLowerLimit | NumericTextBox | 0 | done |
| numUpperLimit | NumericTextBox | 0 | done |
| trkLowerLimit | TrackBar |  | done: `IntSlider` |
| trkUpperLimit | TrackBar |  | done: `IntSlider` |
| label1 | Label | Lower | done |
| label2 | Label | Upper | done |
| label3 | Label | Brightness Limits | done |

### LayerMixingFilter/MaskFill/MaskAndFillSetup (Mask And Fill Configuration)

Lives in: `UI/LayerSheets.swift` `MaskFillSheet`.

| Control | Type | Text | Status |
|---|---|---|---|
| chkExcludeZero | CheckBox | Exclude zero values | done |
| btnOk | Button | OK | done |
| btnCancel | Button | Cancel | done |
| chkRequireMixingPartner | CheckBox | Require Mixing Partner | done |

### OutputFilter/ColorBreakdown/ColorBreakdownItemControl

Lives in: A row of `UI/FilterSheets.swift` `ColorBreakdownSheet`.

| Control | Type | Text | Status |
|---|---|---|---|
| panelColor | Panel |  | done: `ArgbColorPicker` |
| textBoxName | TextBox | New Color | done |
| label1 | Label | Name: | done |
| buttonDelete | Button | Delete | done |

### OutputFilter/ColorBreakdown/ColorBreakdownSetup (Color Breakdown Filter Setup)

Lives in: `UI/FilterSheets.swift` `ColorBreakdownSheet`.

| Control | Type | Text | Status |
|---|---|---|---|
| label1 | Label | Select a pre-configured template from the drop-down box, ... | done: instructions, as one wrapped text |
| label2 | Label | custom color breakdown filter below. | done: instructions, as one wrapped text |
| label3 | Label | Template: | done |
| comboBoxTemplates | ComboBox |  | done |
| buttonApplyTemplate | Button | Apply Template | done |
| buttonCancel | Button | Cancel | done |
| buttonOK | Button | OK | done |
| buttonAddColor | Button | Add Color | done |
| tableLayoutPanelControls | TableLayoutPanel |  | done: `ScrollView` of rows |
| checkBoxMixColors | CheckBox | These colors should be mixed to produce the input color | done |
| checkBox16Bit | CheckBox | 16 Bit Output (For Use with Course Fine Break Down) | done |

### OutputFilter/ColorWheelFilter/ColorWheelFilterSetup (Color Wheel Filter Setup)

Lives in: `UI/FilterSheets.swift` `TagFilterSheet`.

| Control | Type | Text | Status |
|---|---|---|---|
| flowLayoutPanel1 | FlowLayoutPanel |  | layout only |
| buttonCancel | Button | Cancel | done |
| buttonOk | Button | OK | done |
| label1 | Label | Tag: | done |
| textBoxTag | TextBox |  | done |
| checkBoxConvert | CheckBox | Automatically Control Color Wheel | done |

### OutputFilter/DimmingCurve/DimmingCurveHelper (Dimming Curve Configuration)

Lives in: `UI/PropertySheets.swift` `DimmingHelperSheet`.

| Control | Type | Text | Status |
|---|---|---|---|
| label1 | Label | This will set up a dimming curve for each selected element. | done |
| label2 | Label | Use the button below to configure the dimming curve for t... | done |
| buttonSetupCurve | Button | Setup Dimming Curve | done |
| radioButtonExistingDoNothing | RadioButton | Leave the existing dimming curve alone. | done |
| lblQuestion | Label | If some elements are already patched to a dimming curve, ... | done |
| radioButtonExistingUpdate | RadioButton | Update the existing dimming curve with the new data. | done |
| radioButtonExistingAddNew | RadioButton | Add a new dimming curve anyway (not recommended). | done |
| buttonHelp | Button | Help | done |
| buttonCancel | Button | Cancel | done |
| buttonOk | Button | OK | done |
| label4 | Label | Hint: Use a library curve to make editing easier. | done |
| radioButtonInsertAfter | RadioButton | Insert a new dimming curve after the element. | done |

### OutputFilter/DimmingFilter/DimmingFilterSetup (Dimming Filter Setup)

Lives in: `UI/FilterSheets.swift` `TagFilterSheet`.

| Control | Type | Text | Status |
|---|---|---|---|
| flowLayoutPanel1 | FlowLayoutPanel |  | layout only |
| buttonCancel | Button | Cancel | done |
| buttonOk | Button | OK | done |
| label1 | Label | Tag: | done |
| textBoxTag | TextBox |  | done |
| checkBoxConvert | CheckBox | Automatically Control Dimmer | done |

### OutputFilter/PrismFilter/PrismFilterSetup (Prism Filter Setup)

Lives in: `UI/FilterSheets.swift` `TagFilterSheet`.

| Control | Type | Text | Status |
|---|---|---|---|
| flowLayoutPanel1 | FlowLayoutPanel |  | layout only |
| buttonCancel | Button | Cancel | done |
| buttonOk | Button | OK | done |
| label1 | Label | Tag: | done |
| textBoxTag | TextBox |  | done |
| checkBoxConvert | CheckBox | Automatically Open and Close Prism | done |
| label2 | Label | Open Prism Value: | done |
| textOpenPrism | TextBox |  | done |
| label3 | Label | Close Prism Value: | done |
| textClosePrism | TextBox |  | done |
| label4 | Label | Associated Prism Function Name: | done |
| textBoxAssociatedFunctionName | TextBox |  | done |

### OutputFilter/ShutterFilter/ShutterFilterSetup (Shutter Filter Setup)

Lives in: `UI/FilterSheets.swift` `TagFilterSheet`.

| Control | Type | Text | Status |
|---|---|---|---|
| flowLayoutPanel1 | FlowLayoutPanel |  | layout only |
| buttonCancel | Button | Cancel | done |
| buttonOk | Button | OK | done |
| label1 | Label | Tag: | done |
| textBoxTag | TextBox |  | done |
| checkBoxConvert | CheckBox | Automatically Open and Close Shutter | done |
| label2 | Label | Open Shutter Value: | done |
| textOpenShutter | TextBox |  | done |
| label3 | Label | Close Shutter Value: | done |
| textCloseShutter | TextBox |  | done |

### OutputFilter/TaggedFilter/TaggedFilterSetup (Tagged Filter Setup)

Lives in: `UI/FilterSheets.swift` `TagFilterSheet`.

| Control | Type | Text | Status |
|---|---|---|---|
| flowLayoutPanel1 | FlowLayoutPanel |  | layout only |
| buttonCancel | Button | Cancel | done |
| buttonOk | Button | OK | done |
| label1 | Label | Tag: | done |
| textBoxTag | TextBox |  | done |

### Property/Color/ColorPanel

Lives in: done: `UI/Common.swift` `ArgbColorPicker`, the system colour panel.

### Property/Color/ColorSetsSetupForm (Color Sets)

Lives in: `UI/PropertySheets.swift` `ColorSetsSheet`.

| Control | Type | Text | Status |
|---|---|---|---|
| listViewItem1 | ListViewItem |  | designer placeholder, never shown |
| listViewItem2 | ListViewItem |  | designer placeholder, never shown |
| listViewItem3 | ListViewItem |  | designer placeholder, never shown |
| listViewColorSets | ListView |  | done |
| label1 | Label | Defined Color Sets: | done |
| groupBoxColorSet | GroupBox | Selected Color Set | done |
| panelColorSet | Panel |  | layout only |
| buttonAddColor | Button |  | done |
| buttonUpdate | Button | Make New Color Set | done |
| tableLayoutPanelColors | TableLayoutPanel |  | done: a scrolling grid of `ArgbColorPicker`s |
| textBoxName | TextBox |  | done |
| label3 | Label | Colors: | done |
| label2 | Label | Name: | done |
| button1 | Button | OK | done |
| buttonRemoveColorSet | Button |  | done |
| buttonAddColorSet | Button |  | done |

### Property/Color/ColorSetupForm (Element Color)

Lives in: omitted: unreachable upstream, the only caller sends Color properties to the helper first.

| Control | Type | Text | Status |
|---|---|---|---|
| buttonOk | Button | OK | omitted: see above |
| groupBoxOptions | GroupBox | Element Color | omitted: see above |
| panel1 | Panel |  | omitted: see above |
| colorPanelSingleColor | ColorPanel |  | omitted: see above |
| buttonColorSetsSetup | Button | ...... | omitted: see above |
| comboBoxColorSet | ComboBox |  | omitted: see above |
| radioButtonOptionFullColor | RadioButton | Any color (ie. fully mixing RGB) | omitted: see above |
| radioButtonOptionMultiple | RadioButton | Multiple independent colors: | omitted: see above |
| radioButtonOptionSingle | RadioButton | Single color: | omitted: see above |

### Property/Color/ColorSetupHelper (Color Configuration)

Lives in: `UI/PropertySheets.swift` `ColorHelperSheet`.

| Control | Type | Text | Status |
|---|---|---|---|
| buttonCancel | Button | Cancel | done |
| buttonOk | Button | OK | done |
| label1 | Label | How do these items handle color? | done |
| buttonColorSetsSetup | Button | Edit Colors | done |
| comboBoxColorSet | ComboBox |  | done |
| radioButtonOptionFullColor | RadioButton | They can be any color: they are full RGB and mix to make ... | done |
| radioButtonOptionMultiple | RadioButton | They can be multiple colors, but the colors are independe... | done |
| radioButtonOptionSingle | RadioButton | They are a single color, and do not change color at all. ... | done |
| label2 | Label | (eg. multiple strings of different colors wrapped around ... | done |
| label3 | Label | Colors: | done |
| colorPanelSingleColor | ColorPanel |  | done: `ArgbColorPicker` |
| comboBoxColorOrder | ComboBox |  | done |
| label4 | Label | Color Order: | done |

### Property/Face/FaceSetupHelper (Face Setup)

Lives in: `UI/FaceSheet.swift`.

| Control | Type | Text | Status |
|---|---|---|---|
| buttonCancel | Button | Cancel | done |
| buttonOK | Button | OK | done |
| tableLayoutPanel1 | TableLayoutPanel |  | layout only |
| tabControl | TabControl |  | done: segmented control |
| tabMouth | TabPage | Mouth | done: segment "Mouth" |
| dataGridViewMouth | DataGridView |  | done: `FaceTable` |
| tabOther | TabPage | Outlines/Eyes | done: segment "Outlines/Eyes" |
| dataGridViewOther | DataGridView |  | done: `FaceTable` |

### Property/Grid/SetupForm (Grid Setup)

Lives in: `UI/PropertySheets.swift` `GridSheet`.

| Control | Type | Text | Status |
|---|---|---|---|
| label1 | Label | Width | done |
| nudWidth | NumericUpDown |  | done |
| nudHeight | NumericUpDown |  | done |
| label2 | Label | Height | done |
| buttonOK | Button | OK | done |
| buttonCancel | Button | Cancel | done |
| labelProduct | Label | label3 | done: the red message |

### Property/Location/SetupForm (Location Setup)

Lives in: `UI/PropertySheets.swift` `LocationSheet`.

| Control | Type | Text | Status |
|---|---|---|---|
| buttonCancel | Button | Cancel | done |
| buttonOK | Button | OK | done |
| numericUpDownXPosition | NumericUpDown |  | done |
| label1 | Label | X Position | done |
| label2 | Label | Y Position | done |
| numericUpDownYPosition | NumericUpDown |  | done |
| label3 | Label | Z Position | done |
| numericUpDownZPosition | NumericUpDown |  | done |

### Property/Order/OrderSetupHelper (Patching Order Preference)

Lives in: `UI/OrderSheet.swift`.

| Control | Type | Text | Status |
|---|---|---|---|
| tableLayoutPanel1 | TableLayoutPanel |  | layout only |
| btnOk | Button | OK | done |
| btnCancel | Button | Cancel | done |
| elementList | DragDropListView |  | done: `NSTableView` |
| columnOrder | ColumnHeader | Order | done |
| columnName | ColumnHeader | Name | done |

### Property/Order/SetupForm (Order Setup)

Lives in: `UI/PropertySheets.swift` `OrderValueSheet`.

| Control | Type | Text | Status |
|---|---|---|---|
| buttonCancel | Button | Cancel | done |
| buttonOK | Button | OK | done |
| numericUpDownXPosition | NumericUpDown |  | done |
| label1 | Label | Order | done |

### Property/Orientation/SetupForm (Orientation Setup)

Lives in: `UI/PropertySheets.swift` `OrientationSheet`.

| Control | Type | Text | Status |
|---|---|---|---|
| buttonCancel | Button | Cancel | done |
| buttonOK | Button | OK | done |
| label1 | Label | Orientation | done |
| comboBoxOrientation | ComboBox |  | done |
