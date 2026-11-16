# Independent review: Administration

Reviewer build id: `admin-review`. Areas built: `admin`. Test data: `target/show-admin-review` (copy of `target/show-copy`), throwaway
`HOME` in the scratchpad. Harness runs read back: the Administration window (open, Stop, Start), the menu bar dump, Installed Modules,
About, Instrumentation, Release Notes, Options.

Each finding: what, the C# file:line, the Rust or Swift file:line, and fixed or left (with why). Paths: `VA.cs` is
`upstream/Vixen/src/Vixen.Application/VixenApplication.cs`.

## Findings

1. **Options changed in Settings are lost if the app quits with Settings still open.** Q12 saves the system config when the Settings
   window closes, but quitting doesn't close it through SwiftUI, and `runShutdown` never saves (upstream never saves at exit either, but
   upstream's Options is modal: OK saved it, or Cancel threw it away). Harness: interval 10 → 33 in Settings, then System > Shutdown and
   Exit: `SystemConfig.xml` still held 10 and no `Saving System Config.` was logged. C#: `VA.cs:1063-1072` (OK saves), `VS.cs:115`.
   Swift: `app/Sources/VixenAdmin/AdminModel.swift` `runShutdown`. **Status: see below.**
2. **The recent-list entry for a sequence closed at shutdown can be lost.** `AppModel.editorClosed` calls `AdminModel.sequenceClosed`,
   which starts an unstructured `Task` for `add_recent_sequence`; `runShutdown` doesn't wait for it before `core.shutdown()` and
   `NSApp.reply(true)`. Upstream's `_CloseEditor` adds and saves synchronously before `Stop` (`VA.cs:837-881`, `:1137-1156`,
   `:375-409`). Swift: `AdminModel.swift` `sequenceClosed`, `runShutdown`. **Status: see below.**
3. **Archive wizard's default run fails on a fresh Mac.** User Settings is ticked by default and archives `settings_dir()`
   (`~/Library/Application Support/Vixen`), which nothing on the Mac creates until a setting is saved; listing a missing folder is an
   error, so the third pass fails with `An error occurred during the zip process`. Upstream's folder always exists, because every
   `new XMLProfileSettings()` creates it (`Vixen.Common/Controls/XMLProfileSettings.cs:13-19`), including the one `DataZipForm`'s own
   `PopulateProfileList` makes (`DataZipForm.cs:111-113`). This Mac has neither folder today. Rust: `crates/vixen-session/src/archive.rs`
   `run_archive`. **Status: see below.**
4. **Log routing doesn't follow NLog's `final`.** NLog applies `final` only to the levels a rule accepts, so
   `VixenModules.App.WebServer.RequestLoggingMiddleware` at Info (every web request, `RequestLoggingMiddleware.cs:27`) falls through to
   `*` and lands in `Application.Info.log`; Rust dropped it. `NLog.config:55-65`. Rust: `crates/vixen-session/src/logging.rs` `route`.
   **Status: see below.**
5. **Fatal lines logged under the wrong logger.** Upstream's crash handler is `VixenApplication.Program` (`Program.cs:9`, `:83-87`; spec
   F116); Rust wrote `Vixen.Application`, and put the exception against the prefix with no separating space. `logging.rs`
   `install_crash_hooks`, `fatal_exit`. **Status: see below.**
6. **System > Logs sorted case-sensitively.** `di.GetFiles()` on NTFS gives names in upper-cased order (`VA.cs:438-450`, spec F39);
   Rust's byte sort puts `nlog-internal.log` after `Scheduler.Info.log`. `logging.rs` `list_log_files`. **Status: see below.**
7. **Development-build warning came before the lock.** Upstream shows it on `Shown`, after the lock check and exit
   (`VA.cs:148-154`, `:475-486`, `:576-585`); Swift showed it between profile resolution and the lock. `AdminModel.swift` `start`.
   **Status: see below.**
8. **Upstream logs a second warning when the update request itself fails.** `GetLatestReleaseAsync` and
   `GetReleaseNotesReleaseAsync` log `GitHub returned an invalid latest Vixen release.` / `...for tag {tag}.` whenever the release is
   null, which includes an HTTP failure or timeout (`Updates/GitHubUpdateService.cs:71-80`, `:91-103`); Rust returned early.
   `crates/vixen-session/src/updates.rs` `latest_release`, `release_notes`. **Status: see below.**
9. **`Saving System Config.` is logged before the duplicate check upstream** (`VS.cs:163-170`), so a skipped save logs both lines;
   Rust logged only the error. `crates/vixen-session/src/boot.rs` `save_system_config`. **Status: see below.**
10. **Open Sequence's log line named only the file.** `Can't find an appropriate editor to open file {filename}` logs the full path
    (`VA.cs:934`); Swift logged the last component. `AdminModel.swift` `open(path:)`. **Status: see below.**
11. **Archive wizard trimmed `.zip` from the field after the folder/replace question.** Upstream trims at step 6, before the step 7
    questions (`DataZipForm.cs:205-208`), so the field already shows the trimmed name when the question appears or is cancelled.
    `ArchiveWizardSheet.swift` `start`. **Status: see below.**
12. **`ProfileSettings` reads never notice another copy's writes.** F120's guard reloads before a put, but a read after another
    instance saved still sees the old file, and the Data Profile dialog then saves its stale items over the other instance's (upstream
    re-reads the file for every instance, `XMLProfileSettings.cs:11-25`). `crates/vixen-io/src/settings.rs` `text`. **Status: see below.**

## Status of findings 1-12 (second reviewer run, 2026-11-09)

Each was re-checked in the code rather than taken from the note above.

| # | Status | Where it is now | Test |
|---|---|---|---|
| 1 | fixed | `AdminModel.swift` `runShutdown` -> `savePendingOptions()` -> `core.saveOptionsIfChanged()`; also before Setup Previews / Display Setup | none (Swift; seen in harness) |
| 2 | fixed | `AdminModel.swift` `sequenceClosed` chains `recentSave`; `runShutdown` awaits it | none (Swift) |
| 3 | fixed | `crates/vixen-session/src/archive.rs` `write_archive` creates the settings folder before the third pass | `a_user_settings_folder_nothing_has_made_yet_does_not_fail_the_default_run` |
| 4 | fixed | `crates/vixen-session/src/logging.rs` `route` | `the_rules_route_as_nlog_config_does` |
| 5 | fixed | `logging.rs` fatal logger name and message | covered in `logging.rs` tests |
| 6 | fixed | `logging.rs` `list_log_files` | `listing_ignores_case_as_ntfs_does_and_skips_hidden_files` |
| 7 | fixed | `AdminModel.swift` `start` | none (Swift) |
| 8 | fixed | `crates/vixen-session/src/updates.rs` `latest_release`, `release_notes` | added in this run, see 14 |
| 9 | fixed | `crates/vixen-session/src/boot.rs` `save_system_config` | `a_skipped_system_save_logs_saving_first_and_a_skipped_module_save_does_not`, added in this run (`VS.cs:161-225`) |
| 10 | fixed | `AdminModel.swift` `open(path:)` | none (Swift) |
| 11 | fixed | `ArchiveWizardSheet.swift` `start()` trims after the plain checks and before the question | none (Swift) |
| 12 | fixed | `crates/vixen-io/src/settings.rs` `text` reloads when the file changed | `a_read_sees_what_another_store_saved_since` |

## Findings from the second run

13. **Application data from an older Vixen was saved back at its old version.** `SaveData` always writes `DataFormatVersion` 3
    (`VixenApplicationData.cs:14`, `:112`), and `ReadData` reads filter positions from 2 and the high quality flag from 3
    (`:153-190`). The fidelity residue restored the file's `1` or `2`, so the members just written were skipped by the next load.
    `crates/vixen-io/src/appdata.rs` `application_data_from_xml`, `encode`. **Fixed**, test
    `an_older_file_saves_as_the_current_version_so_its_new_members_are_read_back` (failed before). A file from a newer Vixen keeps
    its version and content (GUARD; upstream would stamp 3 on an emptied file), test `a_newer_file_keeps_its_version_and_content`.
14. **Update check warnings didn't read as NLog writes them, and three cases logged the wrong lines.**
    (a) `Logging.Warn("... for tag {ReleaseTag}.", releaseTag)` renders the tag in quotes (`GitHubUpdateService.cs:104`); Rust
    wrote it bare. (b) `{StatusCode}` is an `HttpStatusCode`, which NLog writes by name, `NotFound` (`:130`); Rust wrote `404`.
    (c) JSON of the wrong shape (a list for `releases/latest`, an object for `releases?per_page=5`) throws in
    `JsonConvert.DeserializeObject<T>` and logs `GitHub update check returned malformed JSON.` (`:137`, `:143-146`); Rust logged
    nothing for it. (d) An empty 200 body deserializes to null without a warning; Rust called it malformed. (e)
    `UnavailableUpdateService` logs nothing (`UnavailableUpdateService.cs:5-9`); Rust's logged the two "invalid release" warnings,
    which is what the UI harness runs. `crates/vixen-session/src/updates.rs` `get_json`, `Shape`, `http_status_name`,
    `latest_release`, `release_notes`; test hook `logging::take_captured`. **Fixed**, tests
    `each_failure_logs_the_originals_warnings`, `a_missing_release_for_notes_names_its_tag_in_quotes`,
    `the_unavailable_service_logs_nothing` (all three fail on the old code). Spec line 643 reworded to match.
    **Left:** Json.NET's leniency inside a release (a number for `tag_name`, a string for `draft`, comments, single quotes) isn't
    copied; `serde_json` is strict and GitHub sends none of these. For 300-303 and 307 the enum has two names and .NET doesn't say
    which prints; `HttpClient` follows those redirects first.

## Harness runs on the real show copy (second run, 2026-11-09)

Build `target/build-admin-review/Vixen.app`, data `target/show-admin-review`, update check pointed at a loopback stand-in (`VIXEN_UPDATE_TEST_BASE=http://127.0.0.1:18473/`). Every picture was read.

| Picture | What it showed | Verdict |
|---|---|---|
| main window | `Release 3.13u4`, `Build #1`, the owner's recent list in order, green light, `Execution: Open`, `CPU: 0%` | as `VA.cs:1074-1098`, F13-F33 |
| menus dump | System: Logs, View Installed Modules, Profiles..., System Configuration, Execution Engine, separator, Shutdown and Exit (`VA.Designer.cs:123`); Help: Online Help, Vixen YouTube Channel, Check for Updates, Release Notes, About Vixen (`VA.cs:190-215`) | matches |
| About | version at 20 pt, `License.txt` text, OK (`AboutVixen.cs:10-22`) | matches |
| Installed Modules | groups sorted, rows in file order, five columns (`InstalledModules.cs:19-43`) | matches |
| Instrumentation | fresh values read `min 999 ms,  avg 0 ms,  max 0 ms,  cnt 1` (`MillisecondsValue.cs:8-10`, `:43`) | matches, KEEP |
| Release Notes | the stand-in answered 404 for tag `3.13u4`, so the bundled file showed; the warn log gained `GitHub update check returned HTTP status NotFound.` and `GitHub returned an invalid Vixen release for tag "3.13u4".` | matches `GitHubUpdateService.cs:104`, `:130` after finding 14 |
| Check for Updates | `Release 3.13u4 Installed`, `An updated release build is available.`, `Release 3.14 is the latest.`, the stand-in's body; the info log gained ` Release version 3.14 available.` with its leading space (`VA.cs:547-548`) | matches |
| Options | Update Interval 56, Clear Effect Cache on Exit on, Latency 25, Cache File Type bmp | matches the files |
| Setup Previews | `Christmas Tree 2019` unticked, `Christmas Tree 2022 (OpenGL)` ticked, both `Vixen Display Preview`; buttons off with nothing selected | matches `ConfigPreviews.cs` |
| Data Profile | with an empty `Settings.xml`: one `Default` row at `~/Documents/Vixen 3` | matches `DataProfileForm.cs` |
| Zip Wizard | all eight boxes ticked, `Default`, `~/Documents`, `VixenProfile` | matches `DataZipForm.cs` |
| Stop then Start | red light and `Execution: Closed`, Start on and Stop off; then green and `Execution: Open` | matches `VA.cs:1078-1098` |
| Shutdown and Exit | log: `Vixen System stopping...`, closing, closed, `Vixen System successfully stopped.`, no save lines (`VixenSystem.Stop(false)`, `VA.cs:403`); `.lock` gone | matches |

The harness's own `quit` step calls `exit` and so leaves `.lock` behind. That is the harness, not the shutdown path: the next launch treats it as a stale lock and removes it, which the first run showed.

## Parity rows marked done, confirmed in the code (second run, 2026-11-09)

Each row was read in the Swift or Rust it names and against the C# the spec cites. None needed correcting.

F2, F12, F16, F17, F21, F26, F28, F30, F33, F35, F37, F38, F39, F44, F45, F46, F47, F58, F59, F60, F61, F62, F67, F74, F77, F86, F87, F89, F97, F103, F105, F107, F108, F109, F113 (35 rows).

Rows read in depth while checking behaviours (first and second runs): F3, F4, F6, F7, F23, F63, F64, F68-F73, F75, F78-F85, F90-F96, F98-F102, F104, F114, F115, F117.

## Further behaviours checked line by line (second run)

| Behaviour | C# | Port | Result |
|---|---|---|---|
| Milliseconds value: min starts 999, count 1, `(int)tot / cnt`, format | `Vixen.Core/Instrumentation/MillisecondsValue.cs` | `crates/vixen-session/src/instrumentation.rs:1-120` | matches |
| Rate value: a read resets the count, 0 when no time passed | `RateValue.cs` | same file | matches |
| Engine rows and their names | `Execution.cs`, `ContextManager.cs` | `crates/vixen-session/src/playback.rs:316-326` | matches; dropped rows are in the parity note |
| Command line: `no_controllers`, `no_execution`, `data_dir=value`, split on `=` | `VA.cs:127-131`, `:587-607` | `crates/vixen-session/src/boot.rs:62-83` | matches |
| System menu order, Help menu order and URLs | `VA.Designer.cs:123`, `:150`, `:171`, `Help.cs` | `app/Sources/VixenAdmin/AdministrationCommands.swift` | matches |
| Check for Updates disabled on a test build | `VA.cs:203-207` | `AdministrationCommands.swift` | matches |
| Logs submenu filled once at startup | `VA.cs:436-450` | `AdminModel.swift:166`, `logging.rs:243-255` | matches |
| Light colours, state names, Start/Stop enabling | `VA.cs:1074-1098`, `Execution.cs:98-111` | `crates/vixen-session/src/execution.rs:1-75` | matches |
| Shutdown saves no config and removes the lock | `VA.cs:403`, `VixenSystem.cs:125-143` | harness run with System > Shutdown and Exit | matches |
| Update notice keeps its leading space in the status bar and the log | `VA.cs:541-549` | log of the harness run | matches |
| Version label colours: yellow dev, red test | `VA.cs:511`, `:519` | `AdministrationView.swift:89-93` | matches |
| CPU readout shows `Resource Usage` until a reading, then `CPU: n%` each second | `VA.cs` stats timer | `CpuReadout.swift`, `cpu.rs` | matches (Q6 FIX as specified) |
| About: image, heading at 20 pt regular, licence text | `AboutVixen.cs:10-22` | `SmallWindows.swift` | matches |
| Installed Modules: groups, columns, Copy Module Id | `InstalledModules.cs:19-89` | `SmallWindows.swift`, `modules.rs` | matches |

### 15. The lock's "checking" and "removing" error lines were never logged — fixed

- What: the spec's log table (line 634) lists three lock errors. Only `An error occurred creating the profile lock file.` was written. A lock that couldn't be read, or whose pid wasn't a number, read as locked with nothing in the log, and a lock that couldn't be deleted (a stale one, the one at shutdown, or another machine's on Open Anyway) failed silently.
- C#: `Vixen.Application/VixenApplication.cs:289` (`RemoveLockFile`), `:350` (`IsProfileLocked`'s outer `catch`, which also sets locked).
- Port: `crates/vixen-io/src/lock.rs` (`check_profile_lock` returns `LockCheck { locked, check_error, remove_error }`, `try_remove_lock_file`; `is_profile_locked` and `remove_lock_file` keep their answers), `crates/vixen-ffi/src/admin.rs:180-200` (`is_profile_locked`, `remove_lock_file` and `clear_foreign_lock` log), `crates/vixen-session/src/boot.rs:319-330` (`release_lock` logs).
- Tests: `lock::tests::a_check_that_fails_says_why_and_a_plain_answer_does_not`, `lock::tests::a_stale_lock_that_cannot_be_deleted_is_still_not_locked_and_says_why`, `boot::tests::a_lock_that_cannot_be_removed_at_shutdown_is_logged` (the last fails on the old code).
- Left: the profile list (`profiles.rs`, which also runs the check for each row) still doesn't log, because `vixen-io` has no logger. The launch path and the Load button go through the FFI wrapper and do.
- Left: a lock file that isn't valid UTF-8 logs the checking error here, where `File.ReadAllText` would decode it with replacement characters and most often find another machine's name. Both read as locked.
- Also: `Process info: {pid}@{machine}` is now written once, the first time the id is wanted, as `GetUniqueProcessId` does (`VA.cs:230-241`): by a check that finds a lock file, or by taking the lock. Before, it was written at every take and never when the profile turned out locked. `crates/vixen-ffi/src/admin.rs` `log_process_info_once`. No test (the FFI crate has none); seen in the harness log.
- Harness, on the show copy with `.lock` holding `xyz@{this machine}`: the alert reads `Profile is already in use or unable to the lock the profile.`, title `Error`, button `OK` (`VA.cs:148-154`), and the log has `Process info: ...` then `An error occurred checking the profile lock file.` The picture of the alert sheet shows only its icon; the text was read from the accessibility dump. With the lock removed, a launch and System > Shutdown and Exit ran clean and left no `.lock`.

## Left, with why (second run)

- Add Profile with a blank name that also matches an existing blank-named profile: upstream shows `Profile name can not be blank.` and then also `A profile with the name  already exists.`; the port returns only the first. Needs a profile already named blank, which the dialog can't make.
- The profile list's lock check (`crates/vixen-io/src/profiles.rs`) doesn't log the lock errors: `vixen-io` has no logger. The launch path and Load do.
- Json.NET's leniency inside a release isn't copied, and the names .NET prints for 300-303 and 307 are ambiguous (finding 14).
- Swift-only fixes 1, 2, 7, 10 and 11 have no automated test. 1 was seen in the harness in the first run; the others were read in the code.
- Instrumentation's engine rows read `cnt 1` under the harness, because nothing drives updates when the app is muted. Not a defect; the dropped rows are in the parity note.
- Setup Previews draws empty alternating rows under its two previews. Cosmetic.
- Not driven by the harness: file panels (Open Sequence, the archive's folder chooser), typing into the Options fields, the Documents permission prompt, a second copy of the app running at once (F120), a crash (F116).

## Tests run at the end (second run)

- `cargo test -p vixen-io -p vixen-ffi -p vixen-session --lib` with `CARGO_TARGET_DIR=target/admin-review-cargo`: vixen-io 251 passed, vixen-ffi has no tests, vixen-session 1297 passed and 5 ignored.
- `cargo test -p vixen-io -p vixen-ffi` (with vixen-io's integration tests: migrations 3, sequences 15, system 10): all passed.
- `tools/golden/filecheck.sh`: PASSED (it doesn't cover `VixenApplicationData.xml`; that file's change is covered by the two `appdata` tests).
- `crates/vixen-session/tests/real_show.rs`: not run, because nothing that renders changed.
- `scripts/build_app.sh debug` with `VIXEN_BUILD_ID=admin-review VIXEN_AREAS=admin`: built, and launched on `target/show-admin-review` four times through the harness.

## Verification

Adversarial pass over fixes 1-15 (verifier, build id `admin-review`, 2026-11-09). Each entry: confirmed, corrected or reverted, with the C# read.

- **Fix 3 (archive, missing User Settings folder): confirmed.** `XMLProfileSettings.cs:13-19` makes `%AppData%/Vixen` in its constructor and `DataZipForm.cs:111-113` constructs one while filling the profile list, so upstream's third pass (`DataZipForm.cs:42-68`) always has a folder to walk. `archive.rs` `write_archive` now makes it before the third pass and ignores a failure to make it, as upstream ignores that pass's result.
- **Fix 4 (log routing, `final` per level): confirmed.** `NLog.config:55-65`: the `RequestLoggingMiddleware` rule is `minlevel="Warn" final="true"`, and NLog's `final` only stops the levels the rule itself enables, so the Info line of `RequestLoggingMiddleware.cs:27` falls through to `*` and lands in `Application.Info.log`. `logging.rs` `route()` takes the first rule that matches the name and the level, which is the same thing. The spec's F113 wording ("first match with `final` wins") is looser than the code; the code is right.
- **Fix 5 (fatal lines under `VixenApplication.Program`): confirmed.** `Program.cs:9` is the class logger and `:83-87` logs both Fatal lines through it; the layout `${message} ${exception:format=ToString,StackTrace}` leaves the space. `logging.rs` `PROGRAM_LOGGER` and `format_line` match.
- **Fix 6 (System > Logs order): confirmed, with one Mac-only part.** `VixenApplication.cs:436-450` adds `di.GetFiles()` in the order NTFS returns them, which is case-insensitive by name. `logging.rs` `list_log_files` sorts on the uppercased name. Skipping names that start with `.` has no upstream counterpart: it is there for `.DS_Store`, which Windows never has. See the DIFFERENCES check below.
- **Fix 8 (second update warning): confirmed.** `GitHubUpdateService.cs:71-80` logs `GitHub returned an invalid latest Vixen release.` when `GetAsync` came back null (it already logged the HTTP warning), and `:91-103` does the same for notes with the tag. The development path logs no second line, and neither does `updates.rs` `latest_development_release`.
- **Fix 9 (`Saving System Config.` before the duplicate check): confirmed.** `VixenSystem.cs:163-170` logs, then tests `_systemConfigSaving`; `:208-216` tests `_moduleConfigSaving` first and logs after. `boot.rs:334-357` has both in that order.
- **Fix 11 (wizard trims `.zip` in the field first): confirmed.** `DataZipForm.cs:205-208` rewrites `textBoxFileName.Text` after the five refusing checks and before the `Directory.Exists` and `File.Exists` questions (`:210-232`). `ArchiveWizardSheet.swift` `start()` sets `fileName = trimmedArchiveFileName(...)` between the refusals and the question, so a cancelled question leaves the trimmed name in the field, as upstream.
- **Fix 9, a note on what was left:** upstream leaves `_systemConfigSaving` set if `Save()` throws (`VixenSystem.cs:191-197`); the port's guard always clears it. That is the spec's FIX (F117, Q19), so it stays.
- **Fix 1 (pending Options at quit): confirmed, as the spec's decision rather than upstream's code.** Upstream's Options is modal and its OK saves (`VixenApplication.cs:1063-1072`), so nothing can be pending at exit. The port's Settings window is not modal and applies as you change (Q12: "save the system config when the Settings window closes, if something changed"). Quitting closes that window without `onDisappear` being sure to run, so `runShutdown` (`AdminModel.swift:299`) calls `savePendingOptions()` before `core.shutdown()`. `boot.rs:461-466` `save_options_if_changed` writes only when `options_changed` is set, so F12's "the system config is never saved at exit" still holds whenever nothing is pending. The same call before Setup Previews and Display Setup (`AdminModel.swift:531`, `:539`) keeps a Cancel there, which reloads from disk, from dropping the Options edits.
- **Fix 2 (recent list at quit): confirmed.** `VixenApplication.cs:837-881` adds the closed sequence and saves synchronously, and the shutdown at `:375-409` closes every editor before `VixenSystem.Stop` and the last `SaveData`. In the port `SequencerModel.finishClosing` (`app/Sources/VixenSequencer/SequencerModel.swift:584-589`) calls `AppModel.editorClosed` before `closeForShutdown` returns, `AppModel.swift:97-103` calls `sequenceClosed` synchronously, and `AdminModel.sequenceClosed` (`:416-428`) chains each save on the one before. So `await recentSave?.value` at `:292` covers every editor closed by the loop above it, and the list is on disk before `core.shutdown()`.
- **Fix 7 (development warning after the lock): confirmed.** The lock test is in the constructor and exits (`VixenApplication.cs:148-154`); the warning is on `Shown` (`:475-486`, `:576-585`). `AdminModel.start` now takes the lock at `:122` and shows the alert at `:130-135`. One thing differs and was already so before the fix: the port waits for OK before loading, where upstream's `Load` carries on behind the alert. No release build shows it.
- **Fix 10 (full path in the log line): confirmed.** `VixenApplication.cs:935` logs `"Can't find an appropriate editor to open file " + filename`, the whole path, and the alert below it uses `Path.GetFileName`. `AdminModel.swift:390-391` has both.
- **Fix 12 (Settings.xml reads reload): confirmed.** Every upstream use constructs its own `XMLProfileSettings`, which loads the file (`XMLProfileSettings.cs:11-25`), so a dialog always opens on what is on disk. The port keeps one store per process; `settings.rs` `current()` now compares the file's modification time before a read as well as before a put (the spec's F120 GUARD names only the put). A deleted file reads as empty, as a fresh upstream instance would.
- **Fix 13 (application data version): confirmed, and the spec corrected to match.** `VixenApplicationData.cs:111` always writes `DataFormatVersionNumber` (3), and `:168`, `:199` skip the later members of an older file, so kept at version 1 the port saved members its own next load ignored. `appdata.rs` `encode` writes `max(version, 3)`, and the residue is taken against the file with its version already raised so it doesn't put the old number back. The newer-file half is a GUARD the spec already recorded ("Rust's residue keeps its content"); the same paragraph still said "Rust writes back the version it read", which the fix made untrue, so `docs/spec/administration.md` (the `VixenApplicationData.xml` paragraph) now says what the code does. `boot.rs:199` still logs `given data version was too high: {n}` for the newer file.
- **Fix 15 (lock log lines): corrected.** The three lines and where they come from are right: `Process info` only inside `GetUniqueProcessId` at first use (`VixenApplication.cs:230-241`), which is reached from `IsProfileLocked` only when a lock file exists (`:303-306`) or from `CreateLockFile` when none does (`:254-256`); the removing error only from `RemoveLockFile`'s catch (`:287-290`); the checking error from the outer catch, which also answers locked (`:348-352`). `admin.rs:180-234` and `boot.rs:319-330` match. What was wrong: `lock.rs` read the file with `read_to_string`, so a lock that isn't valid UTF-8 logged the checking error, where `File.ReadAllText` (`:305`) decodes anything and the method then answers locked without a word. The review had listed that as a deliberate difference. `check_profile_lock` now reads the bytes lossily, so only a file that can't be read at all reaches the catch, as upstream. Test `a_check_that_fails_says_why_and_a_plain_answer_does_not` now covers both, and the row came out of `docs/DIFFERENCES.md`. Still different, and small: `profiles.rs` checks locks for the Select a Profile rows without logging (vixen-io has no logger), so `Process info` can be written a moment later than upstream when one of several profiles has a lock file.
- **Fix 6, DIFFERENCES:** the hidden-file skip wasn't recorded. Added to `docs/DIFFERENCES.md` under Administration.
- **Fix 14 (update warnings as NLog writes them): confirmed.** `GitHubUpdateService.cs:104` passes the tag to a named hole, which NLog renders in double quotes; `:130` passes an `HttpStatusCode`, which NLog renders by name; `:137` is `JsonConvert.DeserializeObject<T>(json)`, which returns null for an empty body and throws a `JsonException` for the wrong shape, caught at `:143-146`; `UnavailableUpdateService.cs:5-9` returns null and logs nothing. `updates.rs` `get_json`, `release_notes` and `latest_release` do each of these, and the tag comparison is ordinal as upstream's (`:102`). The spec's line for these warnings was reworded by the review to show the quotes and the status name, which is right. Not copied, and not reachable from GitHub: a `null` inside the development list throws in upstream's `Where` (`:86-88`) and is skipped here.

### Verification: what was run

Build `admin-review`, areas `admin`, on `target/show-admin-review`, with a throwaway `HOME` and the harness.

- **Launch, then System > Shutdown and Exit.** No `.lock` left. `Application.Info.log` has `Vixen app starting.`, one `Process info: {pid}@{machine}`, `Release 3.13u4 - Build #1`, the system and engine lines, and the four stopping lines. System > Logs lists `Application.Error.log`, `Application.Info.log`, `Application.Warn.log` in that order (fix 6, fix 15).
- **Check for Updates and Release Notes against a loopback server that answers 404** (`VIXEN_UPDATE_TEST_BASE=http://127.0.0.1:18473/`). The server saw `GET /releases/latest` and `GET /releases/tags/3.13u4`, and `Application.Warn.log` got, in order: `GitHub update check returned HTTP status NotFound.`, `GitHub returned an invalid latest Vixen release.`, `GitHub update check returned HTTP status NotFound.`, `GitHub returned an invalid Vixen release for tag "3.13u4".` (fixes 8 and 14).
- **A `.lock` holding the bytes `ff fe ff`.** The alert `Profile is already in use or unable to the lock the profile.` came up, the app left on OK, the file stayed, and nothing went to `Application.Error.log` (the correction to fix 15).
- **Tools > Options..., Clear Effect Cache on Exit switched off, then System > Shutdown and Exit with the Options window still open.** `SystemConfig.xml` differs from before in that one element only, and the log has `Saving System Config.` and `System Config saved.` before `Vixen System stopping...` (fix 1). The same script ending in the harness's own `quit`, which skips shutdown, leaves the file as it was.
- Not run in the app: the archive wizard's default run (fix 3) and its `.zip` trim (fix 11), the development-build alert (fix 7: this build is a release), the unknown file type line (fix 10), and the recent-list save at shutdown (fix 2: this build has no sequencer). Fix 3 has a Rust test. The other four were read only.
- **View Installed Modules, then Profiles... and Cancel** (harness, ending in the harness's own quit). The Installed Modules window opens at 915 wide with the grouped rows and its Close button closes it. The Data Profile sheet opens over the main window and Cancel closes it with no `Profiles Changed` alert (rows F40 and F41 below).

### Verification: tests

- `cargo test -p vixen-io`: 251 unit and 28 integration tests passed, with the lock test changed by the correction to fix 15.
- `cargo test -p vixen-session -p vixen-ffi --lib`: vixen-ffi passed. vixen-session: 1321 passed, 5 ignored, 3 failed on the first run, all three in `scheduler` (`run::tests::a_show_runs_from_startup_to_complete`, `run::tests::the_real_actions_run_under_the_driver_thread`, `tests::the_owners_schedule_loads_and_saves_back_unchanged`). Another worker was editing `crates/vixen-session/src/scheduler/` at that moment. Run again later (`cargo test -p vixen-session --lib scheduler`), all 27 scheduler tests passed. Nothing in Administration touches that code, and nothing was changed there.
- `cargo test -p vixen-session --test real_show`: 3 passed.
- `tools/golden/filecheck.sh`: PASSED. `SystemConfig.xml` byte-identical both ways, `ModuleStore.xml` the same XML.
- App build `VIXEN_BUILD_ID=admin-review VIXEN_AREAS=admin scripts/build_app.sh debug`: built, after the `lock.rs` change.

### Verification: ten more parity rows

Rows the table marks done and neither review run had checked. All ten hold; none corrected.

- **F1 (launch order).** `AdminModel.start` (`AdminModel.swift:96-150`): logging, `Processing Profiles`, the lock, the version line, the development alert, then `VixenCore.start`, whose steps (`boot.rs:220-266`) carry the spec's Mac-column percentages and texts, then `finishStartup` (80, 90, 100 `Ready`). The Info log of a real launch has the lines in the spec's order. Upstream: `VixenApplication.cs:98-216`, `:411-466`, `VixenSystem.cs:36-75`.
- **F5 (leaving Select a Profile).** `SelectProfileSheet.swift`: `Quit Vixen` asks `Vixen cannot continue without a valid profile.` and, after a blank line, `Are you sure you want to exit Vixen?`, title `Vixen`, and leaves on the confirming button. Upstream `VixenApplication.cs:660-675`.
- **F8 (folders made at start).** `paths.rs:13`, `:123-141` makes the root, `Sequence`, `Sequence/auto_backup`, `Media`, `SystemData`, `Module Data Files`, `Program`, `Template/Element`, `Effect Defaults`, and the five module data folders. Checked against each `[DataPath]` (`SystemConfig.cs:23`, `ElementNodeTemplate.cs:8-14`, `Program.cs:9-19`, `MediaService.cs:9-10`, `SequenceService.cs:13-14`, `EffectDefaultsService.cs:35`) and the `[ModuleDataPath]` descriptors (Video, Glediator, Picture, Shapes, VixenPreview). `app/Info.plist:19` has the Documents usage text.
- **F9 (a start that fails).** `admin.rs:451-463` logs `Error during system startup!` under `Vixen.Sys.VixenSystem` (`VixenSystem.cs:58-63`); `AdminModel.start`'s catch shows `An error occurred starting the system and the application will be halted.`, title `Error`, removes the lock and exits (`VixenApplication.cs:421-426`).
- **F10 (development build warning).** Text and title match `VixenApplication.cs:576-585`. Read only: this build is a release.
- **F20 (Open Sequence).** Filters from `modules.rs:161-174` are `All Sequence Types (*.tim;*.vix;)`, `Timed Sequence (*.tim)`, `Vixen 2.x Sequence (*.vix)`, `All files (*.*)`, as `VixenApplication.cs:898-912` builds them; `OpenSequencePanel.swift` allows several files and starts in `<root>/Sequence`, then the last folder used; an unknown type gets `Can't find an editor to open this file type. ("{name}")`, title `Error opening file` (`:935-940`); files opened before startup finishes wait for it.
- **F22 (recent sequences).** `AdminModel.openRecent`: a missing file gets `Can't find selected sequence.`, title `Error`, as `VixenApplication.cs:1115-1136`. The list and File > Open Recent go through the same call.
- **F34 (what is disabled while busy).** `AdministrationView.swift:110-171` and `AdministrationCommands.swift`: the four buttons and the File, System, Tools and Scheduler menus follow `isBusy`; Help follows startup only, which is what the spec's F34 says; the recent list stays live. Upstream `EnableButtons`, `VixenApplication.cs:461-466`, switches the four buttons and the whole menu strip, and is only called around startup (`:413`).
- **F40 (View Installed Modules).** The menu item opens the `Installed Modules` window (`VixenApplication.cs:1187-1196`); seen in the app.
- **F41 (Profiles...).** `AdminModel.profilesEdited` shows `You must re-start Vixen for the changes to take effect.`, title `Profiles Changed`, only when the dialog was saved (`VixenApplication.cs:1238-1251`); Cancel in the app shows nothing.

### Verification: seen and left

- The Window menu lists About Vixen, Installed Modules, Vixen Release Notes and Instrumentation before any of them has been opened (SwiftUI `Window` scenes). Harmless; left.
- F20's alert for a file that throws while opening (`Error trying to open file '{path}'.`) is the sequencer window's to show; this build has no sequencer, so it wasn't checked here.
- A `.lock` that is a directory: upstream's `File.Exists` is false for it and `CreateLockFile` then throws; not tried in the port.
