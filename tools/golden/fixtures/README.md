# Fixture effect oracle

This project compiles the nine fixture effects, their fixture bases, fixture models,
and Intelligent Fixture property directly from `upstream/Vixen/src`. These C# files
are not rewritten or copied. The supporting `Vixen.Core`, effect base, curve,
Pulse, and property assemblies are the same existing native .NET assemblies used
by the main golden runner. Their paths and hashes, plus all linked C# sources,
are recorded in `source-manifest.sha256`.

`UiStubs.cs` replaces only the unused Windows fixture editor/image entry points
(the methods throw if called) and supplies an in-memory fixture library. Target
assignment uses the original effect's "created outside the module loader" path:
assign targets before assigning its descriptor. No effect has required property
dependencies. Fixture properties are data-contract round-tripped before assigning
them to either implementation, so property setter ordering matches a loaded show.
The oracle creates a fresh private native profile and no controllers.

Run offline from the repository root:

```sh
tools/golden/fixtures/build.sh run
source env.sh
FIXTURE_GOLDEN="$PWD/target/golden/fixtures/data/fixtures.json.gz" \
  cargo test -p vixen-effect-registry --test fixture_golden -- --nocapture
```

After inspecting a regenerated comparison, copy the gzip file to
`tests/golden/fixtures.json.gz` to make it the ordinary test fixture. `FIXTURE_CASE`
selects case labels by substring; `FIXTURE_MAX` limits printed differences. All
.NET output/cache/profile writes stay under `target/golden`.

The checked-in matrix has 1,069 cases and 408,500 evaluated state values, comparing
both edited instances and serialized/reloaded instances. It covers all nine
fixture effects, 1–6 heads, multiple and duplicate targets, shared leaves, mixed
capabilities, heterogeneous profiles, authored whitespace, zero/short/non-frame
aligned durations, four frame intervals, range and index curves, reversed index
ranges, Line Dance modes/directions/centering/inversion/override boundaries, and
generic Fixture range/index/color-wheel/RGB rows. Intent types and exact time spans
are compared too. Numeric values use absolute tolerance 1e-10; byte values and
metadata match exactly, except C# default null strings map to Rust empty strings.

There are 34 deliberately malformed cases (68 edited/reloaded render attempts)
where unchanged C# reports an insufficient-point curve error. Their empty or
partial output is compared; the generator rejects any other logged exception.
Other cases must render with no logged warnings/errors. Source failure status is
recorded, while Rust returns the matching output without throwing.

Limits: this is a synthetic render/model matrix, not fixture hardware output,
Windows UI, timeline image drawing, every possible malformed profile, library
reference refresh, or a full cross-platform native-base audit. Gobo image drawing
and native fixture editor entry points are deliberately unavailable. RGB rows use
solid red and the main oracle's existing Pulse implementation; these cases do not
claim exhaustive Pulse verification.
