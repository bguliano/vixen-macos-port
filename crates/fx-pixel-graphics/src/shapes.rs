//! Port of VixenModules.Effect.Shapes. Shapes (geometric, Christmas, Halloween or a user SVG file) fly around the
//! element, spin, grow or shrink and fade. The C# draws `Svg.SvgDocument`s into a bitmap and samples the bitmap.
//! Here each shape is a small mutable SVG tree (`svgdoc`) rendered with resvg without anti-aliasing, which is what
//! the C# asks for with `SvgShapeRendering.CrispEdges`.

use crate::generated::shapes as gen;
use crate::shape_assets::embedded_shape;
use crate::svgdoc::{hex, parse_color, walk_descendants, RenderCache, SvgNode};
use vixen_effects::gfx::Graphics;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    param_changed: Some(param_changed),
    param_changed_ctx: Some(param_changed_ctx),
    supports_location: true,
    ported: true,
    ..gen::BASE
};

const SVG_VIEW_BOX_SIZE: i32 = 200;
/// The C# radius constant used by the geometric shape builders (the SvgDocument is 200 wide).
const RADIUS: i32 = 100;

/// `UpdateAllAttributes`: `UpdatePositionAttribute`, `UpdateShapeTypeAttribute`, `UpdateShapeModeAttributes`.
fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    let mode = p.enum_name("ShapeMode");
    let list = p.enum_name("ShapeList");
    let geo = p.enum_name("GeometricShapesList");
    let xmas = p.enum_name("ChristmasShapesList");
    let hallo = p.enum_name("HalloweenShapesList");
    let shape_type = p.enum_name("ShapeType");
    let scale_to_grid = p.boolean("ScaleToGrid");
    let remove_shape = p.boolean("RemoveShape");
    let random_size = p.boolean("RandomSize");
    let random_angle = p.boolean("RandomAngle");
    let fill = p.boolean("Fill");
    let stroke_fill = p.boolean("StrokeFill");
    let fade = p.enum_name("FadeType");
    let _ = fade;

    let is_geo = list == "GeometricShapes";
    let is_file = list == "File";
    let not_grid = !scale_to_grid;

    // UpdateShapeModeAttributes
    v.set("MarkCollectionId", mode != "None");
    v.set("ShapeCount", mode == "RemoveShapesMarkCollection");
    v.set("ScaleToGrid", mode == "None");
    // UpdateShapeTypeAttribute
    v.set("GeometricShapesList", is_geo);
    v.set("ChristmasShapesList", list == "ChristmasShapes");
    v.set("HalloweenShapesList", list == "HalloweenShapes");
    v.set("FileName", is_file);
    v.set("PolygonSides", is_geo && geo == "Polygon");
    v.set("SkipPoints", is_geo && geo == "Star");
    v.set("StarInsideSize", is_geo && geo == "NonIntersectingStar");
    v.set("StarPoints", is_geo && geo == "Star");
    v.set("NonIntersectingStarPoints", is_geo && geo == "NonIntersectingStar");
    v.set("ShapeSizeRatio", not_grid && is_geo && (geo == "Rectangle" || geo == "Ellipse"));
    v.set("CrossSizeRatio", not_grid && is_geo && geo == "Cross");
    v.set("FirstFillColors", !is_file && fill);
    // `&&` binds tighter than `||`, as written in the C#.
    v.set(
        "SecondFillColors",
        (!is_file && fill && !is_geo && (list == "ChristmasShapes" && xmas != "SnowFlake" && xmas != "SnowFlake2"))
            || (list == "HalloweenShapes" && hallo != "Web"),
    );
    v.set("RoundedCorner", is_geo);
    v.set("OutlineColors", stroke_fill);
    v.set("Fill", !is_file);
    v.set("StrokeFill", is_file);
    v.set("SizeCurve", not_grid && !remove_shape && !random_size);
    v.set("MaxSizeCurve", not_grid && remove_shape);
    v.set("CenterSpeedCurve", not_grid && shape_type != "None");
    v.set("SpeedVariationCurve", not_grid && shape_type != "None");
    v.set("YOffsetCurve", not_grid && shape_type == "None");
    v.set("XOffsetCurve", not_grid && shape_type == "None");
    v.set("ShapeCountCurve", not_grid && mode != "RemoveShapesMarkCollection");
    v.set("CenterSizeSpeedCurve", not_grid && (remove_shape || random_size));
    v.set("SizeSpeedVariationCurve", not_grid && (remove_shape || random_size));
    v.set("CenterAngleSpeedCurve", not_grid && random_angle);
    v.set("AngleSpeedVariationCurve", not_grid && random_angle);
    v.set("AngleCurve", not_grid && !random_angle);
    v.set("ShapeType", not_grid);
    v.set("ShapeOutLineCurve", stroke_fill);
    v.set("ShapeOutLineSpaceCurve", stroke_fill);
    v.set("StrokeWidth", stroke_fill);
    v.set("RandomAngle", not_grid);
    v.set("FadeType", !is_file && not_grid);
    v.set("SizeVariationCurve", not_grid && (remove_shape || random_size));
    v.set("CenterSizeCurve", not_grid && (remove_shape || random_size));
    v.set("SizeMode", not_grid && remove_shape);
    v.set("RemoveShape", not_grid);
    v.set("RandomSize", not_grid && !remove_shape);
    // The constructor calls EnableTargetPositioning(true, true).
    v.set("TargetPositioning", true);
    v.set("StringOrientation", p.enum_name("TargetPositioning") == "Strings");
}

/// Setting a mark-based shape source turns ScaleToGrid off.
fn param_changed(name: &str, p: &mut Params) {
    if name == "ShapeMode" && p.enum_name("ShapeMode") != "None" {
        p.set("ScaleToGrid", ParamValue::Bool(false));
    }
}

/// The `ShapeMode` setter calls `ActivateMarkCollectionSelections`.
fn param_changed_ctx(c: &ParamChange, p: &mut Params) {
    if matches!(c.name, MARK_COLLECTIONS | "ShapeMode") {
        normalize_mark_selection(p, c.info, p.enum_name("ShapeMode") != "None", None, true);
    }
}

#[derive(Copy, Clone, PartialEq)]
enum ShapeMove {
    None,
    Bounce,
    Wrap,
}

#[derive(Copy, Clone, PartialEq)]
enum ShapeFade {
    None,
    In,
    Out,
    Random,
}

#[derive(Copy, Clone, PartialEq)]
enum ShapeSource {
    Geometric,
    Christmas,
    Halloween,
    File,
}

struct Shape {
    id: u32,
    location_x: f64,
    location_y: f64,
    velocity_x: f64,
    velocity_y: f64,
    first_fill_index: usize,
    second_fill_index: usize,
    stroke_color_index: usize,
    size: f64,
    rotate_angle: i32,
    rotate_cw: i32,
    doc: SvgNode,
    /// The (possibly 1.42 times expanded) `ViewBox`.
    view_box: (f32, f32, f32, f32),
    location_ratio1: f32,
    fade: f32,
    /// `m.FadeType`: null until set.
    fade_type: ShapeFade,
    fade_step: f32,
    size_in: bool,
    cache: RenderCache,
}

struct Shapes {
    // Parameters
    mode: u8, // 0 None, 1 Create, 2 Remove
    source: ShapeSource,
    geometric: String,
    christmas: String,
    halloween: String,
    shape_type: ShapeMove,
    fade_type: ShapeFade,
    size_mode: String,
    file_name: String,
    mark_collection_id: Uuid,
    shape_count_param: i32,
    polygon_sides: i32,
    skip_points: i32,
    star_points: i32,
    non_intersecting_star_points: i32,
    star_inside_size: i32,
    shape_size_ratio: i32,
    cross_size_ratio: i32,
    stroke_width: i32,
    fill: bool,
    stroke_fill: bool,
    rounded_corner: bool,
    random_size: bool,
    random_angle: bool,
    remove_shape: bool,
    scale_to_grid: bool,
    locations: bool,
    first_fill: Vec<ColorGradient>,
    second_fill: Vec<ColorGradient>,
    outline: Vec<ColorGradient>,
    level_curve: Curve,
    shape_count_curve: Curve,
    center_angle_speed_curve: Curve,
    angle_speed_variation_curve: Curve,
    center_size_speed_curve: Curve,
    size_speed_variation_curve: Curve,
    center_speed_curve: Curve,
    speed_variation_curve: Curve,
    size_curve: Curve,
    size_variation_curve: Curve,
    center_size_curve: Curve,
    x_offset_curve: Curve,
    y_offset_curve: Curve,
    angle_curve: Curve,
    outline_curve: Curve,
    outline_space_curve: Curve,
    // Per render
    module_dir: Option<std::path::PathBuf>,
    shape_frames: Vec<i32>,
    string_count: i32,
    frame_time: i32,
    shapes: Vec<Shape>,
    remove_shapes: Vec<u32>,
    next_id: u32,
    shapes_count: i32,
    interval_pos_factor: f64,
    center_speed: f64,
    speed_variation: f64,
    min_speed: f64,
    max_speed: f64,
    angle: i32,
    min_buffer: i32,
    max_buffer: i32,
    scale_shape_width: f32,
    scale_shape_height: f32,
    total_frames: i32,
    file_path: Option<std::path::PathBuf>,
    rand: Rand,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let mut e = Shapes {
        mode: match p.enum_name("ShapeMode") {
            "CreateShapesMarkCollection" => 1,
            "RemoveShapesMarkCollection" => 2,
            _ => 0,
        },
        source: match p.enum_name("ShapeList") {
            "ChristmasShapes" => ShapeSource::Christmas,
            "HalloweenShapes" => ShapeSource::Halloween,
            "File" => ShapeSource::File,
            _ => ShapeSource::Geometric,
        },
        geometric: p.enum_name("GeometricShapesList").to_string(),
        christmas: p.enum_name("ChristmasShapesList").to_string(),
        halloween: p.enum_name("HalloweenShapesList").to_string(),
        shape_type: match p.enum_name("ShapeType") {
            "Bounce" => ShapeMove::Bounce,
            "Wrap" => ShapeMove::Wrap,
            _ => ShapeMove::None,
        },
        fade_type: match p.enum_name("FadeType") {
            "In" => ShapeFade::In,
            "Out" => ShapeFade::Out,
            "Random" => ShapeFade::Random,
            _ => ShapeFade::None,
        },
        size_mode: p.enum_name("SizeMode").to_string(),
        file_name: p.string("FileName").to_string(),
        mark_collection_id: p.guid("MarkCollectionId"),
        shape_count_param: p.int("ShapeCount"),
        polygon_sides: p.int("PolygonSides"),
        skip_points: p.int("SkipPoints"),
        star_points: p.int("StarPoints"),
        non_intersecting_star_points: p.int("NonIntersectingStarPoints"),
        star_inside_size: p.int("StarInsideSize"),
        shape_size_ratio: p.int("ShapeSizeRatio"),
        cross_size_ratio: p.int("CrossSizeRatio"),
        stroke_width: p.int("StrokeWidth"),
        fill: p.boolean("Fill"),
        stroke_fill: p.boolean("StrokeFill"),
        rounded_corner: p.boolean("RoundedCorner"),
        random_size: p.boolean("RandomSize"),
        random_angle: p.boolean("RandomAngle"),
        remove_shape: p.boolean("RemoveShape"),
        scale_to_grid: p.boolean("ScaleToGrid"),
        locations: TargetPositioning::from_name(p.enum_name("TargetPositioning")) == TargetPositioning::Locations,
        first_fill: p.gradients("FirstFillColors"),
        second_fill: p.gradients("SecondFillColors"),
        outline: p.gradients("OutlineColors"),
        level_curve: p.curve("LevelCurve"),
        shape_count_curve: p.curve("ShapeCountCurve"),
        center_angle_speed_curve: p.curve("CenterAngleSpeedCurve"),
        angle_speed_variation_curve: p.curve("AngleSpeedVariationCurve"),
        center_size_speed_curve: p.curve("CenterSizeSpeedCurve"),
        size_speed_variation_curve: p.curve("SizeSpeedVariationCurve"),
        center_speed_curve: p.curve("CenterSpeedCurve"),
        speed_variation_curve: p.curve("SpeedVariationCurve"),
        size_curve: p.curve("SizeCurve"),
        size_variation_curve: p.curve("SizeVariationCurve"),
        center_size_curve: p.curve("CenterSizeCurve"),
        x_offset_curve: p.curve("XOffsetCurve"),
        y_offset_curve: p.curve("YOffsetCurve"),
        angle_curve: p.curve("AngleCurve"),
        outline_curve: p.curve("ShapeOutLineCurve"),
        outline_space_curve: p.curve("ShapeOutLineSpaceCurve"),
        module_dir: ctx.module_data_root.map(|r| r.join("Shapes")),
        shape_frames: Vec::new(),
        string_count: 0,
        frame_time: ctx.frame_ms,
        shapes: Vec::new(),
        remove_shapes: Vec::new(),
        next_id: 0,
        shapes_count: 0,
        interval_pos_factor: 0.0,
        center_speed: 0.0,
        speed_variation: 0.0,
        min_speed: 0.0,
        max_speed: 0.0,
        angle: 0,
        min_buffer: 0,
        max_buffer: 0,
        scale_shape_width: 0.0,
        scale_shape_height: 0.0,
        total_frames: 0,
        file_path: None,
        rand: Rand::default(),
    };
    render_pixel_effect(ctx, p, &mut e)
}

// ---- Geometric shape builders (GeometricShapes.cs) ----

fn pt(points: &[(i32, i32)]) -> String {
    let mut s = String::new();
    for (i, (x, y)) in points.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        s.push_str(&format!("{x},{y}"));
    }
    s
}

fn svg_circle(radius: i32) -> SvgNode {
    SvgNode::new("circle").with_attr("cx", &RADIUS.to_string()).with_attr("cy", &RADIUS.to_string()).with_attr("r", &radius.to_string())
}

fn svg_ellipse(radius: i32, ratio: f32) -> SvgNode {
    SvgNode::new("ellipse")
        .with_attr("cx", &RADIUS.to_string())
        .with_attr("cy", &RADIUS.to_string())
        .with_attr("ry", &(radius as f32 * ratio).to_string())
        .with_attr("rx", &radius.to_string())
}

fn svg_rectangle(size: i32, ratio: f32) -> SvgNode {
    let width = size;
    let height = (size as f32 * ratio) as i32;
    SvgNode::new("rect")
        .with_attr("x", &(RADIUS - width).to_string())
        .with_attr("y", &(RADIUS - height).to_string())
        .with_attr("width", &(width * 2).to_string())
        .with_attr("height", &(height * 2).to_string())
}

fn polygon(points: &[(f32, f32)]) -> SvgNode {
    // `ConvertFloatSVGPoints` truncates each coordinate.
    let ints: Vec<(i32, i32)> = points.iter().map(|(x, y)| (*x as i32, *y as i32)).collect();
    SvgNode::new("polygon").with_attr("points", &pt(&ints))
}

fn polygon_from_doubles(vals: &[f64]) -> SvgNode {
    let ints: Vec<i32> = vals.iter().map(|v| *v as i32).collect();
    let pairs: Vec<(i32, i32)> = ints.chunks(2).filter(|c| c.len() == 2).map(|c| (c[0], c[1])).collect();
    SvgNode::new("polygon").with_attr("points", &pt(&pairs))
}

fn degrees_to_xy(degrees: f32, radius: f32) -> (f32, f32) {
    let radians = degrees as f64 * std::f64::consts::PI / 180.0;
    let x = (radians.cos() as f32) * radius + RADIUS as f32;
    let y = ((-radians).sin() as f32) * radius + RADIUS as f32;
    (x, y)
}

fn calculate_vertices(radius: i32, sides: i32) -> Vec<(f32, f32)> {
    let mut points = Vec::new();
    if sides <= 0 {
        return points;
    }
    let step: f32 = 360.0f32 / sides as f32;
    let mut angle: f32 = 0.0;
    let mut i: f64 = 0.0;
    // Deliberately mirrors the C# loop, where `step` is a float added to a double counter.
    while i < 360.0 && points.len() < 4096 {
        points.push(degrees_to_xy(angle, radius as f32));
        angle += step;
        i += step as f64;
    }
    points
}

fn make_star_points(radius: i32, num_points: i32, skip: i32) -> Vec<(f32, f32)> {
    let concave = calculate_concave_radius(num_points, skip);
    let n = num_points.max(0) as usize;
    let mut result = vec![(0.0f32, 0.0f32); 2 * n];
    let mut theta = -std::f64::consts::PI / 2.0;
    let dtheta = std::f64::consts::PI / num_points.max(1) as f64;
    let r = RADIUS as f64;
    for i in 0..n {
        result[2 * i] = ((r + radius as f64 * theta.cos()) as f32, (r + radius as f64 * theta.sin()) as f32);
        theta += dtheta;
        result[2 * i + 1] = ((r + radius as f64 * theta.cos() * concave) as f32, (r + radius as f64 * theta.sin() * concave) as f32);
        theta += dtheta;
    }
    result
}

fn calculate_concave_radius(num_points: i32, skip: i32) -> f64 {
    if num_points < 5 {
        return 0.33f32 as f64;
    }
    let dtheta = 2.0 * std::f64::consts::PI / num_points as f64;
    let theta00 = -std::f64::consts::PI / 2.0;
    let theta01 = theta00 + dtheta * skip as f64;
    let theta10 = theta00 + dtheta;
    let theta11 = theta10 - dtheta * skip as f64;
    let p = |t: f64| (t.cos() as f32, t.sin() as f32);
    let (i, _) = find_intersection(p(theta00), p(theta01), p(theta10), p(theta11));
    // PointF multiplication and addition happen in single precision before Math.Sqrt.
    ((i.0 * i.0 + i.1 * i.1) as f64).sqrt()
}

/// `FindIntersection`: only the intersection of the two lines is used. Returns NaN when they are parallel.
fn find_intersection(p1: (f32, f32), p2: (f32, f32), p3: (f32, f32), p4: (f32, f32)) -> ((f32, f32), bool) {
    let dx12 = p2.0 - p1.0;
    let dy12 = p2.1 - p1.1;
    let dx34 = p4.0 - p3.0;
    let dy34 = p4.1 - p3.1;
    let denominator = dy12 * dx34 - dx12 * dy34;
    let t1 = ((p1.0 - p3.0) * dy34 + (p3.1 - p1.1) * dx34) / denominator;
    if t1.is_infinite() {
        return ((f32::NAN, f32::NAN), false);
    }
    ((p1.0 + dx12 * t1, p1.1 + dy12 * t1), true)
}

fn non_intersecting_star_points(radius: i32, num_points: i32, inside_distance: i32, north_star: bool) -> Vec<(f32, f32)> {
    let n = num_points.max(0) as usize;
    let mut pts = vec![(0.0f32, 0.0f32); 2 * n];
    let rad2 = radius as f64 * (inside_distance as f64 / (radius as f64 * 2.0));
    let mut theta = -std::f64::consts::PI / 2.0;
    let dtheta = std::f64::consts::PI / num_points.max(1) as f64;
    let mut new_radius = radius;
    let r = RADIUS as f64;
    let mut i = 0usize;
    while i < 2 * n {
        if north_star {
            new_radius = match i {
                2 | 6 | 10 | 14 => (radius as f64 * 0.5) as i32,
                4 | 12 => (radius as f64 * 0.75) as i32,
                _ => radius,
            };
        }
        pts[i] = ((r + new_radius as f64 * theta.cos()) as f32, (r + new_radius as f64 * theta.sin()) as f32);
        theta += dtheta;
        pts[i + 1] = ((r + rad2 * theta.cos()) as f32, (r + rad2 * theta.sin()) as f32);
        theta += dtheta;
        i += 2;
    }
    pts
}

fn svg_cross(ratio: f32) -> SvgNode {
    let r = ratio as f64;
    let _ = r;
    let a = (199.0 - 199.0 * ratio) as f64;
    let b = (199.0 * ratio) as f64;
    let pts = [70.0, 130.0, a, 130.0, a, 70.0, 70.0, 70.0, 70.0, 1.0, 130.0, 1.0, 130.0, 70.0, b, 70.0, b, 130.0, 130.0, 130.0, 130.0, 199.0, 70.0, 199.0, 70.0, 199.0, 70.0, 130.0];
    polygon_from_doubles(&pts)
}

fn svg_arrow() -> SvgNode {
    polygon_from_doubles(&[1.0, 70.0, 110.0, 70.0, 110.0, 1.0, 199.0, 100.0, 110.0, 199.0, 110.0, 130.0, 1.0, 130.0, 1.0, 70.0])
}

fn geometric_root() -> SvgNode {
    let mut n = SvgNode::new("svg");
    n.set_attr("viewBox", &format!("0 0 {SVG_VIEW_BOX_SIZE} {SVG_VIEW_BOX_SIZE}"));
    n
}

fn embedded_doc(key: &str) -> Option<SvgNode> {
    SvgNode::parse(embedded_shape(key)?)
}

// ---- Calculations ----

impl Shapes {
    fn calculate_shape_count(&self, pos: f64) -> i32 {
        let v = scale_curve_to_value(self.shape_count_curve.get_value(pos), 50.0, 1.0) as i32;
        v.max(1)
    }

    fn calculate_center_angle_speed(&self, pos: f64) -> f64 {
        scale_curve_to_value(self.center_angle_speed_curve.get_value(pos), 45.0, 0.0)
    }

    fn calculate_angle_speed_variation(&self, pos: f64) -> f64 {
        scale_curve_to_value(self.angle_speed_variation_curve.get_value(pos), 45.0, 0.0)
    }

    fn calculate_center_size_speed(&self, pos: f64) -> f64 {
        scale_curve_to_value(self.center_size_speed_curve.get_value(pos), 16.0, 10.0)
    }

    fn calculate_size_speed_variation(&self, pos: f64) -> f64 {
        scale_curve_to_value(self.size_speed_variation_curve.get_value(pos), 4.0, 0.0)
    }

    fn calculate_center_speed(&self, pos: f64) -> f64 {
        scale_curve_to_value(self.center_speed_curve.get_value(pos), self.max_buffer as f64 / 10.0, 0.0) * self.frame_time as f64 / 50.0
    }

    fn calculate_speed_variation(&self, pos: f64) -> f64 {
        scale_curve_to_value(self.speed_variation_curve.get_value(pos), self.max_buffer as f64 / 10.0, 0.0) * self.frame_time as f64 / 50.0
    }

    /// `(int)(_minBuffer * 2 / locationRatio1)`: the largest size, in shape units.
    fn max_size_units(&self, location_ratio1: f32) -> f64 {
        ((self.min_buffer * 2) as f32 / location_ratio1) as i32 as f64
    }

    fn calculate_size(&self, pos: f64, lr1: f32) -> i32 {
        let v = scale_curve_to_value(self.size_curve.get_value(pos), self.max_size_units(lr1), 1.0) as i32;
        v.max(1)
    }

    fn calculate_size_variation(&self, pos: f64, lr1: f32) -> i32 {
        let v = scale_curve_to_value(self.size_variation_curve.get_value(pos), self.max_size_units(lr1), 1.0) as i32;
        v.max(1)
    }

    fn calculate_center_size(&self, pos: f64, lr1: f32) -> i32 {
        let v = scale_curve_to_value(self.center_size_curve.get_value(pos), self.max_size_units(lr1), 4.0) as i32;
        v.max(1)
    }

    fn calculate_x_offset(&self, pos: f64, width: i32) -> i32 {
        scale_curve_to_value(self.x_offset_curve.get_value(pos), width as f64, -(width as f64)) as i32
    }

    fn calculate_y_offset(&self, pos: f64, height: i32) -> i32 {
        scale_curve_to_value(self.y_offset_curve.get_value(pos), -(height as f64), height as f64) as i32
    }

    fn calculate_angle(&self, pos: f64) -> i32 {
        scale_curve_to_value(self.angle_curve.get_value(pos), 360.0, 0.0) as i32
    }

    fn calculate_outline(&self, pos: f64) -> i32 {
        scale_curve_to_value(self.outline_curve.get_value(pos), 100.0, 0.0) as i32
    }

    fn calculate_outline_space(&self, pos: f64) -> i32 {
        scale_curve_to_value(self.outline_space_curve.get_value(pos), 100.0, 0.0) as i32
    }

    // ---- CreateShapes ----

    fn create_shape(&mut self, buffer_ht: i32, buffer_wi: i32) {
        let mut location_x = self.rand.next_range(0, buffer_wi - 1) as f64;
        let mut location_y = self.rand.next_range(0, buffer_ht - 1) as f64;

        let speed = self.rand.next_double() * (self.max_speed - self.min_speed) + self.min_speed;
        let mut vx = self.rand.next_double() * speed;
        let mut vy = self.rand.next_double() * speed;
        if self.rand.next_range(0, 2) == 0 {
            vx = -vx;
        }
        if self.rand.next_range(0, 2) == 0 {
            vy = -vy;
        }
        let (velocity_x, velocity_y) = if self.shape_type != ShapeMove::None { (vx, vy) } else { (0.0, 0.0) };

        self.angle = self.calculate_angle(self.interval_pos_factor);

        let mut doc: SvgNode;
        match self.source {
            ShapeSource::Geometric => {
                let members = gen::ENUM_GeometricShapesList.members;
                let shape = if self.geometric == "Random" {
                    members[self.rand.next_range(1, members.len() as i32) as usize].name.to_string()
                } else {
                    self.geometric.clone()
                };
                let radius = SVG_VIEW_BOX_SIZE / 2 - (self.stroke_width + 1);
                doc = geometric_root();
                match shape.as_str() {
                    "Square" => doc.children.push(svg_rectangle(radius, 1.0)),
                    "Arrow" => doc.children.push(svg_arrow()),
                    "Triangle" => doc.children.push(polygon(&calculate_vertices(radius, 3))),
                    "Circle" => doc.children.push(svg_circle(radius)),
                    "Rectangle" => doc.children.push(svg_rectangle(radius, self.shape_size_ratio as f32 / 100.0)),
                    "Ellipse" => doc.children.push(svg_ellipse(radius, self.shape_size_ratio as f32 / 100.0)),
                    "Cross" => doc.children.push(svg_cross(self.cross_size_ratio as f32 / 100.0)),
                    "Star" => doc.children.push(polygon(&make_star_points(radius, self.star_points, self.skip_points))),
                    "ConcaveStar" | "Heart" => match embedded_doc(&format!("Geometric.{shape}")) {
                        Some(d) => doc = d,
                        None => return,
                    },
                    "NonIntersectingStar" => doc.children.push(polygon(&non_intersecting_star_points(
                        radius,
                        self.non_intersecting_star_points,
                        self.star_inside_size,
                        false,
                    ))),
                    "NorthStar" => doc.children.push(polygon(&non_intersecting_star_points(radius, 8, 42, true))),
                    "Polygon" => doc.children.push(polygon(&calculate_vertices(radius, self.polygon_sides))),
                    _ => {}
                }
                match doc.children.first_mut() {
                    Some(c) => c.set_attr("id", "firstFillColor"),
                    None => return,
                }
            }
            ShapeSource::Christmas | ShapeSource::Halloween => {
                let (members, chosen, folder) = if self.source == ShapeSource::Christmas {
                    (gen::ENUM_ChristmasShapesList.members, &self.christmas, "Christmas")
                } else {
                    (gen::ENUM_HalloweenShapesList.members, &self.halloween, "Halloween")
                };
                let name = if chosen == "Random" {
                    members[self.rand.next_range(1, members.len() as i32) as usize].name.to_string()
                } else {
                    chosen.clone()
                };
                // A missing resource throws in the C#; nothing is created.
                match embedded_doc(&format!("{folder}.{name}")) {
                    Some(d) => doc = d,
                    None => return,
                }
            }
            ShapeSource::File => {
                let Some(path) = self.file_path.clone() else { return };
                let Ok(text) = std::fs::read_to_string(&path) else { return };
                match SvgNode::parse(&text) {
                    Some(d) => doc = d,
                    None => return,
                }
            }
        }

        // `if ((int)ViewBox.Width == 0) ViewBox = new SvgViewBox(0, 0, Width, Height)`
        let mut vb = match doc.view_box() {
            Some(v) if v.2 as i32 != 0 => v,
            _ => {
                let (w, h) = match (doc.length_attr("width"), doc.length_attr("height")) {
                    (Some(w), Some(h)) => (w, h),
                    _ => match resvg::usvg::Tree::from_str(&doc.to_svg(), &resvg::usvg::Options::default()) {
                        Ok(t) => (t.size().width(), t.size().height()),
                        Err(_) => return,
                    },
                };
                (0.0, 0.0, w, h)
            }
        };
        if !(vb.2 > 0.0 && vb.3 > 0.0) {
            return;
        }

        let string_count = self.string_count.max(1);
        let location_ratio1;
        if !self.scale_to_grid {
            // Expand the view box so a rotating shape never clips, and size everything from the effect's setup.
            vb = (-((vb.2 * 1.42f32 - vb.2) / 2.0), -((vb.3 * 1.42f32 - vb.3) / 2.0), vb.2 * 1.42f32, vb.3 * 1.42f32);
            let location_ratio = string_count as f32 / vb.3 * 2.0 * (self.min_buffer as f32 / string_count as f32);
            location_ratio1 = location_ratio * location_ratio;
        } else {
            location_ratio1 = 1.0;
            location_x = 0.0;
            location_y = 0.0;
        }
        doc.set_view_box(vb);
        doc.set_attr("width", &vb.2.to_string());
        doc.set_attr("height", &vb.3.to_string());
        doc.set_attr("preserveAspectRatio", "none");
        // ShapeRendering = CrispEdges, which switches anti-aliasing off.
        doc.set_attr("shape-rendering", "crispEdges");
        doc.remove_attr("transform");

        if self.rounded_corner && self.source == ShapeSource::Geometric {
            doc.set_attr("stroke-linejoin", "round");
        }

        let center_size = self.calculate_center_size(self.interval_pos_factor, location_ratio1);
        let size_variation = self.calculate_size_variation(self.interval_pos_factor, location_ratio1);
        let mut min_size = center_size - size_variation / 2;
        let max_size = center_size + size_variation / 2;
        if min_size < 1 {
            min_size = 1;
        }

        let size_in = match self.size_mode.as_str() {
            "In" => true,
            "Out" => false,
            _ => self.rand.next_range(0, 2) == 0,
        };

        let (size, fade_step);
        if self.remove_shape && !self.scale_to_grid {
            size = if size_in { max_size as f64 } else { min_size as f64 };
            fade_step = 0.9f32 / (max_size - min_size) as f32;
        } else {
            fade_step = 0.9f32 / self.total_frames as f32;
            size = if self.random_size {
                self.rand.next_range(min_size, max_size) as f64
            } else {
                self.calculate_size(self.interval_pos_factor, location_ratio1) as f64
            };
        }

        if self.shape_type == ShapeMove::Bounce {
            // Keep a new shape inside the grid so it cannot get stuck on an edge.
            let location_offset = vb.2 as f64 / 1.42 * location_ratio1 as f64 * self.scale_shape_width as f64 / 2.0;
            let mut i = 0;
            loop {
                let mut not_created = false;
                location_x = self.rand.next_range(0, buffer_wi - 1) as f64;
                location_y = self.rand.next_range(0, buffer_ht - 1) as f64;
                if location_x - location_offset < 0.0
                    || location_y - location_offset < 0.0
                    || location_x + location_offset >= buffer_wi as f64
                    || location_y + location_offset >= buffer_ht as f64
                {
                    not_created = true;
                }
                i += 1;
                if !(not_created && i < 10000) {
                    break;
                }
            }
        }

        let first_fill_index = self.rand.next_range(0, self.first_fill.len() as i32) as usize;
        let second_fill_index = self.rand.next_range(0, self.second_fill.len() as i32) as usize;
        let stroke_color_index = self.rand.next_range(0, self.outline.len() as i32) as usize;

        let rotate_angle = if self.random_angle { self.rand.next_range(0, 360) } else { 0 };
        let rotate_cw = self.rand.next_range(0, 2);
        let (mut fade, mut fade_type) = (0.0f32, ShapeFade::None);
        if self.source != ShapeSource::File {
            match self.fade_type {
                ShapeFade::In => {
                    fade = 0.05;
                    fade_type = ShapeFade::In;
                }
                ShapeFade::Out => {
                    fade = 1.0;
                    fade_type = ShapeFade::Out;
                }
                ShapeFade::Random => {
                    if self.rand.next_range(0, 2) == 0 {
                        fade_type = ShapeFade::In;
                        fade = 0.05;
                    } else {
                        fade_type = ShapeFade::Out;
                        fade = 1.0;
                    }
                }
                ShapeFade::None => {}
            }
        }

        self.next_id += 1;
        self.shapes.push(Shape {
            id: self.next_id,
            location_x,
            location_y,
            velocity_x,
            velocity_y,
            first_fill_index,
            second_fill_index,
            stroke_color_index,
            size,
            rotate_angle,
            rotate_cw,
            doc,
            view_box: vb,
            location_ratio1,
            fade,
            fade_type,
            fade_step,
            size_in,
            cache: RenderCache::new(),
        });
    }

    // ---- UpdateShapes ----

    fn update_shapes(&mut self, min_angle_speed: f64, max_angle_speed: f64, min_size_speed: f64, max_size_speed: f64, buffer_ht: i32, buffer_wi: i32) {
        // Remove shapes if the shape count has been reduced.
        while (self.shapes_count as i64) < self.shapes.len() as i64 - self.remove_shapes.len() as i64 {
            let i = self.rand.next_range(0, self.shapes.len() as i32) as usize;
            self.shapes.remove(i.min(self.shapes.len() - 1));
        }

        let angle_start_speed = (max_angle_speed - min_angle_speed) + min_angle_speed;
        let size_start_speed = ((max_size_speed - min_size_speed) + min_size_speed) / 10.0;
        let ipf = self.interval_pos_factor;
        // These depend only on the frame, not on the shape.
        let prev_center_speed = self.calculate_center_speed(ipf - 1.0);
        let prev_speed_variation = self.calculate_speed_variation(ipf - 1.0);

        for si in 0..self.shapes.len() {
            if self.shape_type != ShapeMove::None {
                // Adjust shape speeds when the user's speed curve changes.
                if self.center_speed != prev_center_speed {
                    let ratio = self.calculate_center_speed(ipf) / prev_center_speed;
                    self.shapes[si].velocity_x *= ratio;
                    self.shapes[si].velocity_y *= ratio;
                }
                if self.speed_variation != prev_speed_variation {
                    let ratio = self.calculate_speed_variation(ipf) / prev_speed_variation;
                    self.shapes[si].velocity_x *= ratio;
                    self.shapes[si].velocity_y *= ratio;
                }
            }

            let angle_speed = self.rand.next_double() * angle_start_speed;
            let size_speed = self.rand.next_double() * size_start_speed;

            let lr1 = self.shapes[si].location_ratio1;
            let center_size = self.calculate_center_size(ipf, lr1);
            let size_variation = self.calculate_size_variation(ipf, lr1);
            let min_size = center_size - size_variation / 2;
            let max_size = center_size + size_variation / 2;
            let shape_id = self.shapes[si].id;

            if self.remove_shape && !self.scale_to_grid {
                if self.shapes[si].size_in {
                    if size_speed > 1.0 {
                        self.shapes[si].size /= size_speed;
                    }
                } else if size_speed > 1.0 {
                    self.shapes[si].size *= size_speed;
                }
                // Shapes outside the size limits are removed and replaced.
                let s = self.shapes[si].size;
                if s > max_size as f64 || s < min_size as f64 {
                    self.remove_shapes.push(shape_id);
                }
            } else if !self.random_size && !self.remove_shape && !self.scale_to_grid {
                self.shapes[si].size = self.calculate_size(ipf, lr1) as f64;
            }

            // Adjust the angle.
            if self.random_angle {
                let sh = &mut self.shapes[si];
                sh.rotate_angle = if sh.rotate_cw == 1 {
                    if sh.rotate_angle > 358 {
                        0
                    } else {
                        sh.rotate_angle + angle_speed as i32
                    }
                } else if sh.rotate_angle < 2 {
                    360
                } else {
                    sh.rotate_angle - angle_speed as i32
                };
            } else {
                self.shapes[si].rotate_angle = self.angle;
            }

            if self.source != ShapeSource::File {
                let sh = &mut self.shapes[si];
                let remove_mode = self.remove_shape && !self.scale_to_grid;
                // Exponential fading looks better on live lights.
                let denom = (max_size - min_size / 2) as f32;
                let k = (1.0f32 / denom) as f64 * (sh.size - (min_size as f32 / 2.0) as f64);
                match sh.fade_type {
                    ShapeFade::In => {
                        if remove_mode {
                            sh.fade = if sh.size_in { (1.0 - k) * (1.0 - k) } else { k * k } as f32;
                        } else {
                            sh.fade += sh.fade_step;
                        }
                        if sh.fade as f64 > 0.95 {
                            self.remove_shapes.push(shape_id);
                        }
                    }
                    ShapeFade::Out => {
                        if remove_mode {
                            sh.fade = if sh.size_in { k * k } else { (1.0 - k) * (1.0 - k) } as f32;
                        } else {
                            sh.fade -= sh.fade_step;
                        }
                        if (sh.fade as f64) < 0.05 {
                            self.remove_shapes.push(shape_id);
                        }
                    }
                    _ => {}
                }
            }

            // Move the shape.
            if self.shape_type != ShapeMove::None {
                let sh = &mut self.shapes[si];
                sh.location_x += sh.velocity_x;
                sh.location_y += sh.velocity_y;
                let location_offset = sh.view_box.2 as f64 / 1.42 * sh.location_ratio1 as f64 * self.scale_shape_width as f64 / 2.0;
                match self.shape_type {
                    ShapeMove::Bounce => {
                        if sh.location_x - location_offset < 0.0 {
                            sh.velocity_x = -sh.velocity_x;
                        } else if sh.location_x + location_offset >= buffer_wi as f64 {
                            sh.velocity_x = -sh.velocity_x;
                        }
                        if sh.location_y - location_offset < 0.0 {
                            sh.velocity_y = -sh.velocity_y;
                        } else if sh.location_y + location_offset >= buffer_ht as f64 {
                            sh.velocity_y = -sh.velocity_y;
                        }
                    }
                    ShapeMove::Wrap => {
                        if sh.location_x + location_offset < 0.0 {
                            sh.location_x = buffer_wi as f64 + location_offset;
                        }
                        if sh.location_y + location_offset < 0.0 {
                            sh.location_y = buffer_ht as f64 + location_offset;
                        }
                        if sh.location_x - location_offset > buffer_wi as f64 {
                            sh.location_x = 0.0 - location_offset;
                        }
                        if sh.location_y - location_offset > buffer_ht as f64 {
                            sh.location_y = 0.0 - location_offset;
                        }
                    }
                    ShapeMove::None => {}
                }
            }
        }
    }

    /// `RemoveShapes`.
    fn remove_shapes_step(&mut self, frame: i32, buffer_ht: i32, buffer_wi: i32) {
        if self.mode == 2 && self.remove_shapes.is_empty() {
            // Remove the oldest shape when a mark falls on this frame.
            if self.shape_frames.iter().any(|f| frame == *f && !self.shapes.is_empty()) {
                self.shapes.remove(0);
            }
        } else {
            if self.shapes_count as usize >= self.shapes.len() && self.remove_shapes.is_empty() {
                return;
            }
            let pending = std::mem::take(&mut self.remove_shapes);
            for id in pending {
                if let Some(i) = self.shapes.iter().position(|s| s.id == id) {
                    self.shapes.remove(i);
                }
                if (self.shapes.len() as i32) < self.shapes_count {
                    self.create_shape(buffer_ht, buffer_wi);
                }
            }
        }
    }

    // ---- InitialRender ----

    fn initial_render(&mut self, frame: i32, g: &mut Graphics, info: &PixelInfo) {
        let (buffer_ht, buffer_wi) = (info.buffer_ht(), info.buffer_wi());
        self.interval_pos_factor = info.interval_position(frame) * 100.0;
        let ipf = self.interval_pos_factor;
        let center_angle_speed = self.calculate_center_angle_speed(ipf);
        let angle_speed_variation = self.calculate_angle_speed_variation(ipf);
        let center_size_speed = self.calculate_center_size_speed(ipf);
        let size_speed_variation = self.calculate_size_speed_variation(ipf);
        self.center_speed = self.calculate_center_speed(ipf);
        self.speed_variation = self.calculate_speed_variation(ipf);
        self.angle = self.calculate_angle(ipf);
        let pos = info.interval_position(frame);

        let min_angle_speed = center_angle_speed - angle_speed_variation / 2.0;
        let max_angle_speed = center_angle_speed + angle_speed_variation / 2.0;
        let min_size_speed = center_size_speed - size_speed_variation / 2.0;
        let max_size_speed = center_size_speed + size_speed_variation / 2.0;
        self.min_speed = self.center_speed - self.speed_variation / 2.0;
        self.max_speed = self.center_speed + self.speed_variation / 2.0;

        match self.mode {
            0 => {
                let adjusted_pixel_count;
                if self.scale_to_grid {
                    // Scale to grid only needs one shape.
                    self.shapes_count = 1;
                    adjusted_pixel_count = 1;
                } else {
                    self.shapes_count = self.calculate_shape_count(ipf);
                    if frame / 3 >= self.shapes_count || !self.remove_shape {
                        adjusted_pixel_count = self.shapes_count;
                    } else if frame % 3 == 0 {
                        adjusted_pixel_count = 1;
                    } else {
                        adjusted_pixel_count = 0;
                    }
                }
                for _ in 0..adjusted_pixel_count {
                    if (self.shapes.len() as i32) < self.shapes_count {
                        self.create_shape(buffer_ht, buffer_wi);
                    } else {
                        break;
                    }
                }
            }
            1 => {
                let frames = self.shape_frames.clone();
                for f in frames {
                    if frame == f {
                        self.create_shape(buffer_ht, buffer_wi);
                        self.shapes_count = self.shapes.len() as i32;
                    }
                }
            }
            _ => {
                if frame == 0 {
                    self.shapes_count = self.shape_count_param;
                    for _ in 0..self.shape_count_param {
                        if (self.shapes.len() as i32) < self.shape_count_param {
                            self.create_shape(buffer_ht, buffer_wi);
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        self.update_shapes(min_angle_speed, max_angle_speed, min_size_speed, max_size_speed, buffer_ht, buffer_wi);
        self.remove_shapes_step(frame, buffer_ht, buffer_wi);

        // Adjust each shape's SVG and draw it onto the bitmap.
        let outline = self.calculate_outline(ipf);
        let outline_space = self.calculate_outline_space(ipf);
        let shapes_len = self.shapes.len();
        for si in 0..shapes_len {
            let (vbw, vbh, size, lr1, fade) = {
                let s = &self.shapes[si];
                (s.view_box.2, s.view_box.3, s.size, s.location_ratio1, s.fade)
            };
            self.scale_shape_width = (((self.max_buffer * 2) as f32 / vbw) as f64 * (size / (self.max_buffer * 2) as f64)) as f32;
            self.scale_shape_height = (((self.min_buffer * 2) as f32 / vbh) as f64 * (size / (self.min_buffer * 2) as f64)) as f32;
            let (sw, string_count) = (self.scale_shape_width, self.string_count.max(1));
            let min_buffer = self.min_buffer;
            let locations = self.locations;
            let stroke_width = self.stroke_width;
            let fill = self.fill;
            let stroke_fill = self.stroke_fill;
            let fade_active = self.fade_type != ShapeFade::None;
            let is_file = self.source == ShapeSource::File;

            let first = self.first_fill.get(self.shapes[si].first_fill_index).map(|g| g.get_color_at(pos));
            let second = self.second_fill.get(self.shapes[si].second_fill_index).map(|g| g.get_color_at(pos));
            let stroke_color = self.outline.get(self.shapes[si].stroke_color_index).map(|g| g.get_color_at(pos));
            let stroke_value = if locations {
                ((stroke_width as f32 / sw) * min_buffer as f32 / string_count as f32 / lr1) as f64
            } else {
                (stroke_width as f32 / sw) as f64
            };
            let stroke_value_file = ((stroke_width as f32 / sw) * min_buffer as f32 / string_count as f32 / lr1) as f64;
            let dashed = outline < 100 || outline_space > 0;

            let doc = &mut self.shapes[si].doc;
            if !is_file {
                // The C# runs the descendant pass once per root child, so fades on non-colour-filled elements compound.
                let children = doc.children.len();
                for ci in 0..children {
                    if fill {
                        walk_descendants(doc, &mut |d, inherited| {
                            let Some(id) = d.attr("id").map(|s| s.to_string()) else { return };
                            if id.contains("firstFill") {
                                if let Some(c) = first {
                                    d.set_attr("fill", &hex(c));
                                }
                            }
                            if id.contains("secondFill") {
                                if let Some(c) = second {
                                    d.set_attr("fill", &hex(c));
                                }
                            }
                            if fade_active {
                                let current = d.attr("fill").or(inherited).map(|s| s.to_string());
                                if let Some(cur) = current {
                                    if cur != "none" {
                                        if let Some(c) = parse_color(&cur) {
                                            let mut hsv = Hsv::from_color(c);
                                            hsv.set_v(hsv.v * fade as f64);
                                            d.set_attr("fill", &hex(hsv.to_color()));
                                        }
                                    }
                                }
                            }
                        });
                    } else if let Some(first_child) = doc.children.first_mut() {
                        // No fill lets other shapes show through.
                        first_child.set_attr("fill-opacity", "0");
                    }

                    if stroke_fill {
                        let child = &mut doc.children[ci];
                        if let Some(sc) = stroke_color {
                            let mut col = sc;
                            child.set_attr("stroke", &hex(col));
                            child.set_attr("stroke-width", &stroke_value.to_string());
                            if fade_active {
                                let mut hsv = Hsv::from_color(col);
                                hsv.set_v(hsv.v * fade as f64);
                                col = hsv.to_color();
                                child.set_attr("stroke", &hex(col));
                            }
                        }
                        if dashed {
                            child.set_attr("stroke-dasharray", &format!("{outline},{outline_space}"));
                        }
                    }
                }
            } else if stroke_fill {
                if let Some(sc) = stroke_color {
                    doc.set_attr("stroke", &hex(sc));
                    doc.set_attr("stroke-width", &stroke_value_file.to_string());
                }
                if dashed {
                    doc.set_attr("stroke-dasharray", &format!("{outline},{outline_space}"));
                }
            }

            // Where the shape goes.
            let (location_x, location_y);
            if self.shapes.len() == 1 && self.shape_type == ShapeMove::None && !self.scale_to_grid {
                location_x = buffer_wi as f64 / 2.0;
                location_y = buffer_ht as f64 / 2.0;
            } else {
                location_x = self.shapes[si].location_x;
                location_y = self.shapes[si].location_y;
            }
            let (mut x_offset, mut y_offset) = (0, 0);
            if self.shape_type == ShapeMove::None && !self.scale_to_grid {
                x_offset = self.calculate_x_offset(ipf, g.width());
                y_offset = self.calculate_y_offset(ipf, g.height());
            }

            let svg = self.shapes[si].doc.to_svg();
            let angle = self.shapes[si].rotate_angle as f32;
            let sh = &mut self.shapes[si];
            let image = if self.scale_to_grid {
                sh.cache.render(&svg, buffer_wi, buffer_ht, 0.0)
            } else {
                // The shape is `Size * LocationRatio1` pixels across, centred on its location, and turns about its centre.
                let d = (sh.size * sh.location_ratio1 as f64).ceil().clamp(1.0, 2048.0) as i32;
                sh.cache.render(&svg, d, d, angle)
            };
            if let Some(img) = image {
                let (px, py) = if self.scale_to_grid {
                    (0, 0)
                } else {
                    (
                        (location_x + x_offset as f64 - (sh.view_box.2 as f64 * sh.location_ratio1 as f64 * sw as f64 / 2.0).ceil()) as i32,
                        (location_y + y_offset as f64
                            - (sh.view_box.3 as f64 * sh.location_ratio1 as f64 * self.scale_shape_height as f64 / 2.0).ceil())
                            as i32,
                    )
                };
                g.draw_image(&img, px as f32, py as f32, img.width() as f32, img.height() as f32);
            }
        }
    }

    fn pixel_color(&self, bitmap: &vixen_effects::gfx::Bitmap, x: i32, y: i32, ht: i32, level: f64) -> Option<Color> {
        let mut color = bitmap.get_pixel(x, ht - y - 1);
        if color.r != 0 || color.g != 0 || color.b != 0 {
            if level < 1.0 {
                let mut hsv = Hsv::from_color(color);
                hsv.set_v(hsv.v * level);
                color = hsv.to_color();
            }
            return Some(color);
        }
        None
    }
}

impl PixelEffect for Shapes {
    fn begin_group(&mut self, targets: &[NodeId], ctx: &RenderCtx) {
        // `StringCount` is the number of strings of the effect's targets, in either positioning mode.
        self.string_count = ctx.tree.leaf_parents(targets).len() as i32;
        self.shape_frames.clear();
        if self.mode != 0 {
            // `MarksInclusiveOfTime(StartTime, StartTime + TimeSpan)`
            let (start, end) = (ctx.start_time, ctx.start_time + ctx.time_span);
            if let Some(mc) = ctx.mark_collection(self.mark_collection_id) {
                for m in mc.marks_inclusive_of_time(start, end) {
                    // The C# divides by a hard-coded 50 ms.
                    self.shape_frames.push(((m.start_time.total_ms() - start.total_ms()) / 50.0) as i32);
                }
            }
        }
    }

    fn setup_render(&mut self, info: &PixelInfo) {
        self.min_buffer = info.buffer_ht().min(info.buffer_wi());
        self.max_buffer = info.buffer_ht().max(info.buffer_wi());
        self.shapes = Vec::new();
        self.remove_shapes = Vec::new();
        self.shapes_count = 0;
        self.total_frames = info.number_frames();
        self.frame_time = info.frame_time;
        self.file_path = None;
        if self.source == ShapeSource::File && !self.file_name.is_empty() {
            if let Some(dir) = &self.module_dir {
                let path = dir.join(&self.file_name);
                if path.exists() {
                    self.file_path = Some(path);
                } else {
                    self.file_name = String::new();
                }
            }
        }
    }

    fn cleanup_render(&mut self) {
        self.shapes = Vec::new();
        self.remove_shapes = Vec::new();
        self.shape_frames = Vec::new();
    }

    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        let (ht, wi) = (info.buffer_ht(), info.buffer_wi());
        let mut g = Graphics::new(wi, ht);
        g.nearest_neighbor = true;
        self.initial_render(frame, &mut g, info);
        let level = self.level_curve.get_value(info.interval_position(frame) * 100.0) / 100.0;
        let bitmap = g.bitmap();
        for x in 0..wi {
            for y in 0..ht {
                if let Some(c) = self.pixel_color(bitmap, x, y, ht, level) {
                    buf.set_pixel(x, y, c);
                }
            }
        }
    }

    fn render_effect_by_location(&mut self, num_frames: i32, info: &PixelInfo, buf: &mut PixelLocationFrameBuffer) {
        let (ht, wi) = (info.buffer_ht(), info.buffer_wi());
        let locations = buf.element_locations().to_vec();
        for frame in 0..num_frames {
            buf.current_frame = frame;
            let level = self.level_curve.get_value(info.interval_position(frame) * 100.0) / 100.0;
            let mut g = Graphics::new(wi, ht);
            g.nearest_neighbor = true;
            self.initial_render(frame, &mut g, info);
            let bitmap = g.bitmap();
            for l in &locations {
                // Flip and offset the coordinates so the location version acts like the string version.
                let mut y = ((info.buffer_ht_offset() - l.y) + (ht - 1 + info.buffer_ht_offset())).abs();
                y -= info.buffer_ht_offset();
                let x = l.x - info.buffer_wi_offset();
                if let Some(c) = self.pixel_color(bitmap, x, y, ht, level) {
                    buf.set_pixel(l.x, l.y, c);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::*;
    use vixen_effects::testing;

    #[test]
    fn smoke() {
        testing::smoke_test(&DESCRIPTOR);
        testing::assert_produces_light(&DESCRIPTOR);
    }

    #[test]
    fn every_embedded_shape_parses_and_renders() {
        for (members, folder) in [
            (gen::ENUM_ChristmasShapesList.members, "Christmas"),
            (gen::ENUM_HalloweenShapesList.members, "Halloween"),
        ] {
            for m in members.iter().skip(1) {
                let key = format!("{folder}.{}", m.name);
                let mut doc = embedded_doc(&key).unwrap_or_else(|| panic!("missing {key}"));
                let vb = doc.view_box().unwrap_or((0.0, 0.0, 200.0, 200.0));
                doc.set_attr("width", &vb.2.to_string());
                doc.set_attr("height", &vb.3.to_string());
                let img = RenderCache::new().render(&doc.to_svg(), 40, 40, 0.0).unwrap_or_else(|| panic!("{key} did not render"));
                let lit = (0..40).flat_map(|y| (0..40).map(move |x| (x, y))).filter(|(x, y)| img.get_pixel(*x, *y).a > 0).count();
                assert!(lit > 0, "{key} drew nothing");
            }
        }
    }

    #[test]
    fn scale_to_grid_square_fills_the_element() {
        // A filled Square scaled to the grid covers the whole buffer in the first fill colour (blue).
        let mut tree = ElementTree::new();
        let m = testing::grid(&mut tree, "M", 6, 6);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[m], 200, |p| {
            p.set("ScaleToGrid", ParamValue::Bool(true));
            p.set("Fill", ParamValue::Bool(true));
            p.set("StrokeFill", ParamValue::Bool(false));
            p.set("ShapeType", ParamValue::Enum("None".into()));
        });
        assert_eq!(color_at(&tree, m, &out, 3, 3, 0), (0, 0, 255));
        assert_eq!(lit_count(&tree, m, &out, 0), 36);
    }

    #[test]
    fn polygon_vertex_count_follows_the_float_loop() {
        // Seven sides: 360/7 as a float summed seven times stays just under 360, so the C# loop makes 8 points.
        assert_eq!(calculate_vertices(90, 7).len(), 8);
        assert_eq!(calculate_vertices(90, 6).len(), 6);
        assert_eq!(calculate_vertices(90, 3).len(), 3);
    }

    #[test]
    fn mark_mode_visibility() {
        let mut p = DESCRIPTOR.new_params();
        p.set("ShapeMode", ParamValue::Enum("RemoveShapesMarkCollection".into()));
        param_changed("ShapeMode", &mut p);
        let vis: Vec<_> = DESCRIPTOR.visible_params(&p).into_iter().map(|i| DESCRIPTOR.params[i].name).collect();
        assert!(vis.contains(&"ShapeCount") && vis.contains(&"MarkCollectionId"));
        assert!(!vis.contains(&"ScaleToGrid"));
        assert!(!p.boolean("ScaleToGrid"));
    }
}

#[cfg(test)]
#[path = "shapes_golden.rs"]
mod golden;
