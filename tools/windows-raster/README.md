# Windows font-free raster capture (schema 2)

This independent source-only pack captures 74 small cases using
the PC's existing .NET Framework compiler and System.Drawing/GDI+. It does not
load Vixen, select an installation, read a show, play audio, contact the network
or install a runtime. The earlier Windows PCM pack remains untouched.

Run `Run-Raster.cmd` from a fully extracted ordinary local folder. Return the
displayed results ZIP, or the entire results folder after any failure. The main
workflow has per-process deadlines, Windows Job Object descendant cleanup,
1 MiB stdout/stderr caps and a packaging reserve. Each capture is a fresh
10-second process. The main workflow keeps its 660-second total and 70-second
packaging reserve, so repeated case timeouts can exhaust the capture budget;
remaining cases are recorded as skipped. It does not promise 74 completions
under stalls. The outer 15-minute watchdog also bounds bootstrap stalls.

The 36 Picture cases use 3×5 and 8×4 synthetic images, each opaque or with alpha values
0,1,127,128,254,255. Each image has an identity/FastPixel baseline, three fit
boxes, four scales and grayscale. `cases.json` is validated against the host's
fixed 74-case schema 2 contract before any image creation. Requested straight pixel bytes
are explicit A,R,G,B order; no PNG decoder or normalization intervenes.

The 20 CountDown cases cover four cross-hash rectangles and two spinner
rectangles at eight angles, including clipping and integer endpoints. The exact
original spinner, cross-hash and radial-line bodies are retained; an explicitly
identified adapter supplies a constant white brush instead of selecting a
gradient. The 18 direct Framework primitive cases cover eight line octants,
zero/horizontal/vertical lines, three rectangle fills, rectangle/ellipse outlines
and Alternate/Winding self-intersecting polygon fills. These start on transparent
32×32 or 40×40 canvases, use white width-1 pens/brushes and shared explicit
coordinates. Direct primitives are Framework references, not original effects.
Primitive calls use the float-coordinate overloads; all integers and half-pixel
values in this grid convert exactly. The CountDown fixture covers non-rotated,
identity-transform helpers. Prior swipe/background, text, transformed rendering
and their compositing interactions are excluded.

`extract_sources.py --check` proves that the three Picture methods, three
CountDown methods and complete FastPixel class match the pinned source. Only
the documented wrappers/imports/constant-brush fixture are added. The source contract also identifies the
reused bounded supervisor portions and its two small adaptations. Full original
source and Vixen's original license are retained in the pack. This is a
checked-out-source-helper oracle, not equivalence to the installed Vixen 3.12.4
graphics binaries or the entire effect pipeline.

Each successful case retains requested input, Bitmap.GetPixel and original
FastPixel readbacks before/after drawing, plus dimensions, stride, pixel format,
fresh Graphics defaults and source/host hashes. Its manifest is written last.
For CountDown/primitives, `actualDrawingDefaults` records the Graphics settings
immediately before drawing, including transform, clip/visible-clip bounds and
whether the clip is infinite. Picture's original bodies remain uninstrumented;
its `freshGraphicsDefaults` is only a newly created Graphics baseline, explicitly
distinguished by `drawingStateScope`.
The preflight records Framework/System.Drawing/loaded gdiplus.dll provenance.
No report treats the locally built net10 checker as a Framework Windows result.

`capture-summary.json` records finalized capture status before packaging; it is
included in the returned ZIP. Changing bootstrap/progress/package-stage logs
remain in the results folder to avoid racing writes. The ZIP includes its own
`return-manifest.json` with exact path, length and hash of each snapshot file.
Packaging failure preserves the folder. A single failed case does not stop
independent later cases; exhausted cases are explicitly marked skipped.
`validWindowsRasterCaptured` means at least one case passed manifest validation;
false does not mean the preflight or failed cases never attempted drawing.

`captured-comparison-pending` means raw Windows capture succeeded. It is **not**
a Mac parity pass. Compare all four channels and dimensions exactly using the
production Rust Picture/gfx/CountDown functions; retain mismatch counts/coordinates and
alpha-baseline differences rather than applying a tolerance. A returned capture
and this final comparison are still required. For CountDown, report raw GDI
drawing versus the actual Rust Canvas post-mask bitmap distinctly; white brush
fixtures do not validate gradient selection. Fonts, SVG, encoded image/GIF
loading and full effect lifecycle remain separate gaps.

Local commands:

```sh
python3 tools/windows-raster/extract_sources.py --check
python3 tools/windows-raster/generate_cases.py --check
python3 tools/windows-raster/check_local.py --out target/windows-raster-local-N
python3 tools/windows-raster/package.py --out target/windows-raster-pack-N --checks target/windows-raster-local-N/result.json
```

Local checks compile all host/source code in C#5 mode with cached net10 reference
assemblies. They validate common case contracts, run real bounded failure
subprocesses and inject workflow outcomes including failed/missing compilers,
preflight errors, disappearing sources, blocked output directories, malformed
case manifests, bad hashes/paths/dimensions, case crashes and package failure.
They do not execute Windows GDI, Framework csc, PowerShell or Windows Job Objects.
Those limitations remain explicit in each local report and package provenance.
