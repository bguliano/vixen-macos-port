# State render and Face color oracle

This project compiles the complete State effect (including its planner, discovery,
mark parser and segment coalescer), State property/model and actual mark classes
unchanged from `upstream/Vixen/src`. It also compiles the original Face data model
for color/dictionary contract checks. Supporting native .NET assemblies are the
same local read-only assemblies as the main golden runner. Source and assembly
hashes are in `source-manifest.sha256`.

The unused Windows State property dialog APIs are throwing stubs. The normal
native runtime creates only a fresh private profile and no controllers. Target
assignment runs before attaching the descriptor, using the effect's standard
outside-module-loader path. The serialized state property is round-tripped before
both implementations receive it.

```sh
tools/golden/states/build.sh run
source env.sh
STATE_GOLDEN="$PWD/target/golden/states/data/states.json.gz" \
  cargo test -p vixen-effect-registry --test state_golden -- --nocapture
```

Copy a verified generated file to `tests/golden/states.json.gz` to update the
ordinary test fixture. `STATE_CASE` filters case labels; `STATE_MAX` limits printed
differences. All build/cache/profile writes stay in `target/golden`.

The checked-in file has 1,022 State render cases, both assigned model data and
serialized/reloaded model data, with 167,280 evaluated state values. It covers
State Item, marks and custom sources; Default and Iterate; grouped/individual
custom rows; duplicate item names and targets; shared leaves; group and leaf
assignments; missing/out-of-scope references; RGB/single-color targets; clipping,
overlaps, adjacent marks, original mark minimum-duration/order behavior; repeated
and blank comma-separated names; cycle offsets and iteration limits; zero and
short durations. Exact intent types/time spans and byte colors are compared;
state values use 1e-10 absolute numeric tolerance. Every oracle render must succeed
without logged warnings/errors.

146 original color contracts cover all non-system KnownColors, RebeccaPurple,
Empty, an unknown named color, and explicit ARGB with alpha. Both State and Face
parsers are compared. Face keys `AI` and ` AI ` remain distinct while lookup is
case-insensitive.

Limits: this is not a LipSync render oracle, Windows editor/timeline drawing,
hardware output, or every State discovery/editor transition. Multiple-discrete
color sets and .NET color identity equality (known name versus identical ARGB)
are not covered by this checkpoint. System-theme colors use the port's existing
Windows-theme table and are not compared to a macOS native palette.


A separate diagnostic probe reproduces the already documented deliberate ARGB
color-equality difference:

```sh
tools/golden/states/build.sh run --discrete-color-probe
STATE_GOLDEN="$PWD/target/golden/states/discrete-probe/states.json.gz" \
  cargo test -p vixen-effect-registry --test state_golden -- --nocapture
```

This comparison intentionally fails on two Named/edited and Named/loaded states:
C# renders red for named Blue on the default multiple-discrete RGB set, because
its ARGB-only blue entry is not equal to the named color. Rust compares ARGB and
renders blue. The ARGB-blue case matches. This probe is not the ordinary test
fixture and does not change the port's recorded value-equality decision.
