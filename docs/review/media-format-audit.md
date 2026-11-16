# Advertised media formats — November 11, 2026

This audit checks formats actually offered by upstream and the port, rather
than arbitrary codecs supported by a large FFmpeg installation.

Video's original `VideoPathEditor.cs` offers MP4, AVI, MOV and MTS.
`target/video-format-audit` contains a private H.264 MOV control, an H.264
MPEG-TS remux, and a MPEG-4 Part2 AVI. All three failed AVFoundation decoding in
the restricted terminal. With media-service access, MOV and MTS decode; the
valid AVI still reports `Cannot Decode`. FFmpeg decodes its first scaled frame.
That is a real omitted advertised path, not evidence that hardware decode is
unavailable. Video fallback implementation/evidence is documented separately
in [video-decode-fallback.md](video-decode-fallback.md).

`AudioOutputManager.GetSupportedFileExtensions` and
`sequencer/commands.rs::AUDIO_EXTENSIONS` both list 32 extensions. Before fixes,
32 generated fixtures (31 extensions plus an ADPCM WAV variant; no MP1 fixture)
produce 22 successful primary decodes and 10 failures. Failures are WMA2/ASF
(`wma`, `asf`, `wm`, `wmv`), PCM AVI, AAC MPEG-TS (`m2ts`), AC3, EAC3 (`ec3`),
MP2 and IMA ADPCM WAV. Every generated fixture decodes through the local FFmpeg
reference. Ogg/Vorbis succeeds after using its available native encoder to
generate the fixture. Artifacts and generation recipe are under
`target/audio-format-audit`; this is functional decode evidence, not Windows
Media Foundation sample-for-sample equivalence.

The cached Symphonia MPEG bundle now enables MP1/MP2 alongside MP3. The MP2
regression verifies ten 1152-sample frames, 440 Hz content and no MP3-only 528-frame
trimming. MP1 is enabled but lacks a generated fixture. The uncached ADPCM crate
was not added; the shared fallback handles that existing advertised WAV path.

The installed Homebrew FFmpeg 9.0.2 closure is 21 Mach-O files / 39,731,472 bytes, but
its executables and several dependencies require macOS 27.0. It cannot satisfy
the port's 26.0 target. `target/ffmpeg-bundle-audit/inventory.json` records exact
paths, hashes, sizes, dependency closure and load commands; installed formula,
receipt, SPDX metadata and license are retained there. That installed build is
GPL-3.0-or-later and is not shipped.

`scripts/build_media_tools.sh` instead verifies the official unmodified source:

- Source: `https://ffmpeg.org/releases/ffmpeg-9.0.2.tar.xz`
- SHA-256: `8c3850283eb25fa026482078a04051e0be17347b09ef81a0849bec15a96e002e`
- Target: arm64/macOS 26.0; static FFmpeg libraries and system frameworks/libraries
  only. External codec dependencies, networking, GPL, nonfree and version3
  components are disabled. The resulting tools report LGPL-2.1-or-later.
- Built-in decoders/parsers, advertised input containers, raw RGBA/float PCM
  output, scaling/resampling and rotation filters are enabled. AVFoundation and
  Symphonia remain the preferred paths, including existing successful MP3 timing.

The source archive, configure arguments, source link and signed executable
hashes are retained in `target/media-tools/provenance`. The app bundles both
tools under `Contents/Helpers`, provenance under `Resources/MediaTools`, and the
upstream notice/LGPL text in `Resources/Licenses/ffmpeg.txt`. Ordinary builds are
offline; `scripts/build_media_tools.sh --fetch` explicitly retrieves the pinned
archive once. No system installation or Homebrew runtime path is required.

## Completed fallback checkpoint

The final private runtime decodes all 32 audit cases (`results-final.tsv`),
including the ten previously rejected cases. `cargo test -p vixen-audio`
passes 32 tests, with the existing device-enumeration test ignored, in
`target/audio-format-audit/fallback-full.log`. The seven permanent fallback
fixtures also include a mono 22.05 kHz WMA, preserving source channels/rate;
the expanded advertised-format rerun passes in `fallback-advertised-final.log`.
Fixture generation recipes are in `crates/vixen-audio/tests/assets/README.md`.

Audio keeps successful Symphonia decoding, including the MP3-only delay trim.
A failed primary decoder uses the first audio stream and bounded float PCM
output from the shared process runner. Metadata is limited to 1 MiB/15 seconds;
PCM to 2 GiB/300 seconds. The runner owns all pipes/children, permits only local
file/pipe protocols, and handles cancellation by killing/reaping its private
process group. `AudioClip::load_cancellable` also checks primary decode packets;
the audio regressions prove pre-cancellation and cancellation after a live
FFmpeg process produces its first non-silent PCM chunk. The latter loops the
tiny WMA through the public runner, cancels from its real output callback, and
returns `Cancelled` in 0.02 seconds (`live-decode-cancel.log`). It needs no
large fixture, timing sleep or test-only production hook. Shared runner tests
separately assert owned-process death/reaping. The new test passes in addition
to the 32-test full-package checkpoint above.

Existing Web PlaySequence and Export Wizard tokens now flow through
`SequenceSession::open_cancellable` into audio decoding. Cancellation before
XML/media work and after synchronous migration is checked, media cancellation
propagates, and neither caller shows an ordinary load-failure prompt for it.
The export regression cancels from its `Loading` callback after the outer
loop check: a valid v0 sequence's reachable WMA is not migrated or decoded,
no render/export task follows, no output is published, and the original files
remain byte-identical (`export-cancel-callback.log`). XML parsing and an
already-started legacy media copy remain synchronous between checks. These
are backend workflow checks, not a claim of a physical window Cancel gesture.

The signed executables total 18,570,768 bytes and require only system libraries:

| Tool | Bytes | SHA-256 |
|---|---:|---|
| ffmpeg | 9,365,264 | `670ea76791d83ed9165dfd3740b143edddb9582e8ad3956b89f201b1236b8046` |
| ffprobe | 9,205,504 | `37792b4070bc36680437d2b7e7f44d9c268b27e0ebad93fd5f9c3f427ea88687` |

Both load commands declare macOS 26.0. Staging signs a new sibling file then
renames it, so an executing prior tool's inode is never overwritten. The
bundle auditor requires both executable helpers, checks their signed hashes
against bundled provenance, and audits their load commands with other Mach-O
files. Final app build/relocation and actual editor waveform verification are
separate integration checks; this generated-format matrix is not a claim that
every codec variant or Windows Media Foundation's exact samples match.
