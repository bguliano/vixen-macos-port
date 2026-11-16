# Mac Picture raster preparation

The schema1 diagnostic renders the same36 deterministic Picture cases as the immutable Windows raster pack01. It calls the actual `scale_picture_image`, `scale_image`, `to_grayscale`, `Bitmap::from_rgba`, `get_pixel`, and `to_rgba` implementations. Its only Rust addition is a `cfg(test)` module with an explicitly ignored generator; normal application builds and rendering behavior are unchanged. No new dependencies, UI, font drawing, audio, controllers, network requests or Windows drawing are involved.

The exact schema1 cases and source contract are frozen in `picture36/`, byte-identical to `target/windows-raster-pack-01/Vixen-Windows-Raster/`. They deliberately remain separate from the subsequent74-case Windows schema2 preparation. Neither generator nor consumer silently accepts the other schema.

From the checkout:

```sh
python3 scripts/capture_picture_rasters.py --out target/picture-raster-new
python3 scripts/check_picture_raster_consumer.py \
  --mac target/picture-raster-new/render --out target/picture-raster-consumer-new
```

Builds use the project-local Rust toolchain, `--offline --locked`, existing cached dependencies and the dedicated graphics target directory. Output directories must be fresh. The generator records actual source and test-executable hashes, preserves raw requested bytes before premultiplication, verifies both pixel-readback APIs, and writes each completed case manifest last. An incomplete render lacks a completed capture summary.

When a schema1 Windows capture is available, compare its ZIP directly without extraction, or a returned result folder:

```sh
python3 scripts/compare_picture_rasters.py \
  --mac target/picture-raster-new/render \
  --windows '/path/to/returned-results.zip' \
  --out target/picture-raster-comparison-new.json
```

The consumer caps file counts/sizes and JSON sizes; rejects traversal, links, duplicates, malformed reports, mismatched provenance/hashes, unexpected case files, incomplete cases and changed requested input; and validates the ZIP return manifest. A folder without that packaging manifest is explicitly recorded as having no returned-archive integrity proof. Runtime metadata must identify Windows Framework4/System.Drawing4.0 with matching captured-host/source hashes. No returned executable is run.

Every raw stream is straight A,R,G,B bytes, top-down row-major, four bytes per pixel. Dimensions are explicit; a dimension mismatch fails exact comparison. Windows `Bitmap.GetPixel` and original `FastPixel.GetPixel` are independently compared with Rust `get_pixel`. In Format32bppArgb, original FastPixel reads contiguous B,G,R,A memory bytes and returns all four channels; the host requires stride==width×4 and does not silently repair padding/stride. Rust uses premultiplied RGBA storage, so it may lose hidden RGB at alpha0 or round RGB at partial alpha before any Picture transform. Those baseline differences are recorded and count against exact parity; no tolerance, alpha stripping or normalization is applied. `to_rgba` is an additional Rust API readback, not an imitation FastPixel implementation.

The comparison reports opaque/alpha groups, differing pixels/channels, A/R/G/B counts, maximum channel delta, mismatch bounds and the first16 coordinates. Exit0 means a complete exact comparison, exit1 means valid complete capture with differences, exit2 means missing/invalid/partial evidence. Tests construct explicitly synthetic Windows-shaped files and force `windowsParityVerified=false`; the normal consumer refuses those files as actual Windows evidence.

This preparation does not establish Windows parity, encoded image/GIF behavior, complete effect rendering, fonts, installed Vixen binary equivalence or macOS26 compatibility. See `docs/review/picture-raster-mac.md` for recorded local results.

## Expanded schema2 /74-case batch

`raster74/` is frozen byte-for-byte from Windows pack03. It retains the36 Picture cases and adds20 CountDown and18 primitive cases. The same producer/consumer supports both explicitly selected schemas:

```sh
python3 scripts/capture_picture_rasters.py \
  --cases tools/mac-raster/raster74/cases.json --out target/raster74-new
python3 scripts/check_picture_raster_consumer.py \
  --mac target/raster74-new/render --cases tools/mac-raster/raster74/cases.json \
  --source-contract tools/mac-raster/raster74/source-contract.json \
  --out target/raster74-consumer-new
python3 scripts/compare_picture_rasters.py \
  --mac target/raster74-new/render --windows '/path/to/returned-results.zip' \
  --cases tools/mac-raster/raster74/cases.json \
  --source-contract tools/mac-raster/raster74/source-contract.json \
  --out target/raster74-comparison-new.json
```

CountDown calls its actual `draw_cross_hash` and `draw_film_spinner` methods through a `cfg(test)` adapter with a constant white gradient, level1 and no fade. It preserves production `Canvas::fill_shape` thresholding and the existing +0.5 device-coordinate offsets. Its32×32 output is explicitly **Canvas post-mask**, compared against **Windows raw GDI** without quietly treating those stages as identical. This is a font-free, identity-transform, no prior swipe/text/background fixture, not the whole CountDown effect. The18 primitives call the actual public `Graphics` APIs on40×40 canvases, including fractional rectangle coordinates and both polygon fill modes. Requested input is transparent zero for these38 cases.

The consumer preserves effective Windows drawing settings and clipping metadata, requires the exact source-scope label for each family, and validates successful initial integrity, Framework compile, preflight and each captured-case stage against its retained stage JSON. Missing/failed final integrity prevents acceptance while preserving valid case comparisons diagnostically. Coherent partial captures retain each successful comparison and list skipped/failed/invalid cases; overall `windowsParityVerified` remains false and exit status is2. Family totals distinguish Picture opaque/alpha, CountDown and primitives.

Final local scope and binary/source provenance: `docs/review/raster-mac-74.md`. The schema1 results, sources and executable remain preserved separately.
