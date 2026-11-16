# LipSync property-render oracle

This project compiles the original upstream LipSync effect, Face property/models,
mark models, and phoneme definitions unchanged. The supporting native .NET base
assemblies are the same existing read-only assemblies used by the other local
oracles. `source-manifest.sha256` records the linked source/assembly hashes.

This oracle exercises **FaceDefinition mode only**. The image-map library and
FastPicture entry points throw if called. Thumbnail resource getters return null
placeholders used only by static dictionary initialization; no timeline drawing
is invoked. Face setup's Windows dialog also throws. Original matrix-render
algorithms are not replaced by a simulated result, and no matrix parity is claimed.

```sh
tools/golden/lipsync/build.sh run
source env.sh
LIPSYNC_GOLDEN="$PWD/target/golden/lipsync/data/lipsync.json.gz" \
  cargo test -p vixen-effect-registry --test lipsync_golden -- --nocapture
```

A verified regeneration is copied to `tests/golden/lipsync.json.gz` for the
ordinary test. `LIPSYNC_CASE` filters case labels and `LIPSYNC_MAX` limits printed
differences. Build/cache/profile writes stay under `target/golden`; the native
profile is newly generated with no controllers.

The checked-in matrix compares 684 cases, 193,828 evaluated state values, plus
exact intent types/time spans. It covers all 11 phonemes, three eye modes, outline
on/off and combined face-component branches; group/leaf/empty/duplicate/shared
targets; RGB and single-color leaves; case/whitespace phoneme keys; marks with
leading/trailing/rest gaps including the strict >10ms boundary, overlap,
out-of-effect times and minimum-duration handling; all mark collection types,
missing IDs/fallback; ten durations and four frame intervals. Every original
render must succeed without logged warnings/errors.

Each case compares explicit model assignment after target/mark setup and actual
file reload order (targets, ModuleData, marks). The first draft incorrectly put
marks before loaded data and normalized explicit assigned data in Rust; its
missing-ID differences were harness errors, corrected before this artifact.

Limits: image-map/FastPicture rendering, map migration, bitmap loading/scaling,
Windows UI and timeline drawing, real audio/hardware, and the existing known-color
identity deviation are outside this matrix. Face color parsing is independently
covered by the 146 original contracts in `../states`.
