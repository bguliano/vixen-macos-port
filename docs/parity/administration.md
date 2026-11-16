# Administration parity

Status of every entry in `docs/spec/administration.md`, then of every upstream control.

**Status.** **done**: built, and exercised where the last column says so. **partial**: built, with what's missing named. **elsewhere**: this area only opens it or leaves it a place, and another area owns the rest. **omitted**: not built, with why.

**Paths.** `Admin/` is `app/Sources/VixenAdmin/`. `Kit/` is `app/Sources/VixenKit/`. `FFI` is `crates/vixen-ffi/src/admin.rs`. `session/` is `crates/vixen-session/src/`. `io/` is `crates/vixen-io/src/`.

**Windows owned by other areas.** The sequencer, Display Setup, the preview designer, Custom Prop Editor, Export Wizard, the two Sequence Package wizards, Web Server, Shows, Schedules and Scheduler Status open by window id. With `VIXEN_AREAS=admin` those windows don't exist, so the buttons and menu items that open them do nothing. That's expected until those areas are in the build.

**How things were checked.** "Rust" means a unit test in the named file. "Harness" means run through `VIXEN_UI_TEST` with a copy of the real show or a throwaway folder, with the snapshot or dump read back. Anything else says why it wasn't run.

## Features

| F | Feature | Status | Where | Checked, and what's different |
|---|---|---|---|---|
| F1 | Launch order | done | `Admin/AdminModel.swift` `start()`, `session/boot.rs` `AppCore::start` | Harness, log lines in order. The update check is no longer a step of the launch: see F29 |
| F2 | Command line | done | `session/boot.rs` `parse_launch_options`, `LaunchOptions::data_folder`; `NSTreatUnknownArgumentsAsOpen` in `VixenMacApp.init` | Rust, and harness for `no_execution`, `no_controllers`, `data_dir=` with a folder that exists and one that doesn't. `VIXEN_DATA` that doesn't exist is made rather than falling back, so a test can't reach the real default folder |
| F3 | Profile resolution at launch | done | `io/profiles.rs` `decide_startup_profile`, `check_profile`; `Admin/AdminModel.swift` `resolveProfile`, `askForProfile` | Rust, harness with a prepared `Settings.xml` |
| F4 | Profile load error | done | `io/profiles.rs` `ProfileLoadError::message`, `AdminModel.showLoadError` | Rust, harness. Adds `Open Anyway` when the lock names another machine (Q3) |
| F5 | Leaving without a profile | done | `Admin/SelectProfileSheet.swift` `quit` | Harness: `Exit` and `Keep Selecting` |
| F6 | Profile lock | done | `io/lock.rs`, `AdminModel.takeLock` | Rust; harness for a second live instance, a killed instance, another machine's lock |
| F7 | Choosing the data folder | done | `session/boot.rs` `resolve_data_folder`, `load_application_data_or_empty` | Rust. A damaged file logs and gives an empty list |
| F8 | Data folders created at launch | done | `io/paths.rs` `ensure_directories`, `app/Info.plist` `NSDocumentsFolderUsageDescription` | Rust, harness (first launch). The Documents prompt itself wasn't seen: tests use a throwaway `HOME` |
| F9 | Startup failure | done | `AdminModel.start` catch | Harness with a config that can't be read: the alert, the error in `Application.Error.log`, the lock removed, the app gone |
| F10 | Development build warning | done | `AdminModel.start` | Not run: this version is a release, so the branch can't be reached without changing the version |
| F11 | Crash path | done | `session/logging.rs` `install_crash_hooks`, `fatal_exit`; nonisolated `logUncaughtException` callback in `AdminModel.swift` | Rust subprocess tests verify fatal exit 1, queued log flush, five-second save grace and lock removal, plus caught-panic survival. A real native exception in a disposable muted app verified fatal message/stack, exit 1 and lock cleanup (`target/admin-fatal-ui-fixed/result.json`). This found and fixed an inherited MainActor callback trap on background exceptions. Source profile hashes unchanged. |
| F12 | Shutdown | done | `AdminModel.shouldTerminate`, `runShutdown`; `session/boot.rs` `AppCore::shutdown`; `Kit/AppHooks.swift` | Harness: menu and closing the window both stop the engine, keep both config files byte for byte, save the application data and remove the lock. With the sequencer in the build and an unsaved sequence open: `Close Vixen?`, `Keep Open` really cancels, and `Close Vixen` goes on to the save prompt with no Cancel |
| F13 | Window | done | `Admin/AdministrationView.swift`, `Admin/AdministrationScene.swift` | Harness snapshots |
| F14 | Logo | done | `AdministrationView.logo` | Harness |
| F15 | "Vixen" label | done | `AdministrationView.titlePanel` | Harness |
| F16 | Release label | done | `AdministrationView.versionLabel`, `session/version.rs` | Harness shows `Release 3.13u4`. Yellow and red branches by code only |
| F17 | Build label | done | as F16 | Harness shows `Build #1`, from `CFBundleVersion` |
| F18 | Sequences group | done | `AdministrationView.sequences` | Harness |
| F19 | New Sequence... | done | `AdminModel.newSequence`, `AdministrationView.newSequenceButton` | Opens the sequencer's window id. Opening errors are the sequencer's to show |
| F20 | Open Sequence... | done | `Admin/OpenSequencePanel.swift`, `AdminModel.chooseSequencesToOpen`, `openFiles`, `open`; `.vix` in `app/Info.plist` | Harness: the panel opens with its file type menu, and Finder's Open With (`open -n`) queued a file until startup finished. A file can't be picked in the panel from the harness, because its contents are out of process. Drops take the same path, not run. Importing a `.vix` is the sequencer area's |
| F21 | Recent Sequences list | done | `io/appdata.rs` `visible_recent_sequences`, `AdministrationView.recentList`, `AdministrationCommands` File > Open Recent | Rust with the owner's 20 entries, harness shows them in order |
| F22 | Open a recent sequence | done | `AdminModel.openRecent` | Harness: Return on a selected row, File > Open Recent, and the alert for a file deleted while the app ran. A real double-click can't be sent, as test windows are never key |
| F23 | Clear Recent Sequences | done | `AdminModel.clearRecents`, `io/appdata.rs` | Rust: no save until the next one |
| F24 | System Configuration group | done | `AdministrationView.systemConfiguration` | Harness |
| F25 | Setup Display | elsewhere | Button and menu open `WindowID.displaySetup`. `AppModel.beginSystemConfigSession` and `endSystemConfigSession`, then `AdminModel.finishDisplaySetup` do the progress and the save or reload | The save and reload path ran through Setup Previews. The Display Setup window must call those two, which isn't wired on its side yet |
| F26 | Setup Previews | done | `AdminModel.showSetupPreviews`, `finishSetupPreviews` | Harness: OK saves, Cancel reloads only if something changed (Q5) |
| F27 | Progress bar | done | `AdministrationView.progressBar` | Harness caught it mid-save. The text sits above the native thin bar rather than on it |
| F28 | Execution light | done | `AdministrationView.statusBar`, `session/execution.rs` | Harness: green and red |
| F29 | Update notice | partial | `session/updates.rs` `update_notice`, `AdminModel.checkForUpdates` | **No check runs at launch**, because no network call may run without the user asking. The notice appears once the user has run Help > Check for Updates and it found something newer. Rust for the text |
| F30 | Execution state text | done | `AdministrationView.statusBar`, `AdminModel.refreshExecutionStatus` | Harness |
| F31 | Opening an editor | elsewhere | `AppModel.editorOpened`, `editorActivated`, `OpenEditor` in `Kit/AppModel.swift` | The sequencer area reports through these, and did in a combined build |
| F32 | Closing an editor | elsewhere | `AppModel.editorClosed`, `AdminModel.sequenceClosed`, `io/appdata.rs` `add_recent_sequence` | The list side is done. Rust, and a combined build: closing a sequence put its POSIX path first and dropped the Windows row for the same file, with the sequence file unchanged. The prompt and save belong to the sequencer window |
| F33 | CPU readout | done | `Admin/CpuReadout.swift`, `session/cpu.rs` | Rust, harness shows `CPU: 0%` |
| F34 | Enabling during startup and busy periods | done | `AdminModel.updateBusy`, `AppModel.isBusy`, `MenuState` | Harness: menus and buttons off during a sheet and a save |
| F35 | Menu bar at run time | done | `Admin/AdministrationCommands.swift` | Harness `menus` dump. A `View` menu stays between Edit and System, because that's where macOS puts Enter Full Screen and the sequencer's View items go |
| F36 | App modules at load | done | `AppCore::add_service` starts registered scheduler and web services after boot; shutdown drains them. Live preview uses owned lazy contexts. `marks/string_maps.rs` migrates non-matrix LipSync maps once during startup, preserving existing face instances; no immediate save | Startup/service and migration tests pass in full workspace suite; native scheduler, web and State Preview integrations verified. Earlier no-service/no-migration note was stale |
| F37 | Execution engine | done | `session/execution.rs`, `session/playback.rs` | Rust: event order, status table, device order, restart rule, Q7. Harness: Stop and Start |
| F38 | Standard Mac menus | done | `AdministrationCommands`, `AdministrationScene` | Harness dump. About replaces the app item, Settings comes from the `Settings` scene |
| F39 | System > Logs | done | `SystemMenuItems`, `session/logging.rs` `list_log_files` | Rust, harness: under test it writes the file's address to `notes.txt` rather than opening an editor |
| F40 | System > View Installed Modules | done | `SystemMenuItems` | Harness |
| F41 | System > Profiles... | done | `SystemMenuItems`, `AdminModel.profilesEdited` | Harness, with its restart alert |
| F42 | System > System Configuration > Setup Display | elsewhere | as F25 | |
| F43 | System > System Configuration > Setup Previews | done | as F26 | Harness |
| F44 | System > Execution Engine > Start | done | `AdminModel.startExecution` | Harness: enabling flips |
| F45 | System > Execution Engine > Stop | done | `AdminModel.stopExecution` | Harness |
| F46 | System > Shutdown and Exit | done | `SystemMenuItems` | Harness |
| F47 | Tools > Options... | done | `OptionsMenuItem` | Harness |
| F48 | Tools > Custom Prop Editor | elsewhere | `openWindow(id: WindowID.customPropEditor, value: UUID())` | |
| F49 | Tools > Export Wizard | elsewhere | `WindowID.exportWizard` | |
| F50 | Tools > Instrumentation... | done | `ToolsMenuItems` | Harness |
| F51 | Tools > Import Sequence Package | elsewhere | `WindowID.sequencePackageImport` | |
| F52 | Tools > Export Sequence Package | elsewhere | `WindowID.sequencePackageExport` | |
| F53 | Tools > Web Server | elsewhere | `WindowID.webServer` | |
| F54 | Scheduler > Shows | elsewhere | `WindowID.shows` | |
| F55 | Scheduler > Schedules | elsewhere | `WindowID.schedules` | |
| F56 | Scheduler > Enabled | elsewhere | `SchedulerMenuItems`, `AppModel.schedulerEnabledToggle` | The item is in place and disabled until the scheduler area sets the toggle |
| F57 | Scheduler Status window | elsewhere | `WindowID.schedulerStatus` | The scheduler area's |
| F58 | Help > Online Help | done | `HelpMenuItems` | Harness: the address went to `notes.txt` |
| F59 | Help > Vixen YouTube Channel | done | `HelpMenuItems` | Harness: the address went to `notes.txt` |
| F60 | Help > Check for Updates | done | `HelpMenuItems` | Harness. Off on a test build, by code |
| F61 | Help > Release Notes | done | `HelpMenuItems`, `UpdateChecker.releaseNotesText` | Harness |
| F62 | Help > About Vixen | done | `HelpMenuItems`, `AboutMenuItem` | Harness |
| F63 | Select a Profile: window and list | done | `Admin/SelectProfileSheet.swift`, `io/profiles.rs` `list_profiles_for_selection` | Rust, harness: order, `- Locked`, stale lock cleared |
| F64 | Select a Profile: Load | done | `AdminModel.profileChosen`, `mark_profile_loaded` | Harness: date written, title set. `Load` is off with no selection |
| F65 | Select a Profile: Profile Editor | done | `SelectProfileSheet` | Same sheet as F67, on top |
| F66 | Select a Profile: leaving | done | `SelectProfileSheet` `Quit Vixen` | Harness |
| F67 | Data Profile: window | done | `Admin/DataProfileSheet.swift`, `io/profiles.rs` `ProfileEditor` | Harness |
| F68 | Data Profile: drop-down | done | `DataProfileSheet.choose` | Rust |
| F69 | Add profile (+) | done | `DataProfileSheet.addProfile`, `ProfileEditor::add` | Rust, harness |
| F70 | Delete profile (−) | done | `DataProfileSheet.delete`, `ProfileEditor::delete` | Rust |
| F71 | Profile Name | done | `DataProfileSheet` | Rust |
| F72 | Data Folder | done | `DataProfileSheet.chooseDataFolder` | Rust. The panel itself wasn't driven |
| F73 | Profile to Use when Loading Vixen | done | `DataProfileSheet`, `ProfileEditor` | Rust, including that + and − put the radios back |
| F74 | Zip Wizard | done | `DataProfileSheet` | Harness |
| F75 | Data Profile: OK | done | `DataProfileSheet.save`, `ProfileEditor::{validate, folders_to_create, save}` | Rust, harness: folder prompt, file written once, last |
| F76 | Data Profile: Cancel | done | `DataProfileSheet` | Harness |
| F77 | Archive: window | done | `Admin/ArchiveWizardSheet.swift` | Harness |
| F78 | Archive: check boxes | done | `ArchiveWizardSheet`, `session/archive.rs` | Rust, harness |
| F79 | Archive: profile | done | `ArchiveWizardSheet`, `io/profiles.rs` `archive_profile_choices` | Rust, harness. A `Picker`, so a typed name that doesn't match can't happen |
| F80 | Archive: folder | done | `ArchiveWizardSheet` | Harness typed one. The panel itself wasn't driven |
| F81 | Archive: file name | done | `ArchiveWizardSheet` | Harness |
| F82 | Archive: Start | done | `session/archive.rs` `check_archive_request`, `resolve_archive_prompt` | Rust, harness: `Create Folder`, `Replace` |
| F83 | Archive: the run | done | `session/archive.rs` | Rust, harness: a real zip of the show copy, read back with Python |
| F84 | Archive: Stop | done | `ArchiveTask.stop` | Rust, and harness: stopped mid-run with `Media` in, leaving a valid zip of what it had |
| F85 | Archive: Close | done | `ArchiveWizardSheet` | Harness |
| F86 | Installed Modules: window and list | done | `Admin/SmallWindows.swift`, `session/modules.rs` | Rust, harness. Sections scroll with the table |
| F87 | Copy Module Id | done | `InstalledModulesView.copyId` | Harness (a private pasteboard under test) |
| F88 | Installed Modules: Close | done | `InstalledModulesView` | |
| F89 | Setup Previews: window and list | done | `Admin/SetupPreviewsSheet.swift`, `session/preview_devices.rs` | Harness: the owner's two rows, ticks as the spec says |
| F90 | Tick a preview | done | `SetupPreviewsSheet.setRunning`, `AppCore::set_preview_running` | Rust |
| F91 | Add New Preview | done | `SetupPreviewsSheet.add`, `AppCore::add_preview` | Rust, harness. The designer is the preview area's |
| F92 | Duplicate Selected | done | `AppCore::duplicate_previews` | Rust, harness |
| F93 | Delete Selected | done | `AppCore::delete_previews` | Rust, harness. Removes both `Preview_` and `OpenGLPreview_` keys |
| F94 | Selected Preview: Name | done | `SetupPreviewsSheet.update` | Harness |
| F95 | Configure Preview | elsewhere | `SetupPreviewsSheet.design`, `AppModel.previewDesignerClosed` | Stops, opens `WindowID.previewSetup` with the id, restarts when told or when the sheet closes. The designer window must take that value and report closing |
| F96 | Setup Previews: OK and Cancel | done | `SetupPreviewsSheet.cancel`, `AppCore::{preview_changes_made, preview_needs_reload}` | Rust, harness |
| F97 | Options: window | done | `Admin/OptionsForm.swift`, `Settings` scene | Harness |
| F98 | Update Interval | done | `OptionsForm`, `AppCore::set_options`, `ShowPlayback::set_update_interval` | Rust, harness: the stepper went 10 to 33 and reached the file when the window closed. Typing in the field wasn't driven: harness text doesn't reach that field |
| F99 | Latency | done | `OptionsForm`, `io/audio_prefs.rs`, `AudioPlayer::set_latency_ms`, `cpal_backend.rs` `buffer_frames` | Rust, harness: `AudioDevice.settings` written at once. Not played, since tests make no sound |
| F100 | Clear Effect Cache on Exit | done | `OptionsForm`; profile Video cache leases | Release AppKit139-step workflow observes actual toggle, retained PNG reopen and clear-on-close removing frames/pairings/staging. Rust/AVFoundation tests also cover shutdown and shared readers. See `docs/review/video-cache.md` |
| F101 | Cache File Type | done | `OptionsForm`; streamed BMP/PNG cache writers | Release AppKit workflow changes PNG→BMP and observes the selected Cache Size label0.07→0.55MB against actual encoded bytes; separate34-step AVI app run passes. Stored legacy values remain unchanged; JPG/GIF/ICO writer tests pass. See `docs/review/video-cache.md` |
| F102 | Options: OK and Cancel | done | `AdminModel.saveOptionsIfChanged` | Rust. No buttons: Settings applies live and saves when it closes (Q12) |
| F103 | About Vixen | done | `Admin/SmallWindows.swift` `AboutView` | Harness |
| F104 | Update check rules | done | `session/updates.rs` | Rust, the upstream tests ported |
| F105 | Check for Updates window | done | `Admin/CheckForUpdatesSheet.swift`, `Admin/UpdateClient.swift`, `update_dialog_content` | Rust, and harness for all three outcomes against a local test server through the real `URLSession` client. Never run against GitHub itself, since tests stay off the network |
| F106 | Download | done | `CheckForUpdatesSheet` | Harness: the address went to `notes.txt`. Opens the release page (Q11) |
| F107 | Release Notes window | done | `ReleaseNotesView`, `updates.rs` `release_notes_text` | Rust, harness: the bundled file, and a release body from the local server |
| F108 | Instrumentation window | done | `InstrumentationView`, `AppCore::instrumentation_lines` | Rust with a dummy controller, harness. See below for the rows that have no counterpart |
| F109 | Instrumentation: Reset | done | `InstrumentationView` | Rust |
| F110 | Instrumentation: Close | done | `InstrumentationView` | Timer stops with the window |
| F111 | Web Server dialog | done | `VixenAdmin/WebServerSheet.swift`, `ffi/web.rs`; bundled upstream logo, bounded80–64000 field, enabled link, Cancel/OK |55 native steps verify real menu, checkbox, typed port, cancellation and save/reopen;8 fresh-launch steps confirm persistence. `target/progress-ui/web-settings-complete`, `web-reload` |
| F112 | Applying web server settings | done | `session/web/mod.rs` via `AppCore::add_service`; transactional ModuleStore save, bind failure reporting, restart/disable/shutdown, execution-reopen recovery, no listener under UI tests |8 lifecycle tests;14 native failed-save steps preserve draft/report error/restore stored settings. `target/progress-ui/web-save-error`; real session HTTP/page verification tracked in scheduler parity F65–74 |
| F113 | Log files | done | `session/logging.rs` | Rust, harness |
| F114 | Version info | done | `session/version.rs`, `version_info` | Rust |
| F115 | `Settings.xml` | done | `io/settings.rs` | Rust |
| F116 | Log lines | done | `session/logging.rs`, `playback.rs` and `output/runner.rs` | Controller lifecycle, update and unexpected-worker failures now reach the application log with upstream logger/message text. Module I/O errors report at their source, including driver-owned workers between updates or after Stop. Failed starts send no frames; Start reaps failed workers and recovers directly.22 runner tests cover recovery, background errors after Stop, logs, sibling isolation and callback failure; session compiles. Web/scheduler/live-preview logging is verified in those areas |
| F117 | A save that fails | done | `session/app.rs` `Saving`, `AdminModel.reportSaveFailure` | Rust, harness with a read-only `SystemData`: an alert for each file, and the app stays up and quits normally |
| F118 | The backup before a save | done | `io/file.rs` | Rust: the old file stays and the error comes back |
| F119 | What a reload does to the app modules | done | `session/boot.rs` `AppService::reloaded` | Rust |
| F120 | A second copy of Vixen | done | `io/settings.rs` `current` | Rust. No menu item, as the spec says: `open -n -a Vixen --args data_dir=<folder>` starts one |

## Instrumentation rows with no counterpart

These names from F108 aren't shown, because nothing on the Mac measures what they measured:

- `System allowed block`, `System denied update`, `System denied block`: one frame thread runs without an adjudicator and without that lock.
- `   Elements update`: elements come out of the same evaluation as contexts, which `   Contexts update` times.
- `{device} triggered engine update`: a controller doesn't drive the engine here.

`Update time for preview`, `Preview pixel set time` and the OpenGL rows belong to the preview area, through `AppCore.instrumentation`.

## Quirk decisions

| Q | Decision |
|---|---|
| Q1 | No startup check at all, which goes further than the spec's background check: no network call without the user asking. See F29 |
| Q2 | Load errors reach the alert, and a migration save finishes before the engine opens |
| Q3 | **Open question, built as the spec proposes.** `Open Anyway` in F4 and in the launch alert when the lock names another machine |
| Q4 | `Keep Open` really cancels, through `.terminateLater` |
| Q5 | Cancel reloads only if something changed, a tick included. Ticks alone don't ask |
| Q6 | Corrected CPU formula, `Resource Usage` until the first reading |
| Q7 | Devices are recorded as disabled only while the engine is open |
| Q8 | Exclusions look below the profile folder |
| Q9 | Fields commit first, and nothing is written until every question has its answer |
| Q10 | All three headings show |
| Q11 | **Open question, built as the spec proposes.** Checks upstream's releases when the user asks, and `Download` opens the release page. The button's tooltip says the installers there are for Windows |
| Q12 | Settings applies live and saves on close |
| Q13 | A new profile shows ticked and `bmp` |
| Q14 | `File` shows the upstream module name without `.dll` |
| Q15 | The box follows the device |
| Q16 | Both window position keys go. The preview area should use one of those two names |
| Q17 | Built and locally verified in `vixen-web` and session web lifecycle/API; native settings and bundled page verified. See shows/scheduler/export F65–74 and `docs/VERIFICATION.md` |
| Q18 | One module store, and services get `reloaded` |
| Q19 | An alert, the flag clears, the app stays up |
| Q20 | No prompt: the old file stays and the error shows |
| Q21 | `Vixen` |
| Q22 | **Open question, as the spec's default.** `openWindow` with an equal `SequenceDocumentRef.existing` focuses the open window |
| Q23 | A `Picker` |

## Not run

- **Outside the harness.** Every run was a harness run. The harness sets an accessory activation policy and keeps windows off screen, so the app hasn't been seen with its real menu bar, with `NSApp.activate`, or on the real default folder with its Documents prompt.
- A file chosen in an open panel (F20, F72, F80), and dropping files (F20).
- GitHub itself (F104 to F107): the real client ran only against a local test server.
- Typing in the Options number fields (F98, F99), which harness text doesn't reach. Their steppers ran.

## Beyond the spec

- `Kit/UITestHarness.swift` gained `menus` (dumps the menu bar), `selectRow` (a test window is never key, so a click can't select a table row), `closeWindow`, and alert support. `menu` now runs its action from the run loop, as a real event does, and asks the menu to update first.
- `VixenMacApp.init` ignores saved window state. The original never reopens windows, and saved state listing none made macOS skip the main window altogether, in every build sharing the bundle id. It also stops the command line's words reaching `application(_:open:)`, and the Administration window is `.defaultLaunchBehavior(.presented)`, so it opens even when the app starts by opening a file.
- `Error.userMessage` in `Kit/ErrorMessage.swift`: Rust's message for a `FfiError`, where `localizedDescription` gives the enum's description.
- `SystemConfigOutcome.performed` in `Kit/AppModel.swift`, for Display Setup, whose session commits its own files and reports progress.

## Controls

### `VixenApplication`

| Control | Text | Status | Where |
|---|---|---|---|
| `menuStripMain` | | done | The menu bar |
| `vixenToolStripMenuItem` | System | done | `SystemMenuItems` |
| `logsToolStripMenuItem` | Logs | done | `SystemMenuItems` |
| `viewInstalledModulesToolStripMenuItem` | View Installed Modules | done | `SystemMenuItems` |
| `profilesToolStripMenuItem` | Profiles... | done | `SystemMenuItems` |
| `systemConfigurationToolStripMenuItem` | System Configuration | done | `SystemMenuItems` |
| `setupDisplayToolStripMenuItem` | Setup Display | done | `SystemMenuItems` |
| `setupPreviewsToolStripMenuItem` | Setup Previews | done | `SystemMenuItems` |
| `executionToolStripMenuItem` | Execution Engine | done | `SystemMenuItems` |
| `startToolStripMenuItem` | Start | done | `SystemMenuItems` |
| `stopToolStripMenuItem` | Stop | done | `SystemMenuItems` |
| `exitToolStripMenuItem` | Shutdown and Exit | done | `SystemMenuItems`. The `&x` mnemonic has no meaning here |
| Tools, Scheduler, Help (run time) | | done | `ToolsMenuItems`, `SchedulerMenuItems`, `HelpMenuItems` |
| `contextMenuStripNewSequence` | | done | `newSequenceButton`, a `Menu` only with a second creatable type, of which there are none |
| `openFileDialog` | | done | `OpenSequencePanel` |
| `contextMenuStripRecent` | | done | `.contextMenu` on the list |
| `toolStripItemClearSequences` | Clear Recent Sequences | done | With its tooltip |
| `toolStripStatusLabelExecutionLight` | | done | `statusBar` |
| `toolStripStatusLabel1` | (two spaces) | done | The bar's spacing |
| `toolStripStatusLabelExecutionState` | Execution: Unknown | done | `statusBar`. Never reads Unknown, as the first update comes before the first paint |
| `toolStripStatusUpdates` | Updates | done | `statusBar`, empty until a check finds something |
| `toolStripStatusLabel_memory` | Resource Usage | done | `CpuReadoutText` |
| `statusStrip` | | done | `statusBar` |
| `progressBar` | | done | `progressBar` |
| `groupBoxSequences` | Sequences | done | `sequences` |
| `buttonNewSequence` | New Sequence... | done | `sequences` |
| `buttonOpenSequence` | Open Sequence... | done | `sequences` |
| `label2` | Recent Sequences: | done | `sequences` |
| `listViewRecentSequences`, `columnHeader1` | | done | `recentList`. No header, as upstream |
| `groupBoxSystemConfig` | System Configuration | done | `systemConfiguration` |
| `buttonSetupDisplay` | Setup Display | done | `systemConfiguration` |
| `buttonSetupOutputPreviews` | Setup Previews | done | `systemConfiguration` |
| `logoImage` | | done | `logo` |
| `labelVixen` | Vixen | done | `titlePanel` |
| `labelRelease` | Release | done | `versionLabel` |
| `labelBuild` | Build | done | `versionLabel` |
| `listViewItem1` to `6` | | omitted | The designer's placeholder rows, cleared at startup upstream |

### `SelectProfile`

| Control | Text | Status | Where |
|---|---|---|---|
| `label1` | Select a Profile to Load: | done | `SelectProfileSheet` |
| `listBoxProfiles` | | done | `SelectProfileSheet` |
| `buttonEditor` | Profile Editor | done | `SelectProfileSheet` |
| `buttonLoad` | Load | done | `SelectProfileSheet` |
| Title bar close | | done | `Quit Vixen`, since a sheet has no close box |

### `DataProfileForm`

| Control | Text | Status | Where |
|---|---|---|---|
| `groupBox2` | Profiles: | done | `DataProfileSheet` |
| `label1`, `comboBoxProfiles` | Profile: | done | `DataProfileSheet` |
| `buttonAddProfile` | + | done | SF Symbol `plus` |
| `buttonDeleteProfile` | - | done | SF Symbol `minus` |
| `label2`, `textBoxProfileName` | Profile Name: | done | `DataProfileSheet` |
| `label3`, `textBoxDataFolder` | Data Folder: | done | `DataProfileSheet` |
| `buttonSetDataFolder`, `folderBrowserDataFolder` | | done | SF Symbol `folder`, `chooseFolder` |
| `groupBox1` | Profile to Use when Loading Vixen | done | `DataProfileSheet` |
| `radioButtonAskMe` | Ask Me Every Time | done | `RadioChoice` |
| `radioButtonLoadThisProfile` | Load This Profile: | done | `RadioChoice` |
| `comboBoxLoadThisProfile` | | done | `DataProfileSheet` |
| `buttonZipWizard` | Zip Wizard | done | `DataProfileSheet` |
| `buttonOK` | OK | done | `DataProfileSheet` |
| `buttonCancel` | Cancel | done | `DataProfileSheet` |

### `DataZipForm`

| Control | Text | Status | Where |
|---|---|---|---|
| `groupBox2` | Profile artifacts to include | done | `ArchiveWizardSheet` |
| `checkBoxModule`, `checkBoxProgram`, `checkBoxSequence`, `checkBoxSystem`, `checkBoxTemplate`, `checkBoxLogs`, `checkBoxMedia`, `checkBoxUserSettings` | | done | `ArchiveWizardSheet`, positions as F78 |
| `label1`, `comboBoxProfiles` | Profile to archive | done | `ArchiveWizardSheet` |
| `label2`, `textBoxSaveFolder` | Folder to save archive to | done | `ArchiveWizardSheet` |
| `buttonSetSaveFolder`, `folderBrowserSaveFolder` | | done | SF Symbol `folder`, `chooseFolder` |
| `label3`, `textBoxFileName` | Archive file name, without extension | done | `ArchiveWizardSheet` |
| `buttonStartCancel` | Start | done | `ArchiveWizardSheet` |
| `buttonClose` | Close | done | `ArchiveWizardSheet` |
| `statusStrip1`, `toolStripProgressBar`, `toolStripStatusLabel` | | done | The row at the bottom |
| `groupBox1` | (no text) | omitted | A frame round the fields, with no caption or function |

### `InstalledModules`

| Control | Text | Status | Where |
|---|---|---|---|
| `listViewModules` and its five headers | Name, Description, Author, Version, File | done | `InstalledModulesView` |
| `buttonCopyModuleId` | Copy Module Id | done | `InstalledModulesView` |
| `buttonClose` | Close | done | `InstalledModulesView` |

### `ConfigPreviews`

| Control | Text | Status | Where |
|---|---|---|---|
| `label3`, `label4` | Name, Type | done | The table's header |
| `listViewPreviews`, `columnHeader1`, `columnHeader2` | | done | `SetupPreviewsSheet` |
| `buttonAddPreview` | Add New Preview | done | `SetupPreviewsSheet` |
| `buttonDuplicateSelected` | Duplicate Selected | done | `SetupPreviewsSheet` |
| `buttonDeletePreview` | Delete Selected | done | `SetupPreviewsSheet` |
| `groupBoxSelectedPreview` | Selected Preview | done | `SetupPreviewsSheet` |
| `label2`, `textBoxName` | Name: | done | `SetupPreviewsSheet` |
| `buttonUpdate` | Update | done | `SetupPreviewsSheet` |
| `buttonConfigurePreview` | Configure Preview | done | `SetupPreviewsSheet` |
| `label1` | Configure details specific to the preview. | done | `SetupPreviewsSheet` |
| `buttonOk` | OK | done | `SetupPreviewsSheet` |
| `buttonCancel` | Cancel | done | `SetupPreviewsSheet` |

### `OptionsDialog`

| Control | Text | Status | Where |
|---|---|---|---|
| `label1` | This form lets you set various options that control V3's operation | done | `OptionsForm`, as a caption |
| `label2`, `ctlUpdateInteral` | Update Interval: | done | `OptionsForm` |
| `chkBoxClearEffectCacheOnExit` | Clear Effect Cache on Exit | done | `OptionsForm`, with its tooltip |
| `grpAudio`, `label3`, `wasapiLatency` | Audio, Latency: | done | `OptionsForm` |
| `grpVideoEffectCache`, `label4`, `cmbBoxVideoEffectCacheFileType` | Video Effect Cache, Cache File Type: | done | `OptionsForm`, with its tooltip |
| `btnOK`, `btnCancel` | OK, Cancel | omitted | Settings applies live and saves when it closes (Q12) |

### `AboutVixen`

| Control | Text | Status | Where |
|---|---|---|---|
| `pictureBoxIcon` | | done | `AboutView` |
| `labelHeading` | | done | `AboutView` |
| `textBoxLicense` | | done | `AboutView`, read only |
| `btnOK` | OK | done | `AboutView` |

### `CheckForUpdates`

| Control | Text | Status | Where |
|---|---|---|---|
| `pictureBoxIcon` | | done | `CheckForUpdatesSheet` |
| `labelHeading` | | done | `CheckForUpdatesSheet` |
| `labelCurrentVersion` | | done | `CheckForUpdatesSheet` |
| `buttonDownload` | Download | done | `CheckForUpdatesSheet`, opens the release page |
| `lblChangeLog` | Change Log for latest version: | done | `CheckForUpdatesSheet` |
| `textBoxReleaseNotes` | | done | `CheckForUpdatesSheet`, read only |
| `btnOK` | OK | done | `CheckForUpdatesSheet` |

### `ReleaseNotes`

| Control | Text | Status | Where |
|---|---|---|---|
| `textBoxReleaseNotes` | | done | `ReleaseNotesView`, read only |
| `btnOK` | OK | done | `ReleaseNotesView` |

### `InstrumentationForm`

| Control | Text | Status | Where |
|---|---|---|---|
| `textBox1` | | done | `InstrumentationView`, read only |
| `btnClose` | Close | done | `InstrumentationView` |
| `btnReset` | Reset | done | `InstrumentationView` |
| `timer` | | done | The view's task, only while it shows |
