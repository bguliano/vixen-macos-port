# Video frame cache and resource lifetime

The two Video cache Options now control real persistent frame images. `vixen-video/src/cache.rs` stores BMP or PNG frames, with a versioned manifest and a settings/source identity. The source identity includes canonical path, size and modification time; the settings include the requested time range, frame rate, dimensions, postprocessing variant and the stored cache type. Existing image encoders also honor stored JPG/GIF/ICO choices. An unrecognized/unavailable extension remains unchanged and fails rendering safely; it is never silently replaced with BMP.

The native cache root is `<temporary-directory>/Vixen/VideoEffect/mac-v1/<profile-hash>`. This retains the original's disposable temporary-cache policy, while isolating profiles. Retention survives a sequence/app reopen, subject to normal OS temporary-directory cleanup. Cache files do not modify source media or the profile's original show files. The cache's native manifest/hash layout is independent of the Windows ffmpeg cache layout.

Production Video rendering streams decoded pictures into a unique staging directory, then atomically publishes the completed cache. Playback rendering reads one requested image at a time, retaining at most two decoded images. This avoids loading a whole long clip into memory. The small standalone Vec decoder API remains bounded; that bound does not apply to production disk-backed cache rendering. Requests have finite-value, frame-count and per-frame allocation guards. Cache data is checked for ownership, image dimensions, regular files and manifest bounds. Corrupted frames invalidate the set so a later render rebuilds it. A decoder retry discards only partial files in its own staging directory.

Each open sequence owns a cache lease. The render context carries that lease and the actual effect instance UUID; background workers cannot extend the owning document's lifetime. Deleting an effect preserves its cache for Undo. At accepted editor close, deleted effects lose their pairings; surviving effects retain them when Clear Effect Cache on Exit is off. With the option on, closing removes the sequence's pairings and collects unreferenced cache folders. Shared active readers and windows pin their folders until their final read ends. Close/cancellation prevents late workers from publishing a new cache. Known effect IDs are tracked even when a reopened effect cannot render, so deletion/clear still removes its older pairing.

The application owns the current cache options, so existing sequence leases read the latest file type on their next render and the latest clear setting on close. Scheduler preparations and web sequence playback transfer the lease out of their temporary prerender session and retain it until the run/context ends. Ad-hoc live effects use a temporary render lease; standalone comparison utilities without a profile use the bounded in-memory API. Shutdown closes remaining profile leases.

Cache Size is calculated from the actual encoded image byte total, stored in the manifest and available in O(1). Applying a completed Video render updates the effect's read-only Cache Size property. When the formatted value changes, the panel's property revision advances so an unchanged selection refreshes its label. This does not mark the document modified, create Undo, or trigger another render.

Verification is local and muted:

- `crates/vixen-video/src/cache/tests.rs` verifies actual image signatures, cold-manager disk reuse, options/source invalidation, legacy stored image formats, delete/Undo timing, unrenderable reopen cleanup, concurrent windows, in-flight disk readers, late workers, corruption/symlink handling, decoder retry staging and a 600-frame 800×600 cache exceeding 1 GiB of total decoded pixels with bounded resident images. All 12 pass: `target/video-cache-tests.log`.
- `crates/vixen-session/src/sequence_video_cache_tests.rs` verifies the actual Options→render→cache path, save/reopen, Delete→Undo→Redo→close, latest clear option, non-editor lease transfer and application shutdown. It also verifies the actual effect-editor panel-model read-only Label against the encoded file byte total. Five tests pass in `target/video-cache-session-tests.log`, including generated MPEG-4 Part 2 AVI through the software decoder into BMP cache files, rendered intents and the Cache Size label model. The fifth regression pins the panel revision change after stale metadata is replaced, clean modified/Undo state, and no revision change for identical metadata.
- These session tests invoke native AVFoundation on a generated movie. They are Rust integration tests; the separate observed AppKit/SwiftUI release runs below verify the displayed Cache Size.
- All six existing Video effect render tests pass in `target/video-cache-effect-tests.log`, covering pixel colors/order, start time, movement, missing media and quality controls.
- Decoder streaming sampling, source-frame sharing, rotation, EOF and cancellation are tested against the existing Vec behavior by `vixen-video/tests/decode.rs`. Decoder fallback and container coverage are tracked separately.

Independent review found no remaining blocker in ownership, publication/close synchronization, deferred cleanup, corruption recovery, retry staging, options propagation or Cache Size updates.

No real display/controller, audio output, original show mutation, external network or paid API was used.

## Observed release application workflow

`scripts/test_video_cache_ui.py` and the `video_cache_fixture` example pass **173 native harness steps across five fresh-process launches** on the audited optimized app. Executable SHA256: `bc24816432051b02f0594148088eeae3837b181f156484d44487792f2f408217`.

The MOV workflow passes **139 steps** in `target/video-cache-ui-release3/run/result.json`:

- Actual Options controls turn clearing off and choose PNG. The selected Video finishes rendering, and its visible Cache Size is **0.07 MB**, matching **70,014 encoded bytes** in 100 PNG frames.
- Closing and reopening in another process reuses the same frames and manifest: every byte hash and modification timestamp remains unchanged.
- Actual Options changes the type to BMP. A normal Maintain Aspect property commit starts another render while the same effect stays selected. The expanded panel visibly shows **0.07 MB before and immediately after the commit**, then **0.55 MB after completion**, matching **572,200 BMP bytes**. All three selection observations remain `selection=1`; the test does not reselect, rebuild the panel, or force a metadata reload. The property edit is saved normally.
- Turning clearing on through the real switch, then accepting the unmodified editor's close, removes every image folder, pairing and staging file; only the owned cache-root marker remains.

The generated MPEG-4 Part 2 AVI workflow passes **34 additional steps** in `target/video-cache-avi-ui-release/result.json`. This is the established AVI rejected by the native AVFoundation probe (source SHA256 `2448144002994bfc07960f53947fc4b6420524a10023e93554ad914a2d5e84c2`). The app's bundled fallback produces the cache and rendered timeline; visible **0.06 MB** matches **62,333 PNG bytes**. The supplied fixture and its private copy retain their original hash. The report records both bundled decoder-helper hashes.

Every final native run asserts `waiting=0`, `failed=0` and the selected effect's `rendered=true`. Both unedited reopen paths stay unmodified with no Undo entry; the intentional property edit becomes modified, then Save clears dirty state. These are offscreen AppKit actions, not physical focus/gesture evidence. SwiftUI omitted this label from its in-process accessibility tree, so the runner retained actual window screenshots and read their text with the separate local CPU Vision helper `scripts/read_ui_snapshot.swift`. The reports identify `snapshot OCR`; no backend value substitutes for the observed label. Options and disclosure arrows receive normal posted UI events. The render-start hook uses the panel's normal property commit.

The complete 173-step cache suite was repeated successfully after the Video
metadata, Fixture/Pattern authoring and Delete/Undo generation fixes, using fresh
profiles and the final audited optimized executable SHA256
`4c98baee4fdbcab6d5571ea9c5fa104f50fd843fdaae1b8f8b7098eba32e42b7`.
Final reports are `target/video-cache-ui-3515/native/result.json` (139 MOV steps)
and `target/video-cache-avi-ui-3515/native/result.json` (34 AVI steps). The encoded
byte totals and observed labels match the earlier checkpoint above. The selected
MOV panel again shows `0.07 MB` before and during the pending render, then
`0.55 MB` on completion without reselecting; reopen preserves frame hashes and
mtimes, and clear-on-close leaves only the ownership marker. The separate
91-step FileName/metadata workflow in `docs/review/video-metadata.md` verifies the
actual duration and Quality refresh lifecycle on this same executable.

The fixture serializes the sequence before copying its generated video, preventing cache prewarming. Its stored Video Length is therefore initially zero; **Video Length lifecycle correctness is a separate audit, not covered or claimed by this cache result**. No original show, audio output, device, foreground activation, external network or paid API was used.
