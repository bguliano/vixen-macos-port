# Video file metadata lifecycle

Changing a Video effect's Filename now refreshes its stored Video Length, VideoSize and Quality through the background render worker. The successful file selection resets Quality to 50 before that generation renders. The completed generation publishes the metadata to the effect editor and future saves. Computed completion advances the panel revision only when values change; it does not add an Undo step, mark the document modified, or queue another render.

The source contract is `upstream/Vixen/src/Vixen.Modules/Effect/Video/Video.cs`: the FileName setter sets `_getNewVideoInfo` (220–230), `ProcessMovie` refreshes on that flag or legacy Quality 0 (603), and `GetVideoInformation` writes Quality 50, duration and VideoSize (1128–1154). Normal ModuleData load (518–526) does not request new information. Start Time remains bounded by its existing 0–10000 control, Quality remains 10–100 and visible only for Locations, and metadata refresh does not change Start Time or Playback Speed. The render continues to enforce the selected duration/start/speed and image sizing.

The original metadata helper (`Vixen.Common/ffmpeg/ffmpeg.cs`, 41–98) parses the container Duration and first Video stream's coded dimensions from FFmpeg stderr. The port uses bounded, cancelable JSON from its bundled ffprobe for those values, with the same local-only protocol restrictions as decoding. It preserves coded dimensions before display rotation or pixel-aspect adjustment. Duration is stored to hundredths with positive half-up rounding, matching the shipped FFmpeg 9.0.2 `libavformat/dump.c` addition of 5000 microseconds before formatting. Rendering still uses the existing AVFoundation-first display metadata and autorotation. This does not assert identical negotiation or metadata rounding in every historical Windows FFmpeg build.

Pending requests are keyed by effect and filename until a matching completed render applies. A later file choice or Undo rejects stale generation results; unresolved earlier filenames remain available for Undo/Redo. A failed probe preserves the prior stored metadata and quality, renders dark and reports the failure. Retrying the same adopted filename re-probes after the chooser copies its bytes, without adding an equal-value property/Undo change. Other effect types keep their existing no-op behavior.

The existing loaded-file behavior is preserved deliberately: a file with stored VideoLength 0 and Quality 50 keeps those stored values on reopen, while the macOS renderer reads actual media information to render it. Upstream can instead render such a malformed/stale loaded effect dark. Selecting its file repairs the stored metadata; loaded legacy Quality 0 also requests the original refresh lifecycle. The cache fixture remains unchanged and is not evidence of an upstream FileName setter having run.

Local, muted verification:

- `target/video-metadata-session-regression.log`: all eleven session Video cache/metadata tests pass. Six new tests use real generated MOV/AVI files and normal panel path commits. They verify 3-second media versus a 2-second effect, coded 160×90 size, reset to 50, unchanged start/speed, read-only label `3`, quality visibility/range, save/reopen retaining a later quality 73, failed-file preservation and retry, same-basename selection without extra Undo/dirty, and actual queued A→B→C results followed by Undo→B→A and Redo→C. Mixed selected Videos A+B both adopt A in one Undo step; Undo restores their separate files/qualities. A pending file change survives Delete→Undo and produces current metadata/intents after the shared snapshot-generation fix.
- The rotated MOV reports stored 160×90 and 1 second while it still renders upright; AVI still produces nonempty intents through bundled software decoding. The old zero-length loaded fixture and legacy Quality 0 refresh are both explicitly covered.
- `target/video-metadata-decoder-regression.log`: all 21 unit tests and 14 native decoder integration tests pass. Controls in `vixen-video/tests/decode.rs` preserve displayed 90×160 versus coded 160×90 for the rotated file. Software parser controls distinguish container duration from stream duration; the rounding boundary test pins 1.005→1.01 and 59.995→60. Existing native output, fallback, cancellation and disk-cache lifecycle checks remain green.

Native AppKit observation now passes on the final optimized bundle:

- `target/video-metadata-ui/native/result.json`: 91 steps across two launches
  (64 file-selection steps, 27 save/reopen steps). The generated profile has 128
  nodes, zero controllers and a 2-second Video effect in Locations mode.
  Audited release executable SHA256:
  `4c98baee4fdbcab6d5571ea9c5fa104f50fd843fdaae1b8f8b7098eba32e42b7`.
- The same selected effect remains selected while the normal file-chooser
  callback commits the private MOV, reselects that MOV, and commits the private
  MPEG4 Part2 AVI. Saved window screenshots/OCR show the actual filenames and
  Video Length `3`, plus a nonzero Cache Size. The native Quality slider reads
  `80 → 50 → 80 → 50 → 50`; the second 80 comes from an actual NSSlider action.
  Undo counts are `0 → 1 → 2 → 2 → 3`, proving equal-filename refresh adds no
  extra Undo entry. No computed values or selection were assigned by the hook.
- Save clears modified state. A fresh process reopens with visible duration `3`,
  native quality 50, no Undo history and unmodified state. Retained cache frames
  and manifest have identical hashes and modification times across reopen.

The test uses guarded normal commit/control actions and actual AppKit snapshots;
it does not claim a physical file-dialog gesture or foreground focus. No original
show, audio output, controller, foreground activation, external network or paid
API is involved.
