# Windows v3.1 returned installation audit

The returned `results-20261111-195257-bf7b18ff.zip` is retained under `target/windows-results/attempt-04`. Archive SHA-256: `8741a09e9920869227bee6667cb069152a97fe737534284aa9ad5858419e304d`. Its six pre-package stages all completed, without timeout or reported errors. The ZIP is readable and contains the expected pre-archive checkpoint, so its internal `package: pending` does not mean packaging stalled. The final archive-stage result remains outside the ZIP.

The installation is Vixen 3.12.4 (`AudioPlayer` assembly 3.12.1371.4, product commit `41c3f4f7e3ea13e8b21648a7398753fb60ad9a5a`), targets .NET 8, and ships NAudio 2.2.1.0. Its verified host lists .NET 6.0.29 and 8.0.6; it does not list .NET 10. Therefore the existing strict current-source .NET 10 / NAudio 2.3 capture correctly skipped. The registry inventory is not evidence that this historical build equals the current upstream source. The product commit is absent from the local shallow upstream checkout.

Nine private audio/dependency DLL copies match their before/copy/after hashes; all nine remain unchanged after the local audit. Actual PCM was **not** captured by v3.1. The Framework GDI result proves drawing availability only, not a Vixen raster oracle.

## Exact binary API and IL evidence

`tools/windows-pcm/audit-metadata` is a local SDK10 helper using `PEReader`/`MetadataReader`. It does not load or execute supplied assemblies. Source is named `.cs.in` to prevent accidental inclusion in the existing strict capture project. Reports are in `target/windows-results/attempt-04-audit`.

- `AudioPlayer.json`: `AudioFileReader(string)` has `_readerStream`, `StreamLength`, `WaveFormat`, and `int Read(float[], int, int)`. Its MP3 branch directly creates `NAudio.Wave.MediaFoundationReader(string)`. The constructor creates `SampleChannel(_readerStream, false)`, preserving channel count. The read path delegates to that channel under its original lock.
- `CachedSampleSource(string)` opens the original reader, allocates a one-second read buffer, reads to exhaustion, and stores the complete `List<float>.ToArray()` as `AudioData`. It exposes `WaveFormat` and has no cache constructor parameters that alter audio.
- `NAudio.Wasapi.json` and `NAudio.Settings.json`: the original `MediaFoundationReader` starts Media Foundation, selects the audio stream, requests its default PCM subtype, and exposes the negotiated current format. Capture must observe that format; it must not infer a 16-bit result merely from these instructions.
- `NAudio.Core.json`: `SampleChannel` uses the original sample-provider conversion, metering, and volume provider. Its `false` argument does not force mono to stereo. The audited 16-bit converter divides signed samples by 32768, but this does not prove the eventual negotiated bit depth.
- `modules-summary.json`: all nine supplied assemblies have no `<Module>` methods/initializers. The reader type's static initializer obtains its NLog logger; the capture sets an empty in-process logging configuration before constructing it.

`AudioPlayer`, `Libraries`, and `Preferences` are x64 PE assemblies; the remaining six are managed I386/AnyCPU. The arm64 Mac cannot load the x64 AudioPlayer. A local reflection preflight was attempted after the metadata audit and failed explicitly with that architecture mismatch before reader construction. It is a platform limit, not a successful .NET8/Windows preflight. No original reader, cache, decoder, playback device, or Vixen application was constructed locally.

## Separate .NET8 capture host

`tools/windows-pcm/legacy-net8/LegacyCapture.cs.in` uses C#5 and the ordinary Framework BCL surface. Windows compiles the source with its already-present Framework `csc.exe`; the resulting x64 managed EXE is launched by the verified .NET8 `dotnet.exe` with `LegacyCapture.runtimeconfig.json`. It must not be launched directly under Framework. The runtime configuration requests `Microsoft.NETCore.App` 8.0.0 with `LatestPatch`, so a newer major runtime cannot silently replace .NET8.

The older compiler is a way to produce the small reflection host without a .NET8 SDK/reference-pack download. It does not compile, replace, or retarget Vixen's DLLs. Microsoft's documentation describes Framework-library compatibility in modern .NET, while explicitly requiring runtime testing; therefore this route retains a bounded actual Windows API preflight. [`Framework compatibility`](https://learn.microsoft.com/en-us/dotnet/core/porting/framework-overview), [`Assembly.LoadFrom`](https://learn.microsoft.com/en-us/dotnet/api/system.reflection.assembly.loadfrom?view=net-8.0), [`AssemblyResolve`](https://learn.microsoft.com/en-us/dotnet/api/system.appdomain.assemblyresolve?view=net-8.0).

The host:

1. Accepts only a private folder containing exactly the nine audited files, with their exact sizes, SHA-256 hashes and assembly names. It rejects extra files/directories and reparse paths. It never loads unrelated installed application assemblies.
2. Offers a hash-only contract check and a separate reflection API preflight. The latter does not explicitly invoke the original constructors or type initializers. The actual capture additionally requires Windows, actual `RuntimeInformation.ProcessArchitecture == X64`, and runtime major 8; architecture is reflected to keep the Framework compiler dependency surface small.
3. Accepts only the unchanged diagnostic MP3. It invokes the original reader once to obtain negotiated format and a full float-stream checksum, then the unchanged original cache constructor. Full sample count and checksum must agree exactly. No trimming, seeking, quantization, mixing, resampling, or replacement decoding is added.
4. Sets empty NLog destinations before the reader is constructed. Its directory contains no unrelated installation/config files. The original profile, application startup, output devices, controllers and network are unused.
5. Requires the known diagnostic float32/stereo/44,100-Hz format; rejects nonfinite samples, oversized output, or changing inputs. It creates an exclusive fresh output, verifies the persisted PCM hash, rechecks input and all nine binaries, and writes the success manifest last.
6. Records a separate `referenceBinaryContract` for Vixen 3.12.4. It does not reuse the current upstream `referenceSourceContract` or claim historical FSEQ decoder identity. Runtime, managed modules, Media Foundation/native decoder modules, negotiated format, and no-trim origin remain recorded in schema1 format.

The independent supervisor supplies all time/output/descendant limits and continues final integrity/archive stages on compiler, dependency, preflight, decode, or output failures. No runtime installation or download is performed.

## Local validation and limits

`target/windows-legacy-net8-checks-final-02/result.json` records 24 subprocess checks and six in-host helper checks. These cover real nine-DLL contract matching; each independently corrupted DLL; missing/extra dependencies; malformed/duplicate arguments; output collisions/containment; missing parents; symlinks; non-Windows capture refusal; JSON escaping; byte-exact float serialization; streaming checksum equality; and unsupported format refusal. Original returned DLL hashes remain unchanged. The local helper build uses SDK10 with `LangVersion=5`, no packages, and zero warnings/errors.

`framework-api-review.json` and `host-api-metadata.json` retain the local validation binary's full member-reference inventory. Source calls use stock Framework `mscorlib`/`System` APIs: collections; paths/streams; assembly/type reflection; SHA256/HashAlgorithm; Buffer.BlockCopy; Process/FileVersionInfo; strings/culture; and the existing Win32 exclusive directory call. It avoids modern hex/hash/span/JSON/runtime helper calls. RuntimeInformation is found through reflection. Modern Roslyn emits its own `RefSafetyRulesAttribute` into the local SDK10 validation binary; that binary is **not shipped** and is not evidence of a Framework compiler result.

Actual Windows Framework compilation of this new host, successful .NET8 reflection preflight, and Media Foundation PCM remain pending until the new supervised Windows result is returned. The previous strict .NET10 capture source and binaries remain unchanged. No production macOS decoder behavior has been altered.
