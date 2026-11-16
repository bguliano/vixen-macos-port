# Custom prop editor, polygons, fixtures and states

Specialized editors: the Custom Prop Editor and its xModel import, the Polygon Editor used by Morph, the Intelligent Fixture wizard with its fixture library and editors, the moving head graphics, and the State property with its mapper.

## Scope

Upstream root: `upstream/Vixen/src`. Every path below is relative to it. Aliases used throughout:

- `CPE/` = `Vixen.Modules/App/CustomPropEditor/`
- `POLY/` = `Vixen.Modules/Editor/PolygonEditor/`
- `PMOD/` = `Vixen.Modules/App/Polygon/`
- `WIZ/` = `Vixen.Modules/Editor/FixtureWizard/`
- `FPE/` = `Vixen.Modules/Editor/FixturePropertyEditor/`
- `FGFX/` = `Vixen.Modules/Editor/FixtureGraphics/`
- `FIX/` = `Vixen.Modules/App/Fixture/`
- `FSM/` = `Vixen.Modules/App/FixtureSpecificationManager/`
- `MODEL/` = `Vixen.Modules/App/Modeling/`
- `STATE/` = `Vixen.Modules/Property/State/`
- `IFIX/` = `Vixen.Modules/Property/IntelligentFixture/`
- `TPL` = `Vixen.Application/Setup/ElementTemplates/IntelligentFixtureTemplate.cs`
- Local aliases, defined again where first used: `PEVM` = `POLY/ViewModels/PolygonEditorViewModel.cs` (section I), `WZ/` = `WIZ/Wizard/` (section J), `FV/` = `FPE/Views/` and `FM/` = `FPE/ViewModels/` (section L), `SM` = `STATE/Setup/ViewModels/StateMapperViewModel.cs` and `SV` = `STATE/Setup/Views/StateMapperView.xaml` (section N). A citation that starts with `:` refers to the file named last.

**Read in full:** all 368 text files listed below, every line. That is 53,580 lines by `wc -l`, including the project files, `FodyWeavers.xml` and the third-party notice. Line counts follow each file name.

**`CPE/`** (158 files, 18,428 lines)

- `CPE/` (root): `CustomPropEditor.csproj` 80, `CustomPropEditorData.cs` 64, `CustomPropEditorDescriptor.cs` 43, `CustomPropEditorModule.cs` 83
- `CPE/Adorners/`: `ResizeAdorner.cs` 445, `RubberbandAdorner.cs` 53
- `CPE/BackgroundImageScaling/`: `BackgroundImageScaleCalculator.cs` 102, `BackgroundImageScaleOptions.cs` 13, `BackgroundImageScaleUnit.cs` 18
- `CPE/Controls/`: `IGroupable.cs` 9, `PropDesigner.cs` 690, `SliderMenuItem.xaml` 30, `SliderMenuItem.xaml.cs` 417
- `CPE/Converters/`: `BoolToGridRowHeightConverter.cs` 20, `BrushToColorConverter.cs` 41, `BulbTypeConverter.cs` 67, `LightNodeToPathConverter.cs` 28, `PointMappingConverter.cs` 39, `RootNodeToCollectionMapping.cs` 55
- `CPE/Extensions/`: `Extensions.cs` 33
- `CPE/Import/`: `IModelImport.cs` 9
- `CPE/Import/XLights/`: `IXModelElementParser.cs` 13, `IXModelSelectionService.cs` 7, `ModelNode.cs` 12, `ModelType.cs` 7, `SubModel.cs` 23, `XModelCandidate.cs` 6, `XModelChildElementImporter.cs` 261, `XModelCommonConfiguration.cs` 15, `XModelCommonConfigurationParser.cs` 39, `XModelCoordinateScale.cs` 74, `XModelElementMetadata.cs` 43, `XModelGeneratedGroup.cs` 11, `XModelImport.cs` 188, `XModelImportModel.cs` 90, `XModelInventoryImporter.cs` 30, `XModelInventoryMapper.cs` 158, `XModelParsedModel.cs` 14, `XModelPropAssembler.cs` 362, `XModelSelectionItem.cs` 4, `XModelSelectionService.cs` 30
- `CPE/Import/XLights/Circle/`: `CircleXModelConfiguration.cs` 31, `CircleXModelConfigurationParser.cs` 194, `CircleXModelElementParser.cs` 57, `CircleXModelGroupGenerator.cs` 86, `CircleXModelNodeGenerator.cs` 69, `CircleXModelRing.cs` 11
- `CPE/Import/XLights/Custom/`: `CustomXModelElementParser.cs` 121, `CustomXModelNodeParser.cs` 98, `CustomXModelParseResult.cs` 29, `CustomXModelSource.cs` 9, `CustomXModelSourceResolver.cs` 62
- `CPE/Import/XLights/Faces/`: `FaceInfo.cs` 45, `FaceItem.cs` 17
- `CPE/Import/XLights/Ranges/`: `NodeRange.cs` 15, `Range.cs` 8, `RangeGroup.cs` 11
- `CPE/Import/XLights/States/`: `StateInfo.cs` 22, `StateItem.cs` 27, `XLightsStateNameNormalizer.cs` 33
- `CPE/Model/`: `ColorMode.cs` 16, `Configuration.cs` 61, `ElementModel.cs` 498, `ElementModelType.cs` 33, `ElementType.cs` 16, `FaceComponent.cs` 35, `FaceDefinition.cs` 40, `InformationMetadata.cs` 20, `Light.cs` 90, `PhysicalMetadata.cs` 92, `Prop.cs` 266, `StateDefinition.cs` 88, `StateDefinitionModel.cs` 116, `StateItemModel.cs` 96, `VendorMetadata.cs` 72
- `CPE/Model/ExternalVendorInventory/`: `Category.cs` 18, `Model.cs` 53, `Vendor.cs` 33, `VendorLink.cs` 10, `Wiring.cs` 21, `XModelInventory.cs` 20
- `CPE/Model/InternalVendorInventory/`: `Category.cs` 73, `ModelInventory.cs` 70, `ModelLink.cs` 90, `ModelType.cs` 8, `Product.cs` 235, `Vendor.cs` 130, `WebLink.cs` 44
- `CPE/Persistence/`: `AtomicPropFileWriter.cs` 43, `IPropDocumentMapper.cs` 11, `IPropFileReader.cs` 6, `IPropImageCodec.cs` 10, `LegacyLiteDbPropReader.cs` 122, `PropDocumentMapper.cs` 115, `PropDocumentValidator.cs` 84, `PropFileReadResult.cs` 6, `PropFileSourceFormat.cs` 7, `PropPackageReader.cs` 87, `PropPackageWriter.cs` 59, `PropPersistenceException.cs` 12, `WpfPropImageCodec.cs` 30
- `CPE/Persistence/Documents/`: `CustomPropJsonSerializerContext.cs` 17, `ElementDocument.cs` 15, `MetadataDocuments.cs` 26, `PropDocument.cs` 17, `PropPackageDocument.cs` 14, `SupportingDocuments.cs` 38
- `CPE/Persistence/Legacy/LiteDb5021/`: `LegacyBsonDocumentReader.cs` 183, `LegacyLiteDbRawDocument.cs` 6, `LegacyLiteDbRawReaderPrototype.cs` 291, `THIRD-PARTY-NOTICES.md` 33
- `CPE/Services/`: `ConfigurationService.cs` 27, `CustomPropStateDataMapper.cs` 99, `CustomPropStateMigrationService.cs` 106, `ElementModelLookUpService.cs` 79, `IPropModelPersistenceService.cs` 26, `PropModelPersistenceService.cs` 67, `PropModelServices.cs` 501, `PropStateModelResolver.cs` 57
- `CPE/Themes/`: `Theme.xaml` 39
- `CPE/ViewModels/`: `BackgroundImageScaleViewModel.cs` 440, `ConfigurationWindowViewModel.cs` 303, `DrawingPanelViewModel.cs` 820, `ElementModelViewModel.cs` 685, `ElementOrderViewModel.cs` 397, `ElementTreeViewModel.cs` 1211, `ElementViewModelCollection.cs` 16, `IExpandable.cs` 10, `ISelectable.cs` 8, `ISelectableExpander.cs` 7, `LightViewModel.cs` 209, `LightViewModelCollection.cs` 16, `PhysicalMetadataViewModel.cs` 216, `PropEditorViewModel.cs` 1576, `VendorInventoryWindowViewModel.cs` 295, `VendorMetadataViewModel.cs` 171, `XModelSelectionViewModel.cs` 60
- `CPE/ViewModels/State/`: `CustomPropStateDefinitionViewModel.cs` 278, `CustomPropStateItemViewModel.cs` 293, `StateDefinitionEditorViewModel.cs` 654
- `CPE/Views/`: `BackgroundImageScaleWindow.xaml` 79, `BackgroundImageScaleWindow.xaml.cs` 16, `ConfigurationWindow.xaml` 71, `ConfigurationWindow.xaml.cs` 10, `CustomPropEditorWindow.xaml` 351, `CustomPropEditorWindow.xaml.cs` 20, `ElementTree.xaml` 506, `ElementTree.xaml.cs` 15, `ModelInventoryTree.xaml` 59, `ModelInventoryTree.xaml.cs` 15, `PhysicalMetadataView.xaml` 47, `PhysicalMetadataView.xaml.cs` 10, `VendorInventoryWindow.xaml` 195, `VendorInventoryWindow.xaml.cs` 10, `VendorMetadataView.xaml` 29, `VendorMetadataView.xaml.cs` 10, `XModelSelectionView.xaml` 56, `XModelSelectionView.xaml.cs` 16

**`POLY/`** (31 files, 10,517 lines)

- `POLY/` (root): `PolygonEditor.csproj` 59
- `POLY/Adorners/`: `IResizeable.cs` 70, `ResizeAdorner.cs` 516, `RubberbandAdorner.cs` 53
- `POLY/Converters/`: `MathConverter.cs` 344, `PointCollectionConverter.cs` 63, `PolygonPointXConverter.cs` 64, `PolygonPointYConverter.cs` 69
- `POLY/Themes/`: `Theme.xaml` 51
- `POLY/ValidationRules/`: `XValidationRule.cs` 62, `YValidationRule.cs` 63
- `POLY/ViewModels/`: `EllipseViewModel.cs` 349, `LineSegmentViewModel.cs` 65, `LineViewModel.cs` 224, `PointBasedViewModel.cs` 80, `PolygonEditorViewModel.cs` 4942, `PolygonLineSegment.cs` 43, `PolygonPointViewModel.cs` 226, `PolygonSnapShotViewModel.cs` 272, `PolygonViewModel.cs` 452, `ShapeViewModel.cs` 400
- `POLY/Views/`: `BindingProxy.cs` 31, `PolygonCanvas.cs` 159, `PolygonControl.xaml` 616, `PolygonControl.xaml.cs` 461, `PolygonEditorView.xaml` 118, `PolygonEditorView.xaml.cs` 345, `PolygonTimeBar.xaml` 79, `PolygonTimeBar.xaml.cs` 56, `PolygonTimeBarCanvas.cs` 97, `SizeObserver.cs` 88

**`PMOD/`** (9 files, 861 lines)

- `PMOD/` (root): `Ellipse.cs` 205, `Line.cs` 42, `PointBasedShape.cs` 193, `Polygon.cs` 42, `Polygon.csproj` 8, `PolygonContainer.cs` 180, `PolygonEditorCapabilities.cs` 77, `PolygonPoint.cs` 48, `Shape.cs` 66

**`WIZ/`** (41 files, 3,145 lines)

- `WIZ/` (root): `FixtureWizard.csproj` 38, `FodyWeavers.xml` 6, `ModuleInitializer.cs` 11
- `WIZ/Wizard/`: `FixtureWizardNavigationController.cs` 110
- `WIZ/Wizard/Models/`: `AutomationWizardPage.cs` 90, `ColorSupportWizardPage.cs` 71, `DimmingCurveSelection.cs` 23, `DimmingCurveWizardPage.cs` 112, `EditProfileFunctionsWizardPage.cs` 22, `EditProfileWizardPage.cs` 22, `FixtureWizardPageBase.cs` 33, `GroupingWizardPage.cs` 74, `IntelligentFixtureWizard.cs` 222, `SelectProfileWizardPage.cs` 169, `SummaryWizardPageBase.cs` 50
- `WIZ/Wizard/Models/Interfaces/`: `IFixtureWizard.cs` 41
- `WIZ/Wizard/ViewModels/`: `AutomationWizardPageViewModel.cs` 137, `ColorSupportWizardPageViewModel.cs` 67, `DimmingCurveWizardPageViewModel.cs` 101, `EditProfileFunctionsWizardPageViewModel.cs` 90, `EditProfileWizardPageViewModel.cs` 152, `EditWizardPageViewModelBase.cs` 63, `FixtureWizardTreeItem.cs` 51, `GroupingWizardPageViewModel.cs` 296, `IIntelligentFixtureWizardPageViewModel.cs` 20, `SelectProfileWizardPageViewModel.cs` 348
- `WIZ/Wizard/Views/`: `AutomationWizardPageView.xaml` 65, `AutomationWizardPageView.xaml.cs` 35, `ColorSupportWizardPageView.xaml` 69, `ColorSupportWizardPageView.xaml.cs` 35, `DimmingCurveWizardPageView.xaml` 61, `DimmingCurveWizardPageView.xaml.cs` 36, `EditProfileFunctionsWizardPageView.xaml` 16, `EditProfileFunctionsWizardPageView.xaml.cs` 17, `EditProfileWizardPageView.xaml` 15, `EditProfileWizardPageView.xaml.cs` 25, `GroupingWizardPageView.xaml` 113, `GroupingWizardPageView.xaml.cs` 35, `SelectProfileWizardPageView.xaml` 102, `SelectProfileWizardPageView.xaml.cs` 34, `WizardUserControlBase.cs` 68

**`FPE/`** (41 files, 7,101 lines)

- `FPE/` (root): `FixturePropertyEditor.csproj` 53
- `FPE/ViewModels/`: `ChannelItemViewModel.cs` 304, `ColorWheelItemViewModel.cs` 257, `ColorWheelViewModel.cs` 200, `FixturePropertyEditorViewModel.cs` 547, `FixturePropertyEditorWindowViewModel.cs` 120, `FixturePropertyWindowViewModelBase.cs` 167, `FixtureViewModelBase.cs` 138, `FunctionItemViewModel.cs` 363, `FunctionTypeViewModel.cs` 1077, `FunctionTypeWindowViewModel.cs` 142, `IFixtureSaveable.cs` 20, `IndexedItemViewModel.cs` 216, `IndexedViewModel.cs` 467, `ItemsViewModel.cs` 334, `ItemViewModel.cs` 131, `PanTiltViewModel.cs` 252, `SelectFixtureSpecificationViewModel.cs` 78, `ZoomViewModel.cs` 86
- `FPE/Views/`: `BindingProxy.cs` 34, `ColorWheelView.xaml` 167, `ColorWheelView.xaml.cs` 93, `DataGridView.cs` 63, `FixturePropertyEditorView.xaml` 168, `FixturePropertyEditorView.xaml.cs` 101, `FixturePropertyEditorWindowView.xaml` 50, `FixturePropertyEditorWindowView.xaml.cs` 90, `FunctionTypeView.xaml` 157, `FunctionTypeView.xaml.cs` 283, `FunctionTypeWindowView.xaml` 67, `FunctionTypeWindowView.xaml.cs` 97, `IndexedView.xaml` 184, `IndexedView.xaml.cs` 95, `IRefreshGrid.cs` 18, `PanTiltView.xaml` 52, `PanTiltView.xaml.cs` 197, `SelectFixtureSpecificationView.xaml` 41, `SelectFixtureSpecificationView.xaml.cs` 79, `VixenDataGrid.cs` 60, `ZoomView.xaml` 32, `ZoomView.xaml.cs` 21

**`FGFX/`** (33 files, 5,542 lines)

- `FGFX/` (root): `FixtureGraphics.csproj` 32, `IMovingHead.cs` 132, `MovingHeadSettings.cs` 214
- `FGFX/Geometry/`: `IMovingHeadGeometryConstants.cs` 158, `MovingHeadGeometryConstants.cs` 284
- `FGFX/OpenGL/`: `IRenderMovingHeadOpenGL.cs` 40, `MovingHeadOpenGL.cs` 714, `MovingHeadRenderStrategy.cs` 383
- `FGFX/OpenGL/Primitives/`: `AttributeInfo.cs` 19, `MetaDataInfo.cs` 27, `UniformInfo.cs` 19
- `FGFX/OpenGL/Shaders/`: `ColorVolumeShader.cs` 99, `ColorVolumeShader1.cs` 30, `ColorVolumeShader2.cs` 30, `GrayVolumeShader.cs` 149, `ISpecifyVolumeIntensity.cs` 13, `IVolumeShader.cs` 29, `VolumeShader.cs` 465
- `FGFX/OpenGL/Volumes/`: `BeamCylinderWithEndCaps.cs` 67, `Cylinder.cs` 53, `CylinderBase.cs` 288, `CylinderWithEndCaps.cs` 173, `IRotatableCylinder.cs` 20, `ISpecifyVolumeColor.cs` 19, `ISpecifyVolumeTransparency.cs` 13, `IUpdateCylinder.cs` 23, `IVolume.cs` 65, `Rectangle.cs` 169, `RotatingCylinderWithEndCaps.cs` 74, `TaperedRectangle.cs` 215, `VolumeBase.cs` 136
- `FGFX/WPF/`: `BitmapSourceHelper.cs` 65, `MovingHeadWPF.cs` 1325

**`FIX/`** (9 files, 1,125 lines)

- `FIX/` (root): `Fixture.csproj` 12, `FixtureChannel.cs` 48, `FixtureColorWheel.cs` 148, `FixtureFunction.cs` 243, `FixtureIndex.cs` 45, `FixtureIndexBase.cs` 53, `FixtureItem.cs` 17, `FixtureRotationLimits.cs` 60, `FixtureSpecification.cs` 499

**`FSM/`** (3 files, 317 lines)

- `FSM/` (root): `FixtureSpecificationManager.cs` 266, `FixtureSpecificationManager.csproj` 18, `IFixtureSpecificationManager.cs` 33

**`MODEL/`** (7 files, 1,076 lines)

- `MODEL/` (root): `ElementModeling.cs` 128, `Modeling.csproj` 21, `WireDiagramExportOptions.cs` 60, `WireDiagramSvgBuilder.cs` 445
- `MODEL/ViewModels/`: `WireDiagramExportOptionsViewModel.cs` 239
- `MODEL/Views/`: `WireDiagramExportOptionsWindow.xaml` 123, `WireDiagramExportOptionsWindow.xaml.cs` 60

**`STATE/`** (31 files, 3,830 lines)

- `STATE/` (root): `State.csproj` 32, `StateData.cs` 86, `StateDefinitionData.cs` 118, `StateDescriptor.cs` 25, `StateItemData.cs` 91, `StateModule.cs` 110, `StateNamingRules.cs` 72, `StateSetupHelper.cs` 122
- `STATE/Setup/Models/`: `StateAssignmentTreeNode.cs` 214, `StateAssignmentTreeSelectionController.cs` 230, `StateBindableBase.cs` 43, `StateDefinition.cs` 93, `StateElementNodeSnapshot.cs` 69
- `STATE/Setup/Preview/`: `BroadcastStatePreviewPublisher.cs` 78, `IStatePreviewPublisher.cs` 30, `StatePreviewCoordinator.cs` 72, `StatePreviewPair.cs` 9
- `STATE/Setup/Services/`: `IStateColorPickerService.cs` 18, `IStateDefinitionDialogService.cs` 43, `IStateMapperDialogService.cs` 17, `StateColorPickerService.cs` 62, `StateDefinitionDialogService.cs` 97, `StateMapperDialogService.cs` 28
- `STATE/Setup/ViewModels/`: `StateDefinitionNameDialogViewModel.cs` 138, `StateDefinitionViewModel.cs` 105, `StateItemViewModel.cs` 174, `StateMapperViewModel.cs` 1260
- `STATE/Setup/Views/`: `StateDefinitionNameDialogView.xaml` 59, `StateDefinitionNameDialogView.xaml.cs` 37, `StateMapperView.xaml` 280, `StateMapperView.xaml.cs` 18

**`IFIX/`** (4 files, 261 lines)

- `IFIX/` (root): `IntelligentFixture.csproj` 21, `IntelligentFixtureData.cs` 60, `IntelligentFixtureDescriptor.cs` 61, `IntelligentFixtureModule.cs` 119

**`TPL`** (1 file, 1,377 lines): `Vixen.Application/Setup/ElementTemplates/IntelligentFixtureTemplate.cs` 1377

**Not read (binary, no logic).**
- 37 PNG icons used by the toolbars and menus: 19 in `CPE/Images/`, 18 in `POLY/Images/`. Example: `CPE/Images/BackgroundImage.png`, 32x32, on the background-image menu item and toolbar button (`CPE/Views/CustomPropEditorWindow.xaml:86`, `:257`).
- `POLY/Cursors/cursor_arrow_dragbox.cur`, a 32x32 Windows cursor with hotspot (4,2). It is shown in Add Point mode while the pointer is over a line of a closed polygon (`POLY/ViewModels/PolygonEditorViewModel.cs:3614-3626`).
- Only their names, sizes and the XAML that uses them were checked. On the Mac, use SF Symbols, and build an `NSCursor` from an image.

**Read outside the scope for context (only the parts named).**
- `Vixen.Modules/Effect/Morph/Morph/Morph.cs`:
  - capability sets, `:118-185`;
  - container getter and setter, `:405-525`;
  - matching returned shapes by id, `:1692-1760`.
- `Vixen.Modules/Editor/EffectEditor/Editors/PolygonContainerTypeEditor.cs`, all of it.
- `Vixen.Core/Commands/FixtureIndexType.cs` and `Vixen.Core/Data/Value/FunctionIdentity.cs`, the enums the fixture data uses.
- Fixture loading in `Vixen.Core`:
  - `Vixen.Core/Services/FixtureSpecificationService.cs`;
  - `Vixen.Core/IO/Loader/FixtureSpecificationLoader.cs`;
  - `Vixen.Core/IO/Xml/FixtureSpecification/FixtureSpecificationXElementMigrator.cs`, which is empty;
  - `FixtureSpecificationXElementWriter.cs`, in part;
  - `Vixen.Core/IO/ObjectVersion.cs`, which sets `FixtureSpecification = 1`.
- The sample models in `upstream/Vixen/docs/references`, named under Tests worth writing.
- The `upstream/Vixen/src/Vixen.Tests` files listed under Tests worth writing: their names, test counts and the assertions quoted there.
- The port's existing code named under Rust API needed, and `docs/UI_GUIDE.md`.

**Not in scope and not read.** These are libraries the area calls. Their behaviour here is inferred from the call sites.
- Catel MVVM:
  - `ViewModelBase`, `Command`, `TaskCommand`;
  - services `IMessageBoxService`, `IUIVisualizerService`, `IOpenFileService`, `ISaveFileService`, `IBusyIndicatorService`, `IProcessService`, `IMessageService`.
- Orc.Wizard: the wizard base classes and side navigation.
- Other packages:
  - GongSolutions.Wpf.DragDrop and Microsoft.Xaml.Behaviors;
  - Newtonsoft.Json and System.Text.Json;
  - NLog;
  - OpenTK and OpenTK.WinForms, and QuickFont;
  - the `Svg` package;
  - Fody PropertyChanged.
- Vixen.Common:
  - `WPFCommon`: behaviours, `IDownloadService`, `Common.WPFCommon.Services`;
  - `Controls`: `NameGenerator`, `SubstitutionRenamer`, the colour picker, `MessageBoxForm`;
  - `WpfPropertyGrid`, `DiscreteColorPicker`, `Broadcast`, `Messages`, `Resources`.
- `Vixen.Core`: the element and module services.
- Properties: `Property/Color` (`ColorModule.getValidColorsForElementNode`), `Property/Location`, `Property/Order`.

## Features

### A. Custom Prop Editor: window, menus, status bar

**F1. Open the Custom Prop Editor.** Tools menu item "Custom Prop Editor" in the main window. `AddApplicationMenu` adds `AppCommand("CustomPropRoot", "Custom Prop Editor")` to the "Tools" app menu, creating the Tools menu if absent (`CPE/CustomPropEditorModule.cs:53-62`, `:72-81`). Each click creates a brand-new modeless window with its own `PropEditorViewModel` (`:64-70`); several can be open at once. The window title is the fixed text "Custom Prop Editor", size 900x700, centred on screen (`CPE/Views/CustomPropEditorWindow.xaml:13`). The title never shows the file name. A new window starts with a new default prop (`PropModelServices.CreateProp()`, `CPE/ViewModels/PropEditorViewModel.cs:35-39`), `FilePath` empty.
 - Module identity (audit addition): descriptor TypeId `{DC844055-E978-47FE-8359-DBCD5B568E80}`, TypeName "Custom Prop Editor", Description "Enables creation of custom props for Previews", Author "Vixen Team", Version "1.0", static data class `CustomPropEditorData` (`CPE/CustomPropEditorDescriptor.cs:7-42`). `Loading` registers the download, message box, state-definition dialog and persistence services, adds the menu item, and wraps the static data in the process-wide `ConfigurationService.Instance().Config` (`CPE/CustomPropEditorModule.cs:19-31`). `Unloading` removes the "CustomPropRoot" item from Tools (`:33-40`).

**F2. Window layout.** Top: menu. Bottom: status bar. Middle grid, columns 1*:2*, rows 3*:Auto:2*:Auto (`CustomPropEditorWindow.xaml:155-164`). Left column top (row 0): the element panel with tabs Layout / Order / State (`views1:ElementTree`, `:166`). Left column bottom (row 2): the info panel with tabs Prop Info / Element Info (`:168-227`). Right column (rows 0-2): toolbar over the scrollable drawing canvas (`:233-343`). A vertical splitter sits between the columns, a horizontal one between the two left panels (`:344-345`). All of those splitter positions are session-only.

**F3. File > New (Ctrl+N).** `NewPropCommand`, `PropEditorViewModel.cs:1213-1227`. Prompts with `GetUserInput("Please enter the model name.", "Create Model", "New Prop")`. On OK: if the typed name does not match `{[0-9]+}` (`TokenPattern`, `:1209`), appends ` {1}`; creates `CreateProp(name)` (replacing the view models via the `Prop` setter, `:51-66`) and clears `FilePath`. The prompt is the shared text prompt (F145): its text is pre-selected, and **OK with an empty box is accepted**, so a blank name gives a prop named `" {1}"` (leading space). **Quirk: no dirty check and no save prompt; unsaved work on the current prop is discarded silently.** The menu item has an icon (`NewFile_6276.png`). Shortcut bound by `KeyPressToCommand Key="N" Modifiers="Control"` (`CustomPropEditorWindow.xaml:30`). On the Mac: ⌘N must keep the window-scoped meaning; add the dirty check (see Windows-only omissions and deviations).

**F4. File > Open.** `OpenPropCommand`, `PropEditorViewModel.cs:769-805`. Filter `Prop Files(*.prp)|*.prp`, single select. Shows the busy indicator, then `LoadPropFromPathAsync` (`:1426-1439`): replaces `Prop`, sets `FilePath`, clears dirty flags. If reading throws: error box "The prop could not be loaded." titled "Prop Load Error" and the current prop stays. **Quirk: no dirty check before Open either.** Selecting a legacy LiteDB `.prp` works transparently (see Data and persistence, "Legacy LiteDB v4 `.prp`"). No shortcut on Ctrl+O in the original (only N and S and Alt+F4 are bound, `CustomPropEditorWindow.xaml:30-32`); add ⌘O as a Mac convention.

**F5. File > Save (Ctrl+S).** `SaveModelCommand`, `PropEditorViewModel.cs:825-841`. Runs state validation first (`TryValidateStateBeforeSave`, `:1460-1472`): on error shows `StateDefinitionEditorViewModel.SaveValidationMessage` titled "Invalid State Definitions" and aborts. Sets `ModifiedDate = DateTime.Now`. If `FilePath` is empty, behaves as Save As. Otherwise `SaveModelAsync(FilePath)` (`:1441-1458`): busy indicator; `IPropModelPersistenceService.SaveAsync(Prop, path)`; on success sets `FilePath`, clears all dirty flags; on exception error box "The prop could not be saved." titled "Prop Save Error".

**F6. File > Save As.** `SaveModelAsCommand`, `PropEditorViewModel.cs:861-887`. Same state validation; sets `ModifiedDate`; save dialog filter `Prop Files(*.prp)|*.prp`, default file name `CleanseNameString(Prop.Name)`, `InitialDirectory = Paths.DataRootPath` (`:872-878`). `CleanseNameString` (`:1547-1571`) removes every `{digits}` token, trims, then collapses runs of 2+ spaces to one.

**F7. File > Import xModel.** `ImportCommand` with parameter "xModel" (`CustomPropEditorWindow.xaml:59`), `PropEditorViewModel.cs:969-1032`. Filter `xModel (*.xmodel)|*.xmodel`, single select. `InitialDirectory = Paths.DataRootPath` only when no folder was remembered yet; `_lastFolder` is a `static` field kept for the process lifetime, updated on each import (`:33`, `:981-984`, `:993`) but note it is never assigned back to the dialog's `InitialDirectory` (the OS dialog's own memory applies). Busy indicator plus a 200 ms delay before work (`:1000`). `new XModelImport().ImportAsync(path)`; a non-null result replaces `Prop`, clears `FilePath`, and **switches the canvas to selection mode** (`DrawingPanelViewModel.IsDrawing = false`, VIX-2784, `:1019-1022`). The prop is then still flagged however the `Prop` setter leaves it (imported prop is not explicitly cleared dirty: `ClearDirtyFlag` is not called here, unlike load). Any exception: error box "An error occurred importing the xModel. Please notify the Vixen Team." titled "Error Importing xModel". No dirty check before replacing the current prop.

**F8. File > Exit (Alt+F4).** `ExitCommand` closes the window (`PropEditorViewModel.cs:909-912`, `CustomPropEditorWindow.xaml:32`, `:60`).

**F9. Close prompt.** `ClosingCommand` on the window `Closing` event (`CustomPropEditorWindow.xaml:35-37`, `PropEditorViewModel.cs:932-948`). When `TestIsDirty()` (`:589-592`: VM `IsDirty` OR any element VM dirty OR any light VM dirty OR state editor dirty) asks `GetUserConfirmation("Save Prop \"<cleansed name>\" ", "Save")`. **Correction (audit):** `GetUserConfirmation` is `Common.WPFCommon.Services.MessageBoxService`, which shows a `MessageBoxForm` with only two buttons, "YES" (DialogResult OK) and "NO" (`Vixen.Common/WPFCommon/Services/MessageBoxService.cs:44-51`, `Vixen.Common/Controls/MessageBoxForm.cs:48-53`, designer `:46`, `:70`). There is no Cancel button. YES saves (`SaveModel()`, async void, not awaited; the window closes anyway); NO maps to `MessageResult.No`, which neither branch tests, so the window closes without saving; only closing the message box with its title-bar close button returns Cancel and keeps the editor open. **Hazard: if the save needs a file dialog or state validation fails, the close still proceeds in the original because `SaveModel` is not awaited and does not set `e.Cancel`.** On the Mac use the standard "Save / Don't Save / Cancel" sheet and only close once the save has completed; a failed validation must keep the window open.

**F147. Dirty tracking and its gaps (audit addition).** "Dirty" is `TestIsDirty()`: the editor VM's own `IsDirty`, or any element view model in the process-wide lookup, or any current light view model, or the State editor (`PropEditorViewModel.cs:589-592`). Several edits lose the flag, so the close prompt (F9) can be skipped after real changes:
 - Adding and deleting lights rebuild every light view model from scratch (`RefreshLightViewModels`, `DrawingPanelViewModel.cs:315-339`), so earlier light edits stop counting.
 - Element view models of removed rows are disposed and leave the lookup (`ElementViewModelCollection`, `ElementModelLookUpService.RemoveModel`), so Delete (which marks the removed row dirty, `ElementModelViewModel.cs:559-567`), Cut and drag moves leave no flag behind.
 - Order renumbering writes `ElementModel.Order` directly (`ElementOrderViewModel.cs:128-135`), and Align and Flip write `Light.X/Y` on the model.
 - Any `ModelsChanged` re-binds the State editor and resets its `IsDirty` to "migration happened" (Section D intro).
 - Save and Load clear every flag through `ClearDirtyFlag` (`:594-600`); xModel import does not (F7).
 The port must use one document revision counter in Rust: every mutating call bumps it, save records the saved revision, and `is_dirty` compares the two (the window's `isDocumentEdited` follows it). Record this as a fix in `docs/DIFFERENCES.md`.

**F10. Edit menu: Cut, Copy, Paste.** Bound to `ElementTreeViewModel.CutCommand/CopyCommand/PasteCommand` (`CustomPropEditorWindow.xaml:66-82`). See F27-F31 for behaviour. Icons `cut.png`, `page_white_copy.png`, `page_white_paste.png`. There is no Undo or Redo anywhere in this editor (no undo service exists in `CPE/`). See the deviations note under F12.

**F11. View menu.** "Assign Background" (F14), "Scale Background Image…" (F15), "Background Opacity" submenu (F16) (`CustomPropEditorWindow.xaml:83-104`).

**F12. No undo in the original.** The whole Custom Prop Editor has no undo or redo; every edit is immediate and only the dirty flag tracks it. Per `docs/UI_GUIDE.md` undo uses the window's `UndoManager`, and the Rust API section below proposes a document-snapshot undo that adds this without changing any behaviour. Treat it as a Mac improvement, not parity.

**F13. Tools menu.** "Vendor Browser" (F21), "Export Wire Diagram" submenu with "Front Wire Diagram" (`CommandParameter=False`) and "Back Wire Diagram" (`CommandParameter=True`) (`CustomPropEditorWindow.xaml:105-111`; F20). Options menu: "Preferences" (F19) (`:112-114`). Help menu: "View Help" opens `http://www.vixenlights.com/vixen-3-documentation/preview/custom-prop-editor/` in the browser (`PropEditorViewModel.cs:1383-1392`).

**F14. Assign Background (menu and toolbar).** `LoadImageCommand`, `PropEditorViewModel.cs:1297-1320`. Filter `Image Files(*.JPG;*.GIF;*.PNG)|*.JPG;*.GIF;*.PNG|All files (*.*)|*.*`, single select, `InitialDirectory` is `Environment.SpecialFolder.MyPictures.ToString()` (**quirk: this is the literal string "MyPictures", not a resolved path**; use the Pictures folder on the Mac). `PropModelServices.SetImage(path)`, then re-evaluates `ScaleBackgroundImageCommand`. Toolbar button tooltip "Assign Background" (`CustomPropEditorWindow.xaml:256-258`).

**F15. Scale Background Image….** `ScaleBackgroundImageCommand`, enabled only when `Prop.Image != null && DrawingPanelViewModel != null` (`PropEditorViewModel.cs:1338-1341`). Opens `BackgroundImageScaleWindow` with image pixel size, current canvas size (ints) and `hasLights` = any light exists (`:1350-1355`). On OK with options: `DrawingPanelViewModel.ApplyBackgroundImageScale` (`CPE/ViewModels/DrawingPanelViewModel.cs:463-493`): validates target dims 1..100,000 (throws `ArgumentException` otherwise), current dims finite and positive; sets `Width`/`Height`; when `ScaleExistingLightPositions` multiplies each unique light's centre by `targetW/currentW`, `targetH/currentH` (grouped by `Light.Id` so a light shown twice is scaled once). Dialog details in F20a.

**F16. Background Opacity submenu.** A radio-style submenu built by `controls:SliderMenuItem` with items 10%...100% (values .1 to 1, `CustomPropEditorWindow.xaml:90-103`), two-way bound to `DrawingPanelViewModel.Opacity` which maps to `Prop.Opacity`. Persisted with the prop. **Precision (audit):** it is not radio items: the submenu body is a **vertical slider beside the ten labels**, top 10% to bottom 100% (`CPE/Controls/SliderMenuItem.xaml:12-20`, reversed direction). The slider snaps to ticks lined up with the labels; dragging it or clicking a label sets the value (`SliderMenuItem.xaml.cs:156-243`, `:412-415`). A stored opacity between two steps (possible from a file) puts the thumb proportionally between ticks (`SetTickToValue`, `:188-236`). On the Mac a submenu of checkmarked steps is fine; an off-step value shows no checkmark.

**F17. Status bar.** Left item: mouse coordinates over the canvas, formatted `{X:0}, {Y:0}` with a literal ", " between; hidden (not collapsed) when the mouse is not over `PropDesigner` (`CustomPropEditorWindow.xaml:124-139`). `Coordinates` updates in the canvas `MouseMove` before any early return (`CPE/Controls/PropDesigner.cs:353-357`). Right item: `StatusMessage`, collapsed when empty (`CustomPropEditorWindow.xaml:140-151`). Only the vendor browser sets it: "Loading vendor list...", "Retrieving inventory from {name}...", then empty (`PropEditorViewModel.cs:1061`, `:1071`, `:1089`).

**F18. Prop Info panel, tab General.** A property grid bound to the `PropEditorViewModel` itself (`CustomPropEditorWindow.xaml:172-187`) with `ShowReadOnlyProperties`. Browsable properties: `Name` (no category attributes, `PropEditorViewModel.cs:81-86`); category "Details": `Type` (order 18, description "A useful category like Christmas, Halloween, General", `:100-108`), `Created By` (order 20, `:122-130`), `Creation Date` (order 20, setter internal, so read-only, `:144-152`), `Modified Date` (order 21, `:167-175`, set on save). Changing a value marks the VM dirty.

**F18a. Prop Info panel, tabs Physical, Vendor, Notes.** Physical: `PhysicalMetadataView` bound to `PhysicalMetadata`. Vendor: `VendorMetadataView` bound to `VendorMetadata`. Notes: multi-line text box bound to `InformationMetadata.Notes`, wraps, accepts Return and Tab, spell check on (`CustomPropEditorWindow.xaml:188-208`). Controls of the first two are listed in their own entries.

**F18b. Element Info panel.** A property grid bound to `ElementTreeViewModel.SelectedItem` (`CustomPropEditorWindow.xaml:211-226`), which is non-null only when exactly one tree row is selected (`CPE/ViewModels/ElementTreeViewModel.cs:119-128`). Shown properties of `ElementModelViewModel` (`DisplayName("Element Model")`, `CPE/ViewModels/ElementModelViewModel.cs:14`), in order: `Name` (0, editable); `Face Component` (1, enum, description "Face component associated with this element for Lip-Sync.", `:207-220`); `Face Color` (2, hex string, description "Face color in Hex associated with this element for Lip-Sync.", returns empty when component is None, otherwise `DefaultColor.ToHex()`; an invalid hex string stores `Color.Empty`, `:229-250`, validation regex `^#(?:[0-9a-fA-F]{3}){1,2}$` `:597-622`); `Model Type` (3, enum, description "Custom prop role for this element when State definitions are authored.", `:259-276`); read-only `Children` (7, count of direct children, `:422-425`); `Lights` (8, count of light leaves, `:434-437`); `Light Size` (9, string: "multiple sizes", empty when no lights, else the number, `:446-449`, `:631-680`). State name, item and colour properties exist but are `[Browsable(false)]` (legacy; `:285-412`). Validation errors: "Name can not be empty", "Duplicate name", "Light size must be > 0" (`:580-595`).
 - Descriptions of the read-only rows (audit addition): Children "Number of child elements associated with this element.", Lights "Number of light elements under this element.", Light Size "Size of light element(s) under this element." (`:422-449`). `Lights` counts light **nodes** (leaf elements with lights) through `GetLeafEnumerator`, so a shared leaf counts once per path, not individual lights.
 - **Name edits from this grid are not uniquified** (the setter writes `ElementModel.Name` and marks dirty, `:188-198`), unlike inline rename (F23) and Rename (F32); a duplicate only shows the "Duplicate name" validation error.
 - Light Size quirk (`:651-680`): a group's children are walked in order; a child group with no lights contributes 0, which is overwritten if it comes first but reads as a mismatch ("multiple sizes") if a sized child came before it.

### B. Custom Prop Editor: element panel

**F22. Layout tab: tree.** `TreeView` of `ElementModelViewModel` rows rooted at `Prop.RootNode`, multi-select through `TreeViewMultipleSelectionBehavior` (`CPE/Views/ElementTree.xaml:226-293`). The same element model may appear under several parents (a model can have several parents; `ElementModelLookUpService` maps model id to all its view models). Tab index binding `SelectedTabIndex` (`:224`). Changing tab: index 0 clears tree selection, index 1 clears order selection, then re-evaluates state preview (`PropEditorViewModel.cs:414-431`). **Selection is deliberately not carried across tabs** (commented-out code).

**F23. Inline rename.** Single left click on the name of a row that has been selected for more than 750 ms starts editing (`BeginEdit`: `IsSelected && _selectedTime + 750 ms < now`, `ElementModelViewModel.cs:469-476`), or F2 (`ElementTree.xaml:253-259`). F2 runs the same `BeginEditCommand`, so it also does nothing until the row has been selected for 750 ms. `_selectedTime` is reset whenever `IsSelected` is set, true or false (`:97-110`). `_textHoldValue` keeps the old name. Text box selects all on focus, commits on Enter, Escape rolls back; focus loss or Enter runs `DoneEditing` (if `IsNameDuplicated`, runs `Uniquify`, then sets `IsDirty`, `:496-504`); Escape runs `CancelEditing` (restores `_textHoldValue`, `:524-528`).

**F24. Layout tab keyboard.** Delete runs `DrawingPanelViewModel.DeleteSelectedLightsCommand` (F40) (**quirk: the tree's Delete key deletes selected lights, not selected groups; the context menu's Delete is different**, F33). Ctrl+X, Ctrl+C, Ctrl+V run tree Cut, Copy, Paste (`ElementTree.xaml:240-246`).

**F25. Context menu "Create Empty Group".** `CreateNodeCommand`, `ElementTreeViewModel.cs:401-421`. Enabled when exactly one row selected, it is a group node, and all its children are group nodes. Prompts "Please enter the group name." / "Create Group" with suggestion `Uniquify(selected name)` (so "Group" suggests "Group - 2"). On OK `CreateNode(response, selection)` uniquifies the answer again with order index 2, creates a child group under the selection and expands it. Does not call `OnModelsChanged`. An empty answer is accepted (F145) and creates a group named "" unless "" is taken.

**F26. Context menu "Copy to New Group" and "Move to New Group".** Both enabled by `CanGroup` (`:719-724`): any selection, none is the root, all same `ElementType`. Copy to New Group (`CreateGroup`, `:176-186`): prompts (empty suggestion), `CreateGroupForElementModels(name, distinct models)`, raises `ModelsChanged`. Move to New Group (`MoveToGroup`, `:214-252`): prompts; if the selection is **all** the children of its first item's parent the new group goes under that parent, otherwise under root (`parentForGroup` null); adds each selected model to the new group and removes it from its old parent; raises `ModelsChanged`; expands and selects the new group.

**F27. Context menu Cut.** `:440-469`. Enabled when `SelectedItems.Any() && CanGroup()`. Puts `List<ElementModel>` on the system clipboard using a private format named `typeof(List<ElementModel>).FullName` (`:33`, `:445-447`); removes each from its parent. A cut model whose last parent goes away is orphaned and removed (`RemoveFromParent` semantics, see services). Raises `ModelsChanged`.
 - **Upstream data-loss hazard (audit addition).** The orphan cascade clears the lights of every orphaned leaf below a cut group, and only the top model leaves the `_models` index; its orphaned descendants stay in the index as lightless "ghosts" (`ElementModel.cs:348-371`, `PropModelServices.cs:303-311`). Paste looks models up by id in that index (`FindOrCreateElementModelTree`, `:138-161`), so pasting a cut group re-creates the group but links the ghost leaves, which no longer have lights. A cut light node pastes back with its lights only if the clipboard holds a serialized copy taken before the removal (the data object is set with `copy: true` before `RemoveFromParent`, `ElementTreeViewModel.cs:445-457`); if the runtime hands back the live object, its lights were already cleared. **Do not port this:** Cut must capture the full subtree (lights included) into the clip, removal must drop orphaned descendants from the id and name index, and Paste must re-create from the clip whenever an id is no longer in the document.

**F28. Context menu Copy.** `:488-505`. Same enabled rule; clipboard only.

**F29. Context menu Paste.** `:524-583`. Enabled when one row selected, clipboard has the format, and either all are light nodes not already children and the target `CanAddLightNodes`, or all are group nodes not already children and the target `CanAddGroupNodes`. Skips models equal to the target or already its children. Uses `FindOrCreateElementModelTree(model, parent)`, i.e. **links the existing model by id when it still exists in this prop, otherwise re-creates it** (a re-created element keeps its id but its name goes through `Uniquify(name)` with order index 2, `PropModelServices.cs:123-136`). Selects the target afterwards. Paste uses the WinForms clipboard (`System.Windows.Forms.Clipboard.GetDataObject()`) while Cut and Copy write through the WPF clipboard (`ElementTreeViewModel.cs:445-447`, `:526`).

**F30. Context menu Paste as New.** `:602-672`. Prompts "Please enter the new name." / "Paste As New" with suggestion `Uniquify(parent.Name)`. Uses `CreateElementModelTree(model, parent, newName)`, i.e. always new ids. Selects the new models. Enabled (`CanPasteAsNew`, `:645-672`) when exactly one row is selected, the clipboard holds the format, and either every clip item is a light node and the target `CanAddLightNodes`, or every item is a group node and the target `CanAddGroupNodes`; **unlike Paste there is no "already a child" check**. Every top-level clip item is created with the same typed name, so several items become "Name", "Name - 2", "Name - 3"...; descendants are named after their new parent with one-based numbering ("Parent - 1", "Parent - 2"...).

**F31. Clipboard is cross-window.** The format is global and `ElementModel` is `[Serializable]`-style data, so a copy in one editor window can paste into another (paste creates via `FindOrCreate`). On the Mac use a private pasteboard type holding JSON of the element subtree so it works between windows.

**F32. Context menu Rename.** `RenameCommand`, `ElementTreeViewModel.cs:338-381`, always enabled. One selected: prompt "Please enter the new name." / "Rename" pre-filled, new name goes through `Uniquify`. **Quirk: `Uniquify(response)` is called without the element being renamed, so the element counts as a clash with itself: pressing OK on the unchanged name renames "Arm" to "Arm - 2"** (`:346`; the same in the Order tab, `ElementOrderViewModel.cs:161`). The port should exclude the renamed element and record the fix. Several selected: opens `Common.Controls.NameGeneration.NameGenerator` (the shared WinForms pattern renamer) with the old names and assigns `Uniquify(names[i], 2, model)` for each (logs a warning if it runs out of names). Find/Replace Rename: `SubstitutionRenameCommand` (`:280-319`), enabled when >1 selected, opens `Common.Controls.SubstitutionRenamer`, same assignment.

**F33. Context menu Delete.** `PropEditorViewModel.DeleteCommand` (`ElementTree.xaml:193`, `PropEditorViewModel.cs:729-749`), disabled if the selection contains the root. **Quirk: `CanDelete` compares view models to `Prop.RootNode` (a model) with `x.Equals`, so it is effectively always true; deleting the root row does nothing harmful only because the root VM has no parent VM (`RemoveFromParent` no-ops, `ElementModelViewModel.cs:559-567`).** Removes each selected row from its parent, deselects all lights, rebuilds canvas and order list, refreshes state assignments and preview. No confirmation. It does not raise `ModelsChanged`, so the State tab is not re-bound (only `RefreshAssignments` runs, `PropEditorViewModel.cs:729-740`).

**F146. Layout tab context menu: items and order (audit addition).** `ElementTree.xaml:167-200`, attached to each row's name text: "Create Empty Group" (F25), "Copy to New Group", "Move to New Group" (F26), separator, "Cut" (icon `cut.png`, F27), "Copy" (`page_white_copy.png`, F28), "Paste" (`page_white_paste.png`, F29), "Paste as New" (same paste icon, F30), separator, "Delete" (`delete_32.png`, F33), "Rename" (F32), "Find/Replace Rename" (F32). On the Mac keep this order and the separators in `.contextMenu`; SF Symbols `scissors`, `doc.on.doc`, `doc.on.clipboard`, `trash`.

**F34. Tree drag and drop: rules.** `ElementTree.xaml:234-237`; drop rules in `CanAcceptData`, `ElementTreeViewModel.cs:745-883`. Rejected: no drag info; other context; dropping onto itself; target not an element; mixed groups and lights; before-target on root; before-target on a group with lights dragged; dragging to a position in a parent where (from another parent) any dragged model is already a child ("already part of this group"); after-target with lights onto a group that can't take lights; centre-drop onto non-group, root with lights, group that can't take groups (`CanAddGroupNodes`) or leaves (`CanAddLeafNodes`), or a target that is already a parent of any dragged model. Same collection always accepted beyond this point; different collections need compatible types and target not a descendant of the source. Over a valid centre target the adorner highlights the row, otherwise an insert line shows (`DragOver`, `:913-932`).

**F35. Tree drag and drop: drop.** `Drop`, `:949-1035`, tested by `Vixen.Tests/App/CustomPropEditor/ElementTreeDragDropTests.cs`. Only `Move` is honoured (the Copy effect returns without doing anything). If key state is exactly Ctrl, dragged items are reversed. Clears selection. Centre drop: for each, add to target then remove from old parent, select with parent expanded (`MoveToTargetGroup`, `:1037-1060`). Before/after drop: target's parent must exist and every source parent must be a group. **All in target parent:** `TryMoveWithinParent(parent, models, UnfilteredInsertIndex)`, keeps view model instances and re-selects them. **None in target parent:** insert into target parent at increasing index, then remove from the old parent, select. **Mixed (some in target parent, some not): no-op after clearing selection.** Note: this does not raise `ModelsChanged`; the lookups stay valid because lights are not orphaned (add-before-remove).

**F36. Tree drag start.** `StartDrag`, `:1127-1177`: one source and one selected: that item. One source and none selected: selects the source and drags it. Several sources: those. Several selected and one source: all selected if the source is in the selection; otherwise deselects all, selects and drags only the source. Effects Copy|Move.

**F37. Order tab.** `ListView` of leaf light nodes ordered by `Order`, distinct by model id (`RefreshElementLeafViewModels`, `CPE/ViewModels/ElementOrderViewModel.cs:118-126`), each row shows `Order` then `Name`, multi-select (`ElementTree.xaml:295-355`). Inline rename same as the tree (F23). Delete key runs `DeleteSelectedLightsCommand` (F40). Context menu: "Reverse", "Rename", "Find/Replace Rename" (`ElementTree.xaml:202-206`). Drag and drop reorders: multi-item move with insert line (`DragOver` always Move; `Drop`, `ElementOrderViewModel.cs:319-381`), then `ReOrder()` renumbers `Order` 1..n over the whole list (`:128-135`). This `Order` is what the wire diagram and the patch order use.
 - The rows are the **same `ElementModelViewModel` instances as the Layout tree** (taken from `ElementModelLookUpService`, `:125`), so `IsSelected`, inline editing and names are shared between the two tabs.
 - Orders are only renumbered by a drop or Reverse. Adding lights uses `GetNextOrder` and deleting lights leaves gaps, so the list can show 1, 2, 7, 9 (F38 depends on this).
 - **Cross-list drags are not blocked upstream:** the tree and the order list share the default GongSolutions drag context, so dropping tree rows on the Order list removes their view models from the tree's child collection and inserts them into the list (`ElementOrderViewModel.cs:345-378`), and a multi-row drag from the Order list can be dropped into the tree and run the tree's move logic (F35). The port must reject drags between the two lists.

**F38. Order tab: Reverse.** `ReverseSelected`, `ElementOrderViewModel.cs:272-283`, enabled with any selection. Each selected item in reverse selection order is removed and inserted at index `workList[ctr].Order - 1`, i.e. selected rows swap into each other's slots, unselected rows stay; `ReOrder` follows. **Quirk (audit): the target index is `Order - 1`, which equals the list position only while orders are contiguous 1..n. After lights have been deleted (gaps, F37) the index can pass the end of the list and `LeafNodes.Insert` throws `ArgumentOutOfRangeException`** (e.g. orders 1, 2, 10: reversing the last two inserts at index 9 of a 2-item list). The port should reverse the selected rows among their own list positions, then renumber, and record the fix. Order tab Rename: one selected prompts (same text as tree) and `Uniquify`; several open `NameGenerator` (`:152-195`). Find/Replace Rename opens `SubstitutionRenamer`, enabled when >1 selected (`:214-253`).

**F39. State tab.** See section D (F50-F62); it is the third tab of this panel (`ElementTree.xaml:356-502`, `SelectedTabIndex == 2`, `PropEditorViewModel.cs:409`).

### C. Custom Prop Editor: drawing canvas

**F40. Delete selected lights.** Delete key on canvas, tree or order list (`CustomPropEditorWindow.xaml:307`, `ElementTree.xaml:242`, `:306`). `DeleteSelectedLights`, `DrawingPanelViewModel.cs:427-448`. With lights selected asks `GetUserConfirmation("Deleting lights will remove them from all any/all groups they are part of. Are you sure?", "Delete lights")`. OK: deselect, `RemoveLights`, rebuild, raise `LightModelsChanged` (order list and state assignments refresh; `PropEditorViewModel.cs:483-487`). Exact wording kept in the original is a typo-like "all any/all"; keep the meaning with a verb button "Delete". The confirmation is the same two-button "YES"/"NO" box as F9; closing it with its close button also cancels. The command has no CanExecute (`DrawingPanelViewModel.cs:42`), so the Delete key with nothing selected silently does nothing. An element left with no lights is removed from every parent (CPE model rules, "Remove light").

**F41. Draw mode vs selection mode.** Radio buttons at the toolbar start: "Draw mode" (tooltip) and "Selection mode" (`CustomPropEditorWindow.xaml:236-244`), bound to `DrawingPanelViewModel.IsDrawing`, default **true** (`DrawingPanelViewModel.cs:46`). Each new `DrawingPanelViewModel` resets it, so New and Open return to draw mode (the setter builds a new VM), while xModel import forces selection mode (F7).

**F42. Add a light (draw mode).** Left click on empty canvas: `AddLightCommand.Execute(point)` (`PropDesigner.cs:282-287`), `PropEditorViewModel.AddLight`, `:1247-1277`: target = single selected tree row (or null); `PropModelServices.AddLight(target, p, Configuration.DefaultLightSize)` (see services); rebuild canvas and order list; if the light was put straight on the target select it, otherwise select the new light's parent and expand it; invalidate commands. Clicking on an existing light in draw mode does nothing except position capture (the `else if (!IsDrawing && e.Source is Path)` branch does not run, `PropDesigner.cs:292`).
 - In draw mode a click on empty canvas does **not** clear the current light selection first (`ClearSelections` only runs when `!IsDrawing`, `PropDesigner.cs:274-280`); the follow-up tree selection of the new light's parent then selects that element's lights on the canvas through F49.
 - When the target is a light node, the extra light is created with that element's own `LightSize`, not the Preferences default size (`AddLightToTarget`, `PropModelServices.cs:388-392`); a new light node uses the default size.
 - A new light node is named after its parent group with one-based numbering: clicking into group "Arm" gives "Arm - 1", "Arm - 2"... (`Uniquify(target.Name, 1)`, `:330`).

**F43. Selection mode: click.** `PropDesigner.cs:257-347`. Click on empty canvas without Ctrl clears selection (not in state mode). Click on a light: without Ctrl, if the light is not selected, clear and select it (an already selected light stays with the group so it can be dragged); with Ctrl toggles it (⌘ on Mac). Sets `_isSelecting = true`, which here means "dragging moves lights".

**F44. Selection mode: drag.** `PropDesigner.cs:349-421`. With left button down: first move only sets `_dragging` (and, when not on a light, captures the mouse and creates the rubber band). Later moves on a light: translate the selection by the delta (`TransformCommand` with `TranslateTransform`, `:378-385`). `DrawingPanelViewModel.Transform` applies the transform to each selected light's centre, in parallel (`DrawingPanelViewModel.cs:535-538`). No clamping to the canvas for this drag. Empty-canvas drag: updates rubber band and selection with `UpdateSelection` (`PropDesigner.cs:503-552`): a light is selected when `rubberBand.Contains(itemBounds)` (fully inside); lights outside are deselected unless Ctrl is held. Mouse up releases, removes the rubber band, refreshes the resize adorner (`:462-484`). Selected light colours and lists update live.

**F45. Resize and rotate adorner.** Shown only in selection mode, not in state mode, and only when **more than one** light is selected (`PropDesigner.cs:554-590`); it is also not created while a rubber-band drag is in progress (`if (_dragging && !_isSelecting) return;`, `:562`), and it is removed and rebuilt on every selection change and on every move step of a light drag (`:231-242`, `:384-385`). The handles are drawn rotated by the adorner's current angle about its rotation centre (`GetDesiredTransform`, `CPE/Adorners/ResizeAdorner.cs:431-438`). Bounds = union of selected lights' visual bounds (`:606-632`). Ten thumbs each 10x10 (`CPE/Adorners/ResizeAdorner.cs:56-69`): four corners, four edge midpoints, centre drag, rotate (positioned above the top edge at `Bounds.Y - 5 * height`, `:382`). Cursor per thumb (`:56-69`).
 - Corners scale about the opposite corner; Shift locks aspect with `max(scaleX, scaleY)` (`:145-229`). Edge midpoints scale on one axis about the opposite edge (`:231-286`).
 - `EnforceSize` (`:414-429`) only allows a scale when the scaled rect fits: `Rect.Scale` multiplies X, Y, width and height, then requires `X >= 0 && Y >= 0 && width <= X + el.Width && Y + height <= el.Height`. **Keep this behaviour, including that it scales the rect's origin too.**
 - Centre drag moves everything by mouse delta, refused when the bounds would leave the canvas (`:128-142`) and moves the rotation centre too (`:326-332`). The test is strict (`Left + dx > 0`, `Right + dx < ActualWidth`, same for y), and `_dragStart` is updated before the test, so a refused step is dropped rather than caught up later (the Polygon Editor's adorner does the opposite, F89).
 - Rotate (`:288-314`): angle = `(atan2(center - mouse) in degrees + 270) mod 360`; holding **Ctrl** snaps to 45° detents with ±22.5° capture (`:295-305`; the code comment says Shift, the code uses Ctrl); applies only the difference since the last event about `_rotationCenter`, which is fixed at the adorner's creation until a centre drag. Scale handles apply reverse-rotate, scale, rotate, so scaling after rotation works in the rotated frame (`:316-324`). The rotation angle is held by the adorner, so it resets whenever the adorner is rebuilt.

**F46. Align, distribute, flip toolbar.** Enabled when any light selected (`DrawingPanelViewModel.cs:73-76`). Tooltips "Align Left", "Align Right", "Align Top", "Align Bottom", "Distribute Horizontal", "Distribute Vertical", "Flip Horizontal", "Flip Vertical" (`CustomPropEditorWindow.xaml:260-284`). **Quirks kept for parity:** Align Left and Align Right both set every X to the *first selected* light's X; Align Top and Align Bottom both set every Y to the first selected light's Y (`DrawingPanelViewModel.cs:540-574`). Flip: `X = max - (X - min)` with `max/min` of `Math.Abs(X)` (`:576-596`). Distribute: needs more than 2, sorts by coordinate, first stays, each next gets `min + i * (max - min)/(n-1)`; the last lands on max (`:598-649`). Writes through `Light.X/Y` so the light VM updates.

**F47. Light size toolbar.** "Increase Light Size", "Decrease Light Size", "Match Light Size" (`CustomPropEditorWindow.xaml:286-294`). Increase and decrease: `ElementModel.LightSize` of each selected light's parent `++`/`--`, enabled with any selection (`DrawingPanelViewModel.cs:693-755`). **The loop runs once per selected light, not once per element**, so an element with k selected lights grows or shrinks by k per click (a 5-light string fully selected goes 3 → 8). Decrease stops at 1 because the setter clamps (`ElementModel.cs:176-183`). The port must reproduce the per-light step or record the change. Match: enabled with more than one, uses the first selected light's parent's size for the rest (`:774-791`). Then refreshes the Element Info grid. Size belongs to the element model, so all its lights change.

**F48. Canvas drawing.** Each light is a circle path at `Center` with radius `Size` filled with `DisplayColor` (`CustomPropEditorWindow.xaml:310-323`). Canvas size `DrawingPanelViewModel.Width/Height`; background is the prop image with `Opacity` and `Stretch="Fill"` (`:327-336`). Scroll viewer with auto scrollbars on black, margin 10 (`:297`). No zoom exists. Colours: normal light `Configuration.LightColor`, selected `SelectedLightColor` (`DrawingPanelViewModel.cs:379-385`), recomputed on every selection change. Selection is mirrored: canvas to tree selects parents; tree or order selection selects all leaf lights (`PropEditorViewModel.cs:498-587`, guarded by `_selectionChanging`).

**F49. Selection sync details.** Canvas `Add` selects each light's parent models in the tree and, in state mode, assigns the new lights to the selected state item (`AssignSelectedLightsToStateItem`, `:602-615`); `Remove` deselects parents; `Reset` deselects tree and order. Tree or order `Add`/`Remove`/`Reset` selects or deselects the leaves on the canvas.

### CPE model rules (from `Services/PropModelServices.cs` and `Model/*.cs`)

These are behaviour, not presentation; they belong in Rust, in a new `vixen-core::custom_prop` module (see Rust API for why it sits there).

- **Shared-singleton hazard.** `PropModelServices.Instance()` holds one `_prop` and one `_models` dictionary per process, and `ElementModelLookUpService.Instance` one view-model map (`CPE/Services/PropModelServices.cs:15-17`, `CPE/Services/ElementModelLookUpService.cs:10-11`). `CreateProp` and `LoadPropAsync` overwrite them (`PropModelServices.cs:53-73`) and the `Prop` setter resets the lookup (`PropEditorViewModel.cs:53`). So with two editor windows open, the older window's commands act on the newer window's prop. **Do not port this:** each window owns its own Rust `PropEditor`, holding its own `Prop`, which makes the multi-window behaviour of F1 actually work.
- **New prop default.** `CreateProp(name = "New Prop {1}", x = 800, y = 600)` (`:53-59`). `Prop(string, int, int)` (`CPE/Model/Prop.cs:29-42`): new id, a root `ElementModel` named after the prop, a solid-black 800x600 image (`Indexed1`, single palette entry), `Opacity = 1`, `CreationDate = ModifiedDate = now`, `CreatedBy = Environment.UserName`, empty vendor, physical and info metadata. `Prop.Name` is an alias of the root element's name (`:70-79`). Width and height come from the image through the `Image` setter when not already valid (`:136-152`; valid means finite and 1..100000, `:236-239`).
- **Assign image.** `SetImage` loads the file, sets `Image`, then **always overwrites** `Width`/`Height` with the image's pixel size (`PropModelServices.cs:83-103`). **Lights are not moved or scaled.** Load failure is logged only (silent to the user). Keep the original file's bytes? No: only the decoded bitmap is kept and it is re-encoded as JPEG on save (see Data and persistence).
- **`ElementModel` type rules** (`CPE/Model/ElementModel.cs`). `DefaultLightSize = 3` (`:14`). `IsLeaf` = no children. `IsLightNode` = leaf with lights. `IsGroupNode` = **no lights** (so an empty leaf counts as a group, `:269`). `IsRootNode` = no parents. `CanAddGroupNodes` = group and (no children or any child is a group). `CanAddLeafNodes` = group and (no children or any child is a leaf). `CanAddLightNodes` = not root and (no children or is a light node or no child is a group) (`:271-275`). `ElementType`: more than one light = String ("Multiple Nodes"), one = Node ("Single Node"), none = Group (`:137-153`, descriptions `CPE/Model/ElementType.cs`). `Order` getter returns `_order` for a leaf and **-1 for non-leaves** (`:114-131`). `LightSize` getter on a non-leaf returns the first leaf's size (or its own `_lightSize` if none) (`:165-183`); the setter clamps values `<= 0` to 1, then pushes to all lights (leaf) or all children recursively (`UpdateLightSize`, `:415-428`). A freshly constructed element gets new `Id`, new `StatePropertyId`, `LightSize = 3`, default `FaceDefinition` (None, White) (`:27-38`).
- **Multi-parent graph.** Elements form a DAG: `Parents` is a list of ids, `Children` a list of element references (`:283-313`). The same leaf can sit in several groups; `Prop.GetLeafNodes` may return duplicates (`CPE/Model/Prop.cs:195-200`). **Orphan cascade:** `RemoveParent` removes the link both ways; when no parents remain a leaf clears its lights and a group recursively detaches from its children (`ElementModel.cs:348-371`). `RemoveFromParent` then also drops the model from `_models` (`PropModelServices.cs:303-311`). **Only that top model is dropped: orphaned descendants stay in `_models` as lightless ghosts**, so `Uniquify` and `IsNameDuplicated` still see their names (a deleted group's old child names keep forcing " - n" suffixes), and Paste can re-link them (F27). In Rust, removal must drop every orphaned descendant from the index.
- **Add light** (`:349-358`). If the target is a non-group with a parent (a light node), the point is added as another light on that same element (`AddLightToTarget`, `:388-392`), making it a String, and the target is returned. Otherwise `AddLightNode(target, p, null, size)` (`:313-347`): target null or root is replaced with `FindOrCreateTargetGroupForLight` (`:360-374`: when root is a leaf create child "`<root name> Group`"; else first node from `GetNodeEnumerator` with `CanAddLightNodes`, **note `First()` throws if none and the `??` fallback is unreachable**; `GetNodeEnumerator` is pre-order and includes leaves, `ElementModel.cs:430-433`, and every non-root leaf satisfies `CanAddLightNodes`, so in practice a match always exists. Edge case: a group that mixes light nodes with an empty group (reachable through a before/after drop, F34) fails the test, so the first match is its first light node, and the original then adds the new element **as a child of that light node** (the "already has lights" check only guards a caller-supplied target), leaving an element with both lights and children. Rust should accept only group nodes here and record it); a target that already has lights throws `ArgumentException("Cannot add light node to leaf element with lights.")`; order = `GetNextOrder`; the new element is named `Uniquify(target name or nodeName, 1)` (so "X", "X - 1"... **one-based**), added as child; size from parameter; one light created at the point.
- **Next order.** `GetNextOrder` = max over all leaves; if `max <= 0` returns 1, else `max + 1` (`:481-493`). `OrderExists` checks any leaf (`:495-498`).
- **Name uniquifying.** `Uniquify(name, orderIndex = 2, renameModel = null)`: when another model (excluding `renameModel`) has that exact name, tries `"{name} - {counter}"` starting at `orderIndex` until no model has it (`:465-479`). **Note the check inside the loop does not exclude `renameModel`.** `IsNameDuplicated` = more than one model with the name (`:460-463`). Names are unique per prop, not per parent.
- **Create child group.** `CreateNode(name, parent = root, oneBasedNaming = false)` uniquifies with order index 1 or 2, adds child (`:110-121`).
- **Copy to New Group.** `CreateGroupForElementModels` creates a node under root and adds each existing model as an additional child (`Children.Add` and `Parents.Add`) (`:192-200`); nothing leaves its old parent.
- **Paste trees.** `FindOrCreateElementModelTree(model, parent)` (`:138-161`): when the id is unknown creates a node with the **same id** under parent; for a light node copies size, **shares the same `Lights` collection reference**, and keeps its order unless taken (then next order); recurses on children. When known, just links via `AddToParent`. `CreateElementModelTree(model, parent, name, recursed)` (`:163-183`): creates a new node named `name` (top) or `parent.Name` with one-based naming (descendants), with `oneBasedNaming = recursed`; for a light node copies size, re-creates each light at the same X,Y on the new element, assigns next order; recurses.
- **Same-parent batch move.** `TryMoveWithinParent(parent, models, insertionIndex)` (`:217-284`): rejects (logged, returns false) null parent or models, insertion index outside `0..=Children.Count`, empty, null entries, duplicates, or any entry not a child. Then for each model in order: `oldIndex`; when `oldIndex < cursor` `cursor--`; move to `cursor` when different; `cursor = destination + 1`. Pinned by `ElementTreeDragDropTests.cs`.
- **Remove light** (`:402-451`). Only for a leaf target. Removes the light (its `ParentModelId` becomes empty, `ElementModel.cs:404-412`). When the element has no lights left, removes it from every parent and from `_models`.
- **Equality.** Elements compare by `Id` (`:455-473`). Lights do not (reference equality), so `Children.Remove`-style code relies on id for elements only.

**`PropStateModelResolver`** (`CPE/Services/PropStateModelResolver.cs`). `GetModelElement(prop)` = first element with `ModelType == Model`, else root (`:10-15`). `CanAssignModelType(model, type)` = type is not Model, or the element is not a leaf, or it is a root with no children (`:26-33`). `TrySetModelType` also demotes every other explicit Model element to None (`:35-55`), so at most one exists. **Note: the property grid's `ModelType` setter writes the field directly and does not call this (`ElementModelViewModel.cs:262-276`)**, so the one-Model rule is enforced only where the resolver is called. **Resolved (audit): `TrySetModelType` has no production caller; only `Vixen.Tests/App/CustomPropEditor/State/CustomPropStateDataFoundationTests.cs:113-137` calls it.** In the editor, two Model elements or a leaf Model element are only reported by the State validation messages, which block Save (F61). The port may enforce the rule in the Element Info picker (demote the old Model) but must then record the difference.

**`CustomPropStateMigrationService`** (`CPE/Services/CustomPropStateMigrationService.cs`). `MigrateLegacyStateData(prop)` per element: when `StateDefinitionModels` is empty and legacy `StateDefinitions` rows exist, group rows by `StateDefinitionName` (trimmed, else "State - 1"), in each group order by `Index`, drop items with no element ids, drop empty groups (`:47-75`). Then on the model element (per resolver): when it still has no definitions, gather `StateDefinition` (singular) off every element, group by name, one item per element with that element's id (`:77-104`). Returns whether anything changed.

**`CustomPropStateDataMapper`** (`CPE/Services/CustomPropStateDataMapper.cs`). Bidirectional mapping `StateData` (Property/State) <-> element-model authored state: empty ids get fresh GUIDs, blank names become defaults ("State - 1", "Item Name 1") and are trimmed, item element ids are distinct (`:12-97`). **Correction (audit): this class has no production caller** (only `CustomPropStateDataFoundationTests.cs:40`, `:82`). Placing a prop in a preview uses `PreviewCustomPropBuilder`'s own mapping (`Vixen.Modules/Preview/VixenPreview/PreviewCustomPropBuilder.cs:173-331`, owned by `docs/spec/preview.md`): for every element with authored definitions (else legacy rows, else legacy per-child `StateDefinition`s) it adds a State property whose id is the element's `StatePropertyId` (a new GUID when empty), keeps definition and item ids (new when empty), trims names with defaults "State - 1"/"Item Name 1", translates element-model ids to the new element-node ids, and drops items with no surviving element and definitions with no items.

**Defaults.** `StateDefinitionModel.DefaultName = "State - 1"`; `CreateDefault(name)` has one default item (`CPE/Model/StateDefinitionModel.cs:18`, `:36-43`). `StateItemModel.DefaultName = "Item Name 1"`, default colour white (`CPE/Model/StateItemModel.cs:19`, `:24-30`). Legacy `StateDefinition`: `StateDefinitionName = "State Name 1"`, `Name = "Item 1"`, white (`CPE/Model/StateDefinition.cs:18-24`). `Configuration` defaults: light White, selected HotPink, state preview base RGB(25,25,25), default light size 3 clamped below 1 back to 3 (`CPE/Model/Configuration.cs:8-10`, `:49-58`, `CPE/CustomPropEditorData.cs:29-39`).

Enums: `ColorMode` FullColor "Full Color", Multiple "Multiple Color", Single "Single Color", Other "Other" (`CPE/Model/ColorMode.cs`). `FaceComponent` None, EyesClosed "Eyes Closed", EyesOpen "Eyes Open", EyesClosed2 "Eyes Closed 2", EyesOpen2 "Eyes Open 2", EyesClosed3 "Eyes Closed 3", EyesOpen3 "Eyes Open 3", Outlines "Outline", Outlines2 "Outline 2", AI, E, FV, L, MBP, O, U, WQ, ETC, REST (`CPE/Model/FaceComponent.cs`). `ElementModel.IsPhoneme`: everything except None, EyesClosed, EyesOpen, Outlines (**note EyesClosed2/3, EyesOpen2/3 and Outlines2 count as phonemes**, `ElementModel.cs:334-346`). `ElementModelType` None, Model, SubModel, FaceInfo, StateInfo (`CPE/Model/ElementModelType.cs`). `BulbType` suggestions: 12mm Bullet, 12mm Square, 1x3 Rectangle, PixaBulb, Strip, Generic Pixel, M5, M6, C6, C7, C9, G40 (`CPE/Converters/BulbTypeConverter.cs:48-66`).

### D. Custom Prop Editor: State tab (definitions authored on the prop)

The third element-panel tab, data context `StateDefinitionEditorViewModel` (`CPE/Views/ElementTree.xaml:356-502`). All edits go to `ModelElement.StateDefinitionModels`, where `ModelElement` = `PropStateModelResolver.GetModelElement(prop)` (the first element whose Model Type is Model, otherwise the root). `SetProp` also migrates legacy data (see the migration note above), selects the first definition and marks dirty only when migration changed something (`CPE/ViewModels/State/StateDefinitionEditorViewModel.cs:213-234`). `SetProp` re-runs on every `ModelsChanged` (`PropEditorViewModel.cs:489-495`), which resets the selected definition to the first one.
 - Every `SetProp` also normalizes the Model element's state data (`ElementModel.NormalizeStateModelData`, `CPE/Model/ElementModel.cs:475-489`): empty ids get new GUIDs, a blank definition name becomes "State - 1", **a blank item name becomes "Item Name 1"**, and names are trimmed (`CPE/Model/StateDefinitionModel.cs` `Normalize`, `CPE/Model/StateItemModel.cs` `Normalize`). So a blank item name that F61 reports is silently repaired by the next operation that raises `ModelsChanged`: Cut, Paste, Paste as New, Copy to New Group or Move to New Group (drag and drop, Delete and Create Empty Group do not raise it).
 - Every `SetProp` rebuilds the definition view models (losing their dirty flags) and sets the editor's own `IsDirty = migrated` (`StateDefinitionEditorViewModel.cs:213-234`), which **clears the state-dirty flag after any tree change**; see F147.

**F50. Definition row.** Label "Name", then a combo of `StateDefinitions` (display `Name`), buttons "Add", "Delete", and a "..." menu with "Rename" and "Copy" (`ElementTree.xaml:375-407`). Changing selection selects that definition's `SelectedItem` as the single selected row and refreshes the preview (`StateDefinitionEditorViewModel.cs:90-107`).

**F51. Add definition.** `AddStateDefinitionAsync` (`:277-296`). Dialog `RequestNameAsync("Add State Definition", StateNamingRules.GetNextStateDefinitionName(existing), existing, null)`; `TryNormalizeName` rejects blank or duplicate results (silently returns); `StateDefinitionModel.CreateDefault(name)` (one default item), appended and selected. Always enabled.

**F52. Delete definition.** `:298-311`, enabled when a definition is selected. `ConfirmDeleteAsync(name)` (dialog text in the State section); removes from model and list; selects the item at the same index or the last. Note the empty state: with no definitions the grid is empty and all item buttons disable.

**F53. Rename definition ("..." > Rename).** `:318-339`. Dialog "Rename State Definition", suggested current name, existing names, current name passed as allowed. Normalized through `TryNormalizeName(name, current, existing, ...)`.

**F54. Copy definition ("..." > Copy).** `:346-371`. Dialog "Copy State Definition", suggestion "`<name> Copy`". `CloneAsNew(normalized)` gives new definition and item ids but **keeps item element assignments and description** (`CPE/Model/StateDefinitionModel.cs:105-114`, `CPE/Model/StateItemModel.cs:85-94`). Appended and selected.

**F55. Description.** Multi-line-height text box (min height 48) bound to `SelectedStateDefinition.Description`, `UpdateSourceTrigger=PropertyChanged` (`ElementTree.xaml:408-411`), dirties on every keystroke (`CPE/ViewModels/State/CustomPropStateDefinitionViewModel.cs:89-103`).

**F56. State items grid.** Header "State Items". Data grid columns: "Name" (editable, width *, commits on each keystroke), "Color" (width 75, shows `ColorHex` with the colour as background and auto-contrast text, double-click opens the colour picker through `EditColorCommand`, editing template is a hex text box; invalid hex is ignored), "Count" (width 70, read-only `AssignmentCount`) (`ElementTree.xaml:421-468`). Extended multi-select full row; no add row; `AssignmentCount` counts distinct assigned ids that still exist in the prop (`CPE/ViewModels/State/CustomPropStateItemViewModel.cs:116-118`). `ColorHex` displays `#RRGGBB` (`Extensions.ToHex`, `CPE/Extensions/Extensions.cs:13-16`); parsing accepts anything `ColorConverter.ConvertFromString` does; on Mac accept #RGB, #RRGGBB, #AARRGGBB and CSS names at minimum. Colour picker: `ColorPicker { LockValue_V = false, Color = XYZ.FromRGB(Color) }`; on OK `Color = picker.Color.ToRGB()` (`:223-235`). Map to the system colour panel.

**F57. Clicking a column header sorts and persists.** `DataGridSortedItemsCommandBehavior` with `ClearSortAfterCommand=True` runs `PersistStateItemSortCommand` with the displayed order, so a click on a column header **permanently reorders the items**, and the sort indicator clears (`ElementTree.xaml:433-434`, `StateDefinitionEditorViewModel.cs:497-510`, `CustomPropStateDefinitionViewModel.cs:203-230`). Colour column sorts by `ColorHex`, Count by `AssignmentCount`.
 - (audit) `ApplyItemOrder` does nothing (no dirty flag) unless the sorted list contains every item of the definition exactly once and differs from the current order (`CustomPropStateDefinitionViewModel.cs:203-213`). After a reorder the selected item stays selected (first item when none was), and the definition is marked dirty (`:228-229`). The command needs only a selected definition (`StateDefinitionEditorViewModel.cs:497-510`).

Current-source naming clarification (2026-11-11): `Property/State/StateNamingRules.cs` generates `State - n` for added definitions and `State Item - n` for added items. The one item created with a new definition remains `Item Name 1`. Copied items append ` - n` to the full trimmed source name. Candidate generation and prompt uniqueness compare ordinally/case-sensitively; save validation compares trimmed names case-insensitively.

**F58. Item buttons** (right-aligned row, `ElementTree.xaml:469-491`).
 - "Add": `AddStateItem`, new `StateItemModel` named `StateNamingRules.GetNextStateItemName(names)`, enabled when a definition is selected (`StateDefinitionEditorViewModel.cs:378-390`).
 - "Copy": exactly one selected row in this definition; `CloneAsNew` (keeps assignments, new id) with the next name from the source name (`:392-405`, `:412-417`).
 - "Remove": any selected rows, or else `SelectedStateItem`. One item: `ConfirmDeleteStateItemAsync(name)`; several: `ConfirmDeleteStateItemsAsync(count)`. Selection moves to the minimum removed index clamped to the end (`:427-453`, `CustomPropStateDefinitionViewModel.cs:147-172`).
 - Up arrow (tooltip "Move selected State Item up"), Down arrow (tooltip "Move selected State Item down"): move `SelectedStateItem` by one; disabled at the ends (`:460-495`).
 - Adding, copying, removing or moving reselects a single row.

**F59. Canvas assignment (state mode).** While the State tab is active (`IsStateDefinitionMode`) the canvas assigns, rather than selects, lights, and the resize adorner is removed (`PropDesigner.cs:72-86`, `:231-242`, `:554-560`). `CanEditCanvasAssignments = SelectedStateItems.Count <= 1` (`StateDefinitionEditorViewModel.cs:137`); with several rows selected the canvas is read-only: clicks and drags do nothing (`PropDesigner.cs:267-272`, `:309-313`, `:364-367`). With one row selected:
 - Click a light (selection mode only, the outer `!IsDrawing` branch, `PropDesigner.cs:292`): toggles that light's **element** on the selected item (`PropEditorViewModel.ToggleStateItemAssignment`, `:670-687`, `CustomPropStateItemViewModel.ToggleElementModelId`, `:192-206`). In draw mode clicking empty canvas still adds a light, and clicking a light does nothing.
 - Rubber band: lights fully inside are assigned; with Ctrl (⌘) they are removed instead; lights outside are untouched (`PropDesigner.cs:515-543`, `PropEditorViewModel.cs:622-663`).
 - **Correction (audit):** the Layout tree is not visible while the State tab is showing (they are tabs of the same control), so tree selection cannot assign. `AssignSelectedLightsToStateItem` (`PropEditorViewModel.cs:602-615`) runs from `DrawingViewModel_SelectedItemsChanged` for canvas-originated selection additions only (tree-to-canvas propagation is suppressed by `_selectionChanging`, `:498-531`). In state mode that path is reached when a light click is not handled as a toggle (for example with no State item selected, `PropDesigner.cs:298-315`). It does **not** check `CanEditCanvasAssignments`, unlike the click and rubber-band paths (`:622-687`). Assignments are by the **parent element of the light**, so assigning one light of a multi-light element assigns all its lights.
 - With several rows selected (assignment disabled), a click on empty canvas returns before the draw-mode branch, so **no light is added in draw mode either** (`PropDesigner.cs:265-272`).
 - Quirk: `_isSelecting` is only reset by a click on empty canvas, and a handled state-toggle click returns before setting it (`:298-307`). A drag that starts on a light in state mode therefore reuses the previous value: if the last selection-mode click was on a light, the drag translates whatever lights are still selected from before the tab switch (switching to the State tab does not clear the canvas selection, `PropEditorViewModel.cs:414-431`). The port must never move lights from a state-mode drag.

**F60. State preview on the canvas.** Active only while the State tab is selected (`PropEditorViewModel.cs:689-709`). Preview items = selected rows, or the single `SelectedStateItem`. `DrawingPanelViewModel.ApplyStatePreview` (`DrawingPanelViewModel.cs:345-368`): each light assigned to one or more preview items is coloured with the rounded per-channel average of those items' colours (alpha 255); other lights get `StatePreviewBaseColor`, default RGB(25,25,25). Leaving the tab restores normal colours (`ClearStatePreview`, `:373-377`). While preview is active selection colours do not apply (`:67-70`). Refreshes on any state data change, selection change, preferences close and tab change.

**F61. Validation messages.** Red (`IndianRed`) wrapping list under the grid, recomputed by `RefreshValidation` (`StateDefinitionEditorViewModel.cs:252-262`) after any state change, and before every save. Blocks Save and Save As (F5, F6) with all lines joined by newline. Messages, in order (`:571-622`):
 - "Only one element can have Model Type set to Model." (more than one explicit Model)
 - `Element "<name>" cannot be a Model because it is a leaf element.`
 - "State definition names cannot be empty."
 - `State definition "<name>" must contain at least one item.`
 - `State definition "<name>" contains an item with no name.`
 - `State item "<item>" in "<definition>" must assign at least one element.`
 - `State definition name "<name>" is duplicated.` (case-insensitive, trimmed)
 Blank names display as `<unnamed>`.

**F62. Assignment clean-up.** `RefreshAssignments` removes ids that no longer exist in the prop from every item and marks dirty when anything was removed (`CustomPropStateItemViewModel.cs:123-135`, `:244-257`). Runs after light deletion and tree changes (`PropEditorViewModel.cs:483-495`, `:738`).
 - (audit) It also runs on the edited item after every assignment change (`OnAssignmentChanged`, `CustomPropStateItemViewModel.cs:216-221`), so toggling or rubber-banding one element also prunes that item's stale ids. Assign and remove ignore `Guid.Empty`, de-duplicate, and report no change (no dirty flag, no preview refresh) when nothing was added or removed (`:142-185`); toggle always dirties (`:192-206`). The "Count" column counts assigned **elements**, not lights. A blank or unparsable `ColorHex` edit leaves the colour unchanged and raises no change (`:85-98`, `:266-291`); setting the same colour is also a no-op (`:69-74`).

### E. Custom Prop Editor: dialogs

**F19. Preferences dialog.** Options > Preferences (`ColorOptionsCommand`, `PropEditorViewModel.cs:1412-1420`). Title "Preferences", 300x200 tool window, Escape closes (`CPE/Views/ConfigurationWindow.xaml:1-7`, `CPE/ViewModels/ConfigurationWindowViewModel.cs:19`). Rows: "Light Color", "Selected Color", "State Preview Base Color" (swatches, double-click opens the colour picker), "Default Light Size" (text box bound to a `uint`) (`ConfigurationWindow.xaml:33-62`). Buttons "Defaults" (restores White, HotPink, RGB(25,25,25), 3 on the dialog only, `ConfigurationWindowViewModel.cs:201-207`), "Ok" (writes to `Configuration`, `:259-267`), "Cancel". **Upstream bug: the "Selected Color" swatch's double-click is bound to `EditStatePreviewBaseColorCommand` (`ConfigurationWindow.xaml:48`), so it edits the wrong colour and Selected Color can only change through Defaults. `EditSelectedLightColorCommand` exists (`ConfigurationWindowViewModel.cs:151-162`) and is unused. Fix on the Mac and record it in `docs/DIFFERENCES.md`.** After the dialog closes (OK or Cancel) the state preview refreshes and `VixenSystem.SaveModuleConfigAsync()` writes the module config. **Note: normal light colours on the canvas are not recomputed here until the next selection change or rebuild.** `Configuration.DefaultLightSize` clamps values below 1 to 3 (`CPE/Model/Configuration.cs:49-58`). On the Mac: a sheet from this window or a section in Settings; keep the labels and the Defaults button. Swatches become `ColorPicker`.

**F20a. Scale Background Image dialog.** Title "Scale Background Image", 440x360, no resize, Escape closes (`CPE/Views/BackgroundImageScaleWindow.xaml:5-13`). Rows top to bottom: "Original bitmap:" `"{w} × {h} px"`; "Current editor:" same format; "Units:" combo of Pixels, Percent; "Width:" and "Height:" numeric text boxes (per-keystroke, with validation adornment); check "Lock aspect ratio"; check "Scale existing light positions" (disabled when the prop has no lights); bold "Resulting size: {w} × {h} px"; buttons "OK" (default, disabled while errors) and "Cancel" (`:46-77`).
 - Construction (`CPE/ViewModels/BackgroundImageScaleViewModel.cs:30-56`): throws when source or current dimensions are outside 1..100000. Targets start at the current canvas size; Unit Pixels; lock on; `ScaleExistingLightPositions = hasLights`; the setter forces false when there are no lights (`:188-192`).
 - Units: Pixels shows target. Percent shows `target * 100 / source` (percent of the **source bitmap's** dimension, not the current canvas) (`:376-381`). Changing unit reformats inputs (`:91-105`).
 - Conversion (`CPE/BackgroundImageScaling/BackgroundImageScaleCalculator.cs:26-42`): Percent input `source * value / 100`, then round **away from zero**; must be finite and 1..100,000.
 - Aspect lock: turning it on (false to true) captures `_lockBase = current target`, then re-derives from the last edited side (`:163-178`, `:316-332`). Locked width edit: `height = round(width * baseH / baseW)`; locked height edit: `width = round(height * baseW / baseH)`, written back into the other field (`CalculateLocked*`, calculator `:52-72`).
 - Errors: "Enter a finite width that results in 1 through 100,000 pixels."; "Enter a finite height that results in 1 through 100,000 pixels."; "The aspect-ratio height must result in 1 through 100,000 pixels."; "The aspect-ratio width must result in 1 through 100,000 pixels." (`:393-420`).
 - OK produces `BackgroundImageScaleOptions(width, height, scaleLights)` (`:258-268`), applied by F15. The bitmap itself never changes: only the logical canvas size, the image stretches to fill it.

**F20b. Metadata tabs (pointer).** The Physical tab is F20c and the Vendor tab is F20d; F18a lists where they sit in the Prop Info panel.

**F20c. Physical metadata tab** (`CPE/Views/PhysicalMetadataView.xaml`, `CPE/ViewModels/PhysicalMetadataViewModel.cs`). Property grid, in `PropertyOrder`: Material (0, description "The predominate material the props is made of."), Height (1, "The overall height of the prop."), Width (1, "The overall width of the prop."), Depth (3, "The overall depth of the prop."), Node Count (4, no description), Bulb Type (5, "The light type used.", editable combo with the suggestions listed above, `BulbTypeConverter`), Color Mode (6, description "Declares color handling for the entire prop.\nIf the prop contains multiple color modes, choose other.\nThis maps to color handling in the Display Setup."; enum with descriptions). All are free strings except Color Mode. Mac form: `Form` with `LabeledContent`, the description text as `.help`.

**F20d. Vendor metadata tab** (`CPE/Views/VendorMetadataView.xaml`, `CPE/ViewModels/VendorMetadataViewModel.cs`). Property grid, category "Vendor": Name (1), Contact (2), Website (3), Email (4), Phone (5). All free strings.

**F145. Shared text prompt (audit addition).** Every name prompt in the Custom Prop Editor except the State tab's (which uses F134) is `Common.WPFCommon.Services.MessageBoxService.GetUserInput(question, title, default)` (`Vixen.Common/WPFCommon/Services/MessageBoxService.cs:11-42`), a WinForms `TextDialog` (`Vixen.Common/Controls/TextDialog.cs:21-46`): the question as a label, one text box pre-filled with the default and **fully selected**, OK and Cancel; Return is OK and Escape is Cancel. The validation loop in `GetUserInput` never repeats (the `continue` jumps to a condition that is false once OK was pressed), so **OK with an empty box returns OK with ""**. Users: New (F3, "Please enter the model name." / "Create Model"), Create Empty Group, Copy to New Group and Move to New Group ("Please enter the group name." / "Create Group", F25, F26), Rename ("Please enter the new name." / "Rename", F32, F38) and Paste as New ("Please enter the new name." / "Paste As New", F30). On the Mac: one small sheet with the same texts, a Cancel and a verb button (Create, Rename, Paste); disable the verb while the field is blank and record that as a fix.

### F. Custom Prop Editor: Vendor Browser

**F21. Tools > Vendor Browser.** `OpenVendorBrowserCommand`, `PropEditorViewModel.cs:1052-1147`. Network flow, shown behind a busy indicator:
 1. Status "Loading vendor list...". GET `https://app.vixenlights.com/vendor.json`, parsed as a JSON array of `{ "Name", "Url" }` (`:1519-1545`). Failure: error "Unable to retrieve vendor list. Ensure you have an active internet connection." titled "Error Retrieving Vendors"; stops silently if the list is empty.
 2. For each vendor: status "Retrieving inventory from {name}...", GET the XML, `XModelInventoryImporter.Import(xml)`. Per-vendor failure: error "Unable to retrieve inventory from {name}\nEnsure you have an active internet connection." titled "Error Retrieving Inventory", then continue. Stops if none succeeded.
 3. Opens the modal `VendorInventoryWindow`.
 4. On dialog OK: `LoadVendorModel(SelectedModelLink)` (`:1474-1517`): download to `<temp>/<guid>`; `ModelType.Prop` opens as a prop file via `LoadPropFromPathAsync`; otherwise `ImportProp` (xModel). Download failure: error "Unable to download the model from the vendor.\nEnsure you have an active internet connection." titled "Error Downloading Model.". A temp file is not removed by the original; on the Mac delete it after loading.
 5. On success with an xModel: copies the product into metadata: `PhysicalMetadata.Width/Height/Depth(=Thickness)/Material/BulbType(=PixelDescription)/NodeCount(=PixelCount string)`, `InformationMetadata.Notes`, `Prop.Type = ProductType` (`:1103-1113`). For both model types fills any empty vendor fields from the vendor: Name, Contact, Email, Phone, and Website from the `WebLink` named exactly "Website" (`:1116-1143`).
 Note: this replaces the current prop with no dirty check.
 6. Audit additions (`PropEditorViewModel.cs:1474-1517`):
    - A downloaded `.prp` goes through `LoadPropFromPathAsync`, which sets `FilePath` to the temporary download path. **A later Save (Ctrl+S) silently writes to that temp file**, not to the data folder. On the Mac treat a vendor `.prp` as untitled, so Save asks for a location.
    - The `.prp` branch reports success even when the loader returned null, and exceptions from it are not caught: they escape the command and leave the busy indicator showing (only the xModel branch has its own error box, F7).
    - The download itself shows the busy indicator but no status-bar text.

**F21a. Vendor Inventory window.** Title "Vendor Inventory", 800x600, columns 1*:2* with a splitter (`CPE/Views/VendorInventoryWindow.xaml:8`, `:41-44`, `:190`; `CPE/ViewModels/VendorInventoryWindowViewModel.cs:23`).
 - Left top: label "Vendor", combo of inventories (display `Vendor.Name`), default first (`VendorInventoryWindow.xaml:56-57`; VM `:17`).
 - Left middle: `ModelInventoryTree`: categories as expandable nodes whose children are subcategories then products; a product is a plain-looking hyperlink (no underline) that runs `SelectProductCommand` (`CPE/Views/ModelInventoryTree.xaml:23-56`). Selecting: `SelectedModelLink = null`, sets `SelectedProduct`, `IsProductVisible`, `IsModelValid = any model link has a Link`, switches to the Product Info tab (VM `:173-180`).
 - Left bottom: vendor logo (async image from `Vendor.Logo`), "Contact", "Phone", an "Email" hyperlink (`SendEmailCommand`: prefixes `mailto:` when absent, VM `:255-262`), and a list of web-link hyperlinks (`NavigateToUrlCommand`, opens in the browser; errors only logged, VM `:218-236`) (`VendorInventoryWindow.xaml:61-98`).
 - Right (only when a product is selected): product image (async from `ImageUrl`) on top (row 1.2*), tabs below (row 1*): **Product Info** with rows Product (hyperlink to `Url`), Type, Material, Height, Width, Thickness, Pixel Count, Pixel Description, Pixel Spacing, Notes (`:113-157`); **Model Options** (hidden unless `IsModelValid`), a scrolling list where each row has a "Select" button (`ImportModelCommand`) plus "Name" and "Description" (`:158-186`).
 - "Select": when a product and link exist sets `SelectedModelLink`, closes with result true (VM `:281-289`). There is no OK or Cancel button: closing the window cancels. The bottom status bar is empty.
 - Quirk (audit): changing the Vendor combo does not clear the selected product (`SelectedValue` binding only, `VendorInventoryWindow.xaml:57`), so the right pane keeps showing the previous vendor's product, and "Select" on it then fills the vendor metadata from the **newly** chosen vendor (F21 step 5 reads `SelectedInventory.Vendor`). The port should clear the product when the vendor changes.
 - The "Email" link calls `address.StartsWith(...)` on the vendor's email, so a vendor with no `<email>` element throws (VM `:255-262`). Hide or disable the link when the address is empty.
 - Window title "Vendor Inventory" comes from the view model (`:23`); 800x600, centred on owner (`VendorInventoryWindow.xaml:8`).

### G. Custom Prop Editor: xModel import

All pure logic: port to Rust and test against `upstream/Vixen/docs/references/GE Stardust 1500.xmodel` and `upstream/Vixen/docs/references/circlemodel/*.xmodel`.

**F63. File routing** (`CPE/Import/XLights/XModelImport.cs:20-44`). Parse XML. No root: error "The xModel file does not contain a model definition." titled "Model import error". A parser that `CanImport` the root is used directly. Root `<models>`: see F64. Anything else: `Unsupported model type: {type}. \nImport supports custom and circle model types at this time.` titled "Model import error" (`:169-172`). `CanImport`: element has attributes and either is named `custommodel` / `circlemodel`, or is named `model` with `DisplayAs` equal (case-insensitive) to `Custom` / `Circle` (`CPE/Import/XLights/XModelElementMetadata.cs:7-18`). Element name matching is case-sensitive ordinal (`:33-36`). Unsupported type label: the element name, or for `model` the lowercased `DisplayAs` plus `model`, or just `model` when `DisplayAs` is missing or blank (`:20-31`). Parser order: custom, circle (`XModelImport.cs:179-186`). These errors use Catel's `IMessageService.ShowErrorAsync` and then return null, so no second "Error Importing xModel" box follows (that one is only for exceptions, F7).

**F64. `<models>` wrapper and "Select xModel" dialog** (`XModelImport.cs:46-110`). Candidates are direct `<model>` children, each `{index, display name, type, isSupported}`. Display name: trimmed `name` attribute, else `Unnamed model {index+1}` (`:112-118`); duplicates get ` (n)` numbered in order (`:120-143`). **Every** occurrence of a duplicated name is suffixed, starting at 1 (ordinal comparison): two models named "Star" show as "Star (1)" and "Star (2)"; a unique name gets no suffix. `index` counts only the `<model>` children. None: "No models were found in the xModel file.". All unsupported: the unsupported error for the first. Exactly one candidate: imported directly (even when unsupported, which then errors). Several: dialog titled "Select xModel" (420x320, resizable; min 360x260): bold "Select the embedded model to import." and "The Custom Prop Editor can import one model at a time."; list rows show display name and type at right; default selection is the first supported, else first; "OK" disabled with no selection; "Cancel" (`CPE/Views/XModelSelectionView.xaml:24-53`, `CPE/ViewModels/XModelSelectionViewModel.cs:9-58`). The busy indicator is hidden while the dialog is up and re-shown after (`CPE/Import/XLights/XModelSelectionService.cs:14-27`). Selecting an unsupported row errors out (`XModelImport.cs:85-89`).

**F65. Custom model parsing** (`CPE/Import/XLights/Custom/CustomXModelElementParser.cs`). Common config: `name`; `PixelSize` positive int else 1 with a warning (`CPE/Import/XLights/XModelCommonConfigurationParser.cs:20-37`); `StringType`, `StrandNames`, `NodeNames` read but not used by the assembler. Dimensions: `CustomWidth` else `parm1`, `CustomHeight` else `parm2`, positive else 0 (`:47-59`). Default scale from `GetDefaultScale(x, y)`: both < 100 gives 4, both < 200 gives 2, else 1 (`CPE/Import/XLights/XModelCoordinateScale.cs:9-22`). Per-axis scale: `ScaleX`, `ScaleY` when parseable (float, invariant) and > 0, else default (warning when present but invalid) (`:24-32`, `:49-72`). `Apply(v, s) = round(v*s, away from zero)` to int.
 - Source order (`CPE/Import/XLights/Custom/CustomXModelSourceResolver.cs:15-60`): non-blank `CustomModelCompressed` first; on a format error fall back to `CustomModel` (warning logged); both failing or both empty: error `Unable to parse a valid CustomModel or CustomModelCompressed for model '{name}'.`
 - `CustomModelCompressed` (`CPE/Import/XLights/Custom/CustomXModelNodeParser.cs:61-96`): `;`-separated (empties removed) entries `order,row,column`; each must have 3 integer parts. Node `{ order, X = ApplyX(column) + 1, Y = ApplyY(row) + 1 }`. **The middle number is the row (Y) and the last the column (X).** Later duplicates overwrite.
 - `CustomModel` (`:12-52`): rows split by `;`, cells by `,`, blank cells skip but still advance X. Node `{ order, X = ApplyX(cellIndex) + 1, Y = ApplyY(rowIndex) + 1 }`. A non-integer cell raises a format error. (audit) That includes xLights' multi-layer cells (`1|2`, used by 3D custom models): there is no layer support, so such a `CustomModel` is rejected unless `CustomModelCompressed` parsed first. Numbers use `int.TryParse` in the current culture, so signs and surrounding spaces are accepted and a negative order becomes a node. A compressed value that is non-blank but holds no entries (`;;`) succeeds with zero nodes and does **not** fall back (`CustomXModelSourceResolver.cs:23-37`).
 - Then child elements (F67). Result: `XModelParsedModel(importModel)`.

**F66. Circle model parsing** (`CPE/Import/XLights/Circle/`). Requires: `LayerSizes` comma list of positive ints, else `Circle model '{name}' has an invalid LayerSizes value.`; `InsideOut` exactly "0" or "1" (`...invalid InsideOut value.`); `StartSide` first char uppercased must be B or T (`...invalid StartSide value.`); `Dir` first char uppercased must be L or R (`...invalid Dir value.`); `PixelCount` when > 0 must equal the sum of `LayerSizes` (`...PixelCount does not match the sum of LayerSizes.`) (`CircleXModelConfigurationParser.cs:19-49`). `centerPercent` else `parm3`, non-numeric becomes 0 with a warning, clamped 0..100 (`:147-170`). Scale default from `GetDefaultScale(maxLayer, maxLayer)`, then `ScaleX`/`ScaleY` (`:51-54`). `NumStrings` else `parm1`, `NodesPerString` else `parm2` (parsed, unused).
 - Rings (`:76-102`): physical layer indexes ascending when InsideOut, descending otherwise; node orders are assigned sequentially from 1; circle numbers from 1.
 - Nodes (`CircleXModelNodeGenerator.cs:7-68`): `maxLights = max(LayerSizes)`, `maxRadius = maxLights/2`, `minRadius = centerPercent/100 * maxRadius`. Radius of a layer: `maxRadius` when one layer, else `minRadius + (maxRadius-minRadius)*layerIndex/(layerCount-1)` (so layer 0 is innermost). `startAngle = StartSide == 'T' ? π : 0`, `direction = Dir == 'L' ? -1 : 1`, `step = 2π/count`. `angle = start + direction*step*i`; `X = round(sin(angle)*radius*ScaleX)`, `Y = round(cos(angle)*radius*ScaleY)`, away from zero. Then subtract the minimum X and Y from every node.
 - `XModelImportModel.X = Y = max(LayerSizes)` (this also sets `Scale`, which circle does not use).
 - Generated groups (`CircleXModelGroupGenerator.cs`): unless a `<subModel name="Circles">` (case-insensitive) of type ranges whose line groups exactly match the rings in any order (`order > 0` filtering, sequence equal per ring) exists, add group `{name} {1} - Circles` with children `{name} {1} - Circle {n}` holding each ring's orders.

**F67. Child elements** (`CPE/Import/XLights/XModelChildElementImporter.cs`). Case-sensitive tag dispatch; anything else is ignored; `modelGroup` is skipped with an info log (`:15-39`).
 - `subModel`: name, `layout`; `type="ranges"` reads `line0`, `line1`... until the first empty or missing attribute, each a range group; `type="subbuffer"` is dropped (`:41-58`, `:211-231`). **A blank `line` ends the list.** Any other `type`, or none, is dropped too (only the exact, case-sensitive `ranges` is kept).
 - Ranges (`:233-259`): comma list; each `a-b` or `a`; blanks skip; `Convert.ToInt32` throws on non-numbers, which surfaces as the generic import error. A token with two or more dashes (`1-2-3`) splits into three parts and is **skipped silently**; a negative number (`-5`) splits into an empty first part and throws.
 - `faceInfo`: only `Type="NodeRange"`. For each known attribute key (`Eyes-Closed`, `Eyes-Open`, `Eyes-Closed2`, `Eyes-Open2`, `Eyes-Closed3`, `Eyes-Open3`, `FaceOutline`, `FaceOutline2`, `Mouth-AI`, `Mouth-FV`, `Mouth-MBP`, `Mouth-E`, `Mouth-L`, `Mouth-O`, `Mouth-U`, `Mouth-WQ`, `Mouth-etc`, `Mouth-rest`, `CPE/Import/XLights/Faces/FaceInfo.cs:7-27`; **`FaceOutline2` maps to `Outlines`, not `Outlines2`**) with a non-empty value: a `FaceItem` with range, `<key>-Color` colour via `ColorTranslator.FromHtml` (default White), name = key (`:60-94`).
 - `stateInfo`: needs attributes and `Type` of `NodeRange` or `SingleNode`. Name through `NormalizeStateName(name, count+1)`: trimmed or `State Name {n}` (`CPE/Import/XLights/States/XLightsStateNameNormalizer.cs:11-17`). Attributes starting with lowercase `s` (skipping Type, NodeRange, Name, CustomColors): split on `-`; key `sN` parsed as index (non-numeric skipped); `sN` alone is the range (NodeRange: ranges; SingleNode: value split by space, needs exactly two parts, second int is a one-node range); `sN-Color` colour; `sN-Name` item name (`NormalizeStateItemName`: trimmed or the tag such as `s3`). Items with no range are dropped (`:96-209`). Default item colour White.
 - (audit) Item order within a `stateInfo` is the XML attribute order in which each `sN` key first appears (a dictionary filled while walking the attributes, `:114-147`, `:199-206`); an attribute name with three or more dash-separated parts (`s1-Name-x`) creates the item but sets nothing. A `faceInfo` or `stateInfo` that ends with no items is still added to the import model (`:93`, `:208`), which matters for F68. The `sN` prefix test is a case-sensitive `StartsWith("s")` (`:127`).

**F68. Assembly into a prop** (`CPE/Import/XLights/XModelPropAssembler.cs`). `Offset = 7`, minimum size 800x600 (`:14-16`). Size: with nodes, `max(X)+7+PixelSize` and `max(Y)+7+PixelSize`, floored to the minimum; without nodes `max(X+14, 800)`, `max(Y+14, 600)` (`:40-53`). `CreateProp("{name} {1}", w, h)`, `CreatedBy = "xModel Import"`, `NodeCount` = distinct light nodes (`:19-38`).
 - **Model group** (`:87-106`): child of root named `{name} {1} - Model`, `ModelType = Model`. Light nodes in ascending order, each `AddLightNode(group, (X+7, Y+7), order, PixelSize, "{name} {1} Px {order}")`. Map `order -> element`.
 - **Authored states** (`:108-144`): each `stateInfo` becomes a `StateDefinitionModel` on the Model group, name unique with ` - n` suffix from 2 (ordinal), empty description, items ordered by index, skipped when no element resolves; ids from enumerating ranges (distinct); empty definitions dropped.
 - **Sub models** (`:186-216`): skip when ranges type with no ranges. Group `{name} {1} - {subModel}`, `ModelType = SubModel`. More than one line adds a child group per line named `{name} {1} - {subModel} - {group name} {index}`; index counts only non-empty lines. **`{group name}` is the sub-model group's full name** (`subModelGroup.Name`, `:206`), so model "Star", sub-model "Arms", line 1 gives "Star {1} - Arms - Star {1} - Arms 1" (or "... - Star {1} - Arms - 2 1" when the group name was uniquified). Keep the literal naming for parity, or fix it and record the difference; no upstream test pins the per-line child names (the circle tests only check orders, `XModelCircleImportTests.cs:349-369`).
 - Creation order is Model group, sub-models, faces, legacy states, generated groups (`:55-85`), and every group name goes through `Uniquify(name, 2)`. So when a "Circles" sub-model does not match the rings, the generated group comes second and is named "{name} {1} - Circles - 2" (`XModelCircleImportTests.cs:371-391` expects two groups starting with "... - Circles"). Each range's nodes are linked into the group (additional parents; duplicates skipped).
 - **Faces** (`:218-248`): group `{name} {1} - Faces ` (trailing space kept) > per face `{name} {1} - {face} ` (`ModelType = FaceInfo`) > per item `{name} {1} - {face} - {key}` with `FaceDefinition` colour (when `Color.Name == "ff000000"` i.e. black from HTML, use White) and component; ranges linked.
 - **Legacy state groups** (`CreateLegacyStateGroups = true`, `:250-272`): `{name} {1} - States ` > per state `{name} {1} - {state} ` (`ModelType = StateInfo`) > per item `{name} {1} - {state} - S{index} - {item}`, ranges linked. **These groups exist in addition to the authored definitions.**
 - **Generated groups** (`:274-300`): recursively, `ModelType = SubModel`.
 - (audit) Empty groups are kept: the "Faces " and "States " parents are created whenever the import model has any `faceInfo` / `stateInfo` (`:71-79`), each face and state gets its group even with no items, and each item gets its group even when none of its orders resolves to a light (`:226-245`, `:258-270`). Only the authored definitions drop empty items and empty definitions (`:122-141`). The legacy state item groups are created in the import model's item order (attribute order, F67), **not** sorted by index as the authored items are (`:264` versus `:121`).
 - `EnumerateRange` walks start to end inclusive, descending when start > end (`:333-350`). Unknown orders are skipped silently.
 - The `{1}` token stays in the names on purpose: it is resolved when the prop is placed in a preview (`PropEditorViewModel.NewProp` enforces it for new props too). Keep it.

**F69. Vendor inventory XML** (`CPE/Model/ExternalVendorInventory/*.cs`, `CPE/Import/XLights/XModelInventoryImporter.cs`, `XModelInventoryMapper.cs`). Root `<modelinventory>`: `<vendor>` (`name`, `contact`, `email`, `phone`, `website`, `facebook`, `notes`, `logolink`), `<categories><category>` (`id`, `name`, nested `categories`), `<models><model>` (`id`, one or more `categoryid`, `name`, `type`, `weblink`, `material`, `width`, `height`, `thickness`, `pixelcount`, `pixeldescription`, `pixelspacing`, one or more `imagefile`, `notes`, one or more `wiring` of `name`, `description`, `xmodellink`, `imageFile`). Mapping: web links "Website", "Facebook" when present (`:36-44`); id and pixel count parsed as uint (bad id logged, pixel count silently 0); image = first `imagefile`; model links only for wirings with a non-empty `xmodellink`: `.prp` suffix gives type Prop, else XModel; blank name becomes `Model {n}` counting only blank-named links; `weblink` without `http` prefix gets `http://`; a product joins every category in `categoryid`. `ModelLink` defaults: "Default Model", "No description provided.", Prop (`CPE/Model/InternalVendorInventory/ModelLink.cs:12-17`); a blank description in the XML keeps "No description provided." (`:19-31`). A non-URL website or logo throws and fails that vendor.
 - Audit precision: every field above is a **child element**, not an attribute (`XmlSerializer` with `[XmlElement]`/`[XmlArray]`, `CPE/Model/ExternalVendorInventory/*.cs`), root element name `modelinventory` (`XModelInventoryImporter.cs:10-14`). Category `<id>` and each `<categoryid>` are `uint` elements, so a non-numeric value fails the whole vendor document; the model `<id>` and `<pixelcount>` are strings parsed afterwards.
 - `new Uri(...)` without a guard is used for `<website>`, `<facebook>`, `<logolink>`, the first `<imagefile>` and every non-empty `<xmodellink>` (`XModelInventoryMapper.cs:39-48`, `:88-97`): a relative or malformed URL in any of them throws and the vendor is skipped with the "Error Retrieving Inventory" box. Only `<weblink>` is guarded (try/catch, logged, `:104-118`). The `.prp` test is case-sensitive `EndsWith(".prp")`. The vendor's `<notes>` maps to `Vendor.Description`, which no view shows. Wiring `<imageFile>` elements are read but unused.

### H. Wire diagram export (`App/Modeling`)

**F20. Export Wire Diagram.** Tools > Export Wire Diagram > Front Wire Diagram (flip false) / Back Wire Diagram (flip true) (F13). Enabled when the prop has at least one light node (`CanCreateWireDiagram`, `PropEditorViewModel.cs:1188-1191`). `CreateWireDiagram` (`:1167-1182`): collects light nodes distinct by id, ordered by `Order`; every light of each adds `(int X, int Y, order)`, so a multi-light element contributes several points with the same number. The coordinates are C# `(int)` casts, i.e. **truncated toward zero**, not rounded. The light nodes come from the process-wide view-model lookup (`ElementModelLookUpService.Instance.GetAllModels()`), part of the singleton hazard. Then `ElementModeling.OrderedPointsToSvg(nodes, flip)` (`MODEL/ElementModeling.cs:19-33`): nothing when empty; options dialog; build; write to a temp `.svg` and open with the default app (`:75-86`). Cancel produces nothing. `ElementModeling.ElementsToSvg(elementNode, flip)` (`:41-73`) is the same export for a real element tree, called from outside this area: it needs every leaf to have a Location property; when all have an Order property `OrderNodes` applies, using the order as label, otherwise a running counter labels them. `OrderNodes` (`:88-126`): if orders are all distinct simply sort by order; otherwise group by first non-leaf parent and sort each group (groups in insertion order; leaves with no such parent are dropped).

**F20e. Wire Diagram Export Options dialog.** Title "Wire Diagram Export Options", 440x430, no resize, Escape closes (`MODEL/Views/WireDiagramExportOptionsWindow.xaml:5-12`). Top to bottom: check "Show all node numbers" (default on); "Number Label Interval" text box, right aligned, default "50", disabled while the check is on (`:39-54`); "Node Marker Size" slider 1..30, default 6, value shown at right (`:66-73`); "Number Label Size" slider 6..24, default 14 (`:85-92`); "Scaling" slider 100..400 shown as `{n}%`, default 100 (`:104-111`); explanatory text "The diagram is fitted to a letter-sized page. Scaling scales the entire result, including the page, labels, markers, and lines." (`:114-115`); buttons "OK" (default, disabled with errors), "Cancel". Settings are per export: not persisted. Errors: "Enter a label interval from 1 through 1000.", "Enter a whole-number label interval from 1 through 1000.", "Enter a number-label font size from 6 through 24.", "Enter a node radius from 1 through 30.", "Enter an scaling percentage from 100 through 400." (typo in the original), business rule "The wire diagram export settings are outside their supported ranges." (`MODEL/ViewModels/WireDiagramExportOptionsViewModel.cs:162-204`). Interval text parses with `NumberStyles.None` (digits only). Options: `LabelInterval = AllNumbers ? 1 : interval`, `LabelFontSize`, `NodeRadius`, `ExportScale = percent/100` (`:226-232`). Validation ranges also in `MODEL/WireDiagramExportOptions.cs:38-59`.
 - (audit) Window placement: centred on its owner, not in the taskbar (`WireDiagramExportOptionsWindow.xaml:8-10`); owner is the active WPF window, else the active WinForms form (`WireDiagramExportOptionsWindow.xaml.cs:43-58`). Cancel, Escape or the close box return no options and nothing is exported (`:33-40`).
 - (audit) Validation runs on every change (`WireDiagramExportOptionsViewModel.cs:234-238`), and the interval is checked **only while "Show all node numbers" is off** (`:164-175`). The numeric `LabelInterval` is updated only by valid text (`:72-79`), so with invalid text it keeps its last valid value: unchecking "Show all" with invalid text disables OK; checking it again clears the error and enables OK, and the bad text stays in the disabled box. Because of this the first message ("Enter a label interval from 1 through 1000.") can never appear; only the "whole-number" one does. Empty text, a sign, spaces or a decimal point are all invalid. The three sliders snap to whole numbers (`IsSnapToTickEnabled`, tick 1; `.xaml:71`, `:90`, `:109`). OK re-validates before accepting (`:208-218`).

**F20f. Wire diagram SVG** (`MODEL/WireDiagramSvgBuilder.cs`, tested by `Vixen.Tests/App/Modeling/WireDiagramSvgBuilderTests.cs`). Pure function; port to Rust so it needs no window.
 - Page: letter at 96 dpi, portrait 816x1056 or landscape 1056x816, whichever gives the larger scale (portrait on ties) (`:313-318`). `CanvasMargin = 24`. Document size in inches `canvas/96*ExportScale`; view box `0 0 canvasW canvasH` (`:44-49`). Font Arial at `LabelFontSize` px (`:15`, `:42`).
 - Layout (`:275-322`): bounds of all points. `maxHalfDiagonal` of labelled text boxes (measured width + 2; height = font ascent + descent + 4). `sideGuard = max(radius*1.25, radius + 2*maxHalfDiagonal + 4)`, used on all four sides. Scale (`:324-331`): `min(usableW/width, usableH/height)`, with `usable = canvas - 48 - 2*sideGuard`, infinite on a zero-size axis, 1 when both are zero. Origin centres the content. Note text measuring uses GDI+ `MeasureString`; on the Mac use Core Text or Arial metrics, and accept sub-pixel differences (upstream tests compare structure, not exact label widths).
 - Points (`:54-66`): `x = originX + (srcX-minX)*scale`; `y` likewise; flip mirrors `x = originX + width*scale - (x - originX)`. **Y is not flipped.**
 - Markers (emitted first, in node order, `:68-79`): first node is a green 10-vertex star (outer `radius*1.5`, inner half of that, first vertex pointing up, `:378-404`); last node a red square (side `2*radius`, `:406-416`); when `LabelInterval > 1`, other labelled nodes are red diamonds (`:418-433`); all others red circles of `radius`.
 - Wires (`:81-84`): black 1-wide lines between consecutive points in list order, after the markers.
 - Labels last (`:86-94`): only for `index == 0 || (index+1) % interval == 0 || index == count-1` (`:270-273`); text is the node's order number, black. Placement (`:119-160`): eight directions in order NE, E, SE, S, SW, W, NW, N (y down), two distances each (gap 2 then 4): `distance = radius + |ux|*halfW + |uy|*halfH + gap*mult`; candidate ordinal increments in that order. Keep candidates inside the page by 2.5 clearance. Among those with no wire intersection (clearance 2.5) pick least overlap area with earlier labels (bounds 1.5x half sizes), then shortest distance from the label's start to the node, then ordinal. Otherwise among in-bounds pick fewest intersections, then the same keys. No in-bounds: shortest distance, then ordinal. Angle is always 0.
 - Text baseline `y = centerY + (ascent-descent)/2`, ascent and descent from font metrics plus 2 each; `x = centerX - width/2`.
 - (audit) Document shape and edge cases: `Build` validates the options first and throws when out of range, then returns no document for an empty list (`:31-38`). Everything sits in one `<g>` (`:51-52`), in the order markers, wires, labels. Markers have a fill and no stroke; each wire has stroke black, width 1 (and a black fill, which has no effect on a line) (`:343-355`); label text carries font family Arial and `font-size` = the option value (the font is created in pixels, `:42`, `:358-370`); a rotate transform would be added only for a non-zero angle, which never happens (`:371-374`). A single node gets the green star only (index 0 wins over "last", `:71-77`) and one label. Label text is the order number passed in, so equal orders print equal labels. The wire list used for label clearance is every wire including the two that end at the labelled node (`:151`). Label overlap is measured against label boxes shrunk to three quarters of their real size (width `1.5 * halfWidth`, `:187-197`), so labels may touch at the edges without penalty. Coordinates are computed in double precision and stored as single precision (`:58-65`).
 - (audit) `ElementsToSvg` cost: `OrderNodes` tests `nonLeafs.Contains(parent)` against a lazy enumerator for every leaf (`MODEL/ElementModeling.cs:100-103`), which walks the group tree again per leaf. Build a set of group ids once in the port.

### I. Polygon Editor: host window and data model

The Polygon Editor is one modal window that effects open to edit shapes. Which tools it shows depends on `PolygonEditorCapabilities` passed by the caller. Rendering of the resulting shapes lives in `crates/fx-pixel-physics` and friends; Morph is the main client. Callers outside the area (effect editors) build a `PolygonContainer`.

**F70. Polygon Editor window.** `POLY/Views/PolygonEditorView.xaml`. Title "Polygon Editor", centred. Top: toolbar. Middle: `PolygonControl` (canvas, point grid, optional time bar). Bottom right: "OK" (default) and "Cancel" (`:29-32`). Delete key: `DeleteShapeOrPointsCommand` (`:24-26`). Size (`POLY/Views/PolygonEditorView.xaml.cs:46-167`): canvas scale fits the display element into 70% of the primary monitor: `factor = min(0.7*screenW/containerW, 0.7*screenH/containerH)`; `CanvasWidth = ceil(containerW*factor)`, `CanvasHeight = ceil(containerH*factor)`. Window width = canvas width, at least 70% of screen width. Window height = canvas height (+190 when the time bar shows), at least 25% of screen height. On the Mac use the current display of the owning window and the same percentages; keep the scale fixed at open (the original never rescales while open).

**F71. Coordinate mapping.** `xScale = (CanvasWidth-1)/(ContainerW-1)`, or `CanvasWidth-1` when the container is 1 wide; same for y (`:73-95`). Display element outline inside the canvas (the container may include a margin): `DisplayElementWidth = (DisplayElementW-1)*xScale`, `DisplayElementHeight` likewise; origin `(canvas - size)/2 - 1` (`:98-101`). `ShowDisplayElement` controls whether that outline is drawn (`:104`). Point grid values use `PolygonPointXConverter`/`YConverter` with these scales and `BufferHt = container height`: **the grid shows display-element coordinates with Y flipped (origin bottom left)**; details under the point grid entry.

**F72. Results.** On OK the caller reads `Polygons`, `PolygonTimes`, `Lines`, `LineTimes`, `Ellipses`, `EllipseTimes` (`:177-341`). In time-bar mode shapes come from the snapshots, otherwise from the model collections. Getting polygons or lines scales points by `1/scale` and **rounds them to whole display-element pixels**; ellipses scale points, centre, width and height, and are **not rounded** (rounding would distort the rectangle; `PMOD/Ellipse.cs:92-103` asserts). **These getters mutate the shapes, so call them once.** In Rust this becomes one `finish()` returning a new `PolygonContainer`.

**F73. Polygon data model.** `PMOD/`, serialized inside effect data via DataContract and already needed by effect parsing. Types:
 - `Shape { ID: Guid = new, Label: string }` (`PMOD/Shape.cs`).
 - `PointBasedShape : Shape { Points: Vec<PolygonPoint>, FillType = Wipe }`, enum `PolygonFillType { Wipe, Solid, Outline }` (`PMOD/PointBasedShape.cs:8-13`, `:27-34`). Methods: `IsShapeOffDisplayElement(w,h)` true if any point has `X > w-1`, `Y > h-1`, `X < 0` or `Y < 0` (`:93-114`); `LimitPoints(w,h)` clamps only the high side (`:119-138`); `MovePoints`, `ScalePoints`, `RoundPoints` (`Math.Round`, so banker's rounding: **match it**) (`:143-180`).
 - `Polygon`, `Line`: `Clone()` deep-copies with a new id (`PMOD/Polygon.cs:28-38`, `PMOD/Line.cs:28-38`).
 - `Ellipse { Angle: f64 degrees, Center: PolygonPoint, Width, Height: f64, StartSideRotation: i32 }`, `Points` holding the four corners of the surrounding rectangle (`PMOD/Ellipse.cs`). `Scale` un-rotates about centre, scales points, centre, width and height, re-rotates about the new centre; while any corner is outside the display element it re-scales by `(w-1.0)/w` and `(h-1)/h` (**quirk: the y factor is `(height - 1) / height` with `int` operands, i.e. integer division, which is 0 for every positive height** (corrected by the audit; it read "0 unless height is 1"), so the first pass of the `while` loop collapses every y onto 0 (`PMOD/Ellipse.cs:108-121`); the loop itself is `while (IsOutsideLimits(width, height))`, `:179-201`). Rotation uses WPF `RotateTransform(angle, cx, cy)` (positive is clockwise on screen with y down).
 - `PolygonPoint { X, Y: f64 }` (`PMOD/PolygonPoint.cs`).
 - `PolygonContainer` (`PMOD/PolygonContainer.cs`): not serialized, passed between effect and editor. Parallel lists `Polygons`/`PolygonTimes`, `Lines`/`LineTimes`, `Ellipses`/`EllipseTimes`; `Width`, `Height` (with margin), `DisplayElementWidth`, `DisplayElementHeight` (without), `ShowDisplayElement`, `EditorCapabilities`. `Clone` deep-copies keeping ids and shares the capabilities reference.
 - `PolygonEditorCapabilities` (`PMOD/PolygonEditorCapabilities.cs`), all bool: `DeletePoints`, `AddPolygons`, `AddPoint`, `PastePolygons`, `CopyPolygons`, `CutPolygons`, `DeletePolygons`, `DefaultToSelect`, `ToggleStartSide`, `ToggleStartPoint`, `ShowStartSide`, `ShowTimeBar`, `AllowMultipleShapes`.

**F73a. Who opens the editor, and with which capabilities (Morph).** The only caller is the Morph effect. It goes through `Vixen.Modules/Editor/EffectEditor/Editors/PolygonContainerTypeEditor.cs`. Morph itself is outside this area and was read for context.

Morph picks one fixed capability set per `PolygonType` (`Vixen.Modules/Effect/Morph/Morph/Morph.cs:125-175`):

| Capability | Pattern | FreeForm | TimeBased |
|---|---|---|---|
| `DeletePolygons` | no | yes | yes |
| `DeletePoints` | yes | yes | yes |
| `AddPolygons` | no | yes | yes |
| `PastePolygons` | no | yes | yes |
| `CutPolygons` | no | yes | yes |
| `CopyPolygons` | no | yes | yes |
| `DefaultToSelect` | yes | yes | no |
| `ToggleStartSide` | yes | yes | no |
| `ToggleStartPoint` | yes | yes | no |
| `ShowStartSide` | yes | yes | no |
| `ShowTimeBar` | no | no | yes |
| `AddPoint` | yes | yes | yes |
| `AllowMultipleShapes` | no | yes | no |

**Getting the container** (`Morph.cs:410-474`):
- Each shape is cloned, but its `ID` is kept.
- Times are copied in the same order as the shapes.
- `Width` and `Height` are the render buffer, which includes the margin.
- `DisplayElementWidth` and `DisplayElementHeight` are `BufferWi` and `BufferHt`.
- `ShowDisplayElement` is `Margin != 0`.

**Setting the container** (`Morph.cs:476-523`, `:1692-1760`):
1. Every existing morph shape is marked removed.
2. Each returned shape is matched to an existing morph shape **by `ID`**. A match keeps that shape's other effect settings and replaces its geometry. A shape with no match becomes a new morph shape.
3. A polygon whose fill is `Wipe` but whose point count is not 3 or 4 is changed to `Solid`.
4. In TimeBased mode the time comes from the parallel list. Otherwise the time is 0.
5. Morph shapes that are still marked removed are deleted.
6. In TimeBased mode the shapes are sorted by time. In Pattern mode the effect's `FillType` is copied from the first shape.

**The type editor** (`PolygonContainerTypeEditor.cs:36-80`):
- It clones the value and opens the editor as a modal dialog.
- On OK it builds a **new** container from the six result lists plus `Width` and `Height`. It does not copy `DisplayElementWidth`, `DisplayElementHeight`, `ShowDisplayElement` or `EditorCapabilities`; Morph's getter fills them in again the next time.
- On any exception it shows "An error occurred loading the polygon editor." titled "Polygon Error", and returns the old value.

**Port requirements:**
- The editor must keep `ID`, `Label`, `FillType` and `StartSideRotation` on every shape it returns. If an `ID` changes, Morph loses that shape's per-shape settings.
- The editor must not invent ids for shapes that the user only moved.
- `fx-pixel-physics` (`morph/polygon.rs` and `generated/morph.rs`) depends on this.
- `docs/spec/effect-editor.md` F37 owns only the entry point; this spec owns the editor.

**F74. Toolbar** (`POLY/Views/PolygonEditorView.xaml:34-114`), in this order, with tooltips:
 - Radio group `SelectDraw`: "Draw Line", "Draw Polygon", "Draw Ellipse" (visible `ShowSelectDraw` = `AddPolygons`); "Add Point" (visible `ShowAddPoint` = `AddPoint`); "Selection mode" (visible `ShowSelect` = either of those) (`POLY/ViewModels/PolygonEditorViewModel.cs:796-817`).
 - Buttons: "Paste Polygon/Ellipse/Line" (visible `ShowPaste` = **`AddPolygons`**, not `PastePolygons`, `:822-825`), "Cut Polygon/Ellipse/Line" (`CutPolygons`), "Copy Polygon/Ellipse/Line" (`CopyPolygons`), "Delete Polygon/Ellipse/Line" (`DeletePolygons`), "Toggle Start Side" (`ToggleStartSide`), "Toggle Start Point" (`ToggleStartPoint`).
 - Always visible: "Convert to Line", "Convert to Polygon", "Convert to Ellipse".
 - Toggle "Shows or hides shape labels" (radio in its own group `ShowLabels`, visible `ShowAddPoint`) (`:107-112`). SnapToGrid button is commented out in the xaml, `SnapToGridCommand` still exists.
 Mode flags are mutually exclusive: setting one clears the rest (`POLY/ViewModels/PolygonEditorViewModel.cs:481-647`). Entering Draw Polygon creates a new partial polygon when none exists, selects it, deselects other polygons and removes the resize adorner. Leaving Draw Polygon or Draw Line removes the partial shape (`RemovePartialPolygon`, `RemovePartialLine`, `:1794-1821`; citation corrected by the audit, it read `:794-822`). Entering Draw Ellipse deselects all shapes. Setting `EditorCapabilities` starts in selection mode when `DefaultToSelect` or the time bar is on (`:986-991`), and `PolygonViewModel.ColorStartSide = !ShowTimeBar` (`:981`). The constructor starts with a new polygon in progress (`:57-62`). **Load auto-select (audit addition):** outside time-bar mode, when `DefaultToSelect` is set and the loaded container holds exactly one polygon, that polygon is selected with all its points in the grid; the same test then runs for exactly one line, then exactly one ellipse (`PEVM:4162-4166`, `:4203-4207`, `:4244-4248`). They run in that order and each selection clears the other kinds, so with one polygon and one line the line ends up selected. When the control finishes loading it calls `DisplayResizeAdornerForSelectedShape` (`POLY/Views/PolygonControl.xaml.cs:185-199`, `PEVM:1521-1529`), so the resize adorner is already on the auto-selected shape when the window appears (in time-bar mode: on the first snapshot's shape). Pattern mode (one shape, no drawing tools) therefore opens with its shape already selected.

Alias for this section: `PEVM` = `POLY/ViewModels/PolygonEditorViewModel.cs`.

**F75. Drawing a polygon.** In Draw Polygon mode each left click on the canvas runs `AddPolygonPoint(click)` (`PEVM:1941-1944`, `:3969-4006`), forwarding to the `NewPolygon` view model (see F85). While the polygon is open a ghost point and ghost line follow the mouse (`PEVM:3750-3763`). When the polygon closes the ghost hides, then (`PEVM:3974-4003`):
 - **in time-bar mode** the snapshot takes the polygon and is re-selected; `NewPolygon = null`; **switches to selection mode**; refreshes shapes;
 - **otherwise (correction by the audit) `AddNewPolygon()` starts the next polygon and the editor stays in Draw Polygon mode**, so FreeForm users can draw several polygons in a row; the closed polygon is not selected. Mouse leaving the canvas while drawing throws the partial polygon or line away and starts another (`PEVM:3719-3738`). Cursor is a cross when hovering the first point with at least two points placed (`PEVM:3592-3597`).

**F76. Drawing a line.** Draw Line mode: click adds a point (`PEVM:1947-1950`, `:4013-4052`); the second point completes it, sets `StartPoint`/`EndPoint` and hides the ghost. **Correction (audit):** only in time-bar mode does it attach to the snapshot and switch to selection mode; otherwise `AddNewLine()` starts the next line and the editor stays in Draw Line mode (`:4030-4052`). Ghost line after the first point. Leaving the canvas while drawing throws the partial line away (`:3719-3738`).

**F77. Drawing an ellipse.** Draw Ellipse mode: click calls `AddEllipse(click)` (`PEVM:2005-2081`): a default ellipse **20% of the smaller canvas dimension across, centred on the click**: corners at `(int)(click ± smallest*0.1)`, each clamped to the canvas, in order top-left, top-right, bottom-right, bottom-left. Adds to the collections, initializes the green line. In time mode: `FillType = Solid`, green line hidden, attaches to the snapshot, re-selects the snapshot, and goes to selection mode. Outside time mode the mode stays Draw Ellipse, so further clicks add more ellipses.

**F78. Adding a point to a polygon.** Add Point mode (`AddPoint`, enabled when any polygon exists, `PEVM:2444-2448`): click over a line segment of a **closed** polygon inserts a point after the segment's start (`InsertPoint`), selects the polygon, and with more than 4 points hides the green wipe line (`PEVM:1911-1936`). Hover cursor is the special `Cursors/cursor_arrow_dragbox.cur` over a segment of a closed polygon (`PEVM:3616-3628`). `IsMouseOverLine` is a delegate set by `PolygonControl`, see the canvas entry.

**F79. Selection mode: click.** `SelectPolygonLineOrPoint(click)` (`PEVM:3882-3963`) deselects all, then tests in this order: polygon point (5 px Chebyshev tolerance, `PEVM:1716-1722`), polygon centre cross hash, line point, line centre cross hash, ellipse centre cross hash. A point selects only that point and starts `MovingPoint` with a ghost at the point; a centre cross hash selects the whole shape and all its points (`shapeSelected = true`, which suppresses the lasso). Otherwise everything deselects. **Ellipse corners cannot be picked individually.** After a click, whole-shape selection shows the resize adorner (`PEVM:1970-1996`). Mouse is captured for the drag (`PEVM:1961`). A "centre cross hash" hit is a point within 5 px on both axes of the shape's centre point (`ShapeViewModel.IsOverCenterCrossHash`, `POLY/ViewModels/ShapeViewModel.cs:268-288`); the 40-px cross arms (and a line's 20-px circle) are drawn only and are not hit targets. Clicks at `x >= ActualWidth` are ignored (`PEVM:1899`).

**F148. Polygon canvas cursors (audit addition).** `UpdateCanvasCursor` runs on every mouse move (`PEVM:3581-3632`), first match wins: cross when a new polygon has 2+ points and the pointer is within 10 px of its first point (closing is possible); cross over any shape's centre (5 px) in selection mode; `SizeAll` over a polygon or line point, or any centre, in selection mode (`IsMouseOverMoveableItem`, `:1234-1279`); the `cursor_arrow_dragbox.cur` add-point cursor over a segment of a closed polygon in Add Point mode; otherwise the arrow. The time bar has its own cursor (F84e). Mac: `NSCursor.crosshair`, an open-hand or move cursor for `SizeAll`, and the add-point image cursor.

**F80. Moving a point.** During the drag the original point stays put and a ghost point plus ghost lines to the previous and next points follow the mouse, clamped to the canvas (`MovePoint`, `PEVM:4360-4399`; polygons use both neighbours, lines only the other end). On mouse up `EndMoveSelectedPoint` clamps and moves the real point (`PEVM:4331-4354`).

**F81. Lasso.** In selection mode with the button down, no point move and no shape picked: rubber band (`PEVM:3787-3795`). On release (`PEVM:2106-2169`): clear selected points, `SelectShapePoints(rect)` over polygons and lines only (`PEVM:4521-4541`; **ellipse points are excluded** because moving a corner would distort the rectangle). More than one selected: if they are exactly one whole polygon's points, select that polygon (`PEVM:4460-4483`), then show the resize adorner for the selected points. Exactly one: select as a point on its polygon or line. Points may therefore span several shapes.

**F82. Resize, move and rotate adorner (`IResizeable`).** Handles: see F89 (`POLY/Adorners/ResizeAdorner.cs`). The view model enforces limits: `IsMoveable(offset)` needs every selected point to stay within `0..width-1`, `0..height-1` (`PEVM:4693-4727`). `IsTransformValid(group)` rounds to 10 decimals before the same test (`PEVM:4819-4861`). `IsRotateable(angle, center)` rejects when any point leaves the canvas, **and in that case removes the adorner and deselects** (`PEVM:4783-4814`). `RotateSelectedItems` also adds the angle to `SelectedEllipse.Angle` (`PEVM:4732-4764`). `TransformSelectedItems` clamps each point after transforming (`PEVM:4866-4890`).

**F83. Cut, Copy, Paste, Delete.** Copy (enabled `IsShapeSelected`): puts a clone on the clipboard, one of the private formats "PolygonClipboardFormat", "LineClipboardFormat", "EllipseClipboardFormat" (`PEVM:108-118`, `:3378-3398`). A polygon counts as selected only in selection mode with all points selected; a line or ellipse only needs to be selected (`PEVM:3454-3484`). Cut = Copy + Delete, enabled by `CanExecuteDelete` (shape selected and `DeletePolygons`) (`PEVM:3203-3207`, `:3432-3439`). Paste (enabled when any format is on the clipboard and `AddPolygons`, and in time mode only when the current snapshot has no shape, `PEVM:3404-3426`): checks Line, Polygon, Ellipse in that order, clones (new id), moves it so its centre sits at the canvas centre with clamping (`MovePoints(dx, dy, LimitPointToCanvas)`), selects it, attaches in time mode, shows the adorner (`PEVM:3212-3372`). Delete: removes the selected polygon, line, ellipse; in time mode clears the snapshot's shape (`PEVM:2529-2579`). **Quirk: after removing a line it clears `SelectedPolygon`, not `SelectedLine`, leaving a stale line reference until `RaiseRemoveResizeAdorner` deselects** (`PEVM:2550`).
 - **The clipboard is the system clipboard** (WPF `Clipboard.SetData`/`GetData`/`ContainsData` with those format names, `PEVM:3219-3393`, `:3409-3411`), not editor state: a shape copied in one Polygon Editor session can be pasted in a later session for another Morph effect, and each Copy replaces whatever was on the clipboard. On the Mac use a private `NSPasteboard` type holding the shape as JSON.
 - Paste does not change the shape's fill type, even in time-bar mode (only drawn shapes become Solid there, F84e). The pasted centre is `(ActualWidth/2, ActualHeight/2)` of the canvas.
 - There is no keyboard shortcut for Cut, Copy or Paste in the original; only Delete is bound (`POLY/Views/PolygonEditorView.xaml:24-26`). ⌘X/⌘C/⌘V on the Mac are additions to record in the parity file.

**F84. Delete key.** `DeleteShapeOrPoints` (`PEVM:2584-2633`). If a shape can be deleted, do that. Otherwise for each selected point: find its polygon; when found and it has more than 3 points, delete the point, and outside time mode with a Wipe polygon at 3 or 4 points show the green line again; points on lines or ellipses or on 3-point polygons are skipped. Then removes the adorner. Note this ignores `DeletePoints`: that capability is only set by callers.

**F84a. Convert to Polygon, Line, Ellipse.** Always shown. With `AllowMultipleShapes`: enabled in selection mode when a shape of another kind is selected (`PEVM:3500-3575`), and only that shape is replaced. Otherwise enabled in selection mode when no shape of the target kind exists, and all other shapes go.
 - To Polygon (`PEVM:2691-2754`): a new polygon covering the whole canvas: points (0,0), (w-1,0), (w-1,h-1), (0,h-1), then (0,0) again, which closes it. Time mode: Solid and attached to the snapshot. Not selected afterwards.
 - To Line (`PEVM:2759-2840`): vertical line at x = w/2 from y = h*0.2-1 to y = h*0.8-1; start point green; time mode Solid and attached; selected with the adorner. **Quirk: it writes through `Lines[0]`, so with `AllowMultipleShapes` and other lines present it modifies the first line in the list rather than the new one; on the Mac use the new line.**
 - To Ellipse (`PEVM:2845-2892`): `AddEllipse` at the canvas centre (time mode clears other snapshot shapes first).
 Converted shapes do not derive from the source shape's geometry.

**F84b. Toggle Start Side** (enabled when a polygon or ellipse is selected, `PEVM:2897-2918`), forwarded to the shape (F86, F88). **Toggle Start Point** (enabled when a line is selected): `SelectedLine.ToggleStartPoint()`, then deselect and remove the adorner (`PEVM:2676-2686`).

**F84c. Toggle labels.** `ToggleLabels` (`PEVM:2639-2671`): because the control is a radio button whose `IsChecked` is two-way, a click sets `ShowLabels = true` first; then if it had been true previously, sets false. Applies `LabelVisible` to every shape. Mac form: plain toggle. Default off.

**F84d. OK and Cancel.** OK (`PEVM:2453-2484`): `SnapToGrid` (polygons and lines only; rounds in display-element pixels, see below; clamps), removes the last polygon if not closed and the last line unless it has 2 points, then closes with a positive result. `SnapToGrid` does nothing while `ActualWidth` or `ActualHeight` is still 0 (`PEVM:2502`), that is, when the canvas size was never reported. Cancel closes with no result, and the caller keeps its original container only if it passed a clone (the view scales the passed models in place on load, `PEVM:4072`, `:4140`). In Rust always edit a copy.

**F84e. Time bar (Morph TimeBased mode).** (Title corrected by the audit: it read "pattern mode"; only Morph's TimeBased capability set turns `ShowTimeBar` on, `Morph.cs:161-176`.) Shown when `ShowTimeBar` (`TimeBarVisible`). See the `PolygonControl` and `PolygonTimeBar` entries for its controls.
 - Snapshots: `PolygonSnapshots`, ordered by time, each holding at most one polygon, line or ellipse, plus `NormalizedTime` 0..1 (`POLY/ViewModels/PolygonSnapshotViewModel.cs`). Load: `InitializePolygonSnapshots` creates one snapshot per shape (polygons, then lines, then ellipses, **with no sort by time on load**) and selects the first (`PEVM:4059-4114`). **Resolved (audit): an empty container with `ShowTimeBar` crashes the open.** `InitializePolygonSnapshots` ends with `SelectPolygonSnapshot(PolygonSnapshots[0])` (`PEVM:4113`), which throws when there are no shapes; the type editor's catch-all then shows "An error occurred loading the polygon editor." / "Polygon Error" (`PolygonContainerTypeEditor.cs:38-74`) and the editor never opens. A user reaches this by deleting the shape from every snapshot and pressing OK: Morph then has no shapes, and it only re-creates a default shape when the target nodes change (`Morph.cs:952`). Port: open with one empty snapshot at time 0 instead, and record it in the parity file.
 - `AdjustedTimeBarActualWidth = TimeBarActualWidth - 10` (`PEVM:788-791`). Pixel position = `(int)(t*adjusted) + HalfWidth` (`PEVM:2187`, `:4125`).
 - Select: click on a marker selects and starts a drag (`PEVM:3101-3120`). **Marker hit test (audit precision):** only X is tested, between the marker rectangle's left and right edges (centre ± `HalfWidth` = 4 px); Y is ignored, and the first snapshot in list order wins (`PolygonSnapShotViewModel.cs:159-180`, `PEVM:1171-1194`). Mouse-up applies one last move at the release position and ends the drag (`PEVM:3126-3143`); `PolygonSnapshotViewModel.Move(x)` ignores positions below `HalfWidth` (`PolygonSnapShotViewModel.cs:186-208`). Selecting a snapshot rebinds the editor collections to its one shape, selects that shape with the adorner, or clears selection when it has none (`PEVM:1328-1407`).
 - Drag: `MovePolygonSnapshot` clamps to `HalfWidth..adjusted+HalfWidth`, sets `NormalizedTime = (pos-HalfWidth)/adjusted`, re-sorts by pixel `Time`, updates next/prev buttons (`PEVM:3159-3198`). Mouse leaving the bar ends the drag (`PEVM:3148-3152`). Cursor `SizeWE` over a marker (`PEVM:3058-3072`).
 - Add ("+"-style command `AddPolygonSnapshot`): `time = (selected.Time + 20)/adjusted`, capped at 1; inserted after the selected snapshot; **the selected snapshot's shape is cloned into it with a new id**; selected (`PEVM:2923-2972`). **Precision (audit):** `selected.Time` is the marker's pixel position, which already includes `HalfWidth` (4), and the new marker is placed at `(int)(time*adjusted) + HalfWidth` (`PEVM:2187`), so the new marker lands 24 px to the right of the selected one and its normalized time is `t + 24/adjusted`. The list is **not re-sorted** on Add (the new snapshot sits at `index + 1` even when a later snapshot is less than 24 px away); the order is only repaired by the next marker drag. Add has no enable rule (no `CanExecute`, `PEVM:66`).
 - Delete: enabled with more than one snapshot; selects the previous, or the new first (`PEVM:2977-3000`, `:3490-3494`).
 - Next and Previous: enabled when not at the end or start (`PEVM:3005-3052`).
 - Drawing and pasting are only enabled when the selected snapshot has no shape (`PEVM:2422-2438`, `:3416-3422`). New shapes get `FillType = Solid`.
 - Results: times returned only for snapshots with that shape type, in list order (`PEVM:4256-4325`).

**F85. Polygon shape rules** (`POLY/ViewModels/PolygonViewModel.cs`).
 - Build from a model with 3+ points: treated as closed immediately; green segment between points 0 and 1; `SegmentsVisible = FillType == Wipe` (`:37-58`). **Note this means a 2-point saved polygon stays "open".**
 - Add point (`:242-285`): the point goes on the model and the VM. With 2+ points a segment is added between the new and previous points. If the new point is within **10 px Chebyshev of the first point** (`IsMouseOverFirstPolygonPoint`, `:294-306`) the new point is removed and the polygon closes. **So clicks near the first point with only one prior point also close it, giving a 1-point "closed" polygon; the cursor only shows the close hint at 2+ points.** Keep the rule (or block closing below 3 points and record the difference).
 - Close (`ClosePolygon`, `:373-415`): when `ColorStartSide` (off in time mode): keep only the first segment; 3 or 4 points colours it Lime (start side of the wipe); other counts with Wipe fill change `FillType` to Solid and hide the line. When not colouring: hide all segments. Then visible.
 - Toggle start side (`:311-337`): rotates the point list by one, first point to end, on VM and model; green segment rebuilt between new points 0 and 1.
 - Delete point (`:342-354`): removes from VM and model, rebuilds the green segment.
 - Insert point (`:219-236`): inserts at index on VM and model.
 - Centre (`:420-447`): bounding box centre when 3+ points; point labels are the 1-based index (`ShapeViewModel.cs:388-395`).
 - Neighbours for the ghost lines wrap around (`:171-212`).

**F86. Line shape rules** (`POLY/ViewModels/LineViewModel.cs`). Build: with 2+ points start = first; **Wipe lines colour the start point Green on load** (`:40-45`), while drawing colours the first point Lime (`:140`), convert colours Green (`PEVM:2810`). End = second when exactly 2 (`:49-58`). Toggle start point (`:151-179`): swaps start and end on VM and model; the old start's deselected colour becomes DodgerBlue and the old end's Lime. Centre is the midpoint (`:209-220`).

**F87. Ellipse shape rules** (`POLY/ViewModels/EllipseViewModel.cs`). Points are the four rectangle corners; **VM points have no parent, so they don't auto-notify** (`:38`). Centre = bounding box centre **of the four corners** (`:85-108`; note: after rotation this is still the true centre for a rectangle). Width and height derive from corner distances after accounting for `StartSideRotation`: indices `ptZero = 0-r`, `ptOne = 1-r`, `ptTwo = 2-r`, each +4 when negative; `Width = |p1-p0|`, `Height = |p2-p1|`; `Left = cx - w/2`, `Top = cy - h/2` (`:236-288`). Drawn as an ellipse of `Width`x`Height` at `Left,Top` rotated by `Angle` about the centre. Toggle start side (`:309-345`) rotates the corner list as for polygons and increments `StartSideRotation` modulo 4, so the green side moves while width and height stay right. `SegmentsVisible = FillType == Wipe` (`:294-304`). Labels: 1-based corner index.

**F88. Point and marker colours** (`POLY/ViewModels/PolygonPointViewModel.cs`, `ShapeViewModel.cs`). Point default and deselected DodgerBlue, selected HotPink (`PolygonPointViewModel.cs:25-26`, `:133-142`). Centre cross hash black, HotPink when the shape is selected (`ShapeViewModel.cs:190`, `:211`). Segments DodgerBlue by default, Lime for the start side (`LineSegmentViewModel.cs:28`). Time markers charcoal RGB(68,68,68), HotPink selected (`PolygonSnapShotViewModel.cs:26-29`, `:107-116`). Snap: `SnapToGrid` rounds `p/scale` to an int with `Math.Round` (banker's), back to canvas coordinates, clamps the high side (`ShapeViewModel.cs:296-321`).

**F89. Polygon resize adorner** (`POLY/Adorners/ResizeAdorner.cs`), differs from the prop one (F45):
 - Built with an initial rotation angle (the ellipse's angle), 10x10 thumbs, the rotate handle above the top at `Bounds.Y - 3*height` (`:473`; the prop one uses 5x).
 - Denominators guard zero size with 1 (`:181-206`), so straight lines can scale.
 - Corners: same as the prop editor, Shift locks aspect. Validity goes through `IsTransformValid` of reverse-rotate, scale, rotate (`:390-398`), not `EnforceSize`.
 - Scale also re-centres the rotation centre while angle is 0 (`:410-414`).
 - Centre drag: `IsMoveable(offset)`; when refused `_dragStart` is not updated, so motion catches up (`:160-175`).
 - Rotate: same angle formula as F45 **without snap detents**; `IsRotateable` failure cancels the drag (`:361-381`).
 - `DragCompleted` calls `DoneRotating` (except for the centre drag), which fires `RepositionResizeAdorner` when points remain selected (`:122-125`, `PEVM:4769-4778`).
 - Handle fill and stroke styles: `POLY/Themes/Theme.xaml`.
 - **Scale maths (audit addition).** Each corner drag builds `ScaleTransform(1 + dx/W, 1 + dy/H)` anchored at the opposite corner of `Bounds`, with the sign of `dx`/`dy` flipped for handles on the left or top; Shift sets both factors to `max(scaleX, scaleY)` (`:209-297`). Side handles scale one axis only, anchored on the opposite side (`:299-359`). `dx`, `dy` are the WPF thumb drag deltas since the previous event, so every mouse move applies a small incremental scale. A refused scale (`IsTransformValid` false) is simply dropped; the drag continues.
 - **Ellipse adorner (audit addition).** For an ellipse the bounds are those of the four corners after rotating them by `-Angle` about the ellipse centre, and the adorner is created with `rotationAngle = Angle` so it draws rotated with the ellipse (`POLY/Views/PolygonControl.xaml.cs:256-326`, `ResizeAdorner.cs:502-509`). The adorner object is **kept on the ellipse view model** (`EllipseViewModel.ResizeAdorner`) and reused each time that ellipse is selected again, so its bounds and rotation centre carry over between selections; polygons, lines and lasso selections get a fresh axis-aligned adorner (angle 0) every time. In Rust: store the un-rotated frame and angle with the ellipse and derive the handles from them; for other selections compute the bounding box of the selected points (`GetSelectedContentBounds`, `PEVM:4598-4643`).
 - While the rotate thumb is being dragged, `Bounds` updates and re-arrangement are suppressed (`:133-146`, `:458`); the rotated look comes only from `GetDesiredTransform`. After a rotation of a polygon, line or lasso selection, `DoneRotating` rebuilds a new axis-aligned adorner around the rotated points (`PEVM:4769-4778`, `PolygonControl.xaml.cs:96-100`).

**F90. Polygon canvas** (`POLY/Views/PolygonControl.xaml`, `POLY/Views/PolygonCanvas.cs`). Left of the panel, in a uniform-stretch `Viewbox` (so it visually scales with the window while logical coordinates stay at `CanvasWidth`x`CanvasHeight`, `PolygonControl.xaml:76-82`); background #444444. Layers, back to front (`PolygonCanvas.cs:104-155` then `PolygonControl.xaml:88-503`):
 - Grid: dashed LightGray 1 px lines; spacing `max(w,h)/10`; final lines at the far edges; solid LightGray ticks of length 10 inward on all four sides every `spacing/5`.
 - Ghost lines: dashed (2 2) DodgerBlue 1 px; ghost point 6x6 LightGray (visible with the previous ghost line).
 - Display element outline: red rectangle at the F71 position when `ShowDisplayElement`.
 - Polygon labels: 24-pt `Label` at centre + (5,5), when `LabelVisible`.
 - Polygons: stroke DodgerBlue 3 px, fill DodgerBlue 50%, hidden until closed. Centre cross: two 40-long black or HotPink 1 px lines. `Segments` (DodgerBlue while open, Lime start side), 2 px. Points 7x7 squares in the point's colour, centred; text labels at point + (5,5).
 - Ellipses: label as polygons; `Ellipse` shape `Width`x`Height` at `Left`,`Top`, stroke DodgerBlue 3, fill 50%, rotated by `Angle` about its own centre; dotted (1,1) 1 px rectangle through the corners; centre cross; segments 3 px; 7x7 corner squares; text labels. **Ellipse has no point hit target (only the centre).**
 - Lines: DodgerBlue 3 px; centre cross **plus a circle of radius 20** around the centre, same colour; 6x6 points; labels. **Line labels (shape name) are not drawn by the xaml**, only polygons and ellipses have the big label layer.
 - All shape visuals have `IsHitTestVisible = false`; hit testing is all computed in the view model (`PEVM:1171-1279`), except segment hover for Add Point, which uses invisible (opacity 0) 4 px WPF lines added to the canvas per polygon segment (`POLY/Views/PolygonControl.xaml.cs:359-375`). **On the Mac replace these with a distance-to-segment test at 2 px half-width in Rust (see hazards: the original leaks a line per rebuild, `UpdatePolygonLines` clears the list but never removes the old child from the canvas).**
 - Segment list when rebuilt: consecutive pairs, plus last-to-first (`POLY/Views/PolygonControl.xaml.cs:381-406`).

**F91. Side panel** (`PolygonControl.xaml:506-613`), fixed 120 wide, right of the canvas:
 - "Name:" label and a text box bound to `SelectedShape.Label` per keystroke, white background, showing the fallback text "Select Shape" when nothing is selected (`:507-508`).
 - Point grid: columns "#" (read-only, `Label`, dark grey background), "X", "Y". Items are `SelectedPoints`. Read-only when `SelectedPointsReadOnly` (true for ellipses and initially, `PEVM:98`, `:1511`; false for polygons, lines and lasso, `:1438`, `:1474`, `:4540`).
 - X and Y use the converters (see F92) and validation rules: out of range shows a red text popup under the cell and an error mark on the row. `CellEditEnding` marks `GridEditMade`; `CurrentCellChanged` then redisplays the adorner for selected points when more than one is selected (`PEVM:1826-1852`).

**F92. Point grid coordinate conversion.** `POLY/Converters/PolygonPointXConverter.cs`, `PolygonPointYConverter.cs`, `POLY/ValidationRules/XValidationRule.cs`, `YValidationRule.cs`; values below read from those files (entry F92a).

**F93. Time bar view** (`POLY/Views/PolygonTimeBar.xaml`, `POLY/Views/PolygonTimeBarCanvas.cs`). Under the canvas when `TimeBarVisible`, height 40, width `WindowWidth`, in a `Viewbox`; grey background with black border. Markers are 6-point polygons (rectangle 8 wide x 20 tall plus a point to y = 25) filled charcoal or HotPink (`POLY/ViewModels/PolygonSnapShotViewModel.cs:214-258`). Below it a row: left-arrow button (previous), right-arrow button (next), "Add Frame", "Delete Frame" (`PolygonControl.xaml:58-67`).

**F92a. Grid value mapping.** X display = `round(x/xScale) + 1`; X back = `(value-1)*xScale`. Y display = `round((BufferHt-1) - y/yScale) + 1`; Y back = `(BufferHt-1-(value-1))*yScale` (`POLY/Converters/PolygonPointXConverter.cs:31-64`, `PolygonPointYConverter.cs:36-64`). So the user sees 1-based display-element pixels with **Y = 1 at the bottom**. Note: X's rule validates against `(CanvasW-1)/xScale+1`, Y likewise (`POLY/ValidationRules/XValidationRule.cs:46`, `YValidationRule.cs:47`). Messages: "Not valid X value.", "X value is larger than display element.", "X value must be greater or equal to 1." and the same for Y. **Static mutable scales are an upstream hazard**: only one polygon editor can exist at a time. Pass the mapping per session in Rust. **Precision (audit):** grid input is parsed with `double.TryParse`/`double.Parse` in the current culture (`XValidationRule.cs:37`, `PolygonPointXConverter.cs:56`), so fractional values such as `3.5` are accepted and stored unrounded (the cell then shows the rounded value); they are only snapped to whole pixels by OK (F84d). The validator's `Width`/`Height` are the canvas `ActualWidth`/`ActualHeight` truncated to `int` (`PEVM:747-791`).

**F93a. Time bar ruler** (`POLY/Views/PolygonTimeBarCanvas.cs`). Black 1 px ticks from the bottom edge, between margins 5: 10 long intervals, each with 10 small intervals; every tenth tick is 15 tall, the rest 10.

### J. Intelligent Fixture Wizard (moving head wizard)

Also described in `docs/spec/display-setup.md` F139-F150 (and F110 for the template), and already partly built: `crates/vixen-session/src/setup/fixture.rs` (`FixtureWizard`, `create_fixture_elements`, profile reader and writer) and `app/Sources/VixenSetup/FixtureWizard/`. This spec is authoritative for the fixture-specific rules; check the existing code against it rather than rebuilding it. Alias `WZ` = `WIZ/Wizard/`.

**F94. Wizard window.** Started by the Intelligent Fixture element template (`TPL`; its Finish step is F104-F107). Title "Intelligent Fixture Wizard", side navigation listing the pages, min 1100x605, max 1400x605, resizable (`WZ/Models/IntelligentFixtureWizard.cs:37`, `:53-58`). Pages in order: Select Profile, Edit Functions, Edit Channels, Color Support, Automation, Dimming Curves, Grouping, Summary (`:40-47`). Nav buttons Back, Forward, Finish, Cancel with Vixen style (`WZ/FixtureWizardNavigationController.cs:55-105`); the base is Orc `FastForwardNavigationController` (NuGet, no source). `CanMoveBack` and `CanMoveForward` also ask a page VM implementing `IIntelligentFixtureWizardPageViewModel` (`WZ/Models/IntelligentFixtureWizard.cs:180-215`). Help button opens the page's URL (`:136-152`), base `http://www.vixenlights.com/vixen-3-documentation/setup-configuration/setup-display-elements/intelligent-fixture-wizard/` + `select-profile/`, `edit-functions/`, `edit-channels/`, `color-support/`, `automation/`, `dimming-curves/`, `grouping/`, `summary/`. Summary page description: "Below is a summary of the fixture selections." (`:50`). Wrapper properties configured by the template: `AllowQuickNavigationWrapper`, `HandleNavigationStatesWrapper`, `NavigationControllerWrapper`, `ShowHelpWrapper`, `ShowInTaskbarWrapper`, `CacheViewsWrapper` (`:77-125`; see the template entry for values).

**F95. Bookend moving heads.** Pages except Edit Functions and Edit Channels show two 3D moving heads either side of the controls (`WZ/Views/WizardUserControlBase.cs:35-67`): no legend, tilt 45, pan 35 (left) and 315 (right), beam length 20, focus 20, drawn with `MovingHeadWPF.DrawFixtureNoBitmap(size, size, 1.0, 0, 0, viewport)`. Size and beam colour per page: Select Profile 300 DodgerBlue; Automation 400 Purple; Color Support 400 Blue; Dimming Curves 400 Pink; Grouping 300 Green (`*.xaml.cs` `UserControl_Loaded`). Decorative only. Mac: render once through `vixen-render` to a texture or a static SceneKit-free image, see the graphics section.

**F96. Page Select Profile.** Title "Select Profile", description "Select or create the profile for the fixture(s)." (`WZ/Models/SelectProfileWizardPage.cs:18-19`). Group "Select Profile": radio "Select Existing Profile" (disabled when no profiles exist) + combo of names (300 wide, enabled when that radio is on); radio "Create New Profile" (`WZ/Views/SelectProfileWizardPageView.xaml:25-57`). Group "Profile Properties": "Profile Name:" (tip "Enter a unique name for the fixture profile."), "Manufacturer:" ("Enter the manufacturer of the fixture."), "Created By:" ("User that created the fixture profile."), "Revision:" ("Revision of the fixture profile.") (`:59-97`). Behaviour (`WZ/ViewModels/SelectProfileWizardPageViewModel.cs`): defaults to existing when any profile exists, else Create New (`:44-58`). Choosing a name loads a **clone** of that profile; empty name makes a new specification with `InitializeBuiltInFunctions` (`:91-105`). Selecting existing with none chosen picks the first (`:161-165`). Choosing Create New clears selection, which creates the new specification (`:196-200`). Validation: "Profile Name is empty.  Profile Name is a required field." blocks moving on and shows the validation bar (`:334-343`). Summary text: the selected fixture name (`WZ/Models/SelectProfileWizardPage.cs:162-165`). **No uniqueness check (audit addition):** the only validation is the empty-name rule, despite the "unique name" tooltip. Finish saves to `Fixtures\<GetFileName()>` and replaces the library entry with the same `Name` (see "Library manager"), so (a) a new profile given an existing profile's name silently overwrites that profile's file and library entry, and (b) changing the Profile Name of an existing profile saves a second profile under the new name and leaves the old file and entry in place. Port: keep (b), and for (a) ask before overwriting; record the difference in the parity file.

**F97. Page Edit Functions.** Title "Edit Functions", description "Define the fixture functions." Hosts `FunctionTypeView` height 425, initial function "Pan", shares `selectProfile.Fixture.FunctionDefinitions`; on leaving, `Fixture.FunctionDefinitions = childVM.GetFunctionData()` (`WZ/ViewModels/EditProfileFunctionsWizardPageViewModel.cs:31`, `:70-85`). No summary. Controls in the Function Type section below.

**F98. Page Edit Channels.** Title "Edit Channels", description "Define the fixture's channels." Hosts `FixturePropertyEditorView` height 400, `DisplayLoadButton=False` (`WZ/Views/EditProfileWizardPageView.xaml:8-11`). Back and Next are blocked while `GetValidationResults()` is not empty (`WZ/ViewModels/EditProfileWizardPageViewModel.cs:102-147`). On leaving writes the fixture back and sets `SelectedFixture = Name` (`:70-91`).

**F99. Page Color Support.** Title "Color Support", description "Select the color support applicable to the fixture.  Color Mixing should be selected when the fixture uses a multiple color light source or multiple light sources of different colors.  Color wheel should be selected when the fixture has a single white light source and is equipped with a color wheel to change colors.  " (`WZ/Models/ColorSupportWizardPage.cs:17-19`). Radios, with tooltips (`WZ/Views/ColorSupportWizardPageView.xaml:37-56`): "No Color Support" ("Select if you want to manually control color through separate display elements.  This option is not recommended."), "Color Mixing" ("Select when the fixture uses multiple color source or multiple light sources of different colors.  Examples are RGB and RGBW"), "Color Wheel" ("Select when the fixture has a single white light source and is equipped with a color wheel to change colors."). Default: `IsColorMixing` then `ContainsColorWheel` else none (`WZ/ViewModels/ColorSupportWizardPageViewModel.cs:26-40`). Summary "No Color Support", "Color Mixing", or "Color Wheel" (`WZ/Models/ColorSupportWizardPage.cs:49-66`).

**F100. Page Automation.** Title "Automation", description "These options configure special filters that make it easier to control the fixtures when sequencing." Group "Automation Options", check boxes with tooltips (`WZ/Views/AutomationWizardPageView.xaml:31-54`):
 - "Automatically Open and Close Shutter": "This option will automatically control the shutter channel based on the color channels.  Whenever an effect is generating color, the shutter will open.  Whenever there is no effect, or the effect is dark, the shutter will be closed."
 - "Automatically Control Color Wheel": "This option will automatically control the color wheel by matching colors generated in the effects to colors supported on the color wheel."
 - "Automatically Control Dimmer": "This option is usually selected for fixtures with a color wheel and fixed white light source.  It will automatically control the fixture's dimmer channel based on the intensity of the colors in the effects."
 - "Automatically Open and Close Prism": "This option is usually selected for fixtures that both a channel for enabling the prism and another channel for selecting the position or movement of the prism."
 Enable and default (`WZ/ViewModels/AutomationWizardPageViewModel.cs:31-79`). **Corrected by the audit: only the Shutter option depends on No Color Support** (this entry said all four were disabled by it). Shutter: enabled and checked when the page says Color Mixing or Color Wheel **and** the fixture supports identity Shutter (`:31-42`); otherwise disabled and left unchecked. Color Wheel: enabled and checked when the Color Support page says Color Wheel, otherwise disabled (`:45-58`). Prism: enabled and checked when the fixture supports identity OpenClosePrism, whatever the colour support (`:61-68`). Dimmer: enabled and checked when the fixture supports identity Dim, whatever the colour support (`:71-79`). "Supports" is `FixtureSpecification.SupportsFunction(identity)`: the first function with that identity is assigned to a channel. A disabled box keeps whatever value the page model already had (false on a first visit). Summary: comma list "Shutter", "Color Wheel", "Dimmer", "Prism" with trailing separators trimmed (`WZ/Models/AutomationWizardPage.cs:54-85`; **note "Dimmer," has no space, so "Dimmer,Prism"**).

**F101. Page Dimming Curves.** Title "Dimming Curves", description "These options configure if any dimming curves are applied to the fixture outputs." Group "Dimming Options" radios: "Do Not Include a Dimming Curve" ("Select when dimming curves are not desired."), "Add Dimming Curve For Fixture" ("This option adds a single dimming curve for all color channels."), "Add Dimming Curve For Each Color Channel" ("This option configures a separate dimming curve for each color channel.  With this option all the dimming curves are initially identical.  The curves can be further adjusted via the Patching Graphical View."), enabled only for Color Mixing. Button "Setup Dimming Curve" opens the curve editor, OK stores the curve (default `new Curve()`) (`WZ/ViewModels/DimmingCurveWizardPageViewModel.cs:29`, `:42-54`; view `WZ/Views/DimmingCurveWizardPageView.xaml:29-50`). Default none. Summary "No Dimming Selected", "Dimming Curve for Fixture", "Dimming Curve for Each Color Channel" (`WZ/Models/DimmingCurveWizardPage.cs:87-107`).

**F102. Page Grouping.** Title "Grouping", description "Configure how many fixtures to create and if they should be grouped." Left: "Number of Fixtures to Create:" read-only count text box (centre), "+" and "−" image buttons (tips "Select to increment the number of fixtures to create.", "Select to decrement the number of fixtures to create."; minus disabled at 1); "Element Prefix:" text (tip "Name prefix used in the display element tree for the fixtures.") default profile name; check "Create Group" (tip "Select to nest the fixtures under a group in the display element tree.") default on; "Group Name:" text (tip "Name of the fixture group in the display element tree."; enabled with the check) default "Fixtures" (`WZ/Views/GroupingWizardPageView.xaml:41-86`, `WZ/ViewModels/GroupingWizardPageViewModel.cs:33-72`). Right: group "Display Elements Preview", tree always expanded, green bullet on leaves only (`WZ/ViewModels/FixtureWizardTreeItem.cs:30-36`). Names `prefix + index` from 1 (`:116`, `:215`). With no group the root is hidden. The "Number of Fixtures to Create:" label has the tooltip "Number of fixture display elements to create."; the count box has min width 30 and the two text boxes are 100 wide (`WZ/Views/GroupingWizardPageView.xaml:38-83`, audit addition). **Quirk: while Create Group is off, changing Group Name is disabled, but `PreviewTree[0].Name = value` would rename the first fixture in the preview if reached; no user path hits it because the box is disabled** (`:271`). Summary: `# Fixtures: {n}`, optional `Group Name: {name}`, `Element Prefix: {prefix}` (`WZ/Models/GroupingWizardPage.cs:30-48`). Default count 1.

**F103. Page Summary.** Orc summary page listing each page's `Title` and summary text (`WZ/Models/SummaryWizardPageBase.cs:29-37`), Edit Functions and Edit Channels omitted (no summary). Finish runs the template (below). Cancel discards the wizard.

 **Where page defaults come from (audit addition).** The page models start empty: all three Color Support flags false, all four Automation flags false, `NumberOfFixtures` 0, no prefix or group name; only the Dimming Curves model sets its own defaults (`NoDimmingCurve = true`, `new Curve()`, `WZ/Models/DimmingCurveWizardPage.cs:23-25`). Every other default in F99, F100 and F102 is written by the page's view model constructor (`ColorSupportWizardPageViewModel.cs:26-40`, `AutomationWizardPageViewModel.cs:31-79`, `GroupingWizardPageViewModel.cs:60-72`), through `[ViewModelToModel]`. Two consequences: (1) the defaults depend on the answers on earlier pages at the moment the page is opened; (2) the template turns view caching off (`TPL:930`), so, as far as can be told without the Orc.Wizard source, a page gets a new view model each time it is shown and its constructor runs again, which puts the Automation boxes, the Color Support radio and the whole Grouping page (count 1, prefix = profile name, Create Group on, "Fixtures") back to their defaults when the user navigates back to them. The Color Support constructor sets one flag true without clearing the other two. The Dimming Curves selection and curve survive revisits (its constructor only recomputes the enable of the third radio, `DimmingCurveWizardPageViewModel.cs:29`), so "Add Dimming Curve For Each Color Channel" can stay selected, disabled, after the user goes back and leaves Color Mixing; Finish then treats it as "a curve is wanted" on the dimmer path of F106. Port: compute a page's defaults on its first visit and again only when an answer it depends on (profile, colour support) has changed; never leave a page model in the empty state at Finish. Uncertain: whether Orc lets the user reach Finish without visiting every page; if it does, upstream would create 0 fixtures.

**F104. Finish: create fixture elements** (`TPL:1218-1313`, also `display-setup` F110). `TemplateName` "Intelligent Fixture"; `SetupTemplate` returns true; `ConfigureColor` and `ConfigureDimming` false (`:54-58`, `:1318-1345`). The wizard is shown in the task bar, with help, quick navigation between visited pages, navigation states handled, views not cached (`:918-933`). Upstream forces a dark base and DodgerBlue accent (`:1232-1239`). `Cancelled = result is false` (`:1245`); a `null` dialog result therefore counts as **not** cancelled and the Finish steps run (audit precision). When not cancelled: **saves the profile first** (`FixtureSpecificationManager.Save`, `:1262`), so cancelling a later step does nothing to undo it. Creates `NumberOfFixtures` nodes named `ElementPrefix + (index+1)`. With Create Group: creates group `GroupName` with `ElementNodeService.CreateSingle(null, name, false)`, adds children, records the individual nodes as `GetElementsToDelete` (they go under the group) and `_leafNodes`, and returns the group alone (`:1284-1309`).

**F105. Per fixture node** (`CreateFixtureNode`, `TPL:1072-1165`). Adds the Intelligent Fixture property with the specification (`IFIX`, shared by reference through the property setter, see F108 and "Intelligent Fixture property data" under Data and persistence). Colour mixing adds a full colour property first (`:1109-1113`, `ElementColorType.FullColor`). Walks channels in order, skipping a channel whose function equals the last processed function, unless it is the None function: `channel.Function != last || channel.Function == "None"` (`:1137-1138`). So consecutive channels of the same function produce one filter chain. `Single` lookups throw when a channel has no matching function (both the channel's own function and the next channel's, `:1130`, `:1134`). `_colorPropertyCreated` and the colour mixing counters are reset per fixture node (`:1085`, `:1105-1106`).

**F106. Per function dispatch** (`ProcessFunction`, `TPL:999-1056`):
 - **ColorWheel**, automatic on: `ColorWheelFilter { ColorWheelData, ConvertRGBIntoIndexCommands = !colorMixing, Tag = function.Name }`, `CreateOutput`, attached to element output 0; adds a discrete colour property when none yet: `MultipleDiscreteColors`, colour set named after the fixture, stored in the colour module's static data with one colour per wheel entry that is not half step, not curve and not `ExcludeColorProperty` (`:248-285`, `:156-186`, `:209-238`). Automatic off: as Indexed.
 - **RGBW**: colour mixing chain with items Red `Color.Red`, Green `Color.Lime`, Blue `Color.Blue`, White `Color.White`. **RGB**: same without White (`:543-568`). Rules below.
 - **Indexed**: identity Shutter with automatic on: `ShutterFilter { Tag = name, ConvertColorIntoShutterIntents = true, OpenShutterIndexValue = first ShutterOpen index's StartValue (nonzero), CloseShutterIndexValue = first ShutterClosed }` (`:752-808`). Identity OpenClosePrism (**regardless of the Automation choice**): `PrismFilter { Tag = name, ConvertPrismIntentsIntoOpenPrismIntents = true, OpenPrismIndexValue, ClosePrismIndexValue, AssociatedFunctionName }` (`:686-744`). Otherwise tagged.
 - **Range**: identity Dim with automatic: `DimmingFilter { Tag = name, ConvertColorIntoDimmingIntents = !colorMixing }`; when a dimming curve is selected and not colour mixing, a dimming curve follows on the last leaf (`:637-678`). Otherwise tagged.
 - **None**: tagged filter with tag "Ignore" (`:814-834`).
 - Tagged: `TaggedFilter { Tag = name }`, `CreateOutput`, attached to output 0 (`:862-880`).
 - After dispatch: a Range function whose next channel has the same Range function name gets a `CoarseFineBreakdown` on the last leaf (`:1049-1056`, `:887-898`). This is how pan fine and tilt fine appear.

**F107. Colour mixing chain** (`ProcessColorMixingFunction`, `TPL:390-537`). State across channels: `found`, `count`. `C` = total channels whose function is RGB or RGBW (`:353-378`). First colour channel:
 1. Per-fixture curve: dimming curve on element output 0.
 2. `ColorBreakdown { BreakdownItems, MixColors = true, _16Bit = C > 4 }`, attached after the curve, else at output 0.
 3. Per-colour curve: for each expected colour (3 or 4) a dimming curve on breakdown output `i`; when 16-bit a coarse fine on each curve. Otherwise when 16-bit a coarse fine on each breakdown output.
 Later colour channels only count; at `count == C` both reset. **Quirk: a fixture with RGB and RGBW both, or two colour groups, relies on these counters resetting.** **Interaction with the F105 skip rule (audit addition):** the built-in profile has one function, "Color" (RGBW), assigned to every colour channel, so with the usual consecutive layout only the first colour channel ever reaches this routine: `count` stays 1, `found` stays true, and the reset never happens. `C` still counts every colour channel (4 for 8-bit RGBW, 6 or 8 for 16-bit), which is what selects `_16Bit`. The counting branch runs only when colour channels are separated by another function or use differently named colour functions; in that case each distinct run after the first only increments the count and adds no filters. The breakdown items come from the function type of the first colour channel processed (RGB 3 items, RGBW 4). In the per-colour 16-bit case the coarse/fine filters are attached to output 0 of the last `n` leaves of the re-walked tree, taken from the end backwards (`:483-498`), which are the `n` dimming curves just added. Each curve filter gets its own copy of the curve (`new Curve(dimmingCurve)`, `:342`). `FindLeafOutputsOrBreakdownFilters` also yields a breakdown module itself with output index -1 (`:1172-1210`), and `AddFilterToNodesLastFilter` takes the last entry. Filter wiring belongs to `vixen-output`; `crates/vixen-session/src/setup/fixture.rs` has `create_fixture_elements`: verify it matches each rule here.

### K. Intelligent Fixture property

**F108. Property** (`IFIX/IntelligentFixtureDescriptor.cs`, `IFIX/IntelligentFixtureModule.cs`, `IFIX/IntelligentFixtureData.cs`) details below; entry point also in `docs/spec/property-filter-controller-setup.md`.

 Descriptor: TypeId `{FDD74C85-1779-4521-B559-0DD5EA71054E}`, TypeName "Intelligent Fixture", Description "Provides ability to specify capabilities of an intelligent fixture", Author "Vixen Team", Version "1.0" (`IFIX/IntelligentFixtureDescriptor.cs:15-58`). Data: one `FixtureSpecification`, default new with built-in functions; `Clone` clones it (`IFIX/IntelligentFixtureData.cs:18-42`). `SetDefaultValues` adds channel number 1 with empty name and function (`IFIX/IntelligentFixtureModule.cs:28-34`). `Setup` opens the Fixture Property Editor window with the specification; on OK replaces it; **always returns true** (`:49-64`). `ModuleData` setter: when the specification has a name and the library doesn't contain it, **adds the element's copy to the in-memory library (not to disk)** (`:73-96`). This is how a profile travels with a show.

### L. Fixture Property Editor (profile channels window)

Alias `FV` = `FPE/Views/`, `FM` = `FPE/ViewModels/`.

**F109. Window.** Title "Fixture Property Editor", 650x500, centred, background #444444 (`FV/FixturePropertyEditorWindowView.xaml:10-12`, `.xaml.cs:29-30`). Body: `FixturePropertyEditorView`. Bottom row height 40: hyperlink "Learn More About Fixture Profiles" (left, to `http://www.vixenlights.com`, opened in the default browser, `.xaml.cs:67-72`) (`:33-37`), warning triangle (visible when errors, tooltip the errors), "OK" (default, disabled with errors, tooltip the errors, shown even while disabled), "Cancel" (`:39-48`). OK: rebuild the specification from the view model, **save it to the library** (`SaveSpecification`, `FM/FixturePropertyEditorViewModel.cs:293-300`), close (`FM/FixturePropertyEditorWindowViewModel.cs:79-89`). Error tooltip: `GetValidationResults()` trimmed of trailing newlines (`FM/FixturePropertyWindowViewModelBase.cs:106-134`). `CanExecuteOK` validates the child and calls `CanSave` (`FM/FixturePropertyEditorWindowViewModel.cs:52-73`).

**F110. Profile fields.** Shown only when `ShowProfileProperties` (window true, wizard false), grid of two label+box pairs per row: "Profile:" (tip "Enter a unique name for the fixture profile."), button "Load Fixture Profile" (150 wide; tip "Select to load an existing fixture profile."; visible when `DisplayLoadButton`, default true, false in the wizard; (audit) it is also marked `IsDefault`, so the window has two default buttons, this one and OK: on the Mac only OK takes Return) (`FV/FixturePropertyEditorView.xaml:43-56`); "Manufacturer:" (tip "Enter the manufacturer of the fixture."), "Revision:" (tip "Revision of the fixture profile."); "Created By:" (tip "User that created the fixture profile."). Validation: "Profile name is empty.  Profile is a required field." (`FM/FixturePropertyEditorViewModel.cs:313-317`).

**F111. Channels grid** (`FV/FixturePropertyEditorView.xaml:82-153`). Columns: "Channel #" (width 70, read-only text box), "Name" (width 2*, tooltip "Unique name for the channel."), "Function" (width *, combo always shown, tooltip "Fixture function associated with the channel.  Note functions may span more than one channel."), and an unnamed column width 32 with a cog button (tip "Select to Edit the function associated with the channel."), visible only with `ShowProfileProperties`. Selecting a row scrolls it into view (`.xaml.cs:36-46`). No user-added rows. Add button (+, tip "Select to Add a new channel."), Remove button (-, tip "Select to Remove the selected channel.") below (`:157-166`).
 - Add (`FM/FixturePropertyEditorViewModel.cs:414-424`, `FM/ItemsViewModel.cs:167-186`): new channel at the end numbered `count+1`, Function "None", Name "Ignore" (`ChannelItemViewModel.IgnoreFunctionName`), selected.
 - Delete (`:395-408`, `FM/ItemsViewModel.cs:115-161`): uses the selected row, or the last selected row if the grid dropped it; renumbers 1..n. No confirmation.
 - Function combo items: function names sorted by name (`:448-462`).
 - Assign function (`FM/ChannelItemViewModel.cs:122-163`): a non-None value whose function is incomplete (Indexed with no index entries or Color Wheel with no wheel entries) shows an error box `Function '{value}' is incomplete.  Please populate the function before assigning it to a channel.` titled "Incomplete Function", then reverts to "None" when idle. If the channel's name is still "Ignore" and the function is not None, the name becomes the function's name. (audit) Order matters: the setter stores the value and renames an "Ignore" channel **before** the idle revert runs, so after the error box the channel is back on None but keeps the incomplete function's name. `IsFunctionValid` (`:269-300`) is also false when the function name is not found in the list. Port: check completeness first and change nothing on failure; record in the parity file.
 - Cog icon: `cog_error_red.png` while the function has no index or wheel data, else `cog_edit.png` (`:220-251`).
 - Validation: "Name is empty.  Name is a required field." (`FM/ItemViewModel.cs:85-88`), "Function is empty.  Function is a required field." (`FM/ChannelItemViewModel.cs:201-204`), "At least one channel is required to define a fixture." (`FM/FixturePropertyEditorViewModel.cs:330-333`), "Function index data is incomplete.", "Function color wheel data is incomplete." (`FM/ChannelItemViewModel.cs:233`, `:242`). **Uniqueness of names isn't enforced despite the tooltip.**
 - Load: `InitializeChildViewModels` works on a clone (`FM/FixturePropertyEditorViewModel.cs:195-244`); a channel whose function isn't defined loads as None. (audit) Load assigns each channel's function through the same setter (`:219-243`), so opening a profile in which a channel already uses an incomplete function shows one "Incomplete Function" error box per such channel and reverts each to None at idle. Port: load without the box, keep the function, and let the cog icon and the "incomplete" validation messages report it.

**F112. Load Fixture Specification dialog.** Title "Load Fixture Specification", 350x400, list of specification names, first selected by default, "OK", "Cancel" (`FV/SelectFixtureSpecificationView.xaml`, `.xaml.cs:24-25`, `:53-59`). OK replaces the editor's contents and re-validates (`FM/FixturePropertyEditorViewModel.cs:514-543`). With an empty library nothing is selected and OK would load null: guard on the Mac by disabling OK.

**F113. Cog button (Edit function).** Opens the Function Type window with the current functions, initial selection the channel's function (`FM/FixturePropertyEditorViewModel.cs:467-510`). On OK: replaces functions, updates combo lists on every channel, any channel whose function was deleted reverts to None, re-validates.

**F113a. `CanSave` aggregate.** `FM/ItemsViewModel.cs:274-330`: each item's `CanSave`, then own fields and rules; messages concatenated with `\n`. Rust should return a single `Vec<String>` of errors.

### M. Function Type editor ("Edit Functions")

**F114. Window.** Title "Edit Functions", 1165x550, centred (`FV/FunctionTypeWindowView.xaml:13`, `.xaml.cs:25-26`). Body: `FunctionTypeView` (1100x500, `FV/FunctionTypeView.xaml.cs:27-28`). Bottom row height 60: hyperlink "Learn More About Fixture Functions" to `http://www.vixenlights.com` (`:50-54`), warning triangle, "OK" (disabled with errors, with error tooltip), "Cancel" (`:56-65`). OK stores `GetFunctionData()` as `UpdatedFunctions` (`FM/FunctionTypeWindowViewModel.cs:105-112`). The same view is embedded in the wizard (F97).

**F115. Functions list (left group "Functions", width 500).** Grid, full-row selection, no user-added rows (`FV/FunctionTypeView.xaml:46-139`). Columns:
 - "Name" (*): tooltip "Enter a unique name for the function.  This name appears in effects."
 - "Type" (105): combo always shown of Range, Indexed, Color Wheel, RGB Color, RGBW Color (`FixtureFunctionTypeVM`, no None), tooltip "Select the type of function."
 - "Preview\nLegend" (55): text, tooltip "Character to display to represent the function in a legend below the fixture graphic on the preview."
 - "Tag" (120): combo of `FunctionIdentities` for the type, tooltip "Select a Tag for functions when Vixen needs to be aware of the function for the Preview or automation.  Click the Help button for additional information."
 Below: Add (tip "Select to Add a new function."), Remove (tip "Select to Remove the selected function.").
 - Tag choices by type (`FM/FunctionItemViewModel.cs:44-73`, `:209-230`): Range: Pan, Tilt, Zoom, Dim, Frost, Custom. Indexed: Shutter, Gobo, Prism, OpenClosePrism, Custom. Color Wheel: SpinColorWheel only. RGB, RGBW: Custom only. **Changing Type doesn't reset Tag; an out-of-list value just isn't shown as selected.** Default Range, Custom.
 - Load (`FM/FunctionTypeViewModel.cs:269-318`): every function except the None one becomes a row, in order; `SelectedItem` = the function named `functionToSelect`. **Note: when `functionToSelect` is "None" or absent, nothing is selected.**
 - Add (`:972-1003`): a new row, name empty, Range, Custom; a random unused timeline colour: start at a random index among all known colours excluding system colours and Transparent, walking forward and wrapping. (audit) Exact rule (`:972-1002`, list built once statically at `:36-42`): start index `rnd.Next(count - 1)` (so the last colour is never the starting index), then advance while the colour is already used by another row **and** the row count is `<=` the colour count (the guard only stops the walk once there are more rows than colours; with exactly as many rows as colours, all distinct, upstream would spin for ever, which is unreachable in practice but the port must simply fall back to the start colour after one full lap). Loaded rows and the row object itself default to White (`FM/FunctionItemViewModel.cs:80-85`). Selection triggers: the name cell goes into edit with focus (`FV/FunctionTypeView.xaml.cs:125-142`). **Port the known-colour list to Rust (`Color.FromKnownColor` set) or use a fixed palette; document which. The value persists as `TimelineColor1Html`.**
 - Remove (`FM/ItemsViewModel.cs:115-161`): no confirmation; nothing checks whether a channel references it (channels revert on return, F113).
 - Save (`:325-391`): writes the active details first; builds functions with name, index, wheel, label, type, identity, limits, zoom type, colour, associated name; **adds the None function back when absent** (None, Custom, Transparent).
 - Validation (`:1041-1072`): "Cannot have duplicate function names.", "Cannot have more than one Pan function.", "Cannot have more than one Tilt function.", "Cannot have more than one Zoom function.", row "Name is empty.  Name is a required field." (`FM/FunctionItemViewModel.cs:352-358`).

**F116. Selection gate.** Changing the selected row first runs `AllowSelectionToChange` (`:437-468`): the visible details panel's `CanSave` must pass, then its data is written to the row. If not, selection returns to the previous row and focus to its name cell (`FV/FunctionTypeView.xaml.cs:114-156`, `:172-192`). **So a user can't leave a row with invalid details.** On the Mac: same gate in Rust; a list row click that fails keeps the row selected and shows the error under the panel.

**F117. Details panel (right group, title "{name} Details" or "<Function> Details").** (`:66`, `:627-631`, `:1009-1022`). One of four panels, chosen by type and tag (`:808-851`; also re-evaluated when type or tag changes, `:859-863`):
 - Indexed: Indexed panel; image column when tag is Gobo; associated function combo when tag is OpenClosePrism, listing names of functions tagged Prism.
 - Color Wheel: Color Wheel panel.
 - Range with tag Pan or Tilt: Pan/Tilt panel.
 - Range with tag Zoom: Zoom panel.
 - Otherwise (RGB, RGBW, other range tags): nothing.

**F118. Indexed panel** (`FV/IndexedView.xaml`, `FM/IndexedViewModel.cs`, `FM/IndexedItemViewModel.cs`): see F118a.

**F118a. Indexed panel** (`FV/IndexedView.xaml`, `FM/IndexedViewModel.cs`, `FM/IndexedItemViewModel.cs`). Grid columns:
 - "Name" (*; tip "Unique name for the index entry.  This name is used to select the option in effects.").
 - "Use Curve" (60, check; tip "Select for entries that define a range where the user neeeds to pick a specific value within that range.  This entry will be edited using a curve in effects.").
 - "Start Value" (65; tip "DMX start value for the index."), "End Value" (65; tip "DMX stop value for the index.").
 - "Tag" (120, combo of `FixtureIndexType` without ColorWheel, `FM/IndexedItemViewModel.cs:165-184`; tip "Select a Tag for index items when Vixen needs to be aware of the index value for the Preview or automation.  Click the Help button for additional information."). **Visible when not a gobo.**
 - "Image" (150, combo of `GetGoboImages()`, first entry a single space; tip "Select an image to associate with the Gobo item.  The image appears on the timeline when using the Gobo effect."). **Visible only for gobo**: `DisplayTag = !displayImage` (`FM/IndexedViewModel.cs:154-157`).
 Below: Add (tip "Select to Add a new index item."), Remove (tip "Select to Remove the selected index item."), "Import Gobo Images" button (gobo only; tip "Select to Import png images for the Gobo items."), "Associated Prism Function:" label and 200-wide combo (OpenClosePrism only; tip "Select the Prism function that is opened and closed by this function.") (`FV/IndexedView.xaml:154-180`).
 - Add puts the new row's Name into edit (`FV/IndexedView.xaml.cs:45-60`).
 - Load and save (`FM/IndexedViewModel.cs:142-251`): an `EndValue` of -1 shows blank; a blank saves as -1. **New rows have no `StartValue`, and `int.Parse` would throw, but validation blocks it.**
 - Import images (`:321-357`): multi-select `*.png`, copied into the gobo image folder overwriting existing names, **errors ignored**; refreshes each row's choices.
 - Validation (`FM/IndexedItemViewModel.cs:195-211`, `FM/ItemViewModel.cs:85-125`): "Name is empty.  Name is a required field."; "Start Value is empty. Start Value is a required field."; `Not a valid Start Value value.  Valid range is 0 to 255.`; with Use Curve the same for "End Value". Business rules (`FM/IndexedViewModel.cs:284-312`): "Index ranges cannot overlap."; "Cannot have duplicate index names." Overlap (`:365-440`): blank is -1; both full: `ls <= re && le >= rs`; one has no end: point in other range; both single: equal. **A non-numeric value in Start would throw in `Overlaps`; `ValidateFields` runs first but the business rule still runs, so guard with parse-or-none in Rust.**
 - (audit) End Value is field-validated only while Use Curve is ticked (`FM/IndexedItemViewModel.cs:204-208`), but `Overlaps` (`FM/IndexedViewModel.cs:384-402`) and `GetIndexData` (`:229-233`) parse it whenever it is non-empty, whatever Use Curve says. So a non-numeric or out-of-range End Value on a row without Use Curve is never reported: a non-number throws in `int.Parse`, and a number is saved and takes part in the overlap rule. Port: validate End Value (0 to 255, or blank) on every row and record it in the parity file. A blank Start Value never overlaps anything (it only fails field validation).
 - (audit) Delete sentinel for this grid: Name "Zombie", Start "0", End "0" (`:272-278`).
 - (audit) Changing the function's Type or Tag in the functions grid rebuilds the details panel from the row's **stored** data (`FM/FunctionTypeViewModel.cs:808-863`) without first writing the visible panel back, so index or wheel rows edited since the row was selected are discarded when the type is switched away and back. The hidden panel's view model also keeps its last contents and is still validated by `CanSave` (F121a), so invalid rows left in a panel that was hidden by a type change keep OK disabled with a message for a panel the user cannot see. Port: write the visible panel back before a type or tag change when it is valid, and validate only the panel that belongs to the selected row.

**F119. Color Wheel panel** (`FV/ColorWheelView.xaml`, `FM/ColorWheelViewModel.cs`, `FM/ColorWheelItemViewModel.cs`). Grid columns:
 - "Name" (*; tip "Unique name for the color entry.  This name is used to select the option in effects.").
 - "Start Value" (63; tip "DMX start value for the color."), "End Value" (63; tip "DMX stop value for the color.").
 - "Use Curve" (60, check; tip "Use curve when your manual shows a range of DMX values that spins the color wheel.").
 - "Half Stop" (60, check; tip "Select Half Stop option if the entry is half way between two colors on the color wheel.").
 - "Color" (60): first swatch button with text "..." (tip "Select '...' button to assign the color."), second swatch visible only with Half Stop. Half stop rows are not editable and show no text (`FM/ColorWheelItemViewModel.cs:134-160`, `:202-224`).
 - "Exclude\nColor\nProperty" (65, check; tip "Excludes the color entry from the color property associated with the fixture element.  This option should be selected for colors like CTB (Color Temperature Blue) and UV (Ultraviolet).").
 Below: Add, Remove (no tooltips). (audit) Add puts the new row's Name cell into edit with focus, as in the Indexed grid (`FV/ColorWheelView.xaml.cs:32-60`). The colour button opens the common colour picker with the brightness lock off (`LockValue_V = false`, `FM/ColorWheelItemViewModel.cs:202-226`); after it closes, OK or Cancel, every half stop is recomputed. Ticking Half Stop blanks the button text and recomputes; unticking restores "..." and does **not** recompute or restore the row's colours (`:134-160`).
 - Half stop colours (`FM/ColorWheelViewModel.cs:173-195`): for each half stop row, `Color1 = previous row's Color1`, `Color2 = next row's Color1`. Recomputed on half-stop change and after a colour edit. Not on add, remove or reorder. (audit) The first row has no previous row, so its `Color1` is left alone; the last row has no next row, so its `Color2` is left alone. Delete sentinel for this grid: Name "Delete Me", Start "0" (`:159-164`).
 - Load: `EndValue` is always shown as text, including -1 (**asymmetric with Indexed**). Save: blank End saves -1 (`:55-58`, `:97-109`).
 - Validation (`FM/ColorWheelItemViewModel.cs:236-252`): Name; Start Value DMX. **Quirk: with Use Curve it validates `StartValue` a second time under the label "End Value"**, so End Value is never actually checked and a non-number crashes `int.Parse` on save (`:248`, `FM/ColorWheelViewModel.cs:103`). Fix: validate End Value when Use Curve and record it. No overlap or uniqueness rule here.
 - Colour picker maps to the system colour panel, as above.

**F120. Pan/Tilt panel** (`FV/PanTiltView.xaml`, `FV/PanTiltView.xaml.cs`, `FM/PanTiltViewModel.cs`): see F120a.

**F121. Zoom panel** (`FV/ZoomView.xaml`, `FM/ZoomViewModel.cs`): see F121a.

**F120a. Pan/Tilt panel.** "Start Position (Degrees):" (tip "Defines the resting position of the fixture."), "Stop Position (Degrees):" (tip "Defines the maximum range of movement."), and an animated 3D moving head below (300 square, no legend, cyan beam, length 20, focus 20) (`FV/PanTiltView.xaml:24-49`, `FV/PanTiltView.xaml.cs:27-31`, `:41`).
 - Animation (`FV/PanTiltView.xaml.cs:77-192`): a background thread waits on `Animate` (cleared while hidden), then every 10 ms: pan mode: tilt fixed at 90, pan sweeps 0..360 by 1 degree; tilt mode: pan fixed at 0, tilt sweeps 180..360 by 1 degree; redraws on the UI thread. **The sweep limits are fixed; they don't follow the Start and Stop values.** Direction flips at each limit.
 - Validation (`FM/PanTiltViewModel.cs:169-246`): `{field} is empty.  Valid range is 0-999 degrees.  `; `Not a valid {field} value.  Valid values are 0-999.  `; "Start Position must be less than or equal to the Stop Position." Fields "Start Position" and "Stop Position".
 - (audit) **Upstream bug: a newly added Pan or Tilt function has no rotation limits.** Loaded functions always carry limits (default 0 and 360, `FIX/FixtureRotationLimits.cs:16-20`, `FIX/FixtureFunction.cs:66`), but a row made by Add uses the parameterless `FunctionItemViewModel` constructor and its `RotationLimits` stays null (`FM/FunctionItemViewModel.cs:80-100`). Tagging it Pan or Tilt calls `InitializeViewModel(null, ...)` (`FM/PanTiltViewModel.cs:41-64`), which leaves the two text boxes showing whatever the previously displayed Pan/Tilt function had (blank when none was shown), and `ValidateFields` skips every check while the limits are null (`:231-247`). Save then parses the boxes unchecked (`:70-82`): blank boxes throw, stale values are silently stored as the new function's limits. Port: give a new Pan/Tilt function the default limits 0 and 360, always validate, and record it in the parity file.
 - (audit) `UserControl_Unloaded` (`FV/PanTiltView.xaml.cs:186-192`) is never attached in the XAML (only `Loaded` is, `FV/PanTiltView.xaml:9`), so the animation thread started on load (`:77-85`) is never told to stop; it is a background thread and dies with the process. Nothing to port, but the Mac animation must stop when the panel is hidden or the window closes.
 - Save parses both as int (`:70-82`). Mac: draw the same frame through `vixen-render` on a display-link at 100 Hz equivalent (1 degree per 10 ms = 100 degrees per second), no thread that blocks the UI (see hazards).

**F121a. Zoom panel.** Radios "Narrow To Wide" (tip "Fixture zooms from a narrow beam to a wide beam."), "Wide To Narrow" (tip "Fixture zooms from a wide beam to a narrow beam.") (`FV/ZoomView.xaml:17-29`, `FM/ZoomViewModel.cs:28-38`). Saved as `ZoomType`.
 - (audit) **Upstream bug: the Zoom choice is not saved on OK.** `GetFunctionData` (`FM/FunctionTypeViewModel.cs:325-391`) calls `SaveActiveFunction` (`:660-683`), which writes back the visible Color Wheel, Indexed (plus the associated function name) or Pan/Tilt panel but has no Zoom branch. `SaveZoomData` (`:928-931`) is reached only through `AllowSelectionToChange` (`:437-468`), which runs when the selected row changes (`FV/FunctionTypeView.xaml.cs:114`). So changing the radio and pressing OK (`FM/FunctionTypeWindowViewModel.cs:105-112`), or leaving the wizard's Edit Functions page (`WZ/ViewModels/EditProfileFunctionsWizardPageViewModel.cs:82`), without first selecting another row loses the change. Port: save Zoom together with the other panels and record it in the parity file.
 - (audit) `FunctionTypeViewModel.CanSave` (`:478-545`) validates the Color Wheel, Indexed and Pan/Tilt child view models whether or not their panel is visible, and never the Zoom one (it has nothing to validate).

**F122. Grid cell behaviour.** Any cell that receives focus goes straight into edit and focuses its control (`FV/DataGridView.cs:23-37`). `VixenDataGrid` lets the user start editing another cell when only row-level validation failed (`FV/VixenDataGrid.cs:36-55`). Mac: `Table` or `NSTableView` with direct-editing text cells; row-level errors do not block editing.

**F123. Delete sentinel values.** `MakeObjectValidBeforeDeleting` sets names to "Zombie" or "Delete Me" so WPF validation stays quiet while a row is being removed (`FM/FixturePropertyEditorViewModel.cs:385-390`, `FM/FunctionTypeViewModel.cs:1030-1035`, `FM/IndexedViewModel.cs:272-278`, `FM/ColorWheelViewModel.cs:159-164`). A WPF workaround: not needed.

### N. State property and State Mapper

Also listed as F193-F211 in `docs/spec/property-filter-controller-setup.md` and implemented in `crates/vixen-session/src/dialogs/state.rs`, `state_preview.rs`, `crates/vixen-ffi/src/dialogs/state.rs`. Since that spec says the owning spec wins, **this section is authoritative: check that code against it.** Alias `SM` = `STATE/Setup/ViewModels/StateMapperViewModel.cs`, `SV` = `STATE/Setup/Views/StateMapperView.xaml`.

**F124. State property.** TypeId `{35B893C3-D10C-4359-82BC-929C0762E809}`, TypeName "State", Description "Provides mechanism for assigning State mapping of display elements", Author "Jeff Uchitjil", Version "1.0" (`STATE/StateDescriptor.cs`). `SetDefaultValues` normalizes (`STATE/StateModule.cs:20-22`). `SetupElements(nodes)`: exactly one node, else false; opens the mapper with `_data` (`:87-105`). `CloneValues` uses `CloneForNewProperty`: new property id, definitions cloned as new (new definition and item ids) with same names, descriptions, colours and element ids (`:45-56`, `STATE/StateData.cs:49-58`). **So copying a state property mints new stable ids.** `Clone` keeps ids (`STATE/StateData.cs:36-44`). (audit) Assigning `ModuleData` also normalizes it, and a null assignment is ignored (`STATE/StateModule.cs:59-68`). `GetStateModuleForElement(node)` is the static lookup other modules use (`:76-79`). `HasElementSetupHelper` is true (`:84`).

**F125. Setup helper "State Mapping".** `STATE/StateSetupHelper.cs`. `HelperName` "State Mapping" (`:40`). Not exactly one node: box "State Mapping can only be applied to one Element Node", title "Too many nodes selected", information (`:45-52`). Node with a property: maps its data. Without: creates an unattached State module, maps it, and only on OK adds it with `AddWithoutDefaults` (so Cancel leaves no property) (`:61-107`). Failures dispose and return false. The result reports success to the caller. (audit) A node that already has the property but whose module data is not `StateData` returns false without opening anything (`:55-59`). After `AddWithoutDefaults` the helper checks the node now holds that same module instance and otherwise disposes it and returns false (`:100-107`). Note the message title says "Too many nodes selected" even when no node is selected.

**F126. Normalization and defaults** (`STATE/StateData.cs`, `STATE/StateDefinitionData.cs`, `STATE/StateItemData.cs`, `STATE/StateNamingRules.cs`). New `StateData`: new id; one definition `CreateDefault("State - 1")` with one item (`STATE/StateData.cs:15-19`). `Normalize` on deserialization: empty id gets a new one; a null or empty definition list gets the default definition; each definition: empty id new, blank name becomes "State - 1", name trimmed, description and items default; each item: empty id new, **null name** (not blank) becomes "State Item - 1", ids default (`STATE/StateItemData.cs:80-89`). `CreateDefault(name)` trims, blank uses the default (`STATE/StateDefinitionData.cs:27-34`). Item default: white, no elements, name "State Item - 1". Naming: `{base} - {n}` first unused from 1, case-sensitive; bases "State" and "State Item" (`STATE/StateNamingRules.cs:8`, `:13`, `:47-59`). `GetNextStateItemName(base, names)` trims, blank base means "State Item" (`:38-45`). `TryNormalizeName(name, current, existing, out n)`: trimmed, not blank, not equal (ordinal) to any existing name other than `current` (`:61-69`).

**F127. State Mapper window.** Title "State Mapper" (`SM:21`, `:112`). 920x560, min 720x420, centre on owner, background themed (`SV:11-17`). Top: Catel validation summary strip (`WarningAndErrorValidator`) (`SV:31-33`). Bottom right "OK" (default), "Cancel" (`SV:35-46`). Middle: top row spanning both panes, then two panes separated by a 1 px splitter (`SV:48-57`, `:203-207`). Works on a clone; `_source` is only replaced in `SaveAsync` (`SM:93-94`, `:392-403`). On close, whatever the result, previews are released (`SM:407-411`).

**F128. Definition row.** "Name", combo (min 180) of definition names, "Add", "Delete", "..." menu with "Rename" and "Copy" (`SV:68-95`). "Description" multi-line box (accepts Return, min height 48, per keystroke) (`SV:97-103`).
 - Selection is refused (snaps back) while blocking errors exist (`SM:211-227`, `HasBlockingValidationErrors`: no definitions, any blank definition name, any blank item name, or an exact-name duplicate, `:930-938`). Selecting clears assignment selections on the old definition and the item selection, resets preview (`SM:229-232`, `:1025-1048`). Note: nothing is selected in the item grid after picking a definition.
 - Add (`SM:415-436`), enabled without blocking errors: prompt "Add State Definition", suggestion next name; default definition with the first item coloured `GetDefaultStateItemColor`.
 - Delete (`SM:438-453`), enabled when more than one definition: confirm `Delete State definition '{name}'?`, title "Delete State Definition", question, Yes/No (`STATE/Setup/Services/StateDefinitionDialogService.cs:57-66`); next selection same index or last. **Unlike the Custom Prop tab, the last definition can't be deleted.**
 - Rename (`SM:457-479`), Copy (`SM:483-507`), both enabled without blocking errors. Copy suggestion "`{name} Copy`", `CloneAsNew` new ids, keeps assignments.
 - (audit) Prompt titles and arguments: Rename opens the name prompt (F134) titled "Rename State Definition" with the current name both as the suggestion and as `currentName` (so keeping the same name is allowed and is a no-op rename); Copy opens it titled "Copy State Definition" with `currentName` null, so the copy must differ from every existing name including its source. In Add, Rename and Copy a null result (Cancel) or a result that fails `TryNormalizeName` (blank or an ordinal duplicate) silently does nothing. The copy is appended at the end of the list and becomes the selected definition.
 - (audit) **There is no name text box in the mapper.** The "Name" label sits beside the definition combo only. The view model has a `Name` property that trims and pushes into the selected definition (`SM:147-161`), but nothing in `SV` binds a text box to it, so a definition's name changes only through "Rename". The field messages of F135 about the definition name therefore describe the selected definition's current name (they can only fire for data that was already bad when loaded, which `Normalize` on open mostly prevents).
 - (audit) Delete is **not** gated on blocking errors, only on there being more than one definition (`SM:438`). The definition selected afterwards goes through the same snap-back setter. On open the draft is normalized (F126) and the first definition is selected (`SM:93-94`, `:114`).

**F129. Preview toggle.** Label "Preview", toggle reading "Off"/"On", default off, min width 48 (`SV:122-139`). On: for selected items only (empty selection means nothing), `(leaf id, #RRGGBB)` pairs, leaves from the effective assignment set (`SM:966-984`). `StatePreviewCoordinator.Refresh` sends only the difference: turns off elements whose pair left, then turns on new pairs (`STATE/Setup/Preview/StatePreviewCoordinator.cs:27-52`). Off: `Clear` (`SM:295-298`). Close: `Release`. Publishes on the live-preview broadcast with context "State Preview": `TurnOnElements` with `Duration = int.MaxValue`, `Intensity = 100`; `TurnOffElements`; `ClearActiveEffects`; `ReleaseContext` (`STATE/Setup/Preview/BroadcastStatePreviewPublisher.cs:11-74`). Refreshes on name, colour or assignment change of a previewed item, selection change, add, remove and reorder (`SM:895-914`).
 - (audit) Details of the difference rule (`StatePreviewCoordinator.cs:27-52`): a pair is `(element id, colour)`, so changing an item's colour removes the old pair, which turns that **element** off and forgets every active pair for it (`:39`), and the new pair is then turned on in the same refresh. Pairs are de-duplicated before sending (`SM:973-977`, publisher `:18-19`, `:45-47`); nothing is published when a list is empty (`:28-31`, `:48-51`). When two selected items both cover a leaf with different colours, both pairs are sent in one `TurnOnElements` message in item order; which colour shows is up to the live-preview consumer (see `docs/spec/preview.md`). Switching definition with preview on clears, then refreshes with the now empty selection (`SM:1043-1047`).

**F130. State Items grid.** Bold centred "State Items". Columns: "Name" (*, editable, commit on focus loss, validation), "Color" (120; hex centred, contrast text, background colour; double-click opens picker), "Count" (72, read-only: distinct effective leaf count, `STATE/Setup/ViewModels/StateItemViewModel.cs:125-128`) (`SV:143-181`). Extended selection, full row, no add row. **Header click sorts and persists the visible order, then clears the indicator** (`DataGridSortedItemsCommandBehavior`, `SV:152-153`, `SM:659-709`). Name setter trims (`StateItemViewModel.cs:93-97`). Row error "State item name is required." (`:139-142`).
 - (audit) With more than one row selected the view model's `SelectedItem` becomes null (`SM:1085-1094`), so the Assigned Elements tree is empty and "Toggle Selected" is disabled (both bind through `SelectedItem`, `SV:219-225`), while Preview shows every selected item. The Name cell commits on focus loss (`UpdateSourceTrigger=LostFocus`), so a name typed but not yet committed is not seen by validation, by the button gates or by OK until focus leaves the cell.

**F131. Colour picking** (`SM:713-733`, `STATE/Setup/Services/StateColorPickerService.cs`). Nodes = effective leaves of the item, or the root if none. Valid colours = union of `ColorModule.getValidColorsForElementNode(node, true)`. Any: discrete single-colour picker with those, pre-selected current. None: full colour picker. Waits for the mouse button to rise before opening (a double-click artefact, not needed on the Mac).

**F132. Item buttons** (right aligned, `SV:182-200`). "Add": `State Item - n` next unused, colour `GetDefaultStateItemColor`, selected (`SM:511-520`). "Copy" (exactly one selected): `CloneAsNew`, name next of `{source} - n` (`SM:522-537`). "Remove": selected items or current; confirm one `Delete State item '{name}'?` title "Delete State Item", several `Delete {count} State items?` title "Delete State Items". Afterward: one removed selects same index or last; several selects nothing (`SM:561-603`). Up and Down arrows: tooltips "Move selected State Item up", "Move selected State Item down"; enabled with at most one selected and room (`SM:605-652`). (audit) Gates: "Add" is enabled whenever a definition is selected, even while blocking errors exist (`SM:509`); "Copy" needs exactly one selected item that belongs to the selected definition (`SM:522-523`); "Remove" uses the multi-selection, or the current item when the multi-selection is empty (`SM:1189-1203`); answering No to the confirmation changes nothing. Default colour (`SM:1225-1258`): for each leaf collect discrete colours (`getValidColorsForElementNode(leaf, false)`), ignoring leaves with none; if all share at least one, the first of the first leaf's list that is common; otherwise white.

**F133. Assigned Elements pane.** Centred "Assigned Elements", "Toggle Selected" button at right (tip "Toggle selected assigned elements", enabled with rows selected) (`SV:219-225`). A virtualized tree of the node's hierarchy shown only when exactly one item is selected (`SM:258-259`), one checkbox per node, name; selected row highlighted (`SV:227-276`). Selecting an item expands to checked descendants: each node is expanded exactly when a descendant is checked, **collapsing others** (`STATE/Setup/Models/StateAssignmentTreeNode.cs:160-171`).
 - Check rules (`StateAssignmentTreeNode.cs:64-81`, `:178-211`): checking a node clears all descendant checks and disables descendants; unchecking re-enables. Disabled nodes ignore changes. Load: a checked ancestor wins (`:28-40`).
 - Stored ids: the highest checked node on each path (`GetSelectedNodeIds`, `:120-125`). Effective leaves: leaves below checked nodes (`:131-136`). Stored as `ElementNodeIds` immediately on change (`StateItemViewModel.cs:165-171`).
 - (audit) Toggle order matters (`StateAssignmentTreeSelectionController.cs:125-144`): selected nodes are flipped in tree order, parents before children. If a parent and one of its descendants are both selected and the parent becomes checked, the descendant has just been unchecked and disabled, so it is skipped and deselected; the result is the parent alone. A plain click on a disabled row clears the whole row selection and the anchor (`:40-55`); ⌘-click and ⇧-click on a disabled row do nothing (`:65-68`, `:91-94`). ⇧-click with no usable anchor (none, disabled, or hidden by a collapsed parent) behaves as a plain click (`:99-106`). The selected-row count that enables "Toggle Selected" counts enabled selected rows only (`:33-34`).
 - Row selection (`STATE/Setup/Models/StateAssignmentTreeSelectionController.cs`): click selects one, ⌘-click toggles, ⇧-click selects the visible enabled range from the anchor; disabled nodes can't be selected; anchor resets to the first selected when toggled off (`:40-120`). Space and Toggle Selected flip each selected enabled node then drop selections that became disabled (`:125-144`; `ITreeViewRangeSelectionController.ToggleCheckedStateForSelectedItems`). Tree behaviour itself lives in `Common.WPFCommon.Behaviors.TreeViewRangeSelectionBehavior`, outside this area.

**F134. State definition name prompt.** Title from the caller, width 360 (min 320), height to content, no resize (`STATE/Setup/Views/StateDefinitionNameDialogView.xaml:9-16`). "Name" field, focused on show (`.xaml.cs:25-35`), OK (default), Cancel. Per-keystroke validation; OK disabled on errors: "State definition name is required."; "State definition names must be unique." (ordinal, excluding `currentName`) (`STATE/Setup/ViewModels/StateDefinitionNameDialogViewModel.cs:102-113`). `Warning` "State definition names differ only by casing. Check you don't have a typo." is computed but the view has no element bound to it (`:115-121`, view file). OK returns the trimmed name. Used by both the mapper and the Custom Prop State tab.
 - (audit) Min height 120; the field is focused once after first render, without selecting its text (`.xaml.cs:25-35`); Return is OK (`IsDefault`), Escape is Cancel (`IsCancel`) (`.xaml:30-37`). Owner is the active WPF window, else the main window, else it is centred on screen (`STATE/Setup/Services/StateDefinitionDialogService.cs:43-50`, `:89-95`). Validation runs on the trimmed text (`StateDefinitionNameDialogViewModel.cs:104`). The view model is validated at construction, so OK is already disabled when the suggested name is a duplicate.

**F135. Mapper validation messages.** Errors: "State definition name is required." (`SM:740`), "Every State item requires a name." (`SM:753`), "State definition names must be unique." (`SM:763`). Warnings shown in the strip, not blocking: "State definition names shorter than three characters may be unclear." (`SM:744`), "State definition names differ only by casing. Check you don't have a typo." (`SM:775`). OK normalizes names, validates, replaces `StateDefinitions` (`SM:392-403`).
 - (audit) Scope differs between the message and the gates: the strip message "Every State item requires a name." is computed for the **selected** definition's items only (`SM:751`), while `HasBlockingValidationErrors`, which gates definition switching, Add definition, Rename, Copy and OK, looks at every definition (`SM:930-938`). Because a definition can't be left while it has a blank item name, the two agree in practice; the port should compute the message over all definitions so the strip can never be empty while OK is refused. Duplicate checks are ordinal (case-sensitive); the casing warning is the ordinal-ignore-case duplicate that is not an exact duplicate.

**F136. Dead code.** `STATE/Setup/Models/StateDefinition.cs` (a Catel model with `Element`, `StateColor`, `StateItemName`, `StateName`) has no callers in this area and registers its properties under the wrong names. Do not port. `STATE/Setup/Models/StateBindableBase.cs` is ordinary change notification.

**F137. Who uses State.** The State effect (`crates/fx-audio-state/src/state.rs`, `Vixen.Modules/Effect/State/`) reads the property; Custom Prop states become this property when a prop is placed in a preview. **Correction (audit): that conversion is done by `Vixen.Modules/Preview/VixenPreview/PreviewCustomPropBuilder.cs:167-331`, not by `CustomPropStateDataMapper`, which has no production caller** (see "CPE model rules"). `StatePropertyId` on an element model is the property id used then (a new GUID when empty, `:328-331`). Per element, bottom up: authored `StateDefinitionModels` win; else legacy imported `StateDefinitions` rows grouped by trimmed definition name and ordered by `Index` within a group; else children carrying a legacy single `StateDefinition`, grouped by definition name, one item per child (`:167-254`). Element-model ids are translated to the created element-node ids, ids with no created node are dropped, then items with no elements and definitions with no items are dropped (`:256-326`).

### O. Moving head graphics (`Editor/FixtureGraphics`)

No window of its own. Two renderers of the same fixture model, both consumed outside this area: `MovingHeadWPF` (wizard pages, Pan/Tilt panel, GDI preview fallback) and `MovingHeadOpenGL` (preview). `crates/vixen-render/src` has no moving head yet (`renderer.rs`, `shaders.wgsl`), while `crates/vixen-preview/src/shape/moving_head.rs` has the shape data. **All geometry below belongs in Rust as one mesh builder drawn by `vixen-render`.**

**F138. Settings** (`FGFX/IMovingHead.cs`, `FGFX/MovingHeadSettings.cs`). Defaults: `BeamLength` 1 (WPF wrapper sets 20), `OnOff` true, `BeamColorLeft`/`Right` Yellow, `Intensity` 100, `Focus` 100, empty `Legend`, `IncludeLegend` true, `LegendColor` Red, `FixtureIntensity` 255, `MountingPosition` Bottom, `OptimizeOpenGLVertexData` true (`:16-33`). Also `PanAngle`, `TiltAngle`, `EnableGDI`, `StrobeRate`, `CommandedPanAngle`, `CommandedTiltAngle`, `UnlimitedPanAngle`, `UnlimitedTiltAngle`. `GetOrientationSign`: Top is -1, else +1 (`:164-177`). `Equals` compares all except `EnableGDI` and `OptimizeOpenGLVertexData` (`:188-210`).

**F139. Geometry constants** (`FGFX/Geometry/MovingHeadGeometryConstants.cs`), all proportional to `length`: base width and depth 0.45, base height 0.07, viewport bottom 0.5, support width 0.05, support height 0.3, support base depth 0.2, support top depth 0.1, housing radius 0.18, support X offset `radius*1.4`, housing length `2*length*0.3`, housing split 0.33 below and 0.67 above the horizontal, horizontal cylinder radius 0.07, beam bottom radius 0.1, beam top radius `8*bottom`, beam length 0.3. Positions: base y `bottom - baseH`; support y `-sign*(bottom - 2*baseH - supportH)`; horizontal cylinder y `-sign*(bottom - 2*baseH - housingLen*0.75)`; housing y `cylinderY - sign*housingLen*0.33`; legend y `-length + baseH`, x `-baseWidth*1.25`, height `3*baseH`.

**F140. OpenGL model** (`FGFX/OpenGL/MovingHeadOpenGL.cs`). `Initialize(length, maxBeamLength, beamTransparency, beamWidthMultiplier, mounting)` (`:591-607`). Volumes in order: base `Rectangle`; left and right `TaperedRectangle` (base and top depth swapped for Top mounting, `:298-333`); horizontal `Cylinder` of length `2*supportXOffset`; housing `RotatingCylinderWithEndCaps` with Y offset `housingLen*0.33`. Two beam halves, `BeamRotatingCylinderWithEndCaps` with panels 5..14 (left) and 0..4 + 15..19 (right), so a beam can show two colours (`:183-209`, `Volumes/BeamCylinderWithEndCaps.cs:32-35`). Beam: length `maxBeamLength * BeamLength/100`, bottom radius 0.1, top radius `multiplier*bottom*Focus/100`, colour HSV with V scaled by Intensity/100 (`:153-180`). When off or `BeamColorLeft == Transparent`: black opaque stub of length 0.5 with equal radii, always visible (a "black disc", `:221-272`).
 - Transforms (`:379-581`): Z at `-2*baseDepth`, base at y `-sign*baseY + yPos`; group translation y `-sign*bottom + sign*2*baseH + yPos`, x `xPos`; pan is group rotation about Y; supports positioned `±supportXOffset`, y `sign*supportH`; horizontal y `sign*housingLen*0.75`; housing and beam tilt via `TiltRotation` about X, `TiltTranslation` y `sign*housingLen*0.75`. Beam position y `housingLen - housingLen*0.33`. **Bottom mounting adds 180 to tilt** (`:636-645`).
 - Model matrix order: rotate XYZ, translate position, rotate tilt XYZ, translate tilt, rotate group XYZ, translate group (`Volumes/RotatingCylinderWithEndCaps.cs:53-69`), others omit tilt (`Volumes/CylinderBase.cs:27-37`, `Volumes/TaperedRectangle.cs:200-210`, `Volumes/Rectangle.cs:158-164` which also omits group).
 - Legend: QuickFont Courier 32 (`cour.ttf`) at `(x + legendX, y - bottom, -baseDepth)` when `IncludeLegend` (`:29`, `:659-681`). **`_qFont` is static and `Dispose` on any instance destroys it for all** (`:57-62`, `:692-708`).
 - Dirty checks avoid work: `AreStaticVolumesDirty` on pan, tilt, x, y; `AreDynamicVolumesDirty` on length, max, focus, beam length, colours, on/off, intensity (`:130-140`, `:363-369`).
 - **Correction (audit): the dynamic check never succeeds.** The eight `_previous...` fields it compares (`:111-118`) are initialized to -1, Transparent or false and are **never assigned afterwards** (only the four pan, tilt, x, y fields are updated, `:409-414`), so `AreDynamicVolumesDirty` is true on every call and `UpdateDynamicVolumes` runs every frame. The real saving is one level down: `CylinderWithEndCaps.Update` rebuilds vertices and sets `VertexDataIsDirty` only when length or a radius changed (`Volumes/CylinderWithEndCaps.cs:74-92`; a change of only `yOffset` or panel list is not detected), and colour and transparency are plain properties sent as uniforms each draw. The port should implement the intended check properly (compare, then store).
 - (audit) First build versus later updates (`:183-273`): `Initialize` creates both beam halves with the lit colours and the configured transparency whatever `OnOff` says (`:183-210`); the off or Transparent case (black, transparency value 1.0, length 0.5) is applied only on later `UpdateVolumes` calls (`:248-273`), which also force `Visible = true` on both halves (`:216-217`). The off test looks at `BeamColorLeft` only; a Transparent right colour with a visible left colour draws the right half with whatever HSV conversion of Transparent gives (black). `InitializeStaticVolumes` writes the mounting position into `MovingHead.MountingPosition` (`:288`). `UpdateVolumes(translateX, translateY, maxBeamLength)` takes integer offsets (`:627-654`).
 - (audit) The two beam halves are not equal halves. With the F141 panel angles, panel 19 lies exactly on top of panel 0, so the "left" list (panels 5..14) covers 10/19 of the circumference and the "right" list (0..4 and 15..19) covers 9/19 with one panel doubled. A clean port should split a 20-sided beam 10 and 10; say so in `docs/DIFFERENCES.md` if the look is changed.

**F141. Volumes.** Cylinder: 20 panels at `theta = x/19 * 2π` (`segments - 1` in the divisor, **so the last panel overlaps the first**; keep the look or use a clean 20 sides) (`Volumes/CylinderBase.cs:209-218`). Vertical panel corners at y `-offset` and `length-offset`; horizontal panels along X (`:70-180`). Normals by cross products with `negateNormals` (`:231-270`). End caps: pie wedges to centres at y `-offset` and **y `+offset` (top cap centre isn't at the top; `yOffset` not `length-offset`)**, normals -Y and +Y (`Volumes/CylinderWithEndCaps.cs:131-168`). `Update` re-creates geometry only when length or radii change (`:74-92`). Rectangle: 36 vertices, half extents (`width`, `height`, `depth` are half sizes) (`Volumes/Rectangle.cs:39-98`). Tapered: base depth at -height, top depth at +height (`Volumes/TaperedRectangle.cs:41-98`).

**F142. Shaders and draw order** (`FGFX/OpenGL/Shaders/*.cs`, `FGFX/OpenGL/MovingHeadRenderStrategy.cs`). Vertex shader: projection, view, model; `FragPos` world position; normal through inverse-transpose (`VolumeShader.cs:442-462`). Fragment: ambient 0.3 white, diffuse `max(dot(n, l), 0)`, specular 0.2 with power 32. Gray: `(ambient + diffuse + specular) * grayColor`, alpha 1, specular tinted by gray; `grayColor` = Gray with HSV V scaled by `Intensity/255`, where `Intensity` is the preview's background alpha (`GrayVolumeShader.cs:53-93`, `:112-125`, `MovingHeadRenderStrategy.cs:246-250`). Colour: same with `color`, specular uses `lightColor`, alpha `transparency` (`ColorVolumeShader.cs:57-95`). Note: `lightColor` is never set by any code read here, and light and view positions are both the camera (`VolumeShader.cs:408-414`). `RenderVolumes(projection, camera, view, backgroundAlpha)`: static volumes then dynamic; global uniforms sent only when changed, vertex buffers only when dirty or first time; all volumes of one shader share a VAO, drawn with `DrawArrays` by offset (`MovingHeadRenderStrategy.cs:185-228`, `:296-345`, `VolumeShader.cs:291-345`). **Both beam halves are registered under both colour shaders** (`MovingHeadOpenGL.cs:618-619`), so each draws twice.

**F143. WPF model** (`FGFX/WPF/MovingHeadWPF.cs`). Same parts built as `MeshGeometry3D` with diffuse materials and 16-sided cylinders (`:206-297`, `:941`). Camera perspective, FOV 45, at (0,0,10), looking at origin, up Y (`:525-540`, `:892-893`). Lights: ambient Gray, directional Gray toward (0,0,-10) (`:546-550`). `length = tan(22.5°)*cameraPosition`, `maxBeamLength = 2*length`, camera position 10 (`:1056-1059`). Beam top radius here `bottom + top*Focus/100` where `top = 8*bottom` (`:373-379`): **differs from the OpenGL model**, and beam colour uses left colour only. Label via a textured quad (Arial, width `len*height`). Pan rotation about Y via `AxisAngleRotation3D`; tilt about X at the horizontal cylinder (`:960-969`, `:1202-1221`). `DrawFixture` renders to a `RenderTargetBitmap` (Pbgra32) and copies pixels into `BitmapColors` for the GDI preview (`:1259-1296`); `DrawFixtureNoBitmap` updates a live `Viewport3D` for the wizard (`:1307-1311`). Beam geometry rebuilt when length, focus, intensity or left colour change (`:1078-1103`). `InvalidateGeometry` forces rebuild (`:1316-1321`).
 - (audit) **Beam off means no beam here**, unlike the OpenGL black stub: the class keeps two model trees and two viewports, with and without the beam, sharing the base, supports and housing (`:861-906`, `:973-1003`); `DrawFixture` renders the with-beam viewport when `OnOff` is true and the other when false (`:1270-1279`). `DrawFixtureNoBitmap` never switches: the caller's `Viewport3D` is always the with-beam one (`:871-880`, `:1307-1311`), so the wizard and Pan/Tilt panel always show a beam.
 - (audit) **Tilt convention differs from OpenGL**: this model applies `TiltAngle` as is (`:1146`, the `+ 180.0` is commented out) and builds the beam from the top of the housing along `sign * maxBeamLength * BeamLength/100` (`:382-388`), whereas the OpenGL model adds 180 for bottom mounting (F140). The shared Rust mesh builder retains an explicit caller-supplied offset. **Owner decision, November 11, 2026:** every port fixture view uses OpenGL's +180° bottom-mounted offset, including editor thumbnails. The original WPF convention above is retained as source evidence, not a separate port requirement.
 - (audit) Constructor overrides: `BeamLength` 20, on, both colours Yellow, intensity 100, focus 100, legend on, legend colour Red (`:28-50`). The legend quad is rebuilt when the text or colour changes and is added to or removed from the base group when `IncludeLegend` flips (`:1106-1140`); it is `text.Length * height` wide, single-sided unless asked, textured from an Arial `TextBlock` through a `VisualBrush` (`:748-838`). Pan is `_baseRotation.Angle = PanAngle` in degrees (`:1143`). The base and legend take only the translation; supports, housing and beam take rotation then translation (`:929-966`, `:994-995`). Tilt rotates about the horizontal cylinder's centre after translation (`:969`, `:1202-1221`). `scaleFactor` and the computed `orgHeight` are unused (`:1047`, `:1062`). When the viewport size and the bitmap size disagree the frame is skipped and a refresh is forced on the next call (`:1282-1295`). `CheckAccess()` exposes the owning thread test (`:1244-1248`): every call must come from the thread that built the model.

**F144. Who depends on the graphics.** `Vixen.Modules/Preview/VixenPreview/Shapes/PreviewMovingHead.cs`, `MovingHeadIntentHandler.cs`, `MovingHeadTimer.cs`, `OpenGL/OpenGLPreviewForm.cs`, `VixenPreviewControl.cs` (see `docs/spec/preview.md`), the wizard and the Pan/Tilt panel. Keep one Rust source of truth, parameterized for the two visible differences: `BeamWidthModel::{PreviewMultiplier, EditorTopPlusBottom}`.

### Open questions for the owner

1. **Priority.** The owner's show uses almost none of this area today.
   - It has no `.prp` or `.xmodel` files and no custom prop shapes in its preview.
   - No Morph effects in its 83 sequence files.
   - No State or Intelligent Fixture properties.
   - An empty `Fixtures/Images` folder.
   - Only the `CustomPropEditorData` settings exist.

   Build this area after the ones the show depends on?
2. **Custom Prop Editor undo** (F12). Add snapshot undo, which is recommended, or keep parity with none?
3. **Fix or keep each upstream bug** listed under "Proposed `docs/DIFFERENCES.md` rows". Three change data and need a yes from the owner:
   - the `Ellipse.Scale` y factor, which moves existing Morph ellipses;
   - `FaceOutline2`;
   - Align Right and Align Bottom.
4. **`.prp` windows.**
   - `WindowGroup(id: WindowID.customPropEditor, for: UUID.self)`, as `WindowID` already declares, with Open and Save inside the window.
   - Or `DocumentGroup`, which gives Finder double-click, autosave, Versions and recent documents.
5. **Background JPEG.** Keep the original bytes, as proposed, or re-encode on every save like upstream?
6. **Legacy LiteDB `.prp` reader.** Is it needed? The owner has no `.prp` files. It means porting about 600 lines: 480 lines adapted from MIT-licensed LiteDB code (`CPE/Persistence/Legacy/LiteDb5021/`) plus the 122-line `LegacyLiteDbPropReader.cs`. A legacy test file would also have to be built.
7. **One fixture profile model.** Keep `fx_fixture::spec::FixtureSpecification` or `vixen_session::setup::fixture::FixtureProfile`, with both serializers on it?
8. **Moving head beam width.** Keep both upstream formulas behind `BeamWidthModel`, or use one everywhere?
9. **Timeline colour of a new function.** Copy upstream's random `KnownColor` choice, or use a fixed palette?
10. **Vendor Browser and App Transport Security.** A vendor whose inventory or model links are plain `http://` is refused by `URLSession` by default. Options: allow exceptions per domain, try `https://` first, or show the download error.
11. **Spec ownership.** This spec says it is authoritative over `docs/spec/display-setup.md` F139-F150 and F110, and over `docs/spec/property-filter-controller-setup.md` F193-F211. Confirm that.
12. **Polygon Editor zoom.** Keep the scale fixed at open, for parity, or allow zooming inside the sheet?

## Data and persistence

### Custom prop file (`.prp`)

Read and written only through `IPropModelPersistenceService` (`CPE/Services/PropModelPersistenceService.cs`). Two on-disk formats:

**Schema-1 package (current).** A ZIP with exactly two root entries and nothing else (`CPE/Persistence/PropPackageReader.cs:40-57`):
- `prop.json`: UTF-8 JSON, compressed `Optimal`, camelCase property names (`Documents/CustomPropJsonSerializerContext.cs:5`), max 4 MiB uncompressed (`PropPackageReader.cs:10`).
- `background.jpg`: JPEG, stored with `NoCompression`, max 64 MiB; first bytes must be `FF D8 FF` (`:11`, `:14`, `:73-84`). The image is **always re-encoded to Bgr24 JPEG at the encoder's default quality on every save** (`Persistence/WpfPropImageCodec.cs:9-19`), so repeated saves are lossy.
- Per-entry checks: length >= 1, compressed length >= 1, length <= 64 MiB, ratio `length / compressedLength <= 100`; names exact and no separators.

JSON shape (`Documents/*.cs`):
```
{ "format": "vixen.custom-prop", "schemaVersion": 1,
  "prop": { "id": guid, "name": str, "type": str, "createdBy": str,
            "creationDate": iso8601, "modifiedDate": iso8601,
            "opacity": f64, "width": f64, "height": f64,
            "vendor": { "name","website","contact","email","phone": str },
            "physical": { "height","width","depth","material","nodeCount","bulbType": str, "colorMode": enum-name },
            "information": { "notes": str } },
  "rootElementId": guid,
  "elements": [ { "id": guid, "statePropertyId": guid, "name": str, "order": i32, "lightSize": i32,
                  "modelType": enum-name,
                  "face": { "component": enum-name, "defaultColor": "#AARRGGBB" },
                  "stateDefinitions": [ { "id": guid, "name": str, "description": str,
                                           "items": [ { "id": guid, "name": str, "color": "#AARRGGBB", "elementIds": [guid] } ] } ],
                  "childIds": [guid],
                  "lights": [ { "id": guid, "x": f64, "y": f64, "z": f64, "size": f64 } ] } ],
  "image": { "entryName": "background.jpg", "mediaType": "image/jpeg" } }
```
Elements are flattened breadth first from the root with de-duplication by id (`Persistence/PropDocumentMapper.cs:12-48`); a shared element appears once and is linked by `childIds` from each parent. Child order is `childIds` order. `Order` of a non-leaf serializes as -1 (`ElementModel.Order` getter) and a parent's `LightSize` as its first leaf's size. Colours: `#` plus 8 hex digits of `Color.ToArgb()` (`:112-113`).

**Validation** (`Persistence/PropDocumentValidator.cs`), run on both read and write, all failures raising "The prop package is invalid." with diagnostics: format and schema version exact; none of prop, image, elements, vendor, physical, information null; prop id and root id non-empty; image declaration exact; opacity, width, height finite and width, height >= 1; element ids unique and non-empty; `statePropertyId` non-empty; `lightSize >= 1`; lights have unique non-empty ids, finite coordinates and size > 0; child ids distinct within a parent and all resolve; state definition and state item ids globally unique; item element ids resolve; item colour is exactly `#` + 8 hex; DFS from the root finds no cycle and every element reachable. Enum names failing to parse raise "The ... is invalid." (`PropDocumentMapper.cs:114`). `FaceDefinition`'s colour format is not validated before parsing.

**Save pipeline** (`Persistence/PropPackageWriter.cs:15-42`, `Persistence/AtomicPropFileWriter.cs`). Map and validate; create the destination directory; write to `.<name>.<guid>.tmp` beside the destination; **read that temp file back fully and validate it**; then publish atomically, serialized per canonical path by a semaphore. When the destination exists uses `File.Replace` (with an optional backup path), falling back to overwrite-move on other systems. The temp file is always deleted. On the Mac: `rename(2)` in the same directory is the atomic step.

**Legacy LiteDB v4 `.prp` (read only).** Detected by the first 53 bytes: `"** This is a LiteDB file **"` at offset 25 and version byte 7 at offset 52 (`Persistence/Legacy/LiteDb5021/LegacyLiteDbRawReaderPrototype.cs:15`, `:34-36`). A ZIP signature `50 4B 03 04` wins over this; anything else raises "The prop file format is not supported." (`PropModelPersistenceService.cs:18-29`). The raw reader walks 4096-byte pages itself (no database engine), reads collections `props` (exactly one document), `_files` (`_id` = `$/image/background.jpg` or `.png`) and `_chunks` (`_id` = `<fileId>\<n>`, ordered by n; concatenated `data`). BSON subset in `LegacyBsonDocumentReader.cs`; a `_type` field is rejected (no type resolution). `LegacyLiteDbPropReader` maps to the model (`Persistence/LegacyLiteDbPropReader.cs`): element `_id` or `Id`; `RootNode`, `Children`, `Lights`, `FaceDefinition`, `StateDefinitionModels`, otherwise legacy `StateDefinition` + `StateDefinitions` grouped by `StateDefinitionName` (**names only; legacy colours and ids are dropped in this path**, `:79-94`); colours are `"A,R,G,B"` strings; **vendor, physical and information metadata and creation and modified dates are not read: they reset to empty and now** (`:22-25`). Elements are de-duplicated by id across the document.

**Upgrade on save.** A legacy prop remembers its source path (`_legacySources`, keyed weakly by the `Prop` instance). Saving to that same path (case-insensitive) writes the new package with `<path>.legacy-litedb.bak` as the backup of the old file; if that backup already exists and is a v4 file it is kept and the save goes ahead without another backup; if it exists and is not v4 the save fails (`PropModelPersistenceService.cs:35-56`, `AtomicPropFileWriter.cs:10`, `:24-25`). Saving a legacy prop elsewhere never touches the source.

**Reader, mapper and legacy details (audit).**
- Package reader order (`PropPackageReader.cs:16-38`): the first four bytes must be the ZIP local-header signature, else "The prop file is not a valid package."; entries are checked, then `prop.json` is parsed and **validated before the image entry is read**; any `InvalidDataException` from the archive becomes "The prop package is invalid.". The ratio test is **integer division** (`entry.Length / entry.CompressedLength > 100`, `:50`), so a ratio of 100.9 passes. The 64 MiB cap is applied to both entries first and the 4 MiB cap to `prop.json` after (`:50`, `:55`). The JPEG must decode to exactly one frame, else "The prop package image is invalid." (`WpfPropImageCodec.cs:21-29`).
- Validator additions (`PropDocumentValidator.cs:26-37`): every element also needs non-null `childIds`, `lights`, `stateDefinitions` and `face` ("The prop package contains an invalid element."), light ids are unique **across the whole document** (one shared set, `:23`, `:33`), and `rootElementId` must name an element in `elements` ("The prop package root element is missing.").
- Writing (`PropDocumentMapper.cs:12-48`): a missing root, an empty element id, two different live elements sharing an id, a null children collection or a null child raise "The prop cannot be saved." before anything is written; null strings are written as `""`; the built document is validated before it is returned (`:46`).
- Reading (`PropDocumentMapper.cs:50-76`): elements are created first, then linked: each `childIds` entry appends the child and adds the parent's id to the child's `Parents` (so a shared element ends with one `Parents` entry per parent), each light gets `ParentModelId` = its element, and `NormalizeStateModelData` runs per element. The legacy `StateDefinition` and `StateDefinitions` fields are always empty on a package-loaded element (`:92`). `Prop.Hydrate` assigns the fields directly and `Name` is set last (`:71-74`, `CPE/Model/Prop.cs:207-227`).
- Legacy reader defaults (`LegacyLiteDbPropReader.cs:15-27`, `:30-55`): prop id = `_id`, else `Id`, else new; no `RootNode` document gives a single root element named after the prop's `Name` (else "Legacy Custom Prop"); `Opacity` default 1; `Width`/`Height` default to the image's pixel size; `Name` falls back to the root's name. Per element: `StatePropertyId` and each light `Id` default to **new GUIDs** (so they differ on every load until the prop is saved as a package), `Order` default 0, `LightSize` default 3, light `Size` defaults to the element's light size, unknown `ModelType` or `FaceComponent` names fall back to None (no error, unlike the package path). Authored `StateDefinitionModels` win; the legacy rows are used only when the element has none (`:52-53`), and item `ElementModelIds` are 16-byte binary GUIDs with anything else dropped (`:73`). The legacy image may be any single-frame format the decoder accepts (JPEG or PNG), else "The legacy prop image is invalid." (`:96-104`). `NormalizeStateModelData` is **not** called on this path.
- After a successful in-place upgrade the legacy source is forgotten (`PropModelPersistenceService.cs:52`), so later saves are plain package saves with no backup.
- **Upstream hazard, failed Open:** `PropModelServices.LoadPropAsync` clears the id index `_models` **before** calling the loader (`PropModelServices.cs:61-73`). When the load throws, the editor keeps showing the old prop (F4) but `_models` is now empty until the next successful New or Open, so name uniquifying (`:460-479`) and id lookups (`GetModel`, `GetElementModel`, `:185-190`, `:376-381`) see no elements and `RemoveLight` silently does nothing because the light's parent is not found (`:402-407`). Do not port: a failed open must leave the open document untouched.

**Module settings.** `CustomPropEditorData` in the module data store: `LightColor`, `SelectedLightColor`, `StatePreviewBaseColor`, `DefaultLightSize` (`CPE/CustomPropEditorData.cs`), written with `VixenSystem.SaveModuleConfigAsync()` after the Preferences dialog closes (`PropEditorViewModel.cs:1412-1420`). Empty colours and a zero size repair to defaults on load (`:41-63`).

### Fixture specification data (`App/Fixture`, `App/FixtureSpecificationManager`)

Fixture specifications ("fixture profiles") are the library data every intelligent fixture element references.

**Type definitions** (`FIX/`):
 - `FixtureSpecification` (`FIX/FixtureSpecification.cs:13-83`): `version` attribute (`"1"`), `Name`, `Manufacturer`, `CreatedBy` (default `Environment.UserName`), `Revision` (default `"1.0"`), `ChannelDefinitions: Vec<FixtureChannel>`, `FunctionDefinitions: Vec<FixtureFunction>`.
 - `FixtureChannel` (`FIX/FixtureChannel.cs`): `Name`, `Function` (name of a function, referential), `ChannelNumber`. Parent `FixtureItem { Name }`.
 - `FixtureFunction` (`FIX/FixtureFunction.cs`): `Name`, `FunctionType` enum `FixtureFunctionType { Range "Range", Indexed "Indexed", ColorWheel "Color Wheel", RGBColor "RGB Color", RGBWColor "RGBW Color", None "None" }` (`:13-32`); `FunctionIdentity` (from `Vixen.Data.Value`, default Custom); `IndexData: Vec<FixtureIndex>`; `ColorWheelData: Vec<FixtureColorWheel>`; `Label`; `RotationLimits` (default new); `ZoomType` enum `FixtureZoomType { NarrowToWide "Narrow To Wide", WideToNarrow "Wide To Narrow" }` (`:37-44`); `TimelineColor` (not stored directly; `TimelineColor1Html` string via `ColorTranslator`, empty becomes `White`, default White, `:121-156`); `AssociatedFunctionName`. `GetIndexDataBase` returns index or colour wheel data by type (`:178-197`).
 - `FixtureIndexBase` (`FIX/FixtureIndexBase.cs`): `StartValue`, `EndValue`, `UseCurve`, `IndexType: FixtureIndexType` default Custom.
 - `FixtureIndex`: adds `Image: string` (gobo file name) (`FIX/FixtureIndex.cs`).
 - `FixtureColorWheel` (`FIX/FixtureColorWheel.cs`): `HalfStep`, `Color1Html`, `Color2Html` (via `ColorTranslator`, default White), `ExcludeColorProperty`, `IndexType = ColorWheel`; setting `UseCurve` true forces `IndexType = ColorWheel` (`:78-97`).
 - `FixtureRotationLimits`: `StartPosition = 0`, `StopPosition = 360` (`FIX/FixtureRotationLimits.cs:16-20`).

**Rules** (`FIX/FixtureSpecification.cs`): `GetFileName` replaces `<` and `>` with `(`, and `:` `"` `/` `\` `|` `?` `*` with `_`, then `.xml` (`:94-127`). **Quirk: `>` becomes `(` not `)`.** Keep it so existing files stay matched. `SupportsFunction(name)`: defined and on some channel (`:150-156`). `SupportsFunction(identity)`: first function with that identity is used on some channel (`:209-226`). `GetInUseFunction`, `IsFunctionUsed` (`:164-202`). `IsColorMixing`: some channel's function is RGB or RGBW; `ContainsColorWheel`; `IsRGBW`, all use `Single` so a channel with an unknown function throws (`:362-438`). `CreateInstanceForClone`/`CopyInto` deep-copy, `CopyInto` appends (does not clear) (`:465-495`).

**`InitializeBuiltInFunctions`** (`:266-356`), clears then adds, in order: Pan (Range, identity Pan, Red, rotation limits), Tilt (Range, Tilt, Green, rotation limits), Color Wheel (ColorWheel, Custom, White), Dimmer (Range, Dim, Purple), Color (RGBW, Custom, DarkGray), Zoom (Range, Zoom, Blue), Shutter (Indexed, Shutter, Black), Gobo Wheel (Indexed, Gobo, Pink), Open Close Prism (Indexed, OpenClosePrism, Orange), Prism (Indexed, Prism, Yellow), Frost (Range, Frost, LightBlue), None (None, Custom, Transparent).

**Library manager** (`FSM/FixtureSpecificationManager.cs`). Singleton. On first use: creates `<DataRoot>\Fixtures\` and `<DataRoot>\Fixtures\\Images\` (`:60-83`, `:258-262`), loads every `*.xml` through `FixtureSpecificationService.Load` (`Vixen.Core/Services/FixtureSpecificationService.cs`, see "Fixture specification files" below). A duplicate `Name`: dialog `A Intelligent Fixture with a name of '<name>' already exists.  The file: '<file>' was ignored.` titled "Invalid Fixture Profile" (`:108-111`). Malformed XML: logged, ignored (`:114-119`). Sorted by name (`:144-154`). `Save(fixture)`: XML with `XmlSerializer`, indented with tabs, to `Fixtures\<GetFileName()>`, then replaces the cached item by name and re-sorts (`:188-228`). **Save does not delete a previous file when a specification is renamed** (the rename case below keeps the old file). `GetGoboImages()`: a leading `" "` entry (to clear an image) plus file names in the images folder (`:233-253`). `GetGoboImageDirectory()`.

### Fixture specification files

**Where they live.**
- Profiles: `<DataRoot>/Fixtures/<GetFileName()>`, one `.xml` file per profile.
- Gobo images: `<DataRoot>/Fixtures/Images/`. `FixtureIndex.Image` holds a bare file name from that folder; `" "` means no image.
- The owner's real show has an empty `Images` folder and no profiles.

**Load path.**
1. `FixtureSpecificationService.Load<T>(path)` calls `FileService.LoadFixtureSpecification`, which uses `FixtureSpecificationLoader`, a migrating loader.
2. The file is parsed to an `XElement`.
3. The version comes from the root's `version` attribute (`XmlRootAttributeVersion.GetVersion`). The current version is 1 (`Vixen.Core/IO/ObjectVersion.cs:10`).
4. The migrator has no steps (`FixtureSpecificationXElementMigrator : EmptyMigrator<XElement>`).
5. `XmlSerializer(typeof(FixtureSpecification))` deserializes it.

**Save path.** `XmlSerializer`, indented with tabs (`FSM/FixtureSpecificationManager.cs:188-228`).

**XML shape.** These are `XmlSerializer` defaults.
- One element per public read/write property, in declaration order with base-class members first.
- Root: `<FixtureSpecification version="1">`. Its children are `Name`, `Manufacturer`, `CreatedBy`, `Revision`, `ChannelDefinitions` and `FunctionDefinitions`.
- `ChannelDefinitions` holds `<FixtureChannel>` elements: `Name`, `Function`, `ChannelNumber`.
- `FunctionDefinitions` holds `<FixtureFunction>` elements: `Name`, `FunctionType`, `FunctionIdentity`, `IndexData` (`<FixtureIndex>`), `ColorWheelData` (`<FixtureColorWheel>`), `Label`, `RotationLimits`, `ZoomType`, `TimelineColor1Html`, `AssociatedFunctionName`.
- Enums are written by member name (`RGBColor`, `ColorWheel`), not by their description text.

**Colours.**
- Colours are strings from `ColorTranslator.ToHtml`: a known colour name such as `White`, or `#RRGGBB`, with no alpha.
- The `Color` properties are `[XmlIgnore]`. Only the `...Html` strings are stored (`FIX/FixtureColorWheel.cs:39-68`, `FIX/FixtureFunction.cs:126-156`).
- A missing or empty `TimelineColor1Html` reads as `White`. This repairs files written by an old bug.

**Port.** `crates/vixen-session/src/setup/fixture.rs` already reads and writes this format (`read_profile`, `profile_file_text`, `load_fixture_profiles`, `save_fixture_profile`). Check it against these rules rather than writing a second reader.

### Intelligent Fixture property data

`IFIX/IntelligentFixtureData.cs` is a DataContract stored in the system config's module data. It holds one member, `FixtureSpecification`, in DataContract form. DataContract writes members in alphabetical order with the DataContract namespace, so it is not the same text as the profile file.

- A new instance calls `InitializeBuiltInFunctions()` (`:18-23`).
- `SetDefaultValues` adds channel 1 with no name and no function (`IFIX/IntelligentFixtureModule.cs:28-34`).
- When the property's data is loaded, the module adds its specification to the in-memory library if no library entry has the same `Name` (`IFIX/IntelligentFixtureModule.cs:73-96`). So a show that was copied without its `Fixtures` folder still offers its profiles in the wizard. The added specification is the same object the property holds; F105 describes what that sharing affects.
- This data is already read by `crates/fx-fixture/src/spec.rs` (`parse_specification`, `INTELLIGENT_FIXTURE_TYPE_ID`). The library merge is `add_profiles_in_tree` in `crates/vixen-session/src/setup/fixture.rs`.

### Polygon shapes (inside Morph effect data)

- `PolygonContainer` is never saved.
- Shapes are saved inside the Morph effect's DataContract, in its `MorphPolygonData` list, in the sequence file. See `crates/fx-pixel-physics/src/generated/morph.rs`.
- The editor exchange must keep each shape's `ID` (F73a).
- Ellipses are stored with `Angle`, `Center`, `Width`, `Height`, `StartSideRotation` and the four rectangle corners (F73).

### State property data

The data is a DataContract per element in the system config, defined in `STATE/StateData.cs`, `STATE/StateDefinitionData.cs` and `STATE/StateItemData.cs`:
```
StateData { Id: Guid,
  StateDefinitions: [ StateDefinitionData { Id: Guid, Name, Description: string,
    Items: [ StateItemData { Id: Guid, Name: string, Color: System.Drawing.Color, ElementNodeIds: [Guid] } ] } ] }
```
- `Color` uses the DataContract form of `System.Drawing.Color`: the fields `knownColor`, `name`, `state` and `value`.
- The property type id is `{35B893C3-D10C-4359-82BC-929C0762E809}` (`STATE/StateDescriptor.cs:6`).
- `crates/fx-audio-state/src/property_data.rs` already reads it. The editors are F124-F136.
- A custom prop's own state definitions, stored in `.prp`, are converted to this form when the prop is added to a preview. (audit) The converter is `PreviewCustomPropBuilder` (F137), not `CustomPropStateDataMapper` (unused in production; see CPE model rules).

### Other files, network and remembered state

- **Vendor catalogue.** `GET https://app.vixenlights.com/vendor.json` (`CPE/ViewModels/PropEditorViewModel.cs:1526`), then each vendor's inventory URL (F21, F21a, F69). Nothing is cached on disk.
- **Downloaded vendor models.** Saved to `<temp>/<guid>` with no extension (`PropEditorViewModel.cs:1476-1491`) and never deleted. The port should use a uniquely named file in the app's temporary directory and delete it after loading.
- **Wire diagram.**
  - Original: `Path.GetTempFileName()` creates `<temp>/tmpXXXX.tmp`. The SVG is written beside it with a `.svg` extension and opened in the default app (`MODEL/ElementModeling.cs:75-86`). Both files are left behind.
  - Port: offer Save As, or write into the app's temporary directory and clean up on quit. `write_wire_diagram(points, flip, options, folder)` already exists.
- **Help link.** `http://www.vixenlights.com/vixen-3-documentation/preview/custom-prop-editor/` (`PropEditorViewModel.cs:1385`). On the Mac, open it with `NSWorkspace`.
- **Last folder.** The Import xModel open dialog remembers its folder in a process-wide static, `_lastFolder` (`PropEditorViewModel.cs:33`, `:981-993`).
  - The first use starts at `Paths.DataRootPath`.
  - It is not persisted.
  - On the Mac, let the open panel remember its own location, but start at the data root the first time.
- **Module settings.** `CustomPropEditorData` (see "Custom prop file" above) is the only persisted setting this area writes, apart from the files themselves.

## Rust API needed

**Reuse first.** This area already has partial Rust code. Extend it, and do not write it a second time:
- `vixen-session::setup::fixture`: profiles, wizard pages, element creation.
- `fx-fixture::spec`: the DataContract specification and its enums.
- `vixen-session::setup::wire_diagram`: `WireDiagramOptions`, `wire_diagram_points`, `wire_diagram_svg`, `write_wire_diagram`.
- `vixen-session::dialogs::state` and `state_preview`: the State Mapper draft.
- `fx-audio-state::property_data`: `StateData`.
- `fx-pixel-physics::morph::polygon` and `generated/morph.rs`: shape records.
- `vixen-preview::custom_prop_builder`: turns a prop into preview elements.
- `vixen-preview::shape::moving_head`: the preview shape data.
- `vixen-session::undo`: the existing undo-stack pattern.

**Settle two duplicate models before adding code.**
1. Profiles are modelled twice: `fx_fixture::spec::FixtureSpecification` (read from property data) and `vixen_session::setup::fixture::FixtureProfile` (read from the library files). Keep one in-memory type with two serializers: the DataContract form for property data, and the `XmlSerializer` form for library files.
2. `vixen_preview::custom_prop_builder::ElementModel` stands in for the missing prop model. Replace it with the model below.

**Placement.** A `.prp` prop document is needed by three crates:
- file formats in `vixen-io`;
- the preview builder in `vixen-preview`;
- the editor in `vixen-session`.

Only `vixen-core` sits below all three, so the prop model goes there as plain data plus graph rules, with no IO. Signatures below are plain declarations. "F" numbers point at the behaviour each must match.

### `vixen-core`: new module `custom_prop` (prop document model and graph rules)

**Types.** Their fields mirror the `.prp` JSON in Data and persistence.
- Ids: `ElementId(Uuid)`, `LightId(Uuid)`.
- Document: `Prop`, `ElementModel` (parents and children are id lists), `Light`, `PropImage { jpeg: Vec<u8>, width_px: u32, height_px: u32 }`.
- Metadata: `VendorMetadata`, `PhysicalMetadata`, `InformationMetadata`.
- States: `StateDefinitionModel`, `StateItemModel`, `LegacyStateDefinition`, `FaceDefinition`.
- Enums: `ElementType`, `ElementModelType`, `FaceComponent`, `ColorMode`, `LightSizeOp { Increase, Decrease, Match }`, `AlignOp`, `Axis`.
- Other: `PropClip`, `PropError`, and `StateDefinitionSpec`. `StateDefinitionSpec` is a plain id, name and description, plus items with a colour and element ids. Both `vixen-preview` and `vixen-session` use it, so it lives here.

**Creating and naming.**
- `fn Prop::new(name: &str, width: u32, height: u32, created_by: &str, now: DateTime) -> Prop`: the new-prop defaults. The root is named after the prop, the image is solid black, opacity is 1 (CPE model rules, F3).
- `fn Prop::name(&self) -> &str` and `fn Prop::set_name(&mut self, name: &str)`: the prop name is the root element's name.
- `fn Prop::uniquify(&self, name: &str, order_index: u32, renaming: Option<ElementId>) -> String`: returns `"{name} - {n}"`. Names are unique across the whole prop. Record whether the in-loop exclusion quirk is kept.
- `fn Prop::is_name_duplicated(&self, name: &str) -> bool`: more than one element has the name.
- `fn Prop::rename(&mut self, id: ElementId, name: &str) -> Result<String, PropError>`: inline rename and Rename (F23, F32). Returns the uniquified name.

**Reading the graph.**
- `fn Prop::element(&self, id: ElementId) -> Option<&ElementModel>`
- `fn Prop::children(&self, id: ElementId) -> &[ElementId]` and `fn Prop::parents(&self, id: ElementId) -> &[ElementId]`
- `fn Prop::is_leaf(&self, id: ElementId) -> bool`, `is_light_node`, `is_group_node`, `is_root`: the ElementModel type rules. An empty leaf counts as a group.
- `fn Prop::can_add_group_nodes(&self, id: ElementId) -> bool`, `can_add_leaf_nodes`, `can_add_light_nodes`: the same rules, used for drop targets and menu enabling (F25, F34).
- `fn Prop::element_type(&self, id: ElementId) -> ElementType`: Group, Node or String, by light count.
- `fn Prop::leaf_nodes(&self) -> Vec<ElementId>`: depth first. A shared leaf repeats (`GetLeafNodes`).
- `fn Prop::node_enumerator(&self) -> Vec<ElementId>`: `GetNodeEnumerator` order, used to find the group a new light goes into.
- `fn Prop::next_order(&self) -> i32` and `fn Prop::order_exists(&self, order: i32) -> bool`: `GetNextOrder` and `OrderExists`.

**Editing the graph.**
- `fn Prop::create_node(&mut self, name: &str, parent: Option<ElementId>, one_based: bool) -> ElementId`: Create Empty Group (F25).
- `fn Prop::add_light(&mut self, target: Option<ElementId>, x: f64, y: f64, size: i32) -> Result<ElementId, PropError>`: F42, including find-or-create of the target group. When no group can take a light, return an error where the original's `First()` throws.
- `fn Prop::remove_lights(&mut self, ids: &[LightId]) -> Vec<ElementId>`: F40. Returns the elements removed because they became empty.
- `fn Prop::add_to_parent(&mut self, id: ElementId, parent: ElementId) -> bool`
- `fn Prop::remove_from_parent(&mut self, id: ElementId, parent: ElementId)`: with the orphan cascade.
- `fn Prop::delete_elements(&mut self, ids: &[ElementId])`: Delete (F33).
- `fn Prop::group_elements(&mut self, ids: &[ElementId], name: &str, move_out: bool) -> ElementId`: Copy to New Group links each element under the new group. Move to New Group also unlinks it from its old parents (F26).
- `fn Prop::extract_clip(&self, ids: &[ElementId]) -> PropClip`: Cut and Copy (F27, F28).
- `fn Prop::paste(&mut self, clip: &PropClip, parent: ElementId, as_new: bool) -> Vec<ElementId>`: Paste finds or creates by id. Paste as New makes fresh names, lights and orders (F29, F30). Rust copies lights where the original shared a list; within one document the same id is linked, so the behaviour matches.
- `fn PropClip::to_json(&self) -> String` and `fn PropClip::from_json(s: &str) -> Result<PropClip, PropError>`: the private pasteboard payload that makes F31 work between windows.
- `fn Prop::try_move_within_parent(&mut self, parent: ElementId, ids: &[ElementId], insertion_index: usize) -> bool`: same-parent batch move. `ElementTreeDragDropTests` pins it.
- `fn Prop::reverse_order(&mut self, ids: &[ElementId])`: Reverse on the Order tab (F38).

**Lights and image.**
- `fn Prop::set_image(&mut self, image: PropImage)`: overwrites width and height and leaves lights where they are (F14).
- `fn Prop::scale_lights(&mut self, sx: f64, sy: f64)`: scales each unique light once, by id (F15).
- `fn Prop::transform_lights(&mut self, ids: &[LightId], m: Affine2)`: translate, scale about an anchor, or rotate about a centre (F44, F45).
- `fn Prop::align_lights(&mut self, ids: &[LightId], op: AlignOp)`, `fn Prop::distribute_lights(&mut self, ids: &[LightId], axis: Axis)`, `fn Prop::flip_lights(&mut self, ids: &[LightId], axis: Axis)`: F46, with the Right and Bottom quirk decision recorded.
- `fn Prop::set_light_size(&mut self, ids: &[ElementId], op: LightSizeOp)`: F47.

**States.**
- `fn Prop::model_element(&self) -> ElementId`, `fn Prop::can_assign_model_type(&self, id: ElementId, t: ElementModelType) -> bool`, `fn Prop::try_set_model_type(&mut self, id: ElementId, t: ElementModelType) -> bool`: `PropStateModelResolver`, which keeps at most one Model element.
- `fn Prop::migrate_legacy_states(&mut self) -> bool`: `CustomPropStateMigrationService`.
- `fn Prop::refresh_state_assignments(&mut self) -> bool`: drops ids that no longer exist (F62).

### `vixen-io`: new modules `custom_prop_file`, `xmodel`, `vendor_inventory`

**`custom_prop_file`**
- `fn sniff_prop_file(head: &[u8]) -> PropFileKind`: `Zip`, `LegacyLiteDbV4` or `Unknown`. A ZIP signature wins.
- `fn read_prop_file(path: &Path) -> Result<(Prop, PropFileKind), PropFileError>`: Open (F4). Errors map to the "Prop Load Error" texts.
- `fn read_prop_package(bytes: &[u8]) -> Result<Prop, PropFileError>`: the schema-1 rules: exactly two entries, size and ratio limits, JPEG magic bytes, `prop.json` of at most 4 MiB.
- `fn prop_to_document(prop: &Prop) -> PropDocument` and `fn prop_from_document(doc: PropDocument) -> Result<Prop, PropFileError>`: breadth-first flattening with de-duplication. A non-leaf's order is written as -1.
- `fn validate_prop_document(doc: &PropDocument) -> Vec<String>`: `PropDocumentValidator` diagnostics. An empty list means valid.
- `fn write_prop_package(prop: &Prop) -> Result<Vec<u8>, PropFileError>`: `prop.json` deflated, `background.jpg` stored.
- `fn save_prop_file(path: &Path, prop: &Prop, legacy_source: Option<&Path>) -> Result<(), PropFileError>`:
  - writes a temp file beside the target;
  - reads it back and validates it;
  - renames it into place atomically;
  - keeps `<path>.legacy-litedb.bak` when upgrading a legacy file.
- `fn read_legacy_litedb_prop(bytes: &[u8]) -> Result<Prop, PropFileError>`: the read-only v4 page reader for the `props`, `_files` and `_chunks` collections. It adapts MIT-licensed LiteDB code, so carry `CPE/Persistence/Legacy/LiteDb5021/THIRD-PARTY-NOTICES.md` along with it.
- `fn load_prop_editor_settings(store: &ModuleStore) -> PropEditorSettings` and `fn save_prop_editor_settings(store: &mut ModuleStore, s: &PropEditorSettings)`: `CustomPropEditorData`, including the light-size clamp (F19).

**`xmodel`**
- `fn read_xmodel(text: &str) -> Result<XModelFile, XModelError>`: file routing, with the F63 and F64 error texts.
- `fn xmodel_candidates(file: &XModelFile) -> Vec<XModelCandidate>`: `Unnamed model {n}`, ` (n)` suffixes for duplicates, and a supported flag (F64).
- `fn parse_common(e: &XElem) -> XModelCommonConfiguration` and `fn coordinate_scale(x: i32, y: i32, e: &XElem) -> XModelCoordinateScale`: the default scale is 4, 2 or 1, then `ScaleX`/`ScaleY`, rounded away from zero (F65).
- `fn parse_custom_nodes(e: &XElem) -> Result<CustomXModelParseResult, XModelError>`: `CustomModelCompressed` entries (`order,row,column`) or the `CustomModel` grid (F65).
- `fn parse_circle(e: &XElem) -> Result<CircleXModelConfiguration, XModelError>`: F66 validation and messages.
- `fn circle_rings(cfg: &CircleXModelConfiguration) -> Vec<CircleXModelRing>`, `fn circle_nodes(cfg: &CircleXModelConfiguration) -> Vec<ModelNode>`, `fn circle_groups(name: &str, rings: &[CircleXModelRing], sub_models: &[SubModel]) -> Vec<XModelGeneratedGroup>`: ring numbering, node positions, and the "Circles" group, which is skipped when a matching submodel exists.
- `fn parse_child_elements(e: &XElem) -> XModelChildren`: subModel, faceInfo, stateInfo and aliases. `modelGroup` is skipped (F67).
- `fn parse_node_ranges(text: &str) -> Result<Vec<NodeRange>, XModelError>`: return an error where the original throws.
- `fn normalize_xlights_state_name(name: &str, n: usize) -> String` and `fn normalize_xlights_state_item_name(name: &str, tag: &str) -> String`
- `fn assemble_prop(model: &XModelParsedModel) -> Prop`: `XModelPropAssembler`, with offset 7, a minimum of 800x600, and `CreatedBy = "xModel Import"` (F68).

**`vendor_inventory`** (parsing only; downloading happens elsewhere, so tests never touch the network)
- `fn parse_vendor_list(json: &str) -> Result<Vec<VendorLink>, String>`: parses `vendor.json`.
- `fn parse_vendor_inventory(xml: &str) -> Result<ModelInventory, String>`: maps `<modelinventory>` to vendors, categories, products and model links (F69). Return an error where the original throws on a bad URL.

### `vixen-preview`
- `fn build_custom_prop(prop: &custom_prop::Prop, tree: &mut ElementTree, zoom_level: f64, substitute: impl FnMut(&str) -> String) -> (Shape, NodeId)`: the existing builder, taking the real prop model instead of its local stand-in.
- `fn custom_prop_state_data(prop: &custom_prop::Prop, element: ElementId) -> Vec<StateDefinitionSpec>`: the plain definitions produced when a prop joins a preview. (audit) Follow `PreviewCustomPropBuilder.cs:167-331` as described in F137 (authored, else legacy rows, else legacy per-child; ids translated; empty items and definitions dropped), not `CustomPropStateDataMapper`. `vixen-session` writes them into `fx-audio-state` `StateData`.

### `vixen-session`: new module `custom_prop` (one `PropEditor` per window, no singletons)

**Document.**
- `fn PropEditor::new_document(name: &str, settings: PropEditorSettings) -> PropEditor`: New (F3). Appends ` {1}` unless the name already holds a `{n}` token, and starts in draw mode (F41).
- `fn PropEditor::open(path: &Path, settings: PropEditorSettings) -> Result<PropEditor, String>`: Open (F4). Run it off the main thread.
- `fn PropEditor::save(&mut self) -> Result<(), String>` and `fn PropEditor::save_as(&mut self, path: &Path) -> Result<(), String>`: F5 and F6, with state validation first (F61). An untitled document goes through `save_as`.
- `fn PropEditor::import_xmodel(&mut self, path: &Path, choose: &dyn Fn(&[XModelCandidate]) -> Option<usize>) -> Result<Vec<String>, String>`: F7 and F63-F68. Replaces the document, switches to selection mode, and returns warnings.
- `fn PropEditor::load_vendor_model(&mut self, bytes: &[u8], kind: ModelLinkKind, product: &Product, vendor: &Vendor) -> Result<(), String>`: opens a downloaded `.prp` or xModel (F21, F21a). After an xModel it copies the product's physical fields and notes into the metadata. For both kinds it fills empty vendor fields from the vendor (F21 step 5).
- `fn PropEditor::is_dirty(&self) -> bool` and `fn PropEditor::file_path(&self) -> Option<&Path>`: the close prompt (F9) and the window title.

**Rows for the views.**
- `fn PropEditor::tree_rows(&self) -> Vec<TreeRow>`: outline rows. Each row carries its id path, so a shared leaf shows under every parent (F22).
- `fn PropEditor::order_rows(&self) -> Vec<OrderRow>`: the Order tab (F37).
- `fn PropEditor::light_list(&self) -> Vec<LightView>`: the canvas draw list with id, x, y, size and colour role: normal, selected or state preview (F48, F60).
- `fn PropEditor::status(&self) -> PropStatus`: pointer coordinates and status message (F17).

**Selection and canvas.**
- `fn PropEditor::select_elements(&mut self, ids: &[ElementId], extend: bool)` and `fn PropEditor::select_lights(&mut self, ids: &[LightId], extend: bool)`: two-way sync between tree and canvas (F49).
- `fn PropEditor::set_mode(&mut self, mode: CanvasMode)`: Draw or Select (F41). State mode follows the selected tab (F59).
- `fn PropEditor::pointer_down(&mut self, p: Point, m: Modifiers) -> CanvasFeedback`, `fn PropEditor::pointer_dragged(&mut self, p: Point, m: Modifiers) -> CanvasFeedback`, `fn PropEditor::pointer_up(&mut self, p: Point, m: Modifiers) -> CanvasFeedback`: add, select, rubber band, move and state assignment (F42-F44, F59). Hit testing is circle maths in Rust, not view lookups.
- `fn PropEditor::adorner(&self) -> Option<AdornerGeometry>`: shown only for more than one selected light in selection mode (F45).
- `fn PropEditor::drag_adorner(&mut self, handle: AdornerHandle, delta: Vector) -> bool`: resize, move and rotate, with the `EnforceSize` clamp (F45).
- `fn PropEditor::align(&mut self, op: AlignOp)`, `fn PropEditor::distribute(&mut self, axis: Axis)`, `fn PropEditor::flip(&mut self, axis: Axis)`, `fn PropEditor::light_size(&mut self, op: LightSizeOp)`, `fn PropEditor::delete_selected_lights(&mut self)`: F40, F46 and F47.

**Edit commands.**
- `fn PropEditor::cut(&mut self) -> Option<PropClip>`, `fn PropEditor::copy(&self) -> Option<PropClip>`, `fn PropEditor::paste(&mut self, clip: &PropClip, as_new: bool) -> bool`: F10 and F27-F31.
- `fn PropEditor::can_drop(&self, dragged: &[ElementId], target: ElementId, at: DropPosition) -> bool` and `fn PropEditor::drop(&mut self, dragged: &[ElementId], target: ElementId, at: DropPosition) -> bool`: F34 and F35.
- `fn PropEditor::create_empty_group(&mut self, parent: Option<ElementId>) -> ElementId`, `fn PropEditor::group_selected(&mut self, move_out: bool) -> Option<ElementId>`, `fn PropEditor::rename(&mut self, id: ElementId, name: &str) -> Result<(), String>`, `fn PropEditor::delete_selected_elements(&mut self)`: F23 and F25-F33.

**Background and info panels.**
- `fn PropEditor::assign_background(&mut self, path: &Path) -> Result<(), String>`: F14. Decoding uses the `image` crate.
- `fn background_scale_target(current: (u32, u32), options: &BackgroundScaleOptions) -> Result<(u32, u32), String>`: a pure calculator (F20a). Values come from `BackgroundImageScaleCalculatorTests`.
- `fn PropEditor::scale_background(&mut self, options: &BackgroundScaleOptions) -> Result<(), String>`: resizes the image, then scales lights (F15).
- `fn PropEditor::set_opacity(&mut self, opacity: f64)`: F16.
- `fn PropEditor::set_prop_field(&mut self, field: PropField, value: &str) -> Result<(), String>` and `fn PropEditor::set_element_field(&mut self, id: ElementId, field: ElementField, value: &str) -> Result<(), String>`: the info panels (F18, F18a, F18b, F20c, F20d). Model type goes through the resolver.

**States.**
- `fn PropEditor::state_draft(&mut self) -> &mut PropStateDraft`: the State tab (F50-F58). Share naming, sorting and colour helpers with `dialogs::state` where the rules match.
- `fn PropEditor::state_preview(&self) -> Vec<(LightId, Color)>`: rounded average colours, with the base colour for unassigned lights (F60).
- `fn PropEditor::state_messages(&self) -> Vec<String>`: validation messages (F61).

**Undo and export.**
- `fn PropEditor::undo(&mut self) -> bool`, `fn PropEditor::redo(&mut self) -> bool`, `fn PropEditor::undo_name(&self) -> Option<String>`, `fn PropEditor::redo_name(&self) -> Option<String>`: whole-document snapshot undo with action names. This is new; the original has none (F12).
- `fn prop_wire_points(prop: &Prop) -> Vec<WirePoint>`: light nodes, distinct by id, ordered by `Order`. Each light adds `(x, y, order)` and goes on to the existing `wire_diagram_svg` (F20).

### `vixen-session`: new module `effect_editor::polygon`

**Setup.**
- `fn polygon_canvas_size(container: &PolygonContainer, screen: (f64, f64)) -> (f64, f64)`: canvas size at 70% of the screen (F70).
- `fn PolygonEditor::new(container: PolygonContainer, caps: PolygonEditorCapabilities, canvas: (f64, f64)) -> PolygonEditor`: scales (F71) and the initial mode (F74).

**Drawing and selection.**
- `fn PolygonEditor::set_mode(&mut self, mode: PolygonMode)`: draw polygon, line or ellipse; add point; select. The modes are mutually exclusive (F74).
- `fn PolygonEditor::pointer_down(&mut self, p: Point, m: Modifiers) -> bool`, `fn PolygonEditor::pointer_dragged(&mut self, p: Point, m: Modifiers) -> bool`, `fn PolygonEditor::pointer_up(&mut self, p: Point, m: Modifiers) -> bool`, `fn PolygonEditor::double_click(&mut self, p: Point) -> bool`: F75-F81. Returns whether a redraw is needed.
- `fn PolygonEditor::cursor_at(&self, p: Point) -> PolygonCursor`: arrow, size-all or add-point.
- `fn PolygonEditor::adorner(&self) -> Option<AdornerGeometry>` and `fn PolygonEditor::drag_adorner(&mut self, handle: AdornerHandle, delta: Vector) -> bool`: F82 and F89, with `IsMoveable`, `IsTransformValid` and `IsRotateable`.

**Commands.**
- `fn PolygonEditor::cut(&mut self) -> Option<String>`, `fn PolygonEditor::copy(&self) -> Option<String>`, `fn PolygonEditor::paste(&mut self, clip_json: &str) -> bool`, `delete_selection`: F83 and F84. **Corrected (audit):** the original uses the system clipboard, so a copied shape outlives the editor session; Rust returns and accepts the shape as JSON, and Swift keeps it on a private pasteboard type.
- `fn PolygonEditor::convert_selected(&mut self, to: ShapeKind) -> bool`: F84a.
- `fn PolygonEditor::toggle_start_side(&mut self) -> bool` and `toggle_start_point`: F84b.
- `fn PolygonEditor::set_show_labels(&mut self, on: bool)`: F84c.
- `fn PolygonEditor::scene(&self) -> PolygonScene`: shapes, points, markers, labels and colours (F85-F90).

**Point grid and time bar.**
- `fn PolygonEditor::point_rows(&self) -> Vec<PointRow>` and `fn PolygonEditor::set_point(&mut self, row: usize, x: Option<f64>, y: Option<f64>) -> Result<(), String>`: the grid in display-element coordinates with Y flipped (F91, F92, F92a).
- `fn PolygonEditor::time_bar(&self) -> TimeBar`, `add_snapshot(time: f64)`, `move_snapshot(index: usize, time: f64)`, `delete_snapshot(index: usize)`, `select_snapshot(index: usize)`: F84e, F93 and F93a.

**Finishing.**
- `fn PolygonEditor::finish(self) -> PolygonContainer`: OK. Snaps polygons and lines, drops unfinished shapes, divides by the scale, rounds polygons and lines but not ellipses, and keeps ids (F72, F84d, F73a). The getters are called once.
- `fn PolygonEditor::undo(&mut self) -> bool` and `redo`: optional snapshot undo inside the sheet. This is new.

### `fx-pixel-physics`: extend `morph::polygon`
- Shapes gain `id: Uuid`, `label: String`, `fill_type: PolygonFillType` and, for ellipses, `start_side_rotation: i32`, so the editor round trip keeps them (F73a).
- `struct PolygonContainer { polygons, polygon_times, lines, line_times, ellipses, ellipse_times, width, height, display_element_width, display_element_height, show_display_element }` and `struct PolygonEditorCapabilities` (13 flags).
- `fn capabilities_for(kind: PolygonType) -> PolygonEditorCapabilities`: the F73a table.
- `fn is_off_display_element(points: &[PolygonPoint], w: i32, h: i32) -> bool`, `fn limit_points(points: &mut [PolygonPoint], w: i32, h: i32)`, `fn round_points(points: &mut [PolygonPoint])`: F73. Rounding is banker's rounding, to match `Math.Round`.
- `fn Ellipse::rotate(&mut self, degrees: f64, about: PolygonPoint)` and `fn Ellipse::corners(&self) -> [PolygonPoint; 4]`: rotation is clockwise-positive with y down. The existing `Ellipse::scale` must keep or deliberately fix the integer-division quirk (F73).
- `fn polygon_container(data: &MorphData, buffer: (i32, i32), display: (i32, i32), margin: i32) -> PolygonContainer` and `fn apply_polygon_container(data: &mut MorphData, c: &PolygonContainer)`: Morph's getter and setter, with matching by id, the Wipe-to-Solid rule and sorting by time (F73a).

### `fx-fixture` and `vixen-session::setup::fixture`: profile editors

**Profile (F109-F113a).**
- `fn validate_profile(p: &FixtureProfile, others: &[String]) -> Vec<String>`: the `CanSave` aggregate. Returns all messages; empty means OK is enabled.
- `fn channel_function_choices(p: &FixtureProfile) -> Vec<String>`: the Function column's combo items (F111).
- `fn add_channel(p: &mut FixtureProfile) -> usize`, `fn delete_channels(p: &mut FixtureProfile, rows: &[usize])`, `fn renumber_channels(p: &mut FixtureProfile)`: the channels grid (F111).
- `fn apply_function_edits(p: &mut FixtureProfile, functions: Vec<ProfileFunction>)`: replaces the functions. A channel whose function was deleted reverts to None (F113).

**Functions (F114-F123).**
- `fn FunctionEditor::new(functions: Vec<ProfileFunction>, select: Option<&str>) -> FunctionEditor`: the Edit Functions window.
- `fn FunctionEditor::try_select(&mut self, row: usize) -> Result<(), Vec<String>>`: the selection gate. A row with invalid details cannot be left (F116).
- `fn FunctionEditor::add(&mut self) -> usize`, `delete(&mut self, row: usize) -> Result<(), String>`, `set_field(&mut self, row: usize, field: FunctionField, value: &str) -> Result<(), String>`: list editing (F115).
- `fn FunctionEditor::panel(&self) -> FunctionPanel`: which details panel shows: indexed, colour wheel, pan/tilt, zoom or none (F117).
- `fn FunctionEditor::index_rows`, `set_index_field`, `colour_wheel_rows`, `set_colour_wheel_field`: F118a and F119, with the range and End Value checks fixed or kept.
- `fn FunctionEditor::validate(&self) -> Vec<String>` and `fn FunctionEditor::finish(self) -> Vec<ProfileFunction>`

**Library and gobos.**
- `fn import_gobo_image(images_dir: &Path, source: &Path) -> Result<String, String>`: copies the file into `Fixtures/Images` and returns its file name.
- `fn gobo_image_names(images_dir: &Path) -> Vec<String>`: the names, after a leading `" "` entry.
- `fn library_save(lib: &mut FixtureProfiles, dir: &Path, p: FixtureProfile) -> Result<(), String>`: save, replace by name, sort. A rename leaves the old file, as in the original.
- `fn library_duplicate_message(name: &str, file: &Path) -> String`: the "Invalid Fixture Profile" text, collected as a load warning instead of a modal.

### `vixen-render`: new module `moving_head`
- `struct MovingHeadParams { pan_deg, tilt_deg, mounting, beam_length_pct, max_beam_length, focus, intensity, on, beam_color_left, beam_color_right, beam_transparency, beam_width_model, legend: Option<(String, Color)>, background_alpha }`: F138.
- `enum BeamWidthModel { PreviewMultiplier(f32), EditorTopPlusBottom }`: the two documented formulas (F143, F144).
- `fn moving_head_parts(length: f32, mounting: Mounting) -> MovingHeadParts`: base, supports, horizontal cylinder and housing, from the F139 constants. Cache it per length and mounting.
- `fn moving_head_beam(params: &MovingHeadParams, length: f32) -> BeamMesh`: two beam halves, or the black stub when off (F140, F141).
- `fn moving_head_model_matrices(params: &MovingHeadParams, length: f32, position: (f32, f32)) -> PartMatrices`: pan and tilt transforms in the documented order, adding 180 degrees of tilt for Bottom mounting (F140).
- `fn Renderer::draw_moving_heads(&mut self, instances: &[MovingHeadInstance])`: instanced draws with gray and colour lighting (F142). Draw each beam half once.
- `fn render_moving_head_thumbnail(params: &MovingHeadParams, size: (u32, u32)) -> Vec<u8>`: offscreen rendering for the wizard and the Pan/Tilt panel, so no second 3D engine is needed (F143).

### `vixen-session::dialogs::state`: additions
- `fn state_clone_for_new_property(data: &StateData) -> StateData`: new property, definition and item ids, with the same names, descriptions, colours and elements (F124). `StateModuleCloneTests` pins it.
- `fn custom_prop_states_to_state_data(defs: &[StateDefinitionSpec]) -> StateData` and `fn state_data_to_custom_prop_states(data: &StateData) -> Vec<StateDefinitionSpec>`: `CustomPropStateDataMapper`, both directions, with defaults "State - 1" and "Item Name 1", trimmed names and distinct ids. (audit) Upstream only its unit tests call this class; build it only if the port needs a prop-state round trip of its own, and do not use it for preview placement (F137).
- Check what exists against F124-F136: `StateMapperDefinitionTests`, `StateMapperValidationTests`, `StateMapperPreviewTests` and `StateMapperDraftTests` list the behaviours.

### `vixen-ffi`: exports
- `tools.rs` (empty today): `PropEditor` as a UniFFI object, `PropClip` JSON, xModel candidates, vendor inventory parsing, and `background_scale_target`.
- `effecteditor.rs`: `PolygonEditor` as an object, with `polygon_canvas_size`.
- `setup/fixture.rs` and `dialogs.rs`: `FunctionEditor`, profile validation and gobo import.
- `preview.rs` and the renderer bridge: moving head drawing parameters.

## SwiftUI and AppKit structure

Follow `docs/UI_GUIDE.md`. Each window is a thin SwiftUI or AppKit shell over one Rust object from the Rust API section. Swift holds no rules.

### Custom Prop Editor: `VixenTools`, exported in `ToolsScene`

**Scene and window.**
- `WindowGroup(id: WindowID.customPropEditor, for: UUID.self)`. The id already exists, and each click opens another window (F1).
- Each window owns an `@Observable` `PropEditorModel` wrapping its own Rust `PropEditor`. Nothing is shared between windows, which removes the original's singleton hazard (CPE model rules).
- Title: the prop name. Set `NSWindow.isDocumentEdited` and `representedURL` through a small window accessor, a new VixenKit file. The close button then shows the unsaved dot and the title gets a proxy icon.
- Close (F9): `windowShouldClose` shows the standard "Do you want to save the changes you made to …?" alert with Save, Don't Save and Cancel. The window closes only after the save finishes. This fixes the original, which closes without waiting.
- New, Open and Import xModel ask the same question first when the document has unsaved changes. This is a fix: the original has no dirty check there (F3, F4, F7).

**Menus** (`.commands`, acting on the focused window through `@FocusedValue`).
- **File:**
  - New Prop ⌘N, Open… ⌘O, Save ⌘S, Save As… ⇧⌘S. The original has only Ctrl+N and Ctrl+S.
  - New Prop first asks for the name in a sheet: "Please enter the model name.", titled "Create Model", default "New Prop", with a Create button (F3).
  - Import xModel… with no shortcut.
  - Close ⌘W replaces Exit (F8).
- **Edit:**
  - Undo and Redo, named by `undo_name` (F12).
  - Cut ⌘X, Copy ⌘C, Paste ⌘V, Paste as New with no shortcut, Delete ⌫ (F10, F27-F33).
  - Add Select All ⌘A only if parity notes record it as an addition.
- **View** (F11), keeping the original's items:
  - Assign Background… (F14);
  - Scale Background Image… (F15);
  - Background Opacity ▸ with checkmarked steps from 10% to 100%, in place of the slider menu item (F16).
  - Then, as additions: Zoom In ⌘+, Zoom Out ⌘−, Actual Size ⌘0 for the canvas magnification.
- **Tools** (F13): Vendor Browser… (F21), and Export Wire Diagram ▸ Front Wire Diagram, Back Wire Diagram (F20).
- **Preferences** (F19): these are module settings, not per window, so they go in the app's Settings scene (⌘,) as a "Custom Prop Editor" pane, with Restore Defaults: light colour, selected light colour, state preview base colour and default light size. All open editors redraw when they change.
- **Help:** "Custom Prop Editor Help" opens the original URL with `NSWorkspace`.

**Layout** (F2). An `HSplitView`.
- Left: a `VSplitView`.
  - Top: a segmented `Picker` (Layout, Order, State) over the panel for the chosen tab.
  - Bottom: the info panel. Prop Info has tabs General, Physical, Vendor and Notes as `Form` views with `.formStyle(.grouped)` (F18, F18a, F20c, F20d). Element Info shows for the selected element (F18b).
- Right: the canvas.
- The window toolbar carries, in the original's order:
  - the Draw and Select segmented control (F41);
  - Assign Background (F14);
  - Align Left, Right, Top, Bottom; Distribute Horizontal, Vertical; Flip Horizontal, Vertical (F46);
  - Increase, Decrease and Match Light Size (F47).
  - Leave out Cut, Copy and Paste. Those toolbar buttons are commented out in the original (`CPE/Views/CustomPropEditorWindow.xaml:245-254`), so they live only in the Edit menu and the context menu.
  - Tooltips use `.help` with the original text.
- The status bar is a thin bottom `safeAreaInset`. It shows pointer coordinates at the left, hidden off the canvas, and the status message at the right (F17).

**Layout tab** (F22-F36).
- An `NSOutlineView` through VixenKit's `OutlineView`, with multiple selection.
- Rename inline with Return or double-click (F23). Keyboard behaviour follows F24.
- The context menu keeps the original items (F25-F33).
- Drag and drop shows insertion lines. Swift asks Rust's `can_drop` while hovering and calls `drop` on release (F34-F36).
- Rows come in bulk from `tree_rows()` after each change and are diffed by id path, which keeps expansion and selection.

**Order tab** (F37, F38). An `NSTableView` with Order and Name columns, plus a Reverse button.

**State tab** (F50-F62).
- The definition row: a picker; a "…" menu with Rename and Copy; Add and Delete buttons.
- A Description field.
- An items `Table` with Name, Colour and Count columns. Clicking a header sorts and writes the order back to Rust, which makes it permanent (F57).
- Item buttons (F58) and validation messages (F61) under the table.

**Canvas** (F40-F49, F59, F60).
- A custom `NSView` inside an `NSScrollView` with `allowsMagnification`, so pinch zoom works.
- The background is a `CALayer` holding the decoded image, with the prop's opacity.
- Lights are drawn in one Core Graphics pass over the dirty rect from the cached `light_list()`. The adorner (F45) is drawn by the same view, with an `NSCursor` per handle.
- Mouse events go straight to `pointer_down`, `pointer_dragged` and `pointer_up`. Selection and hit testing are Rust results.
- The canvas never asks the view hierarchy for per-light bounds.

**Sheets on the editor window.**
- Scale Background Image (F20a): a `Form` with a unit picker, fields and an aspect lock.
- Select xModel (F64): a `List` with OK disabled for unsupported rows.
- Wire Diagram Export Options (F20e). The sheet exists today as `app/Sources/VixenSetup/WireDiagramSheet.swift`, but an area may not use another area. Move a copy into a new VixenKit file so `VixenTools` and `VixenSetup` share it. Output goes through `NSSavePanel`, then `NSWorkspace.open`, instead of a temp file (F20).
- Vendor Browser (F21, F21a): a large sheet with an `HSplitView` at the original's 1:2 proportion.
  - Left: the Vendor picker, then the category and product outline, then the vendor's logo, contact details and links.
  - Right: the product image, over the Product Info and Model Options tabs.
  - A model's "Select" button closes the sheet. The editor then downloads with `URLSession` and calls `load_vendor_model`.
  - Add a Cancel button; the original cancels only by closing the window.
  - Fetching the list and the inventories shows progress in the sheet, in place of the status-bar messages (F17). Fetch the inventories concurrently.

**Busy work.** Open, Import, Save, vendor download and background scaling run in a `Task` calling `nonisolated` Rust functions. The window shows a progress indicator and stays responsive, instead of the original's modal busy cursor.

### Polygon Editor: `VixenEffectEditor`
- A sheet from the effect editor's polygon property row (`docs/spec/effect-editor.md` F37). It is sized by `polygon_canvas_size` against the owning window's screen (F70).
- **Toolbar in the sheet**, in the original's order (F74):
  - a segmented control with Draw Line, Draw Polygon, Draw Ellipse, Add Point and Select, showing only the segments the capabilities allow;
  - Paste, Cut, Copy, Delete, Toggle Start Side, Toggle Start Point;
  - Convert to Line, Convert to Polygon, Convert to Ellipse;
  - a Show Labels toggle.
- **Canvas:** an AppKit view drawing `scene()`. Mouse events go to Rust; cursors come from `cursor_at`.
- **Side panel** (F91, F92): a `Table` of points with editable X and Y. It is read-only for ellipses.
- **Time bar** (F93, F93a): an AppKit strip under the canvas, shown only with `ShowTimeBar`.
- **Buttons:** Cancel (Esc) and OK (Return, default), which calls `finish()`. The effect editor records one undo action for the whole change.
- ⌫, ⌘X, ⌘C and ⌘V go to the sheet's editor (F83, F84). In-sheet undo is optional.

### Fixture editors: `VixenDialogs`, reused by `VixenSetup`
- **Fixture Property Editor** (F109-F113a): a sheet from the Intelligent Fixture property's Setup.
  - A `Grid` of profile fields.
  - A channels `Table` with an editable Name, a Function picker and a cog button that opens Edit Functions.
  - "Load Fixture Profile" opens a `List` sheet whose OK is disabled while nothing is selected (F112).
  - Bottom row: the "Learn More About Fixture Profiles" link, a warning symbol whose `.help` lists the errors, then Cancel and OK. OK is disabled while there are errors.
- **Edit Functions** (F114-F123): a sheet with an `HSplitView`.
  - Left: the functions `Table`. The selection binding goes through Rust's `try_select`, so a row with invalid details keeps the selection and shows its errors under the panel (F116).
  - Right: the details panel for the chosen function: an Indexed table, a Color Wheel table with colour wells, Pan/Tilt fields over a live moving head, or Zoom radio buttons.
- **The wizard** in `app/Sources/VixenSetup/FixtureWizard/` embeds the same views without the profile fields (F97, F98, F110). `VixenSetup` may import `VixenDialogs`, so the editors live there once.
- **Moving head view:** a new VixenKit `MovingHeadView`. It wraps `MetalLayerView` and is drawn by `vixen-render`. The Pan/Tilt demo animation advances on a display link, not on a polling thread (F120a).

### State Mapper and name prompt: `VixenDialogs`
`StateMapperSheet.swift` exists. Check it against F124-F136. The definition name prompt (F134) is a small sheet with one text field, and OK is disabled while the name is invalid.

### Parity files
Write these under `docs/parity/`:
- `custom-prop-editor.md`
- `polygon-editor.md`
- `fixture-property-editor.md`
- `edit-functions.md`
- `vendor-browser.md`
- `scale-background-image.md`
- `select-xmodel.md`
- `wire-diagram-options.md`

Add the State Mapper rows to the existing `property-filter-controller-setup.md`.

## Performance hazards

Props are larger than they look: the GE Stardust sample has 1,499 lights, and real props run to thousands. Each item below names the original's cost, then the Mac design that avoids it.

### Custom Prop Editor

1. **One WPF visual per light.** Every light is a `Path` built by `CPE/Converters/LightNodeToPathConverter.cs`, inside an items control on `PropDesigner`. Layout and hit testing cost grows with the light count.
   - Mac: one canvas view draws all lights in a single Core Graphics pass over the dirty rect.
   - Rust answers hit tests with circle maths over a flat array.
2. **Rubber band asks each light for its bounds.** `UpdateSelection` loops over every light and asks the view for that light's bounds on every mouse move (`CPE/Controls/PropDesigner.cs:503-552`). Adorner bounds are computed the same way (`:606-632`).
   - Mac: compute this in Rust from `x`, `y` and `size`, keeping only the current selection.
   - Throttle to display refresh, and send only the selection changes back to Swift.
3. **Every structural change rebuilds all light view models.** `RefreshLightViewModels` clears and rebuilds every light view model from `GetLeafNodes()`, then recolours them all (`CPE/ViewModels/DrawingPanelViewModel.cs:315-338`). It runs on every prop change (`:94`) and after each delete (`:442`).
   - Mac: Rust returns the light list once per change, as one bulk array.
   - Swift keeps it cached and redraws only the invalidated rect.
4. **Transforms run in parallel and notify per light.** `SelectedItems.AsParallel().ForEach(x => x.Center = ...)` (`DrawingPanelViewModel.cs:535-538`) sets view-model properties from worker threads. Each set raises a change notification, which WPF must marshal back. It is not thread-safe either.
   - Mac: one Rust call transforms the selection in place.
   - The canvas redraws once per display frame.
5. **The state preview recolours everything on any change.** `ApplyStatePreview` walks every light for each name, colour, assignment or selection change (`DrawingPanelViewModel.cs:345-368`; F60).
   - Mac: Rust returns the `(LightId, Color)` pairs.
   - Swift redraws once. A diff against the previous pairs is cheap if it is ever needed.
6. **Quadratic naming and ordering.** `Uniquify` scans every model for each candidate name (`CPE/Services/PropModelServices.cs:465-479`). `GetNextOrder` scans every leaf for each new light (`:481-493`). Importing or pasting n elements is O(n²); the 1,499-node sample makes this visible.
   - Rust: keep a name set and a running maximum order in the document. Rebuild them only after a bulk change.
7. **The Layout tree is not virtualized.** `CPE/Views/ElementTree.xaml` sets no virtualization, and a WPF `TreeView` does not virtualize by default. It also has a view model per node through `ElementModelLookUpService`.
   - Mac: use an `NSOutlineView`, which virtualizes.
   - Fetch rows in bulk, and keep the diff by id path, not one FFI call per row per draw.
8. **Saving re-encodes and reads back on the UI path.**
   - The background is re-encoded as JPEG on every save (`CPE/Persistence/WpfPropImageCodec.cs:9-19`).
   - The temp file is then read back fully and validated (`CPE/Persistence/PropPackageWriter.cs:15-42`).
   - Mac: run the save in a `Task` off the main actor. Keep the original JPEG bytes unless the image changed (open question 5, end of Features). Keep the read-back, because it is the safety net for atomic saves.
9. **Vendor inventories download one at a time.** Each vendor is fetched in turn behind a busy indicator (`CPE/ViewModels/PropEditorViewModel.cs:1069-1089`).
   - Mac: fetch them concurrently with `URLSession` and a small limit, and show progress per vendor.
   - Parsing is a Rust call per document.

### Polygon Editor

10. **Every shape is refreshed after each transform.** `RefreshAllShapes` raises point-collection notifications on every polygon, line and ellipse, so every converter re-runs (`POLY/ViewModels/PolygonEditorViewModel.cs:2199-2222`). It is called from transforms and selection changes (`:3996`, `:4046`, `:4759`, `:4885`).
    - Mac: one `scene()` call per frame while dragging, drawn by one view.
11. **Invisible hit-test lines pile up.** Add Point hover uses an invisible 4 px WPF line per polygon segment, added to the canvas (`POLY/Views/PolygonControl.xaml.cs:359-375`). `UpdatePolygonLines` clears its list but never removes the old lines from the canvas, so every rebuild leaves one more hidden line per segment (F90).
    - Mac: segment distance tests in Rust (`cursor_at`). No hidden views.
12. **The window scales once from the primary monitor.** Size and scale are computed once at open (F70), so a canvas on a large display can be big.
    - Mac: drawing costs only the dirty rect, and the scale stays fixed while open, as in the original.

### Fixture editors and graphics

13. **The Pan/Tilt animation runs its own thread.** A background thread changes the angle, then calls `Dispatcher.Invoke` to redraw a `Viewport3D`, then sleeps 10 ms (`FPE/Views/PanTiltView.xaml.cs:83-85`, `:160-176`). That is about 100 synchronous UI hops a second.
    - Mac: advance the angle on a display link and draw through `vixen-render`.
    - Pause when the panel is hidden.
14. **The GDI preview copies the WPF render pixel by pixel.** `DrawFixture` renders to a `RenderTargetBitmap`, then copies every pixel into `BitmapColors` for each fixture, each frame (`FGFX/WPF/MovingHeadWPF.cs:1259-1296`).
    - Mac: one Metal renderer, with no read-back.
15. **The OpenGL renderer re-uploads when dynamic volumes change.**
    - Beam colour, intensity or focus changes mark the dynamic volumes dirty (`FGFX/OpenGL/MovingHeadOpenGL.cs:130-140`, `:363-369`). Vertices are then copied out of every volume (`FGFX/OpenGL/Volumes/VolumeBase.cs:94-102`) and re-uploaded (`FGFX/OpenGL/Shaders/VolumeShader.cs:291-345`). During playback that can happen every frame for every fixture.
    - (audit) It is worse than "can": upstream's dynamic check is always true because its `_previous...` fields are never updated (F140 correction), so the dynamic volumes are revisited on every frame whether or not anything changed. Do not copy the check; compare against the values actually used for the last upload.
    - Both beam halves are drawn twice (`MovingHeadOpenGL.cs:618-619`).
    - Mac: static parts are cached per length and mounting. Pan, tilt and colour are uniforms or instance data. The beam mesh is rebuilt only when its length or radii change, and each half is drawn once.
16. **A shared font disposed by any instance.** `_qFont` is static, and any `Dispose` destroys it for every instance (`MovingHeadOpenGL.cs:57-62`, `:692-708`). That forces reloads and risks crashes.
    - Mac: use a glyph atlas owned by the renderer.
17. **The fixture library loads synchronously and can block with modals.** The first `FixtureSpecificationManager.Instance()` reads every profile on the calling thread. It can stop on a modal dialog for each duplicate name (`FSM/FixtureSpecificationManager.cs:60-119`).
    - Mac: load in the background when a profile is first opened.
    - Collect duplicates as warnings, and show them once in the window that needed the library.
18. **The channels grid turns off row virtualization.** `EnableRowVirtualization="False"` (`FPE/Views/FixturePropertyEditorView.xaml:85`). This is harmless at fixture sizes, so a SwiftUI `Table` is fine.

### State Mapper and wire diagram

19. **Assignment trees of whole displays.** The Assigned Elements tree is virtualized in the original (`STATE/Setup/Views/StateMapperView.xaml:230-231`). Enable and expand state are recomputed per node on each change (F133).
    - Mac: use an `NSOutlineView`.
    - `AssignmentTree::is_enabled` and `expanded_for` must stay linear in the tree size. The guide's target is 50,000 rows.
20. **The state preview sends only differences.** `StatePreviewCoordinator.Refresh` sends only what changed (`STATE/Setup/Preview/StatePreviewCoordinator.cs:27-52`; F129). Keep that in `state_preview.rs`, so a colour edit does not resend every element.
21. **Wire diagram label measuring.** GDI+ `MeasureString` measures each label (`MODEL/WireDiagramSvgBuilder.cs`; F20f), and each label tries 16 candidate positions against every wire.
    - Rust: measure with fixed Arial metrics, cached per label text.
    - Use a spatial grid for the wire-intersection tests when the node count is large. The output must stay the same.

## Tests worth writing

**Rules for every test here:**
- No sound, hardware or network. Vendor tests use canned JSON and XML strings.
- Never open the owner's show folder.
- The sample models in `upstream/Vixen/docs/references` are read-only inputs.
- Tests that write files use a temp directory.

### Behaviours to pin, with expected values

**Prop model (`vixen-core::custom_prop`).**
1. `Prop::new("New Prop {1}", 800, 600, ..)`: the root name equals the prop name; 800x600; opacity 1; one root element with no children; `next_order()` is 1.
2. `next_order`: leaves with orders 3 and 7 give 8. Leaves whose orders are all 0 or negative give 1.
3. `uniquify("Arm", 2, None)`:
   - with "Arm" present, gives "Arm - 2";
   - with "Arm" and "Arm - 2" present, gives "Arm - 3";
   - with "Arm" absent, gives "Arm".
   - With `one_based`, `create_node("Group", ..)` next to an existing "Group" gives "Group - 1".
4. `element_type`: 0 lights is Group ("Group"), 1 is Node ("Single Node"), 2 or more is String ("Multiple Nodes").
5. `is_group_node` is true for an empty leaf.
6. `add_light` on a light node returns the same element, which now has two lights and is a String.
7. `add_light` with no target on a prop whose root is a leaf creates a child group "`<root name>` Group" and puts the light there.
8. Orphan cascade:
   - Removing a shared leaf from one of two parents keeps its lights.
   - Removing it from its last parent clears its lights.
   - Removing a group from its last parent detaches its children recursively.
9. `try_move_within_parent`, worked through the documented algorithm (check against `ElementTreeDragDropTests`):
   - children `[A,B,C,D]`, move `[A,C]` to index 4: `[B,D,A,C]`;
   - move `[C]` to index 0: `[C,A,B,D]`;
   - an index past `Children.Count`, a duplicate, or a non-child: returns false and nothing changes.
10. `try_set_model_type(x, Model)` demotes any other Model element to None. It refuses a leaf while the prop has other elements.
11. `remove_lights` that empties an element removes that element from every parent. `refresh_state_assignments` then drops its id from every state item and reports a change.
12. State preview: a light assigned to red (255,0,0) and blue (0,0,255) items shows (128,0,128). The average rounds per channel; check the rounding rule against the code. Unassigned lights show the base colour (25,25,25) (F60).
13. Align Left on lights at x = 10, 50, 30, selected in that order, sets every x to 10. Align Right does the same in the original (F46 quirk). If the port fixes Align Right, it gives 50, and the change goes in `docs/DIFFERENCES.md`.

**`.prp` files (`vixen-io::custom_prop_file`).**
14. Write then read keeps a shared leaf as one element listed in both parents' `childIds`, with lights, ids and state assignments unchanged. A non-leaf's `order` serializes as -1 (`PropDocumentMapperTests`).
15. Packages that must be rejected with "The prop package is invalid.":
   - a ZIP with a third entry;
   - an entry with a directory separator;
   - a `background.jpg` not starting `FF D8 FF`;
   - an entry whose length / compressed length is over 100;
   - a `prop.json` over 4 MiB;
   - a non-finite coordinate;
   - a cycle;
   - a state item naming an unknown element.
16. A failure before publish leaves an existing destination byte-identical (`PropPackageWriterTests`).
17. `sniff_prop_file`:
   - bytes starting `50 4B 03 04` are a ZIP, even if bytes 25-52 also hold the LiteDB text;
   - `"** This is a LiteDB file **"` at offset 25 with byte 52 = 7 is a legacy v4 file;
   - anything else is unknown, giving the load error.
18. Saving a legacy file to its own path leaves `<path>.legacy-litedb.bak` holding the original bytes, and the new file is a valid package.
19. `CustomPropEditorData` with a default light size of 0 loads as 3 (F19). A missing or `Color.Empty` colour loads as its default; `OnDeserialized` handles both (`CPE/CustomPropEditorData.cs:41-62`).
    - The owner's `SystemData/ModuleStore.xml` has a real block to test against: `DefaultLightSize` 3, `LightColor` = knownColor 164 (White) with state 1, `SelectedLightColor` = knownColor 82 (HotPink) with state 1, and **no** `StatePreviewBaseColor`.
    - Expected load: White, HotPink, RGB(25,25,25), 3.
    - Copy that XML block into a test fixture. Tests must never read the show folder.

**xModel import (`vixen-io::xmodel`).** The sample values below come from a scratch re-implementation of F65, F66 and F68. The upstream code was not run. Confirm one sample against a C# run, or against the upstream tests, before treating them as golden. Node coordinates are listed; each light sits at node + 7 on both axes.
20. `GE Stardust 1500.xmodel`:
   - A `<models>` wrapper holding one `<model>`, imported without the selection sheet.
   - Custom, `PixelSize` 3, `CustomWidth`/`CustomHeight` 150, `ScaleX` 1.488960, `ScaleY` 1.460752.
   - 1,499 compressed entries, orders 1-1499, all distinct. The first entry is order 1390 at node (101,1). Order 1 is at node (111,109), so its light is at (118,116). Max node X 223, max Y 219.
   - Result: prop "GE Stardust 1500 {1}", 800x600 (the minimum wins), `NodeCount` "1499", light size 3.
   - Children: 23 `subModel` groups (All_Pixels, Half_Left, Half_Right, Half_Top, Half_Bottom, Quad_TopLeft, Quad_TopRight, Quad_BotLeft, Quad_BotRight, Rings, Arms, Outer Balls, Inner Arrow, Outer Arrow, ZigZag, Inner Balls, Flake 1, Zig Hook, LilMan, Petals, Diamonds, SpankIt, VictoryFormation).
   - The 11 `modelGroup`, the `Aliases` and the `dimensions` children are ignored. No faces, no states.
21. `circlemodel/Circle-3-ring-*.xmodel` and `circle-1-ring-outer-cw.xmodel`:
   - Common to all: wrapper with one model "Circle", `PixelSize` 2, `ScaleX` 5.416101, `ScaleY` 4.656205, prop "Circle {1}", 800x600.
   - For the 3-ring files, `LayerSizes` 30,40,50, `centerPercent` 40, 120 nodes, max node (270,232).
   - **bottom-inner** (InsideOut 1, B, R): rings by layer 0 = 1-30, 1 = 31-70, 2 = 71-120. Node 1 at (135,163), node 120 at (118,231).
   - **bottom-outer** and **outer-ccw** (InsideOut 0): layer 2 = 1-50, 1 = 51-90, 0 = 91-120. Node 1 at (135,232), node 120 at (124,162).
   - **top-inner**: node 1 at (135,69), node 120 at (152,1).
   - **top-outer**: node 1 at (135,0), node 120 at (146,70).
   - **1-ring-outer-cw** (`LayerSizes` 50, Dir L, `centerPercent` 0): node 1 at (135,232), node 50 at (152,231).
   - Generated groups: "Circle {1} - Circles", with "Circle {1} - Circle n" numbered in ring order.
22. `circlemodel/circle-vendor.xmodel`:
   - A bare `<circlemodel>` "EFL 2ft Spinner-1", `LayerSizes` 20,20,40,40,40,40, `parm3` 25, `PixelSize` 4. No `ScaleX`/`ScaleY`, so the default scale is 4.
   - Rings: layer 5 = 1-40, 4 = 41-80, 3 = 81-120, 2 = 121-160, 1 = 161-180, 0 = 181-200. Node 1 at (80,160), node 200 at (86,99), max (160,160).
   - Its `Circles` submodel lines expand to exactly those six rings: line0 = 181-200 with blanks, line1 = 161-180, then 121-160, 81-120, 41-80, 1-40. So **no generated Circles group** is added, and the imported submodel is used.
   - 9 children: 8 `subModel` and one `dimensions`.
23. Default scale: `GetDefaultScale(99, 99)` is 4; `(150, 150)` is 2; `(250, 10)` is 1, since both axes must be under the limit.
24. `Apply(2.5, 1.0)` is 3 and `Apply(-2.5, 1.0)` is -3: away from zero.
25. Assembly size: nodes up to (900, 100) with `PixelSize` 2 give 909x600 (F68).
26. Error texts are exact:
   - "The xModel file does not contain a model definition."
   - `Unsupported model type: matrixmodel. \nImport supports custom and circle model types at this time.`
   - "No models were found in the xModel file."
   - The circle messages of F66, for example `Circle model 'X' has an invalid LayerSizes value.`
27. Candidates (F64): blank names give `Unnamed model 1`, `Unnamed model 2`. **Corrected (audit):** two models named "Star" give `Star (1)` and `Star (2)`; every duplicate is suffixed from 1 and a unique name is left alone (`XModelImport.cs:120-143`). Three models "A", "B", "A" give `A (1)`, `B`, `A (2)`.
28. Ranges: `"1-3, 5,,7"` gives 1,2,3,5,7. `"a-3"` is an error. A blank `line1` ends the list even if `line2` exists (F67).
29. `stateInfo`: `s1=1-4 s1-Name=" Open " s1-Color=#FF0000` gives the item "Open" (trimmed), red, nodes 1-4. An `s2` with only a colour is dropped. A blank `Name` becomes "State Name 1".
30. `faceInfo`: `FaceOutline2` maps to `Outlines`. That is the upstream quirk; decide whether to keep it.

**Background scale (`background_scale_target`)** (`BackgroundImageScaleCalculatorTests`).
31. Pixel inputs 640.4, 640.5 and 640.6 give 640, 641 and 641.
32. 25% of 4032x3024 is 1008x756.
33. NaN, +Infinity, 0, -1 and 100000.5 are rejected.
34. A locked 4:3 at width 640 gives height 480. A locked width from height 1 at 3:2 gives 2.
35. Coordinate scaling (`CoordinateScalingTests`): two lights at (100,50) and (300,150), scaled 800x600 to 400x300, end at (50,25) and (150,75), with sizes unchanged. A light shown in two groups moves once. Out-of-bounds lights are not clamped.

**Polygon editor (`vixen-session::effect_editor::polygon`)**. No upstream tests exist.
36. Mapping: container 100x50 on a 991x491 canvas gives `xScale` = 990/99 = 10 and `yScale` = 490/49 = 10. A container 1 wide gives `xScale = CanvasWidth - 1` (F71).
37. Grid: with height 50 and scale 10, the canvas point (0,490) shows as X 1, Y 1. The point (0,0) shows as X 1, Y 50. Entering Y 1 maps back to canvas y 490 (F92a).
38. `finish()` divides by the scale and rounds polygons and lines with banker's rounding: 2.5 becomes 2 and 3.5 becomes 4. Ellipse values are not rounded (F72).
39. `finish()` keeps the `ID` and `Label` of a moved polygon. It drops an unclosed last polygon, and a last line with fewer than 2 points (F84d).
40. Morph setter: a returned 5-point polygon with fill Wipe becomes Solid. A shape whose `ID` is unknown becomes a new morph shape. A missing one is removed. TimeBased results are sorted by time (F73a).
41. `capabilities_for(Pattern)`: `AddPolygons` false, `AddPoint` true, `ShowTimeBar` false, `AllowMultipleShapes` false. `TimeBased`: `ShowTimeBar` true, `DefaultToSelect` false (F73a table).
42. `IsMoveable`: moving a point at x = width-1 by +1 is refused. `IsRotateable` refusing a rotation removes the adorner and clears the selection (F82).
43. Clicking within 10 px of the first point closes the polygon. With 3 or 4 points the start side is Lime; with 5 points and Wipe, the fill becomes Solid (F85).

**Fixtures (`fx-fixture`, `vixen-session::setup::fixture`).**
44. `GetFileName("Moving <Head>: 1/2")` gives `Moving (Head(_ 1_2.xml`. `>` becomes `(`; that quirk is kept.
45. `InitializeBuiltInFunctions` creates the built-in list in the order given in Data and persistence, starting Pan, Tilt, Color Wheel, Dimmer, Color, Zoom. Pan and Tilt have rotation limits 0 and 360.
46. Reading `<TimelineColor1Html></TimelineColor1Html>` gives White. `Color1Html` "#FF8000" round-trips as written. Named colours are written by name.
47. `FixtureColorWheel`: setting `UseCurve` true forces `IndexType = ColorWheel`.
48. Library load with two files of the same `Name` keeps the first and returns one duplicate warning with the "Invalid Fixture Profile" text. A malformed file is skipped without a warning. The result is sorted by name.
49. A specification embedded in property data whose name is not in the library is added to it (`IntelligentFixtureModule.ModuleData` setter).
50. Deleting a function that a channel uses sets that channel's function to None (F113). The function selection gate refuses to leave a row with invalid details (F116).
51. Colour wheel row with Use Curve on: an End Value that is not a number, or is outside the DMX range, is rejected under the End Value label. The original validates Start Value twice and then crashes in `int.Parse` on save (F119). Test the fixed rule, and record the fix.

**Moving head (`vixen-render::moving_head`).**
52. Length 1: base width and depth 0.45, base height 0.07, housing radius 0.18, support X offset 0.252, housing length 0.6, beam bottom radius 0.1 (F139).
53. The beam length is `maxBeamLength * BeamLength / 100`. Bottom mounting adds 180 degrees to tilt. `GetOrientationSign` is -1 for Top and +1 otherwise (F138, F140). (audit) Upstream applies +180 only in OpenGL and tilt as given in WPF (F143 notes). **Owner decision, November 11, 2026:** test +180 consistently for all port views; the reusable builder still accepts an explicit offset.
54. Settings equality ignores `EnableGDI` and `OptimizeOpenGLVertexData` (F138).
55. Beam off, or a transparent left colour, gives the black stub with length 0.5 and equal radii (F140). (audit) That is the OpenGL update path only: the first build ignores `OnOff`, and the WPF model shows no beam at all when off (F140 and F143 notes).
56. `BeamWidthModel::PreviewMultiplier(m)` gives top radius `m*0.1*Focus/100`. `EditorTopPlusBottom` gives `0.1 + 0.8*Focus/100` (F143).

**State (`dialogs::state`).**
57. `state_clone_for_new_property` gives new property, definition and item ids, with the same names, descriptions, colours and element ids (`StateDataCloneTests`, `StateModuleCloneTests`).
58. Mapper defaults: a blank definition name becomes "State - 1", a blank item name "Item Name 1", empty ids get new GUIDs, and element ids are distinct (`CustomPropStateDataMapper`; (audit) the same defaults apply in `PreviewCustomPropBuilder.cs:256-290`, which is the path that runs in production and should be pinned first: also pin that an item whose element ids all fail to translate is dropped, and a definition left with no items is dropped).
59. Preview diffing: removing one of two colours on an element turns that element off once, then turns the remaining colour back on. Turn-offs are sent before turn-ons (`StatePreviewCoordinatorTests`).

**Wire diagram from a prop.**
60. `prop_wire_points`: an element with order 2 and two lights adds two points labelled 2. Elements sort by `Order`. Shared elements are counted once (F20).

### Upstream tests in this area

These are xUnit tests. Each `[Theory]` counts once.

| File (under `upstream/Vixen/src/Vixen.Tests/`) | Lines | Tests | Pins |
|---|---|---|---|
| `App/CustomPropEditor/BackgroundImageScaling/BackgroundImageScaleCalculatorTests.cs` | 62 | 5 | pixel rounding away from zero, percent conversion, invalid inputs, locked aspect (values below) |
| `App/CustomPropEditor/BackgroundImageScaling/BackgroundImageScaleViewModelTests.cs` | 153 | 10 | F20a dialog: defaults, unit switch, aspect lock, cancel |
| `App/CustomPropEditor/BackgroundImageScaling/CoordinateScalingTests.cs` | 67 | 3 | F15: each unique light scaled once, size unchanged, no clamping, scaling can be off |
| `App/CustomPropEditor/BackgroundImageScaling/PropImageDimensionPersistenceTests.cs` | 118 | 4 | image assignment keeps valid logical size; `SetImage` resets to native pixels; save and load keeps scaled size |
| `App/CustomPropEditor/ElementTreeDragDropTests.cs` | 381 | 14 | `TryMoveWithinParent` and tree drop rules (F34, F35) |
| `App/CustomPropEditor/Import/XLights/Custom/CustomXModelNodeParserTests.cs` | 142 | 7 | custom grid and compressed forms decode alike, with and without scale; resolver prefers compressed |
| `App/CustomPropEditor/Import/XLights/States/XLightsStateNameNormalizerTests.cs` | 67 | 4 | state and item name fallbacks |
| `App/CustomPropEditor/Import/XLights/XModelCircleImportTests.cs` | 580 | 21 | circle import: wrapping, selection, ring groups, start side, direction, scale, centre percent, Circles submodel, pixel count |
| `App/CustomPropEditor/Import/XLights/XModelImportHierarchyTests.cs` | 382 | 13 | custom import hierarchy, states, minimum size, dimensions and scale, wrapped models, model groups ignored |
| `App/CustomPropEditor/Persistence/PropDocumentMapperTests.cs` | 102 | 4 | `.prp` JSON round trip with shared elements, deep graphs, validation of cycles and states |
| `App/CustomPropEditor/Persistence/PropPackageWriterTests.cs` | 79 | 3 | two-entry package, unexpected entries rejected, fault before publish keeps the old file |
| `App/CustomPropEditor/PropDesignerSelectionTests.cs` | 223 | 3 | canvas click selection and colour restore (UI-level; port as editor-session tests) |
| `App/CustomPropEditor/State/CustomPropStateDataFoundationTests.cs` | 203 | 7 | state mapper, model resolver, legacy migration |
| `App/CustomPropEditor/State/CustomPropStateEditorViewModelTests.cs` | 1025 | 29 | State tab (F50-F62), preview colours, assignment clean-up |
| `App/Modeling/WireDiagramExportOptionsTests.cs` | 100 | 9 | options defaults and bounds |
| `App/Modeling/WireDiagramExportOptionsViewModelTests.cs` | 180 | 14 | F20e dialog mapping |
| `App/Modeling/WireDiagramSvgBuilderTests.cs` | 608 | 23 | F20f SVG structure, markers, labels, flip, scale, orientation |
| `Property/State/BroadcastStatePreviewPublisherTests.cs` | 61 | 1 | preview publisher message types |
| `Property/State/StateAssignmentTreeSelectionTests.cs` | 279 | 11 | assignment tree selection, ranges, toggling |
| `Property/State/StateAssignmentTreeTests.cs` | 95 | 3 | group check disables and clears descendants |
| `Property/State/StateDataCloneTests.cs` | 187 | 5 | defaults, deep clone, `CloneForNewProperty` new ids, normalisation |
| `Property/State/StateDefinitionNameDialogViewModelTests.cs` | 62 | 3 | F134 name prompt rules |
| `Property/State/StateItemCopyNamingTests.cs` | 218 | 12 | next item name suffixes |
| `Property/State/StateMapperDefinitionTests.cs` | 1143 | 32 | F127-F132 definitions and items |
| `Property/State/StateMapperDraftTests.cs` | 105 | 2 | Cancel leaves data alone, OK applies |
| `Property/State/StateMapperPreviewTests.cs` | 385 | 12 | F129 preview behaviour |
| `Property/State/StateMapperTestCollection.cs` | 9 | 0 | test collection only |
| `Property/State/StateMapperValidationTests.cs` | 436 | 16 | F135 validation, assignment tree selection |
| `Property/State/StateModuleCloneTests.cs` | 73 | 1 | `CloneValues` copies items and assignments |
| `Property/State/StatePreviewCoordinatorTests.cs` | 253 | 12 | preview diffing (turn off before turn on, de-duplication) |
| `Property/State/StateSetupHelperTests.cs` | 169 | 5 | F125 setup helper attach and cancel |
| **Total: 31 files** | 7947 | 288 | |

**Related tests outside the area:**
- `Vixen.Tests/Effect/State/StateRenderPlannerTests.cs` and `StateRenderSegmentCoalescerTests.cs`: the State effect consumes `StateData`.
- `Vixen.Tests/Preview/VixenPreview/PreviewCustomPropStateImportTests.cs` (with its `...TestCollection.cs`): prop states carried into a preview.
- `Vixen.Tests/Preview/VixenPreview/PreviewMovingHeadSelectionTests.cs`: the moving head shape in the preview.

**No upstream tests exist for:** PolygonEditor, App/Polygon, FixtureWizard, FixturePropertyEditor, FixtureGraphics, App/Fixture, FixtureSpecificationManager, IntelligentFixture, or `IntelligentFixtureTemplate.cs`. Items 36-56 above cover them first. The element creation rules of F104-F107 belong in `create_fixture_elements` tests in `vixen-session`.

## Windows-only omissions

These are Windows or WPF mechanics that are not ported. The behaviour they serve is kept in the Mac way shown.

### Windowing and interop
- **WinForms hosting of WPF windows.** These calls let WPF windows live inside the WinForms main app; sheets and owned windows replace them:
  - `ElementHost.EnableModelessKeyboardInterop(...)` (`CPE/CustomPropEditorModule.cs:68`, `MODEL/Views/WireDiagramExportOptionsWindow.xaml.cs:31`, `STATE/StateModule.cs:103`, `STATE/Setup/Services/StateMapperDialogService.cs:25`);
  - `new WindowInteropHelper(w).Owner = Form.ActiveForm.Handle` (`MODEL/Views/WireDiagramExportOptionsWindow.xaml.cs:57`, `STATE/StateModule.cs:100`, `STATE/Setup/Services/StateMapperDialogService.cs:22`).
- **`ShowInTaskbar`, `WindowStyle="ToolWindow"`, `ResizeMode`.** Found in `WIZ/Wizard/Models/IntelligentFixtureWizard.cs:55`, `:86-89`, `TPL:918`, `CPE/Views/XModelSelectionView.xaml:11-12`, `CPE/Views/BackgroundImageScaleWindow.xaml:12-13`, `CPE/Views/ConfigurationWindow.xaml:7` and `MODEL/Views/WireDiagramExportOptionsWindow.xaml:9-10`. On the Mac, dialogs are sheets and the editor is a normal window. Fixed sizes become minimum sizes where the original allowed resizing.
- **Primary-monitor sizing.** `SystemParameters.PrimaryScreenWidth`/`Height` size the Polygon Editor (`POLY/Views/PolygonEditorView.xaml.cs:62-63`, `:142-162`). Use the owning window's `NSScreen` with the same percentages.
- **Shortcuts.** Alt+F4 Exit (F8) becomes ⌘W. Ctrl+N and Ctrl+S become ⌘N and ⌘S, and ⌘O and ⇧⌘S are added. Ctrl-click and Shift-click selection become ⌘-click and ⇧-click.
- **Catel and Orc theming.** `ThemeManager.Current.SynchronizeTheme()` (`TPL:1221-1222`), Catel window styles, and Orc's wizard navigation controller and side bar (F94). Use the system appearance through VixenKit `Theme`, and the existing SwiftUI wizard in `app/Sources/VixenSetup/FixtureWizard/`.

### Controls with no Mac equivalent needed
- **`SliderMenuItem`** (`CPE/Controls/SliderMenuItem.xaml(.cs)`, 447 lines), a slider inside a menu item for Background Opacity (F16). Use a submenu of checkmarked opacity steps, or a slider in the Prop Info panel.
- **Grid workarounds.**
  - The "Zombie" and "Delete Me" sentinel names exist only to calm WPF validation (`FM/FixturePropertyEditorViewModel.cs:387`, `FM/FunctionTypeViewModel.cs:1032`, `FM/IndexedViewModel.cs:274`, `FM/ColorWheelViewModel.cs:161`; F123).
  - `VixenDataGrid`'s editing hack and `DataGridView` focus-into-edit behaviour (`FV/VixenDataGrid.cs`, `FV/DataGridView.cs`; F122).
  - `DataGridSortedItemsCommandBehavior` (F57).
  - Use SwiftUI `Table` or `NSTableView` with sort descriptors.
- **`BindingProxy`, `MathConverter` and the value converters** (`POLY/Views/BindingProxy.cs`, `POLY/Converters/*`, `CPE/Converters/*`). They are XAML binding plumbing. The offsets they compute become constants in the canvas drawing code:
  - point markers at `x-3`;
  - labels at `x+5`;
  - centre cross arms of ±20.
- **The WPF property grid** for prop and element info (F18, F18b). Use SwiftUI `Form` views.
- **GongSolutions drag and drop** (F34-F36). Use `NSOutlineView` drag and drop. Rust decides what is allowed.
- **Busy-wait colour picking.** `StateColorPickerService` loops while `Mouse.LeftButton == Pressed` (`STATE/Setup/Services/StateColorPickerService.cs:22`). Use `NSColorPanel` or `ColorPicker` continuous updates.

### Graphics
- **OpenTK, OpenGL and the GLSL shaders** (`FGFX/OpenGL/**`), and **WPF `Viewport3D`** (`FGFX/WPF/MovingHeadWPF.cs`). Use one Metal renderer in `vixen-render` (WGSL), with an offscreen path for the wizard (Rust API).
- **The GDI fallback.** `EnableGDI`, `RenderTargetBitmap` with a per-pixel copy, and `BitmapSourceHelper` (`FGFX/MovingHeadSettings.cs:98`, `FGFX/WPF/MovingHeadWPF.cs:1259-1296`, `FGFX/WPF/BitmapSourceHelper.cs`). Not needed.
- **QuickFont with `cour.ttf` from the Windows Fonts folder** (`FGFX/OpenGL/MovingHeadOpenGL.cs:29`). Use a glyph atlas from a system monospaced font, such as Menlo or SF Mono, or bundle Courier Prime. Record which.
- **`.cur` cursor resources** (`POLY/Cursors/cursor_arrow_dragbox.cur`). Use `NSCursor` from an image.
- **GDI+ `MeasureString`** in the wire diagram (F20f). Use Core Text or fixed Arial metrics.

### Files, processes and system values
- **`File.Replace` with a backup** (`CPE/Persistence/AtomicPropFileWriter.cs:28`). Use `rename(2)` within the same folder for the atomic publish. Copy the legacy backup first. The temp file stays beside the target, so the rename is on the same volume.
- **`Process.Start` with `UseShellExecute`** for help links, the wire diagram SVG and wizard help (`CPE/ViewModels/PropEditorViewModel.cs:1391`, `MODEL/ElementModeling.cs:85`, `WIZ/Wizard/Models/IntelligentFixtureWizard.cs:142`, `:150`, `FV/FunctionTypeWindowView.xaml.cs:91`, `FV/FixturePropertyEditorWindowView.xaml.cs:70`). Use `NSWorkspace.shared.open`.
- **Backslash paths.** `<DataRoot>\Fixtures\`, `\Images\` and temp files (FSM; Data and persistence). Use `PathBuf` joins.
  - **Keep** the backslash inside LiteDB file ids (`fileId + "\\"`, `CPE/Persistence/Legacy/LiteDb5021/LegacyLiteDbRawReaderPrototype.cs:74`). That is stored data, not a path.
  - **Keep** `GetFileName` turning `\` into `_` (`FIX/FixtureSpecification.cs:112`), so file names match files made on Windows.
- **`Environment.SpecialFolder.MyPictures.ToString()`.** Assign Background passes the literal string "MyPictures" as its start folder (`CPE/ViewModels/PropEditorViewModel.cs:1306`), a bug that falls back to the system default. Start the open panel at `~/Pictures`.
- **`Environment.UserName`** for `CreatedBy` (`CPE/Model/Prop.cs:38`, `FIX/FixtureSpecification.cs:29`). Use `NSUserName()`, the short login name, which matches what Windows stores.
- **`KnownColor` enumeration** for the random timeline colour of new functions (`FM/FunctionTypeViewModel.cs:39-40`). Put the list into Rust as a constant table, or pick a fixed palette and record it.
- **Windows message boxes and Catel `IMessageBoxService`.** Use alerts and sheets with verb buttons ("Delete", "Save", "Don't Save"), per `docs/UI_GUIDE.md`.

### Proposed `docs/DIFFERENCES.md` rows
Decide each one while implementing. Add a row only for a fix; a kept quirk is marked "quirk" in the code and parity table instead.

| Where | Upstream | Proposed port behaviour | Why |
|---|---|---|---|
| Custom Prop Editor windows | One process-wide prop: a second window's commands act on the newest prop | One document per window | Multiple windows are a feature of F1 that only works this way |
| Custom Prop Editor undo (F12) | None | Snapshot undo with named actions | Mac `UndoManager` convention |
| New, Open and Import with unsaved changes (F3, F4, F7, F21) | Replaces the prop without asking | Save, Don't Save, Cancel alert | Data loss |
| Close prompt (F9) | Save is not awaited; the window closes anyway | Closes after the save succeeds | Data loss on a slow or failed save |
| Background JPEG on save (`.prp`) | Re-encoded on every save, so quality drops each time | Keep the stored JPEG bytes unless the image changed | Repeated saves degrade the image |
| Align Right and Align Bottom (F46) | Same as Align Left and Align Top | Align to the rightmost or bottom-most selected light | Upstream bug. Keep it only if the owner prefers parity |
| Preferences "Selected Color" swatch (F19) | Bound to the state preview colour command | Edits the selected light colour | Upstream binding bug |
| Polygon Convert to Line (F84a) | Writes through `Lines[0]` | Edits the new line | Upstream bug with multiple lines |
| Polygon closing with 1 or 2 points (F85) | Allowed; gives a 1-point "closed" polygon | Close only at 3 or more points | Degenerate shape |
| Colour wheel End Value with Use Curve (F119) | Validates Start twice, then crashes on save | Validates End Value | Crash |
| `Ellipse.Scale` y factor (F73) | Integer division collapses y | Floating-point factor, or keep | Upstream bug. Changing it may move existing Morph ellipses, so check golden renders first |
| xModel `FaceOutline2` (F67) | Maps to `Outlines` | Maps to `Outlines2` | Upstream bug. Changes imported face data |
| Moving head beam drawn twice (F142) | Each half drawn under both colour shaders | Drawn once | Wasted work. Check that blended colour looks the same |
| Vendor model temp files (F21) | Left in the temp folder | Deleted after loading | Housekeeping |
| Wire diagram output (F20) | Temp `.svg`, plus a leftover `.tmp`, opened in the default app | Save panel, then open | No temp litter; the user keeps the file |
| Fixture library duplicates (FSM) | One modal per duplicate at first use | One warning list in the window that needed the library | No modal storm during load |

