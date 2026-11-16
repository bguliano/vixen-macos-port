# Real-show audio isolation

This oracle compiles unchanged upstream `AudioUtilities.cs`, `AudioFilters.cs`,
`GetMonoSamples` and cached/mono sample providers against the pinned NAudio.Core
2.3.0 DLL. It verifies that the decoded stereo input produces the exact Rust
mono buffer, then compares filtering and meter ballistics. It does **not** emulate or execute
Windows Media Foundation, NAudio decoding, or a physical audio device.

The parameters intentionally match Vertical Meter instance
`f234e165-a96c-4ff1-8187-f549bc990c00` in the supplied Coldest Winter sequence.
See [the investigation](../../../docs/review/vertical-meter-audio.md) for results,
source hashes and decoder assumptions. The later
[actual Windows PCM oracle](../../../docs/review/windows-pcm-meter-oracle.md)
now passes all mono values and levels on the returned v3.2 capture; production
decoder differences remain separately recorded.

From the repository root, with the already cached .NET SDK under
`target/golden/dotnet-root`:

```sh
source ./env.sh
export CARGO_TARGET_DIR=target/vixen-session
mkdir -p target/vertical-meter
cargo run --offline -p vixen-session --example vertical_meter_probe -- \
  '/Users/developer/Documents/Vixen 3' target/vertical-meter \
  > target/vertical-meter/probe.log
bash tools/golden/real-audio/build.sh
target/golden/dotnet-root/dotnet target/golden/real-audio/bin/RealAudio.dll \
  target/vertical-meter/symphonia.f32 \
  target/vertical-meter/f234e165-a96c-4ff1-8187-f549bc990c00-mono.f64 \
  > target/vertical-meter/csharp-levels.csv
python3 tools/golden/real-audio/check.py target/vertical-meter
```

The cached helpers from `python3 tools/golden/helpers/prepare.py` are required;
that preparation verifies pinned local dependency hashes and performs no network
access. The source show is only read; diagnostic PCM and build output stay under
`target`.

Compare the complete 5,946-frame sequence with alternate PCM precision:

```sh
cargo build --offline -p vixen-session --example compare_fseq
SHOW_DETAILS=1 AUDIO_QUANTIZE=round16 \
  target/vixen-session/debug/examples/compare_fseq \
  '/Users/developer/Documents/Vixen 3' 10 'Coldest Winter'
```

`AUDIO_QUANTIZE` also accepts `trunc16` or `round8`. Without it the example uses
the production float decode. `AUDIO_RAW_F32` accepts interleaved little-endian
floats already aligned to the loaded clip's rate/channels/timeline. These flags
affect this diagnostic example only; production audio is unchanged.
