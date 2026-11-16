# Element property, output filter and controller setup dialogs

## Scope

Upstream paths are relative to `upstream/Vixen/src/Vixen.Modules/`, except paths starting `Vixen.`, which are relative to `upstream/Vixen/src/`. A section's letter aliases are defined at its top. Inside a section a bare file name or short path (`ControllerTree.cs`, `SetupElementsTree.cs`, `UniverseEntry.cs`, `J1Sys/J1MsgBox.cs`, `Controls/IPTextBox.cs`) means the file of that name in the folder that section is about, or under `Vixen.Application/Setup/` and `Vixen.Common/Controls/` for the entry-point sections. A bare `:N` means the file last named before it in the same entry. In an entry that names none, it means the section's main form, and sections say which file that is where it could be read two ways. `Designer:N` means the `.Designer.cs` of the form in question, and doesn't change what a bare `:N` means. Line counts are from `wc -l`. Every file in this first list was read in full.

**Controller/GenericSerial** (focus): CommandHandler.cs 19, Data.cs 50, DataPolicy.cs 19, DataPolicyFactory.cs 11, Descriptor.cs 43, Module.cs 228, SetupDialog.cs 94, SetupDialog.Designer.cs 203.

**Controller/DummyLighting** (focus): CommandHandler.cs 27, DummyLighting.cs 116, DummyLightingData.cs 28, DummyLightingDescriptor.cs 44, DummyLightingOutputForm.cs 191, DummyLightingOutputForm.Designer.cs 51, DummyLightingSetup.cs 54, DummyLightingSetup.Designer.cs 150, MonochromeDataPolicy.cs 19, MonochromeDataPolicyFactory.cs 12, OneChannelColorDataPolicy.cs 19, OneChannelColorDataPolicyFactory.cs 12, ThreeChannelColorDataPolicy.cs 19, ThreeChannelColorDataPolicyFactory.cs 12.

**Controller/E131**: AboutBox.cs 126, AboutBox.Designer.cs 198, Controls/DataGridViewNumbered.cs 54, Controls/IPTextBox.cs 18, DataPolicy.cs 19, DataPolicyFactory.cs 12, E131.cs 70, E131DeviceManagementProtocol.cs 114, E131Framing.cs 112, E131ModuleDataModel.cs 77, E131ModuleDescriptor.cs 43, E131OutputPlugin.cs 937, E131Packet.cs 122, E131Root.cs 117, Extensions.cs 85, Help.cs 117, J1Sys/J1MsgBox.cs 533, J1Sys/MessageBeepClass.cs 19, SetupForm.cs 1250, SetupForm.Designer.cs 510, UnicastForm.cs 73, UnicastForm.Designer.cs 131, UniverseEntry.cs 239.

**Controller/DDP**: DDP Reference Information.txt 112, DDP.cs 300, DDPData.cs 39, DDPDescriptor.cs 44, DDPSetup.cs 138, DDPSetup.Designer.cs 169, DataPolicy.cs 19, DataPolicyFactory.cs 11.

**Controller/Renard**: CommandHandler.cs 19, Data.cs 47, Descriptor.cs 43, IRenardProtocolFormatter.cs 9, Module.cs 206, Protocol1.cs 98, Protocol2.cs 90, ProtocolFormatterService.cs 39, ProtocolVersionAttribute.cs 12, RenardDataPolicy.cs 19, RenardDataPolicyFactory.cs 11, SetupDialog.cs 56, SetupDialog.Designer.cs 143.

**Controller/DMXUsbPro**: Data.cs 37, DataPolicy.cs 19, DataPolicyFactory.cs 11, Descriptor.cs 44, DmxUsbProSender.cs 94, Message.cs 44, MessageType.cs 19, Module.cs 121.

**Controller/OpenDMX**: DataPolicy.cs 19, DataPolicyFactory.cs 11, Device.cs 36, OpenDMX.cs 252, OpenDMXData.cs 18, OpenDMXDescriptor.cs 38, OpenDMXInstance.cs 87, SetupDialog.cs 56, SetupDialog.Designer.cs 103.

**Controller/ElexolEtherIO**: ElexolEtherIOCommandHandler.cs 21, ElexolEtherIOData.cs 30, ElexolEtherIODataPolicy.cs 19, ElexolEtherIODataPolicyFactory.cs 11, ElexolEtherIODescriptor.cs 62, ElexolEtherIOModule.cs 195, SetupDialog.cs 185, SetupDialog.Designer.cs 212.

**Controller/RDSController**: CommandHandler.cs 21, Data.cs 57, DataPolicy.cs 18, DataPolicyFactory.cs 9, Descriptor.cs 43, Module.cs 171, RdsSerialPort.cs 105, SetupForm.cs 184, SetupForm.Designer.cs 435, TData.cs 72.

**Controller/LauncherController**: CommandHandler.cs 21, Data.cs 16, DataPolicy.cs 18, DataPolicyFactory.cs 10, Descriptor.cs 43, Module.cs 105, SetupForm.cs 23, SetupForm.Designer.cs 74.

**Controller/DebugController**: DebugController.cs 130, DebugControllerOutputForm.cs 98, DebugControllerOutputForm.Designer.cs 76.

**OutputFilter/ColorBreakdown**: ColorBreakdownData.cs 34, ColorBreakdownDescriptor.cs 51, ColorBreakdownFilter.cs 76, ColorBreakdownItem.cs 26, ColorBreakdownItemControl.cs 63, ColorBreakdownItemControl.Designer.cs 94, ColorBreakdownMixingFilterBase.cs 59, ColorBreakdownModule.cs 129, ColorBreakdownSetup.cs 149, ColorBreakdownSetup.Designer.cs 215, IBreakdownFilter.cs 9, Outputs/ColorBreakdownOutputBase.cs 106, Outputs/_16BitColorBreakDownOutput.cs 39, Outputs/_8BitColorBreakdownOutput.cs 39, RGBColorBreakdownMixingFilter.cs 46, RGBToRGBWConverter.cs 54, RGBWColorBreakdownMixingFilter.cs 115.

**OutputFilter/CoarseFineBreakdown**: CoarseFineBreakdownData.cs 46, CoarseFineBreakdownDescriptor.cs 62, CoarseFineBreakdownFilter.cs 51, CoarseFineBreakdownModule.cs 176, CoarseFineBreakdownOutput.cs 110, CoarseFineBreakdownOutputConfiguration.cs 17, Setup/ViewModels/CoarseFineBreakdownSetupResult.cs 10, Setup/ViewModels/CoarseFineBreakdownSetupViewModel.cs 229, Setup/Views/CoarseFineBreakdownSetupView.xaml 95, Setup/Views/CoarseFineBreakdownSetupView.xaml.cs 20.

**OutputFilter/ColorWheelFilter**: ColorWheelFilterData.cs 59, ColorWheelFilterDescriptor.cs 46, ColorWheelFilterModule.cs 86, ColorWheelFilterSetup.cs 87, ColorWheelFilterSetup.Designer.cs 136, Filters/ColorWheelFilter.cs 158, Outputs/ColorWheelFilterOutput.cs 63.

**OutputFilter/DimmingCurve**: DimmingCurveHelper.cs 202, DimmingCurveHelper.Designer.cs 227, DimmingCurveModule.cs 430.

**OutputFilter/DimmingFilter**: DimmingFilterData.cs 44, DimmingFilterDescriptor.cs 46, DimmingFilterModule.cs 77, DimmingFilterSetup.cs 86, DimmingFilterSetup.Designer.cs 137, Filters/DimmingFilter.cs 139, Outputs/DimmingFilterOutput.cs 50.

**OutputFilter/PrismFilter**: Filter/PrismFilter.cs 113, Output/PrismFilterOutput.cs 148, PrismFilterData.cs 71, PrismFilterDescriptor.cs 46, PrismFilterModule.cs 109, PrismFilterSetup.cs 158, PrismFilterSetup.Designer.cs 212.

**OutputFilter/ShutterFilter**: Filter/ShutterFilter.cs 115, Output/ShuttterFilterOutput.cs 173, ShutterFilterData.cs 62, ShutterFilterDescriptor.cs 46, ShutterFilterModule.cs 95, ShutterFilterSetup.cs 152, ShutterFilterSetup.Designer.cs 188.

**OutputFilter/TaggedFilter**: Filters/ITaggedFilter.cs 22, Filters/TaggedFilter.cs 50, Filters/TaggedFilterBase.cs 48, ITaggedFilterData.cs 13, Outputs/ITaggedFilterOutput.cs 16, Outputs/TaggedFilterOutput.cs 19, Outputs/TaggedFilterOutputBase.cs 143, TaggedFilterData.cs 25, TaggedFilterDataBase.cs 22, TaggedFilterDescriptor.cs 39, TaggedFilterDescriptorBase.cs 41, TaggedFilterModule.cs 22, TaggedFilterModuleBase.cs 175, TaggedFilterSetup.cs 76, TaggedFilterSetup.Designer.cs 123.

**Property/Color**: ColorData.cs 44, ColorDescriptor.cs 54, ColorPanel.cs 64, ColorPanel.Designer.cs 45, ColorProperty.cs 221, ColorSetsSetupForm.cs 235, ColorSetsSetupForm.Designer.cs 248, ColorSetupForm.cs 115, ColorSetupForm.Designer.cs 180, ColorSetupHelper.cs 404, ColorSetupHelper.Designer.cs 234, ColorStaticData.cs 68.

**Property/Face**: FaceComponent.cs 16, FaceData.cs 42, FaceDescriptor.cs 25, FaceMapItem.cs 52, FaceModule.cs 112, FaceSetupHelper.cs 540, FaceSetupHelper.Designer.cs 212.

**Property/Grid**: Data.cs 19, Descriptor.cs 43, Module.cs 38, SetupForm.cs 77, SetupForm.Designer.cs 173.

**Property/IntelligentFixture** (entry point only, see below): IntelligentFixtureData.cs 60, IntelligentFixtureDescriptor.cs 61, IntelligentFixtureModule.cs 119.

**Property/Location**: LocationData.cs 39, LocationDescriptor.cs 44, LocationModule.cs 90, SetupForm.cs 44, SetupForm.Designer.cs 163.

**Property/Order**: OrderData.cs 14, OrderDescriptor.cs 44, OrderModule.cs 128, OrderSetupHelper.cs 251, OrderSetupHelper.Designer.cs 130, SetupForm.cs 24, SetupForm.Designer.cs 105.

**Property/Orientation**: Orientation.cs 8, OrientationData.cs 16, OrientationDescriptor.cs 44, OrientationModule.cs 75, OrientationSetupHelper.cs 43, SetupForm.cs 35, SetupForm.Designer.cs 100.

**Property/State**: Setup/Models/StateAssignmentTreeNode.cs 214, Setup/Models/StateAssignmentTreeSelectionController.cs 230, Setup/Models/StateBindableBase.cs 43, Setup/Models/StateDefinition.cs 93, Setup/Models/StateElementNodeSnapshot.cs 69, Setup/Preview/BroadcastStatePreviewPublisher.cs 78, Setup/Preview/IStatePreviewPublisher.cs 30, Setup/Preview/StatePreviewCoordinator.cs 72, Setup/Preview/StatePreviewPair.cs 9, Setup/Services/IStateColorPickerService.cs 18, Setup/Services/IStateDefinitionDialogService.cs 43, Setup/Services/IStateMapperDialogService.cs 17, Setup/Services/StateColorPickerService.cs 62, Setup/Services/StateDefinitionDialogService.cs 97, Setup/Services/StateMapperDialogService.cs 28, Setup/ViewModels/StateDefinitionNameDialogViewModel.cs 138, Setup/ViewModels/StateDefinitionViewModel.cs 105, Setup/ViewModels/StateItemViewModel.cs 174, Setup/ViewModels/StateMapperViewModel.cs 1260, Setup/Views/StateDefinitionNameDialogView.xaml 59, Setup/Views/StateDefinitionNameDialogView.xaml.cs 37, Setup/Views/StateMapperView.xaml 280, Setup/Views/StateMapperView.xaml.cs 18, StateData.cs 86, StateDefinitionData.cs 118, StateDescriptor.cs 25, StateItemData.cs 91, StateModule.cs 110, StateNamingRules.cs 72, StateSetupHelper.cs 122.

**LayerMixingFilter**: ChromaKey/ChromaKeyData.cs 58, ChromaKey/ChromaKeyModule.cs 86, ChromaKey/ChromaKeyModuleDescriptor.cs 48, ChromaKey/ChromaKeySetup.cs 113, ChromaKey/ChromaKeySetup.Designer.cs 305, ColorChange/ColorChangeModule.cs 35, ColorChange/ColorChangeModuleDescriptor.cs 43, HighestValue/HighestValueModule.cs 20, HighestValue/HighestValueModuleDescriptor.cs 39, IntensityOverlay/IntensityOverlay.cs 30, IntensityOverlay/IntensityOverlayModuleDescriptor.cs 43, LumaKey/LumaKeyData.cs 30, LumaKey/LumaKeyModule.cs 49, LumaKey/LumaKeyModuleDescriptor.cs 48, LumaKey/LumaKeySetup.cs 72, LumaKey/LumaKeySetup.Designer.cs 194, Mask/MaskModule.cs 29, Mask/MaskModuleDescriptor.cs 43, MaskFill/MaskAndFillData.cs 26, MaskFill/MaskAndFillSetup.cs 30, MaskFill/MaskAndFillSetup.Designer.cs 109, MaskFill/MaskFillModule.cs 68, MaskFill/MaskFillModuleDescriptor.cs 48, MultiplyColor/MultiplyColor.cs 29, MultiplyColor/MultiplyColorModuleDescriptor.cs 44, ProportionalMix/ProportionalMixModule.cs 25, ProportionalMix/ProportionalMixModuleDescriptor.cs 43.

That is 288 files in scope, 28,612 lines.

**Read in full, outside the listed folders, because the dialogs depend on them:** `Vixen.Common/Controls/SerialPortConfig.cs` 237, `SerialPortConfig.Designer.cs` 260, `MessageBoxForm.cs` 115, `NumberDialog.cs` 35, `TextDialog.cs` 50, `BaseForm.cs`; `Vixen.Common/Utilities/HostNameResolver.cs` 43; `Vixen.Core/Module/Controller/ControllerModuleInstanceBase.cs` 84, `ControllerModuleDescriptorBase.cs` 57; `Vixen.Core/Module/Property/PropertyModuleInstanceBase.cs` 79; `Vixen.Core/Module/OutputFilter/OutputFilterModuleInstanceBase.cs` 103; `Vixen.Core/Module/MixingFilter/LayerMixingFilterModuleInstanceBase.cs` 75; `Vixen.Core/Sys/Output/OutputModuleInstanceBase.cs` 77; `Vixen.Core/Rule/IElementSetupHelper.cs` 19; `Vixen.Core/Module/ModuleLocalDataSet.cs` 34; `ModuleDataModelBase.cs` 30; `ModuleInstanceBase.cs` 94; `Vixen.Core/Factory/ControllerFactory.cs` 26; `Vixen.Application/Setup/SetupElementsTree.cs` 513; `Vixen.Common/DiscreteColorPicker/Views/SingleDiscreteColorPickerView.xaml` 57; `Vixen.Tests/DDP/DDPDataTests.cs` 37; `Vixen.Tests/Common/HostNameResolverTests.cs` 47; `Vixen.Tests/OutputFilter/CoarseFineBreakdown/CoarseFineBreakdownSetupViewModelTests.cs` 111; `Vixen.Tests/Sequencer/LayerMixingFilterDataJsonSerializationTests.cs` 126.

**Read in part, only the lines this area touches:** `Vixen.Common/Controls/ControllerTree.cs` 1367 (613-630, 840-900, 1158-1172, 1200-1245, 1352-1362), `ControllerTree.Designer.cs`, `MessageBoxForm.Designer.cs` 176, `NumberDialog.Designer.cs`, `TextDialog.Designer.cs`, `NumericTextBox.cs` 331 (170-262), `DragDropListView/DragDropListView.cs` 544 (76-135, 200-225), `Vixen.Common/Utilities/ElementTemplateHelper.cs` 150 (40-125), `Vixen.Common/Help/Help.cs`; `Vixen.Application/Setup/DisplaySetup.cs` 281 (1-140, 270-278), `SetupControllersSimple.cs` (70-90, 200-220), `SetupPatchingGraphical.cs` 2101 (1266-1300), `SetupPatchingSimple.cs` 698 (235-265), `Vixen.Application/GraphicalPatching/FilterShape.cs` 88, `Vixen.Application/VixenApplication.cs` 1384 (993-1030); `Vixen.Core/Sys/Output/OutputController.cs` 399, `OutputModuleConsumer.cs` 98, `Vixen.Core/Sys/Modules.cs` (495-540), `Vixen.Core/Sys/Execution.cs` (118-121), `Vixen.Core/Sys/VixenSystem.cs` (230-343), `Vixen.Core/Services/ApplicationServices.cs` (138-150), `Vixen.Core/Module/ModuleDataSet.cs` 348 (1-140), `Vixen.Core/IO/Xml/Serializer/XmlModuleDataModelSerializer.cs` 185; `Editor/LayerEditor/Views/LayerEditorView.xaml` 124, `Editor/LayerEditor/ViewModels/LayerEditorViewModel.cs` 357 (190-215), `Editor/LayerEditor/Services/LayerEditorLayerService.cs` 123 (40-90); `Vixen.Common/DiscreteColorPicker/Views/DiscreteColorPickerViewBase.cs`, `Vixen.Common/DiscreteColorPicker/ViewModels/DiscreteColorPickerViewModelBase.cs`, `SingleDiscreteColorPickerViewModel.cs`, read in full; `Editor/TimedSequenceEditor/TimedSequenceEditorForm_Menu.cs` 828 (575-610); `Preview/VixenPreview/PreviewCustomPropBuilder.cs` 425 (60-125); `App/Curves/Curve.cs` 396 (1-65), `App/LipSyncApp/LipSyncMapStaticData.cs` (55-90); `Effect/LipSync/LipSync.cs` (117-180).

**Also consulted, read only:** the owner's `SystemData/ModuleStore.xml` and `SystemConfig.xml` under `/Users/developer/Documents/Vixen 3`, the Rust in `crates/vixen-output`, `vixen-io`, `vixen-core`, `crates/vixen-session/src/dialogs/` and `crates/vixen-ffi/src/dialogs/` (for names only), the names in `app/Sources/VixenDialogs/`, and `docs/spec/display-setup.md` where the two specs meet.

**Could not be read fully, or left out on purpose:**

- `.resx` files hold only base64 icons and pictures. They were not decoded. Same for `RDSController/rds logo.png`, the two `mrds1322.pdf` files and `E131/J1Sys/MessageBoxIcons/*.ico`.
- `IntelligentFixtureModule.Setup` opens `Editor/FixturePropertyEditor`, which is outside this area. This spec covers only that entry point.
- `App/Curves/CurveEditor` is outside this area and only its entry points are specified here. Same for the WinForms colour picker (`Vixen.Common/Controls/ColorPicker`), replaced by the system colour panel.
- The State mapper is a two-pane window, more than a simple form. It was read in full and is listed in F193 to F211 so nothing is lost; if another spec owns it, that spec wins.
- Runtime filter and controller behaviour is summarised only where a setup field controls it. `crates/vixen-output` owns the runtime.
- Nothing was run. .NET framework behaviour quoted below (for example what `SerialPort` rejects) is from knowledge of the framework, not from this repo.

## Features

### Conventions used by every entry

- **Quirk:** what upstream does that looks accidental. **Mac:** the instruction for the port. Anything without a **Mac:** note follows upstream exactly. Every deviation is collected in the register at the end of this section so the owner can veto it.
- **Labels are verbatim**, spelling mistakes included (`docs/UI_GUIDE.md`: "Keep every feature, option and label wording"). Known typos are listed in the register.
- **Undo:** none of these dialogs registers undo upstream, and neither do the Mac versions. The only rollback is Cancel on the sheet, and Cancel on Display Setup, which reloads the whole configuration (`Vixen.Application/VixenApplication.cs:1010-1016`). Text fields keep native undo while being edited. Entries don't repeat this.
- **Persistence:** OK writes to the module's in-memory data. It reaches disk only when Display Setup's OK saves `SystemConfig.xml` and `ModuleStore.xml` (`VixenApplication.cs:1000-1009`). Entries name the data member. Formats are in "Data and persistence".
- **Staging (Mac):** every sheet edits a draft fetched from Rust and calls one Rust `apply` on OK. Upstream dialogs that write through live are marked as a quirk.
- **Keyboard (Mac):** Return is OK and Esc is Cancel on every sheet. Upstream has no Accept/Cancel buttons on `DummyLightingSetup`, `DDPSetup`, `OpenDMX SetupDialog`, RDS and Launcher `SetupForm`, `ColorBreakdownSetup`, `ColorSetupHelper`, `ColorSetupForm`, `ColorSetsSetupForm` and `OrderSetupHelper`; adding them is the Apple-ism.
- **Alerts:** upstream's `MessageBoxForm(text, title, noVisible, cancelVisible)` shows OK only for `(false,false)`, YES/NO for `(true,false)`, YES/CANCEL for `(false,true)`, YES/NO/CANCEL for `(true,true)`. YES returns `DialogResult.OK` (`Vixen.Common/Controls/MessageBoxForm.cs:11-25`). The `MessageBoxButtons` overload gives OK, OK/CANCEL, YES/NO or YES/NO/CANCEL (`:34-60`). **Mac:** `.alert` with upstream's title and text, and verbs on the buttons as each entry says.
- **Sizes:** upstream dialogs are fixed-size unless stated. **Mac:** sheets size to content; keep control order and region.

### macOS device mapping

**Serial ports.** Applies to F7 to F25, F100 (Renard), F102 (DMX Pro), F105 (DMX Open) and F119 (RDS).

1. List callout devices only, `/dev/cu.*`. `crates/vixen-output/src/serial.rs` `list_serial_ports()` does this today. Never offer `/dev/tty.*`: opening one blocks until carrier is detected. If saved data holds `/dev/tty.X`, treat it as `/dev/cu.X`.
2. Hide what isn't a light controller from the picker: `/dev/cu.Bluetooth-Incoming-Port`, `/dev/cu.debug-console`, `/dev/cu.wlan-debug`. Don't open Bluetooth serial ports to test them, since that can stall or start a pairing. List USB ports first, then the rest, each group sorted by name.
3. Show `"{USB product} ({basename})"` when IOKit knows the product (`serial.rs` `list_serial_port_details()`, over `serialport::available_ports()`, gives vendor, product, serial, manufacturer, USB or Bluetooth), otherwise the basename, plus the upstream suffix ` (IN USE)`. Store the full path in `PortName`, for example `/dev/cu.usbmodem1101`.
4. A saved Windows name such as the owner's `COM9` stays untouched in the data until the user picks a port (`serial.rs` `is_windows_port_name`, `PortOpenError::WindowsName`). The dialog shows it after "Current Configured Port:", and the port picker opens with nothing selected.
5. "IN USE" means another process holds the port. Ports held by this app's own running controllers are marked from the engine's list without opening them. Everything else is probed off the main thread with a short timeout (`serial.rs` `port_in_use`). Upstream probes on the UI thread (`Vixen.Common/Controls/SerialPortConfig.cs:19-44`). Opening a port on macOS can pulse DTR and reset an Arduino, so probe only when this dialog opens, never in the background.
6. Open with `Handshake.None`: `dtr_on_open(false)` and RTS off for Generic Serial and DMX Pro (.NET defaults), DTR and RTS on for Renard (`Controller/Renard/Module.cs:131-132`). `serial.rs` `open_port(settings, dtr, rts)` already takes these.
7. macOS termios has no Mark or Space parity and no 1.5 stop bits. `open_port` returns `UnsupportedSetting`. See F16 and F18.
8. If sandboxed, the app needs `com.apple.security.device.serial`.
9. The `serialport` crate sets the rate through `IOSSIOSPEED` on macOS, which is what makes F14's 460800, 921600 and 1048576 possible (from knowledge of the crate). An adapter or driver can refuse one, and that surfaces as `PortOpenError::Unavailable` at start, in the status of F36. A pseudo terminal in tests has no baud rate at all; `open_port` already falls back for that.
10. `/dev/cu.*` is world read-write on macOS, so a failed open almost always means busy or gone, not permissions. `port_in_use` treats `EBUSY` and permission errors as in use, as upstream treats `UnauthorizedAccessException` and `InvalidOperationException` (`Vixen.Common/Controls/SerialPortConfig.cs:32-37`). A port another macOS process holds with `TIOCEXCL` reads as in use too.

**Network interfaces.** Applies to E1.31 (F52 to F60).

1. `NetworkInterface.Id` becomes the BSD name, `en0`. `crates/vixen-output/src/net.rs` `list_network_interfaces()` already reports id, flags and IPv4 addresses, but its `name` is the BSD name too.
2. Upstream offers an interface when it is not a tunnel, not loopback, and supports multicast (`Controller/E131/SetupForm.cs:77`). It doesn't test whether the interface is up, so a down interface stays listed and start then logs F91's "currently down" line. `is_offered_for_multicast` matches that. On macOS this also lets through `awdl0`, `llw0` and `anpi*`, which nobody wants. **Mac:** offer the interfaces `SCNetworkInterfaceCopyAll` returns, intersected with that rule, plus the saved one if it exists (`net.rs` `multicast_choices(saved)`). That list leaves those three out and keeps `bridge0` as "Thunderbolt Bridge". Name is `SCNetworkInterfaceGetLocalizedDisplayName` ("Wi-Fi", "Ethernet", "Thunderbolt Bridge"), falling back to the BSD name.
3. List text is `Multicast {Name} ({Id})`. Upstream shows `Multicast {Name}` and throws on duplicate names (`Controller/E131/SetupForm.cs:80, 86`). The BSD name makes them unique. Upstream's list is sorted by name, multicast entries first, because both tables are `SortedList`s (`Controller/E131/SetupForm.cs:58`, `Controller/E131/E131OutputPlugin.cs:69`). **Mac:** sort each group the same way.
4. A saved Windows GUID is not found and takes the UNKNOWN path in F53. It is kept in the data until the user picks another destination.
5. Upstream's default of `Multicast Local Area Connection` (`Controller/E131/SetupForm.cs:289-290`) means nothing here. **Mac:** default to the primary interface (`State:/Network/Global/IPv4` `PrimaryInterface` from SCDynamicStore), when the saved destination is empty.
6. Sending: UDP on IPv4, `IP_MULTICAST_IF` set to the interface's IPv4 address, TTL 64, group `239.255.{hi}.{lo}`, port 5568 (`Controller/E131/E131OutputPlugin.cs:481-485, 559-566`). Upstream uses the last IPv4 address in the interface's list (`:543-549`).
7. macOS may show its Local Network prompt on first send. The app needs `NSLocalNetworkUsageDescription` in Info.plist. Affects E1.31, DDP, Elexol and RDS HTTP.

### How the dialogs are reached

- [ ] **F1 Configure a controller.** One controller selected in Display Setup's Controllers pane: the cog button (enabled when exactly one is selected, `Vixen.Application/Setup/SetupControllersSimple.cs:75, 203-209`), context menu `&Configure` (`Vixen.Common/Controls/ControllerTree.Designer.cs:89`, `ControllerTree.cs:1162-1165`), or double-click (`ControllerTree.cs:1356-1359`). Calls `Setup()` when `HasSetup` (`:868-880`), and if it returns true raises controllers-changed and refreshes output names shown in the tree (`:613-630`). **Mac:** presents the module's sheet on the Display Setup window. A module with no setup (F134, Debugging Values) does nothing; the button stays enabled. See also `docs/spec/display-setup.md` F87.
- [ ] **F2 Configure a filter.** Double-click a filter shape in graphical patching; only a shape that's already selected reacts, because the diagram sends clicks to every shape under the mouse (`Vixen.Application/Setup/SetupPatchingGraphical.cs:1274-1294`, `Vixen.Application/GraphicalPatching/FilterShape.cs:75-84`). On true it recalculates control points, because output counts may have changed. A filter shape's title is the module's `Name`, its `TypeName` (`Vixen.Core/Module/OutputFilter/OutputFilterModuleInstanceBase.cs:97-100`), except the tag family (F231). Simple patching has no route to filter setup. `docs/spec/display-setup.md` F124 owns the gesture.
- [ ] **F3 Configure a property.** In Display Setup's Elements pane, select exactly one row in the properties list, then the cog button or double-click the row (`SetupElementsTree.cs:212-215, 355, 390-393`). The list shows the properties of the tree's selected node (`:318-325`). Three branches (`:228-260`): a Color property runs `ColorSetupHelper` over all selected elements, preset from the clicked property's type, set name and colour (`:230-232`), not `ColorModule.Setup` (F151); otherwise `HasSetup` runs `Setup()` and on true copies the result to the same property on every other selected element with `CloneValues`; otherwise `HasElementSetupHelper` runs `SetupElements(selected)` (Face, State). **Quirk:** Grid has no `CloneValues`, so the base class resets the other elements to defaults (`Vixen.Core/Module/Property/PropertyModuleInstanceBase.cs:32-35`). **Mac:** copy Grid values instead.
- [ ] **F4 Run a setup helper.** Combo plus cog-go button in the Elements pane (`SetupElementsTree.cs:59-67, 130-144`), enabled when any element is selected (`:352`). Combo lists every `IElementSetupHelper` by `HelperName`, found by reflection (`Vixen.Core/Services/ApplicationServices.cs:141-150`), in no fixed order, first selected: "Color Handling" (F151), "Dimming Curve" (F238), "Face Mapping" (F184), "Orientation" (F179), "Patching Order" (F170), "State Mapping" (F193). **Mac:** a fixed list in that alphabetical order. After running it refreshes the tree and properties list whatever the result.
- [ ] **F5 Configure a layer.** Layer Editor's `Configuration` button, visible only when the layer's filter `HasSetup` (`Editor/LayerEditor/Views/LayerEditorView.xaml:109-115`). On true raises layer-changed (`Editor/LayerEditor/ViewModels/LayerEditorViewModel.cs:196-207`). See F212 to F229.
- [ ] **F6 Other routes to the helpers.** Sequencer menu `Edit Element Face Mapping` picks nodes, runs F184, then re-renders lip sync and saves system and module config immediately (`Editor/TimedSequenceEditor/TimedSequenceEditorForm_Menu.cs:584-607`). Simple patching runs F170 (`Vixen.Application/Setup/SetupPatchingSimple.cs:242-246`). After an element template, `Vixen.Common/Utilities/ElementTemplateHelper.cs:63-83`: alert "Would you like to configure a dimming curve for this Prop?", title "Dimming Curve Setup", question icon, YES/NO; YES runs F238 in simple mode; then, if the template wants colour, F151 preset to full colour. The custom prop importer does the same with "Would you like to configure a dimming curve for this element?". Its colour step runs unless the prop's colour mode is Other, preset from that mode, and full colour uses silent mode (`Preview/VixenPreview/PreviewCustomPropBuilder.cs:68-101`).

### Serial Port Config (shared by Generic Serial, Renard, DMX Pro)

`S` = `Vixen.Common/Controls/SerialPortConfig.cs`, `SD` = `Vixen.Common/Controls/SerialPortConfig.Designer.cs`.

- [ ] **F7 Serial Port Config window.** Title "Serial Port Config" (`SD:235`). Opened from F27 and directly by Configure for Renard (F100) and DMX Pro (F102). Constructor takes the current port, or null (`S:10-11`). The `allow*Edit` flags (`S:45-49`) are never passed by any caller, so all fields are always editable.
- [ ] **F8 Port scan on open.** For each port name, open then close it. Success lists the name; `UnauthorizedAccessException` or `InvalidOperationException` lists it as `name (IN USE)`; any other exception logs a warning and **omits the port** (`S:19-44`). No refresh control exists. **Mac:** off the main thread, per the mapping above. Add a Refresh button beside the picker, since USB adapters come and go. A port whose probe fails for any other reason stays listed instead of vanishing, because the probe only answers "in use" and opening it for real reports the actual error (F36).
- [ ] **F9 "Current Configured Port:" label.** Top of the window (`SD:207`). Value label beside it shows the passed port's name, or "None" when null (`S:57-62`).
- [ ] **F10 "Port Details" group.** Holds F11 to F18 (`SD:64`). Enabled only when there's a port to edit (`S:90-100, 104-108`). When the passed port is null and no ports exist, the group is disabled and OK silently does nothing (`S:172-200, 205-207`); Cancel is the only way out. **Mac:** show `ContentUnavailableView`-style text "No serial ports found" in the group and disable OK.
- [ ] **F11 Default values.** Passed port null and ports exist: first listed port, 57600, None, 8, One (`S:64-67`). Passed port not null: baud, parity, data bits and stop bits load from it (`S:69, 88-97`); the port name is F12. **Quirk:** "ports exist" asks the system again, so if every port was dropped by F8 the group is enabled with an empty picker and OK reports `* Port name has not been selected.`
- [ ] **F12 Port name.** Label "Port name". Drop-down list of scanned ports (`SD:164-175`). **Quirk:** after loading the current port, `SelectedIndex = 0` always overrides it (`S:72-74`), so the picker shows the first port and pressing OK silently changes the port. That is why the "Current Configured Port" label exists. **Mac:** preselect the configured port when it's present. When a configured port is absent (including `COM9`), select nothing and require a choice; never fall back to the first port then, which on a Mac could be anything. With nothing configured, F11's first port stands, and the mapping lists USB ports first.
- [ ] **F13 Port name value rule.** The ` (IN USE)` text is stripped before use (`S:114-129`).
- [ ] **F14 Baud rate.** Label "Baud rate". Drop-down list, exactly: 2400, 4800, 9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600, 1048576 (`SD:73-84`). A saved value not in the list shows blank and fails validation (`S:139-142, 144`).
- [ ] **F15 Parity label and list.** Label "Parity". Drop-down list in enum order: None, Odd, Even, Mark, Space (`S:51`).
- [ ] **F16 Mark and Space on macOS.** **Mac:** list them but disabled with `.help("macOS serial ports don't support this setting")`. If saved data holds one, show it selected with a note under the picker, and let the user change it. The controller's start reports `UnsupportedSetting`.
- [ ] **F17 Data bits.** Label "Data bits". Text box, `MaxLength` 1 (`SD:152`). Non-numeric gives -1 and the validation message (`S:158-161, 183-185`). **Quirk:** any single digit passes validation, then `new SerialPort` throws for values outside 5 to 8 when `SelectedPort` is read (`S:84`). **Mac:** picker with 5, 6, 7, 8. Rust still validates imported data.
- [ ] **F18 Stop bits.** Label "Stop bits". Drop-down list: One, Two, OnePointFive; `None` removed (`S:52-54`). **Mac:** OnePointFive handled as in F16.
- [ ] **F19 Control order and region.** Label and control rows top to bottom: Port name, Baud rate, Parity, Data bits, Stop bits (`SD:116-175`). Group below the current-port label. OK then Cancel at bottom right (`SD:181, 193`).
- [ ] **F20 OK validation.** Builds one line per problem: `* Port name has not been selected.`, `* Baud rate has not been selected.`, `* Invalid value for data bits.` (`S:177-185`). Shows "The following items need to be resolved:", blank line, the lines; title "Serial Port"; hand icon; OK only; dialog stays open (`S:187-193, 205-207`). Message text is built in Rust.
- [ ] **F21 In-use warning.** On a valid OK when the selected item has `(IN USE)`: "Serial Port may be in use, do you wish to continue?", title "Warning", warning icon, YES/NO. NO stays open; YES closes with OK (`S:214-224`). A running controller's own port reads as in use here, because the scan can't open it. **Mac:** buttons "Use Port" and "Choose Another".
- [ ] **F22 OK result.** `SelectedPort` is a new port object with the chosen values (`S:83-85`). Nothing is written to module data here: the caller does that.
- [ ] **F23 Cancel.** Closes, caller keeps its previous port (`SD:192`).
- [ ] **F24 Default and cancel keys.** `AcceptButton` OK, `CancelButton` Cancel (`SD:220, 223`).
- [ ] **F25 Combos are owner-drawn only for the theme.** `comboBox_DrawItem` (`S:233-236`) and `groupBoxes_Paint` (`S:228-231`) have no behaviour. Omit.

### Generic Serial (owner's real controller)

`G` = `Controller/GenericSerial/`. TypeName "Generic Serial", TypeId `d22c9233-16e9-4008-a215-e577f78a7a2e` (`G/Descriptor.cs:7, 36`). The owner's "Arduino" controller: 16 outputs, `COM9`, 9600, None, 8, One, empty header and footer.

- [ ] **F26 Generic Serial Configuration window.** Title "Generic Serial Configuration" (`G/SetupDialog.Designer.cs:181`). Opened by F1 (`G/Module.cs:76-92`). Top group, then packet group, OK and Cancel bottom right.
- [ ] **F27 "Serial Port Settings" group.** (`G/SetupDialog.Designer.cs:140`). Label "Currently:" (`:159`) then the settings text. Button "Setup/Change Serial Port" (`:126`) opens F7 with the current port; on OK adopts the port, enables OK, updates the text (`G/SetupDialog.cs:44-53`).
- [ ] **F28 Settings text.** `"{port}: {baud}, {parity}, {databits}, {stopbits}"`, e.g. `COM9: 9600, None, 8, One` (`G/SetupDialog.cs:79-85`). "Not Set" when `PortName` is null or empty (`:33-41`). The designer's `COM1: 115200, Space, 8, OnePointFive` is a placeholder and never shows. **Quirk:** a saved name with zero baud, data bits or stop bits throws here, in `new SerialPort` (`:34`). **Mac:** show the saved name and values as they are, and let the user fix them through F27.
- [ ] **F29 "Packet Data" group.** (`G/SetupDialog.Designer.cs:79`).
- [ ] **F30 "Send a text header" checkbox.** Enables the field below (`G/SetupDialog.cs:55-58`). Checked at open only when `Header` is non-empty (`:21-25`). Default unchecked, field disabled (`:18`).
- [ ] **F31 Header text field.** Free text on a single line, so no way to type CR, LF or a control byte. WinForms' default limit of 32767 characters, no validation, no escape processing. On send it's ASCII-encoded, and non-ASCII characters become `?` (`G/Module.cs:126`). Text stays in the box when unchecked, but saving unchecked stores empty (`G/SetupDialog.cs:67`). Persists: `Header`.
- [ ] **F32 "Send a text footer" checkbox.** As F30 (`G/SetupDialog.cs:27-31, 60-63`).
- [ ] **F33 Footer text field.** As F31 (`G/SetupDialog.cs:68`, `G/Module.cs:128`). Persists: `Footer`.
- [ ] **F34 OK button.** Disabled until a port is set (`G/SetupDialog.cs:36, 40, 49`). Writes header, footer, baud, data bits, parity, port name, stop bits (`:67-74`). Then the module rebuilds its port and header and footer bytes, reopening at once if running (`G/Module.cs:85, 122-130, 155-158`). `Setup` returns `_data.IsValid`: port name non-null, baud, data bits and stop bits non-zero (`G/Data.cs:39-49`).
- [ ] **F35 Cancel button.** Discards. `Setup` returns false (`G/Module.cs:91`).
- [ ] **F36 Open failure status.** No dialog. Failure to open is only logged (`G/Module.cs:192-220`). `UnauthorizedAccess`, `InvalidOperation` or `IO` errors stop the controller and arm a repeating 120000 ms retry timer, up to 3 retries, then give up and reset the counter (`:31, 200-214`). Success while retrying logs "Serial Port conflict has been corrected..." (`:183-190`). Any other exception (.NET's `ArgumentException` for a bad name, for example) is logged once, with no retry, and the controller stays started with a closed port, sending nothing (`:216-219`). **Quirk:** `Stop` never touches the timer (`:113-120`), so stopping a controller by hand doesn't cancel a pending retry, and the next tick starts it again (`:223-227`). **Mac:** same retry logic (`serial.rs` `RetryTimer`), Stop cancels it (`RetryTimer::stop`), and because a saved `COM9` will fail on every start, show the reason as a status on the controller's row in Display Setup ("COM9 is a Windows port name..." from `PortOpenError`) rather than hiding it in a log.
- [ ] **F37 Data sent, for reference.** One packet per update: header bytes, one byte per output, footer bytes; nothing is sent when the port is closed; a write exception is logged, not raised (`G/Module.cs:36-69`). 8-bit evaluator, highest-wins combinator (`G/DataPolicy.cs:10-18`).

### Dummy Lighting (owner's real controller)

`D` = `Controller/DummyLighting/`. TypeName "Dummy Lighting (for testing)", TypeId `b6ee6308-189c-4268-8996-32a4bab8ab5f` (`D/DummyLightingDescriptor.cs:7, 26`). The owner's "Light Strip Controller": 3351 outputs, Monochrome, title "Dummy Lighting Controller".

- [ ] **F38 Dummy Lighting Configuration window.** Title "Dummy Lighting Configuration" (`D/DummyLightingSetup.Designer.cs:134`). **Quirk:** `Setup` reads `_form.RenderingStyle` and `_form.Text` (`D/DummyLighting.cs:72`), and `_form` is null until the first `Start`, so configuring a controller that was never started throws. `Stop` and `Dispose` throw the same way (`:61, 109`). The owner's saved config lists both controllers under `DisabledDevices`, so this is a live case. **Mac:** always read from the data; this dialog never needs a running controller. No `AcceptButton` or `CancelButton` upstream (Conventions). Sheet is fixed-size.
- [ ] **F39 "Window Title:" field.** Label and text box on one row at the top (`D/DummyLightingSetup.Designer.cs:103-115`). Free text; blank allowed (one of the owner's saved entries has an empty `FormTitle`). Default "Dummy Lighting Controller" (`D/DummyLightingData.cs:18`). Persists: `FormTitle`.
- [ ] **F40 Render style radio group.** Three radios, top to bottom, exactly one selected, set from data at open (`D/DummyLightingSetup.cs:22-35`). Default Monochrome (`D/DummyLightingData.cs:17`). Persists: `RenderStyle`.
- [ ] **F41 Radio "Set up as monochrome channels".** Value `Monochrome` (`Designer:70`): each output is one grey box.
- [ ] **F42 Radio "Set up as RGB channels, with every 3 inputs mapping to the R, G, B components of a single RGB output".** Value `RGBMultiChannel` (`Designer:82-83`): three outputs make one box.
- [ ] **F43 Radio "Set up as RGB channels, with every input" newline "mapping to a full-color RGB channel".** Value `RGBSingleChannel` (`Designer:96`): one output is one colour box.
- [ ] **F44 OK.** Stores style and title, applies both to the output window if it exists, switches the data policy, returns true (`D/DummyLighting.cs:74-81`). Policies: Monochrome is `_8BitEvaluator` plus `_8BitHighestWinsCombinator`; RGBMultiChannel is `_8BitEvaluator` plus `_8BitAverageCombinator`; RGBSingleChannel is `ColorEvaluator` plus `NaiveColorCombinator` (`D/DummyLighting.cs:90-105`, the three `*DataPolicy.cs:10-18`). Changing the factory raises `DataPolicyFactoryChanged`, and the controller rebuilds its policy at once (`Vixen.Core/Module/Controller/ControllerModuleInstanceBase.cs:26-43`, `Vixen.Core/Sys/Output/OutputController.cs:50, 65-68`), so the switch applies while running. **Mac:** the engine must pick up the new policy without a restart.
- [ ] **F45 Cancel.** Returns false (`D/DummyLighting.cs:82`). Bottom right, OK then Cancel (`Designer:43, 54`).
- [ ] **F46 Output window appears on Start.** Starting the controller creates and shows a window titled `FormTitle`, resets the update counter (`D/DummyLighting.cs:49-57`). Stopping hides it (`:59-63`). **Quirk:** each Start builds a new form and old ones are only hidden, so they leak (`:51`). `IsRunning` is `_form != null && (_form.Visible || _form.IsDisposed)` (`:87`), so closing the window by hand leaves the controller "running". The window is an ordinary resizable one, centred on its parent, with no controls and no menu. **Mac:** a `WindowGroup` window per controller, closed on Stop. Closing it by hand leaves the controller running; next Start reopens it. Default size 331 x 302 (`D/DummyLightingOutputForm.Designer.cs:42`).
- [ ] **F47 Output window grid.** Black background (`D/DummyLightingOutputForm.cs:141`). Boxes fill left to right, top to bottom. `across = floor(sqrt(n))`, `down = round(n / across)` away from zero, `boxW = clientW / across - 1`, `boxH = clientH / down - 1`, one pixel gap (`:83-88`). For RGBMultiChannel `n = ceil(outputs / 3)` (`:74`). No outputs means no boxes (`:89-94`). **Quirks:** layout is computed only when output count or style changes, so resizing doesn't reflow; `down` rounds down for some counts, so the last boxes fall off the bottom (10 outputs: across 3, down 3, but 4 rows are needed). **Mac:** reflow on resize and use `down = ceil(n / across)`.
- [ ] **F48 Box colours.** Monochrome: white with alpha = value over black, so grey level = value (`D/DummyLightingOutputForm.cs:145-149`). RGBMultiChannel: R, G, B from three consecutive values, starting index 2 stepping 3; a trailing partial group isn't drawn (`:153-164`). RGBSingleChannel: the colour command, or transparent when none, which leaves black (`:168-172`, `D/CommandHandler.cs:9-13`).
- [ ] **F49 FPS readout.** Red text at (2, 2) over the boxes, `"{fps:F2} fps"` (`D/DummyLightingOutputForm.cs:177`). `fps = 1000 * updates / elapsedMs` since Start, a running average, not instantaneous (`D/DummyLighting.cs:37-46`). **Quirk:** first update divides by zero and shows infinity. **Mac:** show 0.00 until time has passed. Use `.monospacedDigit()`.
- [ ] **F50 State reset.** Becoming visible zeroes all values (`D/DummyLightingOutputForm.cs:97-107`).

### Streaming ACN (E1.31)

`SF` = `Controller/E131/SetupForm.cs`, `SFD` = `Controller/E131/SetupForm.Designer.cs`, `OP` = `Controller/E131/E131OutputPlugin.cs`, `UF` = `Controller/E131/UnicastForm.cs`, `UFD` = `Controller/E131/UnicastForm.Designer.cs`. TypeName "Streaming ACN (E1.31)", TypeId `771D7EBA-662A-48D5-AEDB-445C8708E878`. `HasSetup` true (`OP:142-145`).

- [ ] **F51 Streaming ACN (E1.31) Configuration window.** Title `Streaming ACN (E1.31) Configuration - {controller name}` (`OP:182-184`). Two tabs in order, first selected: "Universes", "Advanced Options" (`SFD:328, 445`, `SF:110`). OK and Cancel below the tabs (`SFD:150, 160`). While open the controller sends nothing (`OP:156, 242, 711-714`). Filled from data at open (`OP:165-180`); each stored universe appears with `Start + 1` because storage is zero-based. With automatic starts, opening recalculates and discards stored starts (`SF:227-237, 713-732`).
- [ ] **F52 Destination label.** "Destination", larger font (14.25 pt in `SF:93`), at bottom left of the Universes tab (`SFD:259`). **Mac:** `.title3`.
- [ ] **F53 Destination picker.** Drop-down list: `Multicast {name}` for every offered interface, then `Unicast {address}` for every known unicast destination (`SF:640-654`). Selected from data at open (`SF:239-296`). If the saved interface id isn't found: alert "The selected multicast interface was not found, be sure to verify your destination.", no title, error icon, OK; adds an entry `Multicast UNKNOWN` that still maps to the saved id, and selects it (`SF:264-282`). So OK keeps the old id. The list of known unicasts is a process-wide table, filled from every plugin's saved `Unicast` on start and at this dialog's open (`OP:69, 411-417`, `SF:246-253`), and not saved separately. If both destinations are saved, the multicast one wins the display (`SF:258-283`). If neither is saved and upstream's default isn't there, nothing is selected, and OK then stores neither (`SF:287-291, 312`), which is F91's "unassigned destinations" case. Whichever entry is selected at OK replaces both members (`OP:203-206`).
- [ ] **F54 Add unicast button** (add icon, right of the Destination label, `SFD:212-217`). Ends any grid edit, opens F56 (`SF:370-402`). On OK with an address already known: alert "Error - Duplicate IP Address", title "IP Address Validation", error icon. Otherwise adds it. Either way selects `Unicast {address}` (`SF:387-400`). The table changes at once, so this dialog's Cancel doesn't undo it. Harmless, since the table isn't saved: keep it. **Mac:** SF Symbol `plus`, label "Add Unicast Destination".
- [ ] **F55 Remove unicast button** (delete icon, `SFD:201-206`). Disabled when a multicast entry or nothing is selected (`SF:653, 1237-1243`). Rejects a destination used by another running plugin: "This destination is in use by another instance of the plugin and cannot be removed.", title "Streaming ACN (E1.31)" (`SF:1217-1230`). Otherwise removes it and selects the first entry (`SF:1232-1234`). The "Multicast destinations cannot be removed." alert (`SF:1204-1211`) is unreachable while the button is disabled, but keep the check in Rust. **Mac:** SF Symbol `minus`, label "Remove Unicast Destination". **Quirk:** with an empty list, selecting index 0 throws. Mac leaves it empty.
- [ ] **F56 New Unicast Destination window.** Title "New Unicast Destination" (`UFD:120`). Always on top (`UFD:121`). Top: the address or name field. Below: radios. Bottom: OK then Cancel.
- [ ] **F57 Radio "IP Address".** Default selected (`UFD:58, 64`). Enables the IP field, default `127.0.0.1` (`UFD:96`, `UF:50-67`). Upstream's field is the Windows `SysIPAddress32` control, four octets, each limited by the control (`Controls/IPTextBox.cs:12`). **Mac:** one text field; OK disabled unless it parses as IPv4.
- [ ] **F58 Radio "Network Name".** Enables the name field, default `myComputer` (`UFD:75, 87`). No validation upstream. Blank would store an empty unicast name. **Mac:** OK disabled when blank. Only one of the two fields shows at a time, same place (`UF:50-67`).
- [ ] **F59 OK and Cancel on F56.** Return the chosen text or nothing (`UF:23, 38-48`). Labels `&OK`, `&Cancel` (`UFD:41, 52`); no mnemonics on Mac.
- [ ] **F60 Unicast resolution at start, for reference.** `Dns.GetHostAddresses`, last IPv4 result wins, port 5568, one shared socket for all universes (`OP:432-473`). Failure logs "Couldn't connect to host {name}." and start isn't clean.
- [ ] **F61 Universe grid.** Left side of the Universes tab (`SFD:341-380`), row numbers in the row header drawn as 1-based index (`Controls/DataGridViewNumbered.cs:17-53`). Columns in order: Start, On, Universe, Size. Cell selection, not row selection (`SFD:367`). No new-row line (`SFD:343`). No user-resizable columns. Numeric columns right-aligned. **Mac:** AppKit `NSTableView` (see structure section).
- [ ] **F62 Start column.** Text, max 5 characters (`SFD:388`), digits only (F66). Range 1 to 99999: "Start must be between 1 and 99999 inclusive" (`SF:768-783`). Read-only unless manual mode, and shown shaded and italic when read-only (`SF:1055, 1062-1075`). Automatic values: first row 1, each next row previous start + previous size (`SF:713-732`), recalculated after editing Start or Size, adding, deleting, and toggling manual mode (`SF:234, 1042-1051, 1077, 1116, 1143, 1160, 1175`). **Quirk:** cell validation also runs on read-only automatic cells, so once a start passes 99999 (row 198 with universes of 510), clicking that cell and then away shows the alert and traps focus there (`SF:768-783, 803-811`). **Mac:** read-only cells aren't validated, and automatic starts may exceed 99999.
- [ ] **F63 On column.** Checkbox. New rows checked (`SF:1115`). Persists: `UniverseEntry.Active`. Inactive universes aren't sent, but all the validation in F87 still counts them.
- [ ] **F64 Universe column.** Text, max 5 characters, digits only. Range 1 to 64000: "Universe must be between 1 and 64000 inclusive" (`SF:750-765`).
- [ ] **F65 Size column.** Text, max 3 characters, digits only. Range 1 to 512: "Size must be between 1 and 512 inclusive" (`SF:785-800`).
- [ ] **F66 Cell input and validation.** Editing a numeric cell allows digits and control characters only; anything else beeps (`SF:457-475, 814-826`). Leaving an invalid cell: the row gets error text and an alert with that text, title "Cell Validation", exclamation icon; focus stays in the cell (`SF:803-811`). Error text clears at end of edit (`SF:659-662`). **Quirk:** neither button sets `CausesValidation = false` (`SFD:146-164`), so while a cell or F79's field is invalid, clicking Cancel does nothing either (framework behaviour). **Mac:** `NSSound.beep()`; the alert is a sheet; validation in Rust. Cancel always works. OK and F54 commit the open editor first (`SF:374-380`), and OK reports an invalid value through this same alert.
- [ ] **F67 Column header click (sort).** With automatic starts: first click only, once per dialog: "Sorting is not available while Vixen manages the start values.", no title, warning icon (`SF:680-690`). With manual starts: left click sorts ascending, right click descending (`SF:693-703`). Header tooltip in manual mode: "Sort (LeftClick = Ascending, RightClick = Descending)", empty otherwise (`SF:1056-1060`). **Quirks:** values are strings, so the sort is lexical (10 before 9), and clicking the corner cell throws. **Mac:** numeric sort; left click ascending, right click descending as upstream, plus the normal header toggle.
- [ ] **F68 Delete key.** Deletes every row with a selected cell (`SF:1191-1195`). Not gated by the row count, so the last row can go. **Quirk:** the `KeyPress` handler compares with `(char)Keys.Delete`, which is 46, the `.` character (`SF:1185-1189`), so a `.` that reaches the grid (on the read-only Start cell or a checkbox cell, where typing doesn't start an edit) deletes rows too. **Mac:** Delete key only; don't port the `.` behaviour. Recorded in the register.
- [ ] **F69 Add universe button** (add icon, top of the vertical strip right of the grid, `SFD:241-250`). Adds a row: universe = highest in table + 1, size = last row's size or `510` if empty, On checked (`SF:1107-1115`). Scrolls to it, enables Delete (`SF:1118-1122`). **Quirks:** at universe 64000 it happily adds 64001; the Start cell is integer 0 in manual mode, not a string, so the OK checks throw on it unless edited first (`SF:345, 956, 1115`), and clicking into it and away fails F62's range check; setting `Row.Selected` does nothing in cell-select mode (framework behaviour), so the selection really stays where it was. **Mac:** apply F70's limit message; new manual row's Start is previous start + previous size; the new row is selected.
- [ ] **F70 Add multiple universes button** (table_add icon, below F69, `SFD:230-239`). Max is `64000 - highest`; if below 1: "No additional universes can be added. Universe numbers must be between 1 and 64000 inclusive.", title "Add Universes", info icon (`SF:1087-1093`). Otherwise a number prompt titled "Add Universes", prompt "Number of universes to add.", default 1, range 1 to max, then adds that many (`SF:1095-1104`). **Mac:** Rust adds them in one operation.
- [ ] **F71 Delete universe button** (delete icon, below F70, `SFD:219-228`). Deletes every row with a selected cell (`SF:1146-1161`). Enabled when row count > 1 (`SF:131, 235, 1159`). **Quirk:** F69 always enables it (`SF:1122`), so right after F72, or after adding to an empty table, it's enabled with one row and can delete the last one. Zero rows then pass every OK check except F83's total. **Mac:** enabled when a row is selected and more than one row exists.
- [ ] **F72 Delete all universes button** (table_delete icon, below F71, `SFD:331-339`). Alert "Are you sure you want to delete all universes?", title "Delete All Universes?", info icon, YES/NO. YES clears all and adds one default row via F69: universe 1, start 1 (0 in manual mode, F69), size 510, On (`SF:1163-1178`). **Mac:** destructive verb "Delete All", then "Cancel".
- [ ] **F73 Icons for the strip.** No tooltips upstream. **Mac:** SF Symbols `plus`, `plus.rectangle.on.rectangle`, `minus`, `trash`; `.help` and accessibility labels "Add Universe", "Add Multiple Universes", "Delete Universe", "Delete All Universes". This text is new, because upstream shows only icons.
- [ ] **F74 "Manually manage start values." checkbox.** Advanced tab, first (`SFD:194`). Checked means manual. Inverse of `AutoPopulate` (`SF:175-179`). Default unchecked (`E131ModuleDataModel.cs:12`). Persists: `AutoPopulate`.
- [ ] **F75 "Display ALL Warnings/Errors and wait For OK" checkbox.** (`SFD:114`). Default off. Persists: `Warnings`. Controls F91.
- [ ] **F76 "Gather statistics and display at end of session" checkbox.** (`SFD:123`). Default off. Persists: `Statistics`. Controls F92.
- [ ] **F77 "Transmit blind data." checkbox.** (`SFD:299`). Default off. Sets the options byte to 128 (`E131Framing.cs:36-39`). Persists: `Blind`.
- [ ] **F78 sACN priority.** Numeric up/down then label "sACN priority." (`SFD:280, 286-290`). Range 0 to 200, step 1, default 100. Persists: `Priority`.
- [ ] **F79 Max Repeat Count.** Centered two-character text box, default 0 (`SFD:129, 133`), label beside it, exact lines: "Max Repeat Count", blank, "Set to 0 to send all events (even 0s)", "to each universe, Set to >0 to stop", "repeating frames after N duplicates", "are sent." (`SFD:144`). Valid 0 to 99. Invalid, including empty: beep and focus stays, no message (`SF:410-429`). This also blocks Cancel upstream (F66). Persists: `EventRepeatCount`.
- [ ] **F80 Max Suppress Count.** Same control, default 0 (`SFD:170, 174`), label: "Max Suppress Count", blank, "Only used if Max Repeat Count", "is not  0. Set to >0 to allow every", "Nth duplicate frame in a universe", "to go out." (`SFD:185`). **Quirk:** validator allows 0 to 10000 (`SF:440`) but `MaxLength` is 2, so 99 is the real maximum. Persists: `EventSuppressCount`.
- [ ] **F81 Repeat and suppress effect, for reference.** See `OP:760-797` and the test table in "Tests worth writing".
- [ ] **F82 Cancel.** Closes with no validation; nothing changes (`SF:880`). `Setup` still returns true (`OP:244`), so the tree refreshes.
- [ ] **F83 OK, first check: channel count.** Only when the controller has outputs (`SF:883`). All rows, active or not, in order, collecting: `Universe {u} start channel exceeds controller output count of {n}` when start > n; `Universe {u} {start} + {size} exceeds controller output count of {n}` when start + size - 1 > n; then if the size total differs: `Total universe channel count of {t} does not match the controller output count of {n}.` blank line `Check your universe setup or adjust the amount of output channels to match.` Joined by blank lines (`SF:888-933`). Alert title "Universe Channel / Output count conflict.", warning icon, OK/CANCEL. CANCEL keeps the dialog open; OK continues (`SF:935-944`). **Mac:** "Save Anyway" and "Go Back".
- [ ] **F84 OK, second check: overlap.** Any two rows (all rows, active or not) whose start ranges overlap (`SF:951-977`): alert "The start values seem to be setup in an unusual way. You are sending identical lighting values to multiple sACN outputs. The start value column refers to where a given universe starts reading values in from the list of output channel data from Vixen. For example, setting Universe 1's start value to 5 will map Channel 1 in Universe 1 to output channel #5 in the Vixen controller setup. Would you like to review your settings?", title "Warning", warning icon, OK/CANCEL (`SF:964`). **Quirk:** OK sets `e.Cancel = false` and returns, which **closes and saves**, skipping the remaining checks; CANCEL carries on to the remaining checks and closes if they pass (`SF:966-972`). Neither lets you review. **Mac:** "Review Settings" keeps the dialog open; "Continue" goes on to F85.
- [ ] **F85 OK, third check: duplicate universes in this controller.** Any two rows with the same universe: "Universe numbers must be unique.", title "Error", error icon; stays open (`SF:982-995`).
- [ ] **F86 OK, fourth check: other controllers.** For each other **running** E1.31 instance (`SF:999-1033`): if not (both unicast and different) and not (both multicast and different), any shared universe number gives `Universe {u} already exists on another controller transmitting to the same device or network. Please configure a different universe to prevent hardware errors.`, title "Existing Universe", exclamation icon; stays open. **Quirk:** one unicast and one multicast count as a conflict. Keep it: harmless and safer.
- [ ] **F87 Order of checks.** F83, F84, F85, F86. Rust returns them as steps so the UI can show them one at a time and resume.
- [ ] **F88 OK result.** Copies options; replaces `Universes`, `Unicast` and `Multicast` (`OP:194-206, 215-226`). Each new entry is `UniverseEntry(i, active, universe, start - 1, size, unicast, multicast)`, so it also carries the destination in its deprecated per-universe members, with `Ttl` 64 and zero counters (`OP:225`, `UniverseEntry.cs:68-90`); these are saved and must round-trip. Renames outputs: all to `Output #{i+1}`, then each channel in a row's range to `#{x+1} {universe}-{channel}: Multicast`, or `: {unicast}`, where channel is 1-based within the universe (`OP:211-212, 227-234`). **Quirk:** then always calls `Start()`, so OK on a stopped controller starts it, and the `running = false; if (running) Stop();` two lines above mean old sockets are never closed (`OP:188-192, 238`). **Mac:** stop and restart only if it was running.
- [ ] **F89 Output names on output count change.** `NameOutputs` runs from `OutputController.UpdateOutputNames` (`Vixen.Core/Sys/Output/OutputController.cs:300-305`) with the same format. **Quirk:** it computes the channel from zero-based `Start` with the same `+ 2`, so universe 5's first channel reads `5-2` instead of `5-1` (`OP:125-135`). **Mac:** one Rust function, 1-based, for both.
- [ ] **F90 Old-version import message.** At start, if both destinations are null and the first universe has its own: copies them and shows once per run: "The E1.31 plugin is importing data from an older version of the plugin. Please verify the new Streaming ACN (E1.31) configuration.", title "Vixen 3 Streaming ACN (E1.31) plugin" (`OP:379-396`). **Quirk:** throws on an empty universe list (`OP:380`). **Mac:** guard it. Never block the engine thread on an alert.
- [ ] **F91 Startup warnings.** Missing interface once per run: "The Streaming ACN (E1.31) plugin could not find one or more of the multicast interfaces specified. Please verify your network and plugin configuration.", same title (`OP:488-506`). With F75 on and collected messages: scrolling message box, heading "The following warnings and errors were detected during startup:", title "Startup Warnings/Errors", exclamation icon, OK (`OP:599-615`). The only collected text is `No IP On Multicast Interface: {name} - {InfoToText}` where InfoToText is `Row {r}: Univ={u} Start={s+1} Size={z}`, plus ` Unicast`, ` Multicast`, and ` TTL={ttl}` (`OP:553-554`, `UniverseEntry.cs:115-141`). `{r}` is `RowNum`, which isn't saved: the 0-based row since the last OK, or 0 for every row after a load (`UniverseEntry.cs:238`). **Mac:** the 1-based row. **Quirk:** an interface with no IPv4 address still counts as a clean start, with a socket and no endpoint, so the first send throws (`OP:551-555, 586-595, 794`). **Mac:** that universe sends nothing and start isn't clean. Other failures only log, including "Couldn't connect to use nic {id} for multicasting." (`OP:492`), "Nic {id} is available for multicasting bur currently down." (`OP:526`) and "E1.31 plugin failed to start due to unassigned destinations. This can happen with newly created plugin instances that have yet to be configured." (`OP:581`).
- [ ] **F92 Session statistics.** With F76 on, on Stop: scrolling message box, heading "Plugin Statistics:", title "J1Sys E1.31 Vixen Plugin", info icon, OK. Text: any collected messages, then `Events: {n}`, `Total Time: {ticks} Ticks; {ms} ms`, then for each active universe a blank line and `Row {r}: Univ={u}  Packets={p}  Slots={s}` (`OP:303-329`, `UniverseEntry.cs:199-215`). `{r}` as in F91. **Quirks:** the stopwatch is never reset, so `_totalTicks` sums cumulative times (`OP:708, 799-801`), and `ms` is just the milliseconds component (`OP:312`). Stop without a Start throws with this option on, since `_messageTexts` is still null (`OP:305`). **Mac:** measure each update properly, show total ms. Scrolling box is a sheet, F93. A stop that isn't user-initiated (quit) must not block on it: Rust queues it (`take_messages` on the module's shared state).
- [ ] **F93 Scrolling message box (J1MsgBox).** Heading text, read-only scrolling text, buttons, centered, max 600 x 400 (`J1Sys/J1MsgBox.cs:115-346`). Upstream's other uses are dead. **Mac:** a small shared sheet with `Text`, a scrolling selectable `Text`, and OK.
- [ ] **F94 Unreachable upstream, not ported.** `OkButtonClick` and its message "Your configurations contains active entries that may cause run time errors..." (`SF:477-597`, not wired in `SFD`); the row context menu with insert, move up, move down (`SF:604-637, 828-875`: the strip has no items); `DestinationContextMenuStripOpening` (`SF:404-408`); the empty `mainMenu` and `mIHelp` (`SFD:94-105`, never added to the form's controls); `AboutBox` (logo, "Product Name", "Version", description box, `&OK`) and `Help.ShowSysClick` (`Controller/E131/Help.cs:44-116`, no caller); the empty handlers `UnivDgvnCellEnter`, `UnivDgvnInsertRow` (`SF:664-670, 1180-1183`), `UnicastForm_Load` and `ipTextBox_TextChanged` (`UF:30-36`); `UnivDgvnCellMouseEnter`, which only feeds the dead menu (`SF:708-711`); and `UnivDgvnDeletedRow` (`SF:1141-1144`), because the grid's own row deletion needs row selection, which cell-select mode never has (framework behaviour). Record in `docs/parity`.

### DDP

`P` = `Controller/DDP/`. TypeName "DDP", TypeId `110B5FE9-CD4D-4E73-BB5B-67552A1AD6D1`.

- [ ] **F95 DDP Configuration window.** Title "DDP Configuration" (`P/DDPSetup.Designer.cs:151`). Group "Configuration" (`:79`). OK then Cancel bottom right. **Quirk:** `Setup` sets `_isRunning = false`; OK leaves it false, so a running controller stops sending until restarted; Cancel sets it true even if the controller was stopped (`P/DDP.cs:76-86`). **Mac:** restore the prior state both ways.
- [ ] **F96 Mode radios and field.** Radios "IP Address" (default) and "Host Name" on one row (`P/DDPSetup.Designer.cs:91, 102`). Below: one label and one field, swapped by mode. Label "IP Address:" or "Host Name:"; the other field is disabled and hidden (`P/DDPSetup.cs:64-80`). At open: `HostName` set selects Host Name mode with that text; otherwise IP mode with `Address`, or `127.0.0.1` if it doesn't parse (`:28-44`). IP field is blank in Host Name mode. Host field's default is `myhost.local` (`Designer:133`). Default `Address` is `127.0.0.1` (`P/DDPData.cs:29`).
- [ ] **F97 OK.** Host Name mode: trims, resolves to IPv4 (first IPv4 result, `Vixen.Common/Utilities/HostNameResolver.cs:21-41`), with a wait cursor. Failure: "Could not resolve host name '{name}' to an IP address. Check the spelling and your network connection, or switch to IP Address mode.", title "DDP Setup Error", error icon; focus returns to the field; stays open (`P/DDPSetup.cs:89-117`). Success stores address and host name. An empty host name takes the same failure path, with `''` in the text (`HostNameResolver.cs:24-27`). IP mode: stores the parsed address and clears host name; `IPAddress.TryParse` also takes shorthand like `10.1` (stored as `10.0.0.1`) and IPv6 (stored, then connecting fails with only a log, `P/DDP.cs:234-241`). **Quirks:** text that doesn't parse silently becomes `127.0.0.1` (`P/DDPSetup.cs:121-126`); only `SocketException` is caught while resolving, so a name .NET rejects outright, such as one over 255 characters, throws out of OK (`HostNameResolver.cs:30-37`). **Mac:** resolve off the main thread with a progress indicator; in IP mode disable OK unless the text parses as IPv4 in dotted-quad form, and treat every resolver error as F97's failure text. Persists: `Address`, `HostName`.
- [ ] **F98 Cancel.** Discards (`P/DDP.cs:85-86`).
- [ ] **F99 Resolve on connect, for reference.** Every connection re-resolves the host name, updates `Address` if it changed, and on failure logs and uses the last address (`P/DDP.cs:217-232`). **Mac:** never on the main thread.

### Renard

`R` = `Controller/Renard/`. TypeName "Renard", TypeId `F66130A6-FF8A-48a7-BF1E-FAC6CEC2FC2C`.

- [ ] **F100 Configure opens Serial Port Config directly.** F7 with the module's port; on OK stores port name, baud, data bits, parity, stop bits, recreates the port and returns true (`R/Module.cs:39-56`). Load defaults: `WriteTimeout` 0 becomes 500, `ProtocolVersion` 0 becomes 1 (`:64-65`). Port opens with DTR and RTS on (`:131-132`). Open failures retry as Generic Serial (`:145-184`), except that any other exception is swallowed without even a log (`:163-183`). The same F36 quirk applies: Stop doesn't cancel the retry. Neither `WriteTimeout` nor `ProtocolVersion` has a control. Writes time out after `WriteTimeout` ms and aren't caught (`:101, 128`); `_WaitForBufferRoom` sleeps in 10 ms steps on the output thread (`:105-110`). **Mac:** a write timeout is logged and the frame dropped.
- [ ] **F101 Unreachable, not ported.** `R/SetupDialog.cs` ("Renard Configuration", group "Setup", disabled "Protocol version:" list of 1 and 2, "Port Setup", OK, Cancel) is never instantiated. Protocol 2 is commented out, and asking for it throws "Renard module using an unsupported protocol version (2)" (`R/ProtocolFormatterService.cs:36`, `R/Protocol2.cs:7`).

### DMX Pro (Enttec USB Pro)

`X` = `Controller/DMXUsbPro/`. TypeName "DMX Pro", TypeId `8ABF27A7-3156-4F4C-9D43-ADA45CDD6D0C`.

- [ ] **F102 Configure opens Serial Port Config directly.** On OK writes port name, baud, `Partity` (the member really is misspelled), data bits, stop bits (`X/Module.cs:26-48`, `X/Data.cs:17`). **Quirk:** it passes the runtime `_serialPort`, null until started and after stopping, so "Current Configured Port" reads None and defaults replace saved values (`X/Module.cs:28, 80-89`). OK doesn't restart anything: a running sender keeps its old port open until the next Stop, and the new settings apply at the next Start (`X/Module.cs:30, 50-60, 104-120`). **Mac:** build the current settings from the data; apply with `controller_apply`, which restarts only if running.
- [ ] **F103 Output limit, for reference.** First 512 outputs only (`X/DmxUsbProSender.cs:56`). No retry on open failure, only a log (`:65-72`). **Quirk:** `Stop` before any `Start` throws (`X/Module.cs:82`). Rust guards it.

### DMX Open (Enttec Open DMX)

`O` = `Controller/OpenDMX/`. TypeName "DMX Open", TypeId `7911568a-8eda-4d1d-9e72-be41d7a843e4`. `OutputLimit` 512 (`O/OpenDMXInstance.cs:23`), which caps the count prompt when adding the controller (`Vixen.Common/Controls/ControllerTree.cs:806`, `docs/spec/display-setup.md` F77). The later "Channel Count" prompt has no cap (`ControllerTree.cs:882-894`, display-setup F88), and a larger count throws "Output count greater than 512" (`O/OpenDMXInstance.cs:33-34`). **Mac:** that prompt caps at `OutputLimit` too. It's the only module with a limit.

- [ ] **F104 OpenDMX Setup window.** Title "OpenDMX Setup" (`O/SetupDialog.Designer.cs:91`). One device combo, OK and Cancel bottom right. Opening enumerates FTDI devices (`O/SetupDialog.cs:17`, `O/OpenDMX.cs:201-250`).
- [ ] **F105 Device combo.** Items `{Description} - {SerialNumber} - {Id}` (`O/SetupDialog.cs:23`). Selects the saved device matching serial, id and description, otherwise the first (`:27-44`). No devices: empty. **Quirk:** the combo style isn't DropDownList (`Designer:70-74`), so typing clears `SelectedItem` and OK throws. **Mac:** `Picker`. **Mac mapping:** no D2XX driver, it conflicts with the system FTDI driver. List `/dev/cu.*` with USB vendor 0x0403. `Description` is the USB product string, `SerialNumber` the USB serial, `Id` the decimal of `(vid << 16) | pid`, so a saved Windows device still matches.
- [ ] **F106 OK.** With devices: stores the selection. Without: leaves it (`O/SetupDialog.cs:48-55`). Then stops and starts the port (`O/OpenDMXInstance.cs:60-64`). **Quirk:** that throws if the controller never started; start failures throw "No devices found to open." or "Failed to open OpenDMX device. Error from Driver: {status}" (`O/OpenDMX.cs:65-67, 73`). **Mac:** restart only if running; show those as alerts. `Setup` always returns true (`O/OpenDMXInstance.cs:66`).
- [ ] **F107 Cancel.** Discards.
- [ ] **F108 Start fallback, for reference.** Saved device missing: uses the first, and writes it back to `_data.Device` (`O/OpenDMX.cs:44-53`). Transmit: 250000 baud, 8N2, break, 513 bytes, then 25 ms sleep (`:15-18, 131-156`). **Quirk:** `Stop` sets `_done` and nothing ever clears it, and `Start` reuses the same object, so after any Stop, or F106's restart, the writer thread exits at once and nothing goes out again until the module is recreated (`O/OpenDMX.cs:12, 90, 134`, `O/OpenDMXInstance.cs:72-77`). **Mac:** a restart resumes sending. **Mac:** `IOSSIOSPEED` for 250000, `TIOCSBRK` and `TIOCCBRK` for break.

### Elexol Ethernet I/O 24

`E` = `Controller/ElexolEtherIO/`. TypeName "Elexol Ethernet I/O 24", TypeId `2565E617-89DB-4761-BE8D-4D5574CAFBA9`. Where an entry names no file, a bare `:N` means `E/SetupDialog.cs`.

- [ ] **F109 Window.** Title "Elexol USB I/O 24 Configuration" (`E/SetupDialog.Designer.cs:189`), USB though it's Ethernet; verbatim. Group "Ethernet Settings" on top, group "Advanced" below, OK then Cancel (`:60, 119`).
- [ ] **F110 IP Address.** Label "IP Address", text box. Saved value, or `127.0.0.1` when none (`E/SetupDialog.cs:22-29`); the designer's `10.10.10.10` is overwritten. Persists: `Address`.
- [ ] **F111 Port.** Label "Port", text box. Saved value, or `2424` when zero or less (`:31-38`). Persists: `Port`.
- [ ] **F112 Test button.** "Test", right of the fields, spanning both rows (`Designer:83-88`). Sends bytes `0x60 0x69` by UDP, waits up to 1 second for a reply (`E/SetupDialog.cs:98-118`). Results, all titled "Error" with error icon: "No Reply."; "Connection Successful!"; "Device replied, but with incorrect data:" newline `[0x60][0x69]`; on exception "An error occured testing the device" newline, one space, and the message (`:119-158`). A bad IP or port text lands in that exception path (`:104-105`). **Quirks:** success is titled "Error"; "incorrect data" prints what was sent, not received (`:145`). **Mac:** run off the main thread; alert title "Test Connection" with the same texts and show the received bytes. Blocks nothing.
- [ ] **F113 Minium 'ON' Intensity.** Label "Minium 'ON' Intensity (0-255):", slider 0 to 255, large step 16, ticks every 16, value text centered below (`Designer:129, 142-148, 134-138`). Default 1 (`E/SetupDialog.cs:13, 40-45`). Live text update (`:48-53`). An output is on when its value is strictly greater (`E/ElexolEtherIOModule.cs:69`). Persists: `MinimumIntensity`. **Quirks:** values 0 and 1 don't update the runtime threshold when lowering, since it's only assigned when above 1 (`:123-127`); a saved 0 reopens as 1, and OK then saves 1 (`E/SetupDialog.cs:40-45`). **Mac:** always assign; show the saved value.
- [ ] **F114 OK.** IP parses: stores IP, intensity, port, recreates, reconnects, closes (`E/SetupDialog.cs:81-88`, `E/ElexolEtherIOModule.cs:93-103`). Not: "Invalid IP Address.", title "Error", error icon; stays open (`E/SetupDialog.cs:89-95`). **Quirks:** `int.Parse` on the port throws on bad text (`:85`); OK opens a socket and sends the 15 output-mode and zero bytes even when the controller is stopped (`E/ElexolEtherIOModule.cs:101, 140-158`). **Mac:** "Invalid Port." for anything outside 1 to 65535. This text is new. Connect only if running.
- [ ] **F115 Cancel.** Returns false (`E/ElexolEtherIOModule.cs:105`).

### RDS

`K` = `Controller/RDSController/`. TypeName "RDS", TypeId `692B66B9-E831-4F39-BB73-7653FA62B053`.

- [ ] **F116 Launcher Commands and RDS Configuration window.** Title as named (`K/SetupForm.Designer.cs:387`). Left: "RDS Hardware" group, "Test Interface" group below it. Right: picture, then "Options" group, "Save" button. Bottom: status strip. **Quirk:** every control writes straight into the module's data as it changes, and the only button is Save, so closing the window keeps edits (`K/SetupForm.cs:21, 52-177`). **Mac:** staged, Save bottom right as upstream, plus Cancel because a sheet has no close box.
- [ ] **F117 Hardware radios.** MRDS192, MRDS1322, HTTP(S), V-FMT212R in a 2 x 2 layout (`Designer:230, 241, 252, 265`, positions `:226, 237, 248, 260`). From data (`K/SetupForm.cs:35-48`). Data default is HTTP, enum 0 (`K/Data.cs:56`). MRDS1322 sets `ConnectionMode` 1, MRDS192 sets 0 (`K/SetupForm.cs:62, 69`). Nothing reads `ConnectionMode`, and HTTP and V-FMT212R leave it at whatever the last MRDS radio set. **Quirk:** each radio's handler also runs for the radio being unchecked, before the new one's (framework order), and it writes `HardwareID` regardless of which is checked. The end state is the new radio's, except what only the old one's handler changes, as in F118. At open with HTTP or V-FMT212R saved, the designer's checked MRDS192 unchecks first, so `ConnectionMode` is forced to 0. Persists: `HardwareID`, `ConnectionMode`.
- [ ] **F118 Field rules per hardware** (`K/SetupForm.cs:73-95`). MRDS192 and MRDS1322: test text max 8; URL, username, password, auth check disabled. V-FMT212R: test text max 64, URL and credentials on, auth forced on and disabled, URL replaced by the default unless it already ends `?action=update_rt&update_rt={text}`, ignoring case, serial port disabled (`:54, 86-87`). HTTP: test text max 128, all on. The URL and credentials are the only controls these rules disable for MRDS; the serial port stays enabled for HTTP. `MaxLength` limits typing only, so longer text already in the box stays. The designer starts with test text max 64 and the auth, user and password controls disabled (`Designer:138, 150, 160, 169, 179, 294`). **Quirk:** when saved hardware is MRDS192 the radio is already checked in the designer, so no event fires and these rules don't run at open (`Designer:259`). **Mac:** run them at open. Leaving V-FMT212R keeps auth on (`:84`).
- [ ] **F119 Serial Port.** Label "Serial Port", drop-down of port names, sorted (`K/SetupForm.cs:18-19, 30-33`, `Designer:123, 127`). **Quirks:** selecting parses `COM{n}` into `PortNumber` (`:176`) and nothing ever stores `PortName`, so the selection isn't restored at open. **Mac mapping:** there's no `COM` number. Store the path in `PortName` and restore from it. `Send` uses `PortName`, and for imported data falls back to reporting `COM{PortNumber}` unavailable. `PortNumber` round-trips untouched. Serial list follows the device mapping above.
- [ ] **F120 URL.** Label "URL", full-width field (`Designer:190-221`). Default `http://127.0.0.1:8080/?action=update_rt&update_rt={text}` (`K/SetupForm.cs:29`). Persists: `HttpUrl`.
- [ ] **F121 Macro hint.** Labels "Macro Fields:" and "{text}, {time}" (`Designer:203, 212`).
- [ ] **F122 "HTTP(s) Requires Authentication" checkbox.** (`Designer:143`). Persists: `RequireHTTPAuthentication`.
- [ ] **F123 UserName and Password.** Labels "UserName", "Password"; text boxes, `MaxLength` 8 each, password shown in clear (`Designer:165, 155, 171, 181`). Persist: `HttpUsername`, `HttpPassword`, plain text in `ModuleStore.xml`. **Mac:** `SecureField`; same storage so a profile round-trips.
- [ ] **F124 Test Interface group.** Label "Macro value {text}", text box, "Send" (`Designer:279, 369, 288`). Send: `Module.Send(data, text, force: true)`. Status "Data Sent" when it returned true, otherwise "Data Not Sent" (`K/SetupForm.cs:105-112`). **Quirks:** serial always returns false even when it worked, HTTP returns true before the request finishes (`K/Module.cs:41, 76`); a serial failure throws out of the click, since nothing catches `RdsSerialPort`'s `Open` (`K/Module.cs:36-40`, `K/RdsSerialPort.cs:13, 41`). **Mac:** report the real result, and "Data Not Sent" on failure. Send uses the sheet's draft, since upstream's data is already live when it sends.
- [ ] **F125 Status strip.** One label, initial text "DIYC" (`Designer:326`).
- [ ] **F126 Picture.** RDS logo top right, disabled, stretched (`Designer:301-306`). From `K/SetupForm.resx`.
- [ ] **F127 Options group.** "BiDirectional" and "Slow" checkboxes (`Designer:66, 87, 76`). Slow selects 2400 instead of 19200 baud (`K/Module.cs:36`). `BiDirectional` is stored and never read. Persist: `BiDirectional`, `Slow`.
- [ ] **F128 Hidden group.** "Launcher Options" with "Hide Launched Windows", `Visible = false` (`Designer:347-348, 358`). Value is loaded and saved (`K/SetupForm.cs:49, 116`) and used when launching. Don't show; keep the data.
- [ ] **F129 Save.** "Save", closes with OK (`Designer:330-336`). `Setup` returns true; closing otherwise returns false.
- [ ] **F130 Runtime, for reference.** Each output's string command splits on `|`: `RDS|{text}` sends unless unchanged; `LAUNCHER|{exe},{args}` launches on change (`K/Module.cs:113-144`). Serial send opens, writes `FE 1F FF`, closes, opens, writes `FE 20`, the text, space padding to 64, `FF`, closes (`K/RdsSerialPort.cs:8-60`). HTTP GET replaces `{text}` with the URL-encoded text and `{time}` with the encoded short time (`K/Module.cs:48`). On failure clears the last text so it retries (`:64, 73`). The last-text memory is static, so it's shared by every RDS controller (`:18`). A command type is matched ignoring case (`:126`). A null or non-string command clears that output's remembered value, so the same launcher command fires again after a gap (`:118-121`). **Quirks:** `args[1]` throws when there's no comma, and anything after a second comma is dropped (`:136-138`); `Split('|')[1]` throws with no `|` (`:128, 136`); `Convert.ToByte` throws on a character above 255 (`K/RdsSerialPort.cs:49`). All of that happens on the output thread. **Mac:** log and skip the command.

### Launcher

`L` = `Controller/LauncherController/`. TypeName "Launcher", TypeId `1C4BFD64-3172-43A7-A7FC-0A77C88B0489`.

- [ ] **F131 Launcher Controller Configuration window.** Title as named (`L/SetupForm.Designer.cs:65`). One checkbox, one Save button below it (`:41, 51`). Same write-through quirk as F116 (`L/SetupForm.cs:18-21`). **Mac:** staged, Save plus Cancel.
- [ ] **F132 "Hide Launched Windows" checkbox.** Default off. Persists: `HideLaunchedWindows`. **Mac meaning:** windows of an app bundle launch hidden with `open -g -j`; a plain executable never shows a window, so it doesn't matter.
- [ ] **F133 Launch, for reference.** Commands `LAUNCHER|{exe},{args}`, on change only, where a null or non-string command in between counts as a change (`L/Module.cs:53-70`). Same `args[1]` and `|` quirks as F130 (`:71-72`). Only launches if the file exists, otherwise logs `File Not found to Launch: [{exe}]`. Runs on a background task and logs exit code and time (`L/Module.cs:17-47, 51-77`). **Mac:** accept `.app` bundles too (`File.Exists` would be false for a folder). A saved `C:\...` path just logs not found.

### Debugging Values

`B` = `Controller/DebugController/`. TypeName "Debugging Values", TypeId `f32bda77-28f0-46c4-a6bd-ba0ba73a1c13`. No setup and no data (`B/DebugController.cs:10-43`).

- [ ] **F134 Debug Controller Output window.** Title "Debug Controller Output" (`B/DebugControllerOutputForm.Designer.cs:66`). 538 x 405. Created on Start, destroyed on Stop (`B/DebugController.cs:62-84`). Multi-line text filling the window (editable upstream, though overwritten each update, with no scroll bars, so long frames run off the bottom; the Mac view scrolls). "Verbose" checkbox at bottom right, default off, not saved (`B/DebugControllerOutputForm.Designer.cs:49-55`, `B/DebugControllerOutputForm.cs:26, 93-96`). One line per output: `[{m:ss.fff}] [{n}]: {kind}: {value}` where kind is `8-bit`, `16-bit`, `32-bit`, `64-bit`, `Color`, `String`, or the whole value is `Unknown command type`; null commands read `null` and show only in verbose. Blank line at the end. If nothing to show, text is left alone (`:42-82`). The time comes from `Vixen.Core/Sys/Execution.cs:120`, and reads `0:00.000` when nothing is playing. A colour value prints as .NET's `Color.ToString()`, such as `Color [A=255, R=255, G=0, B=0]`. 8-bit evaluator, `DiscardExcessCombinator` (`B/DebugController.cs:115-128`). **Quirk:** closing the window by hand makes the next update throw, since `BeginInvoke` runs on a disposed form (`B/DebugControllerOutputForm.cs:81`). `AppendText` is always false and the `_timer` and `_lastUpdateMs` fields do nothing (`:27-32, 40`). **Mac:** read-only `NSTextView`, text built in Rust, refreshed no faster than the screen. Closing it by hand leaves the controller running, as F46; next Start reopens it.

### Output filter: Color Breakdown

`C` = `OutputFilter/ColorBreakdown/`. TypeName "Color Breakdown", TypeId `ab38a16f-0de1-4f6e-a8c0-ae5b20d347e0`. Reached by F2. `ColorBreakdownConfig` and its `template()` exist in `crates/vixen-output/src/filters/color_breakdown.rs`. Where an entry names no file, a bare `:N` means `C/ColorBreakdownSetup.cs`.

- [ ] **F135 Color Breakdown Filter Setup window.** Title (`C/ColorBreakdownSetup.Designer.cs:195`). Resizable, minimum 464 x 548 (`:191`). Top to bottom: instructions, template row, colour list, two checkboxes, Add Color at bottom left, OK and Cancel at bottom right (`:45-170`).
- [ ] **F136 Instructions.** Two lines: "Select a pre-configured template from the drop-down box, or build a", "custom color breakdown filter below." (`Designer:51, 61`).
- [ ] **F137 Template picker.** Label "Template:". List RGB, RGBW, RGBY, RGxB, GRBW, GRWB; RGB selected (`C/ColorBreakdownSetup.cs:41-48`).
- [ ] **F138 Apply Template.** Replaces the whole list, no confirmation (`:80-133`). RGB: Red, Green, Blue. RGBW: plus White. RGBY: plus Yellow. RGxB: Red, Green, `[empty]` (black), Blue. GRBW: Green, Red, Blue, White. GRWB: Green, Red, White, Blue. Green is `Color.Lime`. Unknown template: "Error applying template: Unknown template.", title "Error" (unreachable).
- [ ] **F139 Colour list.** Scrolling list, one row per item, top to bottom (`Designer:131-146`), loaded from data (`C/ColorBreakdownSetup.cs:29-33`).
- [ ] **F140 Row colour swatch.** 60 x 40 at the left (`C/ColorBreakdownItemControl.Designer.cs:38-40`). Click opens the colour picker, OK updates it (`C/ColorBreakdownItemControl.cs:53-62`). **Quirk:** a colour from the picker is a value colour, and mixing compares with `Color.Equals`, so a picked pure red isn't `Color.Red` and lands on the blue channel (`C/RGBColorBreakdownMixingFilter.cs:30-41`; the Rust port keeps that at `crates/vixen-output/src/filters/color_breakdown.rs:244`). **Mac:** when a picked colour is exactly FF0000, 00FF00, 0000FF or FFFFFF store it as the known colour, so the runtime stays faithful and the user gets what they picked.
- [ ] **F141 Row name.** Label "Name:", text box (`Designer:46, 59`). New rows read "New Color" (`C/ColorBreakdownItemControl.cs:14`). Names label the filter's outputs in patching (`C/Outputs/ColorBreakdownOutputBase.cs:89-92`). Persists: `Name`.
- [ ] **F142 Row Delete.** "Delete" button, removes the row, no confirmation (`Designer:67`, `C/ColorBreakdownSetup.cs:56-72`).
- [ ] **F143 Add Color.** Appends White, "New Color" (`C/ColorBreakdownSetup.cs:51-54`, `Designer:127`).
- [ ] **F144 "These colors should be mixed to produce the input color" checkbox.** (`Designer:156`). Persists: `MixColors`. **Quirk:** writes straight to data on change, and Cancel doesn't undo it, though outputs aren't rebuilt (`C/ColorBreakdownSetup.cs:135-138`). **Mac:** staged. Also: with mixing on and exactly 4 items, every item must be red, lime, blue or white, otherwise upstream hits an empty delegate and throws at runtime (`C/ColorBreakdownModule.cs:100-111`, `C/RGBWColorBreakdownMixingFilter.cs:88-110`). So RGBY and RGxB with mixing crash upstream. **Mac:** Rust never panics: an unsupported 4-colour item outputs 0. Show an inline note under the checkbox for this combination.
- [ ] **F145 "16 Bit Output (For Use with Course Fine Break Down)" checkbox.** (`Designer:168`). Persists: `_16Bit`. Same staged note.
- [ ] **F146 OK.** Stores the list and rebuilds outputs, returns true (`C/ColorBreakdownModule.cs:82-87`).
- [ ] **F147 Cancel.** Returns false.

### Output filter: Coarse Fine Breakdown

`F` = `OutputFilter/CoarseFineBreakdown/`. TypeName "Coarse Fine Breakdown", TypeId `7C1465DF-054C-4875-AB61-D7E7F5236897`.

- [ ] **F148 Coarse Fine Breakdown Setup window.** Title (`F/Setup/Views/CoarseFineBreakdownSetupView.xaml:6`). Width 420, height to content, no resize. Checkboxes on top, sliders below, OK and Cancel bottom right; OK is default and Cancel is the cancel button (`:25-32`).
- [ ] **F149 Fields.** "Enable default value mapping" checkbox (`F/Setup/Views/CoarseFineBreakdownSetupView.xaml:49`), default off, persists `EnableDefaultValueMapping`. "Live mode (apply changes immediately)" checkbox (`:55`), default off, not saved. "Resting coarse value (0-255)" and "Resting fine value (0-255)" sliders (`:62, 80`): 0 to 255, whole steps, click moves to point, tooltip shows the value; defaults 0; persist `RestingCoarseValue`, `RestingFineValue`. Validation "Resting coarse value must be between 0 and 255." and "Resting fine value must be between 0 and 255." block OK (`F/Setup/ViewModels/CoarseFineBreakdownSetupViewModel.cs:151-157`); sliders can't reach them, but Rust checks. The filter always has two outputs, coarse first, both named "Coarse-Fine Breakdown" in patching (`F/CoarseFineBreakdownModule.cs:151-155`, `F/CoarseFineBreakdownOutput.cs:105`).
- [ ] **F150 Apply rules.** With Live mode on, each valid change applies straight away, and turning it on applies the current values (`F/...ViewModel.cs:110-118, 189-198`). Cancel, or closing the window, restores the original if anything was applied live (`:180-187`, `F/CoarseFineBreakdownModule.cs:89-93`). OK applies unless the live state already matches (`:95-98`). Each apply rebuilds the two outputs, so **Mac:** swap configuration atomically; don't rebuild the graph per slider tick. Runtime resting value is `(coarse << 8) | fine`, used only when mapping is on and no value arrived (`F/CoarseFineBreakdownOutputConfiguration.cs:15`, `F/CoarseFineBreakdownOutput.cs:93-99`).

### Property: Color

`H` = `Property/Color/`. TypeName "Color", TypeId `bff34727-6b88-4f87-82b7-68424498c725`. In F151 to F161, where an entry names no file, a bare `:N` means `H/ColorSetupHelper.cs`.

- [ ] **F151 Color Configuration window (`ColorSetupHelper`).** Title (`H/ColorSetupHelper.Designer.cs:212`). HelperName "Color Handling". Bold heading "How do these items handle color?" (`:76`). Then three option blocks, OK and Cancel bottom right. Reached by F3 and F4, and after templates (F6).
- [ ] **F152 Radio "They are a single color, and do not change color at all. The color is:".** (`Designer:131`). Enables the swatch below (`H/ColorSetupHelper.cs:388`). No radio is selected by default, and OK is disabled until one is (`Designer:60`, `H/ColorSetupHelper.cs:393`). F3 and F6 preselect (`:305-337`).
- [ ] **F153 Single colour swatch.** Default RoyalBlue (`:26-27`). Click opens the colour picker. **Mac:** `ColorPicker`.
- [ ] **F154 Radio "They can be multiple colors, but the colors are independent: they do not mix.".** (`Designer:119`). Enables the colour set picker and Edit Colors (`:389-390`). Under it, "(eg. multiple strings of different colors wrapped around each item.)" (`Designer:142`).
- [ ] **F155 Colors picker.** Label "Colors:". Drop-down of colour set names in stored order, first selected, or the preset name (`:365-384`). A preset name that no longer exists stays on the first set. **Quirks:** with no colour sets left, selecting index 0 throws at load; a preset whose set name is null leaves nothing selected, and OK then throws (`:59, 378-381`). **Mac:** keep OK disabled while nothing is selected, when this radio is on.
- [ ] **F156 Edit Colors.** "Edit Colors" opens F162 and reloads the picker (`:396-402`, `Designer:85`). **Quirk:** the reload clears the selection and goes back to the first set or the preset, forgetting what the user had picked (`:368, 374-381`). **Mac:** keep the picked set when it still exists.
- [ ] **F157 Radio "They can be any color: they are full RGB and mix to make any color.".** (`Designer:107`). Enables Color Order (`:391`).
- [ ] **F158 Color Order picker.** Label "Color Order:". List: RGB, RBG, GBR, GRB, BRG, BGR, RGBW, GRWB; RGB selected (`Designer:171, 184`, `:28`). **Quirk:** reconfiguring doesn't read the existing order, so it opens on RGB, and overwriting replaces a GRB filter with RGB unless changed. Keep it; the implementer shouldn't guess.
- [ ] **F159 OK: properties.** Works on distinct leaves of the selection (`:78-79`). A leaf with no Color property gets one (`:108-111`). If any already has one, once only: "Some elements already have color properties set up. Should these be overwritten?", title "Color Setup", warning icon, YES/NO; NO skips those (`:95-105`). Sets type, single colour, set name (`:118-120`). Single leaves set name null; multiple leaves colour empty; full both (`:52-66`). **Mac:** "Overwrite" and "Skip".
- [ ] **F160 OK: filters.** For each leaf's data flow, walks to leaf outputs and existing breakdowns (`:136-138, 274-303`). Existing breakdown, once only: "Some elements are already patched to color filters. Should these be overwritten?", same title and buttons (`:151-163`). Dead-end outputs skip (`:165-168`). Otherwise a new breakdown is attached to that output and added (`:169-176`). Unless skipped, sets mix and items (`:178-249`): full colour mixes, one item per letter of the order, Red `Color.Red`, Green `Color.Lime`, Blue `Color.Blue`, White `Color.White`; multiple doesn't mix, one item per set colour named `c.Name`, which for value colours is lowercase ARGB hex like `ffff0000` (`:233`); single doesn't mix, one item with the default name "Unnamed" (`:242-244`). Existing `_16Bit` stays.
- [ ] **F161 Result report.** `Color Properties:  {a} added, {c} configured, {s} skipped. Color Filters:  {a} added, {c} configured, {s} skipped.`, title "Color Setup", info icon (`:263-268`). Two spaces after each colon. Not shown in silent mode (`:259`). **Quirk:** a breakdown with free outputs is listed once for itself and once per free output, so it can count several times (`:280-294`). Keep the counts. Cancel returns false with no change (`:40-42`). Silent mode (`:357`, used by F6's importer) skips the dialog and summary, but the two overwrite questions in F159 and F160 still appear (`:95-105, 151-163`).

### Colour Sets window

In F162 to F169, where an entry names no file, a bare `:N` means `H/ColorSetsSetupForm.cs`.

- [ ] **F162 Color Sets window.** Title "Color Sets" (`H/ColorSetsSetupForm.Designer.cs:220`). Left: "Defined Color Sets:" list with add and remove icon buttons below it. Right: group "Selected Color Set". OK at bottom right (`:79, 94, 179`). Sets are shared by all Color properties, in insertion order (`H/ColorStaticData.cs:60`). Defaults: RGBW (FF0000, 00FF00, 0000FF, FFFFFF), RGB (first three) (`:20-31`). Persists: type-level `ColorSets`; `vixen_io::library::{color_sets, set_color_sets}` exist.
- [ ] **F163 Defined list.** Single selection (`Designer:64`). Selecting shows the set on the right; nothing selected disables the right side and clears it (`H/ColorSetsSetupForm.cs:76-89, 52-61`). Enables remove. **Quirks:** switching selection silently drops unsaved edits to the previous set, and the remove button starts enabled at open with nothing selected, where it does nothing (`Designer:182-189`). **Mac:** keep the first, only the close prompt exists; disable remove until a set is selected.
- [ ] **F164 Add set.** Prompt "New Color Set name?", title "New Color Set". Existing name: "Color Set already exists.", title "Error". Otherwise creates an empty set and shows it (`:91-111`). Empty name isn't rejected here. **Mac:** disable the prompt's OK when empty.
- [ ] **F165 Remove set.** Removes at once, no confirmation (`:113-126`). Failure: "Error removing Color Set!", title "Error". Elements using a removed set log an error and read as no colours (`H/ColorProperty.cs:121-128`). Upstream raises no change event on removal (`H/ColorStaticData.cs:53-56`), so a property that had cached the set keeps showing it until reload. **Mac:** resolving by name applies the removal at once.
- [ ] **F166 Name field.** Label "Name:". Typing a name that exists makes the button read "Update Color Set", otherwise "Make New Color Set" (`:140-151`). Blank leaves it as is.
- [ ] **F167 Colours grid.** Label "Colors:". 5 columns, grows as needed (`Designer:129-142`). One swatch per colour; click edits (`H/ColorPanel.cs:21-29`). Add icon button adds white (`H/ColorSetsSetupForm.cs:134-138`), at the bottom right of the group (`Designer:110`). **Quirk:** there's no way to remove a colour from a set. Keep, and record it. The panel is a fixed 202 x 110 with no scrolling (`Designer:141`), so a big set runs out of room. **Mac:** `Grid` of `ColorPicker`s, in a scroll view.
- [ ] **F168 Update or Make New button.** Saves the panel under the name in the box, so a changed name makes a new set and keeps the old one (`:128-132, 182-203`). Empty name: "You must enter a name for the Color Set.", title "Name Requred" (verbatim).
- [ ] **F169 Closing.** OK or the close box: if the displayed set differs from what's stored, or its name is new: "Do you want to save changes to the displayed color set?", title "Save Changes?", error icon, YES/NO/CANCEL. YES saves (if that fails stays open), NO discards, CANCEL stays (`:205-229, 153-180`). **Mac:** "Save", "Don't Save", "Cancel". This is the one dialog that writes live: its changes hold even if the caller cancels, until Display Setup Cancel reloads.

**Unreachable, not ported:** `ColorSetupForm` ("Element Color", with "Single color:", "Multiple independent colors:", "Any color (ie. fully mixing RGB)", swatch, combo, "......", OK; `H/ColorSetupForm.Designer.cs`). Only `ColorModule.Setup` opens it (`H/ColorProperty.cs:61-69`), and the only caller of a property's `Setup` sends Color properties to the helper first (`SetupElementsTree.cs:228-235`). Record in `docs/parity`.

### Property: Order

`Q` = `Property/Order/`. TypeName "Order", TypeId `47D4A43B-5407-44BD-8BE1-4974B148504F`. In F170 to F177, where an entry names no file, a bare `:N` means `Q/OrderSetupHelper.cs`.

- [ ] **F170 Patching Order Preference window (`OrderSetupHelper`).** Title (`Q/OrderSetupHelper.Designer.cs:117`). HelperName "Patching Order". 462 x 519, resizable. List filling the window, OK and Cancel below it at the right. Reached by F4, F6.
- [ ] **F171 Order list.** Columns "Order", "Name". Rows: distinct leaves of the selection, stable-sorted by existing order, those with no Order property last; first column renumbered 1, 2, 3 (`Q/OrderSetupHelper.cs:192-221`). Multi-select, full row.
- [ ] **F172 Drag to reorder.** Drag one row, insertion line shows, drop moves it and renumbers (`Vixen.Common/Controls/DragDropListView/DragDropListView.cs:85-135, 205-222`, `Q/OrderSetupHelper.cs:245-248`). Single row only upstream: with several selected, only the dragged row moves, and afterwards it's the only one selected (`DragDropListView.cs:124-127`). The line goes above or below a row by which half the pointer is over, and past the last row means after it (`DragDropListView.cs:168-178`). **Mac:** `NSTableView` row drag with its own autoscroll; dragging several selected rows together is a harmless extra.
- [ ] **F173 Select all.** Ctrl+A (`Q/OrderSetupHelper.cs:28-40`). **Mac:** ⌘A.
- [ ] **F174 Context menu.** Right-click on the focused row (`:42-62`). With 2 or more selected: "Reverse", "Zig Zag...". With 1, upstream shows an empty menu. **Mac:** no menu with 1 selected.
- [ ] **F175 Reverse.** Reverses the rows in the selected positions, others stay put, selection stays, renumbers (`:64-82`).
- [ ] **F176 Zig Zag.** Number prompt titled "ZigZag Length", prompt "How many pixels to ZigZag?", default `min(50, selected)`, range 2 to selected (`:92-93`). OK and Cancel disabled while running. If selected count isn't divisible: "The total selected pixels must be evenly divisable by the zigzag length", title "Zigzag Error" (verbatim), exclamation icon (`:141-143`). Otherwise for every second group, reverses that group's selected positions (`:115-131`). Renumbers.
- [ ] **F177 OK.** Renumbers, sets or adds an Order property on each leaf (`:165-185`). Cancel changes nothing.
- [ ] **F178 Order Setup window (single property).** Title "Order Setup" (`Q/SetupForm.Designer.cs:92`). Label "Order", number up/down, OK and Cancel. Range 0 to 99999999999999 (`:58-62`), default 0 (`Q/OrderModule.cs:19`). **Quirk:** reading it as int throws above 2147483647 (`Q/SetupForm.cs:17`). **Mac:** range 0 to Int32.max. Reached by F3, which then copies to other selected elements. Persists: `Order`.

### Property: Orientation

`W` = `Property/Orientation/`. TypeName "Orientation", TypeId `DB0FC9FC-CDD5-4F88-BD66-5E4CF08FEAE4`.

- [ ] **F179 Orientation Setup window.** Title (`W/SetupForm.Designer.cs:87`). Label "Orientation", combo, OK and Cancel. Items Horizontal, Vertical (`W/SetupForm.cs:11-14`, `W/Orientation.cs:5-6`). **Quirk:** the combo is editable, and typed text throws on OK (`W/SetupForm.cs:23`). **Mac:** `Picker`. Reached by F3 with the current value, and by helper "Orientation" (F4).
- [ ] **F180 Helper behaviour.** Opens on Vertical whatever the elements have (`W/OrientationSetupHelper.cs:16`). OK sets or adds the property on the **selected nodes**, not their leaves (`:23-36`). Default for a new property is Vertical (`W/OrientationModule.cs:16`). Persists: `Orientation`.

### Property: Location

`J` = `Property/Location/`. TypeName "Location", TypeId `3FB53423-DD2A-4719-B3E3-19AA6F062F62`.

- [ ] **F181 Location Setup window.** Title (`J/SetupForm.Designer.cs:144`). Rows "X Position", "Y Position", "Z Position", each a number up/down, then OK and Cancel. Reached by F3, which copies to other selected elements.
- [ ] **F182 Fields.** Range 0 to 99999999999999 (`Designer:64-68`). New property defaults 1, 1, 1 (`J/LocationModule.cs:17`). Negative values load as 0 (`J/LocationData.cs:21-37`). **Quirk:** reading above 2147483647 throws (`J/SetupForm.cs:19, 29, 38`). **Mac:** range 0 to Int32.max. Persists: `X`, `Y`, `Z`.

### Property: Grid

`Y` = `Property/Grid/`. TypeName "Grid", TypeId `DE3B4EC9-4E7F-4690-B0FE-3780B17A85EC`.

- [ ] **F183 Grid Setup window.** Title (`Y/SetupForm.Designer.cs:156`). "Width", "Height" number up/downs, range 1 to 10000 (`:49-58, 73-82`). Red message when `width * height` differs from the element count: `The product of these needs to be {n}`, and OK disabled (`Y/SetupForm.cs:40-50`). Rechecked on change and on leaving a field (`:52-70`). New property: width = element count, height 1 (`Y/Module.cs:12-13`). `{n}` is `Owner.Count()`, the elements under the node (`Vixen.Core/Sys/ElementNode.cs:219-240`). **Quirks:** the check runs before the required count is set and OK starts disabled, so at open the message shows and OK is off even when the values are right, until a value changes or focus leaves a field (`Y/SetupForm.cs:14-16`, `Designer:107`); with a saved 1 x 1 no change fires, so OK is off with no message. A saved or default width above 10000, a node with more than 10000 leaves for example, throws at open, because the control's range is narrower (`Y/SetupForm.cs:26`). **Mac:** evaluate correctly at open; clamp the shown value to the range. Persists: `Width`, `Height`.

### Property: Face

`Z` = `Property/Face/`. TypeName "Face", TypeId `EDA73851-8350-4D3F-9DF2-0D3B34E70BB6`. Where an entry names no file, a bare `:N` means `Z/FaceSetupHelper.cs`.

- [ ] **F184 Face Setup window.** Title (`Z/FaceSetupHelper.Designer.cs:190`). HelperName "Face Mapping". 864 x 394, resizable. Tabs "Mouth" and "Outlines/Eyes", Mouth first (`:109, 148`, `Z/FaceSetupHelper.cs:33`). OK and Cancel bottom right. One row per node passed in, not leaves (`:128`). Reached by F3 (property configure), F4, F6.
- [ ] **F185 Mouth tab.** Grid columns: blank header, read-only element name; AI, E, ETC, FV, L, MBP, O, REST, U, WQ checkboxes, 50 wide, each header showing its phoneme image and the text (`:196-209, 236, 321-344`); "Color" (`:21`), 90 wide (`:260`); the name column sizes to its content (`:259`). Header height 75 (`:261`). Images are `Resources.*_Transparent`, drawn 48 x 48 and centred, with the text below (`:330-340`). **Mac:** reuse those PNGs.
- [ ] **F186 Outlines/Eyes tab.** Columns: name; "Outlines", "Eyes Open", "Eyes Closed" (`:161-163`, `Z/FaceComponent.cs:7-14`); "Color". Header images are blank placeholders (`Z/FaceSetupHelper.cs:212-228`).
- [ ] **F187 Check a cell.** Cell selection, multiple allowed (`:276-279`). Clicking a checkbox: for all selected checkbox cells, counts checked versus unchecked; new value is true when unchecked are the majority or tied, and all selected get it (`:377-412`). Selected cells in the name and Color columns don't count and don't change. **Mac:** cell-level selection, ⌘-click and ⇧-click; click applies the same rule, implemented in Rust. Space on the focused checkbox cell does the same as a click.
- [ ] **F188 Color cell.** Filled with the colour (`:362-369`). Double-click: if the selected rows' elements restrict colours, the discrete picker (F248), otherwise the full colour picker (`Z/FaceSetupHelper.cs:499-538`). OK writes all selected colour cells, in both tabs (`:446-462`). Column not sortable (`:262`). Other columns sort by clicking the header (default grid behaviour). **Quirks:** the writes use the displayed row number against the unsorted table, so after a header sort the colour lands on the wrong rows, apart from the double-clicked cell (`:454-460`); the picker opens only when the first selected cell is a colour cell (`:439`), so a double-click can do nothing with other cells selected. **Mac:** write by node id, and open whenever the double-clicked cell is a colour cell.
- [ ] **F189 Colour validation.** At load and at OK, a colour not allowed by the element's Color property becomes its first allowed colour (`:473-485`). Default white (`:147`).
- [ ] **F190 OK.** For each row: gets or adds a Face property; clears and refills `PhonemeList` with every phoneme; sets `DefaultColor`; clears and refills `FaceComponents` (`:64-113`). **Quirks:** `Mouth` is computed from a brand-new empty item, so it's always false (`:71, 102`); nothing reads it (`Effect/LipSync/LipSync.cs:117-129` reads only outlines and eyes). Rows find their node by name, so two nodes with the same name both map to the first (`Z/FaceSetupHelper.cs:286-289`). **Mac:** compute `Mouth` from the row's phonemes; use node ids.
- [ ] **F191 Cancel.** No change.
- [ ] **F192 Sort and row headers.** No row headers, no new-row line (`:281-282`).

### Property: State

`T` = `Property/State/`, `VM` = `Property/State/Setup/ViewModels/StateMapperViewModel.cs`, `.xaml` = `Property/State/Setup/Views/StateMapperView.xaml`. TypeName "State", TypeId `35B893C3-D10C-4359-82BC-929C0762E809`. The name prompt (F206) is the simple form. The mapper is included so nothing is lost.

- [ ] **F193 State Mapping entry.** Helper "State Mapping" with not exactly one node: "State Mapping can only be applied to one Element Node", title "Too many nodes selected", info (`T/StateSetupHelper.cs:45-52`). With a property: edits its data. Without: builds a new module, attaches only on OK; Cancel disposes it (`:54-107`). `HasElementSetupHelper` route returns false silently unless exactly one node (`T/StateModule.cs:87-93`).
- [ ] **F194 State Mapper window.** Title "State Mapper" (`T/Setup/ViewModels/StateMapperViewModel.cs:21`). 920 x 560, minimum 720 x 420 (`T/Setup/Views/StateMapperView.xaml:13-16`). Validation strip on top, OK and Cancel bottom right, top row across, left and right panes below (`:30-46`).
- [ ] **F195 Definition picker.** Label "Name", combo of definitions, first selected (`.xaml:68-79`). Blocked while there are blocking errors: selection snaps back (`VM:211-227`).
- [ ] **F196 Definition Add, Delete, "..." menu.** "Add" prompts F206 titled "Add State Definition", initial next `State - {n}` (`VM:415-436`). "Delete" enabled when more than one, confirm `Delete State definition '{name}'?`, title "Delete State Definition", question, YES/NO (`VM:438-453`, `T/Setup/Services/StateDefinitionDialogService.cs:59-64`). "..." menu "Rename" prompts "Rename State Definition" with the current name (`VM:457-479`) and "Copy" prompts "Copy State Definition" with `{name} Copy`, new ids, same assignments (`VM:483-507`). Add, Rename, Copy disabled during blocking errors. A new definition holds one item, "State Item - 1", in the default colour of F201 (`T/StateDefinitionData.cs:27-34`, `VM:1205-1214`), and becomes the selected one. After a delete, the definition now at that position is selected, or the last (`VM:449-451`). A cancelled prompt changes nothing. A State property always keeps at least one definition, "State - 1" with one item (`T/StateData.cs:15-19, 60-78`).
- [ ] **F197 Description.** Label "Description", multi-line, min height 48 (`.xaml:97-103`). Persists: `Description`.
- [ ] **F198 Preview toggle.** Label "Preview", toggle reading "Off" or "On", default off (`.xaml:122-139`). On shows selected items' colours on the assigned leaves in live preview; off clears (`VM:280-300, 966-979`). Closing releases (`VM:407-411`). Messages use context "State Preview", duration `int.MaxValue`, intensity 100 (`T/Setup/Preview/BroadcastStatePreviewPublisher.cs:11, 18-37`). Updates are incremental (`T/Setup/Preview/StatePreviewCoordinator.cs:27-52`).
- [ ] **F199 State Items heading and grid.** Bold centered "State Items". Grid columns "Name" (edit, commits on focus loss), "Color", "Count" read-only, multi-select, no new-row line (`.xaml:142-181`). Header sort becomes the stored order, then the sort clears (`.xaml:152-153`, `VM:659-709`).
- [ ] **F200 Color cell.** Hex text on the colour, contrasting text. Double-click: discrete picker when the item's leaves, or the root if none, restrict colours; otherwise full picker (`.xaml:159-175`, `VM:713-733`, `T/Setup/Services/StateColorPickerService.cs:36-60`).
- [ ] **F201 Add.** New row named next `State Item - {n}`, colour first discrete colour common to all leaves, or white; selected (`VM:511-520, 1216-1258`).
- [ ] **F202 Copy.** Exactly one selected: appended copy named `{source} - {n}`, same assignments (`VM:522-537`).
- [ ] **F203 Remove.** Confirm one: `Delete State item '{name}'?`, title "Delete State Item"; several: `Delete {count} State items?`, title "Delete State Items" (`T/Setup/Services/StateDefinitionDialogService.cs:70-85`). After one, selects the row now at that place; after several, none (`VM:561-603`).
- [ ] **F204 Move up and down.** Arrow-icon buttons, tooltips "Move selected State Item up", "Move selected State Item down" (`.xaml:186-199`). Enabled with one selected and room to move (`VM:605-618`).
- [ ] **F205 Assigned Elements pane.** Centered "Assigned Elements", "Toggle Selected" button with tooltip "Toggle selected assigned elements", enabled when tree rows are selected (`.xaml:219-225`). Tree of the node's hierarchy with checkboxes, shown only when exactly one item is selected (`VM:258-259`). Checking a group clears and disables its descendants (`T/Setup/Models/StateAssignmentTreeNode.cs:64-82, 178-212`). Stored ids are the checked nodes (`T/Setup/ViewModels/StateItemViewModel.cs:165-172`); Count is distinct effective leaves (`:125-128`). Selecting an item expands to its checked nodes (`VM:1083`). Row selection for the button: click, ⌘-click, ⇧-click over visible enabled rows (`T/Setup/Models/StateAssignmentTreeSelectionController.cs:40-120`). **Mac:** `NSOutlineView`.
- [ ] **F206 State definition name prompt.** Title from caller. Width 360, no resize. Label "Name", text field, focused on show, OK default, Cancel (`T/Setup/Views/StateDefinitionNameDialogView.xaml:49-56, 30-37`). Trimmed on OK (`T/Setup/ViewModels/StateDefinitionNameDialogViewModel.cs:95`).
- [ ] **F207 Name prompt validation.** "State definition name is required." and "State definition names must be unique." (exact, case-sensitive, excluding the current name) disable OK (`T/Setup/ViewModels/StateDefinitionNameDialogViewModel.cs:102-113`). Case-only match gives the warning "State definition names differ only by casing. Check you don't have a typo." and OK stays enabled (`:115-121`). **Quirk:** the view doesn't bind the warning. **Mac:** show it under the field.
- [ ] **F208 Mapper validation.** Errors: "State definition name is required." (`VM:740`), "Every State item requires a name." (`VM:753`), "State definition names must be unique." (`VM:763`), and on a row "State item name is required." (`StateItemViewModel.cs:141`). Warnings: "State definition names shorter than three characters may be unclear." (`VM:744`), the casing warning (`VM:775`). OK is enabled when there are no blocking errors (`VM:930-939`).
- [ ] **F209 OK.** Trims names, validates, replaces the data's definitions (`VM:392-404`). Stable ids are kept.
- [ ] **F210 Cancel.** Draft discarded (`VM:785`). Source data unchanged (the mapper works on a clone, `VM:93`).
- [ ] **F211 Naming rule.** `{base} - {n}`, first unused from 1, case-sensitive (`T/StateNamingRules.cs:47-59`). Bases "State" and "State Item".

**Property: Intelligent Fixture**, TypeId `FDD74C85-1779-4521-B559-0DD5EA71054E`. `Setup` opens `FixturePropertyEditorWindowView` and always returns true (`Property/IntelligentFixture/IntelligentFixtureModule.cs:49-64`). Owned by the fixture editor's spec. F3 must route to it. **Unused:** `T/Setup/Models/StateDefinition.cs` is never referenced by the mapper.

### Layer mixing filter: Chroma Key

`A` = `LayerMixingFilter/ChromaKey/`. TypeName "Chroma Key", TypeId `2D467733-1A64-45AD-BCDC-84BF074747E6`. `vixen_core::layer::{MixingFilter, ChromaKeySettings}` hold the data. New-filter defaults are `new_chroma_key_data()` in `crates/vixen-io/src/sequence.rs:681`, with the zero-brightness flag off as in `A/ChromaKeyData.cs:9-18`. `ChromaKeySettings::default` now has it off too, whatever the comment there says.

- [ ] **F212 Chroma Key Configuration window.** Title (`A/ChromaKeySetup.Designer.cs:277`). Top: colour at left, tolerance at right. Middle: brightness range. Then the checkbox. OK and Cancel bottom right. On OK all values are copied (`A/ChromaKeyModule.cs:70-85`). This filter always requires a mixing partner (`A/ChromaKeyModule.cs:12-15`).
- [ ] **F213 Color.** Label "Color", swatch 64 x 64 (`Designer:170, 176-179`). Click picks a colour (`Property/Color/ColorPanel.cs:21-29`). Tooltip "Click to Select the Key Color" (`A/ChromaKeySetup.cs:94`). Default `(0, 255, 0)` (`A/ChromaKeyData.cs:13`). Persists: `KeyColor`, and derived `KeySaturation` (rounded to 2 places) and `KeyHue` (`A/ChromaKeySetup.cs:32-33`).
- [ ] **F214 Hue tolerance.** Heading "Tolerance", label "Hue", slider 0 to 180, ticks 20, default 5, tooltip shows value (`Designer:190, 195-200, 221`, `A/ChromaKeySetup.cs:97-101`). Persists: `HueTolerance`.
- [ ] **F215 Saturation tolerance.** Label "Saturation", slider 0 to 99, ticks 10, default 5 (`Designer:206-211, 230`). Shows `SaturationTolerance * 100` at open (`A/ChromaKeySetup.cs:23`). Tooltip shows the value once moved (`:106`). **Quirk:** moving it sets `trkHueTolerance.Value / 100`, the wrong slider and integer division, so 0 unless hue is 100 or more, when it's 1 (`:105`). **Mac:** `trkSaturationTolerance / 100.0`. Default 0.05. Persists: `SaturationTolerance`.
- [ ] **F216 Brightness Range.** Heading "Brightness Range" (`Designer:161`). "Lower": slider 0 to 99, ticks 5, plus a numeric box. "Upper": slider 1 to 100, plus a box (`:116-133, 82-111, 143, 152`). Boxes accept whole numbers 0 to 100. Lower can't pass upper and upper can't pass lower; a value past the other snaps to it, on slider move or when a box loses focus (`A/ChromaKeySetup.cs:39-79`). Tooltips show the value. Stored as limit/100, loaded with `(int)(limit * 100)` (`:16-17, 29-30`), which truncates: 0.29 reads as 28. **Quirk:** typing 100 in Lower, or 0 in Upper, throws, since the slider range is narrower (`:59-67`). Emptying the Upper box does too, because empty text reads as 0 (`Vixen.Common/Controls/NumericTextBox.cs:314-315`), and that's on the way to retyping any value. Saved limits outside the slider ranges throw at open (`A/ChromaKeySetup.cs:85-86`). **Mac:** clamp to the slider range; an empty box keeps the last value until it has text. Persists: `LowerLimit`, `UpperLimit`.
- [ ] **F217 "Treat Zero Brightness as Transparent." checkbox.** Tooltip "Determines if the layer should pass the underlying color when the Chroma Key layer's effect(s) have a zero brightness color." (`A/ChromaKeySetup.cs:26`, `Designer:243`). Default off. Persists: `TransparentOnZeroBrightness`. **Quirk:** `Clone` drops this flag (`A/ChromaKeyData.cs:20-29`). Mac copies it.
- [ ] **F218 OK and Cancel.** Default and cancel keys set (`Designer:249, 253`). Cancel returns false.

### Layer mixing filter: Luma Key

`U` = `LayerMixingFilter/LumaKey/`. TypeName "Luma Key", TypeId `7583F7D4-DF99-4492-8D10-257C2677FF7B`.

- [ ] **F219 Luma Key Configuration window.** Title (`U/LumaKeySetup.Designer.cs:175`). Heading "Brightness Limits" (`:151`). Same Lower and Upper rows, rules and quirks as F216 (`U/LumaKeySetup.cs:11-71`), but no tooltips. Defaults 0 and 1 (`U/LumaKeyData.cs:10-11`). OK copies both, Cancel returns false (`U/LumaKeyModule.cs:38-47`). Persists: `LowerLimit`, `UpperLimit`.

### Layer mixing filter: Mask And Fill

`N` = `LayerMixingFilter/MaskFill/`. TypeName "Mask And Fill", TypeId `24878B18-14F8-458E-ABAA-3A9C9A34931C`.

- [ ] **F220 Mask And Fill Configuration window.** Title (`N/MaskAndFillSetup.Designer.cs:98`). Two checkboxes, OK and Cancel bottom right.
- [ ] **F221 "Exclude zero values".** (`:44`). Default on (`N/MaskAndFillData.cs:10`). Persists: `ExcludeZeroValues`.
- [ ] **F222 "Require Mixing Partner".** (`:77`). Default off. Persists: `RequiresMixingPartner`. **Quirk:** `Clone` drops it (`N/MaskAndFillData.cs:13-17`). Mac copies it.
- [ ] **F223 OK and Cancel.** (`N/MaskFillModule.cs:46-58`).

### Layer mixing filters with no setup

The `Configuration` button doesn't show for these (F5).

- [ ] **F224 Color Change.** "Color Change", `93D4169E-5138-403A-B71C-A5A1AB089801`, requires a partner.
- [ ] **F225 Highest Value (Default).** "Highest Value (Default)", `01D6D526-9586-44BB-82F4-10ABACF58B61`.
- [ ] **F226 Intensity Overlay.** "Intensity Overlay", `43F48BF5-4088-403F-AD7A-D0FF2114CA31`, requires a partner.
- [ ] **F227 Mask.** "Mask", `ACE0F097-7982-4EDC-B5DD-8989DCE60E1D`.
- [ ] **F228 Multiply Color.** "Multiply Color", `0C9DFC0B-8E39-4167-8036-79408C6B0B35`.
- [ ] **F229 Proportional Mix.** "Proportional Mix", `1F16D67A-1FFF-485C-8E76-27FF7E7D072B`.

### Output filters with the shared tag form

`TF` = `OutputFilter/TaggedFilter/`, `DF` = `OutputFilter/DimmingFilter/`, `WF` = `OutputFilter/ColorWheelFilter/`, `SH` = `OutputFilter/ShutterFilter/`, `PF` = `OutputFilter/PrismFilter/`. `FilterConfig` and its variants exist in `crates/vixen-output/src/filters/`.

- [ ] **F230 Tagged Filter Setup window.** Title (`TF/TaggedFilterSetup.Designer.cs:110`). TypeName "Tag Filter", TypeId `FE552DCF-1BC0-4588-B23C-2600151D9FC5`. Label "Tag:", text box, OK then Cancel at bottom right.
- [ ] **F231 Tag field, all five filters.** OK is disabled when empty (`TF/TaggedFilterSetup.cs:58-72`). **Quirk:** for a new Tag, Dimming or Color Wheel filter with a null tag no change event fires, so OK starts enabled and an empty tag can be saved (`:35`). Shutter and Prism start disabled, because filling their value fields runs the check first (`SH/ShutterFilterSetup.cs:44-47, 106-121`). **Mac:** always disabled when empty. On OK the output is rebuilt (`TF/TaggedFilterModuleBase.cs:122-142`, and each module's own `Setup`). Shape name becomes `{TypeName}: {tag}` (`:104-108`), and its one output is named with the tag (`TF/Outputs/TaggedFilterOutputBase.cs:29, 63`). **Quirk:** a filter shape reads its title when the diagram builds it (`Vixen.Application/Setup/SetupPatchingGraphical.cs:483`), so it shows the old tag until the next rebuild. **Mac:** refresh the title after OK. Persists: `Tag`.
- [ ] **F232 Dimming Filter Setup window.** Title (`DF/DimmingFilterSetup.Designer.cs:123`). TypeName "Dimming Filter", TypeId `61913688-EF44-473C-91D4-D256CCA6CB91`. "Tag:" as F231 and "Automatically Control Dimmer" checkbox (`:103`), default off. Persists: `ConvertRGBIntoDimmingIntents`. On: dim follows colour intensity; off: dim is 100% when any colour is present (`DF/Filters/DimmingFilter.cs:82-95`).
- [ ] **F233 Color Wheel Filter Setup window.** Title (`WF/ColorWheelFilterSetup.Designer.cs:122`). TypeName "Color Wheel Filter", TypeId `18470C0C-1E75-464F-BD9F-A3E241CD1EB8`. "Tag:" and "Automatically Control Color Wheel" checkbox (`:103`), default off. Persists: `ConvertColorIntentsIntoIndexCommands`. `ColorWheelData` isn't edited here; it comes from the fixture wizard and must round-trip. **Quirk:** `Clone` throws, since the target list is null (`WF/ColorWheelFilterData.cs:29-33`). Mac clones properly.
- [ ] **F234 Shutter Filter Setup window.** Title (`SH/ShutterFilterSetup.Designer.cs:170`). TypeName "Shutter Filter", TypeId `72B86092-0A59-4E40-B0A8-9E5FAD0B8EB3`. "Tag:"; "Automatically Open and Close Shutter" checkbox (`:107`), default off; "Open Shutter Value:" and "Close Shutter Value:" text boxes (`:119, 138`), defaults 0. Persist: `ConvertColorIntoShutterIntents`, `OpenShutterIndexValue`, `CloseShutterIndexValue`.
- [ ] **F235 Shutter value rules.** Toggling the checkbox enables or disables both values (`SH/ShutterFilterSetup.cs:128-148`). OK requires tag and both values non-empty (`:81-88`). **Quirks:** at open with the box unchecked, no change event fires, so the fields stay enabled; `byte.Parse` on OK throws on anything but 0 to 255 (`:71-74`). **Mac:** enable only when checked; validate 0 to 255 in Rust, OK disabled while invalid, `.help("0-255")`.
- [ ] **F236 Prism Filter Setup window.** Title (`PF/PrismFilterSetup.Designer.cs:192`). TypeName "Prism Filter", TypeId `2522BD94-0434-4728-9400-0E18B212DC21`. "Tag:"; "Automatically Open and Close Prism" (`:109`); "Open Prism Value:", "Close Prism Value:" (`:121, 140`); "Associated Prism Function Name:", optional text (`:159`). Persist: `ConvertPrismIntentsIntoOpenPrismIntents`, `OpenPrismIndexValue`, `ClosePrismIndexValue`, `AssociatedFunctionName`. Same rules as F235 (`PF/PrismFilterSetup.cs:87-94, 134-154`). **Quirk:** OK writes the open text into `ClosePrismIndexValue`, then overwrites it, so `OpenPrismIndexValue` is never saved (`:74-77`). **Mac:** save it. An empty function name matches every prism or command intent (`PF/Filter/PrismFilter.cs:46-50`), kept in the runtime.

### Output filter: Dimming Curve

`CV` = `OutputFilter/DimmingCurve/`. TypeName "Dimming Curve", TypeId `2e40d6b1-43d2-4668-b63a-c600fadd7dd5`. Where an entry names no file, a bare `:N` means `CV/DimmingCurveHelper.cs`.

- [ ] **F237 Dimming Curve filter Configure.** F2 opens the curve editor on the filter's curve; OK stores it and rebuilds the output (`CV/DimmingCurveModule.cs:136-148`). Its one output is named "Dimming Curve Output" in patching (`CV/DimmingCurveModule.cs:415-418`). Default curve is a ramp, x = y (`App/Curves/Curve.cs:38-41, 48`). Editor window belongs to the curve spec. Persists: `Curve`.
- [ ] **F238 Dimming Curve Configuration window (`DimmingCurveHelper`).** Title (`CV/DimmingCurveHelper.Designer.cs:206`). HelperName "Dimming Curve". Top to bottom: two lines, hint, button, question, four radios, then Help at bottom left, OK and Cancel at bottom right. Reached by F4, and in simple mode by F6.
- [ ] **F239 Instructions.** "This will set up a dimming curve for each selected element.", "Use the button below to configure the dimming curve for the selected elements.", "Hint: Use a library curve to make editing easier." (`Designer:56, 65, 165`).
- [ ] **F240 "Setup Dimming Curve" button.** Opens the curve editor on the helper's curve, default ramp. OK stores it and enables the dialog's OK (`CV/DimmingCurveHelper.cs:178-186`). OK starts disabled (`:32`).
- [ ] **F241 Existing filter question.** "If some elements are already patched to a dimming curve, what do you want to do?" (`Designer:96`).
- [ ] **F242 Radios, visual order.** "Leave the existing dimming curve alone." default (`Designer:86`); "Update the existing dimming curve with the new data." (`Designer:106`); "Insert a new dimming curve after the element." (`Designer:175`); "Add a new dimming curve anyway (not recommended)." (`Designer:117`). Simple mode disables all but Insert, and selects it (`CV/DimmingCurveHelper.cs:33-39`).
- [ ] **F243 Help.** "Help" with icon opens `https://www.vixenlights.com/docs/usage/display-setup/display-patching/` (`Designer:128-133`, `CV/DimmingCurveHelper.cs:197-200`, `Vixen.Common/Help/Help.cs:53-54`).
- [ ] **F244 OK result.** Distinct leaves of the selection (`:68`). Insert: for each leaf a new filter takes the element's output and the element's former destinations move behind it (`:80-96`). Others walk to leaf outputs and existing curves (`:99, 144-172`; add-new ignores existing curves): existing curve and Leave: skipped; Update: new curve stored; Add new: a new filter after it. A free output gets a new filter (`:101-133`). Then alert `{c} Dimming Curves created, {m} configured, and {s} skipped.`, no title, OK (`:138`). A leaf patched all the way to controller outputs with no curve yields nothing, so it isn't counted at all (`:155-157`). **Quirk:** an existing curve with a free output is returned twice, as itself and as that output, so Leave counts 2 skipped and Update counts 2 configured for it (`:150-171`). Keep the counts, as F161. **Quirk:** the `DimmingCurve` setter doesn't rebuild outputs, so new curves apply only after a reload, and all new filters share one curve object (`CV/DimmingCurveModule.cs:125-129`). **Mac:** apply at once; each filter gets its own copy.
- [ ] **F245 Cancel.** Returns false.

### Shared prompts, picker and grid keys

Numbered last so that the F-numbers code already cites stay put.

- [ ] **F246 Number prompt (`NumberDialog`).** Used by F70 and F176, and by Display Setup's count prompts. Title and prompt from the caller, no title of its own. Prompt label at left, number up/down at right, OK then Cancel below at the right (`Vixen.Common/Controls/NumberDialog.Designer.cs:43-76`). Value, minimum and maximum from the caller, defaults 0 and `int.MaxValue` (`Vixen.Common/Controls/NumberDialog.cs:7-17`). Enter in the field is OK, Esc is Cancel (`:24-34`). A typed value outside the range clamps when the control commits it (framework behaviour). **Mac:** `NumberPromptSheet`: `TextField` with `Stepper`, `.monospacedDigit()`, clamped the same way. Returns nothing on Cancel.
- [ ] **F247 Text prompt (`TextDialog`).** Used by F164, and by Display Setup's name prompts. Prompt label on top, text field across, OK then Cancel below at the right (`Vixen.Common/Controls/TextDialog.Designer.cs:36-66`). Title from the caller, or blank with the one-argument form. Optional initial text, optionally selected (`Vixen.Common/Controls/TextDialog.cs:14-26`). The field has focus on show (`:31-36`). Enter is OK, Esc is Cancel (`:40-44`). The response comes back as typed, not trimmed (`:46-49`), and nothing rejects it here. **Mac:** `TextPromptSheet`.
- [ ] **F248 Discrete colour picker (`SingleDiscreteColorPickerView`).** Used by F188 and F200. Title "Select Color". A wrapping list of 50 x 50 swatches, single selection; "Ok" and "Cancel" below at the right, spelled like that; Ok is the default button (`Vixen.Common/DiscreteColorPicker/Views/SingleDiscreteColorPickerView.xaml:10, 27-55`). Swatches in the order of the set passed in; the current colour starts selected when it's in the set (`Vixen.Common/DiscreteColorPicker/ViewModels/DiscreteColorPickerViewModelBase.cs:90-115`). Ok is disabled until something is selected (`Vixen.Common/DiscreteColorPicker/ViewModels/SingleDiscreteColorPickerViewModel.cs:14-18`). Window is sized to a roughly square grid, at least 250 wide, at most 8 columns, scrolling after that (`Vixen.Common/DiscreteColorPicker/Views/DiscreteColorPickerViewBase.cs:54-127`). Upstream opens it centred on the screen. **Mac:** `DiscreteColorSheet`, a sheet on the owning dialog; never the free colour panel.
- [ ] **F249 E1.31 grid keys.** The grid's defaults, with no code of their own (framework behaviour): arrows and Tab move; F2 or typing into an editable cell starts an edit (`EditMode` is left at its default, `SFD:341-380`); Esc abandons it; Space toggles an On cell; Ctrl+A selects all cells; Ctrl+C copies the selection as tab-separated text. **Mac:** arrows; Tab; Return or typing starts an edit; Esc; Space toggles On on the focused row; ⌘A; ⌘C copies the selected rows as tab-separated text from Rust. Delete is F68.

### Field reference, per dialog

Every field in one place. Entries above hold the detail and the citations. "Mac" marks a value that differs on the Mac; the register lists why. Messages are verbatim.

| Dialog | Field | Control | Range or list | Default | Validation or message | Entry |
|---|---|---|---|---|---|---|
| Serial Port Config | Port name | drop-down | scanned ports, ` (IN USE)` suffix. Mac: `/dev/cu.*` | first port. Mac: configured port; none when that's absent; first port when nothing is configured | `* Port name has not been selected.` Warning "Serial Port may be in use, do you wish to continue?" | F12, F13, F21 |
| | Baud rate | drop-down | 2400, 4800, 9600, 19200, 38400, 57600, 115200, 230400, 460800, 921600, 1048576 | 57600 | `* Baud rate has not been selected.` | F14 |
| | Parity | drop-down | None, Odd, Even, Mark, Space. Mac: last two disabled | None | none | F15, F16 |
| | Data bits | text, 1 character. Mac: picker | any digit; only 5 to 8 survive. Mac: 5, 6, 7, 8 | 8 | `* Invalid value for data bits.` | F17 |
| | Stop bits | drop-down | One, Two, OnePointFive. Mac: last disabled | One | none | F18 |
| Generic Serial | Currently: | read-only text | `{port}: {baud}, {parity}, {databits}, {stopbits}` | "Not Set" | OK disabled while not set | F27, F28, F34 |
| | Setup/Change Serial Port | button | opens Serial Port Config | | | F27 |
| | Send a text header | checkbox | | on when saved text is non-empty | | F30 |
| | Header text | one-line text | any; non-ASCII sends `?` | empty, disabled until checked | none | F31 |
| | Send a text footer | checkbox | | on when saved text is non-empty | | F32 |
| | Footer text | one-line text | any; non-ASCII sends `?` | empty, disabled until checked | none | F33 |
| Dummy Lighting | Window Title: | text | any, blank allowed | Dummy Lighting Controller | none | F39 |
| | Style | 3 radios | monochrome; every 3 inputs one RGB; every input one RGB | monochrome | exactly one | F40 to F43 |
| E1.31, Universes | Start | cell, 5 digits | 1 to 99999 | running total. Mac manual row: previous start + size | "Start must be between 1 and 99999 inclusive" | F62, F66, F69 |
| | On | checkbox cell | | checked | | F63 |
| | Universe | cell, 5 digits | 1 to 64000 | highest + 1 | "Universe must be between 1 and 64000 inclusive" | F64, F69 |
| | Size | cell, 3 digits | 1 to 512 | last row's, or 510 | "Size must be between 1 and 512 inclusive" | F65, F69 |
| | Destination | drop-down | `Multicast {name}`, then `Unicast {address}`, each sorted. Mac: `Multicast {name} ({id})` | saved. Mac when none: primary interface | "The selected multicast interface was not found, be sure to verify your destination." | F53 |
| E1.31, Advanced Options | Manually manage start values. | checkbox | | off | | F74 |
| | Display ALL Warnings/Errors and wait For OK | checkbox | | off | | F75 |
| | Gather statistics and display at end of session | checkbox | | off | | F76 |
| | Transmit blind data. | checkbox | | off | | F77 |
| | sACN priority. | number | 0 to 200, step 1 | 100 | | F78 |
| | Max Repeat Count | text, 2 characters | 0 to 99 | 0 | beep, no message | F79 |
| | Max Suppress Count | text, 2 characters | 0 to 99 (validator allows 10000) | 0 | beep, no message | F80 |
| E1.31, on OK | whole table | | | | F83 to F86's messages, in that order | F83 to F87 |
| New Unicast Destination | IP Address radio, field | radio, IP field | four octets. Mac: OK disabled unless IPv4 | selected, 127.0.0.1 | "Error - Duplicate IP Address" | F54, F57 |
| | Network Name radio, field | radio, text | any. Mac: OK disabled when blank | myComputer | "Error - Duplicate IP Address" | F54, F58 |
| DDP | IP Address radio, field | radio, text | Mac: OK disabled unless IPv4 | selected, saved address or 127.0.0.1 | none upstream: bad text becomes 127.0.0.1 | F96, F97 |
| | Host Name radio, field | radio, text | trimmed | myhost.local | "Could not resolve host name '{name}' to an IP address. Check the spelling and your network connection, or switch to IP Address mode." | F96, F97 |
| OpenDMX | Device | drop-down | `{Description} - {SerialNumber} - {Id}` | saved, else first | "No devices found to open." "Failed to open OpenDMX device. Error from Driver: {status}" | F105, F106 |
| Elexol | IP Address | text | any IP | saved, or 127.0.0.1 | "Invalid IP Address." | F110, F114 |
| | Port | text | Mac: 1 to 65535 | saved, or 2424 | Mac: "Invalid Port." | F111, F114 |
| | Test | button | | | "No Reply." "Connection Successful!" "Device replied, but with incorrect data:" "An error occured testing the device" | F112 |
| | Minium 'ON' Intensity (0-255): | slider, value below | 0 to 255, ticks and large step 16 | 1 | none | F113 |
| RDS | Hardware | 4 radios | MRDS192, MRDS1322, HTTP(S), V-FMT212R | HTTP(S) | sets F118's rules | F117, F118 |
| | Serial Port | drop-down | sorted ports. Mac: `/dev/cu.*` | none | disabled for V-FMT212R | F119 |
| | URL | text | `{text}`, `{time}` macros | `http://127.0.0.1:8080/?action=update_rt&update_rt={text}` | reset by V-FMT212R's rule | F120, F118 |
| | HTTP(s) Requires Authentication | checkbox | | off; forced on for V-FMT212R | | F122 |
| | UserName, Password | text, 8 characters | | empty | none | F123 |
| | Macro value {text}, Send | text, button | 8, 64 or 128 characters by hardware | empty | status "Data Sent" or "Data Not Sent" | F124 |
| | BiDirectional, Slow | checkboxes | Slow: 2400 instead of 19200 baud | off | | F127 |
| | Hide Launched Windows | hidden checkbox | | off | | F128 |
| Launcher | Hide Launched Windows | checkbox | | off | | F132 |
| Debug window | Verbose | checkbox | | off, not saved | | F134 |
| Color Breakdown | Template, Apply Template | drop-down, button | RGB, RGBW, RGBY, RGxB, GRBW, GRWB | RGB | no confirmation | F137, F138 |
| | Row colour, Name, Delete | swatch, text, button | | white, New Color | no confirmation | F140 to F142 |
| | These colors should be mixed to produce the input color | checkbox | | off | Mac: note for 4 colours outside R, G, B, W | F144 |
| | 16 Bit Output (For Use with Course Fine Break Down) | checkbox | | off | | F145 |
| Coarse Fine | Enable default value mapping | checkbox | | off | | F149 |
| | Live mode (apply changes immediately) | checkbox | | off, not saved | | F149 |
| | Resting coarse value (0-255) | slider | 0 to 255, whole steps | 0 | "Resting coarse value must be between 0 and 255." | F149 |
| | Resting fine value (0-255) | slider | 0 to 255, whole steps | 0 | "Resting fine value must be between 0 and 255." | F149 |
| Tag family | Tag: | text | required | empty | OK disabled when empty (Mac always) | F231 |
| Dimming Filter | Automatically Control Dimmer | checkbox | | off | | F232 |
| Color Wheel Filter | Automatically Control Color Wheel | checkbox | | off | | F233 |
| Shutter Filter | Automatically Open and Close Shutter | checkbox | | off | enables the next two | F234, F235 |
| | Open Shutter Value:, Close Shutter Value: | text | 0 to 255 | 0 | OK disabled when empty; Mac also when outside the range | F234, F235 |
| Prism Filter | Automatically Open and Close Prism | checkbox | | off | enables the next two | F236 |
| | Open Prism Value:, Close Prism Value: | text | 0 to 255 | 0 | as Shutter | F236 |
| | Associated Prism Function Name: | text | optional | empty | none | F236 |
| Dimming Curve helper | Setup Dimming Curve | button | opens the curve editor | ramp | OK disabled until the editor's OK | F240 |
| | Existing curve radios | 4 radios | Leave, Update, Insert, Add | Leave. Simple mode: Insert only | | F242 |
| Color helper | Three mode radios | radios | single, multiple, any | none selected, unless preset | OK disabled until one is | F152, F154, F157 |
| | Single colour | swatch | | RoyalBlue | | F153 |
| | Colors: | drop-down, Edit Colors | colour set names | first, or preset | | F155, F156 |
| | Color Order: | drop-down | RGB, RBG, GBR, GRB, BRG, BGR, RGBW, GRWB | RGB | | F158 |
| Color Sets | Defined Color Sets: | list, add, remove | | RGBW, RGB | "Color Set already exists." "Error removing Color Set!" | F163 to F165 |
| | Name: | text | | selected set's | "You must enter a name for the Color Set." | F166, F168 |
| | Colors: | swatches, add | | added colours are white | on close: "Do you want to save changes to the displayed color set?" | F167, F169 |
| Patching Order helper | Order list | table | leaves of the selection | current order | "The total selected pixels must be evenly divisable by the zigzag length" | F171 to F176 |
| Order Setup | Order | number | 0 to 99999999999999. Mac: 0 to 2147483647 | 0 | | F178 |
| Orientation Setup | Orientation | drop-down | Horizontal, Vertical | Vertical | | F179, F180 |
| Location Setup | X, Y, Z Position | numbers | 0 to 99999999999999. Mac: 0 to 2147483647 | 1 | | F182 |
| Grid Setup | Width, Height | numbers | 1 to 10000 | element count, 1 | "The product of these needs to be {n}" | F183 |
| Face Setup | Phoneme and component checkboxes | grid cells | | off | | F185 to F187 |
| | Color | grid cell | allowed colours only | white | invalid becomes the first allowed | F188, F189 |
| State Mapper | Name, Add, Delete, Rename, Copy | combo, buttons, menu | | State - 1 | F208's messages | F195, F196 |
| | Description | multi-line text | | empty | | F197 |
| | Preview | toggle | Off, On | Off, not saved | | F198 |
| | Item Name, Color, Count | grid | | State Item - {n}, default colour | F208's messages | F199 to F204 |
| State name prompt | Name | text | trimmed | caller's | "State definition name is required." "State definition names must be unique." | F206, F207 |
| Chroma Key | Color | swatch | | (0, 255, 0) | | F213 |
| | Hue, Saturation | sliders | 0 to 180; 0 to 99 | 5; 5 | | F214, F215 |
| | Lower, Upper | slider and box | sliders 0 to 99 and 1 to 100; boxes 0 to 100 | 0; 100 | snaps to the other limit | F216 |
| | Treat Zero Brightness as Transparent. | checkbox | | off | | F217 |
| Luma Key | Lower, Upper | slider and box | as Chroma Key | 0; 100 | as Chroma Key | F219 |
| Mask And Fill | Exclude zero values | checkbox | | on | | F221 |
| | Require Mixing Partner | checkbox | | off | | F222 |

### Deviation register and open questions

Each Mac note above that changes behaviour, for the owner's veto. Numbers are the feature.

| Feature | Upstream | Mac |
|---|---|---|
| F12 | Picker resets to first port on open | Preselect the configured port; require a choice when absent |
| F17 | Any digit, then throws | Picker 5 to 8 |
| F36 | Open failure only logged | Also a status on the controller row |
| F38 | Throws when never started | Reads from data |
| F47 | No reflow; can cut off the last row | Reflows; `ceil` rows |
| F67 | Lexical sort | Numeric sort |
| F69 | Can add universe 64001; manual start integer 0 crashes OK | Limit message; computed start |
| F84 | "Review" actually saves | "Review Settings" keeps open |
| F88 | OK starts a stopped controller; leaks sockets | Restart only if running |
| F89 | First channel labelled `-2` by one path | One 1-based function |
| F92 | Timing wrong | Correct |
| F95 | OK stops output, Cancel marks running | Restore prior state |
| F97 | Bad IP silently becomes 127.0.0.1 | OK disabled |
| F112, F114 | Success titled "Error"; bad port throws | Fixed; new "Invalid Port." text |
| F116, F131 | Write-through, no Cancel | Staged, plus Cancel |
| F119 | `COM{n}` only | Path in `PortName` |
| F140 | Picked pure red maps to blue | Store the known colour |
| F144, F145 | Cancel doesn't revert | Staged; 4-colour non-RGBW doesn't crash |
| F178, F182 | Above Int32 throws | Range to Int32.max |
| F183 | OK off at open though correct | Correct at open |
| F190 | `Mouth` always false; rows by name | Computed; by id |
| F215 | Wrong slider, integer division | Correct slider |
| F216 | Typing 100 or 0 throws | Clamp |
| F231, F235 | Empty tag saveable; fields enabled when unchecked; `byte.Parse` throws | Rules applied consistently |
| F236 | Open value never saved | Saved |
| F244 | New curve applies after reload | Applies at once |
| F8, F10, F46, F49, F53, F73, F123 | Various | Refresh button and keeping ports whose probe fails, empty state, window handling, first-frame fps, default interface, labels on icons, `SecureField` |
| F28 | Saved name with zero baud or bits throws at open | Shows what's saved |
| F36, F100 | Stop doesn't cancel a pending retry, which then restarts the controller | Stop cancels it |
| F62 | Read-only automatic starts are range-checked and can trap focus | Not checked; may exceed 99999 |
| F66, F79 | An invalid cell or count blocks Cancel | Cancel always works |
| F68 | `.` deletes rows as well as the Delete key | Delete key only |
| F69 | New row isn't really selected | New row is selected |
| F71 | Enabled with one row right after Add | Needs a selection and two rows |
| F91, F92 | Row numbers are 0 after a load; an interface with no IPv4 throws on the first send | 1-based row; that universe sends nothing |
| F97 | Accepts `10.1` and IPv6; a rejected name throws out of OK | Dotted-quad IPv4 only; rejected name gets the failure text |
| F100, F130 | Write timeout and malformed commands throw on the output thread | Logged; frame or command skipped |
| F102, F114 | DMX Pro's OK doesn't restart; Elexol's OK connects even when stopped | Restart only if running, connect only if running |
| DMX Open count | Channel Count has no cap and throws above 512 | Capped at `OutputLimit` |
| F108 | After any Stop, the writer thread never runs again | Restart resumes |
| F113 | Saved 0 reopens as 1 | Shows saved |
| F124 | Serial failure throws out of Send | "Data Not Sent" |
| F134 | Closing the window by hand throws on the next update | Keeps running, as F46 |
| F155, F156 | Null set name throws on OK; reload forgets the picked set | OK disabled; pick kept |
| F163 | Remove enabled with nothing selected | Disabled |
| F167 | Colour grid doesn't scroll | Scrolls |
| F183 | More than 10000 elements throws at open | Value clamped |
| F188 | Colour lands on the wrong rows after a sort; first-cell rule | By node id; double-clicked cell decides |
| F216, F219 | Emptying Upper throws; saved limits outside the range throw | Keeps the last value; clamps |
| F231 | Shape title stale after a tag change | Refreshed |

**Typos kept verbatim:** "Minium 'ON' Intensity" (F113), "Name Requred" (F168), "evenly divisable" (F176), "Course Fine Break Down" (F145), "bur currently down" (F91, log), "An error occured" (F112), "Your configurations contains" (dead), "Ok" (F248), `Partity` (XML member, must stay).

**Open questions:**

1. Should the Mac remember a controller's USB identity, so `/dev/cu.usbmodem1101` is found again after replugging into another USB port? macOS names some devices by USB location. Nothing in `Data` can hold it without a new member, so it would be a per-machine side table.
2. Is restarting the Arduino on controller start acceptable? `dtr_on_open(false)` avoids most of it, but an open can still pulse DTR; `port_in_use` probing resets it when the dialog opens with the controller stopped. Clearing `HUPCL` would help if the owner's sketch minds.
3. Should Face data round-trip with Windows Vixen? The owner's profile has no Face property, so there's no sample of the `KeyValueOfFaceComponentboolean...` element name, whose suffix is a .NET hash. Read by prefix. A sample from Windows is needed to write the exact name.
4. RDS HTTP authentication: `NetworkCredential` negotiates Basic or Digest. Which Rust HTTP client, and is Digest needed? Not used by the owner, so low priority.
5. Serial write pacing at the owner's settings. See "Performance hazards": 16 bytes at 9600 baud take longer than the 10 ms update interval.

## Data and persistence

**Files.** `{profile}/SystemData/ModuleStore.xml` (`version="5"` in the owner's copy) holds module data. `{profile}/SystemData/SystemConfig.xml` holds controllers, outputs and their names, `DisabledDevices`, and which filter and property instances exist. `crates/vixen-io/src/module_store.rs` keeps each entry as an `XElement`, so unknown members survive. Paths: `DataPaths::module_store_file`, `system_config_file`.

**When.** Read at startup. Written when Display Setup's OK runs `SaveSystemConfigAsync` then `SaveModuleConfigAsync`; Cancel reloads system config (`Vixen.Application/VixenApplication.cs:1000-1016`). F6's face route saves straight away (`TimedSequenceEditorForm_Menu.cs:603`). Before saving, Display Setup removes orphaned filters and properties (`Vixen.Application/Setup/DisplaySetup.cs:270-278`). Layer filter settings live in the sequence, under `_layerMixingFilterDataModels` (`crates/vixen-io/src/sequence.rs:580, 971`), and save with it. Cancel on Display Setup reloads `ModuleStore.xml` as well as `SystemConfig.xml` (`Vixen.Core/Sys/VixenSystem.cs:230-285, 334-343`), which is why it undoes colour set edits and every sheet. On the Mac, Display Setup's dialogs work on its working copy (`DialogContext`, `crates/vixen-ffi/src/dialogs.rs`), with the same result.

**Entry shape.** From the owner's file:

```xml
<Module dataModelType="VixenModules.Output.GenericSerial.Data, GenericSerial" moduleType="d22c9233-16e9-4008-a215-e577f78a7a2e" moduleInstance="ee79fee2-2d7d-4984-a178-288cee39a361" assemblyName="Module.Controller.GenericSerial">
  <Data xmlns="http://schemas.datacontract.org/2004/07/VixenModules.Output.GenericSerial" xmlns:i="http://www.w3.org/2001/XMLSchema-instance">
    <ModuleInstanceId xmlns="">ee79fee2-2d7d-4984-a178-288cee39a361</ModuleInstanceId>
    <ModuleTypeId xmlns="">d22c9233-16e9-4008-a215-e577f78a7a2e</ModuleTypeId>
    <BaudRate>9600</BaudRate>
    <DataBits>8</DataBits>
    <Footer />
    <Header />
    <Parity>None</Parity>
    <PortName>COM9</PortName>
    <StopBits>One</StopBits>
  </Data>
</Module>
```

Rules: `assemblyName` is `Module.{Controller|OutputFilter|Property|LayerMixingFilter}.{project}` (`Controller/Directory.Build.props`), `dataModelType` is `{full type}, {project}` (`Vixen.Core/IO/Xml/Serializer/XmlModuleDataModelSerializer.cs:50-66`). Namespace is `http://schemas.datacontract.org/2004/07/{clr namespace}`. Members: base class first, then ordinal order. Enums are by name. A missing member keeps the zero value, because a data contract is deserialized without running the constructor, except where `OnDeserializing` sets one (`crates/vixen-output/src/xmlutil.rs:5-6`).

**`System.Drawing.Color`** in the owner's file: `<Color xmlns:a="http://schemas.datacontract.org/2004/07/System.Drawing"><a:knownColor>141</a:knownColor><a:name i:nil="true" /><a:state>1</a:state><a:value>0</a:value></Color>` for a known colour (Red 141, Lime 104, Blue 37; White 164), `state` 2 and `value` = unsigned ARGB for a value colour, `state` 0 for empty. `vixen_io::values` and `vixen_output::dotnet::DotNetColor` handle this.

**Module data.** Members in stored order; defaults are the constructor's.

| Module | CLR type, project | Members (default) |
|---|---|---|
| Generic Serial | `VixenModules.Output.GenericSerial.Data`, GenericSerial | BaudRate, DataBits, Footer, Header, Parity, PortName, StopBits (all zero or null) |
| Dummy Lighting | `VixenModules.Output.DummyLighting.DummyLightingData`, DummyLighting | FormTitle ("Dummy Lighting Controller"), RenderStyle (Monochrome) |
| E1.31 | `VixenModules.Output.E131.E131ModuleDataModel`, E131 | AutoPopulate (true), Blind, EventRepeatCount, EventSuppressCount, Multicast, OutputCount, Priority (100; also forced by `OnDeserializing`, `E131ModuleDataModel.cs:17-21`), Statistics, Unicast, Universes (one entry, below), Warnings |
| E1.31 universe | `VixenModules.Controller.E131.UniverseEntry` | Active, EventRepeatCount, EventSuppressCount, Multicast, Size, Start (zero-based), Ttl (64), Unicast, Universe, seqNum. Constructor default: active, universe 1, Start **1**, size 510 (`E131ModuleDataModel.cs:14`), so an unconfigured controller is offset by one until Setup runs. `Multicast`, `Unicast` and `Ttl` are deprecated but still written (F88). `RowNum` isn't a member, so it's lost on load (F91). The two counters and `seqNum` are runtime values that get saved. The model's own `OutputCount` is never written by the plugin, so it stays 0 |
| DDP | `VixenModules.Output.DDP.DDPData`, DDP | Address ("127.0.0.1"), HostName |
| Renard | `VixenModules.Output.Renard.Data`, Renard | BaudRate, DataBits, Parity, PortName, ProtocolVersion, StopBits, WriteTimeout |
| DMX Pro | `VixenModules.Output.DmxUsbPro.Data`, DMXUsbPro | BaudRate, DataBits, Partity, PortName, StopBits |
| DMX Open | `VixenModules.Controller.OpenDMX.OpenDMXData`, OpenDMX | Device { Description, Id, Index, Name, SerialNumber, Type }. Null until chosen. `Name` is never set, so it's null (`O/OpenDMX.cs:228-235`) |
| Elexol | `VixenModules.Output.ElexolEtherIO.ElexolEtherIOData`, ElexolEtherIO | Address, MinimumIntensity, Port |
| RDS | `VixenModules.Output.CommandController.Data`, RDSController | BiDirectional, ConnectionMode, HardwareID, HideLaunchedWindows, HttpPassword, HttpUrl, HttpUsername, PortName, PortNumber, RequireHTTPAuthentication, Slow |
| Launcher | `VixenModules.Output.LauncherController.Data`, LauncherController | HideLaunchedWindows |
| Color Breakdown | `VixenModules.OutputFilter.ColorBreakdown.ColorBreakdownData`, ColorBreakdown | BreakdownItems [ { Color, Name } ], MixColors, _16Bit |
| Coarse Fine | `...CoarseFineBreakdown.CoarseFineBreakdownData`, CoarseFineBreakdown | EnableDefaultValueMapping, RestingCoarseValue, RestingFineValue |
| Dimming Curve | `...DimmingCurve.DimmingCurveData`, DimmingCurve | Curve (ramp) |
| Tag family | `...TaggedFilter.TaggedFilterData`, `...DimmingFilter.DimmingFilterData`, `...ColorWheelFilter.ColorWheelFilterData`, `...ShutterFilter.ShutterFilterData`, `...PrismFilter.PrismFilterData` | Tag, then in order: ConvertRGBIntoDimmingIntents; ColorWheelData, ConvertColorIntentsIntoIndexCommands; CloseShutterIndexValue, ConvertColorIntoShutterIntents, OpenShutterIndexValue; AssociatedFunctionName, ClosePrismIndexValue, ConvertPrismIntentsIntoOpenPrismIntents, OpenPrismIndexValue. `Tag` sits in the TaggedFilter namespace (inferred from data contract rules; the owner has no sample) |
| Color property | `VixenModules.Property.Color.ColorData`, Color | ColorSetName, ElementColorType (FullColor), SingleColor. The two optional members use `EmitDefaultValue = false`, so they're left out when default. Loading clears SingleColor for full colour and turns an empty set name into null (`H/ColorData.cs:12-16, 30-42`) |
| Colour sets | `VixenModules.Property.Color.ColorStaticData`, type-level entry | ColorSets dictionary, items `a:KeyValueOfstringArrayOfColor3odnvp_PE` |
| Location, Order, Orientation | `...LocationData`, `...OrderData`, `...OrientationData` | X, Y, Z; Order; Orientation |
| Grid | `VixenModules.Property.Grid.Data`, Grid | Height, Width |
| Face | `VixenModules.Property.Face.FaceData`, Face | DefaultColor, FaceComponents, PhonemeList (case-insensitive keys) |
| State | `VixenModules.Property.State.StateData`, State | Id, StateDefinitions [ { Description, Id, Items [ { Color, ElementNodeIds, Id, Name } ], Name } ] |
| Chroma Key, in the sequence | `VixenModules.LayerMixingFilter.ChromaKey.ChromaKeyData`, ChromaKey | HueTolerance (5), KeyColor (0, 255, 0), KeyHue, KeySaturation, LowerLimit (0), SaturationTolerance (0.05), TransparentOnZeroBrightness, UpperLimit (1) |
| Luma Key, in the sequence | `VixenModules.LayerMixingFilter.LumaKey.LumaKeyData`, LumaKey | LowerLimit (0), UpperLimit (1) |
| Mask And Fill, in the sequence | `VixenModules.LayerMixingFilter.MaskFill.MaskAndFillData`, MaskFill | ExcludeZeroValues (true), RequiresMixingPartner |

Property data persists as instance entries even when the descriptor names no data class (Face, Location, Order, Orientation): the module's own object is added on first assignment (`Vixen.Core/Module/ModuleDataSet.cs:86-117`). The owner's store has 1127 Color, 1126 Location and 3 Orientation entries, and 1117 Color Breakdown filters.

`FaceDescriptor` also names `FaceData` as static data (`Z/FaceDescriptor.cs:22`), though nothing reads it, and the owner's store has no such entry. If one appears, keep it untouched.

**Not saved, per process:** E1.31's known unicast list and running-instance list (`OP:68-69`), its two once-per-run warning flags (`OP:70-71`), RDS's last-sent text (`K/Module.cs:18`), RDS's test text box (F124), the coarse-fine Live mode checkbox, the Debug window's Verbose checkbox, and the State mapper's Preview toggle.

**Output names** are saved in `SystemConfig.xml` as `<Output name="..." id="..." />`, so F88's renames persist.

## Rust API needed

Signatures are for the implementer's guidance. `crates/vixen-output` is being written in parallel: **check it first** and reuse what exists. Already there as of this writing: `serial::{Parity, StopBits, SerialSettings, list_serial_ports, is_windows_port_name, port_availability, port_in_use, PortOpenError, open_port, RetryTimer}`, `net::{NetworkInterfaceInfo, list_network_interfaces, list_multicast_interfaces, interface_availability}`, `filters::{FilterConfig, ColorBreakdownConfig, CoarseFineConfig, DimmingCurveConfig, TaggedFilterConfig, DimmingFilterConfig, PrismFilterConfig, ShutterFilterConfig, ColorWheelFilterConfig}`, `ids`, `dotnet::DotNetColor`.

**Status at audit.** Most of what follows now exists, and the signatures below are the original sketch. Look there first: `crates/vixen-session/src/dialogs/` (`serial`, `e131`, `controllers`, `filters`, `color`, `dimming`, `order`, `properties`, `face`, `state`, `layers`, `flow`, whose functions name the F-numbers they implement), `crates/vixen-output/src/modules/`, and `crates/vixen-ffi/src/dialogs.rs` with `dialogs/`. The session functions therefore live in `vixen-session`, not `vixen-output`, wherever the headings below say otherwise. Names that differ from the sketch:

| Sketch | Real name |
|---|---|
| `BAUD_RATES` | `vixen_output::serial::STANDARD_BAUD_RATES` |
| `multicast_choices()` | `vixen_output::net::multicast_choices(saved)`; also `net::{MulticastChoice, primary_interface_id}` |
| `GenericSerialData`, `DummyLightingData`, `E131Data`, `DdpData`, `RenardData`, `DmxProData`, `OpenDmxData`, `ElexolData`, `RdsData`, `LauncherData` | `vixen_output::modules::{GenericSerialConfig, DummyLightingConfig, E131Config, DdpConfig, RenardConfig, DmxUsbProConfig, OpenDmxConfig, ElexolConfig, RdsConfig, LauncherConfig}`, each with `from_xml` and `to_xml` |
| `resolve_ipv4` | `modules::ddp::resolve_to_ipv4` |
| `dummy_frame_cells`, `format_fps(updates, ms)` | `dialogs::controllers::{dummy_frame_rgba, fps_from_counts}` |
| `elexol_test` | `dialogs::controllers::elexol_test` over `modules::elexol::test_connection` |
| `rds_send`, `rds_expand_url` | `modules::rds::{send_test_text, build_url}` |
| `order_move(rows, from, to)` | `dialogs::order::order_move(rows, selected, to)`, several rows together |
| `open_dmx_match` | `dialogs::controllers::open_dmx_selection` |
| `e131_take_messages` | `take_messages` on the module's shared state, `crates/vixen-output/src/modules/mod.rs` |
| per-controller output limit | `vixen_session::setup::controllers::output_limit(type_id)` |

### Added by the audit

Not there yet unless a name above covers it:

```text
fn e131_rows_text(rows: &[UniverseRow], selected: &[usize]) -> String          // vixen-session, F249: tab-separated rows for ⌘C
fn percent_box_value(text: &str, last: i32) -> i32                             // vixen-session, F216, F219: empty keeps the last value; then `clamp_limits` applies
fn grid_values_clamped(width: i32, height: i32) -> (i32, i32)                  // vixen-session, F183: into 1 to 10000 for display
```

IPv4 checks (F57, F97) are `Ipv4Addr::from_str`, which takes dotted quads only. Row numbers in F91 and F92 are built in `modules::e131`. Runtime rules these imply, on whatever owns running controllers: Stop cancels a pending open retry (F36, F100); a restart must resume a DMX Open writer (F108); changing Dummy Lighting's style swaps the data policy without a restart (F44); a malformed `RDS|` or `LAUNCHER|` command is logged and skipped (F130, F133).

### vixen-output: serial and network selection

```text
struct SerialPortChoice { path, label, in_use: bool, held_by: Option<String>, vendor_id, product_id, serial_number, product }
fn serial_port_choices(held: &[(String, String)]) -> Vec<SerialPortChoice>     // filtered, sorted, labelled; held = (path, controller name)
const BAUD_RATES: [i32; 11]                                                     // F14
fn parity_choices() -> Vec<(Parity, bool)>                                      // value, supported here
fn stop_bits_choices() -> Vec<(StopBits, bool)>                                 // no None
struct SerialConfigModel { current_label: String, group_enabled: bool, selected_path: Option<String>, baud, parity, data_bits, stop_bits, ports }
fn serial_config_model(current: Option<&SerialSettings>, ports: Vec<SerialPortChoice>) -> SerialConfigModel  // F9-F12, default 57600 N 8 One
fn validate_serial_config(draft: &SerialConfigModel) -> Result<SerialSettings, String>  // exact F20 message
fn serial_settings_label(s: &SerialSettings) -> String                          // F28, "Not Set" handled by the caller's None
struct MulticastChoice { id, name, text }                                       // text = "Multicast {name} ({id})"
fn multicast_choices() -> Vec<MulticastChoice>                                  // SystemConfiguration names intersected with the rule
fn primary_interface_id() -> Option<String>                                     // replaces "Local Area Connection"
```

### vixen-output: controller module data and setup

```text
struct GenericSerialData { ids, settings: SerialSettings, header: Option<String>, footer: Option<String> }
struct DummyLightingData { ids, render_style: RenderStyle, form_title: String }
struct E131Data { ... }  struct UniverseEntry { ... }
struct DdpData { ... }   struct RenardData { ... }  struct DmxProData { ... }  struct OpenDmxData { ... }
struct ElexolData { ... } struct RdsData { ... }    struct LauncherData { ... }
impl each { fn from_xml(&XElement) -> Self; fn to_xml(&self) -> XElement }     // preserve unknown members

enum ControllerSetupKind { GenericSerial, DummyLighting, E131, Ddp, SerialPortOnly, OpenDmx, Elexol, Rds, Launcher, None }
fn controller_setup_kind(type_id: &Uuid) -> ControllerSetupKind                 // F1 routing; SerialPortOnly = Renard, DMX Pro

fn generic_serial_apply(data: &mut GenericSerialData, settings: SerialSettings, header_on: bool, header: &str, footer_on: bool, footer: &str)  // F34
fn dummy_data_policy(style: RenderStyle) -> (Evaluator, Combinator)             // F44
struct DummyGrid { across: u32, down: u32, cells: u32 }
fn dummy_grid_layout(outputs: u32, style: RenderStyle) -> DummyGrid             // F47, ceil rows
fn dummy_frame_cells(style: RenderStyle, values: &FrameValues, out: &mut [u8])  // F48, one RGBA per cell
fn format_fps(updates: u64, elapsed_ms: u64) -> String                          // F49
fn debug_output_text(time: Duration, commands: &[OutputCommand], verbose: bool) -> Option<String>  // F134

struct UniverseRow { start: i32, active: bool, universe: i32, size: i32 }       // start is 1-based here
fn e131_rows_from_data(data: &E131Data) -> Vec<UniverseRow>                     // Start + 1, then recompute when automatic
fn e131_recompute_starts(rows: &mut [UniverseRow])                              // F62
fn e131_add_universe(rows: &mut Vec<UniverseRow>, manual: bool) -> Result<usize, String>  // F69
fn e131_max_to_add(rows: &[UniverseRow]) -> i32                                 // F70
fn e131_add_universes(rows: &mut Vec<UniverseRow>, count: i32, manual: bool)    // F70, one operation
fn e131_delete_rows(rows: &mut Vec<UniverseRow>, indices: &[usize], manual: bool)      // F68, F71
fn e131_delete_all(rows: &mut Vec<UniverseRow>)                                 // F72
fn e131_sort(rows: &mut [UniverseRow], column: UniverseColumn, ascending: bool) // F67, numeric
enum UniverseColumn { Start, Universe, Size, Active }
fn e131_validate_cell(column: UniverseColumn, text: &str) -> Result<i32, String>  // F62, F64, F65 messages
enum E131OkStep { Warn { title, text }, ReviewOrContinue { text }, Block { title, text } }
fn e131_ok_steps(rows: &[UniverseRow], destination: &E131Destination, output_count: u32, others: &[E131Peer]) -> Vec<E131OkStep>  // F83-F87
fn e131_output_names(rows: &[UniverseRow], unicast: Option<&str>, output_count: u32) -> Vec<String>  // F88, F89
fn e131_apply(data: &mut E131Data, rows: &[UniverseRow], options: E131Options, destination: E131Destination)  // F88
fn e131_destination_list(choices: &[MulticastChoice], unicasts: &[String], saved: &E131Destination) -> E131DestinationList  // F53, UNKNOWN path
fn e131_add_unicast(known: &mut Vec<String>, text: &str) -> Result<(), String>  // F54
fn e131_remove_unicast(known: &mut Vec<String>, text: &str, others: &[E131Peer]) -> Result<(), String>  // F55
fn e131_validate_repeat(text: &str) -> Option<i32>                              // F79
fn e131_validate_suppress(text: &str) -> Option<i32>                            // F80
fn e131_take_messages(controller: &Uuid) -> Vec<ControllerMessage>              // F90-F92, for the UI to present; never blocks the engine

fn ddp_apply(data: &mut DdpData, host_mode: bool, ip_text: &str, host_text: &str) -> Result<(), String>  // F97, blocking, call off the main thread
fn resolve_ipv4(host: &str) -> Option<Ipv4Addr>                                 // HostNameResolver
fn open_dmx_devices() -> Vec<OpenDmxDevice>                                     // F105, /dev/cu.* vendor 0x0403
fn open_dmx_match(saved: &OpenDmxDevice, devices: &[OpenDmxDevice]) -> Option<usize>  // serial, id, description
fn elexol_test(ip: &str, port: &str) -> ElexolTestResult                        // F112, up to 1 s, off the main thread
fn elexol_apply(data: &mut ElexolData, ip: &str, port: &str, min_intensity: i32) -> Result<(), String>  // F114
fn rds_field_rules(hardware: Hardware, url: &str) -> RdsFieldRules              // F118
fn rds_send(data: &RdsData, text: &str, force: bool) -> Result<RdsSent, String> // F124, real result
fn rds_expand_url(template: &str, text: &str, now: SystemTime) -> String        // F130
fn launch(executable: &str, arguments: &str, hide: bool) -> Result<(), String>  // F133
```

Runtime hooks the sheets need on whatever owns running controllers: `fn controller_data(id) -> ControllerData`, `fn controller_apply(id, ControllerData)`, which swaps data and restarts only if it was running (F88, F95); `fn controller_is_running(id) -> bool`; `fn controller_setup_opened(id)` and `fn controller_setup_closed(id)` (F51); `fn held_serial_ports() -> Vec<(String, String)>`; `fn e131_peers(excluding: Uuid) -> Vec<E131Peer>` (F86); `fn controller_status(id) -> Option<String>` (F36); `fn monitor_snapshot(id) -> Option<MonitorFrame>` (F46 to F49, F134), one bulk read per display refresh.

### vixen-output: filters

```text
fn color_breakdown_templates() -> &'static [&'static str]                       // F137 order
fn color_for_item(argb: u32) -> DotNetColor                                     // F140: exact R, Lime, B, White become known colours
fn color_breakdown_mix_note(items: &[ColorBreakdownItem], mix: bool) -> Option<String>  // F144
fn validate_index_value(text: &str) -> Result<u8, String>                       // F235, F236
fn shutter_apply(cfg: &mut ShutterFilterConfig, tag: &str, auto: bool, open: &str, close: &str) -> Result<(), String>
fn prism_apply(cfg: &mut PrismFilterConfig, tag: &str, auto: bool, open: &str, close: &str, function: &str) -> Result<(), String>  // saves Open
fn filter_display_name(cfg: &FilterConfig) -> String                            // F231
```

Applying a filter's config must atomically replace what the frame thread reads, without rebuilding the whole graph (F150).

### vixen-core: property data

```text
PropertyData::Grid { width: i32, height: i32 }                                  // today these are Unknown { raw_xml }
PropertyData::Face(FaceData)                                                    // phonemes, components, default colour
PropertyData::State(StateData)
fn add_patching_order(tree, nodes: &[NodeId])                                   // OrderModule.AddPatchingOrder
fn add_patching_order_zigzag(tree, nodes: &[NodeId], every: i32)                // Order/OrderModule.cs:68-94
fn property_defaults(type_id: &Uuid, tree, node: NodeId) -> PropertyData        // SetDefaultValues per module
fn property_clone_values(tree, from: NodeId, to: NodeId, type_id: &Uuid)        // CloneValues; Grid copies (F3)
fn state_next_name(base: &str, existing: &[String]) -> String                   // F211
fn state_normalize_name(name: &str, current: Option<&str>, existing: &[String]) -> Result<String, String>
```

### vixen-session: operations over the whole system

The helpers touch the element tree, the module store and the data flow graph, so they go here.

```text
fn color_helper_model(session, nodes: &[NodeId]) -> ColorHelperModel            // set names, preset from one property
fn color_helper_preflight(session, nodes: &[NodeId], request: &ColorRequest) -> ColorPreflight  // has_existing_properties, has_existing_filters, for F159, F160 alerts
fn color_helper_apply(session, nodes: &[NodeId], request: &ColorRequest, overwrite_properties: bool, overwrite_filters: bool) -> ColorReport  // F159-F161
fn color_report_text(report: &ColorReport) -> String                            // F161
fn color_sets_model(session) -> Vec<ColorSetRow>                                // F162
fn color_set_save(session, name: &str, colors: &[u32]) -> Result<(), String>    // F168
fn color_set_add(session, name: &str) -> Result<(), String>                     // F164
fn color_set_remove(session, name: &str) -> Result<(), String>                  // F165
fn color_set_differs(session, name: &str, colors: &[u32]) -> bool               // F169

fn dimming_helper_apply(session, nodes: &[NodeId], curve: Curve, behaviour: ExistingBehaviour) -> DimmingReport  // F244
fn dimming_report_text(report: &DimmingReport) -> String

struct OrderRow { node: NodeId, name: String }
fn order_helper_rows(session, nodes: &[NodeId]) -> Vec<OrderRow>                // F171
fn order_move(rows: &mut Vec<OrderRow>, from: usize, to: usize)                 // F172
fn order_reverse(rows: &mut [OrderRow], selected: &[usize])                     // F175
fn order_zigzag(rows: &mut [OrderRow], selected: &[usize], every: i32) -> Result<(), String>  // F176
fn order_zigzag_bounds(selected: usize) -> (i32, i32, i32)                      // default, min, max
fn order_helper_apply(session, rows: &[OrderRow])                               // F177

fn orientation_helper_apply(session, nodes: &[NodeId], value: Orientation)      // F180
fn grid_check(width: i32, height: i32, required: i32) -> Option<String>         // F183

fn face_helper_model(session, nodes: &[NodeId]) -> FaceTable                    // F185, F186, F189
fn face_toggle(cells: &[bool]) -> bool                                          // F187 bias rule
fn face_color_choices(session, nodes: &[NodeId]) -> Vec<Color>                  // F188, empty means free choice
fn face_helper_apply(session, table: &FaceTable)                                // F190

fn state_helper_begin(session, nodes: &[NodeId]) -> Result<StateDraft, String>  // F193
fn state_helper_commit(session, node: NodeId, draft: StateDraft)                // new property attaches only here
fn state_draft_issues(draft: &StateDraft) -> StateIssues                        // F208
fn state_common_discrete_color(session, node: NodeId) -> Color                  // F201
```

### vixen-core: layer dialogs

```text
fn limit_to_percent(limit: f64) -> i32                                          // truncates, F216
fn clamp_limits(lower: i32, upper: i32, changed: LimitEnd) -> (i32, i32)        // F216
fn chroma_key_from_dialog(lower: i32, upper: i32, key: Color, hue_tol: f64, sat_tol_percent: i32, transparent: bool) -> ChromaKeySettings  // derives saturation (2 places) and hue
```

### vixen-ffi

`crates/vixen-ffi/src/dialogs.rs` with `dialogs/*.rs`, thin UniFFI records and functions over the above, hung off a `DialogContext` that says whether a dialog writes to the open profile or to Display Setup's working copy. Slow calls (`serial_port_choices`, `elexol_test`, `ddp_apply`, `rds_send`) are `async` or called from a `@concurrent` Swift function.

## SwiftUI and AppKit structure

**Module.** `app/Sources/VixenDialogs`, importing `VixenKit` and `VixenBridge`, with `DialogsTestScene` as its test scene, as `docs/UI_GUIDE.md` names it. `VixenSetup` may use it directly. `crates/vixen-ffi/src/dialogs.rs` with `dialogs/*.rs` is its Rust side. Parity docs go in `docs/parity/` per dialog, including the "Unreachable, not ported" items, because a control missing from the table counts as not done.

**Presentation.**

- Public entry-point views in `EntryPoints.swift`, one for each route: `ControllerSetupSheet` (F1), `FilterSetupSheet` (F2), `PropertySetupSheet` (F3), `SetupHelperSheet` (F4, F6) and `LayerFilterSetupSheet` (F5). The owning window (Display Setup, the sequencer, the Layer Editor's window) attaches `.sheet(item:)`.
- Each sheet owns an `@Observable` draft built from a Rust model, and calls Rust on OK. No validation in Swift.
- Nested dialogs are nested sheets: Generic Serial then Serial Port Config; E1.31 then New Unicast Destination; Color Configuration then Color Sets; then the name prompt.
- Alerts are `.alert` on the sheet. `role: .destructive` on delete-all. The verbs are in each entry.
- Shared prompts, public in `VixenDialogs/Common.swift` so `VixenSetup` can use them: `NumberPromptSheet` (F246, `NumberDialog`: title, prompt, value, min, max, Return is OK, Esc is Cancel, `Vixen.Common/Controls/NumberDialog.cs:7-34`), `TextPromptSheet` (F247, `TextDialog`, `Vixen.Common/Controls/TextDialog.cs:20-44`), `ScrollingMessageSheet` (F93), `DiscreteColorSheet` (F248).
- Output windows are scenes, opened with `openWindow` when Rust reports a monitor started, and dismissed on stop: `WindowGroup("Controller Output", id: "controller-monitor", for: UUID.self)`. Don't restore them at launch (`.restorationBehavior(.disabled)`, `.defaultLaunchBehavior(.suppressed)`). Title comes from Rust.

**Control mapping.**

| Original | Mac |
|---|---|
| `ComboBox` DropDownList | `Picker` `.menu` |
| `ComboBox` DropDown that's meant as a list (Orientation, OpenDMX) | `Picker` `.menu` |
| `RadioButton` group | `Picker` `.radioGroup`, `labelsHidden` |
| `CheckBox` | `Toggle` `.checkbox` |
| `TextBox` | `TextField`; `SecureField` for F123 |
| `NumericUpDown` | `TextField` with `Stepper`, `.monospacedDigit()` |
| `TrackBar` | `Slider` with `step`, tick marks where upstream has them |
| `GroupBox` | `GroupBox` with the same title |
| `ColorPanel`, colour swatches | `ColorPicker` (the system colour panel); convert to sRGB 8-bit before calling Rust |
| Discrete colour restriction | `DiscreteColorSheet`, never the free panel |
| `TabControl` | `TabView` |
| Flat icon buttons | `Button` with SF Symbol, `.help`, accessibility label |
| `Grid`-laid forms | `Grid`, or `Form` with `LabeledContent` |
| `ToolTip`, `ToolTipText` | `.help` with the original text |
| `MessageBeep` | `NSSound.beep()` |
| Wait cursor | `ProgressView` beside the button |
| Mnemonics (`&OK`) | Dropped |

**Layout per dialog.** Keep each control's region. Default to `.fixedSize`-style sheets unless stated.

| Dialog | Regions, top to bottom |
|---|---|
| Serial Port Config (F7) | Current port line; group "Port Details" with five rows in `Grid`; OK, Cancel right |
| Generic Serial (F26) | Group "Serial Port Settings": "Currently:" + text, button right below; group "Packet Data": header toggle, indented field, footer toggle, indented field; OK, Cancel right |
| Dummy setup (F38) | "Window Title:" row; three radios (wrap the long labels); OK, Cancel right |
| E1.31 (F51) | Tabs. Universes: table left, vertical icon strip right of it; below, "Destination" with +/- at the right, then the picker across. Advanced Options: toggles, priority row, then two value + multi-line caption rows. OK, Cancel below the tabs, right |
| New Unicast (F56) | Field; radios; OK, Cancel right |
| DDP (F95) | Group "Configuration": radios row, label + field row; OK, Cancel right |
| OpenDMX (F104) | Picker across; OK, Cancel right |
| Elexol (F109) | Group "Ethernet Settings": two rows left, Test right spanning both; group "Advanced": label, slider, value; OK, Cancel right |
| RDS (F116) | Two columns. Left: group "RDS Hardware" (radios 2 x 2 with Serial Port to their right, URL row, macro hint, auth toggle, credentials row), group "Test Interface". Right: picture, group "Options", Save and Cancel. Status strip across the bottom |
| Launcher (F131) | Toggle; Save, Cancel right |
| Debug window (F134) | Text filling; Verbose at bottom right |
| Color Breakdown (F135) | Text; template row; scrolling rows (SwiftUI `ScrollView`, few rows); two toggles; Add Color left, OK and Cancel right. Resizable |
| Coarse Fine (F148) | Two toggles; two labelled sliders; OK, Cancel right |
| Tag family (F230-F236) | Tag row; toggle; value rows; function row; OK, Cancel right |
| Dimming Curve helper (F238) | Text; hint and button centered; question; radios; Help left, OK and Cancel right |
| Color helper (F151) | Heading; three blocks each with its radio then indented control; OK, Cancel right |
| Color Sets (F162) | `HSplitView`-free two columns: list with +/- below; group on the right; OK bottom right |
| Order helper (F170) | Table; OK, Cancel right. Resizable |
| Order, Orientation, Location, Grid | Rows, then OK and Cancel right. Grid's red message between the rows and the buttons |
| Face (F184) | Tabs filling; OK, Cancel right. Resizable |
| State mapper (F194) | Issue strip; name row; description; left pane and right pane with a divider; OK, Cancel right. Resizable, minimum 720 x 420 |
| Chroma (F212) | Colour left, tolerance right; brightness rows; toggle; OK, Cancel right |
| Luma, Mask And Fill | As the entries |
| Number prompt (F246) | Prompt left, field right on one row; OK, Cancel right |
| Text prompt (F247) | Prompt; field across; OK, Cancel right |
| Discrete picker (F248) | Swatch grid, up to 8 columns; Ok, Cancel right |
| Scrolling message (F93) | Heading; scrolling selectable text; OK right |

**Must be AppKit, via `NSViewRepresentable`.**

- **E1.31 universe grid (F61):** `NSTableView`. Add Multiple can create up to 63,999 rows. Row-number column, inline editors with a digit-only `NSFormatter`, header clicks (left and right) via `NSTableHeaderView`. Selection is per row on the Mac; "any selected cell" becomes "selected rows".
- **Order list (F171):** `NSTableView` with drag reorder and an insertion line. The owner already has over 1,100 leaves, and the guide says 50,000 must work. Rust does the reverse, zig-zag and renumber on the whole array.
- **Face grid (F185):** `NSTableView`, `allowsMultipleSelection`, plus cell-level selection tracked in the coordinator so the bias rule keeps working: click, ⌘-click, ⇧-click rectangle. Header cells draw the phoneme image above the text. Cache one `NSImage` per phoneme.
- **State assignment tree (F205):** `NSOutlineView` from `VixenKit`, which is virtualized.
- **Dummy monitor (F47):** an `NSView` with a `CALayer`. See "Performance hazards".
- **Debug text (F134):** `NSTextView`, replacing its storage no faster than the display.

**Everything else** is plain SwiftUI: these are short forms.

**Project settings:** `NSLocalNetworkUsageDescription` in Info.plist. `com.apple.security.device.serial` and `com.apple.security.network.client` if sandboxed.

**UI tests.** `VIXEN_UI_TEST` scripts for each sheet. Tests must not open real ports or send anything (`docs/UI_GUIDE.md`): gate port probing and Test or Send buttons behind `VIXEN_MUTE`.

## Performance hazards

- **Serial pacing, and this one is real for the owner.** 16 bytes at 9600 8N1 is 10 bits per byte: 16.7 ms per frame, 960 bytes/s capacity. `DefaultUpdateInterval` in the owner's file is 10 ms, which offers 1600 bytes/s. Upstream's `SerialPort.Write` blocks with no timeout (`G/Module.cs:61`; only Renard sets a timeout), which throttles the loop. On macOS `write` returns once bytes are in the tty queue, so frames can pile up and the lights lag further and further behind the music. **Do instead:** each serial controller gets its own writer thread that never blocks the frame thread. Keep only the newest frame. Before writing, check `TIOCOUTQ`, and skip the frame while more than one frame is queued. Airtime = bytes x (1 start + data + parity + stop) / baud.
- **Port scan on open (F8).** Upstream opens every port on the UI thread in the dialog's constructor. A Bluetooth or wedged port can stall the app for seconds. **Do instead:** sheet opens at once with names; probe in the background; ` (IN USE)` fills in; skip Bluetooth.
- **DNS (F97, F99, F60).** `Dns.GetHostAddresses` on the UI thread, and DDP re-resolves on every connect. `.local` names can take seconds. **Do instead:** off the main thread, always.
- **Dummy monitor (F46-F49).** Upstream does `BeginInvoke(Refresh)` per update and `FillRectangle` per output, "Not taking any optimizing steps to see how it does." (`D/DummyLightingOutputForm.cs:133, 139`). For the owner that's 3351 rectangles at up to 100 Hz. **Do instead:** Rust stores the latest frame. A display link reads one RGBA-per-cell buffer in one FFI call (57 x 59 cells is 13 KB), wraps it in a `CGImage` as `layer.contents`, `magnificationFilter = .nearest`, with one overlay layer holding the 1 px grid lines, redrawn only on resize. FPS in a `CATextLayer`. Never a SwiftUI `ForEach` per box.
- **Debug window (F134).** String concatenation per output per update, and a `BeginInvoke` each time (`B/DebugControllerOutputForm.cs:42-82`). **Do instead:** Rust formats into a reused buffer at most at display rate, only for the on-screen window.
- **E1.31 OK checks (F84, F85).** Overlap and duplicate checks are nested loops over all rows, and the first also parses text four times per pair (`SF:951-995`). At the 64,000 rows F70 allows, that's billions of comparisons on the UI thread. **Do instead:** sort once in Rust and compare neighbours, off the main thread if the table is big.
- **E1.31 row numbers (F61).** Every row paint measures a string and makes a new `SolidBrush`, undisposed (`Controls/DataGridViewNumbered.cs:30, 40`). **Do instead:** a plain text cell in `NSTableView`, which only asks for visible rows.
- **Generic Serial allocation.** A new packet array every update (`G/Module.cs:40`). **Do instead:** reuse one buffer (`modules::generic_serial::build_packet` already takes `&mut Vec<u8>`).
- **E1.31 Add Multiple (F70).** Each row rescans for the highest universe, recalculates every start, deselects every row: O(n²) (`SF:1107-1123`). **Do instead:** `e131_add_universes` in one pass, one table reload.
- **E1.31 packets, for reference.** Upstream patches a prebuilt buffer. Keep that. Note `SendTo` isn't guarded, so one failing send throws out of the update (`OP:794`).
- **Elexol reconnects every frame.** `if (_socket.Available == 0)` calls `OpenConnection` each update, closing and reopening the socket and resending the init bytes (`E/ElexolEtherIOModule.cs:30-37`). **Do instead:** connect once.
- **RDS.** A new `HttpClient` per send (`K/Module.cs:57`), and the serial path opens and closes the port twice per send (`K/RdsSerialPort.cs:13-20, 41-59`). **Do instead:** one shared client off the frame thread; a queue for sends.
- **Launcher.** `WaitForExit` on a pool thread per launch (`L/Module.cs:34`). **Do instead:** a background wait, never the frame thread.
- **Device lists at dialog open (F105, F119).** OpenDMX enumerates FTDI devices and RDS lists ports in the constructor, on the UI thread (`O/SetupDialog.cs:17`, `K/SetupForm.cs:18`). **Do instead:** the sheet opens at once and the list fills in; reading IOKit opens nothing.
- **Dimming helper at scale (F244).** One module instance, one graph edit and one filter add per leaf, on the UI thread, then a modal report (`CV/DimmingCurveHelper.cs:74-136`). The owner has over 1,100 leaves. **Do instead:** one bulk edit in Rust off the main thread, with a progress indicator; rebuild the graph once at the end.
- **Color helper at scale (F159-F161).** Setting `MixColors` and then `BreakdownItems` rebuilds outputs twice per leaf (`H/ColorSetupHelper.cs:248-249`, `C/ColorBreakdownModule.cs:55-63, 45-53`). The owner has 1117 of these. **Do instead:** set both once, in Rust, in bulk, off the main thread, with a progress indicator.
- **Colour set change event.** Every Color property subscribes to a static event (`H/ColorProperty.cs:13`), 1127 handlers for the owner, each invalidating a cache. **Do instead:** resolve sets by name at read time; no per-property subscription.
- **Order list.** Upstream's Reverse does `RemoveAt` and `Insert` per row, and each drop renumbers all rows (`Q/OrderSetupHelper.cs:74-79, 227-243`). **Do instead:** operate on the Rust array, reload visible rows.
- **Face grid.** `new Bitmap` per header paint, `new Pen` per cell paint, and a linear name search per row (`Z/FaceSetupHelper.cs:336, 353, 364, 288`). **Do instead:** cache images; use node ids.
- **Live mode slider (F150).** Each tick rebuilds outputs. **Do instead:** atomic config swap; never block the frame thread on a UI event.
- **State mapper.** Each item builds a full tree per definition (`T/Setup/ViewModels/StateItemViewModel.cs:26-29`), so memory grows as items x nodes. **Do instead:** one tree shared by all items plus a set of checked ids per item, computed in Rust.
- **Controller start from a sheet.** Applying settings restarts a controller. Sockets, ports and DNS must not run on the main thread, and the sheet closes straight away.

## Tests worth writing

Rust unit tests, values read from the code. No hardware, no network; use a pty for serial.

**Serial Port Config**
- `serial_config_model(None, ports)` gives 57600, None, 8, One, first port.
- `serial_config_model(Some(COM9...), ports)` keeps `current_label = "COM9"`, `selected_path = None`.
- `BAUD_RATES` equals F14 in order. Parity order None, Odd, Even, Mark, Space. Stop bits One, Two, OnePointFive.
- `validate_serial_config` with no port, baud -1, bits -1 returns exactly `The following items need to be resolved:\n\n* Port name has not been selected.\n* Baud rate has not been selected.\n* Invalid value for data bits.\n`.
- `/dev/tty.X` maps to `/dev/cu.X`. Hidden names don't appear.

**Generic Serial**
- XML round trip of the owner's entry above is byte-identical.
- `serial_settings_label` gives `COM9: 9600, None, 8, One`.
- `is_valid`: port null, baud 0, bits 0 or stop `None` are each invalid.
- Packet for header `AB`, footer `Z`, values `[1,2]`: `41 42 01 02 5A`. Non-ASCII header `é` sends `3F`.
- Header unchecked with text present stores empty.
- `RetryTimer`: a failure and two retries re-arm, the next gives up (already in `serial.rs`). After a failure then `stop`, `due` stays false: Stop cancels the retry, unlike upstream.
- A saved `COM9` with baud 0 opens the sheet without a panic and shows those values.

**Dummy Lighting**
- `dummy_grid_layout`, outputs to (across, rows needed): 1 (1,1), 2 (1,2), 3 (1,3), 4 (2,2), 5 (2,3), 10 (3,4), 17 (4,5), 25 (5,5), 3351 (57,59). Upstream's `down` for 10 and 17 was one short: 3 and 4.
- RGBMultiChannel with 10 outputs: squares `ceil(10/3)` = 4, across 2; only 3 are drawn, and output 10 is ignored.
- Monochrome value 128 gives grey 128. RGB single with no command gives black.
- `format_fps(100, 1000)` gives `100.00 fps`; `format_fps(1, 0)` gives `0.00 fps`.
- Policies per F44.
- Default data: "Dummy Lighting Controller", Monochrome. Blank title round-trips as `<FormTitle />`.

**E1.31**
- `e131_recompute_starts` sizes 510, 510, 100 gives starts 1, 511, 1021.
- Add on empty: universe 1, size 510, active. Add after universe 7, size 170: universe 8, size 170. Manual new row: previous start + size.
- `e131_max_to_add` highest 64000 is 0, 63990 is 10.
- Cell validation: universe 0 and 64001 fail with the exact text; 1 and 64000 pass. Start 0 and 100000 fail. Size 0 and 513 fail.
- Steps: output count 512, one row start 1 size 510: one `Warn` with `Total universe channel count of 510 does not match the controller output count of 512.` + blank + `Check your universe setup or adjust the amount of output channels to match.`
- Output count 10, row start 11 size 5: lines for start and for `Universe {u} 11 + 5 exceeds...`.
- Rows (1,510) and (510,510) overlap; (1,510) and (511,510) don't. Duplicate universes give `Universe numbers must be unique.` Inactive rows count in all of these.
- Peers: same unicast address conflicts; different unicast doesn't; different multicast ids don't; unicast versus multicast does.
- Output names: 10 outputs, universe 5, start 3, size 4, multicast: index 0 is `Output #1`, index 2 is `#3 5-1: Multicast`, index 5 is `#6 5-4: Multicast`. With unicast `10.0.0.5`: `#3 5-1: 10.0.0.5`.
- Multicast group: universe 1 is `239.255.0.1`, 256 is `239.255.1.0`, 64000 is `239.255.250.0`.
- Packet for 510 slots: 636 bytes; root flags-length `0x7000 | 620`; framing `0x7000 | 598`; DMP `0x7000 | 521`; property count 511; preamble `0x0010`; vector 4 then 2 then `0x02`; `"ASC-E1.17"` at 4; source `"Vixen 3"` at offset 44; priority at 108; sequence at 111; options 128 when blind at 112; universe big-endian at 113; start code 0 at 125.
- Repeat 2, suppress 3, identical frames starting from zeros: sent on frames 1, 4, 7, 10. Repeat 0: every frame. A changed frame resets both counters.
- `e131_validate_repeat`: `-1`, `100`, `ab` rejected; `0`, `99` accepted.
- Default data: starts at zero-based 1 until Setup. `Priority` missing from XML reads 100; `AutoPopulate` missing reads false.
- Zero rows with 10 outputs: one `Warn`, total 0, and nothing else.
- Destination list: multicast entries by name, then unicasts by address. A saved Windows GUID gives `Multicast UNKNOWN`, and applying it keeps the GUID. Nothing saved and no primary interface: nothing selected, and applying stores neither.
- Applying writes each row's `Unicast`, `Multicast` and `Ttl` 64 into the entry as well as the model. `Start` is zero-based in the entry.
- 200 automatic rows of 510 build without an error, though the last starts past 99999; a manual Start of 100000 still fails.
- Overlap and duplicate checks on 64,000 rows finish well under a second.

**DDP, Renard, DMX Pro, Elexol, RDS**
- DDP: 1441 outputs gives packets 1440, 1, offsets 0, 1440, flags `0x40`, `0x41`. 1440 outputs gives 1440, 0, the empty final packet carrying Push (upstream quirk; compare before changing it, `P/DDP.cs:164-171`). Header bytes 2 and 3 are 1 and 1. Port 4048.
- `ddp_apply` IP mode clears host name; host failure returns the exact text, including for an empty name and one over 255 characters. `10.1` and `::1` don't enable OK.
- Renard protocol 1: frame `7E`, `80 + chain`, value map `7D` and `7E` to `7C`, `7F` to `80`, pad `7D` at every 100th byte (`R/Protocol1.cs`).
- DMX Pro: 518 bytes, `7E 06 01 02`, start code 0, 512 values, `E7` (`X/Message.cs:27-44`). Member name `Partity` in XML.
- Elexol: buffer `A v B v C v`; value 1 with threshold 1 is off, 2 is on. Test bytes `60 69`. A saved intensity of 0 shows 0 and survives OK. Lowering to 0 lowers the runtime threshold.
- DMX Open: Stop then Start sends again.
- RDS: `rds_expand_url("http://x/?t={text}", "a b&c", _)` gives `http://x/?t=a+b%26c`. Serial frame `FE 20`, text, spaces to 64, `FF`. Field rules: MRDS 8, V-FMT212R 64, HTTP 128. Default URL as F120. V-FMT212R keeps a URL ending `?ACTION=UPDATE_RT&UPDATE_RT={TEXT}` (case ignored) and replaces any other. HTTP to V-FMT212R and back leaves auth on. Saved MRDS192 applies its rules at open: test text max 8, URL disabled.
- `RDS` with no `|`, `LAUNCHER|x` with no comma, and a character above 255 in a serial send are logged and skipped. `LAUNCHER|x,a,b` passes `a`, as upstream. A launcher command repeats only after a null command in between.

**Filters**
- Templates table of F138, and `Color.Lime` for green.
- `color_for_item(0xFFFF0000)` is known colour 141.
- Coarse fine, from upstream's tests: input `0x1234` gives `0x12`, `0x34`; range 0.5 gives `0x7F`, `0xFF`; mapping on with resting `A5`, `3C` and no input gives those; mapping off with no input gives no output; cancel after live restores `(false, 2, 3)`.
- `validate_index_value`: `0`, `255` pass; `256`, `-1`, `abc`, empty fail.
- Prism OK stores open and close separately.
- Dimming curve helper: for each `ExistingBehaviour`, counts on a small graph; report text exact; each new filter has its own curve. Element then one curve with a free output: Leave gives `0 Dimming Curves created, 0 configured, and 2 skipped.`, Update `0 ... 2 configured, and 0 skipped.`, Add new `1 ... 1 configured, and 0 skipped.` Element with a free output and Leave: 1, 1, 0. Element patched straight to a controller output with Leave: 0, 0, 0.
- Tag filter: a new Tag, Dimming or Color Wheel filter opens with OK disabled, as do Shutter and Prism. `filter_display_name` changes with the tag.

**Properties**
- Colour report text with counts, including the double spaces.
- Colour helper full colour order `GRWB`: items Green, Red, White, Blue, mixed. Multiple with default RGB set: names `ffff0000`, `ff00ff00`, `ff0000ff`. Single: name `Unnamed`.
- Colour helper over an element with a three-output breakdown, all free, and Overwrite: filters `0 added, 4 configured, 0 skipped`; Skip: `0 added, 0 configured, 4 skipped`. Silent mode still asks about existing properties and filters.
- Colour helper: a preset whose set is gone opens on the first set; a null set name leaves OK disabled; editing colour sets keeps the picked set.
- Colour sets: defaults RGBW, RGB in that order; save with a new name keeps the old; `color_set_differs` by count and by value.
- Order, from upstream's `OrderSetupHelperZigZagTests`, 1-based identities: 10 rows, selected 3,4,5,6, length 2: `1 2 3 4 6 5 7 8 9 10`. Selected 2,4,6,8, length 2: `1 2 3 4 5 8 7 6 9 10`. All 10, length 2: `1 2 4 3 5 6 8 7 9 10`. Selected 2,4,6,7,9,10, length 3: `1 2 3 4 5 6 10 8 9 7`. 16 rows, even ones, length 2: `1 2 3 4 5 8 7 6 9 10 11 12 13 16 15 14`. Length equal to selected: unchanged. Not divisible: exact error.
- Reverse: 6 rows, selected 2,3,5: `1 5 3 4 2 6`.
- Zig-zag bounds: 10 selected: (10, 2, 10). 200 selected: (50, 2, 200).
- `add_patching_order_zigzag(9 nodes, 3)`: orders `1 2 3 6 5 4 7 8 9`.
- Sort: nodes with no Order come last, stable.
- Grid: `grid_check(4, 3, 12)` is none; `grid_check(4, 2, 12)` is `The product of these needs to be 12`. Clone copies. `grid_values_clamped(20000, 1)` is `(10000, 1)`, and OK stays off with its message.
- Location: negatives load as 0. New property 1, 1, 1.
- Face: `face_toggle` `[true, true, false]` gives false, `[true, false]` gives true, `[false]` gives true. Colour not allowed becomes the first allowed. Two nodes with one name stay separate.
- State: `state_next_name("State", [])` is `State - 1`; with `State - 1` taken, `State - 2`. Blank definition name, duplicate names, and item names per F208. A new definition holds one item, `State Item - 1`. Upstream tests give many more cases.
- Discrete picker: the current colour starts selected when it's in the set; otherwise nothing is, and Ok is disabled.

**Layers**
- `limit_to_percent(0.29)` is 28, `0.57` is 56, `0.58` is 57.
- `clamp_limits`: lower 80 with upper 50 gives 50, 50.
- `percent_box_value("", 50)` is 50; `clamp_limits(100, 100, Lower)` is `(99, 100)`; `clamp_limits(0, 0, Upper)` is `(0, 1)`.
- Chroma: `chroma_key_from_dialog` saturation slider 25 gives `sat_tol` 0.25; key `(0,255,0)` gives hue 120, saturation 1.
- Mask And Fill clone keeps `RequiresMixingPartner`. Chroma clone keeps the flag.
- New `ChromaKeyData` has `TransparentOnZeroBrightness` false.

**Upstream tests covering this area** (`Vixen.Tests/`):

- `OutputFilter/CoarseFineBreakdown/CoarseFineBreakdownModuleTests.cs` 202, `CoarseFineBreakdownSetupViewModelTests.cs` 111
- `DDP/DDPDataTests.cs` 37, `Common/HostNameResolverTests.cs` 47
- `Setup/OrderSetupHelperZigZagTests.cs` 202, `Setup/SetupPatchingSimpleElementOrderTests.cs` 80
- `Sequencer/LayerMixingFilterDataJsonSerializationTests.cs` 126, `Sequencer/LayerImportExportServiceTests.cs` 420
- `ExportWizard/ControllerNetworkConfigurationTests.cs` 309 (network configuration, not setup)
- Next door, not setup dialogs: `Setup/SetupPatchingSimpleOutputOrderTests.cs` 342 and `Setup/SetupPatchingTestOutputModuleConsumer.cs`, `Common/ControllerTreeVirtualizationTests.cs` 400, `Core/OutputControllerOutputIndexTests.cs` 275
- `Property/State/`: `BroadcastStatePreviewPublisherTests.cs` 61, `StateAssignmentTreeSelectionTests.cs` 279, `StateAssignmentTreeTests.cs` 95, `StateDataCloneTests.cs` 187, `StateDefinitionNameDialogViewModelTests.cs` 62, `StateItemCopyNamingTests.cs` 218, `StateMapperDefinitionTests.cs` 1143, `StateMapperDraftTests.cs` 105, `StateMapperPreviewTests.cs` 385, `StateMapperTestCollection.cs` 9, `StateMapperValidationTests.cs` 436, `StateModuleCloneTests.cs` 73, `StatePreviewCoordinatorTests.cs` 253, `StateSetupHelperTests.cs` 169

No upstream tests exist for Generic Serial, Dummy Lighting, Serial Port Config, E1.31, Renard, DMX, Elexol, RDS, Launcher, the Color helper, Color Breakdown setup, or Dimming Curve helper.

## Windows-only omissions

- **`Modules\Controller\E131settings.xml` import** (`OP:822-826, 829-936`). A Windows relative path to a settings file from a much older plugin. Doesn't exist in a Mac profile. F90's import of per-universe destinations stays.
- **`SysIPAddress32`** (`Controller/E131/Controls/IPTextBox.cs:12`). A Win32 control class. Replaced by a validated text field.
- **`user32.dll MessageBeep`** (`Controller/E131/J1Sys/MessageBeepClass.cs:17-18`). Replaced by `NSSound.beep()`.
- **FTDI D2XX driver** (`Controller/OpenDMX/OpenDMX.cs:1, 10`). Replaced by the system serial driver, as F105 and F108 say. `SetDmxValue` and the unused members of the D2XX wrapper go with it.
- **`COM` prefix rule in `SerialPort.Open`.** .NET on Windows rejects a name that doesn't start with `COM`. Not a Mac rule: any `/dev/cu.*` path is valid, and `is_windows_port_name` does the reverse check.
- **`COM{n}` port names** (`Controller/RDSController/SetupForm.cs:176`, `Controller/RDSController/Module.cs:35`). No meaning on a Mac. `PortNumber` still round-trips.
- **`NetworkInterface.Id` GUIDs and the "Local Area Connection" name** (`SF:289-290`). Windows naming. Replaced by BSD names and the primary interface.
- **`CreateNoWindow`** (`Controller/LauncherController/Module.cs:31`). No equivalent. See F132.
- **Theme painting.** `ThemeUpdateControls`, `ThemeGroupBoxRenderer`, `ThemeComboBoxRenderer`, `ThemeToolStripRenderer`, `ThemeColorTable`, `ScalingTools`, every `groupBoxes_Paint` and `comboBox_DrawItem`, and the DPI handling in `SF:92, 112-113`. Native controls in dark appearance replace them.
- **WinForms window chrome.** `FormBorderStyle`, `MinimumSize` on fixed dialogs, `MaximizeBox`, `MinimizeBox`, `ShowInTaskbar`, `TopMost` on `UnicastForm`, `StartPosition`, `SizeGripStyle`, `ControlBox`, icons on dialogs, tab sizing in `SF:111-113`. Sheets don't have these.
- **Mnemonics** (`&OK`, `&Cancel`, `&Help`). macOS has no such thing.
- **`Cursor.Current = Cursors.WaitCursor`** (`P/DDPSetup.cs:94`, `Q/OrderSetupHelper.cs:113`). Replaced by progress indicators.
- **`Process.GetCurrentProcess` system info and `AboutBox`** (`Controller/E131/Help.cs:54-116`). Unreachable upstream as well.
- **`#if VIXEN21`** (`OP:618-620`) and **`TData`** (`Controller/RDSController/TData.cs`), a leftover from a native RDS driver. Dead on every platform.
- **`Encoding.UTF8` on `SerialPort`** (`G/Module.cs:153`). Only `Write(byte[])` is ever called, so it does nothing.
