# Glediator raw-frame oracle

This project compiles original Glediator source unchanged against the existing
native .NET base assemblies. It creates private generated raw RGB files and a
fresh profile with no controllers; GlediatorDescriptor.ModulePath points only to
that private folder. No graphics/GDI, audio, networking, or real hardware is used.
Source/assembly hashes are in `source-manifest.sha256`.

```sh
tools/golden/media/build.sh run
source env.sh
GLEDIATOR_GOLDEN="$PWD/target/golden/media/data/glediator.json.gz" \
  cargo test -p vixen-effect-registry --test glediator_golden -- --nocapture
```

A verified result is copied to `tests/golden/glediator.json.gz`; the ordinary test
recreates the encoded recordings in a unique temporary directory and removes it.
`GLEDIATOR_CASE` filters labels and `GLEDIATOR_MAX` limits printed differences.

277 cases / 167,392 state values compare assigned and serialized/reloaded models,
exact array intent types/time spans and byte RGB values. Cases cover single pixels,
strings, rectangular/uneven matrices, multiple/duplicate targets, both orientations,
Speed/Iterations with six values, four frame intervals, zero/short/non-aligned
durations, brightness curves, missing/blank files, one-frame files and trailing
bytes. Three malformed recordings expect the original caught DivideByZeroException
and no intents, rather than black arrays that could mask other layers. All other
renders must have no logged warning/error.

Two cases also record a second dirty render of the same original C# instance.
Its `_speed` is not reset: 186 element/time samples change for Iterations and330
for Speed. The port deliberately renders deterministically from the fresh-instance
phase instead, consistent with its pure render/cache architecture. These repeated
C# snapshots are evidence of that documented difference, not part of the exact
parity claim. The Rust test separately asserts its repeated output stays identical.

Limits: no arbitrary malformed curves, negative/out-of-UI numeric settings,
location positioning, or per-instance C# rerender phase parity is claimed here.

## GDI availability probe

`tools/golden/media/build.sh run --gdi-probe` separately tries an 8×8
Bitmap/Graphics line and an Arial Font through the original drawing dependency.
It reports both results and exits **1 if either fails**, or 0 if both succeed.
On this Mac both fail loading `gdiplus.dll`; `target/gdi-probe-exit.json` verifies
the nonzero exit and both diagnostics. A successful probe only establishes these
two APIs are available. Full Windows effect raster comparisons remain required.
