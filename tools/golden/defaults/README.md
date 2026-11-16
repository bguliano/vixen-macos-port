# Original-contract Effect Defaults binary XML comparison

This finite oracle verifies the port's `.vfd` NBFX wire format against the
original managed C# contracts on .NET 10/macOS. It does not start Vixen, a profile
runtime, graphics, audio, controllers or network services. No Windows application
or GDI interoperability result is claimed.

`DefaultsGolden.csproj` compiles the unchanged upstream
`Vixen.Core/Services/EffectDefaults/EffectDefaultsStore.cs` data contracts.
`prepare.py` extracts the original `EffectDefaultsService.ReadBinary` and
`WriteBinary` methods verbatim: `XmlDictionaryReader`/`XmlDictionaryWriter` with
`DataContractSerializer`. The nested PulseData, WeaveData (Pattern), and WaveData
types come from the existing original-source `Module.Effect.GoldenExtras.dll`;
the original curve, gradient and inherited model types come from its cached
support assemblies. No replacement decoder or copied model contract is used.

## Reproduction

Requires only the already cached local .NET SDK/assemblies and project Rust
toolchain. The build uses the existing empty NuGet source configuration and
performs no download. Run from the repository root:

```sh
bash tools/golden/defaults/build.sh
source env.sh
cargo build -p vixen-session --example effect_defaults_oracle --offline --jobs 3
python3 tools/golden/defaults/verify.py \
  --out target/golden/defaults/NEW-run \
  --existing-store 'target/show-editor/Effect Defaults/EffectDefaults.vfd'
```

The output parent must exist and the run directory must be new. The optional
`--existing-store` file is read and hashed before/after; it is never rewritten.
Authoritative current-writer inputs are generated fresh, not copied from that
older file. All build and test writes remain under `target`.

## Coverage and assertions

1. The production Rust defaults store writes Pulse, Pattern and Wave in that
   order. Fixtures use a non-default curve and color, Pattern's separate sizing
   values and ordered gradients, and two ordered Wave rows with different phase
   shifts plus a non-nil excluded mark ID before scrubbing.
2. The original C# reader loads the outer store and all three typed payloads.
   Type/instance identities, names, model names, UTC dates, entry order and
   original payload bytes must match. Its serialized typed XML independently
   confirms the non-default curve/sizing/phase values and scrubbed mark IDs.
3. The C# writer reverses the entries, uses Unicode names, assigns UTC timestamps
   with seven fractional tick digits, and inserts an unknown effect at index 1.
   Its deterministic **66,001-byte opaque payload** crosses the 64 KiB boundary
   and deliberately is not an effect model. The original writer chooses typed
   Guid, DateTime and byte-array records naturally.
4. Rust imports and exports that file. Every entry's metadata, exact UTC ticks,
   list position and payload bytes must survive. Known entries must decode into
   the same normalized stored parameter members as the original Rust fixtures;
   the unknown entry must remain opaque.
5. Rust saves each known model through the normal `save` path. This intentionally
   refreshes its saved timestamp and current effect display name, as Save Default
   does. Entry order and the unknown entry's full metadata/payload remain intact.
   The original C# reader accepts the result, and its reserialized **typed payload
   bytes are identical** across the initial port store, C# rewrite, Rust import
   and Rust parameter resave. Thus successful object loading alone cannot hide
   lost fields or changed nested lists.

The verifier retains untouched generated input stores, intermediate files,
original typed XML, JSON observations, command logs, hashes for every artifact,
and `source-manifest.sha256` containing original sources, actual cached assemblies,
the extracted methods and the exact harness sources. A failed run directory is
retained; use a new path when correcting a harness assertion.

## Recorded result and limits

The final run is `target/golden/defaults/verified-final/result.json`; all checks
pass for **three known models plus one opaque entry**. The optional older
`target/show-editor/Effect Defaults/EffectDefaults.vfd` contains one entry and
also loads through the original outer contract without changing its SHA-256
`f593b0cdc56a9acf6fad6e112c0e914964973f1e0aea93dd9e16a39123decbcd`.
An initial fixture assertion expected two Wave rows after appending to defaults
that already contained two; the corrected fixture uses two explicitly distinct
rows. That was a harness error, not a serializer defect.

No production change was needed. This closes the local “port-written NBFX has
not been read by C#” gap for this outer store and representative nested models.
It does not cover every effect type, arbitrary malformed NBFX, binary curve or
gradient library files, or a real Windows Vixen install opening saved defaults.
