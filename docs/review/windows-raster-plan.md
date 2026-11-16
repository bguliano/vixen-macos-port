# Windows GDI raster oracle preparation

Status: **74 font-free cases are implemented and packaged for review**, 2026-11-12 UTC: 36 Picture helpers, 20 CountDown helpers and 18 Framework primitives. No Windows raster capture has run, this archive has not been delivered, and no production drawing code has changed. Current review artifact is `target/windows-raster-pack-03/Vixen-Windows-Raster.zip`; its identity and limits are recorded below. Historical 36-case pack01 and intermediate 74-case pack02 remain unchanged.

The proposed host is a C# 5 program compiled on Windows with its already installed Framework compiler and `System.Drawing.dll`, then run directly under .NET Framework. It needs neither the Vixen install directory nor .NET 10. The returned v3.2 results already prove that the Framework compiler can compile the earlier common-BCL host and that Framework `System.Drawing, Version=4.0.0.0` can create a bitmap, draw a line and resolve Arial. They do **not** prove the proposed raster helper compiles or matches Vixen. Evidence: `target/windows-results/attempt-05/{legacy-compile.stage.json,framework-gdi.json}` and `docs/review/windows-v3-2-capture.md`.

## Existing coverage and remaining gaps

| Existing oracle | Evidence already retained | Gap addressed by this proposal |
| --- | --- | --- |
| `tools/golden/picture` | 306 cases / 542,400 exact opaque pixel samples through original movement/sampling code | Bitmap resizing, grayscale and partial alpha are explicitly excluded |
| `tools/golden/countdown` | 405 cases / 23,764 numeric transitions | No GDI rendering, font metrics, brush drawing or bitmap sampling |
| `tools/golden/shapes` | 5,393 cases / 390,833 geometry coordinates | SVG containers only store values; no SVG parsing, rasterization or effect render |
| `tools/golden/media` | 277 Glediator cases / 167,392 state values | Raw RGB playback is independent of GDI; the separate GDI availability probe is not a raster oracle |

The complete Picture/CountDown golden projects target net10 and depend on their original effect assemblies. Retargeting those programs to net8 without a dependency audit would repeat the earlier compatibility mistake. The proposed small Framework host calls extracted, hash-pinned original methods with only Framework drawing types. It establishes **current checked-out source-helper behavior on Framework GDI+**, not the entire effect pipeline or installed Vixen 3.12.4 binary equivalence. Returned inventory covered selected audio dependencies; it did not establish the installed release's drawing/SVG dependency versions.

## Exact source contract

The original checkout is `upstream/Vixen`, commit `e111ae41963abfa48b70f884445950e94e432e02`. The initial compile-only extraction retains each method's signature and body verbatim inside a minimal wrapper class. It retains the complete FastPixel class; its only adaptation is an explicit `using System;` for the original project's implicit imports. No drawing, rounding, disposal, interpolation or alpha behavior was replaced. The original Picture fit helper's undisposed Graphics is preserved; one-case child processes bound its lifetime.

| Original file and lines | Method | SHA-256 of exact extracted signature/body |
| --- | --- | --- |
| `upstream/Vixen/src/Vixen.Modules/Effect/Picture/Picture.cs:1286–1302` | `ScalePictureImage` | `1f7fabb369e4ad7dbd9e7f00e70e4e9ddc66c20b546f5d9174d51deaeeb971be` |
| Same file, 1304–1328 | `ScaleImage` | `f8878340536121eb501da8e862db1c180e17b88bd534f930b2cf0b75d1080cc1` |
| Same file, 1330–1349 | `ConvertToGrayScale` | `f8a5b17fada09abf8eec359d7664d40296fca1548134bc62bf3fb2fcef78e1f0` |
| `upstream/Vixen/src/Vixen.Modules/Effect/CountDown/CountDown.cs:1115–1139` | `DrawFilmSpinner` | `636d3091cffffa220a73b906b2eaef12ebd7c90de995befca31bc05ef781c064` |
| `upstream/Vixen/src/Vixen.Modules/Effect/CountDown/CountDown.cs:1141–1151` | `DrawFilmSpinnerCrossHash` | `07975989b97404316341b29ce4185eb3d91f55f3bd180f05149fa09445afe78b` |
| Same file, 1153–1160 | `DrawFilmSpinnerRadialLine` | `3d1e3fcb6b0cedecf5482f20e1e164ec60fd7231fa32e1f0a9247f8d0b789c51` |

Full file hashes:

- Picture.cs: `a5102426351a0f1b75c7f06369b1b6d2eb0c06c7491a72dba4bb655a191ecab9`.
- CountDown.cs: `b5ea8344844b6b024091a6359c601455250052868e3505f10b7a8cfec5c182e2`.
- `upstream/Vixen/src/Vixen.Common/FastPixel/FastPixel.cs`: `74afded565f8dc3aa8c68061dbeec0d753f6cb9380c93ae4dac4d394d90bbffb`.

Machine-readable original compile-only extraction hashes remain in `target/windows-raster-compile/source-contract.json`. The implemented schema2 pack's source contract is `tools/windows-raster/source-contract.json`; `extract_sources.py --check` regenerates and compares all six method bodies, FastPixel and the bounded scheduler extract. CountDown adds public forwarding wrappers and an explicitly documented constant-white `CreateCrossHashBrush` fixture; the original method bodies are unchanged. Gradient selection remains outside this oracle.

## Implemented deterministic case grid

The implemented schema2 grid contains **74 cases** below. Implementation and local compilation do not establish passing Windows raster coverage. Every case is a fresh process. No fonts, user pictures, audio or profiles are involved.

| Family | Exact inputs/operations | Count |
| --- | --- | ---: |
| Picture | Four input bitmaps: 3×5 and 8×4, each opaque or alpha palette. For each: identity/FastPixel baseline; `ScalePictureImage` with maximum boxes 1×1, 5×3, 13×11; `ScaleImage` scales 0.25, 0.5, 1, 1.5; `ConvertToGrayScale` | 4 × 9 = 36 |
| CountDown cross hash | Transparent 32×32 canvas; opaque white, width-1 pen; rectangles `(0,0,4,4)`, `(2,3,9,9)`, `(1,1,20,12)`, `(-2,2,9,9)` | 4 |
| CountDown combined spinner | Same canvas/pen, cross hash followed by radial line. `(rect, length)` = `((2,3,9,9),8)` and `((12,10,12,12),18)`; spinner angles 0, 44, 45, 46, 90, 179, 180, 359. The original radial helper receives angle + 90 and the rectangle's integer center, exactly as the calling source does | 2 × 8 = 16 |
| GDI primitives | Eight octant lines, plus zero-length, horizontal and vertical line; three rectangle fills; one rectangle outline; one ellipse outline; a self-intersecting star filled in Alternate and Winding mode | 11 + 3 + 1 + 1 + 2 = 18 |

Picture pixels are synthetic straight RGBA, not encoded image files. At `(x,y)`, choose `R=(71*x+31*y+17)%256`, `G=(29*x+97*y+43)%256`, `B=(113*x+19*y+89)%256`. Opaque alpha is 255; alpha cases cycle `[0,1,127,128,254,255]` using row-major index. This deliberately retains hidden RGB at zero alpha. Record the requested bytes and actual GDI GetPixel/FastPixel reads before any operation. Create only `Format32bppArgb` source bitmaps. Do not add 24-bit inputs to this initial grid: the original FastPixel implementation copies contiguous width×height bytes without interpreting stride, so 24-bit padding would introduce a different compatibility question.

Define primitive coordinates once in the case manifest, consumed by both hosts, rather than recomputing trigonometry in two languages. Suggested 40×40 canvas: octant rays from `(20,20)` with deltas `(9,4),(4,9),(-4,9),(-9,4),(-9,-4),(-4,-9),(4,-9),(9,-4)`; zero line `(2,2)→(2,2)`; horizontal `(1,3)→(10,3)`; vertical `(3,1)→(3,10)`; filled rectangles `(2,3,4,5)`, `(-2,1,5,3)`, `(1.5,2.5,3.5,4.5)`; rectangle outline `(1,1,4,3)`; ellipse `(5,5,20,12)`; star points `[(20,2),(31,35),(3,14),(37,14),(9,35)]` in that order. Use opaque white, width 1, identity transform and a transparent background. Primitive cases are direct Framework API references, **not original effect-method executions**.

## Rust comparison targets

Use the actual production functions from crate-local test modules or a diagnostic adapter. Do not duplicate their algorithms in a comparison program.

| Windows operation | Existing Rust implementation |
| --- | --- |
| Picture fit, scale, grayscale | `crates/fx-pixel-graphics/src/picture.rs:141`, `:151`, `:163` (`scale_picture_image`, `scale_image`, `to_grayscale`) |
| Straight RGBA and pixel readback | `crates/vixen-effects/src/gfx.rs:52`, `:77`, `:104` (`Bitmap::from_rgba`, `get_pixel`, `to_rgba`) |
| Resizing and image drawing | `gfx.rs:97`, `:366` (`Bitmap::resized`, `Graphics::draw_image`) |
| Cross hash and combined spinner | `crates/fx-pixel-graphics/src/count_down.rs:431`, `:441` (`draw_cross_hash`, `draw_film_spinner`), with a constant white Brush |
| Shape mask/color application | `crates/fx-pixel-graphics/src/textdraw.rs:77` (`Canvas::fill_shape`) |
| Primitive drawing | `gfx.rs:194–280`: rectangle, line, polygon and ellipse APIs; existing `gdi_rules` unit tests begin at line 560 |

The port stores premultiplied pixels and demultiplies on read; therefore partial alpha may already differ at baseline. Preserve that difference as evidence rather than silently pre-normalizing the original input. Its grayscale arithmetic, bilinear image drawing, aliased ellipse flattening, thin-line endpoints and device offset all merit measurement. CountDown additionally places lines at `+0.5` relative to the original integer coordinates. These observations identify sensitive cases; they do not establish a bug before the Windows raster is captured.

Rust source hashes at this review: picture `19f514577e8ae05a709138857fa2d13580c77eb97fccf94f50ad0ec9f1572441`; count_down `c5e722517f92a8c0d354f42480f888b44e48ade72e0139d8a3ee44da71ec61c7`; textdraw `606bda6b7088874b9199d9273810aa9cab700831735301ed57196060127b9531`; gfx `33a4b6a51118a3ec7730aadbe41fc3a15d59e29909138ef7a7ed7bf163a998a1`. Re-pin the actual tested revisions when implementing the comparison.

## Capture and comparison contract

Each case writes requested input bytes, observed input bytes, output dimensions, and output pixels as explicitly ordered **A,R,G,B bytes in top-to-bottom row-major order**. This avoids dependence on native integer endianness or a PNG encoder. Capture Bitmap.GetPixel and original FastPixel.GetPixel separately; record pixel format, LockBits stride and readback equality. Unlock without writing. A PNG is optional for inspection and never the exact oracle. For CountDown retain the drawing bitmap and the post-mask output separately if the comparison requires `Canvas::fill_shape`'s alpha threshold.

Record all effective GDI settings (smoothing, pixel offset, interpolation, compositing, page unit/scale, transform, DPI), even when left at defaults. Do not change the helper's defaults to match Rust. Metadata should include OS/architecture, CLR version, compiler path/version/hash, host executable hash, Framework System.Drawing assembly path/version/hash and loaded `gdiplus.dll` path/version/hash. Do not mistake the successful net8 audio host for this Framework runtime. Pin case-list, input, source, output and provenance hashes. Write each case manifest last; a missing manifest denotes incomplete capture.

The checker validates size/hash/uniqueness contracts before parsing, then compares dimensions and all four channels exactly. Report differing pixels/channels, first bounded set of coordinates, maximum channel delta and mismatch bounds, separately for opaque and partial-alpha inputs. Record premultiplied/color-mask diagnostics only as additional views. Neither a tolerance nor normalization can convert a failed exact comparison into acceptance. A discrepancy should yield a small reproducible case, a source-backed fix or a documented behavior decision, followed by the same exact comparison and relevant effect regression tests.

## Bounded runner and failure handling

Reuse the tested `StageSpec` / `CaptureSupervisor.RunStage` / Windows Job Object process machinery from `tools/windows-pcm/Supervisor.cs`, with a new raster workflow. Do not call its PCM/inventory workflow. There is no Vixen selection dialog, original installation access, audio capture, profile initialization, package restore or network request.

Proposed stages: pack integrity; Framework compiler discovery/compile (30 seconds); GDI metadata/preflight (15 seconds); one process per deterministic case (10 seconds each); input/source recheck; bounded packaging. Keep the existing overall 660-second limit and 70-second packaging reserve plus bootstrap watchdog. An individual timeout/crash is recorded and the next independent case proceeds while budget remains; exhausted cases are explicitly skipped. Case children run in jobs so descendants are terminated. Keep bounded stdout/stderr and progress files, with an initial case record before launch and a final record after exit. No native error dialog or console selection may suspend the supervising deadlines.

Use a fresh results directory and exclusive case paths; reject traversal, duplicate IDs, unexpected files, oversized JSON or invalid dimensions. All fixtures are fixed and at most 64×64; cap each raw output at its exact expected length and aggregate captured data at 16 MiB. If ZIP creation fails, preserve the complete or partial results directory with a plainly stated copy-back instruction. Final status distinguishes complete capture, partial capture, comparison pending and comparison passed. A successful child exit without its complete validated manifest is a failed case.

Before any delivery, adapt and rerun the existing supervisor fault tests for compiler missing/failure/stall, case crash/hang, inherited output handles, output flooding, corrupt/missing manifests, permission/disk failures, overall budget exhaustion and packaging failure. Test archive path/hash integrity and clean extraction locally. The user should need only one launch and one copy-back even when some cases fail.

## Local compile-only result and limits

The initial `target/windows-raster-compile/RasterCompileOnly.csproj` compiled the five unchanged method bodies plus complete FastPixel successfully with C# language version 5: zero compiler warnings and zero errors, 0.91 seconds. The build log includes an unrelated `CSSM_ModuleLoad` environment message before success. That DLL was **not executed**; this historical check preceded the runnable host implementation below.

No .NET Framework reference pack or Framework compiler was found in the inspected local caches or `/Library/Frameworks`; this check used cached SDK 10 / net10 references and the already cached System.Drawing.Common dependencies. It validates C# 5 syntax and member resolution under those references, **not Framework 4 compilation or Windows drawing behavior**. The selected helper APIs are longstanding Framework-style Bitmap/Graphics/ImageAttributes/ColorMatrix/LockBits/Marshal APIs, but actual Framework compiler/runtime validation remains a required Windows stage. Do not distribute the locally built net10 library as the Framework host.

Evidence: `target/windows-raster-compile/{source-contract.json,OriginalPictureMethods.cs,OriginalCountDownDrawMethods.cs,OriginalFastPixel.cs,RasterCompileOnly.csproj,build.log}`. Build-log SHA-256: `eba2cf84cff4e62899477d4a855cf7e2c1d58b0f5d7493c7134c485ad8f28b83`; local compile-only library: `dd87ec5dc6a94cffb512be0fc8e6ee5e533d87cebdad6a63c00137ecac113d57`.

## Current 74-case review pack

The final source-only review archive is `target/windows-raster-pack-03/Vixen-Windows-Raster.zip`, **87,272 bytes, 32 entries**, SHA-256 `9297c4b61392157d588330cb03c9bb759ec8df0b9a2d74df8e02388dc63a316d`. Schema2 tags each case as Picture, CountDown or primitive and shares explicit coordinates. Picture retains the same 36 operations and pixels as sealed pack01. CountDown and primitive inputs are transparent zero-filled canvases; their output is captured through the same five-buffer GetPixel/FastPixel contract. All original sources, exact extracted bodies, case definitions and local-check inputs are pinned in the archive.

CountDown is explicitly **non-rotated, identity-transform, uniform-white helper coverage**. Source caller inspection shows fresh Graphics at CountDown.cs:620; its non-rotating branch retains default drawing state, while lines 729–738 add transforms for rotation. Prior swipe/background, text, gradient selection, transformed rendering and their compositing interactions are excluded. The three original private spinner/cross/radial methods are preserved; only the brush selection adapter supplies white. Direct primitive cases use Framework float-coordinate overloads; every integer and half-pixel input converts exactly. These are Framework API references, not installed Vixen binary/effect equivalence.

For CountDown/primitives, `actualDrawingDefaults` records the real Graphics immediately before drawing, including transform, clip/visible-clip bounds and infinite-clip flag. Picture's unchanged internal bodies are not instrumented: its `freshGraphicsDefaults` is separately labelled as a new Graphics baseline. `drawingStateScope` makes this difference machine-readable, and manifest validation rejects a missing actual-state record for non-Picture cases. The Mac CountDown consumer must retain its actual Canvas post-mask result as a distinct comparison target rather than silently thresholding the Windows raw raster.

`target/windows-raster-local-04/result.json` passes **247 checks** against the final source set: C#5 builds using cached net10 references, independent 74-case JSON agreement, 14 malformed-grid rejections, real bounded local process failures and 28 injected workflow scenarios. These include wrong state provenance and a missing CountDown state record, with only the affected case rejected. Actual Windows Framework csc, GDI+, PowerShell and native Job Objects remain unexecuted. The main 660-second budget and 70-second packaging reserve are unchanged; 74 repeated 10-second stalls cannot all fit, so remaining cases are explicitly skipped and partial diagnostics returned. The independent outer bound remains 900 seconds.

`target/windows-raster-package-checks-03/result.json` validates the immutable final archive and rejects **16 altered archives**, including separate original/extracted CountDown changes and wrong-family data. `target/windows-raster-pack-03/source-delta-from-02.{diff,json}` retains the final metadata-only changes for independent review; case grid, source contract and original extracted bodies are unchanged from intermediate pack02. The independent pack02 review passed **92 checks** in `target/windows-raster-pack-02-independent-review.json`; final metadata review is owned by the parent task. No Windows raster or Mac parity gate closes on this preparation evidence.

The separate Mac preparation renders all 74 cases through actual production helpers and retains them at `target/raster-mac-74-final/render`. Independent consumer alignment review in `target/raster-consumer-74-independent-review.json` verifies the frozen pack03 grid/source contract, current source pins and the **54-check synthetic consumer suite** at `target/raster-consumer-74-final-v3/result.json`. Its 15 assertions distinguish computed evidence checks from manual source-review conclusions. Exact stage/file provenance, drawing-state scope, partial-result handling and raw-GDI-versus-Canvas labels align with the Windows return schema. No returned Windows raster exists yet, and synthetic fixtures explicitly cannot establish Windows parity.

## Historical 36-case pack and local verification

The initial `tools/windows-raster` implementation retained in `target/windows-raster-pack-01/Vixen-Windows-Raster.zip` covered the fixed Picture grid, unchanged source extraction, Framework capture host, bounded scheduler, PowerShell bootstrap/packaging workers, independent outer watchdog and read-only archive checker. That historical source-only artifact is **71,480 bytes, 29 entries**, SHA-256 `8ea45fd4a308b529fe59ce30478ce316fc64f3ccd9bcb7878b74abfc19a613ab`. It contains no runtime binaries, returned Windows binaries, private show/audio, credentials or installed Vixen dependency. The immutable Windows PCM pack is unchanged.

The Windows entry is `Run-Raster.cmd`. It needs no installation path, .NET SDK, network or dependency download. It uses the existing Framework compiler with C# 5 and explicit System, System.Core, System.Xml, System.Runtime.Serialization and, for the capture host, System.Drawing references. Compile and preflight failures are dependencies recorded before all 36 cases are marked skipped. A single case failure preserves its partial buffers and allows independent later cases to run. Post-preflight source/read failures and a blocked cases directory still reach final integrity, summary and packaging. Bootstrap failure propagates its nonzero result and preserves the folder.

Bounds are explicit: supervisor compile 60 seconds; host compile 30 seconds; preflight 15 seconds; each case 10 seconds; integrity 20 seconds; packaging 60 seconds; main workflow 660 seconds with 70 seconds reserved for finalization; bootstrap supervisor limit 720 seconds; independent outer process-tree limit 900 seconds. Child stdout/stderr are capped at 1 MiB each. The return ZIP allows at most 1,000 files, 2 MiB per file and 64 MiB total including diagnostics; raw case buffers also have exact dimension-derived sizes. Windows Job Objects own stage descendants. No `ReadToEnd` or transcript loop is used.

Every successful case writes five raw buffers (`input-requested`, input GetPixel/FastPixel and output GetPixel/FastPixel), exact source/host/file hashes, dimensions, positive stride, Format32bppArgb and fresh Graphics defaults. Preflight records OS/CLR, process/OS bitness, System.Drawing identity/hash and loaded GDI+ identity/hash. It does not claim an unobserved CPU architecture. The finalized `capture-summary.json` goes into the return ZIP; changing progress/bootstrap/package-stage files remain in the folder. Its `validWindowsRasterCaptured` flag means at least one case passed manifest validation, not that no drawing was attempted when false. `captured-comparison-pending` explicitly leaves Mac parity unverified.

`target/windows-raster-local-02/result.json` passed **215 local checks** against the exact packaged source set. All three C# projects compile under C# 5 using cached net10 reference assemblies. Common-code checks compare the fixed C# grid with independent JSON and reject six malformed grids. Real local subprocesses exercise success, nonzero/partial output, timeout, oversized/flooding output, descendant-held handles and overall expiry. Injected workflow scenarios exercise missing/failing/empty-output compilers, preflight failures, disappearing source, blocked output directory, malformed/missing manifests, bad hashes/paths/dimensions/readbacks/input bytes, final integrity failure, packaging failure and reserved-budget exhaustion. The external Python fault child is hash-pinned and retained in the pack. These checks execute **no Windows GDI, Framework compiler, PowerShell or Windows Job Objects**.

`target/windows-raster-package-checks/result.json` separately validates the immutable ZIP and rejects **13 altered archives**: checksum, missing/duplicate/case-fold duplicate/unlisted entries, traversal, symlink, oversize, executable content, original/generated source drift, external fixture drift and an unsupported Windows-execution claim. The pack also retains exact original source files and the original Vixen license. Actual Windows compilation/capture and the exact production Rust comparison remain required; no graphics parity gate closes on this preparation evidence.

Independent read-only review passed **45 checks** in `target/windows-raster-pack-independent-review.json`: immutable archive identity/bounds, all 21 local source pins plus the external fixture, 215-check report, 25 workflow scenarios, separately sliced original method hashes, full FastPixel/import, upstream commit and license. Source review found no remaining material issue after the setup-finalization and explicit Framework-reference corrections. This is a review of the pack and local failure evidence, not Windows execution.

This first batch leaves full Text/CountDown font rendering, font-file parity, gradients/transforms, encoded image/GIF frame handling, SVG/embedded Shapes art, complete effect lifecycle and installed-release GUI/render parity open. Fonts and SVG should be separate explicitly sourced sections of a later expanded oracle, not substituted dependencies or inferred passes. macOS 26, real controllers and other release gates remain independent.
