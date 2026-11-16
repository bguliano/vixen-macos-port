# Picture movement and pixel-sampling oracle

The project compiles the complete original Picture effect/data and FastPixel
implementation unchanged. Since GDI+ is unavailable, reflection supplies an
already-decoded **opaque** image directly to the original FastPixel byte buffer.
The original `InitFrameData`, `InitialRender` and `CalculatePixel` methods run for
every frame. String output uses the original PixelFrameBuffer; location output
uses a trivial coordinate/color dictionary. No movement or sampling algorithms
are copied into the oracle. Bitmap/font constructors, image load/resize and full
PixelEffectBase orchestration are not invoked.

```sh
tools/golden/picture/build.sh run
source env.sh
PICTURE_GOLDEN="$PWD/target/golden/picture/data/picture-sampling.json.gz" \
  cargo test -p fx-pixel-graphics original_picture_movement -- --nocapture
```

The checked-in `tests/golden/picture-sampling.json.gz` runs by default. The 306-case
matrix compares 542,400 pixel values exactly: all 16 movement selectors, two
orientations, three string grids, two source sizes, direction octant boundaries,
speed/movement rate, center stop, custom color, level/brightness and offset curves,
plus 64 location cases with zero and mixed-sign coordinate offsets. The Rust test
supplies the same opaque pixels and calls the actual Picture render/sampling
methods through its pixel framework. No Picture algorithm defect was found.

Limits: image decoding, resizing, grayscale, GIF frame scheduling, file/resource
loading, transparency and whole-effect lifecycle are outside this test. The Resize
selector samples an already-sized image; it does not verify resize interpolation.
Arbitrary partially-transparent bytes cannot serve as an equal cross-platform
input because the port bitmap stores premultiplied pixels; without running original
GDI scaling, its final decoded-byte representation is unverified. The published
matrix therefore uses only opaque input and makes no transparency claim.

Source/dependency hashes are in `source-manifest.sha256`. All generated writes stay
under `target/golden`; no profile, hardware, network or native drawing is needed.
