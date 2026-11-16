# Sequencer window parity

> **Continuation (2026-11-11):** F12 now opens the Element Mapper and applies its result before loading the timeline; F89 opens the existing LipSync selector and Face Setup. Map/save/reopen, mapper Cancel and Leave Unmapped passed in the combined app; LipSync recursive selection, Reset and Face Setup cancellation passed. The older handoffs below describe the November 9 state. Audio rate/tempo and tag-manager presentation are now verified; remaining library gesture verification is listed in F169–170/F346–347/F356–357. See `docs/VERIFICATION.md` and `target/resume-ui/`.

> **Pass 3 (2026-11-09):** the LipSync menu is in, on the marks worker's views: Default Image Map (F87), Edit Image Maps and its toolbar button (F88, F146), Lyric Converter and its paste (F90, F301), and the dictionary side of Breakdown Word (F299). New: `UI/SequencerLipSync.swift`; FFI `paste_at_time`, `cursor_ticks`, `breakdown_clean_word`, `lipsync_maps`, `set_default_lipsync_map`, and the Layers import now applies the plan it showed. New hooks: `sequencer.state` [optional ticks: moves the timeline cursor first] gives `effects= selected= modified= undo= cursor= clipboard=`; `sequencer.find` [effect name] gives the Find panel's rows as `row@start`; `sequencer.lipSyncMaps` [optional name: makes it the default first] lists the maps, the default starred; `sequencer.phonemes` [words] starts Breakdown Word's lookup and returns at once (an unknown word puts up a modal sheet for the script to answer), `sequencer.phonemesResult` reads the outcome. Scripts are in `target/shell-test4/` (`run.sh <script> [sequence file]`, `lip1` to `lip5`). Harness notes: a text click finds text in the window's picture, title and labels included, so give `index` when a word occurs twice (0 is the first); toolbar buttons have no text, so click them with `mouse` at their place after showing the strip; a hook that runs a modal loop must schedule itself with `RunLoop.main.perform` and return. The test profile `target/show-shell` now has a second image map, `New Map(1)`, left there for the two-map checks. Still open: F89 (marks worker's node select window), the audio items F81 and F271, F12's mapper, F378's tag manager, and the library drags F169, F170, F346, F347, F356, F357, which work through their hook but whose gesture needs a hand check.

> **Pass 2 (2026-11-09):** the timeline API this window waited on is in: the applied offset of a range move, the remembered playback range and `ok_to_use_alignment`, as three `pub(crate)` functions at the end of `crates/vixen-ffi/src/timeline.rs` plus `Timeline::remembered_playback_range`. Previews hear `AppCore::notify_player_active` and their forwarded keys are handled (preview F69, F72). The library panels now do what `docs/parity/effect-editor.md` F82 and F96 list as missing: drop at the pointer's index, the 10 pt rule, reorder of the whole selection, and the gradient texts. Build this window with `VIXEN_BUILD_ID=shell VIXEN_AREAS=sequencer,admin,dialogs,scheduler scripts/build_app.sh debug`: with fewer areas, old modules left in `target/build-shell` make `canImport` true and the build fails. Pass 2's scripts are in `target/shell-test3/` (`run.sh <script> [sequence file]`); new hooks: `sequencer.libraryMove` [color|curve|gradient, ids joined by "|", target id or ""] `sequencer.bulkMove` [start ms, end ms, offset ms, visible only, clip, marks], `sequencer.dropEffect` [effect name, row] and `sequencer.picker` [show | state | snapshot, name | choose, index]; after `show` a drop that needs the picker opens it under the harness instead of taking the first choice. `sequencer.wheel` [x, y from the top, dy, modifiers joined by "+"] posts a wheel event through the app's queue so event monitors hear it, and `sequencer.panelScale` [settings key] reads a saved library scale. Write Test State also gives `firstEffectStartMs` and `firstMarksMs`.

> **Note from the Administration worker for the next pass:** with both areas in the build, the sequencer shows a second `Tools` menu and puts `Sequence` before `File`. F35 wants its items under the one Tools menu.
> **Answered in pass 1:** `SequencerMenuBar.arrange` (`UI/SequencerScene.swift`) hides the Administration window's Tools menu while a sequencer is in front, so there is one; the order with a sequencer in front is Vixen, Sequence, File, Edit, View, Tools, Window, Help, as upstream's editor window has Sequence first.
> **Library edits** now reach open sequences through `SequenceSession::libraries_changed` (`crates/vixen-session/src/library_links.rs`), which `AppCore`'s library functions in `effect_editor/libraries.rs` already call.
> **From the timeline worker, now finished:** the F16 note below about timeline row height changes is out of date. The delegate call exists and `SequencerModel` implements it. The timeline's public API for this window is listed at the end of `docs/parity/timeline.md`.


Status of every entry in `docs/spec/sequencer-shell.md`. This table is how passes hand over to each other.

Status values: **done** (with where it lives), **partial** (with what is missing), **todo**, **omitted** (with why).

Paths: `S/` is `crates/vixen-session/src/sequencer/`, `FFI` is `crates/vixen-ffi/src/sequencer.rs`, `UI/` is `app/Sources/VixenSequencer/`.

| F | Feature | Status | Where, and what is missing |
|---|---|---|---|
| F1 | Open the sequencer window | done | `S/state.rs` `attach`, `finish_load`, colour collections and the effect defaults store at attach; `UI/SequencerModel.swift` `open`. Dock and toolbar restore done; splitter restore through timeline settings. |
| F2 | Default tool window layout | done | `S/panels.rs`, `UI/SequencerContent.swift` `DockArea`, `DockPane`: NSSplitView, tabs along the bottom, panels hide instead of dispose. Default left width 200 rather than 150 so names fit |
| F3 | Window placement restore | done | `UI/SequencerModel.swift` `attach`: frame autosave and 640x480 minimum, which replace the saved-bounds keys. `WindowWidth` and friends stay in `EditorPrefs` for Windows files |
| F4 | Title bar | done | `S/lifecycle.rs` `window_title`, `UI/SequencerModel.swift` `updateWindowChrome`: name as title, edited dot, profile as subtitle |
| F5 | Window shown | done | `UI/SequencerModel.swift` `open`: window up at once, loading on a worker |
| F6 | Sequence load | done | `S/state.rs` `finish_load`, status text in `SequencerModel.open` |
| F7 | Row settings on load | done | Timeline `load_rows`; Show Hidden resets because `SequencerState` starts it off |
| F8 | Load filtering | done | `S/state.rs` `finish_load`, with `LoadReport` counts |
| F9 | Missing audio on load | done | `UI/SequencerSheets.swift` `MissingAudioSheet`, `UI/SequencerTools.swift` `askAboutMissingAudio`. Checked on a private profile: Forget removes the media. |
| F10 | Deprecated elements warning | done | `S/state.rs`, `S/lifecycle.rs` `deprecated_elements_message` |
| F11 | Mark collection rename summary | done | `S/lifecycle.rs` `rename_duplicate_mark_collections`, `mark_rename_summary` |
| F12 | Unmapped effects at open | done | `UI/SequencerModel.swift` `resolveUnmappedEffects` calls the shared `ElementMapper.present` hook, supplied by VixenScheduler. Accepted pairs reach `apply_element_map` before `finish_load`; Cancel uses an empty map, Leave Unmapped uses None, matching upstream removal behavior. F12’s modified-state fix is preserved. A missing scheduler area gives an explicit warning. November 11: mapped a synthetic unknown source, saved/reopened, and checked exact effect IDs/target names; Cancel and Leave Unmapped also passed with the expected effect counts and modified flags (`target/resume-ui/mapping*`). |
| F13 | Save | done | `S/state.rs` `save_route`, `prepare_save`, `finish_save`; `S/lifecycle.rs` `save_path_with_extension`; `UI/SequencerAlerts.swift` `SaveSequencePanel`. Writes off the main thread (hazard 24) |
| F14 | What SaveGridRowSettings stores | done | Timeline `store_view_state` through `GridPort::store_view_state`; `S/lifecycle.rs` `row_setting_to_store` for the plain grid |
| F15 | Auto save | done | `S/lifecycle.rs` `AutoSaveClock`, `S/state.rs` `poll`, `UI/SequencerModel.swift` `startPolling` |
| F16 | Close prompt | done | `S/lifecycle.rs` `close_action`, `UI/SequencerModel.swift` `shouldClose`, `closeForShutdown`, with the FIX. Zoom, scroll and row height changes all reach the state flag now: the timeline calls `timelineEditorStateModified` (its F153), which lands in `SequencerState::note_editor_state_modified`. |
| F17 | Closing writes | done | `S/state.rs` `finish_close`, `save_window_settings`; one write (hazard 10). Old nudge keys not removed (omitted per Windows-only table) |
| F18 | Cleanup of deleted effects at close | done | `SequenceSession::close_video_cache`, `sequencer/state.rs::finish_close` and `vixen-video/cache.rs`: deleted effects retain their cache for Undo; accepted close unpairs deleted instances and applies the current clear option. Active windows/readers retain shared files; late workers cannot republish. Rust/AVFoundation Delete→Undo→Redo→close and shutdown tests pass; release AppKit139-step cache workflow additionally observes retain/reopen and real clear-on-close. See `docs/review/video-cache.md`. |
| F19 | Active editor tracking | done | `UI/SequencerScene.swift` `SequencerRegistry`, `AppModel.editorActivated` |
| F20 | Multiple editor windows | done | One `SequencerModel` per window; `PLAYBACK_OWNER` in `S/state.rs` keeps playback per window |
| F21 | Effect edits mark the sequence | done | Timeline commits through the session; `poll` sees `is_modified` |
| F22 | Mark collection changes mark the sequence | done | Edits through the timeline and through VixenMarks' panel (`vixen_session::marks::publish`, which sets modified or calls `set_marks`) reach `is_modified`, which `poll` follows. Panel embedded in pass 2. |
| F23 | Library changes re-render | done | `AppCore::add_library_curve` and friends update linked effects in open sequences; the window reads the libraries again in `librariesChanged`. |
| F24 | Sequence > Save | done | `UI/SequencerCommands.swift`, `SequencerModel.save`. Save As is ⌥⌘S |
| F25 | Sequence > Save As | done | `UI/SequencerCommands.swift`, `SequencerModel.save`. Save As is ⌥⌘S |
| F26 | Sequence > Auto Save | done | `set_auto_save`, menu toggle, File strip toggle |
| F27 | Sequence > Playback > Play | done | `UI/SequencerCommands.swift` `SequenceMenu`, `S/transport.rs`. Checked by harness against Rust state |
| F28 | Sequence > Playback > Pause | done | `UI/SequencerCommands.swift` `SequenceMenu`, `S/transport.rs`. Checked by harness against Rust state |
| F29 | Sequence > Playback > Stop | done | `UI/SequencerCommands.swift` `SequenceMenu`, `S/transport.rs`. Checked by harness against Rust state |
| F30 | Sequence > Playback > Loop | done | `UI/SequencerCommands.swift` `SequenceMenu`, `S/transport.rs`. Checked by harness against Rust state |
| F31 | Sequence > Playback > Play Options > Delay Off | done | `UI/SequencerCommands.swift` `SequenceMenu`, `S/transport.rs`. Checked by harness against Rust state |
| F32 | Sequence > Playback > Play Options > Delay 5 Seconds | done | `UI/SequencerCommands.swift` `SequenceMenu`, `S/transport.rs`. Checked by harness against Rust state |
| F33 | ... > Delay 10 Seconds | done | `UI/SequencerCommands.swift` `SequenceMenu`, `S/transport.rs`. Checked by harness against Rust state |
| F34 | ... > Delay 20 Seconds | done | `UI/SequencerCommands.swift` `SequenceMenu`, `S/transport.rs`. Checked by harness against Rust state |
| F35 | ... > Delay 30 Seconds | done | `UI/SequencerCommands.swift` `SequenceMenu`, `S/transport.rs`. Checked by harness against Rust state |
| F36 | ... > Delay 60 Seconds | done | `UI/SequencerCommands.swift` `SequenceMenu`, `S/transport.rs`. Checked by harness against Rust state |
| F37 | Sequence > Export | done | `SequencerModel.export` calls VixenKit's `SequenceExport.present(session, window)`, which the scheduler and export area sets: its Export Sequence sheet on this window over this session, holding the Effect Preview while it's up (its `PreviewHold`). In a build without that area the item is disabled. Checked in pass 2: Sequence > Export showed the sheet with the real show's two controllers, and Close dismissed it. |
| F38 | Sequence > Close | done | Sequence > Close and File > Close, both `performClose`, reach the prompt through `SequencerWindowDelegate` |
| F39 | Edit > Add Effect | done | Edit > Add Effect submenu, with `add_effect` |
| F40 | Edit > Cut | done | `S/clipboard.rs` `plan_copy`, `S/state.rs` `copy_or_cut`, `UI/SequencerAlerts.swift` `SequencerPasteboard` |
| F41 | Edit > Copy | done | `S/clipboard.rs` `plan_copy`, `S/state.rs` `copy_or_cut`, `UI/SequencerAlerts.swift` `SequencerPasteboard` |
| F42 | Edit > Paste | done | `S/state.rs` `paste` |
| F43 | Edit > Paste Special > Invert | done | Paste Special > Invert, Visible Marks; `S/clipboard.rs` `plan_paste`, `paste_menu_state` |
| F44 | Edit > Paste Special > Visible Marks | done | Paste Special > Invert, Visible Marks; `S/clipboard.rs` `plan_paste`, `paste_menu_state` |
| F45 | Edit > Undo | done | Edit menu names the action from Rust's stack; ⌘Y kept as a hidden equivalent |
| F46 | Edit > Redo | done | Edit menu names the action from Rust's stack; ⌘Y kept as a hidden equivalent |
| F47 | Edit > Select All Elements | done | `select_all`, `delete_selected`. ⌫ through the menu, ⌦ through the key monitor |
| F48 | Edit > Delete | done | `select_all`, `delete_selected`. ⌫ through the menu, ⌦ through the key monitor |
| F49 | Edit > Snap To Marks / Effects | done | `UI/SequencerCommands.swift` `EditExtrasItems`; prefs reach the timeline through `apply_prefs` |
| F50 | Edit > Resize / Draw Indicator | done | `UI/SequencerCommands.swift` `EditExtrasItems`; prefs reach the timeline through `apply_prefs` |
| F51 | Edit > Resize / Draw Indicator > Blue, Yellow, Green, White, Red | done | `UI/SequencerCommands.swift` `EditExtrasItems`; prefs reach the timeline through `apply_prefs` |
| F52 | Edit > CAD Style Selection Box | done | `UI/SequencerCommands.swift` `EditExtrasItems`; prefs reach the timeline through `apply_prefs` |
| F53 | Edit > Legacy Cursor / Active Row | done | `UI/SequencerCommands.swift` `EditExtrasItems`; prefs reach the timeline through `apply_prefs` |
| F54 | View > Zoom Time In | done | View items; `GridPort::zoom_time`, `zoom_rows` |
| F55 | View > Zoom Time Out | done | View items; `GridPort::zoom_time`, `zoom_rows` |
| F56 | View > Zoom Under Mouse Position | done | View items; `GridPort::zoom_time`, `zoom_rows` |
| F57 | View > Zoom Rows In | done | View items; `GridPort::zoom_time`, `zoom_rows` |
| F58 | View > Zoom Rows Out | done | View items; `GridPort::zoom_time`, `zoom_rows` |
| F59 | View > Collapse All Element Groups | done | `collapse_all`, `reset_row_heights` |
| F60 | View > Reset Row Height to Default | done | `collapse_all`, `reset_row_heights` |
| F61 | View > Highlight Rows With Effects | done | Prefs through `apply_prefs` |
| F62 | View > Show Effect Info | done | Prefs through `apply_prefs` |
| F63 | View > Show Hidden Rows | done | `set_show_hidden_rows`; never persisted |
| F64 | View > Full Waveform | done | Prefs through `apply_prefs` |
| F65 | View > Lock Ruler Height | done | Prefs through `apply_prefs` |
| F66 | View > Lock Waveform Height | done | Prefs through `apply_prefs` |
| F67 | View > Toolbars | done | View > Toolbars, `ToolbarsList` |
| F68 | View > Effect Window | done | `S/panels.rs` `toggle` with the reopen sides kept; `UI/SequencerCommands.swift` `panelItem` |
| F69 | View > Mark Window | done | `S/panels.rs` `toggle` with the reopen sides kept; `UI/SequencerCommands.swift` `panelItem` |
| F70 | View > Effect Editor Window | done | `S/panels.rs` `toggle` with the reopen sides kept; `UI/SequencerCommands.swift` `panelItem` |
| F71 | View > Layer Editor Window | done | `S/panels.rs` `toggle` with the reopen sides kept; `UI/SequencerCommands.swift` `panelItem` |
| F72 | View > Find Effects or Layers | done | `S/panels.rs` `toggle` with the reopen sides kept; `UI/SequencerCommands.swift` `panelItem` |
| F73 | View > Color Library Window | done | `S/panels.rs` `toggle` with the reopen sides kept; `UI/SequencerCommands.swift` `panelItem` |
| F74 | View > Gradient Library Window | done | `S/panels.rs` `toggle` with the reopen sides kept; `UI/SequencerCommands.swift` `panelItem` |
| F75 | View > Curve Library Window | done | `S/panels.rs` `toggle` with the reopen sides kept; `UI/SequencerCommands.swift` `panelItem` |
| F76 | View > Timeline Window | done | Timeline Window item is always checked: on the Mac the timeline can't be hidden |
| F77 | Tool window visibility checks | done | Checks read `ShellPanels.visible` |
| F78 | Tools > Audio > Associate Audio | done | `S/commands.rs` `add_audio`, `remove_audio`; `UI/SequencerTools.swift`. Checked through the harness on a private profile. |
| F79 | Tools > Audio > Remove Audio | done | `S/commands.rs` `add_audio`, `remove_audio`; `UI/SequencerTools.swift`. Checked through the harness on a private profile. |
| F80 | Tools > Audio > Beat/Bar Detection | done | `UI/SequencerCommands.swift` `ToolsMenu`, `UI/SequencerSheets.swift` `.beatsAndBars`: the item, enabled by `beat_bar_enabled`, presents VixenMarks' `BeatsAndBarsSheet(session:onFinish:)`, which does the detection, the dialog and the modified flag (marks table F67 to F75). Run on the real show (`target/shell-test3/beats.json`): dialog seen, Generate replaced the nine Beats collections, the Mark Collections tab and the state followed. |
| F81 | Tools > Audio > Speed / Tempo | done | `AudioPlayer::set_rate_mode`, `ShowPlayback` and `SequencerState::set_use_tempo` wired. Off changes pitch with rate; on preserves pitch. Mode changes apply at the next speed assignment, as upstream. Output-frequency/timing tests cover both modes, 10–400%, stereo and mismatched sample rates; native menu toggles and transport states verified in `target/progress-ui/audio-asserted` (muted) |
| F82 | Tools > Create Evenly Divided Marks | done | `S/commands.rs` `create_even_marks`, `UI/SequencerSheets.swift` `EvenMarksSheet`, error and reopen checked. |
| F83 | Tools > Sequence Length | done | `UI/SequencerSheets.swift` `SequenceLengthSheet`, `S/state.rs` `set_sequence_length`. |
| F84 | Tools > Curve Editor | done | `UI/SequencerTools.swift`: VixenKit's selectors; choosing an entry opens the editor on it, then the strips rebuild. Double-click-to-edit mode in the selector is VixenKit's. |
| F85 | Tools > Color Gradient Editor | done | `UI/SequencerTools.swift`: VixenKit's selectors; choosing an entry opens the editor on it, then the strips rebuild. Double-click-to-edit mode in the selector is VixenKit's. |
| F86 | Tools > Color Collection Editor | done | `S/libraries.rs` `CollectionEditor` on a deep copy (F374), tested; `UI/SequencerSheets.swift` `ColorCollectionsSheet`. Sheet checked by eye; add, reorder and import are Rust-tested. |
| F87 | Tools > LipSync > Default Image Map | done | Pass 3. `S/commands.rs` `lipsync_maps` (library order, the default flagged; with none flagged the first) and `set_default_lipsync_map` (flags one map alone, marks this sequence modified, saves the module store at once: marks F89's FIX, since upstream loses the choice at exit), tested; FFI `lipsync_maps`, `set_default_lipsync_map`; `UI/SequencerLipSync.swift` `reloadLipSyncMaps`, `setDefaultLipSyncMap`; the submenu of checked items in `UI/SequencerCommands.swift`. The list is read when the window comes forward, after a choice and after Image Maps closes, not on each menu opening. Checked through the menu with two maps (`target/shell-test4/lip4.json`): the check mark moves, choosing the checked one changes nothing, `ModuleStore.xml` has the flags. The marks worker's `DefaultImageMapMenuItems` does the same job from `VixenCore` and marks every open sequence modified; this window uses its own so that only this sequence is, as upstream. |
| F88 | Tools > LipSync > Edit Image Maps | done | Pass 3. `UI/SequencerLipSync.swift` `openImageMaps` presents the marks worker's `ImageMapsSheet(core:onClose:)` as a sheet on this window; their `ImageMaps.close` does the rest when something changed (open sequences modified, LipSync effects rendered again, system and module configuration saved), then this window re-reads the Default Image Map list and refreshes. Checked (`target/shell-test4/lip3.json`): the sheet opens from the menu, New makes `New Map(1)` and opens its editor, OK twice leaves the sequence modified and both maps in the submenu; opened and closed with no change, the sequence stays unmodified. Selecting a row in their table by a harness click didn't take, so Edit, Remove and Clone weren't driven from here (their own hooks cover them in the Marks Test window, which isn't in this build). |
| F89 | Tools > LipSync > Edit Element Face Mapping | done | `UI/SequencerLipSync.swift` `openFaceMapping` presents the marks area’s `FaceMappingNodeSelectSheet`, which opens Face Setup and applies/saves accepted changes. Menu enabled when the marks module is present. Opened and cancelled from the combined app on a private show copy on November 11; `target/resume-ui/`. |
| F90 | Tools > LipSync > Lyric Converter | done | Pass 3. `UI/SequencerLipSync.swift` `openLyricConverter` and `LyricConverterWindow`: the marks worker's `LyricConverterView(session:cursorPosition:pasteAtTime:)` in a "LipSync Text Convert" panel, not modal, a child window above the sequencer that opened it (`Show(this)`) and closed with it. One per process, as upstream's static `Active` flag; choosing the item again brings the open one forward, where upstream does nothing. `SequencerMenuBar` counts a child window as the sequencer being in front, so the menus stay. Checked (`target/shell-test4/lip1.json`, `lip2.json`): opens at 340x504, a second choice opens no second window, closing an unmodified sequencer closes it. |
| F91 | Tools > LipSync > Papagayo Import | done | `S/papagayo.rs`, `S/commands.rs` `papagayo_import`, tested; a bad file shows its error (GUARD). |
| F92 | Tools > Bulk Effects Move | done | `S/commands.rs` `bulk_move`, `UI/SequencerSheets.swift` `BulkMoveSheet`, sheet checked. The marks get the offset the grid really applied: `FFI` `TimelineGrid::move_in_range_by_time` returns it from `TimelineController::move_in_range_reporting` (`crates/vixen-ffi/src/timeline.rs`). Checked on the real show with the `sequencer.bulkMove` hook (`target/shell-test3/bulk.json`): 100 s back without Clip moved 587 effects and 1391 marks by the 313 ms there was room for. |
| F93 | Tools > Effect Defaults > Export | done | `vixen_io::effect_defaults` `export`, `import`, `write_diagnostic_dump`; `S/commands.rs`; `UI/SequencerTools.swift`; `EffectDefaultsExportSheet`. |
| F94 | Tools > Effect Defaults > Import | done | `vixen_io::effect_defaults` `export`, `import`, `write_diagnostic_dump`; `S/commands.rs`; `UI/SequencerTools.swift`; `EffectDefaultsExportSheet`. |
| F95 | Tools > Effect Defaults > Dump (Readable XML) | done | `vixen_io::effect_defaults` `export`, `import`, `write_diagnostic_dump`; `S/commands.rs`; `UI/SequencerTools.swift`; `EffectDefaultsExportSheet`. |
| F96 | Help > Online Help | done | Help > Sequencer Online Help: Help is shared with the Administration window, whose Online Help opens the docs front page |
| F97 | Help > Vixen YouTube Channel | done | The Administration window's item in the shared Help menu opens the same URL |
| F98 | Strip layout and drag | done | `S/toolbars.rs`, `UI/ToolbarBand.swift`: rows, drag, pack left, Windows settings read. Pass 2: rows are `StripRowLayout` and each strip's items `StripItems` (`UI/ToolbarOverflow.swift`), so a row never runs off the left (pass 1's centred rows hid Start and Play below about 1420 pt) and what passes the right edge goes into the strip's » drop-down, as upstream's `ToolStripPanel` and `NSToolbar` both do. Buttons use `StripButtonStyle`, upstream's size (about 34 pt a button), so the default row is about 1000 pt. Checked at 1440 and 800 wide with every strip on; drop-downs read through the `sequencer.overflow.<strip>` hook. Toolbar presses can't be scripted (a test window is never key), so their actions were checked through the menu items that share them. |
| F99 | Toolbars list | done | `StripContextMenu`, `ToolbarsList`, one source of truth |
| F100 | Toolbar context menu | done | `UI/ToolbarBand.swift` `StripContextMenu`, `StripItemsPopover`, `S/toolbars.rs`. Show/Hide Items is a popover that stays open while ticking. |
| F101 | Toolbar context > Show/Hide Items | done | `UI/ToolbarBand.swift` `StripContextMenu`, `StripItemsPopover`, `S/toolbars.rs`. Show/Hide Items is a popover that stays open while ticking. |
| F102 | Toolbar context > Toolbars | done | `UI/ToolbarBand.swift` `StripContextMenu`, `StripItemsPopover`, `S/toolbars.rs`. Show/Hide Items is a popover that stays open while ticking. |
| F103 | Toolbar context > Toggle Image Size | done | `UI/ToolbarBand.swift` `StripContextMenu`, `StripItemsPopover`, `S/toolbars.rs`. Show/Hide Items is a popover that stays open while ticking. |
| F104 | Toolbar context > Reset Toolbar | done | `UI/ToolbarBand.swift` `StripContextMenu`, `StripItemsPopover`, `S/toolbars.rs`. Show/Hide Items is a popover that stays open while ticking. |
| F105 | Start | done | `PlayBackStrip`, `S/transport.rs` |
| F106 | Play | done | `PlayBackStrip`, `S/transport.rs` |
| F107 | Stop | done | `PlayBackStrip`, `S/transport.rs` |
| F108 | End | done | `PlayBackStrip`, `S/transport.rs` |
| F109 | Loop | done | `PlayBackStrip`, `S/transport.rs` |
| F110 | Undo | done | `UndoSplitButton`, list on opening only |
| F111 | Redo | done | `UndoSplitButton`, list on opening only |
| F112 | Cut | done | `EditStrip` |
| F113 | Copy | done | `EditStrip` |
| F114 | Paste | done | `EditStrip` |
| F115 | Paste Special drop-down | done | `EditStrip` |
| F116 | Paste - Invert | done | `EditStrip` |
| F117 | Paste - Visible Mark/s | done | `EditStrip` |
| F118 | Draw Mode | done | `ModeStrip`, `DragBoxFilterList`; filter and draw mode reach the timeline |
| F119 | Selection Mode | done | `ModeStrip`, `DragBoxFilterList`; filter and draw mode reach the timeline |
| F120 | Snap To | done | `ModeStrip`, `DragBoxFilterList`; filter and draw mode reach the timeline |
| F121 | Snap Strength drop-down | done | `ModeStrip`, `DragBoxFilterList`; filter and draw mode reach the timeline |
| F122 | Drag Box Filter | done | `ModeStrip`, `DragBoxFilterList`; filter and draw mode reach the timeline |
| F123 | Drag Box Filter drop-down | done | `ModeStrip`, `DragBoxFilterList`; filter and draw mode reach the timeline |
| F124 | Zoom Time In | done | `ViewStrip` |
| F125 | Zoom Time Out | done | `ViewStrip` |
| F126 | Zoom Row In | done | `ViewStrip` |
| F127 | Zoom Row Out | done | `ViewStrip` |
| F128 | Associate Audio | done | `UI/ToolbarBand.swift`, the button runs F78's flow. |
| F129 | Decrease Timing speed | done | `AudioStrip`, `SpeedGaugeView`, `S/transport.rs` |
| F130 | Timing speed gauge | done | `AudioStrip`, `SpeedGaugeView`, `S/transport.rs` |
| F131 | Increasing Timing speed | done | `AudioStrip`, `SpeedGaugeView`, `S/transport.rs` |
| F132 | Audio Devices drop-down | done | `S/commands.rs` `audio_devices`, `set_audio_device` with F132's FIX; `vixen_session::playback` restores the saved choice at construction and applies changes; `UI/ToolbarBand.swift` `AudioDevicesMenu`. System Default is a row of its own for the `-0` placeholder. Twelve muted playback tests pass, including startup before any editor, missing/default devices and preserving a current choice when another editor opens (`target/audio-device-startup-tests.log`). |
| F133 | Speed gauge drawing | done | `S/transport.rs` `speed_gauge`, `SpeedGaugeView` |
| F134 | Save | done | `FileStrip`; Export wired (F137's FIX) |
| F135 | Save As | done | `FileStrip`; Export wired (F137's FIX) |
| F136 | Auto Save | done | `FileStrip`; Export wired (F137's FIX) |
| F137 | Export | done | `FileStrip`: the Export button calls the same `SequencerModel.export` as F37 (F137's FIX), disabled without the scheduler and export area. |
| F138 | Close | done | `FileStrip`; Export wired (F137's FIX) |
| F139 | Close Gaps | done | `S/state.rs` `close_gaps`, with its warning, through `TimelineController::close_gaps`. |
| F140 | Close Gaps threshold drop-down | done | `ThresholdMenu` |
| F141 | Align To threshold drop-down | done | `ThresholdMenu` |
| F142 | Start Mark | done | `AlignmentStrip`, `S/effect_ops.rs` |
| F143 | End Mark | done | `AlignmentStrip`, `S/effect_ops.rs` |
| F144 | Both Marks | done | `AlignmentStrip`, `S/effect_ops.rs` |
| F145 | Distribute Mark | done | `AlignmentStrip`, `S/effect_ops.rs` |
| F146 | LipSync Edit Map | done | Pass 3. `UI/ToolbarBand.swift` `ToolsStrip`: the button calls `openImageMaps` (F88). Checked with the Tools strip shown (`target/shell-test4/lip5.json`): a click on it opens the Image Maps sheet. |
| F147 | Curve Library | done | `UI/ToolbarBand.swift`: F84, F85. |
| F148 | Color Library | done | `UI/ToolbarBand.swift`: the Color Collection Editor, as upstream wires it. |
| F149 | Color Gradient | done | `UI/ToolbarBand.swift`: F84, F85. |
| F150 | Effect buttons | done | `UI/ToolbarBand.swift` `EffectsStrip`, `EffectDragSource`: an AppKit drag of the type id, a click choosing it in Draw Mode. The grid takes it through `UI/SequencerDrops.swift`, checked through the harness from the pasteboard on. |
| F151 | Effects strip context menu | done | `UI/ToolbarBand.swift` `EffectsStripMenu`, `StripItemsPopover`: a popover stays open while ticking, where a Mac menu can't. Toolbars and Label Postion are ordinary menus that close on a choice. |
| F152 | Effects context > Show/Hide Effects | done | `UI/ToolbarBand.swift` `EffectsStripMenu`, `StripItemsPopover`: a popover stays open while ticking, where a Mac menu can't. Toolbars and Label Postion are ordinary menus that close on a choice. |
| F153 | Show/Hide Effects > Basic | done | `UI/ToolbarBand.swift` `EffectsStripMenu`, `StripItemsPopover`: a popover stays open while ticking, where a Mac menu can't. Toolbars and Label Postion are ordinary menus that close on a choice. |
| F154 | Show/Hide Effects > Pixel | done | `UI/ToolbarBand.swift` `EffectsStripMenu`, `StripItemsPopover`: a popover stays open while ticking, where a Mac menu can't. Toolbars and Label Postion are ordinary menus that close on a choice. |
| F155 | Show/Hide Effects > Device | done | `UI/ToolbarBand.swift` `EffectsStripMenu`, `StripItemsPopover`: a popover stays open while ticking, where a Mac menu can't. Toolbars and Label Postion are ordinary menus that close on a choice. |
| F156 | Show/Hide Effects > Fixture | done | `UI/ToolbarBand.swift` `EffectsStripMenu`, `StripItemsPopover`: a popover stays open while ticking, where a Mac menu can't. Toolbars and Label Postion are ordinary menus that close on a choice. |
| F157 | Effects context > Label Postion | done | `UI/ToolbarBand.swift` `EffectsStripMenu`, `StripItemsPopover`: a popover stays open while ticking, where a Mac menu can't. Toolbars and Label Postion are ordinary menus that close on a choice. |
| F158 | Label Postion > No Label | done | `UI/ToolbarBand.swift` `EffectsStripMenu`, `StripItemsPopover`: a popover stays open while ticking, where a Mac menu can't. Toolbars and Label Postion are ordinary menus that close on a choice. |
| F159 | Label Postion > Above Image | done | `UI/ToolbarBand.swift` `EffectsStripMenu`, `StripItemsPopover`: a popover stays open while ticking, where a Mac menu can't. Toolbars and Label Postion are ordinary menus that close on a choice. |
| F160 | Label Postion > Below Image | done | `UI/ToolbarBand.swift` `EffectsStripMenu`, `StripItemsPopover`: a popover stays open while ticking, where a Mac menu can't. Toolbars and Label Postion are ordinary menus that close on a choice. |
| F161 | Effects context > Toolbars | done | `UI/ToolbarBand.swift` `EffectsStripMenu`, `StripItemsPopover`: a popover stays open while ticking, where a Mac menu can't. Toolbars and Label Postion are ordinary menus that close on a choice. |
| F162 | Effects context > Reset Toolbar | done | `UI/ToolbarBand.swift` `EffectsStripMenu`, `StripItemsPopover`: a popover stays open while ticking, where a Mac menu can't. Toolbars and Label Postion are ordinary menus that close on a choice. |
| F163 | Fixture group quirks | done | One key for Fixture in `S/prefs.rs`, with the popover staying open (both FIXes). |
| F164 | Contents | done | `UI/LibraryPanels.swift` `LibraryStrip`, `LibraryCell`. F170's FIX holds: nothing is written on drag enter. F173's GUARD: Edit needs an item under the pointer. F177's FIX and on-demand entry: `S/toolbars.rs`. |
| F165 | Hover | done | `UI/LibraryPanels.swift` `LibraryStrip`, `LibraryCell`. F170's FIX holds: nothing is written on drag enter. F173's GUARD: Edit needs an item under the pointer. F177's FIX and on-demand entry: `S/toolbars.rs`. |
| F166 | Click | done | `UI/LibraryPanels.swift` `LibraryStrip`, `LibraryCell`. F170's FIX holds: nothing is written on drag enter. F173's GUARD: Edit needs an item under the pointer. F177's FIX and on-demand entry: `S/toolbars.rs`. |
| F167 | Double-click | done | `UI/LibraryPanels.swift` `LibraryStrip`, `LibraryCell`. F170's FIX holds: nothing is written on drag enter. F173's GUARD: Edit needs an item under the pointer. F177's FIX and on-demand entry: `S/toolbars.rs`. |
| F168 | Drag out | done | `UI/LibraryPanels.swift` `LibraryStrip`, `LibraryCell`. F170's FIX holds: nothing is written on drag enter. F173's GUARD: Edit needs an item under the pointer. F177's FIX and on-demand entry: `S/toolbars.rs`. |
| F169 | Reorder | partial | `UI/LibraryPanels.swift` `LibraryCellView`, `LibraryDropCatcher`, `LibraryDropTracker`: drag out with the payload; a drop counts only more than 10 pt from where the drag came in; the whole selection moves to the item under the pointer in upstream's order (`moveSelection`, on `libraryMoveCurves`/`libraryMoveGradients` and, for colours, `S/libraries.rs` `moved_by_position`, tested); a move onto empty space goes last; a curve or gradient from elsewhere is named, then placed at the pointer's index (`libraryInsertCurve`/`libraryInsertGradient`). The reorder is checked through the `sequencer.libraryMove` hook (`target/shell-test3/move.json`: order read back and seen in the panel). Missing: the drag gesture itself and the 10 pt rule, which the harness can't drive. Check by hand. |
| F170 | Drop in | partial | `UI/LibraryPanels.swift` `LibraryCellView`, `LibraryDropCatcher`, `LibraryDropTracker`: drag out with the payload; a drop counts only more than 10 pt from where the drag came in; the whole selection moves to the item under the pointer in upstream's order (`moveSelection`, on `libraryMoveCurves`/`libraryMoveGradients` and, for colours, `S/libraries.rs` `moved_by_position`, tested); a move onto empty space goes last; a curve or gradient from elsewhere is named, then placed at the pointer's index (`libraryInsertCurve`/`libraryInsertGradient`). The reorder is checked through the `sequencer.libraryMove` hook (`target/shell-test3/move.json`: order read back and seen in the panel). Missing: the drag gesture itself and the 10 pt rule, which the harness can't drive. Check by hand. |
| F171 | Library strip context menu | done | `UI/LibraryPanels.swift` `LibraryStrip`, `LibraryCell`. F170's FIX holds: nothing is written on drag enter. F173's GUARD: Edit needs an item under the pointer. F177's FIX and on-demand entry: `S/toolbars.rs`. |
| F172 | Library context > New | done | `UI/LibraryPanels.swift` `LibraryStrip`, `LibraryCell`. F170's FIX holds: nothing is written on drag enter. F173's GUARD: Edit needs an item under the pointer. F177's FIX and on-demand entry: `S/toolbars.rs`. |
| F173 | Library context > Edit | done | `UI/LibraryPanels.swift` `LibraryStrip`, `LibraryCell`. F170's FIX holds: nothing is written on drag enter. F173's GUARD: Edit needs an item under the pointer. F177's FIX and on-demand entry: `S/toolbars.rs`. |
| F174 | Library context > Delete | done | `UI/LibraryPanels.swift` `LibraryStrip`, `LibraryCell`. F170's FIX holds: nothing is written on drag enter. F173's GUARD: Edit needs an item under the pointer. F177's FIX and on-demand entry: `S/toolbars.rs`. |
| F175 | Library context > Import | done | `UI/LibraryPanels.swift` `LibraryStrip`, `LibraryCell`. F170's FIX holds: nothing is written on drag enter. F173's GUARD: Edit needs an item under the pointer. F177's FIX and on-demand entry: `S/toolbars.rs`. |
| F176 | Library context > Export | done | `UI/LibraryPanels.swift` `LibraryStrip`, `LibraryCell`. F170's FIX holds: nothing is written on drag enter. F173's GUARD: Edit needs an item under the pointer. F177's FIX and on-demand entry: `S/toolbars.rs`. |
| F177 | Library context > Show/Hide Items | done | `UI/LibraryPanels.swift` `LibraryStrip`, `LibraryCell`. F170's FIX holds: nothing is written on drag enter. F173's GUARD: Edit needs an item under the pointer. F177's FIX and on-demand entry: `S/toolbars.rs`. |
| F178 | Library context > Toolbars | done | `UI/LibraryPanels.swift` `LibraryStrip`, `LibraryCell`. F170's FIX holds: nothing is written on drag enter. F173's GUARD: Edit needs an item under the pointer. F177's FIX and on-demand entry: `S/toolbars.rs`. |
| F179 | Library context > Toggle Image Size | done | `UI/LibraryPanels.swift` `LibraryStrip`, `LibraryCell`. F170's FIX holds: nothing is written on drag enter. F173's GUARD: Edit needs an item under the pointer. F177's FIX and on-demand entry: `S/toolbars.rs`. |
| F180 | Library context > Reset Toolbar | done | `UI/LibraryPanels.swift` `LibraryStrip`, `LibraryCell`. F170's FIX holds: nothing is written on drag enter. F173's GUARD: Edit needs an item under the pointer. F177's FIX and on-demand entry: `S/toolbars.rs`. |
| F181 | Home | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F182 | Ctrl+Home | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F183 | End | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F184 | Ctrl+End | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F185 | PageUp | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F186 | Ctrl+PageUp | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F187 | PageDown | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F188 | Ctrl+PageDown | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F189 | Up (no modifier) | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F190 | Down (no modifier) | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F191 | Space | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F192 | Ctrl+Space | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F193 | Shift+Space | done | `S/state.rs` `run_hotkey`, `GridPort::add_mark_at`. Checked playing, where the mark and its undo land, and stopped, where nothing happens. |
| F194 | Ctrl+Left | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F195 | Ctrl+Right | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F196 | S | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F197 | Shift+S | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F198 | Ctrl+Shift+S | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F199 | E | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F200 | Shift+E | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F201 | Ctrl+Shift+E | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F202 | B | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F203 | Ctrl+Shift+B | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F204 | Escape | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F205 | Ctrl+- | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F206 | Ctrl+Shift+- | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F207 | Ctrl+= | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F208 | Ctrl+Shift+= | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F209 | T | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F210 | Ctrl+T | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F211 | D | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F212 | Ctrl+D | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F213 | C | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F214 | U | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F215 | Shift+U | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F216 | R | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F217 | Shift+R | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F218 | N | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F219 | Shift+N | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F220 | I | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F221 | F5 key | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F222 | F8 key | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F223 | F9 key | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F224 | Delete | done | `S/keys.rs` whole table, tested; `S/state.rs` `run_hotkey`; `FFI` `TimelineGrid`. Each key run through the harness against Rust state, including the pointer keys at a set window position; the pointer is `window.mouseLocationOutsideOfEventStream` in real use. |
| F225 | Key routing from other windows | done | `UI/SequencerScene.swift` registry and the key monitor: a floating panel's keys go to the active sequencer. Keys the preview window forwards (`.previewForwardedKey`, preview F72) reach the active sequencer through `handleKey(_:forwarded:)`, which skips the key-window and text-field checks for them. |
| F226 | Add Effect(s) | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. |
| F227 | Layer | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. Pass 3: the Layer items' tooltip names the default layer's filter `None`, as `Layer.FilterName` (marks F66), tested. |
| F228 | Alignment | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. The 32-per-row limit is the timeline's own now (`TimelineGrid::ok_to_use_alignment`). |
| F229 | Align Start Times | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. |
| F230 | Align End Times | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. |
| F231 | Align Both Times | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. |
| F232 | Align Centerpoints | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. |
| F233 | Match Duration | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. |
| F234 | Align Start to End | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. |
| F235 | Align End to Start | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. |
| F236 | Distribute Equally | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. The 32-per-row limit is the timeline's own now (`TimelineGrid::ok_to_use_alignment`). |
| F237 | Distribute Effects | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. The 32-per-row limit is the timeline's own now (`TimelineGrid::ok_to_use_alignment`). |
| F238 | Align Start to nearest mark | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. |
| F239 | Align End to nearest mark | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. |
| F240 | Align Both to nearest mark | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. |
| F241 | Manipulation | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. |
| F242 | Manipulation > Divide at cursor | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. |
| F243 | Manipulation > Clone | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. |
| F244 | Manipulation > Clone to selected effects | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. |
| F245 | Edit Time | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. The 32-per-row limit is the timeline's own now (`TimelineGrid::ok_to_use_alignment`). |
| F246 | Context > Cut | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. |
| F247 | Context > Copy | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. |
| F248 | Context > Paste | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. |
| F249 | Context > Paste Special | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. |
| F250 | Context > Paste - Visible Mark/s | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. |
| F251 | Context > Paste - Invert | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. |
| F252 | Delete Effect(s) | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. |
| F253 | Collections | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. |
| F254 | Add Marks to Effects | done | `S/commands.rs` `context_menu`, `UI/SequencerContextMenu.swift`. Menu text and enabling checked against a real effect, Clone and Edit Time run through the harness. F253: `apply_color_collection`, tested, with both GUARDs. |
| F255 | Overlapped effect chooser | done | The timeline's `showOverlapMenu` (`UI/VixenTimeline/TimelineCanvas.swift`). |
| F256 | Status bar | done | `UI/StatusBarView.swift`, `S/state.rs` `ui_state`, `set_status_message` |
| F257 | Current Position display | done | `UI/StatusBarView.swift`, `S/state.rs` `ui_state`, `set_status_message` |
| F258 | Sequence Length display | done | `UI/StatusBarView.swift`, `S/state.rs` `ui_state`, `set_status_message` |
| F259 | Play Delayed display | done | `UI/StatusBarView.swift`, `S/state.rs` `ui_state`, `set_status_message` |
| F260 | Status message | done | `UI/StatusBarView.swift`, `S/state.rs` `ui_state`, `set_status_message` |
| F261 | Rendering Elements display | done | `UI/StatusBarView.swift`, `S/state.rs` `render_progress`. Pass 2 fixed the bar sticking: `poll` now compares the render report with what was last shown, so it moves and hides as renders finish with nothing else changing (`poll_follows_renders_catching_up`). |
| F262 | Context lifetime | done | `S/state.rs` `poll` follows `execution_state` |
| F263 | Play/Pause toggle | done | `S/transport.rs`, tests from the spec's table. Engine gaps in the spec stand |
| F264 | Play with delay | done | `S/transport.rs`, tests from the spec's table. Engine gaps in the spec stand |
| F265 | Play from / between | done | `S/transport.rs`, tests from the spec's table. Engine gaps in the spec stand |
| F266 | Stop | done | `S/transport.rs`, tests from the spec's table. Engine gaps in the spec stand |
| F267 | Context events | done | `S/transport.rs`, tests from the spec's table. Engine gaps in the spec stand . Previews hear `AppCore::notify_player_active` (preview F69): true before Play or Resume in `S/state.rs` `apply_engine`, false in `context_ended`. |
| F268 | Transport enable table | done | `S/transport.rs`, tests from the spec's table. Engine gaps in the spec stand |
| F269 | Start playback | done | `S/transport.rs`, tests from the spec's table. Engine gaps in the spec stand . Previews hear `AppCore::notify_player_active` (preview F69): true before Play or Resume in `S/state.rs` `apply_engine`, false in `context_ended`. |
| F270 | Delay countdown | done | `S/transport.rs`, tests from the spec's table. Engine gaps in the spec stand |
| F271 | Speed set | done | Shell keeps upstream `f32` stepping rules; player now allows 0.1–4.0. Tests measure output frequency and media clock through 400%, with no render-callback allocation. Native gauge displays400 and playback ends/loops at that speed; gauge command invoked through test hook because offscreen SwiftUI omits toolbar AX children. Pointer double-click remains unverified |
| F272 | Speed enablement | done | `S/transport.rs`, tests from the spec's table. Engine gaps in the spec stand |
| F273 | Ruler click (host part) | done | The timeline's ruler; `play_between` for ⌘-started plays |
| F274 | Ruler drag range (host part) | done | The timeline's ruler; `play_between` for ⌘-started plays |
| F275 | Timing interface for other tools | omitted | Unreachable: its only consumer is `MarkTapper`, which nothing constructs |
| F276 | Play | done | `vixen_session::playback::ShowPlayback::play` clamps both ends to the length. `S/transport.rs` `coerce_range` is the same rule as a tested free function; nothing calls it. |
| F277 | Natural end | done | Engine ends at exact end or zero after progress; a new play resets progress history. Audio end/device failure also completes a context. Deterministic clock tests cover exact-end and zero-after-progress; native 400% non-looping playback reaches stopped |
| F278 | Loop | done | Engine pauses/seeks/restarts the source at loop boundaries. Audio preserves a pending nonzero seek when Play follows end before the next callback. Nonzero loop-start tests and native muted 400% looping pass |
| F279 | Pause and resume | done | `S/transport.rs` `coerce_range`, `is_natural_end`; `ShowPlayback` does the rest |
| F280 | Stop | done | `S/transport.rs` `coerce_range`, `is_natural_end`; `ShowPlayback` does the rest |
| F281 | Timing sources | done | `timing_source_exists` in `S/state.rs` |
| F282 | Volume | omitted | There is no volume control upstream: the Mac honours the system volume |
| F283 | Audio output prefs | done | `S/commands.rs` persists the choice; `vixen_session::playback` restores it before scheduler/web playback can start. Per-editor restoration was removed so opening another editor preserves a current explicit choice. Default/missing device fallback and private-file restoration are covered by the twelve muted playback tests (`target/audio-device-startup-tests.log`); physical output remains separate verification. |
| F284 | Add Effect submenu entry | done | `S/state.rs` `add_effect_from_menu` |
| F285 | Effect instance creation | done | `S/state.rs` hands the session the profile's defaults store; `vixen_session::sequence` `add_new_effect` reads it. Clone and paste skip it through `add_copy`. |
| F286 | Default new effect duration | done | `S/effect_ops.rs` |
| F287 | Draw an effect | done | The timeline's gestures; `add_new_effect` picks up saved defaults. |
| F288 | Move an effect to another row | done | The timeline's gestures; `add_new_effect` picks up saved defaults. |
| F289 | Clicking the grid clears mark selection | done | The timeline's `grid_mouse_down` clears the marks selection on any press but a right one (timeline F155). Upstream's `MarksSelectionManager` is used only by the marks bar, the ruler and the form, never the docker, so nothing else is owed. |
| F290 | Delete selected | done | `delete_selected` |
| F291 | Add Multiple Effects | done | `S/dialogs.rs` `AddMultipleState`, `S/commands.rs` `add_multiple_*`, tested; `UI/SequencerSheets.swift` `AddMultipleSheet`. |
| F292 | Add effects on marks | done | `S/dialogs.rs` `AddMultipleState`, `S/commands.rs` `add_multiple_*`, tested; `UI/SequencerSheets.swift` `AddMultipleSheet`. |
| F293 | Distribute Equally | done | `distribute_equally_selected` |
| F294 | Distribute dialog | done | `S/effect_ops.rs` `distribute`, `S/state.rs` `apply_distribute`; `UI/SequencerSheets.swift` `DistributeSheet`. |
| F295 | Align to nearest marks | done | `align_to_marks`, `MarkIndex` |
| F296 | Add marks to effects | done | `S/commands.rs` `add_marks_to_effects`, `create_default_marks`, `enable_default_marks`, tested. Marks bar's own add-at-time prompt is the timeline's. |
| F297 | Add mark at time | done | `S/commands.rs` `add_marks_to_effects`, `create_default_marks`, `enable_default_marks`, tested. Marks bar's own add-at-time prompt is the timeline's. |
| F298 | Mark collection creation | done | `S/commands.rs` `add_marks_to_effects`, `create_default_marks`, `enable_default_marks`, tested. Marks bar's own add-at-time prompt is the timeline's. |
| F299 | Phoneme breakdown from marks bar | done | Pass 3. The timeline does the breakdown and asks this window for the phonemes: `UI/SequencerModel.swift` `timelinePhonemes(for:)`, served by `UI/SequencerLipSync.swift` `phonemes(for:)` from the marks worker's `word_phonemes`. A word the dictionary doesn't have (looked up as `RemovePunctuation(text.Trim())`, FFI `breakdown_clean_word`) puts up their `LyricFailureSheet` as the modal "Text Conversion Failure" sheet (`askForMapping`); OK writes the mapping to the user dictionary, even an empty one, which then skips the word (KEEP); Cancel skips the word. The dictionary is compiled in, so "Unable to find Standard Phoneme Dictionary" can't come up. Checked (`lip2.json`, hooks `sequencer.phonemes` and `sequencer.phonemesResult`): `hello=ETC AI L O`, `World!=WQ E L ETC`, an unknown word mapped with REST and WQ comes back `REST WQ` and is in `user_dictionary`. Difference: answers go back keyed by the word's text, so of two selected marks with the same unknown word the last answer serves both, where upstream would skip the first if cancelled and still break down the second. The same from the marks docker is the marks worker's. |
| F300 | Bulk mark events | done | Moves, deletes and pastes in the marks bar record their undo, and a text change commits with no undo (`TimelineModel::set_selected_marks_text` through `commit_marks`), all through the timeline. The docker's own edits are the marks spec's. |
| F301 | Lyric converter results | done | Pass 3. `S/state.rs` `paste_at_time` (the clipboard pasted at a given time, counted; tested), FFI `paste_at_time` and `cursor_ticks` (the timeline's cursor to the tick); `UI/SequencerLipSync.swift` `pasteAtTime`, handed to the converter with the cursor. Checked on Coldest Winter (`lip2.json`, hook `sequencer.state` and `sequencer.find`): "hello world" with Cursor placement adds 8 LipSync effects on the first row from 0 s at 0.25 s steps, undo reads `Paste 8 effects`, the sequence is modified; with the cursor moved to 30 s the next eight start there; Clipboard placement pastes nothing and shows "Convert Text" / "Conversion Complete and copied to Clipboard \nPaste at first Mark offset". Mark-aligned placements are the converter's own (marks F83). |
| F302 | Associate audio | done | `S/commands.rs`, `UI/SequencerTools.swift`, with F302's FIX. Checked through the harness on a private profile, never the show's Media. |
| F303 | Remove audio | done | `S/commands.rs`, `UI/SequencerTools.swift`, with F302's FIX. Checked through the harness on a private profile, never the show's Media. |
| F304 | Edit enablement from selection | done | `has_selection` in `ui_state` |
| F305 | Effect drop from the Effects panel | done | `S/drops.rs`, `S/effect_props.rs`, tested; `UI/SequencerDrops.swift`. A value drop runs through `testDrop` in the harness; a real drag can't be scripted. |
| F306 | Drop validity | done | `S/drops.rs`, `S/effect_props.rs`, tested; `UI/SequencerDrops.swift`. A value drop runs through `testDrop` in the harness; a real drag can't be scripted. |
| F307 | Colour drop on effect | done | `S/drops.rs`, `S/effect_props.rs`, tested; `UI/SequencerDrops.swift`. A value drop runs through `testDrop` in the harness; a real drag can't be scripted. |
| F308 | Curve drop on effect | done | `S/drops.rs`, `S/effect_props.rs`, tested; `UI/SequencerDrops.swift`. A value drop runs through `testDrop` in the harness; a real drag can't be scripted. |
| F309 | Gradient drop on effect | done | `S/drops.rs`, `S/effect_props.rs`, tested; `UI/SequencerDrops.swift`. A value drop runs through `testDrop` in the harness; a real drag can't be scripted. |
| F310 | Picker thumbnails | done | `UI/SequencerDrops.swift` `ParameterPicker`: popover at the pointer, 8 s timeout, Esc, grouping by owner or group. Seen and driven through the `sequencer.picker` hook (`target/shell-test3/picker.json`, snapshots `picker-color.png`, `picker-gradient.png`, `picker-curve.png`): a colour, a gradient and a curve dropped on an Alternating effect each opened it with two 64 pt bordered thumbnails under the group title `Gradients`, labelled `Gradient 1`/`Gradient 2` or `Curve 1`/`Curve 2`; choosing the second set it (undo text `1 Effect(s) Gradients modified`) and closed it; left alone it closed after 8 s with nothing applied. Not driven by script: Esc, the 48 pt colour swatch (no effect in the run offered several colour properties) and the placement at a real pointer. A script that hasn't called the hook still gets the first choice. |
| F311 | Drop files from Finder onto the timeline | done | `S/drops.rs` `plan_file_drop`, `add_file_effect`, tested; `SequencerShell::drop_files`. |
| F312 | Parameter picker window | done | `UI/SequencerDrops.swift` `ParameterPicker`: popover at the pointer, 8 s timeout, Esc, grouping by owner or group. Seen and driven through the `sequencer.picker` hook (`target/shell-test3/picker.json`, snapshots `picker-color.png`, `picker-gradient.png`, `picker-curve.png`): a colour, a gradient and a curve dropped on an Alternating effect each opened it with two 64 pt bordered thumbnails under the group title `Gradients`, labelled `Gradient 1`/`Gradient 2` or `Curve 1`/`Curve 2`; choosing the second set it (undo text `1 Effect(s) Gradients modified`) and closed it; left alone it closed after 8 s with nothing applied. Not driven by script: Esc, the 48 pt colour swatch (no effect in the run offered several colour properties) and the placement at a real pointer. A script that hasn't called the hook still gets the first choice. |
| F313 | Property discovery rules | done | `S/drops.rs`, `S/effect_props.rs`, tested; `UI/SequencerDrops.swift`. A value drop runs through `testDrop` in the harness; a real drag can't be scripted. |
| F314 | Library drag payload | done | `S/drops.rs`, `S/effect_props.rs`, tested; `UI/SequencerDrops.swift`. A value drop runs through `testDrop` in the harness; a real drag can't be scripted. |
| F315 | Clipboard format | done | `S/clipboard.rs`, `S/state.rs`. Private pasteboard in tests |
| F316 | Paste enable rules | done | `S/clipboard.rs`, `S/state.rs`. Private pasteboard in tests |
| F317 | Cut and Copy | done | `S/clipboard.rs`, `S/state.rs`. Private pasteboard in tests |
| F318 | Paste | done | `S/clipboard.rs`, `S/state.rs`. Private pasteboard in tests |
| F319 | Timeline tab | done | `TimelineRegion`: title follows the audio |
| F320 | Effects window | done | `UI/EffectsPanel.swift`, rows with `EffectDragSource`; clicking still chooses (checked by eye after the change). |
| F321 | Find Effects/Layers window | done | `S/find.rs`, `S/commands.rs` `find_*`, `UI/LayersFindPanels.swift` `FindPanel`. Live refresh follows `edit_count`, as the row events do. Pass 2 put the controls where upstream has them: Find combo and Collapse all Groups on the first row, the Available combo on its own row below, no added "Find" label. |
| F322 | Find Effects/Layers > Find combo | done | `S/find.rs`, `S/commands.rs` `find_*`, `UI/LayersFindPanels.swift` `FindPanel`. Live refresh follows `edit_count`, as the row events do. |
| F323 | Find Effects/Layers > Available combo | done | `S/find.rs`, `S/commands.rs` `find_*`, `UI/LayersFindPanels.swift` `FindPanel`. Live refresh follows `edit_count`, as the row events do. |
| F324 | Find Effects/Layers > Choose an item | done | `S/find.rs`, `S/commands.rs` `find_*`, `UI/LayersFindPanels.swift` `FindPanel`. Live refresh follows `edit_count`, as the row events do. |
| F325 | Find Effects/Layers > List | done | `S/find.rs`, `S/commands.rs` `find_*`, `UI/LayersFindPanels.swift` `FindPanel`. Live refresh follows `edit_count`, as the row events do. |
| F326 | Find Effects/Layers > Live refresh | done | `S/find.rs`, `S/commands.rs` `find_*`, `UI/LayersFindPanels.swift` `FindPanel`. Live refresh follows `edit_count`, as the row events do. |
| F327 | Find Effects/Layers > Double-click, or context menu Find all select... | done | `S/find.rs`, `S/commands.rs` `find_*`, `UI/LayersFindPanels.swift` `FindPanel`. Live refresh follows `edit_count`, as the row events do. |
| F328 | Mark Collections window | done | `UI/SequencerContent.swift` embeds VixenMarks' `MarkCollectionsPanel(session:)` behind `#if canImport(VixenMarks)`, with the placeholder otherwise. Its text fields keep their keys (the key monitor skips an `NSText` first responder); other keys reach the hotkeys. Seen with the real show's collections (pass 2, `sequencer.panel` hook). |
| F329 | Layers window | done | `S/layers.rs`, tested; `UI/LayersFindPanels.swift` `LayersPanel`. Configuration opens VixenDialogs' `LayerFilterSetupSheet`. Rename, quick rename and Type mark modified (F330's FIX). |
| F330 | Layers window content | done | `S/layers.rs`, tested; `UI/LayersFindPanels.swift` `LayersPanel`. Configuration opens VixenDialogs' `LayerFilterSetupSheet`. Rename, quick rename and Type mark modified (F330's FIX). |
| F331 | Layers > Toolbar | done | `S/layers.rs`, tested; `UI/LayersFindPanels.swift` `LayersPanel`. Configuration opens VixenDialogs' `LayerFilterSetupSheet`. Rename, quick rename and Type mark modified (F330's FIX). |
| F332 | Layers > Add Layer | done | `S/layers.rs`, tested; `UI/LayersFindPanels.swift` `LayersPanel`. Configuration opens VixenDialogs' `LayerFilterSetupSheet`. Rename, quick rename and Type mark modified (F330's FIX). Pass 2, as the marks spec's F53 and F56 have it: `Add Layer` and `Remove Layer` are two equal-width text buttons in a row under the Import/Export toolbar, and a new layer is neither selected nor opened. Checked with `target/shell-test3/layers.json` (added at the top as `Default - 2`, selected by a click, removed). |
| F333 | Layers > Remove Layer | done | `S/layers.rs`, tested; `UI/LayersFindPanels.swift` `LayersPanel`. Configuration opens VixenDialogs' `LayerFilterSetupSheet`. Rename, quick rename and Type mark modified (F330's FIX). |
| F334 | Layers > List | done | `S/layers.rs`, tested; `UI/LayersFindPanels.swift` `LayersPanel`. Configuration opens VixenDialogs' `LayerFilterSetupSheet`. Rename, quick rename and Type mark modified (F330's FIX). The default layer's Name is editable (marks spec F58, KEEP); its quick rename and Type stay hidden. |
| F335 | Layers > Quick rename | done | `S/layers.rs`, tested; `UI/LayersFindPanels.swift` `LayersPanel`. Configuration opens VixenDialogs' `LayerFilterSetupSheet`. Rename, quick rename and Type mark modified (F330's FIX). |
| F336 | Layers > Drag to reorder | done | `S/layers.rs`, tested; `UI/LayersFindPanels.swift` `LayersPanel`. Configuration opens VixenDialogs' `LayerFilterSetupSheet`. Rename, quick rename and Type mark modified (F330's FIX). |
| F337 | Layers > Export | done | `S/layers.rs`, tested; `UI/LayersFindPanels.swift` `LayersPanel`. Configuration opens VixenDialogs' `LayerFilterSetupSheet`. Rename, quick rename and Type mark modified (F330's FIX). |
| F338 | Layers > Import | done | `S/layers.rs`, tested; `UI/LayersFindPanels.swift` `LayersPanel`. Configuration opens VixenDialogs' `LayerFilterSetupSheet`. Rename, quick rename and Type mark modified (F330's FIX). Pass 3 (marks F65): the question about skipped layers is answered with `Continue`; "does not contain any layers that can be imported" shows as an error; the import uses the plan that was shown (FFI `layer_plan`), not a second read of the file. |
| F339 | Layers > Undo | done | `S/layers.rs`, tested; `UI/LayersFindPanels.swift` `LayersPanel`. Configuration opens VixenDialogs' `LayerFilterSetupSheet`. Rename, quick rename and Type mark modified (F330's FIX). |
| F340 | Layer export and import rules | done | `S/layers.rs`, with the `.v3l` keys and every message. Tested. |
| F341 | Color Library window | done | `S/libraries.rs` palette and `.vfc`, tested; `UI/LibraryPanels.swift` `LibraryPanel`. |
| F342 | Color Library > Selection | done | `S/libraries.rs` palette and `.vfc`, tested; `UI/LibraryPanels.swift` `LibraryPanel`. |
| F343 | Color Library > New | done | `S/libraries.rs` palette and `.vfc`, tested; `UI/LibraryPanels.swift` `LibraryPanel`. |
| F344 | Color Library > Edit | done | `S/libraries.rs` palette and `.vfc`, tested; `UI/LibraryPanels.swift` `LibraryPanel`. |
| F345 | Color Library > Delete | done | `S/libraries.rs` palette and `.vfc`, tested; `UI/LibraryPanels.swift` `LibraryPanel`. |
| F346 | Color Library > Drag out | partial | `UI/LibraryPanels.swift` `LibraryCellView`, `LibraryDropCatcher`, `LibraryDropTracker`: drag out with the payload; a drop counts only more than 10 pt from where the drag came in; the whole selection moves to the item under the pointer in upstream's order (`moveSelection`, on `libraryMoveCurves`/`libraryMoveGradients` and, for colours, `S/libraries.rs` `moved_by_position`, tested); a move onto empty space goes last; a curve or gradient from elsewhere is named, then placed at the pointer's index (`libraryInsertCurve`/`libraryInsertGradient`). The reorder is checked through the `sequencer.libraryMove` hook (`target/shell-test3/move.json`: order read back and seen in the panel). Missing: the drag gesture itself and the 10 pt rule, which the harness can't drive. Check by hand. |
| F347 | Color Library > Drop in | partial | `UI/LibraryPanels.swift` `LibraryCellView`, `LibraryDropCatcher`, `LibraryDropTracker`: drag out with the payload; a drop counts only more than 10 pt from where the drag came in; the whole selection moves to the item under the pointer in upstream's order (`moveSelection`, on `libraryMoveCurves`/`libraryMoveGradients` and, for colours, `S/libraries.rs` `moved_by_position`, tested); a move onto empty space goes last; a curve or gradient from elsewhere is named, then placed at the pointer's index (`libraryInsertCurve`/`libraryInsertGradient`). The reorder is checked through the `sequencer.libraryMove` hook (`target/shell-test3/move.json`: order read back and seen in the panel). Missing: the drag gesture itself and the 10 pt rule, which the harness can't drive. Check by hand. |
| F348 | Color Library > ⌘-scroll | done | `UI/LibraryPanels.swift` `ScaleCatcher`: ⌘-scroll and pinch, 1.03 a step, floors 0.1 and 0.2, saved a second after the last step. Pass 2 found it never heard a wheel (it sat behind the scroll view, and every tab's copy was alive at once) and that ⇧ turns a wheel's travel sideways; it now listens with an event monitor over its own area, only in the tab that's showing. Checked with wheel events through the app's queue (`sequencer.wheel`, `sequencer.panelScale`, `target/shell-test3/scale.json`): ten ⌘ steps took the curve image scale to 1.3439 and ten ⌘⇧ steps the text scale to the same, both seen in the snapshot; a plain wheel changed nothing; the same steps back gave 1.0; in the colour panel ⌘⇧ scaled the image (1.1593 after five). Not driven: pinch, and the floors. |
| F349 | Color Library > Export | done | `S/libraries.rs` palette and `.vfc`, tested; `UI/LibraryPanels.swift` `LibraryPanel`. |
| F350 | Color Library > Persist | done | `S/libraries.rs` palette and `.vfc`, tested; `UI/LibraryPanels.swift` `LibraryPanel`. |
| F351 | Curve Library window | done | `UI/LibraryPanels.swift`, VixenKit's editors and name loops, `S/libraries.rs` `.vcl` and `.vgl`, tested. |
| F352 | Curve Library > Selection | done | `UI/LibraryPanels.swift`, VixenKit's editors and name loops, `S/libraries.rs` `.vcl` and `.vgl`, tested. |
| F353 | Curve Library > Edit | done | `UI/LibraryPanels.swift`, VixenKit's editors and name loops, `S/libraries.rs` `.vcl` and `.vgl`, tested. |
| F354 | Curve Library > New | done | `UI/LibraryPanels.swift`, VixenKit's editors and name loops, `S/libraries.rs` `.vcl` and `.vgl`, tested. |
| F355 | Curve Library > Delete | done | `UI/LibraryPanels.swift`, VixenKit's editors and name loops, `S/libraries.rs` `.vcl` and `.vgl`, tested. |
| F356 | Curve Library > Drag out | partial | `UI/LibraryPanels.swift` `LibraryCellView`, `LibraryDropCatcher`, `LibraryDropTracker`: drag out with the payload; a drop counts only more than 10 pt from where the drag came in; the whole selection moves to the item under the pointer in upstream's order (`moveSelection`, on `libraryMoveCurves`/`libraryMoveGradients` and, for colours, `S/libraries.rs` `moved_by_position`, tested); a move onto empty space goes last; a curve or gradient from elsewhere is named, then placed at the pointer's index (`libraryInsertCurve`/`libraryInsertGradient`). The reorder is checked through the `sequencer.libraryMove` hook (`target/shell-test3/move.json`: order read back and seen in the panel). Missing: the drag gesture itself and the 10 pt rule, which the harness can't drive. Check by hand. |
| F357 | Curve Library > Drop in | partial | `UI/LibraryPanels.swift` `LibraryCellView`, `LibraryDropCatcher`, `LibraryDropTracker`: drag out with the payload; a drop counts only more than 10 pt from where the drag came in; the whole selection moves to the item under the pointer in upstream's order (`moveSelection`, on `libraryMoveCurves`/`libraryMoveGradients` and, for colours, `S/libraries.rs` `moved_by_position`, tested); a move onto empty space goes last; a curve or gradient from elsewhere is named, then placed at the pointer's index (`libraryInsertCurve`/`libraryInsertGradient`). The reorder is checked through the `sequencer.libraryMove` hook (`target/shell-test3/move.json`: order read back and seen in the panel). Missing: the drag gesture itself and the 10 pt rule, which the harness can't drive. Check by hand. |
| F358 | Curve Library > ⌘-scroll | done | `UI/LibraryPanels.swift` `ScaleCatcher`: ⌘-scroll and pinch, 1.03 a step, floors 0.1 and 0.2, saved a second after the last step. Pass 2 found it never heard a wheel (it sat behind the scroll view, and every tab's copy was alive at once) and that ⇧ turns a wheel's travel sideways; it now listens with an event monitor over its own area, only in the tab that's showing. Checked with wheel events through the app's queue (`sequencer.wheel`, `sequencer.panelScale`, `target/shell-test3/scale.json`): ten ⌘ steps took the curve image scale to 1.3439 and ten ⌘⇧ steps the text scale to the same, both seen in the snapshot; a plain wheel changed nothing; the same steps back gave 1.0; in the colour panel ⌘⇧ scaled the image (1.1593 after five). Not driven: pinch, and the floors. |
| F359 | Curve Library > Export | done | `UI/LibraryPanels.swift`, VixenKit's editors and name loops, `S/libraries.rs` `.vcl` and `.vgl`, tested. |
| F360 | Curve Library > Import | done | `UI/LibraryPanels.swift`, VixenKit's editors and name loops, `S/libraries.rs` `.vcl` and `.vgl`, tested. |
| F361 | Curve Library > Change raises `CurveLibraryChanged`, rebuilding the... | done | `UI/LibraryPanels.swift`, VixenKit's editors and name loops, `S/libraries.rs` `.vcl` and `.vgl`, tested. |
| F362 | Add curve to library | done | `UI/LibraryPanels.swift`, VixenKit's editors and name loops, `S/libraries.rs` `.vcl` and `.vgl`, tested. |
| F363 | Gradient Library window | done | `UI/LibraryPanels.swift`, VixenKit's editors and name loops, `S/libraries.rs` `.vcl` and `.vgl`, tested. |
| F364 | Color Collection Editor | done | `S/libraries.rs` `CollectionEditor` on a deep copy (F374), tested; `UI/SequencerSheets.swift` `ColorCollectionsSheet`. Sheet checked by eye; add, reorder and import are Rust-tested. |
| F365 | Color Collection Editor > Picking a collection | done | `S/libraries.rs` `CollectionEditor` on a deep copy (F374), tested; `UI/SequencerSheets.swift` `ColorCollectionsSheet`. Sheet checked by eye; add, reorder and import are Rust-tested. |
| F366 | Color Collection Editor > New | done | `S/libraries.rs` `CollectionEditor` on a deep copy (F374), tested; `UI/SequencerSheets.swift` `ColorCollectionsSheet`. Sheet checked by eye; add, reorder and import are Rust-tested. |
| F367 | Color Collection Editor > Delete | done | `S/libraries.rs` `CollectionEditor` on a deep copy (F374), tested; `UI/SequencerSheets.swift` `ColorCollectionsSheet`. Sheet checked by eye; add, reorder and import are Rust-tested. |
| F368 | Color Collection Editor > Add Color | done | `S/libraries.rs` `CollectionEditor` on a deep copy (F374), tested; `UI/SequencerSheets.swift` `ColorCollectionsSheet`. Sheet checked by eye; add, reorder and import are Rust-tested. |
| F369 | Color Collection Editor > Remove Color | done | `S/libraries.rs` `CollectionEditor` on a deep copy (F374), tested; `UI/SequencerSheets.swift` `ColorCollectionsSheet`. Sheet checked by eye; add, reorder and import are Rust-tested. |
| F370 | Color Collection Editor > Description | done | `S/libraries.rs` `CollectionEditor` on a deep copy (F374), tested; `UI/SequencerSheets.swift` `ColorCollectionsSheet`. Sheet checked by eye; add, reorder and import are Rust-tested. |
| F371 | Color Collection Editor > Reorder | done | `S/libraries.rs` `CollectionEditor` on a deep copy (F374), tested; `UI/SequencerSheets.swift` `ColorCollectionsSheet`. Sheet checked by eye; add, reorder and import are Rust-tested. |
| F372 | Color Collection Editor > Export | done | `S/libraries.rs` `CollectionEditor` on a deep copy (F374), tested; `UI/SequencerSheets.swift` `ColorCollectionsSheet`. Sheet checked by eye; add, reorder and import are Rust-tested. |
| F373 | Color Collection Editor > Close with changes | done | `S/libraries.rs` `CollectionEditor` on a deep copy (F374), tested; `UI/SequencerSheets.swift` `ColorCollectionsSheet`. Sheet checked by eye; add, reorder and import are Rust-tested. |
| F374 | Color Collection Editor > Quirk (data): | done | `S/libraries.rs` `CollectionEditor` on a deep copy (F374), tested; `UI/SequencerSheets.swift` `ColorCollectionsSheet`. Sheet checked by eye; add, reorder and import are Rust-tested. |
| F375 | Effect Editor window | done | `UI/SequencerContent.swift` embeds `EffectEditorPane`, fed the timeline's selection 200 ms behind it (`SequencerModel.timelineSelectionChanged`); undo entries it records refresh the window. Keys reach the hotkeys except from a text field or a focused editor control (`EditorKeys.focusedEditorTakes`). Real playback takes the engine from the Effect Preview, which comes back when it's free (`ShowPlayback::play`, the panel's `previewPoll`). Export's stop and resume is F37's, through `VixenCore.holdEffectPreview(held:)`. |
| F376 | Row labels | done | Drawn by the timeline's row list: `UI/VixenTimeline/Painters.swift` `paintRowsTile`, `paintExpander` (timeline F21, F23). Tag dots: see the timeline's table. |
| F377 | Effect element | done | Effect times and targets live in the session's `EffectEntry`, and the timeline pushes through it: `vixen_session::timeline::session_link`. |
| F378 | Manage Tags | done | `ElementTagManagerPresenter` registers the existing Setup color sheet with save-on-close. `AppCore::save_element_tag_colors` saves only tag definitions, preserves playback/user tags and rolls back live colors on write failure. Nine tag tests and 30 native UI steps pass: Cancel discards, Save persists, reopening shows the saved color, looping playback continues, sequence stays unmodified, and non-tag SystemConfig data is unchanged (`target/progress-ui/tags-out`) |
| F379 | Edit Time dialog | done | `S/dialogs.rs` `EffectTimeEditorState`, tested; `UI/SequencerSheets.swift` `EditTimeSheet`, `TimeBox`, checked by keys through the harness. |
| F380 | Bulk Effects Move dialog | done | `S/commands.rs` `bulk_move`, `UI/SequencerSheets.swift` `BulkMoveSheet`, sheet checked. The marks get the offset the grid really applied: `FFI` `TimelineGrid::move_in_range_by_time` returns it from `TimelineController::move_in_range_reporting` (`crates/vixen-ffi/src/timeline.rs`). Checked on the real show with the `sequencer.bulkMove` hook (`target/shell-test3/bulk.json`): 100 s back without Clip moved 587 effects and 1391 marks by the 313 ms there was room for. |
| F381 | Effect Distribution dialog | done | `S/effect_ops.rs` `distribute`, `S/state.rs` `apply_distribute`; `UI/SequencerSheets.swift` `DistributeSheet`. |
| F382 | Sequence Length dialog | done | `UI/SequencerSheets.swift` `SequenceLengthSheet`, `S/state.rs` `set_sequence_length`. |
| F383 | Missing Audio File dialog | done | `UI/SequencerSheets.swift` `MissingAudioSheet`: Select New Audio is the default, the three verbs and the path. Forget checked through the harness on a private profile. |
| F384 | Add Multiple Effects dialog | done | `S/dialogs.rs` `AddMultipleState`, `S/commands.rs` `add_multiple_*`, tested; `UI/SequencerSheets.swift` `AddMultipleSheet`. |
| F385 | Add Multiple Effects > Limits | done | `S/dialogs.rs` `AddMultipleState`, `S/commands.rs` `add_multiple_*`, tested; `UI/SequencerSheets.swift` `AddMultipleSheet`. |
| F386 | Add Multiple Effects > Possible count, not aligned (`:396-419`) | done | `S/dialogs.rs` `AddMultipleState`, `S/commands.rs` `add_multiple_*`, tested; `UI/SequencerSheets.swift` `AddMultipleSheet`. |
| F387 | Add Multiple Effects > Possible count, aligned (`:421-445`) | done | `S/dialogs.rs` `AddMultipleState`, `S/commands.rs` `add_multiple_*`, tested; `UI/SequencerSheets.swift` `AddMultipleSheet`. |
| F388 | Add Multiple Effects > Recalculates on any time change and on any o... | done | `S/dialogs.rs` `AddMultipleState`, `S/commands.rs` `add_multiple_*`, tested; `UI/SequencerSheets.swift` `AddMultipleSheet`. |
| F389 | Add Multiple Effects > Double-click the possible label | done | `S/dialogs.rs` `AddMultipleState`, `S/commands.rs` `add_multiple_*`, tested; `UI/SequencerSheets.swift` `AddMultipleSheet`. |
| F390 | Add Multiple Effects > Align to beat marks | done | `S/dialogs.rs` `AddMultipleState`, `S/commands.rs` `add_multiple_*`, tested; `UI/SequencerSheets.swift` `AddMultipleSheet`. |
| F391 | Add Multiple Effects > Check rules | done | `S/dialogs.rs` `AddMultipleState`, `S/commands.rs` `add_multiple_*`, tested; `UI/SequencerSheets.swift` `AddMultipleSheet`. |
| F392 | Add Multiple Effects > OK (`:208-230`) | done | `S/dialogs.rs` `AddMultipleState`, `S/commands.rs` `add_multiple_*`, tested; `UI/SequencerSheets.swift` `AddMultipleSheet`. |
| F393 | Add Multiple Effects > Restoring checked marks with Shift (F291) ru... | done | `S/dialogs.rs` `AddMultipleState`, `S/commands.rs` `add_multiple_*`, tested; `UI/SequencerSheets.swift` `AddMultipleSheet`. |
| F394 | Create Evenly Divided Marks dialog | done | `S/commands.rs` `create_even_marks`, `UI/SequencerSheets.swift` `EvenMarksSheet`, error and reopen checked. |
| F395 | Text dialog | done | `UI/SequencerSheets.swift` `TextPromptSheet`, `KitTextPrompt`. |
| F396 | Export dialog | done | The scheduler and export worker's Export Sequence sheet (`app/Sources/VixenScheduler/ExportSequenceSheet.swift`, `crates/vixen-session/src/export_dialog.rs`), their spec's F1 and F2; see `docs/parity/shows-scheduler-export.md`. Reached from Sequence > Export and the File strip (F37, F137). |
| F397 | Export > Load | done | The scheduler and export worker's Export Sequence sheet (`app/Sources/VixenScheduler/ExportSequenceSheet.swift`, `crates/vixen-session/src/export_dialog.rs`), their spec's F3 to F5; see `docs/parity/shows-scheduler-export.md`. Reached from Sequence > Export and the File strip (F37, F137). |
| F398 | Export > Check or drag reorder | done | The scheduler and export worker's Export Sequence sheet (`app/Sources/VixenScheduler/ExportSequenceSheet.swift`, `crates/vixen-session/src/export_dialog.rs`), their spec's F5; see `docs/parity/shows-scheduler-export.md`. Reached from Sequence > Export and the File strip (F37, F137). |
| F399 | Export > Start (`ExportDialog.cs:114-201`) | done | The scheduler and export worker's Export Sequence sheet (`app/Sources/VixenScheduler/ExportSequenceSheet.swift`, `crates/vixen-session/src/export_dialog.rs`), their spec's F9 and F12; see `docs/parity/shows-scheduler-export.md`. Reached from Sequence > Export and the File strip (F37, F137). |
| F400 | Export > Stop | done | The scheduler and export worker's Export Sequence sheet (`app/Sources/VixenScheduler/ExportSequenceSheet.swift`, `crates/vixen-session/src/export_dialog.rs`), their spec's F10 and F13; see `docs/parity/shows-scheduler-export.md`. Reached from Sequence > Export and the File strip (F37, F137). |
| F401 | Export > States (`ExportDialog.cs:332-359`) | done | The scheduler and export worker's Export Sequence sheet (`app/Sources/VixenScheduler/ExportSequenceSheet.swift`, `crates/vixen-session/src/export_dialog.rs`), their spec's F11; see `docs/parity/shows-scheduler-export.md`. Reached from Sequence > Export and the File strip (F37, F137). |
| F402 | Export > Enable rules (`ExportDialog.cs:434-483`) | done | The scheduler and export worker's Export Sequence sheet (`app/Sources/VixenScheduler/ExportSequenceSheet.swift`, `crates/vixen-session/src/export_dialog.rs`), their spec's F3, F7 and F8; see `docs/parity/shows-scheduler-export.md`. Reached from Sequence > Export and the File strip (F37, F137). |
| F403 | Effect Defaults export selection | done | `vixen_io::effect_defaults` `export`, `import`, `write_diagnostic_dump`; `S/commands.rs`; `UI/SequencerTools.swift`; `EffectDefaultsExportSheet`. |
| F404 | Papagayo file format | done | `S/papagayo.rs`, `S/commands.rs` `papagayo_import`, tested; a bad file shows its error (GUARD). |
| F405 | `EffectsAddedUndoAction` | done | `undo.rs` wording kept with each entry so Redo shows the original's text |
| F406 | `EffectsPastedUndoAction` | done | `undo.rs` wording kept with each entry so Redo shows the original's text |
| F407 | `EffectsCutUndoAction` | done | `undo.rs` wording kept with each entry so Redo shows the original's text |
| F408 | `EffectsRemovedUndoAction` | done | `undo.rs` wording kept with each entry so Redo shows the original's text |
| F409 | `ElementsTimeChangedUndoAction` | done | `undo.rs` wording kept with each entry so Redo shows the original's text |
| F410 | `EffectsLayerChangedUndoAction` | done | `undo.rs` wording kept with each entry so Redo shows the original's text |
| F411 | `EffectsModifiedUndoAction` | omitted | Nothing raises it upstream |
| F412 | `EffectsPropertyModifiedUndoAction` | done | `undo.rs` variants, raised where the commands are in |
| F413 | `MarksAddedUndoAction` | done | `undo.rs` variants, raised where the commands are in |
| F414 | `MarksRemovedUndoAction` | done | `undo.rs` variants, raised where the commands are in |
| F415 | `MarksTimeChangedUndoAction` | done | `undo.rs` variants, raised where the commands are in |
| F416 | `MarksBulkChangeUndoAction` | done | `undo.rs` variants, raised where the commands are in |
| F417 | Layer lost on undo (quirk) | done | Done in `undo.rs` |
| F418 | Actions with no undo: | done | None of these push an undo entry: `S/commands.rs` audio and marks, `S/layers.rs`, `S/libraries.rs`. |
| F419 | Undo stack rules | done | `undo.rs`, `UndoSplitButton` |
| F420 | Undo and Redo buttons | done | `undo.rs`, `UndoSplitButton` |
| F421 | Undo wiring | done | `undo.rs`, `UndoSplitButton` |
| F422 | Add Multiple Effects > Typing the count | done | `S/dialogs.rs` `AddMultipleState`, `S/commands.rs` `add_multiple_*`, tested; `UI/SequencerSheets.swift` `AddMultipleSheet`. |
| F423 | Playback range, in one place | done | `S/transport.rs` `remembered`, `ruler_*`, `S/state.rs` `context_ended`. The timeline owns the remembered range (`Timeline::remembered_playback_range` in `crates/vixen-session/src/timeline/ruler.rs`), and `TimelineGrid::remembered_range` hands it to F82 and F291. |

## Handover

Pass 1 built the window and every group in the spec except the parts that wait on other workers. Checked by Rust tests (`cargo test -p vixen-session --lib sequencer`, about 185) and by harness scripts in `target/shell-test/*.json` against a copy of the real show (`target/show-shell`), reading Rust state with Sequence > Write Test State. Scripts that touch audio use a private profile (`target/shell-test/profile2`), because `target/show-shell/Media` links to the owner's real Media folder: never associate audio against `show-shell`.

Harness-only items under Sequence (shown only when `VIXEN_UI_TEST` is set): Test Context Menu (runs an item of the last right-click menu, which is written to `menu-context.txt` instead of shown), Test Keys (each hotkey through `key_down`), Test Pointer, Test Shift, Test Drop Red/Gradient/Curve (onto the first selected effect), Test Drop Pulse On Row and Test Drop File On Row (`VIXEN_TEST_FILE`), Test Color Red (answers the colour panel). File panels read `VIXEN_TEST_FILE` under the harness.

What other workers need to provide, collected from the rows above:
- Timeline: nothing more. Pass 2 added what was missing (see the note at the top).
- Effect editor: preview start, stop and resume around playback and Export (F375, F37).
- Marks, layers and analysis: the LipSync windows (F87-F90, F146, F301) and the phoneme breakdown from the docker (F299). The docker and the Beats and Bars sheet are in (F328, F80).
- Display Setup: `ElementTagManagerWindow` for Manage Tags (F378).
- Scheduler and export: the Export dialog at `WindowID.exportWizard`, given the sequence path (F37, F396-F402), and `ElementMapperView` for F12.

## Control index

Section O of the spec, control by control, with the status of its F numbers. "layout" is a container or separator with no behaviour of its own, carried by the strip or menu it belongs to.

| Control | Text | F | Status |
|---|---|---|---|
| `menuStrip` | Menu | F77 | done |
| `sequenceToolStripMenuItem` | Sequence | F24 | done |
| `playbackToolStripMenuItem` | Playback | F27 | done |
| `playOptionsToolStripMenuItem` | Play Options | F31 | done |
| `editToolStripMenuItem` | Edit | F316 | done |
| `viewToolStripMenuItem` | View | F54 | done |
| `toolsToolStripMenuItem` | Tools | F78 | done |
| `audioToolStripMenuItem` | Audio | F78 | done |
| `helpDocumentationToolStripMenuItem` | Help | F96 | done |
| `toolStripAudio` | Audio | F99 | done |
| `contextMenuStripAll` |  | F100 | done |
| `add_RemoveContextToolStripMenuItem` | Show/Hide Items | F101 | done |
| `toolbarsToolStripMenuItem` | Toolbars | F102 | done |
| `imageSizeToolStripMenuItem` | Toggle Image Size | F103 | done |
| `resetToolStripMenuItem` | Reset Toolbar | F104 | done |
| `toolStripButton_AssociateAudio` | Associate Audio | F128 | done |
| `toolStripButton_DecreaseTimingSpeed` | Decrease Timing speed | F129 | done |
| `audioToolStripLabel_TimingSpeed` | Timing Speed: 100% | F130 | done |
| `toolStripButton_IncreaseTimingSpeed` | Increasing Timing speed | F131 | done |
| `audioToolStripButton_Audio_Devices` | Audio Devices | F132 | done |
| `toolStripMenuItem_Save` | Save | F24 | done |
| `toolStripMenuItem_SaveAs` | Save As... | F25 | done |
| `autoSaveToolStripMenuItem` | Auto Save | F26 | done |
| `playToolStripMenuItem` | Play | F27 | done |
| `pauseToolStripMenuItem` | Pause | F28 | done |
| `stopToolStripMenuItem` | Stop | F29 | done |
| `toolStripMenuItem_Loop` | Loop | F30 | done |
| `delayOffToolStripMenuItem` | Delay Off | F31 | done |
| `delay5SecondsToolStripMenuItem` | Delay 5 Seconds | F32 | done |
| `delay10SecondsToolStripMenuItem` | Delay 10 Seconds | F33 | done |
| `delay20SecondsToolStripMenuItem` | Delay 20 Seconds | F34 | done |
| `delay30SecondsToolStripMenuItem` | Delay 30 Seconds | F35 | done |
| `delay60SecondsToolStripMenuItem` | Delay 60 Seconds | F36 | done |
| `exportToolStripMenuItem` | Export | F37 | done |
| `toolStripMenuItem_Close` | Close | F38 | done |
| `addEffectToolStripMenuItem` | Add Effect | F39 | done |
| `toolStripMenuItem_Cut` | Cut | F40 | done |
| `toolStripMenuItem_Copy` | Copy | F41 | done |
| `toolStripMenuItem_Paste` | Paste | F42 | done |
| `toolStripMenuItem_PasteSpecial` | Paste Special | F43 | done |
| `toolStripMenuItem_PasteInvert` | Invert | F43 | done |
| `toolStripMenuItem_PasteToMarks` | Visible Marks | F44 | done |
| `undoToolStripMenuItem` | Undo | F45 | done |
| `redoToolStripMenuItem` | Redo | F46 | done |
| `selectAllElementsToolStripMenuItem` | Select All Elements | F47 | done |
| `toolStripMenuItem_deleteElements` | Delete | F48 | done |
| `toolStripMenuItem_SnapTo` | Snap To Marks / Effects | F49 | done |
| `toolStripMenuItem_ResizeIndicator` | Resize / Draw Indicator | F50 | done |
| `toolStripMenuItem_RIColor_Blue` | Blue | F51 | done |
| `toolStripMenuItem_RIColor_Yellow` | Yellow | F51 | done |
| `toolStripMenuItem_RIColor_Green` | Green | F51 | done |
| `toolStripMenuItem_RIColor_White` | White | F51 | done |
| `toolStripMenuItem_RIColor_Red` | Red | F51 | done |
| `cADStyleSelectionBoxToolStripMenuItem` | CAD Style Selection Box | F52 | done |
| `legacyCursorActiveRowToolStripMenuItem` | Legacy Cursor / Active Row | F53 | done |
| `toolStripMenuItem_zoomTimeIn` | Zoom Time In | F54 | done |
| `toolStripMenuItem_zoomTimeOut` | Zoom Time Out | F55 | done |
| `zoomUnderMousePositionToolStripMenuItem` | Zoom Under Mouse Position | F56 | done |
| `toolStripMenuItem_zoomRowsIn` | Zoom Rows In | F57 | done |
| `toolStripMenuItem_zoomRowsOut` | Zoom Rows Out | F58 | done |
| `toolStripMenuItem2` | Collapse All Element Groups | F59 | done |
| `toolStripMenuItem3` | Reset Row Height to Default | F60 | done |
| `highlightRowsWithEffectsToolStripMenuItem` | Highlight Rows With Effects | F61 | done |
| `showEffectInfoToolStripMenuItem` | Show Effect Info | F62 | done |
| `showHiddenToolStripMenuItem` | Show Hidden Rows | F63 | done |
| `fullWaveformToolStripMenuItem` | Full Waveform | F64 | done |
| `lockRulerHeightToolStripMenuItem` | Lock Ruler Height | F65 | done |
| `lockWaveformHeightToolStripMenuItem` | Lock Waveform Height | F66 | done |
| `toolbarToolStripMenuItem` | Toolbars | F67 | done |
| `effectWindowToolStripMenuItem` | Effect Window | F68 | done |
| `markWindowToolStripMenuItem` | Mark Window | F69 | done |
| `gridWindowToolStripMenuItem` | Timeline Window | F76 | done |
| `effectEditorWindowToolStripMenuItem` | Effect Editor Window | F70 | done |
| `mixingFilterEditorWindowToolStripMenuItem` | Layer Editor Window | F71 | done |
| `toolStripMenuItemFindEffects` | Find Effects or Layers | F72 | done |
| `toolStripMenuItemColorLibrary` | Color Library Window | F73 | done |
| `toolStripMenuItemGradientLibrary` | Gradient Library Window | F74 | done |
| `toolStripMenuItemCurveLibrary` | Curve Library Window | F75 | done |
| `toolStripMenuItem_associateAudio` | Associate Audio... | F78 | done |
| `toolStripMenuItem_removeAudio` | Remove Audio | F79 | done |
| `beatBarDetectionToolStripMenuItem` | Beat/Bar Detection | F80 | done |
| `divideMarksEvenlyToolStripMenuItem` | Create Evenly Divided Marks | F82 | done |
| `speedTempoToolStripMenuItem` | Speed / Tempo | F81 | done |
| `modifySequenceLengthToolStripMenuItem` | Sequence Length... | F83 | done |
| `curveEditorToolStripMenuItem` | Curve Editor | F84 | done |
| `colorGradientEditorToolStripMenuItem` | Color Gradient Editor | F85 | done |
| `ColorCollectionsToolStripMenuItem` | Color Collection Editor | F86 | done |
| `lipSyncMappingsToolStripMenuItem` | LipSync | F87 | done |
| `defaultMapToolStripMenuItem` | Default Image Map | F87 | done |
| `phonemeMappingsToolStripMenuItem` | Edit Image Maps | F88 | done |
| `changeMapToolStripMenuItem` | Edit Element Face Mapping | F89 | done |
| `lyricConverterToolStripMenuItem` | Lyric Converter | F90 | done |
| `papagayoImportToolStripMenuItem` | Papagayo Import | F91 | done |
| `bulkEffectMoveToolStripMenuItem` | Bulk Effects Move | F92 | done |
| `effectDefaultsToolStripMenuItem` | Effect Defaults | F93 | done |
| `exportEffectDefaultsToolStripMenuItem` | Export... | F93 | done |
| `importEffectDefaultsToolStripMenuItem` | Import... | F94 | done |
| `dumpEffectDefaultsToolStripMenuItem` | Dump (Readable XML)... | F95 | done |
| `onlineHelpToolStripMenuItem` | Online Help | F96 | done |
| `vixenYouTubeChannelToolStripMenuItem` | Vixen YouTube Channel | F97 | done |
| `timerPlaying` |  | F267 | done |
| `statusStrip` | statusStrip1 | F256 | done |
| `toolStripStatusLabel2` | Current Position: | F257 | done |
| `toolStripStatusLabel_currentTime` | 0:00.000 | F257 | done |
| `toolStripStatusLabel1` | Sequence Length: | F258 | done |
| `toolStripStatusLabel_sequenceLength` | 0:00.000 | F258 | done |
| `toolStripStatusLabel3` | Play Delayed: | F259 | done |
| `toolStripStatusLabel_delayPlay` | 00 Seconds | F259 | done |
| `toolStripStatusLabel4` |  | F260 | done |
| `toolStripStatusLabel_RenderingElements` | Rendering Elements: | F261 | done |
| `toolStripProgressBar_RenderingElements` |  | F261 | done |
| `openFileDialog` |  | F302 | done |
| `toolStripContainer` | toolStripContainer1 | F98 | done |
| `dockPanel` |  | F2 | done |
| `toolStripAlignment` | Alignment | F99 | done |
| `alignmentToolStripButton_CloseGaps` | Close Gaps | F139 | done |
| `alignmentToolStripDropDownButton_CloseGaps` | Close Gaps | F140 | done |
| `alignmentToolStripMenuItemCloseGap100` | .100 | F140 | done |
| `alignmentToolStripMenuItemCloseGap200` | .200 | F140 | done |
| `alignmentToolStripMenuItemCloseGap300` | .300 | F140 | done |
| `alignmentToolStripMenuItemCloseGap400` | .400 | F140 | done |
| `alignmentToolStripDropDownButton_AlignTo` | Align To | F141 | done |
| `alignmentToolStripMenuItem_AlignTo200` | .200 | F141 | done |
| `alignmentToolStripMenuItem_AlignTo400` | .400 | F141 | done |
| `alignmentToolStripMenuItem_AlignTo600` | .600 | F141 | done |
| `alignmentToolStripMenuItem_AlignTo800` | .800 | F141 | done |
| `alignmentToolStripMenuItem_AlignTo1000` | 1.000 | F141 | done |
| `alignmentToolStripMenuItem_AlignTo2000` | 2.000 | F141 | done |
| `alignmentToolStripButton_Start` | Start Mark | F142 | done |
| `alignmentToolStripButton_End` | End Mark | F143 | done |
| `alignmentToolStripButton_Both` | Both Marks | F144 | done |
| `alignmentToolStripButton_Distribute` | Distribute Mark | F145 | done |
| `toolStripEffects` | Effects | F99 | done |
| `contextMenuStripEffect` |  | F151 | done |
| `effectGroupsToolStripMenuItem` | Show/Hide Effects | F152 | done |
| `basicToolStripMenuItem` | Basic | F153 | done |
| `pixelToolStripMenuItem` | Pixel | F154 | done |
| `deviceToolStripMenuItem` | Device | F155 | done |
| `intelligentFixtureToolStripMenuItem` | Fixture | F156 | done |
| `toolStripMenuItemLabelPosition` | Label Postion | F157 | done |
| `noLabelToolStripMenuItem` | No Label | F158 | done |
| `aboveImageToolStripMenuItem` | Above Image | F159 | done |
| `belowImageToolStripMenuItem` | Below Image | F160 | done |
| `toolbarsToolStripMenuItem_Effect` | Toolbars | F161 | done |
| `resetEffectsToolStripMenuItem` | Reset Toolbar | F162 | done |
| `toolStripView` | View | F99 | done |
| `viewToolStripButton_ZoomTimeIn` | Zoom Time In | F124 | done |
| `viewToolStripButton_ZoomTimeOut` | Zoom Time Out | F125 | done |
| `viewToolStripButton_ZoomRowIn` | Zoom Row In | F126 | done |
| `viewToolStripButton_ZoomRowOut` | Zoom Row Out | F127 | done |
| `toolStripFile` | File | F99 | done |
| `fileToolStripButton_Save` | Save | F134 | done |
| `fileToolStripButton_SaveAs` | Save As | F135 | done |
| `fileToolStripButton_AutoSave` | Auto Save | F136 | done |
| `fileToolStripButton_Export` | Export | F137 | done |
| `fileToolStripButton_Close` | Close | F138 | done |
| `toolStripEdit` | Edit | F99 | done |
| `editToolStripButton_Undo` | Undo | F110 | done |
| `editToolStripButton_Redo` | Redo | F111 | done |
| `editToolStripButton_Cut` | Cut | F112 | done |
| `editToolStripButton_Copy` | Copy | F113 | done |
| `editToolStripButton_Paste` | Paste | F114 | done |
| `editToolStripButton_PasteDropDown` |  | F115 | done |
| `editToolStripButton_PasteInvert` | Paste - Invert | F116 | done |
| `editToolStripButton_PasteVisibleMarks` | Paste - Visible Mark/s | F117 | done |
| `toolStripMode` | Mode | F99 | done |
| `modeToolStripButton_DrawMode` | Draw Mode | F118 | done |
| `modeToolStripButton_SelectionMode` | Selection Mode | F119 | done |
| `modeToolStripButton_SnapTo` | Snap To | F120 | done |
| `modeToolStripDropDownButton_SnapToStrength` | Snap To | F121 | done |
| `modeToolStripMenuItem_SnapStrength_1` | 1 | F121 | done |
| `modeToolStripMenuItem_SnapStrength_2` | 2 | F121 | done |
| `modeToolStripMenuItem_SnapStrength_3` | 3 | F121 | done |
| `modeToolStripMenuItem_SnapStrength_4` | 4 | F121 | done |
| `modeToolStripButton_DragBoxFilter` | Drag Box Filter | F122 | done |
| `modeToolStripDropDownButton_DragBoxFilter` | Drag Box Filter | F123 | done |
| `toolStripTools` | Tools | F99 | done |
| `toolsToolStripButton_LipSync` | LipSync Edit Map | F146 | done |
| `toolsToolStripButton_CurveLibrary` | Curve Library | F147 | done |
| `toolsToolStripButton_ColorLibrary` | Color Library | F148 | done |
| `toolsToolStripButton_ColorGradient` | Color Gradient | F149 | done |
| `toolStripPlayBack` | PlayBack | F99 | done |
| `playBackToolStripButton_Start` | Start | F105 | done |
| `playBackToolStripButton_Play` | Play | F106 | done |
| `playBackToolStripButton_Stop` | Stop | F107 | done |
| `playBackToolStripButton_End` | End | F108 | done |
| `playBackToolStripButton_Loop` | Loop | F109 | done |
| `toolStripColorLibrary` | Library Color | F164 | done |
| `contextMenuStripLibraries` | Libraries | F171 | done |
| `newToolStripMenuItemLibraries` | New | F172 | done |
| `editToolStripMenuItemLibraries` | Edit | F173 | done |
| `deleteToolStripMenuItemLibraries` | Delete | F174 | done |
| `importToolStripMenuItemLibraries` | Import | F175 | done |
| `exportToolStripMenuItemLibraries` | Export | F176 | done |
| `add_RemoveLibraryToolStripMenuItem` | Show/Hide Items | F177 | done |
| `toolBarsToolStripMenuItemLibraries` | Toolbars | F178 | done |
| `imageSizeToolStripMenuItemLibraries` | Toggle Image Size | F179 | done |
| `resetLibraryToolStripMenuItem` | Reset Toolbar | F180 | done |
| `toolStripCurveLibrary` | Library Curve | F164 | done |
| `toolStripGradientLibrary` | Library Gradient | F164 | done |
| `saveFileDialog` |  | F13 | done |
| `contextMenuStripElementSelection` |  | F255 | done |
| `timerPostponePlay` |  | F270 | done |
| `timerDelayCountdown` |  | F270 | done |

In `docs/reference/forms` for `TimedSequenceEditorForm` and not named in section O, so layout objects with no text or handler of their own: `AutoScaleDimensions`, `ClientSize`, `Margin`, `components`, `toolStripMenuItem1`, `toolStripSeparator1`, `toolStripSeparator10`, `toolStripSeparator14`, `toolStripSeparator15`, `toolStripSeparator16`, `toolStripSeparator2`, `toolStripSeparator3`, `toolStripSeparator4`, `toolStripSeparator5`, `toolStripSeparator7`, `toolStripSeparator8`.
