# Original Windows MP3 capture

The actual **v3.2 Windows capture succeeded**, resolving the missing decoder evidence for the six Coldest Winter Vertical Meter differences. Eleven pre-archive stages passed, including the existing Windows Framework compiler and .NET8.0.6 X64 preflight. The original Vixen3.12.4 / AudioPlayer3.12.1371.4 / NAudio2.2.1 reader and cache produced identical complete PCM. Returned archive: `target/windows-results/attempt-05-returned.zip` (34,100,215 bytes / 63 entries; SHA-256 `b963da14ab946d15c2a6e32034ee617e34ad433c8988e4027f672786859b4ea3`). Its intentional pre-archive checkpoint does not prove final supervisor exit.

The reader negotiated signed PCM16, stereo44,100Hz. The 6,555,504-frame cached float buffer has SHA-256 `e5fd049950f041a9cd88d267d91a097a65ffb2b184a3a05f1573a6327e14a5a5`. With that unchanged buffer the full FSEQ comparison has zero nonrandom differences, and the C#/Rust oracle has 273,949 identical mono values plus 104 exact levels/indices. Evidence: `target/windows-results/attempt-05-independent-review.json`, `target/windows-pcm/comparison-v3-2/{report.json,comparison.log}` and `target/windows-pcm/vertical-meter-capture-v3-2/result.json`. See the [capture audit](../../docs/review/windows-v3-2-capture.md) and [meter review](../../docs/review/windows-pcm-meter-oracle.md).

Production Mac decoding remains unchanged and retains the six residual values. Approximate PCM16 truncation matches 99.7781% of Windows samples, so no blanket quantization is applied. Random effects, original Windows app/file interoperability and GDI raster comparison remain separate. Earlier failed/partial runs and the successful v3.1 inventory/copy result remain historical evidence, not current pending capture requests.

For the copy-and-run folder, begin with [START-HERE.txt](START-HERE.txt).
The v3.2 runner adds a separate exact-binary .NET8 capture contract after metadata
and IL audit of the returned DLLs. It compiles a C#5/common-BCL host with the
already-installed Windows Framework compiler, then launches it using the existing
.NET8 runtime, pinned to that major version. Exact hash validation and an API
preflight precede two original-reader decodes. Each stage has an independent
supervisor deadline and errors still lead to final integrity/archive stages.
No SDK, runtime, original Vixen DLL or replacement decoder is shipped or installed.
Actual v3.2 Windows compilation, preflight and capture are now verified by the returned run above; no further inventory-only run is required for this input/build.

The original strict host and current-upstream source contract below are unchanged.
The legacy host has its own `legacy-net8/contract.json`; it does not claim that
Vixen3.12.4 was built from the current net10 upstream tree. See
[the binary compatibility audit](../../docs/review/windows-v3-1-compatibility.md).

The executable has no NuGet dependencies. It loads a user-supplied, matching
Windows Vixen installation's `AudioPlayer.dll` and invokes only:

1. `Common.AudioPlayer.FileReader.AudioFileReader`: MP3 selects the original
   `NAudio.Wave.MediaFoundationReader`. Reflection reads its private
   `_readerStream.WaveFormat`, then its public float `Read` method consumes the
   complete stream and computes a hash.
2. `Common.AudioPlayer.SampleProvider.CachedSampleSource`: the unchanged original
   constructor opens its own `AudioFileReader` and fills `AudioData`. The tool
   requires its complete float count/hash to equal the separate format probe.

The original `SampleChannel` performs PCM-to-float conversion. No replacement
decoder, rounding, channel mixing, trimming, seeking, resampling, playback,
output device, controller, Vixen application startup, or profile is involved.
The input is read twice and hashed before/after; all written files belong to a
fresh output directory outside the installation. UNC/network drives and Windows
reparse points are rejected; use ordinary local copies.
The host installs an empty in-process NLog configuration before constructing the
reader, so it cannot inherit application log targets that write a profile.

## Required Windows environment for the unchanged strict host

- Windows with working Media Foundation, using the actual runtime/decoder whose
  behavior is being measured. Record the historical machine separately if trying
  to reproduce the older FSEQ; a new Windows result does not establish what that
  older machine decoded.
- .NET 10 runtime matching the process architecture. The matching upstream source
  targets `net10.0-windows`; its SDK declaration is 10.0.103 with latest-feature
  roll-forward. An SDK is needed only to build, not to run the copied result.
- A user-supplied build/install matching the reference Vixen source. Supply the
  folder containing `AudioPlayer.dll`, `NAudio.Core.dll`, `NAudio.Wasapi.dll`,
  `NAudio.WinMM.dll`, `NAudio.Vorbis.dll`, `NVorbis.dll`, and `NLog.dll`, plus any
  other dependencies from that same install. Core/Wasapi/WinMM must report
  assembly version **2.3.0.0**. No package or missing assembly is downloaded.
- A local, read-only copy of `Coldest Winter Pentatonix.mp3`, SHA-256
  `c1724f9916c61c623b9a19abbc80a7b161a0df79c8c504e427b8ba576289ada3`.
  The runner accepts other MP3s for capture, but the supplied comparison wrapper
  deliberately accepts only this exact diagnostic input.

`reference.json` embeds hashes of the unchanged reader/cache sources and pinned
dependency declarations. This identifies the audited source contract; it does
**not** prove that an arbitrary supplied `AudioPlayer.dll` was built from it.
The capture records the actual binary hashes so that mismatch can be assessed.

## Build without downloads

On a machine already containing the .NET 10 SDK, from the repository root:

```powershell
dotnet build tools/windows-pcm/WindowsPcmCapture.csproj -c Release --configfile tools/windows-pcm/nuget.config -p:UseSharedCompilation=false
```

The empty package-source configuration prevents package downloads. There are no
package references or Windows-targeting-pack requirements. Build outputs stay
under `target/windows-pcm`; original C# source remains untouched.

The existing private Mac SDK can build the same portable managed host:

```sh
DOTNET_ROOT="$PWD/target/golden/dotnet-root" \
DOTNET_CLI_HOME="$PWD/target/golden/cli-home" \
DOTNET_CLI_TELEMETRY_OPTOUT=1 MSBuildEnableWorkloadResolver=false \
  target/golden/dotnet-root/dotnet build \
  tools/windows-pcm/WindowsPcmCapture.csproj -c Release \
  --configfile tools/windows-pcm/nuget.config -p:UseSharedCompilation=false
```

Copy these three resulting files to a new ordinary local folder on Windows
(not into the Vixen install):

- `target/windows-pcm/bin/WindowsPcmCapture.dll`
- `target/windows-pcm/bin/WindowsPcmCapture.deps.json`
- `target/windows-pcm/bin/WindowsPcmCapture.runtimeconfig.json`

The source contract is embedded in the DLL. Retain this README and source with
the copied tool for review. No Mac native binary is being transferred.

## Actual Windows capture

In PowerShell, substitute the existing local tool/install/input paths. The
output parent must already exist, and `capture-01` must **not** exist:

```powershell
$env:DOTNET_CLI_TELEMETRY_OPTOUT = '1'
dotnet C:\VixenPcmTool\WindowsPcmCapture.dll --help
dotnet C:\VixenPcmTool\WindowsPcmCapture.dll `
  --vixen-bin 'C:\VixenReference\Release\Output' `
  --input 'C:\VixenPcmInputs\Coldest Winter Pentatonix.mp3' `
  --out 'C:\VixenPcmResults\capture-01' `
  > 'C:\VixenPcmResults\capture-01.log' 2>&1
if ($LASTEXITCODE -ne 0) { throw 'Capture failed; retain the log and inspect the error.' }
```

Success produces:

- `cached.f32le`: full interleaved IEEE float32 little-endian samples from
  `CachedSampleSource.AudioData`, starting at its original index zero.
- `manifest.json`: input/PCM hashes and counts, negotiated source bit depth and
  format, cached format, separate-reader equality proof, OS/.NET/architecture,
  source contract, loaded managed assembly hashes, and hashes/versions of loaded
  Media Foundation, MP3-decoder and CoreCLR modules.

The manifest is written last. A failed run can leave a PCM file without a
manifest; that is an incomplete capture and must not be compared. Use a new
output directory for every attempt. Keep the log with the two returned files.
The manifest contains supplied absolute paths; review it before sharing outside
the project. The original cache constructor loads the entire file into memory,
just as Vixen does; this tool is intended for the bounded known MP3 above.

## Compare the captured PCM on the Mac

Bring the completed capture back unchanged. Use a private copy of the existing
show, including the unchanged Coldest Winter `.tim`, MP3 and FSEQ. The wrapper
checks their published hashes, captured PCM hash/counts and the expected stereo
44,100 Hz float format, including equality with the separate format-probe decode,
before invoking the existing diagnostic example. The comparison report records
SystemConfig/ModuleStore hashes and rejects a rendering-context change during
the run. The diagnostic fails explicitly if its original audio cannot load;
it cannot silently ignore the raw-PCM override.

```sh
source env.sh
CARGO_TARGET_DIR=target/vixen-session cargo build --offline -p vixen-session \
  --example compare_fseq --jobs 3
python3 tools/windows-pcm/compare_capture.py \
  --capture '/absolute/path/capture-01' \
  --profile '/absolute/path/private-show-copy' --validate-only
python3 tools/windows-pcm/compare_capture.py \
  --capture '/absolute/path/capture-01' \
  --profile '/absolute/path/private-show-copy' \
  --compare-program target/vixen-session/debug/examples/compare_fseq \
  --out target/windows-pcm/comparison-01
```

The result directory must be new. This runs the existing diagnostic-only
`AUDIO_RAW_F32` override, with no quantization or time shift and a **10 ms** render
interval; the original FSEQ remains at **25 ms**. Do not remove 528 frames from
this Windows capture: that is a port decoder adjustment, not evidence about the
Windows reader. Examine `comparison.log`'s nonrandom differences and retained
`report.json`; successful process exit alone does not mean the FSEQ matches.
Capturing a different frame count or a decoder-origin offset is itself evidence
to investigate, not permission to silently align samples until they match.

## Checks actually performed locally

The Release host builds on the existing Mac .NET SDK with zero downloads.
`check_local.py` passes 13 checks: help, invalid/duplicate arguments, existing-output refusal,
non-Windows capture refusal without writes, and synthetic capture-integrity
validation including changed bytes, frame counts, rate and the separate reader's
count/hash. These fixtures do
not exercise or simulate a successful Windows decode.

```sh
python3 tools/windows-pcm/check_local.py \
  --dotnet target/golden/dotnet-root/dotnet \
  --capture-dll target/windows-pcm/bin/WindowsPcmCapture.dll
```

Retained logs: `target/windows-pcm-build.log` and
`target/windows-pcm-local-check.json`. The comparison example also builds, and an
actual invocation on a fresh private profile with deliberately missing audio
exits nonzero with the required refusal before rendering
(`target/windows-pcm-example-build.log`, `target/windows-pcm-missing-audio.log`).
Those local checks are preparation evidence; the actual v3.2 Windows capture and captured-input FSEQ comparison now pass as recorded above. No SDK/VM installation, network service,
output device or original-profile write was used for this preparation.

## Build the offline copy-and-run pack

`gdi-probe/GdiAvailability.csproj` links the existing unchanged
`tools/golden/media/GdiProbe.cs`, using already-cached drawing assemblies from
`target/golden/media/bin`. It tests graphics/font initialization only, separately
from the Windows PowerShell/.NET Framework graphics check and separately from
original-effect raster comparisons. It starts no Vixen runtime.

The private Mac SDK has no Windows runtime packs. An explicit offline
`publish -r win-x64 --self-contained true` fails with `NU1100` for the missing
Windows .NET/ASP.NET runtime 10.0.12 packages (`target/windows-pcm-publish-probe.log`).
Consequently the unchanged strict host is framework-dependent and requires an existing .NET10 host for its PCM and GDI probes. The separate exact-binary v3.2 host uses the already installed .NET8 runtime and has now captured this audited installation successfully. A self-contained or older Vixen
installation may work normally without supplying that host. Discovery records
that limitation and existing audio DLL versions rather than launching Vixen or
substituting another decoder.

Build both managed projects offline with the existing SDK and empty NuGet
configuration, then package a read-only copy of the exact MP3:

```sh
python3 tools/windows-pcm/package_windows.py \
  --input 'target/progress-ui/timeline-bench-show/Media/Coldest Winter Pentatonix.mp3' \
  --out target/windows-capture-pack
python3 tools/windows-pcm/check_package.py \
  target/windows-capture-pack/Vixen-Windows-Capture.zip
```

The pack manifest hashes every other entry; the packager confirms the input hash
before and after copying. The checker validates archive paths, complete hash
coverage, managed PE/CLI assemblies, runtime contracts, copied source identity
and static script safeguards. These are local integrity checks, **not** a claim
that PowerShell or the Windows decoders have executed. Existing 13 capture-host
argument/integrity checks are rerun after rebuilding the host. No original Vixen
application binaries, show files, SDK or runtime are included in the pack.

The drawing DLLs come from the existing cached `System.Drawing.Common` and
`Microsoft.Win32.SystemEvents` 10.0.10 packages. Packaging chooses SystemEvents'
`runtimes/win/lib/net10.0` implementation, replacing the portable fallback that
the Mac oracle build copied. That deliberate pack-only selection is pinned by
hash in the validator and recorded in `provenance.json`. The cached MIT license,
third-party notices and package metadata are included under `licenses/`.

## Visible-progress runner v2

The first user run reported a long silent wait. Source inspection found no
progress messages between install selection and completion, an in-process
.NET Framework GDI probe with no timeout, and an unbounded parameterless
`WaitForExit()` after timeout termination. Those are concrete observability and
timeout gaps; they do not identify the user's actual stalled stage without logs.
The returned first-run files show Framework GDI completed and the first runtime
host wrote its listing; they do not identify the exact blocking call. That host
listed .NET6/8 and no .NET10, so PCM cannot run with this unchanged strict host
there. No runtime installation is requested; installed Vixen binary metadata
must guide any later compatible capture tool.
The original capture creates its `capture/` directory only after both decodes
and hash agreement, so that folder's absence alone does not identify the stage.

The v2 script writes console/disk stage messages, `progress.json`, and per-tool
elapsed/PID/exit/timeout records. Installed binary metadata is collected before
any child runs, and `diagnostics.json` is checkpointed at each stage. It uses
`Diagnostics.Process` directly with concurrent byte-copy tasks for stdout/stderr,
avoiding PowerShell's `Start-Process` redirect machinery. A fixed15-second
stdout/stderr self-check tests this route on the actual Windows host first.
Child waiting polls every250ms with a visible heartbeat every5seconds; the
runtime/GDI/PCM deadlines remain20/30/300seconds. Output draining is capped at5s,
and every cleanup process wait is capped at5s; no parameterless `WaitForExit()`
remains. Framework GDI runs in a separate30-second PowerShell child.
Ordinary local filesystem/registry operations remain OS I/O without a
claimed global deadline; each inspected file is named before reading it. The
original PCM capture code and semantics are unchanged. Windows/PowerShell
execution of this revised runner remains unverified locally.


## Isolated-stage runner v3

The returned v2 files show all three launched children completed, drained output,
and exited0. A full diagnostics checkpoint exists; its runtime list is still empty.
Thus a blocked child or output drain is excluded at the last recorded stage, but
later PowerShell serialization, caller processing, or console pause cannot be
identified from those files. The screenshots do not establish text-selection pause.
No speculative root cause is claimed.

The v2 selected path was the manufacturer directory `C:\Program Files\Vixen Lights`.
The actual upstream installer uses `Manufacturer\ProductName`, normally its `Vixen`
child. V3 inspects that layout automatically, saving registry metadata, filenames,
file/assembly versions, runtime configurations, hashes and runtime-host paths before
optional probes. It reads only selected/normal installation roots (depth3,8roots,
128directories,8192entries,256metadata files and64MiB hashed). It excludes links,
network paths and profile/media folders. Ambiguous installations skip PCM.

The bootstrap invokes Windows' existing Framework C# compiler in a separate
PowerShell worker with a60-second limit and durable compiler.log, then launches
a separate supervisor with a720-second independent watchdog. Before the native
supervisor exists, a timeout uses bounded built-in taskkill on only the owned
diagnostic PID tree, with a process-handle fallback and recorded cleanup errors.
The supervisor's660-second budget reserves70seconds for final packaging/cleanup.
Workers run in separate native Windows Jobs, started suspended, assigned before
resumption, with kill-on-job-close. Only the three specified OS log/stdin handles
are inherited. Stdout/stderr go directly to private files; there are no redirected
pipe-copy tasks, PowerShell event callbacks, transcripts, or unbounded exit waits.
Both each worker's work and its final JSON/cleanup are covered by its deadline.
A single background console writer uses a128-line bounded queue; a paused console
cannot block supervisor stage clocks. Durable file progress precedes queuing.
Log files above32MiB terminate their stage. Stage and overall failures preserve
partial reports, and a later independent stage can still run.

Framework GDI is still availability-only. The unchanged strict PCM host runs only
with an existing .NET10 host and compatible2.3 audio assemblies. V3 initially produced diagnostic evidence on the known .NET6/8 machine. The later v3.2 exact-binary host captured its original reader/cache successfully, without a runtime install or substitute decoder. Successful inventory alone remains distinct from a completed PCM capture.

The archive stage has its own deadline,32-directory/512-file/512MiB limits and
no-reparse traversal. It writes a `.partial` ZIP and only promotes it after success.
If it fails, the full original results folder is the copy-back artifact. Its JSON
contains the pre-archive checkpoint; final package status is retained alongside
it in the results folder. Bootstrap/compiler failure prints that folder first.
Local tests deliberately fail/hang subprocesses, generate substantial output,
leave descendants and malformed/partial files, and exhaust budgets. They do not
execute Windows Jobs, PowerShell5.1, the Framework compiler or Media Foundation.


After inventory, a separate30-second reference-copy stage preserves only the
unique audio directory's exact named DLL set: AudioPlayer, legacyNAudio,
NAudio.Core/Wasapi/WinMM/Vorbis, NVorbis, NLog, Libraries and Preferences. It never
loads those assemblies. Each source uses read-only/FileShareRead access, has a
16MiB per-file/64MiB total cap, and requires before/copy/after SHA256 equality
before promoting its partial file. The manifest survives interrupted copies.
No Vixen executable, show/profile or user configuration is copied. These private
binaries enable offline compatibility audit of the actual3.12.4 install without
another metadata roundtrip. They do not relax the existing strict capture guard.


Timeouts cover child execution and worker postprocessing, not an absolute guarantee
against a blocked operating system/filesystem. Bootstrap preflight reads are
capped at1MiB per source; location checks happen before writes. Its console can
still be paused by terminal selection, but the independently running supervisor
keeps stage deadlines and file progress. Pressing Escape clears a selection.

The retained user evidence is `target/windows-results/attempt-01` and
`target/windows-results/attempt-02` (read-only copies). Attempt02 records Windows
build22631, PowerShell5.1.22621.4391, registry Vixen3.12.4, and successful runner,
FrameworkGDI and runtime-listing children. Its selected manufacturer root lacked
the application/audio files; no PCM capture is claimed. This evidence motivates
the bounded nested-install inventory and independent supervisor, without proving
the earlier stall's exact cause.

The final v3 shared C#5 supervisor passed48 local fault checks in
`target/windows-v3-faults-reviewed/result.json`. These include actual subprocess
success/nonzero exits, partial-report hangs, simultaneous multi-megabyte output,
output flooding, descendant cleanup, missing executables, malformed/BOM/oversize
reports, exhausted overall budgets, later-stage recovery, checkpoint write
failure, ZIP-stage failure/folder preservation, quoting, and a blocked console
writer. That blocked-writer case finishes its stage in0.05seconds while preserving
file progress. Tests use a separate local process adapter; Windows Job Objects,
PowerShell workers and the built-in Windows compiler remain unexecuted here.
Independent static reviews covered native handle ownership and PS5.1 worker flow.

The original strict capture host was rebuilt offline after excluding supervisor
and fault-test sources from its project. Its DLL stayed byte-identical:
`8fd33da731baf66d9d8a788dc18cf177a5f6793392b166bb3847217e0b328ab7`.
All13 existing local host/integrity checks pass again; see
`target/windows-pcm-v3-rebuild-check.json` and `target/windows-pcm-v3-local-check.json`.


## v3.1 checkpoint compatibility patch

[Returned v3 evidence and diagnosis](review-attempt-03.md) records the actual
Windows failure and retained artifacts. This patch supplies an explicit private
backup path to PowerShell File.Replace, skips dependent work when inventory is
unavailable, distinguishes unknown runtime state from no verified.NET10 host,
and continues independent diagnostics after optional PCM preflight errors.
Per-entry failures preserve later inventory/copy opportunities; partial copies
count toward the total byte cap. No capture-reader semantics changed.

The final shared supervisor suite passes79 checks in
`target/windows-v3-1-faults-reviewed/result.json`. Added cases cover16 skip-reason
combinations, failed/malformed/empty-object inventory, valid empty host lists, and
a vanished PCM directory with later input/archive stages still completing.
`target/windows-worker-v3-1-compat-reviewed/result.json` adds11 static production
PowerShell checks and6 actual explicit-backup filesystem probes. No PowerShell
runtime exists locally, so these are not Windows/PowerShell5.1 execution claims.
The returned v3.1 run subsequently verified the corrected inventory/copy workflow; the v3.2 run then verified exact original-reader PCM capture. Both returned archives remain retained.
