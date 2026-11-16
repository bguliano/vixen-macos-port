# Effect editor, curves, gradients, colors and libraries

## Scope

Citation shorthand used everywhere below, all under `upstream/Vixen/src`. A bare file name means the file named in that section's source line, and `:123` alone means the same file as the last citation:

| Prefix | Path |
|---|---|
| `EE/` | `Vixen.Modules/Editor/EffectEditor/` |
| `FormEffectEditor.cs`, `Form_*.cs` | `Vixen.Modules/Editor/TimedSequenceEditor/Forms/` |
| `TSE/` | `Vixen.Modules/Editor/TimedSequenceEditor/` |
| `CE:`, `Curve.cs`, `CurveLibrary*.cs`, `FunctionGenerator.cs`, `ZedGraph/` | `Vixen.Modules/App/Curves/` |
| `CGE:`, `GEP:`, `GE:`, `ColorGradient*.cs`, `GradientLevelPair.cs` | `Vixen.Modules/App/ColorGradients/` (`ColorGradientEditor.cs`, `GradientEditPanel.cs`, `GradientEdit.cs`) |
| `WC/` | `Vixen.Common/WPFCommon/` |
| `Svc/` | `Vixen.Core/Services/EffectDefaults/` |
| `MergedPropertyDescriptor.cs` | `EE/Internal/` |
| `*.json` under "form" | `docs/reference/forms/<upstream path with __>` (repo root) |
| `fw.rs` | `crates/vixen-effects/src/framework.rs` (repo root, not upstream) |

Related specs: `docs/spec/sequencer-shell.md` owns the panel's dock, the View menu, the toolbar strips, Tools > Effect Defaults, and drops onto timeline effects. It also carries entries for the library windows (its F341 to F363), which agree with those here; **this spec is authoritative for the windows' behaviour and for all dialogs.** `docs/audit/editor-pass.md` records table decisions made earlier.

**Read line by line:**

| Group | Files (lines) |
|---|---|
| Host | `FormEffectEditor.cs` 323, `TSE/Undo/EffectsPropertyModifiedUndoAction.cs` 52, `TSE/PropertyMetaData.cs` 74, `TSE/PropertyOwnerMetaData.cs` 35, `TSE/LibraryDragPayloadFactory.cs` 61 |
| Grid | `EE/EffectPropertyEditorGrid.cs` 1344, `EE/PropertyGrid.Commands.cs` 267, `EE/PropertyItem.cs` 864, `EE/PropertyItemValue.cs` 800, `EE/CategoryItem.cs` 316, `EE/GridEntry.cs` 225, `EE/GridEntryCollection.cs` 204, `EE/MetadataRepository.cs` 348, `EE/ObjectServices.cs` 211, `EE/KnownTypes.cs` 99, `EE/CategoryEditor.cs` 83, `EE/PreviewStateChanged.cs` 25, `EE/PropertyDisplayMode.cs` 38, `EE/PropertyEditingEventArgs.cs` 40, `EE/PropertyGridUtils.cs` 38, `EE/PropertyValueChangedEventArgs.cs` 62, `EE/ResourceLocator.cs` 53 |
| Internal | `EE/Internal/MergedPropertyDescriptor.cs` 531, `CollectionItemValue.cs` 476, `CollectionItems.cs` 39, `CollectionPropertyDescriptor.cs` 59, `MathUtil.cs` 70, `Util.cs` 50 |
| Inline controls | `EE/Controls/BaseInlineCurveEditor.cs` 706, `BaseInlineGradientEditor.cs` 608, `SliderPoint.cs` 357, `InlineCurveEditor.cs` 75, `InlineGradientEditor.cs` 75, `InlineGradientPairCurveEditor.cs` 86, `InlineGradientPairGradientEditor.cs` 67, `CollectionView.cs` 148, `EnumDropdown.cs` 147, `DoubleEditor.cs` 478, `IntegerEditor.cs` 625, `RepeatEditor.cs` 147, `SliderEditorBase.cs` 115 |
| Editors | `EE/Editors/*` all 21 files, 1115 lines: `EditorCollection.cs` 272, `EditorKeys.cs` 287, `Editor.cs`, `PropertyEditor.cs`, `TypeEditor.cs`, `BaseColorTypeEditor.cs`, `ColorTypeEditor.cs`, `CurveEditor.cs`, `FilePathEditor.cs`, `FontEditor.cs`, `GledPathEditor.cs`, `GradientTypeEditor.cs`, `ImagePathEditor.cs`, `LevelEditor.cs`, `PolygonContainerTypeEditor.cs`, `SelectionEditor.cs`, `SliderDoubleEditor.cs`, `SliderEditor.cs`, `SliderPercentageEditor.cs`, `SvgPathEditor.cs`, `VideoPathEditor.cs` |
| Converters | `EE/Converters/*` all 26 files, 1132 lines |
| Design | `EE/Design/*` all 13 C# and 2 XAML files, 722 lines |
| Themes | `EE/Themes/`: `EditorResources.xaml` 468, `PropertyGrid.xaml` 126, `ColorGradientEditor.xaml` 102, `CurveEditor.xaml` 102, `ColorGradientPaletteEditor.xaml` 64, `ColorPaletteEditor.xaml` 68, `GradientLevelPairEditor.xaml` 72, `SliderEditor.xaml` 82, `SliderPoint.xaml` 107, `SystemColorEditor.xaml` 34, `TextEditor.xaml` 81, `DoubleEditor.xaml` 55, `IntegerEditor.xaml` 56, `EnumEditor.xaml` 22, `FontEditor.xaml` 27, `PolygonEditor.xaml` 17, `Converters.xaml` 28, `Generic.xaml` 14 |
| Filters, input, metadata | `EE/PropertyEditing/CategoryItemComparer.cs`, `PropertyItemComparer.cs`, `CollectionChangedEventArgs.cs`, `ValueExceptionSource.cs`, `Filters/PropertyFilter.cs`, `Filters/PropertyFilterPredicate.cs`; `EE/Input/PropertyEditorCommands.cs` 214, `PropertyGridCommands.cs` 193; `EE/Metadata/AttributesContainer.cs` 86 |
| Curves | `Curve.cs` 396, `CurveEditor.cs` 690, `CurveLibrary.cs` 176, `CurveLibraryStaticData.cs` 37, `CurveLibrarySelector.cs` 244, `CurveTypeConverter.cs` 75, `CurveType.cs` 15, `FunctionGenerator.cs` 38; `CurveLibraryDescriptor.cs` 52; forms for the Designers. `ZedGraph/` only for the edit gestures: `ZedGraphControl.Events.cs` regions cited |
| Gradients | `ColorGradientEditor.cs` 208, `GradientEditPanel.cs` 303, `GradientEdit.cs` 713, `ColorGradientLibrary.cs` 180, `ColorGradientLibraryStaticData.cs` 37, `ColorGradientLibrarySelector.cs` 239, `GradientTypeConverter.cs` 59, `GradientLevelPair.cs` 90; `ColorGradient.cs` 1617: lines 1-480 and 860-1617 |
| Library windows | `Form_CurveLibrary.cs` 631, `Form_ColorLibrary.cs` 567, `Form_GradientLibrary.cs` 610: lines 96-610, and the head by diff against the curve form; forms for the Designers |
| Discrete colours | `Vixen.Common/DiscreteColorPicker/` all 10 source files, about 500 lines |
| Colour picker | `Vixen.Common/Controls/ColorPicker/ColorPicker.cs` 927: lines 1-190 and 560-927, plus Designer strings; `ColorLabel.cs` 171; `ColorPicker.resx` strings |
| Shared | `WC/Input/DragDropManager.cs` 376, `DragDropUtils.cs` 108, `DragDropTargetType.cs` 21, `IDragSourceAdvisor.cs`, `IDropTargetAdvisor.cs`, `WC/Utils/TextBoxExtender.cs` 297, `WC/Controls/NavigatableHyperLink.cs` 23, `WC/Converters/IndexOfConverter.cs` 34, `AdditionConverter.cs`, `BooleanToVisibilityConverter.cs`, `ColorToSolidBrushConverter.cs`, `WC/Behaviors/IgnoreMouseWheelBehavior.cs` 33, `WC/Services/MessageBoxService.cs` lines 44-70, `WC/Theme/Theme.xaml` for `ListViewItemOutlineHover` and `ToolbarIconButtonStyle` |
| Defaults | `Svc/EffectDefaultsService.cs` 511, `EffectDefaultScrubber.cs` 130, `EffectDefaultsStore.cs` 71, `EffectDefaultSummary.cs`, `EffectDefaultsImportResult.cs`; `Vixen.Core/Sys/Attribute/ExcludeFromEffectDefaultAttribute.cs` 17 |
| Tests | `Vixen.Tests/EffectEditor/PropertyItemCollectionMoveTests.cs` 119, `Vixen.Tests/EffectDefaults/EffectDefaultsServiceTests.cs` 207, `EffectDefaultScrubberTests.cs` 92, `Vixen.Tests/Sequencer/LibraryDragPayloadFactoryTests.cs` 105 |
| Design notes (under `upstream/Vixen/docs`) | `reviews/vix-3964-effect-default-settings-design.md` 451, `references/effects/effect-defaults.md` 50; `plans/effect-editor/vix-3974-collection-editor-ordering.md` lines 1-120 of 247 |
| Support | `Vixen.Core/TypeConverters/BooleanStringTypeConverter.cs`, `TargetElementDepthConverter.cs`, `IMarkCollectionNameConverter.cs`, `EnumDescriptionTypeConverter.cs`, `Vixen.Core/Attributes/NumberRangeAttribute.cs`, `BoolDescriptionAttribute.cs`, `PropertyOrderAttribute.cs`, `ExpandableObjectAttribute.cs`, `Vixen.Modules/EffectEditor/EffectDescriptorAttributes/ProviderCategoryAttribute.cs`; `Vixen.Modules/Effect/Effect/BaseEffect.cs` lines 1-260; effect lines cited as such |
| Rust | `crates/vixen-effects/src/framework.rs` 984, `crates/vixen-io/src/library.rs` lines 1-140, `crates/vixen-session/src/undo.rs` lines 1-130, the fx-* tables by a census of `kind` and `editor` |
| Second reader's audit, in addition to re-reading the Host, Grid, Internal, Inline controls, Editors, Curves, Gradients, Library windows, Discrete colours and Shared rows above | `Vixen.Core/Execution/Context/LiveContext.cs` 119, `PreviewContext.cs` 21; `Vixen.Common/Controls/TextDialog.cs` 50, `MessageBoxForm.cs` 115 and its Designer, `NumericTextBox.cs` lines 185-331, `ControlsEx/ValueControls/ValueControl.cs` lines 10-60 and 120-140, `ValueUpDown.cs` lines 130-631; `TimeLineControl/Grid.cs:2890-2906`, `TimeLineControl/Element.cs:94-97`; `Vixen.Modules/Effect/Effect/`: `ExpandoObjectBase.cs` 150, `ExpandoObjectObservableCollection.cs` 44, `IExpandoObjectCollection.cs`, `MarkCollectionExpandoObjectCollection.cs` 128, `NotifyPropertyObservableCollection.cs` 144, `EffectListTypeConverterBase.cs` 101, `ExpandoMarkCollectionNameConverter.cs` 64, `IDiscreteColorProvider.cs`; `Vixen.Modules/Effect/State/`: `CustomStateItem.cs` 123, `CustomStateItemCollection.cs` 54, `CustomStateItemNameConverter.cs` 61, `State.cs` lines 158-262, 325-370, 480-495, 687-835, 956-979, 1076-1084, 1116-1268; `Fire/FireTargetElementDepthConverter.cs` 71, `Wipe/WipeTargetElementDepthConverter.cs` 87, `LipSync/LipSyncMarkCollectionNameConverter.cs` 103, `LipSync/PhonemeMappingConverter.cs` 23, `Picture/TilePictures.cs` 35, `Video/Video.cs` lines 15-40, `ColorGradient.cs` lines 480-860; a scan of every `[ProviderCategory]` property's C# type and `[PropertyEditor]` under `Vixen.Modules/Effect`; a scan of the setter bodies of `Waveform.cs`, `Emitter.cs`, `Whirl.cs`, `MorphPolygon.cs` and `FixtureFunctionExpando.cs`; `Vixen.Tests/Common/XYZTests.cs` and the `Vixen.Tests/Effects` files named under Tests, by test name. Rust: `crates/fx-pixel-complex/src/wave.rs` lines 45-70, `whirlpool/mod.rs` lines 100-125, `crates/fx-pixel-physics/src/liquid/mod.rs` lines 85-105, `crates/fx-fixture/src/fixture.rs` lines 81-100 and 129-147, `crates/fx-audio-state/src/state.rs` lines 155-190, 252-369 |

**In scope but not read fully:**

- `Vixen.Common/WpfPropertyGrid/` (83 files, about 10,200 lines): `EE/` is a fork of it, and nothing in the Effect Editor uses it. Compared by a normalised diff against `EE/` for the 12 largest common files (the lines only it has were read; the output for `PropertyGrid.cs` was cut off at about 260 lines). Read: `Themes/PropertyGrid.xaml`, `Themes/EditorResources.xaml`, both layouts' XAML. Read only in part: `Design/TabbedLayout.cs`, `Design/ExtendedPropertyEditorTab.cs`, `EditorKeys.cs`, `EditorCollection.cs`, `DataResource.cs`, `PropertyEditing/NamedColor.cs`. Not read: `Controls/SearchTextBox.cs`, `Themes/SearchTextBox.xaml`, `Design/TabbedLayout.xaml`, `Utils/PasswordBoxExtender.cs`, `Metadata/*` attribute copies, `ValueEditors/EditorResources.xaml`. The claim in F104 that the tabbed layout and password box are unused comes from a search of the XAML that hosts the grid and, in the audit, of every `.cs` and `.xaml` under `Vixen.Modules`, `Vixen.Application` and `Vixen.Common` outside this folder, not from reading those files. `Vixen.Common/WpfPropertyGrid.Themes/` not read.
- `Vixen.Common/Controls/ColorManagement/` doesn't exist. The code with that namespace is `Vixen.Common/Controls/ColorPicker/` (2,639 lines). Not read in full: `ColorPicker.cs` lines 190-560 (searched for Designer strings and handlers only), `ColorSelectionPlane.cs`, `ColorSelectionFader.cs` (searched for Shift handling), `ColorSelectionModules*.cs`, `ColorBgra.cs`, `ColorUtility.cs`, `GDI32.cs`. The Mac replaces this picker with the system panel per `docs/UI_GUIDE.md`, so colour-module internals aren't specified. The colour models (`ColorModels`) live in `Vixen.Core/Common/ColorSpaces.cs` and aren't read here; `vixen_core::color` is their port.
- `Vixen.Common/WPFCommon/` (58 files): "as used here", as listed above. `Theme.xaml` 2381 lines only for the named styles.
- `EE/Controls/ToggleSwitch.cs` 815 and `EE/Themes/ToggleSwitch.xaml` 328: searched only, since the Mac uses a native switch. `EE/Controls/Reveal.cs`, `IndexingStackPanel.cs`, `EE/Utils/ClipBorder.cs`, `GeometryHelper.cs`: heads only, and confirmed unreferenced apart from `ClipBorder` in ToggleSwitch's template. `EE/PropertyEditing/ValueExceptionEventArgs.cs`, `Filters/IPropertyFilterTarget.cs`, `Filters/PropertyFilterAppliedEventArgs.cs`: not opened, only referenced by the hidden filter and unused exception events.
- `CurveEditor.Designer.cs`, `FunctionGenerator.Designer.cs`, `ColorGradientEditor.Designer.cs`, `*LibrarySelector.Designer.cs`, `Form_*Library.Designer.cs`, `GradientEditPanel.designer.cs`: through the generated JSON, which omits `GradientEditPanel`'s positions.
- `ColorGradient.cs` lines 480-860 (blend algorithms and accessors): already ported in `crates/vixen-core/src/gradient.rs`, and not re-read for the first draft of this spec. The audit read them for `GetReverseColorGradient` (`ColorGradient.cs:761-796`, F49) and the `Colors` accessor (`ColorGradient.cs:837-849`).
- `plans/effects/vix-3964-effect-default-settings.md`: searched for the decision log only. `TSE/TimedSequenceEditorForm.cs`: only the drop handlers and call sites cited.
- `Vixen.Modules/Editor/PolygonEditor`: only the entry point; it needs its own spec, since this area just opens it (F37).
- No screenshots of the running app were consulted. Behaviour is from source, and WPF behaviour that the code relies on (shared-size columns, mouse capture routing, `Binding.Delay`, `ListView`'s default selection mode, `Slider`'s default keys, `EnumConverter`'s filtering and sort) is from knowledge of WPF and .NET, not from running it or from reading their source.

## Features

### Conventions

**Modifier keys.** Every ⌘ below is upstream Ctrl, ⌥ is upstream Alt, ⇧ is Shift (`docs/UI_GUIDE.md`). "Holding" means the key is among those down (`Modifiers & X != 0`); "exactly" means no other modifier is down (`Modifiers == X`). The difference matters and is stated per gesture.

**Units.** One upstream WPF unit is one point.

**Treatment tags.** KEEP: port as written. GUARD: upstream throws or crashes; do not panic, do the stated safe thing. FIX: a deliberate divergence, listed for the owner's veto in the quirk register at the end of Features.

**Entry format.** *Do*: what the user does. *Does*: exact behaviour. *Rules*: preconditions and enabling. *Default*. *Undo*. *Persist*. A missing line means none.

**Terms.** *Row*: one property's name and editor. *Selection*: the effects on the timeline currently shown in the panel. *First*: the first effect in the selection array, in `SelectedElements` order (`FormEffectEditor.cs:80`). *Live model*: the Swift object for the panel, fed by Rust. *Commit*: one call to Rust that sets a value on the selection and records one undo step.

### A. Panel, selection and header

**F1. Effect Editor panel.**
*Do:* View > Effect Editor Window (see `docs/spec/sequencer-shell.md` F70), right tab in the dock group.
*Does:* a dock panel titled **Effect Editor** (`Vixen.Modules__Editor__TimedSequenceEditor__Forms__FormEffectEditor.json`), filled edge to edge by the grid control (`FormEffectEditor.cs:40-50`), created with `ShowReadOnlyProperties = true` and `PropertyFilterVisibility = Hidden` (`:42-46`). Top to bottom: the Effect Preview bar (F5), the header bar (F7), the scrolling list of categories, then the information box at the end of the scrolled content (F11) (`EE/Themes/PropertyGrid.xaml:31-52`, `EE/Design/CategorizedLayout.xaml:18-75`).
*Rules:* the panel is per sequence window.
*Persist:* the dock position belongs to the sequencer's layout (sequencer-shell F1, F2).

**F2. Showing the selection.**
*Do:* select effects on the timeline.
*Does:* each timeline `SelectionChanged` restarts a **200 ms** one-shot timer (`FormEffectEditor.cs:59-63`, `:126-130`). When it fires, `Elements = TimelineControl.SelectedElements` (`:132-136`). The setter replaces the element list, runs F6's `SetPreviewState` (stop, then play if on), then assigns `SelectedObjects` (`:74-81`). In the grid: if the count is over the multi-edit limit, F4. Otherwise, an identical selection (same count, same object at each index) does nothing at all; anything else reloads every row (`EE/EffectPropertyEditorGrid.cs:197-256`; the single-to-single same-type case calls `SwapSelectedObject`, which only reloads, `:628-635`).
*Note:* an identical selection still restarts the preview, because the preview runs before the grid's comparison (`FormEffectEditor.cs:78-80`).

**F3. Nothing selected.**
*Does:* no categories and no rows. The header bar collapses because the effect name is empty (`EE/Design/CategorizedLayout.xaml:20`). The information box still shows, with the text **"Select an Effect to edit."** and a **More Info** link to `https://www.vixenlights.com/docs/usage/sequencer/sequencer-basics/effect-editor/` (`EE/EffectPropertyEditorGrid.cs:44-45`, `:277-278`). Mac: use the same text and link in the same place, not a `ContentUnavailableView`, unless the owner asks.

**F4. Too many effects selected.**
*Does:* selection count **greater than** `MultiEditLimit` (default **75**, overridden by the app setting `MultiEditLimit`, `EE/EffectPropertyEditorGrid.cs:42`, `:1169-1173`): the grid clears as in F3, and the information text becomes **`Multi-Edit is limited to < 75 Effects`** (`:1174`, `:197-206`, `:277`). The link stays the default one. The header goes too, so Save as Default and Reset to Built-in aren't there.
*Effect Preview is not limited:* the host stores the elements and runs F6 before the grid applies this limit (`FormEffectEditor.cs:76-80`), so with the switch on all the selected effects still play. KEEP.
*Quirk:* exactly 75 is allowed although the message says `< 75`. KEEP both. Mac: a `multi_edit_limit` value in Rust (default 75) in place of the app setting; show the limit in the text.

**F5. Effect Preview switch.**
*Do:* flip the switch labelled **Effect Preview**, showing **On** / **Off**, in the bar at the top of the panel, left aligned, the label before the switch (`EE/Themes/PropertyGrid.xaml:33-48`).
*Does:* sends the new state to the host: `_previewState = state`, then F6 starts or stops (`EE/PropertyGrid.Commands.cs:261-264`, `EE/EffectPropertyEditorGrid.cs:500-504`, `FormEffectEditor.cs:138-142`, `:305-318`).
*Rules:* always enabled, also with nothing selected; then it only remembers the state and plays from the next selection (`:233-240`).
*Default:* off, each time a sequence window opens. *Persist:* not saved. *Undo:* none.
*Mac:* a native `Toggle(.switch)` with the same label. The upstream control's own animated glow (`EE/Controls/ToggleSwitch.cs`, `EE/Themes/ToggleSwitch.xaml`) is look only and is dropped.

**F6. Effect Preview playback.**
*Does:* while on and with effects selected, the selection plays in a loop through a preview context named **"Effect Preview"**, through whichever previews and outputs are live (`FormEffectEditor.cs:256-281`):
1. Create the context on first use, assign the sequence, start it; if it exists but isn't running, log a warning and start it.
2. Order the effects by start time. `startOffset` = first's start. `duration` = **the last-starting effect's end** minus `startOffset`. An earlier-starting, longer effect that ends after that isn't cut off: `Execute` only adds data and nothing clears it (`Vixen.Core/Execution/Context/LiveContext.cs:57-60`), so it keeps playing underneath the next loop's copy (KEEP).
3. Execute copies placed at `start - startOffset`, so the first starts at 0, sharing the live effect objects. The context mixes through the sequence's layers, so the effects' layers apply (`Vixen.Core/Execution/Context/PreviewContext.cs:13-16`).
4. Run a repeating timer with interval `duration`. On each tick, if still on and something is selected, go to step 1 on the main thread (`FormEffectEditor.cs:243-254`). Nothing is cleared between loops.
`PreviewStop`: stop the timer. If the context is running and not paused, `Clear()`; otherwise log a warning and `Clear(false)` (`:283-298`). `ResumePreview` and the switch: on and selected, play; on with nothing selected, nothing; off, stop (`:300-303`, `:305-318`). Selection change: stop, then play if on and selected (`:233-240`). Editing a value doesn't restart anything: the effect goes dirty and the next loop plays the new render.
*Rules:* real playback and Export stop the preview and resume it afterwards (sequencer-shell F37, F267, F269).
*GUARD:* an effect selection with zero `duration` makes upstream's timer throw; do nothing for that loop.
*Teardown:* the context is released when the panel closes (`:95-98`).

**F7. Header bar.**
*Does:* shown only when the effect name isn't empty (`EE/Design/CategorizedLayout.xaml:20`). Left to right: the effect name, bold, 10 pt, centred in the remaining width; **Save as Default** button (disk icon); **Reset to Built-in** button (refresh icon); help icon link to the effect's `InformationLink` (`:29-46`). The name is the value of the `EffectName` property (`EE/EffectPropertyEditorGrid.cs:291-294`). In a mixed-type selection it is First's name, because merged `GetValue` returns First's value when values differ (`EE/Internal/MergedPropertyDescriptor.cs:239-248`).

**F8. Save as Default.**
*Do:* click the disk button (tooltip "Save as Default").
*Rules:* enabled only with exactly one effect selected (`EE/PropertyGrid.Commands.cs:74-78`).
*Does:* no prompt. Captures the effect's current data as the saved default of its effect type, replacing any earlier one (`:80-87`, `Svc/EffectDefaultsService.cs:134-160`). The capture never changes the live effect: serialize, deserialize a copy, scrub that copy, serialize it (`:486-492`). Scrubbing resets every member tagged `[ExcludeFromEffectDefault]` to the type's default, at any depth (Mark Collection ids, for example), leaves library links alone (`Svc/EffectDefaultScrubber.cs:8-15`, `:41-118`). New effects of that type then start from it, and an effect whose saved default can't be applied falls back to built-in values without an error (`Svc/EffectDefaultsService.cs:85-124`).
*Undo:* none. *Persist:* see Data and persistence.
*GUARD:* a failed write throws out of the command in upstream, since neither the command nor `PersistLocked` catches it (`EE/PropertyGrid.Commands.cs:80-87`, `Svc/EffectDefaultsService.cs:448-467`). Show an alert and keep the app running. Same for F9.
*Feedback:* none besides Reset to Built-in becoming enabled (`upstream/Vixen/docs/references/effects/effect-defaults.md` step 3).

**F9. Reset to Built-in.**
*Do:* click the refresh button (tooltip "Reset to Built-in").
*Rules:* enabled only when exactly one effect is selected and its type has a saved default (`EE/PropertyGrid.Commands.cs:89-93`, `EE/EffectPropertyEditorGrid.cs:1126-1133`).
*Does:* asks **"Remove the saved default for {TypeName}?"**, titled **"Reset Effect Default"**, Yes/No (`EE/PropertyGrid.Commands.cs:100-106`). Confirmed: deletes the entry and persists (`Svc/EffectDefaultsService.cs:169-181`). It changes **no** effect, the selected one included; only effects created later (`:163-165`). Mac: `.confirmationDialog` with verbs ("Remove Default", "Cancel"), destructive role.
*Undo:* none.

**F10. Help link.** Help icon in the header, and **More Info** in the information box (F11): opens `InformationLink` in the browser (`EE/Design/CategorizedLayout.xaml:42-44`, `:67`).

**F11. Information box.**
*Does:* at the end of the scrolled content, below the categories, so it scrolls with them; shown when the information text isn't empty, so an effect with empty text has no box and no More Info (`EE/Design/CategorizedLayout.xaml:51-72`, base values `""` and `http://vixenlights.com`, `Vixen.Core/Module/Effect/EffectModuleInstanceBase.cs:201-211`). The effect's `Information` text, wrapped, left aligned, and under it **More Info**. Values come from the `Information` and `InformationLink` properties (`EE/EffectPropertyEditorGrid.cs:295-302`). A mixed-type selection shows First's (F60). Rust: `EffectDescriptor::information`, `info_link` (`fw.rs:684-686`).

**F12. Scrolling.** Vertical scroll bar only when needed, horizontal disabled, so editors track the panel width (`EE/Design/CategorizedLayout.xaml:51`). Collection lists don't scroll on their own: they pass wheel events on (`EE/Themes/EditorResources.xaml:92-94`, and each collection template except the string list, `EE/Themes/TextEditor.xaml:49-55`, which has no such behaviour, so the wheel stops working over Text's lines in upstream). Mac: one scroll view for the panel, none nested, so the wheel works everywhere.

### B. Categories, rows and ordering

**F13. Category sections.**
*Do:* click a category's header to fold it.
*Does:* each category is an expander whose header is its name (`EE/Themes/PropertyGrid.xaml:118-121`). Shown only if browsable, having at least one visible row, and matching the filter (`:104-113`). Category `Order` defaults to -1 (`EE/CategoryItem.cs:109`).
*Default:* expanded (`EE/EffectPropertyEditorGrid.cs:1326-1340`, `EE/CategoryItem.cs:129`), **except Video's "Advanced Settings", which starts collapsed**: `[BrowsableCategory(@"Advanced Settings", true, false)]` (`Vixen.Modules/Effect/Video/Video.cs:21`; the third argument is `Expanded`, `Vixen.Core/Attributes/BrowsableCategoryAttribute.cs:37-42`). It is the only `[BrowsableCategory]`, `[BrowsableProperty]` or `[CategoryOrder]` on any effect (search of `Vixen.Modules`). The attribute is read from First's class only (`EE/EffectPropertyEditorGrid.cs:918-919`), so a multi-selection follows First. The rule below then overrides it, so opening Advanced Settings on one Video keeps it open on the next. See Extra metadata.
*Persist:* session only. On each reload a category takes the fold state of the same-named category that was on screen just before (`EE/EffectPropertyEditorGrid.cs:930-931`, `:992-1001`). So the state survives moving between effects that share a category, and is lost if a selection without that category comes between, or if the selection clears. KEEP.

**F14. Order.**
*Does:* categories sort by order, then name ignoring case, ordinal (`EE/PropertyEditing/CategoryItemComparer.cs:41-54`). Rows sort by `[PropertyOrder]` (0 if absent), then display name ignoring case, culture-sensitive (`EE/PropertyEditing/PropertyItemComparer.cs:41-63`). A category's order is that of the first of its properties met, later ones with a different order are ignored (`EE/EffectPropertyEditorGrid.cs:637-666`, `:1248-1252`). Already implemented by `visible_params_for` (`fw.rs:895-906`) and described in `docs/audit/editor-pass.md`. For a mixed selection see F61.

**F15. Row layout: single value.**
*Does:* 4 pt margin, two columns (`EE/Themes/PropertyGrid.xaml:86-95`). The name column belongs to a group shared by all rows of the panel (`:88`, scope set at `EE/Design/CategorizedLayout.xaml:53`), which makes it as wide as the widest name; the editor takes the rest. Name: right aligned, one line, ellipsis when cut, vertically centred (`EE/Design/PropertyNameTextBlock.cs:31-37`). Mac: `Grid` with a shared leading column, or an alignment guide.

**F16. Row layout: collection.**
*Does:* when the property is a list, the name is a centred header line above, and the editor takes the full width below with 4 pt margin (`EE/Themes/PropertyGrid.xaml:75-85`, `EE/Design/PropertyNameTextBlock.cs:51-56`).

**F17. Tooltips.**
*Does:* hovering a name shows the property's description, or its display name when it has none (`EE/PropertyItem.cs:471-480`, `EE/Themes/PropertyGrid.xaml:80`, `:91`). Mac: `.help(description)`.

**F18. Visibility changes while editing.**
*Does:* many effects hide and show rows depending on values (`UpdateAttributes`, then `TypeDescriptor.Refresh`). The grid responds by recomputing each row's browsable state and re-sorting, without rebuilding editors (`EE/EffectPropertyEditorGrid.cs:679-702`, `:973-990`). A category with no visible rows disappears (`EE/CategoryItem.cs:54-57`).
*Rust:* `EffectDescriptor::visible_params_for` after each commit. Swift must animate nothing and must not recreate surviving row views, so that a drag or text edit in progress stays alive.

**F19. Choice lists refresh.**
*Does:* any effect property change queues a refresh of that row's choices at idle priority; a `TargetNodes` change refreshes every row's choices (`EE/PropertyItem.cs:248-252`, `EE/EffectPropertyEditorGrid.cs:865-904`). Requests coalesce into one refresh. Row's value text refreshes immediately (`EE/PropertyItem.cs:250`).

**F20. Filter box: not shown.** The grid has a property filter (space-separated words, each matched case-insensitively against display name or type name, any match shows the row, `EE/PropertyEditing/Filters/PropertyFilter.cs:51-79`, `EE/PropertyItem.cs:112-122`), its commands and its visibility flag, but the Effect Editor sets it Hidden and neither layout template contains the box (`FormEffectEditor.cs:45`, `EE/Design/CategorizedLayout.xaml`). Not user visible: omit. Listed in Windows-only omissions as dead.

### C. Choosing an editor, and the simple editors

**F21. Which editor a row gets.**
*Does:* first match wins (`EE/Editors/EditorCollection.cs:205-256`):
1. Read-only property: a plain label (`:211-214`).
2. `[PropertyEditor("X")]`: class `X`, looked up as written and then in the Editors namespace; anything that doesn't resolve to an `Editor` class is ignored and the search goes on (`EE/Editors/EditorCollection.cs:123-140`). Values on effects (search of `Vixen.Modules`, with counts): `SliderEditor` 153, `SelectionEditor` 37, `LevelEditor` 5, `SliderDoubleEditor` 3, and one each of `FilePathEditor`, `ImagePathEditor`, `VideoPathEditor`, `SvgPathEditor`, `GledPathEditor`, plus two that resolve to nothing: `DoubleSliderEditor` (below) and `Label` (`MaxPixelsPerString`, which is read-only and hidden anyway, `Vixen.Modules/Effect/Effect/PixelEffectBase.cs:106-113`).
3. By type, in this order, first assignable type wins (`EE/Editors/EditorCollection.cs:37-68`, `:237-245`): bool, int, byte, ushort, uint, ulong, double, Color, Curve, ColorGradient, List of gradient, List of Color, List of GradientLevelPair, List of string, Percentage, any enum, Font, then the record lists and PolygonContainer.
4. A type whose converter has sub-properties: a disabled text box plus an expander of sub-rows (`:247-248`, `EE/Themes/EditorResources.xaml:31-39`). No Rust parameter reaches it; omit unless a table adds one. Video's `VideoSize` is the only `Record`, and it is hidden (`crates/fx-pixel-graphics/src/generated/video.rs:481-496`).
5. Otherwise: a text box (`EE/Editors/EditorCollection.cs:250-253`). This is what `long`, `float`, `short`, `string` and `Guid` get unless a `[PropertyEditor]` says otherwise.
*Known quirk:* Video's `StartTimeSeconds` declares `[PropertyEditor("DoubleSliderEditor")]` (`Vixen.Modules/Effect/Video/Video.cs:348`), and no such class exists (the class is `SliderDoubleEditor`), so step 2 fails and the user sees the numeric field F25, not a slider. The Rust table has `Editor::SliderDouble` for it (`crates/fx-pixel-graphics/src/generated/video.rs:418`). **The table must change** to `Editor::Number { min: 0, max: 10000, step: 1 }` to match; see Extra metadata.
*Rust:* `kind` + `editor` in `ParamDesc` replace the search. Table census (fx-* generated tables): Enum 208, Curve 175, Int/Slider 148, Bool 134, Int/Default 72, Gradient 28, GradientList 27, Str/Selection 13, Int/Selection 10, Int/Number 9, Guid/Selection 9, Str/Default 8, RecordList 7, Color 7, Double/Percent 5, Double/Level 5, Double/SliderDouble 4, Guid/Default 4, GradientLevels 3, Double/Default 3, Font 2, Path 5, Rgb 1, Record 1, Long/Number 1, Other 3 (`ulong`, `PolygonContainer`, `List<string>`).

**F22. Check box (`Bool`, no labels).**
*Does:* a plain check box bound to the value, disabled if read-only (`EE/Themes/EditorResources.xaml:52-54`). One click is one commit. Mac: `Toggle(.checkbox)`.
*Mixed:* shows First's value (F60).

**F23. Yes/No style choice (`Bool` with `bool_labels` and `Editor::Selection`).**
*Does:* a drop-down listing the true word then the false word, e.g. **Forward**/**Reverse** (`Vixen.Core/TypeConverters/BooleanStringTypeConverter.cs:88-97`). With no labels: **True**, **False**. Selection follows F27. Already provided by `selection_values` (`fw.rs:778-784`).

**F24. Integer field (`Int` with `Editor::Default` or `Number`).**
*Do:* type a number, or drag the 7 pt ramp under the field.
*Does:* a text box over a 7 pt wedge-shaped drag strip; the strip brightens on hover and while dragging, the cursor becomes a move cursor (`EE/Themes/IntegerEditor.xaml:11-50`).
*Typing:* commits on Return (on key up), and on focus loss; Escape reverts the text to the value; nothing commits while typing (`:14-16`, `WC/Utils/TextBoxExtender.cs:98-109`, `:199-210`, `FormEffectEditor.cs:180-210`). Parsing accepts an optional sign and surrounding spaces, no decimals; unparseable text reverts (`EE/Controls/IntegerEditor.cs:333-342`). The value truncates toward zero and clamps to the range (`:519-531`). With no range: minimum **0**, maximum the type's limit (`:137-144`). If the minimum exceeds the maximum, the maximum becomes the minimum (`:525-528`).
*Drag on the strip:* pressing captures the mouse and takes focus (`:381-395`). After 2 pt of travel the drag starts (`:410-423`). For each further move, `n = round(distance moved)`, in Banker's rounding; the value changes by `+n` when `dx > dy`, else `-n`, so right and up increase, left and down decrease (`:426-434`). Each step is multiplied: holding ⌘ by `SmallChange`, otherwise holding ⇧ by `LargeChange`, otherwise by the range's tick; `SmallChange` and `LargeChange` are never set, so both are 1 (`:469-475`, `:119-126`, `EE/Themes/EditorResources.xaml:70-81`). The text shows every step immediately. The value itself commits first after **300 ms**, then every **500 ms**, and finally on release (`EE/Controls/RepeatEditor.cs:74-86`, `:105-112`, `EE/Controls/IntegerEditor.cs:447-454`). A press and release with no movement also commits, which does nothing because the value is unchanged.
*Showing a value:* the bound value passes through the same clamp on its way in (`EE/Controls/IntegerEditor.cs:494-517`), so a stored value outside the range shows clamped. **GUARD:** on the Mac, showing a row never writes; the stored value changes only when the user commits. Whether upstream's two-way binding pushes the coerced value back to the effect wasn't established from the code.
*Undo:* every commit that changes the value is its own step (F63).
*Dead commands:* `IntegerEditor.Increase` and `Decrease` are registered, but no template or key binds them (`:96-97`, `:165-170`), and the same in `DoubleEditor`.
*Mac:* an AppKit text field with the strip beneath it. Trackpad scrolling doesn't change the value.

**F25. Decimal field (`Double` with `Editor::Default`).**
*Does:* as F24, in the same layout (`EE/Themes/DoubleEditor.xaml`), except: the text binds straight to the value, and any new value is clamped and rounded to the range's `Precision` decimal places in Banker's rounding, with **0 places when there's no range or none is given** (`EE/Controls/DoubleEditor.cs:296-303`, `:450-453`, `EE/Themes/EditorResources.xaml:57-68`). So a double with no `Precision` is rounded to a whole number. KEEP. The minimum without a range is 0 (`EE/Controls/DoubleEditor.cs:128-135`).
*Which rows:* Video's `StartTimeSeconds` (F21), Spin's `RevolutionCount` and `RevolutionFrequency` (no `[NumberRange]`, `Vixen.Modules/Effect/Spin/Spin.cs:221-251`). So those two Spin values can only be typed as whole numbers.
*Unparseable text:* unlike F24 there's no parse step. The binding's conversion fails, the value stays, and the text stays as typed until Escape or a later value change (`EE/Themes/DoubleEditor.xaml:12-16`; failure handling from knowledge of WPF). Mac: revert as F24, since it reads the same to the user.
*Showing a value:* the rounding runs on any new value once the editor is initialised (`EE/Controls/DoubleEditor.cs:296-303`), so a file's `RevolutionCount` of 2.5 could show and write 2 in upstream; this wasn't established by running it. **GUARD:** on the Mac, showing a row never writes.
*Extra metadata:* `Editor` has no precision; see Extra metadata.

**F26. Unsigned integers (`Long` / `Other("ulong")` with `Editor::Number`, and byte and ushort as `Int`).**
*Does:* CustomValue's `Value8Bit`, `Value16Bit`, `Value32Bit` and `Value64Bit` are byte, ushort, uint and ulong in C# (`Vixen.Modules/Effect/CustomValue/CustomValue.cs:277-325`), all F24, with the range clamped to the type's own limits, and the type kept when committing (`EE/Editors/EditorCollection.cs:41-44`, `EE/Controls/IntegerEditor.cs:494-569`). The table's `Long` is that uint. A real `long` isn't in the type list, and would get a text box, but no table has one.

**F27. Choice from a computed list (`Editor::Selection`).**
*Does:* a drop-down of the property's standard values (`EE/Themes/EditorResources.xaml:46-49`, `EE/PropertyItem.cs:424-433`). The current item is chosen by matching display **text**, so the row is blank when nothing matches, and choosing commits. The values come from the effect's converter: `EffectDescriptor::selection_values` (`fw.rs:771-796`).
*Mark collection pickers:* names of the sequence's mark collections, while the data holds the id (`Vixen.Core/TypeConverters/IMarkCollectionNameConverter.cs:44-73`, `fw.rs:789-791`). The C# property is a **string** that reads the name of the collection with the stored id, null when there's none, so the row is blank; writing finds the first collection with that name, or stores `Guid.Empty` (`Vixen.Modules/Effect/Fireworks/Fireworks.cs:73-93`). So with two collections of one name, choosing either picks the first. KEEP. In a multi-selection the list comes from the first effect that supports marks. Nothing in the list if no marks exist. The list refreshes when a collection is added, removed or renamed, because the effect raises a change on `MarkCollectionId` (`Vixen.Modules/Effect/Effect/BaseEffect.cs:148-162`) (F19). LipSync's list holds only Phoneme collections plus the one selected (`Vixen.Modules/Effect/LipSync/LipSyncMarkCollectionNameConverter.cs:69-87`). Inside a record list (an Emitter's or a Wave's selector) the list comes from the row's own mark collections (`Vixen.Modules/Effect/Effect/ExpandoMarkCollectionNameConverter.cs:44-60`).
*Other lists:* F105 has every converter and how each merges in a multi-selection.
*Depth pickers ("Levels Deep"):* F28.
*Mixed:* blank until the user chooses when the converter is `TargetElementDepthConverter` (F60).

**F28. Depth choices.** `Vixen.Core/TypeConverters/TargetElementDepthConverter.cs:45-92`. `depth` is the first target's `GetMaxChildDepth()`; in a multi-selection, the smallest of each effect's; with no target node, 0, which gives an empty list. Items are `offset + i` for `i` in `0..depth`, as text, where `offset` comes from `[Offset]` on the property, else 0 (`:80-89`). No effect declares `[Offset]` (search of `Vixen.Modules/Effect`), so `offset` is always 0. `fw.rs:793` and `depth_choices` give the single-effect case; `panel_model` must add the multi-selection minimum (F105).
*Fire and Wipe differ:* their `DepthOfEffect` uses its own converter with items `1..depth-1`, and the same minimum across a multi-selection (`Vixen.Modules/Effect/Fire/FireTargetElementDepthConverter.cs:35-70`, `Vixen.Modules/Effect/Wipe/WipeTargetElementDepthConverter.cs:49-85`; Rust `crates/fx-pixel-simple/src/fire.rs:27`, `crates/fx-basic/src/wipe.rs:43`). They don't derive from `TargetElementDepthConverter`, so they do **not** go blank when mixed: F60 shows First's value for them. The ones that do go blank are Chase, Twinkle, Spin, Alternating, Dissolve and the audio effects through `AudioPluginBase` (VU Meter, Vertical Meter, Waveform). In a mix, First's converter decides: Wipe first means not blank, Chase first means blank.

**F29. Enum drop-down (`Enum`).**
*Does:* a drop-down listing the members the type's `EnumConverter` gives, each shown by its `[Description]` text or its name (`EE/Converters/EnumValueConverter.cs:47-68`, `EE/Controls/EnumDropdown.cs:62-63`, `EE/Themes/EnumEditor.xaml:9-21`). That converter sorts by numeric value and leaves out members marked `[Browsable(false)]` (.NET behaviour, from knowledge of `EnumConverter.GetStandardValues`). Every effect enum is in declaration order by value (the only explicit values are `PinWheelSizeScaleBasis`'s 0, 1, 2), so declaration order holds. **One hidden member exists: `TilePictures.None`** (`Vixen.Modules/Effect/Picture/TilePictures.cs:7-10`, comment there: no longer required, but users may have it saved). It isn't offered; a stored `None` is kept and shows a blank row, because nothing in the list matches. The Rust table lists it (`crates/fx-pixel-graphics/src/generated/picture.rs:23`), so `selection_values` must skip it: see Extra metadata. Choosing commits at once (`EE/Controls/EnumDropdown.cs:138-146`). Pressing Down while focused opens the list (`:80-87`). The list follows value changes from elsewhere, undo included (`:116-123`).
*Mixed:* blank until the user chooses (F60). `(null)` is never shown because the blank state has no selected item.
*Mac:* `Picker(.menu)` with an empty selection state for mixed values.

**F30. Label (read-only).**
*Does:* value as text, not editable (`EE/Themes/TextEditor.xaml:22-26`, `EE/Editors/EditorCollection.cs:211-214`). The panel sets `ShowReadOnlyProperties = true`, so read-only properties do appear when browsable (`FormEffectEditor.cs:44`, `EE/EffectPropertyEditorGrid.cs:1279`).
*Extra metadata:* the tables have no read-only flag; see Extra metadata. `MaxPixelsPerString` (`Vixen.Modules/Effect/Effect/PixelEffectBase.cs:106-113`) is read-only but `Browsable(false)`, so it never shows.

**F31. Text field (`Str`, `Guid` and other values with `Editor::Default`).**
*Does:* a text box over the value's text. Commits on Return and on focus loss; Escape reverts; no commit while typing (`EE/Themes/TextEditor.xaml:12-20`). Empty text means null for non-string types (`EE/PropertyItemValue.cs:439-447`). Errors while setting are logged and the value stays (`EE/PropertyItem.cs:805-809`). A converter that rejects the text throws earlier, in `ConvertStringToValue`, outside that catch; the binding swallows it, the value stays and the box keeps the typed text (`EE/PropertyItemValue.cs:579-598`; the swallowing is from knowledge of WPF). Mac: one-line `TextField`; on rejected text, keep the value and revert the field. Which rows: `Str`/`Default` that are browsable are LipSync's `LyricData`, CustomValue's `StringValue`, CountDown's `CountDownTime`, RDS's `Title`, Launcher's `Arguments` and `Description`; the `Guid`/`Default` rows are all hidden (table census).

**F32. Sliders (`Int` + `Editor::Slider`, `Double` + `Editor::SliderDouble`, `Double` + `Editor::Level`, `Double`/`Percentage` + `Editor::Percent`).**
*Do:* drag the thumb; click the track, and the thumb jumps there; use arrow keys or Page Up/Down, or Home and End for the minimum and maximum (arrows, Home and End are `Slider`'s defaults, from knowledge of WPF).
*Does:* all four are the same control: range from the table with fallback 0..100, `SmallChange` = tick, `LargeChange` = 5, value tooltip while dragging at bottom right showing the slider's own value at the range's `Precision` decimals (fallback 0, `AutoToolTipPrecision`), full width of the editor column, no value text beside it (`EE/Themes/SliderEditor.xaml:8-82`). Differences:

| Editor | Value on slider | Snaps to ticks | Hover tooltip | Writes back |
|---|---|---|---|---|
| `SliderEditor` | int shown as double | **yes** | `Convert.ToInt32(value)` | `Convert.ToInt32(slider)`, Banker's rounding |
| `SliderDoubleEditor` | double | no | `Convert.ToInt32(value)`, rounds | double |
| `LevelEditor` | `stored * 100` | **yes** | `(int)(stored * 100)`, truncates | `slider / 100` |
| `SliderPercentageEditor` | `value * 100` | no | `Convert.ToInt32(value * 100)`, rounds | `f = (float)slider / 100`, and `f` outside 0..1 becomes **1**, not a clamp |

Sources: `EE/Converters/IntToDoubleConverter.cs`, `DoubleToIntConverter.cs`, `LevelValueConverter.cs:10`, `:15`, `LevelToIntConverter.cs:10`, `PercentageValueConverter.cs:11-26`, `PercentageToIntConverter.cs:11-26`.
*Commit timing:* the binding has `Delay=300`: the value is written **300 ms after the last slider change**, also during a drag, also when no button is up yet (`EE/Themes/SliderEditor.xaml:12`, `:31`, `:50`, `:69`). So a slow drag commits several times, each one its own undo step (F63), and the thumb moves on the panel before anything renders. Track repeat: first after 300 ms, then every 500 ms (`:10-11`).
*Tick in shown units:* the slider's range and tick are in shown units; see F33.
*Mac:* `NSSlider` in a representable, so that tracking is per-frame and doesn't go through SwiftUI state. Same delay: commit after 300 ms idle and at mouse up, not on every tick. See Performance hazards and F63. **Flush rule:** a pending delayed commit (this one and F24's timer) is flushed to the selection it was made on before the selection changes, the panel closes, a dialog opens, a preview loop restarts or an undo runs. With the 200 ms selection debounce and this 300 ms delay, a change made just before clicking another effect would otherwise be lost or land on the wrong effect. Upstream tears the row down at that point; what its delayed binding does then wasn't established.

**F33. Display transforms.** A few C# properties convert between the stored value and what the editor shows. The editor, its range and its tick work in **shown** units, the table's `display` converts, integer arithmetic as in the setters (`fw.rs:101-127`):

| Effect | Property | Shown label | Table | Shown | Stored | Range, tick | Source |
|---|---|---|---|---|---|---|---|
| VU Meter, Vertical Meter, Waveform | `Gain` | Gain | `Times(10)` | `stored * 10` | `shown / 10` | 0..200, 0.5 | `Vixen.Modules/Effect/AudioHelper/AudioPluginBase.cs:44-61` |
| Fireworks, Liquid | `Gain` | Gain | `Times(10)` | as above | as above | 0..200, 0.5 | `crates/fx-pixel-motion/src/generated/fireworks.rs:235`, `crates/fx-pixel-physics/src/generated/liquid.rs:270` |
| VU Meter, Vertical Meter, Waveform, Liquid | `Range` | Zoom | `Minus(20)` | `20 - stored` | `20 - shown` | 0..20, 1 | `AudioPluginBase.cs:148-163` |
| Waveform | `ScrollSpeed` | Scroll Speed | `Minus(50)` | `50 - stored` | `50 - shown` | 0..50, 1 | `Vixen.Modules/Effect/Waveform/Waveform.cs:34-48` |

Consequences to keep:
- Gain's slider ticks are 0.5, so it passes through non-multiples of 10. A write `shown = 37` stores 3 and reads back 30. After the 300 ms commit the thumb jumps back to 30. Any `Times` property behaves so; KEEP.
- A write that changes the shown value but not the stored one (Gain 30 to 35) still commits. Upstream pushes an undo step that restores nothing visible. **FIX** proposal: no undo step when stored values are equal.
- `Int`-only: `Display::shown` and `stored` take `i32`.
- Undo restores the *shown* value through the setter upstream, which round-trips exactly because shown values only ever come from getters. Rust should store and restore the stored value; the result is the same.
- Swift never computes these: the FFI gives shown values and takes shown values.
- Other conversions are editors rather than `display`: `Level` and `Percent` above, and SetLevel's `Rgb`, which shows and takes a `Color` (F36).

**F34. File path.**
*Do:* click **...** beside the path.
*Does:* a read-only text box with the path, tooltip the same text, and a 20 pt wide **...** button that hides when read-only (`EE/Themes/TextEditor.xaml:28-45`). The button opens an open-file dialog, single selection; Cancel leaves the value; OK commits the full path (`EE/PropertyGrid.Commands.cs:240-251`). Filters and title by kind:

| `Editor::Path` | Filter | Title | Source |
|---|---|---|---|
| `File` | none | default | `EE/Editors/FilePathEditor.cs:15-24` |
| `Image` | "Image Files (*.jpg, *.jpeg, *.png, *.gif, *.bmp)" | display name | `EE/Editors/ImagePathEditor.cs:15-26` |
| `Video` | "Video Files (*.mp4, *.avi, *.mov, *.MTS)", pattern `*.mp4;*.avi;*.mov;*MTS` (no dot in the last, KEEP) | display name | `EE/Editors/VideoPathEditor.cs:15-26` |
| `Svg` | "Image Files (*.svg)" | display name | `EE/Editors/SvgPathEditor.cs:15-26` |
| `Gled` | "Glediator Files (*.gled)" | display name | `EE/Editors/GledPathEditor.cs:15-26` |

*Mac:* `NSOpenPanel` as a sheet on the sequencer window, `allowedContentTypes` from the same extensions. Stored path syntax is a question for the effects: see Data and persistence.

**F35. Font.**
*Do:* click **...**.
*Does:* a read-only text box with the font's text, tooltip the value, and a 20 pt **...** button (`EE/Themes/FontEditor.xaml:9-26`). Opens the system font dialog without colour, starting at the current font (`EE/Editors/FontEditor.cs:16-45`). OK commits. Any exception: an error box **"An error occurred loading the font."**, titled **"Font Error"** (`:36-41`).
*Text format:* the C# Font's converter output, for example `Arial, 8pt` (`EE/PropertyItemValue.cs:454-473`). Rust should provide it: `font_display_text(&FontDesc)`.
*Mac:* `NSFontPanel`, sheet style not available, so a panel anchored by the button, committing when the user dismisses it or on each change coalesced to one undo step per panel session. Font size stays points; `FontDesc` has name, size, bold, italic (`fw.rs:188-195`), and the font panel can pick families and faces that upstream's name cannot address, so Rust keeps only what `FontDesc` holds.

**F36. Single colour (`Color`, `Rgb`).**
*Do:* double-click the 25 pt swatch.
*Does:* a bordered 25x25 swatch, then the value's text to its right (`EE/Themes/SystemColorEditor.xaml:10-33`). Null shows DimGray (`EE/Converters/SystemColorToSolidBrushConverter.cs:13-18`). The swatch is a drag source and drop target (F67, F68). Double-click (only, there's no Enter binding on this one) opens (`EE/Themes/SystemColorEditor.xaml:20-23`):
- Targets with discrete colours: F100, starting from the current colour, or the first discrete colour if null.
- Otherwise: the full colour picker (F103), with the row's display name as title, value lock off, starting from the current colour or white if null (`EE/Editors/ColorTypeEditor.cs:17-63`). Mac: system colour panel per `docs/UI_GUIDE.md`, one commit when it's dismissed.
*Text:* `System.Drawing.Color`'s converter text, e.g. `255, 128, 0` or a known name (`EE/PropertyItemValue.cs:454-473`). With a named-colour identity not kept in Rust (`docs/audit/editor-pass.md`, ".NET Color equality"), Rust must produce the text: `R, G, B`, with `A, R, G, B` when alpha isn't 255.
*Rules:* read-only rows don't open (`EE/PropertyGrid.Commands.cs:247`).

**F37. Polygons (`Other("PolygonContainer")`).**
*Do:* click **Edit Polygons**.
*Does:* a single button, hidden when read-only (`EE/Themes/PolygonEditor.xaml:9-16`). Clones the value, opens the modal polygon editor (`Vixen.Modules/Editor/PolygonEditor`, its own spec), and on OK commits a **new** container with the edited polygons, lines, ellipses and their times, keeping width and height (`EE/Editors/PolygonContainerTypeEditor.cs:36-78`). Any failure: **"An error occurred loading the polygon editor."**, titled **"Polygon Error"**.

### D. Inline curve row (`Curve`)

Sources: `EE/Controls/BaseInlineCurveEditor.cs` (the logic), `EE/Controls/InlineCurveEditor.cs`, `EE/Themes/CurveEditor.xaml`, `EE/Converters/CurveToImageConverter.cs`, `Vixen.Modules/App/Curves/Curve.cs`.

**F38. The curve strip: look.**
*Does:* a 1 pt `#888888` border around a strip 32 pt tall filling the editor's width (`EE/Themes/CurveEditor.xaml:25-33`). The strip stretches a 300x30 bitmap (302x32 when editable), black background, 2 pt line, line colour `(0,128,255)` if editable and `(136,136,136)` if linked to the library, points as 6x6 `(136,136,136)` dots (`EE/Converters/CurveToImageConverter.cs:19-34`, `Curve.cs:210-257`). Point mapping: `X = (int)x`, `Y = |(int)y - 100|`, then `Y = (int)(Y * (h/100f))`, `X = (int)(X * ((w-1)/100f))` (`Curve.cs:325-332`). A null value draws a default ramp on DimGray (`CurveToImageConverter.cs:34`).
*Library badge:* the 22x22 `librarylink.png` at the top left, when linked (`EE/Themes/CurveEditor.xaml:41-42`, `EE/Converters/ToolTipVisibilityConverter.cs:12-16`).
*Tooltip:* **`Library: {name}`** when linked, none otherwise (`Vixen.Modules/App/Curves/CurveTypeConverter.cs:57-67`).
*Level bar:* below the strip, a 10 pt tall area with a single handle, margin 3 above and below, collapsed unless ⇧ is held (F42).
*Mac:* one AppKit view draws strip, dots, badge and handle directly with Core Graphics, in real size, no bitmap (Performance hazards).

**F39. Curve strip: move a point.**
*Do:* press within 8 curve units of a point, and drag.
*Does:* positions convert to curve units: `x = 100 * px / width`, `y = 100 * (32 - py) / 32` against the strip (`BaseInlineCurveEditor.cs:623-634`). Distance is Euclidean **in curve units**, so 8 units is 8% of the width but only about 2.5 pt vertically (`:67`, `:618-621`). The closest point by that measure is chosen on press (`:263-265`, `:601-616`). Press near a point, or with exactly ⇧ or exactly ⌘ held, captures the mouse (`:266-277`). After 2 pt of travel the drag starts, and that first move only starts it: it moves nothing and **replaces the press position with its own** (`_dragStartPoint`, `:309-315`), which F40 and F41 use later. Each further move with the pointer not past `width + 5` or `height + 5` (no check on the left or top): the point's value is rounded away from zero to whole units and clamped to 0..100, written at its original index, **without sorting**, so a point can pass its neighbours (`:516-530`, `:636-662`). A tooltip below the control shows `x, y` as whole numbers (`:524-525`, `:664-683`). The strip redraws live from a hold value; the effect isn't touched until release.
*Mode follows the keys at each move:* exactly ⇧ at that move makes the flat level of F42, anything else moves the point (`:319-326`), and each works from the original curve. So holding or releasing ⇧ mid-drag flips between them, and the last move's result is what's held.
*Any button:* press capture needs the left button (`:261`), but the release tests of F40, F41 and F43 don't look at which button came up (`:372-388`), and a release with no press on the strip uses the last press position. GUARD: only a left press and its own release count.
*Hover cursor:* crosshair within 8 units of a point, arrow elsewhere (`:347-355`).
*Commit:* on release, if dragging or pressed or the hold value differs (`:390-396`). One undo step. Nothing renders during the drag.
*Rules:* a library-linked curve ignores all of F39 to F43 (`:259`, `:291`, `:376`) and any drag on it is a value drag, F67. So is a press not near a point, with no exact ⇧ or ⌘. Upstream keeps these apart by capture: capture on press routes moves away from the drag source. Mac: decide at mouse-down in one view.

**F40. Curve strip: add a point.**
*Do:* ⌘-click the strip, exactly ⌘.
*Does:* exactly ⌘ is read at **release**. Adds a point at the press location (rounded, clamped), sorted by X, and commits (`:381-384`, `:549-562`, `:390-396`).
*Quirk:* ⌘-**drag** shows the closest point moving during the drag (`:319-326`), then on release that is thrown away and a new point is added to the original points where the drag began (`:553`), which is the first move past 2 pt, not the press (`:314`). Any drag that ends with exactly ⌘ held does the same, however it started. KEEP.

**F41. Curve strip: remove a point.**
*Do:* ⌥-click near a point, exactly ⌥.
*Does:* exactly ⌥ is read at **release**, and the press must have been captured. Removes the point closest to the press location (after a drag, the first move past 2 pt, as in F40) and commits; refuses below 3 points, since a curve needs two (`:385-388`, `:564-582`). A press not within 8 units isn't captured, and nothing happens (`:266-268`). So a press near a point that ends with ⌥ held removes a point, even though ⌥ wasn't down at the press.

**F42. Curve strip: set a flat level.**
*Do:* hold ⇧ over the strip: the level bar appears, with a handle. ⇧-drag anywhere on the strip, or drag the handle.
*Does:* the bar is visible while ⇧ is held and the pointer is inside, and hides on leaving (`BaseInlineCurveEditor.cs:238-249`, `:293-300`). It only updates on a mouse enter or move, so pressing ⇧ over a still pointer shows nothing until the pointer moves; opening the bar pushes the rows below it down by 16 pt. Hover cursor with ⇧: resize-horizontal over the handle, resize-vertical elsewhere (`:335-346`).
- ⇧-drag on the strip (exactly ⇧ while moving): replaces the curve with two points `(0,y),(100,y)`, `y` rounded and clamped; tooltip `y` (`:532-547`). ⇧-click with no movement changes nothing.
- Dragging the handle, sideways: `level = Round(pos * 100, AwayFromZero)`, where `pos` is the handle's horizontal position over the bar's width, makes the same flat curve (`:492-514`). Handle tooltip `pos * 100`, whole, open from press to release (`EE/Controls/SliderPoint.cs:273-292`, `:231`, `:217`). The handle's position is `y/100` when the curve is flat with 2 points, else 0 (`BaseInlineCurveEditor.cs:432-467`).
*FIX, same upstream bug as F46:* this handle is a `SliderPoint` too, so the curve and hold value only update when two successive moves land on the same whole x. Mac: apply every move.
*Commit:* on release (`:483-490`), one undo step.

**F43. Curve strip: invert, reverse.**
*Do:* ⇧⌘-click to invert (`Y = 100 - Y`). ⇧⌥-click to reverse (`X = 100 - X`). With all three, reverse wins.
*Does:* tested at release, holding (`BaseInlineCurveEditor.cs:377-380`, `:407-430`). Sorted by X, and commits one undo step. The press needs no nearness.
*Open dialog:* double-click the strip, or Return while it has focus: F70, row name as title (`EE/Themes/CurveEditor.xaml:34-39`, `EE/Editors/CurveEditor.cs:13-24`). Only OK commits.

### E. Inline gradient row (`Gradient`)

Sources: `EE/Controls/BaseInlineGradientEditor.cs`, `EE/Controls/InlineGradientEditor.cs`, `EE/Controls/SliderPoint.cs`, `EE/Themes/ColorGradientEditor.xaml`, `EE/Converters/ColorGradientToImageConverter.cs`.

**F44. The gradient strip: look.**
*Does:* 1 pt `#888888` border around a strip 32 pt tall filling the width, stretching a 300x30 bitmap (`EE/Themes/ColorGradientEditor.xaml:27-41`, `EE/Converters/ColorGradientToImageConverter.cs:12-30`). Null draws DimGray (`:33-35`). Library badge at top left when linked; tooltip **`Library: {name}`** (`Vixen.Modules/App/ColorGradients/GradientTypeConverter.cs:42-53`). Below it, always, a 10 pt band with a handle per distinct stop position; the band is hidden, keeping its space, when linked (`EE/Themes/ColorGradientEditor.xaml:47-53`, `EE/Converters/ColorGradientIsLibraryToVisibilityConverter.cs:15-22`).
*Handles:* a 10x10 pentagon, flat base, peak up, with points `(0,10) (0,5) (5,0) (10,5) (10,10)` (`SliderPoint.cs:137-148`), centred on `position * width`. Metallic gradient; lighter when focused (`EE/Themes/SliderPoint.xaml:8-56`). Stops sharing an exact position share a handle (`BaseInlineGradientEditor.cs:485-512`). A handle is omitted if its centre falls outside the band (`:526`).
*Discrete rendering:* when the selection has discrete colours the strip draws the gradient in its discrete form (`:595-606`), see F94.

**F45. Gradient strip: add a stop.**
*Do:* click an empty part of the band.
*Does:* adds a stop at `x / width` and commits at once, one undo step (`:109-120`, `:566-593`). Colour: discrete: the first valid colour; otherwise the gradient's colour at that position (`:570-583`). Hidden on linked gradients, so not available there.

**F46. Gradient strip: move a stop.**
*Do:* drag a handle.
*Does:* press takes focus and captures the mouse; after 2 pt the drag starts; each move: `center = Round(x)` (Banker's), applied if `0 <= center <= width` (`SliderPoint.cs:166-195`). Every stop under the handle moves to `center / width`, and the strip redraws from a hold value (`BaseInlineGradientEditor.cs:180-217`). Commit on release, one undo step (`:168-177`). Handles don't reorder or merge during the drag; the band reloads from the new value after commit. Since `center` is a whole point, a handle dropped on the same point as another gets exactly its position, and from then on they are one handle with both stops (F44). Cursor over a handle: resize-horizontal (`:69-80`).
*FIX, upstream bug:* `SliderPoint.Center`'s setter tests `value == Center` and raises its change event only when the value did **not** change (`SliderPoint.cs:36-49`). So the strip and hold value update only when two successive moves land on the same whole x, and the last positions of a fast drag are lost; if no such pair occurs the hold value stays null and nothing commits (`BaseInlineGradientEditor.cs:171-175`), leaving the handle drawn where it was dropped. Mac: apply every move and commit on release. This is the intended behaviour, and the choppy feel the owner remembers.

**F47. Gradient strip: delete a stop.**
*Do:* ⌥-click a handle (exactly ⌥, no drag), or focus one and press Delete.
*Does:* removes all stops under that handle and commits (`SliderPoint.cs:211-215`, `BaseInlineGradientEditor.cs:154-166`, `:540-564`).
*GUARD:* nothing prevents deleting the last stop. See F94 for what an empty gradient does.

**F48. Gradient strip: edit a stop's colour.**
*Do:* double-click a handle.
*Does:* needs the handle focused, which the first click does (`:96-107`, `:476-483`).
- Not discrete: the colour picker opens with the last mode and fader used by any inline strip in this session, default `HSV_RGB` and `HSV_H`, value lock off, the stop's colour (`:32-33`, `:460-471`). OK stores colour and the remembered mode and fader, and commits. A handle holding more than one stop first shows an error box **"Non-discrete color gradient, >1 selected point. oops! please report it."**, titled **"Delete library gradient?"** (sic), then edits the first (`:446-453`). Mac: system colour panel, F36.
- Discrete: F99 with the valid colours and the stops' colours ticked. OK with none ticked: F47. OK with some: all the handle's stops are replaced by one stop per ticked colour at the same position, in list order, and commit (`:397-442`).

**F49. Gradient strip: reverse.**
*Do:* ⇧⌥-press on the strip. On the band a press adds a stop instead (F45), because that handler marks the press handled first.
*Does:* holding both, any mouse button; if not linked and more than one stop: commits the reversed gradient at once, one undo step (`BaseInlineGradientEditor.cs:82-94`). Every mouse press reaching the row is swallowed, so a press on a strip never selects a list row underneath (`:93`).
*Reversal:* each stop's position becomes `1 - position`, and each stop's focus becomes `1 - focus` of the stop that followed it; the first after reversal keeps its own focus (`ColorGradient.cs:761-796`). Its sort has a quirk: `colorPosition` isn't reset per stop, so a list whose stops aren't in order can pair focuses oddly (KEEP). `ColorGradient::reversed` already implements this (`crates/vixen-core/src/gradient.rs:360`).

**F50. Gradient strip: open dialog.**
*Do:* double-click the strip, or Return with the strip focused: F87, row name as title (`EE/Themes/ColorGradientEditor.xaml:36-39`, `EE/Editors/GradientTypeEditor.cs:14-36`). OK commits.

**F51. Discrete awareness.** The strip reads its effect's `HasDiscreteColors` and valid colours when created and whenever its component changes (`BaseInlineGradientEditor.cs:332-356`). Several effects: discrete if any is, valid colours the union. A strip whose component is a row of a record list isn't an effect, so **gradients inside emitters, waves and so on are never treated as discrete**, in the strip or the dialog (`:334-343`, `EE/Internal/Util.cs:19-48`). KEEP.

### F. Collections

Sources: `EE/Themes/ColorGradientPaletteEditor.xaml`, `ColorPaletteEditor.xaml`, `GradientLevelPairEditor.xaml`, `TextEditor.xaml`, `EditorResources.xaml`, `EE/Controls/CollectionView.cs`, `EE/PropertyItemValue.cs`, `EE/PropertyItem.cs`, `EE/Internal/CollectionItemValue.cs`.

**F52. Collection row kinds.** Each is a list with a selected row, then buttons. All are full width with the name as a header (F16). The list doesn't scroll by itself (F12).

| Kind | Items | Item header | Buttons | Button place | Source |
|---|---|---|---|---|---|
| `GradientList` | gradient strip, F44 to F50, stacked | none | Add, Remove, Up, Down | below | `ColorGradientPaletteEditor.xaml:12-62` |
| `ColorList` | 25x25 swatch, wrapped left to right, F36's gestures, Return too | none | Add, Remove, Up, Down | below | `ColorPaletteEditor.xaml:13-66` |
| `GradientLevels` | curve strip above gradient strip, F53 | none | Add, Remove, Up, Down | below | `GradientLevelPairEditor.xaml:13-70` |
| `List<string>` | one text box each, F54 | none | Add, Remove | below | `TextEditor.xaml:47-81` |
| `RecordList` Emitter | sub-rows, F55 | "Emitter {n}" between two rules | Add, Remove | below | `EditorResources.xaml:83-146` |
| `RecordList` Waveform | sub-rows | "Wave {n}" | Add, Remove | below | `EditorResources.xaml:216-279` |
| `RecordList` MorphPolygon | sub-rows | "Polygon {n}" | Add, Remove | below | `EditorResources.xaml:403-466` |
| `RecordList` Whirl | sub-rows | "Whirl {n}" | **none** | - | `EditorResources.xaml:281-335` |
| `RecordList` Fixture function | sub-rows | the item's own text, bold, between two 2 pt rules in the item's timeline colour | Add, Remove | **above** | `EditorResources.xaml:149-214` |
| `RecordList` CustomStateItem | one line: State Item drop-down, then Color at the end; no labels | none | Add, Remove | below | `EditorResources.xaml:337-400` |

`{n}` is the 1-based row (`WC/Converters/IndexOfConverter.cs:20-26`). All item headers are bold, centred.

Notes on the table:
- `ColorList` has no user today: no effect declares a `List<Color>` property (scan of `Vixen.Modules/Effect`), and no table has `ParamKind::ColorList`. Build it last; it can be tested only by a fixture. Its Return binding is dead in upstream too, because the swatch's `Rectangle` can't take focus (`EE/Themes/ColorPaletteEditor.xaml:33-41`; focus rule from knowledge of WPF).
- Pair and record items get an outline on hover through `ListViewItemOutlineHover` (`Vixen.Common/WPFCommon/Theme/Theme.xaml:803-826`, `EE/Themes/EditorResources.xaml:91`, `EE/Themes/GradientLevelPairEditor.xaml:21`); gradient, colour and string items don't.
- Add, Remove, Up and Down are hidden when the row is read-only (`Visibility` on each button). No table row is both read-only and a list.
- A dialog opened from a list item is titled with the list's display name (`EE/PropertyGrid.Commands.cs:178-186`, `:253-259`); that path has no read-only check.
- The two controls of a Custom State row show whatever their browsable state: that item template has no visibility binding (`EE/Themes/EditorResources.xaml:340-349`). F106 has the rest of that row.
- Morph's list can't add when empty, because `MorphPolygonsObservableCollection` isn't an expando collection (`Vixen.Modules/Effect/Morph/Morph/MorphPolygonsObservableCollection.cs:8`, F56).

**F53. Gradient and level pair rows.** One item is a curve strip with margin 5 top and bottom, then a gradient strip below, each editing its half: `new GradientLevelPair(gradient, curve)` written at the item's index (`EE/Controls/InlineGradientPairCurveEditor.cs:61-67`, `InlineGradientPairGradientEditor.cs:59-65`). Gestures as D and E, one undo step each. Double-click on the curve half or Return: curve dialog, committing the new curve only; double-click on the gradient half (no Return): gradient dialog, committing the gradient only (`EE/PropertyGrid.Commands.cs:192-238`, `EE/Themes/CurveEditor.xaml:76-81`, `EE/Themes/ColorGradientEditor.xaml:76-79`). A new pair: default gradient and a **Flat 100** curve (`Vixen.Modules/App/ColorGradients/GradientLevelPair.cs:12-16`). Hover outline on the item (`Vixen.Common/WPFCommon/Theme/Theme.xaml:803-826`). Each half's strip shows the same `Library: {name}` tooltip and badge as a single row when that half is linked (`EE/Themes/CurveEditor.xaml:82-87`, `EE/Themes/ColorGradientEditor.xaml:80-86`), and a linked gradient half hides its band (`EE/Converters/ColorGradientIsLibraryToVisibilityConverter.cs:19-23`). The dialogs' title is the list's display name.
*Quirk, an unchanged pair still commits:* `GradientLevelPair` has no `Equals`, so `SetCollectionValue`'s equality test (`EE/PropertyItem.cs:647`) only passes for the same object, and every write here is a new pair. So a click near a curve point with no movement, and **Cancel in either dialog** (the handlers build a new pair from what the dialog returned, `EE/PropertyGrid.Commands.cs:204-209`, `:227-232`), each push an undo step, mark the effect dirty and reload the list. **FIX:** compare both halves and commit nothing when both are equal.

**F54. String lists.** Each row is a text box that commits on Return and focus loss, Escape reverts, not while typing (`EE/Themes/TextEditor.xaml:63-67`). Editing commits one undo step of the whole list. Used only by Text's `TextLines`. This list alone sets `IsSynchronizedWithCurrentItem` (`:54`), which selects the first row when the list loads (from knowledge of WPF), so Remove is enabled at once when there's more than one line and removes the first. Clicking inside a text box doesn't select its row; only the margin around it does, and focus leaving a box clears the selection (F58). **FIX** with F58: clicking in a text box selects its row.

**F55. Record lists (sub-rows).** Each item renders its own property rows with the same layout and editors (F15), sorted by the same comparer (`EE/Internal/CollectionItemValue.cs:28-66`). The rows edit a **copy** of the item, and any change writes that copy back into the list at its index (`:36-44`, `:259-262`). After that the whole list reloads: every item rebuilt, its editors recreated (`EE/PropertyItemValue.cs:356-365`, `:92-116`). Per-item visibility: `RecordDesc::visibility` (`fw.rs:72-80`), and browsable state per sub-row.
*One undo step for each sub-row edit.* It covers the whole list (`EE/PropertyItem.cs:638-659`).
*The exception is the Custom State row:* `CustomStateItem` isn't `ICloneable` (`Vixen.Modules/Effect/State/CustomStateItem.cs:13`), so its rows edit the live item, not a copy (`EE/Internal/CollectionItemValue.cs:36-44`). The write-back then finds the same object in the list and returns before pushing anything (`EE/PropertyItem.cs:647-651`). So changing a Custom State row's State Item or Color has **no undo step** and doesn't flag the element from the host; the effect marks itself dirty through its child-changed handler (`Vixen.Modules/Effect/State/State.cs:1175-1180`). **FIX:** give it one undo step like any other row. Emitter, Waveform, Whirl, MorphPolygon and FixtureFunction all clone.
*Rust:* a sub-row is addressed as `(param, row_index, sub_param)`. Swift must keep view identity so that editing in a row doesn't lose focus (Performance hazards).
*Row setters:* a scan of the setter bodies of the row classes finds these side effects on other values, visibility aside. `Waveform.WaveType` and `UseMarks` select the first mark collection when a Decaying Sine follows marks (`Vixen.Modules/Effect/Wave/Wave/Waveform.cs:80`, `:104`, `:597-609`), and `Waveform.MovementType` sets `PrimeWave` for Snake (`:177`). `Emitter.FlowControl` selects a first mark collection (`Vixen.Modules/Effect/Liquid/Liquid/Emitter.cs:723`, `:912`). `Whirl.WhirlMode` forces `ColorMode` to `GradientOverTime` for Meteor (`Vixen.Modules/Effect/Whirlpool/Whirlpool/Whirl.cs:236-252`). `FixtureFunctionExpando.IndexValue` and `ColorIndexValue` fall back to the first entry for a value not in the list (`Vixen.Modules/Effect/Fixture/FixtureEffect/FixtureFunctionExpando.cs:211-255`, `:266-300`). F106's State Item sets the colour.
`ParamChange` names the list and not the row (`fw.rs:666-670`), so there's no row hook. The existing pattern is `param_changed_ctx` on the whole list, comparing old rows with new ones (`crates/fx-pixel-complex/src/wave.rs:50-66`). So `commit` on a sub-row writes the whole list through `EffectDescriptor::set_param` and never pokes the row. Wave and Emitter already do what they need that way (`crates/fx-pixel-physics/src/liquid/mod.rs:93-105`), and Fixture's fallback runs at load (`crates/fx-fixture/src/fixture.rs:129-133`), which is enough while the drop-down only offers listed values. **Not covered yet, and needed:** Whirl's row Meteor, because `crates/fx-pixel-complex/src/whirlpool/mod.rs:106-112` reacts only to the effect-level `WhirlMode`; and Custom State's colour (F106). Tests pin each one.

**F56. Add.**
*Does:* with at least one item, creates a new item of the type of the **first** item: an empty string for strings; otherwise the default-constructed value (`EE/PropertyItemValue.cs:148-175`). Empty list: only expando-backed record lists can add, using their declared type (`:97-104`, `:167-174`); any other empty list does nothing (KEEP). That's Emitter, Wave, Whirl, Fixture function and Custom State on one side; Morph, gradient, pair and string lists on the other (`Vixen.Modules/Effect/Effect/ExpandoObjectObservableCollection.cs:7`, `Vixen.Modules/Effect/Morph/Morph/MorphPolygonsObservableCollection.cs:8`). Appends at the end. Commits one undo step. Clicking Add clears the selected row (F58), because Add moves the focus (`EE/Themes/ColorPaletteEditor.xaml:49-51`, `EE/Controls/CollectionView.cs:40-47`).
*Multi-selection, all lists empty:* the merged read returns null when every list is equal and empty (`EE/Internal/MergedPropertyDescriptor.cs:252-253`), so the row shows no items and `AddCollectionValue` finds nothing to add to (`EE/PropertyItem.cs:682-683`). So Add does nothing, even on an expando list. A selection where First's list is empty and another's isn't does add, since that read returns First's empty list, not null. KEEP.
*Defaults:* a Color is `Color.Empty`, which is transparent, so the new swatch shows the row behind it (F36); a gradient is `new ColorGradient()`; both are what upstream's `Activator.CreateInstance` gives. KEEP. Gradient default: F94. No discrete check is made on an added item. A new Custom State row takes the first State item no other row uses, and that item's colour (`Vixen.Modules/Effect/State/State.cs:1146-1173`).

**F57. Remove.**
*Rules:* enabled only with a row selected and more items than the minimum (`EE/Controls/CollectionView.cs:22-38`, `EE/PropertyItemValue.cs:177-181`). The minimum is **1** by default, or the record list's own value (`:46-47`, `:103`, `Vixen.Modules/Effect/Effect/ExpandoObjectObservableCollection.cs:36-40`): fixture functions **0** (`Vixen.Modules/Effect/Fixture/FixtureEffect/FixtureFunctionExpandoCollection.cs:27-31`); custom State items **1 when the effect's render source is Custom, else 0** (`Vixen.Modules/Effect/State/CustomStateItemCollection.cs:49-52`). `min_items` must therefore be able to read the effect's values.
*Does:* removes the selected row, one undo step. The list reloads and the selection clears (`EE/Controls/CollectionView.cs:49-59`).

**F58. Selected row.** No `SelectionMode` is set, and `ListView`'s default is Extended (from knowledge of WPF), so ⌘-click and ⇧-click can select several rows in upstream, but nothing uses that: Remove and Up/Down act on `SelectedIndex` alone (`EE/Controls/CollectionView.cs:49-59`, `:103-114`). Mac: single selection. Losing focus clears it (`:40-47`), which is why Remove, Up and Down are their own focus scopes, keeping the selection (`EE/Themes/ColorPaletteEditor.xaml:53`, `:56`, `:60`). `LostFocus` bubbles, so an editor inside a row losing focus clears it too. Mac: keep the selection while clicking those buttons; clear it when focus goes elsewhere. Upstream's strips swallow their mouse presses (F49), so a gradient row is selected by clicking its margin. **FIX** proposal: a click on a strip that causes no edit also selects its row, so the row is easy to pick.

**F59. Up and Down.**
*Rules:* only on the three list kinds in F52; enabled with a row selected, the list editable and the target in range (`EE/Controls/CollectionView.cs:61-91`, `EE/PropertyItemValue.cs:198-205`).
*Does:* moves the row by one, keeps it selected, scrolls it into view, one undo step (`EE/Controls/CollectionView.cs:103-114`, `EE/PropertyItem.cs:715-754`).

### G. Mixed values and multi-selection

**F60. What a multi-selection shows.** Source `EE/Internal/MergedPropertyDescriptor.cs`, `EE/ObjectServices.cs:110-140`, `EE/MetadataRepository.cs:215-230`.
*Rows:* only properties common to all selected effects, same name and same type (`EE/MetadataRepository.cs:128-131`, `:152-156` compare descriptors, and a `PropertyDescriptor`'s equality is name and type) and mergeable, plus `EffectName` always (`:225`). No effect in the tables opts out of merging: `[MergableProperty]` appears nowhere under `Vixen.Modules/Effect`, so everything with matching name and type merges.
*Value shown:* First's value, when values differ too (`MergedPropertyDescriptor.cs:212-256`). There's no mixed indicator anywhere, with one exception:
*Blank when mixed:* enum rows and `TargetElementDepthConverter` rows show nothing selected, "so the user must choose" (`:239-249`).
*Collections:* First's list is shown; items are compared in order with `Equals`; a difference only sets the unused `allEqual` flag (`:216-238`). So a mixed list looks like First's list. If all the lists are equal and empty the read is null (`:252-253`, F56).
*Read-only / browsable:* read-only if any effect's is; browsable if all effects' are (`:389-427`). The converter comes from First (`:191-194`, `:359-362`), and what it lists depends on the converter: F105.

**F61. Merged metadata.** Attributes are the intersection across the selection, by attribute equality (`MergedPropertyDescriptor.cs:177-189`):
- *Display name:* First's (`:364-367`).
- *Description:* shared text, else none, so the tooltip falls back to the display name (F17).
- *Category:* equality compares only the name, and the result keeps First's. So same name, different order: First's order. Different names: **`Misc`**, order -1, which sorts first with any other -1 categories (`EE/CategoryItem.cs:109`).
- *Row order:* `[PropertyOrder]` equality includes the value. Different orders: none, so 0 (`EE/PropertyEditing/PropertyItemComparer.cs:47-57`).
- *Editor:* `[PropertyEditor]` equality compares the text; a difference falls to the type's default editor (F21). `[NumberRange]` differing: no range, so the fallbacks: sliders 0..100 tick 1 and numeric fields minimum 0.
*Rust:* `panel_model` implements exactly this from the `ParamDesc`s, comparing `kind`, `category`, `category_order`, `order`, `description`, `editor`. See Rust API. `Editor` holds C#'s `[PropertyEditor]` and `[NumberRange]` together, so compare them apart: same variant with a different range stays that variant with the fallback range (a slider stays a slider at 0..100 tick 1); a different variant is the type's default editor. Each effect's own `display` applies to its own value (F33).
*Mac:* same rows and values. A mixed-state check box or a dash would be friendlier, but it changes what the first click does (F62), so it isn't done without the owner's yes.

**F62. Setting a mixed value.**
*Does:* a new value is compared with **First's** current value; if equal, **nothing happens to any effect** (`EE/PropertyItem.cs:778-783`). So to give a mixed selection First's value you must set another value first. KEEP. Otherwise each effect gets its own copy (`MergedPropertyDescriptor.cs:317-340`, `:127-175`), through the effect's own setter, so side effects run per effect.
*Enum and depth rows from blank:* the merged read gives null when mixed, so no chosen value equals it, and any choice applies to all, First's value included (`EE/PropertyItem.cs:778-783`, `MergedPropertyDescriptor.cs:244-247`).
*Collections:* any edit (item, add, remove, move) acts on First's list and then copies **First's whole list into every selected effect** (`MergedPropertyDescriptor.cs:281-315`). Different lists are overwritten without warning. KEEP. Each effect is marked dirty (`:301-305`). Upstream shares item objects between effects; Rust's values are copies, which has the same visible result except that upstream's shared objects then change together, which nothing relies on.
*Curves and gradients:* a strip shows First's value and a release writes the result to all. The dialog opens on First's.
*Library references:* copied along with the value, so a link stays a link.

**F63. Undo.**
*Do:* ⌘Z with focus anywhere in the sequencer window. Edit menu names the action.
*Does:* each value change raises `PropertyValueChanged`, and the host makes **one** `EffectsPropertyModifiedUndoAction` covering every selected effect (`FormEffectEditor.cs:145-163`). It holds, per effect: the old value (per-effect, cloned lists for a collection) and the property's descriptor and owner (`EE/PropertyItem.cs:661-676`). Undo and Redo swap old and current through the property setter, then flag the element's content changed (`TSE/Undo/EffectsPropertyModifiedUndoAction.cs:18-45`). Description: **`{count} Effect(s) {DisplayName} modified`**, with the display name of the first (`:47-50`, `:12-13`).
*Grouping:* there is none. One commit is one step, whatever the gesture: F24's timer commits, F32's 300 ms-idle commits, F55's sub-row writes. Upstream's only coalescing is those delays. The undo stack belongs to the sequencer (`docs/spec/sequencer-shell.md` F375 and its undo entries).
*Empty step:* when no old value exists, upstream still pushes an empty action reading `0 Effect(s)  modified`. FIX: skip it (sequencer-shell F375).
*Setter side effects:* upstream's undo goes through the setter again with the old value, so a setter that changes other properties runs again and doesn't restore them unless it happens to.
*After undo or redo:* the setter's change event reaches the row through the same path as an edit, so the panel updates in place if the effect is still selected (`EE/PropertyItem.cs:248-252`). Mac: re-read through `panel_delta`, or `panel_model` if the selection has changed since. The element is flagged as in F64.
*Rust:* `UndoAction::ParamsChanged` in `crates/vixen-session/src/undo.rs:93-95` holds whole `Params` per effect and already words the description. Restoring the whole snapshot also undoes a setter's side effects; that is a small deliberate improvement. One action per user action per `docs/UI_GUIDE.md`.
*Mac:* one undo step per gesture for sliders, field-strip drags and the colour panel (F32, FIX). The step count is lower than upstream's for a slow drag; that is the only intended difference.

**F64. After a commit.** For each selected element, content is flagged changed (`FormEffectEditor.cs:152`), so the timeline re-renders that effect's thumbnail and the effect goes dirty. The panel re-reads values, visibility and choices (F18, F19). Text box focus: F66.

**F65. Selection of targets or marks changes while showing.** A `TargetNodes` change refreshes all choice lists (F19), and discrete state follows on the next component change (F51). Mark collection changes re-validate selectors and refresh (`Vixen.Modules/Effect/Effect/BaseEffect.cs:66-109`, `fw.rs:799-803`).

**F66. Keyboard and focus.**
*Does:* moving the focus out of the panel commits the focused text box first (`FormEffectEditor.cs:180-200`), so a click on another effect commits to the old selection, 200 ms before the panel switches (F2). Keys pressed in the panel, unless in a text box or handled by the focused editor (for example Delete on a gradient handle, arrows on a slider, Return on a strip), are forwarded to the timeline's shortcuts (`:217-227`; the handler isn't registered for handled events, `:53`). Entering the panel gives its content focus (`:175-178`). Tab moves between editors in the usual way; the grid's own Tab code is commented out (`EE/EffectPropertyEditorGrid.cs:728-743`).
*Description tracking:* focus also sets a `CurrentDescription` property that nothing displays (`:1193-1202`). Omit.

### H. Drag and drop in the panel

Sources: `WC/Input/DragDropManager.cs`, `WC/Input/DragDropUtils.cs`, `EE/PropertyItemValue.cs:235-354`, `EE/Internal/CollectionItemValue.cs:286-447`.

**F67. Dragging a value out.**
*Do:* press on a colour swatch or on a curve or gradient strip (not where F39's rules capture) and drag past the system drag distance.
*Does:* a copy drag carrying the value. Sources: any Curve, Color or ColorGradient row with a non-null value (`EE/PropertyItemValue.cs:342-349`), and items of those types in lists; a pair's strip carries its own half (`EE/Internal/CollectionItemValue.cs:420-447`). A double-click never starts a drag (`WC/Input/DragDropManager.cs:231`). Links travel with the value (F69).
*Mac:* `NSDraggingSource` with a private pasteboard type per kind (`com.vixen.curve`, `.gradient`, `.color`), plus the standard `NSColor` type on colours so that they can drop on system colour wells.

**F68. Dropping a value in.**
*Does:* the operation is always Copy; the target highlights nothing, upstream's adorner shows no visual (`EE/PropertyItemValue.cs:313-316`, `:321`).

| Target row | Curve | Gradient | Color |
|---|---|---|---|
| Curve | sets | - | - |
| Gradient | - | sets | `new ColorGradient(c)` |
| Color | - | accepted, then **nothing** (see FIX) | sets |
| Gradient list item | - | sets item | solid gradient in item |
| Color list item | - | accepted, nothing | sets item |
| Pair, curve half | sets curve | - | - |
| Pair, gradient half | - | sets gradient | solid gradient in gradient half |

Sources: `EE/PropertyItemValue.cs:235-311`, `EE/Internal/CollectionItemValue.cs:286-400`.
*Discrete rule:* when the owner's discrete colours aren't empty, a Color must be in the set; a gradient's colours must all be in it; otherwise the drop is refused (`EE/PropertyItemValue.cs:240-265`, `EE/Internal/CollectionItemValue.cs:290-314`).
*Commit:* one undo step, as usual. A linked curve or gradient row accepts drops too.
*FIX proposal:* refuse the "accepted, nothing" drops, so the cursor tells the truth.

**F69. Drags from the libraries and toolbars.**
*Do:* drag from the Curve or Gradient Library window, from a toolbar strip, or from the Color Library, onto a row or an effect.
*Does:* the payload is a copy, marked not current, carrying the entry's name **only when linking was asked for** (`TSE/LibraryDragPayloadFactory.cs:21-59`):

| Source | Link when | Source lines |
|---|---|---|
| Curve Library window | **Maintain Library Link** checked | `Form_CurveLibrary.cs:346-357` |
| Gradient Library window | **Maintain Library Link** checked | `Form_GradientLibrary.cs:343-354` |
| Toolbar strip | ⌘ held at drag start | `TSE/TimedSequenceEditorForm_Toolstrip.cs:665-666`, sequencer-shell F168 |
| Color Library, colour strip | Colours have no link | `Form_ColorLibrary.cs:463-471` |

A row-to-row drag keeps whatever the value had, link included (`EE/PropertyItemValue.cs:332-335`). A dropped link shows the badge, blocks inline editing (F39) and opens the dialog in its Linked mode. Dropped on several selected effects, each gets its own copy (F62). Drops on the timeline's effects go through `TSE/TimedSequenceEditorForm.cs:860-1050`, specified in sequencer-shell, with the same discrete checks.
*Rust:* `DragPayload` carries the value serialised by Rust; `can_drop` and `drop_value` apply F68.

### I. Curve Editor dialog

Sources: `Vixen.Modules/App/Curves/CurveEditor.cs` (`CE:` below), `CurveEditor.Designer.cs`, `docs/reference/forms/Vixen.Modules__App__Curves__CurveEditor.json`. Editing is built on ZedGraph; only the behaviour it gives is specified. **Mac: a modal sheet with a custom AppKit graph view; ZedGraph is not ported.**

**F70. Opening.**
*Do:* double-click or Return on a curve strip (F43, F53), Edit from the library windows (F86), or Edit Library Curve (F78).
*Does:* a fixed-size modal dialog, minimum 591x784, centred on its parent, OK default and Cancel cancel (`CurveEditor.json`). Title: the row's display name when opened from a row (`EE/Editors/CurveEditor.cs:17`); Load or Unlink then sets F71's title. Edits a **copy** of the curve (`CE:68-76`). OK returns the copy; Cancel leaves everything (`EE/Editors/CurveEditor.cs:18-21`). A null value starts from the default ramp (`:16`). OK with nothing changed commits nothing: curve equality compares point values, the link name and the current flag (`Vixen.Modules/App/Curves/Curve.cs:369-372`, `ZedGraph/PointPairList.cs:441-453`, F62).
*Region layout, top to bottom:* graph (549x482), **Curve** group, **Library** group, then instruction labels at left, OK and Cancel at right.
*Startup guard:* mouse input to the graph is ignored for the first **300 ms** after the dialog shows, so the release of the double-click that opened it can't add or move a point (`CE:108-112`, `:659-688`). KEEP, Mac sheet too.

**F71. Three modes, from the curve's state** (`CE:362-445`).

| State | Label | Title | Graph | Save Curve | Load Curve | Unlink | Edit Library Curve | Invert, Reverse | X, Y fields | Instructions |
|---|---|---|---|---|---|---|---|---|---|---|
| This *is* the library's copy (`IsCurrentLibraryCurve`), name known | "This curve is the library curve: {name}" | "Curve Editor: Library Curve {name}" | editable, blue | off | off | off | off | on | off, empty | shown |
| Same, name unknown | "This curve is a library curve." | "Curve Editor: Library Curve" | as above | off | off | off | off | on | off, empty | shown |
| Linked to the library | "This curve is linked to the library curve: {name}" | "Curve Editor" | read-only, grey | off | on | **on** | **on** | off | off, empty | hidden |
| Not linked | "This curve is not linked to any in the library." | "Curve Editor" | editable, blue | on | on | off | off | on | **on**, empty | shown |

Update is off in every mode until a point is picked (F73). Blue is `(0,128,255)`, grey `Color.DarkGray` (`Curve.cs:17-18`). Draw Curve and f(x) Curve aren't disabled in the linked mode; see the quirk under F76.

**F72. The graph.**
*Does:* X and Y fixed 0..100, major grid dashed (2 on, 4 off) in the group-border colour, axis numbers, no titles, no legend, no context menu, no zoom, pan or selection (`CE:29-55`). Line with point symbols. Dialog background on the theme background.
*Move a point:* press within **7 pt** (screen) of a point, with no modifier held, and drag (`EditModifierKeys = None`, `CE:39`; `ZedGraph/GraphPane.cs:193`, `ZedGraph/ZedGraphControl.Events.cs:491-507`). Delta-based: the point moves by the mouse's delta in data units (`ZedGraphControl.Events.cs:1107-1130`), clamped to 0..100 after each move (`CE:254-261`), and the list is **re-sorted by X on each move** so it can't pass a neighbour without reordering (`CE:170-174`). No rounding; X and Y fields show `0.####` (`CE:265-267`, `:333-335`). Escape during the drag restores the point (`ZedGraphControl.Events.cs:556-573`, `:1137-1145`). Cursor: crosshair within range of a point (`:667-691`).
*Add:* holding ⌘, press away from any point inside the chart: inserts at the clamped press location and sorts (`CE:289-316`).
*Remove:* holding ⌥, press near a point: removes it if more than 2 remain (`CE:318-326`).
*Move whole curve:* drag on empty graph with no modifier: every point's Y moves by the change in pointer Y, stopped at 100 or 0 once any point reaches it in that direction, with a correction for overshoot (`CE:177-239`). X is unchanged.
*Flatten:* holding ⇧, drag: the curve becomes `(0,y),(100,y)` following the pointer, clamped; Y field shows it, X field empty (`CE:180-199`).
*Read-only (linked):* press and edit are ignored (`CE:280-281`, `:424-425`), but **the move-whole-curve and flatten branches don't check**, see the quirk below.
*Quirk, linked curve:* a left drag runs the Y-move or flatten code on the displayed points regardless of `ReadonlyCurve` (`CE:177-239`). FIX: refuse them when linked. It is the only place a linked curve's points could change in this dialog. The graph edits the copy's own point list (`CE:413`), and a linked curve re-reads the library only when its entry stops being current or on a fresh load (`Curve.cs:131-170`). So upstream's OK commits the changed points, still linked, and they stay until the library entry changes or the sequence reloads.

**F73. Selected Point: X, Y, Update.**
*Does:* pressing or dragging a point fills **X** and **Y** with `0.####` and enables them and **Update** (`CE:263-268`, `:331-336`). The fields accept only decimal numbers in 0..100 (`CurveEditor.json`: `NumericTextBox`, `MinValue 0`, `MaxValue 100`, 15 decimals, no group separator): characters other than digits and one decimal separator are stripped as typed, and a keystroke that would pass 100 is undone at once (`Vixen.Common/Controls/NumericTextBox.cs:189-210`). **Update** writes whichever field parses onto the picked point and redraws, so an emptied field leaves that coordinate alone (`CE:554-566`). It doesn't re-sort (KEEP). Return in a field presses OK and closes the dialog, because OK is the form's accept button (`CurveEditor.json`), so the way to apply is the Update button. KEEP. After points are replaced by F75 or F76, or flattened, the picked point is stale; GUARD: disable Update.

**F74. Reverse Curve, Invert Curve.** Top and bottom of the Curve group's first column (`CurveEditor.json`). Reverse: `X = 100 - X`, sorted. Invert: `Y = 100 - Y` (`CE:521-552`). Both act on the dialog's copy.

**F75. f(x) Curve.**
*Do:* click **f(x) Curve** to open the **Function Curve Generator** (`FunctionGenerator.cs`, `Vixen.Modules__App__Curves__FunctionGenerator.json`), a fixed modal with: read-only description "Enter the mathmatical f(x) that provides the value of y. The function will be run with values of x from 0-100 incremented by the Draw Interval to get the x,y values in the curve.  Example gamma function where 2.2 is the gamma and 80 is the max level: Pow(x/100, 2.2) * 80."; **Help** link to `http://www.vixenlights.com/vixen-3-documentation/basic-concepts-of-vixen-3/curve-editor/`; **Function** text field; **Generate** (default) and **Cancel**. Starts with the last function used in this Curve Editor (`CE:18`, `:585`, `:601`).
*Does:* Cancel: nothing (`CE:586-590`). Generate: **clears the graph first** (`CE:591-592`). Empty: error **"The function entered is empty."**, titled **"Function Error"**, with the graph now empty (`:594-599`). Parsed with NCalc, ignoring function-name case (`:604`); parameters `x` and `Pi` and grammar keywords remain case-sensitive (confirmed with the pinned NCalc2 2.1.0 oracle). Syntax errors: **"The function entered has the following syntax errors.\n {error}"**, **"Function Error"** (`:605-610`). Then `step` = Draw Interval, for `x = 0, step, 2*step...` to 100, then at 100 if that wasn't hit, adds `(x, y)`, with `y = Convert.ToDouble(result)` clamped to 0..100, `x` and `Pi` as parameters (`:612-623`, `:633-642`). Exceptions: same text with the exception message, titled **"Function Errors"** (`:626-630`). Points appended before a later exception remain in the graph.
*Quirks:* a Draw Interval of `""` or 0 throws; with `""` it's the catch above, and with **0 the loop never ends**. GUARD: treat 0 or empty as an error with the same dialog.
*Rust:* the expression evaluator is logic: `evaluate_curve_function(text, step) -> Result<points, message>`. NCalc functions to support, case-insensitive: Abs, Acos, Asin, Atan, Ceiling, Cos, Exp, Floor, IEEERemainder, Log, Log10, Max, Min, Pow, Round, Sign, Sin, Sqrt, Tan, Truncate, `if`, `in`, the usual operators, and the parameters `x` and `Pi`. Int32/Int32 division returns a real number, which the example `x/100` relies on; Int64 division retains integer truncation. `Abs` returns Decimal; numeric types affect operator compatibility and promotion. These rules and the listed functions are confirmed against the original handlers and pinned NCalc2 2.1.0 in the 4,282-case oracle (`tools/golden/ncalc/README.md`), superseding the original memory-based specification. Nothing is stored beyond the last text, per dialog instance.

**F76. Draw Curve.**
*Do:* click **Draw Curve**, then press and drag left to right across the graph.
*Does:* shows a 4 s balloon, titled **"Draw Curve"**: "Draw curve from left side to right side of grid using the left mouse button" (`CE:573-581`). Clears the curve, drawing mode on. While the button is held and moving: clamped point; if the list is empty and `tempX < x - 1`, adds `(0, y)` first, with `x` and `y` before the clamp (`CE:137-140`). Adds `(x, y)` whenever `x` passed the last X by more than **Draw Interval**, sorted (`:159-166`). On release: adds `(100, y)`, sorted, drawing mode off (`:348-357`). Y at release isn't clamped (`:351-353`): clamp is a GUARD, on the first point's Y too. A click with no movement leaves the single point `(100, y)`, which draws nothing and reads flat at that level. KEEP.
*Draw Interval:* text box, digits and control keys only, 2 characters, centred, default **5** (`CE:27`, `:644-647`, `CurveEditor.json`), over 20 resets to **20** with the 3 s balloon "Draw Threshold has a maximun value of 20" (`CE:649-657`). Label tooltip "Min distance to create Curve point". Empty counts as 0 when drawing (`CE:131`).
*Quirk:* Draw Curve and f(x) Curve stay enabled for a linked curve (`CE:411-440` doesn't touch them), and they clear the graph's displayed points. As in F72, upstream's OK then commits those points, still linked, until the library entry changes or the sequence reloads. FIX: disable them when linked, with the rest of the Curve group.

**F77. Load Curve.** Opens the **Curve Library** selector (F79). OK with a selection: the dialog's curve becomes a copy of the chosen entry, linked to its name (`CE:447-457`). The graph updates to the Linked row in F71.

**F78. Library group.**
- **Save Curve** (not linked): prompts **"Curve name?"** via a text dialog, looping (`CE:459-497`). Empty: **"Please enter a name."**, titled **"Curve Name"**, loop again. Exists: **"There is already a curve with that name. Do you want to overwrite it?"**, **"Overwrite curve?"**, with Yes, No and Cancel: Yes writes and stops, No asks for a name again, Cancel stops. New: writes and stops. It stores an independent copy and **does not link** the dialog's curve (`CE:480`, `:488`). Mac: a name sheet, `.confirmationDialog` with "Overwrite", "Choose Another Name", "Cancel".
- **Unlink Curve** (linked): the dialog's curve keeps its points and loses its link; updates to the Not Linked row (`CE:499-503`, `Curve.cs:204-208`).
- **Edit Library Curve** (linked): opens F70 on the library entry; OK replaces the library entry, then this dialog's points refresh from it (`CE:505-519`, `Vixen.Modules/App/Curves/CurveLibrary.cs:107-123`).
*Instructions:* "Hold down Ctrl to add new points on the grid." and "Hold down Alt to remove a point." at bottom left; Mac wording: "Hold down ⌘ to add new points on the grid." and "Hold down ⌥ to remove a point."

### J. Curve library: selector and window

**F79. Curve Library selector.**
*Do:* Load Curve (F77).
*Does:* modal, titled **Curve Library**, minimum 572x472, list of 68x68 thumbnails with names, default icon view, with New Curve, Edit Curve, Delete Curve at bottom left and OK, Cancel at bottom right (`CurveLibrarySelector.json`). Thumbnails: `GenerateGenericCurveImage(68,68)`, black background, grey line, 2 pt `(136,136,136)` border (`CurveLibrarySelector.cs:51-80`).
*Rules:* Edit and Delete enabled with exactly one selected; off after every refill (`:76-88`). The list allows several selected (the designer's default); OK then takes the first selected (`:90-99`). OK or Return with nothing selected closes the selector and the caller changes nothing (`CE:450`).
*Double-click* one item: OK; `Mode.Edit` exists but nothing uses it here, and the code then **also** presses Edit (`CurveLibrarySelector.cs:190-202`): with `DialogResult` set the dialog closes after Edit returns. Mac: double-click accepts. Return: OK. Escape: Cancel (`:217-223`).
*New Curve:* same name loop as F78 (Yes, No, Cancel), adds a default ramp, then opens F70 on it, refills (`:101-149`). Cancel in that editor leaves the new default ramp in the library (KEEP), and on an overwrite the old curve was replaced by the ramp before the editor opened, so Cancel loses it (KEEP).
*Delete Curve:* **"If you delete this library curve, ALL places it is used will be unlinked and will become independent curves. Are you sure you want to continue?"**, **"Delete library curve?"**, Yes/No (`:168-188`).
*Persist:* window bounds in the library's static data (`:29-49`). Mac: `NSWindow` frame autosave, or nothing for a sheet.

**F80. Curve Library window: layout.**
*Do:* View > Curve Library Window (sequencer-shell F75).
*Does:* a right-side tab titled **Curve Library** (`Form_CurveLibrary.json`). Top toolbar, in order: **Edit Curve**, **New Curve**, **Delete Curve**, **Export** (tooltip "Export Curve Library"), **Import** ("Import Curve Library"); icons (`Form_CurveLibrary.cs:87-96`). Centre: a thumbnail list. Bottom: check box **Maintain Library Link**.
*Thumbnails:* `48 * scale`, 2 pt border in the theme colour, name below (`:124-133`, `:172-207`). Order is the library's order.
*Rules:* Delete enabled with any selected, Edit with exactly one; both off after a refill (`:206`, `:318-322`). Keys are forwarded to the timeline (`:167-170`). Entering the check box or the toolbar puts the focus back on the list, so those keys keep working (`:115-118`, `:157-160`); clicking the toolbar's buttons and the check box still works. Mac: make the list the key view when the panel is entered.
*Default:* `Maintain Library Link`: **see F83**.

**F81. Curve Library window: New, Edit, Delete.**
- *New Curve:* name loop, then adds a ramp and opens the editor on it, F86 (`Form_CurveLibrary.cs:229-289`). Same messages as F78, Cancel stops. "Please enter a name." titled "Curve Name".
- *Edit Curve* or double-click one: `EditLibraryCurveAsync` (`Form_CurveLibrary.cs:213-227`, `:324-328`).
- *Delete Curve:* **"If you delete the Selected library curve(s), ALL places it is used will be unlinked and will become independent curves. Are you sure you want to continue?"**, **"Delete library curve?"**, Yes, No and Cancel (`Vixen.Common/Controls/MessageBoxForm.cs:11-23`). Yes removes all selected in one bulk update, one save (`Form_CurveLibrary.cs:291-316`).

**F82. Curve Library window: reorder and add by drop.**
*Do:* drag items within the list; or drop a curve from elsewhere (an effect's curve row, the toolbar).
*Does:* a drop no more than 10 pt away from where the drag entered, in both axes, is ignored (`Form_CurveLibrary.cs:393`). *Reorder:* moves all selected to the target item's index; reversed when moving left; no target: to the end (`:416-430`). *Drop a curve:* accepted only if not linked (`:372-381`); prompts for a name with the F78 loop without opening the editor (`edit=false`, `:241`, `:403`); Cancel aborts (`:403`). New item is placed at the target item's index, or last (`:405-414`). After either: the library dictionary is **cleared and rewritten in list order** (`:432-435`), one save.
*Quirks:* naming a dropped curve over an existing name overwrites that entry in place, and the code then moves whichever item is last to the target index, not the overwritten one (`:405-414`). FIX: move the entry that was just written. And any drag carrying a list item counts as a reorder in the entry test (`:366-370`), so a Gradient window item dropped on the Curve window would reorder the Curve list's selection, or throw and log with none selected. GUARD: only this list's own drag reorders.
*Undo:* none, for all library operations.
*Rust:* order is significant in the store: `Vec<(String, Curve)>`, or an ordered map. `CurveLibrary` in `crates/vixen-io/src/library.rs:30-35` must keep file order on write.

**F83. Dragging out: Maintain Library Link.**
*Do:* drag an item out of the list.
*Does:* the payload is a copy of the first selected curve, with `IsCurrentLibraryCurve = false`, and `LibraryReferenceName` = the item's name **if the box is checked**, else empty (`Form_CurveLibrary.cs:346-357`, `TSE/LibraryDragPayloadFactory.cs:21-59`). Copy and move allowed (`Form_CurveLibrary.cs:356`). The target takes it (F68, timeline drop in sequencer-shell).
*Default:* unchecked. The sequencer restores and saves the state, as `TimedSequenceEditorForm/CurveLinkCurves` (`TSE/TimedSequenceEditorForm.cs:364`, `:5827`). The toolbar strip uses ⌘ held at drag start for the same choice (`TSE/TimedSequenceEditorForm_Toolstrip.cs:665-666`).

**F84. Export and Import.**
- *Export* (`Form_CurveLibrary.cs:450-487`): save panel, `Vixen 3 Curve Library (*.vcl)`, `All Files`, starting in the last folder of this window. Writes indented XML: a `DataContractSerializer` of `Dictionary<string, Curve>`, in library order. Failure: **"Unable to export data, please check the error log for details"**, **"Unable to export"**.
- *Import* (`:489-541`): open panel, same filter. Each entry is added; a name already used gets `" 2"`, `" 3"`... appended, no prompt (`:515-521`). One bulk update. Failure: **"Sorry, we didn't reconize the data in that file as valid Curve Library data."** (sic), **"Invalid file"**. Adding goes through F86's rules, so an imported entry becomes current and any link it carried is cleared (`CurveLibrary.cs:63-65`).
*Toolstrip:* the sequencer's Curve toolbar also calls these (sequencer-shell).

**F85. Thumbnail scale.**
*Do:* ⌘-scroll over the window: thumbnail size; hold ⇧ while the list has focus and ⌘-scroll: label size. Mac: also pinch.
*Does:* each wheel step multiplies or divides by 1.03; image minimum 0.1, text minimum 0.2 (`Form_CurveLibrary.cs:578-616`, `:618-629`). Text font is `(int)(7 * textScale)`, image `(int)Math.Round(48 * imageScale)` (`:126-128`).
*Persist:* `{Name}/CurveLibraryImageScale`, `{Name}/CurveLibraryTextScale`, invariant culture, default `1`, saved on close (`:100-107`, `:572-574`). Loaded values are floored at the same minimums.

**F86. Editing a library curve.**
*Does:* `EditLibraryCurveAsync`: opens F70 with `LibraryCurveName` set, so title and label are the library ones. OK: removes the old entry, adds the result under the same name (`CurveLibrary.cs:107-123`). `AddCurveAsync` marks the new entry current, clears any link on it, and flags the old entry `IsCurrentLibraryCurve = false` so every linked curve re-reads the points on next use (`:54-77`, `Curve.cs:131-170`). Raises `CurvesChanged`, saves module config. The sequencer answers `CurvesChanged` by re-rendering dirty effects (`TSE/TimedSequenceEditorForm.cs:2133-2136`).
*Order:* the entry keeps its place, by way of .NET's `Dictionary`: an add right after a remove reuses the freed slot (from knowledge of .NET). So `replace_curve` replaces in place. By the same rule a new curve added after a delete lands in the deleted one's slot rather than last, until the next reorder rewrites the list. The Mac appends new entries at the end; nobody can rely on that accident.
*Linked curves:* a linked curve always reads through the library: **its points are the library entry's, whatever was stored** (`Curve.cs:73-81`). If the entry has gone, the curve silently unlinks and keeps its last points (`:164`). This applies at render, in the strip, and in every dialog.
*Rust gap:* `butterfly-3` in `docs/audit/editor-pass.md` says render uses the stored copy. A library edit must **push the new points into every linked curve in open sequences** and mark them dirty, or render must resolve names. See Rust API.

### K. Color Gradient Editor dialog

Sources: `Vixen.Modules/App/ColorGradients/ColorGradientEditor.cs` (`CGE:`), `GradientEditPanel.cs` (`GEP:`), `GradientEdit.cs` (`GE:`), `ColorGradient.cs`, forms `ColorGradientEditor.json`, `GradientEditPanel.json`.

**F87. Opening.**
*Do:* double-click or Return on a gradient strip (F50), Edit from the library (F97), or Edit Library Gradient (F93).
*Does:* fixed-size modal, minimum 522x352, titled by state (F92), then for a row the row's display name (`EE/Editors/GradientTypeEditor.cs:29`). Edits a **deep copy** (`CGE:30-38`); OK returns it (`EE/Editors/GradientTypeEditor.cs:30-33`), and an unchanged result commits nothing (F94, F62). Return key: OK; Escape: Cancel (`CGE:193-199`). The form previews keys, so Return closes the dialog from anywhere in it, also while typing in Location (`ColorGradientEditor.json`). KEEP. Null value: a gradient of the first discrete colour, or white (`EE/Editors/GradientTypeEditor.cs:19-26`). Passes the row's discrete state and colours (`:18`, `:28`).
*Regions:* the gradient edit panel on top; the **Library** group below; OK and Cancel bottom right (`ColorGradientEditor.json`).

**F88. Gradient bar and stops.**
*Do:* click below the bar to add. Drag a fader. Click the small diamond between stops. Double-click a fader. Delete.
*Does:* `_border` = 10 pt. Gradient area `(10, 10, W-21, H-31)` on a checkerboard of Silver and White, border Silver (`GE:33`, `:299-311`). Position from pointer: `(x-10)/max(1, W-21)`, clamped 0..1 (`GE:41-51`). Each distinct position has one **fader** below the bar: polygon 20 wide, 20 tall, pointing up, its inset colour swatch showing the stop's colour, or a horizontal blend of all its colours when several share the position (`GE:82-89`, `:111-116`, `:131-170`). Selected fader fills Highlight (`:137`, `:326-327`). Alpha faders aren't drawn or hit-tested ("we don't want to use them (yet)") (`GE:237-240`, `:330-340`).
- *Select:* press a fader. All stops at that position become the selection (`GE:205-236`). Where faders overlap, the current selection wins, then its diamond, then the leftmost fader under the pointer (`GE:211-235`). A press that hits nothing clears the selection (`GE:354`).
- *Add:* press below the bar's area (`y > H - 10`), away from any fader (`GE:356-392`): the cursor there is a hand (`:436-446`). The bar ends at `H - 21`, so the upper half of the fader lane does nothing but clear the selection. Not discrete: one stop at the pointer with the gradient's colour there. Discrete: copies of the stops at the **nearest position at or before the pointer**, that is `targetPos` = the last sorted position below `pos`, or the first stop's if none; each with that same position, so a new fader isn't created at the pointer but on top of the existing one until dragged. The new stops are selected, so the same press drags them (`GE:366-391`).
- *Move:* drag: every selected stop's position = pointer minus grab offset, on each move and release (`GE:250-284`, `:425`, `:463`). Pointer outside the control: ignored, not removed (`:261-273`).
- *Release always writes:* `OnMouseUp` runs the same update whatever the button and whether or not anything moved (`GE:461-465`). So just clicking a fader snaps its position to the pixel grid, `(int)(pos * (W-21)) / (W-21)`, which changes the value and makes OK commit; and a right-click anywhere in the control moves the selected stops to the pointer. **FIX:** write only after a left-button drag has moved. The diamond's release has the same two effects, on its focus.
- *Focus diamond:* the selected fader's previous-gap midpoint marker, a 10x10 diamond at `previous.pos + focus * (this.pos - previous.pos)`; hidden for the first stop (`GE:91-106`, `:343-345`, `ColorGradient.cs:1045-1056`). Press and drag sets `focus` = `(pointer - previous.pos) / (this.pos - previous.pos)`, clamped, and .5 if the gap is 0 (`GE:213-218`, `:255-260`, `ColorGradient.cs:1070-1088`). Selected diamond fills Highlight. Focus shifts the colour midpoint between two stops: see the blend in `crates/vixen-core/src/gradient.rs`.
- *Cursor:* resize-horizontal over a fader or diamond, hand below the bar, arrow otherwise (`GE:432-447`).
- *Read-only (linked):* no select, no add, no cursor change (`GE:352`, `:426`). Faders still draw.
- *Double-click a fader:* F90; needs a fader under the pointer (`GE:406-419`).
Every change redraws immediately (`GE:468-478`).
*Rust:* `sorted_colors` exists; add focus get and set (Rust API).

**F89. Selected Color Point row** (`GEP:`, `GradientEditPanel.designer.cs`).
*Does:* below the bar, a group titled **Selected Stop** with one flow row: **Color:**, a colour swatch (`ColorLabel`), **Location:**, a position value control (`ValueUpDown`), **Delete** (`GradientEditPanel.resx`).
- *Swatch:* selection's first colour; `DimGray` when none or when a diamond is selected (`GEP:41-60`). It's split in two, the current colour above the previous one, each with its `RRGGBB` text (`Vixen.Common/Controls/ColorPicker/ColorLabel.cs:56-73`). Click opens F90 (`GEP:84-90`). Disabled when a diamond is selected or nothing is. Pressing it and dragging out samples a screen pixel (`Vixen.Common/Controls/ColorPicker/ColorLabel.cs:82-117`); release sets the stop's colour if one stop is selected (`GEP:173-185`). Mac: the colour panel's eyedropper replaces this.
- *Location* `vColorLoc`: 0..100, whole (`ValueControl` defaults, `Vixen.Common/Controls/ControlsEx/ValueControls/ValueControl.cs:14`), with a text field and a pop-up vertical tracker; shows `(int)(pos * 100)`, or the focus when a diamond is selected (`GEP:45-46`). Changing it sets `value / 100` on all selected stops' position, or their focus (`GEP:188-201`). Enabled with a selection and not read-only (`:42`, `:58`, `:279`).
  The control (`Vixen.Common/Controls/ControlsEx/ValueControls/ValueUpDown.cs`): the field takes digits and control keys only, and applies on Return or losing focus, clamped to the range, with unparseable text reverting (`:411-430`, `:539-546`). Beside it, spin up and down buttons step by 1 and repeat while held, first after 400 ms, then with the interval halving down to 10 ms (`:498-535`). A third button drops the tracker under the pointer, with the press already dragging its thumb, and each value applies as it moves (`:81-100`, `:559-563`). A thin bar along the bottom shows the value's proportion (`:331`, `:372`). Setting it from the program doesn't raise a change (`Vixen.Common/Controls/ControlsEx/ValueControls/ValueControl.cs:124-128`), so selecting a stop doesn't write back. Mac: `NSTextField` with `NSStepper`, the tracker as a small popover with a vertical slider, and no bottom bar, since the bar above already shows it.
  At opening, Location and Delete are enabled with nothing selected and do nothing, because the `ReadOnly = false` setter runs after the first `UpdateUI` (`CGE:77`, `:92`, `:106`; `GEP:278-279`). Mac: disabled, per the rule above.
- *Delete:* removes every selected stop (`GEP:204-218`); also Delete key (`:291-295`). Disabled with a diamond, or nothing selected, or read-only (`:43`, `:58`, `:278`). Doesn't guard the last stop (F94).
*Mac:* the flow row becomes one line, same order.

**F90. Editing a stop's colour.**
*Does:* not read-only, no diamond (`GEP:76-77`, `:95-96`).
- *Not discrete:* with more than one stop selected, the box **"Non-discrete color gradient, >1 selected point. oops! please report it."**, titled **"Delete library gradient?"**, then edits the first (`:149-155`). Colour picker with panel-level remembered mode and fader (initially `HSV_RGB`, `HSV_H`), V-lock off (`:17-18`, `:27`, `:159-168`). Mac: colour panel.
- *Discrete:* F99 with valid colours and the selection's colours ticked. OK with none: deletes the selection. OK with some: replaces them with one stop per ticked colour at the first's position (`:98-147`).

**F91. Discrete display.** With discrete on, the bar and strip draw one horizontal band per distinct colour, equal height, each a blend filtered to that colour (others black), so overlapping stops show as stacked ramps (`ColorGradient.cs:1094-1122`, `:363-393`). Colours are in first-seen stop order, by list order, and are distinct by their RGB value. Focus is ignored in a filtered blend (`:397`). With no stops: nothing drawn.

**F92. Library state** (`CGE:75-116`):

| State | Label | Title | Panel | Save Preset | Load Preset | Unlink Gradient | Edit Library Gradient |
|---|---|---|---|---|---|---|---|
| Is the library's copy, name known | "This gradient is the library gradient: {name}" | "Color Gradient Editor: Library Gradient {name}" | editable | off | off | off | off |
| Same, name unknown | "This gradient is a library gradient." | "Color Gradient Editor: Library Gradient" | editable | off | off | off | off |
| Linked | "This gradient is linked to the library: {name}" | "Color Gradient Editor" | read-only | off | on | on | on |
| Not linked | "This gradient is not linked to any in the library." | "Color Gradient Editor" | editable | on | on | off | off |

Button positions: Load Preset top left, Save Preset below it, Unlink and Edit Library Gradient to its right, label to the right of Load (`ColorGradientEditor.json`). Note the words: **Load Preset**, **Save Preset**.

**F93. Library group actions.**
- *Save Preset:* prompt **"Gradient name?"**; empty: "Please enter a name.", **"Color Gradient Name"**; exists: **"There is already a gradient with that name. Do you want to overwrite it?"**, **"Overwrite gradient?"** (Yes, No, Cancel); stores a deep copy; no link (`CGE:131-171`).
- *Load Preset:* opens F95; OK sets a deep copy linked to the chosen name (`CGE:119-129`). No discrete check: on a discrete target this can load colours the target can't show, unlike a drop (F68), and the dialog stays in its discrete mode. KEEP.
- *Unlink Gradient:* keeps the stops, drops the link (`CGE:173-177`, `ColorGradient.cs:1379-1383`).
- *Edit Library Gradient:* opens the editor on the library entry, then refreshes (`CGE:179-191`).
*Library item editing* passes `discrete = false` and no valid colours (`ColorGradientLibrary.cs:116`). So library gradients are edited without a discrete limit.

**F94. Gradient data rules to keep.**
- `new ColorGradient()`: one white stop at 0, focus .5, alpha 255 at 0 and 1 (`ColorGradient.cs:270-279`).
- A solid colour gradient: one stop at 0 (`:290-295`). A drop of a colour makes that (F68).
- Positions and focus must be in 0..1; the setter throws otherwise (`:51-53`, `:73-74`, `:90-91`). GUARD: clamp in Rust.
- Empty stops are allowed: the blend then falls back to transparent at both ends (`:418-420`, `:438-443`). In discrete drawing, nothing (F91).
- Equality: linked gradients compare by library name; a linked and an unlinked are unequal; otherwise stops and alphas in list order, each comparing position, focus and colour (`:1264-1285`, `:103-110`, `:149-161`, `:1549-1553`). `IsCurrentLibraryGradient`, title and gamma don't count, unlike a curve's current flag (F70). This equality decides F62's no-op check. `vixen_core::ColorGradient`'s `PartialEq` must be checked against it (Tests).
- `IsCurrentLibraryGradient` and `LibraryReferenceName` are saved, and copied by copying.

### L. Gradient library: selector and window

**F95. Color Gradient Library selector.** As F79, with: titled **Color Gradient Library**, minimum 574x397, buttons **New Gradient**, **Edit Gradient**, **Delete Gradient** (`ColorGradientLibrarySelector.json`). Thumbnails 68x68 with a 2 pt grey border, `GenerateColorGradientImage(68,68,false)` (`ColorGradientLibrarySelector.cs:52-87`). New's prompt: **"Gradient name?"**, empty **"Please enter a name."**, **"Color Gradient Name"** (`:113-151`). Delete: **"If you delete this library gradient, ALL places it is used will be unlinked and will become independent gradients. Are you sure you want to continue?"**, **"Delete library gradient?"**, Yes/No (`:176-197`). Double-click, Return, Escape as F79 (`:199-232`). The Designer's Cancel has no `DialogResult` (`ColorGradientLibrarySelector.json`) but is the form's Cancel button in code; on the Mac it cancels.

**F96. Gradient Library window.** As F80 to F85 with these differences, all from `Form_GradientLibrary.cs`:
- Title **Gradient Library**; toolbar **Edit Gradient**, **New Gradient**, **Delete Gradient**, **Export** ("Export Gradient Library"), **Import** ("Import Gradient Library"); check box **Maintain Library Link** at the bottom (`Form_GradientLibrary.json`).
- Thumbnail via `GenerateColorGradientImage(size, false)` (`:156-180`). Scale settings `{Name}/GradientLibraryImageScale`, `{Name}/GradientLibraryTextScale` (`:105-112`, `:552-554`).
- Name prompt **"Gradient name?"**; empty: **"Please enter a name."** titled **"Warning"**; overwrite as F93 (`:237-286`).
- Delete: **"If you delete the selected library gradient(s), ALL places it is used will be unlinked and will become independent gradients. Are you sure you want to continue?"**, **"Delete library gradient?"** (`:288-313`).
- Drag payload `LibraryDragPayloadFactory.Create(gradient, name, LinkGradients)` (`:343-354`). Drop accepted only unlinked (`:367-376`). Reorder and placement as F82 (`:381-431`).
- Export `Vixen 3 Color Gradient Library (*.vgl)`, `Dictionary<string, ColorGradient>` (`:437-475`), failure text **"Unable to export data, please check the error log for details."** (with the period here). Import same filter; failure **"Sorry, we didn't recognize the data in that file as valid Color Gradient Library data."**, `"Invalid file"`; duplicates renamed `" 2"`... (`:477-530`).

**F97. Editing a library gradient.** `EditLibraryItemAsync`: as F86 with the gradient editor, no discrete limit (`ColorGradientLibrary.cs:110-126`, `:55-78`). Entry flags as in F86, and a linked gradient reads through the library when its data is used (`ColorGradient.cs:1340-1377`).

### M. Color library window and discrete colour pickers

**F98. Color Library window.** Fully specified in `docs/spec/sequencer-shell.md` F341 to F350; those entries match `Form_ColorLibrary.cs` as read here. Additions and details the implementer needs:
- Colours are favourites, app-wide, not per profile: `%AppData%/Vixen/ColorPalette.xml` (`:105`). Duplicates are allowed; removal takes the first equal value (`:417`). Colour equality there is `System.Drawing.Color` equality.
- Swatches `48 * scale`, spacing `size + 10` both ways, tooltip `R: {r} G: {g} B: {b}`, no visible label (`:113-120`, `:225-265`, `:426-434`).
- Unlike the curve and gradient windows the scale has **no minimum** (`:542-556`). KEEP, GUARD at 1 pt.
- Drag: a Move drag of the selection inside the list, and a Copy drag of the first selected colour once the pointer leaves the list (`:463-471`). Mac: one drag session carrying the colour, moving when dropped inside the list.
- Delete refills and saves whether or not the user confirmed (`:406-424`).
- Hosts also call `AddColor`, `AddColorSet`, `RemoveColor`, `RemoveColors`, `UpdateColorSet` (`:168-223`) from the toolbars (sequencer-shell). Each saves and raises `ColorsChanged`.

**F99. Multiple Discrete Color Picker** (`Vixen.Common/DiscreteColorPicker/Views/MultipleDiscreteColorPickerView.xaml`).
*Does:* modal, titled **Select Color(s)**, centred. A wrapping list of items, each a check box followed by a 50x50 swatch; item width 92. Ticking selects, clicking the item toggles; both are the same state (`:40-41`, `:54-60`). OK (default) and Cancel (cancel) at bottom right; **OK is always enabled** (`ViewModels/DiscreteColorPickerViewModelBase.cs:156-160`). Available colours in set order; the given colours ticked at start (`:90-115`).
*Result:* colours whose box is ticked, in list order (`ViewModels/MultipleDiscreteColorPickerViewModel.cs:27-31`). Used by F48 and F90.

**F100. Single Discrete Color Picker** (`.../SingleDiscreteColorPickerView.xaml`).
*Does:* modal, titled **Select Color**, centred. A wrapping list of 50x50 swatches; item width 72; selecting highlights. **Ok** (sic) and Cancel at bottom right. **Ok** enabled only with a selection (`ViewModels/SingleDiscreteColorPickerViewModel.cs:14-18`). The current colour starts selected if in the set; if it isn't, nothing is and Ok is off.
*Used by:* F36, for colour rows on discrete targets (`EE/Editors/ColorTypeEditor.cs:31-46`).

**F101. Picker window size** (`Views/DiscreteColorPickerViewBase.cs:54-127`).
`root = round(sqrt(n) + 0.5, away)`, minus 1 when `(root-1)^2 == n`. If `n < threshold` (3 for multiple, 4 for single): `root = 1`; else if `n < 5`: `root = 2`. Capped at 8 columns, adding 10 for a scroll bar. `Width = root * itemWidth + 10 + scroll`, at least `max(250, 3 * itemWidth + 10)`. `delta = root*root - n`, `heightAdjust = max(0, delta / root)`, `Height = (root - heightAdjust) * 72 + 75`. Scrolls vertically past 8 rows, no horizontal scroll. Rust: pure function `discrete_picker_size(n, item_width, threshold) -> (w, h)`; or simply a `LazyVGrid` in a sheet of about that size.
*Mac:* sheets on the sequencer window. Item layout as a grid of swatches, check marks drawn in the corner for the multiple kind; keep the check box visible, since the original has them.

**F102. Which colours are "discrete".** `Util.GetDiscreteColors(component)` (`EE/Internal/Util.cs:19-48`): a row implementing `IDiscreteColorProvider` asks itself (only `CustomStateItem`, `Vixen.Modules/Effect/State/CustomStateItem.cs:13`, F106); an effect: union over `TargetNodes` of `ColorModule.getValidColorsForElementNode(node, true)`; an array of those: union of each. Empty means full colour. Rust: `TargetInfo::valid_colors` (`fw.rs:645-647`) for effects, union across the selection.

### N. Colour picker

**F103. Vixen's Color Picker** (`Vixen.Common/Controls/ColorPicker/ColorPicker.cs`, `ColorPicker.resx`, `ColorLabel.cs`, `ColorSelectionFader.cs`, `ColorSelectionPlane.cs`, `ColorSelectionModules*.cs`). Everything it can do, so that the Mac replacement can be judged:
- Modal, fixed size, 306x268, titled **Color Picker**, or the row's display name from a colour row (`EE/Editors/ColorTypeEditor.cs:53`).
- A 2D colour plane and a vertical fader for the chosen channel. Radio buttons **H, S, V** and **R, G, B** (or **L, a*, b*** in the other mode) choose the fader's channel (`ColorPicker.cs:569-583`, `:630-647`). Tooltips: plane **"Hold Down Shift for snap to grid"**, fader **"Hold down Shift for snap at 10% steps"** (`ColorPicker.resx`); the snap in both is really to eighths (`ColorSelectionPlane.cs`, `ColorSelectionFader.cs`: `Math.Round(v * 8) / 8`).
- Number fields for all six channels, with `°`, `%`, `%` after H, S, V. Typing applies on key up, parsed as an integer; Return or leaving the field reformats (`:669-742`).
- A colour readout showing new above old, each with its hex text. Tooltip **"Right Click here for more options.\nLeft Click and drag outside to pick a color from screen."** Dragging out samples the screen under the pointer (`ColorLabel.cs:82-117`).
- Readout context menu: **HSV - RGB**, **HSV - LAB** (radio), **Previous Color** (back to the old colour), **Copy to Clipboard** (`RRGGBB` hex) (`ColorPicker.cs:589-618`, `ColorLabel.cs:37-40`).
- **Quick Picks** group: white, red `(255,0,0)`, green `(0,255,0)`, blue `(0,0,255)` buttons, each setting the colour at once and not closing (`ColorPicker.cs:895-921`).
- `LockValue_V`: forces HSV V to 100%, disables the V field and radio (`ColorPicker.cs:830-858`). Every caller in this area passes false, or the panel's `LockColorEditorHSV_Value`, also false (`GEP:27`).
- Colours are held as CIE XYZ, which is what `ColorPoint` stores (`ColorGradient.cs:1494`). OK returns the picked colour; the caller converts with `ToRGB().ToArgb()` for plain colours.
*Mac:* `NSColorPanel` per `docs/UI_GUIDE.md`. It covers the plane, sliders, numeric entry, eyedropper and hex. **Quick Picks** are kept as a small accessory view on the panel with the same four swatches, since the owner will miss them: `NSColorPanel.setAccessoryView`. Commit on panel close or on the row losing it, one undo step per session. Panel mode isn't remembered by us; the system does. The panel can't be modal: a gradient stop or colour row opens it and writes when it closes or when the target changes. Alpha off (`showsAlpha = false`), since upstream's result has alpha 255 (`ColorLabel.cs:133`).
*Convert exactly:* Swift gives Rust device RGB bytes in sRGB; Rust does `Xyz::from_rgb` for stops, so colours chosen by number stay exact.

### O. The other property grid

**F104. `Vixen.Common/WpfPropertyGrid`.** This is **not** the Effect Editor. The Effect Editor's `EE/` is a fork of the same open-source grid (Denys Vuika's WPF Property Grid), and doesn't reference it. It's used by the Custom Prop Editor window and Timing Track Browser's vendor metadata panes, all with `ItemSource`, `ShowReadOnlyProperties="True"`, `PropertyFilterVisibility="Collapsed"`, in the alphabetical layout (`Vixen.Modules/App/CustomPropEditor/Views/CustomPropEditorWindow.xaml:173-178`, `VendorMetadataView.xaml:7-12`, `PhysicalMetadataView.xaml:22-30`, `Vixen.Modules/App/TimingTrackBrowser/Views/VendorMetadataView.xaml:8-13`). Those views belong to their own specs. Differences that matter if they reuse this spec's row views:
- Default layout `AlphabeticalLayout`: rows sorted as F14 with no categories; a visible vertical scroll bar (`Design/AlphabeticalLayout.xaml:11`, `PropertyGrid.cs` ctor).
- Row: name left of editor, whole row tooltip (`Themes/PropertyGrid.xaml:41`). No collection rows.
- Editors: default text box with validation messages, enum, check box, font family combo, named-colour brush combo, plain slider (no snap, no delay), password box, double editor (`Themes/EditorResources.xaml`). Per-property templates by `DeclaringType` and `PropertyName`, such as Physical Metadata's `BulbType` (`PhysicalMetadataView.xaml:29`).
- `ItemsSource` tracks a collection through a weak listener (`PropertyGrid.cs`). Undo: the view model's own business.
- Unused by anything in the app: `TabbedLayout`, `ExtendedPropertyEditorTab`, `DataResource`, the search box while filter visibility is Collapsed, `PasswordBoxExtender` (no property declares it; to confirm in the Custom Prop Editor spec).
*Mac:* use `Form` or `Grid` directly for those panes. **VixenKit should hold reusable row views** (`NumberFieldRow`, `CheckRow`, `ChoiceRow`), so both areas share look without sharing the panel model.

### P. Keys and modified gestures, in one place

Modifier states are read at the moment stated. "Exactly" and "holding" as in Conventions.

| Where | Input | Effect | Entry |
|---|---|---|---|
| Panel, not in a text box | any key the focused editor didn't handle | forwarded to the sequencer's shortcuts | F66 |
| Text box | Return | commit | F24, F31 |
| Text box | Escape | revert | F24, F31 |
| Leaving a text box or the panel | | commit | F66 |
| Enum drop-down | Down | open | F29 |
| Slider | arrows | ± tick | F32 |
| Slider | Page Up, Page Down | ± 5 | F32 |
| Slider | Home, End | minimum, maximum | F32 |
| Int or double strip | drag, ⌘ or ⇧ held | × 1 instead of × tick | F24 |
| Curve strip, focused | Return | Curve Editor | F43 |
| Curve strip | exactly ⌘, click | add point at press | F40 |
| Curve strip | exactly ⌥, click near a point | remove point | F41 |
| Curve strip | exactly ⇧, drag | flat level | F42 |
| Curve strip | holding ⇧ | show level bar | F42 |
| Curve strip | ⇧⌘, click | invert | F43 |
| Curve strip | ⇧⌥ or ⇧⌘⌥, click | reverse | F43 |
| Gradient strip, focused | Return | Gradient Editor | F50 |
| Gradient strip | ⇧⌥, press | reverse | F49 |
| Gradient handle, focused | Delete | delete stop | F47 |
| Gradient handle | exactly ⌥, click | delete stop | F47 |
| Curve Editor graph | ⌘ held, press | add point | F72 |
| Curve Editor graph | ⌥ held, press near a point | remove point | F72 |
| Curve Editor graph | ⇧ held, drag | flatten | F72 |
| Curve Editor graph | Escape during a point drag | restore the point | F72 |
| Curve Editor | Return, also in X, Y, Draw Interval; Escape | OK, Cancel | F70, F73 |
| Function generator | Return, Escape | Generate, Cancel | F75 |
| Gradient Editor | Delete on the bar | delete selected | F89 |
| Gradient Editor | Return, also in Location; Escape | OK, Cancel | F87 |
| Gradient Editor Location | digits, Return, leaving | apply, unless Return closes first | F89 |
| Library selectors | Return, Escape | OK, Cancel | F79, F95 |
| Name prompt | Return, Escape | OK, Cancel | F107 |
| Library windows | ⌘-scroll | thumbnail size | F85 |
| Curve, Gradient windows | ⇧ with list focused, ⌘-scroll | label size | F85 |
| Colour Picker plane, fader | ⇧ held | snap to eighths | F103 |
| ⌘Z, ⇧⌘Z | | undo, redo a commit | F63 |

Mouse summary: a double-click opens the row's dialog on any swatch or strip, a stop's colour on a handle. Press and drag on a swatch or strip where no gesture captures starts a value drag (F67).

### Q. Upstream controls to features

For `docs/parity/`: every named control of these windows.

| Window, source | Control | Entry |
|---|---|---|
| Panel, `EE/Themes/PropertyGrid.xaml`, `EE/Design/CategorizedLayout.xaml` | "Effect Preview" label, `previewToggleSwitch` | F5 |
| | `EffectName` label | F7 |
| | Save as Default, Reset to Built-in | F8, F9 |
| | help link, More Info link | F10, F11 |
| | Information text | F11, F3, F4 |
| | Category expander | F13 |
| | Row name | F15 to F17 |
| | Editors: check box, combo, enum, int, double, sliders, text, label, file, font, colour, polygon, curve, gradient, lists | F22 to F37, D, E, F |
| `CurveEditor` form | `zedGraphControl` | F72 |
| | `grpCurve` "Curve": `btnReverse`, `btnInvert` | F74 |
| | `btnDraw`, `label1`, `textBoxThreshold` | F76 |
| | `btnFunctionCurve` | F75 |
| | `lblSelectedPoint`, `xLabel`, `txtXValue`, `yLabel`, `txtYValue`, `btnUpdateCoordinates` | F73 |
| | `groupBoxLibrary` "Library": `labelCurve` | F71 |
| | `buttonLoadCurveFromLibrary` "Load Curve" | F77 |
| | `buttonSaveCurveToLibrary` "Save Curve", `buttonUnlinkCurve`, `buttonEditLibraryCurve` | F78 |
| | `labelInstructions1`, `labelInstructions2` | F71, F78 |
| | `buttonOK`, `buttonCancel` | F70 |
| `FunctionGenerator` | `txtDescription`, `lnkHelp`, `label1`, `txtFunction`, `btnGenerate`, `btnCancel` | F75 |
| `CurveLibrarySelector` | `listViewCurves`, `buttonNewCurve`, `buttonEditCurve`, `buttonDeleteCurve`, `buttonOK`, `buttonCancel` | F79 |
| `Form_CurveLibrary` | `toolStripCurves`: Edit, New, Delete, Export, Import; `listViewCurves`; `checkBoxLinkCurves` | F80 to F85 |
| `ColorGradientEditor` | `gradientEditPanel` | F88 to F90 |
| | `groupBoxLibrary`: `labelCurve`, `buttonLoadFromLibrary` "Load Preset", `buttonSaveToLibrary` "Save Preset", `buttonUnlink`, `buttonEditLibraryItem` | F92, F93 |
| | `buttonOK`, `buttonCancel` | F87 |
| `GradientEditPanel` | `edit` | F88 |
| | `grpStops` "Selected Stop": `label4`, `lblColorSelect`, `label5`, `vColorLoc`, `btnDeleteColor` | F89 |
| `ColorGradientLibrarySelector` | `listViewColorGradients`, New, Edit, Delete, OK, Cancel | F95 |
| `Form_GradientLibrary` | `toolStripGradients`, `listViewGradients`, `checkBoxLinkGradients` | F96 |
| `Form_ColorLibrary` | `toolStripColors`, `listViewColors` | F98 |
| `MultipleDiscreteColorPickerView` | list, OK, Cancel | F99 |
| `SingleDiscreteColorPickerView` | list, Ok, Cancel | F100 |
| `ColorPicker` | plane, fader, radios, number fields, `lblColorOut` and its menu, `quickPickBox` and four buttons, OK, Cancel | F103 |
| `TextDialog` | `labelPrompt`, `textBoxResponse`, `buttonOk`, `buttonCancel` | F107 |
| `MessageBoxForm` | `txtMessage`, `messageIcon`, `buttonOk`, `buttonNo`, `buttonCancel` | F108 |
| Grid commands with no bound UI | `ResetFilter`, `Reload`, `Show`/`Hide`/`Toggle` Read-Only, Attached and Filter, `ClosePropertyTab`, `ShowExtendedEditor`, `ShowEmitter*`; `IntegerEditor` and `DoubleEditor`'s `Increase`, `Decrease` | Omitted |

### R. Added by the audit

**F105. Choice lists in a multi-selection.**
*Does:* a selection row's converter object comes from First (`EE/Internal/MergedPropertyDescriptor.cs:359-362`), but it's handed the whole array as `context.Instance` (`EE/PropertyItem.cs:202-205`, `:424-433`), so what it lists depends on the converter:

| Converter | Used by | One effect | Several effects | Blank when mixed |
|---|---|---|---|---|
| `EnumConverter`, `EnumDescriptionTypeConverter` | every enum row | F29 | the same | **yes**, because the value is an enum |
| `BooleanStringTypeConverter` | `Bool` rows on Spin, Alternating, Text | true word, false word | the same; words that differ between effects drop out of the merged attributes, giving `True`, `False` (`Vixen.Core/TypeConverters/BooleanStringTypeConverter.cs:81-96`) | no, First's |
| `TargetElementDepthConverter` | Chase, Twinkle, Spin, Alternating, Dissolve, audio effects | `0..depth` (F28) | smallest depth among effects with a target (`Vixen.Core/TypeConverters/TargetElementDepthConverter.cs:48-63`) | **yes** |
| `FireTargetElementDepthConverter`, `WipeTargetElementDepthConverter` | Fire, Wipe | `1..depth-1` | smallest depth | no, First's |
| `IMarkCollectionNameConverter` | Fireworks, Strobe, Shapes, State, Alternating, Text, Dissolve | all names | those of the first effect that supports marks | no, First's |
| `LipSyncMarkCollectionNameConverter` | LipSync | Phoneme collections and the selected one | those of the first LipSync that supports marks | no, First's |
| `EffectListTypeConverterBase<T>` | State's definition and item; function and index names of Frost, Prism, Gobo, Spin Color Wheel, Fixture Strobe, Fixture | the effect's own list | **the union, in first-seen order, over the effects that are a `T`** (`Vixen.Modules/Effect/Effect/EffectListTypeConverterBase.cs:55-97`) | no, First's |
| `PhonemeMappingConverter` | LipSync | names in the LipSync map library | the same | no, First's |
| Row converters: `ExpandoMarkCollectionNameConverter`, `EmitterMarkCollectionNameConverter`, `CustomStateItemNameConverter`, `FixtureIndexCollectionNameConverter`, `ColorIndexConverter` | Wave, Emitter, Custom State, Fixture function rows | from the row | from the row, so First's list's rows | no, First's |

The chosen **text** goes through each effect's own setter, which looks the text up for itself (F62). So an effect whose own list lacks that name does what its setter does with an unknown name: State's definition and item setters ignore it (`Vixen.Modules/Effect/State/State.cs:182-187`, `:250-255`), and Frost's function setter, like the other function and index setters of that row of the table, stores it as given (`Vixen.Modules/Effect/Frost/FrostModule.cs:53-57`). A mark collection picker would store no collection (`Vixen.Modules/Effect/Fireworks/Fireworks.cs:81-82`), but every effect in a sequence sees the same collections, so that can't arise from these lists.
*Rust:* `selection_values` answers for one effect (`fw.rs:771`). `panel_model` runs it per effect and merges by `selection_merge` (Extra metadata), and a choice's index refers to the **merged** list, not First's. `commit` gives every effect the chosen item's `value`, and the effect-level lists that exist today have values that mean the same on every effect: text that the effect's own `param_changed_ctx` looks up (`crates/fx-audio-state/src/state.rs:331-367`), the sequence's shared mark collection ids, depth numbers. A future list whose values differ per effect would have to be resolved by its label instead. `PhonemeMappingConverter`'s list comes from outside the effect: the row's `selection_values` is empty today (`crates/fx-audio-state/src/lip_sync.rs:5-6`, `:40-42`), so `panel_model` needs the LipSync map library for it.

**F106. Custom State rows** (State effect, `CustomStateItems`, shown when Render Source is Custom, `crates/fx-audio-state/src/state.rs:32`).
*Do:* pick a State Item in the drop-down and set the colour at the end of its line; Add and Remove below.
*Does:* one line per row, no labels, the drop-down taking the free width (F52).
- *Options* (`Vixen.Modules/Effect/State/State.cs:718-748`): no State definition or no items: `<No State Items Available>`. Playback Default: the definition's items, leaving out those on other rows; a row keeps its own. Playback Cycle (`Iterate`): `<None>` first, then all items. A row whose item no longer exists also gets `<Missing State Item: {short id}>`, at the top, or after `<None>`. Labels that would repeat add what the item is assigned to in brackets, then a number, then a short id (`:956-979`).
- *Choosing* an item sets the row's item and also **its colour, to that item's colour**; `<None>` clears the item and keeps the colour; a text that isn't offered is ignored (`:777-806`).
- *Colour:* discrete colours are those valid for the elements assigned to the row's item; no item means full colour (`:814-832`, F102). A drop or dialog on it follows that set.
- *New row:* the first item no other row uses, with its colour (`:1146-1173`).
- *The list also changes from other rows:* Render Source becoming Custom with an empty list adds a row, if an item is free (`:331`, `:1234-1247`). Choosing another State definition clears the list and does the same (`:192-196`). Playback returning to Default drops rows with no item and later duplicates (`:361-364`, `:1249-1268`). On the Mac these are undone with the commit that caused them (F63).
- *Minimum:* F57. *Undo:* F55.
*Rust:* the options and those three list changes exist (`crates/fx-audio-state/src/state.rs:280-300`, `:162-187`, `:342-345`). The colour change on choosing doesn't: `param_changed_ctx` has no `CustomStateItems` case (`:306-369`). Add one that compares rows with the old list, as F55 says.

**F107. Name prompt** (`Vixen.Common/Controls/TextDialog.cs`), used by F78, F79, F81, F82, F93, F95, F96.
*Does:* a fixed modal, centred on its parent, with no title in these uses: the prompt text, a one-line text field with the focus, then OK and Cancel at bottom right (`Vixen.Common__Controls__TextDialog.json`, `Vixen.Common/Controls/TextDialog.cs:7-12`, `:31-36`). Return is OK and Escape is Cancel, from anywhere in it, through the form's key preview and not through default buttons (`:40-44`). It starts empty. The loops in F78 and the others reuse one instance, so after "No" on the overwrite question, or after the empty-name error, the typed text is still there (`CE:463-465`).
*Names of spaces:* the only check is `== string.Empty` (`CE:466`), with no trim, so a name of only spaces is stored. KEEP. With Maintain Library Link on, dragging that entry out then throws, because the payload refuses a blank name (`TSE/LibraryDragPayloadFactory.cs:26-29`, `Form_CurveLibrary.cs:353`, `Form_GradientLibrary.cs:350`). GUARD: the drag carries an unlinked copy.
*Mac:* a name sheet with the prompt as its message, Cancel and OK, keeping the text between rounds (`docs/UI_GUIDE.md`, modal dialogs).

**F108. Message boxes in this area** (`Vixen.Common/Controls/MessageBoxForm.cs:11-25`).
*Does:* an icon at left, the message, and a right-aligned row of buttons. With neither flag of the two-flag constructor, only **OK**; with either flag the first button reads **YES**, and **NO** and **CANCEL** show as flagged, in that order from the left (`Vixen.Common/Controls/MessageBoxForm.cs:15-24`, `Vixen.Common/Controls/MessageBoxForm.Designer.cs:97-100`). YES and OK answer OK, NO answers No, CANCEL answers Cancel (`:46`, `:58`, `:70`), and so does the close box (from knowledge of WinForms). The form sets no default or cancel button, so Return presses the button with the focus, the first one on opening, and Escape does nothing (also from knowledge of WinForms; its `KeyPreview` has no handler). Fixed size, centred on its parent, kept on top (`:146`, `:153`, `:155`). The icon is a static field each caller sets first, and closing clears it (`Vixen.Common/Controls/MessageBoxForm.cs:9`, `:77-86`): Error for empty names; Question or Warning for overwrite and delete, varying by file (`CE:468`, `:476`, `Form_GradientLibrary.cs:255`). The function errors of F75 use the second constructor with OK only (`CE:596`).
*Mac:* `.alert` with verbs per `docs/UI_GUIDE.md`, and the entries above name the verbs where a choice matters. Return takes the default action and Escape cancels, as on any Mac alert; that's the one behaviour that differs from upstream's box.

### Quirk register

Treatment per `docs/PORTING_GUIDE.md`: port quirks as written, except crashes. FIX items are divergences for the owner's veto.

| Entry | Quirk | Treatment |
|---|---|---|
| F4 | Limit message says `< 75`; 75 is allowed | KEEP |
| F4 | Effect Preview still plays a selection over the limit | KEEP |
| F6 | Loop length uses the last-starting effect's end, not the latest end, so a longer effect overlaps the next loop | KEEP |
| F6 | Zero-duration selection throws in upstream's timer | GUARD: skip |
| F8 | A failed write throws out of Save as Default and Reset | GUARD: alert, keep running |
| F13 | Fold state lost if a selection without that category or none comes between | KEEP |
| F13 | Video's Advanced Settings starts collapsed | KEEP |
| F21 | Video `StartTimeSeconds` gets a numeric field because of a misspelt editor class | KEEP, change table |
| F24 | ⌘ and ⇧ during strip drag multiply by 1, not by a tick | KEEP |
| F24, F25 | A stored value outside the range shows clamped or rounded; upstream may write it back | GUARD: showing never writes |
| F25 | Double with no precision rounds to whole numbers | KEEP |
| F29 | `TilePictures.None` isn't offered; a stored one shows blank | KEEP |
| F32 | Slider commits while dragging; each is an undo step | **FIX**: coalesce into one step per gesture, keep the 300 ms live commit |
| F32 | A delayed commit could be lost or misplaced by a selection change | GUARD: flush first |
| F33 | Gain write with no stored change still pushes undo | **FIX**: no step when stored values are equal |
| F39 | Inline move doesn't sort, so a point can pass its neighbour | KEEP |
| F39 | Curve release tests ignore the button, and a release with no press uses the last press | GUARD: left press and its own release |
| F39 | The drag's mode follows the keys at each move | KEEP |
| F40 | ⌘-drag previews a move, then adds a point where the drag began | KEEP |
| F42 | The level handle has F46's `SliderPoint.Center` bug | **FIX**: apply every move |
| F46 | `SliderPoint.Center` raises its event only when unchanged; drag updates are lost | **FIX**: apply every move |
| F47 | Last stop can be deleted | KEEP; transparent gradient, no crash (F94) |
| F48 | Error box with a wrong title for non-discrete multi-stop handles | KEEP text; can't occur from the UI except for same-position stops |
| F51 | Gradients in record lists ignore discrete limits | KEEP |
| F53 | An unchanged pair still commits, Cancel in its dialogs included | **FIX**: no step when both halves are equal |
| F54 | The string list selects its first row on load, and a click in a box doesn't select | **FIX** with F58: clicking in a box selects |
| F55 | Custom State row edits push no undo step | **FIX**: one step |
| F56 | Add on a colour list adds transparent `Color.Empty` | KEEP |
| F56 | Add on an empty non-expando list does nothing | KEEP |
| F56 | Add does nothing when all selected lists are empty | KEEP |
| F58 | Strips swallow the click, so a gradient row is selected only by its margin | **FIX**: a no-op click selects the row |
| F62 | Setting First's value on a mixed row does nothing | KEEP |
| F62 | Any collection edit overwrites every effect's list with First's | KEEP |
| F63 | Empty undo step `0 Effect(s)  modified` | **FIX**: skip |
| F68 | Gradient on a colour row: accepted, does nothing | **FIX**: refuse the drop |
| F72 | Move-whole-curve and flatten run on a linked curve | **FIX**: refuse |
| F73 | Update can act on a stale point | GUARD: disable after replacing points |
| F73 | Return in X or Y closes the dialog | KEEP |
| F75 | Draw Interval of 0 loops forever in f(x) | GUARD: error dialog |
| F76 | Draw and f(x) enabled on a linked curve | **FIX**: disable |
| F76 | Y at draw release and on the first point isn't clamped | GUARD: clamp |
| F76 | A click with no drag leaves one point | KEEP |
| F79 | Selector's double-click accepts and also presses Edit | **FIX**: accept only |
| F79 | New Curve then Cancel leaves a default ramp in the library | KEEP |
| F79 | Overwriting by New Curve, then Cancel, loses the old curve | KEEP |
| F82 | Naming a drop over an existing name moves the last item, not that one | **FIX**: move the written entry |
| F82 | A list item from the other library window counts as a reorder | GUARD: own drags only |
| F87 | Return closes the gradient dialog from the Location field | KEEP |
| F88 | Any mouse release writes the selected stops, so a click snaps them and a right-click moves them | **FIX**: left-button drags only |
| F89 | Location and Delete are enabled at opening with nothing selected | **FIX**: disabled |
| F93 | Load Preset ignores discrete limits | KEEP |
| F98 | Delete refills and saves even when cancelled | KEEP, harmless; no visible effect |
| F98 | Colour scale has no minimum | GUARD: 1 pt |
| F105 | A union's chosen name may not exist on every effect | KEEP: each effect's setter ignores or stores it |
| F107 | A name of only spaces is accepted | KEEP |
| F107 | Dragging such an entry with the link on throws | GUARD: unlinked copy |

## Data and persistence

| What | Where | Format | Read | Written | Upstream source |
|---|---|---|---|---|---|
| Effect parameter values | The sequence (`.tim`) | Each effect's `[DataContract]` data, already handled by `vixen-io` and `Params` | Sequence load | Sequence save; a commit only marks the sequence modified | `FormEffectEditor.cs:152`, `fw.rs:203-210` |
| Curve library | `<Vixen 3>/SystemData/ModuleStore.xml`, type data of module `4e258de2-7a75-4f0f-aa43-c8182e7f3400`, member `_library`, as a `DataContractSerializer` dictionary in order | XML, read and written by `crates/vixen-io/src/library.rs:53-116` | Load; lazily through every linked curve | After every library change, async (`VixenSystem.SaveModuleConfigAsync`), once per bulk update | `CurveLibrary.cs:54-105`, `:143-165`, `CurveLibraryStaticData.cs:14-29` |
| Gradient library | Same file, module `64f4ab26-3ed4-49a3-a004-23656ed0424a` | XML, `library.rs:59-62`, `:119-137` | As curves | As curves | `ColorGradientLibrary.cs:55-108`, `ColorGradientLibraryStaticData.cs` |
| Selector window bounds | Same store: `SelectorWindowBounds` in each library's static data | `Rectangle` | Selector open; used only if it fits the screen and isn't narrower than the minimum | Selector closing (in memory; saved with the next module save) | `CurveLibrarySelector.cs:36-49`, `ColorGradientLibrarySelector.cs:37-50` |
| Effect defaults | `<Vixen 3>/Effect Defaults/EffectDefaults.vfd`, `crates/vixen-io/src/paths.rs:78` | **.NET binary XML** (`XmlDictionaryWriter.CreateBinaryWriter`, MC-NBFX) of `EffectDefaultsStore { Version = 1, Entries }`. Each `EffectDefaultEntry { TypeId, EffectName, DataModelTypeName, SavedUtc, Payload }`, where `Payload` is itself binary XML of the scrubbed data model (`Svc/EffectDefaultsStore.cs`, `Svc/EffectDefaultsService.cs:494-509`) | Lazily on first use; again if the profile root changes (`:410-418`). Corrupt or absent: an empty store, no error (`:421-445`) | On save, clear and import. Written to `EffectDefaults.vfd.tmp`, then replaces the file (`:448-467`) | |
| Effect defaults export, import, dump | User-chosen; `Vixen 3 Effect Defaults (*.vfd)` | Same binary store; dump is indented XML with settings written out | Menu commands: see sequencer-shell F93 to F95, F403 | | `Svc/EffectDefaultsService.cs:218-407` |
| Colour favourites | `%AppData%/Vixen/ColorPalette.xml`: global, not per profile. Mac: `~/Library/Application Support/Vixen/ColorPalette.xml` (decision shared with sequencer-shell) | `DataContractSerializer` of `List<Color>`, indented | Panel construction and load | Every change | `Form_ColorLibrary.cs:105`, `:141-166` |
| Curve, gradient, colour exchange | `.vcl`, `.vgl`, `.vfc`, user-chosen | `Dictionary<string, Curve>`, `Dictionary<string, ColorGradient>`, `List<Color>` with `DataContractSerializer`, indented (tabs) | Import | Export | F84, F96, F98 |
| Library window scales | `XMLProfileSettings` AppSettings: `Form_CurveLibrary/CurveLibraryImageScale`, `/CurveLibraryTextScale`; `Form_GradientLibrary/GradientLibraryImageScale`, `/GradientLibraryTextScale`; `Form_ColorLibrary/ColorLibraryScale`. Invariant culture, default `1` | Profile settings, `UserDefaults` or Rust settings per the sequencer spec | Panel construction | Panel closing | F85 |
| Maintain Library Link | `TimedSequenceEditorForm/CurveLinkCurves`, `/GradientLinkGradients`; default false (sequencer-shell settings table) | Profile settings | Sequencer open (`TSE/TimedSequenceEditorForm.cs:364-365`) | Sequencer closing (`:5827-5828`) | F83 |
| Effect Preview on or off, fold state, picker mode | Memory only | | | | F5, F13, F48 |
| Picked files | Picture, Glediator, Shapes and Video: choosing an absolute path **copies the file into `<Vixen 3>/Module Data Files/<Effect>/` overwriting any file of that name, and stores just the name** (`Vixen.Modules/Effect/Picture/Picture.cs:231-238`, `:599-622`, `Glediator/Glediator.cs:114`, `:201-221`, `Shapes/Shapes.cs:221`, `:1067-1087`, `Video/Video.cs:554`). A relative path is stored as given. Launcher's `Executable` stores the path as chosen (`Launcher/Module.cs:105-119`) | | | On commit | F34 |
| Sequence-scoped default scrub | `[ExcludeFromEffectDefault]`, `ParamDesc::exclude_from_default`, ten members | | | | `Svc/EffectDefaultScrubber.cs` |

**Library links in a sequence.** A linked curve stores its name and its last-known points (`Curve.cs:71-111`). On every read of its points upstream re-reads the library (F86). So a sequence saved with stale points gets the library's on the next read, and the file has them once saved. `vixen_core::Curve` has the same fields (`crates/vixen-core/src/curve.rs:5-9`), `ColorGradient` too.

**Saved defaults keep library links.** Upstream's final behaviour: the scrubber doesn't unlink (`Svc/EffectDefaultScrubber.cs:13-14`, `docs/references/effects/effect-defaults.md` under `upstream/Vixen/docs`). The earlier design note `reviews/vix-3964-effect-default-settings-design.md` says the opposite and was superseded (`plans/effects/vix-3964-effect-default-settings.md` Decision Log). Follow the code.

**Applying defaults.** New effects from these paths use the default: draw on timeline, Add Multiple Effects, add at marks, `AddNewEffectById` (toolbox drop, hotkeys), replace with a different effect type, drop media. Clone, paste and LipSync don't, because they overwrite the data (`reviews/vix-3964...` table, `TSE/TimedSequenceEditorForm.cs:2271`, `:2570`). The sequencer spec owns the call sites; Rust: `AppCore::new_effect_params(type_id)`.

## Rust API needed

The first draft of this spec treated everything below as new. Code built from that draft has since appeared: `crates/vixen-session/src/effect_editor/` (`panel.rs`, `curve_strip.rs`, `gradient_strip.rs`, `dialogs.rs`, `function.rs`, `libraries.rs`, `drag.rs`, `numeric.rs`, `values.rs`), `crates/vixen-session/src/library_links.rs`, `crates/vixen-ffi/src/effecteditor.rs` and `app/Sources/VixenEffectEditor`. It was changing during this audit and wasn't compared with this spec line by line. So the signatures below are the contract and not a description of that code, and where it reaches the same behaviour with another shape, such as `ListStyle` in `panel.rs` for F52's table, its shape is fine. Anything the audit added (F105 to F108, the corrected entries, the metadata rows below) came after it and needs checking against it. One known case: `is_depth_list` in `panel.rs` makes every `Int` selection row blank when mixed, Fire and Wipe included, where F28 and F105 say they aren't. The `#[uniffi::export]` surface goes in `crates/vixen-ffi/src/effecteditor.rs`, feature `effecteditor`; records cross as `uniffi::Record`s. Logic lives in `vixen-session` so it can be tested without a window.

### Table metadata the `ParamDesc` tables need ("Extra metadata" in the entries above)

`crates/vixen-effects/src/framework.rs` and the fx-* tables; `scripts/check_effect_metadata.py` should check each.

| Field | Why | Values | Upstream |
|---|---|---|---|
| `read_only: bool` | F30. `Editor::Label` exists and no table uses it. `VideoLength` and `CacheSize` are `[ReadOnly(true)]` and browsable and so must show as labels | Video `VideoLength`, `CacheSize`; `StringCount` and `MaxPixelsPerString` (hidden anyway). Either a field or set `Editor::Label` on them | `Video/Video.cs:327`, `:417`, `PixelEffectBase.cs:89`, `:106` |
| `precision: u8` on `Number` and `SliderDouble`, or a table-wide default of 0 | F25 and the slider tooltip | From `[NumberRange]`'s fourth argument. Only `Strobe.cs:172` and `Alternating.cs:254` give one, both 0. So: precision is always 0 in today's tables; `Double`/`Default` rows round to whole numbers | `NumberRangeAttribute.cs:45-51` |
| Range for `Level` and `Percent` | No field needed. `Editor::Level` and `Percent` carry no range, and none of the five `LevelEditor` or five `Percentage` properties declares `[NumberRange]`, so upstream's fallbacks apply: 0..100, tick 1 (`Chase.cs:226`, `SetLevel.cs:68`, `Twinkle.cs:152`, `:168`, `Spin.cs:340`, `Candle/CandleModule.cs:94-154`, checked by search) | Fixed: 0, 100, 1 | `EE/Themes/SliderEditor.xaml:51-53` |
| Video `StartTimeSeconds` | F21 | `Editor::Number { 0, 10000, 1 }`, not `SliderDouble` | `Video/Video.cs:348` |
| PinWheel's Number on a `Curve` | `crates/fx-pixel-motion/src/generated/pin_wheel.rs:155`: harmless, the type's editor wins. Row builder must look at `kind` first | | |
| `RecordDesc` list metadata | F52 | `item_header: ItemHeader { None, Numbered("Emitter"), Numbered("Wave"), Numbered("Whirl"), Numbered("Polygon"), ItemText { rule_color_param } , InlineRow }`, `can_add_remove: bool` (false for Whirl), `buttons_above: bool` (true for Fixture), `add_when_empty: bool` (F56: true for the expando lists, false for Morph), `min_items: fn(&Params) -> u32` (F57: default 1, Fixture 0, Custom State reads Render Source), `can_reorder: bool` (false) | `EditorResources.xaml` |
| `ParamKind::Other("List<string>")` | F54 | Model as `ParamKind::StringList`, with raw XML preserved. Used by Text `TextLines` | `TextEditor.xaml:47-81` |
| `ParamKind::Other("PolygonContainer")` | F37 | Keep; row builder recognises it | |
| `ParamKind::Long` holding a `uint`, `Other("ulong")` | F26 | Type bounds from the kind: CustomValue's `Value32Bit` is uint, `Value64Bit` ulong; 8 and 16 bit are `Int` with ranges. Model the ulong as `ParamKind::ULong` if possible | `CustomValue/CustomValue.cs:277-325` |
| `Offset` on depth lists | F28 | No field needed: no effect declares `[Offset]`, and Fire's and Wipe's own converters are already ported as `selection` (`crates/fx-pixel-simple/src/fire.rs:28`, `crates/fx-basic/src/wipe.rs:44`) | `Vixen.Core/Attributes/OffsetAttribute.cs` |
| `EnumMember::hidden: bool` | F29 | Order and labels are in `EnumDesc.members` already. `[Browsable(false)]` members need a flag so `selection_values` (`fw.rs:786`) skips them while a stored value still loads and saves. Today only `TilePictures.None`, which `crates/fx-pixel-graphics/src/generated/picture.rs:23` lists as an ordinary member | `Vixen.Modules/Effect/Picture/TilePictures.cs:7-10` |
| `EffectDescriptor::collapsed_categories: &'static [&'static str]` | F13 | Categories that start folded. Video: `["Advanced Settings"]`; all others empty. Read from First | `Vixen.Modules/Effect/Video/Video.cs:21` |
| `ParamDesc::selection_merge: SelectionMerge { First, Union, MinDepth, FirstWithMarks }` | F105 | How `panel_model` merges per-effect lists for a selection; default `First`. `Union`: the `EffectListTypeConverterBase<T>` rows. `MinDepth`: all depth rows, Fire and Wipe included. `FirstWithMarks`: mark collection pickers and LipSync's. First's table decides, as First's converter does | `EE/Internal/MergedPropertyDescriptor.cs:359-362` |
| `ParamDesc::blank_when_mixed: bool` | F60, F105 | True for the `TargetElementDepthConverter` rows (Chase, Twinkle, Spin, Alternating, Dissolve, audio effects); false for Fire and Wipe. Enum rows are blank when mixed whatever this says | `EE/Internal/MergedPropertyDescriptor.cs:244` |
| Slider units | F33 | `Editor::Slider` min, max, step are **shown** units. Document it on the enum; it's already so | `fw.rs:83-99`, `:101-127` |
| File copying | F34 | A `file_kind: Option<FileAdopt { folder }>` on Path params, or a `param_changed_ctx` that receives the module data root; `ParamChange` has no root today | |

### `crates/vixen-session/src/effect_editor.rs`: the panel

```
pub struct PanelRequest { pub effects: Vec<EffectId> }
pub fn panel_model(s: &SequenceSession, effects: &[EffectId], limit: usize) -> PanelModel
    // Rows for a selection: F2 to F4, F13, F14, F21, F60, F61. Empty and too-many states included.
pub struct PanelModel { effect_name, information, information_link, message: PanelMessage, can_save_default: bool, has_stored_default: bool, categories: Vec<CategoryModel> }
pub struct CategoryModel { name, order: i32, collapsed_by_default: bool, rows: Vec<RowModel> }
pub struct RowModel { key: RowKey, label, tooltip, read_only: bool, control: RowControl }
pub struct RowKey { param: String, index: Option<u32>, sub_param: Option<String> }
    // A row of a list is (param, index, sub_param). Stable across commits, so that Swift keeps view identity.
pub enum RowControl { Check{value}, Choice{items: Vec<Choice>, selected: Option<u32>}, Int{text, min, max, step, kind}, Double{text, min, max, step, precision}, Slider{shown, min, max, tick, snap, tip, kind}, Text{value}, Label{value}, Path{value, kind}, Font{text, desc}, Color{value, text, valid: Vec<Color>}, Curve(CurveRow), Gradient(GradientRow), Items{kind, items, ...}, Polygons }
pub fn panel_delta(s: &SequenceSession, effects: &[EffectId], key: &RowKey) -> PanelDelta
    // After a commit: changed values, row visibility and choice lists, without rebuilding rows (F18, F19).
pub fn commit(s: &mut SequenceSession, effects: &[EffectId], key: &RowKey, value: ShownValue, opts: CommitOptions) -> CommitOutcome
    // F62, F63, F64. Compares with First; converts by Display; each effect gets its own copy through set_param; records undo; marks dirty.
    // A choice index refers to the merged list (F105). A sub-row writes the whole list through set_param (F55).
pub struct CommitOptions { gesture: Option<GestureToken> }
    // Same token: same undo step, for slider, strip and colour panel gestures (F32, FIX).
pub enum CommitOutcome { NoChange, Committed { undo_label: String } }
pub fn list_add(s, effects, key) -> CommitOutcome          // F56
pub fn list_remove(s, effects, key, index) -> CommitOutcome // F57; refuses at min items
pub fn list_move(s, effects, key, from, to) -> CommitOutcome // F59; PropertyItem.TryMoveItem semantics
pub fn can_remove(s, effects, key, selected: Option<u32>) -> bool
pub fn can_move(s, effects, key, selected: Option<u32>, delta: i32) -> bool
pub fn discrete_colors(s, effects, key) -> Vec<Color>     // F102, F51; empty means free colour
```

Delays: the 300 ms and timer delays of F24 and F32 live in the Swift model, which flushes them by F32's flush rule before any other panel call, so Rust never sees a commit for a selection that has gone.

Undo: use the existing `UndoAction::ParamsChanged { label, params }` (`crates/vixen-session/src/undo.rs:93-95`, description already `{n} Effect(s) {label} modified`), one action per commit or gesture. It restores whole `Params`, where upstream sets one property through its setter. The visible difference: a setter's side effects on other properties are undone as well, which is what a user expects. FIX, recorded here rather than in the register because it isn't a gesture.

### Inline gestures, in the same module, pure

```
pub fn curve_strip_points(points) -> Vec<(f32,f32)>        // TransformPoint, for drawing (F38)
pub struct CurveGesture                                    // Held by Swift for one press.
pub fn curve_mouse_down(curve: &Curve, pos: StripPos, mods: Mods) -> Option<CurveGesture>  // F39-F43 capture rule; None means "may start a value drag"
pub fn curve_mouse_move(g: &mut CurveGesture, pos: StripPos, mods: Mods) -> GesturePreview  // Preview points, tooltip text; no commit
pub fn curve_mouse_up(g: CurveGesture, pos: StripPos, mods: Mods) -> Option<Curve>          // F40, F41, F43 at release
pub fn curve_hover(curve: &Curve, pos: StripPos, mods: Mods, over_handle: bool) -> Cursor   // F39, F42
pub fn curve_level_from_handle(pos: f64) -> Curve                                           // F42
pub struct GradientHandle { position: f64, stops: Vec<u32>, color: Color }
pub fn gradient_handles(g: &ColorGradient) -> Vec<GradientHandle>                           // Stops at one position share a handle (F44)
pub fn gradient_add_stop(g: &ColorGradient, pos: f64, discrete_first: Option<Color>) -> ColorGradient // F45
pub fn gradient_move_handle(g: &ColorGradient, stops: &[u32], pos: f64) -> ColorGradient    // F46
pub fn gradient_delete_handle(g: &ColorGradient, stops: &[u32]) -> ColorGradient            // F47
pub fn gradient_set_handle_colors(g: &ColorGradient, stops: &[u32], colors: &[Color]) -> ColorGradient // F48
pub fn gradient_strip_bands(g: &ColorGradient, discrete: bool) -> Vec<StripBand>             // Blends to draw, F91; one band when not discrete
pub fn drop_value(s, effects, key, payload: DragPayload) -> CommitOutcome                    // F68
pub fn can_drop(s, effects, key, payload: &DragPayload) -> bool                              // F68's table and discrete rule
pub fn drag_payload(s, effects, key) -> Option<DragPayload>                                  // F67
pub enum DragPayload { Curve(Curve), Gradient(ColorGradient), Color(Color) }                 // Serialised with serde for the pasteboard
```

### Dialogs: `crates/vixen-session/src/effect_editor/dialogs.rs`

```
pub struct CurveEditSession     // The dialog's copy, its mode row of F71 and picked point
pub fn curve_edit_open(curve: &Curve, library_name: Option<&str>) -> CurveEditSession
impl CurveEditSession { mode(); hit_test(x, y, scale); press(..); drag(..); release(..); set_point(x, y); invert(); reverse(); draw_begin(); draw_move(..); draw_end(..); apply_function(text, step) -> Result<(), String>; load_from_library(name); unlink(); result() -> Curve }
pub struct GradientEditSession  // Same for F88 to F92: faders, focus, discrete, read-only
impl GradientEditSession { faders(); press(x, y); drag(x, y); release(); add_at(x); delete_selected(); set_position(percent); set_selected_colors(colors); focus_position(i) -> Option<f64>; load_from_library(name); unlink(); result() -> ColorGradient }
pub fn evaluate_curve_function(expression: &str, step: u32) -> Result<Vec<(f64, f64)>, String> // F75. crates/vixen-core; an NCalc-compatible expression subset
pub fn discrete_picker_size(n: u32, item_width: u32, threshold: u32) -> (u32, u32)            // F101
```

`vixen-core` additions: `ColorGradient::focus_position(i) -> f64`, `set_focus_position(i, pos)` (`ColorGradient.cs:1045-1088`); `ColorGradient::library_eq(&other) -> bool` for F94's equality, since its hand-written `PartialEq` compares all stored fields, title and gamma included (`crates/vixen-core/src/gradient.rs:76-85`) and upstream's returns equal for two links to one name. `Curve::library_eq` likewise (`Curve.cs:369-372`: note its equality includes the private library and referenced-curve objects, so two separately loaded linked curves are unequal unless both resolved the same entry; in practice treat as points and name and current flag). `Curve::inverted()`, `reversed()`, `flat_from(y)`.

### Libraries: `crates/vixen-session/src/libraries.rs`

```
pub struct Libraries { curves: CurveLibrary, gradients: GradientLibrary, favourites: Vec<Color> }   // On AppCore
pub fn add_curve(&mut self, name, curve) -> bool; pub fn remove_curve(&mut self, names: &[String]); pub fn reorder_curves(&mut self, names: &[String]); pub fn replace_curve(&mut self, name, curve);
pub fn import_curves(&mut self, path) -> Result<ImportCount, LibraryError>; pub fn export_curves(&self, path) -> Result<(), LibraryError>   // Same set for gradients, and .vfc for favourites
pub fn unique_import_name(existing, name) -> String      // F84: name, "name 2", "name 3"
pub fn library_drag_payload(&self, kind, name, link: bool) -> Option<DragPayload>  // F69, F83: a copy, not current, named only when linking; a blank name stays unlinked (F107)
pub fn save_libraries(&self, store) -> Task              // vixen_io::set_curve_library and module store save, off the main thread
pub fn resolve_links(session: &mut SequenceSession, libraries: &Libraries) -> Vec<EffectId>
    // F86. Walks every Curve and ColorGradient in every effect's Params, nested records included. Linked with entry: copy points or stops, mark dirty. Entry gone: clear the name, keep values. Called on load and after any library change.
pub fn changes() -> LibraryChangeEvents                  // Replaces CurvesChanged, GradientsChanged, ColorsChanged; one event per change
```

Addresses `docs/audit/editor-pass.md` butterfly-3: with `resolve_links` the stored copies are kept current, so `RenderCtx` needs no library.

### Effect defaults: `crates/vixen-io/src/effect_defaults.rs` and `crates/vixen-xml`

```
vixen_xml::nbfx::read(bytes) -> Result<XElement, XmlError>    // MC-NBFX binary XML with the WCF dictionary strings; no crate does this today
vixen_xml::nbfx::write(&XElement) -> Vec<u8>
pub struct EffectDefaultEntry { type_id, effect_name, data_model_type_name, saved_utc, payload_xml: XElement }
pub struct EffectDefaultsStore { load(paths) -> Self; has(type_id) -> bool; create_params(desc) -> Option<Params>; save(desc, params) -> Result<(), IoError>; clear(type_id) -> Result<bool, IoError>; summaries(); export(path, ids); import(path) -> (imported, overwritten); write_dump(path) }
pub fn scrub(params: &mut Params)     // Resets exclude_from_default members to the kind's default, in nested records too; leaves library names alone
```

If `DataModelTypeName` differs from `EffectDescriptor::data_ns + data_type`, ignore the entry (`Svc/EffectDefaultsService.cs:109-113`). Entries for effects not known are kept and rewritten (`:432-436`). The owner has used this feature only if it exists in their version; test with `target/show-copy`, since **if the real file is absent nothing is lost, and NBFX is needed only to read files from Windows**. Writing NBFX keeps export files exchangeable with the Windows app.

### Preview: `crates/vixen-session/src/playback.rs`

```
pub fn effect_preview_play(&self, effects: &[EffectId]) -> Option<TimeSpan>   // F6 steps 1 to 3; returns the loop length, None if empty or zero
pub fn effect_preview_stop(&self)                                              // F6: Clear(), or Clear(false)
```
The loop timer: in Rust on the playback thread, so a stalled main thread can't starve or double it. `effect_preview_set_state(on: bool)`, `effect_preview_set_selection(effects)`; the sequencer's play and export call `pause` and `resume`.

### FFI

`EffectEditorApi`, an object on the `SequencerSession`, exposes the functions above, with `RowKey`, `RowControl` and friends as `uniffi::Enum` and `Record`. Bulk: one `panel_model` call per selection and one `panel_delta` per commit. Drawing data is plain arrays. No per-row or per-draw FFI.

## SwiftUI and AppKit structure

Module `app/Sources/VixenEffectEditor`, exporting `EffectEditorPanel` (a view for the sequencer's right tabs), `CurveLibraryPanel`, `GradientLibraryPanel`, `ColorLibraryPanel`, and `EffectEditorTestScene` for building alone (`docs/UI_GUIDE.md`). Reusable pieces in `VixenKit` under new files: `CurveStripView`, `GradientStripView`, `CurveGraphView`, `GradientBarView`, `SwatchView`, `DiscreteColorPicker`, and small row views. `VixenSequencer` assembles the panels. Model: `@Observable final class EffectEditorModel`, holding `PanelModel` values and pending gesture state; no business logic.

| Upstream | Mac | Region kept |
|---|---|---|
| Dock panel `FormEffectEditor` | Right tab in the sequencer's panel group, detachable, toggled from View | Same tab |
| `EffectPropertyEditorGrid` | SwiftUI `ScrollView` with `LazyVStack` of `DisclosureGroup`s, one per category, starting folded when `collapsed_by_default` says so (F13). Rows lazily built. A panel has at most a few hundred rows (a large effect with record lists), so this is within SwiftUI's comfort; **strips and sliders inside are AppKit** | Same |
| "Effect Preview" bar | `Toggle(.switch)` with label, leading, fixed above the scroll view | Top |
| Header bar | `HStack`: name centred and bold, then SF Symbols `square.and.arrow.down` for Save as Default, `arrow.counterclockwise` for Reset to Built-in, `questionmark.circle` for help, each with `.help` and the original tooltip | Under the bar |
| Name column | `Grid` per panel, with a shared width from a `PreferenceKey`, right aligned, `truncationMode(.tail)` | Same |
| Information box | Grouped `VStack` with `Link("More Info", destination:)`, last in the scroll | End of list |
| CheckBox | `Toggle(.checkbox)` | Value column |
| ComboBox, EnumDropdown | `Picker(.menu)`; blank selection shown as an empty title | Value column |
| IntegerEditor, DoubleEditor | Representable `NSTextField` plus a small `NSView` drag strip (F24), to control commit and focus exactly | Value column |
| Slider editors | Representable `NSSlider`, `isContinuous`, ticks when snap; value in shown units | Value column |
| Text, label | `TextField`, `Text` | Value column |
| Path, Font | Read-only `TextField`, `...` button; `NSOpenPanel`, `NSFontPanel` | Value column |
| Colour row | `SwatchView` in AppKit, double-click and drag, then text | Value column |
| InlineCurveEditor | `CurveStripView`: `NSView` with Core Graphics: 32 pt strip, 10 pt level bar when ⇧; `NSDraggingSource`, `NSDraggingDestination`; forwards events to Rust gesture functions | Value column |
| InlineGradientEditor | `GradientStripView`: `CGGradient`, 10 pt band with handle layers | Value column |
| GradientLevelPair | Curve strip then gradient strip | Full width below header |
| CollectionView | `VStack` in the same scroll view with a selection highlight; buttons in `HStack`; colour list uses a `WrapLayout`. Not `List`: rows are few, and the panel scrolls as one | Full width below header |
| Curve Editor | Sheet on the sequencer window, min 591x784 equivalent, resizable. `CurveGraphView` in AppKit | Graph top, Curve group, Library group, OK and Cancel bottom right, instructions bottom left |
| Function Generator | Nested sheet | |
| Gradient Editor | Sheet. `GradientBarView` in AppKit | Bar top, Selected Stop row, Library group, buttons |
| Library selectors | Sheet with a `LazyVGrid` of thumbnails | Grid, then New, Edit, Delete left, OK, Cancel right |
| Library windows | Tabs with toolbar `HStack` of SF Symbol buttons, `pencil`, `plus`, `trash`, `square.and.arrow.up`, `square.and.arrow.down`, and original tooltips; `LazyVGrid`, with thumbnails cached as `NSImage`; link check box at the bottom | Same |
| Discrete pickers | Sheets, `LazyVGrid` of swatches, with check boxes in the multiple kind | |
| MessageBoxForm | `.alert`, `.confirmationDialog` with verbs (F108) | |
| TextDialog | Name sheet: message, `TextField`, Cancel and OK; the text stays across a retry (F107) | |
| ValueUpDown | `NSTextField` plus `NSStepper`, with the tracker as a small popover holding a vertical `NSSlider` (F89) | Gradient Editor, Selected Stop row |
| Colour picker | `NSColorPanel` with accessory Quick Picks | |
| Drag and drop | `NSDraggingSource` and pasteboard types in AppKit views; `.onDrop` on the Library grids | |
| Undo | The window's `UndoManager` registering Rust undo and redo, with `setActionName` from `CommitOutcome.undo_label` | Edit menu |

Key handling: ⌘-modified presses on strips are read from the event, not from global state. Forward keys per F66. VoiceOver: strips expose a value description, e.g. "Curve, 3 points" or "Library: name", with custom actions Edit, Invert, Reverse. ⌘ replaces upstream Ctrl and ⌥ replaces Alt throughout; ⌃-click is the context menu and isn't a gesture here.

## Performance hazards

1. **Whole grid rebuilt per selection.** `DoReload` runs even for two effects of one type; `SwapSelectedObject`'s update-in-place is commented out (`EE/EffectPropertyEditorGrid.cs:628-635`). Each row builds WPF templates, and each load runs `MetadataRepository.Clear()` and reflection for all properties (`:909`). Selecting through a sequence is slow, hence the 200 ms debounce. Mac: keep the debounce; `panel_model` is table-driven and cheap; SwiftUI identity by `RowKey` reuses views when two effects share a type.
2. **Bitmaps for strips.** Each strip encodes a PNG to memory and decodes it on every value change (`EE/Converters/BitmapImageConverter.cs:27-44`), redone per mouse move during a drag (`BaseInlineCurveEditor.cs:584-590`). A 300x30 bitmap is stretched to the row, so it blurs. Mac: draw points and gradients directly at real size. A move redraws just that view.
3. **Mouse-move hit testing in curve units across the point list**, trivial, but the upstream move handler also catches a recurring null error (`BaseInlineCurveEditor.cs:358-364`). Mac: gestures are pure functions on a held copy.
4. **Commit during a drag.** Sliders push through a 300 ms delay, and int fields through a timer, so a drag that pauses renders the effect and rebuilds thumbnails repeatedly (`EE/Themes/SliderEditor.xaml:12`, `EE/Controls/RepeatEditor.cs:74-112`). Each commit queues a render, or starts a task when the queue is busy (`Vixen.Common/Controls/TimeLineControl/Grid.cs:2890-2906`), and re-encodes the strip bitmap on the UI thread (hazard 2). Mac: commits at most at the same cadence; rendering is off the main thread (`SequenceSession::render_all_dirty`), newest wins per effect. A drag never blocks.
5. **List rows rebuilt on any item change.** `LoadCollectionValues` recreates all `CollectionItemValue`s and clones each record for every commit (`EE/PropertyItemValue.cs:92-116`, `:358-365`, `EE/Internal/CollectionItemValue.cs:36-44`); with sub-editors, editing a Wave collapses focus and recreates controls. Mac: identity by `RowKey`, `panel_delta`, no rebuild.
6. **Multi-edit copies.** Each commit clones per effect by `ICloneable`, `InstanceDescriptor`, string, or a **BinaryFormatter round trip** (`MergedPropertyDescriptor.cs:127-175`), and undo clones every list (`EE/PropertyItem.cs:661-676`), times up to 75 effects. Mac: Rust `Clone`, off the main thread if over a threshold. Thumbnails: only for effects whose value changed.
7. **`CheckLibraryReference` on every points read** (`Curve.cs:73-81`): a lookup per read inside render loops. Mac: `resolve_links` once per library change; points are plain data.
8. **Library lists rebuild every thumbnail on any change and on each ⌘-scroll tick** (`Form_CurveLibrary.cs:172-207`, `:603-612`), and the host's `CurveLibraryChanged` rebuilds toolbars too. Mac: `LazyVGrid` with a thumbnail cache keyed by content hash and size; scale by a CALayer transform during the gesture and re-rasterise when it ends. Library save off the main thread: upstream's `SaveModuleConfigAsync` writes the whole module store per change.
9. **Colour list events fire twice per change** (sequencer-shell hazard 26). One event.
10. **Preview timer on the wrong thread.** `System.Timers.Timer` posts `BeginInvoke(PreviewPlay)`; a blocked UI thread piles them up (`FormEffectEditor.cs:243-254`). Mac: loop in Rust, F6.
11. **ToggleSwitch's storyboard and effects.** Dropped.
12. **Render triggered by reading images:** `IsDiscrete` setter redraws (`BaseInlineGradientEditor.cs:245-253`) and `OnComponentChanged` rereads valid colours on each row for each row load. `GetValidColors` walks targets per row. Mac: `discrete_colors` once per selection in `PanelModel`.
13. **SwiftUI traps.** No `GeometryReader` per row: strips are AppKit views that lay out by themselves. Don't hold a whole `PanelModel` in a single `@Observable` property that invalidates every row: split rows into observable objects, or per-row `@State` keyed by `RowKey`. Curve and gradient dialogs must never call Rust per draw: gesture calls return the points to draw.
14. **A spin wait at every selection change.** Once the preview context exists, a selection change stops the preview first, and `Clear()` busy-waits on the UI thread for the render side to finish its reset, for up to 1 s (`Vixen.Core/Execution/Context/LiveContext.cs:62-85`, `FormEffectEditor.cs:233-240`, `:283-298`). Each click on an effect pays this before the panel can draw, which is a likely part of what the owner remembers as choppy with the switch on. Not measured. Mac: `effect_preview_stop` only posts to the playback thread and returns at once; never wait on it from the main thread.
15. **Idle refresh rebuilds lists a second time.** After a commit the grid queues a refresh at idle priority, which raises both `StandardValues` and `PropertyValue` on the row (`EE/EffectPropertyEditorGrid.cs:892-904`, `EE/PropertyItem.cs:190-195`), and `PropertyValue` reloads a list row (`EE/PropertyItemValue.cs:356-365`), so hazard 5's rebuild runs twice per commit. A `TargetNodes` change refreshes every row this way. Mac: one `panel_delta` per commit.

## Tests worth writing

**Upstream tests in this area:**

- `Vixen.Tests/EffectEditor/PropertyItemCollectionMoveTests.cs`: `TryMoveItem` moves index 1 to 0 and 1 to 2; keeps identity; rejects `(-1,0) (0,-1) (1,1) (3,0) (0,3) (5,0) (0,5)` on a 3-list unchanged; rejects an empty list and a one-item list.
- `Vixen.Tests/EffectDefaults/EffectDefaultsServiceTests.cs`: store round trip; capture doesn't change live data; `BuildExportStore` filtering and skipping unknown ids; `MergeEntries` counts Imported 1, Overwritten 1, keeps the third; diagnostic dump is indented, contains type id, name, `2026-01-02T03:04:05`, `LevelCurve`, `ColorGradient` and no base64 of the payload; an uninstalled effect's entry says "not currently installed".
- `Vixen.Tests/EffectDefaults/EffectDefaultScrubberTests.cs`: excluded member reset at the top level, keeps others; reset inside a list; linked curve keeps its name and points; terminates on a cycle.
- `Vixen.Tests/Sequencer/LibraryDragPayloadFactoryTests.cs`: linked payload has the name and `IsCurrent = false`, an independent copy; unlinked has an empty name; gradient cases; colour passes through; null throws; unsupported type throws; empty or space name with link throws.
- `Vixen.Tests/Common/WPFCommon/DragDropManagerTests.cs`: `ResolveDropEffect`, `ShouldStartDrag`.
- `Vixen.Tests/Common/XYZTests.cs`: `XYZ.ClipValue` in range, below and above; NaN and negative infinity give the minimum, positive infinity the maximum. For `vixen_core::color`, which the colour rows use.
- `Vixen.Tests/Effects/LipSyncMarkCollectionNameConverterTests.cs`: Phoneme collections and the selected one are offered, in sequence order (F105).
- `Vixen.Tests/Effects/WaveAndLiquidMarkCollectionSelectionTests.cs`: Use Marks on a Waveform or Emitter, and Decaying Sine with marks already on, select the first collection when none is selected (F55); renaming or adding a collection refreshes the selector without dirtying the effect. `BaseEffectMarkCollectionSelectionTests.cs`, `MarkCollectionSelectionServiceTests.cs` pin the same selector rules. `FireTargetNodeSelectionTests.cs`, `WipeTargetNodeSelectionTests.cs`: the depth converters exclude zero and the maximum depth (F28). These were read by test name only; the effect specs own what else they cover.

**Rust tests to add**, expected values read from the code:

*Panel:*
1. Exact order for Pulse: from its table, `visible_params_for`; categories by order then name, rows by order then label.
2. Selection of 75 gives rows; 76 gives none and message `Multi-Edit is limited to < 75 Effects`.
3. Two Pulses: `LevelCurve` row shows First's; an enum row differing has `selected: None`; Check shows First's.
4. Pulse and Spin: `ColorGradient` common; rows with different categories go under `Misc`, order -1, first.
5. Different `[PropertyOrder]` falls to 0, same category sorted by label.
6. `commit` with First's current value on a mixed row: `NoChange`, no undo, others unchanged.
7. `commit` on an enum row from blank with First's value: applies to all (F62).
8. `list_add` on two effects with different palettes: both end with First's list plus the new item.

*Display (F33):*
9. Gain: stored 3 shows 30. Commit 37: stored 3, shows 30, no undo step (FIX). Commit 47: stored 4, shows 40.
10. Range: stored 10 shows 10. Commit 15: stored 5.
11. ScrollSpeed: stored 10 shows 40. Commit 50: stored 0.

*Numeric rules:*
12. Int: "12.7" is rejected as unparseable (NumberStyles.Integer); " -5 " on min 0 becomes 0; empty reverts; 300 on byte becomes 255; `minimum > maximum` gives `maximum = minimum`.
13. Double: rounded to precision in Banker's rounding: 2.5 with precision 0 is 2, 3.5 is 4; clamped first.
14. Strip drag: `n = round(len)`; `dx > dy` positive. `(5,-1)` increases, `(-1,5)` decreases, `(3,3)` decreases.
15. Percent write-back: slider 150 gives `f = 1`, not clamped by accident; -5 gives 1.
16. Level tooltip: stored `0.29` gives 28 (`(int)(0.29 * 100)`, floating truncation); `Convert.ToInt32` of a slider 2.5 is 2.

*Curve gestures:*
17. Mapping: width 300, press at (150,16): x 50, y 50.
18. Near rule: point (0,0), press at x 7.9% and y 0: captured; press at x 8.1%: not captured, may drag value. Press at y 3 pt (9.4 units): not captured.
19. Exact ⇧ and exact ⌘ capture anywhere; ⇧⌘ doesn't capture but inverts at release.
20. Move: rounding away from zero: 2.5 gives 3; clamps; order preserved, no sort.
21. Add: `[(0,0),(100,100)]` ⌘-click at (50,20) gives `[(0,0),(50,20),(100,100)]`.
22. ⌘-drag: `[(0,0),(100,100)]`, press at (50,20), first move past 2 pt at (51,21), last move (60,70): release gives `[(0,0),(51,21),(100,100)]`, added where the drag began, which is that first move and not the press; nothing moved. ⇧ released mid-drag: the last move decides between flat level and moved point (F39).
23. Remove refuses at 2 points.
24. Invert `[(0,0),(100,100)]` gives `[(0,100),(100,0)]`. Reverse gives `[(0,100),(100,0)]`, sorted by X. ⇧⌘⌥ is reverse.
25. Level: ⇧-drag at y 33.4 gives `[(0,33),(100,33)]`. Handle at 0.335 gives 34 (`Round(33.5, AwayFromZero)`).
26. Linked curve: all gestures give `None`.
27. Thumbnail `TransformPoint`: `(50,50)` in 300x30: `X = (int)(50 * 2.99) = 149`, `Y = (int)(50 * 0.3) = 15`.

*Gradient:*
28. Default gradient: one white stop at 0, focus .5; solid colour same. Handles: stops at one position share one.
29. Add at 0.5 on black-to-white gives a grey stop; discrete with valid `[red, green]` gives red.
30. Delete under a shared handle removes both; last stop allowed, blend gives transparent at 0 and 1.
31. Discrete picker result: none means delete; two ticked give two stops at the same position.
32. Reversal of 1 stop: no change, no commit (`value.Colors.Count > 1`).
33. Equality: two links to the same name are equal with different stops; linked versus unlinked unequal.
34. Focus: stops at 0 and 1, focus position 0.25 sets focus 0.25, `focus_position` of the first stop is `None`, zero-width gap gives focus .5.

*Drop rules:* 35. the whole table in F68, and the discrete rule: red onto a gradient on `[red, green]` targets accepted, blue refused, gradient with one foreign colour refused.

*Libraries:* 36. `add_curve` over an existing name flags the old as not current, and `resolve_links` updates a linked effect and marks it dirty. 37. Removing unlinks and keeps the points. 38. `unique_import_name("A", ["A", "A 2"])` is `"A 3"`. 39. Round trip `.vcl`, `.vgl`, `.vfc` against files written by C#; reorder writes that order. 40. Picture's path: an absolute path copies the file, stores the bare name; a relative one is stored as given.

*Function:* 41. `Pow(x/100, 2.2) * 80` at step 5: 21 points, x 0..100, y(100) = 80, y(50) = 80 * 0.5^2.2. Step 30: x 0, 30, 60, 90, 100. Values over 100 clamp, negatives clamp. Function names case-insensitive, `pow` as `Pow`; parameters are exactly `x` and `Pi`; uppercase `X`/`PI` fail (confirmed against NCalc2, F75). Empty and syntax errors return their text. Step 0: error, no hang.

*Defaults:* 42. All upstream tests above. 43. Scrub resets `MarkCollectionId` in `EmitterData` and `WaveformData` rows. 44. `.vfd` from a Windows run reads back; a rewritten one reads in C# (golden).

*Panel metadata:* 45. Video's Advanced Settings has `collapsed_by_default`; expanded, it stays expanded on selecting another Video (F13). 46. Picture's tile list has 12 items, starting with Blue Glow Dots, without `None`; a stored `None` gives `selected: None` and survives a save (F29). 47. All selected lists empty: `list_add` gives `NoChange`; First empty and another not: it adds (F56). 48. `check_effect_metadata.py` fails if a row's `selection_merge` or `blank_when_mixed` disagrees with its C# converter.

*Choice lists (F105):* 49. Chase on depth 3 and depth 5 targets lists 0, 1, 2 and shows blank when the two differ. 50. Fire on the same targets lists 1, shows First's when they differ; Wipe the same, also when it's First in a mix. 51. Two States with items `[a, b]` and `[b, c]` list `<All>, a, b, c`; choosing `c` selects it on the second and leaves the first's selection alone. 52. Choosing a State definition only the second effect has changes the second alone; a Frost whose fixture lacks the chosen function stores the name as given. 53. A `Bool` row with differing words shows `True`, `False`.

*Rows and row setters:* 54. Whirl row to Meteor sets that row's `ColorMode` to `GradientOverTime`, other rows unchanged. 55. Wave row to Snake sets `PrimeWave`; Emitter to Use Marks selects the first collection. 56. Custom State: choosing an item sets its id and its colour, in one undo step (F55, FIX), and undo restores both; `<None>` in Cycle keeps the colour; a label not offered changes nothing; Render Source to Custom on an empty list adds one row; Playback to Default drops duplicates. 57. Pair row: OK unchanged and Cancel each give `NoChange` and no undo step (F53, FIX).

*Gesture fixes:* 58. Level handle moved to 0.2 then 0.4 previews each, with no need for two moves at one x (F42, FIX). 59. `GradientEditSession`: press and release on a fader with no move, and a right press anywhere, leave `result()` equal to the start (F88, FIX).

*Libraries and preview:* 60. `replace_curve` keeps the entry's place in the order; a new name appends (F86). 61. `library_drag_payload` with link on and a name of spaces gives an unlinked copy; a normal name is linked (F107). 62. Effects at 0..10 s and 1..2 s: `effect_preview_play` returns 2 s, and the first keeps playing past the loop boundary (F6). 63. `effect_preview_stop` returns without waiting for the playback thread (hazard 14).

*Swift model:* 64. A pending slider value is committed to the old selection before `panel_model` runs for the new one, and before a dialog opens or an undo runs (F32).

*UI harness:* snapshot Pulse selected, a mixed selection, the two dialogs in each mode, a discrete target's pickers, Wave's list, and read the PNGs as `docs/UI_GUIDE.md` asks.

## Windows-only omissions

| Upstream | Why |
|---|---|
| `SendMessage(LVM_SETICONSPACING)` list spacing (`Form_CurveLibrary.cs:29-42` and the other two) | Grid spacing is a layout value on the Mac; same numbers |
| `ScalingTools.GetScaleFactor` | Points on the Mac are already scaled |
| `MultiEditLimit` in `app.config` | Rust constant or setting; no `app.config` on the Mac |
| `ThemeUpdateControls`, `ThemeGroupBoxRenderer`, `ThemeToolStripRenderer`, `WPFCommon/Theme/*` | Native dark controls per `docs/UI_GUIDE.md`; canvases use `Theme` |
| `ToggleSwitch`, `ClipBorder`, `GeometryHelper`, `Reveal`, `IndexingStackPanel`, `SliderEditorBase` | Look only, or dead: `Reveal`, `IndexingStackPanel` and `SliderEditorBase` are referenced nowhere in `EE/` or in effects |
| `AlphabeticalLayout` | Selected by nothing: `Layout = new CategorizedLayout()` always (`EE/EffectPropertyEditorGrid.cs:1186`) |
| Filter box, `PropertyFilter`, filter commands | The panel hides it, and templates lack it (F20) |
| `ShowAttachedProperties`, `PropertyDisplayMode`, `FontStretch`, `FontStyle` and `FontWeight` decorators, `CultureInvariantTypes`, `PropertyDescriptor`, `TypeDescriptor`, `DependencyPropertyDescriptor`, `MetadataRepository`, `AttributesContainer`, `ObjectServices`, `KnownTypes` | WPF and reflection plumbing, replaced by the tables |
| `CategoryEditor` and category-level editors, `ComplexPropertyEditor`, `ExtendedEditor` and `ClosePropertyTab` commands, emitter curve commands (`PropertyEditorCommands` `ShowEmitter*`) | Never used by a template or property. The commands are declared but nothing binds them (`EE/Input/PropertyEditorCommands.cs:52-86`) |
| `ShowReadOnlyProperties`, `Reload`, `ResetFilter` and their commands | No UI bound; the panel fixes read-only on |
| `PropertyEditingStarted`, `PropertyEditingFinished` events | Nobody subscribes ("not used by EffectPropertyEditorGrid control directly", `EE/EffectPropertyEditorGrid.cs:510`) |
| `CurrentDescription` | Nothing shows it (F66) |
| `DropPreviewAdorner` | Upstream gives it no visual (`GetVisualFeedback` returns null) |
| `BinaryFormatter` in `CopyValue` | Rust `Clone` |
| JSON in the data object (`DragDropUtils`) | A pasteboard payload from Rust serves the same purpose; can't cross apps anyway |
| `GDI32.GetScreenPixel`, `ShiftKeyFilter`, `IgnoreMouseWheelBehavior` | System colour panel eyedropper; event modifiers; a single scroll view |
| `%AppData%`, Windows path syntax | `paths.rs` and Application Support; stored effect file names are bare already (F34) |
| Windows Explorer file drag to library | Not in this area |
| `CollectionPropertyDescriptor`, `CollectionItems`, `DebugConverter`, `NullValueConverter`, `CurveToDoubleConverter` and other unused converters | Dead code |
| `IntegerEditor` and `DoubleEditor` `Increase`, `Decrease` commands; `PropertyItem.ClearValue`, `CanClearValue`, `RefreshDescriptors`, `IsDefaultValue` | No UI reaches them |
