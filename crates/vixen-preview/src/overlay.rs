//! Everything the design surface draws over the lights, as geometry, with the original's colours.
//!
//! The C# picks each light's edit colour in `PreviewLightBaseShape.DrawPixel` while drawing every frame. Here
//! those colours are runs over the scene's light indices, rebuilt only when something they depend on
//! changes, so reading the overlay while dragging costs nothing per light.
//!
//! Positions are in the [scene's units](crate::scene#coordinates): the renderer's view applies the zoom. Line
//! widths and square sizes are screen pixels at any zoom.

use crate::data::{PreviewData, ShapeId};
use crate::pixel::PreviewPixel;
use crate::point::{PointType, PreviewPoint};
use crate::scene::Scene;
use crate::shape::{RotateTypes, Shape, ShapeKind, StringType, SELECT_POINT_SIZE};
use crate::tools::{to_int32, Rect};
use std::collections::HashSet;
use std::ops::Range;
use std::sync::Arc;
use uuid::Uuid;
use vixen_core::Color;

/// `ThemeColorTable`'s preview colours, and the fixed colours the preview draws with.
pub mod theme {
    use vixen_core::Color;

    /// `ThemeColorTable.Selected`: LimeGreen.
    pub const SELECTED: Color = Color::rgb(50, 205, 50);
    /// `ThemeColorTable.Linked`: Turquoise.
    pub const LINKED: Color = Color::rgb(64, 224, 208);
    /// `ThemeColorTable.Unlinked`: White.
    pub const UNLINKED: Color = Color::WHITE;
    /// `ThemeColorTable.ElementSelected`: HotPink.
    pub const ELEMENT_SELECTED: Color = Color::rgb(255, 105, 180);
    /// `ThemeColorTable.Locked`: DarkRed.
    pub const LOCKED: Color = Color::rgb(139, 0, 0);
    /// `ThemeColorTable.Pixel1`: Yellow.
    pub const PIXEL1: Color = Color::YELLOW;
    /// Select points and the rotate handle.
    pub const HANDLE: Color = Color::WHITE;
    /// `Pens.White`.
    pub const RUBBER_BAND: Color = Color::WHITE;
    /// The outline `PreviewMovingHead.DrawRectangleOutline` puts around a selected fixture: HotPink.
    pub const FIXTURE_OUTLINE: Color = Color::rgb(255, 105, 180);
    /// `Pens.LightBlue`, the Show Info rectangle.
    pub const INFO_OUTLINE: Color = Color::rgb(173, 216, 230);
}

/// A straight line. Same fields as the renderer's, in scene coordinates.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct OverlayLine {
    pub a: [f32; 2],
    pub b: [f32; 2],
    pub color: Color,
    pub width: f32,
}

/// A select point: a hollow square. Same fields as the renderer's, in scene coordinates.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct OverlaySquare {
    pub center: [f32; 2],
    pub size: f32,
    pub color: Color,
}

/// A filled disc, the rotate handle. The radius is the real radius.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct OverlayCircle {
    pub center: [f32; 2],
    pub radius: f32,
    pub color: Color,
}

/// The rubber band, in scene coordinates.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct OverlayRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: Color,
}

impl OverlayRect {
    /// The four sides, for a renderer that only draws lines.
    pub fn lines(&self) -> [OverlayLine; 4] {
        let (x0, y0) = (self.x, self.y);
        let (x1, y1) = (x0 + self.width, y0 + self.height);
        let line = |a: [f32; 2], b: [f32; 2]| OverlayLine { a, b, color: self.color, width: 1.0 };
        [line([x0, y0], [x1, y0]), line([x1, y0], [x1, y1]), line([x1, y1], [x0, y1]), line([x0, y1], [x0, y0])]
    }
}

/// Consecutive lights with the same edit colour. `size_delta` is the extra size the first pixel of a pixel
/// string gets.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct LightRun {
    pub start: u32,
    pub len: u32,
    pub color: Color,
    pub size_delta: f32,
}

impl LightRun {
    #[inline]
    pub fn range(&self) -> Range<usize> {
        self.start as usize..(self.start + self.len) as usize
    }
}

/// A fixture's rectangle in scene coordinates and its beam colour.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FixtureOverlay {
    pub shape: ShapeId,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub beam_color: Color,
}

/// Show Info's label: a white box at the shape's top left with this text in black.
#[derive(Clone, Debug, PartialEq)]
pub struct InfoLabel {
    pub x: f32,
    pub y: f32,
    pub text: String,
}

/// See the [module documentation](self).
#[derive(Clone, Debug, Default)]
pub struct Overlay {
    pub lines: Vec<OverlayLine>,
    /// Select points.
    pub squares: Vec<OverlaySquare>,
    /// Rotate handles.
    pub circles: Vec<OverlayCircle>,
    pub rubber_band: Option<OverlayRect>,
    /// Every visible light's edit colour, including the element highlights. A light in no run is hidden.
    pub light_runs: Arc<Vec<LightRun>>,
    pub fixtures: Vec<FixtureOverlay>,
    pub labels: Vec<InfoLabel>,
}

impl Overlay {
    /// The lights highlighted because their element is selected in the element tree.
    pub fn element_highlights(&self) -> impl Iterator<Item = &LightRun> {
        self.light_runs.iter().filter(|r| r.color == theme::ELEMENT_SELECTED)
    }

    /// Writes each light's colour with `f`, for the renderer's colour buffer. Hidden lights get `hidden`.
    pub fn fill_light_colors<T: Copy>(&self, out: &mut [T], hidden: T, f: impl Fn(Color) -> T) {
        out.fill(hidden);
        for r in self.light_runs.iter() {
            if let Some(slice) = out.get_mut(r.range()) {
                slice.fill(f(r.color));
            }
        }
    }

    /// From the canvas pixels the C# draws these in, zoom included, to the scene's units.
    pub(crate) fn take_off_zoom(&mut self, zoom: f32) {
        if zoom == 1.0 {
            return;
        }
        let unzoom = |p: &mut [f32; 2]| *p = [p[0] / zoom, p[1] / zoom];
        for line in &mut self.lines {
            unzoom(&mut line.a);
            unzoom(&mut line.b);
        }
        for square in &mut self.squares {
            unzoom(&mut square.center);
        }
        for circle in &mut self.circles {
            unzoom(&mut circle.center);
        }
        if let Some(band) = &mut self.rubber_band {
            *band = OverlayRect { x: band.x / zoom, y: band.y / zoom, width: band.width / zoom, height: band.height / zoom, color: band.color };
        }
        for f in &mut self.fixtures {
            *f = FixtureOverlay { x: f.x / zoom, y: f.y / zoom, width: f.width / zoom, height: f.height / zoom, ..*f };
        }
        for label in &mut self.labels {
            label.x /= zoom;
            label.y /= zoom;
        }
    }
}

/// What decides a light's edit colour, apart from its shape.
pub(crate) struct RunContext<'a> {
    pub data: &'a mut PreviewData,
    pub scene: &'a Scene,
    pub selected_keys: HashSet<(i32, i32, i32, i32)>,
    pub highlighted: &'a HashSet<Uuid>,
    /// Goes up whenever `highlighted` changes, to tell which cached runs are stale.
    pub highlight_generation: u64,
    pub hide_locked: bool,
}

/// One display item's runs, relative to its first light, kept until something they depend on changes. A
/// selection change then costs a pass over the items instead of a pass over the lights.
#[derive(Clone, Debug)]
pub(crate) struct ItemRuns {
    selected: bool,
    locked: bool,
    highlight_generation: u64,
    runs: Vec<LightRun>,
}

struct RunBuilder {
    runs: Vec<LightRun>,
    next: u32,
}

impl RunBuilder {
    #[inline]
    fn push(&mut self, len: u32, color: Color, size_delta: f32) {
        if let Some(last) = self.runs.last_mut() {
            if last.start + last.len == self.next && last.color == color && last.size_delta == size_delta {
                last.len += len;
                self.next += len;
                return;
            }
        }
        self.runs.push(LightRun { start: self.next, len, color, size_delta });
        self.next += len;
    }
}

/// `DrawPixel`: the colour of one pixel, in edit mode.
#[inline]
fn pixel_color(p: &PreviewPixel, is_pixel_one: bool, string_type: StringType, selected: bool, locked: bool, highlighted: &HashSet<Uuid>) -> (Color, f32) {
    if !locked && string_type == StringType::Pixel && is_pixel_one {
        return (theme::PIXEL1, 2.0);
    }
    let color = if selected {
        theme::SELECTED
    } else if !p.node_id.is_nil() {
        if highlighted.contains(&p.node_id) {
            theme::ELEMENT_SELECTED
        } else if locked {
            theme::LOCKED
        } else {
            theme::LINKED
        }
    } else {
        theme::UNLINKED
    };
    (color, 0.0)
}

/// One `Draw` call: a group draws each of its strings with their own type and first pixel.
fn runs_for_shape(s: &Shape, selected: bool, locked: bool, highlighted: &HashSet<Uuid>, links: &[crate::scene::LightLink], at: &mut usize, b: &mut RunBuilder) {
    if matches!(s.kind(), ShapeKind::Custom { .. }) {
        if let Some(strings) = s.strings().filter(|s| !s.is_empty()) {
            for child in strings {
                runs_for_shape(child, selected, locked, highlighted, links, at, b);
            }
            return;
        }
    }
    let string_type = s.string_type();
    // IsPixelOne: the first string's first pixel, so no pixel is first if that string is empty.
    let mut first = s.strings().filter(|ss| !ss.is_empty()).is_none_or(|ss| ss[0].pixel_count() > 0);
    s.for_each_pixel(&mut |p| {
        let (color, delta) = pixel_color(p, first, string_type, selected, locked, highlighted);
        first = false;
        // All of a discrete pixel's lights take its colour.
        let start = *at;
        *at += 1;
        while *at < links.len() && links[*at].color_slot != 0 {
            *at += 1;
        }
        b.push((*at - start) as u32, color, delta);
    });
}

/// Rebuilds the runs: `displayItem.Draw` over the whole preview, minus the drawing.
pub(crate) fn build_light_runs(ctx: &mut RunContext) -> Vec<LightRun> {
    let mut b = RunBuilder { runs: Vec::new(), next: 0 };
    let scene = ctx.scene;
    for item in &mut ctx.data.display_items {
        let Some(range) = scene.range_of(item.id) else { continue };
        let s = &item.shape;
        if !s.is_light_shape() || range.is_empty() {
            continue;
        }
        // IsDisplayItemVisibleOnCanvas
        if ctx.hide_locked && s.locked {
            continue;
        }
        let selected = s.selected || ctx.selected_keys.contains(&s.bounds_key());
        let fresh = item.runs.as_ref().is_some_and(|r| r.selected == selected && r.locked == s.locked && r.highlight_generation == ctx.highlight_generation);
        if !fresh {
            let mut own = RunBuilder { runs: Vec::new(), next: 0 };
            let mut at = 0;
            runs_for_shape(s, selected, s.locked, ctx.highlighted, &scene.links[range.clone()], &mut at, &mut own);
            item.runs = Some(Box::new(ItemRuns { selected, locked: s.locked, highlight_generation: ctx.highlight_generation, runs: own.runs }));
        }
        b.next = range.start as u32;
        if let Some(r) = &item.runs {
            for run in &r.runs {
                b.push(run.len, run.color, run.size_delta);
            }
        }
    }
    b.runs
}

#[inline]
fn pos(p: PreviewPoint) -> [f32; 2] {
    [p.x as f32, p.y as f32]
}

/// `DrawSelectPoints`.
pub(crate) fn add_select_points(s: &Shape, o: &mut Overlay) {
    if let ShapeKind::CustomProp(c) = s.kind() {
        // It draws its drag points itself. The square's corner is at (x, y), so its centre is 3 further on, and
        // the rotate handle's circle is centred on that corner rather than the point. Deliberate, as in the C#.
        let half = SELECT_POINT_SIZE as f64 / 2.0;
        for p in &c.drag_points {
            let x = to_int32(p.x - half) as f32;
            let y = to_int32(p.y - half) as f32;
            match p.point_type {
                PointType::Size => o.squares.push(OverlaySquare { center: [x + half as f32, y + half as f32], size: SELECT_POINT_SIZE as f32, color: theme::HANDLE }),
                PointType::RotateHandle => o.circles.push(OverlayCircle { center: [x, y], radius: half as f32, color: theme::HANDLE }),
                _ => {}
            }
        }
        return;
    }
    // A flood's Draw never calls DrawSelectPoints.
    if matches!(s.kind(), ShapeKind::Flood) {
        return;
    }
    let Some(list) = s.select_points.as_ref() else { return };
    let zoom = s.zoom();
    for r in list {
        let p = s.point_of(*r);
        // The session runs the transform's side effect before the overlay is read, so this needs none.
        let t = s.transform_pure(p, zoom, RotateTypes::Clockwise);
        match p.point_type {
            PointType::Size | PointType::SizeTopLeft | PointType::SizeTopRight | PointType::SizeBottomLeft | PointType::SizeBottomRight => {
                o.squares.push(OverlaySquare { center: pos(t), size: SELECT_POINT_SIZE as f32, color: theme::HANDLE });
            }
            PointType::RotateHandle if s.show_rotation => {
                o.circles.push(OverlayCircle { center: pos(t), radius: (SELECT_POINT_SIZE / 2) as f32, color: theme::HANDLE });
            }
            _ => {}
        }
    }
}

/// The fixture graphic's rectangle, beam colour and selection outline.
pub(crate) fn add_fixture(id: ShapeId, s: &Shape, selected: bool, highlighted: &HashSet<Uuid>, o: &mut Overlay) {
    let ShapeKind::MovingHead(d) = s.kind() else { return };
    let beam_color = if selected {
        theme::SELECTED
    } else if highlighted.contains(&d.node_id) {
        theme::ELEMENT_SELECTED
    } else if s.locked {
        theme::LOCKED
    } else if !d.node_id.is_nil() {
        theme::LINKED
    } else {
        theme::UNLINKED
    };
    let zoom = s.zoom();
    let (l, t, r, b) = (s.left() as f64 * zoom, s.top() as f64 * zoom, s.right() as f64 * zoom, s.bottom() as f64 * zoom);
    o.fixtures.push(FixtureOverlay { shape: id, x: l as f32, y: t as f32, width: (r - l) as f32, height: (b - t) as f32, beam_color });
    if selected {
        let band = OverlayRect { x: l as i32 as f32, y: t as i32 as f32, width: (r - l) as i32 as f32, height: (b - t) as i32 as f32, color: theme::FIXTURE_OUTLINE };
        o.lines.extend(band.lines());
    }
}

/// `DisplayItem.DrawInfo`. It draws at the unzoomed Left and Top even when the view is zoomed, as the C# does.
pub(crate) fn add_info(s: &Shape, o: &mut Overlay) {
    if !s.is_light_shape() {
        return;
    }
    let Some(first) = s.first_pixel() else { return };
    let (l, t) = (s.left(), s.top());
    let rect = OverlayRect { x: l as f32, y: t as f32, width: s.right().wrapping_sub(l) as f32, height: s.bottom().wrapping_sub(t) as f32, color: theme::INFO_OUTLINE };
    o.lines.extend(rect.lines());
    o.labels.push(InfoLabel { x: l as f32, y: t as f32, text: format!("Z:{}", first.z) });
}

/// The rubber band as `RenderInForeground` draws it, while the mouse is captured with nothing singly selected.
pub(crate) fn rubber_band(rect: Rect) -> Option<OverlayRect> {
    (rect.width != 0 || rect.height != 0).then_some(OverlayRect { x: rect.x as f32, y: rect.y as f32, width: rect.width as f32, height: rect.height as f32, color: theme::RUBBER_BAND })
}
