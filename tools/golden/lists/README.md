# Wave and Whirlpool nested-parameter oracle

The shared golden generator compiles Wave and Whirlpool directly from unchanged
upstream source. `../ListCases.cs` adds individually authored child records to the
existing cases, preserving all historical case indices and seeded streams. The
C# adapter edits each original child property in order, then renders both that
instance and the original DataContractSerializer reload. JSON stores ordered
field pairs; Rust replays each through its normal whole-list editor transaction.
This ordering matters: changing a Whirlpool child to Meteor resets ColorMode,
but a subsequent explicit ColorMode setter replaces that value.

Wave's Fractal Ivey path uses the existing GoldenRandom namespace shim: the same
fastrand-compatible seeded stream, no rewritten rendering arithmetic. Each random
case also retains the shared harness's 24-render statistical comparison. Source
hashes, including both random-stream adapters, are in `source-manifest.sha256`.
No original source, real show, graphics API or output hardware is modified.

```sh
tools/golden/build.sh run --only Wave,Whirlpool --out target/golden/list-parameters
source env.sh
GOLDEN_DIR="$PWD/target/golden/list-parameters" \
  cargo test --offline -p vixen-effect-registry --test golden -- \
  --exact wave whirlpool --nocapture
```

The verified artifacts are `tests/golden/Wave.json.gz` and `Whirlpool.json.gz`.
`GOLDEN_CASE=lists/` restricts comparison to the new nested cases.

| Effect | Complete cases | Added list cases | Complete compared values | Added compared values |
|---|---:|---:|---:|---:|
| Wave |307|257|4,548,256|4,244,260|
| Whirlpool |1,425|253|14,923,756|4,734,400|
| Total |1,732|510|19,472,012|8,978,660|

All comparisons pass with zero mismatches. Wave includes 35 cases compared both
statistically and against the same random stream. Whirlpool retains 13 pre-existing
original-render failure cases; their empty/partial output is compared explicitly.
None of the 510 added cases throws. All 50 historical Wave and 1,172 historical
Whirlpool case objects are unchanged after JSON decoding.

Coverage: all six wave types, four directions, three movements, mirror/priming,
phase/snake-length boundaries, five curves, gradient/color-handling variants,
empty/single/multiple lists and overlapping waves, string/location positioning,
uneven/discrete/multiple targets, valid/missing/empty mark IDs and parent-wide
activation. Whirlpool varies each of the four child slots independently across
all mode/direction/rotation/corner/color choices, enabled/reverse/3D toggles,
spacing/thickness/tail/band boundaries, individual gradients and palettes,
list-count normalization and mixed configuration. Child X/Y/Width/Height are
computed by the original grid layout, not supplied as independent authored fields.

The oracle exposed one production lifecycle defect: Wave normalized mark IDs on
every child edit. The original normalizes active selections when WaveType or
UseMarks changes, or when mark collections are assigned; an explicit missing/nil
ID or unrelated color edit is preserved until that event. The narrow fix has a
regression covering global activation and row deletion without false activation.
All 28 fx-pixel-complex tests pass. Logs: `target/golden-lists-final-test.log` and
`target/golden-lists-complex-tests.log`.

Limits: this is a finite matrix on .NET/macOS, not exhaustive input coverage or a
Windows runtime. Production .NET and Rust random streams are different. The
existing documented Whirlpool settled-second-render convention is retained.
