# Shapes geometry differential oracle

This compiles the unchanged original `Shapes/GeometricShapes.cs`. It calls every
geometric shape builder and captures vertices/dimensions using value-only SVG
containers in `SvgValues.cs`. Those containers only store values assigned by the
original code; they do not replace any geometry, numeric conversion or drawing
algorithm. System.Drawing.PointF is the real .NET value type; GDI+ is never used.

```sh
tools/golden/shapes/build.sh run
source env.sh
SHAPES_GOLDEN="$PWD/target/golden/shapes/data/shapes-geometry.json.gz" \
  cargo test -p fx-pixel-graphics original_shape_geometry -- --nocapture
```

The normal test uses checked-in `tests/golden/shapes-geometry.json.gz`.
5,393 cases / 390,833 coordinate values cover circles, ellipses, rectangles,
triangles, polygons, intersecting/nonintersecting/north stars, crosses and arrows.
Seven radii (1–100), six aspect ratios, 3–30 sides/points, all positive skip counts
below point count, five inside distances and all 100 cross ratios are included.
Some combinations exceed editor sliders; no claim covers arbitrary malformed input.
Polygon vertices compare exactly; other decimal dimensions must encode the same
Float32 value, allowing harmless differences in decimal spelling.

The matrix caught 34 one-pixel star vertex differences. The original squares and
adds PointF coordinates in Float32 before Math.Sqrt; the port promoted those
operands too early. Keeping those operations in Float32 now matches all cases.

Limits: this does not run the complete Shapes effect, SVG parsing/rasterization,
embedded art, animation, marks, color/fade application or frame sampling. Those
still need separate full-render verification. Source/harness hashes are recorded
in `source-manifest.sha256`. All build/generated writes stay under `target/golden`;
there are no profiles, hardware, network or native drawing dependencies.
