//! Port of `PixelEffectBase`, `PixelFrameBuffer` and `PixelLocationFrameBuffer`.

use crate::framework::{Params, RenderCtx};
use std::collections::HashMap;
use std::sync::Arc;
use vixen_core::{Color, Curve, EffectIntents, FrameArray, Hsv, Intent, IntentData, NodeId, RgbValue, TimeSpan, Uuid};

/// `StringOrientation`.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum StringOrientation {
    #[default]
    Vertical,
    Horizontal,
}

impl StringOrientation {
    pub fn from_name(name: &str) -> StringOrientation {
        if name == "Horizontal" {
            StringOrientation::Horizontal
        } else {
            StringOrientation::Vertical
        }
    }
}

/// `TargetPositioningType`.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum TargetPositioning {
    #[default]
    Strings,
    Locations,
}

impl TargetPositioning {
    pub fn from_name(name: &str) -> TargetPositioning {
        if name == "Locations" {
            TargetPositioning::Locations
        } else {
            TargetPositioning::Strings
        }
    }
}

/// `IPixelFrameBuffer`, so drawing helpers work on both buffer types.
pub trait PixelBuffer {
    fn set_pixel(&mut self, x: i32, y: i32, color: Color);
    fn get_color_at(&self, x: i32, y: i32) -> Color;

    #[inline]
    fn set_pixel_hsv(&mut self, x: i32, y: i32, hsv: Hsv) {
        self.set_pixel(x, y, hsv.to_color());
    }
}

/// `PixelFrameBuffer`: 0,0 is lower left. Stored as `[x][y]` in one flat allocation.
#[derive(Clone, Debug)]
pub struct PixelFrameBuffer {
    pixels: Vec<Color>,
    wi: i32,
    ht: i32,
    base_color: Color,
}

impl PixelFrameBuffer {
    pub fn new(width: i32, height: i32) -> PixelFrameBuffer {
        PixelFrameBuffer::with_base(width, height, Color::TRANSPARENT)
    }

    pub fn with_base(width: i32, height: i32, base_color: Color) -> PixelFrameBuffer {
        let (w, h) = (width.max(0), height.max(0));
        PixelFrameBuffer { pixels: vec![Color::EMPTY; (w as usize) * (h as usize)], wi: w, ht: h, base_color }
    }

    #[inline]
    pub fn width(&self) -> i32 {
        self.wi
    }

    #[inline]
    pub fn height(&self) -> i32 {
        self.ht
    }

    /// `ClearBuffer()`.
    #[inline]
    pub fn clear_buffer(&mut self) {
        self.pixels.fill(self.base_color);
    }

    /// `ClearBuffer(double level)`.
    pub fn clear_buffer_level(&mut self, level: f64) {
        let mut hsv = Hsv::from_color(self.base_color);
        hsv.set_v(hsv.v * level);
        self.pixels.fill(hsv.to_color());
    }

    /// Fills with one colour, for effects that keep scratch buffers.
    pub fn fill(&mut self, c: Color) {
        self.pixels.fill(c);
    }
}

impl PixelBuffer for PixelFrameBuffer {
    #[inline]
    fn set_pixel(&mut self, x: i32, y: i32, color: Color) {
        if x >= 0 && x < self.wi && y >= 0 && y < self.ht {
            self.pixels[x as usize * self.ht as usize + y as usize] = color;
        }
    }

    /// The C# array access throws out of range, leaving the effect dark. Reads here return
    /// `Color::EMPTY` instead, so one bad read cannot take down a show.
    #[inline]
    fn get_color_at(&self, x: i32, y: i32) -> Color {
        if x >= 0 && x < self.wi && y >= 0 && y < self.ht {
            self.pixels[x as usize * self.ht as usize + y as usize]
        } else {
            Color::EMPTY
        }
    }
}

/// `ElementLocation`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ElementLocation {
    pub x: i32,
    pub y: i32,
    pub node: NodeId,
}

/// `PixelLocationFrameBuffer`: one per-frame array for each occupied location.
pub struct PixelLocationFrameBuffer {
    data: Vec<RgbValue>,
    index: HashMap<(i32, i32), u32>,
    num_frames: usize,
    locations: Vec<ElementLocation>,
    pub current_frame: i32,
}

impl PixelLocationFrameBuffer {
    pub fn new(element_locations: &[ElementLocation], num_frames: i32) -> PixelLocationFrameBuffer {
        let num_frames = num_frames.max(0) as usize;
        let mut index = HashMap::with_capacity(element_locations.len());
        let mut locations = Vec::with_capacity(element_locations.len());
        for l in element_locations {
            let next = index.len() as u32;
            index.entry((l.x, l.y)).or_insert(next);
            if !locations.iter().any(|e: &ElementLocation| e.node == l.node) {
                locations.push(*l);
            }
        }
        PixelLocationFrameBuffer {
            data: vec![RgbValue::BLACK; index.len() * num_frames],
            index,
            num_frames,
            locations,
            current_frame: 0,
        }
    }

    /// `ElementLocations`: distinct by node.
    pub fn element_locations(&self) -> &[ElementLocation] {
        &self.locations
    }

    /// `ClearFrame()`.
    pub fn clear_frame(&mut self) {
        let f = self.current_frame as usize;
        if f < self.num_frames {
            for slot in self.data.chunks_exact_mut(self.num_frames) {
                slot[f] = RgbValue::BLACK;
            }
        }
    }

    /// Consumes the buffer, returning the shared block and each location's slot.
    pub fn into_block(self) -> (Arc<[RgbValue]>, HashMap<(i32, i32), u32>, usize) {
        (Arc::from(self.data), self.index, self.num_frames)
    }
}

impl PixelBuffer for PixelLocationFrameBuffer {
    #[inline]
    fn set_pixel(&mut self, x: i32, y: i32, color: Color) {
        if let Some(&slot) = self.index.get(&(x, y)) {
            let f = self.current_frame as usize;
            if f < self.num_frames {
                self.data[slot as usize * self.num_frames + f] = RgbValue::new(color);
            }
        }
    }

    #[inline]
    fn get_color_at(&self, x: i32, y: i32) -> Color {
        match self.index.get(&(x, y)) {
            Some(&slot) => self.data[slot as usize * self.num_frames + self.current_frame as usize].full_color(),
            None => Color::EMPTY,
        }
    }
}

/// The read-only part of `PixelEffectBase` an effect sees during rendering.
#[derive(Clone, Debug)]
pub struct PixelInfo {
    pub string_orientation: StringOrientation,
    pub target_positioning: TargetPositioning,
    /// `StringPixelCounts`.
    pub string_pixel_counts: Vec<i32>,
    /// `StringCount`, the number of leaf-parent strings.
    pub string_count: i32,
    /// `MaxPixelsPerString`.
    pub max_pixels_per_string: i32,
    /// `ElementLocations`, for location rendering.
    pub element_locations: Vec<ElementLocation>,
    pub start_time: TimeSpan,
    pub time_span: TimeSpan,
    /// `FrameTime`, in ms.
    pub frame_time: i32,
    raw_ht: i32,
    raw_wi: i32,
    raw_ht_offset: i32,
    raw_wi_offset: i32,
}

impl PixelInfo {
    /// `BufferHt`.
    #[inline]
    pub fn buffer_ht(&self) -> i32 {
        if self.string_orientation == StringOrientation::Horizontal {
            self.raw_ht
        } else {
            self.raw_wi
        }
    }

    /// `BufferWi`.
    #[inline]
    pub fn buffer_wi(&self) -> i32 {
        if self.string_orientation == StringOrientation::Horizontal {
            self.raw_wi
        } else {
            self.raw_ht
        }
    }

    /// `BufferHtOffset`.
    #[inline]
    pub fn buffer_ht_offset(&self) -> i32 {
        if self.string_orientation == StringOrientation::Horizontal {
            self.raw_ht_offset
        } else {
            self.raw_wi_offset
        }
    }

    /// `BufferWiOffset`.
    #[inline]
    pub fn buffer_wi_offset(&self) -> i32 {
        if self.string_orientation == StringOrientation::Horizontal {
            self.raw_wi_offset
        } else {
            self.raw_ht_offset
        }
    }

    /// `GetEffectTimeIntervalPosition(int frame)`.
    #[inline]
    pub fn interval_position(&self, frame: i32) -> f64 {
        if self.time_span == TimeSpan::ZERO {
            0.0
        } else {
            (frame * self.frame_time) as f64 / self.time_span.total_ms()
        }
    }

    /// `GetNumberFrames()`.
    #[inline]
    pub fn number_frames(&self) -> i32 {
        if self.time_span == TimeSpan::ZERO {
            0
        } else {
            (self.time_span.total_ms() / self.frame_time as f64) as i32
        }
    }

    /// `GetRemainingTime(int frame)`.
    #[inline]
    pub fn remaining_time(&self, frame: i32) -> f64 {
        let total = self.time_span.total_ms();
        total - total / 100.0 * (self.interval_position(frame) * 100.0)
    }
}

/// `CalculateAcceleration`.
pub fn calculate_acceleration(ratio: f64, accel: f64) -> f64 {
    if accel == 0.0 {
        return ratio;
    }
    let pct = (accel.abs() - 1.0) / 9.0;
    let new1 = pct * 5.0 + (1.0 - pct) * 1.5;
    let new2 = 1.5 + (ratio * new1);
    let final_accel = pct * new2 + (1.0 - pct) * new1;
    if accel > 0.0 {
        ratio.powf(final_accel)
    } else {
        1.0 - (1.0 - ratio).powf(new1)
    }
}

/// `GetStepAngle`.
pub fn step_angle(width: i32, height: i32) -> f64 {
    let biggest = width.max(height);
    if biggest > 350 {
        0.1
    } else if biggest > 250 {
        0.2
    } else if biggest > 150 {
        0.3
    } else if biggest > 50 {
        0.4
    } else {
        0.5
    }
}

/// `IsAngleBetween`.
pub fn is_angle_between(a: f64, b: f64, n: f64) -> bool {
    let n = (360.0 + (n % 360.0)) % 360.0;
    let a = (3600000.0 + a) % 360.0;
    let b = (3600000.0 + b) % 360.0;
    if a < b {
        a <= n && n <= b
    } else {
        a <= n || n <= b
    }
}

/// `DegreesDiffernce` (sic).
#[inline]
pub fn degrees_difference(a1: f64, a2: f64) -> f64 {
    (360.0 - (a1 - a2).abs()).min((a1 - a2).abs())
}

/// `GetAngleDegree`.
#[inline]
pub fn angle_degree(ox: i32, oy: i32, x: i32, y: i32) -> f64 {
    let n = 270.0 - ((oy - y) as f64).atan2((ox - x) as f64) * 180.0 / std::f64::consts::PI;
    n % 360.0
}

/// `AddDegrees`.
#[inline]
pub fn add_degrees(angle: f64, degrees: f64) -> f64 {
    let mut a = (angle + degrees) % 360.0;
    if a < 0.0 {
        a += 360.0;
    }
    a
}

/// The abstract members of `PixelEffectBase`. Effects hold their parameters and scratch state as fields.
pub trait PixelEffect: Send {
    /// `SetupRender`.
    fn setup_render(&mut self, _info: &PixelInfo) {}

    /// `RenderEffect`.
    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer);

    /// `RenderEffectByLocation`. Implement only when the descriptor sets `supports_location`.
    fn render_effect_by_location(&mut self, _num_frames: i32, _info: &PixelInfo, _buf: &mut PixelLocationFrameBuffer) {}

    /// `CleanUpRender`.
    fn cleanup_render(&mut self) {}

    /// `UseBaseColor`, `BaseColor` and `BaseLevelCurve`.
    fn base_color(&self) -> Option<(Color, Curve)> {
        None
    }

    /// `GetRenderGroups`. By default all targets render together as one group.
    fn render_groups(&self, targets: &[NodeId]) -> Vec<Vec<NodeId>> {
        vec![targets.to_vec()]
    }

    /// Runs after buffers are configured for a render group, when an effect needs the group's targets.
    fn begin_group(&mut self, _targets: &[NodeId], _ctx: &RenderCtx) {}
}

/// Reads the two parameters every pixel effect has, by their conventional names, and renders.
pub fn render_pixel_effect<E: PixelEffect>(ctx: &RenderCtx, p: &Params, effect: &mut E) -> EffectIntents {
    let (orientation, positioning) = orientation_and_positioning(p);
    render_pixel(ctx, orientation, positioning, effect)
}

fn orientation_and_positioning(p: &Params) -> (StringOrientation, TargetPositioning) {
    let orientation = if p.index_of("StringOrientation").is_some() {
        StringOrientation::from_name(p.enum_name("StringOrientation"))
    } else {
        StringOrientation::Vertical
    };
    let positioning = if p.index_of("TargetPositioning").is_some() {
        TargetPositioning::from_name(p.enum_name("TargetPositioning"))
    } else {
        TargetPositioning::Strings
    };
    (orientation, positioning)
}

/// `PixelEffectBase._PreRender`.
pub fn render_pixel<E: PixelEffect>(
    ctx: &RenderCtx,
    orientation: StringOrientation,
    positioning: TargetPositioning,
    effect: &mut E,
) -> EffectIntents {
    let mut data: Option<EffectIntents> = None;
    for group in effect.render_groups(ctx.targets) {
        let mut targets: Vec<NodeId> = Vec::with_capacity(group.len());
        for t in group {
            if !targets.contains(&t) {
                targets.push(t);
            }
        }
        if targets.is_empty() {
            continue;
        }
        let info = configure(ctx, &targets, orientation, positioning);
        effect.begin_group(&targets, ctx);
        effect.setup_render(&info);
        let out = data.get_or_insert_with(|| EffectIntents::with_capacity(info.string_pixel_counts.iter().sum::<i32>().max(0) as usize));
        match positioning {
            TargetPositioning::Strings => render_by_strings(ctx, &targets, &info, effect, out),
            TargetPositioning::Locations => {
                // `elementLocation.ElementNode.Element.Id` throws for a leaf with no element, before `_elementData` is set.
                if !render_by_location(ctx, &info, effect, out) {
                    return EffectIntents::default();
                }
            }
        }
        effect.cleanup_render();
    }
    data.unwrap_or_default()
}

/// `(BufferWi, BufferHt)` of the display element, for editor rules that size things to the targets. The orientation and
/// positioning are this effect's own, by the names `render_pixel_effect` reads.
pub fn display_element_size(t: &crate::framework::TargetInfo, p: &Params) -> (i32, i32) {
    let (orientation, positioning) = orientation_and_positioning(p);
    display_element_size_as(t, orientation, positioning)
}

/// `display_element_size` for a given orientation and positioning.
pub fn display_element_size_as(t: &crate::framework::TargetInfo, orientation: StringOrientation, positioning: TargetPositioning) -> (i32, i32) {
    let info = configure_in(t.tree, t.targets, orientation, positioning, t.start_time, t.time_span, 0);
    (info.buffer_wi(), info.buffer_ht())
}

/// `ConfigureDisplayElementSize`.
fn configure(ctx: &RenderCtx, targets: &[NodeId], orientation: StringOrientation, positioning: TargetPositioning) -> PixelInfo {
    configure_in(ctx.tree, targets, orientation, positioning, ctx.start_time, ctx.time_span, ctx.frame_ms)
}

fn configure_in(
    tree: &vixen_core::ElementTree,
    targets: &[NodeId],
    orientation: StringOrientation,
    positioning: TargetPositioning,
    start_time: TimeSpan,
    time_span: TimeSpan,
    frame_time: i32,
) -> PixelInfo {
    let mut info = PixelInfo {
        string_orientation: orientation,
        target_positioning: positioning,
        string_pixel_counts: Vec::new(),
        string_count: 0,
        max_pixels_per_string: 0,
        element_locations: Vec::new(),
        start_time,
        time_span,
        frame_time,
        raw_ht: 0,
        raw_wi: 0,
        raw_ht_offset: 0,
        raw_wi_offset: 0,
    };
    // `CalculateStringCounts` runs for every positioning type.
    let nodes = tree.leaf_parents(targets);
    info.string_pixel_counts = nodes.iter().map(|n| tree.element_count(*n) as i32).collect();
    info.string_count = nodes.len() as i32;
    info.max_pixels_per_string = info.string_pixel_counts.iter().copied().max().unwrap_or(0).max(0);
    match positioning {
        TargetPositioning::Strings => {
            info.raw_ht = info.string_count;
            info.raw_wi = info.max_pixels_per_string;
        }
        TargetPositioning::Locations => {
            let mut locs = Vec::new();
            for t in targets {
                for leaf in tree.leaves(*t) {
                    let (x, y) = tree.location(leaf);
                    locs.push(ElementLocation { x, y, node: leaf });
                }
            }
            let x_max = locs.iter().map(|l| l.x).max().unwrap_or(0);
            let x_min = locs.iter().map(|l| l.x).min().unwrap_or(0);
            let y_max = locs.iter().map(|l| l.y).max().unwrap_or(0);
            let y_min = locs.iter().map(|l| l.y).min().unwrap_or(0);
            info.raw_wi = (y_max - y_min) + 1;
            info.raw_ht = (x_max - x_min) + 1;
            info.raw_wi_offset = y_min;
            info.raw_ht_offset = x_min;
            info.element_locations = locs;
        }
    }
    info
}

/// `RenderNodesByStrings`.
fn render_by_strings<E: PixelEffect>(ctx: &RenderCtx, targets: &[NodeId], info: &PixelInfo, effect: &mut E, out: &mut EffectIntents) {
    let n_frames = info.number_frames();
    let (wi, ht) = (info.buffer_wi(), info.buffer_ht());
    if n_frames <= 0 || wi == 0 || ht == 0 {
        return;
    }
    let base = effect.base_color();
    let mut buffer = PixelFrameBuffer::with_base(wi, ht, base.as_ref().map(|b| b.0).unwrap_or(Color::TRANSPARENT));
    let buffer_size: usize = info.string_pixel_counts.iter().map(|c| (*c).max(0) as usize).sum();
    let frames = n_frames as usize;
    let mut pixels = vec![RgbValue::BLACK; buffer_size * frames];

    for frame in 0..n_frames {
        if ctx.cancelled() {
            return;
        }
        match &base {
            Some((_, curve)) => {
                let level = curve.get_value(info.interval_position(frame) * 100.0) / 100.0;
                buffer.clear_buffer_level(level);
            }
            None => buffer.clear_buffer(),
        }
        effect.render_effect(frame, info, &mut buffer);

        // Peel off this frame's pixels.
        let f = frame as usize;
        let mut i = 0usize;
        if info.string_orientation == StringOrientation::Horizontal {
            'outer_h: for y in 0..ht {
                let count = info.string_pixel_counts.get(y as usize).copied().unwrap_or(0);
                for x in 0..count {
                    if i >= buffer_size {
                        break 'outer_h;
                    }
                    pixels[i * frames + f] = RgbValue::new(buffer.get_color_at(x, y));
                    i += 1;
                }
            }
        } else {
            'outer_v: for x in 0..wi {
                let count = info.string_pixel_counts.get(x as usize).copied().unwrap_or(0);
                for y in 0..count {
                    if i >= buffer_size {
                        break 'outer_v;
                    }
                    pixels[i * frames + f] = RgbValue::new(buffer.get_color_at(x, y));
                    i += 1;
                }
            }
        }
    }

    // `nodes.SelectMany(node => node.Distinct()).Distinct()`.
    let mut elements: Vec<Uuid> = Vec::with_capacity(buffer_size);
    let mut seen = std::collections::HashSet::with_capacity(buffer_size);
    for t in targets {
        for e in ctx.tree.distinct_elements(*t) {
            if seen.insert(e) {
                elements.push(e);
            }
        }
    }

    let block: Arc<[RgbValue]> = Arc::from(pixels);
    let frame_ts = TimeSpan::from_ms_i(info.frame_time as i64);
    for (eidx, element) in elements.iter().enumerate() {
        if eidx >= buffer_size {
            break;
        }
        let array = FrameArray::new(block.clone(), eidx * frames, frames, frame_ts);
        out.add_intent_for_element(*element, Intent::new(IntentData::ArrayRgb(array), info.time_span), TimeSpan::ZERO);
    }
}

/// `RenderNodeByLocation`. False where the C# throws.
fn render_by_location<E: PixelEffect>(ctx: &RenderCtx, info: &PixelInfo, effect: &mut E, out: &mut EffectIntents) -> bool {
    let n_frames = info.number_frames();
    if n_frames <= 0 || info.buffer_wi() == 0 || info.buffer_ht() == 0 {
        return true;
    }
    if ctx.cancelled() {
        return true;
    }
    let mut buffer = PixelLocationFrameBuffer::new(&info.element_locations, n_frames);
    effect.render_effect_by_location(n_frames, info, &mut buffer);
    if ctx.cancelled() {
        return true;
    }

    let (block, index, frames) = buffer.into_block();
    let frame_ts = TimeSpan::from_ms_i(info.frame_time as i64);
    for loc in &info.element_locations {
        let Some(&slot) = index.get(&(loc.x, loc.y)) else { continue };
        let Some(element) = ctx.tree.node(loc.node).element_id else { return false };
        let array = FrameArray::new(block.clone(), slot as usize * frames, frames, frame_ts);
        out.add_intent_for_element(element, Intent::new(IntentData::ArrayRgb(array), info.time_span), TimeSpan::ZERO);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use vixen_core::{ElementTree, IntentData};

    struct Solid(Color);

    impl PixelEffect for Solid {
        fn render_effect(&mut self, _frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
            for x in 0..info.buffer_wi() {
                for y in 0..info.buffer_ht() {
                    buf.set_pixel(x, y, self.0);
                }
            }
        }
    }

    fn ctx<'a>(tree: &'a ElementTree, targets: &'a [NodeId], cancel: &'a AtomicBool) -> RenderCtx<'a> {
        RenderCtx { video_cache: None,
            tree,
            targets,
            start_time: TimeSpan::ZERO,
            time_span: TimeSpan::from_ms_i(1000),
            frame_ms: 50,
            marks: &[],
            audio: None,
            module_data_root: None,
            cancel,
        }
    }

    #[test]
    fn strings_buffer_shape_follows_orientation() {
        let mut t = ElementTree::new();
        let a = t.add_string("A", 10);
        let b = t.add_string("B", 10);
        let group = t.add_node(Uuid::new_v4(), "Tree", None);
        t.add_child(group, a);
        t.add_child(group, b);
        let targets = [group];
        let cancel = AtomicBool::new(false);
        let c = ctx(&t, &targets, &cancel);

        let v = configure(&c, &targets, StringOrientation::Vertical, TargetPositioning::Strings);
        assert_eq!((v.buffer_wi(), v.buffer_ht()), (2, 10));
        let h = configure(&c, &targets, StringOrientation::Horizontal, TargetPositioning::Strings);
        assert_eq!((h.buffer_wi(), h.buffer_ht()), (10, 2));
        assert_eq!(v.number_frames(), 20);
    }

    #[test]
    fn renders_a_static_array_for_every_element() {
        let mut t = ElementTree::new();
        let s = t.add_string("S", 5);
        let targets = [s];
        let cancel = AtomicBool::new(false);
        let c = ctx(&t, &targets, &cancel);
        let mut e = Solid(Color::RED);
        let out = render_pixel(&c, StringOrientation::Vertical, TargetPositioning::Strings, &mut e);

        assert_eq!(out.map.len(), 5);
        for id in t.distinct_elements(s) {
            let nodes = out.nodes_for(&id).unwrap();
            assert_eq!(nodes.len(), 1);
            match &nodes[0].intent.data {
                IntentData::ArrayRgb(a) => {
                    assert_eq!(a.len, 20);
                    assert_eq!(a.at(TimeSpan::from_ms_i(500)), RgbValue { r: 255, g: 0, b: 0 });
                }
                other => panic!("{other:?}"),
            }
        }
    }

    struct CountsLocationRenders(i32);

    impl PixelEffect for CountsLocationRenders {
        fn render_effect(&mut self, _frame: i32, _info: &PixelInfo, _buf: &mut PixelFrameBuffer) {}

        fn render_effect_by_location(&mut self, _num_frames: i32, _info: &PixelInfo, _buf: &mut PixelLocationFrameBuffer) {
            self.0 += 1;
        }
    }

    #[test]
    fn a_cancelled_location_render_stops_before_the_effect_runs() {
        let mut t = ElementTree::new();
        let s = t.add_string("S", 3);
        let targets = [s];
        let cancel = AtomicBool::new(true);
        let c = ctx(&t, &targets, &cancel);
        let mut e = CountsLocationRenders(0);
        let out = render_pixel(&c, StringOrientation::Vertical, TargetPositioning::Locations, &mut e);
        assert_eq!(e.0, 0);
        assert!(out.is_empty());
    }

    #[test]
    fn a_leaf_with_no_element_leaves_a_location_render_dark() {
        let mut t = ElementTree::new();
        let group = t.add_node(Uuid::new_v4(), "G", None);
        let a = t.add_element("A");
        let empty = t.add_node(Uuid::new_v4(), "Empty", None);
        t.add_child(group, a);
        t.add_child(group, empty);
        let targets = [group];
        let cancel = AtomicBool::new(false);
        let c = ctx(&t, &targets, &cancel);
        let mut e = CountsLocationRenders(0);
        let out = render_pixel(&c, StringOrientation::Vertical, TargetPositioning::Locations, &mut e);
        assert_eq!(e.0, 1);
        assert!(out.is_empty());
    }

    #[test]
    fn display_element_size_follows_the_values() {
        let mut t = ElementTree::new();
        let a = t.add_string("A", 10);
        let b = t.add_string("B", 10);
        let group = t.add_node(Uuid::new_v4(), "Tree", None);
        t.add_child(group, a);
        t.add_child(group, b);
        let targets = [group];
        let info = crate::testing::target_info(&t, &targets);
        // No orientation parameter reads as vertical, as render_pixel_effect does.
        let p = Params::defaults(&[]);
        assert_eq!(display_element_size(&info, &p), (2, 10));
    }

    #[test]
    fn transparent_pixels_render_black() {
        let mut t = ElementTree::new();
        let s = t.add_string("S", 2);
        let targets = [s];
        let cancel = AtomicBool::new(false);
        let c = ctx(&t, &targets, &cancel);
        let mut e = Solid(Color::TRANSPARENT);
        let out = render_pixel(&c, StringOrientation::Vertical, TargetPositioning::Strings, &mut e);
        let id = t.distinct_elements(s)[0];
        match &out.nodes_for(&id).unwrap()[0].intent.data {
            IntentData::ArrayRgb(a) => assert_eq!(a.at(TimeSpan::ZERO), RgbValue::BLACK),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn cancelled_render_returns_nothing() {
        let mut t = ElementTree::new();
        let s = t.add_string("S", 2);
        let targets = [s];
        let cancel = AtomicBool::new(true);
        let c = ctx(&t, &targets, &cancel);
        let mut e = Solid(Color::RED);
        let out = render_pixel(&c, StringOrientation::Vertical, TargetPositioning::Strings, &mut e);
        assert!(out.is_empty());
    }

    #[test]
    fn location_rendering_writes_each_occupied_cell() {
        struct ByLoc;
        impl PixelEffect for ByLoc {
            fn render_effect(&mut self, _f: i32, _i: &PixelInfo, _b: &mut PixelFrameBuffer) {}
            fn render_effect_by_location(&mut self, n: i32, info: &PixelInfo, buf: &mut PixelLocationFrameBuffer) {
                for f in 0..n {
                    buf.current_frame = f;
                    for l in &info.element_locations {
                        buf.set_pixel(l.x, l.y, Color::BLUE);
                    }
                }
            }
        }
        use vixen_core::element::{property_ids, PropertyData, PropertyInstance};
        let mut t = ElementTree::new();
        let g = t.add_string("S", 3);
        for (i, leaf) in t.leaves(g).into_iter().enumerate() {
            t.node_mut(leaf).properties.push(PropertyInstance {
                type_id: property_ids::LOCATION,
                instance_id: Uuid::new_v4(),
                data: PropertyData::Location { x: i as i32 * 10, y: 5, z: 0 },
            });
        }
        let targets = [g];
        let cancel = AtomicBool::new(false);
        let c = ctx(&t, &targets, &cancel);
        let mut e = ByLoc;
        let out = render_pixel(&c, StringOrientation::Vertical, TargetPositioning::Locations, &mut e);
        assert_eq!(out.map.len(), 3);
    }

    #[test]
    fn acceleration_is_identity_at_zero() {
        assert_eq!(calculate_acceleration(0.3, 0.0), 0.3);
    }

    #[test]
    fn add_degrees_wraps() {
        assert_eq!(add_degrees(350.0, 20.0), 10.0);
        assert_eq!(add_degrees(10.0, -20.0), 350.0);
    }

    #[test]
    fn angle_between_handles_wraparound() {
        assert!(is_angle_between(350.0, 10.0, 0.0));
        assert!(!is_angle_between(350.0, 10.0, 180.0));
    }
}
