# Windows v3.2 capture — returned and independently verified

The actual Windows run succeeded. It compiled the separate legacy host with the existing Framework compiler, passed its .NET8 API preflight, and captured the original Vixen reader/cache PCM. This closes the pending execution checks for this exact installed audio build. It does not close Windows file interoperability, original GDI raster parity, or all macOS-port verification.

Returned archive: `results-20261111-201537-ebc7b651.zip`, 34,100,215 bytes, 63 entries, SHA-256 `b963da14ab946d15c2a6e32034ee617e34ad433c8988e4027f672786859b4ea3`. It is retained as `target/windows-results/attempt-05-returned.zip` and safely extracted to `target/windows-results/attempt-05`.

Independent evidence: `target/windows-results/attempt-05-independent-review.json`. Its reproducible read-only check is `attempt-05-independent-check.py`; PE/CLI metadata from the returned executable is in `attempt-05-audit/LegacyCapture-metadata.json`. No returned EXE or DLL was executed on the Mac.

## Run and binary provenance

All eleven pre-archive stages completed with exit code 0, no timeout, no output-limit breach, and no reported error. The legacy compile took 0.109 seconds, API preflight 0.098 seconds, and two-pass PCM capture 0.804 seconds. The actual environment reports Microsoft Windows NT 10.0.22631.0, .NET 8.0.6, X64 process and OS.

The returned `LegacyCapture.cs.in`, runtime configuration and nine-DLL contract exactly match the immutable delivered v3.2 package (package SHA `e697ccbc0c39b77c37eb56d5944d3804a9c186885fb8729977c0e52fa340c0a2`). Source SHA remains `a8889a655bfc9df60997b757f9dae55fa3b2b94dc0cefd4eee44a3a37d555603`.

The returned 30,720-byte `LegacyCapture.exe` has SHA `3c47898d352916ce5a02fc606d0178ba75d232b1dd052728eb912bc97526560a`, exactly matching the runtime-loaded host recorded in the capture manifest. Read-only PE inspection confirms AMD64 and only `mscorlib, Version=4.0.0.0` / `System, Version=4.0.0.0` assembly references. The actual manifest also records the .NET8 mscorlib/System facades. Thus the previously untested Framework-compiled-host/.NET8 execution route now has a successful real Windows result. This is not a deterministic rebuild claim: the compiler binary itself was not copied or hashed.

All nine copied dependency sizes/hashes match both their before/copy/after records and the prior audited Vixen 3.12.4 installation. The manifest's loaded private assemblies are AudioPlayer, NAudio.Core, NAudio.Wasapi, NAudio.WinMM, NAudio.Vorbis and NLog; each matches its exact contract. NVorbis, Libraries and Preferences were verified but not loaded during this MP3 path. All other recorded managed assemblies are from the .NET8.0.6 runtime or the private diagnostic host.

Native module provenance includes coreclr.dll, mfplat.dll, mfreadwrite.dll, mfcore.dll, mfsrcsnk.dll and mp3dmod.dll. Full paths, sizes, versions and SHA-256 values are retained in the manifest and independent report. Those native binaries were not copied, so their reported hashes cannot be recomputed locally. `mp3dmod.dll` reports version 10.0.22621.2506; coreclr reports 8.0.624.26715.

## Exact PCM result

| Property | Observed value |
|---|---|
| Input | Unchanged `Coldest Winter Pentatonix.mp3`, 3,279,966 bytes |
| Input SHA-256 | `c1724f9916c61c623b9a19abbc80a7b161a0df79c8c504e427b8ba576289ada3` |
| Original decoder | NAudio 2.2.1.0 `MediaFoundationReader` |
| Negotiated format | Signed PCM, 16 bits, stereo, 44,100 Hz |
| Cached format | IEEE float32, stereo, 44,100 Hz |
| Frames / samples | 6,555,504 / 13,111,008 |
| PCM bytes | 52,444,032 |
| PCM SHA-256 | `e5fd049950f041a9cd88d267d91a097a65ffb2b184a3a05f1573a6327e14a5a5` |

The original standalone reader and original `CachedSampleSource` produced identical complete sample counts and hashes. Input hashing before/after capture also matches the delivered MP3. The independent Mac audit recomputed the returned PCM hash and scanned every float: zero nonfinite values, zero values outside signed 16-bit range, and every value lies exactly on the original 1/32768 conversion grid. No synthetic quantization was applied to this returned data.

The original reader reports `StreamLength` 26,224,127 bytes, while the actual full decoded data corresponds to 26,222,016 bytes of PCM16. This 2,111-byte difference is retained as an observation, not used to trim or pad anything. Both audited read paths consumed their original streams to EOF and agreed exactly. The captured time origin remains `CachedSampleSource.AudioData[0]`.

## Archive and verification scope

The source and retained ZIP hashes agree; every archive CRC and every extracted file hash passed. The ZIP contains the deliberately pre-archive diagnostic checkpoint, so its `package: pending` and progress ending during packaging are expected. The valid returned ZIP proves that usable capture artifacts were packaged and transferred. `package.stage.json` and the final supervisor status live outside the ZIP and were not separately returned; their status is not invented.

The Framework GDI probe again passed availability only. The strict current-source .NET10/NAudio2.3 capture correctly remained skipped. This successful result belongs to Vixen 3.12.4, AudioPlayer 3.12.1371.4, product commit `41c3f4f7e3ea13e8b21648a7398753fb60ad9a5a`, and the measured Windows/.NET8 environment. It does not establish equivalence with the current upstream .NET10 source or prove which historical decoder/runtime generated the existing FSEQ.

The subsequent [actual Windows PCM meter oracle](windows-pcm-meter-oracle.md) passed: 273,949 mono values and all 104 C#/Rust levels/indices are identical. The full FSEQ comparison with the preserved captured buffer and original 10 ms render / 25 ms export intervals has zero nonrandom differences. Random effects remain excluded, and no production macOS decoding policy has been changed.
