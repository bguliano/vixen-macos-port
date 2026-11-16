# Single Export Sequence sheet: native lifecycle verification

The November 11 native check closes the previous single-sheet gaps for an
`Export Failed` alert and playback pause/resume. It uses the actual
`Sequence > Export` menu and sheet controls, separately from the bulk wizard.
No app or Rust library code changed.

Evidence: `target/export-sheet-native-final/native/result.json`, **118 passing
steps in three processes**, all exit 0, on audited release executable SHA-256
`4c98baee4fdbcab6d5571ea9c5fa104f50fd843fdaae1b8f8b7098eba32e42b7`.
Each stage retains its native `steps.json`, `ui/result.json`, accessibility dumps,
screenshots, process log/status and postprocessed `evidence.json`.

| Stage | Native steps | Observed result |
|---|---:|---|
| `01-stop-resume` | 44 | A playing sequence continues while the sheet opens. Actual Start begins export; status is working, settings/Start are disabled and Stop enabled. Two transport observations remain paused at exactly **2201.2226 ms**. Actual Stop produces `Export Canceled`, then playback advances to **3001.3263** and **3701.3489 ms**. The earlier destination bytes remain unchanged and no `.part` remains. |
| `02-failure-resume` | 39 | The supplied destination is below a regular private file, causing `ENOTDIR`. The real alert shows **Export Failed**, **Not a directory (os error 20)** and **OK**. Dismissing OK reveals the idle sheet, its enabled Start control, and `Export Failed` status. Playback is playing and advances across the failure/dismissal observations. The collision file is unchanged; no export/partial file is created. |
| `03-complete-resume` | 35 | Actual Start completes the short sequence export. Status becomes `Export Complete`; playback is playing and advances. Independent Python readback verifies FSEQ 2.0, **600 declared frames**, **50 ms**, **zero patched channels**, the producer header, and both complete zstd blocks (frame starts 0 and 10) decoding to zero bytes, with no trailing or unused payload. No `.part` remains. |

`ExportPause` in `crates/vixen-session/src/export.rs` pauses only previously
playing playback and its `Drop` resumes it on all worker exits.
`export_dialog::run` owns that guard; `ExportSequenceSheet.finished` displays the
returned error through a real `NSAlert` and leaves the sheet idle. The native
observations exercise those existing paths, without assigning transport or
completion state. The short failure/success work can finish between observations;
their results prove resumed playback afterward, while the longer Stop run directly
observes the paused interval.

## Reproduction and limits

`crates/vixen-session/examples/export_sheet_fixture.rs` generates only a fresh
folder under `target`: 32 light elements, one constant Set Level effect, no
controllers, no audio, a long sequence for an observable cancel interval, a
30-second sequence for completion, and the private collision file. The long
duration is below the original signed-millisecond format limit and keeps only 32
constant intents in memory; the test stops its export after two observations.

`scripts/test_export_sheet_ui.py --app target/build-release/Vixen.app --profile
target/NEW/profile --out target/NEW/native` runs the generated profile muted and
offscreen. It uses existing hooks to open/focus the sequencer, invoke its ordinary
transport commands, and read engine/playhead state. Export menu, format picker,
Start, Stop, OK and Close use actual UI actions. The guarded `VIXEN_TEST_FILE`
supplies the destination; this does **not** exercise `NSSavePanel`, physical
focus, drag reordering, hardware outputs, or nonzero exported channel values.
Each child has a 65-second timeout and only its own process group is killed on
timeout.

The live runner compared input hashes after the Stop and failure stages. This
first run did not separately retain its initial digest manifest; the final
report explicitly records `baselineManifest: false` and includes the current
sequence/collision hashes. The reusable runner now writes `baseline.json` before
launch and checks it during `--check-only`. That mode reads the exact retained
steps/results/files without launching the app or rewriting the sentinel, inputs,
or native evidence.

Two excluded setup attempts are retained: `target/export-sheet-native/native`
clicked the Network table's **Start** heading, so no worker began; the runner now
targets the lower Start button. `target/export-sheet-native-retry/native` used
one light and completed the export before the OCR Stop click; its two paused
observations and resumed completion were valid, but it did not satisfy the
cancellation assertion. The final fixture uses 32 lights.

The final three native runs required no retry. Their postprocessing initially
expected CSV in the last stage; test mode resets settings on each launch, so only
the first stage's explicit picker action selects CSV. Failure and completion use
the default Falcon format. The supplied final filename ends `.csv`, but its
format is valid FSEQ, as selected by the sheet. The expectation was corrected to
independently parse the actual FSEQ and recheck the retained artifacts; the
application was not relaunched or changed.
