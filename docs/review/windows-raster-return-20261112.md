# Windows raster return — November 12, 2026

The returned Windows capture is complete and valid. All 74 cases and 78 process stages completed without timeout, output-limit termination or errors. Initial and final checks each verified the 31-file capture kit. The returned manifest verifies all 684 files; the ZIP contains 685 entries including that manifest. No returned executable was run on this Mac.

The strict, unchanged comparator finds **19 exact cases and 55 different cases**, with no invalid cases and matching dimensions throughout. This establishes measured compatibility defects; it does not establish raster parity.

| Scope | Exact | Different | Findings |
|---|---:|---:|---|
| Picture, opaque | 4/18 | 14/18 | Identity and 100% scale match for both sizes. Other resizes differ in sampling phase/edges. Grayscale differs by one channel level where Windows rounds above the port's truncation. |
| Picture, partial alpha | 0/18 | 18/18 | Mac input readback already loses straight-alpha RGB through premultiplication. Resize/grayscale outputs have additional differences. Input differences remain counted even where output converges. |
| CountDown, font-free | 0/20 | 20/20 | Cross hashes and combined spinners differ at 16–102 binary white/transparent pixels per case. The current port adds half-pixel offsets absent from the original, and the shared ellipse rasterizer differs independently. |
| Drawing primitives | 15/18 | 3/18 | All 11 lines, including zero-length, all three fills and rectangle outline match. Ellipse differs at 14 pixels. Both polygon fill modes miss the same left vertex at (3,14). |

For example, GDI bilinear scaling 3×5 to 1×1 retains source pixel (0,0), ARGB `[255,17,43,89]`; the port samples its center, producing `[255,150,10,240]`. Scaling 3×5 to 6×11 gives GDI alpha128 at the right edge while the port clamps to opaque. These are observed samples; a generic corrected rasterizer still needs verification. GDI reports Bilinear interpolation, Default pixel offset and SourceOver composition.

This host uses x64 .NET Framework CLR 4.0.30319.42000, Framework compiler4.8.9232.0, System.Drawing4.0.0.0 and GDI+10.0.22621.4830. The legacy OS-version API reports Windows NT6.2.9200.0; this is retained verbatim, not interpreted as the actual Windows release.

Evidence is retained under `target/windows-raster-results/attempt-01/`: original `returned.zip` (274,440 bytes, SHA256 `0d8a00e11b0f406078221cccc6145e9f21f8dc8a7444ac256c72a410827a78aa`), exact `comparison.json`, and `acceptance.json` with 18 capture/provenance checks and the complete case breakdown. The nine recorded Mac source hashes and frozen test executable matched the original capture at audit time. The frozen Mac74 source contract and cases match delivered Windows pack03 byte-for-byte.

This comparison runs extracted, hash-pinned upstream Picture/CountDown helper bodies on Windows Framework GDI+ against actual Rust production helpers. CountDown is explicitly Windows raw drawing versus the port's Canvas post-mask with constant white brush. The 18 primitives directly call drawing APIs. It does not establish installed Vixen binary equivalence, complete effect lifecycle, fonts, encoded images/GIF, transformed scenes or macOS26 behavior. No additional Windows run is needed to diagnose these retained 74 cases.
