# Export Sequence dialog (`ExportDialog`): control parity

Upstream: `Vixen.Modules/Editor/TimedSequenceEditor/ExportDialog.cs`, designer table `docs/reference/forms/Vixen.Modules__Editor__TimedSequenceEditor__ExportDialog.json`.
Spec: `docs/spec/shows-scheduler-export.md` F1 to F13. Mac: `app/Sources/VixenScheduler/ExportSequenceSheet.swift`, a sheet presented through
`SequenceExport.present` (VixenKit), with all logic in `crates/vixen-session/src/export_dialog.rs` behind `ExportDialog` in `crates/vixen-ffi/src/scheduler.rs`.

| Upstream control | Text | Status | Mac |
|---|---|---|---|
| Form | `Export Sequence`, fixed, centred on parent, Accept Start, Cancel Close | done | A sheet on the window passed in, titled `Export Sequence`, fixed 540 pt wide. Return starts (`.defaultAction`), Esc closes (`.cancelAction`), both exercised |
| `groupBox1` | `Settings` | done | `GroupBox("Settings")` |
| `label3`, `outputFormatComboBox` | `Format:`, drop-down list of the seven sorted names | done | `Picker`, saved by index under `AppSettings/ExportDialog/ExportFormat`. FIX first-run default `Falcon Player Sequence 2.6+` |
| `label4`, `resolutionComboBox` | `Timing (ms):`, editable, 25 50 100 | done | `TextField` and a menu of the three. A choice saves its index (`ExportResolution`); typed text is used, never saved (KEEP). 1 to 255 enforced (GUARD) |
| `groupBox2` | `Network` | done | `GroupBox("Network")` |
| `networkListView` and its columns | `Controller`, `Channels`, `Start`, `End`, check boxes, row reorder | done | `Table` with a check box in the Controller column; drag a row to reorder (`draggable` and `dropDestination`). Disabled while exporting (spec's Mac rule). The tick was exercised in Rust tests; **drag reordering wasn't exercised in the app**, only through `move_rows` in Rust |
| `listViewItem1` | designer placeholder `Test` | omitted | Designer sample row, replaced at load upstream |
| `textBox1` | the patching note | done | Static `Text`, unchanged wording |
| `tableLayoutPanel1`, `panel1` | layout only | omitted | Layout containers, no behaviour |
| `chkGenerateControllerInfo` | `Generate Controller File` | done | `Toggle`, saved as `GenerateUniverse` |
| `radio1x`, `radio2x` | `FPP 1.x`, `FPP 2.x` | done | Segmented `Picker` in the same row, saved as `Universe2x`, enabled only with the box ticked and a Falcon writer (KEEP: off for `Falcon Player Sequence`) |
| `chkCompress` | `Enable Compression` | done | `Toggle`, saved as `Compress`. FIX enabled from the format alone |
| `buttonStart` | `Start` | done | Left. Save panel (`NSSavePanel` with the format's extension and All Files), then the run off the main thread |
| `buttonStop` | `Stop` | done | Left, enabled while running |
| `btnUserCancel` | `Close` | done | Right. FIX while exporting asks `An export is in progress. Stop it and close?` with `Stop and Close` and `Keep Exporting` |
| `statusStrip1`, `progressLabel`, `exportProgressBar`, `currentTimeLabel` | status text, bar, `{n}%` | done | Bottom strip: `Saving: {name}`, a `ProgressView`, the percentage in monospaced digits, shown only while working; then `Export Complete` or `Export Canceled` |
| `backgroundWorker1` | unused | omitted | Declared and never started upstream; the run has its own thread |
| `ShowDestinationMB` (`File saved to ...`) | never called | omitted | Dead code upstream |
| Hover on Stop clears the wait cursor | | omitted | A WinForms cursor trick with no Mac meaning |

## What was checked, and how

- Rust: `crates/vixen-session/src/export_dialog.rs` tests (settings keys and defaults, out-of-range indexes, typed timing not saved, radio and compression
  enabling, row ranges and reorder, the warning rule, the save panel's folder and name, a run with its controller file, cancel, failure, drop),
  `crates/vixen-export/src/run.rs` tests (`.part` and rename, cancel keeps the earlier file, failing writer, unwritable folder, throttled progress), and
  `crates/vixen-session/tests/export_dialog_real.rs`, which drives the dialog's own route on the test copy of the show and compares Snoopy Christmas,
  Carol of the Bells and Christmas Can-Can with the owner's real exports in every header field and every frame, and Coldest Winter's `mf` header and frame count.
- The app, with the UI harness (`SchedulerTestHooks`): the dialog opened over the Administration window for a sequence from `target/show-scheduler`,
  Timing set to 25 from its menu, and every format exported with Return: FSEQ 2.0, CSV, ESEQ, Vixen 2 Routine and Vixen 2.1 for Snoopy Christmas, then FSEQ 1.0 and
  Helix. Each file was read back by `verify_exports.py` (scratch) and every frame of every format equals the real FSEQ export's frames. The FSEQ 2.0 file was
  also read by a copy of the owner's the owner's FSEQ player (with Python's `compression.zstd` standing in for `zstandard`), every frame through `FSEQFrame`,
  and its frame digest equals the real file's. Generate Controller File with FPP 2.x wrote `-co-universes.json` and raised the warning with upstream's text;
  FPP 1.x wrote `-universes`; Helix wrote `_Network.xml`. Stop during Christmas Eve Sarajevo showed `Export Canceled` and left the earlier export byte for byte
  as it was, with no `.part` file. Close during a run raised the prompt, and Esc closed the idle sheet.
- November 11: **118 native steps** on the audited release exercise the actual sequencer `Sequence > Export` menu, Start/Stop, a real `Export Failed`
  alert caused by a private path collision, OK/Close, and playback during cancel/failure/success. Two samples stay paused at exactly 2201.2226 ms during
  the long run; Stop resumes playback and preserves the earlier destination. Failure returns the sheet to idle with Start enabled. A normal export has
  a validated 600-frame/50 ms FSEQ header and complete zstd payload. The generated profile has no controllers, so this proves the sheet lifecycle,
  not nonzero exported channel values. Artifacts: `target/export-sheet-native-final/native/result.json`; scope and excluded setup attempts:
  [`docs/review/export-sheet.md`](../review/export-sheet.md).
- **Not exercised in the app**: the real `NSSavePanel` (the guarded test supplies a private destination) and physical drag reordering.
