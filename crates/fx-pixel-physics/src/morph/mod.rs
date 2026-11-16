//! Port of VixenModules.Effect.Morph: shapes (polygons, lines and ellipses) that wipe, sit still, repeat in a
//! pattern, or move from one key shape to the next over the effect's time line.
//!
//! Every shape lives in a virtual buffer, the display element plus `Margin` percent on each side. Shapes are stored
//! with y going down, and the frame buffer has y going up, so everything flips on the way in.

mod gdi;
pub mod polygon;
mod wipe;

use crate::generated::morph as gen;
use gdi::EllipseDraw;
use polygon::{Ellipse, PolygonPoint, Shape};
use vixen_effects::prelude::*;
use wipe::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    param_changed: Some(param_changed),
    param_changed_ctx: Some(param_changed_ctx),
    targets_changed: Some(targets_changed),
    supports_location: true,
    ported: true,
    ..gen::BASE
};

/// `MaxAcceleration`.
const MAX_ACCELERATION: f64 = 10.0;

/// The largest repeat count, repeat skip or head length this port will work with.
const MAX_SETTING: i32 = 10_000;

/// The largest margin this port will work with. The editor stops at 100.
const MAX_MARGIN: i32 = 1_000;

/// The most pixels it will allocate for a virtual buffer.
const MAX_VIRTUAL_PIXELS: i64 = 1 << 26;

/// `EnableTargetPositioning(true, true)`, `UpdateStringOrientationAttributes` and `UpdatePolygonTypeAttributes`,
/// then, in pattern mode, `UpdatePolygonFillTypeAttributes`. The C# applies the last of those only when the fill
/// type is set, so after a reload it forgets. This function has no such history and always applies it.
fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    v.set("TargetPositioning", true);
    v.set("StringOrientation", p.enum_name("TargetPositioning") == "Strings");

    let polygon_type = p.enum_name("PolygonType");
    let pattern = polygon_type == "Pattern";
    let time_based = polygon_type == "TimeBased";
    v.set("RepeatCount", pattern);
    v.set("RepeatSkip", pattern);
    v.set("Stagger", pattern);
    v.set("FillColor", time_based);
    v.set("MorphPolygonData", polygon_type == "FreeForm");
    v.set("RepeatDirection", pattern);
    v.set("HeadColor", pattern);
    v.set("TailColor", pattern);
    v.set("HeadDuration", pattern);
    v.set("HeadLength", pattern);
    v.set("Acceleration", pattern);
    v.set("FillPolygon", time_based);
    v.set("FillType", pattern);
    v.set("HeadBrightness", pattern);
    v.set("TailBrightness", pattern);
    v.set("FillBrightness", time_based);

    if pattern {
        let wipe = p.enum_name("FillType") == "Wipe";
        v.set("HeadLength", wipe);
        v.set("HeadDuration", wipe);
        v.set("Acceleration", wipe);
        v.set("HeadColor", wipe);
        v.set("TailColor", wipe);
        v.set("Stagger", wipe);
        v.set("FillColor", !wipe);
    }
}

/// `MorphPolygon.UpdatePolygonFillTypeAttributes`, for an editor showing a `MorphPolygonData` record.
pub(crate) fn polygon_visibility(record: &Params, _effect: &Params, v: &mut Visibility) {
    let wipe = record.enum_name("FillType") == "Wipe";
    v.set("HeadLength", wipe);
    v.set("HeadDuration", wipe);
    v.set("Acceleration", wipe);
    v.set("HeadColor", wipe);
    v.set("TailColor", wipe);
    v.set("FillColor", !wipe);
    v.set("HeadBrightness", wipe);
    v.set("StartOffset", wipe);
    v.set("TailBrightness", wipe);
    v.set("FillBrightness", !wipe);
}

/// The shape record of a `MorphPolygonData`, the first of its members that is set.
fn shape_record_mut(record: &mut Params) -> Option<&mut Params> {
    let name = ["Polygon", "Line", "Ellipse"].into_iter().find(|n| record.record(n).is_some())?;
    let i = record.index_of(name)?;
    match &mut record.values[i] {
        ParamValue::Record(r) => Some(r),
        _ => None,
    }
}

/// `MoveMorphPolygons`.
fn move_morph_polygons(p: &mut Params, x: i32, y: i32) {
    let Some(list) = polygon_records_mut(p) else { return };
    for record in list {
        let Some(shape) = shape_record_mut(record) else { continue };
        let i = shape.index_of("Points");
        if let Some(ParamValue::RecordList(points)) = i.map(|i| &mut shape.values[i]) {
            for point in points {
                let (px, py) = (point.double("X"), point.double("Y"));
                point.set("X", ParamValue::Double(px + x as f64));
                point.set("Y", ParamValue::Double(py + y as f64));
            }
        }
    }
}

/// `ScaleShapesToFitDisplayElement`, on the stored shapes.
fn scale_morph_polygons(p: &mut Params, previous: (i32, i32), buffer: (i32, i32)) {
    if !((previous != buffer) && previous.0 != 0 && previous.1 != 0 && buffer.0 != 0 && buffer.1 != 0) {
        return;
    }
    let x_scale = (buffer.0 as f64 - 1.0) / if previous.0 == 1 { 1.0 } else { previous.0 as f64 - 1.0 };
    let y_scale = (buffer.1 as f64 - 1.0) / if previous.1 == 1 { 1.0 } else { previous.1 as f64 - 1.0 };
    if let Some(list) = polygon_records_mut(p) {
        for record in list {
            let Some(mut shape) = Shape::from_record(record) else { continue };
            shape.scale(x_scale, y_scale, buffer.0, buffer.1);
            if let Some(stored) = shape_record_mut(record) {
                shape.store_in(stored);
            }
        }
    }
}

/// `TargetNodesChanged`: gives an effect with no shapes its first one and scales the others to the new display element.
/// The C# scales from the virtual buffer it held for the old targets. What it saved for that buffer, the display element
/// size, is what `SetupRender` scales from, so an effect that hasn't been rendered yet (size 0) doesn't scale.
fn targets_changed(p: &mut Params, t: &TargetInfo) {
    if t.targets.is_empty() {
        return;
    }
    let (wi, ht) = element_size(t, p);
    let margin = p.int("Margin");
    let with_margin = |size: i32| size.wrapping_add(percent_of(size, margin).wrapping_mul(2));
    let buffer = (with_margin(wi), with_margin(ht));
    if p.records("MorphPolygonData").is_empty() {
        let polygon = default_polygon_record(p, (wi, ht), buffer, (percent_of(wi, margin), percent_of(ht, margin)));
        p.set("MorphPolygonData", ParamValue::RecordList(vec![polygon]));
    }
    rescale_to_targets(p, t);
}

/// `ScaleShapesToFitDisplayElement` from the size last saved to the size of these targets. The render never writes back to
/// the values, so the new size is saved here, where `SetupRender` would have saved it.
fn rescale_to_targets(p: &mut Params, t: &TargetInfo) {
    let (wi, ht) = element_size(t, p);
    let margin = p.int("Margin");
    let with_margin = |size: i32| size.wrapping_add(percent_of(size, margin).wrapping_mul(2));
    let previous = (with_margin(p.int("DisplayElementWidth")), with_margin(p.int("DisplayElementHeight")));
    scale_morph_polygons(p, previous, (with_margin(wi), with_margin(ht)));
    p.set("DisplayElementWidth", ParamValue::Int(wi));
    p.set("DisplayElementHeight", ParamValue::Int(ht));
}

/// The first shape: a polygon filling the display element, or a line on a really skinny one. The C# adds it as soon as the
/// targets arrive, before the user can have changed anything, and the setters then push their values into it, so here the
/// values already set are applied to it.
fn default_polygon_record(p: &Params, element: (i32, i32), buffer: (i32, i32), margin: (i32, i32)) -> Params {
    let (wi, ht) = element;
    let mut shape = if buffer.0 == 1 || buffer.1 == 1 {
        if buffer.0 > buffer.1 {
            Shape::Line(vec![PolygonPoint::new(0.0, 0.0), PolygonPoint::new((wi - 1) as f64, 0.0)])
        } else {
            // Deliberate: the C# builds a line along the y axis here and never assigns it, which leaves the
            // polygon the constructor made, 10 x 10 when no display element size has been seen yet.
            Shape::rectangle(10, 10)
        }
    } else {
        Shape::rectangle(wi, ht)
    };
    shape.move_points(margin.0, margin.1);
    let mut record = Params::defaults(&gen::MORPH_POLYGON_PARAMS);
    record.set("FillType", p.get("FillType").clone());
    for name in ["HeadBrightness", "TailBrightness", "FillBrightness"] {
        record.set(name, p.get(name).clone());
    }
    match &shape {
        Shape::Line(points) => {
            let line: Vec<(f64, f64)> = points.iter().map(|pt| (pt.x, pt.y)).collect();
            record.set("Line", ParamValue::Record(Box::new(polygon::line_record(&line))));
            record.set("Polygon", ParamValue::None);
        }
        other => {
            let corners: Vec<(f64, f64)> = other.points().iter().map(|pt| (pt.x, pt.y)).collect();
            record.set("Polygon", ParamValue::Record(Box::new(polygon::polygon_record(&corners))));
        }
    }
    let fill = p.get("FillType").clone();
    if let Some(stored) = shape_record_mut(&mut record) {
        stored.set("FillType", fill);
    }
    record
}

fn param_changed_ctx(c: &ParamChange, p: &mut Params) {
    match c.name {
        "Margin" => {
            // Moves the shapes as if there were no margin, then for the new one.
            let (wi, ht) = element_size(c.info, p);
            let old = if let ParamValue::Int(v) = c.old { *v } else { 0 };
            let new = p.int("Margin");
            move_morph_polygons(p, -percent_of(wi, old), -percent_of(ht, old));
            move_morph_polygons(p, percent_of(wi, new), percent_of(ht, new));
        }
        // The `StringOrientation` setter and `TargetPositioningChanged` scale the shapes when the user changes the buffer's
        // shape. Target changes don't come through here.
        "StringOrientation" | "TargetPositioning" => {
            if !c.info.targets.is_empty() {
                rescale_to_targets(p, c.info);
            }
        }
        _ => {}
    }
}

/// The setters that reach into the polygons. The `PolygonType` setter also asks whether to convert a repeating pattern
/// into free form polygons, which needs a prompt, so that stays out of this.
fn param_changed(name: &str, p: &mut Params) {
    match name {
        "PolygonType" => {
            let polygon_type = p.enum_name("PolygonType").to_string();
            if polygon_type == "FreeForm" {
                p.set("RepeatCount", ParamValue::Int(0));
            }
            // Pattern mode only has one polygon, and time based mode starts out with one.
            if polygon_type == "Pattern" || polygon_type == "TimeBased" {
                if let Some(list) = polygon_records_mut(p) {
                    list.truncate(1);
                }
            }
        }
        "FillType" => {
            let value = p.get("FillType").clone();
            if let Some(list) = polygon_records_mut(p) {
                for record in list {
                    record.set("FillType", value.clone());
                    // `GetPointBasedShape().FillType = value`.
                    for shape in ["Polygon", "Line", "Ellipse"] {
                        if let Some(ParamValue::Record(r)) = record.index_of(shape).map(|i| &mut record.values[i]) {
                            r.set("FillType", value.clone());
                        }
                    }
                }
            }
        }
        "HeadBrightness" | "TailBrightness" | "FillBrightness" => {
            let value = p.get(name).clone();
            if let Some(list) = polygon_records_mut(p) {
                for record in list {
                    record.set(name, value.clone());
                }
            }
        }
        "TargetPositioning" if p.enum_name("TargetPositioning") == "Locations" => {
            p.set("StringOrientation", ParamValue::Enum("Vertical".to_string()));
        }
        _ => {}
    }
}

fn polygon_records_mut(p: &mut Params) -> Option<&mut Vec<Params>> {
    let i = p.index_of("MorphPolygonData")?;
    match &mut p.values[i] {
        ParamValue::RecordList(list) => Some(list),
        _ => None,
    }
}

/// `PolygonType`.
#[derive(Copy, Clone, PartialEq, Debug)]
enum PolygonType {
    Pattern,
    FreeForm,
    TimeBased,
}

/// `PolygonFillType`.
#[derive(Copy, Clone, PartialEq, Debug)]
enum FillType {
    Wipe,
    Solid,
    Outline,
}

impl FillType {
    fn from_name(name: &str) -> FillType {
        match name {
            "Solid" => FillType::Solid,
            "Outline" => FillType::Outline,
            _ => FillType::Wipe,
        }
    }
}

/// `WipeRepeatDirection`.
#[derive(Copy, Clone, PartialEq, Debug)]
enum RepeatDirection {
    Left,
    Right,
    Up,
    Down,
}

/// What a C# exception does to this effect: `PreRender` catches and logs it, and the effect stays dark.
#[derive(Debug, PartialEq)]
struct Thrown;

/// `IMorphPolygon`.
#[derive(Clone, Debug)]
struct MorphPolygon {
    fill_type: FillType,
    head_length: i32,
    head_duration: i32,
    acceleration: i32,
    head_color: ColorGradient,
    tail_color: ColorGradient,
    fill_color: ColorGradient,
    time: f64,
    shape: Shape,
    start_offset: i32,
    tail_brightness: Curve,
    head_brightness: Curve,
    fill_brightness: Curve,
}

/// A brightness curve, or 100% if the member is null.
fn brightness(record: &Params, name: &str) -> Curve {
    match record.get(name) {
        ParamValue::Curve(c) => c.clone(),
        _ => Curve::flat100(),
    }
}

impl MorphPolygon {
    /// One iteration of `UpdatePolygonModel`.
    fn from_record(r: &Params) -> MorphPolygon {
        MorphPolygon {
            fill_type: FillType::from_name(r.enum_name("FillType")),
            head_length: r.int("HeadLength"),
            head_duration: r.int("HeadDuration"),
            acceleration: r.int("Acceleration"),
            head_color: r.gradient("HeadColor"),
            tail_color: r.gradient("TailColor"),
            fill_color: r.gradient("FillColor"),
            time: r.double("Time"),
            // A record with no shape keeps the polygon `new MorphPolygon()` starts with.
            shape: Shape::from_record(r).unwrap_or_else(|| Shape::rectangle(10, 10)),
            start_offset: r.int("StartOffset"),
            tail_brightness: brightness(r, "TailBrightness"),
            head_brightness: brightness(r, "HeadBrightness"),
            fill_brightness: brightness(r, "FillBrightness"),
        }
    }

    /// The filter inside `GetWipePolygons`.
    fn is_wipe(&self) -> bool {
        self.fill_type == FillType::Wipe
            && match &self.shape {
                Shape::Polygon(p) => p.len() == 4 || p.len() == 3,
                Shape::Line(_) | Shape::Ellipse(_) => true,
            }
    }
}

struct Morph {
    polygon_type: PolygonType,
    fill_type: FillType,
    repeat_count: i32,
    repeat_direction: RepeatDirection,
    repeat_skip: i32,
    stagger: i32,
    head_length: i32,
    head_duration: i32,
    acceleration: i32,
    head_color: ColorGradient,
    tail_color: ColorGradient,
    fill_color: ColorGradient,
    fill_brightness: Curve,
    fill_polygon: bool,
    margin: i32,
    display_element_width: i32,
    display_element_height: i32,
    polygons: Vec<MorphPolygon>,
    /// `_patternExpandedMorphPolygons`.
    expanded: Vec<MorphPolygon>,
    wipe_data: Vec<WipeRenderData>,
    /// `_bufferWi` and `_bufferHt`, the virtual buffer.
    buffer_wi: i32,
    buffer_ht: i32,
    margin_offset_x: i32,
    margin_offset_y: i32,
    /// The buffer rendered into when there is a margin or when rendering by location.
    virtual_buffer: PixelFrameBuffer,
    total_ms: f64,
    frame_time: i32,
    failed: bool,
}

/// The orientation and positioning the effect renders with. The TargetPositioning setter forces vertical strings for
/// locations, so the two always go together.
fn layout(p: &Params) -> (StringOrientation, TargetPositioning) {
    let positioning = TargetPositioning::from_name(p.enum_name("TargetPositioning"));
    let orientation = match positioning {
        TargetPositioning::Locations => StringOrientation::Vertical,
        TargetPositioning::Strings => StringOrientation::from_name(p.enum_name("StringOrientation")),
    };
    (orientation, positioning)
}

/// `BufferWi` and `BufferHt` for these values.
fn element_size(t: &TargetInfo, p: &Params) -> (i32, i32) {
    let (orientation, positioning) = layout(p);
    display_element_size_as(t, orientation, positioning)
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let mut e = Morph::from_params(p);
    let (orientation, positioning) = layout(p);
    let out = render_pixel(ctx, orientation, positioning, &mut e);
    if e.failed {
        EffectIntents::new()
    } else {
        out
    }
}

/// `(int)Math.Round(value * percent / 100.0)`, the margin arithmetic. The multiply wraps as an unchecked C# one does.
#[inline]
fn percent_of(value: i32, percent: i32) -> i32 {
    (value.wrapping_mul(percent) as f64 / 100.0).round_ties_even() as i32
}

impl Morph {
    fn from_params(p: &Params) -> Morph {
        Morph {
            polygon_type: match p.enum_name("PolygonType") {
                "FreeForm" => PolygonType::FreeForm,
                "TimeBased" => PolygonType::TimeBased,
                _ => PolygonType::Pattern,
            },
            fill_type: FillType::from_name(p.enum_name("FillType")),
            repeat_count: p.int("RepeatCount"),
            repeat_direction: match p.enum_name("RepeatDirection") {
                "Left" => RepeatDirection::Left,
                "Right" => RepeatDirection::Right,
                "Down" => RepeatDirection::Down,
                _ => RepeatDirection::Up,
            },
            repeat_skip: p.int("RepeatSkip"),
            stagger: p.int("Stagger"),
            head_length: p.int("HeadLength"),
            head_duration: p.int("HeadDuration"),
            acceleration: p.int("Acceleration"),
            head_color: p.gradient("HeadColor"),
            tail_color: p.gradient("TailColor"),
            fill_color: p.gradient("FillColor"),
            fill_brightness: brightness(p, "FillBrightness"),
            fill_polygon: p.boolean("FillPolygon"),
            margin: p.int("Margin"),
            display_element_width: p.int("DisplayElementWidth"),
            display_element_height: p.int("DisplayElementHeight"),
            polygons: p.records("MorphPolygonData").iter().map(MorphPolygon::from_record).collect(),
            expanded: Vec::new(),
            wipe_data: Vec::new(),
            buffer_wi: 0,
            buffer_ht: 0,
            margin_offset_x: 0,
            margin_offset_y: 0,
            virtual_buffer: PixelFrameBuffer::new(0, 0),
            total_ms: 0.0,
            frame_time: 50,
            failed: false,
        }
    }

    /// `UpdateVirtualFrameBufferSize`.
    fn update_virtual_frame_buffer_size(&mut self, info: &PixelInfo) {
        self.margin_offset_x = percent_of(info.buffer_wi(), self.margin);
        self.margin_offset_y = percent_of(info.buffer_ht(), self.margin);
        self.buffer_wi = info.buffer_wi() + 2 * self.margin_offset_x;
        self.buffer_ht = info.buffer_ht() + 2 * self.margin_offset_y;
    }

    /// `ScaleShapesToFitDisplayElement`.
    fn scale_shapes_to_fit_display_element(&mut self, previous_wi: i32, previous_ht: i32) {
        if (previous_wi != self.buffer_wi || previous_ht != self.buffer_ht)
            && previous_ht != 0
            && previous_wi != 0
            && self.buffer_wi != 0
            && self.buffer_ht != 0
        {
            let width_denominator = if previous_wi == 1 { 1.0 } else { previous_wi as f64 - 1.0 };
            let height_denominator = if previous_ht == 1 { 1.0 } else { previous_ht as f64 - 1.0 };
            let x_scale = (self.buffer_wi as f64 - 1.0) / width_denominator;
            let y_scale = (self.buffer_ht as f64 - 1.0) / height_denominator;
            for polygon in &mut self.polygons {
                polygon.shape.scale(x_scale, y_scale, self.buffer_wi, self.buffer_ht);
            }
        }
    }

    /// `SetupRender`, plus the transfers the C# makes on the first frame (`ExpandPatternMorphPolygons` and the fall
    /// back to free form with one time based polygon), since nothing reads those values in between.
    fn setup(&mut self, info: &PixelInfo) -> Result<(), Thrown> {
        self.total_ms = info.time_span.total_ms();
        self.frame_time = info.frame_time;

        // Far outside what the editor allows. A damaged file could hold them, and the C# would run out of memory
        // or fail on a negative size. Going ahead here could overflow or abort instead.
        if !(0..=MAX_MARGIN).contains(&self.margin)
            || self.repeat_count > MAX_SETTING
            || self.repeat_skip.abs() > MAX_SETTING
            || self.polygons.iter().any(|p| p.head_length.abs() > MAX_SETTING)
        {
            return Err(Thrown);
        }

        let with_margin = |size: i32| size.wrapping_add(percent_of(size, self.margin).wrapping_mul(2));
        let (previous_wi, previous_ht) = (with_margin(self.display_element_width), with_margin(self.display_element_height));
        self.update_virtual_frame_buffer_size(info);
        if self.buffer_wi as i64 * self.buffer_ht as i64 > MAX_VIRTUAL_PIXELS {
            return Err(Thrown);
        }

        // Effects get their first shape when their targets are assigned. With none, `MorphPolygons[0]` throws.
        if self.polygons.is_empty() {
            return Err(Thrown);
        }
        if !self.polygons.iter().all(|polygon| polygon.shape.is_sane()) {
            return Err(Thrown);
        }
        self.scale_shapes_to_fit_display_element(previous_wi, previous_ht);
        if !self.polygons.iter().all(|polygon| polygon.shape.is_sane()) {
            return Err(Thrown);
        }

        if self.polygon_type == PolygonType::Pattern {
            let first = &mut self.polygons[0];
            first.head_length = self.head_length;
            first.head_duration = self.head_duration;
            first.acceleration = self.acceleration;
            first.head_color = self.head_color.clone();
            first.tail_color = self.tail_color.clone();
        }

        self.wipe_data.clear();
        self.setup_render_pattern()?;

        match self.polygon_type {
            PolygonType::Pattern if self.fill_type != FillType::Wipe => self.expand_pattern_morph_polygons(),
            PolygonType::TimeBased if self.polygons.len() <= 1 => {
                // Time based really needs two polygons. This gives the effect some output rather than none.
                self.polygons[0].fill_color = self.fill_color.clone();
                self.polygons[0].fill_type = if self.fill_polygon { FillType::Solid } else { FillType::Outline };
            }
            _ => {}
        }

        if self.margin != 0 || info.target_positioning == TargetPositioning::Locations {
            self.virtual_buffer = PixelFrameBuffer::new(self.buffer_wi, self.buffer_ht);
        }
        Ok(())
    }

    /// `SetupRenderPattern`.
    fn setup_render_pattern(&mut self) -> Result<(), Thrown> {
        let (total_ms, frame_time, buffer_ht) = (self.total_ms, self.frame_time, self.buffer_ht);
        let repeat_count = self.repeat_count;
        for morph_polygon in self.polygons.iter().filter(|p| p.is_wipe()) {
            let mut data = WipeRenderData::default();

            if self.polygon_type == PolygonType::Pattern {
                data.head_is_done = vec![false; (repeat_count + 1).max(0) as usize];
                data.stagger = self.stagger as f64;
                // Adjust the stagger so that it does not exceed the total duration of the effect.
                if total_ms * (data.stagger / 100.0) * repeat_count as f64 > total_ms {
                    data.stagger = (((total_ms / repeat_count as f64) / total_ms) * 100.0) as i32 as f64;
                }
            } else {
                data.head_is_done.push(false);
                data.stagger = morph_polygon.start_offset as f64;
            }
            data.masks = vec![None; data.head_is_done.len()];

            let model = wipe_polygon_model(&morph_polygon.shape)?;

            // The points along the sides of the polygon, neither of them the start or end side.
            store_line(
                model[1].x as i32,
                buffer_ht - model[1].y as i32 - 1,
                model[2].x as i32,
                buffer_ht - model[2].y as i32 - 1,
                &mut data.x1_points,
                &mut data.y1_points,
            );
            store_line(
                model[0].x as i32,
                buffer_ht - model[0].y as i32 - 1,
                model[3].x as i32,
                buffer_ht - model[3].y as i32 - 1,
                &mut data.x2_points,
                &mut data.y2_points,
            );
            data.direction = calculate_direction(&model, buffer_ht);

            let length = data.length();

            // The time each polygon has to perform the wipe, subtracting off all the staggers.
            let time = if repeat_count == 0 && morph_polygon.start_offset != 0 {
                total_ms - total_ms * (data.stagger / 100.0)
            } else {
                total_ms - total_ms * (data.stagger / 100.0) * repeat_count as f64
            };

            // The head has to travel past the end of the shape by the length of the head.
            let polygon_length_for_head = length + morph_polygon.head_length;
            let head_duration = morph_polygon.head_duration as f64 / 100.0;

            if morph_polygon.acceleration != 0 {
                let acceleration = morph_polygon.acceleration;
                if acceleration < 0 {
                    let scale = (acceleration as f64 / MAX_ACCELERATION).abs();
                    data.head_acceleration =
                        negative_acceleration(polygon_length_for_head, time * head_duration, acceleration, frame_time)? * scale;
                    data.tail_acceleration = negative_acceleration(length, time, acceleration, frame_time)? * scale;
                } else {
                    let scaled = |a: f64| a * acceleration as f64 / MAX_ACCELERATION;
                    data.head_acceleration = scaled(positive_acceleration(length, time * head_duration));
                    data.tail_acceleration = scaled(positive_acceleration(length, time));
                }
            }

            data.head_velocity_zero =
                increasing_velocity_zero(data.head_acceleration, polygon_length_for_head as f64, time * head_duration);
            data.tail_velocity_zero = increasing_velocity_zero(data.tail_acceleration, length as f64, time);

            self.wipe_data.push(data);
        }
        Ok(())
    }

    /// `ExpandPatternMorphPolygons`. Each copy clones the one before it, so the moves accumulate.
    fn expand_pattern_morph_polygons(&mut self) {
        self.expanded.clear();
        self.polygons[0].fill_type = self.fill_type;
        self.polygons[0].fill_color = self.fill_color.clone();
        self.polygons[0].start_offset = 0;
        self.expanded.push(self.polygons[0].clone());

        for index in 0..self.repeat_count {
            let mut copy = self.expanded[self.expanded.len() - 1].clone();
            copy.start_offset = self.stagger * (index + 1);
            let (x, y) = match self.repeat_direction {
                RepeatDirection::Down => (0, self.repeat_skip),
                RepeatDirection::Up => (0, -self.repeat_skip),
                RepeatDirection::Left => (-self.repeat_skip, 0),
                RepeatDirection::Right => (self.repeat_skip, 0),
            };
            copy.shape.move_points(x, y);
            // Deliberate: the C# moves an ellipse's points a second time here. Only its center is drawn from.
            if let Shape::Ellipse(e) = &mut copy.shape {
                update_ellipse_offset(e, x, y);
            }
            self.expanded.push(copy);
        }
    }

    /// `RenderEffectInternal`.
    fn render_internal(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) -> Result<(), Thrown> {
        let interval_pos = info.interval_position(frame);
        match self.polygon_type {
            PolygonType::TimeBased => {
                if self.polygons.len() > 1 {
                    self.render_time_based(frame, buf, interval_pos)
                } else {
                    self.render_free_form(frame, buf, interval_pos)
                }
            }
            PolygonType::Pattern => self.render_pattern(frame, buf, interval_pos),
            PolygonType::FreeForm => self.render_free_form(frame, buf, interval_pos),
        }
    }

    /// `RenderEffectPattern`.
    fn render_pattern(&mut self, frame: i32, buf: &mut PixelFrameBuffer, interval_pos: f64) -> Result<(), Thrown> {
        if self.fill_type == FillType::Wipe {
            self.render_wipes(frame, buf, interval_pos)
        } else if matches!(self.polygons[0].shape, Shape::Polygon(_) | Shape::Line(_)) {
            render_static_polygons(buf, self.buffer_wi, self.buffer_ht, interval_pos, self.expanded.iter())
        } else {
            render_static_ellipses(buf, self.buffer_wi, self.buffer_ht, interval_pos, self.expanded.iter());
            Ok(())
        }
    }

    /// `RenderEffectFreeForm`.
    fn render_free_form(&mut self, frame: i32, buf: &mut PixelFrameBuffer, interval_pos: f64) -> Result<(), Thrown> {
        self.render_wipes(frame, buf, interval_pos)?;

        let statics = self.polygons.iter().filter(|p| !matches!(p.shape, Shape::Ellipse(_)) && p.fill_type != FillType::Wipe);
        render_static_polygons(buf, self.buffer_wi, self.buffer_ht, interval_pos, statics)?;

        let ellipses = self.polygons.iter().filter(|p| matches!(p.shape, Shape::Ellipse(_)) && p.fill_type != FillType::Wipe);
        render_static_ellipses(buf, self.buffer_wi, self.buffer_ht, interval_pos, ellipses);
        Ok(())
    }

    /// `RenderEffectWipes`.
    fn render_wipes(&mut self, frame: i32, buf: &mut PixelFrameBuffer, interval_pos: f64) -> Result<(), Thrown> {
        let (wi, ht) = (self.buffer_wi, self.buffer_ht);
        let time = self.total_ms;
        let time_now = (frame * self.frame_time) as f64;
        let repeat_count = self.repeat_count;
        let (step_x, step_y) = match self.repeat_direction {
            RepeatDirection::Down => (0, -self.repeat_skip),
            RepeatDirection::Up => (0, self.repeat_skip),
            RepeatDirection::Left => (-self.repeat_skip, 0),
            RepeatDirection::Right => (self.repeat_skip, 0),
        };

        let wipes = self.polygons.iter().filter(|p| p.is_wipe());
        for (morph_polygon, data) in wipes.zip(self.wipe_data.iter_mut()) {
            let length = data.length();
            let colors = WipeColors {
                head: shaded(morph_polygon.head_color.get_color_at(interval_pos), &morph_polygon.head_brightness, interval_pos),
                tail: shaded(morph_polygon.tail_color.get_color_at(interval_pos), &morph_polygon.tail_brightness, interval_pos),
            };
            let ellipse = match &morph_polygon.shape {
                Shape::Ellipse(e) => Some(e),
                _ => None,
            };
            // `ellipseMask`: the clone that moves along with the repeats.
            let mut mask_center = ellipse.map(|e| e.center);

            for index in 0..(repeat_count + 1) {
                let offset = if repeat_count == 0 && morph_polygon.start_offset != 0 {
                    (data.stagger / 100.0) * time
                } else {
                    index as f64 * (data.stagger / 100.0) * time
                };
                let time_in_wipe = time_now - offset;

                if time_in_wipe >= 0.0 {
                    let mut head_position =
                        increasing_head_position(data.head_acceleration, time_in_wipe, data.head_velocity_zero);

                    let done = data.head_is_done.get_mut(index as usize).ok_or(Thrown)?;
                    // Once the head has completed the wipe it must not back up onto the display element when
                    // decelerating.
                    if head_position > (length + morph_polygon.head_length - 1) as f64 {
                        *done = true;
                    }
                    if *done {
                        // Just past the end of the shape, so that the tail can wipe off it.
                        head_position = (length + morph_polygon.head_length) as f64;
                    }

                    let tail_position =
                        increasing_head_position(data.tail_acceleration, time_in_wipe, data.tail_velocity_zero) as i32;
                    let interval_head_pos = head_position / length as f64;

                    let mask = match (ellipse, mask_center) {
                        (Some(e), Some(center)) => {
                            let slot = data.masks.get_mut(index as usize).ok_or(Thrown)?;
                            Some(&*slot.get_or_insert_with(|| ellipse_mask(e, center, wi, ht)))
                        }
                        _ => None,
                    };

                    let x1 = Side::new(&data.x1_points, &data.y1_points);
                    let x2 = Side::new(&data.x2_points, &data.y2_points);
                    // The shorter side comes first.
                    let (short, long) = if data.x1_points.len() > data.x2_points.len() { (x2, x1) } else { (x1, x2) };

                    let mut target = WipeTarget {
                        buf: &mut *buf,
                        mask,
                        wi,
                        ht,
                        offset_x: step_x * index,
                        offset_y: step_y * index,
                        direction: data.direction,
                    };
                    let head_length = morph_polygon.head_length;
                    draw_wipe(&mut target, interval_head_pos, tail_position, short, long, length, colors, head_length)?;
                }

                // The repeating pattern moves the mask with every pass, whether or not this one drew.
                if let Some(center) = &mut mask_center {
                    center.x += step_x as f64;
                    center.y += step_y as f64;
                }
            }
        }
        Ok(())
    }

    /// `RenderEffectTimeBased`.
    fn render_time_based(&self, frame: i32, buf: &mut PixelFrameBuffer, interval_pos: f64) -> Result<(), Thrown> {
        match &self.polygons[0].shape {
            Shape::Polygon(first) => {
                // The first polygon decides the number of points. `mp.Polygon.Points` throws when a later
                // shape is not a polygon.
                let mut count = 0;
                for morph_polygon in &self.polygons {
                    match &morph_polygon.shape {
                        Shape::Polygon(points) if points.len() == first.len() => count += 1,
                        Shape::Polygon(_) => {}
                        _ => return Err(Thrown),
                    }
                }
                self.render_time_based_polygon(frame, count, buf, interval_pos)
            }
            Shape::Line(_) => {
                let count = self.polygons.iter().filter(|p| matches!(p.shape, Shape::Line(_))).count();
                self.render_time_based_polygon(frame, count, buf, interval_pos)
            }
            Shape::Ellipse(_) => {
                let count = self.polygons.iter().filter(|p| matches!(p.shape, Shape::Ellipse(_))).count();
                self.render_time_based_ellipse(frame, count, buf, interval_pos)
            }
        }
    }

    /// `FindTwoPolygonsOnTimeline`. Deliberate: it loops over the count of the matching shapes but indexes the full
    /// list, as the C# does. Returns the first of the two.
    fn find_two_polygons_on_timeline(&self, time: f64, count: usize) -> Option<usize> {
        let time_fraction = time / self.total_ms;
        (1..count)
            .rev()
            .find(|&index| self.polygons[index - 1].time < time_fraction && time_fraction <= self.polygons[index].time)
            .map(|index| index - 1)
    }

    /// `RenderEffectTimeBasedMorpPolygon`.
    fn render_time_based_polygon(
        &self,
        frame: i32,
        count: usize,
        buf: &mut PixelFrameBuffer,
        interval_pos: f64,
    ) -> Result<(), Thrown> {
        let time = (frame * self.frame_time) as f64;
        if let Some(polygon_index) = self.find_two_polygons_on_timeline(time, count) {
            let start = &self.polygons[polygon_index];
            let end = &self.polygons[polygon_index + 1];
            let points = self.intermediate_points(time, start, end)?;

            let color = self.fill_color_at(interval_pos, &self.fill_brightness);
            let (wi, ht) = (self.buffer_wi, self.buffer_ht);
            // `RenderStaticPolygon`.
            let mut plot = |x: i32, y: i32| buf.set_pixel(x, ht - y - 1, color);
            if points.len() > 2 {
                initial_render_polygon(&points, self.fill_polygon, wi, ht, &mut plot);
            } else {
                initial_render_line(&points, &mut plot)?;
            }
        }
        Ok(())
    }

    /// `RenderEffectTimeBasedMorphEllipse`.
    fn render_time_based_ellipse(
        &self,
        frame: i32,
        count: usize,
        buf: &mut PixelFrameBuffer,
        interval_pos: f64,
    ) -> Result<(), Thrown> {
        let time = (frame * self.frame_time) as f64;
        if let Some(polygon_index) = self.find_two_polygons_on_timeline(time, count) {
            let start = &self.polygons[polygon_index];
            let end = &self.polygons[polygon_index + 1];
            // The points are not drawn from, but a list that runs short still throws.
            self.intermediate_points(time, start, end)?;

            // `.Ellipse` is null for any other shape, because the index comes from the full list.
            let (Shape::Ellipse(from), Shape::Ellipse(to)) = (&start.shape, &end.shape) else {
                return Err(Thrown);
            };
            let center = self.intermediate_point(time, from.center, to.center, start.time, end.time);
            let intermediate = |a: f64, b: f64| self.intermediate_value(time, a, b, start.time, end.time);
            let draw = EllipseDraw {
                center_x: center.0 as f64,
                center_y: center.1 as f64,
                width: intermediate(from.width, to.width),
                height: intermediate(from.height, to.height),
                angle: intermediate(from.angle, to.angle),
            };

            let color = self.fill_color_at(interval_pos, &self.fill_brightness);
            let (wi, ht) = (self.buffer_wi, self.buffer_ht);
            let mut plot = |x: i32, y: i32| buf.set_pixel(x, ht - y - 1, color);
            initial_render_ellipse(draw, self.fill_polygon, wi, ht, &mut plot);
        }
        Ok(())
    }

    fn intermediate_points(&self, time: f64, start: &MorphPolygon, end: &MorphPolygon) -> Result<Vec<(i32, i32)>, Thrown> {
        let (from, to) = (start.shape.points(), end.shape.points());
        if to.len() < from.len() {
            return Err(Thrown);
        }
        Ok(from.iter().zip(to).map(|(a, b)| self.intermediate_point(time, *a, *b, start.time, end.time)).collect())
    }

    /// `CalculateItermediatePoint` (sic).
    fn intermediate_point(
        &self,
        time: f64,
        start: PolygonPoint,
        end: PolygonPoint,
        start_time: f64,
        end_time: f64,
    ) -> (i32, i32) {
        let total_time_on_line = (end_time - start_time) * self.total_ms;
        let velocity_x = (end.x - start.x) / total_time_on_line;
        let velocity_y = (end.y - start.y) / total_time_on_line;
        let current_time_on_line = time - (start_time * self.total_ms);
        let x = start.x + velocity_x * current_time_on_line;
        let y = start.y + velocity_y * current_time_on_line;
        (x.round_ties_even() as i32, y.round_ties_even() as i32)
    }

    /// `CalculateIntermediateValue`.
    fn intermediate_value(&self, time: f64, start_value: f64, end_value: f64, start_time: f64, end_time: f64) -> f64 {
        let total_time_on_line = (end_time - start_time) * self.total_ms;
        let velocity = (end_value - start_value) / total_time_on_line;
        let current_time_on_line = time - (start_time * self.total_ms);
        start_value + velocity * current_time_on_line
    }

    /// `GetFillColor(double, Curve)`, for time based mode.
    fn fill_color_at(&self, interval_pos: f64, intensity: &Curve) -> Color {
        shaded(self.fill_color.get_color_at(interval_pos), intensity, interval_pos)
    }

    /// `UpdateFrameBufferForLocationPixel`, as far as finding the colour. None where the C# array access throws.
    fn location_pixel(&self, x: i32, y: i32, info: &PixelInfo, virtual_buffer: &PixelFrameBuffer) -> Option<Color> {
        let flipped = ((info.buffer_ht_offset() - self.margin_offset_y) + self.buffer_ht - y - 1).abs();
        let shifted = x - (info.buffer_wi_offset() - self.margin_offset_x);
        if shifted < 0 || flipped < 0 || shifted >= self.buffer_wi || flipped >= self.buffer_ht {
            return None;
        }
        Some(virtual_buffer.get_color_at(shifted, flipped))
    }
}

impl PixelEffect for Morph {
    fn setup_render(&mut self, info: &PixelInfo) {
        self.failed = self.setup(info).is_err();
    }

    /// `RenderEffect`.
    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        if self.failed {
            return;
        }
        if self.margin != 0 {
            // A virtual matrix based on the display element and the margin.
            let mut virtual_buffer = std::mem::replace(&mut self.virtual_buffer, PixelFrameBuffer::new(0, 0));
            virtual_buffer.fill(Color::EMPTY);
            self.failed = self.render_internal(frame, info, &mut virtual_buffer).is_err();
            if !self.failed {
                // Extract the pixels that correspond to the actual frame buffer.
                for x in 0..info.buffer_wi() {
                    for y in 0..info.buffer_ht() {
                        buf.set_pixel(x, y, virtual_buffer.get_color_at(x + self.margin_offset_x, y + self.margin_offset_y));
                    }
                }
            }
            self.virtual_buffer = virtual_buffer;
        } else {
            self.failed = self.render_internal(frame, info, buf).is_err();
        }
    }

    /// `RenderEffectByLocation`. The C# has an "internal" and a "virtual" version, picked by whether there is a
    /// margin, which do the same thing.
    fn render_effect_by_location(&mut self, num_frames: i32, info: &PixelInfo, buf: &mut PixelLocationFrameBuffer) {
        let locations = buf.element_locations().to_vec();
        let mut virtual_buffer = std::mem::replace(&mut self.virtual_buffer, PixelFrameBuffer::new(0, 0));
        for frame in 0..num_frames {
            if self.failed {
                break;
            }
            buf.current_frame = frame;
            self.failed = self.render_internal(frame, info, &mut virtual_buffer).is_err();
            for l in &locations {
                match self.location_pixel(l.x, l.y, info, &virtual_buffer) {
                    Some(color) => buf.set_pixel(l.x, l.y, color),
                    None => self.failed = true,
                }
            }
            virtual_buffer.clear_buffer();
        }
        self.virtual_buffer = virtual_buffer;
    }
}

/// `hsv.V *= intensity(interval position)`, the core of `GetHeadColor`, `GetTailColor` and `GetFillColor`.
fn shaded(color: Color, intensity: &Curve, interval_pos: f64) -> Color {
    let mut hsv = Hsv::from_color(color);
    hsv.set_v(hsv.v * (intensity.get_value(interval_pos * 100.0) / 100.0));
    hsv.to_color()
}

/// `UpdateEllipseForYOffset` and `UpdateEllipseForXOffset`.
fn update_ellipse_offset(ellipse: &mut Ellipse, x: i32, y: i32) {
    for pt in &mut ellipse.points {
        pt.x += x as f64;
        pt.y += y as f64;
    }
    ellipse.center.x += x as f64;
    ellipse.center.y += y as f64;
}

/// `InitialRenderPolygon`. Filling alone misses the right and bottom sides, so the outline always follows.
fn initial_render_polygon(points: &[(i32, i32)], fill: bool, wi: i32, ht: i32, plot: &mut impl FnMut(i32, i32)) {
    if fill {
        let corners: Vec<(f64, f64)> = points.iter().map(|p| (p.0 as f64, p.1 as f64)).collect();
        gdi::fill_polygon(&corners, wi, ht, plot);
    }
    gdi::draw_polygon(points, plot);
}

/// `InitialRenderLine`.
fn initial_render_line(points: &[(i32, i32)], plot: &mut impl FnMut(i32, i32)) -> Result<(), Thrown> {
    if points.len() < 2 {
        return Err(Thrown);
    }
    gdi::draw_line(points[0].0, points[0].1, points[1].0, points[1].1, plot);
    Ok(())
}

/// `InitialRenderEllipse`.
fn initial_render_ellipse(draw: EllipseDraw, fill: bool, wi: i32, ht: i32, plot: &mut impl FnMut(i32, i32)) {
    if fill {
        draw.fill(wi, ht, plot);
    } else {
        draw.outline(plot);
    }
}

/// `RenderStaticPolygons`. The C# draws to one bitmap and then copies it across, skipping empty pixels. Every
/// colour here is opaque, so plotting straight into the frame buffer gives the same result.
fn render_static_polygons<'a>(
    buf: &mut PixelFrameBuffer,
    wi: i32,
    ht: i32,
    interval_pos: f64,
    polygons: impl Iterator<Item = &'a MorphPolygon>,
) -> Result<(), Thrown> {
    let mut points: Vec<(i32, i32)> = Vec::new();
    for morph_polygon in polygons {
        points.clear();
        points.extend(morph_polygon.shape.points().iter().map(|pt| (pt.x.round() as i32, pt.y.round() as i32)));
        let color = shaded(morph_polygon.fill_color.get_color_at(interval_pos), &morph_polygon.fill_brightness, interval_pos);
        let mut plot = |x: i32, y: i32| buf.set_pixel(x, ht - y - 1, color);
        if points.len() > 2 {
            initial_render_polygon(&points, morph_polygon.fill_type == FillType::Solid, wi, ht, &mut plot);
        } else {
            initial_render_line(&points, &mut plot)?;
        }
    }
    Ok(())
}

/// `RenderStaticEllipse(frameBuffer, intervalPos, polygons)`.
fn render_static_ellipses<'a>(
    buf: &mut PixelFrameBuffer,
    wi: i32,
    ht: i32,
    interval_pos: f64,
    polygons: impl Iterator<Item = &'a MorphPolygon>,
) {
    for morph_polygon in polygons {
        if let Shape::Ellipse(e) = &morph_polygon.shape {
            let color = shaded(morph_polygon.fill_color.get_color_at(interval_pos), &morph_polygon.fill_brightness, interval_pos);
            let draw =
                EllipseDraw { center_x: e.center.x, center_y: e.center.y, width: e.width, height: e.height, angle: e.angle };
            let fill = morph_polygon.fill_type == FillType::Solid || morph_polygon.fill_type == FillType::Wipe;
            let mut plot = |x: i32, y: i32| buf.set_pixel(x, ht - y - 1, color);
            initial_render_ellipse(draw, fill, wi, ht, &mut plot);
        }
    }
}

#[cfg(test)]
mod tests;
