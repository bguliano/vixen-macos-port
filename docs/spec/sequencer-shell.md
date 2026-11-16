# Sequencer window: menus, toolbars, hotkeys and tool windows

## Scope

Citation shorthand used everywhere below. Upstream paths are relative to `upstream/Vixen/src`.

| Prefix | Path |
|---|---|
| `TSE/` | `Vixen.Modules/Editor/TimedSequenceEditor/` |
| `Form.cs` | `TSE/TimedSequenceEditorForm.cs` |
| `Menu.cs`, `Toolstrip.cs`, `ContextMenu.cs`, `Hotkeys.cs` | `TSE/TimedSequenceEditorForm_<name>.cs` |
| `Designer.cs` | `TSE/TimedSequenceEditorForm.Designer.cs` |
| Other bare file names | In `TSE/` or `TSE/Forms/` unless a fuller path is given |

The timeline control itself is specified in `docs/spec/timeline.md`. This spec names the contact points and does not repeat its behaviour. The effect editor grid and the mark collection docker belong to their own areas.

**Read in full, in scope:**

| File | Lines | File | Lines |
|---|---|---|---|
| `Form.cs` | 6396 | `Forms/Form_ColorLibrary.cs` | 567 |
| `Toolstrip.cs` | 2189 | `Forms/Form_CurveLibrary.cs` | 631 |
| `Menu.cs` | 828 | `Forms/Form_Effects.cs` | 236 |
| `ContextMenu.cs` | 493 | `Forms/FindEffectForm.cs` | 282 |
| `Hotkeys.cs` | 334 | `Forms/Form_AddMultipleEffects.cs` | 450 |
| `ExportDialog.cs` | 490 | `Forms/FormParameterPicker.cs` | 174 |
| `PapagayoDoc.cs` | 605 | `Forms/ColorCollectionLibrary_Form.cs` | 442 |
| `PropertyDiscovery.cs` | 215 | `Forms/FormEffectEditor.cs` | 323 |
| `EffectDistributionDialog.cs` | 207 | `Forms/EffectDefaultsExportSelectionForm.cs` | 44 |
| `EffectTimeEditor.cs` | 140 | `Forms/Form_Grid.cs` | 36 |
| `LayerEditor.cs` | 102 | `Forms/Form_Marks.cs` | 68 |
| `EffectModelCandidate.cs` | 99 | `Undo/` (14 files) | 625 |
| `TimedSequenceRowLabel.cs` | 95 | `BulkEffectMoveForm.cs` | 87 |
| `EffectParameterPickerControl.cs` | 80 | `PropertyMetaData.cs` | 73 |
| `LibraryDragPayloadFactory.cs` | 61 | `TimedSequenceEditorDescriptor.cs` | 56 |
| `TimedSequenceElement.cs` | 54 | `InvalidAudioPathDialog.cs` | 45 |
| `CreateEvenMarksForm.cs` | 43 | `PropertyOwnerMetaData.cs` | 34 |
| `SetSequenceLength.cs` | 20 | `PastingMode.cs` | 14 |
| `TimedSequenceEditor.cs` | 7 | | |

Playback, in full: `Vixen.Common/BaseSequence/SequenceExecutor.cs` 541; `Vixen.Core/Execution/Context/`: `SequenceContext.cs` 239, `ContextBase.cs` 200, `LiveContext.cs` 119, `PreCachingSequenceContext.cs` 73, `NonCachingSequenceContext.cs` 52, `PreviewContext.cs` 21; `Vixen.Core/Execution/`: `TimingProviders.cs` 57, `MediaTimingProvider.cs` 30, `SelectedTimingProvider.cs` 22, `ContextFeatures.cs` 16, `ContextCaching.cs` 8.

Layer tool window, in full: `Vixen.Modules/Editor/LayerEditor/`: `Views/LayerEditorView.xaml` 124, `Views/LayerEditorView.xaml.cs` 215, `ViewModels/LayerEditorViewModel.cs` 357, `Services/LayerEditorLayerService.cs` 123, `Services/LayerImportExportService.cs` 272, `ImportExport/LayerExportDocument.cs` 44, `ImportExport/LayerExportRecord.cs` 47.

Supporting, in full: `Vixen.Common/Controls/Undo/UndoManager.cs`, `UndoAction.cs`, `UndoButton.cs`, `UndoDropDownControl.cs`; `Vixen.Modules/Sequence/Timed/TimedSequence.cs` 60, `RowSetting.cs`, `ColorCollection.cs`; `Vixen.Modules/Effect/Effect/StandardMediaExtensions.cs`.

**Read through other means, in scope.**

- `Designer.cs` (2606 lines) and the other `*.Designer.cs` files in scope (`ExportDialog`, `EffectTimeEditor`, `EffectDistributionDialog`, `BulkEffectMoveForm`, `SetSequenceLength`, `InvalidAudioPathDialog`, `CreateEvenMarksForm`, the `Forms/` designers): through their generated tables in `docs/reference/forms/`, the item order lists extracted from `Designer.cs`, and targeted searches. Not line by line. A scripted check confirmed that every `.Text`, `.ToolTipText` and `.Click` in `Designer.cs` is in the JSON. Section O shows every one of its 220 objects accounted for.
- `Forms/Form_GradientLibrary.cs` (610 lines): by a full diff against `Form_CurveLibrary.cs` with names normalized, and a search of its message strings. All differences were seen.

**Read in part, not owned by this spec:** `Vixen.Application/VixenApplication.cs` lines 770-960 of 1384 (open, close prompt, activation). `Vixen.Common/Controls/XMLProfileSettings.cs` lines 1-140 of 176. `Vixen.Common/AudioPlayer/AudioOutputManager.cs` lines 1-180 of 315. `Vixen.Common/AudioPlayer/CoreAudioPlayer.cs` lines 238-284 and searches. `Vixen.Common/Controls/MessageBoxForm.cs` lines 1-75 of 115. `Vixen.Common/Help/Help.cs` lines 1-80. `Vixen.Modules/Sequence/Timed/TimedSequenceData.cs`, by search. `Vixen.Core/Services/EffectDefaults/EffectDefaultsService.cs`, signatures only. Upstream tests under `Vixen.Tests/Sequencer/`: `SequenceExecutorLifecycleTests.cs` and `LibraryDragPayloadFactoryTests.cs` by their asserts; the layer and mark name tests by their test names only. `docs/spec/timeline.md`: its scope, command, Rust API and structure sections, for the boundary. Rust: `crates/vixen-engine/src/playback.rs`, `crates/vixen-session/src/*`, and signatures of `vixen-audio`, `vixen-export`, `vixen-io`.

**Added by the independent audit of this spec.** Re-read line by line: `Form.cs`, `Menu.cs`, `Toolstrip.cs`, `ContextMenu.cs`, `Hotkeys.cs`, `Undo/`, `Forms/FindEffectForm.cs`, `Forms/Form_Effects.cs`, `Forms/Form_AddMultipleEffects.cs`, `Forms/FormParameterPicker.cs`, `Forms/ColorCollectionLibrary_Form.cs`, `Forms/Form_ColorLibrary.cs`, `Forms/Form_CurveLibrary.cs`, `Forms/FormEffectEditor.cs`, `ExportDialog.cs`, `EffectDistributionDialog.cs`, `EffectTimeEditor.cs`, `BulkEffectMoveForm.cs`, `PropertyDiscovery.cs`, `EffectModelCandidate.cs`, `SequenceExecutor.cs` lines 25-40, 180-300, 325-395, 470-500, `SequenceContext.cs` lines 60-239, every designer table in scope, and all menu and strip item orders in `Designer.cs`. Also read: `Vixen.Core/Export/Export.cs` lines 60-175 and 585-640; `Vixen.Modules/App/Marks/MarkCollection.cs` by search; `Vixen.Common/Controls/TimeLineControl/Grid.cs` lines 50-60, 970-1000, 1335-1420; `Vixen.Common/BaseSequence/Sequence.cs` lines 160-170; `Vixen.Modules/Editor/LayerEditor/` by search and the lines cited; `PapagayoDoc.cs` lines 130-182, 255-272, 305-350. Rust: `crates/vixen-session/src/{undo,sequence,playback,transport,export}.rs` by signatures and `crates/vixen-ffi/src/sequence.rs`. Every file:line citation in the spec was checked by script to be in range, and its enclosing method compared by eye with what the entry says; `Form_GradientLibrary.cs` citations were checked by function position only.

**Not read:** `TSE/User32.cs` (236 lines, P/Invoke for the clipboard listener, Windows only; only its call sites were read). `TSE/AutomaticMusicDetection*`, `MarkTapper*` (their only references are their own files, so unreachable), `MarkCollectionImportDialog*` and `Forms/WPF/MarksDocker/*` (marks area). `TSE/EffectRasterizer.cs`, `IntentRasterizer.cs` (thumbnails, in `vixen-session`). `Vixen.Core/Execution/Context/ProgramContext.cs` 248 and `Vixen.Core/Execution/ProgramExecutor.cs` 294 (scheduled shows, not used by this window). `Forms/*.resx` and `TimedSequenceEditorForm.resx` (images only). The screenshots at vixenlights.com were not opened; control placement comes from the designer tables and `upstream/Vixen/Assets/Editor.png`, which was viewed.

## Features

Conventions used below. ⌘ is upstream Ctrl, ⌥ is Alt and ⇧ is Shift. "Holding" a modifier means it is among those down; "exactly" means no other modifier is down. Function keys are always written like "F5 key", to tell them from entry numbers; quoted tooltips such as "Play F5" are literal text. A citation starting with a colon continues the last file named in that entry, or in the window's parent entry just above when the entry names none. Each entry says what is done and what results; a missing line about defaults, undo or persistence means there is none. Persisted setting names are under `AppSettings/TimedSequenceEditorForm/` unless stated (see Data and persistence). KEEP, GUARD and FIX are defined in section N.

### A. Window lifecycle

**F1. Open the sequencer window.** Form `TimedSequenceEditorForm` loads in this order (`Form.cs:292-579`): wait cursor; register clipboard viewer; read dock layout from `%AppData%/Vixen/TimedSequenceEditorForm.xml` (corrupt file: discard and rebuild defaults, `Form.cs:303-319`, `Form.cs:581-646`); force a hidden or off-screen timeline back to docked (`Form.cs:322-327`); resize a floating timeline smaller than 100x100 to 400x400 (`Form.cs:330-335`); show Effect Editor and Layer Editor docked right if the saved layout does not know them (`Form.cs:337-345`); restore every profile setting listed in "Data and persistence"; wire timeline events; `LoadAvailableEffects`, `PopulateDragBoxFilterDropDown`, `InitUndo`, `UpdateButtonStates`, `UpdatePasteMenuStates`, `LoadColorCollections`; restore splitter (`Form.cs:548-561`); restore playback range from the sequence if `DefaultPlaybackEndTime != 0` (`Form.cs:563-567`); `SetToolBarLayout` (`Form.cs:570`).

**F2. Default tool window layout.** `SetDockDefaults` (`Form.cs:648-658`): Timeline = main document; right side tabs in this order Color Library, Curve Library, Gradient Library, Marks, Layer Editor, Find Effects; Effects left. The right side's tabs sit along the bottom of that pane (`upstream/Vixen/Assets/Editor.png`). Effect Editor added right when absent (`Form.cs:337-340`). Dock portion defaults left 150, right 150 (values above 1 are points in DockPanelSuite's convention; that library's source is not in this tree) (`Form.cs:353-354`). Tool windows are created lazily and re-created if disposed (`Form.cs:1054-1196`). None sets `HideOnClose`, so closing one (its tab's close button, or the View menu, `Menu.cs:434`) disposes it and the next use builds a fresh one. Find's mode and results and the Effects tree's expansion and highlighted row are lost, and a re-created Curve or Gradient window has **Maintain Library Link** unchecked, which `EditorClosing` then saves (`Form.cs:5827-5828`). The Mac panels hide rather than dispose, so all of this survives (see Differences forced by the Mac in section N). The Effect Editor added at load comes after the default tabs, so it is the last right-hand tab.

**F3. Window placement restore** (`Form.cs:435-495`): min 640x480; maximized if saved "Maximized" and visible; saved bounds if top-left corner on a screen (quirk: second test passes `Point(rect.Top, rect.Right)`, swapped arguments, `Form.cs:664-668`); otherwise default location and saved size, or 800x600 if saved "Minimized". Mac: use `NSWindow` frame autosave; keep the 640x480 minimum.

**F4. Title bar.** `<sequence name or "Untitled">[ *] - <profile name>` (`Form.cs:1798-1819`). Mac: `NSWindow.title` plus `isDocumentEdited`; keep profile name as subtitle.

**F5. Window shown.** `TimedSequenceEditorForm_Shown` (`Form.cs:6068-6074`): disables the form and fixes its border during load, then `LoadSequence`. `UpdateToolStrip4("")` re-enables them (`Form.cs:1405-1428`, `Form.cs:1414-1419`). The Mac form must never be blocked: show progress in the status bar with cancel on close.

**F6. Sequence load.** `LoadSequence` (`Form.cs:1440-1535`): length 0 becomes 1 minute (`Form.cs:99`, `Form.cs:1463-1464`); status text `Please Wait. Loading: <elapsed>` every 250 ms (`Form.cs:1390-1393`, `Form.cs:1446-1453`); effects added on a background task, then render all rows, show the mark collection rename summary and `CheckDeprecatedEffects` (`Form.cs:1482-1497`). A new sequence (blank `FilePath`) starts modified (`Form.cs:1506-1513`). Restores `TimePerPixel` and `VisibleTimeStart` from the sequence when above zero (`Form.cs:1515-1523`). Any exception in this method is only logged, leaving the window as far as it got (`Form.cs:1529-1532`). The timeline shows its loading state through `SequenceLoading` (timeline spec).

**F7. Row settings on load.** `LoadSystemNodesToRows` (`Form.cs:1318-1388`): row height `DefaultRowHeight` if nonzero else the timeline default; for each row present in `RowSettings` by `TreeId`, set `TreeOpen`, `Height`, `Visible`. Show Hidden always resets to off, then `ApplyHiddenRowFilter` runs (`Form.cs:1382-1384`).

**F8. Load filtering.** `AddElementsForEffectNodes` (`Form.cs:3890-3981`): effect starting after sequence length: dropped with a log warning. Effect ending after sequence length: shortened to fit. Target node with no row: effect dropped from the sequence (log only). These silently change data.

**F9. Missing audio on load.** `PopulateWaveformAudio` shows `InvalidAudioPathDialog` when the file is gone (`Form.cs:1757-1793`, `Form.cs:1738-1755`): KeepAudio does nothing, RemoveAudio removes without a warning, LocateAudio adds without a warning, and cancelling its open panel changes nothing. Before that check Remove Audio and Beat/Bar Detection are enabled whenever the sequence has any media, so with Proceed As Is they stay enabled for a file that is not there, while Speed / Tempo, the audio tooltip and the panel title stay at their defaults (`Form.cs:1763-1764`). When the file exists, the load also applies the Speed / Tempo check to the audio, enables that item, and sets the tooltip `Associated Audio: <file>` and the panel title `Timeline - <file>` (`Form.cs:1767-1775`). No media at all disables Speed / Tempo (`Form.cs:1791`).

**F10. Deprecated elements warning.** After load, if any effect targets a node tagged `Deprecated`: alert titled "Deprecated Elements": "This sequence has effects on the following elements tagged as Deprecated:\n\n" then `<name> (<n> effect[s])` per line (`Form.cs:4000-4022`).

**F11. Mark collection rename summary.** After load if duplicates were renamed: "The following duplicate Mark Collections were renamed so effects can select them correctly:\n\n" + `<old> -> <new>` per line (blank shows `(blank)`), title "Mark Collections Renamed"; then the sequence is marked modified (`Form.cs:4024-4050`).

**F12. Unmapped effects at open** (`CheckMissingEffectMappings`, `Form.cs:5695-5772`, run from `StartEditor` before the window shows, `Form.cs:5778-5783`). Unmapped = effect whose first target node `IsProxy`. If any: alert "The sequence has Effects that belong to Elements not known to this profile.\nWould you like to map them to existing elements?" title "Unmapped Effects" Yes/No. `foreignSequence` = sequence's folder differs from `SequenceService.SequenceDirectory`. No: foreign clears `FilePath` so Save will ask for a name. **The effects do not survive:** no row exists for a proxy node, so F8's load pass drops them from the sequence moments later, with only a log line (`Form.cs:3945-3958`, whose comment says "The user has likely chosen not to map it, so we will drop it"). The next save makes that permanent. Yes: `ElementMapperView` window with the unique source id-to-name list. On OK: effects whose id maps to a live node retarget; others are removed. On cancel: **all unmapped effects are removed**. Foreign clears `FilePath`. Marked modified, **but only a foreign sequence stays that way:** this runs before the window loads, and F6 then calls `SequenceNotModified` for any sequence that still has a path (`Form.cs:5770`, `:1506-1513`). So a mapped sequence in the normal folder shows as unmodified and closes with no prompt, and the mapping is lost unless a later edit gets saved. FIX in the quirk register. The mapper window is `App/TimedSequenceMapper` (its own area).

**F13. Save.** `SaveSequence(filePath, forcePrompt)` (`Form.cs:1543-1611`). With no path and no prompt: if `FilePath` is blank, show the save dialog; else save in place. Dialog starts in `SequenceService.SequenceDirectory`, default extension is the sequence's own (`.tim`), filter `Timed Sequence Editor (*.tim)|*.tim`, title "Save Timed Sequence" (`Designer.cs` `saveFileDialog`). If the typed extension differs, the proper extension is **appended** rather than replaced (`Form.cs:1576-1580`). Cancelling returns without clearing modified. `SaveGridRowSettings` runs before every save (`Form.cs:1613-1636`).

**F14. What SaveGridRowSettings stores.** `DefaultRowHeight`, `DefaultPlaybackStartTime/EndTime`, and `RowSettings` is cleared then rebuilt: a row is stored when its height differs from the default by more than 7 (`h > rowHeight+7 || h < rowHeight-7`, with `(height, treeOpen, visible)`), or else when it is open (`(height, true, visible)`). Rows that are closed and within 7 of default are dropped, so they return to default height on load (`Form.cs:1627-1634`).

**F15. Auto save.** Timer interval from Preferences `AutoSaveInterval`, default 300000 ms (`Form.cs:350`). Enabled only when the item is checked **and** the sequence is modified (`Form.cs:1668-1679`). On tick: stop timer; if modified, `Save()` (`Form.cs:2143-2150`). `SequenceModified` / `SequenceNotModified` re-arm only on the transition of the modified flag (`Form.cs:1822-1839`), so the 5 minutes counts from the first edit after a save, not the last edit. **Quirk (KEEP):** a never-saved sequence shows the save dialog when the timer fires. Cancelling leaves it modified with the timer stopped, and it does not re-arm until the next save or Auto Save toggle (`Form.cs:1549`, `:1588`, `:2145`, `:1824`).

**F16. Close prompt.** `_CloseEditor` in the app (`Vixen.Application/VixenApplication.cs:845-881`), run from the window's Closing event. Modified: alert `Save changes to the sequence <name>?` titled "Save Changes?", Yes/No/Cancel; Yes/No only when the app itself is closing. Cancel keeps the window open. Yes: `Save()`. **Data loss:** if that raises the save panel (a never-saved sequence) and the user cancels it, the close carries on and the work is gone, because `Save` returns nothing and `_CloseEditor` goes on to return true (`Form.cs:1586-1589`, `VixenApplication.cs:860-861`, `:880`). FIX in the quirk register: keep the window open. Otherwise if not modified but `IsEditorStateModified`: **saves silently** (`:863-866`). That editor state flag is set by row height changes (`Form.cs:2294-2300`), zoom (`Form.cs:2368-2375`) and scroll (`Form.cs:2377-2384`), and is never cleared: so simply zooming an already saved sequence causes a save to disk at close, with no prompt. Then the sequence's path is added to the recent list, and `EditorClosing` runs (F17). Mac: `DocumentGroup`-style prompt with verbs "Save", "Don't Save", "Cancel". **KEEP** the silent view-state save, and pin that a blank-path sequence cannot reach it since a new sequence is always modified.

**F17. Closing writes.** `EditorClosing` (`Form.cs:5790-5856`): un-minimize; save dock layout XML; close Marks, Effects, Layer Editor, Find Effects; write settings (see Data and persistence); `Save_ToolsStripItemsFile()`; close Colour, Gradient, Curve libraries (after saving, because their settings are read first); remove old settings nodes `StandardNudge`, `SuperNudge`, `NudgeSettings` and `<Name>`.

**F18. Cleanup of deleted effects at close.** Removed effects stay alive for undo. At close, each removed effect that undo did not bring back gets `Effect.Removing()`, which lets it delete external resources such as cache folders, and then disposal; effects still in the sequence are only disposed (`Form.cs:144-147`, `Form.cs:691-704`). So a deleted Video or similar effect keeps its cache until the window closes. **Keep the timing:** never delete at the moment of removal, because undo must be able to restore it.

**F19. Active editor tracking.** Window activation: previous editor `EditorLostActivation`, new `EditorGotActivation` (`VixenApplication.cs:830-835`). Used for key routing (F225).

**F20. Multiple editor windows.** The app keeps a list of open editors (`VixenApplication.cs:823-828`), so several sequences can be open at once, each with its own context and undo stack. `_properties` for Add Multiple (F291) and the clipboard are shared.

**F21. Effect edits mark the sequence.** Content change: re-render that effect and modified. Time change: modified (`Form.cs:2197-2208`). There is no time-change undo here, it comes from the grid.

**F22. Mark collection changes mark the sequence.** A collection added, removed or reset, or a property change on a collection or its decorator: modified, with handlers kept in step (`Form.cs:5638-5693`). `MarkCollection` raises a property change for any mark added or removed (`Vixen.Modules/App/Marks/MarkCollection.cs:162-167`), so every mark edit lands here, including paths such as F299 that do not mark the sequence themselves.

**F23. Library changes re-render.** `CurvesChanged` and `GradientsChanged` trigger a scan that re-renders effects flagged dirty (`Form.cs:2133-2141`, `Form.cs:4106-4122`). Effects linked to a library item become dirty that way.

### B. Menu bar

Menu order is `Sequence, Edit, View, Tools, Help` (`Designer.cs:370-375`). On the Mac these sit in the system menu bar after the App menu, with `Sequence` where a document app has `File`. Item order, separators and wording come from `Designer.cs:386-395`, `:434-439`, `:479-485`, `:561-578`, `:625-627`, `:711-716`, `:775-802`, `:1010-1019`, `:1026-1031`, `:1111-1116`, `:1166-1169`, `:1197-1199`. Separators: after Auto Save; after Playback; after Export. After Add Effect; after Paste Special; after Redo; after Delete. After Zoom Under Mouse Position; after Reset Row Height; after Show Hidden Rows; after Lock Waveform Height. After Remove Audio.

**F24. Sequence > Save** ⌘S. Upstream Ctrl+S (`Designer.cs:403`). `SaveSequence()` (`Menu.cs:30-33`, F13).

**F25. Sequence > Save As...** upstream Ctrl+Alt+S (`Designer.cs:411`). `SaveSequence(null, true)` (`Menu.cs:41-44`). Mac ⌥⌘S: **do not use ⇧⌘S**, which is the align-to-mark-start hotkey (F198).

**F26. Sequence > Auto Save** check, default on. Mirrors the toolbar button and re-arms (`Menu.cs:35-39`, F15). Persist `AutoSaveEnabled`.

**F27. Sequence > Playback > Play** F5 key. `PlayPauseToggle` (`Menu.cs:51-54`). Shortcut cleared when there is no context (F268).

**F28. Sequence > Playback > Pause.** `PlayPauseToggle` (`Menu.cs:56-59`). In practice always disabled (`Form.cs:3521`, `:3529`, `:3539`); pausing is done by Play. **KEEP** the item (enable it on the Mac only if the owner asks).

**F29. Sequence > Playback > Stop** F8 key. `StopSequence` (`Menu.cs:61-64`).

**F30. Sequence > Playback > Loop** F9 key, check, default off, session only. Mirrors the toolbar button (`Menu.cs:66-79`). Turning it on while a delay is set clears all Play Options checks, checks Delay Off and hides the delay labels. It does not reset the postpone interval, which is harmless because Delay Off is checked (`Form.cs:3284`).

**F31. Sequence > Playback > Play Options > Delay Off.** Default checked (`Designer.cs:492-493`). Postpone interval 100 ms; `ClearDelayPlayItemChecks` clears all checks **and turns Loop off** (`Form.cs:6099-6107`); check this; hide delay labels; Play tooltip "Play F5" (`Menu.cs:81-88`). Session only.

**F32. Sequence > Playback > Play Options > Delay 5 Seconds.** Interval 5000 ms; clear checks and Loop; check this; show `Play Delayed:` with `5 Seconds` (`Menu.cs:90-96`). Session only. Behaviour on Play is F264.

**F33. ... > Delay 10 Seconds.** Same, 10000 ms, `10 Seconds` (`Menu.cs:98-104`).

**F34. ... > Delay 20 Seconds.** Same, 20000 ms, `20 Seconds` (`Menu.cs:106-112`).

**F35. ... > Delay 30 Seconds.** Same, 30000 ms, `30 Seconds` (`Menu.cs:114-120`).

**F36. ... > Delay 60 Seconds.** Same, 60000 ms, `60 Seconds` (`Menu.cs:122-128`).

**F37. Sequence > Export.** `EffectEditorForm.PreviewStop()`; `ExportDialog(Sequence).ShowDialog()`; `ResumePreview()` (`Menu.cs:130-136`).

**F38. Sequence > Close** upstream Ctrl+Q (`Designer.cs:554`). `Close()` (`Menu.cs:46-49`). Mac ⌘W: ⌘Q must remain Quit (App menu).

**F39. Edit > Add Effect** submenu, F284.

**F40. Edit > Cut** ⌘X. Disabled at start, enabled by F304. `ClipboardCut` (`Menu.cs:142-145`), F317. Undo "Cut n effect(s)".

**F41. Edit > Copy** ⌘C. Disabled at start, enabled by F304. `ClipboardCopy` (`Menu.cs:147-150`), F317. No undo; not marked modified.

**F42. Edit > Paste** ⌘V. `PastingMode = Default`, `PasteEffects` (`Menu.cs:152-156`). `PasteEffects` (`Menu.cs:170-174`): anchor row = `SelectedRow ?? ActiveRow ?? TopVisibleRow`; if that row is `Selected`, paste time 0 (keeps original times), otherwise the cursor.

**F43. Edit > Paste Special > Invert.** Mode Invert, then `PasteEffects` (`Menu.cs:158-162`). Enable rules F316. Paste Special itself enables when either child is enabled.

**F44. Edit > Paste Special > Visible Marks.** Mode VisibleMarks, then `PasteEffects` (`Menu.cs:164-168`). Enable rules F316.

**F45. Edit > Undo** ⌘Z. Acts only if something is undoable; enable tracks the stack (`Menu.cs:176-182`, `Form.cs:5474-5488`). Mac: the window's `UndoManager`, so the menu shows `Undo <description>`.

**F46. Edit > Redo** upstream Ctrl+Y. Acts only if something is redoable (`Menu.cs:184-190`, `Form.cs:5490-5504`). Mac: ⇧⌘Z, and also accept ⌘Y via a hidden equivalent.

**F47. Edit > Select All Elements** ⌘A. `SelectAllElements` (`Menu.cs:198-201`).

**F48. Edit > Delete** Delete key. `RemoveSelectedElements` (`Menu.cs:192-196`); enabled from F304. Mac: ⌫ and ⌦.

**F49. Edit > Snap To Marks / Effects** check, default on. Mirrors button, sets `EnableSnapTo` (`Menu.cs:203-207`). Persist `SnapToSelected`.

**F50. Edit > Resize / Draw Indicator** check. Sets `ResizeIndicator_Enabled` (`Menu.cs:237-240`). Persist `ResizeIndicatorEnabled` (false). Effect in the timeline spec.

**F51. Edit > Resize / Draw Indicator > Blue, Yellow, Green, White, Red.** Choosing a colour checks only that one and turns the indicator on (`Menu.cs:242-280`). Persist `ResizeIndicatorColor` ("Red"), checked at load by `CheckRiColorMenuItem` (`Form.cs:368`).

**F52. Edit > CAD Style Selection Box** check, `aCadStyleSelectionBox` (`Menu.cs:282-285`). Persist `CadStyleSelectionBox`. The timeline spec covers behaviour.

**F53. Edit > Legacy Cursor / Active Row** check, `LegacyCursorActiveRow` (`Menu.cs:287-290`). Persist `LegacyCursorActiveRow`.

**F54. View > Zoom Time In** Mac ⌘=, with keypad ⌘+ as a hidden equivalent. Upstream shows "Ctrl+ +" (keypad Add), and takes Ctrl+= through F207. `Zoom(0.8)` (`Menu.cs:301-304`). **Do not give this item the key equivalent ⌘+:** on most keyboards + is Shift and =, so that is the same keystroke as Zoom Rows In (F57, F208). Zoom is saved into the sequence object (Data and persistence).

**F55. View > Zoom Time Out** ⌘-, shown "Ctrl+ -" (keypad Subtract), `Zoom(1.25)` (`Menu.cs:306-309`).

**F56. View > Zoom Under Mouse Position** check, `ZoomToMousePosition` (`Menu.cs:321-324`). Persist `ZoomUnderMousePosition`.

**F57. View > Zoom Rows In** Mac ⇧⌘=, with keypad ⇧⌘+ as a hidden equivalent; upstream shows "Ctrl+Shift+ +". `ZoomRows(1.25)` (`Menu.cs:311-314`).

**F58. View > Zoom Rows Out** ⇧⌘-, shown "Ctrl+Shift+ -", `ZoomRows(0.8)` (`Menu.cs:316-319`).

**F59. View > Collapse All Element Groups.** `RowListMenuCollapse` (`Menu.cs:331-334`).

**F60. View > Reset Row Height to Default.** `ResetRowHeight` (`Menu.cs:326-329`).

**F61. View > Highlight Rows With Effects** check, `HighlightRowsWithEffects` (`Menu.cs:381-384`). Persist `HighlightActiveElements`.

**F62. View > Show Effect Info** ⌘I, shown "Ctrl+ I", check, `ShowEffectToolTip` (`Menu.cs:296-299`). Persist `ShowEffectInfo` (true).

**F63. View > Show Hidden Rows** check, tooltip "Show rows tagged Hidden for this session. Resets to off the next time a sequence is opened." (`Designer.cs:901`, `Form.cs:2302-2306`). Never persisted. Applies `ApplyHiddenRowFilter` (`Form.cs:2345-2354`); filter details are in the timeline spec. Resets at load (F7).

**F64. View > Full Waveform** check, Full or Half style (`Menu.cs:209-214`). Persist `FullWaveform`.

**F65. View > Lock Ruler Height** check. `LockRulerHeight` (`Menu.cs:216-219`). Persist `LockRulerHeight` (false).

**F66. View > Lock Waveform Height** check. `LockWaveformHeight` (`Menu.cs:221-224`). Persist `LockWaveformHeight` (false).

**F67. View > Toolbars** submenu, built by `PopulateContextToolBars`; see the toolbar section.

**F68. View > Effect Window.** Tool window toggle: `HandleDockContentToolStripMenuClick`: hidden or never shown: show at its dock side; else close (`Menu.cs:391-394`, `:426-436`). Left. Check state F77. Menu order: Toolbars, Effect Window, Mark Window, Timeline Window, Effect Editor Window, Layer Editor Window, Find Effects or Layers, Color Library Window, Gradient Library Window, Curve Library Window.

**F69. View > Mark Window.** Same, right (`Menu.cs:396-399`).

**F70. View > Effect Editor Window.** Same, right (`Menu.cs:386-389`).

**F71. View > Layer Editor Window.** Same, **left** when reopened, though the default layout puts it right (`Menu.cs:401-404`, `Form.cs:655`). **KEEP.**

**F72. View > Find Effects or Layers.** Same, **left** when reopened, though the default is right (`Menu.cs:406-409`, `Form.cs:656`). **KEEP.**

**F73. View > Color Library Window.** Same, right (`Menu.cs:411-414`).

**F74. View > Gradient Library Window.** Same, right (`Menu.cs:421-424`).

**F75. View > Curve Library Window.** Same, right (`Menu.cs:416-419`).

**F76. View > Timeline Window.** Only shows it if hidden or unknown; never hides (`Menu.cs:368-378`).

**F77. Tool window visibility checks.** When the menu bar is activated each tool window item is checked if its window exists and `DockState != Unknown`; Timeline if not hidden and known (`Form.cs:6076-6091`). The check does not create a window.

**F78. Tools > Audio > Associate Audio...** F302 (`Menu.cs:352-355`).

**F79. Tools > Audio > Remove Audio.** F303 (`Menu.cs:347-350`). Disabled until audio present (`Form.cs:1763`, `:1875`, `:1990`).

**F80. Tools > Audio > Beat/Bar Detection** (disabled until audio present). First `Audio` media: `new BeatsAndBars(audio).DoBeatBarDetection(LabeledMarkCollections)`; modified (`Menu.cs:609-623`).

**F81. Tools > Audio > Speed / Tempo** check, disabled until audio. `Audio.UseTempo = checked` (`Menu.cs:339-345`); also applied whenever audio loads or is associated (`Form.cs:1770`, `:1957`). Enabled by F9 and F302, disabled by F303. Persist `UseTempoForSpeed`.

**F82. Tools > Create Evenly Divided Marks.** `CreateEvenMarksForm(_mPrevPlaybackStart, _mPrevPlaybackEnd, SequenceLength)` in a loop; cancel returns. If `(end-start)/divisions` in ms is below the time of 12 pixels at the current zoom: error "Individual mark length is smaller than allowed.\n\nEither increase the difference in time between Start and End times or decrease the number of divisions." title "Mark Length Error", and the dialog reopens. On OK: remove **the first** collection that exactly matches (`Marks.Count == divisions+1`, first start = Start, last start = End). New collection named `<n> Divisions`; if a collection already starts with that name, the number after the prefix (digits, or 1 when none) sets next = that+1, name `<n> Divisions #<next>`. Marks: divisions+1 marks at Start + i*division, duration 80% of division. Added; modified. No undo (`Menu.cs:630-706`). The dialog itself belongs to the marks spec.

**F83. Tools > Sequence Length...** `SetSequenceLength(SequenceLength)`; OK sets length, modified; no undo (`Menu.cs:357-366`). This handler does not touch effects past a shorter length; F8 drops or shortens them at the next load.

**F84. Tools > Curve Editor.** `CurveLibrarySelector{DoubleClickMode=Edit}` dialog; afterwards `_currentToolStrip = null`, `Populate_Curves()` so the strip rebuilds (`Menu.cs:452-458`). Selector window is in the curves area.

**F85. Tools > Color Gradient Editor.** Same with `ColorGradientLibrarySelector` and `Populate_Gradients` (`Menu.cs:460-466`).

**F86. Tools > Color Collection Editor.** `ColorCollectionLibrary_Form(copy of _colorCollections)`; OK: replace and `SaveColorCollections`; else `LoadColorCollections` (reverts) (`Menu.cs:438-450`).

**F87. Tools > LipSync > Default Image Map.** Submenu rebuilt on opening with `LibraryReferenceName` items, the default checked; click sets `DefaultMappingName` if different, and marks modified (`Menu.cs:481-501`).

**F88. Tools > LipSync > Edit Image Maps.** `LipSyncMapSelector`; if changed: modified, `ResetLipSyncNodes`, save system config (`Menu.cs:468-479`, `Form.cs:6115-6130`).

**F89. Tools > LipSync > Edit Element Face Mapping.** `LipSyncNodeSelect` with `MaxNodes = int.MaxValue`, `MatrixOptionsOnly = false`, `AllowGroups = true`, `AllowRecursiveAdd = false`; on OK with nodes: `FaceSetupHelper.Perform`; on success `RenderLipSyncElementsAsync`, save config (`Menu.cs:584-607`).

**F90. Tools > LipSync > Lyric Converter.** Non-modal `LipSyncTextConvertForm` if not already `Active`, with the handlers from F301 and the mark collections (`Menu.cs:567-577`).

**F91. Tools > LipSync > Papagayo Import.** Open filter `Papagayo files (*.pgo)|*.pgo|All files (*.*)|*.*`; parse (F404). For each voice with phonemes: each phoneme with nonzero duration becomes a LipSync effect with `StaticPhoneme`, `LyricData`, start ms, duration `DurationMS - 1`, row offset = voice index; `EarliestStartTime` = minimum; written to the clipboard after each voice, and modified. Then alert titled "Papagayo Import": `<rownum> Voices imported to clipboard as separate rows\n\nRow #1 - <voice>\n...` (`Menu.cs:503-565`). The user then pastes.

**F92. Tools > Bulk Effects Move.** `BulkEffectMoveForm(cursor, SequenceLength)`; OK: offset = +/- dialog offset; `grid.MoveElementsInRangeByTime(start, end, offset, visibleRows, clip)` returns the applied offset; if `ProcessMarks` and offset nonzero `MoveMarksInRangeByTime` with its undo `MarksBulkChangeUndoAction` (skips Locked collections, and only collections with `ShowMarkBar` when visible rows only) (`Menu.cs:708-722`, `Form.cs:3663-3688`). With marks included the effect move and the mark move are two undo steps, and the mark action is pushed even if no mark moved, reading `Bulk Change 0 Mark`.

**F93. Tools > Effect Defaults > Export...** No summaries: "There are no saved effect defaults to export." titled "Export Effect Defaults". Otherwise F403; nothing checked returns silently; then a save panel default `.vfd`, filter `Vixen 3 Effect Defaults (*.vfd)|*.vfd|All Files (*.*)|*.*`. Error "Unable to export effect defaults, please check the error log for details" titled "Unable to Export" (`Menu.cs:724-759`).

**F94. Tools > Effect Defaults > Import...** Same filter; `ImportMode.Overwrite`; result "Imported {n} new and {m} updated effect default(s)." titled "Import Effect Defaults"; error "Unable to import effect defaults, please check the error log for details" titled "Unable to Import" (`Menu.cs:761-787`).

**F95. Tools > Effect Defaults > Dump (Readable XML)...** `WriteDiagnosticDump`, default `.xml`, filter `XML Files (*.xml)|*.xml|All Files (*.*)|*.*`; error "Unable to write the diagnostic dump, please check the error log for details" titled "Unable to Write Dump" (`Menu.cs:789-810`).

**F96. Help > Online Help.** Opens `https://www.vixenlights.com/docs/usage/sequencer/` (`Menu.cs:815-818`, `Vixen.Common/Help/Help.cs`). Mac: `NSWorkspace.open`, in the system Help menu.

**F97. Help > Vixen YouTube Channel.** Opens `https://www.youtube.com/user/VixenLightsSoftware` (`Menu.cs:820-823`).

### C. Toolbars

Twelve strips live in one top panel (`toolStripContainer`; only the top panel is enabled, `Designer.cs:1344-1355`). Designer default visibility: **on** Audio, View, Edit, Mode, PlayBack; **off** Alignment, Effects, File, Tools, Library Color, Library Curve, Library Gradient. Item order within each strip below is the upstream order, tooltips default to the item's text (`AutoToolTip`) unless given.

**F98. Strip layout and drag.** Users drag strips between rows. On drag end and whenever a toolbar menu closes, every strip is re-packed to X = 3 in its row, keeping row order (`Toolstrip.cs:1948-1959`, `:1925-1929`). Saved at close as `ToolStripName<i>`, `ToolStripVisible<i>`, `ToolStripX<i>`, `ToolStripY<i>` and `ToolStrip/ToolStripCount` (`Toolstrip.cs:2005-2019`); restored with `Y + 5` (`Toolstrip.cs:248-309`). Default row when nothing is saved (`ToolStripCount == 0`): PlayBack, Edit, Mode, View, Audio, side by side (`Toolstrip.cs:312-324`). Mac: see `ToolbarBand` in the SwiftUI section.

**F99. Toolbars list.** Lists every strip by its `Text` (Alignment, Audio, Edit, Effects, File, Library Color, Library Curve, Library Gradient, Mode, PlayBack, Tools, View), sorted alphabetically (`Toolstrip.cs:1884-1918`); check = visible; click toggles the strip whose `Text` matches (`Toolstrip.cs:408-431`). Appears in View > Toolbars and in all three toolbar context menus. **Quirk:** View > Toolbars is built only at load (`Toolstrip.cs:342`) and `ToolStripItem_Changed` rebuilds only the context menu's copy, so View's checks can go stale. **FIX**: one source of truth.

**F100. Toolbar context menu** (`contextMenuStripAll`, `Designer.cs:280-284`). Right-click a strip: all four items below. Right-click the empty panel: only Toolbars (`Toolstrip.cs:434-459`). Library and Effects strips have their own menus (F171, F151).

**F101. Toolbar context > Show/Hide Items.** Lists that strip's items by `ToolTipText` with image and check; click toggles the item's visibility and stores it in the tool strip states if the strip has an entry there (`Toolstrip.cs:380-405`, `:1868-1882`; see F177).

**F102. Toolbar context > Toolbars.** F99.

**F103. Toolbar context > Toggle Image Size.** `ChangeToolStripImageSize(strip, 28)`: current size not 28: 28 with image above text. Otherwise 16 with text before image (`Toolstrip.cs:918-940`). Persist `ToolStrip/<Strip>`. Defaults: main strips large (28), library strips small (16) (`Toolstrip.cs:93-114`). The Effects strip has no size toggle.

**F104. Toolbar context > Reset Toolbar.** Every item visible, states true (`Toolstrip.cs:462-482`).

#### PlayBack strip (`Designer.cs:2106-2111`)

Icons `control_start_blue`, `control_play_blue`, `control_pause_blue`, `hourglass`, `control_stop_blue`, `control_end_blue`, `arrow_repeat` (`Toolstrip.cs:121-129`). On the main strips every item is image only (`DisplayStyle = Image`), so its text appears only as the tooltip.

**F105. Start.** `PlaybackStartTime = _mPrevPlaybackStart = 0`, `VisibleTimeStart = 0` (`Toolstrip.cs:1595-1600`). Not marked modified. Range persists at Save.

**F106. Play**, tooltip "Play F5". `PlayPauseToggle` (`Toolstrip.cs:1602-1605`). Image, text and tooltip switch per F268; hourglass per F264.

**F107. Stop**, tooltip "Stop F8". `StopSequence` (`Toolstrip.cs:1607-1610`). Enable per F268.

**F108. End.** `PlaybackEndTime = _mPrevPlaybackEnd = sequence length`, `VisibleTimeStart = TotalTime - VisibleTimeSpan` (`Toolstrip.cs:1612-1616`).

**F109. Loop**, tooltip "Loop F9", toggle, mirrors the menu item (`Toolstrip.cs:1618-1621`). F30.

#### Edit strip (`Designer.cs:1821-1827`)

Icons `arrow_undo`, `arrow_redo`, `cut`, `page_white_copy`, `page_white_paste`, `paste_special`, `paste_invert`, `paste_marks` (`Toolstrip.cs:136-149`).

**F110. Undo.** Split button: main click one step; drop-down list of descriptions, newest first, with a footer `Undo <n> Action[s]`; hovering highlights down to that row, choosing undoes that many (`Form.cs:5445-5461`, `Vixen.Common/Controls/Undo/UndoButton.cs`, `UndoDropDownControl.cs:39-92`). Disabled when empty.

**F111. Redo.** Same, footer `Redo <n> Action[s]` (`Form.cs:5449`, `:5463-5471`).

**F112. Cut.** F40. Enable F304.

**F113. Copy.** F41. Enable F304.

**F114. Paste.** F42. Enable F316.

**F115. Paste Special drop-down**, image `paste_special`, tooltip "Paste Special". Refreshes states on open (`Designer.cs:1892-1901`). Enable F316.

**F116. Paste - Invert.** F43. **Hidden** rather than disabled when not applicable (`Form.cs:5125`).

**F117. Paste - Visible Mark/s.** F44. **Hidden** when not applicable (`Form.cs:5124`).

#### Mode strip (`Designer.cs:1923-1929`)

Icons `pencil`, `cursor_arrow`, `magnet`, `table_select_big` (`Toolstrip.cs:230-237`). The two drop-downs have no image of their own.

**F118. Draw Mode**, tooltip "Draw Mode". `EnableDrawMode = true`, Draw checked, Selection unchecked (`Toolstrip.cs:1813-1818`). Clicking while on stays on. Persist `DrawModeSelected` (false).

**F119. Selection Mode.** `EnableDrawMode = false`, Selection checked, Draw unchecked, all Effects strip buttons unchecked (`Toolstrip.cs:1820-1831`). Persist `SelectionModeSelected` (true). Esc does all of this except unchecking the Effects strip buttons, because it goes through `DeselectAllNodes` instead (`Hotkeys.cs:190-197`, `Form.cs:2255-2261`). So after Esc a strip button can stay checked with no effect chosen, and the next press on it only unchecks it (`Toolstrip.cs:1433-1437`). KEEP.

**F120. Snap To**, tooltip "Snap To Marks / Effect", toggle, mirrors the menu item, sets `EnableSnapTo` (`Toolstrip.cs:1843-1847`). F49.

**F121. Snap Strength drop-down**, tooltip "Snap Strength", items 1, 2, 3, 4. Exclusive; clicking the checked one does nothing; default 2 (`Toolstrip.cs:1849-1862`, `Form.cs:358`, `Form.cs:396-407`). Persist `SnapStrength`.

**F122. Drag Box Filter** toggle. `DragBoxFilterEnabled` (`Toolstrip.cs:1833-1836`). Session only, default off.

**F123. Drag Box Filter drop-down**, tooltip "Drag Box Filter". First **Invert Selection**, then one checkable item per effect with its image, in descriptor order. Each check adds or removes the type id in `DragBoxFilterTypes` and **turns the filter on** (`Form.cs:1234-1265`). Stays open while clicking. Session only, default empty. **Quirk (KEEP):** Invert flips every item, itself included.

#### View strip (`Designer.cs:1688-1692`)

Icons `zoom_time_in`, `zoom_time_out`, `zoom_row_in`, `zoom_row_out` (`Toolstrip.cs:188-195`).

**F124. Zoom Time In.** F54.

**F125. Zoom Time Out.** F55.

**F126. Zoom Row In**, tooltip "Zoom Row In". F57.

**F127. Zoom Row Out**, tooltip "Zoom Row Out". F58.

#### Audio strip (`Designer.cs:264-269`)

Icons `music`, `minus`, the drawn gauge, `plus`, `Audio_Devices` (`Toolstrip.cs:214-222`).

**F128. Associate Audio.** F302. Tooltip `Associate Audio`, or `Associated Audio: <file>` when set.

**F129. Decrease Timing speed.** Speed - 0.1 (`Toolstrip.cs:1728-1731`). Enable F271, F272.

**F130. Timing speed gauge**, tooltip "Timing speed". Drawn image (F133). Click: speed 100% (`Toolstrip.cs:1627-1630`). Double-click: speed 400% (`Toolstrip.cs:1632-1635`). Scroll over it: rounds current speed to 1 decimal, then +0.1 up or -0.1 down (`Toolstrip.cs:1684-1711`). Click and double-click need it enabled, per F272. **The wheel is not gated:** the handler is on the whole strip and only tests the pointer's X against the label (within 5 pt), so it works on a disabled gauge, and `_SetTimingSpeed` then enables both +/- buttons whatever the timing source (`Form.cs:5972-5973`). KEEP: without audio the stored speed is harmless and F272 disables the controls again at the next Play. Per window, session only, default 100% (`Form.cs:110`).

**F131. Increasing Timing speed** (keep the wording as tooltip). Speed + 0.1 (`Toolstrip.cs:1723-1726`). Enable F271, F272.

**F132. Audio Devices drop-down** (`Form.cs:1691-1734`, rebuilt each opening, `Toolstrip.cs:1920-1923`). Lists `AudioOutputManager.Instance().AudioOutputList` by `FriendlyName`; preferred id checked. If none matches, the first is checked and made the preferred device. Click: exclusive check; when audio is loaded sets `Audio.AudioOutputManager.AudioOutputDeviceId` (`Toolstrip.cs:1732-1744`). Persist `AudioDevice.settings`. **Quirk:** with no audio loaded the check moves but the device does not change. **FIX**: set the manager's device directly.

**F133. Speed gauge drawing** (`Toolstrip.cs:1637-1682`). 32x32. Text = `round(speed,1)*100` with no decimals, 10 pt bold, at x = 3 when speed >= 0.97, else x = 6, y = 0, in the main colour. 180 degree arc from 180 degrees over rect (3,16,26,29), pen 5, main colour. Needle from (16,32), length 19, pen 3, foreground colour, angle degrees = `350 - (160 - (int)(10f * speed * 4f))`, with x and y truncated to int. Main colour: `HSBColor(h,255,255)` with byte hue (ZedGraph, 0 to 255), `h = 120 - (int)(7.5f * speed * 4f)` when speed >= 1, otherwise `(int)(2.5f * speed * 60f) - 20`. Expected values are under Tests.

#### File strip (`Designer.cs:1745-1750`)

Icons `Save`, `SaveAs`, `Auto_Save`, `folder_go`, `Exit` (`Toolstrip.cs:173-182`).

**F134. Save.** F24.

**F135. Save As.** F25.

**F136. Auto Save**, toggle, mirrors the menu item and re-arms (`Toolstrip.cs:1802-1807`). F26.

**F137. Export.** **Upstream bug:** the button has no click handler, so it does nothing (`Designer.cs:1794-1802`). **FIX**: wire it to F37.

**F138. Close.** F38.

#### Alignment strip (`Designer.cs:1376-1383`)

Icons `fill_gaps`, `alignment`, `alignStartMark`, `alignEndMark`, `alignBothMark`, `distribute` (`Toolstrip.cs:155-167`). The Close Gaps drop-down has no image of its own.

**F139. Close Gaps.** `grid.CloseGapsBetweenElements()` (`Toolstrip.cs:1838-1841`), timeline spec.

**F140. Close Gaps threshold drop-down**, tooltip "Close Gaps", items .100, .200, .300, .400, exclusive; sets `CloseGap_Threshold` from the item's tag, which equals its text (`Toolstrip.cs:1750-1762`). Persist `CloseGapThreshold` (".100"). No item is checked in the designer: at load the one matching the saved string is clicked, so a hand-edited value matching none leaves nothing checked but stays in effect (`Form.cs:409-420`).

**F141. Align To threshold drop-down**, no arrow, items .200, .400, .600, .800, 1.000, 2.000, exclusive; sets `AlignTo_Threshold` to the item's text (`Toolstrip.cs:1764-1776`). Persist `AlignToThreshold` (".800"), matched at load the same way as F140 (`Form.cs:422-433`).

**F142. Start Mark**, tooltip "Align Start to nearest mark". F295 Start (`Toolstrip.cs:1778-1781`).

**F143. End Mark**, tooltip "Align End to nearest mark". F295 End (`Toolstrip.cs:1783-1786`).

**F144. Both Marks**, tooltip "Align Both to nearest mark". F295 Both (`Toolstrip.cs:1788-1791`).

**F145. Distribute Mark.** F293 (`Toolstrip.cs:1793-1796`).

#### Tools strip (`Designer.cs:2045-2049`)

Icons `Lipsync`, `Curve`, `Color`, `ColorGradient` (`Toolstrip.cs:201-207`).

**F146. LipSync Edit Map**, tooltip "Edit LipSync Map". F88.

**F147. Curve Library.** F84.

**F148. Color Library.** F86: despite its label this opens the Color Collection Editor, not the Color Library window (`Designer.cs` wires it to `ColorCollectionsToolStripMenuItem_Click`).

**F149. Color Gradient**, tooltip "Color Gradient Library". F85.

#### Effects strip

**F150. Effect buttons.** One per effect module, grouped by `EffectGroup` in Basic, Pixel, Device, Fixture order, in descriptor order within a group; name, tooltip and text = `EffectName`, image = representative image, at 16 pt (`Toolstrip.cs:1349-1427`, `Form.cs:191`). Left press starts a drag with the effect's type GUID (`Toolstrip.cs:1532-1556`), dropped per F305. In Draw Mode a press also chooses that effect to draw: pressing a checked button clears `grid.SelectedEffect` and unchecks it, otherwise sets it and is the only one checked (`Toolstrip.cs:1429-1448`). Outside Draw Mode a button is never left checked.

**F151. Effects strip context menu** (`contextMenuStripEffect`, `Designer.cs:1567-1571`), items below. Show/Hide Effects, its Basic, Pixel and Device lists, and Toolbars stay open while ticking (`Form.cs:223-231`). Fixture does not (F163), and Label Postion closes on each choice because nothing hooks its closing.

**F152. Effects context > Show/Hide Effects.** Group checks with effect lists. A button shows when its group and its effect are both checked (`Toolstrip.cs:1574-1589`). Clicking an effect check turns its group on, sets visibility, stores it (`Toolstrip.cs:1502-1530`).

**F153. Show/Hide Effects > Basic.** Group check and effect list. Persist `EffectToolStrip/BasicEffectToolStrip` (true).

**F154. Show/Hide Effects > Pixel.** Persist `EffectToolStrip/PixelEffectToolStrip` (true).

**F155. Show/Hide Effects > Device.** Persist `EffectToolStrip/DeviceEffectToolStrip` (true).

**F156. Show/Hide Effects > Fixture.** See F163.

**F157. Effects context > Label Postion** (sic). Exclusive; default Below Image, tag 2 (`Toolstrip.cs:1558-1572`, `:1450-1489`). Label font is 2 pt smaller than the window font. Persist `EffectToolStrip/EffectLabelIndex`.

**F158. Label Postion > No Label.** Text blank.

**F159. Label Postion > Above Image.**

**F160. Label Postion > Below Image.**

**F161. Effects context > Toolbars.** F99.

**F162. Effects context > Reset Toolbar.** All groups and effects checked and visible (`Toolstrip.cs:1336-1347`). Does not update stored state until the next change.

**F163. Fixture group quirks.** The Fixture group is read from `EffectToolStrip/FixtureEffectToolStrip` but written as `EffectToolStrip/IntelligentFixtureEffectToolStrip` (`Toolstrip.cs:81`, `:2037`), so it always restores checked. And its menu closes on click because the constructor *removes* the closing handler rather than adding it (`Form.cs:227`). **FIX** both: one key, kept open.

#### Library strips ("favorites"): Library Color, Library Curve, Library Gradient

**F164. Contents.** Colours from the Color Library window's list: 28x28 swatch with border, tooltip `R: r G: g B: b` (`Toolstrip.cs:968-1020`). Curves and gradients from the app library: 28x28 thumbnails with border, tooltip = name (`Toolstrip.cs:1093-1141`, `:1220-1270`). Rebuilt whenever a library window raises its changed event. The builders skip when `_currentToolStrip` is set, but the event handlers clear it first and so do the Tools menu paths, so nothing actually blocks a rebuild, even with the pointer over the strip (`Toolstrip.cs:957-961`, `:1082-1086`, `:1209-1213`, `Menu.cs:456`, `:464`). Hidden items by name (F177).

**F165. Hover.** Button shows checked while hovered and reverts on leave unless already selected (`Toolstrip.cs:574-594`). Leaving the strip clears all checks and focuses the timeline, unless a right press is holding a context menu open (`_mouseDown`, `Toolstrip.cs:596-608`).

**F166. Click.** Acts on mouse down, not up. Exclusive selection: every other button in the strip unchecks. ⌘-press toggles this button and leaves the others (`Toolstrip.cs:524-543`, `:633-635`). A right press does not change the selection (`:628-632`). The selection only matters to Delete (F174).

**F167. Double-click.** Edit, F173 (`Toolstrip.cs:622-627`).

**F168. Drag out.** Payload from `LibraryDragPayloadFactory.Create(tag, name, linkToLibrary = ⌘ held)` (F314); allowed Move and Copy (`Toolstrip.cs:647-674`). Drop targets F306 to F309.

**F169. Reorder.** Drag within its own strip: destination = item under pointer offset by the button's width, minus 1, floor 0; the library is rewritten in the new order: colours `UpdateColorSet`; curves and gradients clear and re-add inside a bulk update (`Toolstrip.cs:676-710`, `:490-522`). The comment there says "This is just plain wrong": on the Mac add an explicit order to the library model, keeping one transaction.

**F170. Drop in.** Colour: `AddColor` (`Toolstrip.cs:1033-1076`). Curve, gradient: accepted only if **not** a library reference: `AddCurveToLibraryAsync(c,false)`, `AddGradientToLibraryAsync(c,false)`, which ask for a name (`Toolstrip.cs:1147-1203`, `:1276-1328`). **Upstream bug:** a drag from outside entering a strip overwrites the `Tag` of the last hovered button with the dragged value, even if the drag then leaves without dropping (`Toolstrip.cs:1048`, `:1194`, `:1319`). A rebuild hides it, but a reorder or colour edit before then writes that value into the library under the old button's name (`:503-506`, `:1025`). FIX in the quirk register: leave existing items alone on drag enter.

**F171. Library strip context menu** (`contextMenuStripLibraries`, `Designer.cs:2198-2207`), items below. On closing without choosing, clears checks and focuses the timeline (`Toolstrip.cs:1931-1946`).

**F172. Library context > New.** Colour `AddNewColor` (F343); curve `AddCurveToLibraryAsync(new Curve())` (F362); gradient `AddGradientToLibraryAsync(new ColorGradient())` (F363) (`Toolstrip.cs:763-796`).

**F173. Library context > Edit.** Colour: picker with V unlocked; updates tooltip, tag, image, library set. Curve `EditLibraryCurveAsync(name)`. Gradient `EditLibraryItemAsync(name)`. Each refreshes the button (`Toolstrip.cs:798-857`). It acts on `_selectedButton`, the last button hovered or pressed, so right-clicking the strip's empty area edits that stale button; with none yet, the error is caught and logged. GUARD: on the Mac, Edit needs an item under the pointer.

**F174. Library context > Delete.** Every **checked** button: colours `RemoveColors`; curves `RemoveCurveAsync`; gradients `RemoveColorGradientAsync`, in a bulk update (`Toolstrip.cs:859-916`). **No confirmation here**, unlike the windows. Because hovering checks a button (F165), the right-clicked button is among those deleted, along with any selected earlier.

**F175. Library context > Import.** Delegates to the window, F349, F360, F363 (`Toolstrip.cs:716-739`).

**F176. Library context > Export.** Same (`Toolstrip.cs:741-757`).

**F177. Library context > Show/Hide Items.** F101. **Quirk:** `PopulateColors` looks up hidden state under the **gradient** strip's name, so hidden colours come back on reload (`Toolstrip.cs:975`). **FIX**. Stored per profile. **Second quirk:** state entries are only created when `ToolStripState.xml` is missing (`Toolstrip.cs:1997-2000`, `:2125-2171`). A later profile with no `<Profile>_ToolStripState.xml` has no entry for its library strips, and `ToolStripItemState_Changed` and Reset Toolbar only update an existing entry (`:389-399`, `:467`), so that profile's Show/Hide choices are never stored. **FIX**: create the entry on demand.

**F178. Library context > Toolbars.** F99.

**F179. Library context > Toggle Image Size.** F103.

**F180. Library context > Reset Toolbar.** F104.

### D. Hotkeys

All are processed by the window with key preview (`KeyPreview = true`, `Hotkeys.cs:56-310`). Menu shortcuts win first (`Hotkeys.cs:61-62`). `IgnoreKeyDownEvents` short-circuits everything, but nothing ever sets it (`Hotkeys.cs:58`, `Form.cs:5070`), so omit it. Modifier tests are upstream's; Ctrl means ⌘ on the Mac and Alt means ⌥. Pointer-based keys read the pointer position at key time. They need a selected effect under the pointer with more than one selected, and do nothing during a resize drag. Align semantics and undo labels are in the timeline spec (Commands on effects).

Keys in the switch that do not exclude other modifiers still fire: Shift+D is D, Ctrl+Shift+D is Ctrl+D, Alt+T is T, Ctrl+Shift+T is Ctrl+T. Ctrl+E, Ctrl+B, Ctrl+U, Ctrl+R and Ctrl+N have no menu binding, so they behave like plain E, B, U, R and N with Shift false. Keep all this in one key-to-command table in Rust so it can be tested. Space and the F5 key dereference `_context` with no null check, so with execution closed (F262) upstream throws (`Hotkeys.cs:10`, `Form.cs:3270`). GUARD: do nothing.

| # | Upstream key | Mac | Behaviour | Source |
|---|---|---|---|---|
| F181 | Home | Home (fn ←) | `VerticalOffset = 0` | `Hotkeys.cs:68-73` |
| F182 | Ctrl+Home | ⌘Home | `VisibleTimeStart = 0` | same |
| F183 | End | End (fn →) | `VerticalOffset = int.MaxValue`, capped by the grid | `Hotkeys.cs:75-80` |
| F184 | Ctrl+End | ⌘End | `VisibleTimeStart = TotalTime - VisibleTimeSpan` | same |
| F185 | PageUp | PageUp (fn ↑) | `VerticalOffset -= VisibleHeight/2` | `Hotkeys.cs:82-87` |
| F186 | Ctrl+PageUp | ⌘PageUp | `VisibleTimeStart -= VisibleTimeSpan*0.5` | same |
| F187 | PageDown | PageDown (fn ↓) | `VerticalOffset += VisibleHeight/2` | `Hotkeys.cs:89-94` |
| F188 | Ctrl+PageDown | ⌘PageDown | `VisibleTimeStart += VisibleTimeSpan*0.5` | same |
| F189 | Up (no modifier) | ↑ | `MoveActiveRow(-1)`; consumed when it returns true; taken ahead of child controls in `ProcessCmdKey` | `Hotkeys.cs:96-102`, `:313-331` |
| F190 | Down (no modifier) | ↓ | `MoveActiveRow(1)`, same | `Hotkeys.cs:104-110`, `:313-331` |
| F191 | Space | Space | Not running or paused: Play. Otherwise Stop | `Hotkeys.cs:8-26`, `:112-121` |
| F192 | Ctrl+Space | ⌥Space (⌘Space is Spotlight, ⌃Space switches input source) | Not running or paused: Play. Otherwise Pause | same |
| F193 | Shift+Space | ⇧Space | If a timing source exists: `AddMarkAtTime(position, false)`, with undo | same |
| F194 | Ctrl+Left | ⌘← | `MoveSelectedElementsByTime(TimePerPixel * -2)` | `Hotkeys.cs:123-129` |
| F195 | Ctrl+Right | ⌘→ | `MoveSelectedElementsByTime(TimePerPixel * 2)` | `Hotkeys.cs:131-137` |
| F196 | S | S | `AlignElementStartTimes(selected, pointed, hold = false)` | `Hotkeys.cs:139-154` |
| F197 | Shift+S | ⇧S | Same with hold = true | same |
| F198 | Ctrl+Shift+S | ⇧⌘S | `AlignEffectsToNearestMarks("Start")`, F295. **This is why Save As cannot use ⇧⌘S** | same |
| F199 | E | E | `AlignElementEndTimes(selected, pointed, hold = false)` | `Hotkeys.cs:155-170` |
| F200 | Shift+E | ⇧E | Same with hold = true | same |
| F201 | Ctrl+Shift+E | ⇧⌘E | Nearest marks, End | same |
| F202 | B | B | `AlignElementStartEndTimes(selected, pointed)` | `Hotkeys.cs:172-188` |
| F203 | Ctrl+Shift+B | ⇧⌘B | Nearest marks, Both | same |
| F204 | Escape | Esc | Not while drawing an effect: deselect Effects window, draw mode off, selection mode on | `Hotkeys.cs:190-197` |
| F205 | Ctrl+- | ⌘- | `Zoom(1.25)` | `Hotkeys.cs:199-204` |
| F206 | Ctrl+Shift+- | ⇧⌘- | `ZoomRows(0.8)` | same |
| F207 | Ctrl+= | ⌘= | `Zoom(0.8)` | `Hotkeys.cs:206-211` |
| F208 | Ctrl+Shift+= | ⇧⌘= | `ZoomRows(1.25)` | same |
| F209 | T | T | `ToggleSelectedRows(false)` | `Hotkeys.cs:212-214` |
| F210 | Ctrl+T | ⌘T | `ToggleSelectedRows(true)` | same |
| F211 | D | D | `DistributeSelectedEffectsEqually`, F293 | `Hotkeys.cs:216-225` |
| F212 | Ctrl+D | ⌘D | `SplitSelectedElementsAtMouseLocation` | same |
| F213 | C | C | `AlignElementCenters(selected, pointed)`. ⌘C is Copy when enabled | `Hotkeys.cs:227-238` |
| F214 | U | U | `AlignElementDurations(selected, pointed, exactly Shift = false)` | `Hotkeys.cs:240-251` |
| F215 | Shift+U | ⇧U | Same with true | same |
| F216 | R | R | `AlignElementStartToEndTimes(selected, pointed, false)` | `Hotkeys.cs:253-264` |
| F217 | Shift+R | ⇧R | Same with true | same |
| F218 | N | N | `AlignElementEndToStartTime(selected, pointed, false)` | `Hotkeys.cs:266-277` |
| F219 | Shift+N | ⇧N | Same with true | same |
| F220 | I | I | `DistributeSelectedEffects` dialog, F294 | `Hotkeys.cs:279-289` |
| F221 | F5 key | F5 key (fn on laptops) | Play/Pause toggle | `Hotkeys.cs:291-294` |
| F222 | F8 key | F8 key (fn on laptops) | Stop | `Hotkeys.cs:296-299` |
| F223 | F9 key | F9 key (fn on laptops) | Clicks the Loop button | `Hotkeys.cs:301-304` |
| F224 | Delete | ⌫ and ⌦ | Delete selected effects, through the menu item, F48 | `Designer.cs:687` |

Menu shortcuts are listed with their items in section B: ⌘S, ⌥⌘S, ⌘W, ⌘X, ⌘C, ⌘V, ⌘Z, ⇧⌘Z, ⌘A, ⌘I and keypad ⌘+, ⌘-, ⇧⌘+, ⇧⌘-.

**F225. Key routing from other windows.** Broadcast `"KeydownSWF"` / `"KeydownSWI"` subscribers are added on `EditorGotActivation` and removed on `EditorLostActivation` and on close (`Form.cs:248-263`, `Form.cs:5805-5806`). Floating panels forward keys to whichever editor is active. Mac: panels share the window's responder path, and the app routes to the key sequencer window.

### E. Effect context menu

Right-click on the grid. Built fresh each time (`timelineControl_ContextSelected`, `ContextMenu.cs:56-490`) with `AutomaticallyHandleSelection = false`, shown at the pointer. `element` = first effect under the pointer; `rowDeprecated` = clicked row has the `Deprecated` tag. Top level order as here.

**F226. Add Effect(s)** (image `effects`). Effects sorted by group then name, separator between groups, each with image and tooltip "Use Shift key to add multiple effects of the same type." (`ContextMenu.cs:66-103`). Click on a row: with exactly ⇧ or ⇧⌘ opens Add Multiple (F291) at the grid time; otherwise adds one at the grid time with default duration, selected. Disabled with tooltip "Disabled, this row is tagged as Deprecated." on a deprecated row (`ContextMenu.cs:477-480`).

**F227. Layer** (only when effects are selected or under the pointer, and the sequence has more than one layer) (`Form.cs:2387-2500`). Image `layers`, tooltip "Assign effects to a layer". Items in reverse layer order with tooltip = filter name. A check mark shows on layers in use: full-strength image when only one is in use, medium image when several are. Click: applies to selected effects if any, else effects under the pointer; only effects whose layer differs. Undo `EffectsLayerChangedUndoAction`, modified, only if something changed.

**F228. Alignment** (image `alignment`) (`ContextMenu.cs:115-296`). `alignmentCountAllowed = OkToUseAlignmentHelper(selected)`; `canReferenceAlign = element != null && allowed && (selected > 1 || selected == 1 && !element.Selected)`; `canDistribute = allowed && selected > 1`. Parent enabled when either holds, or when any collection has marks. Tooltip when disabled: not allowed: "Disabled, maximum selected effects per row is 32."; selected == 1: "Select more then one effect or ensure you have Marks added to enable the Alignment feature."; otherwise "Select more then one effect to enable the Alignment feature." Keep "then". Disabled entirely on a deprecated row, even with marks. Images `alignStart`, `alignEnd`, `alignBoth`, `alignCenter`, `matchDuration`, `alignStartEnd` (both), `distribute` (both), `alignStartMark`, `alignEndMark`, `alignBothMark`.

| # | Item, in order | Shown shortcut | Tooltip | Action | Enabled |
|---|---|---|---|---|---|
| F229 | Align Start Times | S, Shift+S | S - Move start time only / Shift+S - Move start time and hold duration | `AlignElementStartTimes(selected, element, exactly Shift)` | reference |
| F230 | Align End Times | E, Shift+E | E - Move end time only / Shift+E - Move end time and hold duration | `AlignElementEndTimes(..)` | reference |
| F231 | Align Both Times | B | Move both start and end times to match duration | `AlignElementStartEndTimes` | reference |
| F232 | Align Centerpoints | C | Move both start and end times, keeping durations constant | `AlignElementCenters` | reference |
| F233 | Match Duration | U, Shift+U | U - Make durations the same by adjusting the end time / Shift+U - Make durations the same by ajusting the start time | `AlignElementDurations(.., exactly Shift)` | reference |
| F234 | Align Start to End | R, Shift+R | R - Move start time, holding the duration constant / Shift+R - Move start time only | `AlignElementStartToEndTimes(.., exactly Shift)` | reference |
| F235 | Align End to Start | N, Shift+N | N - Move end time, holding the duration constant / Shift+N - Move end time only | `AlignElementEndToStartTime(.., exactly Shift)` | reference |
| F236 | Distribute Equally | D | Distribute selected effects equally over the total time span of effects | F293 | distribute |
| F237 | Distribute Effects | I | Customize start and end times | F294 | distribute |
| F238 | Align Start to nearest mark | Ctrl+Shift+S (⇧⌘S) | | F295 Start | any collection has marks |
| F239 | Align End to nearest mark | Ctrl+Shift+E (⇧⌘E) | | F295 End | any collection has marks |
| F240 | Align Both to nearest mark | Ctrl+Shift+B (⇧⌘B) | | F295 Both | any collection has marks |

Move and align semantics, limits and undo labels are in the timeline spec. On the Mac show shortcuts as ⇧S and so on.

**F241. Manipulation** submenu (only when an effect is under the pointer and the row is not deprecated) (`ContextMenu.cs:299-365`).

**F242. Manipulation > Divide at cursor** (image `divide`, shown ⌘D). With a selection: `SplitElementsAtTime` for selected effects strictly spanning the grid time. Otherwise: the clicked effect at the grid time (`ContextMenu.cs:302-319`).

**F243. Manipulation > Clone** (image `page_copy`). `CloneElements(selected)` or the clicked effect (`ContextMenu.cs:321-332`, `Form.cs:3568-3607`): new effect with cloned data, same row, times and layer; one `EffectsAddedUndoAction`; modified. Clone-drag on the grid uses the same function (`Form.cs:527`).

**F244. Manipulation > Clone to selected effects** (image `copySelect`). Enabled only with **more than 2** selected (`ContextMenu.cs:360`), so 2 selected is not enough: **KEEP**. If some differ in type: warn "Some of the selected effects are not of the same type, only effects of {EffectName} type will be modified." titled "Multiple type effect selected" with Cancel. Copies `ParameterValues` from the clicked effect to each other selected effect of the same type, and re-renders. **No undo and no explicit modified** (`ContextMenu.cs:351-358`). FIX proposal: add both.

**F245. Edit Time** (image `clock_edit`; only with an effect under the pointer and a non-deprecated row) (`ContextMenu.cs:367-389`). `EffectTimeEditor(start, span, SequenceLength, MinimumElementDuration)`. OK: all selected effects go to that same start and duration, via `MoveResizeElements`; otherwise `MoveResizeElement(element, start, duration)`. Enabled when alignment helper allows; otherwise tooltip "Disabled, maximum selected effects per row is 32." Undo from the grid.

**F246. Context > Cut** after a separator, shown "Ctrl+X" (⌘X), image `cut`. F40. Enabled with a selection (`ContextMenu.cs:402-407`).

**F247. Context > Copy**, shown "Ctrl+C" (⌘C), image `page_copy`. F41. Enabled with a selection (`ContextMenu.cs:397-407`). Upstream adds Cut before Copy in this menu (`ContextMenu.cs:432`).

**F248. Context > Paste**, shown "Ctrl+V" (⌘V), image `page_white_paste`. F42. Enabled when clipboard count > 0; disabled on a deprecated row (`ContextMenu.cs:411-415`).

**F249. Context > Paste Special** submenu, image `paste_special`, holding F251 and F250, shown only when count > 1 and marks are present (`ContextMenu.cs:434-445`). On a deprecated row its children are disabled but it stays enabled.

**F250. Context > Paste - Visible Mark/s**, image `paste_marks`. F44. In the submenu as above; or on its own when marks are present and count > 0; enabled by the same rule; disabled on a deprecated row (`ContextMenu.cs:420-424`, `:446-449`).

**F251. Context > Paste - Invert**, image `paste_invert`. F43. In the submenu as above; or on its own when count > 1 and no marks; disabled on a deprecated row (`ContextMenu.cs:426-430`, `:450-453`).

**F252. Delete Effect(s)** (only with a selection; shown "Del"; image `delete`) (`ContextMenu.cs:455-460`). F290. Not disabled on a deprecated row.

**F253. Collections** (with a selection, not deprecated) (`ContextMenu.cs:16-54`). Separator and **Collections** (image `collection`) > **Colors** (image `colors`) > **Random** (`randomColors`) and **Sequential** (`sequentialColors`) > one item per non-empty colour collection, tooltip = description. Shown only when (more than 1 selected, or exactly 1 which `SupportsColorLists`, meaning a browsable `List<ColorGradient>` or `List<GradientLevelPair>`, so a single effect with only a plain colour does not get it, `Form.cs:2012-2017`) and collections exist. `ApplyColorCollection(collection, random)` (`Form.cs:2035-2127`): for each selected effect supporting colour; colours filtered to the effect's discrete colours when it has any (`Form.cs:2019-2027`); Sequential uses one running index across **all** effects and properties. Color: one colour; Gradient: one-colour gradient; `List<ColorGradient>`: each entry; `List<GradientLevelPair>`: each entry, curve kept. One undo step **per effect**. Effects without colour are skipped, then alert "One or more effects were selected that do not support colors.\nAll effects that do were updated." titled "Information", whose YES and NO buttons do the same thing (`Form.cs:2119-2125`). Modified is set at the end whatever happened (`Form.cs:2126`). **Crash in upstream:** an effect with two colour properties adds the same dictionary key twice (`Form.cs:2059`, `:2070`, `:2086`, `:2103`). GUARD: keep the first old value per property. **Second crash:** when an effect has discrete colours and none of them are in the collection, the filtered list is empty: random then indexes it and sequential divides by zero (`Form.cs:2024`, `:2031-2032`, `:2058`). GUARD: skip that effect.

**F254. Add Marks to Effects** (image `marks`), always shown (`ContextMenu.cs:470-473`). F296.

**F255. Overlapped effect chooser.** Left-clicking where more than one effect lies under the pointer pops a menu listing each as `<TypeName> (m:ss.fff)`; selection is taken over from the grid (`AutomaticallyHandleSelection = false`); choosing one calls `SelectElement` (`Form.cs:2824-2858`).

### F. Status bar

**F256. Status bar.** Bottom of the window. Left to right: Current Position, Sequence Length, Play Delayed (hidden unless a delay is set), status message taking all spare width, Rendering Elements with its progress bar (hidden unless rendering) (`Designer.cs:1226-1235`). Monospaced digits on the Mac.

**F257. Current Position display.** Label "Current Position:" and a time, initial `0:00.000`. Shows the playback time while playing, blank when playback time clears, and the cursor time on every cursor move; format `m:ss.fff` (`Form.cs:3465-3479`). Format quirk: no hours, so past 59:59.999 only the minutes component shows. **KEEP** the format.

**F258. Sequence Length display.** Label "Sequence Length:" and `m:ss.fff` from the length setter (`Form.cs:1292-1309`).

**F259. Play Delayed display.** Label "Play Delayed:" and `<n> Seconds`, visible while a delay option other than Off is set; counts down each second during the delay and resets to the full value after stop or end (`Menu.cs:90-128`, `Form.cs:3403-3407`, `Form.cs:3438-3442`, `Form.cs:6109-6113`). Hidden at the moment playback really starts (`Form.cs:3304`), and shown again at the end.

**F260. Status message.** `UpdateToolStrip4(text, timeoutSeconds = 0)` (`Form.cs:1405-1435`): sets the text; blank text re-enables the window after loading; a positive timeout clears it after that many seconds. Texts: `Please Wait. Loading: <elapsed>`, `Choose the property to set, press Escape to cancel.` (8 s), `Choose the Effect to use, press Escape to cancel.` (12 s), `<Color|Curve|Gradient> applied to <n> <EffectName> effect(s).` (30 s), `Skipped pasting to Deprecated row(s): <names>.` (10 s). Picker cancelled: cleared at once. Default blank. Quirk: a second message with a timeout restarts the timer rather than stacking, and an earlier message without a timeout is not protected.

**F261. Rendering Elements display.** Label "Rendering Elements:" and a progress bar 0 to 100; shown whenever percent is below 100, hidden at 100 (`Form.cs:2152-2178`). Fed by the grid's render queue (`Form.cs:500`; `render_progress` in the timeline spec). Percent outside 0 to 100 leaves the value unchanged but still drives visibility.

### G. Playback

Host behaviour first, then the engine contract it relies on (`Vixen.Common/BaseSequence/SequenceExecutor.cs`, `Vixen.Core/Execution/Context/SequenceContext.cs`, `ContextBase.cs`, `NonCachingSequenceContext.cs`).

**F262. Context lifetime.** Context opened with `ContextCaching.NoCaching` (`Form.cs:3238`); failure: "Unable to play this sequence.  See error log for details." titled "Unable to Play" (`Form.cs:3239-3247`). On `Execution.State == "Closing"` context is closed and nulled; on `"Open"` reopened (`Form.cs:3215-3228`). So when live execution is toggled the editor re-binds. The window releases its context when closed (`Form.cs:5074-5077`).

**F263. Play/Pause toggle.** `PlayPauseToggle`: not running or paused: play; otherwise pause (`Form.cs:3268-3278`).

**F264. Play with delay.** `PlaySequence` (`Form.cs:3280-3327`): delay active only when Off is unchecked, not already counting and Stop is disabled: countdown = interval / 1000; both timers on; Play image = hourglass; Play button stays enabled; Play menu item disabled; Stop enabled. While the postpone timer runs Play is ignored. Then: Play image blue; delay labels hidden. Paused: resume from `TimingSource.Position`, to MaxValue. Otherwise from `PlaybackStartTime ?? 0` to `PlaybackEndTime ?? MaxValue`. `_PlaySequence` resumes and ignores those values, so the original range still holds (`Form.cs:6000-6004`).

**F265. Play from / between.** `PlaySequenceFrom(start)`: paused: resume; else end = range end ?? max; if start >= end, start = range start ?? 0 (`Form.cs:3333-3360`). Nothing in the source tree calls it, so it can be left out. `PlaySequenceBetween(s, e)`: paused: stop first; s < 0: range start ?? 0; e > total: range end ?? max (`Form.cs:3362-3386`). This is how the marks bar plays a range: its `PlayRangeAction` event lands here (`Form.cs:517`, `:2903-2906`; timeline and marks specs). Neither touches the range or the remembered range.

**F266. Stop.** `StopSequence` (`Form.cs:3400-3426`): delay on: show labels with `<n> Seconds`; countdown running: cancel it, reset Play icon, disable Stop, return. Else `_context.Stop()`. **Quirk:** a cancelled countdown leaves the Play menu item disabled, since F264 disabled it and only `UpdateButtonStates` re-enables it; the F5 key still works through the key handler (`Form.cs:3292`, `:3411-3415`). FIX in the quirk register.

**F267. Context events.** Started: start `timerPlaying`, set `TimingSource`, update buttons (`Form.cs:3428-3433`). Ended: delay labels reset, stop timer, clear source (`Form.cs:3435-3445`). ContextEnded: update buttons, restore `_mPrev*` to range, clear current time, `EffectEditorForm.ResumePreview()`, `SetPreviewsTopMost(false)` (`Form.cs:3447-3455`). Tick: `PlaybackCurrentTime = TimingSource.Position` (`Form.cs:3457-3463`).

**F268. Transport enable table** (`UpdateButtonStates`, `Form.cs:3500-3545`). No context: Play and Stop disabled, F5 key shortcut cleared, Play shows its play state. Running: Play enabled with the F5 key, Stop enabled, Pause menu item disabled and without shortcut; button shows Pause when not paused, Play when paused. Stopped: Play enabled with the F5 key, Stop disabled, Pause menu item disabled. `UpdatePlayButton` sets image, text "Play" or "Pause" and tooltip "Play F5" or "Pause F5" (`Form.cs:3547-3562`). The context gives no pause event, so the UI updates itself after pause and resume (`Form.cs:3319`, `:3397`).

**F269. Start playback.** `_PlaySequence` (`Form.cs:5996-6019`): stops the effect editor preview, calls `PlayerStarted()` on every open preview module (`SetPreviewsTopMost`, `Form.cs:6021-6036`); paused: resume; else Loop on: `PlayLoop(s,e)`, else `Play(s,e)`.

**F270. Delay countdown.** Tick each second, `<n> Seconds` (`Form.cs:6109-6113`); expiry disables both timers and calls `PlaySequence` (`Form.cs:6093-6097`). `ClearDelayPlayItemChecks` turns Loop off too and clears checks on every Play Options item (`Form.cs:6099-6107`): delay and loop are mutually exclusive.

**F271. Speed set.** `_SetTimingSpeed` (`Form.cs:5949-5974`): `<= 0.10`: stores 0.10 and returns without updating the source or the display; `> 4.0`: stores 4.0 and returns the same way; otherwise stores, forwards to timing source if any, updates display; Decrease enabled if speed > delta 0.1; Increase enabled if speed < 4.0. Display image comes from `SpeedVisualisation()`. **Precision matters here:** `speed` and the delta are `float` and `0.10` is a `double`, so exactly `0.1f` widens to 0.10000000149, passes the first test and reaches the source and the display. The +/- buttons step without rounding and the error accumulates: from 1.0, Decrease goes 0.9, ..., 0.19999993, then 0.09999993 hits the early return, so 0.1 is stored while the source and gauge stay at 20% and Decrease stays enabled. Increase goes on to 3.9999983, then 4.0999985 stores 4.0 the same way, gauge at 400 and Increase still enabled. The wheel rounds first, so it lands on exactly 0.1f and 4.0f, and only the step beyond those takes the early return. A stored value reaches the source at the next Play (F272). Port with `f32` arithmetic and compare `speed as f64 <= 0.10`.

**F272. Speed enablement.** `_SetTimingToolStripEnabledState` (`Form.cs:5981-5994`): Increase, Decrease and the speed label enable only if the sequence's timing source exists and `SupportsVariableSpeeds`. `TimingSource` setter applies the same rule while playing, and pushes `_timingSpeed` to a new source (`Form.cs:6038-6063`). `_UpdateTimingSourceToSelectedMedia` picks the first `Media` timing source (`Form.cs:5909-5934`), error title "Error parsing time". The default timing source does not support variable speeds (`Vixen.Core/Module/Timing/TimingModuleInstanceBase.cs:19-21`), and `GetTiming` returns none without a selected provider (`Vixen.Common/BaseSequence/Sequence.cs:164-168`), so **speed only works with audio**. Two more writers of the same controls: `_SetTimingSpeed` sets +/- without checking the source (F271), and F303 refreshes nothing, so after audio is removed they stay as they were until the next Play or load.

**F273. Ruler click (host part).** No context: logs and returns. Left: ⌘ held = autoPlay: remember range, set start to the time and end to nil, modified, `_PlaySequence(time, MaxValue)`. Otherwise: previous start = time, previous end = nil, playback start = time, end = nil, **SequenceModified**, cursor = time (`Form.cs:2860-2896`). Right: `AddMarkAtTime(time, suppressUndo false, fillGap = exactly Ctrl or Ctrl+Shift, promptForName = exactly Shift or Ctrl+Shift)` (`Form.cs:2897-2900`).

**F274. Ruler drag range (host part).** Begin: remember range (`Form.cs:3168-3172`). End: ⌘ held: play that range (range restored at end); otherwise keep it as the new remembered range (`Form.cs:3174-3199`). Not marked modified here.

**F423. Playback range, in one place.** Two pairs of values. The live range is `TimelineControl.PlaybackStartTime` and `PlaybackEndTime`, which Play uses (F264) and Save stores (F14). The remembered range is `_mPrevPlaybackStart` and `_mPrevPlaybackEnd`, which F82 and F291 read as defaults and which ContextEnded copies back over the live range at every stop (F267). Set both: load (F1), Start and End buttons (F105, F108), plain ruler click (F273), finished ruler drag (F274). ⌘-click and ⌘-drag on the ruler play with a temporary live range and restore it at the end. Only the plain ruler click marks the sequence modified, so a range made by dragging or by Start and End is stored only if something else causes a save, and the F16 silent save does not fire for it. **Quirk (KEEP):** a save during a ⌘-started play, including auto save, stores the temporary range, since `SaveGridRowSettings` reads the live values (`Form.cs:1619-1620`).

**F275. Timing interface for other tools.** The window implements `ITiming` and `IExecutionControl`: Start and Resume play, Pause, Stop; `Position` reads the current timing source, setting it does nothing; `SupportsVariableSpeeds` false; `Speed` gets and sets the timing speed; `PositionHasValue` = playback time present (`Form.cs:5860-5906`). Its only consumer is `MarkTapper` (`TSE/MarkTapper.cs:13`), and nothing in the source tree constructs that form or `AutomaticMusicDetection`, so neither is reachable. Not needed on the Mac unless the marks area revives them.

**F276. Play.** Ignored if disposed, already running, or no sequence (`SequenceExecutor.cs:189`). `start = min(start, length)`, `end = min(end, length)` (`:271-279`). `IsTimed = end >= start` (`:200`). Start equal to end: a non-loop natural end is posted at once, and playback still starts then stops (`:202-206`). Timing source = sequence's selected source, else the default timing service (`:208`). Load media at start, start media, set position, start timing, then running (raising started), then end-check timer at 10 ms (`:210-226`, `:32`).

**F277. Natural end.** `position >= end || position == 0`, but only after position has once exceeded start (`_timingHasAdvanced`) (`SequenceExecutor.cs:479-492`). With end before start (`IsTimed` false) it never ends on its own. Posted to the UI context. Loop: F278. Otherwise Stop.

**F278. Loop.** On the UI context: stop timer, stop timing, position = start, start timing, raise restarted, restart timer (`SequenceExecutor.cs:230-259`). Media is not reloaded here: timing is the media when audio exists. Context resets its current effects list and restarts its data pump on restart (`SequenceContext.cs:167-171`, `NonCachingSequenceContext.cs:29-34`).

**F279. Pause and resume.** Pause only if running, not paused and the timer is running: timing pause, media pause, timer stop (`SequenceExecutor.cs:331-348`). Resume: timing resume, media resume, timer start (`SequenceExecutor.cs:350-367`).

**F280. Stop.** Only if running; bump generation so stale posted ends are ignored; stop timer, timing and media; not running (raising ended) and not paused (`SequenceExecutor.cs:369-391`). Context resets effects, stops the pump, raises ended and context ended (`SequenceContext.cs:186-192`). End position does not stay: `PlaybackCurrentTime` clears at context end (F267).

**F281. Timing sources.** "Media" provider lists media with a timing source, by file name (`MediaTimingProvider.cs:15-18`). `GetTimingSource` joins that name to `MediaService.MediaDirectory` and matches the path exactly, returning none if no media matches (`:20-29`). `SelectedTimingProvider` is valid only with nonblank type and source (`SelectedTimingProvider.cs:15-21`). Not on the toolbar: chosen only by audio association (F272).

**F282. Volume.** There is no volume control in the sequencer window. `CoreAudioPlayer` starts at volume 1.0 and clamps 0 to 1 (`Vixen.Common/AudioPlayer/CoreAudioPlayer.cs:28`, `:288-300`), and no code in `TimedSequenceEditor` sets it (no `Volume` match in the folder). So nothing to add here; the Mac honours the system volume. If the owner wants one, that is a new feature, not parity.

**F283. Audio output prefs.** `AudioOutputManager` is a singleton; `AudioOutputDeviceId` setter saves preferences and raises refresh (`Vixen.Common/AudioPlayer/AudioOutputManager.cs:80-89`). Default placeholder id `-0` means system default; a missing device falls back to it (`:14`, `:165-178`). Output mode, latency and share mode are also there but no UI in this area sets them. Supported extensions: `mp3, mpeg3, wav, wave, flac, fla, aiff, aif, aifc, aac, adt, adts, m2ts, mp2, 3g2, 3gp2, 3gp, 3gpp, m4a, m4v, mp4v, mp4, mov, asf, wm, wmv, wma, mp1, avi, ac3, ec3, ogg`; filter text `Supported Files|*.mp3;...` (`:126-139`).

### H. Editing commands

**F284. Add Effect submenu entry** (`LoadAvailableEffects`, `Form.cs:1267-1285`): one item per effect module descriptor (in descriptor enumeration order, `EffectName`). Click: destination row = `ActiveRow ?? SelectedRow`; nothing if null; `AddNewEffectById(id, row, cursor, GetDefaultEffectDuration(cursor), select: true)`.

**F285. Effect instance creation.** Always `CreateEffectInstanceWithDefaults`: built-in defaults overlaid with saved effect-type defaults from `EffectDefaultsService` (`Form.cs:3789-3801`). `AddEffectInstance` (`Form.cs:3812-3855`): if duration <= 250 ms and start+duration exceeds length: start = length - 250 ms; if negative: start 0, duration = length. Selection set when asked. Modified. Undo `EffectsAddedUndoAction`. `CreateEffectNode` sets target node to the row's node, time span, start, media for media-supporting effects and mark collections for mark-supporting ones (`Form.cs:3857-3879`). Clone and paste skip the saved defaults, since they overwrite the module data anyway (`Form.cs:3787-3788`). If an effect being added has a target with no row, an alert shows `TimedSequenceEditor: <AddElementForEffectNodeTpl> - No Timeline.Row is associated with a target ElementNode for this EffectNode. It now exists in the sequence, but not in the GUI.` with a blank title (`Form.cs:4078-4089`).

**F286. Default new effect duration.** `GetDefaultEffectDuration(start, visibleDuration = true)` (`Form.cs:6360-6378`): 2 s. If `visibleEnd - start <= 2`: duration = (visibleEnd - start) * 0.8. If <= 250 ms: 250 ms.

**F287. Draw an effect.** In Draw Mode with an effect chosen in the Effects window or strip, drag a box on the grid. `DrawElement` (`Form.cs:2263-2292`, wired at `Form.cs:528`): needs duration above 10 ms; for each row in the box, a new effect with saved defaults at the box's start and duration; per-row errors are logged and skipped. Modified. One `EffectsAddedUndoAction`. The new effects become the selection. Gesture itself is in the timeline spec. Draw Mode and its selection persist as F118 says.

**F288. Move an effect to another row.** `ElementChangedRowsHandler` (`Form.cs:2220-2251`): retarget the effect to the new row's node; there is one element across rows, so it is removed from every other row showing the old node and added to every other row showing the new node. Modified. Undo is the grid's move entry.

**F289. Clicking the grid clears mark selection.** Any non-right press on the grid clears the marks selection and repaints (`Form.cs:2187-2195`).

**F290. Delete selected.** `RemoveSelectedElements` (`Form.cs:4162-4182`): nothing if none; undo `EffectsRemovedUndoAction` recorded before removal; each removed via `RemoveEffectNodeAndElement` (deselect, remove from all rows, from sequence and from layers, kept in `_removedNodes` for later disposal, `Form.cs:3728-3757`). Missing node alert: "Node to remove not found, the editor is in a bad state! Please close the editor and restart it."

**F291. Add Multiple Effects** (`AddMultipleEffects`, `Form.cs:2502-2593`). Opens `Form_AddMultipleEffects(SequenceLength)` with the sequence's mark collections. Its only caller is F226, which requires Shift, so in practice only the second test decides: once the dialog has been used in this editor process (`_properties.effectCount != -1`; the holder is a `static` class, so shared between editor windows), the previous values are restored: start = previous playback range start or last start; end = previous playback range end or last end; duration; between; align to beat marks; skip every other; fill; align to mark start/end; select effects; panel beat alignment; checked marks by name (`Form.cs:2506-2522`). Otherwise defaults: start = previous playback range start or the given time, end = previous playback range end or sequence length, duration 2 s, between 2 s (`Form.cs:2525-2529`). **Count:** the host sets 2 on first use and nothing on a restore, but neither shows, because the dialog recalculates at load and on every time change and sets the count to the maximum possible (F386, `Forms/Form_AddMultipleEffects.cs:173-183`). So it always opens at the possible count. Then `EffectName = effectName`; `SequenceLength = EndTime`, which does nothing: the dialog's limits are the constructor's full length (F385) and nothing reads that property. OK saves all values to `_properties`. Not aligned: loop `count` times; break when `next + duration > SequenceLength`; next = next + duration + between (`Form.cs:2562-2582`). Aligned: F292. Undo `EffectsAddedUndoAction`, which the plain path pushes, with modified, even for zero effects (`Form.cs:2583-2586`). Select Effects on means the new effects become the selection (`Form.cs:2588-2591`).

**F292. Add effects on marks** (`AddEffectsToBeatMarks`, `Form.cs:2595-2642`). Times = all marks from checked collections (matched by name) with `StartTime >= startTime`, sorted. For each until `effectCount` created: if skip flag false: Fill: last mark ends the loop (no effect); duration = next mark start minus this mark start, floor 0.01 s. Else Align to mark start/end: duration = `mark.Duration`. Otherwise the dialog's duration. Skip Every Other toggles the skip flag after each mark. Undo and modified only if at least one mark time matched.

**F293. Distribute Equally** (`DistributeSelectedEffectsEqually`, `Form.cs:2644-2713`). Precondition `OkToUseAlignmentHelper`, else show `alignmentHelperWarning`: "Too many effects selected on the same row for this action.\nMax selected effects per row for this action is 4" (`Vixen.Common/Controls/TimeLineControl/Grid.cs:57`; the real limit is 32, timeline spec). Span: first selected start to last selected end; if first.start > last.start, swap and walk from the last. Each effect length = span / count. Fails with warning if below 0.050 s: "Unable to complete request. The resulting duration would fall below 50 milliseconds.\nCalculated duration: {0}". Applied with `MoveResizeElements(..., ElementMoveType.Distribute)`, whose undo is produced by the grid. The first warning has a blank title and the duration warning is titled "Warning"; the calculated duration prints as a plain double. Nothing selected returns quietly. One selected is allowed: it stays where it is but still pushes `Distributed 1 effect`. `MoveResizeElements` skips any item ending past the total or under 1 ms long, yet all items stay in the undo (`Vixen.Common/Controls/TimeLineControl/Grid.cs:1350-1375`).

**F294. Distribute dialog** (`DistributeSelectedEffects`, `Form.cs:2715-2822`). Needs alignment helper OK and at least 2 selected, else "Select at least two effects to distribute." titled "Select more effects". Span is first start to last end; swapped only when `startTime > endTime`. Opens `EffectDistributionDialog(SequenceLength)` with ElementCount, Start, End, Equal Duration on, Stair Step on, Start With First on. Algorithm, with `offset = (end-start)/count` in seconds, walking selection order 0..n-1 (Start With First), or n-1..0 (Start With Last):
- start_i = range start for the first; otherwise previous item's computed end.
- not first and Overlap: start_i -= overlap. Not first and Spaced: start_i += spacing.
- Do Not Change Duration and neither Overlap nor Spaced: start_i = range start + offset*i (for last-first, offset*placeCount).
- end_i: initial start + offset; Equal Duration: start_i + offset; Do Not Change: start_i + effect's duration; Specified: start_i + specified.

**F295. Align to nearest marks.** `AlignEffectsToNearestMarks("Start"|"End"|"Both")` (`Form.cs:6223-6270`): no selection: warn "No effects have been selected and action will be applied to your entire sequence. This can take a considerable length of time, are you sure ?" title "Align effects to marks" (explicit No aborts); applies to all effects, in every row they occur. `FindNearestMark` (`Form.cs:6316-6352`): threshold = `AlignTo_Threshold` seconds, invariant culture. Looks at marks in collections with `ShowGridLines` only: mark start strictly within +-threshold, and mark end too if the collection has `ShowTailGridLines`; closest wins; first found wins ties; returns `0` when none. Any effect whose new start or new end is exactly zero is skipped (`Form.cs:6261-6265`). An unchanged edge keeps its value, so a real start at 0 also skips. Applied with `MoveResizeElements(moveElements)` (default type), so the undo reads `Moved {n} effect{s}` (`Vixen.Common/Controls/TimeLineControl/Grid.cs:1350`).

**F296. Add marks to effects** (`AddMarksToSelectedEffects`, `Form.cs:6272-6302`): same warning (Yes/No, Information). `AddMarkAtTime(start, suppressUndo: true)` for each; one undo `MarksAddedUndoAction` for all, target collection is the first `IsDefault`, even if the marks went to a different one (F297 uses the first `IsDefault && IsVisible`). With no collections F297's prompt comes up once per effect until the user creates one.

**F297. Add mark at time** (`AddMarkAtTime`, `Form.cs:3032-3096`). Prompt shows `TextDialog("Enter the Mark label text.", "")`; cancel keeps blank label and still adds. No mark collections: pause if running; "Marks are stored in Mark Collections. There are no mark collections available to store this mark. Would you like to create a new one?", title "Create a Mark Collection", Yes/No; yes creates or finds "Default Marks" white. Otherwise target is first collection with `IsDefault && IsVisible`; none: pause if running; "The active mark collection is not visible on the timeline. Would you like to enable it to add the mark?" title "New Mark", OK/Cancel; OK shows default collection's grid lines and bar, tail lines off. Does nothing if a mark at exactly that time exists. Fill gap: `mc.FillGapTimes`. Modified, dirty re-render, undo `MarksAddedUndoAction` unless suppressed. Playback paused by a prompt is not resumed afterwards. If no collection is flagged `IsDefault`, the second prompt appears and OK then adds nothing. `IsVisible` means `ShowGridLines || ShowMarkBar` (`Vixen.Modules/App/Marks/MarkCollection.cs:160`).

**F298. Mark collection creation.** `GetOrAddNewMarkCollection` matches trimmed name, case-insensitively (`Form.cs:3098-3109`). `CreateNewMarkCollection` gets a unique name via `MarkCollectionNameService`, sets colour, first collection is default, modified (`Form.cs:3111-3123`).

**F299. Phoneme breakdown from marks bar.** Phrase: split text on whitespace; each word gets equal share of duration (integer tick division); goes in the linked Word collection (`GetOrCreatePhonemeMarkCollection`). Word: requires the standard dictionary else "Unable to find Standard Phoneme Dictionary" titled "Error"; strip punctuation; `TryConvert`; on no result, ask user for a mapping (`GetUserMappingForFailedWord`), skip the word if blank; phonemes get equal shares. Name for a new linked collection: `<parent name> <type>`, or `<phrase parent name> <type>` when a parent is not a phrase but has a phrase parent. New collection has parent colour, `LinkedMarkCollectionId`, `ShowMarkBar = true` (`Form.cs:2909-3029`). Undo `MarksAddedUndoAction` when anything was added; modified comes through F22. **Quirks:** `Split()` keeps empty entries, so two spaces in a phrase make an empty word mark. The linked collection is created before the dictionary check, so that error leaves it behind. GUARD: if the user's mapping still converts to nothing, upstream divides by zero (`Form.cs:2965`, `:2972`); skip the word.

**F300. Bulk mark events.** Moved: undo `MarksTimeChangedUndoAction`, update snap times, dirty re-render, modified (`Form.cs:3131-3137`). Deleted: undo `MarksRemovedUndoAction`, same (`Form.cs:3139-3151`). Pasted: undo `MarksAddedUndoAction` (`Form.cs:3153-3165`). Text changed: modified and dirty, **no undo** (`Form.cs:3125-3129`).

**F301. Lyric converter results.** `textConverterHandler` (`Form.cs:6132-6185`): for each phoneme with nonzero duration build a LipSync effect (`StaticPhoneme`, `LyricData`, pre-render) with row offset 0 and put on the clipboard. Placement Cursor: `FirstMark += cursor`. Placement not Clipboard: Paste with Default mode at `FirstMark`. If nothing pasted, alert "Conversion Complete and copied to Clipboard \n Paste at first Mark offset" title "Convert Text". Modified. Failure: `GetUserMappingForFailedWord`: `LipSyncTextConvertFailForm` label "Unable to find mapping for <word>\nPlease map using buttons below"; OK calls `AddUserMaping("<word> <mapped>")` (`Form.cs:6192-6211`). `ResetLipSyncNodes` marks all LipSync effects dirty and renders all rows (`Form.cs:6115-6130`).

**F302. Associate audio.** `AddAudioAssociation` (`Form.cs:1888-1997`): if audio exists and warning on, alert "Current audio file: <path>\n\nOnly one audio file can be associated with a sequence at a time. If you choose another, the first will be removed. Continue?" titled "Remove existing audio?"; open dialog filter `AudioOutputManager.GetSupportedFilesFilter()`; unsupported: warning "The selected file is not a supported type."; zero duration: error "The selected audio file cannot be processed.\n\nFile: <path>" titled "Error" and all media cleared (`Form.cs:1943-1953`). **Quirk:** by then the old audio has already gone from the timeline too, yet this path returns without marking modified and leaves Remove Audio, the tooltip and the panel title as they were. FIX in the quirk register: mark modified and reset them as F303 does. After adding: timing source switches to the media; if sequence length is exactly the 1 minute default it silently takes the audio length, else if different asks "Do you want to resize the sequence to the size of the audio?" titled "Resize sequence?"; effects get new media (`UpdateMediaOnSupportedEffects`); Remove Audio and Beat/Bar Detection enable; Speed / Tempo enables when the media is audio, and the audio takes that item's setting (`Form.cs:1954-1961`); the audio button tooltip becomes `Associated Audio: <file>`; timeline panel title becomes `Timeline - <file>`; modified. Not undoable.

**F303. Remove audio.** `RemoveAudioAssociation` (`Form.cs:1845-1882`): confirm "Are you sure you want to remove the audio association?" titled "Remove existing audio?"; removes media; tooltip back to "Associate Audio"; panel title "Timeline"; disables Remove Audio, Beat Bar Detection and Speed Tempo; modified. Not undoable. It does not clear the sequence's selected timing provider or refresh the speed controls (F272). At the next Play the provider finds no such media (F281), so the default timing source is used, but the stale provider is saved with the file. With no audio there is nothing to confirm, so no prompt.

**F304. Edit enablement from selection.** On selection change: Copy, Cut buttons and menu items, and Delete Effects enable exactly when any effect is selected (`Form.cs:2180-2185`).

### I. Drag and drop

**F305. Effect drop from the Effects panel.** `TimelineControlGrid_DragDrop` (`Form.cs:861-940`): file drop calls `HandleFileDrop`. Otherwise an effect GUID: not on a Deprecated row (drop rejected at hover and at drop): `EffectDropped`. Note the method then continues to the element test, with no effect because a GUID is not a colour. `EffectDropped` (`Form.cs:4232-4263`): ⌘ held with selection: alert "This action will replace {n} effects, are you sure ?" titled "Replace existing effects?" (only an explicit No cancels), then each selected effect is replaced by a new effect with the same row, start and duration; removed and added, new set selected. Undo: one `EffectsRemovedUndoAction` then one `EffectsAddedUndoAction`, so **two undo steps**. Otherwise add at drop time and row with default duration, selected.

**F306. Drop validity** (`IsValidDataObject`, `Form.cs:976-1044`). Effect GUID over a row: Copy unless Deprecated. Colour over an effect with colour properties: Copy unless the effect has discrete colours and the colour is not among them. Gradient over gradient-capable effect: Copy unless discrete colours exist and are not a superset of the gradient's colours. Curve over curve-capable effect: Copy. Result is cached per hovered effect while hovering (`_dragDropToElementEffect`). File drops always Copy on DragOver (`Form.cs:957-958`).

**F307. Colour drop on effect.** `HandleColorDrop` (`Form.cs:4373-4382`). Applies to selected effects **of the same type** if the target effect is selected, else just the target (`GetElementsForDrop`, `Form.cs:4335-4341`). If selection has other types, warn "Multiple type effects selected, this will only apply to effects of the type: <name>" title "Multiple Type Effects" with Cancel (`Form.cs:4273-4286`, `Form.cs:4343-4353`). If no plain Color properties: treat as gradient drop with one-colour gradient, so the status says `Gradient applied ...` (`Form.cs:4392-4397`). One property: set it. More than one (colour plus gradient properties): parameter picker at the mouse with a picture of each current value, status text "Choose the property to set, press Escape to cancel." (8 s) (`Form.cs:4355-4358`). Applies to each effect by property path via `ToNewOwner`. Completion status: `<type> applied to <n> <EffectName> effect(s).` (30 s) and undo `EffectsPropertyModifiedUndoAction` (`Form.cs:4360-4368`). **Upstream crash:** list property branches use `element`, not `e`, so with 2+ effects `elementValues.Add` throws on a duplicate key (`Form.cs:4474`, `Form.cs:4482`). Treat as GUARD: use the per-effect item.

**F308. Curve drop on effect.** `HandleCurveDrop` (`Form.cs:4510-4649`): as F307. One plain `Curve` property: set directly. Otherwise build pickers from Curve properties and each list's level pair curves ("Curve n", group = property name). Exactly one control: no picker. Picks apply to every effect by index, skipping effects with too few pairs (`Form.cs:4628-4631`). `HandleCurveDropOnGradientLevelPairList` is unreachable from the one-property branch since the guard requires `PropertyType == Curve` (`Form.cs:4524`, `Form.cs:4542`).

**F309. Gradient drop on effect.** `HandleGradientDrop` (`Form.cs:4686-4695`), `HandleGradientDropOnElements` (`Form.cs:4707-4839`): as F307. Exactly one property and it is a plain `ColorGradient`: set on each. Otherwise pickers: ColorGradient properties, each `List<ColorGradient>` item ("<property> n"), each level pair gradient ("Gradient n", group = property name). One control: no picker. Picks apply to every effect, by index, skipping those with too few items. Replacing a level pair keeps its curve. Status and undo via `CompleteDrop(..., "Gradient")`. Type groups: gradients = ColorGradient, List<ColorGradient>, List<GradientLevelPair>; colours = Color; all colours = both; curves = Curve, List<GradientLevelPair> (`Form.cs:4702-4705`).

**F310. Picker thumbnails.** Colour swatch 48x48, gradient and curve thumbnails 64x64, all with a 1 px border of RGB 136,136,136 (`Form.cs:4903-4935`).

**F311. Drop files from Finder onto the timeline.** `HandleFileDrop` (`Form.cs:4943-5033`). Modifiers are read at drop time: exactly ⌥ or ⌥⌘: horizontal orientation (default is the effect's own). Exactly ⌘ or ⌥⌘: place sequentially. Candidate effects = effect descriptors whose `SupportedFileExtensions` contain the lower-cased file extension. None: skip (the counter `i` still increments). One: use it. Several: reuse a choice already made in this drop, otherwise parameter picker "Choose the Effect to use, press Escape to cancel." (12 s status), 64x64 effect image, Escape skips that file. Each: new effect with defaults; sets every `PictureSource` property to File, every `ShapeList` to File, a String property named exactly `FileName` to the path, and `StringOrientation` to Horizontal when asked (`Form.cs:5007-5027`). Non-sequential: every file at the drop time on the drop row. Sequential: time + 2 s * index (`TimeSpan.FromTicks(20000000 * i)`). Duration default, selected, each with its own undo step. **Quirks (KEEP):** the file drop path has no Deprecated row check (`Form.cs:5029`); a drop outside any row logs and adds nothing, because `AddEffectInstance` catches the error (`Form.cs:3847-3852`); `.MTS` never matches since the dropped extension is lower-cased and the list holds `.MTS` (`Form.cs:4965`, `Vixen.Modules/Effect/Effect/StandardMediaExtensions.cs:5`). Extensions today: Video `.mp4 .avi .mov .MTS`, Picture `.jpg .jpeg .png .gif .bmp`, Shapes `.svg`, Glediator `.gled`, none shared, so the several-candidates path cannot currently happen.

**F312. Parameter picker window** (`Forms/FormParameterPicker.cs`, `EffectParameterPickerControl.cs`). Small borderless popup positioned at the pointer: left = pointer X if it fits in the screen width, otherwise pointer X - width; top = pointer Y, moved up to `height - picker height - 5` if it would overflow (`Form.cs:4288-4307`). Items are image plus label; clicking either selects (`EffectParameterPickerControl.cs:70-78`). A label is the property's display name unless the caller overrides it (`<property> n`, `Gradient n`, `Curve n`, or the effect's name) (`EffectParameterPickerControl.cs:20-31`). **Auto-cancels after 8000 ms** (`FormParameterPicker.cs:17`, `:113-115`, `:118-126`). Esc cancels (`FormParameterPicker.cs:164-172`). Layout: when items span more than one owner object: top-down, one group box per owner titled `OwnerDisplayName`, with ` <index+1>` for a collection child, cut to 10 characters (`FormParameterPicker.cs:21-63`). Otherwise left-to-right: ungrouped items first, then a group box per `GroupName`, ordered by group then index (`FormParameterPicker.cs:65-110`). **Upstream crash (not reachable today):** the constructor groups by `x.PropertyInfo.OwnerMetaData.Owner`, and the effect list used for file drops (F311) has no `PropertyInfo`, so a file claimed by 2+ effects would throw (`FormParameterPicker.cs:21`, `Form.cs:5051-5065`). GUARD: treat those as ungrouped. Mac: `NSPopover` at the pointer, with the 8 s timeout and Esc retained.

**F313. Property discovery rules** (`PropertyDiscovery.cs`). Only browsable, valid properties count. `ContainsTypes` and `CollectProperties` look at each property: type in set: match; otherwise a collection: recurse into each item, passing the item index as collection index (`:28-52`, `:79-105`). Note the second overload also includes the collection property itself (`:107-129`). `ToNewOwner(prop, newOwner)` (`:193-213`): same owner type: same descriptor with the new owner and the same index. Otherwise find a property of same name and type in the new owner tree that is a collection child with the **same index**; none: failure and that effect is skipped. In Rust the effect parameter tables replace reflection: `ParamKind` plus visibility (`visibility`) give browsable, and a stable "parameter path" (name, optional collection index) replaces descriptors.

**F314. Library drag payload** (`LibraryDragPayloadFactory.cs`). Curve or gradient: independent copy with `LibraryReferenceName` = name when linked (⌘ held), else empty, and "is current" false. Colour: the value. Linked with blank name: invalid argument. Other type: invalid argument. Upstream tests pin this (see Tests).

### J. Clipboard

**F315. Clipboard format.** Format name = full type name of `TimelineElementsClipboardData` (`Form.cs:114-115`). Written with `Clipboard.SetDataAsJson`; read with `Clipboard.TryGetData<T>` (`Form.cs:5161-5183`). Other Vixen windows and processes share it; a clipboard listener updates paste enablement across windows (`Form.cs:5099-5128`).

**F316. Paste enable rules** (`UpdatePasteMenuStates`, `Form.cs:5120-5128`), with count = effects on clipboard: Paste button and item: count > 0. Paste to visible marks button **visible** and menu item **enabled**: count > 0 and any visible collection has a mark at or after the cursor (`GetMarksPresent`, `Form.cs:3481-3498`). Paste Invert button visible and item enabled: count > 1. Paste dropdown and Paste Special item enabled: either of the last two enabled. Refreshed at load, on Edit menu opening (`Form.cs:3201-3204`), mouse entering the Edit toolbar (`Form.cs:3206-3209`), and on clipboard change when timeline data is present.

**F317. Cut and Copy** (`ClipboardAddData`, `Form.cs:5185-5251`). Nothing without a selection. Walks `VisibleRows` in order, skipping any row whose **name** was already seen, even in another group (`Form.cs:5203`). If this row has selected effects already taken, it counts a row number and skips (`Form.cs:5208-5212`). Otherwise each selected effect becomes an `EffectModelRecord(EffectModelCandidate(effect){Duration,StartTime,LayerId,LayerName,LayerTypeId}, relativeVisibleRow)` with relative row = row number minus first row having a selection; `EarliestStartTime` = minimum start. Note: a row skipped for a duplicate name does **not** advance the row number. Cut removes each effect immediately, and adds one `EffectsCutUndoAction` of all affected. Written to the clipboard last. Copy does not mark modified. Only visible rows are walked, so selected effects on collapsed or hidden rows are left out without a word, and a cut leaves them in the sequence.

**F318. Paste** (`ClipboardPaste(pasteTime)`, `Form.cs:5268-5429`). Returns effects pasted. Modes (`PastingMode` property set by the caller):
- Standard: record order. Target time = `effect.Start - offset + pasteTime`, with offset = earliest start unless paste time is 0, in which case offset 0 (so effects keep their **original times** when paste time is exactly zero).
- Visible Marks: effects sorted by start. For each visible collection: first mark at or after paste time, then take following marks, as many as effect count, stopping at the collection's end; all gathered, sorted ascending. nth effect goes to the nth mark time; stops when there are no more.
- Invert: row offset = last record's row offset minus this record's offset.
Target row anchor: `SelectedRow ?? ActiveRow ?? TopVisibleRow`; row = anchor visible index + relative row. Skips: target time > total; row index out of range; Deprecated row (names collected, status `Skipped pasting to Deprecated row(s): <names>.` for 10 s). If target + duration > total the duration is shortened. Layer: same id if present in this sequence, else same name and type, else default (`Form.cs:5385-5399`). Missing module data: "Effect(s) on clipboard are incomplete.  Please close Vixen and restart it." After: new set selected, modified, one `EffectsPastedUndoAction` **even when nothing was pasted**. **Index quirk (KEEP):** the count of effects pasted so far drives both the mark and the invert lookup (`Form.cs:5346`, `:5349`), so after a skipped effect (past the end, row out of range, Deprecated) the next effect takes the skipped one's mark or inverted row. An anchor that is not in the visible list gives index -1, which shifts everything up one row (`Form.cs:5333`, `:5354`).

### K. Tool windows

The Find controls and the library lists forward their key presses to the active editor (F225), and Marks, Layers and the Effect Editor do too except inside text boxes (`Forms/FindEffectForm.cs:47-50`, `Forms/Form_ColorLibrary.cs:282-285`, `Forms/Form_Marks.cs:43-52`, `LayerEditor.cs:55-64`, `Forms/FormEffectEditor.cs:217-227`). **The Effects window does not:** its `ProcessCmdKey` swallows every key, so hotkeys do nothing while its tree has focus, apart from Esc (F320).

**F319. Timeline tab** (`Forms/Form_Grid.cs`). Title "Timeline"; F9, F302 and F303 change it. No close button. Forwards keys when floating (`:14-19`, `:31-34`).

**F320. Effects window** (`Forms/Form_Effects.cs`, title "Effects", no window control box). Tree, roots in order "Basic Lighting", "Pixel Lighting", "Device Action", "Intelligent Fixture", children = effect names with representative image (18 pt) in descriptor order; first two groups expanded at load (`:35-97`). Group row icons: right arrow when collapsed, down arrow when expanded (`:160-168`). Full-row click on a group toggles it (`:128-138`). Selection is cleared after any select so no row stays highlighted (`:170-174`). Leaf drag: left press then move starts a drag with the type GUID (`:103-120`), including for the Device group. Leaf click: Device group: alert "Currently, you must drag this item to the grid to place it." titled "Effect Selection"; otherwise toggle as effect to draw: already chosen (blue back colour): `SelectedEffect = empty`, clear; else `SelectedEffect = guid`, all others cleared, this one blue (`:140-158`, `:194-217`). `DeselectAllNodes`: effect empty, cursor default, all backgrounds cleared (`:178-192`). Esc inside calls `EscapeDrawMode` so draw mode ends (`:226-234`, `Form.cs:2255-2261`). Every other key is swallowed there too, including tree navigation (`Forms/Form_Effects.cs:232-233`). The tree's highlight and the Effects strip's checks are not kept in step with each other. On the Mac let the list navigate with the arrow keys and pass other keys to F225: a small deliberate difference. **Note:** effect click only selects; it does not switch Draw Mode on. Default location left. Mac: SwiftUI `List` (about 60 items).

**F321. Find Effects/Layers window** (`Forms/FindEffectForm.cs`, title "Find Effects/Layers", menu item "Find Effects or Layers").
Layout: top row Find combo, then the **Collapse all Groups** check; next row Available combo; then the list, which has a one-item context menu (F327). Mac: `NSTableView`, since results can reach thousands. Query results come from Rust in bulk.

**F322. Find Effects/Layers > Find combo** (`comboBoxFind`): Effects, Layers; default Effects (`FindEffectForm.Designer.cs:100`, `FindEffectForm.cs:29`). Changing sets third column header "Layers" or "Effects", refills the second combo, clears the list (`:274-279`).

**F323. Find Effects/Layers > Available combo.** Filled on click and on mode change from every effect in every row: unique effect names, or unique layer names; sorted (`:190-215`, `:268-271`). Filled only then, so a new effect type does not appear until the combo is clicked again.

**F324. Find Effects/Layers > Choose an item.** Stores mode and search string, updates the list (`:261-266`).

**F325. Find Effects/Layers > List.** Columns Element, Effect Start Time, Layer (or Effects): one row per unique effect instance whose name or layer matches, on a background task, sorted by start (`:122-176`). Row text: first column `element.Row.RowLabel.Name`, second `StartTime.ToString()` (raw `hh:mm:ss.fffffff`, and with no fraction at all when it is zero), third the other attribute (`:178-188`).

**F326. Find Effects/Layers > Live refresh.** Row effect add or remove, only in effect mode and when name matches; and `ElementsFinishedMoving` always (`:85-111`). Row handlers are added once on first update and removed at close.

**F327. Find Effects/Layers > Double-click, or context menu Find all selected effects.** The menu item is enabled when anything is selected, updated on mouse up. `DisplaySelectedEffects`: if **Collapse all Groups** is on (default on; tooltip "When enabled will collapse all groups prior to finding selected Effects."), collapse rows; clear selection; `grid.DisplaySelectedEffects(selected)` (`:217-232`, `:238-253`).

**F328. Mark Collections window** (`Forms/Form_Marks.cs`, title "Mark Collections"). Hosts `MarkDockerView`; content belongs to the marks spec. Text boxes keep their own keys; other keys forward (`:43-52`).

**F329. Layers window.** See `LayerEditor` below: title "Layers", menu item "Layer Editor Window". On change: modified (`Form.cs:1217-1220`).

**F330. Layers window content** (`Vixen.Modules/Editor/LayerEditor/Views/LayerEditorView.xaml`, `ViewModels/LayerEditorViewModel.cs`, `Services/LayerEditorLayerService.cs`). Host `TSE/LayerEditor.cs` forwards keys except inside text boxes (`:55-64`). Add, remove, move, import and a successful Configuration mark the sequence modified (`LayerEditorViewModel.cs:339-355`, `Form.cs:1217-1220`). **Renaming a layer, Quick rename and changing the Type combo do not:** those change the layer object, and only the collection's change event and `Setup` returning true are watched. A change like that is lost at close unless something else is saved. FIX in the quirk register.

**F331. Layers > Toolbar.** Import Layers and Export Layers, in that order (icons `folder_go`, `folder_open`).

**F332. Layers > Add Layer.** Adds with the first available mixing filter type; new layer is at index 0 (`LayerEditorLayerService.cs:21-28`, `LayerEditorViewModel.cs:170-179`).

**F333. Layers > Remove Layer.** Selected layer, not the default; enable rule `layer != null && !default` (`:181-194`).

**F334. Layers > List.** Expander per layer titled with its name. Inside: **Name** text (commit on Enter and focus loss; Esc rolls back; not on typing); quick rename button, tooltip "Rename to filter type", hidden on the default layer; **Type** combo of filter types (`Descriptor.TypeName`), hidden on default; **Configuration** button, shown when the filter has setup: `Setup()`, raises `LayerChanged` when it returns true (`LayerEditorView.xaml:55-120`, `LayerEditorViewModel.cs:196-212`).

**F335. Layers > Quick rename.** Desired = filter type name; unique among other layers: `<name>`, else `<name> - 2`, `- 3`, ...; blank desired: "Layer" (`LayerEditorLayerService.cs:73-116`).

**F336. Layers > Drag to reorder.** Any non-default layer; drop on a row; on the last row (default, which stays last) index - 1. `MoveLayer` refuses default, unknown, negative, `>= count - 1`, same index (`LayerEditorView.xaml.cs:90-140`, `LayerEditorLayerService.cs:49-64`).

**F337. Layers > Export.** (`.v3l`, filter `Vixen 3 Layers (*.v3l)|*.v3l|All Files (*.*)|*.*`, title "Export Layers"): enabled when a non-default layer exists. Existing file: "A file with this name already exists. Do you want to replace it?" titled "Confirm Save As". IO error: `The layers could not be exported: <message>`, titled "Export Layers" (`LayerEditorViewModel.cs:234-276`).

**F338. Layers > Import.** (title "Import Layers"): read plan; IO error `The file could not be read: <message>`; invalid: plan's reason; none: "The file does not contain any layers that can be imported."; with skipped: `The following layers cannot be imported:\n\n<name>: <reason>...\n\nContinue importing the remaining layers?`; then import (`LayerEditorViewModel.cs:278-337`). Errors are titled "Import Layers". Plan rules: `Services/LayerImportExportService.cs`, pinned by `Vixen.Tests/Sequencer/LayerImportExportServiceTests.cs`.

**F339. Layers > Undo.** None for any of it. The Layer submenu in F227 lists layers in reverse of this list's order (`Form.cs:2445`).

**F340. Layer export and import rules** (`Services/LayerImportExportService.cs`, `ImportExport/*.cs`). Export: JSON, indented: `format` = `Vixen3Layers`, `version` = 1, `exportedUtc`, `layers` = non-default layers in list order, each `name`, `order` (index), `filterTypeId`, `filterName`, `filterDataType` (`<FullName>, <AssemblyName>`, omitted when blank), `filterData` (module data as JSON minus `ModuleTypeId` and `ModuleInstanceId`; omitted when null) (`:43-70`, `:186-214`, `:224-246`). Import plan, in order: read error; invalid JSON `The file is not valid JSON: <msg>`; null `The file does not contain a layer export document.`; other format `'<format>' is not a supported layer export format.`; other version `Layer export version <n> is not supported.`; no list `The file does not contain any layer records.` (`:73-117`). Records by ascending `order`: empty filter id: ignored; blank name: fails whole file `A layer record is missing its required name.`; filter not installed: skipped with `The '<name>' mixing filter (<id>) is not installed.`; has module data but file has none: skipped `The layer record does not contain filter configuration data.`; restore error: skipped `The filter configuration could not be restored: <msg>` (`:119-152`, `:250-270`). Import adds entries in reverse order so the exported order sits above existing layers; each gets a unique name (`:156-178`). The filter data JSON must match .NET `DataContractJsonSerializer` shape for files to move between platforms.

**F341. Color Library window** (`Forms/Form_ColorLibrary.cs`). Global favourites palette, not per sequence. Title "Color Library". Toolbar (`toolStripColors`): Edit Color, New Color, Delete Color, Export (tooltip "Export Favorite Colors"), Import ("Import Favorite Colors"), with icons `configuration`, `addItem`, `delete_32`, `folder_go`, `folder_open`, the same on the Curve and Gradient windows (`:79-88`). Grid of swatches, base 48 pt times scale, with a 2 pt border, tooltip `R: r G: g B: b` (`:113-120`, `:243-265`); spacing is swatch + 10.

**F342. Color Library > Selection.** Delete enabled with any selection; Edit enabled with exactly one (`:426-434`); after every refill both disabled (`:240`).

**F343. Color Library > New.** Colour picker from white, V not locked (`:192-207`). Mac: system colour panel.

**F344. Color Library > Edit.** Or double-click (one selected): picker from the colour, replace, save (`:376-399`, `:436-440`).

**F345. Color Library > Delete.** Alert "Are you sure you want to delete the selected color(s)?" titled "Delete color?", with Cancel button; OK removes (first match per value, since duplicates are allowed) (`:406-424`). **Note:** refills and saves regardless of the answer.

**F346. Color Library > Drag out.** `ItemDrag` starts a Move drag of the selected items; leaving the list starts a Copy drag with the first selected colour (`:463-471`). Mac: one drag with the colour on the pasteboard; reorder inside.

**F347. Color Library > Drop in.** (ignored unless moved more than 10 pt from the entry point): colour: inserted before the item under the pointer, else appended. Own items (Move): moved to the item's index, list order reversed when dragging leftward; else to the end (`:491-537`). Saves, raises changed.

**F348. Color Library > ⌘-scroll.** Scale *1.03 or /1.03, rebuilds (`:542-556`). Persist `Form_ColorLibrary/ColorLibraryScale` on close, invariant culture, default "1" (`:96`, `:560-564`). Mac: also pinch.

**F349. Color Library > Export.** `.vfc`, filter `Vixen 3 Favorite Colors (*.vfc)|*.vfc|All Files (*.*)|*.*`, starts in the last folder; error "Unable to export data, please check the error log for details" titled "Unable to export" (`:290-325`). **Import**: same filter; appends; error "Sorry, we didn't reconize the data in that file as valid Favorite Color data." titled "Invalid file" (`:327-370`).

**F350. Color Library > Persist.** `%AppData%/Vixen/ColorPalette.xml`, a `DataContractSerializer` of `List<Color>`, written on every change (`:105`, `:153-166`). Any change raises `ColorsChanged`, which rebuilds the strip (F164). **Not in undo.**

**F351. Curve Library window** (`Forms/Form_CurveLibrary.cs`, title "Curve Library"). App-wide `CurveLibrary` module. Toolbar: Edit Curve, New Curve, Delete Curve, Export (tooltip "Export Curve Library"), Import ("Import Curve Library"); below **Maintain Library Link** check (`LinkCurves`, default off, persist `CurveLinkCurves`, `Form.cs:364`, `Form.cs:5827`). Grid: 48 pt thumbnail times scale with name beneath, 7 pt times text scale, border 2 pt (`Forms/Form_CurveLibrary.cs:124-133`, `:172-207`).

**F352. Curve Library > Selection.** Delete enabled with any; Edit with one; refill disables both (`:206`, `:318-322`).

**F353. Curve Library > Edit.** Or double-click: `EditLibraryCurveAsync(name)` (`:213-227`, `:324-328`). Curve editor window from the curves spec.

**F354. Curve Library > New.** `AddCurveToLibraryAsync(new Curve())`, F362.

**F355. Curve Library > Delete.** Alert "If you delete the Selected library curve(s), ALL places it is used will be unlinked and will become independent curves. Are you sure you want to continue?" titled "Delete library curve?"; OK removes all in a bulk update (`:291-316`).

**F356. Curve Library > Drag out.** Payload `LibraryDragPayloadFactory.Create(curve, name, link = LinkCurves)`, Move and Copy (`:346-357`). The strip uses ⌘ for the link instead (F168).

**F357. Curve Library > Drop in.** (ignored within 10 pt of entry): own item (Move): reorder, as F341. Curve not a library reference (Copy): `AddCurveToLibraryAsync(c,false)`; returns on cancel; inserted at the pointer's item index if counts match. Then the library is rebuilt in list order inside a bulk update (`:359-444`).

**F358. Curve Library > ⌘-scroll.** Image scale *1.03 or /1.03, floor 0.1. With Shift held in the list (`_scaleText`): text scale, floor 0.2 (`:578-629`). Persist `Form_CurveLibrary/CurveLibraryImageScale`, `CurveLibraryTextScale`, invariant culture; load floors 0.1 and 0.2 (`:101-107`, `:566-576`).

**F359. Curve Library > Export.** `.vcl`, filter `Vixen 3 Curve Library (*.vcl)|*.vcl|All Files (*.*)|*.*`, `Dictionary<string, Curve>` by `DataContractSerializer`; error "Unable to export data, please check the error log for details" titled "Unable to export" (`:450-487`).

**F360. Curve Library > Import.** Same filter. Existing names get ` 2`, ` 3`, ... appended with no prompt. Error "Sorry, we didn't reconize the data in that file as valid Curve Library data." titled "Invalid file" (`:489-541`).

**F361. Curve Library > Change raises `CurveLibraryChanged`, rebuilding the strip; `CurvesChanged` on the library rebuilds this list and triggers dirty re-render (`Form.cs:2133-2136`). Not in undo.** 

**F362. Add curve to library** (`AddCurveToLibraryAsync(c, edit=true)`, `Forms/Form_CurveLibrary.cs:241-289`; returns true when cancelled). `TextDialog("Curve name?")` loop. Blank: "Please enter a name." titled "Curve Name", retry. Existing: "There is already a curve with that name. Do you want to overwrite it?" titled "Overwrite curve?" with Yes, No, Cancel: Yes adds (and opens the editor if edit); No loops back; Cancel returns. Otherwise adds (opens editor if edit).

**F363. Gradient Library window** (`Forms/Form_GradientLibrary.cs`, title "Gradient Library"). Same structure and behaviour as F351, read by comparing the two files. Differences: buttons Edit Gradient, New Gradient, Delete Gradient, Export (tooltip "Export Gradient Library"), Import ("Import Gradient Library"), check **Maintain Library Link** (persist `GradientLinkGradients`). Thumbnails by `GenerateColorGradientImage(size, false)`. Add to library (`:237-286`): blank name "Please enter a name." titled "Warning"; `TextDialog("Gradient name?")` as in F362; existing "There is already a gradient with that name. Do you want to overwrite it?" titled "Overwrite gradient?". Delete: "If you delete the selected library gradient(s), ALL places it is used will be unlinked and will become independent gradients. Are you sure you want to continue?" titled "Delete library gradient?" (`:296-298`). Export and Import: `.vgl`, filter `Vixen 3 Color Gradient Library (*.vgl)|*.vgl|All Files (*.*)|*.*`, a `Dictionary<string, ColorGradient>`; export error "Unable to export data, please check the error log for details." (with a full stop) titled "Unable to export"; import error "Sorry, we didn't recognize the data in that file as valid Color Gradient Library data." titled "Invalid file" (`:437-530`). Persist `Form_GradientLibrary/GradientLibraryImageScale`, `GradientLibraryTextScale`.

**F364. Color Collection Editor** (`Forms/ColorCollectionLibrary_Form.cs`, modal, opened by F86). Controls: top row "Color Collection" combo, + and - buttons (icons `add`, `minus`), "Description" text; swatch grid (48 pt, tooltip `R: r G: g B: b`); bottom row left to right: Import Collection, Export Collection, Add Color, Remove Color, OK, Cancel.

**F365. Color Collection Editor > Picking a collection.** Fills description, enables Delete and description, shows colours (`:81-87`). Add Color is enabled once a collection shows (`:396`); Remove Color with a selection (`:109-112`). Refilling the combo disables Delete and Add (`:362`).

**F366. Color Collection Editor > New.** `TextDialog("Collecton Name?")` (sic). Blank: "Please enter a name." titled "Color Collection Name", retry. Duplicate (`Contains`, by name equality in `ColorCollection`): "A collection with the name <name> already exists. Do you want to overwrite it?" titled "Overwrite collection?" with Cancel; OK replaces; either ends. New is selected (`:298-338`).

**F367. Color Collection Editor > Delete.** "Are you sure you want to delete the collection: <name> ?" titled "Delete collection?" (`:278-296`).

**F368. Color Collection Editor > Add Color.** Picker beside this window, starting at the last picked colour (`_colorValue`); adds (`:177-197`).

**F369. Color Collection Editor > Remove Color.** Removes selected (first match per value) (`:160-175`).

**F370. Color Collection Editor > Description.** Updated on each key up (`:114-120`).

**F371. Color Collection Editor > Reorder.** Drag within the grid, first selected only; no target: nothing (`:126-154`).

**F372. Color Collection Editor > Export.** `.vcc`, `Vixen 3 Color Collections (*.vcc)|*.vcc|All Files (*.*)|*.*`, all collections (`:199-221`). **Import**: per duplicate "A collection with the name <name> already exists. Do you want to overwrite it?" Yes/No; then "Imported <n> Color Collections." titled "Color Collections Import" (`:223-275`).

**F373. Color Collection Editor > Close with changes.** "You will loose any changes, do you wish to save them now ?" titled "Warning": Yes keeps as OK; Cancel stays open (`:89-102`). OK clears dirty first (`:104-107`).

**F374. Color Collection Editor > Quirk (data):.** The caller passes a shallow list copy, so edits to existing collections' colours and description mutate the live objects; Cancel relies on reloading from file (`Menu.cs:440-449`). Rust: edit a deep copy and commit on OK; cancel then has the same visible result.

**F375. Effect Editor window** (`Forms/FormEffectEditor.cs`). Grid content is the effect editor spec's. Host contract here: selection change is debounced **200 ms** and then shows selected effects (`:59-63`, `:126-136`). Value change: each selected effect `UpdateNotifyContentChanged`; one `EffectsPropertyModifiedUndoAction`, always pushed: with no old values it is empty and reads `0 Effect(s)  modified` (`:145-163`). FIX proposal: skip an empty one. Text box commit on focus loss (`:180-210`). Keys forward except inside a text box (`:217-227`). **Preview**: when the editor's preview toggle is on and effects are selected, a preview context `"Effect Preview"` plays the selected effects shifted so the first starts at 0, looping by a timer whose interval is total duration (`:243-281`). `PreviewStop`: stop loop; `Clear()`, or `Clear(false)` if not running or paused (`:283-298`). `ResumePreview` resumes if toggled on (`:300-303`). Real playback and Export stop the preview and resume it at their end (F269, F267, F37).

**F376. Row labels** (`TimedSequenceRowLabel.cs`). Expander shows when `HasExpandableChildRows`: +/- icon centred in the icon area; when child active indicators are on and any child is active: active icon on a yellow fill. Coloured dots for each tag with a `DisplayColor`: 8 pt dots, 3 pt gap, starting `textWidth + 6` after the label, vertically centred, tags sorted by `SortOrder`. Mostly drawn by the timeline's row list (see timeline spec) but the dots and icons are defined here.

**F377. Effect element** (`TimedSequenceElement.cs`). Time change pushes `EffectNode.StartTime`, effect `TimeSpan` and `StartTime`, target changes push to `TargetNodes`, unless events are suspended; drawing goes through `EffectRasterizer`.

**F378. Manage Tags.** The timeline's row context menu raises it (timeline spec). `ElementTagManagerWindow.ShowAsDialog(saveOnClose: true)`, because the sequencer has no OK or Cancel of its own to save it on; when it returns true, row labels repaint (`Form.cs:2317-2326`). Changing a row's tags reapplies the hidden row filter (`Form.cs:2308-2315`). Tags live in the system configuration, so no sequence modified flag.

### L. Dialogs and file formats

`MessageBoxForm` alerts are given with the action they belong to. Sheets on the Mac unless noted.

**F379. Edit Time dialog** (`EffectTimeEditor.cs`, title "Effect Time Editor"). Fields top to bottom: Start, Duration, End (time boxes); button "Set" beside "Match sequence start/end" (start 0, end = length, duration = length), OK, Cancel. Limits: duration minimum = minimum element duration; start maximum = length - minimum; end maximum = length; duration maximum = length - start; end minimum = start + minimum (`:9-25`, `:70-80`). Set: `:96-101`. Cross-field, with handlers off during the update: changing Start sets End = Start + Duration, and if Start + Duration passes the length, Duration = length - Start; updates duration maximum and end minimum. Changing Duration sets End. Changing End sets Duration = End - Start (`:103-138`). Host F245.

**F380. Bulk Effects Move dialog** (`BulkEffectMoveForm.cs`, title "Bulk Effect Move"). Start Time and End Time, both initially the cursor; Offset, initially 0; all maximum = sequence length (`:13-22`). Group Direction: Forward (default), Backward. "Process visible rows only", "Clip Effects", "Include Marks", all off. OK, Cancel. Host F92; clipping rules in the timeline spec.

**F381. Effect Distribution dialog** (`EffectDistributionDialog.cs`, title "Effect Distribution"). Label `Effects selected: <n>`. All time boxes maximum = length. Group Time Control: Start Time, End Time. Group Duration Control: Equal Duration, Do not change duration, Specified duration with its time box. Group Placement Control: Stair stepped, Overlapping with box, Spaced duration with box. Group Starting point: First Effect, Last Effect. OK, Cancel. Host F294 sets Equal, Stair stepped and First Effect. Setter quirk: `EffectPlacementOverlap` setter writes to the Spaced box, and nothing calls it (`:90-100`).

**F382. Sequence Length dialog** (`SetSequenceLength.cs`): title "Sequence Length"; label "Enter new sequence length:"; time box; OK, Cancel. No validation in this file beyond the time box.

**F383. Missing Audio File dialog** (`InvalidAudioPathDialog.cs`). `The audio file <name> could not be found at the following location:`, full path, "Selected the action you wish to take:" (sic), buttons Select New Audio (Locate), Forget This Audio (Remove), Proceed As Is (Keep) (`:15-17`, `:20-36`). Closing it any other way leaves the default result, Locate (`:41`). Host F9.

**F384. Add Multiple Effects dialog** (`Forms/Form_AddMultipleEffects.cs`). Title `Add <effect name> effects` (`:34-37`). Controls top to bottom: label `<n> effects possible.`, Number of effects to add (numeric), Starting time, Ending time, Duration, Duration between, expander "Show beat mark alignment options" (+/- buttons), panel (collapsed by default) with Align to beat marks, Skip every other beat, Fill duration between marks, Align to mark start / end, mark collection check list (coloured by collection colour, hidden until Align is on), then Select / Edit effects, OK, Cancel. No window close box.

**F385. Add Multiple Effects > Limits.** Start, duration, between, end maxima = constructor's length; duration min 10 ms (`:161-165`).

**F386. Add Multiple Effects > Possible count, not aligned (`:396-419`).** I from 1: uses = duration*i + between*(i-1); while uses > 0 and uses < (end - start): i++. Then i--. Count value and maximum = i. Label updates. OK enabled when i != 0. **Note the strict test:** a set that fits exactly is not counted. Duration 0 gives 0 effects.

**F387. Add Multiple Effects > Possible count, aligned (`:421-445`).** Times = marks from checked collections (by name) with start <= t < end. possible = times.count, minus 1 with Fill (floor 0); Skip Every Other: ceil(possible/2). Count value and maximum set. OK enabled when times is nonempty. Note the host's `AddEffectsToBeatMarks` uses only `>= start`, with no end limit (F292), but stops at count.

**F388. Add Multiple Effects > Recalculates on any time change and on any of the check boxes or mark checks.** 

**F389. Add Multiple Effects > Double-click the possible label.** Count = maximum (`:191-194`).

**F390. Add Multiple Effects > Align to beat marks.** Disables Duration Between; shows the list; `SetCheckboxStates`; repopulates list (clears checks); warns "You have Beat Mark collections with duplicate names.\nBecause of this, your results may not be as expected." titled "Duplicate Names" (`:232-263`).

**F391. Add Multiple Effects > Check rules.** Fill, Skip, Mark Start/End can only be toggled while aligned (`AutoCheck`), and appear greyed otherwise (`:265-279`). Fill on: disables Mark Start/End and Duration (`:307-322`). Mark Start/End on: disables Fill and Duration; **turning it off does not re-enable Duration** (`:324-339`).

**F392. Add Multiple Effects > OK (`:208-230`).** Count 0: "OOPS! Your effect count is set to 0 (zero)" titled "Warning", stays open. Not aligned and not fill and `start + duration*count + between*(count-1) > end`: "At least one effect would be placed beyond the sequence length, and will not be added.\n\nWould you like to proceed anyway?"; No stays open.

**F393. Add Multiple Effects > Restoring checked marks with Shift (F291) runs before the list is populated unless Align set it first.** `AlignToBeatMarks` setter fires the change handler which populates, then `CheckMarkItem` finds items by text (`Form.cs:2512`, `:2518-2521`).

**F422. Add Multiple Effects > Typing the count.** Key down in the count box hides its tooltip and enables OK; key up enables OK exactly when the text parses as an integer (`Forms/Form_AddMultipleEffects.cs:196-206`). So typing can re-enable an OK that F386 or F387 disabled. F392 then catches a count of 0 and an overrun. Any later recalculation (F388) overwrites what was typed.

**F394. Create Evenly Divided Marks dialog** (`CreateEvenMarksForm.cs`, title "Create Evenly Divided Marks"). Labels Start Time, End Time, Number of Divisions (range 1 to 10000, see `docs/reference/forms/Vixen.Modules__Editor__TimedSequenceEditor__CreateEvenMarksForm.json`), OK, Cancel. Start = last remembered playback start or 0; End = last remembered playback end or 0; both maximum = sequence length. Note a remembered end of nil gives End 0 and so a zero span, which fails the check in F82 until the user edits it.

**F395. Text dialog.** `Common.Controls.TextDialog`: prompt label, text box, OK, Cancel; optional title and initial text (`Vixen.Common/Controls/TextDialog.cs`). Used by F297, F362, F363, F364. Sheet on the Mac.

**F396. Export dialog** (`ExportDialog.cs`, title "Export Sequence"). Controls top to bottom. Group Settings: Format combo at the left, sorted: CSV File, Falcon Player Effect, Falcon Player Sequence, Falcon Player Sequence 2.6+, Helix File, Vixen 2 Routine, Vixen 2.1 Sequence (from `FileTypeDescr` in `Vixen.Core/Export/*Writer.cs`, verified by search); Timing (ms) combo (`resolutionComboBox`) at the right. Group Network: reorderable, checkable table with columns Controller, Channels, Start, End. A read-only note "You must have all of your required display elements to controller outputs fully patched in the Display Setup for the export to produce correct results." Generate Controller File check beside FPP 1.x and FPP 2.x radios (2.x default). Enable Compression check. Buttons Start, Stop, and Close at the right. Status bar: progress label, progress bar, percent label. Resolution items: 25, 50, 100; it is a free-text combo (`ExportDialog.Designer.cs:113-116`). **This is the owner's main workflow: their show plays from an exported FSEQ, so `.fseq` output is critical** (the Falcon formats; `vixen-export` has the writers).

**F397. Export > Load.** Format index from `<Name>/ExportFormat` default 0, which is CSV File in the sorted list; resolution index from `<Name>/ExportResolution` default 1; `GenerateUniverse` false; `Universe2x` true; `Compress` true; compress enabled = `CanCompress(format)`; Stop disabled; list built by `Controller.Index`; active rows checked and given running Start and End (1-based); inactive blank (`ExportDialog.cs:70-90`, `:239-282`). **Controller choices are not saved:** each opening builds a new `Export` whose controller list starts all active, in system order (`ExportDialog.cs:42`, `Vixen.Core/Export/Export.cs:148-167`, `:596`). KEEP, and flag it to the owner, since this is the FSEQ path.

**F398. Export > Check or drag reorder.** `ReIndexControllerChannels`: `Index` = row, `IsActive` = checked, renumbers Start and End for active rows (`ExportDialog.cs:284-313`). Session only (F397).

**F399. Export > Start (`ExportDialog.cs:114-201`).** If the sequence had **no file name when the dialog opened, returns silently** (`ExportDialog.cs:45`, `:119-123`), so a never-saved sequence cannot export and saving while the dialog is up does not help. KEEP. Save dialog in `ExportDir`, file `<sequence>.<ext>`, filter `<format>|*.<ext>|All Files(*.*)|*.*`. Resolution must parse as integer, else "Invalid timing interval {text}, check the value and try again." `await DoExport(sequence, format, compress, progress)`. Falcon formats: if `!AllSelectedControllersSupportUniverses` warn "Some of the selected controllers do not support universes\nThese controllers will not be included in the universes file.\nSome manual FPP output configuration will be required."; if Generate is enabled and checked: 2.x writes `<name>-co-universes.json`, 1.x writes `<name>-universes`. Other formats with Generate checked: `WriteControllerInfo`. These steps follow the `await` with no check, so they also run after a cancel.

**F400. Export > Stop.** `_cancelled = true`, `Cancel()` (`ExportDialog.cs:209-213`). **Close** only closes the dialog: it has no handler, so a running export carries on unobserved, and the host resumes the effect preview at once (F37, `:412-415`). FIX in the quirk register: closing cancels the export.

**F401. Export > States (`ExportDialog.cs:332-359`).** Working: Start, format and resolution disabled; Stop enabled; progress visible; status `Saving: <sequence file name without extension>`. End: `Export Complete` or `Export Canceled`. Each progress report sets the bar and the label `<n>%` (`ExportDialog.cs:203-207`).

**F402. Export > Enable rules (`ExportDialog.cs:434-483`).** Format change stores index, updates compress and universe radios. Radios enabled only when Generate is checked and format `IsFalconFormat`. FPP 2.x check enables compress: **`chkCompress.Enabled = radio2x.Checked`**, overriding the format's rule. Resolution with non-integer resets to default index. Each setting is stored at once, except that a typed Timing value is not: only a list choice stores anything, though Start uses whatever is in the box. `ShowDestinationMB` is not called from this file.

**F403. Effect Defaults export selection** (`Forms/EffectDefaultsExportSelectionForm.cs`, title "Export Effect Defaults"): "Select saved effect defaults to export:", checked list sorted by effect name, all checked, click toggles, OK, Cancel. Result is checked type ids.

**F404. Papagayo file format** (`TSE/PapagayoDoc.cs`). Line-based, leading whitespace trimmed: line 1 must contain `lipsync`, else `IOException("Invalid File Format")`; sound path; FPS (int); sound frames (int); voice count; then per voice: name; phrase text (`|` means newline); phrase count; per phrase: text, start frame, end frame, word count; per word: `<text> <start> <end> <phonemes>` (word lines with other than 4 fields are left empty); per phoneme: `<frame> <name>`. Extra trailing lines throw (`:177-178`). End of file inside a voice, phrase or word throws `Corrupt File Format` (`:265-268`, `:396-399`, `:474-477`). Phoneme end = next phoneme's frame - 1; last phoneme end = word end (`:501-504`, `:563-566`). Unknown phoneme name: `Rest` (`:554-561`). Types: AI, E, O, U, FV, L, MBP, WQ, etc, Rest (`:5`). Times: sample = 1000/FPS ms; start = sample*StartFrame; end = sample*(EndFrame+1) (`:15-21`, `:34-56`). Per voice phonemes are **coalesced** (`:311-348`): walk every frame 0..SoundFrames-1, get its phoneme (Rest where nothing covers it); merge runs of the same type, keeping the first phoneme's lyric word; the last run is kept. FPS and SoundFrames are static (one doc at a time). Note `Load` errors are not caught by the menu handler (`Menu.cs:514`): GUARD with an alert. Voice dictionary keys are trimmed names; a duplicate voice name throws.

### M. Undo actions

Upstream stack: each action's `Undo()` and `Redo()` act directly (`Undo/*.cs`). Mac: one Rust undo stack per sequence window (the document's `UndoManager` mirrors its current name). Descriptions below are the Edit menu names and the dropdown list texts. Colon citations in a row are in that class's file under `TSE/Undo/`. `s` is "s" unless the count is 1 (bulk marks: only when count > 1).

| # | Class | Description | Undo | Redo | Raised by |
|---|---|---|---|---|---|
| F405 | `EffectsAddedUndoAction` | `Added {n} effect{s}` | remove | add | F291, F292, F243, F284, F285, F305, F287 |
| F406 | `EffectsPastedUndoAction` | `Paste {n} effect{s}` | remove | add | F318 |
| F407 | `EffectsCutUndoAction` | `Cut {n} effect{s}` | add | remove | F317 |
| F408 | `EffectsRemovedUndoAction` | `Removed {n} effect{s}` | add | remove | F290, F305 |
| F409 | `ElementsTimeChangedUndoAction` | by move type: `Moved`, `Resize`, `Start Align`, `End Align`, `Align`, `Duration Align`, `Start to End Align`, `End to Start Align`, `Center Align`, `Distributed` + ` {n} effect{s}` | swap | swap, first relinking effects that were deleted and re-added (`:29-46`) | grid `ElementsMovedNew` (`Form.cs:5507-5511`) |
| F410 | `EffectsLayerChangedUndoAction` | `Modified {n} effect layer{s}` | swap | swap | F227 |
| F411 | `EffectsModifiedUndoAction` | `{n} Effect(s) {label} modified` (label default `properties`) | swap serialized module data; missing data: "Incomplete effect data encountered.  Please close Vixen and restart it." | same | **Nothing raises it:** no code in the source tree calls `AddEffectsModifiedToUndo(dict, label)` (`Form.cs:5564-5567`); every caller uses the overload that takes F412 |
| F412 | `EffectsPropertyModifiedUndoAction` | `{n} Effect(s) {property display name} modified` | swap per effect property | same | F307-F309, F253, F375 |
| F413 | `MarksAddedUndoAction` | `Added {n} Mark{s}` | remove | add | F297, F299, F300, F296 |
| F414 | `MarksRemovedUndoAction` | `Removed {n} Mark{s}` | add | remove | F300 |
| F415 | `MarksTimeChangedUndoAction` | **says effect:** `Moved {n} effect{s}` or `Resize {n} effect{s}`; other move types throw | swap, raise moving event, dirty re-render | same, skipping marks that left their collection | F300 |
| F416 | `MarksBulkChangeUndoAction` | `Bulk Change {n} Mark{s}` | re-add deleted marks (value null) or swap | remove, or swap | F92 |

**F417. Layer lost on undo (quirk).** `EffectsAddedRemovedUndoAction` captures each effect's layer (`Undo/EffectsAddedRemovedUndoAction.cs:12-21`), but `addEffects` calls `AddEffectNode(node)` with no layer (`:31-35`), and removal already removed it from the layer map (`Form.cs:3746`). So undoing a delete, cut, or redoing an add places the effect on the default layer. FIX proposal for veto: use the captured layer through `AddEffectNode(node, layer)` (`Form.cs:3614`), which is what the stored value was evidently for.

**F418. Actions with no undo:** audio add or remove, sequence length, Beat/Bar detection, Create Evenly Divided Marks, mark text edits, clone to selected effects, colour, curve and gradient library changes, colour collections, lip sync map changes, layer editing (F339), mark collection creation (F298), row height and tree changes, zoom, and playback range. Keep them outside the stack.

**F419. Undo stack rules** (`Vixen.Common/Controls/Undo/UndoManager.cs`). Max 1000 undo items; the oldest falls off (`:21`, `:115-122`). `Undo(n)`, `Redo(n)` run n steps; out of range throws (`:71-83`, `:85-107`); redo re-pushes on the undo stack. **Quirk:** `AddUndoAction` does **not** clear the redo stack (`:65-69`), so after Undo plus a new edit, Redo still offers stale actions, which can act on removed effects. FIX proposal: clear on new action, as `NSUndoManager` does.

**F420. Undo and Redo buttons** (`Vixen.Common/Controls/Undo/UndoButton.cs`). Split button with a 200x200 drop-down list of descriptions, newest first; choosing an item undoes down to it (`ItemChosen` with count) (`:9-65`). Mac: a `Menu`-styled toolbar control listing action names, built from the Rust stack.

**F421. Undo wiring.** `InitUndo` (`Form.cs:5437-5450`): Undo and Redo buttons have dropdown lists of descriptions (newest first from the manager) with a multi-step choice `Undo(n)`/`Redo(n)` (`Form.cs:5458-5471`). Enable with `NumUndoable`/`NumRedoable` above zero (`Form.cs:5474-5504`). Element moves come from the grid (`Form.cs:5507-5511`). Layer swap reassigns and modified (`Form.cs:5539-5549`).

### N. Quirk register

Upstream oddities in one place, for the owner's review. **KEEP** means ported as written, with a code comment saying it is deliberate (`docs/PORTING_GUIDE.md`). **GUARD** means upstream throws or crashes; do the stated safe thing rather than panic. **FIX** is a proposed divergence, listed here for the owner's veto; if vetoed, port it as written.

| Tag | Where | What |
|---|---|---|
| FIX | F99 | View > Toolbars checks go stale. Use one source of truth |
| FIX | F163 | Fixture group setting read and written under different keys, and its menu closes on click |
| FIX | F244 | Clone to selected effects has no undo and does not mark modified |
| FIX | F417 | Undoing a delete or cut loses the effect's layer. Already done this way in `vixen-session::undo`; a veto means changing it there |
| FIX | F419 | New edits do not clear the redo stack. Already done this way in `vixen-session::undo`; a veto means changing it there |
| FIX | F132 | Choosing an audio device with no audio loaded moves the check but not the device |
| FIX | F137 | File strip Export button does nothing |
| FIX | F177 | Hidden colour items come back on reload (looked up under the gradient strip's name); a later profile's library strips never store their Show/Hide choices |
| FIX | F12 | A mapped sequence in the normal folder loses its modified flag at load, so the mapping can be lost with no prompt |
| FIX | F16 | Cancelling the save panel raised by the close prompt still closes the window and loses a never-saved sequence |
| FIX | F170 | Dragging a colour, curve or gradient over a library strip overwrites the last hovered button's value |
| FIX | F266 | A cancelled countdown leaves the Play menu item disabled |
| FIX | F302 | Zero-length audio removes the old audio without marking modified and leaves the audio controls stale |
| FIX | F330 | Renaming a layer or changing its type does not mark the sequence modified |
| FIX | F375 | An effect editor change with no old values pushes an empty undo step |
| FIX | F400 | Closing the Export dialog during an export leaves it running unobserved |
| GUARD | F307 | Colour drop on 2+ effects with a list property adds a duplicate dictionary key |
| GUARD | F253 | Colour collection on an effect with 2+ colour properties adds a duplicate key |
| GUARD | F312 | Picker constructor with no `PropertyInfo`, not reachable with today's effects |
| GUARD | F404 | Papagayo parse errors are not caught by the menu handler |
| GUARD | F415 | Mark undo description throws on move types other than Move and Resize; show the Move wording |
| GUARD | F253 | Colour collection with none of an effect's discrete colours indexes an empty list or divides by zero; skip that effect |
| GUARD | F299 | A word mapping that still converts to nothing divides by zero; skip the word |
| GUARD | F173 | Library Edit with no button under the pointer acts on a stale button or a null one |
| GUARD | section D | Space and the F5 key with no context dereference null; also `AddMarkAtTime` (`Form.cs:3048`, `:3061`); do nothing |
| GUARD | Data and persistence | A settings value that does not parse throws at load; use the default and log it |
| GUARD | `Menu.cs:172-173` | Paste with no rows at all dereferences a null row; do nothing |
| GUARD | `Form.cs:1652-1662` | Corrupt `ColorCollections.xml` throws at load; treat as empty and log it |
| GUARD | `ColorCollectionLibrary_Form.cs:199-221`, `:238-266` | Collection import and export have no error handling; show an alert |
| KEEP | F15 | Auto save counts from the first edit, and a never-saved sequence raises the save dialog |
| KEEP | F273 | A plain ruler click marks the sequence modified |
| KEEP | F305 | Replace by ⌘-drop takes two undo steps |
| KEEP | F311 | `.MTS` never matches on file drop; file drop has no Deprecated row check |
| KEEP | F318 | Paste time exactly 0 keeps original times, including with the cursor at 0; an undo entry is added even when nothing pasted |
| KEEP | F295 | Effects whose nearest mark or kept edge is exactly 0 are skipped |
| KEEP | F28 | Pause menu item is always disabled |
| KEEP | F31 | Choosing any delay option, Off included, turns Loop off |
| KEEP | F82 | Exact-match mark collection is removed with no undo |
| KEEP | F16 | Zoom, scroll or row height changes cause a silent save at close |
| KEEP | F384 | A set of effects that fits exactly is not counted; turning Mark Start/End off leaves Duration disabled |
| KEEP | F341 | Declining the delete prompt still refills and rewrites the palette |
| KEEP | F415 | Mark move undo says "effect" |
| KEEP | F71, F72 | Reopened Layer Editor and Find windows go to the left |
| KEEP | F244 | Clone to selected needs more than 2 selected |
| KEEP | F123 | Invert also flips its own check |
| KEEP | F119 | Esc leaves an Effects strip button checked with no effect chosen |
| KEEP | F130 | The wheel changes speed over a disabled gauge and enables the +/- buttons |
| KEEP | F271 | The +/- buttons stop at a shown 20% and 400% with their enable state stale; the wheel reaches 10% |
| KEEP | F291 | Add Multiple always opens at the maximum possible count, not 2 or the last count |
| KEEP | F317 | Copy and cut leave out selected effects on rows that are not visible |
| KEEP | F318 | After a skipped effect, the next one takes its mark or inverted row |
| KEEP | F397 | Export controller order and checks reset with each opening |
| KEEP | F399 | Export does nothing, silently, for a sequence with no file name |
| KEEP | F423 | Saving during a ⌘-started play stores the temporary range |
| KEEP | wording | Upstream typos stay unless the owner objects: "then" for "than" (F228), "ajusting" (F233), "Label Postion" (F157), "Selected the action" (F383), "Collecton Name?" and "loose" (F364), "reconize" (F341, F351), and `Increasing Timing speed` (F131) |

**Differences forced by the Mac** (not quirks): Close is ⌘W, not ⌘Q (F38). Save As is ⌥⌘S because ⇧⌘S is taken (F198). Pause is ⌥Space (F192). Multi-tick menus become popovers (see Windows-only omissions). Redo is ⇧⌘Z. Zoom Time In is ⌘=, not ⌘+ (F54). Tool panels hide rather than dispose, so their state survives (F2). The Effects list navigates with arrow keys (F320).

### O. Control index for parity

Every object in `TimedSequenceEditorForm.Designer.cs` (from `docs/reference/forms/Vixen.Modules__Editor__TimedSequenceEditor__TimedSequenceEditorForm.json`), for `docs/parity/sequencer.md`. Only separators and the designer's layout containers are left out. Dialog and tool window controls are named in their own entries.

| Upstream object | Text | Entry |
|---|---|---|
| `menuStrip` | Menu | F77 |
| `sequenceToolStripMenuItem` | Sequence | F24 |
| `playbackToolStripMenuItem` | Playback | F27 |
| `playOptionsToolStripMenuItem` | Play Options | F31 |
| `editToolStripMenuItem` | Edit | F316 |
| `viewToolStripMenuItem` | View | F54 |
| `toolsToolStripMenuItem` | Tools | F78 |
| `audioToolStripMenuItem` | Audio | F78 |
| `helpDocumentationToolStripMenuItem` | Help | F96 |
| `toolStripAudio` | Audio | F99 |
| `contextMenuStripAll` |  | F100 |
| `add_RemoveContextToolStripMenuItem` | Show/Hide Items | F101 |
| `toolbarsToolStripMenuItem` | Toolbars | F102 |
| `imageSizeToolStripMenuItem` | Toggle Image Size | F103 |
| `resetToolStripMenuItem` | Reset Toolbar | F104 |
| `toolStripButton_AssociateAudio` | Associate Audio | F128 |
| `toolStripButton_DecreaseTimingSpeed` | Decrease Timing speed | F129 |
| `audioToolStripLabel_TimingSpeed` | Timing Speed: 100% | F130 |
| `toolStripButton_IncreaseTimingSpeed` | Increasing Timing speed | F131 |
| `audioToolStripButton_Audio_Devices` | Audio Devices | F132 |
| `toolStripMenuItem_Save` | Save | F24 |
| `toolStripMenuItem_SaveAs` | Save As... | F25 |
| `autoSaveToolStripMenuItem` | Auto Save | F26 |
| `playToolStripMenuItem` | Play | F27 |
| `pauseToolStripMenuItem` | Pause | F28 |
| `stopToolStripMenuItem` | Stop | F29 |
| `toolStripMenuItem_Loop` | Loop | F30 |
| `delayOffToolStripMenuItem` | Delay Off | F31 |
| `delay5SecondsToolStripMenuItem` | Delay 5 Seconds | F32 |
| `delay10SecondsToolStripMenuItem` | Delay 10 Seconds | F33 |
| `delay20SecondsToolStripMenuItem` | Delay 20 Seconds | F34 |
| `delay30SecondsToolStripMenuItem` | Delay 30 Seconds | F35 |
| `delay60SecondsToolStripMenuItem` | Delay 60 Seconds | F36 |
| `exportToolStripMenuItem` | Export | F37 |
| `toolStripMenuItem_Close` | Close | F38 |
| `addEffectToolStripMenuItem` | Add Effect | F39 |
| `toolStripMenuItem_Cut` | Cut | F40 |
| `toolStripMenuItem_Copy` | Copy | F41 |
| `toolStripMenuItem_Paste` | Paste | F42 |
| `toolStripMenuItem_PasteSpecial` | Paste Special | F43 |
| `toolStripMenuItem_PasteInvert` | Invert | F43 |
| `toolStripMenuItem_PasteToMarks` | Visible Marks | F44 |
| `undoToolStripMenuItem` | Undo | F45 |
| `redoToolStripMenuItem` | Redo | F46 |
| `selectAllElementsToolStripMenuItem` | Select All Elements | F47 |
| `toolStripMenuItem_deleteElements` | Delete | F48 |
| `toolStripMenuItem_SnapTo` | Snap To Marks / Effects | F49 |
| `toolStripMenuItem_ResizeIndicator` | Resize / Draw Indicator | F50 |
| `toolStripMenuItem_RIColor_Blue` | Blue | F51 |
| `toolStripMenuItem_RIColor_Yellow` | Yellow | F51 |
| `toolStripMenuItem_RIColor_Green` | Green | F51 |
| `toolStripMenuItem_RIColor_White` | White | F51 |
| `toolStripMenuItem_RIColor_Red` | Red | F51 |
| `cADStyleSelectionBoxToolStripMenuItem` | CAD Style Selection Box | F52 |
| `legacyCursorActiveRowToolStripMenuItem` | Legacy Cursor / Active Row | F53 |
| `toolStripMenuItem_zoomTimeIn` | Zoom Time In | F54 |
| `toolStripMenuItem_zoomTimeOut` | Zoom Time Out | F55 |
| `zoomUnderMousePositionToolStripMenuItem` | Zoom Under Mouse Position | F56 |
| `toolStripMenuItem_zoomRowsIn` | Zoom Rows In | F57 |
| `toolStripMenuItem_zoomRowsOut` | Zoom Rows Out | F58 |
| `toolStripMenuItem2` | Collapse All Element Groups | F59 |
| `toolStripMenuItem3` | Reset Row Height to Default | F60 |
| `highlightRowsWithEffectsToolStripMenuItem` | Highlight Rows With Effects | F61 |
| `showEffectInfoToolStripMenuItem` | Show Effect Info | F62 |
| `showHiddenToolStripMenuItem` | Show Hidden Rows | F63 |
| `fullWaveformToolStripMenuItem` | Full Waveform | F64 |
| `lockRulerHeightToolStripMenuItem` | Lock Ruler Height | F65 |
| `lockWaveformHeightToolStripMenuItem` | Lock Waveform Height | F66 |
| `toolbarToolStripMenuItem` | Toolbars | F67 |
| `effectWindowToolStripMenuItem` | Effect Window | F68 |
| `markWindowToolStripMenuItem` | Mark Window | F69 |
| `gridWindowToolStripMenuItem` | Timeline Window | F76 |
| `effectEditorWindowToolStripMenuItem` | Effect Editor Window | F70 |
| `mixingFilterEditorWindowToolStripMenuItem` | Layer Editor Window | F71 |
| `toolStripMenuItemFindEffects` | Find Effects or Layers | F72 |
| `toolStripMenuItemColorLibrary` | Color Library Window | F73 |
| `toolStripMenuItemGradientLibrary` | Gradient Library Window | F74 |
| `toolStripMenuItemCurveLibrary` | Curve Library Window | F75 |
| `toolStripMenuItem_associateAudio` | Associate Audio... | F78 |
| `toolStripMenuItem_removeAudio` | Remove Audio | F79 |
| `beatBarDetectionToolStripMenuItem` | Beat/Bar Detection | F80 |
| `divideMarksEvenlyToolStripMenuItem` | Create Evenly Divided Marks | F82 |
| `speedTempoToolStripMenuItem` | Speed / Tempo | F81 |
| `modifySequenceLengthToolStripMenuItem` | Sequence Length... | F83 |
| `curveEditorToolStripMenuItem` | Curve Editor | F84 |
| `colorGradientEditorToolStripMenuItem` | Color Gradient Editor | F85 |
| `ColorCollectionsToolStripMenuItem` | Color Collection Editor | F86 |
| `lipSyncMappingsToolStripMenuItem` | LipSync | F87 |
| `defaultMapToolStripMenuItem` | Default Image Map | F87 |
| `phonemeMappingsToolStripMenuItem` | Edit Image Maps | F88 |
| `changeMapToolStripMenuItem` | Edit Element Face Mapping | F89 |
| `lyricConverterToolStripMenuItem` | Lyric Converter | F90 |
| `papagayoImportToolStripMenuItem` | Papagayo Import | F91 |
| `bulkEffectMoveToolStripMenuItem` | Bulk Effects Move | F92 |
| `effectDefaultsToolStripMenuItem` | Effect Defaults | F93 |
| `exportEffectDefaultsToolStripMenuItem` | Export... | F93 |
| `importEffectDefaultsToolStripMenuItem` | Import... | F94 |
| `dumpEffectDefaultsToolStripMenuItem` | Dump (Readable XML)... | F95 |
| `onlineHelpToolStripMenuItem` | Online Help | F96 |
| `vixenYouTubeChannelToolStripMenuItem` | Vixen YouTube Channel | F97 |
| `timerPlaying` |  | F267 |
| `statusStrip` | statusStrip1 | F256 |
| `toolStripStatusLabel2` | Current Position: | F257 |
| `toolStripStatusLabel_currentTime` | 0:00.000 | F257 |
| `toolStripStatusLabel1` | Sequence Length: | F258 |
| `toolStripStatusLabel_sequenceLength` | 0:00.000 | F258 |
| `toolStripStatusLabel3` | Play Delayed: | F259 |
| `toolStripStatusLabel_delayPlay` | 00 Seconds | F259 |
| `toolStripStatusLabel4` |  | F260 |
| `toolStripStatusLabel_RenderingElements` | Rendering Elements: | F261 |
| `toolStripProgressBar_RenderingElements` |  | F261 |
| `openFileDialog` |  | F302 |
| `toolStripContainer` | toolStripContainer1 | F98 |
| `dockPanel` |  | F2 |
| `toolStripAlignment` | Alignment | F99 |
| `alignmentToolStripButton_CloseGaps` | Close Gaps | F139 |
| `alignmentToolStripDropDownButton_CloseGaps` | Close Gaps | F140 |
| `alignmentToolStripMenuItemCloseGap100` | .100 | F140 |
| `alignmentToolStripMenuItemCloseGap200` | .200 | F140 |
| `alignmentToolStripMenuItemCloseGap300` | .300 | F140 |
| `alignmentToolStripMenuItemCloseGap400` | .400 | F140 |
| `alignmentToolStripDropDownButton_AlignTo` | Align To | F141 |
| `alignmentToolStripMenuItem_AlignTo200` | .200 | F141 |
| `alignmentToolStripMenuItem_AlignTo400` | .400 | F141 |
| `alignmentToolStripMenuItem_AlignTo600` | .600 | F141 |
| `alignmentToolStripMenuItem_AlignTo800` | .800 | F141 |
| `alignmentToolStripMenuItem_AlignTo1000` | 1.000 | F141 |
| `alignmentToolStripMenuItem_AlignTo2000` | 2.000 | F141 |
| `alignmentToolStripButton_Start` | Start Mark | F142 |
| `alignmentToolStripButton_End` | End Mark | F143 |
| `alignmentToolStripButton_Both` | Both Marks | F144 |
| `alignmentToolStripButton_Distribute` | Distribute Mark | F145 |
| `toolStripEffects` | Effects | F99 |
| `contextMenuStripEffect` |  | F151 |
| `effectGroupsToolStripMenuItem` | Show/Hide Effects | F152 |
| `basicToolStripMenuItem` | Basic | F153 |
| `pixelToolStripMenuItem` | Pixel | F154 |
| `deviceToolStripMenuItem` | Device | F155 |
| `intelligentFixtureToolStripMenuItem` | Fixture | F156 |
| `toolStripMenuItemLabelPosition` | Label Postion | F157 |
| `noLabelToolStripMenuItem` | No Label | F158 |
| `aboveImageToolStripMenuItem` | Above Image | F159 |
| `belowImageToolStripMenuItem` | Below Image | F160 |
| `toolbarsToolStripMenuItem_Effect` | Toolbars | F161 |
| `resetEffectsToolStripMenuItem` | Reset Toolbar | F162 |
| `toolStripView` | View | F99 |
| `viewToolStripButton_ZoomTimeIn` | Zoom Time In | F124 |
| `viewToolStripButton_ZoomTimeOut` | Zoom Time Out | F125 |
| `viewToolStripButton_ZoomRowIn` | Zoom Row In | F126 |
| `viewToolStripButton_ZoomRowOut` | Zoom Row Out | F127 |
| `toolStripFile` | File | F99 |
| `fileToolStripButton_Save` | Save | F134 |
| `fileToolStripButton_SaveAs` | Save As | F135 |
| `fileToolStripButton_AutoSave` | Auto Save | F136 |
| `fileToolStripButton_Export` | Export | F137 |
| `fileToolStripButton_Close` | Close | F138 |
| `toolStripEdit` | Edit | F99 |
| `editToolStripButton_Undo` | Undo | F110 |
| `editToolStripButton_Redo` | Redo | F111 |
| `editToolStripButton_Cut` | Cut | F112 |
| `editToolStripButton_Copy` | Copy | F113 |
| `editToolStripButton_Paste` | Paste | F114 |
| `editToolStripButton_PasteDropDown` |  | F115 |
| `editToolStripButton_PasteInvert` | Paste - Invert | F116 |
| `editToolStripButton_PasteVisibleMarks` | Paste - Visible Mark/s | F117 |
| `toolStripMode` | Mode | F99 |
| `modeToolStripButton_DrawMode` | Draw Mode | F118 |
| `modeToolStripButton_SelectionMode` | Selection Mode | F119 |
| `modeToolStripButton_SnapTo` | Snap To | F120 |
| `modeToolStripDropDownButton_SnapToStrength` | Snap To | F121 |
| `modeToolStripMenuItem_SnapStrength_1` | 1 | F121 |
| `modeToolStripMenuItem_SnapStrength_2` | 2 | F121 |
| `modeToolStripMenuItem_SnapStrength_3` | 3 | F121 |
| `modeToolStripMenuItem_SnapStrength_4` | 4 | F121 |
| `modeToolStripButton_DragBoxFilter` | Drag Box Filter | F122 |
| `modeToolStripDropDownButton_DragBoxFilter` | Drag Box Filter | F123 |
| `toolStripTools` | Tools | F99 |
| `toolsToolStripButton_LipSync` | LipSync Edit Map | F146 |
| `toolsToolStripButton_CurveLibrary` | Curve Library | F147 |
| `toolsToolStripButton_ColorLibrary` | Color Library | F148 |
| `toolsToolStripButton_ColorGradient` | Color Gradient | F149 |
| `toolStripPlayBack` | PlayBack | F99 |
| `playBackToolStripButton_Start` | Start | F105 |
| `playBackToolStripButton_Play` | Play | F106 |
| `playBackToolStripButton_Stop` | Stop | F107 |
| `playBackToolStripButton_End` | End | F108 |
| `playBackToolStripButton_Loop` | Loop | F109 |
| `toolStripColorLibrary` | Library Color | F164 |
| `contextMenuStripLibraries` | Libraries | F171 |
| `newToolStripMenuItemLibraries` | New | F172 |
| `editToolStripMenuItemLibraries` | Edit | F173 |
| `deleteToolStripMenuItemLibraries` | Delete | F174 |
| `importToolStripMenuItemLibraries` | Import | F175 |
| `exportToolStripMenuItemLibraries` | Export | F176 |
| `add_RemoveLibraryToolStripMenuItem` | Show/Hide Items | F177 |
| `toolBarsToolStripMenuItemLibraries` | Toolbars | F178 |
| `imageSizeToolStripMenuItemLibraries` | Toggle Image Size | F179 |
| `resetLibraryToolStripMenuItem` | Reset Toolbar | F180 |
| `toolStripCurveLibrary` | Library Curve | F164 |
| `toolStripGradientLibrary` | Library Gradient | F164 |
| `saveFileDialog` |  | F13 |
| `contextMenuStripElementSelection` |  | F255 |
| `timerPostponePlay` |  | F270 |
| `timerDelayCountdown` |  | F270 |

## Data and persistence

Three separate stores are involved, and which one a setting lands in decides what travels with a show.

**1. The sequence file (`.tim`, read and written by `vixen_io::load_sequence` and `save_sequence`).** Travels with the show. Written only by Save (F13).

| Field (`TimedSequenceData`) | Type, default | Set when | Read when |
|---|---|---|---|
| `TimePerPixel` | `TimeSpan`, `MinValue` | Each zoom, on the object (`Form.cs:2368-2375`); saved on next Save | Load, if above 0 (`Form.cs:1515-1518`) |
| `VisibleTimeStart` | `TimeSpan`, `MinValue` | Each horizontal scroll (`Form.cs:2377-2384`) | Load, if above 0 (`Form.cs:1520-1523`) |
| `DefaultRowHeight` | int, 0 | Save (`Form.cs:1616`) | Load; 0 means timeline default (`Form.cs:1325-1332`) |
| `RowSettings` | map of `TreeId` to `{Expanded, RowHeight, Visible}` | Rebuilt on Save with the 7 pt rule (`Form.cs:1622-1635`) | Load (`Form.cs:1364-1378`) |
| `DefaultPlaybackStartTime`, `DefaultPlaybackEndTime` | `TimeSpan?`, 0 | Save (`Form.cs:1619-1620`) | Load, unless end is exactly 0. A nil end counts as set, so a saved start with no end is restored (`Form.cs:563-567`) |
| `DefaultSplitterDistance` | int, 0 | Never by this window | Never: the splitter is a user setting |
| `LabeledMarkCollections`, `SequenceLayers`, effects, media, selected timing provider | see `vixen-io` | Live, saved on Save | Load |

Defaults: `Vixen.Modules/Sequence/Timed/TimedSequenceData.cs:45-51`. `vixen_io` already has all of these (`crates/vixen-io/src/sequence.rs:129-157`). Note `new_sequence` matches `MinValue` defaults, hence the "above 0" guards.

Saved in the file but not undoable: zoom and scroll, row layout, playback range. Zoom, scroll and row height changes set the editor state flag so the sequence saves silently at close (F16). A plain ruler click marks the sequence modified (F273).

**2. User settings (`%AppData%/Vixen/Settings.xml`, `Vixen.Common/Controls/XMLProfileSettings.cs`).** Per user, across all sequences, and across profiles. XML `<settings><AppSettings>...</AppSettings><Preferences>...</Preferences></settings>` with slash-separated element paths. Missing or unreadable file: empty document. Values are text via `Convert` (`:11-25`, `:81-114`), so a value that does not parse throws at load (GUARD: use the default and log it). **Every put saves the entire file** (`:69-79`). Read once at window load. Written at window close (`Form.cs:5808-5838`, `Toolstrip.cs:2003-2083`), so a crash keeps the previous values. With several windows open the last to close wins. Mac location: `~/Library/Application Support/Vixen/Settings.xml`, overridable for tests (see Rust API). Same XML, so existing Windows values can be copied in.

Keys, all under `AppSettings/TimedSequenceEditorForm/` unless noted:

| Key | Type | Default | Used by |
|---|---|---|---|
| `DockLeftPortion`, `DockRightPortion` | int (`(int)` of a double) | 150, 150 | Panel widths (F2) |
| `AutoSaveEnabled` | bool | true | F26 |
| `SnapToSelected` | bool | true | F49 |
| `FullWaveform` | bool | false | F64 |
| `SnapStrength` | int | 2 | F121 |
| `CloseGapThreshold` | string | ".100" | F140 |
| `AlignToThreshold` | string | ".800" | F141 |
| `ResizeIndicatorEnabled` | bool | false | F50 |
| `ResizeIndicatorColor` | string | "Red" | F50 |
| `DrawModeSelected` | bool | false | F118 |
| `SelectionModeSelected` | bool | true | F119 |
| `CurveLinkCurves`, `GradientLinkGradients` | bool | false | F351, F363 |
| `CadStyleSelectionBox` | bool | false | F52 |
| `LegacyCursorActiveRow` | bool | false | F53 |
| `ZoomUnderMousePosition` | bool | false | F56 |
| `HighlightActiveElements` | bool | false | F61 |
| `WaveFormHeight`, `RulerHeight` | int | 50, 50 | Timeline |
| `LockRulerHeight`, `LockWaveformHeight` | bool | false | F65 |
| `ShowEffectInfo` | bool | true | F62 |
| `UseTempoForSpeed` | bool | false | F81 |
| `SplitterDistance` | int | 200 | Timeline |
| `WindowWidth`, `WindowHeight`, `WindowLocationX`, `WindowLocationY`, `WindowState` | int, string | current, "Normal" | F3 |
| `ToolStrip/ToolStripCount` | int | 0 | F98 |
| `ToolStripName<i>`, `ToolStripVisible<i>`, `ToolStripX<i>`, `ToolStripY<i>` | string, bool, int, int | "", false, 0, 0 | F98 |
| `ToolStrip/ColorLibrary`, `ToolStrip/GradientLibrary`, `ToolStrip/CurveLibrary` | int (scale) | 16 | F103 |
| `ToolStrip/PlayBack`, `Alignment`, `Audio`, `Edit`, `Mode`, `View`, `File`, `Tools` | int (scale) | 28 | F103 |
| `EffectToolStrip/EffectLabelIndex` | string | "2" | F157 |
| `EffectToolStrip/BasicEffectToolStrip`, `PixelEffectToolStrip`, `DeviceEffectToolStrip` | bool | true | F152 |
| `EffectToolStrip/FixtureEffectToolStrip` (read) and `IntelligentFixtureEffectToolStrip` (written) | bool | true | F163 |
| `<GroupText>/<EffectNameNoSpaces>`, `<GroupText>/<EffectNameNoSpaces>1` | bool, int | written | Written only (`Toolstrip.cs:2040-2059`); nothing reads them. Do not port |

`ToolStrip/<Strip>` quirk: at load it does not store a size to apply, it compares. `ChangeToolStripImageSize(strip, saved)`: saved differs from the current 16 means large (28), otherwise small (16) (`Toolstrip.cs:93-114`, `:923-940`). Rust: store large or small.

Other forms: `Form_ColorLibrary/ColorLibraryScale`; `Form_CurveLibrary/CurveLibraryImageScale`, `CurveLibraryTextScale`; `Form_GradientLibrary/GradientLibraryImageScale`, `GradientLibraryTextScale` (invariant culture strings, default "1"). `ExportDialog/ExportFormat` (index into sorted list, 0), `ExportResolution` (index, 1 meaning 50 ms), `GenerateUniverse` (false), `Universe2x` (true), `Compress` (true), each written at once on change.

Preferences: `Preferences/TimedSequenceEditorForm/AutoSaveInterval`, int ms, default 300000, read at load (`Form.cs:350`). **Nothing in the source tree writes it** (the only other match is the commented-out line at `Form.cs:5847-5848`), so it is 5 minutes unless hand edited. Keep reading the key, and do not add UI without the owner asking.

Close-time cleanup removes `StandardNudge`, `SuperNudge`, `NudgeSettings` and the top-level `TimedSequenceEditorForm` node left by old versions (`Form.cs:5850-5854`).

**3. Other per-user files in `%AppData%/Vixen/`.**

| File | Format | Contents | Read, written |
|---|---|---|---|
| `TimedSequenceEditorForm.xml` | WeifenLuo dock layout | Panel layout | Load; close (`Form.cs:296-319`, `:5799`). Windows-only format, omitted, replaced by a Mac layout file |
| `ToolStripState.xml` | `DataContractSerializer` of `ToolStripStates { States: map<strip name, map<item name, visible>> }` | Item visibility, all strips except the three libraries | Load; close (`Toolstrip.cs:1961-2123`) |
| `<ProfileName>_ToolStripState.xml` | same | Library strips' item visibility, per profile because libraries differ | Load only when the standard file exists; close |
| `ColorPalette.xml` | `DataContractSerializer` of `List<Color>` | Colour favourites, global | Load; every change (F341) |
| `ColorCollections.xml` | `DataContractSerializer` of `List<ColorCollection> {Name, Description, Color[]}` | Colour collections, global | Load (no error handling: GUARD); OK in the editor (`Form.cs:1638-1662`) |
| `AudioDevice.settings` | `AudioPreferences` | Output device id and other audio prefs | Manager start; on change (`Vixen.Common/AudioPlayer/AudioPreferences.cs:10`) |

**4. Profile data (`vixen_io`).** Curve and gradient libraries are in the module store and saved with the system configuration. Lip sync maps and face mapping call `SaveSystemAndModuleConfigAsync` (`Menu.cs:477`, `:603`). Effect defaults live in `<Vixen 3>/Effect Defaults/` (`Vixen.Core/Services/EffectDefaults/EffectDefaultsService.cs:35`; `DataPaths::effect_defaults_dir`). Export output defaults to `<Vixen 3>/Export/` (`DataPaths::export_dir`).

**5. Clipboard.** Format name `VixenModules.Editor.TimedSequenceEditor.TimelineElementsClipboardData`, JSON via `SetDataAsJson`: `{ EffectModelCandidates: [{ EffectModel, RowOffset }], FirstVisibleRow, EarliestStartTime }`. `EffectModel` holds `_moduleDataClass` (assembly qualified name), `_effectData` (`DataContractSerializer` output through the **binary** XML writer), `TypeId`, `StartTime`, `Duration`, `LayerId`, `LayerTypeId`, `LayerName` (`Form.cs:6381-6395`, `TSE/EffectModelCandidate.cs`). Mac: custom pasteboard type `org.vixenlights.timeline-elements`, JSON from Rust carrying the effect's parameters in `vixen-io`'s XML text form. Same fields. It needn't interoperate with Windows Vixen, since the formats differ by platform anyway. Clipboard survives the window and is shared between windows, as upstream.

**6. Session only, never persisted.** Loop (F30), delay option (F31), timing speed (F130), Show Hidden Rows (F63), drag box filter (F123), Add Multiple last values (F291, process wide), undo stacks, last folder of each library import or export, last colour in the collection editor, Export's controller order and checks (F397) and any typed Timing value (F402).

## Rust API needed

Everything below is **new** unless it says otherwise. It sits on the timeline spec's `vixen-session::timeline` API (`docs/spec/timeline.md`, "Rust API needed"), and reuses its `ElementId`, `RowId`, `Mods`, `ElementTimes`, `ElementMoveType`, `load_rows`, `save_row_settings`, `align`, `ok_to_use_alignment`, `split_at`, `close_gaps`, `select_*`, `queue_render` and `render_progress`. New window state goes in a new module `vixen-session::sequencer`. The FFI side is `#[uniffi::export] impl SequencerSession` blocks in `crates/vixen-ffi/src/sequencer.rs` (currently empty), extending the object that already exists in `crates/vixen-ffi/src/sequence.rs`, not a second object (`docs/UI_GUIDE.md`). Pure functions are free functions so tests need no session. Times are `vixen_core::TimeSpan`.

**Already there, so build on it and do not add it again:**

- `vixen_session::sequence::SequenceSession`: `open`, `new_untitled`, `save`, `is_modified`, `set_modified`, `length`, `set_length`, `add_new_effect`, `add_copy`, `remove_effects`, `set_times`, `set_target`, `set_param`, `set_params`, `set_layer`, `marks`, `set_marks`, `render_all_dirty`, `apply_finished_renders`, `take_problems`, `record_undo`, `undo`, `redo`. Its edit methods record nothing: call `record_undo` once per user action.
- `vixen_session::undo`: `UndoStack`, `UndoAction`, `EffectSnapshot`, `AddRemoveKind`, `MoveKind`.
- `vixen_session::playback::ShowPlayback`, reached as `AppCore.playback`: play, pause, resume, stop, seek, position, transport, take_finished, set_speed, execution open and close. One sequence plays at a time.
- `vixen_session::export`: `ExportRequest`, `PreparedExport`, `default_controller_info`.
- `SequencerSession` in `crates/vixen-ffi/src/sequence.rs`: `VixenCore::open_sequence` and `new_sequence`, `save`, `render_status`, `play`, `pause`, `resume`, `stop`, `seek`, `position_ms`, `transport`, `take_finished`, `set_speed`, `undo`, `redo`, `undo_descriptions`, `redo_descriptions`.

Where a signature below overlaps these, it is what this area adds on top.

**Lifetime and saving: `vixen-session::sequencer`**

- `struct SequencerState`: the new per-window state, holding the `Arc<Mutex<SequenceSession>>` that `SequencerSession` already has, plus rows, elements, transport, the editor state flag and clipboard mode. The methods below are on it unless they are free functions.
- `fn open(core: Arc<AppCore>, path: &Path) -> Result<SequencerState, SessionError>`: `SequenceSession::open`, then apply F8's rules and build rows and elements on a worker.
- `fn new_untitled(core: Arc<AppCore>) -> SequencerState`: `SequenceSession::new_untitled`, length 1 minute, modified (F6).
- `fn load_progress(&self) -> LoadProgress`: phase, elapsed, finished, for the status text.
- `fn load_report(&self) -> LoadReport`: dropped past end, shortened, dropped without row, `renames: Vec<(String, String)>`, `deprecated: Vec<(String, u32)>` (F8, F10, F11).
- `fn unmapped_targets(&self) -> Vec<(Uuid, String)>`, `fn is_foreign_location(&self) -> bool`, `fn apply_element_map(&mut self, map: Option<&[(Uuid, Uuid)]>)`: F12; `None` removes all unmapped.
- `fn window_title(&self, profile: &str) -> String`: F4.
- `fn is_editor_state_modified(&self) -> bool`: F16; `is_modified` exists.
- `fn save_with_row_settings(&mut self, to: Option<&Path>, range: PlaybackRange) -> Result<(), SessionError>`: F14, then `SequenceSession::save`.
- `fn save_path_with_extension(chosen: &Path, extension: &str) -> PathBuf`: F13's append rule.
- `fn close_action(modified: bool, state_modified: bool, app_closing: bool) -> CloseAction`: `Prompt { can_cancel }`, `SaveSilently` or `Close` (F16). If the owner keeps the F16 FIX, a cancelled save panel returns to the prompt's caller as "stay open".
- `fn finish_close(&mut self)`: F18.
- `struct AutoSaveClock`, `fn on_modified_changed(&mut self, modified: bool, enabled: bool, now: Instant)`, `fn set_enabled(&mut self, on: bool, modified: bool, now: Instant)`, `fn due(&mut self, now: Instant) -> bool`: F15, testable with a fake clock.
- `fn command_for_key(key: Key, mods: Mods) -> Option<SequencerCommand>`: the whole hotkey table (section D).
- `fn run_command(&mut self, command: SequencerCommand, pointer: Option<GridPoint>) -> CommandOutcome`: alerts come back as data for Swift to show.
- `fn menu_state(&self) -> MenuState`: enable and check state of every item, read in one call when a menu opens or the selection changes.
- `fn context_menu(&self, row: Option<RowId>, element: Option<ElementId>, time: TimeSpan, under_pointer: &[ElementId]) -> ContextMenuModel`: F226 to F254 with all enable rules and tooltips.
- `fn element_chooser(under_pointer: &[ElementId]) -> Vec<(ElementId, String)>`: F255's labels.

**Transport: `vixen-session::transport` (extends the existing file)**

- `struct TransportController`: one per window. Drives `AppCore.playback` (`ShowPlayback`, which owns the engine) and holds speed, loop, delay and the remembered range `_mPrevPlaybackStart`, `_mPrevPlaybackEnd` (F423).
- `fn play_pause_toggle(&mut self)`, `fn stop(&mut self)`, `fn play_from(&mut self, t: TimeSpan)`, `fn play_between(&mut self, s: TimeSpan, e: TimeSpan)`: F263 to F266.
- `fn set_loop(&mut self, on: bool)`, `fn set_delay(&mut self, d: DelayOption)`: the mutual exclusion in F30, F31.
- `fn tick(&mut self, now: Instant) -> TransportEvents`: called from the display link only while active. Delay countdown, finished detection (`Engine::take_finished`), range restore (F267). Never polls faster than the display.
- `fn transport_ui(&self) -> TransportUi`: Play enabled, Stop enabled, icon kind, title, tooltip, delay text (F268, F264).
- `fn ruler_clicked(&mut self, t: TimeSpan, mods: Mods) -> RulerAction`, `fn ruler_range_begin(&mut self)`, `fn ruler_range_end(&mut self, mods: Mods)`: F273, F274.
- `fn goto_start(&mut self)`, `fn goto_end(&mut self)`: F105, F108.
- `fn coerce_range(start: TimeSpan, end: TimeSpan, length: TimeSpan) -> (TimeSpan, TimeSpan)`: F276.
- `fn is_natural_end(position: TimeSpan, start: TimeSpan, end: TimeSpan, advanced: &mut bool) -> bool`: F277.
- `fn set_speed(&mut self, s: f32) -> SpeedState`, `fn speed_wheel(current: f32, up: bool) -> f32`: F271, F130, with its early-return quirk: `f32` arithmetic, `s as f64 <= 0.10`, `s > 4.0f32`. Forwards to `ShowPlayback::set_speed` only on the non-early path.
- `fn speed_gauge(speed: f32) -> SpeedGauge`: text, text x, hue byte, needle degrees (F133).
- `fn speed_controls_enabled(&self) -> bool`: F272.
- `fn audio_output_devices(&self) -> Vec<AudioDeviceItem>`, `fn set_audio_output_device(&mut self, id: &str)`: F132, F283, wrapping `vixen_audio::output_devices`, which today returns names only.

**Engine and audio gaps to raise with their owners (do not change them from this area):**

- `vixen_engine::Engine` ends at `time > range_end` (`crates/vixen-engine/src/playback.rs`, `frame_thread`). Upstream ends at `position >= end || position == 0`, after position first exceeds start (F277). It also seeks in place on loop, where upstream does stop, seek, start and raises restarted (F278). Clamp to length in the caller (F276).
- `vixen_audio::AudioPlayer::set_speed` clamps to 0.1..=2.0 and always preserves pitch (`crates/vixen-audio/src/player.rs:339-343`). Upstream allows up to 4.0 (F271) and, with Speed / Tempo off (the default), changes **rate**, so pitch moves with speed; only with it on does it keep pitch (`Vixen.Common/AudioPlayer/CoreAudioPlayer.cs:263-280`). Needed: `fn set_rate_mode(&self, preserve_pitch: bool)` and a 4.0 upper limit.

**Audio association: `vixen-session::sequencer`**

- `fn add_audio(&mut self, path: &Path) -> AddAudioOutcome`: `Unsupported`, `Unprocessable`, `Added { resize_prompt: bool }`, with the silent length rule (F302).
- `fn remove_audio(&mut self)`, `fn resize_to_audio(&mut self)`, `fn set_length(&mut self, len: TimeSpan)`: F303, F302, F83.
- `fn missing_audio(&self) -> Option<PathBuf>`: F9.
- `fn supported_audio_extensions() -> &'static [&'static str]`: F283's list.
- `fn set_use_tempo(&mut self, on: bool)`: F81.
- `fn detect_beats_and_bars(&mut self, settings: BeatBarSettings) -> usize`: F80, on `vixen-analysis`; its windows are that area's.

**Effects: `vixen-session::sequencer`**

- `fn default_effect_duration(visible_end: TimeSpan, start: TimeSpan) -> TimeSpan`: F286.
- `fn fit_new_effect(start: TimeSpan, duration: TimeSpan, length: TimeSpan) -> (TimeSpan, TimeSpan)`: F285.
- `fn add_effect(&mut self, type_id: Uuid, row: RowId, start: TimeSpan, duration: TimeSpan, select: bool) -> Option<ElementId>`: F284, F226.
- `fn draw_effects(&mut self, type_id: Uuid, rows: &[RowId], start: TimeSpan, duration: TimeSpan)`: F287.
- `fn delete_selected(&mut self)`: F290.
- `fn clone_elements(&mut self, elements: &[ElementId]) -> Vec<ElementId>`, `fn clone_to_selected(&mut self, source: ElementId) -> CloneOutcome`: F241.
- `fn set_elements_time(&mut self, elements: &[ElementId], start: TimeSpan, duration: TimeSpan)`: F245.
- `fn replace_selected(&mut self, type_id: Uuid) -> usize`: F305.
- `fn assign_layer(&mut self, elements: &[ElementId], layer: Uuid)`, `fn layer_menu(&self, elements: &[ElementId]) -> LayerMenu`: F227.
- `fn possible_effects(start: TimeSpan, end: TimeSpan, duration: TimeSpan, between: TimeSpan) -> i32`: F384's strict loop.
- `fn possible_effects_on_marks(times: &[TimeSpan], fill: bool, skip_every_other: bool) -> i32`: F384.
- `fn time_exists_for_addition(start: TimeSpan, end: TimeSpan, duration: TimeSpan, between: TimeSpan, count: i32) -> bool`: F384.
- `fn plan_add_multiple(p: &AddMultipleParams, length: TimeSpan) -> Vec<(TimeSpan, TimeSpan)>`, `fn plan_add_on_marks(p: &AddOnMarksParams, marks: &[MarkTimes]) -> Vec<(TimeSpan, TimeSpan)>`: F291, F292.
- `struct AddMultipleMemory`: process-wide last values, `fn dialog_initial(shift: bool, ...) -> AddMultipleParams`: F291.
- `fn distribute_equally(selected: &[ElementTimes]) -> Result<Vec<(TimeSpan, TimeSpan)>, DistributeError>`: F293, order preserved so a golden test can compare, with the `.050` limit.
- `fn distribute(selected: &[ElementTimes], p: &DistributeParams) -> Vec<(TimeSpan, TimeSpan)>`: F294, in double seconds as upstream.
- `fn find_nearest_mark(t: TimeSpan, threshold_secs: f64, collections: &MarkIndex) -> TimeSpan`, `fn align_to_marks(kind: MarkAlign, elements: &[ElementTimes], threshold_secs: f64, collections: &MarkIndex) -> Vec<Option<(TimeSpan, TimeSpan)>>`: F295.
- `fn apply_color_collection(&mut self, collection: &ColorCollection, random: bool, rng: &mut Rand) -> ApplyColorsOutcome`: F253.

**Marks host: `vixen-session::sequencer`** (mark storage is in `vixen-core`)

- `fn add_mark_at(&mut self, t: TimeSpan, opts: AddMarkOptions) -> AddMarkOutcome`: `NeedsCollection`, `NeedsEnable`, `Added`, `AlreadyThere` (F297).
- `fn add_marks_to_effects(&mut self, elements: Option<&[ElementId]>) -> usize`: F296.
- `fn plan_even_marks(start: TimeSpan, end: TimeSpan, divisions: i32, min_ms: f64, existing: &[&str]) -> Result<EvenMarksPlan, EvenMarksError>`: F82.
- `fn phoneme_breakdown(&mut self, kind: BreakdownKind, marks: &[MarkId]) -> BreakdownOutcome`: F299, with the failed-word callback result as data.

**Drag and drop: `vixen-session::sequencer`**

- `enum DropPayload { EffectType(Uuid), Color(Color), Gradient(ColorGradient), Curve(Curve), Files(Vec<PathBuf>) }`.
- `fn drop_validity(&mut self, kind: PayloadKind, row: Option<RowId>, element: Option<ElementId>) -> bool`: F306, cached per hovered effect.
- `fn plan_drop(&self, element: ElementId, payload: &DropPayload) -> DropPlan`: `Direct`, `Picker(Vec<PickerItem>)` with thumbnails, or `WarnMixedTypes(String)` (F307 to F309).
- `fn apply_drop(&mut self, element: ElementId, payload: &DropPayload, choice: Option<ParamPath>) -> DropResult`: status message, undo.
- `struct ParamPath { name: String, collection_index: Option<i32>, sub_index: Option<i32> }`.
- `fn param_paths(table: &[ParamDesc], values: &Params, kinds: &[ParamClass]) -> Vec<ParamPath>`: F313, using `visibility` for browsable. Belongs in `vixen-effects`, or as a free function here since that crate must not change.
- `fn resolve_path(path: &ParamPath, values: &Params) -> Option<ParamPath>`: `ToNewOwner`, F313.
- `fn plan_file_drop(files: &[PathBuf], mods: Mods, effects: &[&EffectDescriptor]) -> Vec<FileDropItem>`: F311, including its case quirk; `effects` comes from `vixen_effect_registry::all()`.
- `fn library_drag_payload(value: LibraryValue, name: &str, link: bool) -> Result<DropPayload, PayloadError>`: F314.

**Clipboard: `vixen-session::sequencer`**

- `struct ClipboardData { candidates: Vec<ClipboardEffect>, first_visible_row: i32, earliest_start: TimeSpan }`: serde JSON, F315.
- `fn copy_selected(&self) -> Option<ClipboardData>`, `fn cut_selected(&mut self) -> Option<ClipboardData>`: F317.
- `fn paste(&mut self, data: &ClipboardData, mode: PastingMode, time: TimeSpan) -> PasteOutcome`: F318.
- `fn plan_paste(data: &ClipboardData, mode: PastingMode, time: TimeSpan, anchor: usize, rows: &[RowSummary], total: TimeSpan, marks: &MarkIndex) -> Vec<PasteItem>`: pure, for tests.
- `fn paste_menu_state(count: usize, marks_present: bool) -> PasteMenuState`: F316.
- `fn marks_present_after(cursor: TimeSpan, marks: &MarkIndex) -> bool`: F316.
- `fn clipboard_count(json: &[u8]) -> usize`: reads only the count, without decoding effects.

**Undo: `vixen-session::undo`**

- Already in `crates/vixen-session/src/undo.rs`, with the limit of 1000 and the F417 and F419 fixes applied: `UndoStack` (`push`, `can_undo`, `can_redo`, `undo_descriptions`, `redo_descriptions`) and `UndoAction` with `description()`. Section M maps onto it as F405 `EffectsAdded { AddedOrRemoved }`, F406 `EffectsAdded { Pasted }`, F407 `EffectsRemoved { Cut }`, F408 `EffectsRemoved { AddedOrRemoved }`, F409 `TimesChanged`, F410 `LayersChanged`, F412 `ParamsChanged` (F411 is never raised), F413 to F416 `MarksChanged`, whose `label` must carry the exact wording, including `Moved {n} effect{s}` for a mark move.
- This area adds no stack code: each command calls `record_undo` with the right variant. Effects stay in memory when removed (`EffectSnapshot`), as F18 requires. F18's cleanup at close still has to be written in `finish_close`.

**Find: `vixen-session::sequencer`**

- `fn find_candidates(&self, mode: FindMode) -> Vec<String>`, `fn find_results(&self, mode: FindMode, name: &str) -> Vec<FindRow>`, `fn reveal(&mut self, elements: &[ElementId], collapse_first: bool)`: F321, fetched in bulk off the main thread. `fn find_generation(&self) -> u64` lets Swift refresh only when something relevant changed.

**User settings and small files: `vixen-io::user_settings`, new module**

- `fn settings_dir() -> PathBuf`: `~/Library/Application Support/Vixen`; `VIXEN_SETTINGS_DIR` overrides, and the UI test harness sets it under `VIXEN_DATA`.
- `struct SettingsFile`, `fn load(path: &Path) -> SettingsFile`, `fn get<T>(&self, section: Section, path: &str, default: T) -> T`, `fn put<T>(&mut self, section: Section, path: &str, value: T)`, `fn save(&self) -> Result<(), IoError>`: same XML, but a single write per batch.
- `struct EditorPrefs` with one field per key table row, `fn read(&SettingsFile) -> EditorPrefs`, `fn write(&self, &mut SettingsFile)`.
- `struct ToolStripStates`, `fn load_tool_strip_states(dir: &Path, profile: &str) -> ToolStripStates`, `fn save_tool_strip_states(dir: &Path, profile: &str, s: &ToolStripStates) -> Result<(), IoError>`: F98, with the two-file split; entries are created on demand under the F177 FIX.
- `fn load_palette(dir: &Path) -> Vec<Color>`, `fn save_palette(dir: &Path, colors: &[Color]) -> Result<(), IoError>`, `fn read_vfc(path: &Path) -> Result<Vec<Color>, IoError>`, `fn write_vfc(path: &Path, colors: &[Color]) -> Result<(), IoError>`: F341.
- `fn load_color_collections(dir: &Path) -> Vec<ColorCollection>`, `fn save_color_collections(dir: &Path, c: &[ColorCollection]) -> Result<(), IoError>`, `fn read_vcc(path: &Path) -> Result<Vec<ColorCollection>, IoError>`, `fn write_vcc(path: &Path, c: &[ColorCollection]) -> Result<(), IoError>`: F364. A corrupt file loads as empty, with a logged warning.
- `fn read_vcl(path: &Path) -> Result<Vec<(String, Curve)>, IoError>`, `fn write_vcl(path: &Path, items: &[(String, Curve)]) -> Result<(), IoError>`, `fn read_vgl(path: &Path) -> Result<Vec<(String, ColorGradient)>, IoError>`, `fn write_vgl(path: &Path, items: &[(String, ColorGradient)]) -> Result<(), IoError>`: F351, F363.
- `fn import_name(existing: &dyn Fn(&str) -> bool, name: &str) -> String`: the ` 2`, ` 3` rule (F351).

**Library edits: `vixen-session::libraries`**

- `fn add_curve(&mut self, name: &str, c: Curve, overwrite: bool) -> AddLibraryOutcome`, same for gradients: F362.
- `fn remove_curves(&mut self, names: &[String])`, `fn remove_gradients(&mut self, names: &[String])`: bulk, one change event.
- `fn reorder_curves(&mut self, names: &[String])`, `fn reorder_gradients(&mut self, names: &[String])`: F169, one transaction.
- `fn library_thumbnail(kind: LibraryKind, name: &str, size: u32) -> Pixels`: cached by content and size.

**Layers: `vixen-session::layers`** (data in `vixen_core::layer`)

- `fn add_layer(layers: &mut SequenceLayers, filter: MixingFilter) -> Uuid`, `fn remove_layer(layers: &mut SequenceLayers, id: Uuid) -> bool`, `fn move_layer(layers: &mut SequenceLayers, id: Uuid, new_index: usize) -> bool`, `fn quick_rename(layers: &mut SequenceLayers, id: Uuid) -> bool`, `fn unique_layer_name(existing: &[String], desired: &str) -> String`, `fn has_exportable(layers: &SequenceLayers) -> bool`: F330.
- `fn rename_layer(layers: &mut SequenceLayers, id: Uuid, name: &str) -> bool`, `fn set_layer_filter(layers: &mut SequenceLayers, id: Uuid, filter: MixingFilter) -> bool`: F334's Name and Type fields. Whether they set modified is the F330 quirk. Configuration opens the filter's own setup, which belongs to the mixing filter modules; only `has_setup` is needed here.
- `fn export_v3l(layers: &SequenceLayers, path: &Path) -> Result<(), IoError>`, `fn read_v3l_plan(path: &Path) -> LayerImportPlan`, `fn import_plan(layers: &mut SequenceLayers, plan: &LayerImportPlan) -> usize`: F340.

**Effect defaults: `vixen-session::effect_defaults`** (port of `Vixen.Core/Services/EffectDefaults/*`, 804 lines, which is outside this spec's reading except the calls used here)

- `fn create_default_data(&self, type_id: Uuid) -> Option<Params>`, `fn summaries(&self) -> Vec<EffectDefaultSummary>`, `fn export(&self, path: &Path, types: &[Uuid]) -> Result<(), IoError>`, `fn import(&mut self, path: &Path, mode: ImportMode) -> Result<ImportResult, IoError>`, `fn write_dump(&self, path: &Path) -> Result<(), IoError>`: F285, F93, F403.

**Lip sync glue and Papagayo: `vixen-io::papagayo`, `vixen-session::sequencer`**

- `fn parse_papagayo(reader: impl BufRead) -> Result<PapagayoDoc, IoError>`, `fn phoneme_list(doc: &PapagayoDoc, voice: &str) -> &[PapagayoPhoneme]`: F404.
- `fn papagayo_to_clipboard(doc: &PapagayoDoc) -> ClipboardData`, `fn lipsync_to_clipboard(items: &[LipSyncConvertData]) -> ClipboardData`: F87, F301.
- `fn reset_lipsync_elements(&mut self)`: F301, F88.
- `fn lipsync_maps(&self) -> Vec<(String, bool)>`, `fn set_default_lipsync_map(&mut self, name: &str) -> bool`: F87.

**Export glue: `vixen-session::export`** (extends the existing file; `vixen_export::run::run_export` and `fpp_config` exist)

- `fn export_formats() -> Vec<String>`: sorted list as the dialog shows it.
- `fn can_compress(format: &str) -> bool`, `fn is_falcon_format(format: &str) -> bool`, `fn file_extension(format: &str) -> &'static str`: F396.
- `fn controller_rows(&self) -> Vec<ExportControllerRow>` (on `default_controller_info`, all active in system order each time, F397), `fn reindex_controllers(order: &[ControllerSelection]) -> Vec<ExportControllerRow>`: F398.
- `fn start_export(&self, settings: ExportSettings, on_progress: Arc<dyn ExportProgress>) -> ExportHandle`, `fn cancel(&self)`: off the main thread, with progress at most ten times a second.

**Toolbars: `vixen-session::toolbars`**

- `fn default_toolbar_layout() -> ToolbarLayout`: F98's default row and visibility.
- `fn restore_toolbar_layout(prefs: &EditorPrefs) -> ToolbarLayout`, `fn save_toolbar_layout(l: &ToolbarLayout, prefs: &mut EditorPrefs)`.
- `fn effect_button_visible(group_on: bool, effect_on: bool) -> bool`: F152.

## SwiftUI and AppKit structure

**Modules.** `app/Sources/VixenSequencer` (window, menus, toolbars, status bar, dialogs, tool windows), importing `VixenKit`, `VixenBridge`, and `VixenTimeline` (which `docs/spec/timeline.md` calls `TimelineUI`), `VixenEffectEditor` and `VixenMarks`, the one exception the UI guide allows. It exports `SequencerScene` with `init(model: AppModel)`, which `VixenApp` includes with `#if canImport`. Other areas open a sequence window by id through `WindowID` and `openWindow`. Swift holds no logic: `SequencerModel` (`@Observable`, `MainActor`, one per window) wraps the `SequencerSession` UniFFI object and caches `MenuState`, `TransportUi`, `ToolbarLayout` and the status fields.

**Scene.** `WindowGroup(id: "sequencer", for: SequenceRef.self)` driven by the app's open and new commands. **This departs from the UI guide's "Documents go through `DocumentGroup`", deliberately, and needs the lead's agreement:** a `.tim` cannot open without the profile's element tree, so it opens from the Administration window, not Finder. And `DocumentGroup` brings autosave-in-place, Duplicate instead of Save As and its own close behaviour, which would replace upstream's explicit Auto Save toggle (F15), Save As (F25), appended extension rule (F13) and prompts (F16), and SwiftUI exposes no way to turn those off. Use `NSWindow.isDocumentEdited`, `representedURL`, and `windowShouldClose` for the sheet. Multiple windows as upstream (F20). Minimum content size 640x480 (F3); frame autosave per the window.

**Window regions, top to bottom, same as upstream** (`upstream/Vixen/Assets/Editor.png`). The UI guide's "right-side panels" describes the panels upstream docks right; Effects stays on the left, under the guide's first rule that controls keep their region:

```
Title bar: <sequence>[ edited dot] with profile as subtitle
ToolbarBand                         F98 to F171
HSplit: Effects (left) | Timeline (centre, TimelinePane) | tabbed panels (right)
StatusBar                           F256 to F261
```

**Why not `NSToolbar`.** The upstream toolbars are rows of independently draggable strips which can wrap to several rows, and hold live items such as a speed gauge and thumbnail strips. `NSToolbar` is a single row with overflow, so items would move away from where the owner expects them. So `ToolbarBand` is a SwiftUI `Layout` under the title bar, with no `.toolbar` items. Each strip is a `StripView` with a drag grip. A strip dropped into a row inserts at that point; on drop every strip packs left (F98). Row order comes from `ToolbarLayout`. This is the one deliberate custom control in the window.

**Control mapping**

| Upstream | Mac | Region |
|---|---|---|
| `menuStrip` | `.commands`: `CommandMenu("Sequence")`, plus Edit, View, Tools and Help via `CommandGroup`. Standard Undo and Redo use the window's `UndoManager`, with names from Rust. Commands read `@FocusedValue(\.sequencerModel)` | System menu bar |
| `CheckOnClick` menu items | `Toggle` in commands | Same menu and order |
| Dropdowns that stay open on click (Toolbars, Show/Hide, Drag Box Filter; `Form.cs:223-233`) | `NSMenu` closes on click. Show/Hide Items, Show/Hide Effects and the Drag Box Filter use an `NSPopover` of checkboxes, so several can be ticked in one visit; Toolbars stays a plain menu and may close. Listed in Windows-only omissions | Same button or menu item |
| `ToolStripButton` | `Button` in `StripView`, `.help` with the upstream tooltip, `.buttonStyle(.accessoryBar)` or similar | Same strip and order |
| Tool button icons, listed per strip in section C | SF Symbols where one clearly means the same: `play.fill`, `pause.fill`, `stop.fill`, `backward.end.fill`, `forward.end.fill`, `repeat`, `hourglass`, `arrow.uturn.backward`, `arrow.uturn.forward`, `scissors`, `doc.on.doc`, `doc.on.clipboard`, `pencil`, `cursorarrow`, `music.note`, `plus`, `minus`, `hifispeaker`, `square.and.arrow.down`, `xmark`. Keep Vixen's own for effects, alignment, distribute, the zoom-time and zoom-row pairs, snap, drag box filter, paste special, close gaps and the other distinctive ones (UI guide) | Same strip and order |
| Checked `ToolStripButton` | `Toggle(.button)` | Same |
| `ToolStripDropDownButton` | `Menu` with `.menuIndicator` | Same |
| `UndoButton` | `Menu` with `primaryAction` (one step); list of `undo_labels`, choosing item k does `undo(k+1)` | Edit strip |
| Speed label (`audioToolStripLabel_TimingSpeed`) | `SpeedGaugeView` drawn with `Canvas` from `speed_gauge`; `onTapGesture(count: 1)` and `count: 2`; an `NSView` overlay for wheel and two-finger scroll | Audio strip |
| Library strip buttons | `ScrollView(.horizontal)` + `LazyHStack` of cached `NSImage` thumbnails; hover state, ⌘-click, double-click, `onDrag`, `onDrop` | Library strips |
| `statusStrip` | `StatusBarView`, `HStack`, `.monospacedDigit()`, `ProgressView(value:)` | Bottom |
| `TimelineControl` in `Form_Grid` | `TimelinePane` (timeline spec), always the centre | Centre, not closable |
| `DockPanel` | `NSSplitViewController` with collapsible left and right items. Right pane has tabs in upstream order along its bottom edge, as in `upstream/Vixen/Assets/Editor.png`. Each panel has a detach control, which reparents it into an `NSPanel` (`.floating`, `hidesOnDeactivate = false`) | Left and right |
| View menu window items | Show or hide, restoring the panel's last dock or float state | Menu |
| `Form_Effects` tree | SwiftUI `List` with `DisclosureGroup`s (about 60 items) and `onDrag`; selected effect row highlighted; icons from the effect images, per UI guide | Left |
| `FindEffectForm` list | `NSTableView` in `NSViewRepresentable`, results fetched in bulk | Right tab |
| `Form_ColorLibrary`, `Form_CurveLibrary`, `Form_GradientLibrary` lists | `NSCollectionView` in a representable, with cached thumbnails, reorder and pinch scaling | Right tabs |
| `LayerEditorView` | SwiftUI `List` with `DisclosureGroup` rows, `.onMove` | Right tab |
| `FormEffectEditor` | From the effect editor spec | Right tab |
| `Form_Marks` | From the marks spec | Right tab |
| `MessageBoxForm` | `NSAlert` sheet (see below) | On window |
| Modal dialogs (`EffectTimeEditor`, `BulkEffectMoveForm`, `EffectDistributionDialog`, `Form_AddMultipleEffects`, `SetSequenceLength`, `CreateEvenMarksForm`, `TextDialog`, `InvalidAudioPathDialog`, `EffectDefaultsExportSelectionForm`, `ColorCollectionLibrary_Form`) | `.sheet`, `Form` with `Grid`, and a time field view shared with the timeline area | Attached to the window |
| `ExportDialog` | `Window` scene, non-modal is fine because the export runs off the main thread; keep controls in the same order | Own window |
| `LipSyncTextConvertForm` | Non-modal window, from its own spec | Own window |
| `FormParameterPicker` | `NSPopover` at the pointer, 8 s timer, Esc | At pointer |
| `ColorPicker` | `NSColorPanel`, restricted by discrete colours where the drop or effect needs it | System panel |
| `SaveFileDialog`, `OpenFileDialog` | `NSSavePanel`, `NSOpenPanel` as sheets, with `allowedContentTypes` from the filters | On window |
| Drag and drop of effects, colours, gradients and curves | `NSPasteboard` custom types `org.vixenlights.effect-type`, `.gradient`, `.curve`, plus `NSColor`; `NSDraggingDestination` on the grid (timeline spec) forwards to `drop_validity` and `plan_drop`. Files as file URLs | Same gestures |

**Alerts: verbs on the buttons.** Upstream `MessageBoxForm` shows `YES`/`NO`/`Cancel` or `OK` (`Vixen.Common/Controls/MessageBoxForm.cs:11-62`). Examples for the implementer: F16: Save, Don't Save, Cancel. F303: Remove Audio, Cancel. F302 replace: Replace Audio, Cancel; resize: Resize Sequence, Keep Length. F297: Create Collection, Cancel; Enable and Add, Cancel. F305: Replace Effects, Cancel. F295, F296: Apply to Entire Sequence, Cancel. F12: Map Elements, Leave Unmapped. F341, F351, F364 delete: Delete (`role: .destructive`), Cancel. Text otherwise as quoted, including the upstream typos unless the owner vetoes them in the quirk register.

**Keyboard.** A sequencer-wide `NSEvent.addLocalMonitorForEvents(matching: .keyDown)`, inactive when the first responder is an `NSTextView`, translates to `Key` and `Mods`, calls `command_for_key`, then `run_command` with the pointer location. Menu key equivalents fire first (`performKeyEquivalent`), as upstream's menu shortcuts do. Floating panels are `NSPanel`s in the same app, so the monitor sees their keys too, routed to the key sequencer window (F225). Text fields inside panels keep their own keys. Include ↑ and ↓ even with focus in a table, matching `ProcessCmdKey`, but only when the focused view is not a text field or a list that uses arrows itself (`Hotkeys.cs:313-331`); F321's list and F320's tree take them when focused, a small deliberate difference.

**Pasteboard.** `NSPasteboard` has no change notification. Read `changeCount` on window key, on menu open (`validateMenuItem`) and on the mouse entering the Edit strip, and only when it differs, call `clipboard_count` (F316).

**Status and playback time.** `TimelineView(.animation)`, only while playing, for Current Position, reading a cached Rust position once per frame. Never a timer at 40 ms (`timerPlaying`, `Designer.cs:1220`). The play cursor layer is the timeline spec's. Delay countdown runs from `transport_ui` updates, via `tick`.

**Window load.** The window appears at once with the timeline showing `SequenceLoading`, and stays responsive; upstream's `Enabled = false` is replaced by disabling only sequence commands until `load_progress` finishes (F5, F6). Alerts from `load_report` then show in order: missing audio (F9), renames (F11), deprecated (F10). Unmapped effects (F12) comes before the window shows as upstream, as a sheet on the Administration window.

**Settings scene.** The upstream form has nothing in Preferences with UI. So no Settings pane is added by this area.

**Accessibility and testing.** Use real controls so VoiceOver works. Give `ToolbarBand` strips and items accessibility identifiers from their upstream names (`playBackToolStripButton_Play`, etc.) so `VIXEN_UI_TEST` can click them by label. Each finished window writes `docs/parity/sequencer.md` from the control index at the end of Features, as the UI guide requires: a control missing from that table counts as not done.

## Performance hazards

Each item is what the original does, with what to do instead.

1. **Whole-clipboard decode on hover.** `UpdatePasteMenuStates` deserializes the full clipboard JSON just to count effects, on Edit menu open, on the mouse entering the Edit strip, and on every clipboard change (`Form.cs:3201-3209`, `:5120-5171`). A large copy makes the mouse movement stutter. Cache by `changeCount`, and read a separate small count item.
2. **Marks scan with each hover.** `GetMarksPresent` walks all marks for the same event (`Form.cs:3481-3498`). Keep sorted mark times per collection and binary search.
3. **`DoEvents` and a disabled window during load.** `AddNodeAsRow` calls `Application.DoEvents()` every 600 rows and the form is disabled while loading (`Form.cs:4216-4218`, `:6070`). Build rows and elements on a Rust worker; never block the main thread; the window is responsive from the first frame.
4. **Linear removal.** `RemoveEffectNodeAndElement` walks every row for every effect (`Form.cs:3737-3738`), and `ElementChangedRowsHandler` does likewise (`Form.cs:2234-2242`): O(effects x rows), bad for a 1,133-element show when deleting many. Use the node-to-rows map, and remove in one batch that fires one change notice.
5. **Repeated LINQ over the selection.** `SelectedElements.ElementAt(i)` in loops (`Form.cs:2691-2706`, `:2779-2812`), repeated `.Count()` and `.First()`, each re-walking. In Rust, take a slice once.
6. **Clipboard building.** `affectedElements.Intersect(row.SelectedElements)` for each row grows quadratically (`Form.cs:5208`). Use a `HashSet`. The behaviour with duplicate rows must stay (F317), so pin it with a test first.
7. **Mark insert and bulk add.** `mc.Marks.All(x => x.StartTime != time)` per add makes `AddMarksToSelectedEffects` O(effects x marks) (`Form.cs:3075`, `:6288-6295`), and each added mark also starts a full dirty scan (`Form.cs:3087`, hazard 9). Binary search, apply as a batch, and scan once.
8. **Align to marks.** `FindNearestMark` scans every mark for every effect edge (`Form.cs:6323-6348`). Use a sorted index, but pin tie-breaking first: the first found in collection order wins a tie, and start before end within a mark.
9. **Dirty re-render scan.** `CheckAndRenderDirtyElementsAsync` scans every effect in parallel after every mark change, including each mark moved, and again for library edits (`Form.cs:4106-4122`). Track which effects depend on marks or a library item and dirty only those. Coalesce with a trailing delay during a drag.
10. **Profile file writes.** `PutSetting` saves the whole XML on every call (`XMLProfileSettings.cs:72-82`), about 60 file writes at each close, more with effect entries (`Form.cs:5808-5838`, `Toolstrip.cs:2006-2083`). Batch to one write off the main thread.
11. **Reflection on hot paths.** `PropertyDiscovery` and `TypeDescriptor.GetProperties` during drag hover and context menu builds (`Form.cs:997-1038`, `:2005-2017`). Use the static parameter tables in `vixen-effects`, and cache the result by effect type.
12. **Discrete colours during hover.** `GetDiscreteColors` walks the effect's target nodes (`Form.cs:1046-1052`); only upstream's per-effect cache saves it (`_dragDropToElementEffect`). Keep that cache keyed by hovered effect and payload kind.
13. **Thumbnails rebuilt in full.** `PopulateColors`, `Populate_Curves`, `Populate_Gradients` regenerate every bitmap on any change, in both the panels and the strips, and on every ⌘-scroll step (`Toolstrip.cs:968-1004`, `:1093-1141`, `:1220-1270`; `Form_CurveLibrary.cs:173-208`, `:579-617`). Use a content-keyed thumbnail cache. Scale the cached image while scrolling and regenerate on gesture end.
14. **Library reorder rewrites the library.** Clear plus re-add for every item, with `await` per item, firing bulk change events that re-render effects (`Toolstrip.cs:490-522`). Add an order vector to the library model, so a reorder fires no re-render.
15. **Undo list rebuilt per action.** `UndoItems.Clear()` then adding up to 1000 strings into a `ListBox` per push (`Form.cs:5474-5504`). Build the list only when the drop-down opens.
16. **Serialized undo payloads.** `EffectModelCandidate` serializes each effect's whole data by `DataContractSerializer` (`TSE/EffectModelCandidate.cs:38-61`). In Rust, `Arc` the parameter values, and swap pointers on undo.
17. **Per-effect re-render.** `UpdateEffectProperty` triggers a render for each property of each effect during a colour collection apply (`Form.cs:4266-4271`, `:2054-2110`). Queue once per effect, after the loop.
18. **Find refresh per occurrence.** `RowElementAddedRemoved` fires for every row occurrence, and `UpdateListView` rescans all rows each time (`FindEffectForm.cs:84-98`, `:122-180`). Debounce, and use a generation counter.
19. **Layer drag and playback polling.** `timerPlaying` at 40 ms sets the status text and the cursor (`Designer.cs:1220`, `Form.cs:3457-3468`). Use a display link only while playing; update the status text at most at display rate, and let SwiftUI skip identical strings.
20. **Toolbar states and menus at load.** Five loops over strips with layout suspended, and every effect item built twice (Add Effect, Drag Box Filter, Show/Hide, strip) (`Form.cs:1234-1285`, `Toolstrip.cs:1349-1427`). Fetch one effect list from Rust. `menu_state` is one call.
21. **Export progress.** Reporting per frame to the UI thread can flood it. Limit to about 10 updates a second and run the export on a worker with `run_export`.
22. **Effect editor debounce.** Keep the 200 ms selection debounce (`FormEffectEditor.cs:59-63`, `:126-136`): rubber band selection would otherwise rebuild the property grid for each change. Reading selection from Rust must be bulk.
23. **Preview during editing.** `SetPreviewsTopMost` and `PreviewStop` run on each play (`Form.cs:5996-6036`). Keep them cheap in the preview area. The frame thread must never wait on the UI.
24. **Saving on the UI thread.** Auto save and every other save serialize and write the whole sequence on the UI thread, and the timer does not check for playback (`Form.cs:1543-1611`, `:2143-2150`). On a big show that freezes the window and can stutter the preview in mid-play. Snapshot the document, write on a worker, and do not let the frame thread wait for it.
25. **Fill by marks.** `times.IndexOf(mark)` runs twice per mark in the Fill path, so a long mark list is quadratic (`Form.cs:2614-2616`). Walk by index.
26. **Colour library events.** `Save_ColorPaletteFile` raises `ColorsChanged` and most callers then raise it again, so the strip's bitmaps are all rebuilt twice per change (`Forms/Form_ColorLibrary.cs:165`, `:173`, `:181`, `:189`). One event, and hazard 13's cache.
27. **Settings re-read.** Each `new XMLProfileSettings()` parses the whole file, and one window open or close does that several times (`Form.cs:347`, `:5808`, `Toolstrip.cs:64`, `:243`, `:2007`, each library window). `EditorPrefs` reads once.
28. **Paste decode.** `GetEffectData` deserializes each effect through the binary XML reader on the UI thread (`Form.cs:5374`, `TSE/EffectModelCandidate.cs:72-98`), so a large paste blocks. Rust: decode on a worker from the JSON, then apply as one batch with one render queue.

## Tests worth writing

Rust unit tests unless marked UI. Expected values are read from the code cited.

**Saving and row settings**
- `save_path_with_extension("Song.txt", ".tim")` is `Song.txt.tim` and `"Song.tim"` stays (`Form.cs:1576-1580`).
- Row settings with default height 30: height 38 closed is stored; 37 closed is dropped; 22 closed is stored; 23 closed is dropped; 30 open is stored as (30, true, visible); a height outside the buffer while open keeps its real `treeOpen` (`Form.cs:1627-1634`).
- Title: `Untitled - P` for no name, `Song * - P` when modified (`Form.cs:1809-1812`).
- `close_action`: modified and app closing: prompt without Cancel. Modified, not closing: prompt with Cancel. Not modified but state modified: `SaveSilently`. Neither: `Close` (`VixenApplication.cs:847-867`).
- Auto save: arms on the transition to modified only; a second edit does not reset the deadline; saving stops it; unchecking stops it; re-checking while modified re-arms; never-saved sequence asks for a path when due; cancelling leaves it unarmed until the next transition (`Form.cs:1668-1679`, `:1822-1839`, `:2143-2150`).
- Loading: effect start past length is dropped; end past length is shortened to `length - start`; effect with no row dropped; length 0 becomes 1 minute (`Form.cs:3897-3914`, `:1463-1464`).
- Unmapped effects (F12): answering No, then loading: they are gone from the sequence. Mapping on a sequence in the normal folder: not modified after load, under upstream's behaviour. Mapping on a foreign sequence: path blank and modified.

**Duration and fit**
- `default_effect_duration`: visible end 10 s, start 0: 2 s. Visible end 10 s, start 9 s: 800 ms. Visible end 10 s, start 9.9 s: 250 ms (80 ms is floored). Visible end - start exactly 2 s: 1.6 s (`Form.cs:6360-6378`).
- `fit_new_effect`: start 59.9 s, duration 250 ms, length 60 s: start 59.75 s. Length 100 ms: start 0, duration 100 ms. Duration 300 ms: unchanged (`Form.cs:3821-3830`).

**Add multiple**
- `possible_effects(0, 10 s, 2 s, 2 s)` is 2 (uses 2, 6, then 10 is not below 10, and the last is taken back). `possible_effects(0, 10.001 s, 2 s, 2 s)` is 3; with an end of 14.001 s it is 4. Duration 0: 0. Span shorter than one effect: 0 (`Form_AddMultipleEffects.cs:404-416`).
- `possible_effects_on_marks`: 5 times, plain 5; fill 4; skip 3; fill and skip 2; 0 times with fill 0 (`:440-442`).
- Plain plan stops when `next + duration > length` (`Form.cs:2565-2569`).
- The dialog opens at the possible count: start 0, end 60 s, 2 s and 2 s gives 15 by the strict loop, not 2 (`Form_AddMultipleEffects.cs:173-183`, `:404-416`).
- On marks: fill gives each effect up to the next mark with a floor of 10 ms and no effect on the last mark; skip-every-other takes marks 1, 3, 5; `alignToMarkDuration` uses each mark's duration; count caps the result (`Form.cs:2603-2635`).

**Distribute**
- Equally, 4 effects spanning 0 to 8 s, selection in time order: (0,2), (2,4), (4,6), (6,8). Reverse selection order (first starts after last) walks from the last in the same way. Span 0.1 s over 4: fails at 0.025 s; exactly 0.05 s per effect passes (`Form.cs:2663-2686`).
- Dialog: all nine combinations of duration mode and placement, with start at first and at last, comparing against hand computed values from `Form.cs:2760-2815`. Pin that Do Not Change plus Stair Step gives `start + offset*i`, and that Specified with Spaced adds spacing to the previous end.

**Align to marks**
- `find_nearest_mark`: threshold strict, mark exactly `threshold` away not matched; two equal distances: first wins; `ShowGridLines` false ignored; tail only with `ShowTailGridLines`; no match returns 0 (`Form.cs:6316-6352`).
- `align_to_marks("Start")` where no mark is near: skipped. Effect start 0 with End mode: skipped. A found mark at time 0: treated as no match (`Form.cs:6261-6265`).

**Clipboard**
- Copy of effects on rows 2, 5 (visible order) gives offsets 0 and 3; `earliest_start` is the minimum. A row with a duplicate name is skipped, without counting; an element already taken counts the row (`Form.cs:5196-5243`).
- Paste Default: earliest 10 s, effects at 10 and 12, paste at 30: 30 and 32. Paste time 0: 10 and 12. Past end: dropped; overrunning: shortened to fit; row out of range: dropped; Deprecated: dropped with its name reported (`Form.cs:5330-5372`).
- Paste Visible Marks: 3 effects and marks at 1, 2, 3, 4 with cursor 2: times 2, 3, 4; with only 2 marks left: 2 pasted. Two visible collections merge sorted (`Form.cs:5287-5315`).
- Paste Invert: offsets 0, 1, 3 become 3, 2, 0 (`Form.cs:5318-5321`). With the first record skipped for starting past the end, the second record takes index 0 of the inverted list, row offset 3 rather than 2, because the index counts effects pasted (`Form.cs:5349`).
- Copy skips selected effects on rows that are not visible (`Form.cs:5200`).
- Layer: id present reuses it; absent falls back to name and type; else default (`Form.cs:5385-5399`).
- Paste menu: count 0, 1, 2 with marks on and off: table from `Form.cs:5123-5127`, and the context menu's variant from `ContextMenu.cs:434-453`.

**Transport**
- `transport_ui` table: no context; stopped; playing; paused (`Form.cs:3500-3545`).
- Delay: Off; set 5 s, play: countdown 5,4,3,2,1 then play; second Play during countdown ignored; Stop during countdown cancels and disables Stop; set Loop clears delay; set delay clears Loop (`Form.cs:3284-3299`, `:3409-3416`, `Menu.cs:66-128`, `Form.cs:6099-6107`).
- `play_from` with start >= range end: from range start or 0 (`Form.cs:3354-3357`).
- `play_between` when paused stops first; negative start falls back; end past total falls back (`Form.cs:3370-3383`).
- Ruler: plain click sets start and nil end and modified; ⌘-click plays from there with range restored at the end (`Form.cs:2868-2896`, `:3447-3452`).
- Speed, with `f32` inputs: `0.1` forwards and updates the display, because it widens to 0.10000000149; `0.09999993` stores 0.1 and leaves the source and display untouched; 0.5 forwards; 4.0 forwards; 4.1 stores 4.0 without forwarding; Decrease disabled at or below 0.1; Increase disabled at 4.0 (`Form.cs:5949-5974`).
- Speed stepping from 1.0 (F271). Decrease, no rounding: eight clicks give 0.19999993 and gauge `20`; the ninth stores 0.1, gauge still `20`, Decrease still enabled. Increase: thirty clicks give 3.9999983; the next stores 4.0, gauge `400`, Increase still enabled. Wheel, rounding first: nine steps down give exactly 0.1; the tenth stores 0.1 with no display update. Thirty up give exactly 4.0; the next stores 4.0 the same way (`Toolstrip.cs:1698`, `:1702`, `:1725`, `:1730`).
- Playback range (F423): a ruler drag does not mark modified; a plain ruler click does; a ⌘-started play restores the old range at the end; a save during it stores the temporary one.
- `speed_gauge`: speed 1: text "100", x 3, needle 230 ending at (3,17), hue 90. Speed 4: text "400", needle 350, hue 0. Speed 0.5: text "50", x 6, needle 210, hue 55. Speed 0.1: text "10", needle 194, hue 251 (-5 as a byte) (`Toolstrip.cs:1640-1664`, `HSBColor.cs:53-59`). Note banker's rounding in `Math.Round(_timingSpeed, 1)`.
- `is_natural_end`: from `SequenceExecutorLifecycleTests`: not while at start; position 0 after having advanced is an end (F277).

**Hotkeys**
- `command_for_key` for each table row, plus: Shift+D is D; Ctrl+Shift+D is Ctrl+D; Ctrl+E is plain E with hold false; Up with Shift does nothing. Space when playing: Stop; Ctrl+Space: Pause; Shift+Space: mark, only when a timing source exists (`Hotkeys.cs:8-26`, `:112-121`).

**Toolbars and settings**
- `ChangeToolStripImageSize` compare rule: saved 28 from current 16 gives large; saved 16 gives small; toggling flips; default main strips large and library strips small (`Toolstrip.cs:93-114`, `:923-940`).
- Tool strip state split: library strips go in the per-profile file, others in the standard file; missing standard file: per-profile is not read (`Toolstrip.cs:1965-2000`, `:2092-2106`). Standard file present, no profile file: a library Show/Hide change creates its entry under the F177 FIX (upstream stores nothing).
- Settings round trip with every key from the table, including an unknown extra key surviving. Upstream text for bools is `True` and `False`.
- Export dialog rules: FPP radios disabled unless Generate is checked and format is Falcon; 2.x sets compress enabled (`ExportDialog.cs:442-460`, `:479-483`). A new dialog state has every controller active in system order, whatever the last one did (`Export.cs:162-167`).

**Files**
- Papagayo: a two-voice sample: phonemes coalesced; last phoneme end = word end; gap filled by Rest; unknown name is Rest; `Rest` duration; `etc`; missing header fails; truncated file fails; trailing line fails (`PapagayoDoc.cs`).
- Even marks: 4 divisions, 0 to 8 s: marks at 0, 2, 4, 6, 8, duration 1.6 s, name `4 Divisions`; second time, an existing match removed and name stays; `4 Divisions` and `4 Divisions #2` exist: `#3` (`Menu.cs:658-699`).
- Library import names: `A`, `A 2`, `A 3` (`Form_CurveLibrary.cs:516-522`).
- `.vfc`, `.vcc`, `.vcl`, `.vgl`: read real `DataContractSerializer` samples, and write back identically.

**Undo**
- Every description string with count 1 and 2, including `Moved 1 effect` for a mark move and `Bulk Change 2 Marks` (`Undo/*.cs`).
- Limit 1000; order newest first; `undo(3)` then `redo(3)` restores; the Mac clears redo on a new action only if the owner accepts the proposal in F419, otherwise pin upstream's behaviour.

**UI harness (`VIXEN_UI_TEST`)**
- Open a copied real sequence; snapshot; every menu command by path exists and is enabled per `menu_state`; toolbar strip show and hide; alert texts. Never against the read-only show folder. Tests stay silent, with no hardware.

**Upstream tests covering this area** (`upstream/Vixen/src/Vixen.Tests/Sequencer/`):

- `SequenceExecutorLifecycleTests.cs` (455 lines), `SequenceExecutorTestCollection.cs` (15): play returns promptly while timing stays at start; loop restart uses Stop, Position, Start; partial range restarts at configured start and defers natural end until timing advances; position 0 after advancing is a natural end; stop or dispose before the posted callback does nothing; natural end stops once and raises ended once. Port to `is_natural_end` and the transport.
- `LibraryDragPayloadFactoryTests.cs` (105): link, independence, colour, null, unsupported, blank name.
- `Vixen.Tests/EffectDefaults/EffectDefaultsServiceTests.cs` (207), `EffectDefaultScrubberTests.cs` (92): saved defaults, export, import (F93 to F95, F285). `TimelineActiveRowNavigationTests.cs` (121): `MoveActiveRow` used by F189 and F190. `GridCloneDragDuplicateRowsTests.cs` (263): clone drag used by F243. `Vixen.Tests/AudioPlayer/CoreAudioPlayerOwnershipTests.cs` (106): output ownership only, of little use on the Mac.
- `LayerEditorLayerServiceTests.cs` (221), `LayerEditorViewModelTests.cs` (150), `LayerImportExportServiceTests.cs` (420): layer add above default, remove and move rules, quick rename, `.v3l` export, plan, import, name collisions.
- `MarkCollectionNameServiceTests.cs` (176): unique names and duplicate renames (F298, F11); shared with the marks spec.
- `MarkCollectionTypeTests.cs` (59), `MarkCollectionImportCommitterTests.cs` (164), `MarkCollectionImportSelectionViewModelTests.cs` (119), `PangolinBeyondMarkImportTests.cs` (227): marks spec.
- `LayerMixingFilterDataJsonSerializationTests.cs` (126): filter data JSON shape for `.v3l` (F340).
- `Timeline*`, `Grid*`, `MarksBar*`, `Ruler*`, `Waveform*`, `RowEventScopeTests`: timeline spec; `GridAlignmentNullReferenceTests` also pins null reference alignment used by F228.

## Windows-only omissions

| Upstream | Why omitted | Mac behaviour |
|---|---|---|
| `User32.AddClipboardFormatListener`, `WM_CLIPBOARDUPDATE` (`TSE/User32.cs`, `Form.cs:5086-5118`) | Win32 | `NSPasteboard.changeCount` (see SwiftUI section) |
| WeifenLuo `DockPanel`, its XML (`TimedSequenceEditorForm.xml`), `DestroyAndRecreateDockPanel`, `Form_ToolPalette` legacy name (`Form.cs:265-290`, `:581-646`) | Windows docking library | Native split panels, own layout file; a corrupt layout still resets to defaults |
| Off-screen and small floating timeline rescue (`Form.cs:322-335`) | The timeline is not floatable on the Mac | None. It is always the centre |
| `SendKeys` focus tricks (`Form_Marks.cs:58-64`, `LayerEditor.cs:43-46`), `ElementHost` | Hosting WPF in WinForms | Normal first responder |
| `SendMessage(LVM_SETICONSPACING)` (`Form_ColorLibrary.cs:28-41`, curve and gradient likewise) | Win32 list view | `NSCollectionView` layout |
| `ScalingTools`, `PerformAutoScale`, `_scaleFactor` | DPI scaling; macOS handles it | Points. 16 and 28 stay as point sizes |
| `Application.DoEvents`, `InvokeRequired` plumbing, `Cursors.WaitCursor`, `UseWaitCursor` | Threading model | Async work with progress |
| `FormBorderStyle.FixedSingle` and `Enabled=false` during load (`Form.cs:6070-6071`) | Blocks the UI | Sequence commands disabled instead |
| `ThemeToolStripRenderer`, `ThemeUpdateControls`, `ThemeGroupBoxRenderer`, owner-draw combo boxes | Hand-drawn dark theme | Native dark appearance |
| `ToolStripEx` / `MenuStripEx` `ClickThrough` | First click activation quirk | `acceptsFirstMouse` |
| `Keys`-to-WPF conversion (`Hotkeys.cs:41-54`), two broadcast channels | Two UI toolkits | One responder path |
| Menus that stay open on click (`toolStripMenuItem_Closing`, `Toolstrip.cs:1925-1929`) | `NSMenu` always closes | Popover of checkboxes for multi-tick lists; the Toolbars list closes after each pick |
| Zero-size empty `linkLabel1` in `FindEffectForm` and `BulkEffectMoveForm` | Invisible: no text, no size (`FindEffectForm.Designer.cs:51`) | Not built |
| `toolTip` components with no text in `BulkEffectMoveForm`, `EffectDistributionDialog`, `Form_AddMultipleEffects`, `CreateEvenMarksForm` | No `SetToolTip` calls in their designers, so they never show | None |
| Windows audio settings: share mode, latency, output mode, `MMDevice` ids, power resume hooks (`AudioOutputManager.cs:100-124`) | WASAPI specific | Core Audio via `vixen_audio`; device chosen by name, keeping the default placeholder idea |
| Old settings cleanup (`StandardNudge`, `SuperNudge`, `NudgeSettings`, `Form.cs:5850-5854`) | Migration from old Windows versions only | Not needed, harmless if a copied file has them |
| Written-but-unread `<Group>/<Effect>` settings (`Toolstrip.cs:2040-2059`) | Dead data | Not written |
| `ShowDestinationMB` in `ExportDialog.cs:223-237`, `backgroundWorker1` | Dead code | Not built |
| `ShowMarkManager`, `RefreshSequence`, `ResetTimeLineSettings` (`Form.cs:5588-5591`, `:5936-5947`, `:6304-6308`) | Dead code: nothing in this folder calls them. `ResetTimeLineSettings` would set `VisibleTimeStart` to 0 and `TimePerPixel` to 100000 ticks | Not built |
| `MarkTapper`, `AutomaticMusicDetection` and the form's `ITiming` and `IExecutionControl` members that served them (`Form.cs:5860-5906`), `PlaySequenceFrom` (`Form.cs:3333`), `AddEffectsModifiedToUndo(dict, label)` and `EffectsModifiedUndoAction` (`Form.cs:5564`), `ShowGenericErrorMessage` (`:3983`), `ElementAddedToRowHandler` and `ElementRemovedFromRowHandler` (`:2210-2218`), `changeMappings_Click` (`:6213`), `lipSyncMappingsToolStripMenuItem_DropDownOpening` (`Menu.cs:579`), `AddColorSet` (`Form_ColorLibrary.cs:176`), the unused `Selection` getter that throws (`Form.cs:5602`) | Dead code: no caller in the source tree, or an empty body | Not built |
| Focus hand-offs: `TimelineControl.Focus()` after library actions, `toolStripAudio.Focus()` on gauge hover so the wheel reaches it, `Form_CurveEnter`, `toolStrips_MouseEnter` focus (`Toolstrip.cs:570`, `:607`, `:1715`, `:1720`, `Form_CurveLibrary.cs:157-160`) | WinForms keyboard and wheel routing | First responder and hover tracking; scroll events reach the view under the pointer |
| `UseWaitCursor`, `stopButton_MouseEnter` in Export (`ExportDialog.cs:121`, `:142`, `:215-218`) | Wait cursor plumbing | Progress bar only |
| `comboBox_DrawItem`, `buttonTextColorChange`, `groupBoxes_Paint`, `flowLayoutPanel1_Paint` | Theme drawing | Native controls |
| Windows clipboard JSON format interoperability | Different module data encodings across platforms | Mac-only pasteboard type |

