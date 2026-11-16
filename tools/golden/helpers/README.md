# Original Bars rotation and audio mixing helpers

The original golden generator reconstructed WPF point rotation and NAudio stereo
mixing. Those calculations now come from their original implementations:

- **WPF v10.0.0:** the complete original `RotateTransform.cs` and verbatim methods
  extracted from Matrix, MatrixUtil, Transform and GeneralTransform. This includes
  the original identity multiplication shortcut, degree reduction, rotation matrix
  and point multiplication order. `extras/WpfShim.cs` only hosts the property
  system and point/matrix fields. It supplies no arithmetic. Source is the MIT
  Microsoft `dotnet/wpf` release matching Vixen's .NET 10 target family.
- **NAudio.Core 2.3.0:** the actual netstandard2.0 assembly from the exact package
  pinned by upstream. The original Vixen `Audio.GetMonoSamples` method and complete
  CachedSoundSampleProvider/MonoSampleProvider classes call that assembly. Only the
  decoded-data container and playback/media hosts are replaced; no audio decoder
  or output device is constructed.

Microsoft source and MIT notices are retained unmodified in `reference/`.
`references.json` records official URLs and SHA256 hashes. The NAudio package is
kept privately under `target/helper-reference`, and `prepare.py` verifies its hash
before extracting the DLL. Its metadata identifies source commit
`c89fee940ee6f8d7374d18714a6b85d8b7a18ab0`; the assembly, rather than a separately
chosen tag's reimplementation, is executed. NAudio has no package dependencies.
`upstream-manifest.sha256` records the unchanged Vixen source and version pins.
Nothing here enters the production app or changes its runtime dependencies.

For a fresh cache, explicitly fetch the free public package once:

```sh
python3 tools/golden/helpers/fetch_reference.py
```

Normal builds use only the local cache and source files. Reproduce the checkpoint:

```sh
tools/golden/build.sh run --only Bars,VerticalMeter,VUMeter,Waveform \
  --out target/golden/helpers/regenerated
python3 tools/golden/helpers/compare.py
source env.sh
cargo test -p vixen-effect-registry --test golden --offline -- \
  bars vertical_meter vu_meter waveform --nocapture
```

November 11 result: every regenerated JSON file is **byte-identical** to the
checked-in golden data; no Rust rendering or fixture correction was necessary.

| Effect | Cases | Frame values compared |
|---|---:|---:|
| Bars | 1,335 | 13,524,212 |
| Vertical Meter | 342 | 1,222,828 |
| VU Meter | 288 | 973,432 |
| Waveform | 346 | 1,234,108 |
| Total | 2,311 | 16,954,580 |

All four Rust comparisons pass with zero mismatches. The totals include 22
original failure cases, which compare the original partial/empty output rather
than treating an exception as success. Detailed logs are
`target/helper-reference/regenerate.log`, `rust-golden.log` and
`target/golden/helpers/comparison.json`.

Limits: the WPF property/animation/native rendering systems are not instantiated;
Bars only needs an immutable point transform. This runs pinned managed source on
.NET 10/macOS, not a Windows runtime. Audio begins with synthetic decoded floats;
Media Foundation decoding, device playback and the six real-show Vertical Meter
threshold differences are separate scopes. These results remove the prior
memory-based rotation/stereo-helper limitation; they do not claim GDI or decoder
parity.
