# Administration window and app services

## Scope

Citation shorthand, used everywhere below. Paths are relative to `upstream/Vixen/src/`.

| Prefix | Path |
|---|---|
| `VA.cs`, `VA.Designer.cs` | `Vixen.Application/VixenApplication.cs`, `.Designer.cs` |
| other bare names (`Program.cs`, `DataZipForm.cs`, `VersionInfo.cs`, ...) | `Vixen.Application/` |
| `Updates/` | `Vixen.Application/Updates/` |
| `VS.cs` | `Vixen.Core/Sys/VixenSystem.cs` |
| `Paths.cs`, `AppCommand.cs`, `LatchedAppCommand.cs`, `IApplication.cs`, `Modules.cs`, `Execution.cs`, `SystemConfig.cs`, `Helper.cs` | `Vixen.Core/Sys/` |
| `State/` | `Vixen.Core/Sys/State/Execution/` |
| `MBF.cs`, `MBF.Designer.cs` | `Vixen.Common/Controls/MessageBoxForm.cs`, `.Designer.cs` |
| `XPS.cs` | `Vixen.Common/Controls/XMLProfileSettings.cs` |
| `App/` | `Vixen.Modules/App/` |

**Read in full, in scope (`Vixen.Application/`):** VixenApplication.cs 1384, VixenApplication.Designer.cs 632, Program.cs 102, VixenApplicationData.cs 218, DataProfileForm.cs 349, DataProfileForm.Designer.cs 283, SelectProfile.cs 175, SelectProfile.Designer.cs 145, ProfileItem.cs 44, DataZipForm.cs 371, DataZipForm.Designer.cs 352, InstalledModules.cs 105, InstalledModules.Designer.cs 144, ConfigPreviews.cs 355, ConfigPreviews.Designer.cs 273, OptionsDialog.cs 40, OptionsDialog.Designer.cs 219, AboutVixen.cs 24, AboutVixen.Designer.cs 109, CheckForUpdates.cs 159, CheckForUpdates.Designer.cs 192, ReleaseNotes.cs 82, ReleaseNotes.Designer.cs 103, CpuUsage.cs 109, VersionInfo.cs 161, KnownFolders.cs 36, NLogListener.cs 40, NLog.config 64, app.config 11. The ten `.resx` files of these forms hold only the stock sample data (`Name1`, `Color1`, `Bitmap1`, `Icon1`), no user-visible content. The one `.resx` in this area that matters is `Vixen.Modules/App/WebServer/Settings.resx`, which holds the Web Server dialog's picture (F111).

**`Updates/`:** CachedRelease.cs 4, GitHubRelease.cs 28, GitHubReleaseAsset.cs 13, GitHubUpdateService.cs 200, IUpdateService.cs 13, ReleaseNotesResult.cs 8, UnavailableUpdateService.cs 11, UpdateChannel.cs 8, UpdateCheckRequest.cs 7, UpdateCheckResult.cs 11, VixenReleaseVersion.cs 46.

**`Vixen.Core`:** Sys/VixenSystem.cs 469, Sys/Paths.cs 131, Sys/AppCommand.cs 342, Sys/LatchedAppCommand.cs 76, Sys/IApplication.cs 23, Sys/Modules.cs 533, Sys/Execution.cs 208, Sys/SystemConfig.cs 175, Sys/State/Execution/ (ClosedState 36, ClosingState 29, ExecutionStateEngine 71, OpenState 31, OpeningState 30, State 33, TestOpenState 33, TestOpeningState 30, Behavior/StandardClosedBehavior 9, StandardClosingBehavior 18, StandardOpenBehavior 9, StandardOpeningBehavior 25), Sys/Instrumentation/Instrumentation.cs 56, Instrumentation/ (IInstrumentation 9, IInstrumentationValue 11, InstrumentationValue 40, MillisecondsValue 53, PercentValue 41, CountValue 27, RateValue 40, DoubleValue 26, AveragePerCountValue 30, AveragePerSecondValue 28), Sys/Managers/PreviewManager.cs 139, Sys/Managers/OutputDeviceExecution.cs 254, Sys/Managers/OutputDeviceFacade.cs 60, Sys/Output/OutputPreview.cs 121, Sys/Output/BasicOutputModuleExecutionControl.cs 69, Sys/BuiltInElementTags.cs 79, Factory/PreviewFactory.cs 28, Module/App/ (AppModuleInstanceBase 58, AppModuleRepository 54, AppModuleImplementation 12, AppModuleDescriptorBase 54, IApp 10), Module/ModuleManagement.cs 45, Module/ModuleImplementation.cs 38, Module/Editor/IEditorUserInterface.cs 46, Module/SequenceType/SequenceTypeModuleDescriptorBase.cs 60, Services/ApplicationServices.cs 193, Services/EditorService.cs 51, Services/SequenceService.cs 191.

**`Vixen.Modules/App/`, every module:** Instrumentation/ (InstrumentationModule.cs 77, InstrumentationForm.cs 39, InstrumentationForm.Designer.cs 136, InstrumentationDescriptor.cs 38), CustomPropEditor/CustomPropEditorModule.cs 83, ExportWizard/ExportWizardModule.cs 88, Shows/ShowsModule.cs 67, SuperScheduler/SuperSchedulerModule.cs 159, SuperScheduler/ScheduleExecutor.cs 227, SuperScheduler/SuperSchedulerData.cs 59, TimedSequenceMapper/TimedSequencePackagerModule.cs 140, WebServer/Module.cs 174, WebServer/Settings.cs 82, WebServer/Settings.Designer.cs 202, WebServer/Data.cs 29, WebServer/Descriptor.cs 44, WebServer/WebHost.cs 63, FPPClient/Module.cs 21, LivePreview/Module.cs 86. Every module's descriptor. `Loading` and `Unloading` of ColorGradients/ColorGradientLibrary.cs, Curves/CurveLibrary.cs and LipSyncApp/LipSyncMapLibrary.cs. A repository-wide search for `AppCommand` found only the files named here, so that list of contributors is complete. The folders Fixture, FixtureSpecificationManager, Marks, Modeling, Polygon and TimingTrackBrowser build no `Module.App.*` assembly and have no descriptor, so they contribute nothing here.

**`Vixen.Common`:** Controls/XMLProfileSettings.cs 176, Controls/MessageBoxForm.cs 115, Controls/MessageBoxForm.Designer.cs 176, Controls/TextDialog.cs 50, Controls/TextProgressBar.cs 179, Controls/BaseForm.cs 14, BaseSequence/Sequence.cs (1-60 of 222), Help/Help.cs 92, AudioPlayer/AudioOutputManager.cs 315, AudioPlayer/AudioPreferences.cs 33, Preferences/PreferenceBase.cs 45.

**Tests and design notes:** `Vixen.Tests/Application/Updates/GitHubUpdateServiceTests.cs` 179. `upstream/Vixen/docs/reviews/github-update-release-notes-design.md` 328. `upstream/Vixen/docs/plans/admin/vix-3982-github-update-release-notes-design.md` (lines 1-80 of 158).

**Read partly, only to pin a contract:** `Vixen.Common/Controls/DragAndDropListView.cs` (`ListViewEx.ColumnAutoSize`, 191-203), `Vixen.Core/Sys/Managers/OutputDeviceCollectionExecutionMediator.cs` (1-150), `HardwareUpdateThread.cs` (150-190), `ContextManager.cs`, `ElementManager.cs`, `FilterManager.cs`, `OutputController.cs` (instrumentation registration lines), `Vixen.Core/IO/Xml/SystemConfig/XmlSystemConfigFilePolicy.cs` (the options), `Vixen.Core/IO/Loader/SequenceLoader.cs` (36-60), `Vixen.Modules/Sequence/Timed/TimedSequenceDescriptor.cs`, `Vixen.Modules/Sequence/Vixen2x/Vixen2xSequenceDescriptor.cs`, `Vixen2xSequenceTypeModule.cs`, `Vixen.Modules/Preview/VixenPreview/VixenPreviewDescriptor.cs`, `VixenPreviewModuleInstance.cs` (1-185), `Vixen.Modules/Effect/Video/Video.cs` (1175-1200), `TimedSequenceEditorForm.cs` (3213-3236, 5786-5800), `Vixen.Common/Controls/Wizard/Wizard.cs` (40-70), the window titles in `App/Shows/ShowListForm.Designer.cs`, `App/SuperScheduler/StatusForm.Designer.cs`, `App/SuperScheduler/SetupForm.Designer.cs` and `App/CustomPropEditor/Views/CustomPropEditorWindow.xaml`. Added at the audit: `Directory.Build.props` 40 and `Vixen.Application/Vixen.Application.csproj` 89 (no `<Product>`, F5), `Vixen.Installer/VixenFolders.wxs` (:34, the exe name), `Vixen.Core/IO/Xml/XElementFileWriter.cs` (15-118, F117, F118), `Vixen.Core/Sys/ModuleStore.cs` 29 and `Vixen.Core/Module/ModuleDataSet.cs` (61-72) (F119), `Vixen.Core/Sys/Managers/ContextManager.cs` (36-42, 97-107, 194-205) (F112), `Vixen.Core/Module/Editor/EditorModuleManagement.cs` (Q22), `App/SuperScheduler/StatusForm.cs` (37, 95-112, 150-196) (F56, F57), `App/WebServer/Settings.resx` (:121), `Vixen.Common/Controls/Theme/ThemeUpdateControls.cs` (22-45, label colours), `Vixen.Common/Controls/Theme/ThemeComboBoxRenderer.cs`, `Vixen.Common/Controls/TextDialog.Designer.cs`, `Vixen.Common/Controls/MessageBoxForm.Designer.cs` in full, `Vixen.Tests/Sys/BuiltInElementTagsTests.cs` 70, and the test names in `Vixen.Tests/LivePreview/`.

**Also read, because the Mac version builds on them:** `crates/vixen-io/src/appdata.rs`, `crates/vixen-io/src/paths.rs`, the `SystemData` part of `crates/vixen-io/src/system.rs`, `crates/vixen-session/src/app.rs`, `crates/vixen-ffi/src/core.rs`, `crates/vixen-ffi/src/lib.rs`, `crates/vixen-ffi/src/admin.rs`, `crates/vixen-effect-registry/src/lib.rs`, `app/Package.swift`, `app/Info.plist`, `app/Sources/VixenApp/VixenApp.swift`. The owner's real profile, read only: `VixenApplicationData.xml`, `logs/`, `SystemData/SystemConfig.xml` (options, previews, disabled devices), `SystemData/ModuleStore.xml` (Windows paths, scheduler and web server values). The docs pages `usage/general/profiles/`, `usage/general/zip-wizard/`, `usage/programming-api/command-line-api/`, `usage/web-interface/` on vixenlights.com.

**What already exists in the port.** Work is going on alongside this spec, so parts of it are already code, and this note is a snapshot from the audit. In Rust: `vixen-io` has `settings.rs`, `profiles.rs`, `lock.rs`, `audio_prefs.rs` and `datetime.rs`, and `appdata.rs` has the recent-list rules. `vixen-session` has `boot.rs`, `execution.rs`, `modules.rs`, `archive.rs`, `updates.rs`, `version.rs`, `instrumentation.rs`, `cpu.rs`, `logging.rs` and `preview_devices.rs`. `crates/vixen-ffi/src/admin.rs` still exports only the four recent-list calls and `max_recent_sequences`. Where a name below differs from that code the code wins, and where the code lacks a behaviour below the code changes. The gaps found at the audit: `modules.rs` has no `Tag Filter` row, puts `DMX Pro` before `Debugging Values`, and sorts effects case-sensitively (F86); previews have no `needs_reload` (F96); `ProfileSettings` does not reload a file changed on disk (F120); `AppService` has no reload hook (F119); there is no scheduler service (F56). In Swift: `app/Sources/VixenAdmin` has a first cut, `AdministrationScene`, `AdministrationView`, `SystemCommands`, `HelpCommands` and `ProcessStats`. Its placeholders do not match this spec and must change: `Logs` holds one `Open Logs Folder` item (F39), Start and Stop do nothing (F44, F45), Help has the wrong YouTube address and no Check for Updates or About Vixen (F59 to F62), the light is a `Circle` (F28), the CPU figure is computed in Swift and shows -1 (F33), and the Tools and Scheduler menus are missing (F35).

**Not read fully, in scope:** `Vixen.Application/NLog.xsd` (166 KB, the NLog schema, no behaviour). Nothing else.

**Named here but specified elsewhere.** This spec gives each one's entry point and what happens around it, not its contents: Display Setup (`Setup/DisplaySetup.cs`), the preview designer (`VixenPreviewSetup3`), the sequencer window, the Vixen 2.x importer, Custom Prop Editor, Export Wizard, the Sequence Package wizards, Shows, Schedules, Scheduler Status, and the Web Server's REST and hub code (`App/WebServer/Controllers`, `Hubs`, `Service`, `Startup.cs`, `wwwroot`).

## Features

### Conventions

**Treatment tags for odd behaviour** (`docs/PORTING_GUIDE.md` says port quirks as written, except crashes). **KEEP**: port as written and comment that it is deliberate. **GUARD**: upstream throws or hangs; do not panic, do the stated safe thing. **FIX**: a deliberate divergence, listed for the owner's veto in the quirk register (`Qn`) at the end of this section.

**Entry format.** *Do*: what the user does. *Does*: the exact behaviour. *Rules*: preconditions and enabling. *Default*. *Persist*. *Mac*: how it appears on the Mac. A missing line means none. **Nothing in this area has undo.** Upstream registers no undo action in any of these windows, so no entry has an Undo line. Setup Previews' Cancel, which reverts by reloading from disk, is the nearest thing (F96).

**Dialog buttons.** Upstream dialogs show OK then Cancel, left to right. On the Mac they stay in the same bottom-right region in the platform order, Cancel then OK, using `.cancelAction` and `.defaultAction`. Everything else keeps upstream's region and order.

**Upstream's message box.** `MessageBoxForm` decides what the entries below quote. Its button labels and results are:

| Constructed as (`MBF.cs:11-25`, `:34-62`) | Buttons | Result |
|---|---|---|
| `(text, title, false, false)`, or `MessageBoxButtons.OK` | OK | OK |
| `(text, title, true, false)`, or `MessageBoxButtons.YesNo` | YES, NO | YES returns OK, NO returns No |
| `(text, title, false, true)` | YES, CANCEL | YES returns OK, CANCEL returns Cancel |
| `MessageBoxButtons.OKCancel` | OK, CANCEL | OK, Cancel |
| `(text, title, true, true)`, or `MessageBoxButtons.YesNoCancel` | YES, NO, CANCEL | OK, No, Cancel |

YES always returns `DialogResult.OK` (`MBF.Designer.cs:46`). So `DialogResult == OK` in the C# means the user pressed YES. The title bar's close box on any of them returns `Cancel`, even on a YES and NO box, which is how F82 step 7's `== No` test can fall through to creating the folder. A Mac alert has no close box, so this cannot arise. On the Mac each is an `.alert` with the upstream title and text, and verbs on the buttons per `docs/UI_GUIDE.md`. Each entry names the verbs. Destructive verbs use `role: .destructive`.

**Threads.** Whatever touches the disk, the network or hardware runs off the main actor and reports back. Upstream blocks its UI thread in many of these places. `## Performance hazards` lists them.

### A. Launch, profile, data folder and shutdown

**F1. Launch order.** The steps below happen in this order. `Mac` says what each becomes. Steps marked omitted are listed under `## Windows-only omissions`.

| # | Upstream step | Cite | Mac |
|---|---|---|---|
| 1 | Log `Vixen app starting.` This first log call also loads `NLog.config`, so the log folder exists from here on | `Program.cs:25` | `logging::init` first, same message (F11, F113) |
| 2 | Catel log listener | `Program.cs:27` | Omitted |
| 3 | Install crash handlers | `Program.cs:28-29` | `install_crash_hooks` (F11) |
| 4 | Create two shared HTTP clients and `GitHubUpdateService` | `Program.cs:34-53` | `UpdateService` built lazily (F104) |
| 5 | Build the main form. Title is `Vixen Administration` | `VA.cs:58`, `VA.Designer.cs:559` | The `administration` window opens with everything disabled |
| 6 | Remember the UI thread and context | `VA.cs:60-61` | Omitted |
| 7 | WPF application object and Orc theme | `VA.cs:65-96` | Omitted |
| 8 | Clear the designer's placeholder rows, theme, fit big labels | `VA.cs:100-125` | The list starts empty; no fitting needed (F13) |
| 9 | Read the command line | `VA.cs:127-131` | F2 |
| 10 | Start the JIT profile in `%AppData%\Vixen` | `VA.cs:133-135`, `:357-373` | Omitted |
| 11 | If no `data_dir`: resolve the profile, which may show Select a Profile | `VA.cs:137-142`, `:611-682` | F3, F4, F5 |
| 12 | Create the root `AppCommand` on the main form | `VA.cs:143` | Menus are static (F35) |
| 13 | Load `VixenApplicationData`. This settles the data folder | `VA.cs:144-146`, `VixenApplicationData.cs:50-69` | F7 |
| 14 | Check and take the profile lock, or show an error and exit | `VA.cs:148-154` | F6 |
| 15 | Clear the update notice | `VA.cs:157` | Empty on launch |
| 16 | Subscribe to execution state; first status paint reads `Execution: Closed`, red | `VA.cs:160-161` | `AppModel` subscribes (F30) |
| 17 | Create the `Tools` menu and add `Options...` | `VA.cs:164-172` | Static, first in Tools (F47) |
| 18 | `Load` event: disable the four buttons and the whole menu strip, wait cursor | `VA.cs:413-414`, `:461-466` | F34 |
| 19 | Log the version and set the label colour, **await** the update check, up to 10 s, before anything else loads, and only then set the version label texts and log the .NET runtime version | `VA.cs:416`, `:504-540` | **FIX**: labels set at once; check runs in the background after step 38 (F29, Q1) |
| 20 | Progress `10 Starting up` | `VA.cs:418` | Same |
| 21 | `VixenSystem.Start`: state `Starting`, keep the client application, new `Instrumentation` | `VS.cs:39-47` | `AppCore` boot |
| 22 | On a pool thread: progress `10 Preparing modules` | `VS.cs:72-74` | Same, off the main actor |
| 23 | `Paths.DataRootPath = dir`: trim a trailing separator, create the folder or throw `IOException`, then create every `[DataPath]` folder | `VS.cs:77`, `Paths.cs:37-52`, `:64-73` | `DataPaths::ensure_directories` (F8) |
| 24 | Progress `20 Loading modules`. `Modules.LoadAllModules` scans `Module.*.dll` next to the executable. Each descriptor's `[ModuleDataPath]` folder is created | `VS.cs:95-98`, `Modules.cs:41-48`, `:91` | Modules are compiled in. Create the same folders (F8) |
| 25 | Progress `30 Loading system config`, then `LoadSystemConfig`: wait for any save, register the four `System ...` instrumentation values, build the managers, load the module store, progress `50 Module config loaded`, load the system config, progress `70 System config loaded`, add elements, nodes, controllers, previews and filters, seed built-in element tags, initialize data flow, drop orphaned preview module data, drop orphaned element tag assignments | `VS.cs:100-101`, `:230-287`, `:289-332` | `load_system`, then the two cleanups |
| 26 | `DefaultUpdateTimeSpan` from the loaded interval | `VS.cs:103` | Same |
| 27 | `Modules.PopulateRepositories`: creates every module instance. App modules run `Loading()` here, in module file order, filling Tools and Scheduler and starting services (F36) | `VS.cs:106`, `Modules.cs:419-431`, `Vixen.Core/Module/App/AppModuleRepository.cs:15-25` | Explicit `start_services` step (F36) |
| 28 | `no_controllers`: mark every output device disabled, controllers and previews alike | `VS.cs:108-110` | F2 |
| 29 | If a file migration happened: progress `80 Saving migrated config`, save both configs. This `await` is the lambda's first real yield, so it completes the outer task of `VS.cs:72`: upstream carries on with steps 31 to 41 on the UI thread while this save is still running | `VS.cs:112-118` | Same. **GUARD**: finish the save before step 31 (Q2) |
| 30 | Progress `70 Config loaded`. With no migration this follows `70 System config loaded` and steps 26 to 28. After a migration it arrives when the save ends, which can be after `80 Initializing editors` or even `100 Ready`, so the bar can move backwards or change while hidden | `VS.cs:120` | **KEEP** the texts; the bar may just hold |
| 31 | Unless `no_execution`: `Execution.OpenExecution()` | `VS.cs:51-53` | F37 |
| 32 | State `Started`, log `Vixen System successfully started.` | `VS.cs:55-56` | Same |
| 33 | If `Start` returned false: error alert, exit | `VA.cs:421-426` | F9 |
| 34 | Progress `80 Initializing editors`. Start the 1 s CPU timer, painting once at once | `VA.cs:428-429`, `:1203-1212` | F33 |
| 35 | Catel service registration | `VA.cs:431` | Omitted |
| 36 | Fill the New Sequence popup from sequence types that can be created | `VA.cs:432`, `:779-819` | F19 |
| 37 | Open dialog's initial folder is the profile's `Sequence` folder | `VA.cs:435` | F20 |
| 38 | Progress `90 Finalizing`. Add the Help menu, then the `Logs` submenu | `VA.cs:442-450` | F58, F39 |
| 39 | Progress `100 Ready`, hide the bar | `VA.cs:452-454` | Same |
| 40 | Fill the recent list, enable buttons and menus | `VA.cs:455-456` | Same |
| 41 | Bring the window to front after 1 s, default cursor | `VA.cs:457-458`, `:488-502` | Omitted; `NSApp.activate` once |
| 42 | `Shown` event. `Load` is `async void`, so this fires the first time `Load` yields, which on a release build is at step 19, long before step 41: development warning (F10), then front again after 1 s | `VA.cs:475-486` | F10 |

*Persist*: nothing is written during launch except the lock file, new empty folders, a migrated config, and logs. The exact log lines of this sequence are in F116.

**F2. Command line.** *Do*: pass `no_controllers`, `no_execution` or `data_dir=<folder>` as arguments. *Does*: every argument including the program name is split on `=` and the first part matched exactly (`VA.cs:127-131`, `:587-609`).
- `no_controllers`: `_disableControllers = true`. All output devices, previews included, count as disabled for this run (`VS.cs:108-110`). **KEEP**. A later save writes them all to `DisabledDevices` (F37).
- `no_execution`: `_openExecution = false`. The engine stays closed at launch.
- `data_dir=<folder>`: use this folder and skip profile resolution. `data_dir` alone sets it to null, which means resolve normally. The text after a second `=` is lost, because only `argParts[1]` is used (**KEEP**). With `data_dir`, the title stays `Vixen Administration`, `ProfileName` stays null, and the lock check still runs.

*Mac*: read `CommandLine.arguments`, same tokens. Add `VIXEN_DATA=<folder>` from `docs/UI_GUIDE.md`, which acts as `data_dir=`. An argument overrides the variable. Under `VIXEN_UI_TEST`, start no controllers, though previews start as normal; do not let that reach any save; send nothing to the network; start no web server (`docs/UI_GUIDE.md`, Building and testing).

**F3. Profile resolution at launch.** *Does*, when no `data_dir` was given (`VA.cs:611-682`, using `Settings.xml` section `Profiles`):
1. `ProfileCount` defaults to 0. If it is 0, return now. The data folder falls through to the default (F7). No title change, no `ProfileName`, no dialog, and no `Profile root` log line (`:616-620`). This is the first launch on the Mac, because the owner's `Settings.xml` stayed on Windows. It was also the owner's path on Windows: their real `Application.Info.log` goes from `Processing Profiles` straight to `Finished Processing Profiles`, so they have never used profiles.
2. Read `LoadAction` (default `LoadSelected`) and `ProfileToLoad` (default `-1`) (`:623-624`).
3. If `LoadAction != "Ask"` and `-1 < ProfileToLoad < ProfileCount`: read `Profile{n}/DataFolder` (default empty). If it is non-empty, exists and is not locked (F6), use it and set the title and `ProfileName` from `Profile{n}/Name` (`:627-636`). Otherwise show the load error (F4) and continue to step 4.
4. While there is no folder: show Select a Profile (F63). On Load: same three checks. Pass sets the title and breaks; fail shows F4 and loops. On close without loading: F5. Any other result throws `NotImplementedException`, unreachable (`:646-679`).
5. Log `Profile root : {dir}` and call `SetLogFilePaths` (`:680-681`). That call replaces the token `[VIXENPROFILEDIR]` in the log targets. `NLog.config` no longer contains the token, so logs stay in `Documents/Vixen 3/logs` for every profile. Omitted on the Mac.

`DateLastLoaded` is written only by Select a Profile's Load (F64), never by the automatic path in step 3 (**KEEP**).

*Mac*: `decide_startup_profile` returns what to do and Swift only presents it. The Administration window is on screen behind the Select a Profile sheet, all disabled. In upstream the main form has not been shown yet at this point; the sheet keeps the same modality with no ordering problems.

**F4. Profile load error.** *Does*: alert, title `Error`, error icon, centered on screen, one button, text `Selected profile {name} is locked by another instance!\n\nSelect a different profile to load or use the Profile Editor to create a new profile.` or, when not locked, `Selected profile {name} data directory does not exist!\n\n...` with the same second sentence (`VA.cs:684-693`). A locked and missing folder cannot both occur: `IsProfileLocked` returns false for a missing folder. *Mac*: button `OK`.

**F5. Leaving without a profile.** *Do*: close Select a Profile. *Does*: alert with `Application.ProductName` as title, warning icon, text `{product} cannot continue without a valid profile.\n\nAre you sure you want to exit {product}?`, YES and NO. YES calls `Environment.Exit(0)`; NO loops back to the dialog (`VA.cs:663-673`). The product name upstream is literally `Vixen.Application`: no project sets `<Product>` (`Directory.Build.props` sets only `<Title>`), so the SDK's default, the assembly name, applies, the same name as `Vixen.Application.exe` (`Vixen.Installer/VixenFolders.wxs:34`). *Mac* (Q21): `Vixen` in the title and text. A sheet has no close box, so Select a Profile gets a `Quit Vixen` button (F66) that raises this alert, with buttons `Exit` and `Keep Selecting`.

**F6. Profile lock.** *Does*: the file `.lock` in the data folder holds `{pid}@{machine}` (`VA.cs:31`, `:230-241`). `IsProfileLocked(path)` (`:295-355`):
- Folder missing, or no `.lock`: not locked.
- Text has no `@`, on either side: locked.
- Machine part differs from this machine: locked ("assume it is locked on some other device").
- Same machine: parse the pid. A parse failure lands in the outer `catch`: locked. Look the process up. Running and its name starts with `Vixen`: locked. Running under another name, or no such process: delete the lock file, not locked.
- Any other exception: log and report locked.

`CreateLockFile` (`:243-269`) creates the folder if missing, and succeeds only if `.lock` did not exist. It records the path in `Program.LockFilePath` for the crash path. At launch, `IsProfileLocked(dir) || !CreateLockFile()` shows alert `Profile is already in use or unable to the lock the profile.`, title `Error`, error icon, OK, then `Environment.Exit(0)` (`:148-154`). The typo stays. The check has a side effect: checking removes a stale lock, including from Select a Profile's list (F63).

*Mac*: same text format, and create with `O_CREAT|O_EXCL`. Machine name: the stable `LocalHostName` from SystemConfiguration, not `gethostname`, which changes with the network and would strand a crashed profile. Process lookup: `proc_name(pid)`, which gives `Vixen`. **GUARD**: a lock naming this process's own pid is stale; upstream would call it locked. The owner's copy of the show has no `.lock`, but one copied from Windows names another machine and locks the profile for good, with no way out in the UI (Q3). The archive wizard never archives `.lock` (F83). Running a second copy: F120.

*Persist*: `.lock` is removed at normal exit and on the crash path (`VA.cs:406`, `Program.cs:89-97`).

**F7. Choosing the data folder.** *Does*: `new VixenApplicationData(dir)` (`VixenApplicationData.cs:41-69`) sets `DataFileDirectory = dir`, which reads back as `Paths.DataRootPath` when null. Then: if `VixenApplicationData.xml` is absent there: if that folder exists keep it with empty data; if not, switch to the default folder and read its file if any. The app then adopts `DataFileDirectory` as its data folder (`VA.cs:146`). So a `data_dir` that does not exist silently falls back to the default folder (**KEEP**). The folder chosen in steps 3 and 4 of F3 always exists. *Default*: `~/Documents/Vixen 3` (`Paths.cs:54-57`; `DataPaths::default_root` already does this).

A damaged `VixenApplicationData.xml` never stops the app. No `DataFormatVersion` logs `VixenApplication: loading application data: couldn't find data format version` and bad XML logs `VixenApplication: error loading application data`; both carry on with an empty list (`VixenApplicationData.cs:71-99`), and the next save overwrites the file. **KEEP** on the Mac, with one trap: `vixen_io::load_application_data` returns `Err` for these (`crates/vixen-io/src/appdata.rs`, `application_data_from_xml`). `AppCore::start` in `crates/vixen-session/src/boot.rs` already logs it and uses `ApplicationData::new()`, so F9 does not fire. `AppCore::open` in `crates/vixen-session/src/app.rs` still passes it up with `?`, so launch must not go through `open`.

**F8. Data folders created at launch.** *Does*: creates, if missing, the root and these children: `SystemData`, `Sequence`, `Sequence/auto_backup`, `Media`, `Program`, `Template/Element`, `Effect Defaults` (the `[DataPath]` members: `SystemConfig.cs:23`, `Vixen.Core/Services/SequenceService.cs:13-14`, `Vixen.Core/Services/MediaService.cs:9-10`, `Vixen.Core/Sys/Program.cs:15-19`, `Vixen.Core/Sys/ElementNodeTemplate.cs:8-14`, `Vixen.Core/Services/EffectDefaults/EffectDefaultsService.cs:35`), and `Module Data Files/{Glediator, Picture, Shapes, Video, VixenDisplayPreview}` (each `[ModuleDataPath]`; relative paths join `Module Data Files`, `Paths.cs:109-118`). `Export` and `Fixtures` are made on demand. A failure to create the root throws `IOException` (`Paths.cs:44-50`), which on upstream is swallowed (Q2). *Mac*: `DataPaths::ensure_directories` covers all but the module folders; add those five. Failure raises F9. macOS asks once for access to the Documents folder, so add `NSDocumentsFolderUsageDescription` to `app/Info.plist`; a refusal lands in F9 too.

**F9. Startup failure.** *Does*: if `VixenSystem.Start` returns false: alert `An error occurred starting the system and the application will be halted.`, title `Error`, error icon, OK, then `Application.Exit()`. The code then carries on with the rest of `Load` anyway (`VA.cs:421-426`). **GUARD**: stop after the alert. Upstream's `Start` almost never returns false, because `LoadConfiguration` wraps its work in `Task.Factory.StartNew(async ...)`, and awaiting that only awaits the outer task (`VS.cs:72`). An exception inside is lost and startup continues half-loaded; `State/Behavior/StandardOpeningBehavior.cs:13` even null-checks for this. On the Mac any load error reaches this alert, with the error's own text logged (Q2).

**F10. Development build warning.** *Does*: only if `VersionInfo.IsDevBuild`, on `Shown`: alert `Please be aware that this is a development version. Some parts of the software may not work, and data loss is possible! Please backup your data before using this version of the software.`, title `Development/Test Software`, exclamation icon, OK (`VA.cs:576-585`). Test builds and release builds do not see it. *Mac*: same rule (F114 gives the version). A version of 3.13.x is a release, so the owner does not see it. Button `OK`.

**F11. Crash path.** *Does*: `Application.ThreadException`, `AppDomain.UnhandledException` and any exception out of `Main` go to `LogMessageAndExit` (`Program.cs:56-59`, `:68-99`): log Fatal with `An application error occurred. Please contact the Vixen Dev Team with the following information:\n\n` and the exception; if `VixenSystem.IsSaving()`, log `Save was in progress during the fatal crash. Trying to pause 5 seconds to give it a chance to complete.` and sleep 5 s; remove the lock (through the app, or the failsafe `Program.LockFilePath`); `Environment.Exit(1)`. No UI. *Mac*: a Rust panic hook and an `NSSetUncaughtExceptionHandler` do the same: log, 5 s wait if saving, remove the lock. Per `docs/PORTING_GUIDE.md` a render panic must not get here at all.

**F12. Shutdown.** *Do*: System > Shutdown and Exit (`VA.cs:1047-1050` just calls `Close()`), close the window, or on the Mac ⌘Q. *Does* (`VA.cs:375-409`):
1. If any open editor `IsModified`: alert `You have open editor(s) with unsaved changes, are you sure you want to close Vixen?\n\n If you choose yes, you will be prompted to save those editors as they are closed.`, title `Close Vixen?`, YES and NO, warning icon. NO cancels the close.
2. `_closing = true`. Call `CloseEditor()` on a copy of the open editor list. Each runs F32, with the Cancel button removed because `_closing` is true.
3. `while (VixenSystem.IsSaving())`: log a warning `Waiting for save to finish before closing.` and sleep 250 ms.
4. `_stopping = true`, so state changes no longer repaint the status strip (`VA.cs:1054`).
5. `VixenSystem.Stop(false)` (`VS.cs:125-143`): state `Stopping`, log `Vixen System stopping...`, drop the client application, `SaveDisabledDevices` (in memory only), `CloseExecution`, `Modules.ClearRepositories` (every app module's `Unloading`, F36), no save, state `Stopped`, log `Vixen System successfully stopped.`
6. `_applicationData.SaveData()` writes `VixenApplicationData.xml`.
7. Remove the lock. `Application.Exit()`.

**Neither `SystemConfig.xml` nor `ModuleStore.xml` is written at exit** (comment at `VS.cs:115`: "We no longer automatically save on close"). Only the explicit saves count, including the disabled-device list, so a controller toggled and never saved comes back as it was (**KEEP**).

**FIX** (Q4): the handler is `async void`, so `e.Cancel = true` runs after the `await` of step 1, when the close has already gone through. `ShowDialogAsync` here is the .NET 10 WinForms method, which posts the dialog and returns at once (no project defines its own). So with a modified editor upstream the form closes regardless, and the prompt and steps 2 to 7 race the end of the message loop; this is read from the code, not run. If they lose, the lock stays behind, and F6 clears it at the next launch because the pid is dead. With no modified editor there is no yield at all, because `Stop(false)` never awaits (`VS.cs:135-138`), and steps 2 to 7 all run inside `FormClosing`. The Mac uses `applicationShouldTerminate` returning `.terminateLater`, so NO really cancels. *Mac*: closing the Administration window runs this same flow, as the main form's close does upstream, through `windowShouldClose` and `NSApp.terminate`. Alert buttons: `Close Vixen` and `Keep Open`.

### B. The Administration window

**F13. Window.** *Does*: title `Vixen Administration`, or `Vixen Administration - {name} Profile` when a profile was chosen through F3 (`VA.cs:695-699`, `VA.Designer.cs:559`). Client 456x539, minimum 472x578, starts centered, resizable (`VA.Designer.cs:549`, `:556-558`). Regions, top to bottom: menu strip; title panel 30% of the height; button panel 70% (`:539-540`); status strip. Button panel: two equal columns, then a progress bar across both (`:439-451`). On resize the Sequences group grows and the recent list with it (`VA.cs:1322-1334`), and the big labels are refitted by trial and error (`:1356-1382`). *Mac*: `Window("Vixen Administration", id: "administration")`, the title set through `.navigationTitle` or the `NSWindow`. Same regions and order. `minWidth` and `minHeight` keep the original proportion. The text fits with `.minimumScaleFactor` (F15, hazard H2). Menus go to the menu bar. Dark appearance as elsewhere. Remembering the frame with the `NSWindow` autosave name is harmless; upstream does not remember it.

**F14. Logo.** *Does*: `Properties.Resources.V3Logo`, zoomed, top-left, spanning all three rows of the title panel, in a column 40% wide (`VA.Designer.cs:459`, `:504-513`). *Mac*: `VixenResources.image("images/V3Logo.png")`, already bundled, scaled to fit.

**F15. "Vixen" label.** *Does*: bold, fitted to its cell, top right of the title panel, spanning the three right-hand columns of a 70% row (`VA.cs:118-119`, `VA.Designer.cs:472`, `:515-527`). *Mac*: `Text("Vixen")`, bold, large, `.minimumScaleFactor`.

**F16. Release label.** *Does*: `VersionInfo.VersionName`: `Release 3.13u4`, `Development Build #1501` or `Test Build` (`VA.cs:534`, `VersionInfo.cs:119-134`). Below the Vixen label, in the third column, row 2. Yellow on development builds, red on test builds, otherwise the highlight text colour (`VA.cs:511`, `:519`, `VA.Designer.cs:482`). Its font is kept equal to the Build label's (`VA.cs:120-125`). Until step 19 of F1 finishes it reads the placeholder `Release `, and the Build label `Build`. *Mac*: from `version_info()`, set at once. Same colours.

**F17. Build label.** *Does*: `VersionInfo.BuildName`: `Build #1371`, or `Build #` when the build number is 0 (`VA.cs:535`, `VersionInfo.cs:148-159`). Below the Release label, same colour rule. *Mac*: from `version_info()`.

**F18. Sequences group.** *Does*: group box `Sequences`, left column, holding in order `New Sequence...`, `Open Sequence...`, the label `Recent Sequences:`, and the list (`VA.Designer.cs:285-316`). *Mac*: `GroupBox("Sequences")` with the same four, in this order.

**F19. New Sequence...** *Do*: click. *Does*: if the popup built from sequence types has exactly one item, perform it without showing anything; otherwise pop it up under the button (`VA.cs:885-897`). The popup holds one item per sequence type whose descriptor has `CanCreateNew`, labelled with the type name (`VA.cs:779-819`). Only `Timed Sequence` qualifies; `Vixen 2.x Sequence` returns false (`Vixen.Modules/Sequence/Vixen2x/Vixen2xSequenceDescriptor.cs:61-65`). So in practice the click directly calls `EditorService.CreateEditor(".tim")`, which finds no file by that name, creates a new sequence and opens the editor (`Vixen.Core/Services/EditorService.cs:19-50`, F31). The new sequence's `FilePath` is empty until first saved (`Vixen.Common/BaseSequence/Sequence.cs:21`). If no editor is found: alert `Can't find an editor to open this file type. ("{fileType}")`, title `Error opening file`, error icon. Unlike F20 there is no `try` around `CreateEditor` here (`VA.cs:793-815`), so an exception goes to the crash path and ends the app (F11). **GUARD**: show F20's error alert instead. *Rules*: disabled until startup finishes (F34). *Mac*: `Button("New Sequence...")` and File > New Sequence... (⌘N), which call `openWindow(id: WindowID.sequencer, value: SequenceDocumentRef.new())`. Keep the one-item shortcut; list the types from Rust, so a popup appears only if a second creatable type ever exists.

**F20. Open Sequence...** *Do*: click. *Does*: file dialog, multiple selection allowed (`VA.Designer.cs:107`), starting in the profile's `Sequence` folder (`VA.cs:435`). The dialog object is reused for the session (`VA.Designer.cs:44`), so it keeps its last file name and later opens tend to start in that file's folder. Filter, built in module file order (`VA.cs:902-912`): `All Sequence Types (*.tim;*.vix;)`, `Timed Sequence (*.tim)`, `Vixen 2.x Sequence (*.vix)`, `All files (*.*)`. Each chosen file goes through `OpenSequenceFromFile` in turn (`VA.cs:919-922`, `:926-956`):
- `EditorService.CreateEditor(file)` returns null: alert `Can't find an editor to open this file type. ("{file name}")`, title `Error opening file`, error icon.
- Throws: log, alert `Error trying to open file '{full path}'.`, same title. A `.vix` import cancelled by the user lands here, because `Vixen.Core/IO/Loader/SequenceLoader.cs:44-45` hands back null and `Vixen.Core/Services/SequenceService.cs:82` dereferences it (**KEEP**).
- Otherwise F31.

Nothing checks whether the file is already open. `CreateEditor` always builds a new sequence and a new editor (`Vixen.Core/Module/Editor/EditorModuleManagement.cs:13-19`), so the same file can sit in two windows with separate undo and saves. On the Mac `openWindow(id:value:)` with an equal `SequenceDocumentRef.existing(path:)` focuses the open window instead (Q22).

On error the wait cursor is never reset (`VA.cs:928`, `:945`); not applicable on the Mac. Vixen 2.x files run the importer, a custom loader (`Vixen.Modules/Sequence/Vixen2x/Vixen2xSequenceTypeModule.cs:48-68`, specified with the importer). *Rules*: disabled until startup finishes. *Mac*: `NSOpenPanel` with `allowsMultipleSelection`, `directoryURL` the `Sequence` folder on the first open of a session and the last folder used after that, an accessory popup with the same four filter names (default the first), and File > Open Sequence... (⌘O). Add `.vix` to `Info.plist` as an imported type; only `.tim` is declared now. Dropping `.tim` or `.vix` files on the Sequences group, and Finder's Open With, take the same path; if startup is not finished, queue them until it is.

**F21. Recent Sequences list.** *Does*: label `Recent Sequences:`, then a one-column list with no header, one selection at a time, full row (`VA.Designer.cs:318-348`). Shows `Path.GetFileName(path)`, most recent first (`VA.cs:1158-1175`). A stored path whose file does not exist is skipped, not removed (`:1165-1166`). The column is 82% of the list's width (`:1177-1185`). Filled at step 40 of F1 and after every F32. *Default*: empty until a sequence has been closed. *Persist*: `VixenApplicationData.xml` (`## Data and persistence`).

**Windows paths on the Mac.** The owner's file holds 20 entries with Windows paths under two different prefixes, `C:\Users\developer\OneDrive\Documents\Vixen 3\Sequence\` and `C:\Users\userb\Documents\Vixen 3\Sequence\`, and six names occur under both. Upstream's `File.Exists` would drop all 20 on a Mac. The rule, in Rust:
1. A stored path that is absolute in POSIX form and names an existing file is used as is. `ApplicationData::resolve_recent_sequences` does not do this: it sends every entry to `<profile>/Sequence/<file name>`, which would misdirect a sequence opened from another folder. Add it there or wrap it.
2. Anything else, a Windows path included, resolves by file name in this profile's `Sequence` folder (the existing behaviour, using `windows_file_name`).
3. Keep rows that exist. Drop later rows that resolve to the same file as an earlier one, first wins.

A Windows entry for a file in a subfolder of `Sequence` loses the subfolder under rule 2, so it hides, or shows a same-named file from the top folder. The owner's 20 entries are all directly in `Sequence`, so this does not arise for them; accept it and leave the stored string alone.

The owner's real data gives 14 rows, in this order: Coldest Winter, Carey Grinch, Christmas Eve Sarajevo, Dance of the Sugar Plum Fairy, Carol of the Bells, Snoopy Christmas, Christmas Intro 1, Wizards in Winter, The Night Before Christmas, Do They Know It's Christmas, Christmas Wrapping (Glee), Christmas Can-Can, Zoom, Carol of the Bells (Lindsey Stirling), each `.tim`. All 14 exist in the copy.

Stored strings stay untouched until they fall off the end of the list or are cleared, so the file still works on Windows. *Mac*: a small SwiftUI `List` is fine; there are at most 20 rows. Also File > Open Recent with the same rows and its Clear Menu item, which runs F23.

**F22. Open a recent sequence.** *Do*: double-click a row. *Does*: if the file exists, `OpenSequenceFromFile`; otherwise alert `Can't find selected sequence.`, title `Error`, error icon, OK (`VA.cs:1117-1135`). Nothing else opens it upstream. *Mac*: `contextMenu(forSelectionType:primaryAction:)` gives double-click and Return. The check runs on the resolved path, not the stored string.

**F23. Clear Recent Sequences.** *Do*: right-click the list, choose `Clear Recent Sequences` (tooltip `Clears the Recent Sequence list`) (`VA.Designer.cs:202-215`, `VA.cs:177`). *Does*: empties the stored list and the rows (`VA.cs:1311-1315`). No confirmation. **It does not save**: the file is written at the next F32 or at exit (**KEEP**). The menu is on the whole list, including its empty area and with nothing selected. *Mac*: `.contextMenu` on the list; `.help` with the tooltip.

**F24. System Configuration group.** *Does*: group box `System Configuration`, right column, top aligned, `Setup Display` above `Setup Previews` (`VA.Designer.cs:378-435`). *Mac*: `GroupBox` with the two buttons.

**F25. Setup Display.** *Do*: button or System > System Configuration > Setup Display. *Does* (`VA.cs:991-1028`): opens Display Setup modally. On return, disables buttons and menus and shows the bar. OK: `0 Preparing Display Setup Changes`, `form.ApplyConfirmedChanges()`, `50 Saving System Configuration`, `SaveSystemConfigAsync`, `100 Saving Module Configuration`, `SaveModuleConfigAsync`. Otherwise: `0 Reloading System Configuration`, `ReloadSystemConfig` on the UI thread, then front. Finally hide the bar and re-enable. Reload closes and reopens the engine if it was open (`VS.cs:334-343`). *Rules*: disabled until startup finishes and while this runs. The dialog is modal to the whole app, so the sequencer windows are unusable while it is up, and they must stay so on the Mac: Cancel replaces the whole element tree. *Mac*: `openWindow(id: "display-setup")`, with an `AppModel.systemConfigSession` that disables the Administration controls, the menu commands and sequencer editing until it ends. The window's contents have their own spec. A failed save: F117. What Cancel's reload does to the app modules: F119.

**F26. Setup Previews.** *Do*: button or System > System Configuration > Setup Previews. *Does*: opens Setup Previews (F89) modally (`VA.cs:958-989`). OK: disable, `0 Saving System Configuration`, `SaveSystemConfigAsync`, `50 Saving Module Configuration`, `SaveModuleConfigAsync`, re-enable. Otherwise, even if nothing changed: `0 Reloading Configuration`, `ReloadSystemConfig`, front. **FIX** (Q5): reload only if something changed, because a reload stops and restarts every controller and preview (H4). A tick or untick (F90) counts as a change here even though it sets no `_changesMade`, because upstream's reload is what puts the running states back after Cancel. *Mac*: a sheet on the Administration window, under the same `systemConfigSession` as F25, since Cancel can replace the system data under an open sequencer. A failed save: F117. What the reload does to the app modules: F119.

**F27. Progress bar.** *Does*: spans both columns under the groups. A bar with centered text: value 0-100, `CustomText` in the middle, `Color.DarkGreen` (0,100,0) fill (`VA.cs:116-117`, `VA.Designer.cs:269-281`, `Vixen.Common/Controls/TextProgressBar.cs:123-173`). Visible from launch until `100 Ready`, and during F25 and F26. Hidden otherwise, and the row stays reserved at 11.6% of the panel (`VA.Designer.cs:449-450`). The texts are listed in F1, F25 and F26. *Mac*: `ProgressView(value:)` with the text overlaid, `.tint` dark green. Fed by a Rust progress callback, hopped to the main actor. Keep the row's space when hidden with `.opacity`, so the layout does not jump.

**F28. Execution light.** *Does*: 22x22 solid cell at the far left of the status strip. Colour by state (`VA.cs:1080-1095`): `IsOpen` (Open or Opening) ForestGreen (34,139,34); `IsClosed` (Closed or Closing) Firebrick (178,34,34); `IsInTest` (Opening for testing or Open for testing) DodgerBlue (30,144,255); otherwise Gold (255,215,0), which no state reaches. Nothing calls `OpenTest` (a search finds no caller), so blue is also unreachable; keep all of it, since `docs/PORTING_GUIDE.md` says to port as written. *Mac*: a small filled `RoundedRectangle` in those colours, in a bottom bar inside the window, with `accessibilityLabel` carrying the state. The colour comes from Rust (`ExecutionStatus`).

**F29. Update notice.** *Does*: status strip, after the state label. Empty unless the startup check finds a newer version: ` Release version {version} available.` or ` Development build {version} available.`, with the leading space, also logged (`VA.cs:542-560`). Test builds never check (`VA.cs:515-522`). Channel and rules are in F104. Failure leaves it empty. Not clickable. *Mac*: same text, set when the background check returns. Skipped under `VIXEN_UI_TEST`.

**F30. Execution state text.** *Does*: `Execution: {Execution.State}`, with a right border, after the light and a two-space spacer (`VA.cs:1078`, `VA.Designer.cs:224-237`). Before the first update, the designer text is `Execution: Unknown`; the first update comes in the constructor. Updates on every state change, on the UI thread, except once shutdown has started (`VA.cs:1052-1061`). The handler re-reads the state rather than taking it from the event (comment at `VA.cs:1074-1075`). State names are in F37.

**F31. Opening an editor.** *Does*: `_OpenEditor` adds it to the open list, subscribes `Closing` and `Activated`, calls `StartEditor` (`VA.cs:821-828`). On `Activated`: tell the previous editor `EditorLostActivation`, remember this one, `EditorGotActivation` (`VA.cs:830-835`). `IApplication.ActiveEditor`, `AllEditors` and `ApplicationId` (GUID `7b903272-73d0-416c-94b1-6932758b1963`) exist; a search finds no reader outside `ApplicationServices.GetActiveEditor`, which itself has no caller. *Mac*: `AppModel` keeps the open editor list; the sequencer area reports open, activate and close through it.

**F32. Closing an editor.** *Do*: close a sequence window. *Does* (`VA.cs:837-881`):
1. If `IsModified`: alert `Save changes to the sequence {name}?`, title `Save Changes?`, question icon, centered on screen, YES, NO and CANCEL (no CANCEL when `_closing`). CANCEL vetoes the close. YES calls `editor.Save()`.
2. Else if `IsEditorStateModified`: `editor.Save()`, silently.
3. Remove from the open list; `_activeEditor = null` even if another editor was active.
4. `AddSequenceToRecentList(editor.Sequence.FilePath)`: remove every exact match, insert at the front, cut to 20 (`_maxRecentSequences`), `SaveData`, refill (`VA.cs:1115`, `:1137-1156`).
5. Unsubscribe, then `editor.EditorClosing()`.

So a sequence joins the list when it closes, never when it opens (**KEEP**). A never-saved sequence adds an empty string, which uses up a slot and stays hidden. **GUARD**: skip empty paths. *Mac*: the same order. Duplicates are matched by resolved file as well as by exact string, so a new `/Users/.../Foo.tim` removes `C:\...\Foo.tim`; otherwise the real list would hold both. Buttons `Save`, `Don't Save`, `Cancel`. `docs/spec/sequencer-shell.md` chooses a `WindowGroup` over `DocumentGroup`, so this alert stays, as a sheet on that window; if that decision reverses, the system's own save prompt replaces it, and steps 3 to 5 stay.

**F33. CPU readout.** *Does*: right aligned, taking the spare width (`VA.Designer.cs:239-247`). Reads `Resource Usage` until the stats timer starts at step 34 of F1, then `CPU: {n}%` every 1000 ms (`VA.cs:1199-1235`). `CpuUsage.GetUsage` returns -1 on its first call, so the label shows `CPU: -1%` for the first second. Value: this process's CPU time since last time over all CPUs' kernel plus user time since last time, as a percentage, cast to `short` (`CpuUsage.cs:36-85`). It skips the calculation if under 250 ms passed. On Windows kernel time includes idle, so it is percent of the whole machine. Its 64-bit assembly from `FILETIME` is wrong: `(UInt64)(a.dwHighDateTime << 32)` shifts an `int` by 32, which C# treats as a shift of 0, and `dwLowDateTime` sign-extends (`CpuUsage.cs:87-93`). The result is unreliable, and can read near 0 whenever the low word changes sign. **FIX** (Q6): compute what was meant: process CPU time delta ÷ (wall time delta × logical CPUs) × 100, truncated. And keep showing `Resource Usage` until the first real sample, rather than `-1`. *Mac*: `getrusage(RUSAGE_SELF)` in `CpuUsage`, 1 Hz, in its own small `@Observable` so the tick does not invalidate the window. Use `.monospacedDigit()`.

**F34. Enabling during startup and busy periods.** *Does*: `EnableButtons(false)` disables `New Sequence...`, `Open Sequence...`, `Setup Display`, `Setup Previews` and the whole menu strip (`VA.cs:461-466`). Off from step 18 of F1 until step 40, and during the save or reload part of F25 and F26. The recent list stays live, so a double-click during startup would run (upstream's list is empty then, so it cannot). *Mac*: `AppModel.isBusy` drives the same controls and every command in the System, Tools, Scheduler and File menus. Help stays usable only after startup, as upstream, since it is added at step 38.

### C. Menu bar, modules and the execution engine

**F35. Menu bar at run time.** *Does*: the designer gives one top-level menu, `System` (`VA.Designer.cs:112`, `:123-126`). The rest arrives at run time, in this order: `Tools` in the constructor (`VA.cs:164-169`), `Scheduler` when the Shows module loads during `VixenSystem.Start` (`App/Shows/ShowsModule.cs:38-43`), `Help` after that (`VA.cs:444`). Final order: **System, Tools, Scheduler, Help**. Full contents, assuming upstream's module file order (F36):

```
System
  Logs ▸                     one item per file in the log folder (F39)
  View Installed Modules     F40
  Profiles...                F41
  System Configuration ▸     Setup Display (F42), Setup Previews (F43)
  Execution Engine ▸         Start (F44), Stop (F45)
  ─────────
  Shutdown and Exit          F46 (upstream text "Shutdown and E&xit")
Tools
  Options...                 F47
  Custom Prop Editor         F48
  Export Wizard              F49
  Instrumentation...         F50
  Import Sequence Package    F51
  Export Sequence Package    F52
  Web Server                 F53
Scheduler
  Shows                      F54
  Schedules                  F55
  ─────────
  Enabled (toggle)           F56
Help
  Online Help                F58
  Vixen YouTube Channel      F59
  Check for Updates          F60
  Release Notes              F61
  About Vixen                F62
```

The only keyboard shortcut in the whole window upstream is the `&x` mnemonic on Shutdown and Exit. *Rules*: the whole strip is disabled by F34.

**How upstream builds it.** `AppCommand` (`AppCommand.cs`) wraps a `ToolStripItem`. The root is `new AppCommand(form)` (`VA.cs:143`); adding a child to the root also puts it on the form's `MainMenuStrip` (`:194-211`, `:34-65`). A command's `Name` is its id and `Text` its label. Text `-` makes a separator and clears the name (`:111-121`). `Find("a\\b")` walks direct children by name, with `\` as delimiter (`:300-326`). `Remove(name)` looks only at direct children (`:213-233`). `Add` and `Remove` marshal to the UI thread, because modules load on a pool thread (`:196-200`). `Click` forwards the item's click (`:291-298`). `Enabled`, `Visible`, `Image` and `MergeIndex` pass through. `LatchedAppCommand` adds `CheckOnClick` and a `Checked` event with the new state (`LatchedAppCommand.cs`). A module receives the root through `IApplication.AppCommands`, assigned just before `Loading()` (`Vixen.Core/Module/App/AppModuleRepository.cs:22-24`). Every module uses the same pattern: `Find("Tools")`, create it if null, add its command (`InstrumentationModule.cs:48-62`).

| Module file | Command id | Text | Parent | Cite |
|---|---|---|---|---|
| `VA.cs` | `Tools` | Tools | root | `VA.cs:164-169` |
| `VA.cs` | `Options` | Options... | Tools | `VA.cs:170-172` |
| CustomPropEditor | `CustomPropRoot` | Custom Prop Editor | Tools | `App/CustomPropEditor/CustomPropEditorModule.cs:58-60` |
| ExportWizard | `ExportWizard_Main` | Export Wizard | Tools | `App/ExportWizard/ExportWizardModule.cs:56-58` |
| Instrumentation | `Instrumentation_Main` | Instrumentation... | Tools | `App/Instrumentation/InstrumentationModule.cs:58-60` |
| TimedSequenceMapper | `SequenceImportExportRoot` (both share this id) | Import Sequence Package, then Export Sequence Package | Tools | `App/TimedSequenceMapper/TimedSequencePackagerModule.cs:55-60` |
| WebServer | `WebserverRoot` | Web Server | Tools | `App/WebServer/Module.cs:75-106` |
| Shows | `SuperSchedulerRoot` | Scheduler | root | `App/Shows/ShowsModule.cs:38-43` |
| Shows | `ShowEditor` | Shows. A separator `s1` comes first only if the menu already has items | Scheduler | `App/Shows/ShowsModule.cs:45-58` |
| SuperScheduler | `SuperSchedulerRoot` | Scheduler, created if Shows has not | root | `App/SuperScheduler/SuperSchedulerModule.cs:101-110` |
| SuperScheduler | `SuperSchedulerCreateSchedule` | Schedules | Scheduler | `App/SuperScheduler/SuperSchedulerModule.cs:116-126`, `:139` |
| SuperScheduler | `s1` | separator | Scheduler | `:140` |
| SuperScheduler | `SuperSchedulerEnabled` | Enabled (latched) | Scheduler | `:128-135`, `:141` |

The `Unloading` of Instrumentation, ExportWizard and TimedSequenceMapper call `Remove` on the root, where their commands are not, so those removals do nothing; TimedSequenceMapper also passes display names. None of this matters at shutdown.

*Mac*: no plug-ins, so no `AppCommand` tree. The menus are fixed `.commands` on `AdministrationScene` in `app/Sources/VixenAdmin`, where `SystemCommands` and `HelpCommands` already are, in the order above, with `Window` and `Help` where macOS puts them: **Vixen, File, Edit, System, Tools, Scheduler, Window, Help**. Each item opens a scene with `openWindow(id:)` and a `WindowID` constant, so `VixenAdmin` needs no import from another area (`app/Package.swift` forbids that between areas). In a build without that area the item just does nothing, so `admin` still builds alone. Upstream's sequencer has its own menu strip with `Sequence, Edit, View, Tools, Help` (`Vixen.Modules/Editor/TimedSequenceEditor/TimedSequenceEditorForm.Designer.cs:370-375`). On the Mac there is one menu bar, so when a sequencer window is key its Tools items are added below these, after a divider, and the System, Tools and Scheduler items here stay available. The sequencer spec owns its side of that.

**F36. App modules at load.** *Does*: `Modules.LoadAllModules` gets `Directory.GetFiles(dir, "Module.App.*.dll")`, which on NTFS is alphabetical, into a `HashSet` that keeps insertion order. `PopulateRepositories` adds each, which runs `Loading()` on the pool thread. An exception in one is only logged, as `Error occurred while adding module {file}` (`Modules.cs:41-48`, `:419-431`). So the order is the table's. `PopulateRepositories` walks the module types in reflection order, which nothing pins, but no other type adds menus, so only the order within App matters. This order is what puts Custom Prop Editor before Web Server, and Shows above Schedules. The sort is NTFS's, which ignores case, so `DebugController` comes before `DMXUsbPro` (F86).

| File, in load order | Adds | `Loading()` | `Unloading()` |
|---|---|---|---|
| Module.App.ColorGradients | nothing | nothing | nothing |
| Module.App.Curves | nothing | nothing | nothing |
| Module.App.CustomPropEditor | Custom Prop Editor | registers services, adds menu, reads config | removes menu |
| Module.App.ExportWizard | Export Wizard | adds menu | forgets the wizard |
| Module.App.FPPClient | nothing | nothing; "no UI or background service" (`App/FPPClient/Module.cs:6-10`) | nothing |
| Module.App.Instrumentation | Instrumentation... | builds its form **eagerly**, adds menu (`InstrumentationModule.cs:17-21`) | disposes form |
| Module.App.LipSyncApp | nothing | `MigrateMapsToProperties` (`App/LipSyncApp/LipSyncMapLibrary.cs:15-18`, `:282`) | nothing |
| Module.App.LivePreview | nothing | live preview service and six broadcast subscriptions (`App/LivePreview/Module.cs:37-43`, `:55-74`) | disposes |
| Module.App.Shows | Scheduler menu, Shows | adds menu | nothing |
| Module.App.SuperScheduler | Schedules, separator, Enabled | adds menu, creates `ScheduleExecutor`, runs `CheckSchedule` once (`SuperSchedulerModule.cs:22-30`) | `Enabled = false`, which hides the status window and logs `Schedule Disabled` |
| Module.App.TimedSequenceMapper | Import, Export | registers services, adds menu | no-op removal |
| Module.App.WebServer | Web Server | adds menu, creates `WebHost`, starts it if enabled (`App/WebServer/Module.cs:42-47`) | removes menu, stops server, releases its context |

*Mac*: `start_services` runs the non-menu `Loading` work explicitly, in this order, after the system config loads and before the engine opens (F1 steps 27 and 31). The real log confirms the order: `Webserver started.` before `entering the open state`. Each service's start failing is logged, never fatal. At shutdown: stop in the same list order.

**F37. Execution engine.** *Does*: a state machine with six states, names exact, because `Execution: {name}` shows them: `Closed`, `Opening`, `Open`, `Closing`, `Opening for testing`, `Open for testing` (`State/*.cs`). It starts in `Closed` without running `Enter` (`State/ExecutionStateEngine.cs:7-17`).
- `OpenExecution` calls `OnOpen`. Only `Closed` reacts, going to `Opening`. Every other state ignores it (`State/ClosedState.cs:27-30`, `State/State.cs:20-22`).
- `CloseExecution` calls `OnClose`. `Open` and `Open for testing` go to `Closing`. Every other state ignores it.
- `OpenTest` goes from `Closed` to `Opening for testing`. It has no caller.
- A change runs `Leave`, sets the state, raises `StateChanged`, then `Enter` (`State/ExecutionStateEngine.cs:21-31`). So listeners see `Opening`, then `Open`, inside one synchronous call, on the caller's thread.
- `Opening.Enter`: raise the Windows timer resolution; if the device facade exists, start every output device not in `DisabledDevices`; go to `Open`. `Open.Enter`: start the system clock (`State/Behavior/StandardOpeningBehavior.cs:8-24`, `StandardOpenBehavior.cs`).
- `Closing.Enter`: restore the timer resolution, release every context (so all playing sequences stop), stop all output devices, go to `Closed`. `Closed.Enter`: stop the clock (`State/Behavior/StandardClosingBehavior.cs:5-17`, `StandardClosedBehavior.cs`).
- The log lines, each starting `Vixen execution engine `: `entering the open state...`, `entered the open state.`, `entering the closing state...`, `entered the closed state.`, `entering the testing state...`, `entered the testing state.` (`State/*.cs`).
- `IsOpen` is `Open` or `Opening`; `IsClosed` is `Closed` or `Closing`; `IsInTest` is the two test states (`Execution.cs:98-111`).

Device start and stop (`Vixen.Core/Sys/Managers/OutputDeviceExecution.cs`): each manager (controllers, previews) has its own run state. `StartAll` runs only from `Stopped`; `StopAll` only from `Started`. Devices start one at a time on the calling thread, a comment says deliberately. One device's failure is logged and the rest continue. A device whose update thread reports an error is stopped and logged, `Device {name} experienced an error during execution and was shutdown.` Adding a device while the manager is `Started` starts it at once (`OutputDeviceCollectionExecutionMediator.cs:89-106`). A single `Start` does nothing unless the manager is running (`OutputDeviceExecution.cs:169`, `:187-194`). Controllers go first, then previews, both starting and stopping, because that is the order they join the facade (`VS.cs:257-260`, `OutputDeviceFacade.cs:19-32`). Stopping waits for each device's update thread, with a timeout, so closing blocks as starting does (`OutputDeviceExecution.cs:222-233`, `HardwareUpdateThread.cs:75-80`).

Stop then Start restarts what `DisabledDevices` says, not what was running before: a preview unticked (F90) or a controller stopped in Display Setup since the last save comes back, because nothing refreshes the list until a save (`State/Behavior/StandardOpeningBehavior.cs:17-19`) (**KEEP**). Closing also releases every context, the web server's included (F112).

The sequencer listens too: on `Closing` it closes its context; on `Open` it opens a new one (`TimedSequenceEditorForm.cs:3215-3229`).

**Disabled devices.** `SaveDisabledDevices` records the id of every device not running (`VS.cs:145-150`). It runs at `Stop` (memory only) and at every `SaveSystemConfigAsync`, but only tests `_state == Started`, which is the system's state, not the engine's (`VS.cs:177-180`). **FIX** (Q7): if the user stops the engine and then anything saves the system config (Options OK, Setup Display OK, Setup Previews OK), upstream records every device as disabled, and at the next launch nothing starts. The Mac snapshots only while the engine is open. The owner's real file disables 3 of 4 devices.

`ReloadSystemConfig`: remember `IsOpen`, close, `LoadSystemConfig`, reopen if it was open (`VS.cs:334-343`). `SaveSystemConfigAsync` returns false with an error log if one is already running, and likewise the module save; `IsSaving` reports either (`VS.cs:32-35`, `:161-228`).

*Mac*: `ExecutionEngine` in `crates/vixen-session/src/execution.rs`, wrapping `vixen-engine::Engine`, `vixen-output::ControllerRunner` and the running previews. `ShowPlayback::open_execution`, `close_execution` and `execution_state` in `crates/vixen-session/src/playback.rs` delegate to it, so the sequencer shares it. Same six names and same event order. Called off the main actor, because starting a device can block on a serial port (H7). Events reach Swift asynchronously.

**F38. Standard Mac menus.** *Does*: added per `docs/UI_GUIDE.md`, no upstream counterpart unless noted. **Vixen**: About Vixen (F103; `CommandGroup(replacing: .appInfo)`), Settings… ⌘, (F97), Services, Hide, Hide Others, Show All, Quit Vixen ⌘Q (F12). **File**: New Sequence... ⌘N (F19), Open Sequence... ⌘O (F20), Open Recent ▸ (F21), Close ⌘W. Closing the Administration window runs F12. **Edit**: the standard text commands for the dialogs' fields. **Window**: standard.

**F39. System > Logs.** *Do*: choose a file name. *Does*: at step 38 of F1, one item per file directly in `Documents/Vixen 3/Logs` (`di.GetFiles()`, so no subfolders; a missing folder throws) (`VA.cs:438-450`). The list is not refreshed until the next launch. Choosing one starts `notepad.exe {path}` (`VA.cs:1101-1111`). The owner's folder lists `Application.Error.log`, `Application.Fatal.log`, `Application.Info.log`, `Application.Warn.log`, `Scheduler.Info.log`; `archive/` is left out. *Mac*: `list_log_files()`. Choosing one calls `NSWorkspace.shared.open`, the system's editor for logs. **GUARD**: a missing folder gives an empty, disabled submenu. Files sort by name, as NTFS gives them. Folder: F113.

**F40. System > View Installed Modules.** Opens F86 modally (`VA.cs:1189-1195`). *Mac*: `openWindow(id: "installed-modules")`.

**F41. System > Profiles...** *Do*: choose it. *Does*: opens Data Profile (F67) modally. On OK: alert `You must re-start Vixen for the changes to take effect.`, title `Profiles Changed`, information icon, OK. Cancel: nothing (`VA.cs:1239-1250`). The running session does not change. *Mac*: a sheet on the Administration window, then the alert.

**F42. System > System Configuration > Setup Display.** Same as the button (F25) (`VA.cs:1252-1255`).

**F43. System > System Configuration > Setup Previews.** Same as the button (F26) (`VA.cs:1257-1260`).

**F44. System > Execution Engine > Start.** `Execution.OpenExecution()` (`VA.cs:1037-1040`). *Rules*: enabled when `!IsOpen` (`VA.cs:1097`), so also in the test states, where it does nothing. *Default*: the engine opens at launch, so Start starts disabled. Nothing saved; nothing prompts. *Mac*: runs off the main actor.

**F45. System > Execution Engine > Stop.** `Execution.CloseExecution()` (`VA.cs:1042-1045`). *Rules*: enabled when `!IsClosed` (`VA.cs:1098`). Stops every output and playing sequence with no prompt (**KEEP**). The same rule pair comes from Rust as `ExecutionStatus`.

**F46. System > Shutdown and Exit.** Runs F12. The mnemonic has no meaning on the Mac; the label is `Shutdown and Exit`.

**F47. Tools > Options...** Opens Options (F97) (`VA.cs:170-172`, `:1063-1072`). *Mac*: `openSettings`, the same scene as ⌘,.

**F48. Tools > Custom Prop Editor.** `new CustomPropEditorWindow().Show()`: modeless; every click opens another window (`App/CustomPropEditor/CustomPropEditorModule.cs:64-70`). Title `Custom Prop Editor`. *Mac*: `WindowGroup(id: "custom-prop-editor", for: UUID.self)`, a new id each time. Contents in its own spec.

**F49. Tools > Export Wizard.** `_exportWizard.Start(true)`, modal, creating it if needed from the module's static data (`App/ExportWizard/ExportWizardModule.cs:31-44`). When finished: on Cancel `_data.ActiveProfile = null`; drop the wizard (`:70-78`). This is the owner's route to FSEQ. *Mac*: `openWindow(id: "export-wizard")`. Contents in its own spec.

**F50. Tools > Instrumentation...** Opens F108 (`InstrumentationModule.cs:39-46`).

**F51. Tools > Import Sequence Package.** Modal wizard with a new `ImportConfig` each time (`App/TimedSequenceMapper/TimedSequencePackagerModule.cs:80-85`, `:108-116`). Nothing persisted by the module. *Mac*: `openWindow(id: "sequence-package-import")`.

**F52. Tools > Export Sequence Package.** Modal wizard on a deep copy of the static data. On OK copies `ExportOutputFile`, `ExportIncludeAudio` and `ExportSequenceFiles` back; on any finish calls `SaveModuleConfigAsync` (`TimedSequencePackagerModule.cs:73-78`, `:93-106`, `:118-126`). The default file name uses `VixenSystem.ProfileName`, which is null with no profile (`App/TimedSequenceMapper/SequencePackageExport/SequencePackageExportOutputStage.cs:40-44`, `:72`). *Mac*: `openWindow(id: "sequence-package-export")`.

**F53. Tools > Web Server.** Opens F111.

**F54. Scheduler > Shows.** `ShowListForm(_data)`, modal; result ignored (`App/Shows/ShowsModule.cs:45-55`). Title `Shows`. *Mac*: `openWindow(id: "shows")`. Contents in the scheduler spec.

**F55. Scheduler > Schedules.** `SetupForm(_data)`, modal; result ignored (`SuperSchedulerModule.cs:116-126`). The menu says `Schedules`, but the window's title is `Scheduler` (`App/SuperScheduler/SetupForm.Designer.cs:251`). *Mac*: `openWindow(id: "schedules")`.

**F56. Scheduler > Enabled.** *Do*: click, which toggles the check. *Does*: `_data.IsEnabled = checked`, `_executor.CheckSchedule()`, `SaveModuleConfigAsync()` (`SuperSchedulerModule.cs:128-135`). The tick is set from the stored value before the handler is attached, so loading does not fire it (`:129-130`). *Default*: false (`SuperSchedulerData.cs:13`); the owner's is false. *Persist*: the module store, at once. *Mac*: `Toggle("Enabled", isOn:)` in the `CommandMenu`, which shows a check.

**Off then on does not resume the schedule** (**KEEP**). Off runs `Stop(false)`, which sets `ManuallyDisabled` and stops any running item at once, then hides the status window (`ScheduleExecutor.cs:192-196`, `:208-219`). On again sets `Enabled`, shows the window and puts every item back to `Waiting` (`:183-190`), but only the status window's start button clears `ManuallyDisabled` (`:221-225`, `App/SuperScheduler/StatusForm.cs:185-189`). Until it is pressed no item is processed (`ScheduleExecutor.cs:199`) and the status reads `All Show Disabled` (`ScheduleExecutor.cs:127-130`). The flag is not saved, so a restart clears it. Rust's scheduler service keeps the same flag, separately from `IsEnabled`.

**F57. Scheduler Status window.** *Does*: visible exactly while the schedule is enabled (`ScheduleExecutor.cs:50-53`, `:167-178`), so enabling it, or launching with it enabled, opens a window by itself. Title `Scheduler Status`. Its close box is refused (`App/SuperScheduler/StatusForm.cs:109-112`), so unticking Enabled is the only way to hide it. Each move stores `StatusForm_Position` in the module data (`:104-107`), read back on load (`:37`), and that reaches the disk only with the next module save; the owner's is 90,204. The executor checks every `CheckIntervalInSeconds` (default and minimum 1) and logs through logger `Scheduler` (`ScheduleExecutor.cs:12`, `:85-95`, `SuperSchedulerData.cs:19-33`). *Mac*: `Window(id: "scheduler-status")` opened and closed by the same flag, with `windowShouldClose` returning false. Contents in the scheduler spec.

**F58. Help > Online Help.** Opens `https://www.vixenlights.com/docs/` in the browser (`VA.cs:1272-1275`, `Vixen.Common/Help/Help.cs:62-63`, `:66-75`). *Mac*: `openURL`. Help items appear only after startup.

**F59. Help > Vixen YouTube Channel.** Opens `https://www.youtube.com/user/VixenLightsSoftware` (`VA.cs:1277-1280`, `Vixen.Common/Help/Help.cs:56-57`).

**F60. Help > Check for Updates.** Opens F105 (`VA.cs:1282-1287`). *Rules*: disabled when `VersionInfo.IsTestBuild` (`VA.cs:203-207`).

**F61. Help > Release Notes.** Opens F107 with a tag: `DevBuild-{BuildNumber}` on a development build, `ReleaseVersion` on a release, otherwise null (`VA.cs:1289-1297`).

**F62. Help > About Vixen.** Opens F103 (`VA.cs:1299-1304`).

### D. Select a Profile

**F63. Window and list.** *Does*: title `Select a Profile`, fixed size, centered on screen. Label `Select a Profile to Load:`, a list, then `Profile Editor` on the left and `Load` on the right (`SelectProfile.Designer.cs:52-127`). List (`SelectProfile.cs:97-134`): for each `i < ProfileCount`, `Profile{i}/Name` (default `New Profile`), `/DataFolder` (default empty), `/DateLastLoaded` (default `DateTime.MinValue`), `IsLocked = IsProfileLocked(folder)`. Sorted newest first, with no tie rule. A locked row reads `{name} - Locked` in the disabled colour, with no focus rectangle (`:32-53`). Selecting a locked row clears the selection at once (`:55-72`). Nothing is selected at first. *Default*: `Load` is the default button. *Mac*: a sheet on the Administration window. `List` with single selection; a locked row is `.selectionDisabled()` and `.foregroundStyle(.secondary)`. Stable sort (`docs/PORTING_GUIDE.md`). **GUARD**: a date that does not parse counts as `MinValue`; upstream's `Convert.ToDateTime` throws. The row data, including locks, come from Rust, off the main actor.

**F64. Load.** *Do*: click `Load`, press Return, or double-click a row. *Does*: nothing if the selection is empty or locked. Otherwise take the folder and name, write `Profile{n}/DateLastLoaded = DateTime.Now` to `Settings.xml` using the row's original index, return OK (`SelectProfile.cs:135-167`). F3 then rechecks the folder and lock. *Mac*: `Load` is disabled with no selection. Date text: F115.

**F65. Profile Editor.** *Do*: click. *Does*: opens Data Profile (F67) modally. On OK the list is rebuilt; no restart message here (`SelectProfile.cs:169-174`). If the user deletes every profile here, the list comes back empty and the only ways on are to add one or to exit: F3's loop does not test `ProfileCount` again (`VA.cs:646-679`) (**KEEP**). *Mac*: a sheet on this sheet.

**F66. Leaving.** Closing the dialog runs F5. *Mac*: `Quit Vixen` on the left of the bottom row, `.cancelAction`, because a sheet has no close box. This is the one added control in this area, and it replaces the title bar's close box.

### E. Data Profile

**F67. Window.** *Does*: title `Data Profile`, fixed size (`DataProfileForm.Designer.cs:233-253`). Group `Profiles:` with `Profile:` drop-down and + and − buttons on its row, `Profile Name:`, `Data Folder:` with a folder button. Below it group `Profile to Use when Loading Vixen`. Bottom row: `Zip Wizard` on the left, OK and Cancel on the right. *Rules*: the + − and folder buttons are icon-only; their text is cleared in the constructor (`DataProfileForm.cs:17-23`). *Mac*: a sheet. `Grid` for the fields. SF Symbols `plus`, `minus`, `folder`. Rust holds the rules in `ProfileEditor`, so Swift only binds.

**F68. Profile drop-down.** *Do*: choose a profile. *Does*: first commits the fields to the previously shown item (`Name` as typed, `DataFolder` trimmed), then shows the chosen item's values (`DataProfileForm.cs:232-261`). Items are edited in memory until OK. *Default*: with `ProfileCount` 0, one item `Default` at `~/Documents/Vixen 3`; otherwise each `Profile{i}` with name default `New Profile` and folder default that same path; first selected (`:207-230`, `:11`).

**F69. Add profile (+).** *Do*: click. *Does*: commit the fields, then a text prompt: `Enter a name for the new profile`, title `Profile Name`, initial `New Profile` (`DataProfileForm.cs:263-302`). Loop on OK: blank, meaning exactly empty and not trimmed, so a name of spaces is accepted, shows `Profile name can not be blank.` titled `Error`, error icon; a name already there (exact match) shows `A profile with the name {name} already exists.` with an empty title, warning icon; either way the prompt returns with the text kept. Cancel stops. A new item gets `DataFolder = ~/Documents/Vixen 3 {name}`, is selected, and the load drop-down is refilled. Enter and Esc work in the prompt (`Vixen.Common/Controls/TextDialog.cs:40-44`). The folder is not created until OK (F75). *Mac*: a small sheet with the same prompt, then `.alert`s. Default path built in Rust.

**F70. Delete profile (−).** *Do*: click. *Does*: alert `Are you sure you want to delete the {name} profile? The data folder and all its contents will remain and must be removed manually.`, title `Delete a Profile`, warning icon, YES, NO, CANCEL. YES removes it from the in-memory list, selects the first item, refills the load drop-down (`DataProfileForm.cs:304-325`). Nothing on disk changes. Deleting the last one leaves the fields showing stale text, bound to an orphan, and OK then stores `ProfileCount` 0, so the next launch uses the default folder. **GUARD**: with an empty list, clear and disable the fields. *Mac*: buttons `Delete Profile` (destructive) and `Cancel`; NO and CANCEL have the same effect upstream.

**F71. Profile Name.** *Do*: type. *Does*: commits on leaving the field, on changing profile, on + (F69), and at the end of OK's checks. Not trimmed (`DataProfileForm.cs:232-239`, `:334-337`). Both drop-downs draw each item's current name, so a committed rename shows at once (`Vixen.Common/Controls/Theme/ThemeComboBoxRenderer.cs:28`). *Mac*: `TextField` bound to Rust through a draft, with commit on focus loss.

**F72. Data Folder.** *Do*: type, or click the folder button. *Does*: the button opens a folder browser at the current text and puts the choice in the field (`DataProfileForm.cs:198-205`). The field has no leave handler of its own, so it commits on changing profile, on + (F69) and at OK, and also whenever Profile Name loses focus, because `SaveCurrentItem` always commits both fields. Trimmed. *Mac*: `NSOpenPanel` with `canChooseDirectories`, `canCreateDirectories`.

**F73. Profile to Use when Loading Vixen.** *Do*: choose `Ask Me Every Time`, or `Load This Profile:` and a drop-down item. *Does*: on open, `LoadAction == "Ask"` picks the first radio, anything else the second. The drop-down mirrors the profile list. Its selection is set from `ProfileToLoad` (default 0, if below the count), only when opening with the second radio (`DataProfileForm.cs:33-67`). Adding and deleting refill it, keeping the selected item if it survives. **Each refill also reads `LoadAction` from the file again and resets the radios to it, so + or − undoes an unsaved change of radio** (`:37-38`, called from `:301` and `:323`) (**KEEP**). Choosing an item does not change the radio, and `Ask` does not disable the drop-down. Opening with `Ask` leaves the drop-down unselected, so switching to the second radio without choosing an item leaves `ProfileToLoad` unwritten at OK (F75 step 6), and the next launch shows F63. *Default*: `LoadSelected`.

**F74. Zip Wizard.** *Do*: click. *Does*: opens F77 modally. It reads saved settings, so unsaved edits here do not show there (`DataProfileForm.cs:339-343`, `DataZipForm.cs:111-141`). *Mac*: a sheet on this sheet.

**F75. OK.** *Does* (`DataProfileForm.cs:69-196`):
1. For each item, on stored values: blank name or folder shows `One or more of your profile entries has a blank name or data folder. You must correct this before continuing.`, title `Warning - Blank Entries`, warning icon, OK, **once per blank item, and continues regardless of what it says** (**KEEP**). Collect duplicate names and duplicate folders as `{name}:\t {folder}`, and invalid folders the same way. Invalid means not rooted or holding no `\`.
2. Any duplicates: alert `A duplicate profile name, or data path exists.\n\nThe duplicate items found were:\n\n{lines}\n\nClick OK to accept and continue, or Cancel to go back and edit.`, title `Warning - Duplicate Entries`, buttons YES, CANCEL. Not YES: stay in the dialog.
3. Any invalid: alert `An invalid profile data folder exists.\n\nThe items with invalid paths were:\n\n{lines}\n\nClick OK to accept and continue, or Cancel to go back and edit.`, title `Warning - Invalid Data Path`. Same handling.
4. Commit the fields. So upstream's checks can miss what was just typed in Data Folder.
5. Write `ProfileCount`. For each item write `Profile{i}/Name` and `/DataFolder`. If that folder does not exist: alert `The data directory '{folder}' for profile '{name}' does not exist.  Would you like to create it?`, title `Application.ProductName` (`Vixen.Application` upstream, `Vixen` on the Mac; F5, Q21), exclamation, YES, NO. YES creates it; failing that, alert `Could not create new profile directory: {folder}\n\nClick OK to ignore and continue, or Cancel to go back and edit.`, title `Error`, YES, CANCEL. CANCEL stays in the dialog, with what was written so far left in the file. NO goes on with the folder missing.
6. Write `LoadAction` (`Ask` or `LoadSelected`), and `ProfileToLoad` if the drop-down has a selection. Return OK.

`Profile{n}` nodes above the new count stay in the file, unused. `DateLastLoaded` is keyed by index, so after a delete a date attaches to the wrong profile, which affects only sort order (**KEEP**).

**FIX** (Q9): commit the fields first, and resolve all folder prompts before writing anything, so Cancel really leaves the file alone. *Mac*: the invalid test is "does not start with `/`". A Windows path therefore shows as invalid. Button text matches the alert's wording: `OK` and `Cancel`; for the folder prompts `Create Folder` and `Don't Create`.

**F76. Cancel.** Closes; nothing saved (`DataProfileForm.Designer.cs:215`, `:239`). Esc works.

### F. Profile Archive Wizard

**F77. Window.** *Does*: title `Profile Archive Wizard` (the docs and the button call it the Zip Wizard), fixed size. From the top: group `Profile artifacts to include`, `Profile to archive`, `Folder to save archive to`, `Archive file name, without extension`; below, `Start` and `Close`; a status strip with a progress bar, hidden until running, and a text (`DataZipForm.Designer.cs:63-314`). *Mac*: a sheet. `Start` is the default action, `Close` the cancel action, as upstream (`:299`, `:302`). Status row at the bottom.

**F78. Artifact check boxes.** *Do*: tick or untick. All on by default (`DataZipForm.Designer.cs:98-192`). In the original positions:

| Row | Left | Middle | Right |
|---|---|---|---|
| 1 | Module | Program | Sequence |
| 2 | System | Template | Logs |
| 3 | Media | User Settings | |

Unticking adds an exclusion: Module `\Module Data Files`, Program `\Program`, Sequence `\Sequence`, System `\SystemData`, Template `\Template`, Media `\Media` (`DataZipForm.cs:74-98`). Logs and User Settings control extra passes (F83). Everything else in the folder is always included: `Effect Defaults`, `Export`, `Fixtures`, `logs`, `VixenApplicationData.xml`. Not persisted. They stay enabled during a run, read once at its start.

**F79. Profile to archive.** *Does*: drop-down of saved profiles: `Default` at the default folder if there are none, otherwise each `Profile{i}`, with empty defaults; first selected (`DataZipForm.cs:111-141`). It accepts typing, and typed text with no match gives `Unable to find datafolder for that profile.` (title `Error`) at Start. *Mac*: `Picker`, so that alert cannot arise.

**F80. Folder.** *Does*: text plus folder button (`DataZipForm.cs:143-149`). *Default*: Documents (`:140`). *Mac*: `NSOpenPanel` for folders.

**F81. File name.** *Default*: `VixenProfile` (`DataZipForm.cs:139`).

**F82. Start.** *Do*: click. *Does*, in order (`DataZipForm.cs:151-243`):
1. No profile: F79's alert.
2. Empty folder: `Please choose a folder to create the zip file in.`, title `Missing save folder`.
3. Invalid path characters: `The folder path for the zip file contains invalid characters.`, title `Invalid Folder Path.`
4. Empty name: `Please choose a filename for the zip file.`, title `Missing Zip file name`, information icon.
5. Invalid file name characters: `The filename for the zip file contains invalid characters.`, title `Invalid Zip file name`.
6. A name ending in `.zip` (exact case) has it removed, in the field too.
7. Folder missing: `The destination folder does not exist, would you like to create it ?`, title `Folder not found`, YES, NO. NO stops. YES creates it. **Else** if the zip exists: `The file name you have enter already exists, do you wish to overwrite it ?`, title `File exists`. YES deletes it; otherwise stop.
8. Button becomes `Stop`, `Close` disables, the background work starts. It also disables the profile, name, folder and folder button, and shows the bar (`:245-261`).

Icons: error on steps 1, 2, 3 and 5, information on step 4, question on step 7. `Directory.CreateDirectory` and `File.Delete` in step 7 sit outside any `try` (`:221`, `:231`), so a failure there goes to the crash path and ends the app (F11). **GUARD**: show the error text in an alert and stay in the dialog.

*Mac*: the character rules are the Mac's: `/` and NUL for the name; NUL for the folder. Add `:` to the name's list, because Finder shows it as `/`. Buttons `Create Folder`/`Cancel` and `Replace`/`Cancel`.

**F83. The archive run.** *Does* (`DataZipForm.cs:42-68`, `:70-103`, `:293-364`): output `{folder}/{name}.zip`. `parent` is the profile folder's last path segment, from a split on the separator, so a stored folder with a trailing separator gives an empty `parent` and entries with no top folder (`:52-53`) (**KEEP**: `Path` in Rust would hide this, so split the text the same way).
1. Status `Zipping profile please wait...`. Archive the profile folder under `parent`.
2. If that worked and Logs is on: status `Zipping logs please wait...`. Archive `Documents/Vixen 3/logs` under `{parent}/Core Logs`.
3. If that worked and User Settings is on: status `Zipping user settings please wait...`. Archive `%AppData%/Vixen` under `{parent}/User Settings`.
4. Status `Complete`, controls restored, button back to `Start`, bar hidden (`:263-281`). It says `Complete` after a failed pass and after a Stop too (**KEEP**). The User Settings pass's result is ignored (`:65`).

Each pass: list all files recursively; `fileCount` counts all of them. For each file: stop if cancelled. Skip if excluded (below) or if it is the archive itself. Entry name is `parent` + path below the folder. `filesComplete++`, then progress `(int)(filesComplete / (double)fileCount * 80.0)`. After the loop progress 100. So the bar restarts each pass and tops out below 80 until the jump. On an exception: log, alert `An error occurred during the zip process.\n\r {message}`, title `Error Zipping Files`; the pass fails and later passes are skipped.

Excluded: path ends with `.lock`; or the file's **whole directory path** contains an exclusion as a substring (`:343-364`). So unticking Media also drops everything if the profile sits under a folder like `D:\Media Backup\`, and Program matches `\Program Files\`. **FIX** (Q8): match against the path relative to the profile folder, keeping the substring test: `("/" + relative_dir).contains("/Media")`.

The profile pass includes the profile's own `logs` folder whatever the Logs box says, so with the default profile logs appear twice when it is ticked (**KEEP**).

**GUARD**: upstream's `Directory.EnumerateFiles` is outside the `try` (`:298`). A missing folder ends the background worker silently with `_working` stuck true, so the button says `Stop` forever and Close stays disabled. Report it through the same alert and finish.

*Mac*: a background Rust task. Zip entry separators are always `/` (upstream's are `\`, from `Path.Combine` and `Substring`). Examples: `Vixen 3/SystemData/SystemConfig.xml`, `Vixen 3/Core Logs/Application.Info.log`, `Vixen 3/User Settings/Settings.xml`. Deflate, zip64 on, because `Media` is large. One writer across the three passes, not upstream's reopen in update mode (H6). User settings folder: `~/Library/Application Support/Vixen`.

**F84. Stop.** *Do*: click while running. *Does*: `_bw.CancelAsync()`. The loop ends after the current file, the zip closes as a valid file holding what it has, the remaining passes end at once, and the status reads `Complete` (`DataZipForm.cs:153-157`, `:310-313`). The partial file stays (**KEEP**).

**F85. Close.** Closes. Disabled while running. Upstream's title bar close is not blocked during a run; a sheet has none.

### G. Installed Modules

**F86. Window and list.** *Does*: title `Installed Modules`; width fixed at 915; height 456 to 681 (`InstalledModules.Designer.cs:114-127`). A details list with headers `Name`, `Description`, `Author`, `Version`, `File`, not clickable, rows not reorderable (`InstalledModules.cs:16`). Groups: every module type name, sorted; a type with no modules shows nothing (`:24-27`). Rows in each group in module file order. A property that throws shows `(Not Provided)` (`:10`, `:55-89`). Any other exception shows a plain message box. Columns auto-size to content but no narrower than header + 20, and on each layout the last column takes the remaining width (`Vixen.Common/Controls/DragAndDropListView.cs:62-65`, `:191-203`, `:205-213`). *Mac*: `Window(id: "installed-modules")`. SwiftUI `Table` with `Section`s, 113 rows: the 55 in `## Data and persistence` plus 58 effects. Rows sort case-insensitively by `File`, as NTFS gives them. `vixen_effect_registry::all()` sorts by display name, so `installed_modules()` re-sorts the effects. Data from `installed_modules()`; the table is in `## Data and persistence`. Groups that appear: Analysis, App, Controller, Editor, Effect, LayerMixingFilter, Media, OutputFilter, Preview, Property, SequenceType, Timing. `File` shows the upstream name without `.dll`, since no such file exists (Q14). Same column order.

**F87. Copy Module Id.** *Do*: select a row, click. *Does*: puts `TypeId.ToString()`, lower case with hyphens, on the clipboard (`InstalledModules.cs:96-104`). *Rules*: enabled when a row is selected (`:91-94`), disabled at first (`InstalledModules.Designer.cs:94`). The list allows several selected rows, since nothing turns `MultiSelect` off, and the button copies the first (`InstalledModules.cs:98`). Bottom left. *Mac*: `NSPasteboard`, with the `Table` on single selection. Rust gives the same lower-case text.

**F88. Close.** Bottom right, also the Return key (`InstalledModules.Designer.cs:103-116`).

### H. Setup Previews

**F89. Window and list.** *Does*: title `Setup Previews`, fixed size, client 482x569. Header labels `Name` and `Type` above a list with check boxes, full row select, multiple selection, no header row (`ConfigPreviews.Designer.cs:109-134`, `:190-206`). Below: `Add New Preview`, `Duplicate Selected`, `Delete Selected`; then group `Selected Preview`; then OK and Cancel at the bottom right (`:146-244`). Rows: preview name, module type name, ticked if running; after every fill, the shown preview is reselected (`ConfigPreviews.cs:195-220`). Selecting one row fills the group; none or many clears and disables it (`:34-46`, `:233-251`). Duplicate and Delete are enabled with one or more selected. The owner's real data gives two rows, both type `Vixen Display Preview`: `Christmas Tree 2019`, unticked since it is in `DisabledDevices`, and `Christmas Tree 2022 (OpenGL)`, ticked. *Mac*: a sheet. `Table` with a `Toggle` column.

**F90. Tick a preview.** *Do*: tick or untick. *Does*: untick and running: stop it. Tick and not running: start it, then after 250 ms bring this dialog to front (`ConfigPreviews.cs:279-312`). Does **not** set `_changesMade`, so Cancel does not ask (F96), yet Cancel's reload still undoes the tick (F26). With the engine closed a start does nothing (F37), yet the box stays ticked until the next fill. **FIX** (Q15): re-read `is_running` after the call. Errors logged, `Failed to change the preview running state.` *Persist*: only through disabled devices at the next save (F37).

**F91. Add New Preview.** *Do*: click. *Does* (`ConfigPreviews.cs:48-95`): takes the first preview module. A list-picker branch exists but is dead, because `outputModules` is empty when tested (`:52-56`) (**KEEP** by omission). Creates a device named after the type, `Vixen Display Preview`, with a new id (`Vixen.Core/Factory/PreviewFactory.cs:10-27`). Adds it, which starts it if the manager is running. Selects it, refills, opens the preview designer at once (F95). Sets `_changesMade`. Errors logged, `Failed to add and configure a preview.`

**F92. Duplicate Selected.** *Do*: click. *Does*: for each selected, a new device of the same module with name `{name}-copy`, a new id, and a clone of the module data, replaced in the store under the new id. Adds it; the last one added becomes the shown preview. Refills, sets `_changesMade` (`ConfigPreviews.cs:133-167`). It starts at once if the manager is running.

**F93. Delete Selected.** *Do*: click. *Does*: alert `Are you sure you want to delete the selected preview?`, title `Delete preview?`; with several, `...selected previews?` and `Delete previews?`. Error icon, YES, CANCEL. On YES, for each: delete `Preview_{ModuleInstanceId}` from `Settings.xml` `AppSettings` (window position) **immediately, so Cancel does not bring it back** (**KEEP**), and `Previews.Remove`, which removes its module data from the store and stops it (`ConfigPreviews.cs:97-131`, `Vixen.Core/Sys/Managers/PreviewManager.cs:104-108`, `:135-138`). Refill, set `_changesMade`. The newer `OpenGLPreview_{id}` key is left behind (`Vixen.Modules/Preview/VixenPreview/OpenGL/OpenGLPreviewForm.cs:1040`). **FIX** (Q16): delete whichever keys the Mac preview uses. *Mac*: `Delete` (destructive), `Cancel`.

**F94. Selected Preview: Name.** *Do*: type in `Name:`, click `Update`. *Does*: sets the name, refills, sets `_changesMade`. Typing alone changes nothing. No validation: empty and duplicate names are allowed (`ConfigPreviews.cs:169-179`). *Rules*: enabled with exactly one selected.

**F95. Selected Preview: Configure Preview.** *Do*: click. Beside it the text `Configure details specific to the preview.` *Does* (`ConfigPreviews.cs:181-193`, `:253-277`): with exactly one selected: if running, stop it. `preview.Setup()` opens the designer modally (`Vixen.Modules/Preview/VixenPreview/VixenPreviewModuleInstance.cs:162-183`). If it was running, start it, wait 250 ms, bring this dialog to front. `_changesMade` is set whatever happened. Errors logged, `Failed to configure the selected preview.` *Mac*: `openWindow` for the designer, awaited; contents in the preview spec.

**F96. OK and Cancel.** *Do*: click, or close the dialog. *Does*: list edits are applied live to `VixenSystem.Previews`, so OK and Cancel only decide what happens next (F26). Closing with `_changesMade` (`ConfigPreviews.cs:314-347`): `Cancel` asks `All changes will be lost if you continue, do you wish to continue?`, title `Are you sure?`, question icon, YES, NO; NO stays. OK passes. Any other close with changes is refused. Without changes, it closes without asking. The title bar's close counts as Cancel, since a modal form's close sets `DialogResult.Cancel`. Return is OK, Esc is Cancel. *Mac*: edits stay live on the running previews, as upstream, and Rust tracks `changes_made` and `needs_reload`. A list edit sets both; a tick sets only `needs_reload`. OK saves. Cancel with `changes_made` asks, then reloads; Cancel with only ticks reloads without asking, as upstream; Cancel with neither just closes (Q5). Buttons `Discard Changes` (destructive), `Keep Editing`.

### I. Options

**F97. Window.** *Do*: Tools > Options..., or on the Mac ⌘,. *Does*: title `Options`, fixed size. At the top the text `This form lets you set various options that control V3's operation`; then one row with `Update Interval:` on the left and `Clear Effect Cache on Exit` on the right; group `Audio`; group `Video Effect Cache`; OK and Cancel (`OptionsDialog.Designer.cs:49-194`). *Mac*: the `Settings` scene with `.formStyle(.grouped)`, per `docs/UI_GUIDE.md`. Same order, same words. Keep the top sentence as a footer or caption.

**F98. Update Interval.** *Do*: type or step. *Does*: range 0-500, step 23 (`OptionsDialog.Designer.cs:90-96`), loaded from `VixenSystem.DefaultUpdateInterval`, milliseconds, though no unit is shown (`OptionsDialog.cs:17`). Applying sets the value and `DefaultUpdateTimeSpan` (`VS.cs:364-377`). Read live by the update adjudicator, which never updates faster than `Math.Max(10, interval)` ms, so a value below 10 does not speed the engine (`Vixen.Core/Execution/ControllerUpdateAdjudicator.cs:41`, `Execution.cs:84`), each output device's default interval (`Vixen.Core/Module/Controller/ControllerModuleDescriptorBase.cs:25`, `Vixen.Core/Module/Preview/PreviewModuleDescriptorBase.cs:25`), manual cache timing (`Vixen.Core/Cache/Sequence/FixedIntervalManualTiming.cs:10`), and effect frame time (`Vixen.Modules/Effect/Effect/BaseEffect.cs:27`). A typed value is held to the range when the field commits. *Default*: 50 (`SystemConfig.cs:11`). The owner's is 10. **GUARD**: a stored value above 500 throws upstream in `Load`; clamp the display. *Persist*: `DefaultUpdateInterval` in `SystemConfig.xml`. *Mac*: `TextField` plus `Stepper(step: 23)`. Applying also calls `Engine::set_interval`. **GUARD**: 0 is allowed here, so pass at least 1 ms, because `set_interval` itself only clamps to 1 microsecond (`crates/vixen-engine/src/playback.rs:174-176`).

**F99. Latency.** *Do*: type or step. *Does*: group `Audio`, label `Latency:`, range 5-500, step 10 (`OptionsDialog.Designer.cs:109-116`). Loaded from `AudioOutputManager.Latency`; applying saves `AudioDevice.settings` at once (`Vixen.Common/AudioPlayer/AudioOutputManager.cs:102-110`). Takes effect when the next output opens (`:178`). *Default*: 25 (`AudioPreferences.cs:20`). **GUARD**: a stored value outside 5-500 throws upstream in `Load`, as F98 does; clamp the display. *Mac*: requests a cpal buffer of `latency_ms * rate / 1000` frames, clamped to the device's range, when `vixen-audio` next opens a stream.

**F100. Clear Effect Cache on Exit.** *Do*: tick. *Does*: tooltip `Frees disk space, but adds some\ntime to rebuild at sequence open` (`OptionsDialog.cs:22-28`). The Video effect reads it in `Dispose` and deletes its cache folder (`Vixen.Modules/Effect/Video/Video.cs:1183-1193`). *Default*: on when the file omits it (`XmlSystemConfigFilePolicy.cs:157`). A profile with no `SystemConfig.xml` upstream shows it off and the file type blank, because `new SystemConfig()` sets neither (`SystemConfig.cs:27-35`); `SystemData::default` gives on and `bmp`, which stays (Q13). *Persist*: `ClearEffectCacheOnExit`. *Mac*: `Toggle`, `.help`.

**F101. Cache File Type.** *Do*: choose. *Does*: group `Video Effect Cache`, label `Cache File Type:`, an editable drop-down with `bmp` and `png`, tooltip `bmp - Fastest/Bigger\npng - Fast/Smaller` (`OptionsDialog.Designer.cs:149-157`, `OptionsDialog.cs:29`). The text goes into the Video effect's settings hash and file extension (`Video.cs:595`, `:659`). *Default*: `bmp` when the file omits it. *Persist*: `VideoEffectCacheFileType`. Because the box is editable, upstream takes any typed text as the extension. *Mac* (Q23): `Picker` with the two values, plus a stored unknown value as a third item, so it survives; typing a new one is dropped.

**F102. OK and Cancel.** *Does*: OK copies all four values, and the caller then runs `SaveSystemConfigAsync` (`OptionsDialog.cs:32-38`, `VA.cs:1063-1072`). Cancel discards. Return is OK, Esc is Cancel (`OptionsDialog.Designer.cs:173`, `:176`). A failed save: F117. *Mac* (Q12): `Settings` applies as you change, so there is no OK or Cancel. Save the system config when the Settings window closes, if something changed. Latency saves at once, as upstream.

### J. About

**F103. About Vixen.** *Does*: title `About Vixen`, fixed at 900x800. Image `Resources.VixenImage` top left, heading `VersionInfo.VersionName` at 20 pt, regular weight, because `new Font(name, 20F)` drops the designer's bold, beside it, the licence text below with both scrollbars, OK bottom right (`AboutVixen.cs:10-22`, `AboutVixen.Designer.cs:38-96`). The text is read from `License.txt` beside the executable, which is UTF-16 with a BOM. Its box is not read-only, but edits go nowhere. A missing file throws from `Load` (**GUARD**: show the box empty and log). *Mac*: `Window("About Vixen", id: "about")`, from App > About Vixen and Help > About Vixen. Read-only, selectable text in a scroll view. `app/Resources/License.txt` is already there, as plain ASCII, with the same text as `upstream/Vixen/License.txt` once that is decoded. Copy `Vixen.Common/Resources/Resources/VixenImage.png` into `app/Resources/images`.

### K. Updates

**F104. Update check rules.** *Does* (`Updates/GitHubUpdateService.cs`, client setup in `Program.cs:34-51`). API base `https://api.github.com/repos/VixenLights/Vixen/`, 10 s timeout, user agent `Vixen-UpdateChecker/1.0`, `Accept: application/vnd.github+json`, `X-GitHub-Api-Version: 2026-03-10`.
- Channel: development builds use `Development` with the build number as text; others `Release` with `ReleaseVersion` (`CheckForUpdates.cs:52-53`, `VA.cs:504-540`).
- `Release`: `GET releases/latest`. Null if missing, draft, prerelease, or the tag does not parse. `Development`: `GET releases?per_page=5`, no paging. Among non-draft, prerelease, valid `DevBuild-N` tags, the largest N, not the newest date (`:71-89`).
- Result: `LatestVersion` (tag text for releases, the number for development), `LatestBuildNumber`, `IsUpdateAvailable`, `ReleaseNotes` (only if requested; a null body becomes empty), `ReleasePageUri` (`html_url`, or `https://github.com/VixenLights/Vixen/releases`), `PublishedAt`, `InstallerDownloadUri` (first asset whose name ends `-Setup-64bit.exe`, ignoring case) (`:155-190`). `IsUpdateAvailable` for releases compares `VixenReleaseVersion`, independent of the body. On the release channel an unparsable installed version gives null. On the development channel it gives a result with `IsUpdateAvailable` false (`:159-163`); the installed value there is always a number, so this cannot arise from the app.
- `VixenReleaseVersion` parses `major.minor` or `major.minoruN`, digits only (`NumberStyles.None`), and compares major, minor, update (`Updates/VixenReleaseVersion.cs`). `DevBuild-N` needs the exact prefix, case-sensitive, and `int.TryParse`, which permits surrounding spaces, with `N >= 0` (`:192-198`).
- Cache: success only, per channel, 5 minutes (`:40-69`). A startup check followed by the dialog makes one request. The body is kept, so asking for notes later needs no call.
- Release notes: `GET releases/tags/{escaped tag}`; null if missing, draft or the returned tag differs. Cache per tag, 5 minutes (`:91-115`). Prereleases are allowed here.
- Failure: non-success status, timeout, malformed JSON or `HttpRequestException` logs a warning and returns null. Never "up to date" (`:117-153`). 403 with `X-RateLimit-Remaining: 0` logs `GitHub update check was rate limited.`

`UnavailableUpdateService` returns null for everything (`Updates/UnavailableUpdateService.cs`). *Mac*: all of this in `vixen-session::updates` with a fake-able transport. Used by the startup notice (F29) with notes off, and by F105 with notes on. Same URLs, since upstream's releases are what the Mac port follows (Q11). `VIXEN_UI_TEST` selects the unavailable service.

**F105. Check for Updates window.** *Does*: title `{VersionName} Installed`, fixed size that fits its contents. Icon top left spanning three rows; heading; version text; `Download` button; `Change Log for latest version:`; notes box; OK at the bottom right (`CheckForUpdates.Designer.cs:131-156`). Starts with `Checking for updates, please wait.` (`CheckForUpdates.cs:42-47`). Closing cancels the request (`:39`). Outcomes (`:66-103`), with `{type}` = `Release`, `Development` or `Test`:

| Outcome | Heading | Version text | Download | Notes |
|---|---|---|---|---|
| Null | `Update check unavailable.` (hidden, see below) | `Unable to check for updates. Please try again later.` | hidden | hidden |
| Newer, with installer | `An updated {type lower} build is available.` | `{type} {latest} is the latest.` | shown | shown, 225 high, read only, no wrap, scrollbars as needed |
| Newer, no installer | same | `An update is available, but its installer is unavailable.` | hidden | shown |
| Current | `You have the latest {type lower} build installed.` (hidden) | `{type} {latest} is the latest.` | hidden | hidden |

The heading label starts hidden and only the "newer" branch shows it, so the other two heading texts are set and never seen. **FIX** (Q10): show the heading in all outcomes. Line endings are normalized (`:108-109`). OK closes; Return works. *Mac*: a sheet. `NSTextView` for notes (H9). Icon from `VixenImage.png`.

**F106. Download.** *Do*: click. *Does*: `GetByteArrayAsync` of the installer, written to the Downloads folder under its URL's file name, then alert `Latest version downloaded to {folder}.`, title `Info`; on failure `The update could not be downloaded. Please try again later.`, title `Download Failed` (`CheckForUpdates.cs:111-143`). A separate client, base `https://github.com/VixenLights/Vixen/releases/download/`, 5-minute timeout. *Mac*: **FIX** (Q11): a Windows installer means nothing here. The button keeps its place and name and opens `ReleasePageUri` in the browser. It shows when there is a newer version. The "installer unavailable" text does not apply.

### L. Release Notes

**F107. Release Notes window.** *Does*: title `Vixen Release Notes`, resizable, 774x571. A large text box and OK at the bottom right (`ReleaseNotes.Designer.cs:37-90`). On load (`ReleaseNotes.cs:34-77`): null tag shows `Release notes are unavailable for test builds.` Otherwise `Loading release notes...`, then the release body for that tag. A blank or missing body falls back to the file `Release Notes.txt` beside the executable. If that is also missing, `Release notes could not be loaded.` Any exception gives the same text. The box is not read-only; edits go nowhere. *Mac*: `Window("Vixen Release Notes", id: "release-notes")`, read-only `NSTextView` (2862 lines in the file, H9). Bundle `upstream/Vixen/Release Notes.txt`. Its first entry is 3.13u4, matching this port.

### M. Instrumentation

**F108. Instrumentation window.** *Do*: Tools > Instrumentation... *Does*: modeless, title `Instrumentation`, resizable, client 807x404. A rich text box with a forced vertical scrollbar, then `Close` and `Reset` at the bottom right, Reset rightmost (`InstrumentationForm.Designer.cs:26-118`). A 1000 ms timer rewrites all the lines as `{Name}: {FormattedValue}`, in registration order (`InstrumentationForm.cs:15-24`, `Vixen.Core/Sys/Instrumentation/Instrumentation.cs`). The timer's first tick is 1 s after opening, so the box is blank for the first second. It is created when the module loads, shown on click, and re-created after being closed; a click while it is open does nothing more (`InstrumentationModule.cs:33-46`, `:72-76`). The box is editable until the next tick, and it holds the focus at first. Return closes when the focus is elsewhere (`AcceptButton`). The values below are the only ones ever registered, in this order except that device rows appear and leave as devices start and stop; a search for `AddValue` finds no others, and the `CommandsQualified`, `TotalEffects` and similar classes in `Vixen.Core/Sys/Instrumentation/` have no registration:

| Value name | Registered at | Format |
|---|---|---|
| `System allowed update`, `System allowed block`, `System denied update`, `System denied block` | `Execution.cs:27-34`, each config load | ms |
| `   Elements update`, `   Contexts update`, `   Filters update` (leading spaces are in the names) | `ElementManager.cs:37`, `ContextManager.cs:33`, `FilterManager.cs:30` | ms |
| `{controller} Output Total Update` | `Vixen.Core/Sys/Output/OutputController.cs:53-57`, `:187`, added on start and removed on stop (`:193`) | ms |
| `{device} delta`, `{device} triggered engine update`, `{device} refresh rate`, `{device} sleep time` | `Vixen.Core/Sys/Managers/HardwareUpdateThread.cs:46`, `:160-170`, while the device runs (`:58`) | ms, except refresh rate `0.00 Hz` |
| `Update time for preview` | `VixenPreviewModuleInstance.cs:16`, `:22`; same name for every preview, last wins | ms |
| `Preview pixel set time` | `Vixen.Modules/Preview/VixenPreview/GDIPreview/GDIPreviewForm.cs:53-54` | ms |
| `OpenGL preview background draw`, `OpenGL preview points update`, `OpenGL preview points draw`, `OpenGL preview update` | `Vixen.Modules/Preview/VixenPreview/OpenGL/OpenGLPreviewForm.cs:93-101` | ms |

ms format: `min {min} ms,  avg {(int)tot / cnt} ms,  max {max} ms,  cnt {cnt}`, two spaces after each comma. `cnt` starts at 1, not 0, and `min` starts at 999, so a fresh value reads `min 999 ms,  avg 0 ms,  max 0 ms,  cnt 1` (`Vixen.Core/Instrumentation/MillisecondsValue.cs:8-10`, `:18-26`, `:41-44`) (**KEEP**). `refresh rate`'s read also resets its counter (`RateValue.cs:17-29`). *Mac*: `Window(id: "instrumentation")`, single instance. `instrumentation_lines()` is called at 1 Hz only while the window is visible. Selectable read-only text. The values come from `EngineStats`, `ControllerStats` and `LiveStats`. Keep every name whose concept still exists on the Mac; the GDI and OpenGL ones become `Preview pixel set time` and the `vixen-render` equivalents, and record any dropped in the parity note.

**F109. Reset.** *Do*: click. *Does*: `Reset()` on every value (`InstrumentationForm.cs:31-37`).

**F110. Close.** Closes; timer stops with the form.

### N. Web Server

**F111. Web Server dialog.** *Do*: Tools > Web Server. *Does*: title `Configuration`, fixed. Picture, label `Web Server`, `Http Port:` stepper, range 80-64000, a check box labelled `IsEnabled` with the label left of the box, `Point your browser here:` with a link `http://{hostname}:{port}/`, the note `Turn off the web server when not in use to conserve resources.`, OK and Cancel, in that order, with no default or cancel key (`App/WebServer/Settings.Designer.cs:44-182`, `Settings.cs`). The picture is `pictureBox1.Image` in `App/WebServer/Settings.resx:121`, a base64 PNG, 385x165, of the Vixen logo with the word Vixen, stretched to 358x149. The label and link show only when enabled and track both controls live (`Settings.cs:33-37`, `:66-70`). Clicking the link opens the browser. *Default*: port 8888, enabled (`App/WebServer/Data.cs:12-14`); the owner's is the same, so the first Mac launch of their show starts the server if this ships (Q17). A stored port outside the range throws in the constructor (**GUARD**: clamp). The docs' `Tools -> Web Server -> Configure` path is out of date; there is one item. *Mac*: a sheet, with Return and Esc as usual. Host name from `ProcessInfo.hostName`. Extract the picture to `app/Resources/images/WebServerLogo.png`.

**F112. Applying web server settings.** *Does* (`App/WebServer/Module.cs:77-104`): on OK, copy port and enabled if they differ and note that; **always** `SaveModuleConfigAsync`; if anything changed, apply the state. Cancel: nothing. Applying enabled: stop, start Kestrel on `http://*:{port}`, catch and log `Unable to start web _server.` and stop there; create live context `Web Server` if there is none, and start it (`:111-131`). Applying disabled: stop, release the context (`:133-142`). `WebHost.Start` is `async void`, so most start failures escape that catch (`App/WebServer/WebHost.cs:40-49`). The context does not survive the engine: closing execution releases every context (`State/Behavior/StandardClosingBehavior.cs:13`, `Vixen.Core/Sys/Managers/ContextManager.cs:97-107`), but `Module.LiveContext` stays set, and `App/WebServer/Module.cs:126` only creates one when it is null. So after Stop then Start, or any reload, the web server's live control has a dead context until the server is turned off and on again. This is read from the code, not run; the web server's spec decides whether to keep it. *Persist*: module store. *Mac*: no crate yet. Its REST and hub surface needs its own spec (Q17). Starts from `start_services` when enabled, so the first launch triggers the macOS firewall prompt, and listens on all interfaces, as upstream. Never under `VIXEN_UI_TEST`.

### O. Services behind the window

**F113. Log files.** *Does*: NLog, `NLog.config`. All under `Documents/Vixen 3/logs/`, whatever the profile. Line format `MM/dd/yyyy HH:mm:ss.fff [{logger}] {message} {exception with stack}`. Targets:

| Target | File | Archive to `archive/` | Keep |
|---|---|---|---|
| `Logs` | `Application.{level}.log` | `Application.{level}.{shortdate}.{#}.log` | 30 |
| `WebRequestAccessLogs` | `WebAccess.{level}.log` | `Application.{level}.{shortdate}.{#}.log` (sic) | 5 |
| `ScheduleLogs` | `Scheduler.{level}.log` | `Scheduler.{level}.{shortdate}.{#}.log` | 30 |
| `V2ConvertLogs` | `V2Convert.{level}.log` | `V2Convert.{level}.{shortdate}.{#}.log` | 30 |

All archive above 1,000,000 bytes and daily, numbered by date, file not kept open. Rules, first match with `final` wins: `VixenModules.App.WebServer.RequestLoggingMiddleware` Warn+ to WebAccess; `VixenModules.SequenceType.Vixen2x.*` Info+ to V2Convert; `Scheduler` Debug+ to Scheduler; `Microsoft.Hosting.Lifetime` Info+ to Logs; `Microsoft.*` up to Info dropped; everything else Info+ to Logs. A real line: `12/20/2025 17:43:19.588 [Vixen.Sys.VixenSystem] Vixen System starting up... `, with the trailing space from the empty exception. `throwExceptions="false"` means a logging failure never reaches the app (`NLog.config:6`). NLog's own trace goes to `logs/nlog-internal.log` (`:7`), which appears in F39's list if it exists. *Mac*: `vixen-session::logging`, same files, names and format, off the main actor. Folder: `DataPaths::default_root()/logs`, or `<VIXEN_DATA>/logs` under test so tests never touch the real one. Rust crates log through a small facade, keyed by the upstream logger name where one maps; there is no logging crate in the workspace yet. The first log write is F1 step 1, so it is what raises macOS's Documents prompt (F8), before any window shows. **GUARD**: if that write fails, drop the lines and carry on. A profile kept outside Documents must still start. The lines themselves: F116.

**F114. Version info.** *Does*: from the four-part assembly version (`VersionInfo.cs`). Windows reads `Major.Minor.Build.Revision`.

| Version | `IsTestBuild` | `IsDevBuild` | `IsReleaseBuild` | `CurrentVersionType` | `VersionName` | `ReleaseVersion` | `BuildName` |
|---|---|---|---|---|---|---|---|
| 0.0.0.x | yes | no | no | `Test` | `Test Build` | empty | `Build #` |
| 0.m.b.x, b > 0 | no | yes | no | `Development` | `Development Build #{b}` | `{b}` | `Build #{b}` |
| 3.13.1400.0 | no | no | yes | `Release` | `Release 3.13` | `3.13` | `Build #1400` |
| 3.13.1400.4 | no | no | yes | `Release` | `Release 3.13u4` | `3.13u4` | `Build #1400` |
| 0.m.0.x, m > 0 | no | no | no | `Release` (fall through) | `Release ` | empty | `Build #` |

A null assembly version becomes 0.0.0 (`:27-32`). *Mac*: one constant in `vixen-session::version`. `Info.plist` and `Cargo.toml` hold `3.13.4`, which reads as major 3, minor 13, revision 4, so `Release 3.13u4`. `CFBundleVersion` gives the build number. `app_info` in `vixen-ffi` is replaced by `version_info`.

**F115. `Settings.xml`.** *Does*: `XMLProfileSettings` (`XPS.cs`) reads `%AppData%/Vixen/Settings.xml` into a new document per instance; a missing or bad file starts from `<settings></settings>` (`:11-25`). Every `PutSetting` saves the whole file (`:69-79`). Keys are XPaths under `settings/{Profiles|AppSettings|Preferences}/`, so `Profile0/Name` nests. Missing elements are created along the path (`:147-162`). Values are `Convert.ToString`, so a date is culture text, `12/20/2025 5:43:19 PM` here, and `Convert.ToInt32` and the like throw on bad text. Two live instances overwrite each other's changes. Eight files outside this area use it for window positions and similar: `GDIPreviewForm.cs`, `OpenGLPreviewForm.cs`, and in `Vixen.Modules/Editor/TimedSequenceEditor/` `ExportDialog.cs`, `TimedSequenceEditorForm.cs`, `TimedSequenceEditorForm_Toolstrip.cs`, `Forms/Form_CurveLibrary.cs`, `Forms/Form_GradientLibrary.cs`, `Forms/Form_ColorLibrary.cs`. *Mac*: `vixen-io::settings::ProfileSettings`, one shared store behind a lock, atomic writes, batched puts (H5). **GUARD**: bad text gives the default. Dates write as `M/d/yyyy h:mm:ss a` and parse that or ISO 8601. File: `~/Library/Application Support/Vixen/Settings.xml`. `VIXEN_UI_TEST` redirects it to `VIXEN_UI_TEST_OUT`.

### P. Added at the audit

**F116. Log lines.** *Does*: System > Logs shows these files, so the text is user-visible. Every line this area writes, by logger (`GetCurrentClassLogger` gives the class's full name):

| Logger | Level | Text | When |
|---|---|---|---|
| `VixenApplication.Program` | Info | `Vixen app starting.` | F1 step 1 |
| | Fatal | `An application error occurred. Please contact the Vixen Dev Team with the following information:\n\n` and the exception; `Save was in progress during the fatal crash. Trying to pause 5 seconds to give it a chance to complete.` | F11 |
| `VixenApplication.VixenApplication` | Info | `Processing Profiles`, `Finished Processing Profiles` | Around F3, only with no `data_dir` (`VA.cs:139-141`) |
| | Info | `Process info: {pid}@{machine}` | First use of the lock's id (`VA.cs:237`, F6) |
| | Info | `Profile root : {dir}` | F3 step 5 |
| | Info | `{release} - {build}`, or `{release}` alone on a test build | F1 step 19 (`VA.cs:512`, `:521`, `:529`) |
| | Info | ` Release version {v} available.`, ` Development build {v} available.` | F29 |
| | Error | `An error occurred creating the profile lock file.`, `An error occurred removing the profile lock file.`, `An error occurred checking the profile lock file.` | F6 |
| | Warn | `Waiting for save to finish before closing.` | F12 step 3 |
| | Error | `Can't find an appropriate editor to open file of type {type}`, `Can't find an appropriate editor to open file {file}`, `Error trying to open file '{file}': ` | F19, F20 |
| | Error | `Exceptionj showing the Vixen App form` (the typo is upstream's) | F10 fails (`VA.cs:484`) |
| `VixenApplication.VixenApplicationData` | Error | `VixenApplication: loading application data: couldn't find data format version`, `VixenApplication: error loading application data`, `VixenApplication: error saving application data`, `VixenApplication: error reading application data; given data version was too high: {n}` | F7, data file |
| `VixenApplication.DataZipForm` | Error | `An error occurred adding files to archive` | F83 |
| `VixenApplication.ConfigPreviews` | Error | `Failed to add and configure a preview.`, `Failed to configure the selected preview.`, `Failed to change the preview running state.` | F91, F95, F90 |
| `VixenApplication.CheckForUpdates` | Error | `An error occurred while downloading the latest version.` | F106, upstream only |
| `VixenApplication.ReleaseNotes` | Warn | `Release notes failed to load.` | F107 |
| `VixenApplication.Updates.GitHubUpdateService` | Warn | `GitHub returned an invalid latest Vixen release.`, `GitHub returned an invalid Vixen release for tag "{tag}".` (NLog quotes a string argument), `GitHub update check was rate limited.`, `GitHub update check returned HTTP status {status}.` (the `HttpStatusCode` name, such as `NotFound`), `GitHub update check timed out.`, `GitHub update check returned malformed JSON.`, `GitHub update check failed.` | F104 |
| `Vixen.Sys.VixenSystem` | Info | `Vixen System starting up...`, `Vixen System successfully started.`, `Vixen System stopping...`, `Vixen System successfully stopped.` | F1, F12 |
| | Error | `Error during system startup!` | F9 |
| | Info | `Saving System Config.`, `System Config saved.`, `Saving Module Config.`, `Module Config saved.` | Each save |
| | Error | `System config is already being saved. Skipping duplicate request.`, `Module config is already being saved. Skipping duplicate request.` | F37 |
| | Info | `Reload Requested while save in progress. Waiting 5 ms.` | `VS.cs:234` |
| | Info, Warn | `Seeded built-in element tag '{name}' ({key}).`, `Removed orphaned Preview data for instance {id}`, `Removed orphaned element tag assignment {id} from node '{name}' ({nodeId}).` | F1 step 25 |
| `Vixen.Sys.State.Execution.*State` | Info | The six lines in F37 | F37 |
| `Vixen.Sys.Managers.OutputDeviceExecution` | Error | `Error starting device {name}`, `Error trying to stop device {name}`, `Device {name} experienced an error during execution and was shutdown.` | F37 |
| `Vixen.Sys.Modules` | Error | `Error occurred while adding module {file}`, `Error occurred while removing module {file}` | F36 |
| `VixenModules.App.WebServer.Module` | Error | `Unable to start web _server.` | F112 |
| `VixenModules.App.WebServer.Startup` | Info | `Webserver started.` (`App/WebServer/Startup.cs:40`) | F36, F112 |
| `VixenModules.App.LivePreview.Module` | Info | `LivePreview module loaded.`, `LivePreview module unloaded.` | F36 |
| `Scheduler` | Info | `Schedule Enabled`, `Schedule Disabled` | F56, F36 |

Left out on the Mac: `Starting JIT Profiler`, `Completed JIT Profiler`, `JIT Profiling Disabled`, `.NET Runtime is: {v}`, and the WPF error at `VA.cs:95`. The rest keep their text and logger name.

**F117. A save that fails.** *Does*: `SaveSystemConfigAsync` and `SaveModuleConfigAsync` set their flag, then save on a pool thread, and clear the flag only on success (`VS.cs:171`, `:191-197`, `:215`, `:217-223`). A failed write logs `An error occurred trying to save the file. Attempting to protect any backups`, renames any `{file}_backup*` to `protected`, and rethrows (`Vixen.Core/IO/Xml/XElementFileWriter.cs:27-32`, `:197-220`). The callers in this area are `async void`, so the exception reaches `Application.ThreadException` and F11: Fatal log, the 5 s wait because `IsSaving()` is now stuck true, `Environment.Exit(1)`, no message. In a migration (F1 step 29) the exception is lost with the inner task, the flag stays true, and F12 step 3 then never ends. *Mac* (**FIX**, Q19): `save_system_config` and `save_module_config` always clear the flag, through a guard in `boot.rs`, and return the error. Swift shows an alert, title `Error`, text `Could not save the configuration.\n\n{error}`, button `OK`, and the app stays up with its changes still in memory. That text is new, since upstream has none.

**F118. The backup before a save.** *Does*: every save through `XElementFileWriter` first moves the current file into `auto_backup` (rules already in `vixen-io::file`). If that fails: native message box `Backups failed prior to save! Possible file system issue!`, title `Warning!`, OK and Cancel (`Vixen.Core/IO/Xml/XElementFileWriter.cs:39-48`). OK writes with no backup. Cancel returns with nothing written, and the caller still logs `System Config saved.` This covers sequences as well as the two system files; `docs/spec/sequencer-shell.md` does not mention it. *Mac* (**FIX**, Q20): no prompt. `save_with_backups` leaves the old file in place and returns the error (`crates/vixen-io/src/file.rs:35-38`), which F117 then shows.

**F119. What a reload does to the app modules.** *Does*: `ReloadSystemConfig` (Cancel in F25 and F26) replaces `ModuleStore` with a new one read from disk (`VS.cs:268`). `PopulateRepositories` does not run again, and an app module's `StaticModuleData` is assigned only when the module is created (`Modules.cs:318`), so every app module keeps the old store's object. This is read from the code, not run. From then until the app closes, whatever those modules change goes to the old object: Scheduler Enabled (F56), schedules and shows, web server settings (F112), export profiles (F49), package export settings (F52), curve and gradient library edits. `SaveModuleConfigAsync` writes the new store, so those changes are silently lost at the next launch. *Mac* (**FIX**, Q18): `AppCore` holds one module store, and after a reload each service re-reads its data from it, so these changes save normally.

**F120. A second copy of Vixen.** *Does*: upstream can run several instances at once, each on its own profile, which is what the lock and the `- Locked` rows are for (F6, F63). A second launch is a new process. It shares `Settings.xml`, and each use re-reads that file (F115), so it sees the other's changes between uses. *Mac*: Launch Services activates the running app instead of starting another, so a second instance needs `open -n -a Vixen`, optionally with `--args data_dir=<folder>`. F6 and F63 work unchanged then. **GUARD**: because `ProfileSettings` is one shared store per process, reload the file before a put if its modification time has changed, or the second instance would overwrite the first's changes. No menu item is added; record this in `docs/parity/administration.md`.

### Quirk register

Each **FIX** above, for the owner's veto. All else is as upstream.

| # | Where | Upstream | Mac | Why |
|---|---|---|---|---|
| Q1 | F1 step 19 | Startup waits up to 10 s for GitHub before loading anything | Check runs in the background after startup | An offline Mac would start 10 s slower for a status bar message |
| Q2 | F1 step 29, F8, F9 | Load errors inside `LoadConfiguration` are lost; the engine opens while a migration save runs | Errors reach F9; the save finishes before the engine opens | A half-loaded show should not reach the lights |
| Q3 | F6 | A `.lock` from another machine locks the profile with no way out | **Open question.** Offer `Open Anyway` in F4 when the lock names another machine, showing that name. Without it the owner must delete the file by hand | Moving a crashed profile from Windows would strand it |
| Q4 | F12 | NO on `Close Vixen?` cannot stop the close, because the handler has already returned | NO cancels | The dialog says it can |
| Q5 | F26, F90, F96 | Cancel always reloads the config and restarts all outputs, even with no change | Reload only if something changed, a tick included. Ticks alone still do not raise the prompt | H4 |
| Q6 | F33 | Wrong 64-bit time assembly; `CPU: -1%` for a second | Correct calculation; `Resource Usage` until the first sample | The formula does not exist on the Mac anyway |
| Q7 | F37 | Saving while the engine is stopped disables every device for next launch | Snapshot only while the engine is open | It silently breaks the next show |
| Q8 | F83 | Exclusions match the whole absolute path | Match below the profile folder | A folder like `/Volumes/Media/` would empty the archive |
| Q9 | F75 | Checks run before committing the field; Cancel at a folder failure leaves a partial save | Commit first, write last | Cancel should mean cancel |
| Q10 | F105 | Two headings are set but never visible | Show them | The text exists; a hidden label is the bug |
| Q11 | F104, F106 | Downloads `-Setup-64bit.exe` | **Open question.** Keep the check against upstream's releases; `Download` opens the release page | No Windows installer on a Mac, and no Mac releases exist upstream |
| Q12 | F102 | OK and Cancel | Settings applies live, saves on close | `docs/UI_GUIDE.md` maps Options to `Settings` |
| Q13 | F100 | A new profile shows unticked and blank | Ticked and `bmp`, from `SystemData::default` | A null file type would reach the Video effect |
| Q14 | F86 | `File` shows the DLL name | Shows the upstream module name without `.dll` | There is no file; the name still identifies it |
| Q15 | F90 | Box stays ticked when the start did nothing | Box follows the device | It should not lie |
| Q16 | F93 | Only `Preview_{id}` removed | Remove whichever keys the Mac preview uses | No leaked settings |
| Q17 | F112 | Web server is built in | **Implemented November 11, 2026.** `vixen-web` plus session lifecycle/API and native settings; enabled by default on port 8888, respecting stored disabled settings | Implementation is locally verified; changing the upstream default remains an optional policy choice in `docs/DIFFERENCES.md` |
| Q18 | F119 | After a reload, app modules keep writing to the old store's objects, so later changes to schedules, shows, web server settings and libraries never reach `ModuleStore.xml` | One store; services re-read it after a reload | Silent data loss |
| Q19 | F117 | A failed config save is a crash with no message; a failed migration save hangs shutdown | An alert with the error; the flag clears; the app stays up with its changes in memory | The owner should be told, and a full disk should not end the session |
| Q20 | F118 | `OK` on the backup prompt overwrites with no backup | No prompt: the old file stays and the error shows. Already so in `vixen-io::file` | The write is atomic, so a backup failure loses nothing. A veto needs a force flag |
| Q21 | F5, F75 | Two alerts read `Vixen.Application` | They read `Vixen` | That is an assembly name, with no meaning on the Mac |
| Q22 | F20 | The same file can be open in two editors, with separate undo and saves | **Open question.** Opening it again focuses the open window, which is what `openWindow(id:value:)` does with an equal `SequenceDocumentRef` | Two editors on one file overwrite each other. If the owner wants upstream's way, `existing` needs a unique id too |
| Q23 | F101 | `Cache File Type` accepts any typed text | A `Picker`: the two values, plus a stored unknown one | Its tooltip names only those two, and typing an unknown type there has no documented use |

## Data and persistence

### Files

| What | Upstream location | Mac location | Format | Read | Written |
|---|---|---|---|---|---|
| Profiles and window settings | `%AppData%\Vixen\Settings.xml` (`XPS.cs:15-17`) | `~/Library/Application Support/Vixen/Settings.xml` | XML, below | At launch (`Profiles`); by other windows on demand | Each put, F115. Profiles: F64, F75. Preview positions removed: F93 |
| Audio preferences | `%AppData%\Vixen\AudioDevice.settings` (`AudioPreferences.cs:10`) | Same folder, same name | `XmlSerializer` root `<Settings>`: `SoundOutDeviceId` (default `-0`), `AudioOutputMode` (`WASAPI`), `AudioClientShareMode` (`Shared`), `Latency` (25). A blank or missing file gives defaults (`PreferenceBase.cs:16-34`) | First use of audio | At each change. Latency: F99 |
| Application data | `{profile}/VixenApplicationData.xml` (`VixenApplicationData.cs:12`, `:36-39`) | Same. `vixen-io::appdata` already reads and writes it | XML, below | F1 step 13 | After each F32 (`VA.cs:1154`) and at exit (`VA.cs:405`). Not on F23. Upstream writes in place and logs errors (`VixenApplicationData.cs:102-145`); Rust writes atomically |
| Profile lock | `{profile}/.lock` | Same | Text `{pid}@{machine}` | F6 | Created at F1 step 14, removed at F12 and F11 |
| System config | `{profile}/SystemData/SystemConfig.xml` | Same, `vixen-io::system` | Existing | F1 step 25, and each reload | F25, F26, F102; also Display Setup and others (`VS.cs:161-202`). Never at exit |
| Module store | `{profile}/SystemData/ModuleStore.xml` | Same | Existing | F1 step 25 | F25, F26, F52, F56, F112; also library and editor saves (`VS.cs:204-228`). Never at exit |
| Logs | `Documents\Vixen 3\logs\` for every profile (`NLog.config:31-38`) | `~/Documents/Vixen 3/logs/`; `<VIXEN_DATA>/logs` under test | F113 | F39 | All the time |
| Profile archive | The folder and name the user picks | Same | Zip, F83 | Never | F83 |
| JIT profile | `%AppData%\Vixen\~perfData.tmp` | Omitted | | | |

`%AppData%\Vixen` is what F83's User Settings pass archives. The owner's copy of the show holds no `Settings.xml`, since that stayed on Windows, and no `.lock`. So the first Mac launch sees no profiles, takes F3 step 1, and opens `~/Documents/Vixen 3`. The owner then adds the real show with System > Profiles... There is no automatic import of a Windows `Settings.xml`; if one is copied in, its `C:\` folders fail F3's checks and lead to F4 and Select a Profile, which is the right outcome.

### Windows paths elsewhere in the owner's data

The recent list is not the only place (F21). `SystemConfig.xml` has none, but `ModuleStore.xml` holds eleven, all under another user's home, `C:\Users\olduser\`:

| Module data | Member | Value | Who handles it |
|---|---|---|---|
| Timed Sequence Packager | `ExportOutputFile` | `...\Documents\Vixen 3\Export` | F52. The output stage has `Path.HasExtension` false and `Directory.Exists` false, so it falls back to `{data root}/Export/{ProfileName}.vpkg` (`App/TimedSequenceMapper/SequencePackageExport/SequencePackageExportOutputStage.cs:36-46`), which is `.vpkg` when there is no profile name (F3 step 1). That works on the Mac as written, as long as the checks run on the raw text |
| Export Wizard, `Profiles`, the export profile `Default` | `OutputFolder`, `AudioOutputFolder`, `FalconOutputFolder` | `...\Desktop\Stuff\Christmas Tree Lights 2018\BACKUP\...` | The Export Wizard's spec, F49 |
| Export Wizard, the same profile | `SequenceFiles`, seven `.tim` files | `...\Documents\Vixen 3\Sequence\{name}.tim` | The same. Resolve them as F21 does |

All of them must stay untouched in the file unless the user changes them, and none may reach a file system call as if it were a Mac path.

### `Settings.xml`, section `Profiles`

```xml
<settings>
  <Profiles>
    <ProfileCount>2</ProfileCount>
    <Profile0>
      <Name>Default</Name>
      <DataFolder>/Users/me/Documents/Vixen 3</DataFolder>
      <DateLastLoaded>12/20/2025 5:43:19 PM</DateLastLoaded>
    </Profile0>
    <Profile1>
      <Name>Halloween</Name>
      <DataFolder>/Users/me/Documents/Vixen 3 Halloween</DataFolder>
    </Profile1>
    <LoadAction>LoadSelected</LoadAction>
    <ProfileToLoad>0</ProfileToLoad>
  </Profiles>
  <AppSettings>
    <!-- Preview_{id} and others, written by other windows -->
  </AppSettings>
</settings>
```

| Key | Type | Default when read | Written by |
|---|---|---|---|
| `ProfileCount` | int | 0 | F75 |
| `Profile{i}/Name` | text | `New Profile` in F63 and F68; empty in F3 and F79 | F75 |
| `Profile{i}/DataFolder` | text | empty in F3, F63, F79; default folder in F68 | F75 |
| `Profile{i}/DateLastLoaded` | culture date text | `DateTime.MinValue` | F64 |
| `LoadAction` | `Ask`, or anything else meaning load selected | `LoadSelected` | F75 |
| `ProfileToLoad` | int index | `-1` in F3, `0` in F73 | F75, only with a selection |

### `VixenApplicationData.xml`

```xml
<VixenApplicationData>
  <DataFormatVersion>3</DataFormatVersion>
  <RecentSequences>
    <SequenceFile>C:\Users\developer\OneDrive\Documents\Vixen 3\Sequence\Coldest Winter.tim</SequenceFile>
  </RecentSequences>
  <FilterShapePositions />
  <FilterSetupFormHighQualityRendering value="false" />
</VixenApplicationData>
```

Format rules are at `VixenApplicationData.cs:102-211` and already in `crates/vixen-io/src/appdata.rs`. A bad file: F7. Upstream always writes version 3 (`:111`), and so does Rust for a file at version 3 or below: kept at an older version, the members that version lacks would be saved and then skipped by the next load. `FilterPosition` holds `FilterId`, `xPositionProportion` and `yPosition`. A search finds nothing else that reads `FilterSetupFormShapePositions` or `FilterSetupFormHighQualityRendering`, so they are round-tripped only. A version above 3 is read as empty. Upstream would then overwrite that file on save; **GUARD**: Rust's residue keeps its content and its version. Entries stay as stored (F21). New entries are absolute POSIX paths. A Windows Vixen reading them treats them as missing and skips them, so the file stays usable in both.

### System config values this area changes

`DefaultUpdateInterval` (F98), `ClearEffectCacheOnExit` (F100), `VideoEffectCacheFileType` (F101), `Previews/Preview@name,typeId,instanceId` (F91-F94), `DisabledDevices/Device@id` (F37). All exist in `SystemData`. Element tags are seeded at each load if their id is absent, logging `Seeded built-in element tag '{name}' ({key}).` (`VS.cs:308-318`, `Vixen.Core/Sys/BuiltInElementTags.cs:66-77`): `Deprecated`, key `deprecated`, id `753115da-e27a-4261-a136-8222ccc3f22e`, colour `#FF0000`, order 0; `Hidden`, `hidden`, `2f21bfcb-07a3-4cac-bdc5-4b6e37773c04`, `#000000`, 1; `Prop`, `prop`, `6db1852d-71d8-449d-afa5-193a6def26a0`, `#0000FF`, 2. The owner's file has no tags yet, so all three seed on first load. The owner's values: interval 10, clear on exit true, file type `bmp`, previews `Christmas Tree 2019` and `Christmas Tree 2022 (OpenGL)`, disabled `Arduino`, `Light Strip Controller` and `Christmas Tree 2019`.

### Module store values this area changes

Super Scheduler `IsEnabled` (F56) and `StatusForm_Position` (F57). Web Server `HttpPort`, `IsEnabled` (F112). Timed Sequence Packager export fields (F52). Export Wizard `ActiveProfile` (F49). The owner's values: Scheduler off, position 90,204, check interval 1; Web Server 8888, on. A reload's effect on all of these: F119.

### Bundled files

Add to `app/Resources`, which `scripts/build_app.sh` copies into the bundle: `Release Notes.txt` (from `upstream/Vixen/Release Notes.txt`, UTF-8), `images/VixenImage.png` (from `Vixen.Common/Resources/Resources/VixenImage.png`), `images/WebServerLogo.png` (the base64 PNG at `Vixen.Modules/App/WebServer/Settings.resx:121`, F111). `images/V3Logo.png` and `License.txt` are already there. That `License.txt` is plain ASCII, with the same text as upstream's UTF-16 file.

### Installed module table

F86 shows this. Upstream reads it from descriptors at run time; the Mac has no DLLs. **Effects** come from `vixen_effect_registry::all()`, which already has name, description, author, version and type id; `File` is `Module.Effect.{folder}`, taken from the folder after `Effect/` in the descriptor's `icon` path. The one exception is the Fixture effect: its icon path is `Effect/Fixture/FixtureEffect/...` and its project is `FixtureEffect`, so its `File` is `Module.Effect.FixtureEffect`. All other modules are a static table in `vixen-session`, read from their descriptors and listed here, 55 rows, in upstream's file order. The five tagged filters (Color Wheel, Dimming, Prism, Shutter, Tag Filter) inherit author `Vixen Team` and version `1.0` from `Vixen.Modules/OutputFilter/TaggedFilter/TaggedFilterDescriptorBase.cs:32-37`. Generic timing's id really is empty (`Vixen.Modules/Timing/Generic/Descriptor.cs:7`).

| Group | Name | Description | Author | Version | File | Type id |
|---|---|---|---|---|---|---|
| Analysis | Beats and Bars | Beats and Bars Autodetection | Ed Brady | 1.0 | `Module.Analysis.BeatsAndBars` | `cf6326f5-424e-4c04-bf7b-ef511d9ca4e5` |
| App | ColorGradients | Provides a data type which represents an arbitrary color gradient/transition over time, and additional functionality (such as a gradient editor and library). | Vixen Team | 1.0 | `Module.App.ColorGradients` | `64f4ab26-3ed4-49a3-a004-23656ed0424a` |
| App | Curves | Provides a data type which represents a curve and additional functionality (such as a curve editor and library). | Vixen Team | 1.0 | `Module.App.Curves` | `4e258de2-7a75-4f0f-aa43-c8182e7f3400` |
| App | Custom Prop Editor | Enables creation of custom props for Previews | Vixen Team | 1.0 | `Module.App.CustomPropEditor` | `dc844055-e978-47fe-8359-dbcd5b568e80` |
| App | Export Wizard | Wizard to export sequences to other formats. | Jeff Uchitjil | 1.0 | `Module.App.ExportWizard` | `8bf8d0c5-b7d6-417c-9eff-ec1c5e1ef9ab` |
| App | FPP Client | REST API client for Falcon Player (FPP) instances | Vixen Team | 1.0 | `Module.App.FPPClient` | `c9eda00c-e5ac-4880-8bb5-00230a426703` |
| App | Instrumentation Panel | Text display of a few stats. | Vixen Team | 1.0 | `Module.App.Instrumentation` | `cbc07bf2-7e1b-4a29-aeed-9ee235cb0dee` |
| App | LipSync App | Breaks down Phoneme mappings into discrete animation components. | Ed Brady | 1.0 | `Module.App.LipSyncApp` | `a475c535-73fa-4d31-a1bc-ed2b50ad21cf` |
| App | Live Preview | Shared live element control service for broadcast-driven previews | Vixen Team | 1.0 | `Module.App.LivePreview` | `6f857027-6cbc-4729-bd00-6769c326e2cc` |
| App | Shows | Create and edit a massive show extravaganza | Derek Backus | 1.0 | `Module.App.Shows` | `67a6dfdb-ddec-4a5f-a158-129421e2366f` |
| App | Super Scheduler | Expansive Show Scheduler. | Derek Backus | 1.0 | `Module.App.SuperScheduler` | `985315e8-2b83-4d1a-89d1-58bfea7835a0` |
| App | TimedSequencePackager | Import / Export / Map Timed Sequences between profiles. | Jeff Uchitjil | 1.0 | `Module.App.TimedSequenceMapper` | `b13997c2-d4b5-4b5a-9047-1dced97ad99b` |
| App | Vixen WebServer | Web interface to Vixen 3 | Darren McDaniel | 1.0 | `Module.App.WebServer` | `8ac1cdf9-8d57-43b6-81de-93003ed2f907` |
| Controller | DDP | DDP Output Controller Module | Jon Chuchla | 1.0 | `Module.Controller.DDP` | `110b5fe9-cd4d-4e73-bb5b-67552a1ad6d1` |
| Controller | Debugging Values | A debugging controller that will display raw values in an output window. | Vixen Team | 1.0 | `Module.Controller.DebugController` | `f32bda77-28f0-46c4-a6bd-ba0ba73a1c13` |
| Controller | DMX Pro | DMX USB Pro hardware controller module | Vixen Team | 1.0 | `Module.Controller.DMXUsbPro` | `8abf27a7-3156-4f4c-9d43-ada45cdd6d0c` |
| Controller | Dummy Lighting (for testing) | A dummy/test output module that will display lighting commands in a new window. | Vixen Team | 1.0 | `Module.Controller.DummyLighting` | `b6ee6308-189c-4268-8996-32a4bab8ab5f` |
| Controller | Streaming ACN (E1.31) | An output plugin that emits E1.31 data. | Erik Mathisen and Joshua 1 Systems Inc. | 0.1 | `Module.Controller.E131` | `771d7eba-662a-48d5-aedb-445c8708e878` |
| Controller | Elexol Ethernet I/O 24 | Elexol Ether I/O 24 | Tony Eberle | 1.0 | `Module.Controller.ElexolEtherIO` | `2565e617-89db-4761-be8d-4d5574cafba9` |
| Controller | Generic Serial | Generic Serial hardware module | John McAdams (macebobo) | 1.0 | `Module.Controller.GenericSerial` | `d22c9233-16e9-4008-a215-e577f78a7a2e` |
| Controller | Launcher | Generic Windows Command / App Launcher | Jon Chuchla / Darren McDaniel / Steve Dupuis | 1.0 | `Module.Controller.LauncherController` | `1c4bfd64-3172-43a7-a7fc-0a77c88b0489` |
| Controller | DMX Open | Enttec Open DMX Output Module | Jon Chuchla - based on the original work of Joshua Moyerman & piense. | 2.0 | `Module.Controller.OpenDMX` | `7911568a-8eda-4d1d-9e72-be41d7a843e4` |
| Controller | RDS | Generic Windows Command and RDS hardware module | Darren McDaniel / Steve Dupuis | 1.0 | `Module.Controller.RDSController` | `692b66b9-e831-4f39-bb73-7653fa62b053` |
| Controller | Renard | Renard hardware module | Vixen Team | 1.0 | `Module.Controller.Renard` | `f66130a6-ff8a-48a7-bf1e-fac6cec2fc2c` |
| Editor | Timed Sequence Editor | An editor for a standard timed sequence | Vixen Team | 1.0 | `Module.Editor.TimedSequenceEditor` | `d342eedd-ae39-4b30-b557-b9329e6d3a7c` |
| LayerMixingFilter | Chroma Key | Creates a Chroma mask based on the selected brightness and fills with the higher layer. | Jon Chuchla | 1.0 | `Module.LayerMixingFilter.ChromaKey` | `2d467733-1a64-45ad-bcdc-84bf074747e6` |
| LayerMixingFilter | Color Change | Replaces the current color with the effect color at the same intensity. | Jeff Uchitjil | 1.0 | `Module.LayerMixingFilter.ColorChange` | `93d4169e-5138-403a-b71c-a5a1ab089801` |
| LayerMixingFilter | Highest Value (Default) | Combines two layers by passing the higher value | Jon Chuchla | 1.0 | `Module.LayerMixingFilter.HighestValue` | `01d6d526-9586-44bb-82f4-10abacf58b61` |
| LayerMixingFilter | Intensity Overlay | Uses the higher layer intensity to apply to the lower layer. | Jeff Uchitjil | 1.0 | `Module.LayerMixingFilter.IntensityOverlay` | `43f48bf5-4088-403f-ad7a-d0ff2114ca31` |
| LayerMixingFilter | Luma Key | Creates a luma mask based on the selected brightness and fills with the higher layer. | Jon Chuchla | 1.0 | `Module.LayerMixingFilter.LumaKey` | `7583f7d4-df99-4492-8d10-257c2677ff7b` |
| LayerMixingFilter | Mask | Uses the higher layer to mask the next lower layer. | Jeff Uchitjil | 1.0 | `Module.LayerMixingFilter.Mask` | `ace0f097-7982-4edc-b5dd-8989dce60e1d` |
| LayerMixingFilter | Mask And Fill | Uses the higher layer to mask and fill in the next lower layer. | Jeff Uchitjil | 1.0 | `Module.LayerMixingFilter.MaskFill` | `24878b18-14f8-458e-abaa-3a9c9a34931c` |
| LayerMixingFilter | Multiply Color | Multiplies the colors between layers.  Useful for modifying a color or brightness with another layer as input. | Jon Chuchla | 1.0 | `Module.LayerMixingFilter.MultiplyColor` | `0c9dfc0b-8e39-4167-8036-79408c6b0b35` |
| LayerMixingFilter | Proportional Mix | Combines two layers by using an inverse proportion of the intensity of the higher layer applied to the lower layer | Jeff Uchitjil | 1.0 | `Module.LayerMixingFilter.ProportionalMix` | `1f16d67a-1fff-485c-8e76-27ff7e7d072b` |
| Media | Audio | Allows an audio file to be used as a media item in a sequence. | Vixen Team | 1.0 | `Module.Media.Audio` | `fe460392-3fab-4c63-99dd-d76c48354150` |
| OutputFilter | Coarse Fine Breakdown | An output filter that breaks an intent value into a coarse (high byte) and a fine (low byte). | Vixen Team | 1.0 | `Module.OutputFilter.CoarseFineBreakdown` | `7c1465df-054c-4875-ab61-d7e7f5236897` |
| OutputFilter | Color Breakdown | An output filter that breaks down color intents into discrete color components. | Vixen Team | 1.0 | `Module.OutputFilter.ColorBreakdown` | `ab38a16f-0de1-4f6e-a8c0-ae5b20d347e0` |
| OutputFilter | Color Wheel Filter | An output filter that converts color intents into color wheel index commands. | Vixen Team | 1.0 | `Module.OutputFilter.ColorWheelFilter` | `18470c0c-1e75-464f-bd9f-a3e241cd1eb8` |
| OutputFilter | Dimming Curve | An output filter that translates lighting intensity values according to a curve (to compensate for non-linear lighting response). | Vixen Team | 1.0 | `Module.OutputFilter.DimmingCurve` | `2e40d6b1-43d2-4668-b63a-c600fadd7dd5` |
| OutputFilter | Dimming Filter | An output filter that converts color intents into dimmer intents. | Vixen Team | 1.0 | `Module.OutputFilter.DimmingFilter` | `61913688-ef44-473c-91d4-d256cca6cb91` |
| OutputFilter | Prism Filter | An output filter that converts prism intents into open prism commands. | Vixen Team | 1.0 | `Module.OutputFilter.PrismFilter` | `2522bd94-0434-4728-9400-0e18b212dc21` |
| OutputFilter | Shutter Filter | An output filter that converts color intents into shutter commands. | Vixen Team | 1.0 | `Module.OutputFilter.ShutterFilter` | `72b86092-0a59-4e40-b0a8-9e5fad0b8eb3` |
| OutputFilter | Tag Filter | An output filter that filters intents based on a tag. | Vixen Team | 1.0 | `Module.OutputFilter.TaggedFilter` | `fe552dcf-1bc0-4588-b23c-2600151d9fc5` |
| Preview | Vixen Display Preview | Vixen Display Preview | Derek Backus | 1.0.0 | `Module.Preview.VixenPreview` | `f43c31bc-cfcd-4dbc-adb7-d349c9ec623d` |
| Property | Color | Defines the element to have color, and the particular color(s) it can produce | Vixen Team | 0.1 | `Module.Property.Color` | `bff34727-6b88-4f87-82b7-68424498c725` |
| Property | Face | Provides mechanism for assigning Phoneme mapping of display elements | Jeff Uchitjil | 1.0 | `Module.Property.Face` | `eda73851-8350-4d3f-9df2-0d3b34e70bb6` |
| Property | Grid | Marks the attributed node as a grid | Vixen Team | 1.0 | `Module.Property.Grid` | `de3b4ec9-4e7f-4690-b0fe-3780b17a85ec` |
| Property | Intelligent Fixture | Provides ability to specify capabilities of an intelligent fixture | Vixen Team | 1.0 | `Module.Property.IntelligentFixture` | `fdd74c85-1779-4521-b559-0dd5ea71054e` |
| Property | Location | Provides XYZ positioning of display elements | Darren McDaniel | 1.0 | `Module.Property.Location` | `3fb53423-dd2a-4719-b3e3-19aa6f062f62` |
| Property | Order | Provides order mechanism for patching of display elements | Jeff Uchitjil | 1.0 | `Module.Property.Order` | `47d4a43b-5407-44bd-8be1-4974b148504f` |
| Property | Orientation | Provides Orientation information about display elements | Jeff Uchitjil | 1.0 | `Module.Property.Orientation` | `db0fc9fc-cdd5-4f88-bd66-5e4cf08feae4` |
| Property | State | Provides mechanism for assigning State mapping of display elements | Jeff Uchitjil | 1.0 | `Module.Property.State` | `35b893c3-d10c-4359-82bc-929c0762e809` |
| SequenceType | Timed Sequence | A basic timed sequence, which is a collection of effects configured to occur at predefined times. | Vixen Team | 9.0 | `Module.SequenceType.Timed` | `296bdba2-9bf3-4bff-a9f2-13efac5c8ecb` |
| SequenceType | Vixen 2.x Sequence | For importing sequences from Vixen 2.x. | John McAdams | 1.0 | `Module.SequenceType.Vixen2x` | `92bbd2cb-b750-437f-8a88-49864d569ab4` |
| Timing | Generic timing | Stopwatch-based timing module | Vixen Team | 1.0 | `Module.Timing.Generic` | `00000000-0000-0000-0000-000000000000` |

## Rust API needed

Everything below is new, unless marked or listed under "What already exists in the port" in `## Scope`. Swift stays thin. Each signature is a plain description, not an implementation. Errors are `IoError` or `SessionError`, whose text is shown to the user. `DateTime` means a local-time value: the workspace has no date crate, so add `time` to `vixen-io`, which logging and `DateLastLoaded` both need.

### `vixen-io`

```text
// settings.rs: port of XMLProfileSettings (F115)
enum SettingSection { Profiles, AppSettings, Preferences }
fn settings_dir() -> PathBuf                                     // ~/Library/Application Support/Vixen, or VIXEN_UI_TEST_OUT under test
struct ProfileSettings                                           // one shared instance, behind a lock
ProfileSettings::open(path) -> ProfileSettings                   // missing or bad file gives an empty <settings>
.get_string(section, xpath, default) -> String
.get_i32 / get_f64 / get_f32 / get_bool (section, xpath, default) -> T   // bad text gives the default
.get_date(section, xpath) -> Option<DateTime>                    // M/d/yyyy h:mm:ss a, or ISO 8601
.put_string / put_i32 / put_f64 / put_bool / put_date(section, xpath, value) -> Result   // saves atomically
.put_many(section, &[(xpath, value)]) -> Result                  // one save for many keys; every put first reloads a file changed on disk (F120)
.delete_node(section, xpath) -> Result                           // F93
.rename_node(section, xpath, new_name) -> Result
.remove_node(xpath) -> Result

// profiles.rs (F3, F4, F63-F76)
struct ProfileEntry { name, data_folder, date_last_loaded: Option<DateTime>, number: i32, is_locked: bool }
enum LoadAction { Ask, LoadSelected }                           // any stored text but Ask reads as LoadSelected
enum StartupProfile { NoProfiles, Load { folder, name }, Ask { error: Option<ProfileLoadError> } }
struct ProfileLoadError { name, is_locked }
ProfileLoadError::message() -> String                            // F4's exact text
fn decide_startup_profile(&ProfileSettings) -> StartupProfile    // F3 steps 1 to 3
fn check_profile(folder, name) -> Result<(), ProfileLoadError>   // non-empty, exists, not locked
fn list_profiles_for_selection(&ProfileSettings) -> Vec<ProfileEntry>   // F63: stable, newest first, locks checked
fn mark_profile_loaded(&ProfileSettings, number, now) -> Result  // F64
fn default_new_profile_folder(name) -> PathBuf                   // ~/Documents/Vixen 3 {name}

struct ProfileEditor                                             // F67-F75: in-memory list and the two radios
ProfileEditor::open(&ProfileSettings) -> ProfileEditor           // F68 defaults, F73 first-time rule
.items() -> Vec<ProfileEntry>; .select(index); .selected() -> Option<usize>
.set_name(text); .set_data_folder(text)                          // F71, F72; the folder trims on commit
.add(name) -> Result<(), AddProfileError>                        // Blank | Duplicate(name), with F69's texts; resets the radio to the stored value (F73)
.delete(index)                                                   // F70; resets the radio to the stored value (F73)
.load_action() / .set_load_action(LoadAction)
.load_selection() / .set_load_selection(Option<usize>)           // F73, keeps the item across refills
.validate() -> ProfileIssues                                     // F75 step 1
.folders_to_create() -> Vec<(name, folder)>                      // F75 step 5 prompts
.save(&ProfileSettings, created: &[folder]) -> Result            // F75 steps 5 and 6: writes once, at the end
struct ProfileIssues { blank_count, duplicates: Vec<String>, invalid_paths: Vec<String> }
ProfileIssues::duplicates_message() / invalid_message() -> Option<String>   // F75's exact texts

// lock.rs (F6)
fn machine_name() -> String                                      // LocalHostName
fn is_profile_locked(folder) -> bool                             // removes a stale lock, as upstream
fn create_lock_file(folder) -> Result<PathBuf, IoError>          // O_CREAT | O_EXCL, creates the folder
fn remove_lock_file(path) -> bool
fn lock_owner(folder) -> Option<LockOwner>                       // { pid, machine } for the Q3 alert

// appdata.rs, additions (F21, F23, F32)
const MAX_RECENT_SEQUENCES: usize = 20
ApplicationData::visible_recent_sequences(&DataPaths) -> Vec<RecentSequence>   // F21's three rules
ApplicationData::add_recent_sequence(path, &DataPaths)           // F32 step 4, skips empty, matches by resolved file too
ApplicationData::clear_recent_sequences()                        // F23, no save

// paths.rs, change
DataPaths::ensure_directories                                    // also the five Module Data Files folders (F8)
DataPaths::shared_logs_dir() -> PathBuf                          // default_root()/logs, or VIXEN_DATA/logs under test

// audio_prefs.rs (F99)
struct AudioPreferences { sound_out_device_id, latency_ms, .. }  // unknown members survive
fn load_audio_preferences(dir) -> AudioPreferences; fn save_audio_preferences(&AudioPreferences, dir) -> Result
```

### `vixen-audio`

```text
AudioPlayer::set_latency_ms(ms)                                  // applied when the next stream opens: F99
```

### `vixen-session`

```text
// boot.rs (F1, F2, F7, F8, F9, F12)
struct LaunchOptions { data_dir: Option<PathBuf>, disable_controllers, open_execution, under_test }
fn parse_launch_options(args, env_lookup) -> LaunchOptions       // F2, including VIXEN_DATA and VIXEN_UI_TEST
fn resolve_data_folder(requested: Option<PathBuf>) -> PathBuf    // F7's fallback
trait StartupProgress { fn report(&self, percent: i32, text: &str) }
AppCore::start(root, profile_name, LaunchOptions, &dyn StartupProgress) -> Result<Arc<AppCore>, SessionError>
                                                                 // steps 21 to 32: ensure folders, load, cleanups, services, engine
                                                                 // a bad VixenApplicationData.xml logs and gives empty data, not Err (F7)
AppCore::start_services(&self)                                   // F36's non-menu Loading work, in order
AppCore::shutdown(&self)                                         // F12 step 5, then step 6, then the lock
AppCore::profile_name(&self) -> Option<String>                   // VixenSystem.ProfileName
AppCore::save_system_config(&self) -> Result<bool>               // false if one is running; disabled snapshot rule, Q7; always clears the flag (F117)
AppCore::save_module_config(&self) -> Result<bool>               // same flag rule (F117)
AppCore::is_saving(&self) -> bool
AppCore::reload_system_config(&self) -> Result<()>               // closes and reopens the engine: VS.cs:334-343; services re-read their data (F119)
AppService::reloaded(&self, &AppCore)                            // new hook, called by reload_system_config, for a service that holds a copy of its data (F119)
fn ensure_built_in_element_tags(&mut SystemData)                 // VS.cs:308-318 with Vixen.Core/Sys/BuiltInElementTags.cs; no crate has this today
fn cleanup_orphans(&mut SystemData)                              // VS.cs:289-306, :320-332: preview module data, element tag assignments; logs as upstream

// execution.rs (F28, F30, F37, F44, F45)
enum ExecutionState { Closed, Opening, Open, Closing, OpeningForTesting, OpenForTesting }
ExecutionState::name() -> &'static str                           // the exact names
struct ExecutionStatus { text, light_rgb: (u8, u8, u8), start_enabled, stop_enabled }   // text is "Execution: {name}"
struct ExecutionEngine
.open(); .close(); .open_test(); .close_test()                   // no-ops per the state machine
.state() -> ExecutionState; .is_open(); .is_closed(); .is_in_test()
.status() -> ExecutionStatus
.subscribe(Box<dyn Fn(ExecutionState) + Send + Sync>)            // fired in change order, off the main actor

// previews.rs (F89-F96)
struct PreviewRow { id, name, type_name, is_running }
AppCore::previews() -> Vec<PreviewRow>
AppCore::add_preview() -> Uuid                                   // F91: first type, named after it, started if running
AppCore::duplicate_previews(&[Uuid]) -> Vec<Uuid>                // F92: "{name}-copy", module data cloned
AppCore::delete_previews(&[Uuid])                                // F93: settings key, module data, stop
AppCore::rename_preview(id, name)                                // F94
AppCore::set_preview_running(id, running) -> bool                // F90: returns is_running after the call
AppCore::preview_changes_made() -> bool; AppCore::reset_preview_changes()
AppCore::preview_needs_reload() -> bool                          // list edits and ticks; ticks do not set changes_made (F96, Q5)

// modules.rs (F19, F20, F86)
struct ModuleRow { type_name, description, author, version, file, type_id_text }
struct ModuleGroup { name, rows: Vec<ModuleRow> }
fn installed_modules() -> Vec<ModuleGroup>                       // sorted group names, empty groups left out, rows by File ignoring case, 113 rows
fn effect_file(icon) -> String                                   // exists: Module.Effect.{folder}, which gets the Fixture exception of the table's note right
struct SequenceTypeInfo { type_name, file_extension, can_create_new }
fn sequence_types() -> Vec<SequenceTypeInfo>                     // Timed Sequence, Vixen 2.x Sequence
fn open_panel_filters() -> Vec<FileFilter>                       // F20's four, with labels and extensions

// version.rs (F16, F17, F114)
struct VersionInfo { major, minor, build, revision }
fn current_version() -> VersionInfo
VersionInfo::is_test_build / is_dev_build / is_release_build -> bool
VersionInfo::current_version_type / release_version / version_name / build_name -> String
VersionInfo::release_notes_tag() -> Option<String>               // F61

// updates.rs (F29, F60, F104-F107)
enum UpdateChannel { Release, Development }
struct UpdateCheckRequest { channel, installed_version, include_release_notes }
struct UpdateCheckResult { latest_version, latest_build_number: Option<i32>, is_update_available, release_notes: Option<String>,
                           release_page_uri, published_at: Option<DateTime>, installer_download_uri: Option<String> }
struct ReleaseNotesResult { release_tag, release_notes, release_page_uri, published_at: Option<DateTime> }
trait HttpTransport { fn get(&self, relative_uri: &str) -> Result<HttpResponse, TransportError> }   // fake in tests
struct UpdateService
UpdateService::github(Box<dyn HttpTransport>) -> UpdateService   // caches as F104
UpdateService::unavailable() -> UpdateService                    // always None
.check(&UpdateCheckRequest) -> Option<UpdateCheckResult>
.release_notes(tag) -> Option<ReleaseNotesResult>
fn parse_release_version(&str) -> Option<ReleaseVersion>         // VixenReleaseVersion
fn parse_dev_build_number(&str) -> Option<i32>                   // TryGetDevelopmentBuildNumber
fn startup_update_notice(&VersionInfo, &UpdateService) -> Option<String>   // F29's exact text
fn fallback_release_notes(bundle_dir) -> Option<String>          // F107
fn normalize_line_endings(&str) -> String
// HTTPS needs a client the workspace lacks. Use ureq with rustls on a background thread (new dependency), or implement
// HttpTransport over URLSession through a UniFFI callback. No async runtime is needed either way.

// archive.rs (F77-F85)
struct ArchiveSelection { module, program, sequence, system, template, logs, media, user_settings }   // all true by default
struct ArchiveRequest { profile_folder, save_folder, file_name, selection }
enum ArchiveIssue { MissingSaveFolder, InvalidSaveFolder, MissingFileName, InvalidFileName, FolderMissing, FileExists }   // with F82's texts
fn check_archive_request(&ArchiveRequest) -> Result<PreparedArchive, ArchiveIssue>   // F82, including the .zip trim
fn resolve_archive_prompt(&ArchiveRequest) -> Result<PreparedArchive, SessionError>   // exists: F82 step 7's yes, creates the folder or deletes the zip; a failure is an error, never a crash
fn excluded(relative_dir, file_name, exclusions) -> bool         // F83, Q8
fn progress_value(files_complete, file_count) -> i32             // (int)(done / (double)total * 80.0)
fn run_archive(&PreparedArchive, cancel: &AtomicBool, &dyn ArchiveProgress) -> Result<(), SessionError>
trait ArchiveProgress { fn status(&self, text: &str); fn value(&self, percent: i32) }
// New dependency: the zip crate, deflate and zip64. flate2 is already in the lock file.

// instrumentation.rs (F108, F109)
trait InstrumentationValue: Send + Sync { fn name(&self) -> &str; fn formatted(&self) -> String; fn reset(&self) }
struct MillisecondsValue                                         // F108's format; set(value_ms) with atomics, no lock
struct RateValue                                                 // "0.00 Hz"; reading resets the count
struct Instrumentation                                           // ordered; a re-add under the same name keeps its slot
.add(Arc<dyn InstrumentationValue>); .remove(name); .lines() -> Vec<String>; .reset_all()

// cpu.rs (F33)
struct CpuUsage
.get_usage(&mut self) -> Option<i16>                             // None until the second call; throttled to 250 ms

// logging.rs (F11, F113)
enum LogLevel { Debug, Info, Warn, Error, Fatal }
fn init(logs_dir)                                                // targets, rules and archive as NLog.config; a write failure drops the line, never fails (F113)
fn log(level, logger, message, error: Option<&str>)
fn list_log_files(logs_dir) -> Vec<String>                       // F39: top level only, by name, empty if missing
fn install_crash_hooks(lock_path, is_saving: fn() -> bool)       // F11
```

Also, for the Web Server (F112), whoever writes that crate: `web_server_start(port) -> Result`, `web_server_stop()`, plus a live context named `Web Server`.

For the Scheduler (F56, F57), whoever writes that service, in `vixen-session`:

```text
struct ScheduleExecutor                                          // port of ScheduleExecutor; ticks off the main actor (H16)
.check_schedule(&mut self)                                       // the three branches of ScheduleExecutor.cs:180-206
.stop(graceful); .start()                                        // stop sets manually_disabled, only start clears it (F56)
.is_enabled() -> bool; .manually_disabled() -> bool; .status_text() -> String
```

### `vixen-ffi/src/admin.rs` (feature `admin`)

Today this file holds `RecentSequenceRow { name, path }`, the four recent-list calls and `max_recent_sequences()`. Add UniFFI records for the structs above and these exports. Long calls return through a callback or are called from a `@concurrent` Swift function.

```text
parse_launch_options(args: [String]) -> LaunchOptions
decide_startup_profile() -> StartupProfile
list_profiles_for_selection() -> [ProfileEntry]; check_profile(folder, name) -> ProfileLoadError?; mark_profile_loaded(number)
ProfileEditor object: every method above
create_lock_file(folder) -> String throws; remove_lock_file(path) -> Bool; is_profile_locked(folder) -> Bool
VixenCore.start(root, profile_name, options, progress) throws    // replaces VixenCore.open at launch
VixenCore: recent_sequences() -> [RecentSequenceRow { name, path }]; add_recent_sequence(path); clear_recent_sequences(); save_application_data()   // exist
VixenCore: execution_status() -> ExecutionStatus; execution_open(); execution_close(); subscribe_execution(listener)
VixenCore: save_system_config(); save_module_config(); reload_system_config(); is_saving() -> Bool; shutdown()
VixenCore: previews and F90-F96 calls; instrumentation_lines() -> [String]; instrumentation_reset()
VixenCore: options() -> Options; set_options(Options)            // interval, clear cache, file type, latency
VixenCore: services' flags: scheduler_enabled / set_scheduler_enabled; scheduler_manually_disabled; web_server_settings / set_web_server_settings
installed_modules() -> [ModuleGroup]; sequence_types(); open_panel_filters()
version_info() -> VersionInfoRecord                              // replaces app_info in core.rs
CpuUsage object; list_log_files() -> [String]; bundled_license_text(path) -> String   // plain text today; decode by BOM if the file ever returns to UTF-16
UpdateService object: check_for_updates(notes: Bool) -> UpdateCheckResult?; release_notes(tag) -> ReleaseNotesResult?
ArchiveTask object: start(request, listener); stop()
```

## SwiftUI and AppKit structure

Area module `app/Sources/VixenAdmin`, importing `VixenKit` and `VixenBridge`, with `crates/vixen-ffi/src/admin.rs` behind it. `AdministrationScene` there declares this area's scenes and every `.commands` group (F35), as `docs/UI_GUIDE.md` says. `VixenApp.swift` only includes it under `#if canImport`, as `app/Package.swift` expects, and gains nothing but the delegate forwarding under State. Other areas' windows are opened by id only, and their own scenes (`SetupScene`, `PreviewScene`, `SequencerScene`, `SchedulerScene`, `ToolsScene`) declare them. New ids go in `WindowID`, as an extension in a new VixenKit file where possible: `about`, `release-notes`, `instrumentation`, `custom-prop-editor`, `export-wizard`, `sequence-package-import`, `sequence-package-export`, `shows`, `schedules`, `scheduler-status`. `WindowID.profiles` and `WindowID.previewSetup` are placeholders today; the sheets in this spec replace them, unless the preview spec wants the second. Swift 6, `MainActor` by default, `@Observable`, per `docs/UI_GUIDE.md`. Write `docs/parity/administration.md` for each window, from `docs/reference/forms/Vixen.Application__*.json` and `Vixen.Modules__App__Instrumentation__InstrumentationForm.json`.

### State

`AppModel`, `@Observable`, one instance, passed by `@Environment`. It already exists in VixenKit, with `phase`, `core`, `dataRoot`, `isMuted` and `start()`. `docs/UI_GUIDE.md` prefers a new file to changing shared code, so state only this area reads can live in a VixenAdmin model that `AdministrationScene` owns. The state: `phase` (`resolvingProfile`, `loading`, `ready`, `failed`), `title`, `progress: (value, text, visible)`, `executionStatus`, `updateNotice`, `recent: [RecentSequenceRow]`, `openEditors`, `isBusy`, `systemConfigSession`, `pendingFiles` (F20's queue). `CpuReadout`, a separate small `@Observable`, owns the 1 Hz task and is read only by its `Text`. `ProfileEditorModel`, `ArchiveModel`, `SetupPreviewsModel`, `UpdatesModel` live with their sheets. `AppDelegate` lives in `VixenApp.swift` and handles `applicationShouldTerminate` (`.terminateLater`, F12), `application(_:open:)` (F20) and the harness, forwarding the first two to closures that VixenAdmin sets on `AppModel`, so no area's logic goes in `VixenApp`. An `NSWindowDelegate` on the Administration window routes `windowShouldClose` to `NSApp.terminate`.

### Scenes

| Scene | Kind | Id | Contents |
|---|---|---|---|
| Vixen Administration | `Window`, opens at launch | `administration` | `AdministrationView` |
| Settings | `Settings` | none | `OptionsForm` (F97-F102) |
| About Vixen | `Window` | `about` | F103 |
| Installed Modules | `Window` | `installed-modules` | F86-F88 |
| Vixen Release Notes | `Window` | `release-notes` | F107 |
| Instrumentation | `Window` | `instrumentation` | F108-F110 |
| Display Setup | `Window`, setup area | `display-setup` | Own spec |
| Custom Prop Editor | `WindowGroup(for: UUID.self)`, tools area | `custom-prop-editor` | Own spec |
| Export Wizard, Sequence Package Import, Export | `Window`, tools area | `export-wizard`, `sequence-package-import`, `sequence-package-export` | Own specs |
| Shows, Schedules, Scheduler Status | `Window`, scheduler area | `shows`, `schedules`, `scheduler-status` | Own spec |
| Preview designer | `Window`, preview area | Named by its spec | F95 awaits it |
| Sequence windows | `WindowGroup(for: SequenceDocumentRef.self)`, sequencer area | `sequencer` | F31, F32, Q22. `docs/spec/sequencer-shell.md` calls the value `SequenceRef`; `WindowID.swift` already names it `SequenceDocumentRef` |

Non-primary `Window` scenes use `.defaultLaunchBehavior(.suppressed)` and `.restorationBehavior(.disabled)`, so only Administration appears at launch and macOS does not reopen the others, which upstream never does. Scheduler Status opens only through F57.

Sheets on `administration`: `SelectProfileSheet` (F63-F66), `DataProfileSheet` (F67-F76), `SetupPreviewsSheet` (F89-F96), `CheckForUpdatesSheet` (F105-F106), `WebServerSheet` (F111). `ArchiveWizardSheet` (F77-F85) and the profile editor from Select a Profile are sheets on those sheets. Modal state keeps the menu commands off through `isBusy`, as a modal form does upstream.

### Mapping: Administration window

| Original control | Region | Mac form |
|---|---|---|
| `menuStripMain`, `System` etc. | Top | Menu bar, `.commands` |
| `logoImage` | Top left | `Image`, resizable, scaled to fit |
| `labelVixen`, `labelRelease`, `labelBuild` | Top right, stacked | `Text`s in a `VStack`; `.minimumScaleFactor` |
| `groupBoxSequences` | Left column | `GroupBox("Sequences")` |
| `buttonNewSequence`, `buttonOpenSequence` | Top of that group, full width | `Button`, `.frame(maxWidth: .infinity)` |
| `label2`, `listViewRecentSequences` | Below them, list takes the height | `Text`, `List(selection:)` |
| `contextMenuStripRecent` | On the list | `.contextMenu` |
| `contextMenuStripNewSequence` | Under the button | `Menu`, only if more than one type |
| `groupBoxSystemConfig` | Right column, top | `GroupBox("System Configuration")` |
| `buttonSetupDisplay`, `buttonSetupOutputPreviews` | Stacked in it | `Button`s, full width |
| `progressBar` | Across both columns, below | `ProgressView` with overlaid text |
| `statusStrip` | Bottom | An `HStack` with a top `Divider`, not `.toolbar`: light, spacer, state text, `Divider`, update text, `Spacer`, CPU |
| `openFileDialog` | | `NSOpenPanel` with accessory |

Use `Grid` for `titlePanel`'s 40/25/25/10 split only if plain stacks do not hold the proportion. No `GeometryReader`. No fixed sizes except the window's minimum.

### Mapping: dialogs

| Original | Mac form |
|---|---|
| `SelectProfile.listBoxProfiles` | `List(selection:)`, with `.selectionDisabled` on locked rows |
| `DataProfileForm.comboBoxProfiles`, `comboBoxLoadThisProfile` | `Picker` |
| `radioButtonAskMe`, `radioButtonLoadThisProfile` | `Picker(.radioGroup)`, the second with its `Picker` beside it |
| `buttonAddProfile`, `buttonDeleteProfile`, `buttonSetDataFolder`, `buttonSetSaveFolder` | `Button` with `Image(systemName:)` and `accessibilityLabel` |
| `TextDialog` | Sheet with `TextField`, Return accepts, Esc cancels |
| `FolderBrowserDialog` | `NSOpenPanel` |
| `DataZipForm` check boxes | `Grid` of `Toggle`s, three columns, positions as F78 |
| `statusStrip1` | Bottom row: `ProgressView`, `Text` |
| `InstalledModules.listViewModules` | `Table` with `Section`s, one selection |
| `ConfigPreviews.listViewPreviews` | `Table`, multiple selection, `Toggle` column |
| `groupBoxSelectedPreview` | `GroupBox`, disabled unless one is selected |
| `OptionsDialog.ctlUpdateInteral`, `wasapiLatency` | `TextField` and `Stepper` in a `LabeledContent` |
| `cmbBoxVideoEffectCacheFileType` | `Picker` |
| `AboutVixen.textBoxLicense`, `ReleaseNotes.textBoxReleaseNotes`, `CheckForUpdates.textBoxReleaseNotes`, `InstrumentationForm.textBox1` | Shared `ReadOnlyTextView`, `NSViewRepresentable` over `NSTextView` |
| `MessageBoxForm` | `.alert`, `.confirmationDialog`, with verbs |

### What must be AppKit

Only `ReadOnlyTextView` (H9), `NSOpenPanel` with the accessory view, and the `NSWindowDelegate` and `NSApplicationDelegate` hooks. Everything else here is short lists and plain controls, which SwiftUI handles well. The sequencer, preview and setup canvases are their areas' concern.

### Harness

Button labels are the upstream strings, so scripts can click them: `{ "action": "click", "label": "New Sequence..." }`, `{ "action": "menu", "path": ["System", "Execution Engine", "Stop"] }`. `{ "action": "window", "id": "installed-modules" }` opens by id. The harness runs with `VIXEN_DATA`, which skips profile logic, so test F3 and F63 in Rust and through `VIXEN_DATA` unset plus `VIXEN_UI_TEST_OUT` holding a prepared `Settings.xml`.

## Performance hazards

| # | Upstream | Why it hurts | Instead |
|---|---|---|---|
| H1 | `await PopulateVersionStringsAsync()` before `VixenSystem.Start` (`VA.cs:416`), with a 10 s HTTP timeout (`Program.cs:41`) | Every launch waits on GitHub, up to 10 s offline, before loading anything | Q1: check after startup, in the background |
| H2 | `AutoResizeText` on each `SizeChanged`: up to 199 font constructions and measures for each of 3 labels, then a forced `Refresh()` (`VA.cs:1322-1382`) | Up to about 600 GDI+ measures per resize event, so window resizing crawls | `.minimumScaleFactor`, no measuring |
| H3 | `ReloadSystemConfig` and module start on the UI thread (`VA.cs:982`, `:1015`); `while (IsSaving) Thread.Sleep(250)` on the UI thread at exit (`VA.cs:396-400`) | Beachball while a large config loads; a frozen window at exit | `AppCore` work off the main actor; `.terminateLater` and await |
| H4 | Setup Previews and Display Setup Cancel always reload and restart the engine (`VA.cs:976-986`, `:1011-1017`) | Cancel with no change closes every preview window, restarts every controller, and reparses a 17,000-line config | Q5 for previews. Display Setup's Cancel: skip it too when its session made no change |
| H5 | `XMLProfileSettings`: a new parse per instance and a full save per `PutSetting`, and `CreateMissingNode`'s `InnerXml +=` reparses the section (`XPS.cs:69-79`, `:147-162`). F75 does this 2 × profile count + 3 times | Slow, grows with every window that stores settings there, and instances overwrite each other | One store, `put_many`, debounce in other areas |
| H6 | `ZipArchiveMode.Update`, reopened for each pass; the update mode keeps entries in memory (`DataZipForm.cs:304-306`). `files.Count()` walks the tree again first (`:298-300`). Progress and status marshal per file | A profile with gigabytes of `Media` can exhaust memory, and passes 2 and 3 reread the whole archive | One streaming writer over all passes, zip64, progress throttled to display rate |
| H7 | Devices start serially on the thread that opened execution (`OutputDeviceExecution.cs:82-90`), the UI thread for F44 and F45 | Opening a serial port or a slow preview blocks the window, and `Invoke` back from the pool thread at startup makes it worse | `ExecutionEngine` runs off the main actor; status arrives by event; bar shows nothing extra |
| H8 | `EditorService.CreateEditor` loads the sequence on the UI thread (`VA.cs:931`, `Vixen.Core/Services/SequenceService.cs:75-85`) | A large `.tim` freezes the window | Sequencer area loads off the main actor; the Administration controls stay live |
| H9 | Plain text boxes with very long text: Release Notes has 2862 lines | SwiftUI `Text` lays it all out at once | `NSTextView`, which is lazy |
| H10 | The status strip's CPU tick repaints in place | In SwiftUI a tick on the main model would invalidate the whole window every second | Separate `@Observable` read by one `Text` (F33) |
| H11 | Instrumentation builds its form at load and rewrites `Lines` each second, which also loses the scroll position (`InstrumentationModule.cs:17-21`, `InstrumentationForm.cs:20-24`) | Startup cost for a window few people open; values with `lock`-free fields are read across threads | Create on demand; poll only while visible; keep scroll by updating text storage only if changed; atomics in `MillisecondsValue` |
| H12 | `executionStateChangedHandler` uses synchronous `Invoke`, and `AppCommand` too, and `ScheduleExecutor.Enabled` uses a synchronous `Send` from the pool thread during module load (`VA.cs:1057-1058`, `AppCommand.cs:196-200`, `App/SuperScheduler/ScheduleExecutor.cs:53`) | A pool thread blocked on a busy UI thread, a deadlock if the UI waits on it | Async hop to the main actor |
| H13 | `Download` buffers the installer in memory (`CheckForUpdates.cs:135`) | Large allocation | Not applicable: Q11 |
| H14 | `File.Exists` on up to 20 paths and a synchronous XML write on each editor close (`VA.cs:1154-1155`, `:1163-1171`) | Small, but on the UI thread, and a slow volume would show it | Off the main actor; update `recent` when done |
| H15 | New on the Mac: the default `~/Documents/Vixen 3` may be synced by iCloud | Evicted `Media` files block on download, a large show uploads, and `.lock` can travel to another Mac and read as locked there (F6) | Keep the default for parity, as `Paths.cs:54-57`. Never read media on the main actor. A profile can point anywhere (F72), and the owner already keeps this project under `~/nosync` |
| H16 | With the scheduler on, a 1 s timer posts `CheckSchedule` to the UI thread, which walks every item and may start a show there (`App/SuperScheduler/ScheduleExecutor.cs:90-95`, `:180-206`) | A tick that loads a show stalls every window while a show is meant to be running smoothly | Tick and process in Rust off the main actor; send only the status text and show changes to Swift, and only when they change |
| H17 | `Application.DoEvents()` inside Add New Preview, with a comment regretting it (`ConfigPreviews.cs:76-80`) | Re-entrant event handling during a device start | Not needed: the preview starts off the main actor and the designer opens when it reports back |
| H18 | The stop half of H7: closing execution stops devices one by one and waits on each update thread (`OutputDeviceExecution.cs:222-233`), and `Cancel` in F26 runs it on the UI thread | The window freezes for as long as the slowest device takes to stop | `ExecutionEngine` off the main actor, as H7 |

## Tests worth writing

No window needed for any of these, except the last group. **Upstream tests covering this area:** `Vixen.Tests/Application/Updates/GitHubUpdateServiceTests.cs`, ported below; `Vixen.Tests/Sys/BuiltInElementTagsTests.cs`, for the tags seeded in F1 step 25: three definitions, all built-in, stable ids, the same ids on a second call; `Vixen.Tests/LivePreview/*`, for the Live Preview service F36 starts, whose port belongs with the preview spec; `Vixen.Tests/FPPClient/*`, for the library behind a module with no UI. Nothing upstream tests `VixenApplication`, profiles, the archive, `AppCommand`, `Execution` or `Paths`.

**`updates.rs`, ported from upstream with the same inputs** (`GitHubUpdateServiceTests.cs`):
- Development: from `DevBuild-1500` (body `older build`), `DevBuild-1502` (body `chosen`, older date, with installer asset), `DevBuild-invalid`, a draft `DevBuild-9999` and a stable `3.13`, installed `1501`: latest `1502`, build 1502, available, notes `chosen`, installer URL as given, one request, path `/repos/VixenLights/Vixen/releases?per_page=5`.
- Release: `3.13u1` against installed `3.13`: available, notes `latest notes`, installer URL as given, one request to `.../releases/latest`.
- `3.14` with a null body, notes not requested, installed `3.13u1`: available, notes `None`.
- Second check within 5 minutes makes no request and returns `cached notes`.
- `release_notes("3.13u1")`: tag equal, body kept as `GitHub notes\r\nfor 3.13u1`, one request to `.../releases/tags/3.13u1`.
- Compare: `3.13` < `3.13u1`, `3.13u1` < `3.14`, `3.14` > `3.13u9`.
- 403, 429, 500: `None`. `not JSON`: `None`.

**Additional `updates.rs`:**
- `parse_release_version` rejects empty, `3`, `3.13.1`, `3.13u`, `3.13u1u2`, `a.b`, ` 3.13`, `+3.13`, `-1.2`, and takes `3.13` as update 0.
- `parse_dev_build_number`: `DevBuild-1502` gives 1502, `DevBuild-` and `devbuild-1` give none, `DevBuild--5` gives none.
- `releases/latest` returning a prerelease or a bad tag: `None`. A notes response with a different tag: `None`.
- Asset match ignores case: `x-setup-64BIT.EXE` matches.
- `startup_update_notice`: release newer gives ` Release version 3.14 available.`; development gives ` Development build 1502 available.`; failure gives none; a test build makes no request.

**`version.rs`:** every row of F114's table. `release_notes_tag`: test `None`, dev `DevBuild-1501`, release `3.13u4`. `3.13.4` from `Cargo.toml` reads `Release 3.13u4`.

**`execution.rs`:**
- Closed `open()` gives events `Opening`, `Open`, in that order; `open()` again gives none. `close()` gives `Closing`, `Closed`; `close()` again gives none.
- `open_test()` from Closed gives `Opening for testing`, `Open for testing`; then `open()` does nothing and `close()` closes.
- Status table: Closed: `Execution: Closed`, (178,34,34), start on, stop off. Open: `Execution: Open`, (34,139,34), start off, stop on. Open for testing: (30,144,255), start on, stop on.
- A device in `disabled_devices` does not start. One that fails to start does not stop the others.
- `reload_system_config` from Open ends Open; from Closed ends Closed.
- Q7: saving while closed leaves `disabled_devices` unchanged; saving while open records the non-running devices.
- Stop then Start after a tick with no save: the device listed as enabled is running again.
- Controllers start before previews, and stop before them.

**`appdata.rs`:**
- The owner's real file, copied, with a profile folder holding those files: 14 rows in F21's order.
- An absolute POSIX path to an existing file outside `Sequence` stays as is.
- `add_recent_sequence`: goes first; removes the exact match and any Windows entry with the same resolved file; 21 entries cut to 20; empty ignored.
- `clear_recent_sequences` writes nothing until a save.
- Existing tests in `appdata.rs` remain.
- `AppCore::start` with a `VixenApplicationData.xml` of bad XML, and with one that has no `DataFormatVersion`: starts, empty list, no F9. The next save replaces the file.

**`lock.rs`:** no folder, not locked. No file, not locked. Text `abc`, locked. Other machine, locked. This machine with a dead pid: not locked and the file is gone. This machine with a live pid named other than `Vixen*`: not locked and the file is gone. Text `xyz@{this machine}`: locked. `create_lock_file` twice: second fails. File text is `{pid}@{machine}`.

**`profiles.rs`, using a temp `Settings.xml`:**
- No file: `NoProfiles`. `ProfileCount` 0: `NoProfiles`.
- `LoadAction` `Ask`: `Ask` with no error. `LoadSelected` and `ProfileToLoad` -1 or equal to the count: `Ask` with no error.
- Selected folder exists: `Load`. Missing: `Ask` with error, message `Selected profile X data directory does not exist!\n\nSelect a different profile to load or use the Profile Editor to create a new profile.` Locked: `...is locked by another instance!...`.
- `list_profiles_for_selection`: sorted newest first, missing dates last, a locked row flagged, a bad date not fatal.
- `ProfileEditor::open` with no settings: one item `Default`, the default folder. `add("Halloween")`: folder `~/Documents/Vixen 3 Halloween`. `add("")`: `Blank`. `add("Default")`: `Duplicate`. `add("  ")` is accepted, since blank means exactly empty.
- Radios: stored `LoadSelected`, change to `Ask`, then `add` or `delete`: back to `LoadSelected`. Change to `Ask`, then only `save`: stored `Ask`.
- The drop-down keeps its item across `add`, and loses it when that item is deleted, and then `save` leaves `ProfileToLoad` as it was.
- `validate`: two items named the same give one duplicates line per later match, as `{name}:\t {folder}`; `C:\Vixen` is invalid; a blank name counts in `blank_count` and still saves.
- `save` writes `ProfileCount`, names and folders, `LoadAction`, `ProfileToLoad`; without a selection leaves `ProfileToLoad` as it was.

**`settings.rs`:** nested XPath creates `Profiles/Profile0/Name`. Reading back a written int, bool, date. A corrupt file gives defaults. `delete_node` on `Preview_x`. Real-format date `12/20/2025 5:43:19 PM` parses. Unknown sections survive a save. Two stores on one file: a put in the second keeps what the first wrote (F120).

**`archive.rs`:**
- `check_archive_request` per F82, in order: empty folder first, then a name of `foo.zip` trims to `foo`, `foo.ZIP` stays and writes `foo.ZIP.zip`.
- A fixture profile with all folders, `.lock` and a nested `Sequence/auto_backup` file. All on: all but `.lock`, names `Vixen 3/...`. Media off: nothing under `Media`. Sequence off: `auto_backup` gone too. A profile under `/tmp/Media Backup/Vixen 3` with Media off keeps its other folders (Q8).
- Logs on: `Vixen 3/Core Logs/...`. User Settings on: `Vixen 3/User Settings/Settings.xml`.
- `progress_value(1, 4)` = 20, `(3, 4)` = 60, `(4, 4)` = 80.
- Cancel flag set before start: valid, empty zip. Missing profile folder: an error, no hang.
- A profile folder given with a trailing separator: entries have no top folder, `SystemData/SystemConfig.xml` (F83).
- A save folder that cannot be created, and a zip that cannot be deleted: an error from `resolve_archive_prompt`, not a panic (F82).

**`instrumentation.rs`:** fresh `MillisecondsValue` formats `min 999 ms,  avg 0 ms,  max 0 ms,  cnt 1`. After `set(10.0)`, `set(20.0)`: `min 10 ms,  avg 10 ms,  max 20 ms,  cnt 3`. `reset` returns to fresh. `RateValue` formats two decimals plus ` Hz`, and a second immediate read gives 0. `Instrumentation` keeps order, and a re-add keeps the slot. `lines()` gives `{name}: {formatted}`.

**`cpu.rs`:** first call `None`; a second within 250 ms `None`; after a busy spin, a value between 0 and 100.

**`modules.rs`:** groups in F86's order; 113 rows in all, 58 of them effects; `Timed Sequence` is the only type with `can_create_new`; filters as F20; Copy Module Id text is lower case with hyphens, for example `7b791008-56a2-4bff-8ce3-a7fb89ea4637` for Alternating. Controllers run DDP, Debugging Values, DMX Pro, Dummy Lighting. Effects sort by file ignoring case, so `Module.Effect.Snowflakes` comes before `Module.Effect.SnowStorm`, and `Module.Effect.VerticalMeter` and `Module.Effect.Video` before `Module.Effect.VUMeter`. The Fixture effect's `File` is `Module.Effect.FixtureEffect`. `Tag Filter` is in the OutputFilter group, last.

**Built-in tags:** a config with no tags gains the three of `## Data and persistence` in order; one that already holds `Hidden` under that id keeps it and gains the other two. A node holding an unknown tag id loses it. Preview module data whose instance id has no preview is removed.

**Previews (F96, Q5):** an edit sets `changes_made` and `needs_reload`; a tick sets only `needs_reload`; `add`, `duplicate`, `delete`, `rename` and configure each set both. A fresh session has neither.

**Saving (F117, F118):** a store write that fails returns the error and leaves `is_saving()` false, so a later save runs and shutdown does not wait. A backup that fails returns the error and leaves the old file as it was.

**Reload (F119, Q18):** enable the scheduler, reload, disable it, save, load again from disk: it reads disabled. The same with the web server port.

**Scheduler (F56):** enabled at launch: `manually_disabled` false, items processed. Off then on: `manually_disabled` true, status `All Show Disabled`, no item processed. Then `start()`: false, and items process again. A running item is stopped by off.

**`boot.rs`:** `parse_launch_options`: `no_controllers`, `no_execution`, `data_dir=/x` set their fields; `data_dir` alone gives none; `data_dir=/a=b` gives `/a`; `VIXEN_DATA` used when no argument, argument wins. `resolve_data_folder`: existing folder kept; a missing one falls back to the default. `ensure_directories` makes the five module folders.

**`logging.rs`:** the Scheduler logger at Debug goes to `Scheduler.Debug.log`; `Microsoft.Foo` at Info goes nowhere, at Warn to `Application.Warn.log`; the line format matches `12/20/2025 17:43:19.588 [Vixen.Sys.VixenSystem] Vixen System starting up... `; above 1,000,000 bytes rolls to `archive/`.

**Through the UI harness**, with a copy of `SystemData` and `Sequence/*.tim` from the real show under `target/`, as `VIXEN_DATA`:
- Snapshot `administration`: title `Vixen Administration` (no profile, F2), 14 recent rows, `Execution:` text, version labels.
- `dump`: menu order System, Tools, Scheduler, Help, and each menu's items in F35's order.
- Stop then Start from the menu: light and text change; enabling flips.
- Open `installed-modules`: 12 group headers.
- Open Setup Previews: two rows, as F89.

## Windows-only omissions

| Upstream | Why it has no meaning on a Mac | Cite |
|---|---|---|
| WPF application object, theme dictionaries, Orc theme | No WPF | `VA.cs:65-96` |
| `ProfileOptimization` JIT profile, `~perfData.tmp` | No .NET runtime | `VA.cs:357-373` |
| Catel log listener and service registration (`IDownloadService`, `IMessageBoxService`) | No Catel | `Program.cs:27`, `NLogListener.cs`, `VA.cs:468-473`, `CustomPropEditorModule.cs:21-25`, `TimedSequencePackagerModule.cs:27-29` |
| `ApplicationConfiguration.Initialize`, `ScalingTools` | DPI and visual styles are the system's | `Program.cs:33`, `DataProfileForm.cs:17` |
| `VixenSystem.UIThread`, `UIContext`, `MainWindow.Invoke` marshalling | Swift's main actor | `VA.cs:60-61`, `AppCommand.cs` |
| `TopMost = true; TopMost = false` to raise windows | `NSApp.activate`; macOS orders windows | `VA.cs:488-502`, `ConfigPreviews.cs:268-272`, `:302-304` |
| Wait cursors | Disabled controls and the progress bar say it | `VA.cs:414`, `:458`, and others |
| Status strip grip workaround, theme renderers, owner-drawn group boxes and combo boxes | Native controls | `VA.cs:104-110`, `:1306-1309`, `DataProfileForm.cs:329-332` |
| `AutoResizeText`, `ColumnAutoSize`, `_sequenceGroupBoxOffsetFromBottom` | Layout containers do this | `VA.cs:180`, `:1177-1185`, `:1322-1382` |
| `notepad.exe` | `NSWorkspace.open` | `VA.cs:1107` |
| `SHGetKnownFolderPath` for Downloads | Not used, since F106 does not download | `KnownFolders.cs` |
| `-Setup-64bit.exe` installer selection and download | Windows installer | `GitHubUpdateService.cs:12`, `CheckForUpdates.cs:129-140` (Q11) |
| `BeginEnhancedResolution` and `EndEnhancedResolution` (`winmm` timer period) | macOS has no such global timer setting | `State/Behavior/StandardOpeningBehavior.cs:10-11`, `StandardClosingBehavior.cs:9-10` |
| NAudio device enumeration, power mode events, WASAPI share mode | `vixen-audio` uses CoreAudio. Only `Latency` carries over. `SoundOutDeviceId` belongs to the sequencer's audio menu | `AudioOutputManager.cs` |
| `CpuUsage` P/Invoke of `GetSystemTimes` | `getrusage` | `CpuUsage.cs:10-15` (Q6) |
| `SetLogFilePaths` and `[VIXENPROFILEDIR]` | The token is gone from `NLog.config`, so the call changes nothing | `VA.cs:704-741` |
| `Modules.LoadAllModules` file scan, `Module.*.dll` | Modules are compiled in; the order is fixed in F36 | `Modules.cs:41-48` |
| `SystemConfig.IsContext` beside the binary, `ConfigurationManager.AppSettings["DataPath"]` | An app bundle's inside is read-only, and there is no `app.config`. `is_context` still round-trips in `SystemData` | `VS.cs:442-466`, `app.config` |
| `AboutVixen`, `ReleaseNotes` reading from `Paths.BinaryRootPath` | Bundle resources | `AboutVixen.cs:21`, `ReleaseNotes.cs:70` |
| `Path.GetInvalidPathChars`, backslash in the profile path check | POSIX rules (F75, F82) | `DataProfileForm.cs:107`, `DataZipForm.cs:178`, `:195` |
| `VersionBeyondWindowsXP`, OpenGL 3.3 probe | No such choice; `vixen-render` uses Metal | `VS.cs:405-416`, `VixenPreviewModuleInstance.cs:60-107` |
| `MessageBoxForm`'s static icon field and manual height fit | `.alert` | `MBF.cs:9`, `:93-103` |
| `Application.ProductName` as the assembly name, `Vixen.Application` | The app's name is `Vixen` (Q21) | `VA.cs:665-667`, `DataProfileForm.cs:162` |
| NLog's internal log, `logs/nlog-internal.log`, and `autoReload` | No NLog. The Mac's log facade has no config file to watch | `NLog.config:5-7` |
| Log lines about the JIT profile and the .NET runtime version | No such things here (F116) | `VA.cs:133-135`, `:371`, `:538-539` |
| `Application.DoEvents` when adding a preview, and `ShowInTaskbar = false` | A sheet has no taskbar entry, and the start does not block (H17) | `ConfigPreviews.cs:23`, `:80` |
| The waits for a Windows file lock before a save, `IsFileLocked` | Sharing violations are a Windows matter; `vixen-io` writes atomically | `Vixen.Core/IO/Xml/XElementFileWriter.cs:18-22`, `:52-68` |
| The `MessageBox.Show` for an exception in Installed Modules | The Mac's table is static and cannot throw | `InstalledModules.cs:45-48` |
