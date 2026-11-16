# Coldest Winter Vertical Meter investigation — November 11, 2026

**Update:** The real Windows v3.2 capture now verifies the remaining input boundary:
273,949 original/Rust mono samples and all 104 levels match exactly; the full
captured-input comparison has zero nonrandom differences. See the
[actual Windows PCM oracle](windows-pcm-meter-oracle.md) for pinned evidence and
the remaining production-decoder limitation. The investigation below retains
the preceding controlled experiments.

The six production real-show differences are one pixel's RGB output during one 30 ms meter
decision. The same decoded stereo input produces **273,949 identical mono
samples through the original NAudio provider chain**, followed by exactly
identical C# and Rust filter/ballistics results at 104 diagnostic times. Converting the decoded
stereo samples to 16-bit PCM removes all six differences without introducing
other nonrandom differences in this sequence. This rules out a difference in the
current mixing/filter/ballistics calculations on this input. PCM precision is
sufficient to explain the archived mismatch. Actual Windows PCM now confirms
the identical-input result. Production decoding remains unchanged.

## Exact mismatch

The source profile renders effects at **10 ms**; its FSEQ samples output at
**25 ms**, with 5,946 frames and 3,367 channels. Changing the render interval to
the export interval is an invalid comparison and creates unrelated differences.

All indices below are zero-based. Output channels 2119–2121 are **R Pixel-148
Red/Green/Blue**, element `5aa5caf6-a06e-4645-a88c-408acd04e5c7`, under Light Strip R.

| FSEQ frame | Sequence time | Port RGB | Stored FSEQ RGB |
|---|---:|---|---|
| 4694 | 117,350 ms | 35, 16, 21 | 0, 0, 0 |
| 4695 | 117,375 ms | 38, 17, 22 | 0, 0, 0 |

Active effects are Vertical Meter `f234e165-a96c-4ff1-8187-f549bc990c00`
(start 116,807 ms, duration 3,106 ms) and Pulse
`7d105733-bcbf-44b6-9ee3-764e4b2618b0` (same start, duration 2,067 ms).
The meter is inverted, with range 13 dB, attack 50 ms, decay 1,500 ms, gain 0,
Normalize=false, LowPass=false, HighPass=true at 500 Hz, and DepthOfEffect=0.

The pixel is index 147 of 563 leaves. Its on threshold is
`(147 / 563 - 1) * 13 = -9.605683836589698 dB`. Both FSEQ frames sample the meter
decision made at relative 540 ms (sequence time 117,347 ms), lasting until the
next decision at relative 570 ms. The production level at 540 ms is
`-9.605631000694732 dB`, only **0.000052835895 dB above** that threshold.
The level at 510 ms is -9.525812406590 dB and at 570 ms is -9.470709794436 dB.

## Audio and input boundaries

- Media is `Coldest Winter Pentatonix.mp3`, 44,100 Hz stereo. The production
  Symphonia decode has 6,555,504 frames after the existing 528-frame decoder-delay
  trim, with no concealed corrupt frames. Gapless trimming is disabled.
- The upstream effect read starts at interleaved sample
  `int(44100 * 116.807 * 2) = 10,302,377`, an odd index. Consequently its first
  mono pair is a right sample followed by the next frame's left sample. The port
  preserves this behavior; rounding the start to a stereo-frame boundary would
  change the signal.
- `GetMonoSamples` is requested to produce 273,949 mono values, despite that
  count originally being calculated with the stereo channel multiplier. The
  port retains that upstream over-read. Each pair is mixed in `f32` with weights
  0.5, then widened to `f64` for filtering. Sample index 23,814 is used at 540 ms.
- The isolated C# oracle receives the exact decoded stereo buffer. The original
  `GetMonoSamples`, cached/mono providers and pinned NAudio.Core 2.3.0 DLL produce
  273,949 values identical to Rust's mono buffer, including the odd start index.
  The original `AudioUtilities.cs` and `AudioFilters.cs` are compiled unchanged.
  Only the decoded-data host is replaced; no decoder or device is constructed.
  All 104 times from 0 through
  3,090 ms in 30 ms steps have identical sample indices and identical floating
  values in Rust and C#. (The meter's render loop uses only through 3,060 ms.)

## Controlled decoder/precision experiments

Each complete comparison uses the profile's 10 ms render interval and the
unchanged 25 ms FSEQ. Only the diagnostic example's audio input is replaced.
Twinkle, Dissolve and Candle Flicker differences remain excluded by the existing
comparison rule; these results do not make random renders byte-identical.

| Input | Level at relative 540 ms, dB | Nonrandom differing values | Largest difference |
|---|---:|---:|---:|
| Production Symphonia float | -9.605631000695 | 6 | 38 |
| Actual returned Windows PCM (v3.2 capture) | -9.606047108390 | 0 | 0 |
| Symphonia rounded to signed 16-bit, then divided by 32768 | -9.605745201348 | 0 | 0 |
| Symphonia truncated to signed 16-bit, then divided by 32768 | -9.606035492034 | 0 | 0 |
| FFmpeg float, gapless trimming disabled and first 528 frames removed | -9.605655582312 | 99 | 187 |
| Symphonia rounded to 8-bit PCM | not sampled separately | 530 | 194 |

Rounding/truncation is applied per stereo sample **before** mono mixing. Samples
are clipped to the corresponding signed PCM range. Both 16-bit variants passing
means this export alone cannot distinguish their rounding rules. The alternate
float decoder also introduces differences elsewhere, so substituting FFmpeg is
not a demonstrated fix.

## Original decode chain and remaining evidence

Vixen's `AudioFileReader.CreateReaderStream` uses the one-argument
`MediaFoundationReader` for MP3; `SampleChannel` then supplies the float samples
stored by `CachedSampleSource`. `upstream/Vixen/Directory.Packages.props` pins
NAudio.Core and NAudio.Wasapi 2.3.0.

The cached NAudio.Core 2.3.0 package identifies source commit
`c89fee940ee6f8d7374d18714a6b85d8b7a18ab0`. At that exact commit,
[MediaFoundationReader](https://raw.githubusercontent.com/naudio/NAudio/c89fee940ee6f8d7374d18714a6b85d8b7a18ab0/NAudio.Wasapi/MediaFoundationReader.cs)
defaults `RequestFloatOutput` to false and requests integer PCM. It specifies
channel count and rate, **but not BitsPerSample**; it reads the negotiated bit
depth afterward. Microsoft's [MP3 decoder documentation](https://learn.microsoft.com/en-us/windows/win32/medfound/windows-media-mp3-decoder)
lists 8-bit and 16-bit integer PCM plus floating-point output.

The returned Vixen 3.12.4 installation uses NAudio 2.2.1 and has now supplied the
original negotiated 16-bit format and full cached PCM. Both meter implementations
pass on that exact buffer, as documented in the linked oracle report. Truncating
Symphonia float values matches 99.7781% of Windows samples, so rounding behavior
alone is not an exact replacement for the decoder. No production quantization
change is made. The historical FSEQ may also have been produced with a different
Windows decoder/source version; the file itself does not record that detail.

The standalone [Windows PCM capture tool](../../tools/windows-pcm/README.md) is
now prepared. It invokes the supplied original AudioFileReader and
CachedSampleSource, records the negotiated format, and requires their separate
complete float decodes to have identical sample counts and hashes. Its manifest
records actual input, assembly and runtime identities. The return-comparison
wrapper validates the exact inputs and PCM, preserves the original time origin,
and records the private profile's SystemConfig/ModuleStore hashes. The diagnostic
comparison now refuses a raw-PCM override if the original clip cannot load,
instead of silently comparing without that audio.

The initial strict host built locally with no downloads and 13 argument/integrity checks passed
(`target/windows-pcm-build.log`, `target/windows-pcm-local-check.json`). A real
diagnostic process verified the missing-audio refusal
(`target/windows-pcm-missing-audio.log`). Those were preparation evidence. The
subsequent [v3.2 capture audit](windows-v3-2-capture.md) and
[captured-PCM oracle](windows-pcm-meter-oracle.md) now contain actual Windows
execution and parity evidence.

## Reproduction and retained evidence

- `crates/vixen-session/examples/vertical_meter_probe.rs`: exact differences,
  output-channel names, target indices, levels, mono PCM and sample-time CSV.
- `crates/vixen-session/examples/compare_fseq.rs`: diagnostic-only
  `AUDIO_QUANTIZE=round16|trunc16|round8` and `AUDIO_RAW_F32` overrides.
- `tools/golden/real-audio/README.md`: offline build/run instructions for the
  unchanged C# oracle; `check.py` requires all 104 samples to match exactly.
- `target/vertical-meter/`: `probe.log`, `compare-round16.log`,
  `compare-trunc16.log`, `compare-round8.log`, `compare-ffmpeg.log`,
  `csharp-levels.csv`, `csharp-mono.log`, source/PCM `sha256.json`, and decoded diagnostic inputs.
  Production mismatch output is also in `probe.log`.

The optional FFmpeg input was generated locally with
`ffmpeg -flags2 +skip_manual -i <mp3> -map 0:a:0 -f f32le <output>`, followed by
removing `528 * 2 * 4` bytes. No resampling was requested. The resulting frame
count equals the production clip's. All source-show access was read-only; no
app activation, audio playback, controller output, or paid service was used.

SHA-256 of the principal inputs and unchanged original algorithms:

| Input | SHA-256 |
|---|---|
| Coldest Winter.tim | `0a75560184e31e25e752f6883b18d42e7428be346983012ca009dcd9b85eaad9` |
| Coldest Winter.fseq | `25f66c9343a9177094c598868ce39598ccbab89b824a0b101af2d5d8fc3ea902` |
| Coldest Winter Pentatonix.mp3 | `c1724f9916c61c623b9a19abbc80a7b161a0df79c8c504e427b8ba576289ada3` |
| AudioUtilities.cs | `a46564cc6a7c75dedb95183d33dee8ef083d3847ab179f839c13c1b0d621ea68` |
| AudioFilters.cs | `da04de1d90c6fa4fff06b3b4ff58a5ca061ca6350e0a9373a8b3cdc093fad3ac` |
| Cached NAudio.Core 2.3.0 nupkg | `2940c2dbeabe7736424549227de3d27f10100858be56920ab120e4621b7f320c` |
