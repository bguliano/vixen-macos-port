# Marks, layers, audio analysis and lip sync

## Scope

Citation shorthand used everywhere below. Upstream paths are relative to `upstream/Vixen/src`. Paths starting `crates/` or `app/` are this repo's.

| Prefix | Path |
|---|---|
| `TSE/` | `Vixen.Modules/Editor/TimedSequenceEditor/` |
| `Form.cs`, `Menu.cs` | `TSE/TimedSequenceEditorForm.cs`, `TSE/TimedSequenceEditorForm_Menu.cs` |
| `Docker/` | `TSE/Forms/WPF/MarksDocker/` |
| `MDVM`, `MCVM` | `Docker/ViewModels/MarkDockerViewModel.cs`, `Docker/ViewModels/MarkCollectionViewModel.cs` |
| `MIES` | `Docker/Services/MarkImportExportService.cs` |
| `MC.cs`, `Mark.cs`, `MD.cs` | `Vixen.Modules/App/Marks/MarkCollection.cs`, `.../Mark.cs`, `.../MarkDecorator.cs` |
| `NameSvc` | `Vixen.Core/Marks/MarkCollectionNameService.cs` |
| `LE/`, `LEVM`, `LESvc`, `LIES` | `Vixen.Modules/Editor/LayerEditor/`, `LE/ViewModels/LayerEditorViewModel.cs`, `LE/Services/LayerEditorLayerService.cs`, `LE/Services/LayerImportExportService.cs` |
| `SL.cs` | `Vixen.Core/Sys/LayerMixing/SequenceLayers.cs` |
| `BB/`, `QM/` | `Vixen.Modules/Analysis/BeatsAndBars/`, `Vixen.Modules/Analysis/QMLibrary/` |
| `LS/`, `TCF`, `PU` | `Vixen.Modules/App/LipSyncApp/`, `LS/LipSyncTextConvertForm.cs`, `LS/LipSyncPhonemeUtils.cs` |
| `LML`, `LMS`, `LME`, `LMD` | `LS/LipSyncMapLibrary.cs`, `LS/LipSyncMapSelector.cs`, `LS/LipSyncMapMatrixEditor.cs`, `LS/LipSyncMapData.cs` |

Sibling specs this one meets at its edges: `docs/spec/sequencer-shell.md` (menu entries F69, F71, F80-F92, F297-F301, F328-F340, F404, which say the docker's content and the even-marks dialog belong here), `docs/spec/timeline.md` (mark rows, ruler, snapping, the marks bar, F110-F139), `docs/spec/property-filter-controller-setup.md` (Face Setup F184-F192, layer filter sheets F212-F229), `docs/spec/effect-editor.md` (mark collection pickers), `docs/spec/display-setup.md` (LipSync element template F106). Where those describe a behaviour fully, this spec cites them rather than restating it. Where this spec read the source more closely, the detail here wins.

**Read in full, in scope:**

| File | Lines | File | Lines |
|---|---|---|---|
| `Vixen.Core/Marks/IMarkCollectionSelector.cs` | 17 | `Vixen.Core/Marks/IMarkCollection.cs` | 56 |
| `Vixen.Core/Marks/IMarkType.cs` | 33 | `Vixen.Core/Marks/MarkCollectionRenameRecord.cs` | 10 |
| `Vixen.Core/Marks/MarkCollectionNameService.cs` | 146 | `Vixen.Core/Marks/IMarkCollectionSelection.cs` | 35 |
| `Vixen.Core/Marks/IMark.cs` | 13 | `Vixen.Core/Marks/IMarkDecorator.cs` | 11 |
| `Vixen.Core/Marks/MarkCollectionSelectionService.cs` | 52 | `Vixen.Modules/App/Marks/MarkCollection.cs` | 385 |
| `Vixen.Modules/App/Marks/Mark.cs` | 105 | `Vixen.Modules/App/Marks/MarkDecorator.cs` | 72 |
| `Vixen.Modules/App/Marks/BindableBase.cs` | 30 | `Vixen.Modules/App/Marks/Mode.cs` | 8 |
| `Vixen.Modules/App/Marks/Marks.csproj` | 11 | `TSE/Forms/Form_Marks.cs` | 68 |
| `TSE/Forms/Form_Marks.Designer.cs` | 52 | `Docker/Views/MarkDockerView.xaml` | 107 |
| `Docker/Views/MarkDockerView.xaml.cs` | 14 | `Docker/Views/MarkCollectionView.xaml` | 173 |
| `Docker/Views/MarkCollectionView.xaml.cs` | 10 | `Docker/Views/MarkExportWindow.xaml` | 82 |
| `Docker/Views/MarkExportWindow.xaml.cs` | 13 | `Docker/Views/MarkCollectionExportRowView.xaml` | 40 |
| `Docker/Views/MarkCollectionExportRowView.xaml.cs` | 13 | `Docker/Views/MarkCollectionImportSelectionWindow.xaml` | 65 |
| `Docker/Views/MarkCollectionImportSelectionWindow.xaml.cs` | 16 | `Docker/ViewModels/MarkDockerViewModel.cs` | 338 |
| `Docker/ViewModels/MarkCollectionViewModel.cs` | 814 | `Docker/ViewModels/CheckBoxStateBase.cs` | 43 |
| `Docker/ViewModels/LevelCheckBoxState.cs` | 7 | `Docker/ViewModels/LinkedToCheckBoxState.cs` | 7 |
| `Docker/ViewModels/CollectionTypeCheckBoxState.cs` | 9 | `Docker/ViewModels/MarkExportWindowViewModel.cs` | 209 |
| `Docker/ViewModels/MarkCollectionExportRowViewModel.cs` | 72 | `Docker/ViewModels/MarkCollectionImportOptionViewModel.cs` | 47 |
| `Docker/ViewModels/MarkCollectionImportSelectionViewModel.cs` | 96 | `Docker/Services/MarkImportExportService.cs` | 709 |
| `Docker/Services/PangolinBeyondMarkParser.cs` | 82 | `Docker/Services/PangolinBeyondMarkCollectionFactory.cs` | 57 |
| `Docker/Services/PangolinBeyondMarkRecord.cs` | 6 | `Docker/Services/PangolinBeyondImportMode.cs` | 8 |
| `Docker/Services/ExportableMarkCollection.cs` | 10 | `Docker/Services/MarkCollectionImportResult.cs` | 65 |
| `Docker/Services/MarkCollectionImportCommitter.cs` | 73 | `Docker/Services/MarkCollectionImportStatus.cs` | 9 |
| `Docker/Services/MarkCollectionImportType.cs` | 13 | `TSE/MarkCollectionImportDialog.cs` | 58 |
| `TSE/CreateEvenMarksForm.cs` | 43 | `TSE/MarkTapper.cs` | 85 |
| `TSE/MarkTapper.Designer.cs` | 189 | `TSE/AutomaticMusicDetection.cs` | 178 |
| `TSE/PapagayoDoc.cs` | 605 | `TSE/LayerEditor.cs` | 102 |
| `TSE/LayerEditor.Designer.cs` | 35 | `LE/LayerEditor.csproj` | 18 |
| `LE/ViewModels/LayerEditorViewModel.cs` | 357 | `LE/Views/LayerEditorView.xaml` | 124 |
| `LE/Views/LayerEditorView.xaml.cs` | 215 | `LE/Services/LayerEditorLayerService.cs` | 123 |
| `LE/Services/ILayerEditorLayerService.cs` | 74 | `LE/Services/LayerMixingFilterResolver.cs` | 17 |
| `LE/Services/ILayerMixingFilterResolver.cs` | 17 | `LE/Services/LayerImportExportService.cs` | 272 |
| `LE/Services/ILayerImportExportService.cs` | 36 | `LE/ImportExport/LayerExportDocument.cs` | 44 |
| `LE/ImportExport/LayerExportRecord.cs` | 47 | `LE/ImportExport/LayerImportEntry.cs` | 38 |
| `LE/ImportExport/LayerImportPlan.cs` | 65 | `LE/ImportExport/LayerImportResult.cs` | 29 |
| `LE/ImportExport/LayerImportSkippedRecord.cs` | 29 | `LE/Converters/DefaultLayerVisibilityConverter.cs` | 32 |
| `LE/Converters/HasSetupVisibilityConverter.cs` | 32 | `BB/BeatsAndBars.cs` | 551 |
| `BB/BeatsAndBarsSettings.cs` | 215 | `BB/MusicStaff.cs` | 252 |
| `BB/PreviewWaveform.cs` | 134 | `BB/PreviewWaveform.Designer.cs` | 47 |
| `BB/BeatsAndBarsProgress.cs` | 46 | `BB/BeatsAndBarsDescriptor.cs` | 40 |
| `BB/BeatsAndBars.csproj` | 32 | `QM/QMBarBeatTrack.h` | 15 |
| `QM/QMBarBeatTrack.cpp` | 11 | `QM/WrapperFiles/ManagedPlugin.h` | 57 |
| `QM/WrapperFiles/ManagedPlugin.cpp` | 293 | `QM/WrapperFiles/ManagedFeature.h` | 27 |
| `QM/WrapperFiles/ManagedRealtime.h` | 67 | `QM/QMFiles/BarBeatTrack.cpp` | 478 |
| `QM/QMFiles/UPSTREAM-PROVENANCE.md` | 84 | `LS/LipSyncPhonemeUtils.cs` | 789 |
| `LS/LipSyncTextConvertForm.cs` | 453 | `LS/LipSyncTextConvertFailForm.cs` | 107 |
| `LS/LipSyncMapLibrary.cs` | 295 | `LS/LipSyncMapData.cs` | 172 |
| `LS/LipSyncMapItem.cs` | 98 | `LS/LipSyncMapStaticData.cs` | 92 |
| `LS/LipSyncMapSelector.cs` | 277 | `LS/LipSyncMapMatrixEditor.cs` | 668 |
| `LS/LipSyncMultiPicSelect.cs` | 140 | `LS/LipSyncNodeSelect.cs` | 301 |
| `LS/LipSyncMapDescriptor.cs` | 49 | `LS/LipSyncApp.csproj` | 24 |
| `Vixen.Modules/Property/Face/FaceComponent.cs` | 16 | `Vixen.Modules/Property/Face/FaceMapItem.cs` | 52 |
| `Vixen.Modules/Property/Face/FaceModule.cs` | 112 | `Vixen.Modules/Property/Face/FaceData.cs` | 42 |
| `Vixen.Modules/Property/Face/FaceDescriptor.cs` | 25 | `Vixen.Modules/Property/Face/FaceSetupHelper.cs` | 540 |
| `Vixen.Modules/Property/Face/Face.csproj` | 20 | | |

**Read for layout through the generated `docs/reference/forms/*.json`, with targeted greps of the designer where the JSON left a property out, rather than line by line:** `TSE/MarkCollectionImportDialog.Designer.cs` 187, `TSE/CreateEvenMarksForm.Designer.cs` 155, `TSE/AutomaticMusicDetection.Designer.cs` 2260, `BB/BeatsAndBarsSettings.Designer.cs` 264, `BB/MusicStaff.Designer.cs` 203, `BB/BeatsAndBarsProgress.Designer.cs` 92, `LS/LipSyncTextConvertForm.Designer.cs` 266, `LS/LipSyncTextConvertFailForm.Designer.cs` 360, `LS/LipSyncMapSelector.Designer.cs` 140, `LS/LipSyncMapMatrixEditor.Designer.cs` 289, `LS/LipSyncMultiPicSelect.Designer.cs` 362, `LS/LipSyncNodeSelect.Designer.cs` 284, `Vixen.Modules/Property/Face/FaceSetupHelper.Designer.cs` 212.

**Read as the supporting code this area touches.** Whole files: `Vixen.Common/WPFCommon/Services/DownloadService.cs` 83, `Vixen.Modules/App/TimingTrackBrowser/` (`ViewModels/VendorInventoryWindowViewModel.cs` 358, `ViewModels/VendorMetadataViewModel.cs` 171, `Views/VendorInventoryWindow.xaml` 120, `Views/SongInventoryTree.xaml` 53, `Views/VendorMetadataView.xaml` 23, `Import/XLights/XTimingLyricInventoryImporter.cs` 32, `Import/XLights/XTimingSongInventoryMapper.cs` 142, and all of `Model/`, 617 lines), `Vixen.Modules/Sequence/Timed/MarkCollection.cs` 69, `Vixen.Core/Sys/LayerMixing/ILayer.cs` 32, `DefaultLayer.cs` 16, `StandardLayer.cs` 14, `Layer.cs` 118, `SequenceLayers.cs` 229, `Vixen.Core/Module/MixingFilter/ILayerMixingFilter.cs` 15, `LayerMixingFilterModuleInstanceBase.cs` 75, `Vixen.Core/Services/LayerMixingFilterService.cs` 28, `Vixen.Core/Module/SequenceType/Surrogate/LayerMixingFilterSurrogate.cs` 34, every `Vixen.Modules/LayerMixingFilter/*` non-designer `.cs` (9 filters), `Vixen.Common/Controls/NumberDialog.cs` 35, `Vixen.Modules/Effect/LipSync/PhonemeMappingConverter.cs` 23.

Ranges only: `Form.cs` (1095-1112, 1205-1225, 2395-2460, 2890-3125, 3430-3445, 3655-3690, 4120-4150, 5630-5700, 6115-6211, 6280-6300), `Menu.cs` (395-402, 468-510, 503-625, 625-706), `TSE/TimedSequenceEditorForm_Hotkeys.cs` (100-125), `Vixen.Modules/Sequence/Timed/TimedSequenceData.cs` (45-116 of 116), `Vixen.Core/Module/SequenceType/SequenceTypeDataModelBase.cs` (80-100, 135-160), `Vixen.Core/Sys/Modules.cs` (36-92, 165-205, 370-400), `Vixen.Common/WPFCommon/Services/MessageBoxService.cs` (88-123), `Vixen.Modules/Media/Audio/Audio.cs` (29-70, 116-148), `Vixen.Modules/Effect/LipSync/LipSync.cs` (75-100, 186-240), `Vixen.Common/Controls/TimeLineControl/Waveform.cs` (`WaveformStyle` only), `Vixen.Common/Controls/TimeLineControl/Ruler.cs:890`.

**Tests read,** under `Vixen.Tests/`. In full: `Sequencer/MarkCollectionNameServiceTests.cs` 176, `Sequencer/PangolinBeyondMarkImportTests.cs` 227, `Sequencer/MarkCollectionTypeTests.cs` 59, `Sequencer/LayerImportExportServiceTests.cs` 420, `Sequencer/LayerMixingFilterDataJsonSerializationTests.cs` 126, `Analysis/BeatBarSettingsDataTests.cs` 36. Names and assertions: `Sequencer/LayerEditorLayerServiceTests.cs` 221, `Sequencer/LayerEditorViewModelTests.cs` 150, `Sequencer/MarkCollectionImportCommitterTests.cs` 164, `Sequencer/MarkCollectionImportSelectionViewModelTests.cs` 119, `Analysis/QmBarBeatTrackTests.cs` 159. Names only: `Effects/MarkCollectionSelectionServiceTests.cs` 181, `Effects/LipSyncMarkCollectionNameConverterTests.cs` 192, `Effects/BaseEffectMarkCollectionSelectionTests.cs` 279, `Effects/DissolveMarkCollectionSelectionTests.cs` 34, `Effects/WaveAndLiquidMarkCollectionSelectionTests.cs` 236.

**Real data inspected, read-only:** `/Users/developer/Documents/Vixen 3`: `Sequence/Zoom.tim`, `Sequence/Snoopy Christmas.tim`, `SystemData/ModuleStore.xml`. No `.v3m`, `.v3l`, `.pgo`, `user_dictionary` or `Module Data Files/LipSync` exists there.

**This repo's code read to avoid re-specifying it:** `crates/vixen-analysis` (`settings.rs`, `beats_and_bars.rs`, the head of `samples.rs`, the test list), `crates/vixen-core/src/marks.rs`, the API of `crates/vixen-core/src/layer.rs`, parts of `crates/vixen-io/src/sequence.rs`, `crates/vixen-io/src/module_store.rs`, `crates/vixen-session/src/timeline/marks.rs`, `crates/vixen-session/src/dialogs/layers.rs`, `crates/vixen-session/src/dialogs/face.rs`, `crates/vixen-session/src/undo.rs`, `crates/vixen-session/src/sequencer/layers.rs`, `crates/vixen-session/src/sequencer/papagayo.rs`, `crates/fx-audio-state/src/lip_sync.rs`, `app/Sources/VixenDialogs/EntryPoints.swift`. Other workers were changing `crates/vixen-session` and `crates/vixen-io` during this read, so citations into them go by function name, and what exists there should be re-checked before relying on this spec's account of it.

**In scope and not read fully:**

- `QM/QMFiles/dsp/**`, `QM/QMFiles/maths/**`, `QM/QMFiles/base/**`, `QM/QMFiles/ext/**` and the vendored `QM/QMFiles/vamp-plugin-sdk-2.10/**`, about 37,000 lines. The brief asks how BeatsAndBars *uses* QMLibrary, and `crates/vixen-analysis` is already a port pinned against this C++, so the DSP itself was not re-read. The plugin (`BarBeatTrack.cpp`) and the wrappers were. `QM/WrapperFiles/ManagedRealtime.cpp` 124 was read to line 60, which includes `totalMilliseconds`. `QM/WrapperFiles/ManagedParameter.h`, `ManagedOutput.h` and `QM/QMFiles/BarBeatTrack.h` were skimmed: data holders only.
- `QM/VampParamCtrl.h` 253, `QM/VampOutputCtrl.h` 133: unused WinForms controls; class headers only.
- `LS/Resources/standard_dictionary` 133,910 lines: header and sample lines read, and every line analysed by script for its tokens.
- `*.resx`: images and strings, not read, apart from `LS/LipSyncResources.resx`'s one entry. `TSE/BeatDetection.resx` and `LS/LipSyncColorEditor.resx` have no matching class.
- `Vixen.Modules/App/Marks/Annotations.cs` 57, `LE/Converters/DebugConverter.cs` 45: tooling stubs, skimmed; no behaviour.
- `LS/LipSyncResources.Designer.cs` 73: generated accessor; not read.

**Read by the independent audit of this spec, on top of the above.** Whole files: `Vixen.Modules/LayerMixingFilter/ChromaKey/ChromaKeyData.cs` 58, `LumaKey/LumaKeyData.cs` 30, `MaskFill/MaskAndFillData.cs` 26, `Vixen.Common/Controls/TimeControl.cs` 537, `Vixen.Core/Module/ModuleLocalDataSet.cs` 34, `Vixen.Modules/App/TimingTrackBrowser/Model/ExternalVendorInventory/*.cs`. Ranges: `Vixen.Core/Module/ModuleDataSet.cs` (of 348: 25-110, 164-172, the `DataModels` property), `Vixen.Core/Sys/VixenSystem.cs` (of 469: 100-160), `Vixen.Application/VixenApplication.cs` (of 1384: 391-407), `Vixen.Modules/Sequence/Timed/TimedSequenceMigrator.cs` (of 1132: 695-800), `Vixen.Modules/Effect/LipSync/LipSync.cs` (of 895: 30-110, 112-245), `Vixen.Common/Controls/TimeLineControl/Waveform.cs` (40-60, 130-175, 225-300), `Vixen.Common/WPFCommon/Utils/TextBoxExtender.cs` (of 297: 29-212), `Menu.cs` (426-436), `Form.cs` (262-290, 335-350, 648-658, 1755-1770, 1870-1880, 1985-1995, 4024-4050, 6076-6090), and the designers in the list above for sizes and default buttons. Real data: the layer surrogates of all 83 sequences in the owner's show, and the whole LipSync module of `SystemData/ModuleStore.xml`. No screenshots.

The screenshots at vixenlights.com were not opened. Two text pages were fetched, `usage/sequencer/layers/` and `usage/sequencer/layers/layer-editor/`, and neither says which filter a new layer gets (F56).

## Features

Conventions. ⌘ is upstream Ctrl. A citation starting with a colon continues the last file named in that entry. Treatment tags follow the sibling specs and `docs/PORTING_GUIDE.md`: **KEEP** means port as written and comment that it is deliberate; **GUARD** means upstream throws or crashes, so do the stated safe thing instead; **FIX** is a proposed divergence, listed for the owner's veto. "Modified" means the sequence's dirty flag is set and the title shows it (`SequenceModified`, sequencer-shell F22). "No undo" means the action never reaches the undo stack upstream.

### A. Mark Collections panel

- [ ] **F1. Mark Collections panel.** A dock panel titled "Mark Collections" with no close box of its own (`ControlBox = false`), initial client size 192x356, opened docked right with the sequencer (`TSE/Forms/Form_Marks.Designer.cs:40-46`, `Form.cs:654`). Toggled by View > Mark Window (`TSE/TimedSequenceEditorForm.Designer.cs:956`, `Menu.cs:398`, F114, sequencer-shell F69, F328): that menu wording stays, though the panel's title is "Mark Collections". The panel shows the live `_sequence.LabeledMarkCollections` list; there is one docker view model per panel, created when the panel is created (`TSE/Forms/Form_Marks.cs:27`). Top-down layout: toolbar, then a scrolling table with a fixed header row and one row per collection, in list order (`Docker/Views/MarkDockerView.xaml:21-106`). Background of the table is black behind the rows (`:37`). Both scrollbars appear only when needed (`:37`). Row order is list order and there is no way to reorder collections anywhere in the docker. Layout is saved with the dock layout (`Form.cs:271-272`); the marks themselves are saved in the sequence file.

- [ ] **F2. Keys typed in the panel.** Any key pressed while focus is in the panel and not in a text box is forwarded to the sequencer's hotkey handler, so timeline hotkeys keep working; a key in a text box stays there and normal editing is untouched (`TSE/Forms/Form_Marks.cs:43-52`). On the Mac the panel does not consume keys it does not use, so they travel up the responder chain to the sequencer window's key handler.

- [ ] **F3. Clicking into the panel takes keyboard focus even when it has no rows.** Upstream does this with a hidden zero-size `target_button` and a synthetic ⇧Tab (`TSE/Forms/Form_Marks.cs:59-66`, `Docker/Views/MarkDockerView.xaml:33`). Keep the outcome: the panel accepts first responder when clicked, with no rows. The mechanism is listed in Windows-only omissions.

- [ ] **F4. Toolbar: New** (tooltip "New", icon `add_Collection.png`, `Docker/Views/MarkDockerView.xaml:24-26`). Always enabled. Appends one collection at the end named by `GetUniqueName("Mark Collection", collections)`, which gives `Mark Collection`, then `Mark Collection - 2`, `- 3`... (case-insensitive, trimmed; F50). If the list was empty the new one is the default (active) collection (`MDVM:94-103`). Defaults from the constructor: `Level` 1, `ShowGridLines` true, `ShowTailGridLines` false, `ShowMarkBar` false, `Locked` false, `CollectionType` Generic, decorator white, not bold, not solid, new random `Id`, `LinkedMarkCollectionId` empty, no marks (`MC.cs:26-39`, `MD.cs:14-19`). Modified, through the collection-changed handler (`Form.cs:5649-5667`). No undo.

- [ ] **F5. Toolbar: Import** (tooltip "Import", icon `folder_go.png`, `Docker/Views/MarkDockerView.xaml:27-29`). Always enabled. Runs the import flow, section B.

- [ ] **F6. Toolbar: Export** (tooltip "Export", icon `folder_open.png`, `Docker/Views/MarkDockerView.xaml:30-32`). Enabled only while at least one collection exists (`MDVM:218-221`). Runs the export flow, section C.

- [ ] **F7. Column headers.** Six columns, left to right, sharing widths with the rows (`Docker/Views/MarkDockerView.xaml:45-52`, `:57-100`):

  | # | Header content | Tooltip | Click |
  |---|---|---|---|
  | 1 | Icon `Pencil.png` | "Active Collection" | none |
  | 2 | Icon `MarkLineVisible.png` | "Show Mark Lines" | F8 lines |
  | 3 | Icon `MarkTailLineVisible.png` | "Show Tail Mark Lines" | F8 tail lines |
  | 4 | Icon `MarkBarVisible.png` | "Show Mark Bar" | F8 bar |
  | 5 | Icon `Resources/locked.png` | "Lock Access" | F8 lock |
  | 6 | Text "Collection" | none | none |

  The last column takes all remaining width. Keep Vixen's `MarkLineVisible`, `MarkTailLineVisible` and `MarkBarVisible` images, since they are distinctive; the `pencil` and `lock` SF Symbols say the same as the other two. The click fires on left button up anywhere in the header cell (`:64-68`).

- [ ] **F8. Click a column header to set that flag on every collection.** Each header keeps its own next-value flag. The flag is **not** derived from the rows at click time; it is computed once when the panel is created (and again whenever the panel is rebuilt, F114) and then flipped by each click (`MDVM:17-40`, `:242-246`, `:265-269`, `:288-292`, `:320-324`):

  | Header | Sets | Flag starts at | If every collection already has it on at panel creation |
  |---|---|---|---|
  | Lines | `ShowGridLines = flag` | true | false |
  | Tail lines | `ShowTailGridLines = flag` | false | true |
  | Bar | `ShowMarkBar = flag` | true | false |
  | Lock | `Locked = flag` | true | false |

  Every click sets all collections to the flag, then flips it. `All(...)` on an empty list is true, so opening a sequence with no collections starts Lines, Bar and Lock at false and Tail at true (`MDVM:24-39`). Only the Bar header has an enable rule, at least one collection (`MDVM:298-301`), and `DisableAssociatedObjectOnCannotExecute="False"` means the header never greys (`Docker/Views/MarkDockerView.xaml:84`). So on an empty list a Bar click does nothing and leaves its flag alone, while a Lines, Tail lines or Lock click has nothing to set but still flips its flag. **KEEP** for Lines, Bar and Lock: with rows in mixed states a click applies the remembered next value, not the inverse of what is showing. **Quirk, tail lines:** the tail flag starts inverted compared with the others. With no tail lines on anywhere, the first click sets everything to false (no visible change) and only the second turns them on; with all on, the first click sets all on (no change) and the second turns them off. **FIX** (proposed): derive the tail flag's first value the same way as the others (start true; false if all on). Veto keeps it as written. Each collection that actually changes raises a property change, so the sequence is modified only if at least one value changed (`MC.cs:62`, `Form.cs:5644-5647`). No undo.

- [ ] **F9. Row: Active radio.** Radio bound to `IsDefault`, one group across all rows (`GroupName="Active"`), so choosing one clears the others (`Docker/Views/MarkCollectionView.xaml:27-29`). The active collection is where new marks go (sequencer-shell F297). There is no way to have the radio on two rows from the panel, but the model does not enforce one: imports and upgrades normalise it themselves (F36, and the old-format conversion in Data and persistence). A row can show no radio selected if no collection is flagged. Modified, per change. No undo. Saved in the sequence as `IsDefault`.

- [ ] **F10. Row: Lines checkbox.** Bound to `ShowGridLines` (`Docker/Views/MarkCollectionView.xaml:30-32`). Controls whether the collection draws grid lines in the timeline and counts as a snap source (timeline spec). Modified. No undo.

- [ ] **F11. Row: Tail lines checkbox.** Bound to `ShowTailGridLines`. **Disabled and greyed (`DarkGray`) while that row's Lines checkbox is off**; its value is retained while disabled (`Docker/Views/MarkCollectionView.xaml:33-46`). Tail lines draw a line at each mark's end time as well as its start (timeline spec). Modified. No undo.

- [ ] **F12. Row: Bar checkbox.** Bound to `ShowMarkBar`, whether the collection has a row in the marks bar above the timeline (`Docker/Views/MarkCollectionView.xaml:47-50`). Modified. No undo. A collection `IsVisible` when `ShowGridLines || ShowMarkBar` (`MC.cs:160`), which several commands read.

- [ ] **F13. Row: Lock checkbox.** Bound to `Locked` (`Docker/Views/MarkCollectionView.xaml:51-54`). What locking prevents is in timeline spec F139: marks are drawn hatched and cannot be pressed. A locked collection is also skipped by Bulk Effects Move (`Form.cs:3668-3670`). Modified. No undo.

- [ ] **F14. Row: Name.** Collection name drawn in the collection's decorator colour, min width 50 (`Docker/Views/MarkCollectionView.xaml:57-58`). The colour comes from the decorator with no contrast adjustment: a black collection on the dark background is effectively invisible upstream. **KEEP** the colour itself.

- [ ] **F15. Row: Double-click the name to rename.** Upstream detects a double-click by hand: left button **up** starts a 300 ms timer; a second left button up while it runs begins the edit (`MCVM:255-276`). On the Mac use a normal double-click. Beginning an edit: remembers the old name, fills the edit buffer with it, clears any error, swaps the text for a field with all text selected and focus in it (`Docker/Views/MarkCollectionView.xaml:125-130`, `:157-161`).

  While editing, every keystroke validates (`MCVM:291-302`), and validation re-runs if collections are added or removed meanwhile (`MCVM:31-38`). Validation trims the text; empty is treated as fine; otherwise `IsUniqueName(all, proposed, excluding self)` (`MCVM:433-451`). When not unique: field border `#CC3B2E`, 2 px; tooltip `A Mark Collection named "<trimmed name>" already exists.`; a label beside it reading `Duplicate name` in the same colour (`Docker/Views/MarkCollectionView.xaml:126`, `:131-140`, `:151-152`, `:162-164`).

  Ending the edit, on Enter or loss of focus (`MCVM:381-405`):
  - Trimmed text empty: restore the old name, no change.
  - Duplicate: **stays in edit mode**, showing the error, even if focus has left. The user must fix it or press Escape.
  - Otherwise: if it differs from the name by exact (case-sensitive) comparison, set the name to the **trimmed** text (modified), and leave edit mode. So a case-only rename works.

  Escape restores the old name and leaves edit mode (`MCVM:424-429`). No undo. Effects select collections by id, with the name only for display, so a rename does not break them (`Vixen.Core/Marks/IMarkCollectionSelector.cs:6-8`); the effect editor's pickers refresh (effect-editor spec F65).

- [ ] **F16. Row: Context menu.** Right-click on the **name text** only (not the checkbox cells) (`Docker/Views/MarkCollectionView.xaml:59-118`). Item order: Delete, Offset Time, Solid Line, Bold, Color, Weight, Type, Linked To. Opening rebuilds the Linked To submenu (`MCVM:755-758`).

- [ ] **F17. Context menu: Delete** (icon `delete_32.png`). No confirmation. Removes the collection and all its marks (`MCVM:521-525`). If it was the default and others remain, the new default is the first visible collection (`IsVisible`), else the first (`MDVM:61-77`). Links are **not** cleaned: a collection linked to the deleted one keeps the now-dangling `LinkedMarkCollectionId`, and the Linked To menu then shows nothing checked for it, not even None (F24). Effects that referenced it are normalised by `BaseEffect` when the list changes (effect-editor spec F65, `Vixen.Core/Marks/MarkCollectionSelectionService.cs:18-50`). Modified. No undo. Use `role: .destructive` on the Mac, with no added confirmation, since upstream has none and muscle memory matters.

- [ ] **F18. Context menu: Offset Time** (icon `clock_go.png`). Numeric prompt, title "Offset Mark Time", prompt "Time to offset (in milliseconds):", default 1000, range -30000 to 30000 (`MCVM:544-555`). Enter accepts, Escape cancels (`Vixen.Common/Controls/NumberDialog.cs:24-34`). Result 0 returns 0 and changes nothing (`Vixen.Common/WPFCommon/Services/MessageBoxService.cs:108-111`). Otherwise every mark's `StartTime += ms`; durations unchanged (`MC.cs:260-266`).
  - **Quirk:** no clamp. A negative offset can put marks before time zero and a positive one past the sequence end. Nothing removes them. The clamping overload at `MC.cs:279-348` is only used by `MoveMarksInRangeByTime` (`Form.cs:3664-3687`, reached from Tools > Bulk Effects Move, sequencer-shell F92). **KEEP**, because the marks can be moved back by offsetting the other way; the timeline must not crash on negative mark times.
  - **Quirk:** this path raises no `Marks` change on the collection, and nothing watches an individual mark: the form and the timeline hook only the collection and its decorator (`Form.cs:5644-5647`, `:5686-5693`, `Vixen.Common/Controls/TimeLineControl/Grid.cs:2492-2493`, `Vixen.Common/Controls/TimeLineControl/Ruler.cs:836-837`, `Vixen.Common/Controls/TimeLineControl/MarkRow.cs:20-21`). So **Offset Time does not set Modified**, no save prompt appears for it, and the grid's lines and snap points stay at the old times until some other change rebuilds them (`Vixen.Common/Controls/TimeLineControl/Grid.cs:2549-2553`). **FIX** (proposed): mark Modified and publish the normal change. Veto keeps it as written.
  - No undo.

- [ ] **F19. Context menu: Solid Line.** Check item bound to `Decorator.IsSolidLine` (`Docker/Views/MarkCollectionView.xaml:76`). Default off. Modified, via the decorator handler (`Form.cs:5686-5693`). No undo.

- [ ] **F20. Context menu: Bold.** Check item bound to `Decorator.IsBold` (`Docker/Views/MarkCollectionView.xaml:77`). Default off. Grid lines draw thicker, and the ruler's draw 3 instead of 1 wide (`Vixen.Common/Controls/TimeLineControl/Ruler.cs:890`, timeline spec Q20). Modified. No undo.

- [ ] **F21. Context menu: Color** (icon `colors.png`). Opens Vixen's colour picker on the current colour; OK sets `Decorator.Color` to the picked RGB (`MCVM:493-502`). Use the system colour panel (`docs/UI_GUIDE.md`). Modified. No undo. Saved as a `System.Drawing.Color` in the sequence file; see Data and persistence for the shape.

- [ ] **F22. Context menu: Weight.** Submenu items `1` through `6`, the current one checked (`MCVM:771-778`, `Docker/Views/MarkCollectionView.xaml:83-93`). Choosing an unchecked item sets `Level` and unchecks the rest (`MCVM:677-687`). **Quirk:** choosing the already-checked item unchecks it visually and leaves `Level` alone, so the menu then shows nothing checked until the panel is rebuilt, because this list is built once per row and not on open. **FIX** (proposed): a native picker submenu where the current value stays checked. No data difference. `Level` is both the snap priority and the dash length `[Level, Level]` of non-solid lines (timeline spec, ruler marks and snap points). The model allows any int and old files can hold others, in which case no item is checked. Modified. No undo.

- [ ] **F23. Context menu: Type.** Submenu items in enum order `Generic`, `Phrase`, `Word`, `Phoneme`, `State`, the current one checked (`MCVM:762-769`, `Vixen.Core/Marks/IMarkType.cs:6-32`). Choosing an unchecked item sets `CollectionType`, unchecks the rest and rebuilds Linked To (`MCVM:646-658`). Same re-click quirk and same FIX as F22. Changing the type does **not** clear `LinkedMarkCollectionId`: going Word to Generic hides the menu but keeps the id, and it comes back if the type returns. **KEEP**. The type decides what text a mark can carry through the phoneme breakdown (sequencer-shell F299), which breakdown items the marks bar offers (timeline spec), and which collections the LipSync and State effects prefer. Modified. No undo.

- [ ] **F24. Context menu: Linked To.** Visible only when the type is linkable: Phoneme links to Word collections, Word links to Phrase collections (`MCVM:780-812`, `Docker/Views/MarkCollectionView.xaml:106`). Items: each collection of the target type in list order, by name, then `None` last. An item is checked if its id equals the stored link; `None` is checked if the link is empty. Rebuilt on open, on type change and on list change (`MCVM:31-33`, `:656`, `:757`). Choosing an unchecked item (`MCVM:706-727`):
  - A real target: first clears the link on **every** collection currently linked to that target (including this one), so one parent has at most one child; then sets this collection's link.
  - `None`: sets this link to empty; others are untouched.
  - Unchecks the other items.

  Re-clicking the checked item does nothing to the data, and unchecks it until the next open. **Quirk:** `LinkedMarkCollectionId` is a plain property that raises no change (`MC.cs:157-158`), so **changing a link does not set Modified**. **FIX** (proposed): mark Modified. Veto keeps it as written. No undo.

- [ ] **F114. View > Mark Window and View > Layer Editor Window.** Each toggles its panel: hidden or never shown, it shows docked at its side (right for marks, left for layers); otherwise it closes (`Menu.cs:396-404`, `:426-436`). Both start shown, docked right (`Form.cs:654-655`, `:342-345`). The menu's check mark reads the panel's state each time the menu opens (`Form.cs:6081-6083`). Closing really closes: the form is disposed, since neither sets `HideOnClose`, and the next open builds a new one with a new view model (`Form.cs:1075-1093`, `:1097-1112`). What that resets: F8's header flags, the checks in F22 and F23's submenus, which rows are expanded in F58, and F117's filter list. No data is lost, since it all lives in the sequence. **KEEP** the reset on the Mac: `HeaderFlags::new` runs each time the panel is shown after being hidden. Visibility persists with the dock layout. No undo. Label and order: sequencer-shell F69, F71.

- [ ] **F115. A second sequencer window.** Row view models find their list with `GetFirstOrDefaultInstance<MarkDockerViewModel>` (`MCVM:22-24`), which is the first docker alive in the app, not the row's own parent. So with two sequencer windows open, the second window's rows check duplicate names against the first window's sequence (F15), build Linked To from it (F24), and a link chosen there clears links in the first window's collections (`MCVM:712-718`). Delete is right, since it goes through `ParentViewModel` (`MCVM:523`). **FIX** (implied by one session per window): every panel uses its own sequence's collections.

### B. Import

- [ ] **F25. Import Type Selection dialog.** Toolbar Import first opens a modal fixed dialog titled "Import Type Selection", centred on its parent, client size 207x270 (`TSE/MarkCollectionImportDialog.Designer.cs`, `docs/reference/forms/Vixen.Modules__Editor__TimedSequenceEditor__MarkCollectionImportDialog.json`). Radios, top to bottom, with "Vamp Beat Tracker: Bars" preselected:

  1. Vamp Beat Tracker: Bars
  2. Vamp Beat Tracker: Beats
  3. Audacity Beats
  4. Vixen 3
  5. xTiming
  6. Papagayo Voice Tracks
  7. Singing Faces Lyric Browser
  8. Pangolin Beyond

  `OK` (default) then `Cancel` (Esc) along the bottom. Cancel ends the flow. The selection is not remembered. Make this a sheet with a `Picker(.radioGroup)`.

- [ ] **F26. Route to the importer** (`MDVM:120-160`):

  | Radio | Importer | Format entry |
  |---|---|---|
  | Vamp Beat Tracker: Bars | `LoadBarLabelsAsync` | F29 |
  | **Audacity Beats** | `LoadBarLabelsAsync` | F29 |
  | Vamp Beat Tracker: Beats | `LoadBeatLabelsAsync` | F30 |
  | Vixen 3 | `ImportVixen3BeatsAsync` | F28 |
  | xTiming | `LoadXTimingAsync` | F31 |
  | Papagayo Voice Tracks | `ImportPapagayoTracksAsync` | F32 |
  | Singing Faces Lyric Browser | `ImportSingingFacesTracksAsync` | F33 |
  | Pangolin Beyond | `ImportPangolinBeyondMarksAsync` | F34 |

  **Quirk:** "Audacity Beats" runs the *bar* label reader, not the beat reader, and "Vamp Beat Tracker: Beats" runs the *beat* reader, whose picker says "Audacity Beat Labels". **KEEP** the routing and the labels. A result other than success, or a success with no collections, ends the flow with nothing changed and no further message (`MDVM:162-165`); importer errors have already been shown by then. Otherwise the selection dialog follows (F35).

- [ ] **F27. Last folder.** All import and Vixen export pickers start in a shared last-folder value, updated after each OK. It starts at the profile data root (`Paths.DataRootPath`) and lasts for the process, in a static (`MIES:23`, `:49`, `:156`, `:255`, `:313`, `:362`, `:486`, `:597`). **Quirk:** the Audacity and Pangolin Beyond export pickers neither read nor update it (`MIES:637-640`, `:675-678`); Singing Faces has no picker. **KEEP**. Not saved between runs; on the Mac, set each panel's `directoryURL` explicitly, so the system's remembered directory does not override it.

- [ ] **F28. Vixen 3 format import** (`.v3m`). Picker filter `Vixen 3 Mark Collection (*.v3m)|*.v3m|All Files (*.*)|*.*`, default extension `.v3m` (`MIES:144-150`). Reads the text, parses XML, dispatches on the **root element's namespace** (`MIES:159-182`):
  - `http://schemas.datacontract.org/2004/07/Vixen.Marks.IMarkCollection` (the string is built from `typeof(IMarkCollection)`, which is its full name): current format; deserialize a list of `MarkCollection` with `Mark`, `MarkDecorator` (`MIES:200-208`).
  - `http://schemas.datacontract.org/2004/07/VixenModules.Sequence.Timed`: legacy; list of old `MarkCollection` (`Name`, `Enabled`, `Bold`, `SolidLine`, `Marks` as `List<TimeSpan>`, `MarkColor`, `Level`, `Id`; `Vixen.Modules/Sequence/Timed/MarkCollection.cs:33-55`), migrated (`MIES:211-233`): new id, `Name`, `Level`, `ShowGridLines = Enabled`, tail off, unlocked, decorator from colour/bold/solid, each time becoming a `Mark` with default 450 ms duration. The old `Id` is dropped.
  - Anything else: logged, error alert, failed. A null root returns failed with **no alert** (`MIES:161-164`).
  - The namespace only picks the type. The serializer then requires the exact root name, `ArrayOfIMarkCollection` or `ArrayOfMarkCollection`, and a collection with no `Marks` element throws, in `OnDeserialized` for the current format (`MC.cs:350-356`) and in the migration for the legacy one (`MIES:228`). All of these reach the alert below. **KEEP**: `read_v3m` must check for itself, because `mark_collection_from_xml` reads a missing `Marks` as empty.

  Any exception: error alert "There was an error importing the Vixen Marks." titled "Vixen Marks Import Error" (`MIES:192-197`, `:235-239`). Candidates keep the ids, names, flags, level, type, link and default flag from the file. So a `.v3m` imported into the sequence it came from has **the same ids as existing collections**: **Quirk:** nothing prevents two collections with one id. **KEEP**, with a note in tests; id lookups use the first match.

  Rust: `mark_collection_from_xml` in `crates/vixen-io/src/sequence.rs` already reads one element. A wrapper for the document root is new (see Rust API).

- [ ] **F29. Audacity bar labels import** (`LoadBarLabelsAsync`, used by "Vamp Beat Tracker: Bars" and "Audacity Beats"). Picker filter `Audacity Bar Labels|*.txt|All Files|*.*` (`MIES:243-249`). Algorithm (`MIES:258-290`):
  1. Remove all `\r`, split on `\n`, drop empty lines. No lines: success with no collections (flow ends silently).
  2. One collection, name `Audacity Marks`, colour `Yellow`. Everything else is the constructor's: grid lines on, bar off, level 1, Generic (`MIES:345-352`).
  3. Per line: if it contains a tab at a position greater than 0, split on tab; otherwise trim and split on single spaces. Field 0 start seconds, field 1 end seconds (default `"0"`), field 2 text (default empty). Each trimmed. **Quirk:** fields past the third are dropped, so label text with a tab is cut; with space splitting, only the first word of a label survives, and double spaces shift the fields. A line of only spaces survives step 1 and fails the import, because `Convert.ToDouble("")` throws; so does a line that starts with a tab and holds another tab, since `IndexOf` is then 0 and it takes the space path.
  4. `Convert.ToDouble` uses the machine's **current culture**. `TimeSpan.FromSeconds` of each. **GUARD**: parse with the invariant culture on the Mac. A European-locale Windows machine reads `1.5` differently upstream; files are written by Audacity with a period.
  5. `Mark(start)` with `Duration = end > 0 ? end - start : 0`, `Text`. The mark setter raises a duration below 25 ms to 25 ms (`Mark.cs:62-65`), so point labels (start = end, duration 0) become 25 ms marks and a 0 `end` also gives 25 ms.

  Any exception (a bad number, for instance): error alert "There was an error importing the Audacity bar marks." titled "Audacity Import Error" (`MIES:292-296`).

- [ ] **F30. Audacity beat labels import** (`LoadBeatLabelsAsync`, used by "Vamp Beat Tracker: Beats"). Picker filter `Audacity Beat Labels|*.txt|All Files|*.*` (`MIES:301-307`). Algorithm (`MIES:316-336`):
  1. Empty file: success with no collections.
  2. Whole-file regex `(\d*\.\d*)\s(\d*\.\d*)\s(\d)`, per match group 1 start seconds, group 2 unused, group 3 a single-digit beat number. Lines that do not match are silently ignored. Group 3 takes only the **first digit** of the third column.
  3. `numberOfBeats` = maximum group 3 **compared as strings**, then converted (single digits, so the same as numeric). Collections are made for 1..numberOfBeats, named `Audacity Beat <n> Marks`, colours by index `Yellow, Gold, Goldenrod, SaddleBrown, CadetBlue, BlueViolet`, the rest as F29's defaults.
  4. Each match adds `Mark(start)` (default 450 ms) to collection `beat-1`.

  **No match at all is not an error.** LINQ's `Max` over strings returns null for an empty sequence rather than throwing, and `Convert.ToInt32(null)` is 0 (`MIES:324`). So a non-empty file with nothing that matches makes no collections and succeeds, and the flow ends with no message (F26). **KEEP**.

  Errors alert "There was an error importing the Audacity beat marks." titled "Audacity Import Error" (`MIES:338-342`), including these throwing cases, all of which keep the same outcome: **more than six beats** (colour index out of range, so 7, 8 or 9 fail the whole import), a beat number of **0** (`beat-1 = -1`), and a first column that is a bare `.`, which the pattern matches and `Convert.ToDouble` refuses. **GUARD** in Rust: return the same error, never panic. Parse numbers with the invariant culture.

- [ ] **F31. xTiming import.** Picker: default extension `.txt`, filter `xTiming|*.xTiming|xTiming xml|*.xTiming.xml|All Files|*.*` (`MIES:356`). XML parsed from the file's text. Errors alert "There was an error importing the xTiming marks." titled "xTiming Import Error" (`MIES:370-374`).

  Materialising (`MIES:377-450`), shared with F33:
  1. Timing nodes: `/timings/timing` if `/timings` exists, else the single `/timing`, else none. A document with neither is a success with no collections, so the flow just ends.
  2. Per `timing`: `name` attribute, default `xTiming`. Effect layers: its `EffectLayer` children. `lipSyncTrack` = **more than one** effect layer.
  3. Per layer, `layerNumber` starts at 1: new collection, colour `Brown`, name `<name> - <layerNumber>`, `ShowMarkBar = true`. If `lipSyncTrack`, configure by layer number (`MIES:452-477`): 1 is `Phrase` named `<name> - Phrase`; 2 is `Word` named `<name> - Word` linked to the last candidate so far; 3 is `Phoneme` named `<name> - Phoneme` linked to the last candidate. Layers past 3 stay Generic with the numbered name.
  4. Per `Effect` child: attributes `starttime`, `endtime` (milliseconds), `label`. Skipped if either time is missing or their **strings** are equal. Otherwise `Mark(start)`, `Duration = end - start` (25 ms minimum via the setter), `Text = label` (null when absent, see below).
  5. **A layer with no marks is dropped and does not advance `layerNumber`.** So in a 3-layer file with an empty first layer, words become `- Phrase`. **KEEP**.

  `Convert.ToDouble` uses the current culture: **GUARD**, invariant on the Mac. A null `Text` from a missing `label`: Rust's text is a `String`, so store empty; the `.v3m` writer then writes an empty element rather than a nil one. `AddMark` sorts after every addition, so a large layer is quadratic upstream; see Performance hazards.

- [ ] **F32. Papagayo Voice Tracks import (to mark collections).** Picker `Papagayo files (*.pgo)|*.pgo|All files (*.*)|*.*`, filter index 1 (`MIES:481`). Reads bytes, decodes as a text stream with BOM detection, parses with the Papagayo reader (F87, `TSE/PapagayoDoc.cs`). File name without extension is `fileName`. Per voice, three collections in this order (`MIES:533-561`):

  | Collection | Name | Colour | Type | Link |
  |---|---|---|---|---|
  | Phrases | `<fileName> <voice> Phrases` | RGB(205,242,162) | Phrase | none |
  | Words | `<fileName> <voice> Words` | RGB(242,205,162) | Word | Phrases |
  | Phonemes | `<fileName> <voice> Phonemes` | RGB(235,185,210) | Phoneme | Words |

  All `ShowMarkBar = true`; other flags default. Marks: per phrase `Mark(StartMS)`, duration `DurationMS`, text = phrase text; per word in the phrase `Mark(StartMS)`, duration `EndMS - StartMS`, text = word text; per phoneme in the word `Mark(StartMS)`, duration `EndMS - StartMS`, text = `TypeName`. All times are `double` ms, all durations clamp to 25 ms min. **Then, before the selection dialog,** shows an information alert titled "Papagayo Import", text `<n> voices imported as Mark Collections\n\nRow #1 - <voice>\nRow #2 - <voice>...` (`MIES:563-569`). **Quirk:** the alert says "imported" before the user has selected anything and can still cancel. **KEEP** the alert's wording and order. Errors: "There was an error importing the Papagayo marks." titled "Papagayo Import Error" (`MIES:504-508`). This flow is separate from Tools > LipSync > Papagayo Import, which makes effects on the clipboard (sequencer-shell F91).

- [ ] **F33. Singing Faces Lyric Browser import.** Opens the `VendorInventoryWindow` (section D). If it closes with a selected song, downloads the song's timing XML and materialises it as F31 (`MIES:511-531`). Errors: "There was an error importing the Singing Faces timing marks." titled "Singing Faces Import Error". Cancel: cancelled, nothing changes.

- [ ] **F34. Pangolin Beyond import.** Picker filter `Pangolin Beyond CSV (*.csv)|*.csv|All Files (*.*)|*.*`, default extension `.csv` (`MIES:37-43`). Parses the whole file first, then prompts; nothing changes until commit (`MIES:28-33`). Parser, `Docker/Services/PangolinBeyondMarkParser.cs:10-61`:
  1. Null or empty text fails, "The Pangolin Beyond CSV file is empty."
  2. Normalise `\r\n` and `\r` to `\n`, split on `\n`; one trailing empty line is dropped.
  3. Line 1 must equal exactly `#,Name,Start,Color` (ordinal), else fails, "Line 1 must be the Pangolin Beyond header '#,Name,Start,Color'."
  4. Each further line, numbered from 2, splits on `,` and must have exactly 4 fields, else "Line <n> must contain exactly four comma-separated columns." So a blank line in the middle, and any quoted comma, fail the import. Column 1 (`M1`...) is ignored. Column 2 is the text, not trimmed, can be empty.
  5. Column 3 parses with `TimeSpan.TryParseExact`, invariant, formats `mm\:ss\.fff` then `hh\:mm\:ss\.fff`, else "Line <n> has an invalid start time '<value>'." Exactly 3 fraction digits, 2-digit fields, and each field in range: `hh` 00 to 23, `mm` and `ss` 00 to 59, so `61:00.000` is invalid.
  6. Column 4 must be exactly 6 hex digits, read as **BGR** (first pair blue, second green, third red), else "Line <n> has an invalid BGR color '<value>'."

  The parser's message is only logged. The user sees "There was an error importing the Pangolin Beyond marks." titled "Pangolin Beyond Import Error" (`MIES:53-58`, `:131-136`). Exceptions reading the file show the same (`MIES:91-96`).

  Choosing the arrangement (`MIES:60-89`, `:99-114`): if the records hold **two or more distinct colours**, ask "Create a Mark Collection for each Beyond color?" titled "Pangolin Beyond Import", Yes/No/Cancel (on the Mac: "One for Each Color", "Single Collection", "Cancel"). Yes groups by colour; No gives a single collection and **then opens the colour picker**, cancelling that cancels the import; Cancel cancels. Zero or one distinct colours go straight to group by colour with no prompt.

  Building collections (`Docker/Services/PangolinBeyondMarkCollectionFactory.cs:15-55`):
  - Group by colour: one collection per colour in order of first appearance, named `Beyond Marks - #RRGGBB` (upper-case hex), colour that colour.
  - Single: named `Beyond Marks`, colour the one picked.
  - Each: `ShowMarkBar = true`, `Mark(start)` with default 450 ms, `Text`. A header-only file has zero records: no colour prompt, group by colour yields **no** collections, and the flow ends silently.

- [ ] **F35. Select Imported Mark Collections dialog.** Modal, 600x400, tool window style, centred on screen, titled "Select Imported Mark Collections", Esc closes (`Docker/Views/MarkCollectionImportSelectionWindow.xaml`). Top text: "Select the imported Mark Collections to add to this sequence." A list with one row per candidate in source order: checkbox (not focusable, all start checked), the name, and when the name collides with one already in the sequence a caution icon `caution-icon.png` with tooltip "Collection name already exists in this sequence. This will be renamed during import." (`Docker/ViewModels/MarkCollectionImportOptionViewModel.cs:9-18`, `:45`, `Docker/Views/MarkCollectionImportSelectionWindow.xaml:40-60`). Collision is `IsUniqueName`, so it ignores case and surrounding space, and a blank candidate name counts as a collision and shows the icon (`NameSvc:22-26`). It is computed once against the sequence, not between candidates.
  - The first row starts selected. **Space** toggles the selected row's checkbox (`Docker/Views/MarkCollectionImportSelectionWindow.xaml:36-38`, `Docker/ViewModels/MarkCollectionImportSelectionViewModel.cs:82-88`).
  - OK (default) is disabled when nothing is checked (`Docker/ViewModels/MarkCollectionImportSelectionViewModel.cs:54`, `:64`, `:90-94`). Cancel closes with no change (Esc too). Button text is "OK" and "Cancel", bottom right (`Docker/Views/MarkCollectionImportSelectionWindow.xaml:21-22`).
  - Selected candidates are kept in source order (`Docker/ViewModels/MarkCollectionImportSelectionViewModel.cs:73-76`).

- [ ] **F36. Commit an import.** (`Docker/Services/MarkCollectionImportCommitter.cs:8-72`.) Candidates in order; nothing to do if none. Three passes, in this order:
  1. **Links.** The valid set is target ids plus **selected** candidate ids. A candidate linked to anything outside it has its link set to empty. So deselecting the Phrases of a Papagayo voice leaves its Words unlinked. `Guid.Empty` is not in the set, so an empty link stays empty.
  2. **Default.** If the sequence already has a default, every candidate's default flag is cleared. Otherwise the default becomes the first candidate flagged default, else the first visible, else the first; all the rest are cleared.
  3. **Append.** Each candidate gets `GetUniqueName(name, target)` (against the growing list), so duplicates among candidates also get ` - 2` etc., and is appended.

  One collection-changed event per candidate; each marks Modified. No undo. The candidates' effect on effect pickers follows F17's normalisation. A blank name from a file throws in `GetUniqueName` (`NameSvc:47-50`), which is uncaught in this path. **GUARD**: use `Mark Collection` as the desired name.

- [ ] **F116. How every import reads its file.** `.v3m`, both Audacity readers, xTiming, Pangolin Beyond and `.v3l` use `File.ReadAllTextAsync`; Papagayo uses a `StreamReader` over the bytes (`MIES:52`, `:159`, `:258`, `:316`, `:365`, `:489-493`, `LIES:78`). All of them detect and strip a UTF-8, UTF-16 or UTF-32 byte order mark, and otherwise read UTF-8 with bad bytes replaced by U+FFFD rather than failing. So a file Notepad saved with a BOM works, and a stray Latin-1 byte in a label survives as a replacement character. **KEEP**, through one helper in Rust (`read_import_text`), not `read_to_string`, which fails on bad bytes and keeps the BOM.

  Numbers through `Convert.ToDouble` take `NumberStyles.Float | AllowThousands`: surrounding spaces, an exponent and a thousands separator are all fine, so with the invariant culture `1,234.5` is 1234.5, where `f64::from_str` refuses it. `TimeSpan.FromSeconds` and `FromMilliseconds` truncate to a whole tick, as `TimeSpan::from_secs` and `from_ms` in `vixen_core` do, and throw on NaN or overflow, which lands in that importer's error alert. **GUARD**: the same alert, never a panic.

### C. Export

- [ ] **F37. Export Mark Collections dialog.** Modal, 600x400, tool window style, centred, titled "Export Mark Collections", Esc closes (`Docker/Views/MarkExportWindow.xaml:9-11`). Left half: bold "Select Mark Collections To Export", then a scrolling table over black. Right half: bold "Export Format" with radios `Vixen` (default), `Audacity`, `Pangolin Beyond`, in that order (`:73-76`). Bottom right: `Ok` (default) and `Cancel`. Upstream spells it `Ok` here only, and `OK` everywhere else; use `OK`.
  Columns (`:53-62`): `select-forecolor.png` tooltip "Include Collection"; `text-forecolor.png` tooltip "Include Mark Text", **visible only when Pangolin Beyond is selected** (`Docker/ViewModels/MarkExportWindowViewModel.cs:99-107`, `Docker/Views/MarkCollectionExportRowView.xaml:24-25`); then "Collection" with the name in the collection's colour. Rows start all included (`Docker/ViewModels/MarkCollectionExportRowViewModel.cs:12`), and include text defaults to on for Phrase, Word, Phoneme and State types, off for Generic (`:14`). OK with none included still runs the export of an empty list (F38-F40). No state is kept between openings.

- [ ] **F38. Export as Vixen** (`.v3m`). Save panel default extension `.v3m`, filter `Vixen 3 Mark Collection (*.v3m)|*.v3m|All Files (*.*)|*.*`, in the last folder (`MIES:590-597`). Writes a list of the selected collections in the order shown, with `DataContractSerializer` configured as F28, indented with tabs (`MIES:598-612`). Exact shape in Data and persistence. All fields are written, so a re-import brings back every flag, the id and the link. A serialization or write exception is **logged and nothing is shown** (`MIES:614-617`). **FIX** (proposed): show an alert, so a failed export is not silent. The busy indicator shows during the export (`MDVM:205-210`), meaningless on modern hardware; omit unless it takes long.

- [ ] **F39. Export as Audacity.** Built **before** the save panel opens (`MIES:622-643`). `iMarkCollection` starts 0. For each collection, in order: increment, then for each mark add a line `<start>\t<start>\t<i>`, where both times are the mark's start in seconds formatted `"0000.000"` (the culture's decimal separator: **GUARD**, invariant), and `<i>` is `iMarkCollection`. If exactly **one** collection is being exported, `iMarkCollection` also increments after each mark, so the label counts marks (1, 2, 3...) and, per the source comment, that one is treated as bars (`:578-580`). With two or more, the label is the collection's 1-based position and marks are treated as beats.
  Lines are then sorted as plain strings (`OrderBy(mark => mark)`), so ties in time order by `<i>` as text (`10` before `2`), and written with `WriteAllLines`, whose line ending is the platform's: `\r\n` on Windows. **KEEP** `\r\n`, so that a file is byte-for-byte what a Windows export gives. All lines have one shape, so byte order equals upstream's culture order. The fixed-width `0000.000` makes this a time sort up to 9999 s. A time from Offset Time (F18) can be negative: `-0001.000`, which sorts by its `-`. Mark end time and text are never written. Number format: .NET's custom format takes 15 significant digits and then rounds half up, which differs from exact-binary rounding at sub-millisecond marks; pin with a test. Picker: default extension `.txt`, filter `Audacity Marks (*.txt)|*.txt|All Files (*.*)|*.*`. **Quirk:** this picker has no initial directory and does not update the last folder. Errors are not caught upstream. **GUARD**: show an alert.

- [ ] **F40. Export as Pangolin Beyond.** Built before the save panel (`MIES:645-681`). For each collection, colour = decorator colour as `BBGGRR` upper-case hex (`MIES:652`, `:691-694`). For each mark: text = `mark.Text` with `,` replaced by a space if include text is on, else empty; record (start, text, colour). Records are sorted by start time with a **stable** sort, ties staying in collection order. Output lines: header `#,Name,Start,Color`, then `M<n>,<text>,<time>,<colour>` numbered from 1. Time is `hh:mm:ss.fff` when the hours component is above 0, otherwise `mm:ss.fff`, with milliseconds truncated, not rounded, and no sign for a negative time. **Quirk:** hours past 23 drop into days and are not shown; a sequence that long is not realistic. A newline inside mark text would break the file: **KEEP** (the importer then fails cleanly). Filter `CSV Files (*.csv)|*.csv|All Files (*.*)|*.*`, default extension `.csv`; no initial directory; line endings as F39. Exceptions: **GUARD**, alert. Round trip with F34: `Beyond Marks` group-by-colour names come back with `#RRGGBB`, the duration is lost (default 450 ms on import), and text keeps its spaces.

### D. Singing Faces Lyric Browser (`Vixen.Modules/App/TimingTrackBrowser/`)

- [ ] **F41. Song Inventory window.** Modal, 800x600, centred on owner, titled "Song Inventory" (`ViewModels/VendorInventoryWindowViewModel.cs:63`, `Views/VendorInventoryWindow.xaml:9`). Empty status bar at the bottom. A vertical splitter divides a left pane (1 part) and a right pane (2 parts) (`Views/VendorInventoryWindow.xaml:24-27`, `:116`). Left pane, top to bottom: `Provider` label and combo of inventories by vendor name (`:39-40`), whose change swaps the tree and leaves the Song Info tab on the last song chosen; the category tree (F43); `Name` label with the vendor name; `Website` label with a link whose text is the vendor name and which opens the vendor's site in the browser (`:53-60`). Right pane: hidden until a song is selected (F44).

- [ ] **F42. Loading the inventories.** On open, without blocking the window (`ViewModels/VendorInventoryWindowViewModel.cs:34-60`):
  1. GET `https://app.vixenlights.com/singing-faces-vendor.json`, a JSON array of `{ "Name": ..., "Url": ... }` (`:329-355`). Failure: error "Unable to retrieve singing faces vendor list. Ensure you have an active internet connection." titled "Error Retrieving Vendors", and the window stays open and empty.
  2. For each vendor, GET its URL and parse the inventory (format below), adding it to the combo. A failure on one vendor: error `Unable to retrieve inventory from <vendor name>\nEnsure you have an active internet connection.` titled "Error Retrieving Inventory", and the loop continues.
  3. The first inventory becomes selected.

  Inventory XML (`Model/ExternalVendorInventory/*.cs`, `Import/XLights/XTimingLyricInventoryImporter.cs:12-16`): root `musicinventory`; `vendor` with `name`, `website`, `logolink`; `categories` holding `category` with `id`, `name`, nested `categories`; `music` holding `song` with `artist`, `title`, `creator`, `download`, `categoryid`, `weblink`.

  Mapping (`Import/XLights/XTimingSongInventoryMapper.cs:15-140`): artist, category id, creator and title are trimmed; `weblink` and `download` become absolute URLs or are left unset with a log line when they do not parse; a song that throws is logged and skipped. Songs group by category id; a song whose id matches no category never appears. Each category's songs are sorted by title ascending (current culture, `Model/InternalVendorInventory/Song.cs:96-104`); categories keep file order. A missing nested `categories` element is treated as empty. **GUARD** (upstream would throw there and report the whole vendor as failed, if `XmlSerializer` leaves the list null). These do fail that one vendor with step 2's message, **KEEP**: a malformed vendor `website` or `logolink`, because they go through `new Uri`, not `TryCreate` (`Import/XLights/XTimingSongInventoryMapper.cs:29-45`), and a missing `vendor`, `music` or top-level `categories`, which throw on the null (`:19-22`).

- [ ] **F43. Category tree.** Hierarchy of categories, each showing its child categories first and then its songs (`Views/SongInventoryTree.xaml:25-47`, `Common.WPFCommon` composite collection). Songs appear as black hyperlink-style text with no underline; **clicking a song selects it** (`:43`). Use VixenKit's `OutlineView`; on dark appearance use the normal label colour rather than black.

- [ ] **F44. Song Info tab.** Appears once a song is selected, in a tab titled "Song Info" that is selected at that moment (`ViewModels/VendorInventoryWindowViewModel.cs:195-202`, `Views/VendorInventoryWindow.xaml:65`, `:73`). Grid of rows (`:93-108`): `Song:` a link showing the title which opens `SongLink` in the browser, doing nothing if unset (`ViewModels/VendorInventoryWindowViewModel.cs:239-245`); `Title:`; `Artist:`; `Creator:`; a **Select** button. Beneath: "All song links and timing are provided by the contributing project and are not validated by the Vixen Team." (`Views/VendorInventoryWindow.xaml:110`). Select closes the window with success (`ViewModels/VendorInventoryWindowViewModel.cs:307-313`). `IsSongValid` is computed but nothing binds to it, so **Select stays enabled even when the song has no download link**; the import then parses an empty string and shows the F33 error (`ViewModels/VendorInventoryWindowViewModel.cs:317-327`, `MIES:518-526`). **KEEP**. There is no OK button, and closing the window cancels. `SendEmailCommand` and `VendorMetadataView` have no route to the screen (dead code, `ViewModels/VendorInventoryWindowViewModel.cs:273-288`, `Views/VendorMetadataView.xaml`).

  Network rule: every fetch has a 20 second timeout and the user agent `Mozilla/5.0 (compatible; Vixen/1.0)` (`Vixen.Common/WPFCommon/Services/DownloadService.cs:58-66`), and there is no retry. A timeout is a failure like any other. Tests must not touch the network (`docs/UI_GUIDE.md`): make the fetcher an injected trait.

### E. Create Evenly Divided Marks

- [ ] **F45. Create Evenly Divided Marks dialog.** Tools > Create Evenly Divided Marks (sequencer-shell F82 has the command's logic; this is the dialog). Modal, fixed, centred on parent, titled "Create Evenly Divided Marks" (`docs/reference/forms/Vixen.Modules__Editor__TimedSequenceEditor__CreateEvenMarksForm.json`). Fields top to bottom: `Start Time` (time control), `End Time` (time control), `Number of Divisions` (integer stepper, minimum 1, maximum 10000, so it starts at 1). Buttons `OK` (default) and `Cancel` (Esc). Start defaults to the sequence's previous playback start or zero; End to the previous playback end or zero (`TSE/CreateEvenMarksForm.cs:8-16`, `Menu.cs:635`). Both are capped at the sequence length (`TSE/CreateEvenMarksForm.cs:13`, `:15`). Upstream sets the value before the maximum on each, and the cap only applies when a value is set (`Vixen.Common/Controls/TimeControl.cs:50-54`, `:523-533`), so a remembered time past a shortened sequence shows as it is until that field is edited. **KEEP**. The control shows and parses `mm:ss.fff` (`:12`), so a time past an hour loses its hours; sequencer-shell F394 owns the field itself. Client 307x180 (`TSE/CreateEvenMarksForm.Designer.cs:123`). Dialog validation is in the caller: no check that End is after Start here; the length test is `(end - start) / divisions` against 12 pixels of time at the current zoom (`Menu.cs:630-657`). End before Start gives a negative length, which fails that test and reopens with "Individual mark length is smaller than allowed." Retains no settings. See sequencer-shell F82 and F394 for what it creates. Built already, by the sequencer shell: `create_even_marks` and `even_marks_values` in `crates/vixen-session/src/sequencer/commands.rs`, `plan_even_marks` in `effect_ops.rs`, `EvenMarksSheet` in `app/Sources/VixenSequencer/SequencerSheets.swift`. Check it against this entry. No undo.

### F. Tapping marks

- [ ] **F46. Tap a mark while playing: ⇧Space.** Adds a mark at the current playback position through `AddMarkAtTime(TimingSource.Position, suppressUndo: false)` (`TSE/TimedSequenceEditorForm_Hotkeys.cs:112-121`, `Form.cs:3032-3096`). `TimingSource` is set when the sequence starts and cleared when it ends (`Form.cs:3431`, `:3444`), so the key works while playing or paused and does nothing when stopped. This is the **only** reachable way to tap marks in upstream. Plain Space is play or pause, ⌘Space is its variant (sequencer-shell section D). The tap goes to the collection that is both default and visible; if none, playback pauses and the prompts of sequencer-shell F297 appear, and **it does not resume**. A second tap on exactly the same time is ignored. `Locked` is not checked, so taps go into a locked active collection. Each tap is one undo step (`MarksAddedUndoAction`), Modified, dirty re-render. New marks have the default 450 ms duration and blank text.

  ⇧Space must be handled while the sequencer window has focus even when focus is in a panel (F2), since that is how it works with the panels upstream. Read the position from the transport, not from a UI clock: the owner taps by ear.

- [ ] **F47. Right-click on the ruler adds a mark**, ⌘ to fill the gap, ⇧ to prompt for a label (`Form.cs:2899`). See timeline spec F111. `FillGapTimes` is in the model (F49).

- [ ] **F48. Two dialogs that upstream cannot reach.** Nothing constructs either anywhere in `src` (searched `.cs`, `.xaml`, `.csproj`; sequencer-shell found the same), so no user has seen them. **Do not put them in a menu** unless the owner asks. They are recorded so nothing is lost; mark both "omitted, unreachable" in parity.
  - **Mark Tapper** (`TSE/MarkTapper.cs:13-84`, `TSE/MarkTapper.Designer.cs:122`, `:144`, `:161-169`). Fixed dialog titled "Mark Tapper", client 237x228, centred on parent; label "Start the sequence, then tap the panel or the\r\nspacebar in time to create marks."; a "Controls" group with Play, Pause, Resume, Stop in that order, disabled when there is no execution control or timing source; a tap panel with a border, anchored to all sides; OK (default) and Cancel (Esc). Play, Pause, Resume and Stop call the transport and set a `playing` flag (true, false, true, false). Space (form-wide key preview) records a tap **whether or not playing**; mouse down on the panel records one only when playing. A tap appends `round(position in ms)` to the result list and flashes the panel `SkyBlue` for 80 ms. Closing always stops the transport. Results are a `List<TimeSpan>`, available even after Cancel.
  - **Automatic Detection Settings** (`TSE/AutomaticMusicDetection.cs`, `docs/reference/forms/Vixen.Modules__Editor__TimedSequenceEditor__AutomaticMusicDetection.json`). Client 670x565. Ten groups `Octave 0 (16-30hz)`, `Octave 1 (32 - 64hz)`, `Octave 2 (65-104hz)`, `Octave 3 (131-247hz)`, `Octave 4 (262-494hz)`, `Octave 5 (523-988hz)`, `Octave 6 (1046-1976hz)`, `Octave 7 (2093-3951hz)`, `Octave 8 (4186-7902hz)`, `Octave 9 (8372-15804hz)`, each with twelve note checkboxes C to B, tagged 0 to 119. `Accuracy` stepper up to 1000, the user setting `MusicAccuracy`, default 50 (`TSE/Properties/Settings.settings:5-6`). `Preview Audio` toggles to `Stop` and starts or stops the audio. `OK`, `Cancel`. The frequency detection it was built for is commented out, and the 10 ms timer's body is empty (`TSE/AutomaticMusicDetection.cs:66-74`, `:78-100`), so it does nothing but keep a list of ticked notes. The feature that does detect music is Beat/Bar Detection, section I.

- [ ] **F49. Model edits used by tapping and the timeline.** Exact upstream semantics, since `crates/vixen-core/src/marks.rs` is simpler and these belong in the session layer (`crates/vixen-session/src/timeline/marks.rs` has several already; check each against this list):
  - `Mark` default duration 450 ms, default text empty (`Mark.cs:18-23`). Setting `Duration` below 25 ms stores 25 ms (`Mark.cs:60-65`). Setting `StartTime` to a different value raises any duration below 25 ms to 25 ms (`Mark.cs:43-48`). Reading a file applies the same setters, since `DataContract` uses the properties here, so a stored smaller duration comes back as 25 ms, except that a mark with start and duration both zero stays at zero, because both setters then return early (`Mark.cs:43`, `:60`). `Mark::apply_minimum_duration` in `vixen_core` does this.
  - Sort key: `StartTime`, then `EndTime` (`Mark.cs:76-85`). `sort_marks` in `vixen-core` sorts by start only, which differs for equal starts.
  - `AddMark`, `AddMarks`, `RemoveMark`, `RemoveAll`, `SwapPlaces` all re-sort and raise a change named `Marks` (`MC.cs:162-207`). `SwapPlaces` exchanges start times and durations, keeping texts in place.
  - `MarksInclusiveOfTime(start, end)`: skips marks ending before `start`, stops at the first starting after `end` (`MC.cs:215-228`). Ported already.
  - `FillGapTimes(mark)` (`MC.cs:237-258`): re-sort, find the index, then **backwards** from the previous mark: at the first mark whose end is before this mark's start, `mark.StartTime = thatEnd + 1 ms`, stop. (So a mark overlapping its predecessors skips back past them.) Then, unless it is the last, `mark.Duration = next.StartTime - 1 ms - mark.StartTime`. It does not re-sort afterwards. Duration below 25 ms still clamps to 25 ms, so it may overlap the next.
  - `OffsetMarksByTime(time)`: F18.
  - `OffsetMarksByTime(start, end, offset, maxTime)` (`MC.cs:279-348`), used only by Bulk Effects Move (F18): for each mark whose start is inside the inclusive window: if `end + offset < 0`, remove; else if `start + offset > maxTime`, remove; else shift, and if the new start is below zero clip (`Duration += StartTime; StartTime = 0`), else if the new end exceeds `maxTime`: **Quirk:** `Duration += EndTime`, which makes it *longer*. **KEEP**, as a known upstream bug the owner can veto: a correct clip is `Duration -= EndTime - maxTime`. Returns old clone to new mark (null for removed) for undo; returns null for an empty collection. **Quirk:** the only caller loops over that result with no null check (`Form.cs:3672-3680`), so Bulk Effects Move with marks crashes upstream as soon as it reaches an empty, unlocked collection. **GUARD**: nothing moved for that collection. Already right in Rust: `offset_marks_in_range` in `crates/vixen-session/src/sequencer/dialogs.rs`, called by `bulk_move`, which also keeps the `Duration += EndTime` quirk.
  - `IsVisible`: F12. `Clone`: copies all but id, default flag and link (`MC.cs:367-381`). No window in this area reaches it. **Quirk:** the clone's `Marks` reads empty, since `_marks` is replaced after the read-only view is built (`MC.cs:26-31`, `:369-380`). `TimedSequenceData.Clone` is the only caller in `src` (`Vixen.Modules/Sequence/Timed/TimedSequenceData.cs:113`), and nothing observable depends on the emptiness, so clone properly in Rust. `MarksWithinRange` (`MC.cs:230-235`) returns an empty collection and has no callers; `Mode.cs` has no users. Omit both.

### G. Name rules

- [ ] **F50. Unique names.** `MarkCollectionNameService` (`NameSvc:18-144`). Comparison: trimmed, ordinal case-insensitive.
  - `IsUniqueName(all, proposed, excludedId?)`: false for empty after trim; true if no other collection matches.
  - `GetUniqueName(desired, all, excludedId?)`: trims; throws on empty; returns the trimmed name if free, else `<name> - 2`, `- 3`, first free.
  - Used by New (F4), marks-bar creation (sequencer-shell F298), import (F36).

- [ ] **F51. Duplicate repair on sequence load** (`NameSvc:71-116`, called at `Form.cs:5627`). In list order, keeping track of all occupied names and seen names:
  - Blank name: replaced by a unique `Mark Collection`; recorded.
  - First time a trimmed name is seen: kept. If trimming changed it, the trimmed form is stored, and recorded.
  - Duplicate: gets a unique suffixed name; recorded. **Note:** the suffix avoids every name in the list including later ones.
  - After a load with records, an alert lists them (sequencer-shell F11) and the sequence is modified. Expected values in Tests.

- [ ] **F52. Effect selection normalisation.** `MarkCollectionSelectionService.Normalize` (`Vixen.Core/Marks/MarkCollectionSelectionService.cs:18-50`): inactive selection keeps its id. Otherwise walk the collections in order: the first with the selected non-empty id returns it unchanged. Remember the first collection overall, and the first matching the preferred type (a null preferred type matches nothing). If the id was not found, return the preferred; else the first if fallback is allowed; else empty. `crates/vixen-effects` uses this for pickers (effect-editor spec F65); check it matches.

### H. Layers panel

Sequencer-shell F329-F340 summarises this panel; the detail here supersedes it where they differ. The Chroma Key, Luma Key and Mask And Fill sheets are specified in `docs/spec/property-filter-controller-setup.md` F212-F229, with logic already in `crates/vixen-session/src/dialogs/layers.rs` and `crates/vixen-ffi/src/dialogs/layers.rs`. Do not re-spec or re-implement those.

- [ ] **F53. Layers panel.** A dock panel titled "Layers", initial client size 331x302, toggled by the menu item "Layer Editor Window" (`TSE/LayerEditor.Designer.cs:26-29`, `TSE/TimedSequenceEditorForm.Designer.cs:977`). One per sequencer window, created on first use (`Form.cs:1097-1112`). Docked right by default, and reopening from the View menu docks it left (`crates/vixen-session/src/sequencer/panels.rs`). Top to bottom (`LE/Views/LayerEditorView.xaml:19-122`): a toolbar with Import Layers and Export Layers; a row with two equal-width buttons, `Add Layer` on the left and `Remove Layer` on the right; then the list filling the rest. List order is stacking order, top of the list is the highest layer, and the default layer is always last. Reorder keeps the invariant (F62).

- [ ] **F54. Keys in the panel.** Same as F2: keys outside text fields forward to the sequencer hotkeys (`TSE/LayerEditor.cs:55-64`). Upstream also sends ⇧Tab to itself on entry to find a focus target (`:42-48`): no meaning on the Mac (Windows-only omissions).

- [ ] **F55. Layer model.** `LayerLevel = count - index - 1`, recomputed after every add, remove, replace, insert and move, so the default layer is level 0 and the top layer is highest (`SL.cs:205-213`). `Layers[0]` is the top. New `SequenceLayers` holds just the default layer: name `Default`, type Default, level 0, no filter, `FilterName` "None", `FilterTypeId` empty (`Vixen.Core/Sys/LayerMixing/DefaultLayer.cs:8-13`, `Vixen.Core/Sys/LayerMixing/Layer.cs:62-74`). Upstream's default layer in a *new* `SequenceLayers` is one static shared instance (`SL.cs:18`, `:31`): **Quirk**, in a new, unsaved sequence renaming it (F58) also changes it for every other unsaved sequence in that process; a sequence loaded from a file has its own copy. **FIX** (implied by Rust's ownership): `crates/vixen-core/src/layer.rs` gives each sequence its own. Effects live on a layer through a map of effect id to layer id; effects absent from the map are on the default layer (`SL.cs:126-140`, `:188-203`). Assigning to the default just removes the entry. `RequiresMixingPartner` is `Type == Standard && filter.RequiresMixingPartner` (`Vixen.Core/Sys/LayerMixing/Layer.cs:76-79`). Already ported as `SequenceLayers`.

- [ ] **F56. Add Layer.** Always enabled, as long as a filter type exists (it always does here). Creates a new layer at index 0 (top) using the **first** filter type in the panel's list (`LEVM:170-179`, `LESvc:21-28`, `SL.cs:68-72`). Name: `EnsureUniqueName("Default")`, exact comparison: `Default - 2`, `Default - 3`... since `Default` is taken by the default layer. It does not use the filter's name.

  **The first filter type.** The list is `GetModuleDescriptors<ILayerMixingFilterInstance>()`, in module discovery order (`LEVM:159-168`, `Vixen.Core/Sys/Modules.cs:44-46`, `:165-171`, `:378-390`), where module files come from `Directory.GetFiles(modules, "Module.LayerMixingFilter.*.dll")`. That API promises no order, but on NTFS it returns names sorted, so the effective order is by DLL name (`Vixen.Modules/LayerMixingFilter/Directory.Build.props:4`). The Mac has no modules, so pin the order as a constant in Rust:

  | # | Project (DLL suffix) | TypeName | TypeId | Has setup | Requires partner |
  |---|---|---|---|---|---|
  | 1 | ChromaKey | Chroma Key | `2D467733-1A64-45AD-BCDC-84BF074747E6` | yes | **yes** |
  | 2 | ColorChange | Color Change | `93D4169E-5138-403A-B71C-A5A1AB089801` | no | **yes** |
  | 3 | HighestValue | Highest Value (Default) | `01D6D526-9586-44BB-82F4-10ABACF58B61` | no | no |
  | 4 | IntensityOverlay | Intensity Overlay | `43F48BF5-4088-403F-AD7A-D0FF2114CA31` | no | **yes** |
  | 5 | LumaKey | Luma Key | `7583F7D4-DF99-4492-8D10-257C2677FF7B` | yes | no |
  | 6 | Mask | Mask | `ACE0F097-7982-4EDC-B5DD-8989DCE60E1D` | no | no |
  | 7 | MaskFill | Mask And Fill | `24878B18-14F8-458E-ABAA-3A9C9A34931C` | yes | per data (default no) |
  | 8 | MultiplyColor | Multiply Color | `0C9DFC0B-8E39-4167-8036-79408C6B0B35` | no | no |
  | 9 | ProportionalMix | Proportional Mix | `1F16D67A-1FFF-485C-8E76-27FF7E7D072B` | no | no |

  So on the owner's Windows install **Add Layer almost certainly creates a Chroma Key layer**, named `Default - 2`. That is inferred from the file system's ordering, not stated anywhere in code; it is an open question for the owner to confirm by clicking Add Layer once on Windows. The source's own description of `HighestValue` calls it "(Default)", but nothing selects it by default. The owner's show neither confirms nor refutes it: its eight sequences with layers hold Intensity Overlay (10 layers), Mask (3), Highest Value (3), Luma Key (2), Color Change and Proportional Mix, and no Chroma Key, so the owner evidently picks a Type after adding. Some are still named `Default 2`, an older naming without the dash.

  Modified via the collection-changed handler (`TSE/LayerEditor.cs:72-82`, `Form.cs:1217-1220`). No undo (sequencer-shell F339).

- [ ] **F117. Layers made with Add Layer share one filter.** `_filterTypes` is built once when the panel is created, one instance per type (`LEVM:27`, `:68`, `:159-168`), and Add Layer hands over that same first instance every time (`LEVM:172`, `:178`). So every layer the button adds while the panel lives has the same filter object and the same settings: Configuration on one changes them all, until a Type change gives that layer a fresh instance (F60). It lasts through save and load: the layers' surrogates carry one `InstanceId`, the file holds one data model for it, and each layer gets that model back (`Vixen.Core/Module/SequenceType/SequenceTypeDataModelBase.cs:84-96`, `:137-152`, `Vixen.Core/Module/ModuleDataSet.cs:80-99`). Layers from import don't share, since each record resolves its own instance (`LIES:132`). **FIX** (implied by Rust's ownership, listed for veto): each new layer gets its own settings and `filter_instance_id`. No sequence in the owner's show has a shared id. Reading and writing such a file from someone else: Data and persistence, Layers XML.

- [ ] **F57. Remove Layer.** Removes the list's **selected** layer (`LE/Views/LayerEditorView.xaml:42`). Enabled when the selection is not nil and not the default layer (`LEVM:191-194`), so it needs a click on a row first. No confirmation. Effects on the removed layer move to the default layer by deleting their map entries (`SL.cs:107-124`). Modified. No undo. The removed layer's filter settings go with it. Upstream throws if asked to remove the last index; the service guards with `IsDefaultLayer` first (`LESvc:31-46`).

- [ ] **F58. Layer row and Name.** Each row is an expander, starting collapsed, whose header shows the layer name (`LE/Views/LayerEditorView.xaml:55-60`). Inside, on a 3-column grid: `Name` label, text field, quick rename button; `Type` label and combo; `Configuration` button under the field column. Rows are selectable by clicking. Expansion is not remembered.

  **Name** edits commit on Enter and on focus loss; Esc puts back the last committed value, which after an Enter is that text and not the one at focus (`Vixen.Common/WPFCommon/Utils/TextBoxExtender.cs:199-210`); typing alone does not commit (`LE/Views/LayerEditorView.xaml:76-84`). **No validation at all**: duplicates and blanks are accepted, and the **default layer can be renamed** since this field has no visibility rule. **KEEP**. `GetLayer(name, typeId)` picks the first match, and paste uses it (`SL.cs:153-156`, `Form.cs:5392`). **Not Modified**, since only the list's change and `Setup` returning true are watched (`LEVM:339-355`, sequencer-shell F330): **FIX** per sequencer-shell, mark Modified. No undo.

- [ ] **F59. Quick rename** (icon `layer_rename.png`, 24x24, tooltip "Rename to filter type"). Hidden on the default layer (`LE/Views/LayerEditorView.xaml:85-96`, `LE/Converters/DefaultLayerVisibilityConverter.cs:13-19`). Sets the name to the filter's TypeName, made unique against the other layers by exact comparison: `<TypeName>`, `<TypeName> - 2`, `- 3`... (`LESvc:73-116`). Blank desired name returns `Layer` in the helper, though quick rename itself returns early on a blank type name. Not Modified: same FIX as F58. No undo.

- [ ] **F60. Type combo.** Lists the nine filter types by TypeName (F56's order), hidden with its label on the default layer (`LE/Views/LayerEditorView.xaml:98-107`). Choosing one sets `FilterTypeId`, whose setter asks the service for a **brand-new filter instance** (`Vixen.Core/Sys/LayerMixing/Layer.cs:67-74`). So settings are discarded: changing Luma Key to Mask and back gives default limits. **KEEP**. The name does not follow. Not Modified: same FIX as F58. No undo. Takes effect on next render or play; upstream does not re-render on any layer change (`Form.cs:1217-1220`).

- [ ] **F61. Configuration button.** Under the field, visible only when the current filter `HasSetup` (`LE/Views/LayerEditorView.xaml:109-115`, `LE/Converters/HasSetupVisibilityConverter.cs:13-19`): Chroma Key, Luma Key, Mask And Fill. It shows or hides as soon as the Type changes, since the layer raises that change (`Vixen.Core/Sys/LayerMixing/Layer.cs:81-92`). Opens that filter's sheet. OK returns true and marks Modified; Cancel changes nothing (`LEVM:196-207`, `LESvc:67-70`). Details in `docs/spec/property-filter-controller-setup.md` F212-F223. No undo. Defaults: Chroma Key `new_chroma_key_data()` in `crates/vixen-io/src/sequence.rs` (lime, tolerance 5 and 0.05, range 0 to 1); Luma Key 0 and 1; Mask And Fill exclude-zero on, require-partner off.

- [ ] **F62. Drag to reorder.** Drag a row; any non-default row can start a drag (`LE/Views/LayerEditorView.xaml.cs:96-114`). Drop on a row: new index is that row's index; dropping on the **last** row (the default) becomes the index just above it (`:123-145`). A drop outside any row does nothing. The service then refuses if the layer is default or unknown, or the index is negative, is at or past `count - 1`, or is unchanged (`LESvc:49-64`). `Move` shifts rows between the two indexes; levels recompute (`SL.cs:79-84`). No drag image (`LE/Views/LayerEditorView.xaml.cs:148-151`). Modified. No undo. On the Mac, either `.onMove`, restricted so the default row neither moves nor accepts a drop below it, or the row-to-row `.draggable` and `.dropDestination` that `LayersPanel` uses now, which carry upstream's rule as written. `.onMove` reports an insertion offset, so if it is used convert in Rust: `to = if offset > from { offset - 1 } else { offset }`, then clamp to `count - 2`. Pin with tests.

- [ ] **F63. Export Layers** (tooltip "Export Layers", icon `folder_open.png`). Enabled when any non-default layer exists (`LEVM:229-232`, `LESvc:118-121`). Save panel titled "Export Layers", filter `Vixen 3 Layers (*.v3l)|*.v3l|All Files (*.*)|*.*` (`LEVM:22`, `:239-244`). Existing file: confirm "A file with this name already exists. Do you want to replace it?" titled "Confirm Save As" (`:252-265`); `NSSavePanel` confirms replacement itself, so do not ask twice. Writes the document in F64. IO or permission errors: `The layers could not be exported: <message>` titled "Export Layers" (`:271-275`). Nothing marks the sequence modified; nothing shown on success. Does not change `_lastFolder`.

- [ ] **F64. `.v3l` format** (`LIES:42-68`, `:177-240`, `LE/ImportExport/LayerExportDocument.cs`, `LayerExportRecord.cs`). UTF-8 JSON, 2-space indented, keys in this order:

  ```json
  {
    "format": "Vixen3Layers",
    "version": 1,
    "exportedUtc": "2026-11-09T12:34:56.1234567Z",
    "layers": [
      {
        "name": "Top",
        "order": 0,
        "filterTypeId": "7583f7d4-df99-4492-8d10-257c2677ff7b",
        "filterName": "Luma Key",
        "filterDataType": "VixenModules.LayerMixingFilter.LumaKey.LumaKeyData, Module.LayerMixingFilter.LumaKey",
        "filterData": { "LowerLimit": 0.3, "UpperLimit": 0.7 }
      }
    ]
  }
  ```

  Layers are the non-default ones, top to bottom; `order` is the position among them. `filterDataType` and `filterData` are **omitted** when the filter has no data (the six setup-less types). `filterData` is the module data as `DataContractJsonSerializer` writes it, minus `ModuleTypeId` and `ModuleInstanceId`. The three data classes have no `[DataContract]` (`Vixen.Modules/LayerMixingFilter/LumaKey/LumaKeyData.cs:6`), so the serializer treats them as plain types: every public read/write property, and their constructors run on read (F65). Key sets (`Vixen.Modules/LayerMixingFilter/*/*Data.cs`):
  - Luma Key: `LowerLimit`, `UpperLimit` (doubles).
  - Mask And Fill: `ExcludeZeroValues`, `RequiresMixingPartner` (booleans).
  - Chroma Key: `HueTolerance`, `KeyColor`, `KeyHue`, `KeySaturation`, `LowerLimit`, `SaturationTolerance`, `TransparentOnZeroBrightness`, `UpperLimit`, where `KeyColor` is a `System.Drawing.Color` as `{ "knownColor": n, "name": null, "state": s, "value": argb }`. In the real show's XML, a known colour has `knownColor` non-zero, `state` 1, `value` 0, and a plain ARGB colour has `knownColor` 0, `state` 2 and `value` the ARGB as an unsigned 32-bit number; read both, and write ARGB form.

  Files must round-trip with Windows Vixen, so the owner can move layer sets between machines. `filterDataType`, `filterName` and the date are for human readers: nothing acts on them, apart from `filterName` in F65's skip message. They are still parsed, so a malformed date or a value of the wrong JSON type fails the file at F65's step 2. The date uses `DateTime.UtcNow` ISO round-trip format. Upstream's default encoder writes non-ASCII and `<>&'+"` as `\uXXXX`; any JSON reader takes either form, so write plain UTF-8.

- [ ] **F65. Import Layers** (tooltip "Import Layers", icon `folder_go.png`). Always enabled. Open panel titled "Import Layers", same filter, one file (`LEVM:283-289`). Then the plan, computed without touching the sequence (`LIES:71-150`). In order; any failure shows its message in an error alert titled "Import Layers" and stops (`LEVM:297-314`):

  1. Read error: `The file could not be read: <message>`.
  2. JSON error: `The file is not valid JSON: <message>`. That covers bad syntax, trailing commas and comments, and any known key with a value of the wrong type: a malformed GUID or date, `"version": "1"`, a `null` `filterTypeId` (`LE/ImportExport/LayerExportDocument.cs`, `LayerExportRecord.cs`).
  3. Document is JSON `null`: `The file does not contain a layer export document.`
  4. `format` not exactly `Vixen3Layers`: `'<format>' is not a supported layer export format.`
  5. `version` not 1: `Layer export version <n> is not supported.`
  6. `layers` is JSON `null`: `The file does not contain any layer records.`

  Property names are case-sensitive; unknown ones are ignored; a missing `format` or `version` takes the class default and so **passes**; a missing `layers` is an empty list and reaches step 8's message.

  7. Records, by ascending `order`, stable:
     - A `null` entry in `layers`: the sort throws a null reference, which upstream doesn't catch (`LIES:118`). **GUARD**: step 2's message.
     - `filterTypeId` empty: ignored silently (a hand-added default layer).
     - Blank `name` (including missing): **fails the whole file**, `A layer record is missing its required name.` (This check comes after the empty-id one.)
     - Filter type unknown: skipped, `The '<filterName>' mixing filter (<filterTypeId>) is not installed.`
     - Filter has data but the record has none: skipped, `The layer record does not contain filter configuration data.`
     - Data that cannot be applied: skipped, `The filter configuration could not be restored: <message>`.
     - A filter with no data ignores any `filterData`.
  8. None importable: `The file does not contain any layers that can be imported.`
  9. Any skipped: confirm `The following layers cannot be imported:\n\n<name>: <reason>\n<name>: <reason>\n\nContinue importing the remaining layers?` titled "Import Layers" (`LEVM:322-334`). Cancel stops. Use the buttons "Continue" and "Cancel".
  10. Import (`LIES:153-175`): sorted by order, added in **reverse**, so the exported order sits above all existing layers. Each new layer has a fresh id and a name made unique against the others by exact comparison (`Top` collides to `Top - 2`). Each add is one collection change, so Modified. No summary is shown. No undo.

  Undefined JSON values (`filterData` missing) and `null` are treated alike. On missing keys inside `filterData`, the field keeps its **constructor's default**, because these classes have no `[DataContract]` and `DataContractJsonSerializer` runs the public constructor of such a type: `{"LowerLimit":0.3}` on Luma Key gives `UpperLimit` 1, `{}` on Mask And Fill gives exclude-zero on, and a Chroma Key with no `KeyColor` stays lime (`Vixen.Modules/LayerMixingFilter/LumaKey/LumaKeyData.cs:6-12`, `Vixen.Modules/LayerMixingFilter/MaskFill/MaskAndFillData.cs:6-11`, `Vixen.Modules/LayerMixingFilter/ChromaKey/ChromaKeyData.cs:7-18`). Unknown keys inside are ignored. **KEEP**. No upstream test covers a missing key; `Vixen.Tests/Sequencer/LayerImportExportServiceTests.cs` pins the rest of this entry.

- [ ] **F66. Assign effects to a layer.** Effect context menu (`Layer` submenu, sequencer-shell F227, `Form.cs:2395-2460`). Listed bottom layer first (`layers.Reverse()`). Each item shows the layer name, tooltip its filter name, and a check on used layers (a lighter check when more than one layer is in use among the selection). Undo exists there (`EffectsLayerChangedUndoAction`, sequencer-shell section M). `Find Effects` reads the same layers (sequencer-shell). This spec adds nothing; it is listed so that the panel's rename and removal are known to reach them.

### I. Beat and bar detection

**The DSP is done. Do not compile the C++.** `crates/vixen-analysis` (GPL-2.0-or-later, licences kept beside it) is a Rust port of the QM Bar and Beat Tracker, QM DSP, KissFFT and all the non-UI logic of `BeatsAndBars.cs`, `BeatBarSettingsData` and the dialog's state (`BeatsAndBarsDialogModel`), pinned bit for bit against the C++ (`crates/vixen-analysis/tests/golden_units.rs`, `golden_synthetic.rs`, `golden_real_music.rs`), against the C# mark building (`tests/golden_marks.rs`), and against marks the owner's Windows install actually stored (`tests/real_show.rs`). What's missing is only the session glue, FFI and windows. Nothing in `crates/` outside that crate uses it yet.

- [ ] **F67. Tools > Audio > Beat/Bar Detection** (sequencer-shell F80). Off in the designer (`TSE/TimedSequenceEditorForm.Designer.cs:1058`). Enabled when the sequence has any media at load or audio is associated, and off again when audio is removed (`Form.cs:1759-1764`, `:1876`, `:1991`). The check comes before the one for the file, so it is on even when the audio file is missing, and then step 6 of F68 is all that happens. Takes the first `Audio` in the sequence's media (`Menu.cs:609-623`). Marks the sequence modified **even if the dialog is cancelled**. **FIX** (proposed): mark Modified only when something changed. No undo (sequencer-shell F418).

- [ ] **F68. The sequence of steps** (`BB/BeatsAndBars.cs:418-467`). Channels 0, which is what a missing audio file gives (`Vixen.Modules/Media/Audio/Audio.cs:59-69`, `:240-242`): skip to step 6.
  1. Read the whole file as raw bytes and convert to floats (`BB/BeatsAndBars.cs:425-435`). The conversion is a faithful quirk, documented at the top of `crates/vixen-analysis/src/samples.rs`: not a mixdown, not normalised, every 4 raw bytes read as an `Int32`. 16-bit stereo effectively tracks the right channel; 16-bit mono runs at double speed, so marks land at half their true time. **KEEP**: the owner's existing marks were made this way, and `real_show.rs` proves the match. Use `BeatsAndBars::from_interleaved` with `RawSampleFormat::for_path`.
  2. Build the first 10 seconds, zero padded (`BB/BeatsAndBars.cs:427`, `:437-439`).
  3. Run the tracker on that preview with 4 beats per bar and **no progress UI, on the UI thread** (`BB/BeatsAndBars.cs:327-364`, `:442`). On the Mac: `generate_preview_data` on a background thread, showing a spinner in the sheet.
  4. Show the dialog (F69).
  5. On Generate: set `bpb` to the chosen beats per bar, initialise, run the tracker on the whole file behind the progress window (F74), then build collections (F72).
  6. Finally, whatever happened: if any collections exist and none is default, the first becomes default (`BB/BeatsAndBars.cs:463-466`).

  **How it uses QMLibrary** (`QM/QMBarBeatTrack.cpp`, `QM/WrapperFiles/ManagedPlugin.cpp`, `QM/QMFiles/BarBeatTrack.cpp`): one channel; step `size_t(rate * 0.01161 + 0.0001)` (512 at 44.1 kHz, `:28`, `:251`), block twice that (`:260`); `Process` each block with `frame2RealTime(j, rate)`; a final zero-padded block; then `GetRemainingFeatures`, which does all the real work in one go (`:364-375`, `:378-477`). Outputs: 0 beats labelled with metrical position, 1 bars labelled by bar number, 2 beat counts with value, 3 spectral difference (`:443-447`). Vixen reads 0, 1 and 2. Only `bpb` is set; `alpha` 0.9, `inputtempo` 120, `constraintempo` off and tightness 4 stay at defaults (`:63-67`). All feature times come through `ManagedRealtime::totalMilliseconds`, whose integer arithmetic truncates to a whole millisecond (`QM/WrapperFiles/ManagedRealtime.cpp`, `crates/vixen-analysis/src/vamp.rs:99-103`), so bar and beat marks sit on whole milliseconds. `VampParamCtrl` and `VampOutputCtrl` are unused WinForms controls (no references outside themselves): omit.

- [ ] **F69. Beats and Bars dialog** (`BB/BeatsAndBarsSettings.cs`, `BB/BeatsAndBarsSettings.Designer.cs:227`). Fixed single border, centred on screen, titled "Beats and Bars", client 740x600. Three group boxes, top to bottom: **"Divisons"** (sic, upstream's spelling; **FIX** (proposed) to "Divisions"), **"Preview"**, **"Output"**. At bottom right: `Generate` (result OK), `Cancel` (result Cancel; designer `:70`, `:81`). The form sets neither `AcceptButton` nor `CancelButton` (designer `:225-237`), so Return and Esc do nothing upstream; closing with the title bar's X counts as Cancel. **FIX** (proposed): give Cancel `.cancelAction`, so Esc works as on any sheet, and leave Return alone, because a Return in the base name field must not start an analysis. `Generate` is enabled while at least one output is checked (`BB/BeatsAndBarsSettings.cs:119-126`). Nothing persists between openings: the static settings are overwritten from fresh controls at each open (`:49-57`). State model: `BeatsAndBarsDialogModel` in `crates/vixen-analysis/src/settings.rs`; bind to it through FFI.

- [ ] **F70. Divisions group: music staff** (`BB/MusicStaff.cs`).
  - **Read-outs** across the top, in this order: bold `BPM:` value `60000 / beatPeriod` with format `F1`; bold `Bar Period (sec):` value `beatPeriod * beatsPerBar / 1000` with `F2`; bold `Div Time (ms):` value `beatPeriod`, or half when Split Beats is on, with `F0` (`:134-149`). Beat period comes from the preview (F68 step 3) and is **not** recomputed when the time signature changes. `crates/vixen-analysis` has `bpm()`, `bar_period_seconds()`, `div_time_ms()`. With a zero period BPM is infinite; show `∞`, as .NET's formatting does.
  - **Staff picture** `Resources/fullstaff.png` stretched across the width, time signature digits in bold 12 pt on the left at x = 80 (top: beats per bar, bottom: note size) (`:32-33`, `:125-132`).
  - **Note glyphs** above the staff, `beatsPerBar * (split ? 2 : 1)` of them, evenly spaced from x = 125 to width - 16 at y = staff top - 20 (`:158-175`, `:22`). Glyph by `noteSize * (split ? 2 : 1)`: 2 half, 4 quarter, 8 eighth, 16 sixteenth, 32 thirty-second (`:88-119`). PNGs in `BB/Resources/`; bundle them.
  - **Label** "Click staff to change time signature" (`docs/reference/forms/Vixen.Modules__Analysis__BeatsAndBars__MusicStaff.json`).
  - **Clicking the staff or either digit** opens a menu at the click (digits: at their corner): items `2/2`, `3/4`, `4/4`, `5/8`, `6/8`, `7/8`, `8/8`, `9/16`, `10/16`... `16/16`, one per beats-per-bar from 2 to 16, with the current checked (`BB/MusicStaff.cs:220-235`, `:59-81`, `:237-250`). Default 4/4. Choosing one sets beats per bar and raises the settings change (F71). Keep the picture and click target; a `Menu` on the staff view does the job. Each opening also makes that menu the control's context menu (`:233`), so after the first click a right-click anywhere on the control shows it too, with the check from when it was built. **FIX** (proposed): the same menu on right-click anywhere on the staff from the start, always current.
  - **Split Beats** checkbox beside the label; default off (`BB/MusicStaff.cs:54-57`). Raises the settings change.

- [ ] **F71. Settings change.** On Split Beats or time signature change (`BB/BeatsAndBarsSettings.cs:150-162`): Beat Splits' checkbox and its colour panel enable to match Split Beats, the checkbox checks to match it too; then the preview updates. So turning Split Beats on checks Beat Splits, and changing the signature re-checks it after a manual uncheck. At open Beat Splits is enabled and unchecked even though Split Beats is off, and its swatch is enabled too, because the constructor's `Checked = false` changes nothing and so raises no event (`BB/BeatsAndBarsSettings.cs:54`, `BeatsAndBarsDialogModel::default`). So a user can check Beat Splits with one division. **KEEP**, including its result: with one division names are `#1a`, `#1b`, `#2a`, `#2b` and only that many exist (`crates/vixen-analysis/src/settings.rs:61-78`).

- [ ] **F72. Output group.** Label "Collection Base Name:", text field default `Beats`, tooltip "Base name of each collection"; read when Generate is pressed. Then, left to right, each with a colour swatch, 28x24, clicking opening the colour picker; a swatch is disabled while its box is unchecked, apart from Beat Splits at open (F71) (`BB/BeatsAndBarsSettings.cs:39-47`, `:131-143`, `:186-208`, designer `:122-149`):

  | Checkbox | Default | Colour default | Tooltip | Swatch tooltip | Collection name |
  |---|---|---|---|---|---|
  | Bars | on | RGB(255,255,128) | "Single Collection containing starting location of each measure/bar" | "Color of Bars Collection" | `<base> 1/<noteSize> Beat #1 (Bar)` |
  | Beat Counts | on | RGB(255,128,0) | "Generates a beat collection for each beat count" | "Color of Beat Counts Collection" | `<base> Beat #<n>`, one per beat |
  | Beat Splits | off | Lime (0,255,0) | "Generates a beat collection for each beat count and each beat count split" | "Color of Beat Splits Collection" | `<base> Beat #<n>a`, `#<n>b` |
  | All Features | on | White | "Single Collection containing all features" | "Color of All Features Collection" | `<base> - All` |

  The picker opens at its own default colour, not the swatch's (`:134-137`): with the system panel, starting at the current colour is the native behaviour and harmless. **FIX**.

  **Building** (`BB/BeatsAndBars.cs:366-416`, `crates/vixen-analysis/src/beats_and_bars.rs:259-319`). Marks have the default 450 ms and blank text; collections have constructor defaults, so `ShowGridLines` on and bar off, Generic, level 1. Order of extraction is Bars, Beat Counts, Splits, All. Before each kind runs, **any existing collection with exactly that name is removed, with no warning**, marks and all. That is how re-running replaces. Comparison is case-sensitive, so `beats - all` is left. The new ones get **new ids**, so an effect that selected the old collection loses it and is re-normalised (F52). Names do not go through the name service, so a duplicate differing only by case can appear; the next load's repair renames it (F51).

  De-duplication: marks equal in start and duration to one already in an earlier kind are dropped from Beat Counts and Splits (`BB/BeatsAndBars.cs:156-165`). With Bars on, **`Beat #1` usually ends up empty**, and with Counts on, every `a` split does too. Empty collections are not added. The surviving new ones are appended, sorted by name. The owner's real sequences hold exactly these names (for example `Beats - All` in `Zoom.tim`). All Features is not de-duplicated.

  A blank base name is accepted. **KEEP**: it gives names such as ` - All` (the next load's trim repair turns that into `- All`).

  Other quirks ported in the crate: the first beat of All Features appears twice and the last beat never (`crates/vixen-analysis/src/beats_and_bars.rs:141-161`); splits add nothing for the last beat.

  **Undo:** none (sequencer-shell F418). Modified. **Persists** only as mark collections in the sequence.

- [ ] **F73. Preview group.** A waveform of the audio from time 0 for `PreviewPeriod`, on light grey with a single-pixel border, half the group's height, full width minus 25, centred vertically (`BB/BeatsAndBarsSettings.cs:61-66`, `BB/PreviewWaveform.cs:19-31`). `PreviewPeriod` is the **last split mark's time** whichever mode is showing, so it does not change with Split Beats (`BB/BeatsAndBarsSettings.cs:172-175`). Over it, yellow 2 px vertical lines at each preview mark: the split marks when Split Beats is on, else the plain ones (`:168-171`, `BB/PreviewWaveform.cs:92-120`). `timePerPixel` is `PreviewPeriod` ticks over the strip's width, in integer ticks (`BB/PreviewWaveform.cs:59-69`). Each line goes at the first pixel `j` where `mark <= timePerPixel * j`, one mark per pixel at most, so a mark that loses a pixel to the one before it draws a pixel late. With no split marks at all, the period stays at its first value, 1 second (`BB/PreviewWaveform.cs:19-22`). The waveform is drawn as the timeline's, but in `Full` style, with the bottom half as well as the top, where the timeline uses `Half` (`Vixen.Common/Controls/TimeLineControl/Waveform.cs:234`, `:297`; timeline spec section N). Draw it with `vixen_audio::Peaks::range`, not a second decoder, and redraw lines without recomputing peaks. Nothing in the strip responds to the mouse on purpose; the height drag it inherits from the timeline's waveform is an accident (`Vixen.Common/Controls/TimeLineControl/Waveform.cs:42`, `:136-160`) and is left out (see the end of Windows-only omissions).

- [ ] **F74. Progress window.** Tool window, centred on screen, titled "Beats and Bars" (`docs/reference/forms/Vixen.Modules__Analysis__BeatsAndBars__BeatsAndBarsProgress.json`). Label `Generating Marks:`, a bar, and `<n>%` beside it (`BB/BeatsAndBarsProgress.cs:16-35`). Updates only when the integer percent changes. When feeding blocks is done the tracker's long final step starts: the label becomes `Finalizing beat analysis...`, the percent hides and the bar goes indeterminate (`:37-43`, `BB/BeatsAndBars.cs:125`). So 100% is not the end: the beat tracking itself all happens in that final step (`QM/QMFiles/BarBeatTrack.cpp:378-477`). On the Mac: a sheet with `ProgressView`.

  Upstream has no cancel, and the window has no close box (`ControlBox = false`, `BB/BeatsAndBarsProgress.Designer.cs:71`). Alt+F4 still closes it, and during analysis that returns a null result while the worker runs on, and crashes on `_featureSet[1]` (`BB/BeatsAndBars.cs:45-87`, `:371`, `:378`). **GUARD**: give the sheet a Cancel button wired to the crate's cancel flag (`for_each_block`, `remaining_features_cancellable`). Cancel leaves collections as they were, apart from F68 step 6, because the crate removes nothing until the features exist. The sheet is modal to the sequencer window, as the dialog was to the app.

- [ ] **F75. Analysis errors.** Upstream throws `KeyNotFoundException` when an output has no features, as with a clip too short to hold a beat or a bar, with the already-removed old collection then lost (`BB/BeatsAndBars.cs:339`, `:377-378`). **GUARD**: `AnalysisError::MissingOutput` leaves everything alone. Show an alert titled "Beats and Bars" saying the audio did not produce any beats or bars, and do not open the dialog when it comes from the preview step. This wording is new, since upstream has none.

### J. Lyric converter (text to phonemes)

The phoneme types are `AI, E, O, U, FV, L, MBP, WQ, ETC, REST, Unknown` (`PU:7`).

- [ ] **F76. LipSync Text Convert window.** Tools > LipSync > Lyric Converter (sequencer-shell F90, `Menu.cs:567-577`). Non-modal, owned by the sequencer, so it floats above it while the user works and closes with it; only one at a time (static `Active`, set on shown and cleared on closed, `TCF:370-378`). `Active` is process-wide, and the menu item does nothing while it is set, not even bring the window forward, including from another sequencer (`Menu.cs:569`). It works on the sequence of the sequencer that opened it, whose collections and handlers it was given (`Menu.cs:572-574`). On the Mac the item brings the open window forward, which is the native behaviour and loses nothing. There is no Close button and no `CancelButton`, so Esc does nothing; the title bar closes it. Fixed dialog titled "LipSync Text Convert", client 300x403 (`docs/reference/forms/Vixen.Modules__App__LipSyncApp__LipSyncTextConvertForm.json`). Top to bottom: group "Target / Spacing"; group "Marks" with its labels to its left; "Text to Convert" label with "Clear after conversion" checkbox to its right; the text box; `Convert` at bottom right, the default button. A window, not a sheet: the guide says separate windows for tools you work alongside. Nothing persists between openings. No undo inside the window; the results' undo is the sequencer's (F83).

- [ ] **F77. Target / Spacing.** Radios in order, `Cursor / Even Spacing` (default), `Clipboard / Even Spacing`, `Per Mark Collection` (`TCF:229-234`). Per Mark Collection enables the Marks group and refreshes it; anything else disables it and clears its selections (`TCF:286-289`, `:291-363`). **The Per Mark Collection radio can't be chosen when no collection has marks**: `AutoCheck` is off, and it is greyed when the marks lists are empty (`:313`, `:334-336`). Its grey colour is only updated while collections exist, a small upstream glitch; on the Mac disable it whenever no collection has marks.

- [ ] **F78. Marks group.** Three drop-down lists (`TCF:291-363`, `:236-284`), all enabled only with Per Mark Collection, labels greyed in step (`:399-404`):
  - **Collection:** names of collections with at least one mark, in list order. Rebuilt on every window activation (`:365-368`), keeping the previous index if the count is unchanged; else the first.
  - **Start Offset:** the start times of the chosen collection's marks, listed by .NET's default `TimeSpan` text: `hh:mm:ss` with no fraction, otherwise `hh:mm:ss.fffffff` (seven digits). Refreshed on opening the list and on collection change (`:236-266`, `:281-284`). Keeps its index if still valid, else the last item; first if nothing was selected. Changing the collection resets it (`:269-279`).
  - **Alignment:** `Phoneme`, `Word`, `Phrase`, default `Word`, filled the first time the group enables (`:345-352`). Going back to Cursor or Clipboard clears the other two lists but not this one (`:354-362`), and F80 reads it whatever the radio says. **KEEP**: after choosing `Phrase` once, an even-spacing Convert treats each line as one string, with the whole line as the lyric, and a line's unknown words drop silently instead of prompting. `Phoneme` there gives the same times as `Word`, 250 ms each.

- [ ] **F79. Text box and Clear.** Multi-line plain text. `Convert` enables only while the text is not all whitespace (`TCF:380-392`), and starts disabled (`:231`). A vertical scrollbar shows only while the text contains a line break (`:382-389`); `NSScrollView`'s automatic scrollers replace this. If text is selected only the selection converts (`:35-38`). **"Clear after conversion"** default off; when on, the box empties (`:179`).

  Return key: upstream's multi-line box has no `AcceptsReturn`, so Enter presses Convert and Ctrl+Enter makes a line break (designer: `AcceptButton`, `Multiline`). **KEEP**: Return converts; on the Mac, ⌥Return makes a line break, which `NSTextView` already understands. Paste keeps line breaks.

- [ ] **F80. Convert.** (`TCF:105-227`.)
  1. If the bundled dictionary is missing: error "Unable to find Standard Phoneme Dictionary" titled "Error", stop. It is compiled in on the Mac, so this cannot happen; keep the check in the sequencer's mark breakdown only if the implementation can fail (F84).
  2. Load the dictionaries (F84).
  3. Build the list of strings (`:31-56`): lines of the selection or of the whole text. Each line has the characters `. , ? ! "` removed. Then, if Alignment is `Phrase`, the line is one string. Otherwise (Alignment `Word`, `Phoneme` or nothing selected) it is split on whitespace, **keeping empty entries**, which are later skipped. Apostrophes and hyphens stay.
  4. Timing source: the chosen collection's mark times, or, when no start offset is selected (`mcIndex == -1`, as with Cursor and Clipboard), no marks and index 0 (`:127`, `:131-135`). Phoneme alignment applies only if Alignment reads `Phoneme` (`:128-129`). Alignment is only filled once the group has been enabled.
  5. For each non-blank string: `TryConvert(string.Trim())`. If empty, run the failure path (F82) with the string untrimmed and `TryConvert` the untrimmed string again. **Still empty: return at once, with nothing delivered and no message**, including phonemes already built for earlier words (`:146-161`). With "Clear after conversion" on the text may already have gone: `:179` runs after every string. **FIX** (proposed): clear only after delivery, so a bail does not lose the user's text. Veto keeps it as written.
  6. Timing for each string (F81), then one output per phoneme: `(start, duration, phoneme, lyric = the string)`. Start is `timing.start + timing.duration * i`, where `i` counts phonemes of that string; in Phoneme alignment a fresh timing per phoneme and `i` stays 0 (`:163-178`).
  7. Delivers placement Mark, Cursor or Clipboard by radio, and the first mark as the Start Offset's value (`:182-203`). Result handling: F83.
  8. With Per Mark Collection, advance Start Offset: by the number of phonemes (Phoneme), by the number of strings **including blanks** (Word), or 1 (Phrase), provided the new index is in range; otherwise leave it (`:205-224`). So repeated Converts walk down the song.

- [ ] **F81. Timing** (`CalcPhonemeTimespans`, `TCF:58-103`). Inputs: mark times or none, index, sub-element count. The list is sorted in place first.
  - Duration: `(marks[index+1] - marks[index]) / subElements` in integer ticks, if `index < count - 1`; otherwise 250 ms flat.
  - At index 0, an "unmarked" counter resets to 0.
  - Start: if `index < count`, `marks[index] - marks[0]` and the counter resets. Otherwise `lastMark + duration * counter`, with `counter += subElements`. With no marks `lastMark` is 0.
  - **Quirk:** past the end of the marks the start is measured from time zero using the absolute last mark, not relative to `marks[0]`, so if the first mark is not at zero there is a jump of `marks[0]` in the phrase timing. **KEEP**.
  - Even spacing (Cursor, Clipboard): each string starts where the counter says, phonemes 250 ms each, one after another. Example: "hello world" gives ETC, AI, L, O, WQ, E, L, ETC at 0, 250, ... 1750 ms, each 250 ms.

- [ ] **F82. Text Conversion Failure dialog.** (`LS/LipSyncTextConvertFailForm.cs`, `Form.cs:6192-6211`.) Modal, centred on parent, titled "Text Conversion Failure", client 446x320, minimum and maximum both 462x358, so it can't be resized (`LS/LipSyncTextConvertFailForm.Designer.cs:294`, `:320-321`). Top: two-line label `Unable to find mapping for <word>` / `Please map using buttons below`. A read-only text field with a `Clear` button to its right. Two rows of five image buttons, 56x50, each with the phoneme's picture at 45 px and a caption underneath (`docs/reference/forms/Vixen.Modules__App__LipSyncApp__LipSyncTextConvertFailForm.json`):
  - First row: AI, E, ETC, FV, L. Upstream's caption under the L button reads **"I"**, a typo (`LS/LipSyncTextConvertFailForm.Designer.cs:241`): **FIX** to "L".
  - Second row: MBP, O, REST, U, WQ.
  Each button appends `<NAME> ` to the field, so order and repeats are kept. Clear empties it. `OK` (default, bottom right) and `Cancel` (Esc). Images: `Vixen.Common/Resources/LipSync/{AI,E,etc,FV,L,MBP,O,rest,U,WQ}.jpg`, also at `crates/fx-audio-state/assets/`.

  OK: `AddUserMaping("<word> <text>")`, so the mapping is live and appended to the user dictionary (F85). OK with nothing chosen writes a word with an empty mapping and behaves as a bail. Cancel: nothing. The dialog does not validate the word. In Phrase alignment the "word" is the whole line, so the line's first token becomes the key and its other words are logged as corrupt phoneme tokens and dropped (`PU:663-688`). The retry then finds that key, so the line converts, and only its first word has the mapping. The whole line stays in the user dictionary, and each later load logs it again. **KEEP**, so the file stays what Windows would write.

- [ ] **F83. What Convert produces** (`textConverterHandler`, `Form.cs:6132-6185`; sequencer-shell F301). For each output with a nonzero duration, a LipSync effect with `StaticPhoneme` (the type parsed from the upper-cased phoneme), `LyricData` (the string), the start and duration, all at row offset 0, put on the timeline clipboard. Placement Cursor adds the cursor position to first mark. Placement not Clipboard pastes at first mark in default mode. If nothing pasted, including every Clipboard conversion: information alert "Conversion Complete and copied to Clipboard \n Paste at first Mark offset" titled "Convert Text". Modified, even if nothing pasted. Undo is the paste's (sequencer-shell section M).

- [ ] **F84. Dictionaries** (`PU:552-788`).
  - **Standard:** `LS/Resources/standard_dictionary`, CMU Pronouncing Dictionary 0.07: 133,910 lines, 3.7 MB, ASCII, `WORD  PH0 PH1` with two spaces, comments start `;;;`, alternates as `WORD(1)`. Bundle it in the Rust binary with its BSD-style notice, which is at the top of the file, and add a row to `docs/LICENSES.md`. Alternates are stored under their literal key and so are never looked up by a word: the first pronunciation wins.
  - **Load:** once per process, on first need (`PU:711-739`). Each line (`PU:663-688`): split on single spaces; skip when the first token is exactly `;;;`; key is the first token; later non-empty tokens map through the table below, with unknown tokens logged as `Phoneme Dictionary Corruption detected!, invalid Phoneme Type <t>` and dropped; the entry is stored, last write wins. **Empty entries are stored** (a key with no valid phonemes). Every token in the shipped dictionary is in the table (checked by script).
  - **Table** (`PU:564-661`):

    | Phoneme | Tokens |
    |---|---|
    | AI | AA, AA0-2, AE, AE0-2, AH, AH0-2, **AO** (no stress digit), AY, AY0-2, IH, IH0-2, AI |
    | O | **AO0-2**, AW, AW0-2, OW, OW0-2, O |
    | E | EH, EH0-2, ER, ER0-2, EY, EY0-2, IY, IY0-2, E21, E |
    | U | UH, UH0-2, UW, UW0-2, U |
    | FV | F, V, FV |
    | L | L |
    | MBP | B, M, P, MBP |
    | WQ | OY, OY0-2, W, WQ |
    | ETC | CH, D, DH, G, HH, JH, K, N, NG, R, S, SH, T, TH, Y, Z, ZH, ETC |
    | REST | REST |

    The plain names in each row are there so the user dictionary can hold them directly.
  - **`TryConvert(text)`** (`PU:747-773`): split on whitespace; upper-case each word; look up (`ToUpper` is the current culture's; use a plain per-character upper-case, since Rust's `to_uppercase` turns `ß` into `SS`, and only a user dictionary can notice); words not found are **skipped silently**. Within one word, consecutive equal phonemes collapse; across words they do not. So a phrase fails only when no word at all is known; a phrase with one unknown word silently loses it. **KEEP**.
  - **`RemovePunctuation`** removes exactly `. , ? ! "` (`PU:741-745`).
  - Performance: see Performance hazards. Parse off the main thread with `OnceLock`.

- [ ] **F85. User dictionary** (`PU:560`, `:706-709`, `:775-787`). A text file named `user_dictionary` in the profile's data root, same line format. Read once after the standard dictionary, so it overrides it. Written only by `AddUserMaping`: upper-cases the whole line, applies it at once, and appends `<newline><line>`, creating the file if needed, so the file begins with a blank line. Windows files have `\r\n`; read both, write `\n`. Note that the path is fixed at first use in upstream, so a profile switch keeps the old one; on the Mac take it from the active profile, and reload the dictionaries on a profile change. A write failure is uncaught upstream. **GUARD**: an alert, with the mapping still live for this run. No UI lists or edits this file. Not undoable. Not in the real show today (the file does not exist there): tests must treat it as optional.

- [ ] **F86. Phoneme breakdown.** Marks bar context menu items (timeline spec F137 and neighbours; sequencer-shell F299, `Form.cs:2909-3029`). `breakdown_phrase` and `breakdown_word` in `crates/vixen-session/src/timeline/marks.rs` have it, the second taking the word conversion as a callback. Wire that callback to `TryConvert(RemovePunctuation(text.trim()))`, showing F82 when empty and retrying. **GUARD**: empty after mapping, skip (upstream divides by zero, `Form.cs:2972`). The phoneme marks' text is the upper-case type name, as `ImageForPhoneme` and the LipSync effect expect.

### K. Papagayo

Upstream has two copies of the parser. `TSE/PapagayoDoc.cs` is the live one; the copy in `PU:9-550` is unreferenced (searched `src`) and should not be ported. The live enum is `AI, E, O, U, FV, L, MBP, WQ, etc, Rest` (`TSE/PapagayoDoc.cs:5`), so note the mixed case.

- [ ] **F87. `.pgo` format and parser** (`TSE/PapagayoDoc.cs:114-181`; sequencer-shell F404). Line based; each line has its leading whitespace trimmed. Integers via `Convert.ToInt32`.

  ```
  lipsync version 1                ← must contain "lipsync", case-sensitive, else "Invalid File Format"
  C:\path\to\sound.wav             ← unused
  24                               ← FPS
  1200                             ← sound frames
  1                                ← voice count, then for each voice:
    Voice 1                        ← name; its trimmed form is the dictionary key, and a duplicate throws; what shows keeps trailing spaces
    full text|with bars            ← unused ('|' becomes a newline)
    1                              ← phrase count, then for each phrase:
      hello                        ← text
      10                           ← start frame
      22                           ← end frame
      1                            ← word count, then for each word:
        hello 10 22 4              ← word, start, end, phoneme count (exactly 4 fields)
          10 etc                   ← frame, name (exactly 2 fields)
          13 AI
          16 L
          19 O
  ```
  - **End of file before the voice count is not an error.** The header loop just stops (`TSE/PapagayoDoc.cs:126-180`), so an empty file, or one that ends within the first four header lines, is a document with no voices: F32 then shows `0 voices imported as Mark Collections` and ends with nothing to select, and F88 shows `0 Voices imported to clipboard as separate rows`. **KEEP**.
  - End of file inside a voice, phrase or word header: `Corrupt File Format`. End of file within a word's phoneme list: leaves empty slots, which upstream later crashes on. **GUARD**: treat a missing phoneme as absent.
  - Any further line after the voices throws, so a file with **an extra blank line at the end fails** (`:177-178`). A single final newline is fine. **KEEP** the failure as an error alert.
  - A word line without exactly 4 fields gets no phoneme list and frames 0 to 0, which upstream crashes on in `Coalesce` if frame 0 is inside the phrase, and in F32's `Phonemes` always, where the crash becomes that import's error alert. **GUARD**: leave the word out, as `parse_papagayo` does.
  - A phoneme line without 2 fields is `Rest` at frame 0 and does not end the previous phoneme. Otherwise the previous phoneme ends at `frame - 1`, and the word's last phoneme ends at the word's end frame (`:501-504`, `:563-566`).
  - Phoneme names parse by `Enum.Parse`, case-sensitive: an unknown name is `Rest` (`:554-561`). **Quirk:** `Enum.Parse` also accepts a number, so a phoneme named `3` becomes `U`. **KEEP** only if cheap; such files are unknown.
  - Times are `f32`: sample = `(1 / fps) * 1000`; start = `sample * startFrame`; end = `sample * (endFrame + 1)`; duration = end - start (`:15-56`). Keep `f32` so the marks match Windows to the tick. FPS 0 gives infinity; **GUARD**: error alert.
  - FPS and frame count are static upstream, with one document at a time. Hold them in the document in Rust.
  - `Coalesce` for the phoneme list (`:311-348`): for each frame `0..soundFrames`, find the phoneme: the first phrase containing the frame, its first word containing the frame, its first phoneme containing the frame, otherwise a `Rest` for just that frame with empty lyric. Then merge runs of the same type, each keeping the first phoneme's text and lyric and the last's end frame. The last run is kept. Zero frames gives an empty list.

- [ ] **F88. Tools > LipSync > Papagayo Import** (to effects). (`Menu.cs:503-565`; sequencer-shell F91.) Open panel `Papagayo files (*.pgo)|*.pgo|All files (*.*)|*.*`, with no initial directory, so it doesn't use F27's last folder. Parse errors are uncaught upstream, which would crash: **GUARD** with an alert using the message of F32. One clipboard object accumulates across voices, with `FirstVisibleRow` 0 and `rownum` starting at 0, advancing for every voice even an empty one. For a voice with any phonemes in its `PhonemeList`, every phoneme with nonzero `DurationMS` becomes a LipSync effect: `StaticPhoneme` = the upper-cased type name parsed, `LyricData` = the word, start = `StartMS`, **duration = `DurationMS - 1` ms**, row offset = `rownum`, and earliest start tracked. The clipboard is rewritten after each such voice (`:549-550`), so it ends up holding all of them, and the sequence is marked modified. Then information alert titled "Papagayo Import": `<n> Voices imported to clipboard as separate rows\n\nRow #1 - <voice>\nRow #2 - <voice>...`, each row ending in a newline, so unlike F32's the text ends with one (`Menu.cs:558`). The user then pastes. No undo for the import itself. No marks are made here; the docker's import (F32) makes marks.

### L. Image maps (phoneme mapping editors)

The toolbar button and menu items: sequencer-shell F87, F88, F146.

- [ ] **F89. Tools > LipSync > Default Image Map.** Submenu rebuilt when opened, listing every map in library order with the default checked (`Menu.cs:491-501`). Clicking another sets it as default: all maps' flags clear, this one's set (`LML:61-70`). Marks the **sequence** modified even though the data is in the module store (`Menu.cs:481-489`), and that does **not** save the change: saving a sequence never writes the module store, only `SaveSystemAndModuleConfigAsync` and `SaveModuleConfigAsync` do, and the app no longer saves at exit (`Vixen.Application/VixenApplication.cs:403`, `Vixen.Core/Sys/VixenSystem.cs:115`). So a default chosen here is lost at exit unless something else in that run writes the module store. **FIX** (proposed): write the module store at once, as Edit Image Maps does (`Menu.cs:477`), and still mark the sequence modified. Veto keeps it as written. If no map is flagged the first in the library acts as default, and the flag is not written until set (`LML:37-59`).

- [ ] **F90. Image Maps window** (`LMS`). Tools > LipSync > Edit Image Maps, or the toolbar's LipSync button. Modal, centred on parent, titled "Image Maps", client 462x356, min 375x300, max 640x480; `KeyPreview`, so Enter means OK and Esc closes (`LMS:173-179`). A list with columns `Name` and `Notes`, sorted ascending by name, full row selection, multi-select, in-place editing of the name (`LS/LipSyncMapSelector.Designer.cs:88-99`, `LMS:17`, `:53-82`). To its right top to bottom: `New`, `Edit`, `Remove`, `Clone` (`docs/reference/forms/Vixen.Modules__App__LipSyncApp__LipSyncMapSelector.json`). `OK` at the bottom right; there is no Cancel button. Column widths auto-size to the longer of header and content on each rebuild and on resize (`LMS:219-222`, `:271-275`); a resize also rebuilds the rows, which drops the selection upstream, and native list behaviour needn't copy that. Enable rules (`LMS:238-243`): Edit, exactly one selected; Clone, one or more; Remove, one or more and more than one map in the library. Start disabled. Every action rebuilds the list, which drops the selection, so those three go off again until the next click (`LMS:84-114`).
  - **Quirk:** the code paints the default map's row bold but then immediately rebuilds the list, losing it (`LMS:100-113`, `:53-82`), so the default is not marked. **FIX** (proposed): show it bold as intended. Veto keeps it plain.
  - On close, if anything changed (F96): sequencer marks modified, marks every LipSync effect dirty and re-renders all rows, saves system and module config (`Menu.cs:468-479`, `Form.cs:6115-6130`). Closing with Esc behaves the same as OK.

- [ ] **F91. New.** Adds an empty map named `New Map`, or `New Map(<n>)` if that is taken, where `n` is a process-wide counter that only rises: first collision `New Map(1)`, next `New Map(2)` (`LML:13`, `:124-142`). Uniquing repeats on its own result, so after a restart, with `New Map(1)` already there and the counter back at 0, the name becomes `New Map(1)(2)`. The name goes through `string.Format`, so a name holding `{` or `}` throws (`LML:140`): **GUARD**, plain concatenation. Opens the editor (F97). Cancel removes the library entry again, **leaving the default pictures folder behind** (`LMS:181-192`, `LME:266-282`). **FIX** (proposed): delete the folder on Cancel if it did not exist before. `Changed` becomes the editor's result, overwriting any earlier true (see F96).

- [ ] **F92. Edit.** Button, or double-click on a row (`LMS:127-135`, `:224-227`). Needs exactly one selected. Opens the editor on a copy (F97).

- [ ] **F93. Remove.** Button or Delete key on the list (`LMS:137-160`, `:229-236`). Refuses silently with no selection or only one map. Confirm: "If you delete this mapping, ALL places it is used will be unlinked and will revert to the default Mapping. Are you sure you want to continue?" titled "Delete the mapping?", with the warning icon. Use a destructive "Delete" and "Cancel". On confirm each selected map is removed **with its picture folder** (`LML:157-183`). If the folder can't be deleted: error "Unable to mapping data!\n\n<message>" (sic) titled "Error!" and the map stays. **FIX** wording to "Unable to remove mapping data!". If the default was removed, the default falls to the first flagged, else first in the library. Selecting all and removing empties the library, which upstream tolerates. **GUARD**: keep at least one, since LipSync needs a map; refuse with the same silence as the one-map case. Effects naming a missing map fall back to the default at their next render, and the effect's data is rewritten to the default's name (`Vixen.Modules/Effect/LipSync/LipSync.cs:84-87`). **Backend gap:** `crates/fx-audio-state/src/lip_sync.rs:6-7`, `:320-326` has no library, so a missing folder there means a dark effect. See Rust API, `resolve_lipsync_mapping`.

- [ ] **F94. Clone.** Button. Each selected map in turn (`LMS:194-206`, `LML:237-259`): a copy under the same name made unique by F91's rule, so `Faces` becomes `Faces(1)`, with `IsDefaultMapping` false. Copies all files of its folder to the new folder, deleting that folder first if it exists. If the source has `UsingDefaults`, the default pictures are written to its folder first, because the file-copying object runs the editor's constructor (F98). Copy failure: error "Unable to copy mapping data!\n\n<message>" titled "Error!", returns blank, and **the library entry stays anyway**. **KEEP**. `Changed` true on success.

- [ ] **F95. Rename in place.** Clicking a selected name begins editing; Enter accepts, Esc cancels. While editing all four buttons disable; afterwards only `New` is enabled, until selection changes (`LMS:245-269`). A non-null, non-empty new label: `CloneLibraryMapping(old, new)` then `RemoveMapping(old, removeFiles: true)` (`LML:220-235`). So the new name is made unique rather than refused, and **a renamed default loses its default flag** because the clone clears it (`LMD:33-43`). **FIX** (proposed): keep the default flag through a rename, here and in F105. Effects still naming the old map fall back to the default at next render. An empty new label does nothing, and neither does Enter on an unchanged one, which the list reports as no label. `Changed` becomes the result, overwriting earlier true. Where this goes wrong: F118.

- [ ] **F118. Renames that lose data.** Three paths through F95 upstream, each **GUARD**:
  - **A failed copy still deletes the original.** `RenameLibraryMapping` ignores the clone's result and goes on to `RemoveMapping(old, true)` (`LML:229-231`), so a copy error shows F94's message and then the pictures are gone. Keep the old map and its folder when the copy fails.
  - **A case-only rename destroys the pictures.** `Faces` to `faces` is a new library key, so nothing uniquifies it. On a case-insensitive volume, which NTFS and default APFS both are, `CloneMappingFiles` finds the "target" folder, which is the source, and deletes it before copying (`LME:109-120`); the source then no longer exists, so nothing is copied, and `RemoveMapping` removes what's left. The map survives with no pictures, and its effects go dark. Rename through a temporary name.
  - **Odd labels reach the path.** No character check here, as in F105: a path separator or `..` goes into the folder path (`LME:109`). Refuse them with the same alert as F105. A label of only spaces passes the empty test and then becomes `New Map`, or a uniquified one, through `AddMapping`'s blank rule (`LML:130-133`). That one is harmless: **KEEP**.

- [ ] **F96. The Changed flag.** Assignments in Edit, New and Rename overwrite the flag instead of or-ing it (`LMS:132`, `:186`, `:254`), so a Remove followed by a cancelled Edit reports "no change", and the removal is not saved or re-rendered until something else saves. **FIX**: sticky. No veto expected; this is a data-loss bug.

- [ ] **F97. Image Map editor** (`LME`). Modal over Image Maps, titled "Image Map", client 258x384, fixed dialog, not in the dock; AllowDrop on the whole window (`LS/LipSyncMapMatrixEditor.Designer.cs:222`, `LME:55`). Top to bottom:
  - `Name` label and field.
  - Phoneme navigator: `<` button, the phoneme's 48x48 picture in the middle with its name above (`phonemeLabel`), `>` button.
  - The mapped image on the left, with buttons `File`, `Edit`, `Clear` stacked to its right.
  - `Notes:` label and multi-line field.
  - `OK` (default) and `Cancel` (Esc) at the bottom right.
  Upstream positions itself relative to the active form (`LME:48`) and resizes the window to fit each image (`:418-427`): on the Mac use a sheet with a fixed 128x128 image area. Works on a copy of the map (`:51`); nothing but the defaults (F98) touches disk until OK. No undo.

- [ ] **F98. Default pictures.** If the map's `UsingDefaults` is true, then **as soon as the editor object is constructed**, the ten bundled `*_Transparent.png` images are saved to the map's folder and the flag clears in the copy (`LME:266-282`). So opening and cancelling still writes them, whatever the folder held, and a map whose editor has never opened has **no pictures at all**: a LipSync effect using it stays dark (`crates/fx-audio-state/src/lip_sync.rs:286-287`). The owner's `New Map` is in that state (`UsingDefaults` true, no `Module Data Files/LipSync` folder). Because that save copies the notes box before the box has been filled (`LME:306`, `:292-293`), the copy's notes also read blank, and OK then saves them blank. **KEEP** the write-on-open, and the blank notes, which only a map still on its defaults can meet. Images: `Vixen.Common/Resources/LipSync/{AI,E,etc,FV,L,MBP,O,rest,U,WQ}_Transparent.png`. Bundle them.

- [ ] **F99. Phoneme navigation.** `<` and `>` step through `AI, E, ETC, FV, L, MBP, O, REST, U, WQ`, wrapping both ways, starting at AI (`LME:214-227`, `:249-262`, `:284-285`). `Unknown` is never shown. Icon: the phoneme's reference picture (`Common.Resources` jpg) scaled to 48x48 (`:237-244`). The name label shows the phoneme.

- [ ] **F100. Mapped image.** Shows the current phoneme's image scaled, kept aspect, between 96 and 128 on each side (`LME:376-428`). With no image: the tiled `NoImage.png` (`:429-434`). Double-click: `File` if there is no image, `Edit` if there is (`:364-374`).

- [ ] **F101. File and drop.** `File` opens a multi-select open panel, filter `Image Files (*.bmp, *.jpg, *.png)|*.bmp;*.jpg;*.png|All Files(*.*)|*.*` (`LME:464-474`). Dropping files on the window accepts only when all have `.jpg`, `.png` or `.bmp` extensions, any case (`:476-497`). **Quirk:** the code means to allow GIF but compares `"gif"` with a dotted extension, so GIF drops are refused. **KEEP**. One file: replaces the current phoneme's image. More than one: Image Mapper (F102) (`:525-562`). A file that won't load is logged and the old image stays, with no message (`:445-461`).

- [ ] **F102. Image Mapper dialog** (`LS/LipSyncMultiPicSelect.cs`). Modal, fixed, centred on parent, titled "Image Mapper", client 245x401. Label: "Multiple images detected, \nPlease select the desired mapping". Ten rows in order `AI, E, ETC, FV, L, MBP, O, REST, U, WQ`, each a label and a drop-down with `<Unmapped>`, `<Mapped>`, then every dropped file by file name, extension included, in drop order (`LS/LipSyncMultiPicSelect.cs:42-48`). `Accept` (default) and `Cancel` (Esc).

  Initial choices (`:55-91`): walking the names in that same sorted order: the current phoneme takes the first file and starts the run; each following phoneme takes the next file while any remain; every other phoneme takes `<Mapped>` if it has an image, else `<Unmapped>`. So files go in from the current phoneme on; earlier phonemes are not touched, and surplus files are unused. Accept: a file sets that phoneme; `<Unmapped>` clears it; `<Mapped>` leaves it alone, even on a phoneme with no image (`:94-122`, `LME:546-557`).

- [ ] **F103. Edit button.** Upstream saves the current image to `<phoneme>_tmp.bmp` in the map's folder, disposes the in-memory copy, runs `mspaint.exe` on it and **blocks until Paint exits**, reloads it, deletes the temp file and redraws (`LME:581-623`). With no image it opens Paint with a name that doesn't exist yet. If the reload fails the in-memory image is left disposed, a latent crash (`:595-599`, `:608-620`). Mac version: write the temp PNG, open it with `NSWorkspace` in the default editor, show a non-blocking "Editing in <App>" state with `Done` in the sheet, reload on `Done` (or on file change, with `DispatchSource`), and delete the temp. With no image, make a blank 128x128 transparent PNG first. Keep the in-memory image until the reload succeeds (**GUARD**). Since `NSWorkspace` can't wait for a process to end, this is an adaptation; see Windows-only omissions.

- [ ] **F104. Clear button.** Removes the current phoneme's image from the copy; shows the tile (`LME:564-579`). Reaches disk at OK.

- [ ] **F105. Name, Notes and OK.** (`LME:301-347`, `:625-666`.) Name starts as the map's name, Notes as its notes. On OK:
  1. **Rename the folder** when the name differs: the target folder existing gives "Can not rename, this new name already exists!" titled "Error!" and the name reverts, while OK **carries on with the old name**. An IO failure gives "Can not set new name!\n\n<message>" and reverts too. No validation of characters: a name containing `/` or other path separators is unsafe, and upstream's other exceptions are uncaught. A blank name reaches the exists error, since the target is then the parent folder. **GUARD**: reject names with path separators, or that are `.` or `..`, with the "Can not set new name!" alert. APFS by default is case-insensitive, as NTFS is, so a case-only rename hits the exists error in both.
  2. For each of the eleven phoneme types including `Unknown`: delete `<PHONEME>.bmp`; if an image is in memory, save it. `Save(path)` with no format keeps the source's raw format, or PNG for an in-memory bitmap, so **`.bmp` files can hold PNG or JPEG bytes**: read by sniffing, as `Bitmap::from_file` does with `image::load_from_memory`. Write PNG data to `<PHONEME>.bmp`, which all readers accept. The file name's case comes from the enum name, upper-case.
  3. `IsMatrix` is set to true if at least one image saved, else false; then `AddMapping` forces it true regardless. Errors are logged only.
  4. Clear the map's image cache (`LMD:119-127`).

  Back in the library (`LML:185-218`): if the name changed and another library entry has it (only reachable if it has no folder, since step 1 catches folders): question "Overwrite existing <name> mapping?" titled "Map exists", with OK and No (on the Mac: "Overwrite", "Keep Both"). OK: remove the old entry, add under the new name, replacing. No: keep the old entry too and add the edited one under a uniquified name, though the folder has already moved, so the old entry then has no pictures. An unchanged name, or a new name no entry has: remove the old entry and add the edited copy. A .NET `Dictionary` reuses the slot a removal frees, so that keeps the map's place in the library's order, whereas F95's rename adds first and so moves it. Keep that order in Rust by replacing in place, not removing and pushing: with no map flagged, "first in the library" is the default (F89), so moving an entry can change which map is default. Either way the entry is an edited **clone, so `IsDefaultMapping` is false**: **editing the default map silently loses the default** (`LMD:33-43`, `LME:51`, `:83-89`, `LML:166-169`). **FIX** (proposed) as in F95. `IsCurrentLibraryMapping` true. Notes are written.

  Cancel: the copy is dropped; the name and images stay as on disk, apart from F98's defaults.

### M. Face mapping and node selection

- [ ] **F106. LipSync Mapping Node Select** (`LS/LipSyncNodeSelect.cs`). Tools > LipSync > Edit Element Face Mapping (`Menu.cs:584-607`, sequencer-shell F89). Modal, centred on parent, titled "LipSync Mapping Node Select", client 675x433, no minimise or maximise (`docs/reference/forms/Vixen.Modules__App__LipSyncApp__LipSyncNodeSelect.json`). Three columns: element tree on the left, multi-select; a centre stack of `->`, `<-`, `Reset`, the hidden "Strings are:" group, `Allow Groups`, `Add Recursively`; on the right the chosen list, multi-select. `OK` (default) and `Cancel` (Esc) bottom right. This call sets `MaxNodes` unlimited, `MatrixOptionsOnly` false (hiding "Strings are:" with its Horizontal and Vertical radios and showing Allow Groups), `AllowGroups` on, `AllowRecursiveAdd` off. It is the only caller in `src`. So the "Strings are:" group, its closing confirmation "Changing Matrix Orientation will modify existing matrix data!..." (`LS/LipSyncNodeSelect.cs:276-294`) and `MatrixOptionsOnly` true are unreachable: omit, and record it in parity.

- [ ] **F107. Element tree.** Root elements, with children loaded on expand (`LS/LipSyncNodeSelect.cs:102-155`). Use the VixenKit tree (`NSOutlineView`). Multi-select with ⌘ and ⇧.

- [ ] **F108. Add (`->`).** For each selected tree row (`LS/LipSyncNodeSelect.cs:246-253`, `:200-244`): find **every element named the same**, by name over all elements (names are unique in practice), and add it if Allow Groups is on or it is a leaf; with Add Recursively on, do the same for all descendants. A name already in the chosen list is skipped; nothing is added past `MaxNodes`. **KEEP** name matching in effect but use node ids in Rust, per the F190 note in the property spec. Order is add order.

- [ ] **F109. Remove (`<-`) and Reset.** `<-` removes the selected rows of the chosen list. `Reset` empties it. No confirmation (`LS/LipSyncNodeSelect.cs:193-198`, `:255-263`).

- [ ] **F110. Allow Groups and Add Recursively.** Checkboxes; in this call Allow Groups starts on and Add Recursively off. Toggling Allow Groups, before the user's first add or remove: clears the chosen list and re-adds any preset names (none here), so effectively clears it (`LS/LipSyncNodeSelect.cs:265-274`). After that it only affects later adds.

- [ ] **F111. OK.** With at least one chosen, runs the Face Setup helper on the chosen nodes (`docs/spec/property-filter-controller-setup.md` F184-F192, `Vixen.Modules/Property/Face/FaceSetupHelper.cs`, Rust in `crates/vixen-session/src/dialogs/face.rs`). If that returns OK: re-renders all LipSync effects (`RenderLipSyncElementsAsync`, `Form.cs:4124-4142`) and saves system and module config (`Menu.cs:597-604`). OK with nothing chosen does nothing.

- [ ] **F112. Face property** (`Vixen.Modules/Property/Face/`). `FaceData` holds `PhonemeList` (name to bool, case-insensitive), `FaceComponents` (`Mouth`, `Outlines`, `EyesOpen`, `EyesClosed` to bool), `DefaultColor` default white (`FaceData.cs:9-14`, `FaceComponent.cs:5-15`). TypeId `EDA73851-8350-4D3F-9DF2-0D3B34E70BB6`, TypeName "Face". `SetDefaultValues` empties both and white; `CloneValues` copies all three (`FaceModule.cs:14-37`). `HasElementSetupHelper` is true and runs the helper (`:101-108`). `PhonemeState`, `IsFaceComponentType` default false (`:86-96`). `ConfiguredColorAndIntensity` gives `(1, DefaultColor)`, no intensity control (`:71-74`). `FaceMapItem` is a leftover type. Its one live use is the empty one Face Setup makes, which is why `Mouth` always saves false (`Vixen.Modules/Property/Face/FaceSetupHelper.cs:71`, `:102`, property spec F190). `ConfiguredIntensity` takes one as a parameter and ignores it. Everything is already in `crates/vixen-session/src/dialogs/face.rs`; **this spec adds only the tie-in in F111**.

- [ ] **F113. Old string maps migrate to Face properties.** On app load, if any library map has `IsMatrix` false (`LML:15-18`, `:282-292`, `LS/LipSyncMapStaticData.cs:48-90`): for each such map and item, find the element by `ElementGuid`, skipping if gone; get or add its Face property; set `DefaultColor` from `ElementColor`, copy `PhonemeList` case-insensitively and `FaceComponents`; remove the map from the library. Matrix maps have `MapItems` nulled. Old items without `FaceComponents` get `Mouth` = any phoneme true on deserialize (`LS/LipSyncMapItem.cs:78-94`). The real show has one matrix map with a leftover empty item, so this is not exercised there. Note that a removed map's effects then fall to the default. Administration spec's table lists the same load hook. Module store version 3 to 4 is already in `version_3_to_4`, `crates/vixen-io/src/module_store.rs`, except for painting bitmaps from drawings, recorded there.

- [ ] **F119. The library when nothing is stored, and what old sequences do to it.** With no `_library` in the module store, the first read makes one map named `New Map` with the constructor's values and nothing on disk (`LS/LipSyncMapStaticData.cs:24-39`, `LMD:15-21`). So the library is never empty on a fresh profile, and F98 applies to that map. Opening a sequence from before data version 5 runs a migration that clears `UsingDefaults` on any matrix map one of its LipSync effects names (`Vixen.Modules/Sequence/Timed/TimedSequenceMigrator.cs:713-731`), so such a map's pictures aren't overwritten by F98. That change sits in the live library until something writes the module store. `crates/vixen-io/src/sequence_migrate.rs` (`migrate_lip_sync_4_to_5`) does the effect's half and not this one. **KEEP**: apply it when the migration reports the names it saw.

### N. Quirk register

Upstream oddities in one place, for the owner's review. Tags as defined at the top of Features, plus INFERRED for something this spec concluded that no line of code states. A FIX here is listed for veto; if vetoed, port it as written.

| Tag | F | What |
|---|---|---|
| KEEP | F8 | Lines, Bar and Lock headers apply a remembered next value, not the inverse of what shows |
| FIX | F8 | Tail lines header starts inverted, so its first click does nothing visible |
| KEEP | F14 | Name colour has no contrast adjustment |
| KEEP | F17 | Deleting a collection leaves other collections' links dangling |
| KEEP | F18 | Offset Time is not clamped, so marks can go below zero or past the end |
| FIX | F18 | Offset Time does not set Modified |
| FIX | F22, F23 | Re-clicking the checked Weight or Type unchecks it until the panel rebuilds |
| KEEP | F23 | Changing type keeps a now-hidden link |
| FIX | F24 | Changing a link does not set Modified |
| KEEP | F26 | "Audacity Beats" runs the bar reader; "Vamp Beat Tracker: Beats" runs the beat reader |
| KEEP | F27 | Audacity and Pangolin export panels ignore the last folder |
| KEEP | F28 | Importing a `.v3m` can create a second collection with the same id |
| GUARD | F29, F30, F31, F39 | Numbers use the machine's culture upstream; use invariant |
| GUARD | F30 | More than six beats or beat 0 throw upstream; report the same error |
| KEEP | F30 | A file with no matching line makes nothing and says nothing |
| KEEP | F29 | Space-separated label text keeps only its first word |
| KEEP | F31 | An empty xTiming layer does not advance the layer number, shifting the Phrase, Word and Phoneme roles |
| KEEP | F32 | Papagayo says "imported" before the selection dialog |
| GUARD | F36 | Blank candidate name throws upstream; use `Mark Collection` |
| FIX | F38 | `.v3m` export failure is only logged; show an alert |
| GUARD | F39, F40 | Export write errors are uncaught; show an alert |
| KEEP | F44 | Select stays enabled for a song with no timing link |
| KEEP | F49 | The bulk-move clip at the sequence end lengthens the mark (`Duration += EndTime`); correct is `-=` overshoot. Owner may veto |
| INFERRED | F56 | Add Layer's filter comes from file enumeration order; Chroma Key on NTFS. Confirm on Windows |
| FIX | F55 | Default layer of a new sequence is not shared between sequences |
| KEEP | F58 | Layer names have no validation, and the default layer can be renamed |
| FIX | F58, F59, F60 | Layer rename, quick rename and type change do not set Modified (sequencer-shell F330) |
| KEEP | F60 | Changing filter type discards its settings |
| KEEP | F65 | Missing keys inside `filterData` keep the constructor's default |
| GUARD | F65 | A `null` entry in `layers` throws upstream; report it as invalid JSON |
| FIX | F67 | Cancelling Beat/Bar Detection still marks modified |
| KEEP | F68 | Raw sample conversion, with half-time marks on 16-bit mono |
| FIX | F69 | Group title "Divisons" |
| KEEP | F71 | Beat Splits can be on with one division, giving half the names |
| FIX | F72 | Colour picker starts at its default, not the swatch |
| KEEP | F72 | Generating replaces same-named collections with no warning, under new ids; `Beat #1` is usually empty; blank base name allowed |
| GUARD | F74 | Alt+F4 on the progress window crashes upstream; Cancel button |
| FIX | F69 | Return and Esc do nothing in Beats and Bars; Esc cancels on the Mac |
| FIX | F70 | The staff's right-click menu appears only after a first click, and can go stale |
| GUARD | F75 | Missing output throws upstream, after already removing the old collection; alert and no change |
| KEEP | F78 | Alignment stays in force, though greyed, after leaving Per Mark Collection |
| KEEP | F80 | A word the user declines to map abandons the whole conversion silently |
| FIX | F80 | "Clear after conversion" clears before delivery, losing text on a bail |
| KEEP | F81 | Past the last mark, start uses the absolute last mark, so a first mark above zero causes a jump |
| FIX | F82 | L button's caption reads "I" |
| KEEP | F84 | Words not in the dictionary drop silently from a phrase; `AO` is AI while `AO0-2` are O |
| KEEP | F82 | In Phrase alignment a mapping's key is the line's first token |
| GUARD | F85 | A failed write to the user dictionary is uncaught |
| GUARD | F86 | Divide by zero when a mapping converts to nothing |
| KEEP | F87 | An extra blank line at the end of a `.pgo` fails; `Enum.Parse` accepts numbers; end of file before the voice count is no voices, not an error |
| GUARD | F87 | Truncated word lists and odd word lines crash upstream; FPS 0 |
| GUARD | F88 | Tools > Papagayo Import does not catch parse errors |
| FIX | F89 | A default map choice isn't saved unless something else writes the module store |
| FIX | F90 | Default map's bold row is immediately lost |
| FIX | F91 | Cancelling New leaves a folder of default pictures |
| GUARD | F91 | A map name with braces throws in `string.Format` |
| FIX | F93 | Message "Unable to mapping data!" |
| GUARD | F93 | Selecting all and removing empties the library |
| KEEP | F94 | A clone whose file copy failed stays in the library |
| FIX | F95, F105 | Renaming or editing the default map loses the default |
| FIX | F96 | `Changed` is overwritten, not accumulated, so a removal can go unsaved |
| KEEP | F98 | Default pictures are written on editor open, even if cancelled; a never-opened map has none; such a map's notes read blank |
| KEEP | F101 | GIF drops are refused through a typo |
| GUARD | F103 | Failed reload leaves a disposed bitmap in the map |
| GUARD | F105 | Name with path separators, or blank |
| KEEP | F45 | A remembered time past a shortened sequence shows until that field is edited |
| GUARD | F49 | Bulk Effects Move with marks crashes on an empty unlocked collection |
| FIX | F115 | With two sequencers open, rows use the first one's collections |
| FIX | F117 | Layers made with Add Layer share one filter and its settings, through save and load |
| GUARD | F118 | Map rename: a failed copy deletes the original, a case-only rename destroys the pictures, odd labels reach the path |
| KEEP | F119 | Opening a pre-version-5 sequence clears `UsingDefaults` on the maps it names |

### O. Control index for parity

Every designer or XAML control and its entry, for `docs/parity/`. Names are upstream's.

| Window | Control | F |
|---|---|---|
| TimedSequenceEditorForm | `markWindowToolStripMenuItem`, `mixingFilterEditorWindowToolStripMenuItem` | F114 |
| | `beatBarDetectionToolStripMenuItem`, `divideMarksEvenlyToolStripMenuItem` | F67, F45 |
| | `defaultMapToolStripMenuItem`, `phonemeMappingsToolStripMenuItem`, `toolsToolStripButton_LipSync` | F89, F90 |
| | `changeMapToolStripMenuItem`, `lyricConverterToolStripMenuItem`, `papagayoImportToolStripMenuItem` | F106, F76, F88 |
| Form_Marks | form, `host` | F1, F2 |
| MarkDockerView | `AddCollectionCommand` button | F4 |
| | `ImportCollectionCommand` button | F5, F25 |
| | `ExportCollectionCommand` button | F6, F37 |
| | `target_button` | F3 |
| | Header cells: Pencil, MarkLineVisible, MarkTailLineVisible, MarkBarVisible, locked, "Collection" | F7, F8 |
| MarkCollectionView | `RadioButton` IsDefault | F9 |
| | `ShowGridLines` | F10 |
| | Tail `CheckBox` | F11 |
| | ShowMarkBar `CheckBox` | F12 |
| | Locked `CheckBox` | F13 |
| | `ReadOnlyText` | F14, F15, F16 |
| | `EditableText`, `NameValidationText` | F15 |
| | Menu: Delete, Offset Time, Solid Line, Bold, Color, Weight, Type, Linked To | F17, F18, F19, F20, F21, F22, F23, F24 |
| MarkCollectionImportDialog | `radioBars`, `radioBeats`, `radioAudacityBeats`, `radioVixen3Beats`, `radioXTiming`, `radioPapagayo`, `radioTimingTrackBrowser`, `radioPangolinBeyond`, `btnOk`, `btnCancel` | F25, F26 |
| MarkCollectionImportSelectionWindow | intro text, options list, row checkbox, row name, caution image, OK, Cancel | F35 |
| MarkExportWindow | heading, include header, `TextIncludedOptionHeader`, "Collection" header, rows, "Export Format", Vixen, Audacity, Pangolin Beyond, Ok, Cancel | F37 |
| MarkCollectionExportRowView | `IncludedOption`, `TextIncludedOption`, `ReadOnlyText` | F37 |
| VendorInventoryWindow | Provider combo, `ProviderDetails`, splitter, status bar | F41 |
| | `SongInventoryTree` | F43 |
| | Song Info tab: Song link, Title, Artist, Creator, Select, disclaimer | F44 |
| CreateEvenMarksForm | `labelStart`, `txtStartTime`, `labelEnd`, `txtEndTime`, `labelDivide`, `updownDivide`, `btnOk`, `btnCancel`; `toolTip` has no entries | F45 |
| MarkTapper | all | F48, omitted |
| AutomaticMusicDetection | all | F48, omitted |
| LayerEditor, LayerEditorView | form | F53, F54 |
| | Import and Export buttons | F65, F63 |
| | Add Layer, Remove Layer, `_lbLayers` | F56, F57, F58, F62 |
| | Expander, Name, quick rename, Type, Configuration | F58, F59, F60, F61 |
| BeatsAndBarsDialog | `grpDivisions`, `PreviewGroupBox`, `groupBox2` | F69 |
| | `GenerateButton`, `CancelBtn` | F69 |
| | `musicStaff1` | F70 |
| | `label1`, `BeatsNameTB`, `BarsCB`, `BarsColorPanel`, `BeatCountsCB`, `BeatCountsColorPanel`, `BeatSplitsCB`, `BeatSplitsColorPanel`, `AllFeaturesCB`, `AllColorPanel` | F72 |
| MusicStaff | `staffPictureBox`, `tsLabel`, `BeatsPerBarLabel`, `NoteSizeLabel`, `splitBeatsCB`, `BPMLabel`, `BPMLabelVal`, `BarPeriodLabel`, `BarPeriodLabelVal`, `DivTimeLabel`, `DivTimeLabelVal`, context menu | F70, F71 |
| PreviewWaveform | `_waveform` | F73 |
| BeatsAndBarsProgress | `generateLabel`, `progressBar1`, `percentLabel` | F74 |
| LipSyncTextConvertForm | `groupBox1`, `cursorRadio`, `clipBoardRadio`, `markCollectionRadio` | F77 |
| | `marksGroupBox`, `markCollectionLabel`, `markCollectionCombo`, `markStartOffsetLabel`, `startOffsetCombo`, `markAlignmentLabel`, `alignCombo` | F78 |
| | `label3`, `checkBoxClearText`, `textBox` | F79 |
| | `buttonConvert` | F80 |
| LipSyncTextConvertFailForm | `errorLabel`, `phonemeTextBox`, `buttonClear`, ten phoneme buttons, `label1` to `label10`, `buttonOk`, `buttonCancel` | F82 |
| LipSyncMapSelector | `mappingsListView` | F90, F92, F93, F95 |
| | `buttonNewMap`, `buttonEditMap`, `buttonDeleteMap`, `buttonCloneMap`, `buttonOK` | F91, F92, F93, F94, F90 |
| LipSyncMapMatrixEditor | `nameLabel`, `nameTextBox`, `notesLabel`, `notesTextBox`, `buttonOK`, `buttonCancel` | F105 |
| | `phonemeLabel`, `phonemeIcon`, `prevPhonemeButton`, `nextPhonemeButton` | F99 |
| | `renderedPicture`, `tableLayoutGridArea`, `flowLayoutImportExportButtons` | F100 |
| | `openButton`, `editButton`, `clearButton` | F101, F103, F104 |
| LipSyncMultiPicSelect | `infoLabel`, ten labels, ten combos, `acceptButton`, `cancelButton` | F102 |
| LipSyncNodeSelect | `nodeTreeView` | F107 |
| | `buttonAdd`, `buttonRemove`, `buttonReset` | F108, F109 |
| | `allowGroupsCheckbox`, `recurseCB` | F110 |
| | `stringsGroupBox`, `rowsRadioButton`, `colsRadioButton` | F106, omitted (unreachable) |
| | `chosenTargets`, `buttonOk`, `buttonCancel`, `panel1` | F106, F111 |
| FaceSetupHelper | all | `docs/spec/property-filter-controller-setup.md` F184-F192 |

## Data and persistence

### What is stored where

| Data | Where | Format | Read | Written |
|---|---|---|---|---|
| Mark collections and marks | Sequence file, `LabeledMarkCollections` | `DataContractSerializer` XML | On open. Done: `decode_marks`, `mark_collection_from_xml` in `crates/vixen-io/src/sequence.rs` | Save. Done: `mark_collection_to_xml` |
| Old mark collections | Sequence file, `MarkCollections`, when `LabeledMarkCollections` is absent | Same | On open, converted (`Vixen.Modules/Sequence/Timed/TimedSequenceData.cs:54-106`). Done: `convert_old_marks`, same file | Never |
| Layers, effect-to-layer map, filter data | Sequence file: `SequenceLayers`, `_layerMixingFilterSurrogates`, `_layerMixingFilterDataModels` | Same | On open. Done: `decode_layers`, same file | Save. Done |
| Docked panel layout | Sequencer preferences | See sequencer-shell F2; `crates/vixen-session/src/sequencer/panels.rs` | Window open | Window close |
| Lip sync map library | `SystemData/ModuleStore.xml`, the module `a475c535-73fa-4d31-a1bc-ed2b50ad21cf` | `DataContractSerializer` XML, below | App load. Migration of old versions done in `version_3_to_4`, `crates/vixen-io/src/module_store.rs`; **the library model is not** | After Image Maps closes with a change, and after Face Mapping OK (`Menu.cs:468-479`, `:597-604`). Default-map changes wait for the next system save, which may never come (F89) |
| Map pictures | `<profile>/Module Data Files/LipSync/<map name>/<PHONEME>.bmp` | Any image bytes, sniff on read (F105) | Editor open; LipSync render | Editor construction when `UsingDefaults` (F98); editor OK (F105); clone and rename (F94, F95) |
| Face properties | Per element, with the system config | See `crates/vixen-session/src/dialogs/face.rs` | System load | Face Mapping OK |
| User phoneme dictionary | `<profile>/user_dictionary` | Text: `WORD PHONEME PHONEME...` per line, `\n` or `\r\n` | First conversion | Each mapping, appended at once (F85) |
| Standard dictionary, default pictures, staff and note images, phoneme images | App bundle / Rust binary | Copied from `LS/Resources/standard_dictionary`, `Vixen.Common/Resources/LipSync/`, `BB/Resources/` | On use | Never |

**Not persisted, by design.** Header toggle states (F8), last folder (F27), Beats and Bars settings (F69), lyric converter window state (F76), layer row expansion (F58), the selection in any list.

### Sequence XML, real shape

From the owner's `Zoom.tim`, with `Locked` and `ShowTailGridLines` added where the current writer puts them: that older file has neither. Missing means false, because `DataContractSerializer` does not run constructors, and missing `Level` is 0 for the same reason. Prefixes are the writer's choice; readers go by namespace.

```xml
<LabeledMarkCollections xmlns:d1p1="http://schemas.microsoft.com/2003/10/Serialization/Arrays">
  <d1p1:anyType xmlns:d2p1="http://schemas.datacontract.org/2004/07/VixenModules.App.Marks" i:type="d2p1:MarkCollection">
    <d2p1:CollectionType>Generic</d2p1:CollectionType>
    <d2p1:Decorator i:type="d2p1:MarkDecorator">
      <d2p1:Color xmlns:d4p1="http://schemas.datacontract.org/2004/07/System.Drawing">
        <d4p1:knownColor>164</d4p1:knownColor><d4p1:name i:nil="true" /><d4p1:state>1</d4p1:state><d4p1:value>0</d4p1:value>
      </d2p1:Color>
      <d2p1:IsBold>false</d2p1:IsBold>
      <d2p1:IsSolidLine>false</d2p1:IsSolidLine>
    </d2p1:Decorator>
    <d2p1:Id>763f55a4-4769-49ad-bba6-e23c534d01c6</d2p1:Id>
    <d2p1:IsDefault>true</d2p1:IsDefault>
    <d2p1:Level>1</d2p1:Level>
    <d2p1:LinkedMarkCollectionId>00000000-0000-0000-0000-000000000000</d2p1:LinkedMarkCollectionId>
    <d2p1:Locked>false</d2p1:Locked>
    <d2p1:Marks>
      <d1p1:anyType i:type="d2p1:Mark">
        <d2p1:Duration>PT0.45S</d2p1:Duration>
        <d2p1:StartTime>PT0.069S</d2p1:StartTime>
        <d2p1:Text></d2p1:Text>
      </d1p1:anyType>
    </d2p1:Marks>
    <d2p1:Name>Beats - All</d2p1:Name>
    <d2p1:ShowGridLines>true</d2p1:ShowGridLines>
    <d2p1:ShowMarkBar>false</d2p1:ShowMarkBar>
    <d2p1:ShowTailGridLines>false</d2p1:ShowTailGridLines>
  </d1p1:anyType>
</LabeledMarkCollections>
```

Member order is alphabetical, and matters to .NET's reader. `mark_collection_to_xml` in `crates/vixen-io/src/sequence.rs` writes this; `mark_collection_from_xml` reads it. Field names map to `vixen_core::marks`: `ShowMarkBar` is `show_marks`, `ShowTailGridLines` is `show_tail_lines`, `Locked` is `is_locked`, an empty `LinkedMarkCollectionId` is `None`. `MarkDecorator::compact_mode` has no upstream field.

Two places where the Rust model and upstream meet:

- `mark_collection_from_xml` applies upstream's load rule through `Mark::apply_minimum_duration`: a duration under 25 ms becomes 25 ms, apart from a mark with start and duration both zero (`Mark.cs:43-48`, `:60-65`, F49). Upstream never writes less, so only hand-edited files differ.
- `Mark.id` is invented on each load, because marks have no id in the file. Do not persist or compare it across loads.

### `.v3m` file

**No `.v3m` sample exists in the repo or the owner's show, so this shape is inferred** from the serializer's settings (`MIES:200-208`, `:598-612`) and the real element above. Check it against a file from a Windows Vixen before calling it done. UTF-8, indented with tabs, with an XML declaration (.NET's defaults also write a BOM; accept with or without):

```xml
<?xml version="1.0" encoding="utf-8"?>
<ArrayOfIMarkCollection xmlns:i="http://www.w3.org/2001/XMLSchema-instance" xmlns="http://schemas.datacontract.org/2004/07/Vixen.Marks.IMarkCollection">
	<d2p1:anyType xmlns:d2p1="http://schemas.microsoft.com/2003/10/Serialization/Arrays" xmlns:d3p1="http://schemas.datacontract.org/2004/07/VixenModules.App.Marks" i:type="d3p1:MarkCollection">
		<!-- the members above -->
	</d2p1:anyType>
</ArrayOfIMarkCollection>
```

Dispatch is on the root's namespace only (F28). The legacy form has root `ArrayOfMarkCollection` in `http://schemas.datacontract.org/2004/07/VixenModules.Sequence.Timed`, with children `MarkCollection`, members `Bold`, `Enabled`, `Id`, `Level`, `MarkColor`, `Marks` (each a `duration` in the Arrays namespace), `Name`, `SolidLine` (`Vixen.Modules/Sequence/Timed/MarkCollection.cs:33-55`). `convert_old_marks` already reads those members, but note the import differs from it: F28's migration sets no default.

### Other mark file formats

| Format | Read | Write | Fields |
|---|---|---|---|
| Audacity labels, bars | F29 | No | `start<TAB>end<TAB>text` in seconds, or space-separated |
| Audacity labels, beats | F30 | No | Regex over the whole file, single-digit beat number |
| Audacity marks export | No | F39 | `0000.000<TAB>0000.000<TAB>n`, both columns the start |
| xTiming | F31 | No | `<timings><timing name><EffectLayer><Effect label starttime endtime>` in ms, or root `<timing>` |
| Papagayo `.pgo` | F87 | No | Line based |
| Pangolin Beyond CSV | F34 | F40 | Header `#,Name,Start,Color`; `M<n>,<text>,<mm:ss.fff or hh:mm:ss.fff>,<BBGGRR>` |
| Singing Faces vendor list | F42 | No | JSON array of `{Name, Url}` at a fixed URL |
| Singing Faces inventory | F42 | No | XML `musicinventory` |
| `.v3l` layers | F65 | F63, F64 | JSON |

All of these are read whole into memory; none is large. How the text is decoded and how numbers parse: F116.

### Layers XML, real shape

```xml
<SequenceLayers xmlns:d1p1="http://schemas.datacontract.org/2004/07/Vixen.Sys.LayerMixing" xmlns="">
  <d1p1:Layers xmlns:d2p1="http://schemas.microsoft.com/2003/10/Serialization/Arrays">
    <d2p1:anyType i:type="d1p1:DefaultLayer">
      <d1p1:Id>ec5dea5b-349d-4097-a665-b23bdd41ef72</d1p1:Id>
      <d1p1:LayerLevel>0</d1p1:LayerLevel>
      <d1p1:LayerName>Default</d1p1:LayerName>
      <d1p1:Type>Default</d1p1:Type>
    </d2p1:anyType>
  </d1p1:Layers>
  <d1p1:_effectLayerMap xmlns:d2p1="http://schemas.microsoft.com/2003/10/Serialization/Arrays" />
</SequenceLayers>
```

A filter is stored apart from its layer: `_layerMixingFilterSurrogates` holds `TypeId`, `InstanceId`, `LayerReferenceId`, and `_layerMixingFilterDataModels` holds the data by instance id (`Vixen.Core/Module/SequenceType/Surrogate/LayerMixingFilterSurrogate.cs:18-25`, `Vixen.Core/Module/SequenceType/SequenceTypeDataModelBase.cs:84-96`, `:137-152`). A surrogate with an unknown type: keep it through the save, already done by `filter_surrogate_residue`.

Two surrogates can name one `InstanceId` in a file Windows wrote, with one entry for it in `_layerMixingFilterDataModels` (F117). Windows reads those entries with `Dictionary.Add` (`Vixen.Core/Module/ModuleDataSet.cs:164-168`), so **a file holding two entries with the same type and instance makes it throw on load**. `decode_layers` gives each layer its own copy of the data and keeps the shared id, and the save writes one model per layer, so opening and saving such a file would write exactly that. See the fix in Rust API. The owner's show has no such file.

### Lip sync library XML

From the owner's `ModuleStore.xml`, one map, version 5:

```xml
<Module dataModelType="VixenModules.App.LipSyncApp.LipSyncMapStaticData, LipSyncApp" moduleType="a475c535-73fa-4d31-a1bc-ed2b50ad21cf" moduleInstance="a475c535-73fa-4d31-a1bc-ed2b50ad21cf" assemblyName="Module.App.LipSyncApp">
  <LipSyncMapStaticData xmlns="http://schemas.datacontract.org/2004/07/VixenModules.App.LipSyncApp" xmlns:i="http://www.w3.org/2001/XMLSchema-instance">
    <ModuleInstanceId xmlns="">a475c535-73fa-4d31-a1bc-ed2b50ad21cf</ModuleInstanceId>
    <ModuleTypeId xmlns="">a475c535-73fa-4d31-a1bc-ed2b50ad21cf</ModuleTypeId>
    <_library xmlns:a="http://schemas.microsoft.com/2003/10/Serialization/Arrays">
      <a:KeyValueOfstringLipSyncMapDataiatCCBjB>
        <a:Key>New Map</a:Key>
        <a:Value>
          <ModuleInstanceId xmlns="">00000000-0000-0000-0000-000000000000</ModuleInstanceId>
          <ModuleTypeId xmlns="">00000000-0000-0000-0000-000000000000</ModuleTypeId>
          <IsCurrentLibraryMapping>false</IsCurrentLibraryMapping>
          <IsDefaultMapping>false</IsDefaultMapping>
          <IsMatrix>true</IsMatrix>
          <LibraryReferenceName>New Map</LibraryReferenceName>
          <MapItems>…</MapItems>
          <Notes />
          <StringsAreRows>false</StringsAreRows>
          <UsingDefaults>true</UsingDefaults>
        </a:Value>
      </a:KeyValueOfstringLipSyncMapDataiatCCBjB>
    </_library>
```

**This is the current shape, whatever the attributes suggest.** `LMD:48-70` marks a `[DataMember] protected string _libraryReferenceName` and gives `MapItems`, `StringsAreRows` and `Notes` `EmitDefaultValue = false`, but `LipSyncMapData` itself has no `[DataContract]` (`LMD:10`). `DataContractSerializer` therefore treats it as a plain type: the base class's two members, then every public read/write property in alphabetical order, with those attributes ignored, and it runs the public constructor on read, which `_imageCache` depends on (`LMD:12`, `:83`). That is exactly what the owner's file holds: `LibraryReferenceName` from the property, no `_libraryReferenceName`, and `MapItems`, `Notes` and `StringsAreRows` always present. So a member missing from a file keeps the constructor's value: `IsMatrix` true, `Notes` empty, `UsingDefaults` true (`LMD:15-21`). `LipSyncMapStaticData` and `LipSyncMapItem` do have `[DataContract]`, so theirs are as declared.

**Read rule:** the dictionary key is the map's name in the library. `LibraryReferenceName` names its folder and is what the menus show (`LMD:67`, `Menu.cs:496`), and upstream keeps the two equal (`LML:150`). **Write rule:** edit the existing XML in place through the module store, so an unchanged map stays byte-identical. A new or edited map writes this same shape: `LibraryReferenceName`, `<MapItems i:nil="true" />` since a clone drops them, `<Notes />` when empty, and `<StringsAreRows>false</StringsAreRows>`. **Never write `_libraryReferenceName`**: Windows would ignore it and read a blank name. `IsDefaultMapping` may be false on every map, as it is in this file; the first map in file order then acts as the default (`LML:37-59`).

`LipSyncMapData`'s other fields: `IsCurrentLibraryMapping` is written but never read back by anything; keep it. `UsingDefaults` defaults to true (`LMD:15-21`). The copy constructor copies everything but `MapItems` and `StringsAreRows`; `Clone` also clears `IsDefaultMapping` and `IsCurrentLibraryMapping` (`LMD:23-43`). Dictionary order is insertion order, with a freed slot reused by the next add (F105), which is the order of `Default Image Map`'s submenu and of "first in the library". An empty store reads as one map: F119.

### Undo

**None of the docker, layer panel, Beat/Bar, map editor or converter window actions has undo upstream** (sequencer-shell F339, F418). Mark edits made in the timeline do.

**Backend hazard, now narrower.** The timeline's own mark edits have moved to per-mark actions (`MarkTimesChanged`, `MarksAdded`, `MarksRemoved`, `crates/vixen-session/src/undo.rs`), which touch only the marks they name and skip a collection that has gone, so they are safe. `UndoAction::MarksChanged` remains for two commands in `crates/vixen-session/src/sequencer/commands.rs`: `add_marks_to_effects` (`Added <n> Mark`) and `bulk_move` (`Bulk Change <n> Mark`). It holds a snapshot of **every collection**, flags and all, and applying it replaces the whole list (its arm of the undo apply in `crates/vixen-session/src/sequence.rs`). Upstream's mark undo actions hold only marks. So with no-undo panel actions as specified: run one of those two commands, add a collection in the panel, press Undo, and **the new collection vanishes**. Rename, colour, default and link changes revert too, and collections generated by Beat/Bar Detection disappear. This must be resolved before the panel ships; see `apply_marks_snapshot` in the Rust API. An open question for the owner: panel actions could instead gain real undo, which upstream lacks.

## Rust API needed

Everything in Features that isn't drawing belongs here. Existing and reusable: `vixen_core::marks`, `vixen_core::layer`, `vixen_io::sequence::{mark_collection_from_xml, mark_collection_to_xml}`, `vixen_session::timeline::marks::{set_mark_start, set_mark_duration, ensure_order, fill_gap_times, unique_collection_name, breakdown_phrase, breakdown_word, add_mark_at}`, `vixen_session::sequencer::lifecycle` (name repair), `vixen_analysis::*`, `vixen_audio::Peaks`, `vixen_session::dialogs::{layers, face}`, `crates/vixen-ffi/src/dialogs/layers.rs`. `crates/vixen-ffi/src/marks.rs` is empty and is where this area's FFI goes, under the `marks` feature. Bindings stay bulk: one call returns all rows.

**Fixes to shared code** (the implementer's call, since it touches other areas)

- `vixen_session::sequence`: `fn apply_marks_snapshot(&mut self, snapshot: &[MarkCollection])`: per collection, by id, replace only `marks`, leaving the list, flags, decorator, link and name as they are; ignore ids gone from the list. Replaces the body of the `MarksChanged` arm in that file. Pin with the test below.
- `vixen_io::sequence`: one data model per `(type, instance)`. In `decode_layers`, when a second surrogate names an `InstanceId` a layer already has, give the later layer a new `filter_instance_id` with its own copy of the data, so each layer owns its settings (F117) and the save can't write the duplicate Windows throws on (Data and persistence, Layers XML).
- `vixen_session::render` or `fx-audio-state`: `fn resolve_lipsync_mapping(params: &mut Params, library: &LipSyncLibrary)`: an effect whose `PhonemeMapping` is not in the library takes the default's name, as upstream does at render (`Vixen.Modules/Effect/LipSync/LipSync.cs:84-87`). Run from `ResetLipSyncNodes` and on load, since the effect crate can't see the library (`crates/fx-audio-state/src/lip_sync.rs:6-7`).

**Mark collections panel: `vixen-session::marks` (new)**

- `struct MarkRow { id: Uuid, name: String, color: Color, is_default: bool, show_lines: bool, tail_enabled: bool, show_tail: bool, show_bar: bool, locked: bool, level: i32, kind: MarkCollectionType, linked_to: Option<Uuid>, solid: bool, bold: bool }`, `fn rows(c: &[MarkCollection]) -> Vec<MarkRow>`: the table, in one call. `tail_enabled` is `show_lines`.
- `enum MarkHeader { Lines, TailLines, Bar, Lock }`; `struct HeaderFlags`; `fn HeaderFlags::new(c: &[MarkCollection]) -> HeaderFlags`, `fn click(&mut self, h: MarkHeader, c: &mut [MarkCollection]) -> bool`: F8, returns whether anything changed.
- `fn add_collection(c: &mut Vec<MarkCollection>) -> Uuid`: F4.
- `fn delete_collection(c: &mut Vec<MarkCollection>, id: Uuid) -> bool`: F17, including the new default.
- `fn set_active(c: &mut [MarkCollection], id: Uuid)`: F9, clearing the others.
- `fn rename_error(c: &[MarkCollection], id: Uuid, text: &str) -> Option<String>`: F15's message.
- `enum RenameEnd { Renamed, Unchanged, Restored, StillInvalid }`, `fn end_rename(c: &mut [MarkCollection], id: Uuid, text: &str) -> RenameEnd`: F15.
- `fn offset_time(c: &mut [MarkCollection], id: Uuid, ms: i32) -> bool`: F18.
- Bulk Effects Move needs nothing new: `offset_marks_in_range` in `crates/vixen-session/src/sequencer/dialogs.rs`, called by `bulk_move`, is F49's. It returns a count where upstream returns clones, which the snapshot undo doesn't need, and 0 for an empty collection.
- `struct LinkChoice { id: Option<Uuid>, name: String, checked: bool }`, `fn link_choices(c: &[MarkCollection], id: Uuid) -> Option<Vec<LinkChoice>>`: F24. `None` hides the submenu.
- `fn set_link(c: &mut [MarkCollection], id: Uuid, parent: Option<Uuid>)`: F24, including clearing the parent's other child.
- `fn is_unique_name(c: &[MarkCollection], proposed: &str, excluded: Option<Uuid>) -> bool`: F50. `unique_collection_name` needs its `excluded` argument and the blank case added.

**Import and export: `vixen-io::marks_files` (new)**

- `fn read_import_text(bytes: &[u8]) -> String`: F116, for every text import here and for `.pgo` and `.v3l`.
- `fn read_v3m(text: &str) -> Result<Vec<MarkCollection>, MarkFileError>`, `fn write_v3m(c: &[&MarkCollection]) -> Vec<u8>`: F28, F38.
- `fn read_bar_labels(text: &str) -> Result<Vec<MarkCollection>, MarkFileError>`, `fn read_beat_labels(text: &str) -> Result<Vec<MarkCollection>, MarkFileError>`: F29, F30. No regex match is `Ok` with an empty list.
- `fn write_audacity(c: &[&MarkCollection]) -> String`: F39.
- `fn read_xtiming(xml: &str) -> Result<Vec<MarkCollection>, MarkFileError>`: F31, F33.
- `struct PangolinRecord { text: String, start: TimeSpan, color: Color }`, `fn parse_pangolin(csv: &str) -> Result<Vec<PangolinRecord>, String>`, `fn pangolin_needs_choice(r: &[PangolinRecord]) -> bool`, `enum PangolinMode { GroupByColor, Single(Color) }`, `fn pangolin_collections(r: &[PangolinRecord], m: PangolinMode) -> Vec<MarkCollection>`, `fn write_pangolin(rows: &[(&MarkCollection, bool)]) -> String`: F34, F40.
- `struct VendorLink { name: String, url: String }`, `fn parse_vendor_list(json: &str) -> Result<Vec<VendorLink>, MarkFileError>`, `struct SongInventory`, `fn parse_song_inventory(xml: &str) -> Result<SongInventory, MarkFileError>`: F42.
- Importers start from `MarkCollection::default()`, which matches upstream's constructor (`impl Default for MarkCollection` in `crates/vixen-core/src/marks.rs`), and set what each entry above says.

**Import flow: `vixen-session::marks`**

- `struct ImportOption { name: String, has_duplicate_name: bool }`, `fn import_options(existing: &[MarkCollection], candidates: &[MarkCollection]) -> Vec<ImportOption>`: F35.
- `fn commit_import(target: &mut Vec<MarkCollection>, selected: Vec<MarkCollection>)`: F36, all three passes.
- `trait InventoryTransport: Send + Sync { fn get(&self, url: &str) -> Result<String, String>; }`, `fn load_inventories(t: &dyn InventoryTransport, on_error: &mut dyn FnMut(String, String)) -> Vec<SongInventory>`: F42. Same shape as `HttpTransport` in `crates/vixen-session/src/updates.rs`, so tests inject it.

**Layers: `crates/vixen-session/src/sequencer/layers.rs`.** Built by another worker from sequencer-shell F330-F340 while this spec was being written, and now wired through `crates/vixen-ffi/src/sequencer.rs` (`ShellLayer`, `layers`, `add_layer` and so on) to `LayersPanel` (SwiftUI section below). It has `filter_choices`, `filter_type_name`, `has_setup`, `is_default`, `unique_layer_name`, `add_layer`, `remove_layer`, `move_layer`, `quick_rename`, `rename_layer`, `set_layer_filter`, `has_exportable`, `export_v3l`, `read_v3l_plan`, `import_plan`, `skipped_message`. Reuse it, and bring it into line with this spec. Differences found, each checked against upstream:

- `add_layer` names the layer `Layer <n>`. Upstream gives `Default - 2`, `Default - 3` (F56, `SL.cs:68-72`, `:158-172`, pinned by `LayerEditorLayerServiceTests.AddLayer_GeneratesUniqueDefaultLayerName`). The panel's new layers would be visibly misnamed.
- `rename_layer` refuses the default layer. Upstream's field renames it like any other (F58).
- `filter_from_record` skips a record when a key inside `filterData` is missing. Upstream keeps the constructor's default (F65), so start from the type's defaults in `filter_choices` and overwrite the keys present.
- `read_v3l_plan` fails on a missing `format` or `version` and reports a missing `layers` as "does not contain any layer records". Upstream takes the class defaults, so a missing `format` and `version` pass, and a missing `layers` reaches "does not contain any layers that can be imported" (F65). A malformed `filterTypeId` is ignored there, where upstream fails the file as invalid JSON, and likewise any known key of the wrong type, such as `"version": "1"` or a malformed date (F65 step 2). It also uses `read_to_string`, which fails on bad bytes and, through `serde_json`, on a BOM (F116).
- `KeyColor` is read from `value` alone, so a known colour, whose `value` is 0, comes in black. Use `DotNetColor`'s rules (F64, `crates/vixen-io/src/values.rs`).
- `filterDataType` writes `..., LumaKey`; upstream's assembly name is `Module.LayerMixingFilter.LumaKey`. Not read on import.
- The effect menu's layer list in `commands.rs` uses `Default` as the default layer's filter name for its tooltip; upstream's `FilterName` is `None` (`Vixen.Core/Sys/LayerMixing/Layer.cs:64`).
- `serde_json` here has no `preserve_order`, so keys come out sorted, not in F64's order. Readers go by name, including .NET's, so this only matters for diffing against a Windows file.

What F56 to F62 need that is there already, and what isn't:

- `filter_choices` serves as F56's table: it sorts by name, which gives the same order, and `set_layer_filter` in the FFI finds a fresh filter by type name for F60. Keep F56's TypeIds in `vixen_io::sequence::mixing_filter_ids`.
- The FFI's `add_layer` takes `filter_choices().remove(0)` as the first filter: the one place to change if the owner corrects F56.
- `ShellLayer` in `crates/vixen-ffi/src/sequencer.rs` is the row.
- Still needed only if the panel moves to `.onMove`: `fn move_target(from: usize, insert_offset: usize, count: usize) -> usize`, `.onMove`'s offset to F62's index.

**Beat detection: `vixen-session::beats` (new), over `vixen-analysis`**

- `struct BeatDetection`, `fn start(clip: Arc<AudioClip>, path: &Path) -> BeatDetection`: F68 steps 1 to 3 on a worker thread, using `BeatsAndBars::from_interleaved`, `generate_preview_data`.
- `enum BeatJob { Preparing, Ready(BeatsAndBarsDialogModel, BeatBarPreviewData), Running(AnalysisProgress), Done(FeatureSet), Failed(AnalysisError), Cancelled }`, `fn state(&self) -> BeatJob`: for the sheet to poll at display rate at most.
- `fn generate(&self, settings: BeatBarSettingsData)`: step 5, off the main thread. `fn cancel(&self)`: sets the crate's cancel flag.
- `fn apply_beat_features(c: &mut Vec<MarkCollection>, features: &FeatureSet, settings: &BeatBarSettingsData) -> Result<(), AnalysisError>`: `build_mark_collections` then `ensure_default_collection`, on the main thread, as one change. Fast; only the features are slow. Cancel at any stage and a track with no channels still run `ensure_default_collection` (F68 step 6), which is the one change those paths can make, and F67's FIX has to count it.
- `fn preview_lines(model: &BeatsAndBarsDialogModel, preview: &BeatBarPreviewData) -> (Vec<TimeSpan>, Option<TimeSpan>)`: the marks and period for F73.

**Lip sync: `vixen-session::lipsync` (new)**

- `enum PhonemeType { Ai, E, O, U, Fv, L, Mbp, Wq, Etc, Rest, Unknown }` with `fn name(self) -> &'static str` giving upstream's upper-case names; `const MAP_PHONEMES: [PhonemeType; 10]` in F99's order. `fx-audio-state` works in names today, so keep them as the common form across FFI. `sequencer/papagayo.rs` has its own enum with upstream's `etc` and `Rest` spellings, as upstream has two enums; keep the two apart, and upper-case when going from one to the other (`Menu.cs:535`).
- `fn remove_punctuation(text: &str) -> String`: F84.
- `struct PhonemeDictionary`; `fn load(user_path: Option<&Path>) -> Result<PhonemeDictionary, io::Error>`: parses the bundled text, then the user file. `fn try_convert(&self, text: &str) -> Vec<PhonemeType>`: F84. `fn add_user_mapping(&mut self, word: &str, mapped: &str, user_path: &Path) -> io::Result<()>`: F85. Cached per profile behind a lock, built off the main thread.
- `enum Alignment { Phoneme, Word, Phrase }`, `enum Placement { Cursor, Clipboard, Mark }`.
- `fn lyric_strings(lines: &[&str], align: Option<Alignment>) -> Vec<String>`: F80 step 3, empties kept.
- `struct ConvertData { start: TimeSpan, duration: TimeSpan, phoneme: PhonemeType, lyric: String }`.
- `enum ConvertStep { NeedsMapping(String), Done(Vec<ConvertData>) }`, `struct LyricConversion`, `fn new(strings: Vec<String>, marks: Option<Vec<TimeSpan>>, start_index: usize, align: Option<Alignment>) -> LyricConversion`, `fn step(&mut self, dict: &PhonemeDictionary) -> ConvertStep`, `fn abandon(self)`: F80 steps 4 to 6 as a resumable state, since the failure dialog is asynchronous in Swift. `fn next_start_offset(index: usize, count: usize, align: Alignment, phonemes: usize, strings: usize) -> usize`: step 8.
- `fn phoneme_timing(marks: Option<&[TimeSpan]>, index: usize, sub: i64, unmarked: &mut i64) -> (TimeSpan, TimeSpan)`: F81, pure.
- `fn lipsync_to_clipboard`, `fn papagayo_to_clipboard`, `fn reset_lipsync_elements`: as named in sequencer-shell's Rust API.

**Papagayo: `crates/vixen-session/src/sequencer/papagayo.rs`.** Also built by that worker, with `parse_papagayo(text: &str) -> Result<PapagayoDoc, String>`, `PapagayoDoc`, `PapagayoVoice`, `PapagayoPhoneme`, `PhonemeType`, `import_summary`, and wired for F88 by `papagayo_import` in `crates/vixen-session/src/sequencer/commands.rs` and `crates/vixen-ffi/src/sequencer.rs`. It holds only coalesced phonemes, enough for F88. Differences:

- `papagayo_import` shows the parser's own text titled `Papagayo Import`, where F88 asks for F32's message, and reads with `read_to_string` (F116).

- F32 needs the voice's **phrases, their words and their uncoalesced phonemes**, so `PapagayoVoice` must keep them: `fn voice_collections(doc: &PapagayoDoc, file_stem: &str, voice: &str) -> Vec<MarkCollection>`.
- Times are `f64` from `1000.0 / fps`. Upstream's are `f32` (F87), which differ slightly, more at high frame numbers. Marks and effects from the same `.pgo` should match Windows, so use `f32`.
- Word and phoneme lines split on any whitespace, where upstream splits on single spaces, so a double space differs. The lookup is flat, where upstream goes phrase, word, phoneme, which differs only for malformed files whose phoneme lies outside its word.
- FPS 0 gives zero times rather than an error (F87).
- An empty file, or one ending before the voice count, is an error there. Upstream's is a document with no voices (F87).
- A phoneme list cut short by end of file is an error there, where F87 leaves the missing phonemes out. A phoneme line with other than two fields takes the first two or fails there, where upstream's is `Rest` at frame 0, ending nothing.
- A voice name is trimmed for display as well as for the key, where upstream trims only the key.
- `coalesce` ends a run at the last frame it walked, where upstream takes the last phoneme's own end frame, which can lie past the last sound frame, so a clipped run has a shorter `DurationMS` there.

**Map library: `vixen-session::lipsync`**

- `struct LipSyncLibrary` over the module store entry; `fn from_store(s: &ModuleStore) -> LipSyncLibrary`, `fn write_to(&self, s: &mut ModuleStore)`. Keeps library order, replacing in place (F105). No `_library` reads as F119's one map. Writes the shape in Data and persistence.
- `struct MapSummary { name: String, notes: String, is_default: bool }`, `fn list(&self) -> Vec<MapSummary>`: sorted by name for F90.
- `fn default_name(&self) -> Option<String>`, `fn set_default(&mut self, name: &str) -> bool`: F89. With the FIX, the caller then writes the module store.
- `fn clear_using_defaults(&mut self, names: &[String])`: F119, for what a sequence migration reports.
- `fn contains(&self, name: &str) -> bool`, `fn unique_name(&mut self, name: Option<&str>) -> String`: F91's rule, including the rising counter.
- `fn new_map(&mut self) -> String`, `fn clone_map(&mut self, name: &str, root: &Path) -> Result<String, String>`, `fn remove_maps(&mut self, names: &[String], root: &Path) -> Result<(), String>`, `fn rename_map(&mut self, old: &str, new: &str, root: &Path) -> Result<String, String>`: F91, F93, F94, F95. Refuses to remove every map. `rename_map` follows F118: keeps the old map if the copy fails, goes through a temporary name on a case-only change, and refuses path separators and `.` or `..`.
- `struct MapEdit { original: String, name: String, notes: String, images: HashMap<&'static str, EditImage>, current: usize }`, `enum EditImage { Stored(PathBuf), New(PathBuf), Scratch(PathBuf) }`, `fn begin_edit(&mut self, name: &str, root: &Path) -> Result<MapEdit, String>`: F97, writing defaults under F98.
- On `MapEdit`: `fn next(&mut self)`, `fn prev(&mut self)`, `fn current_phoneme(&self) -> &'static str`, `fn current_image(&self) -> Option<PathBuf>`, `fn set_image(&mut self, phoneme: &str, file: &Path) -> bool`, `fn clear_image(&mut self, phoneme: &str)`: F99 to F104.
- `fn accepts_drop(paths: &[PathBuf]) -> bool`: F101's extension rule.
- `enum MapperChoice { Unmapped, Mapped, File(usize) }`, `fn mapper_initial(current: &str, file_count: usize, mapped: &[&str]) -> Vec<(&'static str, MapperChoice)>`, `fn apply_mapper(&mut self, choices: &[(&'static str, MapperChoice)], files: &[PathBuf])`: F102.
- `fn begin_external_edit(&mut self, root: &Path) -> io::Result<PathBuf>`, `fn finish_external_edit(&mut self, file: &Path) -> io::Result<()>`: F103. `Err` keeps the old image.
- `enum EditCommit { Saved(String), NeedsOverwriteAnswer(String), RenameRefused(String) }`, `fn commit_edit(&mut self, edit: MapEdit, overwrite: Option<bool>, root: &Path) -> EditCommit`: F105.
- `fn migrate_string_maps(&mut self, world: &mut dyn DialogWorld) -> usize`: F113, using Face's `properties` helpers.

**Node selection: `vixen-session::lipsync`**

- `struct NodeSelect { chosen: Vec<Uuid>, allow_groups: bool, recurse: bool, user_touched: bool }`, `fn add(&mut self, tree: &ElementTree, selected: &[Uuid])`, `fn remove(&mut self, ids: &[Uuid])`, `fn reset(&mut self)`, `fn set_allow_groups(&mut self, on: bool)`: F108 to F110.

**FFI: `crates/vixen-ffi/src/marks.rs`.** `#[uniffi::export] impl SequencerSession` methods over the above: panel rows and commands; the import flow as `pick`, `candidates`, `commit`; export; beat job; lyric converter; library, and `MapEdit` as a uniffi object. Layers, Create Evenly Divided Marks and Papagayo Import are already in `crates/vixen-ffi/src/sequencer.rs`. Every mutating call that changes collections or layers reports through the per-window change set the timeline already uses (timeline spec, Notifications), which is how the marks bar and snap points refresh; drain once per user action. Every call returning rows returns the whole array at once.

## SwiftUI and AppKit structure

Module `app/Sources/VixenMarks`, exporting `MarksPanel` and the sheets below, plus `MarksTestScene` for building alone (`docs/UI_GUIDE.md`). `VixenSequencer` replaces the `.marks` placeholder in `app/Sources/VixenSequencer/SequencerContent.swift`, and fills in the stubs waiting for this area: in `SequencerCommands.swift` the disabled Beat/Bar Detection, Default Image Map, Edit Image Maps, Edit Element Face Mapping and Lyric Converter items, and in `ToolbarBand.swift` the disabled `toolsToolStripButton_LipSync`. `VixenDialogs` is already in `VixenSequencer`'s `areaDependencies` in `app/Package.swift`, and an area may not import another, so the Face Setup sheet in F111 is presented from `VixenSequencer`, as `LayersPanel` presents `LayerFilterSetupSheet` (`app/Sources/VixenDialogs/EntryPoints.swift`). Add `WindowID.lyricConverter`.

**Built already by the sequencer shell, so don't redo them:** Create Evenly Divided Marks (`EvenMarksSheet`), Papagayo Import, and the Layers panel: `LayersPanel` in `app/Sources/VixenSequencer/LayersFindPanels.swift`, which stays in `VixenSequencer`. Bring `LayersPanel` in line with F53 to F65. Differences found by reading it:

- Add and Remove are `plus` and `minus` icons in one bar with Import and Export. F53 has a toolbar row for Import and Export, then two equal-width text buttons, `Add Layer` and `Remove Layer`, below it. Restore that: same region, same wording.
- The Name field is disabled on the default layer. F58's KEEP says it isn't, and `rename_layer` needs the same change.
- Import's confirmation says `Import`, where F65 says `Continue`, and the none-importable alert is an information one, where upstream's is an error.
- `import_layers` reads the file again after the plan was shown, where upstream imports the plan it showed (`LEVM:336`). Hold the plan, or check nothing changed.
- Add selects and expands the new layer, where upstream's starts collapsed with the selection unchanged (F58). Harmless, and handy: leave it unless the owner objects.

All state in `@Observable` models holding the uniffi objects; no logic in Swift beyond presentation. Everything here is small enough for SwiftUI except where noted.

| Upstream | Mac | Region kept |
|---|---|---|
| `Form_Marks` dock panel | Dock panel tab via `PanelId::Marks`, title "Mark Collections", detachable as the panel system does | Right side |
| Toolbar New, Import, Export | Small borderless buttons with `.help` tooltips. `plus` SF Symbol for New, `square.and.arrow.down` for Import, `square.and.arrow.up` for Export | Top of panel, left to right |
| Header row and rows | `Grid`-aligned rows in a `ScrollView`, not `Table`, to keep the icon headers clickable and the custom first columns; `LazyVStack` for rows. Collections number in tens | Below toolbar |
| Header icons | `pencil` and `lock` SF Symbols, and Vixen's `MarkLineVisible`, `MarkTailLineVisible`, `MarkBarVisible` PNGs; clicks via `Button(.plain)` | Same columns |
| Active radio | A `Button` with `circle` / `largecircle.fill.circle`, since `Picker(.radioGroup)` can't sit across rows | Column 1 |
| Checkboxes | `Toggle(.checkbox)`, labels hidden; Tail `.disabled(!showLines)` | Columns 2 to 5 |
| Name, double-click | `Text` in the decorator colour, `.onTapGesture(count: 2)`; swaps to `TextField` with focus and select-all, red border and caption | Last column |
| Context menu | `.contextMenu` on the name only. Delete `role: .destructive`. `Picker` submenus for Weight and Type; Linked To built on open and hidden when `nil` | Same order |
| Offset Time prompt | Small sheet with a stepper and field | Sheet |
| Colour picker | `NSColorPanel`, or `ColorPicker` | Panel |
| Import Type Selection | `.sheet` with `Picker(.radioGroup)` | Same order |
| File pickers | `NSOpenPanel` / `NSSavePanel` with the filters as `UTType`s plus All Files; start in the last folder per F27 | |
| Selection dialog | `.sheet`, `List` with selection and a checkbox per row, `.onKeyPress(.space)`, caution `Image(systemName: "exclamationmark.triangle")` with `.help` | |
| Export dialog | `.sheet`, two columns in a `Grid`: table on the left, radio group on the right | Same sides |
| Song Inventory | `.sheet`, 800x600, `HSplitView`: left `Picker`, `OutlineView` from VixenKit (lists can be thousands of songs), details; right `TabView` with one tab. Status bar omitted, since it never held anything | Left and right kept |
| Create Evenly Divided Marks | Built as `EvenMarksSheet`: `.sheet`, `Grid`; time fields as the sequencer's other time editors; `Stepper` with field | |
| `LayerEditorView` | Built as `LayersPanel`: a dock panel tab with `DisclosureGroup` rows in a `ScrollView` and row-to-row drag (F62) | Toolbar top, two buttons, list |
| Layer fields | `TextField` committing on submit and focus loss, Esc restoring; `Picker` for Type; `Button("Configuration")` | Inside the row |
| Beats and Bars dialog | `.sheet`, three `GroupBox`es stacked. Cancel has `.keyboardShortcut(.cancelAction)`; Return is left alone (F69) | Same order |
| Music staff | SwiftUI `Canvas` with the PNGs; `.onTapGesture` opens `Menu` via `NSMenu.popUp` at the click, and `.contextMenu` has the same items (F70); read-outs as `Text` with `.monospacedDigit()` | Inside Divisions |
| Preview waveform | **AppKit:** `NSView` drawing `Peaks::range` once to a bitmap layer, yellow lines in a `CAShapeLayer` so Split Beats doesn't recompute the waveform | Inside Preview |
| Progress | `.sheet` with `ProgressView`, `determinate` then indeterminate, and a Cancel button | |
| Lyric converter | `Window(id: WindowID.lyricConverter)`, `.windowResizability(.contentSize)`, `.windowLevel(.floating)` so it stays above the sequencer as an owned form does. A `Window` scene gives one instance. It holds the session of the sequencer that opened it and closes with it; the menu item from any sequencer brings it forward (F76) | Same top-to-bottom order |
| Start Offset | **AppKit:** `NSPopUpButton`, filled when opened. A beat collection has thousands of items, too many for a SwiftUI `Picker` | Marks group |
| Text box | `NSTextView`-based, for Return handling (F79) | |
| Failure dialog | `.sheet` on whichever window asked, the converter or the sequencer (F86), two rows of image `Button`s via `Grid`, captions beneath | Same order |
| Image Maps | `.sheet`, `Table` with `Name`, `Notes`, sorted, multi-select, `.onDeleteCommand`, inline rename; buttons on the right | Right column kept |
| Image Map editor | Nested `.sheet`, `.dropDestination(for: URL.self)` on the whole content, fixed image area 128x128 with `.aspectRatio(.fit)` and tiled `NoImage` background | Same order |
| Image Mapper | Nested `.sheet`, `Grid` of labels and `Picker`s | Same order |
| Node Select | `.sheet`, `HSplitView`; left `OutlineView` from VixenKit (50,000 elements must work), right `List` | Three columns kept |
| Face Setup | `app/Sources/VixenDialogs` | Per its spec |
| Alerts | `.alert` and `.confirmationDialog`, verbs on buttons | |

**Menus** (`.commands`): Tools > Audio > Beat/Bar Detection, Tools > Create Evenly Divided Marks, Tools > LipSync submenu, as sequencer-shell F80, F82, F87 to F91 list. View > Mark Window and View > Layer Editor Window are already in `app/Sources/VixenSequencer/SequencerCommands.swift`: reopening a panel builds its model anew (F114).

**UI test harness.** Every sheet needs accessible labels equal to the upstream wording, so `{ "action": "click", "label": "Add Layer" }` works. The import and beat flows must be drivable with file paths injected from Rust under `VIXEN_UI_TEST`, since an `NSOpenPanel` can't be driven headless.

## Performance hazards

| Upstream | Where | What to do |
|---|---|---|
| `AddMark` sorts the whole collection and raises a change per mark, so a large import or beat generation is quadratic, and every change reaches `SequenceModified` and the timeline's snap rebuild | `MC.cs:162-168`, `BB/BeatsAndBars.cs:186`, `:211`, `:235`, `MIES:283`, `:438`, `:549-555` | Build the mark list first, sort once with `ensure_order`, publish one change. `vixen_core`'s `add_mark` is a sorted insert, so it's fine for single adds but use bulk for imports |
| The preview tracker runs on the UI thread before the dialog shows, on 10 s of audio, with no indicator | `BB/BeatsAndBars.cs:442` | Worker thread and a spinner in the sheet (F68) |
| Whole-file analysis, with most time in the final step and no cancel | `QM/QMFiles/BarBeatTrack.cpp:378-477`, `BB/BeatsAndBarsProgress.cs:37-43` | Worker thread, indeterminate progress after feeding, working cancel (F74). `crates/vixen-analysis/examples/analyze_song.rs` times it |
| Raw audio goes through a `List<byte>` with `AddRange(buffer.Take(n))`, per chunk | `Vixen.Modules/Media/Audio/Audio.cs:116-131` | `AudioClip` is already decoded; reuse the session's, don't decode again |
| Preview waveform repaints from the audio with a whole-dialog `Refresh()` for each change | `BB/BeatsAndBarsSettings.cs:177`, `BB/PreviewWaveform.cs:92-120` | Peak bitmap once; lines in their own layer |
| A header click sets one property per collection, each raising the change that rebuilds snap points | `MDVM:242-246`, `Form.cs:5644-5647`, timeline spec "Rebuild triggers" | Apply to all, then one change |
| One collection-changed event per imported candidate, and every row re-subscribes to the list and rebuilds its Linked To menu on each | `Docker/Services/MarkCollectionImportCommitter.cs:64-71`, `MCVM:31-38` | Commit as one change; build Linked To when the menu opens |
| Per-row Catel view models, found through `IViewModelManager`, reflecting property maps | `MCVM:19-29`, `:92-97` | Plain row structs from `rows()` |
| `.v3m` is parsed twice: a DOM to sniff the namespace, then the serializer | `MIES:160-186` | One parse |
| `setMarkControls` rebuilds both lists on every window activation, including all marks | `TCF:291-363`, `:365-368` | Refresh from a change count; fill Start Offset on open |
| `marks.Sort()` and `Last()` per string | `TCF:64-73` | Sort once at the start of a conversion |
| First conversion parses a 3.7 MB dictionary on the UI thread | `PU:711-739` | `OnceLock`, on a worker, started when the window opens. Parse into `(&'static str, compact phoneme list)` over the embedded text, with no per-entry `String` |
| Any map change re-renders every row, though only LipSync effects can change | `Form.cs:6115-6130` | Dirty and render only LipSync effects, as `RenderLipSyncElementsAsync` already does (`Form.cs:4124-4142`) |
| Cloning or removing a map constructs a whole editor form, including its images, just to copy files | `LML:171-175`, `:251-255` | Plain file operations, off the main thread |
| Image cache shared between threads and cleared only on the edited copy | `LMD:12`, `:119-127` | Per-render cache already in `crates/fx-audio-state/src/lip_sync.rs:288-308` |
| Add scans every element per selected tree row, and the chosen list per add (and Face Setup finds each row's node by name the same way) | `LS/LipSyncNodeSelect.cs:200-244`, `Vixen.Modules/Property/Face/FaceSetupHelper.cs:286-289` | Quadratic on large displays, and 50,000 elements must work. Node ids and a `HashSet` in Rust; one pass per add (F108) |
| De-duplicating beats compares every mark with every mark of each earlier collection | `BB/BeatsAndBars.cs:156-165` | A few thousand marks at most, so harmless off the main thread. `build_mark_collections` runs on the main thread in `apply_beat_features`, so time it on a long song, and move it off if it shows |
| `Console.WriteLine` per Papagayo line | `TSE/PapagayoDoc.cs:439` | Drop it |
| `Coalesce` scans phrases, words and phonemes per frame | `TSE/PapagayoDoc.cs:311-366` | Fine at these sizes (a few thousand frames); keep it simple and correct |
| Inventory fetch is sequential, with a blocking error box per vendor | `ViewModels/VendorInventoryWindowViewModel.cs:41-55` | Fetch off the main actor; collect errors and show them together or one by one, not blocking the fetch |
| `UndoAction::MarksChanged` clones all marks of all collections per use | `crates/vixen-session/src/sequencer/commands.rs` (Add Marks to Effects, `bulk_move`) | Beat collections hold thousands of marks, and undo depth multiplies it. Snapshot only touched collections when revisiting `apply_marks_snapshot`. The timeline's own edits no longer do this |

Never poll Rust faster than the display: `BeatJob` state can be read in `TimelineView(.animation)`, or better pushed. Nothing here needs FFI per row per draw.

## Tests worth writing

Expected values below are read from the code or computed by the code's rules. Rust unit tests, no window, no network, no sound.

**Names.** `unique_name("Mark Collection", [])` gives `Mark Collection`; with `Mark Collection` present, `Mark Collection - 2`; with `- 2` too, `- 3`. `is_unique_name(["Beat Marks"], "beat marks")` false; `" Beat Marks "` false; excluding its own id, true. Repair: `[Beat Marks, Beat Marks, Beat Marks]` gives `Beat Marks`, `- 2`, `- 3`, with records for the last two only. `[Beat Marks, Beat Marks - 2, Beat Marks]` gives third `Beat Marks - 3`. `["", Mark Collection]` gives `Mark Collection - 2`, `Mark Collection`. `crates/vixen-session/src/sequencer/lifecycle.rs` already has a mixed case in its tests.

**Model.** `Mark::new` 450 ms; setting duration 10 ms stores 25 ms. `FillGapTimes`: marks at 0 (450), 1000, 2000; fill the middle: start becomes 451, duration 2000 - 1 - 451 = 1548. Last mark keeps its duration. Sort ties: equal starts order by end. `offset_marks_in_range` with offset -100 on a mark at 50, duration 450: start 0, duration 400. A mark at 10, duration 30, ending below zero, is removed. A mark ending past `maxTime` gets `Duration += EndTime` (KEEP). Empty collection returns 0 and doesn't panic. Load: duration 10 at start 100 reads 25; duration 0 at start 0 stays 0.

**Header toggles** (F8). Fresh, mixed collections: Lines click sets all on, second sets all off. All on at creation: first click sets all off. Tail with the FIX: same as Lines. Without: first click sets all off, second on. Empty list: Lines flag false, Tail flag true. On an empty list a Bar click leaves its flag and a Lines click flips it. A rebuilt panel recomputes all four.

**Panel.** New on empty is default; second is not. Delete the default of three: if the first remaining is invisible and the second visible, the second becomes default; with none visible, the first. Link: Phoneme offers Word collections plus the `None` item, Word offers Phrase collections plus `None`, and other types return no menu. Two Words linked to one Phrase: linking the second clears the first. Type change keeps the id. Rename: blank restores; duplicate stays editing with message `A Mark Collection named "X" already exists.`; `"  New  "` stores `New`; case-only change allowed.

**Audacity bars.** `1.0\t2.0\tVerse\n3.5\t3.5\t\n6.0\n` gives `Audacity Marks`, yellow: (1000 ms, 1000 ms, `Verse`), (3500, 25, ``), (6000, 25, ``). `4.0 5.0 two words` keeps `two`. `abc` errors. A line of only spaces errors. Empty file: no collections. `read_import_text`: a UTF-8 or UTF-16 BOM is stripped, a bad byte becomes U+FFFD.

**Audacity beats.** `1.000000\t1.000000\t1\n1.500000\t1.500000\t2\n` gives `Audacity Beat 1 Marks` yellow and `Audacity Beat 2 Marks` gold, 450 ms each. Beat 7 errors. Beat 0 errors. No matching line: `Ok`, no collections, no alert. A first column of `.` errors.

**Audacity export.** One collection, marks 1.5 and 2.25 s: `0001.500\t0001.500\t1`, `0002.250\t0002.250\t2`. Two collections: label is collection number. Sorted as strings; `\r\n`.

**xTiming.** Upstream's test: three layers named `Voice` give `Voice - Phrase`, `Voice - Word`, `Voice - Phoneme`, with links. One layer named `T` gives `T - 1`, Generic, brown, bar on. `starttime == endtime` skipped. An empty first layer of three makes the second `Phrase`. Root with neither element: no collections. `/timings/timing` handled.

**Pangolin.** Upstream's: `#,Name,Start,Color\r\nM1,Intro,00:01.250,112233\r\nM2,Chorus,01:02:03.004,A0B0C0\r\n` gives `Intro`, 1.25 s, RGB(33,22,11 hex); `Chorus`, 1:02:03.004, RGB(C0,B0,A0). Errors name lines: `invalid header` Line 1; missing column Line 2; `1.25` time Line 2; `GG2233` Line 2. Group by colour keeps first-appearance order and names `Beyond Marks - #332211`. Export: red mark at 61.2345 s with text `a,b`, include on: `M1,a b,01:01.234,0000FF`; include off: `M1,,01:01.234,0000FF`. Ties stay in collection order. Round trip through the parser.

**Commit** (upstream's `MarkCollectionImportCommitterTests`). Sequential names. An existing default clears candidates'. No default: first flagged, then first visible, then first. A link to an unselected candidate clears; to an existing or selected one stays. Nothing selected changes nothing.

**Papagayo.** Sample in F87: phoneme `etc` has type name `etc`; an unknown name gives `Rest`. Times at FPS 24 as `f32`: sample 41.666668, frame 10 start 416.66669, frame 2's end 125.0. Coalesce: FPS 10, 6 frames, phrase 1 to 4, word `hi` 1 to 4 with `1 etc`, `3 AI`: `Rest` 0-0, `etc` 1-2 lyric `hi`, `AI` 3-4 lyric `hi`, `Rest` 5-5; `StartMS` 0, 100, 300, 500; `DurationMS` 100, 200, 200, 100. First line without `lipsync` fails. Truncated phrase fails. Trailing blank line fails. Duplicate voice fails. An empty file, and one ending after the FPS line, give no voices and no error. Voice collections: names `<file> <voice> Phrases`, `Words`, `Phonemes`, links in chain, colours as F32. Tools import duration is `DurationMS - 1`.

**Dictionary.** Computed from the shipped file: `hello` ETC AI L O; `world` WQ E L ETC; `the` ETC AI; `a` AI; `mom` MBP AI MBP; `happy` ETC AI MBP E; `christmas` ETC AI ETC MBP AI ETC (collapsing the leading double ETC); `don't` ETC O ETC; `santa` ETC AI ETC AI; `all` O L; `xyzzy` nothing; `hello xyzzy world` ETC AI L O WQ E L ETC. Case-insensitive. 133,854 entries, 8,780 of them numbered alternates such as `A(1)`, no unknown tokens. `a(1)` is a key, but the word `a` uses the first. A user mapping `WIBBLE AI WQ` wins over any standard entry and appends a line; empty mapping stores an empty list. Punctuation: `Hello, "World"!` gives `Hello World`, and `don't` keeps its apostrophe.

**Timing** (F81). No marks: `hello world`, starts 0, 250, 500, 750, 1000, 1250, 1500, 1750, each 250 ms. Marks 1000, 2000, 3000, Word: `hello` (4 phonemes) at relative 0, each 250; `world` at 1000; third string at index 2: relative 2000, 250 ms; fourth at index 3: relative 3000 (the jump). Phoneme align, marks 0, 100, 300: durations 100, 200, 250. Start Offset advance: Word adds strings including blanks; beyond range stays.

**Layers** (upstream's `LayerEditorLayerServiceTests`, `LayerEditorViewModelTests`, `LayerImportExportServiceTests`). First add is `Default - 2`, second `Default - 3`, at index 0 above the default. Removing the default is refused. Moving onto the default's index is refused. Moving the default is refused. Levels count up from the bottom. Quick rename to `Mask And Fill`, then `Mask And Fill - 2` for a duplicate; not on default. Export disabled with only the default. `.v3l`: non-default only, top to bottom, `order` 0, 1; no `ModuleTypeId`, `ModuleInstanceId`; no `filterData` or `filterDataType` for a setup-less filter; indented. Import: unknown filter skipped, named in the reason; default record ignored; blank name fails; bad JSON fails; format other than `Vixen3Layers` and version other than 1 fail; imports sit above existing in exported order; new ids; `Top` collides to `Top - 2`. Round trip all three data models, including `KeyColor`. A missing key keeps the constructor's value: Luma Key `{"LowerLimit":0.3}` gives upper 1; Mask And Fill `{}` gives exclude-zero on. Missing `format` and `version` pass; missing `layers` reaches "does not contain any layers that can be imported"; `"version": "1"` and a malformed `exportedUtc` fail as invalid JSON. A default-layer rename is allowed. Two surrogates with one `InstanceId`: each layer ends with its own id, and the save writes no duplicate model. `move_target`: moving row 0 of 4 to offset 2 gives 1; offset past the default clamps.

**Beats.** Already covered by `crates/vixen-analysis/tests/*` and `settings.rs`. Add: `apply_beat_features` as one change; Cancel at any stage leaves collections identical, apart from the first one becoming default when none was (F68 step 6); `MissingOutput` leaves them identical.

**Map library.** `unique_name`: `New Map`, then `New Map(1)`, then `New Map(2)`; with counter 0 and `New Map(1)` present, `New Map(1)(2)`. Remove refuses the last. Rename moves the folder's files, and with the FIX keeps the default. Clone copies files; `Faces` gives `Faces(1)`. `mapper_initial("E", 3, ["AI"])`: AI `Mapped`, E file 0, ETC file 1, FV file 2, others `Unmapped`. `accepts_drop`: `.PNG` yes, `.gif` no, mixed no, empty no. `commit_edit`: rename onto an existing folder keeps the old name and saves; path separators refused. Default pictures appear on `begin_edit` only when `UsingDefaults`. Reading a file whose name is `AI.bmp` holding PNG bytes works. The owner's `ModuleStore.xml` loads the name `New Map` from `LibraryReferenceName`. A new map writes `LibraryReferenceName`, never `_libraryReferenceName`, with `MapItems` nil, and Windows's member order. Rename: a case-only change keeps the pictures; a failed copy keeps the old map and folder. Editing a map keeps its place in the library. No `_library` reads as one `New Map`.

**Undo hazard.** Run `bulk_move` or Add Marks to Effects, add a collection, undo: the collection stays, and the marks revert. The same with a rename. An ordinary timeline mark edit passes already.

**Node select.** Add with groups off and recurse off on a group does nothing; recurse on adds its leaves; groups on adds the group. A duplicate is skipped. Toggling Allow Groups before any add clears.

**Upstream tests covering this area** (`upstream/Vixen/src/Vixen.Tests/`):

- `Sequencer/MarkCollectionNameServiceTests.cs`, `Sequencer/MarkCollectionImportCommitterTests.cs`, `Sequencer/MarkCollectionImportSelectionViewModelTests.cs`, `Sequencer/MarkCollectionTypeTests.cs`, `Sequencer/PangolinBeyondMarkImportTests.cs`.
- `Sequencer/LayerEditorLayerServiceTests.cs`, `Sequencer/LayerEditorViewModelTests.cs`, `Sequencer/LayerImportExportServiceTests.cs`, `Sequencer/LayerMixingFilterDataJsonSerializationTests.cs`.
- `Analysis/BeatBarSettingsDataTests.cs`, `Analysis/QmBarBeatTrackTests.cs`: both ported to `crates/vixen-analysis`.
- `Effects/MarkCollectionSelectionServiceTests.cs`, `Effects/LipSyncMarkCollectionNameConverterTests.cs`, `Effects/BaseEffectMarkCollectionSelectionTests.cs`, `Effects/DissolveMarkCollectionSelectionTests.cs`, `Effects/WaveAndLiquidMarkCollectionSelectionTests.cs`: F52 and effect pickers.
- `Sequencer/GridMarkSnapPointTests.cs`, `Sequencer/MarksBarAutoScrollTests.cs`, `Sequencer/MarksBarMouseWheelTests.cs`: timeline spec.

No upstream test covers: a missing key inside `filterData`, the docker's view models, Audacity or `.v3m` import, xTiming beyond the one case, Audacity or Pangolin export, Papagayo, the phoneme dictionary, map library or editor, or node select. So those expected values come from reading the code, as above.

## Windows-only omissions

- **`SendKeys` focus tricks and zero-size target button** (`TSE/Forms/Form_Marks.cs:59-66`, `TSE/LayerEditor.cs:42-48`): the responder chain gives the same outcome.
- **`Broadcast` "KeydownSWI"** (`TSE/Forms/Form_Marks.cs:51`, `TSE/LayerEditor.cs:63`): keys travel up the responder chain.
- **`mspaint.exe`** (`LME:601-606`): adapted, F103. Waiting on a process is replaced by a Done button, because `NSWorkspace` can't wait and Preview doesn't exit when a document closes.
- **WeifenLuo docking, `ControlBox`, DPI scaling (`ScalingTools`, `AutoScaleMode`), `ThemeUpdateControls`, `groupBoxes_Paint`, `ThemeComboBoxRenderer`, button hover images, `UseWaitCursor`**: native controls and the panel system replace them.
- **Window repositioning and resizing per image** (`LME:48`, `:418-427`): a sheet has a fixed size.
- **`Application.OpenForms` lookups for a dialog's parent** (`LML:192`, `LME:540-541`): sheets know their window.
- **Catel and WPF plumbing:** `IBusyIndicatorService` (`MDVM:205-210`), `ServiceLocator`, `UIVisualizerService`, `Model` attributes, `TextBoxExtender`, `DragDropManager`.
- **C++/CLI wrappers** (`QM/WrapperFiles/*`, `QM/QMBarBeatTrack.*`): replaced by direct Rust calls. `VampParamCtrl`, `VampOutputCtrl`: WinForms controls with no users.
- **Backslash paths** (`LME:40`, `:58`, `LMD:67`): use `Path::join`.
- **`Directory.GetFiles` order as a source of the filter list:** replaced by F56's constant.
- **NTFS-only case-insensitivity for phoneme files:** APFS is case-insensitive by default too. On a case-sensitive volume `Rest` wouldn't find `REST.bmp`; marks from `.pgo` imports carry `etc` and `Rest` (F32). Upper-case the name in `MapImages::image` for safety.
- **Media Foundation decoding:** `AudioClip` reproduces its behaviour as far as `real_show.rs` pins; not an omission, but marks from a different decoder can differ slightly.

**Also not ported, though not because of Windows:**

- `TSE/MarkTapper.*`, `TSE/AutomaticMusicDetection.*`, `TSE/BeatDetection.resx`: unreachable (F48).
- `LipSyncNodeSelect`'s "Strings are:" group, `SelectedNodeNames` and its closing confirmation: only `MatrixOptionsOnly` reaches them, and nobody sets it (F106).
- The Papagayo copy in `PU:9-550`, `LipSyncMapMatrixEditor.RenameMappingFiles`, `LipSyncMapMatrixEditor`'s parameterless constructor, `HelperName`: no callers.
- `LipSyncMapData.ConfiguredColorAndIntensity`, `PhonemeState`, `IsFaceComponentType`, `FindMapItem`, `MapItems`, `StringsAreRows`: deprecated string-map remnants. Read `MapItems` only for F113.
- `FaceMapItem`, `FaceModule.ConfiguredIntensity`: no meaningful callers.
- `VendorMetadataViewModel`, `VendorMetadataView`, `VendorMetadata`, `InformationMetadata`, `SongLink`, `SendEmailCommand`: not reachable from the Song Inventory window.
- `MarkCollection.MarksWithinRange`, `Mode`, `MarkDocker.LaunchTimingTrackBrowser`, `Annotations`, `BindableBase.SetProperty`, `DebugConverter`, `LipSyncMapData.EyesClosedTag`: unused.
- The Beats and Bars preview strip's height drag and double-click, inherited from the timeline's `Waveform` with `LockWaveformHeight` off (F73): an accident of reuse, inside a fixed box.
