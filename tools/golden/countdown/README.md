# CountDown non-raster differential oracle

The project compiles the complete original CountDown effect and data unchanged,
plus the original FastPixel implementation. GDI+ is unavailable in this C# runtime.
The oracle therefore uses uninitialized instances and reflection to set only the
fields read by `SetFadePositionLevel`, `GetCountDownTime` and
`GetFilmSpinnerRadialLineAngle`. No constructor, font setup, bitmap, full render or
serialization callback is invoked. Unused UI scaling fails loudly if entered.
The numeric setup values are copied from the original `SetupRender`; previous
count starts at zero, as it does on a fresh original instance.

The actual original methods produce all expected text, movement position, fade,
count iteration, previous number, chosen direction, size reset and spinner angle.
The test runs the port's same production methods through the pixel framework,
without drawing, and compares those fields. The existing shared `GoldenRandom`
stream makes the random-direction transitions reproducible; production RNG
statistics are not part of the claim.

```sh
tools/golden/countdown/build.sh run
source env.sh
COUNTDOWN_GOLDEN="$PWD/target/golden/countdown/data/countdown-math.json.gz" \
  cargo test -p fx-pixel-graphics original_countdown_timing -- --nocapture
```

The checked-in `tests/golden/countdown-math.json.gz` runs by default. The matrix has
405 cases / 23,764 frame transitions, covering all count/fade modes, directions,
per-iteration toggles, speeds, Int16 endpoints, minute/hour boundaries, frame
intervals, short/nonaligned durations and an exhaustive millisecond spinner sweep.
It found a one-degree boundary error: C# decimal arithmetic gives 198 degrees for
550 ms, where binary floating point plus ceiling gave 199. The port now uses
exact integer ceiling after the original remaining-millisecond calculation.

This is **not a raster or whole-effect parity test**. Font metrics, GDI brushes,
bitmap sampling, movement geometry during drawing, malformed settings, property
loading and repeated-instance lifecycle remain outside this oracle. Build/cache
writes stay under `target/golden`; no profile, hardware or network is needed.
`source-manifest.sha256` records the unchanged source and read-only dependency hashes.
