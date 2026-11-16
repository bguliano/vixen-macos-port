# Actual Windows PCM meter oracle — November 11, 2026

The returned v3.2 capture produces **273,949 exactly equal mono samples** through Rust and the unchanged original C#/NAudio provider chain. All **104 filter/ballistics levels and sample indices match exactly**, with maximum difference 0 dB. The separate full FSEQ comparison also has **zero nonrandom differing values** with this captured input. The existing production float decoder remains unchanged and still produces the previously recorded six differing values.

This closes the identical-input meter investigation for this show. It establishes the measured residual in the decoded audio input rather than the mono mix, filtering, ballistics, or sampled nonrandom output. It does not establish identical output for all decoders, formats, random effects, or Windows versions.

## Inputs, original algorithms, and boundaries

The [capture audit](windows-v3-2-capture.md) identifies the exact Vixen 3.12.4 / AudioPlayer 3.12.1371.4 / NAudio 2.2.1 installation and Windows/.NET8 runtime. Its 6,555,504-frame, stereo 44,100 Hz float buffer has SHA-256 `e5fd049950f041a9cd88d267d91a097a65ffb2b184a3a05f1573a6327e14a5a5`. The original reader negotiated signed PCM16; the cached floats are its exact integer samples divided by 32768.

The new diagnostic example `crates/vixen-session/examples/vertical_meter_capture_probe.rs` reads this buffer directly. It does not decode, shift, trim, resample, quantize, or open an audio device. It validates the capture shape, fixture identity, and target effect parameters. Cryptographic verification is performed separately by `target/windows-pcm/vertical-meter-capture-v3-2/verify.py`. The original show was not used for writes; all 14 files of the faithful private profile have identical before/after hashes.

The target remains Vertical Meter `f234e165-a96c-4ff1-8187-f549bc990c00`, start 116,807 ms, duration 3,106 ms. The source's odd interleaved start index 10,302,377 and 273,949-value mono request remain intact. Stereo mixing occurs in float32 before widening to float64. The probe calls the existing Rust `read_effect_audio` and `AudioHelper` implementations.

The unchanged `tools/golden/real-audio` project rebuilds the pinned upstream `GetMonoSamples`, cached/mono providers, `AudioUtilities.cs`, and `AudioFilters.cs` against the already cached NAudio.Core 2.3.0. Its small host supplies the captured buffer rather than constructing a decoder. `SequenceEqual` verifies every mono value before filtering. The unchanged `check.py` compares all 104 times from 0 through 3,090 ms in 30 ms steps, requiring identical sample indices and zero numerical difference.

To assess the 2.2.1/2.3.0 dependency boundary without executing returned DLLs on the Mac, the PE metadata helper compared all 29 methods in `StereoToMonoSampleProvider`, `Pcm16BitToSampleProvider`, `SampleProviderConverters`, `SampleChannel`, and `WaveExtensionMethods`. Their resolved IL, branches, locals, and exception regions match exactly. Evidence is in `target/windows-pcm/naudio-conversion-audit/{2.2.1.json,2.3.0.json,comparison.json}`. This supports these helpers only; it is not a claim that the complete installed release equals current upstream source.

## Measured result

| Measurement | Result |
|---|---:|
| Original C#/Rust mono values | 273,949 identical |
| C#/Rust level and sample-index rows | 104 identical |
| Maximum level difference | 0 dB |
| Level at relative 540 ms | -9.606047108390209 dB |
| Pixel threshold | -9.605683836589698 dB |
| Margin below threshold | 0.0003632718005111002 dB |
| Full FSEQ nonrandom differences with captured PCM | 0 values / 0 frames |

The full FSEQ run uses the original 10 ms render interval and 25 ms export interval: 5,946 frames and 3,367 channels. Its report and log are `target/windows-pcm/comparison-v3-2/{report.json,comparison.log}`. Random-effect differences remain excluded by the existing comparator; the complete output is not byte-identical.

Principal SHA-256 values are recorded below. `target/windows-pcm/vertical-meter-capture-v3-2/result.json` additionally pins source files, programs, capture manifest, logs, helper metadata, and all private profile files.

| Artifact | SHA-256 |
|---|---|
| Rust diagnostic source | `14957e83cf841640b3e6db93c6e525fb272f7a640e7f84e4ef0ef89d316924bf` |
| Rust diagnostic executable | `9517c6a808b114fc6e9598917cd2af9628d80342cdb5d12941b3f86574303136` |
| C# oracle executable | `daf9770a47d915b796d023a69d32d9d105653c7baabedc6bab9f7b39c3c9c378` |
| Windows-captured mono float64 file | `485dffaf20bdc27935519374f021155bfa57cf381262bd3cafa9994ace58febb` |
| Rust levels CSV | `4a1d7c971ebd90ce3ee8c9e0dc587e1f293149f053f7f90ccb218d4881dd6bc4` |
| C# levels CSV | `8bc41a4899cd3ac4f434df0b5bffff09b38a41d7d34a8fb19190cf8a9550bdd1` |

The CSV byte hashes differ because the two programs format numbers differently; the parsed numeric values and index strings match exactly.

## PCM16 conversion assessment

NAudio's [2.2.1 reader](https://raw.githubusercontent.com/naudio/NAudio/v2.2.1/NAudio.Wasapi/MediaFoundationReader.cs) and [pinned 2.3 source](https://raw.githubusercontent.com/naudio/NAudio/c89fee940ee6f8d7374d18714a6b85d8b7a18ab0/NAudio.Wasapi/MediaFoundationReader.cs) both default to integer PCM and specify channel count/rate without selecting a bit depth. They observe the negotiated format afterward. Microsoft's [MP3 decoder documentation](https://learn.microsoft.com/en-us/windows/win32/medfound/windows-media-mp3-decoder) lists both 8-bit and 16-bit integer output, plus float output. Thus the actual PCM16 capture is stronger evidence than assuming a bit depth from the API call.

The complete Windows buffer and existing aligned Symphonia buffer have equal frame counts. Comparison from their unchanged index zero gives:

| Conversion of Symphonia samples | Exact Windows matches / 13,111,008 | Percentage |
|---|---:|---:|
| Unmodified float | 3,498 | 0.0267% |
| Round to PCM16, then divide by 32768 | 6,597,815 | 50.3227% |
| Truncate to PCM16, then divide by 32768 | 13,081,914 | 99.7781% |

All Windows samples lie on the PCM16 grid. Nevertheless, truncation still differs at 29,094 samples. The maximum Windows-versus-rounded-Symphonia difference is one PCM16 step; the largest float difference is approximately 0.0000315383. Full statistics and input hashes are in `target/windows-pcm/windows-vs-symphonia.json`.

**Recommendation:** retain production decoding until a deliberately scoped compatibility policy is selected and validated. Truncation is the closer approximation for this input, but does not reproduce the Windows decoder. Both rounded and truncated variants happen to put this meter below one threshold; that is insufficient evidence for a universal conversion rule. Replacing the decoder with the tested FFmpeg float path also introduces additional nonrandom differences.

An additional November 12 comparison explicitly selected the bundled FFmpeg **fixed-point `mp3` decoder**, which had not been exercised by the earlier default float-decoder experiment. The log confirms stereo 44,100 Hz `s16p` input decoding, converted losslessly to the supported `pcm_f32le` output. Gapless trimming remained disabled, and the established 528 leading frames were removed without fitting an offset or resampling. All 13,111,008 samples were compared with the unchanged Windows capture: 6,587,097 match and 6,523,911 differ, with a maximum difference of two PCM16 steps. Every fixed-decoder sample lies on the PCM16 grid. This rules out that bundled decoder as an exact replacement for this captured input. It does not establish broader decoder equivalence or a new FSEQ result; no production change was made. Evidence: `target/windows-pcm/fixed-decoder-comparison-01/{prepared,result}.json`, `decode.log`, and `target/windows-pcm/compare_fixed_decoder.py`.

A future MP3-only PCM16 compatibility mode could be evaluated as an explicit approximation with multiple stereo/mono, rate, bitrate, clipping, silence, and short-file captures. It should preserve the proven frame origin and format-specific behavior, and must not quantize WAV/float/other media or alter the separately specified F68 raw-byte analysis path. Exact Windows behavior requires its decoded samples or a demonstrably equivalent decoder; mapping a different decoder's floats onto the same integer grid alone is insufficient.

## Reproduction

Build the new diagnostic example using the cached Rust environment, then supply the absolute private-profile path, capture directory, and a fresh output directory. Pass `cached.f32le` and the produced `windows-captured-mono.f64` to the unchanged C# oracle. To use its unchanged checker, copy `windows-captured-levels.csv` to a checker-only folder as `f234e165-a96c-4ff1-8187-f549bc990c00-levels.csv`, beside `csharp-levels.csv`.

The retained exact run is under `target/windows-pcm/vertical-meter-capture-v3-2`; build/run logs are in its parent directory. `python3 target/windows-pcm/vertical-meter-capture-v3-2/verify.py` rechecks the evidence and regenerates the result manifest. No network, playback, controller output, application activation, original-profile write, or production decoder change was needed.


## November 16 native-format discriminator

The bounded Windows diagnostic completed all three stages in 0.71 seconds against the exact installed NAudio 2.2.1 copies and .NET 8.0.6. Both fresh readers returned 13,296 stereo frames. The strict consumer validates the separately approved executable/runtime contract, exact formats, native decoder modules, sequential reads and all PCM bytes. Inputs and installed/private dependency hashes remained unchanged. Evidence: `target/windows-mp3-capture-01/{capture,comparison,interpretation}.json`.

For this short fixture, all 26,592 requested float values exactly equal the float32 result of dividing the captured PCM16 value by 32,767. Division by 32,768 differs at 23,538 values; multiplying by a rounded float reciprocal of 32,767 differs at 2,138. The fixture's low amplitude makes all three tested PCM16 rounding policies agree, so it does not distinguish an independent high precision decoder from a shared integer decode followed by normalization. No production decoder change follows from it.

Testing the newly observed denominator against the already retained full-song Symphonia buffer makes the integer comparison worse: truncation at 32,767 differs at 1,885,952 samples, compared with the existing 29,094 differences at 32,768. Both comparisons retain the exact established frame origin without alignment fitting. `target/windows-mp3-capture-01/full-song-two-scales.json` records this rejected hypothesis.

A separate Apple Core Audio probe works outside the restricted terminal session; the initial restricted-session format error is preserved. Its default gapless frame count differs from Windows. Disabling declared gapless trimming gives the full song's expected raw frame count, but applying the existing Symphonia/Windows 528-frame rule does not yield matching samples. This rejects that particular drop-in invocation, not Core Audio's audio quality or a comparison at independently established equal timestamps. No offset fitting, playback, resampling or production change occurred (`target/coreaudio-mp3-probe-01`, `target/coreaudio-mp3-full-01`).
