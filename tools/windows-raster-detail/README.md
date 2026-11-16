# Windows raster detail diagnostic (schema 3)

This separate source-only pack collects 80 focused numeric cases using the
PC's existing .NET Framework compiler and System.Drawing/GDI+. The successful
original schema-2 74-case pack is unchanged. The follow-up resolves numerical
ambiguities in the returned raster data; it does not replace those results.
No Vixen installation, show, audio, controller, network access or install is used.

Run `Run-Raster.cmd` from a fully extracted ordinary local folder. Return the
results ZIP, or the entire results folder after any failure. The reused supervisor
has per-process deadlines, Windows Job Object descendant cleanup, 1 MiB stream
caps, a 660-second workflow budget and a 70-second packaging reserve. Each case
is a fresh process with a 10-second limit. Repeated stalls can exhaust the budget;
remaining cases are explicitly skipped. The independent outer 15-minute watchdog
also bounds bootstrap stalls. No guarantee of 80 completions under stalls is made.

The fixed case grid contains:

- 60 exact-size resize probes: source widths 3/4/7/8/9, destination widths
  6/7/8/9/12/13, fixed source height 4 and destination height 6, each opaque or
  with cyclic alpha 0/1/127/128/254/255. RGB follows the original pack's formula.
  The destination is `new Bitmap(width,height)` and drawing uses the exact
  integer destination-size `DrawImage(image,0,0,width,height)` overload used by
  both original Picture resize helpers. Independent width/height are intentional
  numerical probes, not the aspect-fit helper's dimension-selection behavior.
- Four original `ConvertToGrayScale` calls on 16×3 explicit RGB swatches. Each
  case's alpha rows are 0, selected alpha 1/127/128/254, and 255. Columns include
  gray 0/128/255 and 13 colors whose candidate float32 dot product gives a half
  boundary. These are experimental inputs, not an assertion of GDI's arithmetic
  order or rounding rule. Exact swatches and row layout are in `cases.json`.
- Eight ellipse rectangles: small, odd, wide, shifted, clipped and fractional,
  on transparent 40×40 canvases using a white width-1 pen. Each records native
  DrawEllipse output, original AddEllipse path, DrawPath(original), and cloned
  paths flattened at 0.25/0.5/1.0 with their respective DrawPath output.
- Four fractional line segments and their reverse directions, on transparent
  40×40 canvases with white width-1 pens. Coordinates convert explicitly to float.

Each successful case retains requested ARGB8 bytes, Bitmap.GetPixel and original
FastPixel readbacks, exact dimensions/stride/format, source/host hashes and
Graphics state. Each ellipse additionally retains the path's float coordinates,
exact IEEE float32 bit patterns, path types, flattening value, fill mode, pen
state, per-variant Graphics state and both readback buffers. Its manifest is
written last. No PNG decoder, color conversion or tolerance is used in capture.

The resize primitive records actual Graphics settings immediately before its
DrawImage call. Grayscale's original method remains uninstrumented; its state
scope explicitly records only newly created Graphics defaults. DrawEllipse and
line cases record actual state before drawing, including transform and clipping.
Preflight records the actual loaded Framework/System.Drawing/gdiplus.dll hashes.

`extract_sources.py --check` verifies unmodified original Picture methods and
FastPixel source. The retained CountDown adapter is provenance-only and is not
executed by this detail grid. The source-extraction contract remains schema 2;
the independent detail case/report contract is schema 3. Full original sources,
the original license and the reused supervisor source are bundled for audit.
This is not proof of equivalence to installed Vixen binaries or full effects.

`capture-summary.json` is finalized before packaging and returned with a hash
manifest. Changing progress/bootstrap/package logs stay in the results folder.
A failed independent case does not stop later cases. `validWindowsRasterCaptured`
means at least one case passed exact manifest validation. Capture success is not
comparison success; the data still needs exact offline channel/path analysis.
The original 74-case comparator is deliberately not reused for this different grid.

Local commands:

```sh
python3 tools/windows-raster-detail/extract_sources.py --check
python3 tools/windows-raster-detail/generate_cases.py --check
python3 tools/windows-raster-detail/check_local.py --out target/windows-raster-detail-local-N
python3 tools/windows-raster-detail/package.py --out target/windows-raster-detail-pack-N --checks target/windows-raster-detail-local-N/result.json
```

Local checks compile all C# hosts in language-version-5 mode with cached net10
references, validate independent case contracts, run bounded real fault processes
and inject compiler/preflight/output/path/state/stream/package failures. They do
not execute Windows GDI, Framework csc, PowerShell or Windows Job Objects. These
limits are explicit in the package provenance. No Windows success is claimed
before the actual returned data is validated.
