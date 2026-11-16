# Snowflakes seeded differential oracle

This project compiles the complete original Snowflakes effect and data model
unchanged from upstream, against the existing read-only native .NET assemblies.
It uses pixel buffers directly and needs no GDI, bitmap/font stub, audio or hardware.
A fresh private native profile has no controllers.

The single controlled substitution is the random stream: reflection sets the
original ThreadSafeRandom thread-local value to the existing `../GoldenRandom.cs`.
That System.Random implementation supplies the same seeded stream as Rust's
`Rand::default()` (one fork). No Snowflakes source is rewritten. This verifies
render/math/state-transition logic for a shared stream, not distributional parity
between production .NET Random and fastrand. Source and assembly hashes are in
`source-manifest.sha256`.

```sh
tools/golden/snowflakes/build.sh run
source env.sh
SNOWFLAKES_GOLDEN="$PWD/target/golden/snowflakes/data/snowflakes.json.gz" \
  cargo test -p vixen-effect-registry --test snowflakes_golden -- --nocapture
```

A verified output is copied to `tests/golden/snowflakes.json.gz` for the ordinary
test. `SNOWFLAKES_CASE` filters labels and `SNOWFLAKES_MAX` limits printed errors.
All build/cache/profile writes stay under `target/golden`.

The matrix has 343 cases / 5,126,164 evaluated state values and compares both
assigned and serialized/reloaded data, exact intent types/spans and RGB bytes.
Coverage includes all ten flake types, five effects, six movements, both string
orientations, four seeds, all color/fade modes, cycle/random brightness toggles,
buildup boundaries, all eleven curves at 0/50/100, grid/count variants, matrix sizes
from 1×1 to 21×23, and frame/duration boundaries. C# random draw counts are recorded.
Every original render must succeed without logged warnings/errors.

Limits: location positioning, production RNG statistics, bitmap thumbnails and
arbitrary malformed/out-of-editor configurations are not covered. The seeded
matrix found no production Snowflakes defect.
