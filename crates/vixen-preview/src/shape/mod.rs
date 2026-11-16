//! Port of Shapes/PreviewBaseShape.cs and PreviewLightBaseShape.cs, and the dispatch to each shape type.
//!
//! The C# keeps a class per shape with named `PreviewPoint` fields, and `_selectedPoint` is a reference to
//! one of them. Here a shape's points live in `Shape::points`, indexed by the per-type constants, and
//! `selected_point` is a [`PointRef`]. Everything else follows the C# method by method, including quirks
//! that are marked as deliberate. Anything that throws in C# (null points, bad indices) is a guarded
//! no-op here, because a C# exception must not become a panic.

mod arch;
mod cane;
mod custom;
mod custom_prop;
mod ellipse;
mod flood;
mod icicle;
mod line;
mod mega_tree;
mod moving_head;
mod multi_string;
mod net;
mod pixel_grid;
mod poly_line;
pub mod properties;
mod rectangle;
mod single;
mod star;
mod star_burst;
mod transform;
mod triangle;
pub(crate) mod xml;

#[cfg(test)]
mod tests;

pub use custom_prop::{CustomPropData, CustomPropLightNode};
pub use moving_head::{MountingPosition, MovingHeadData, YesNo};
pub use transform::RotateTypes;

use crate::pixel::PreviewPixel;
use crate::point::{PointType, PreviewPoint};
use crate::tools::{to_int32, Rect};
use crate::xml_util::XmlMemo;
use uuid::Uuid;
use vixen_core::{ElementTree, NodeId};
use vixen_xml::XElement;

/// `PreviewBaseShape.SelectPointSize`.
pub const SELECT_POINT_SIZE: i32 = 6;

/// `PreviewLightBaseShape.StringTypes`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum StringType {
    #[default]
    Standard,
    Pixel,
    Custom,
}

impl StringType {
    pub fn name(self) -> &'static str {
        match self {
            StringType::Standard => "Standard",
            StringType::Pixel => "Pixel",
            StringType::Custom => "Custom",
        }
    }

    pub fn parse(s: &str) -> Option<StringType> {
        match s.trim() {
            "Standard" => Some(StringType::Standard),
            "Pixel" => Some(StringType::Pixel),
            "Custom" => Some(StringType::Custom),
            _ => None,
        }
    }
}

/// `PreviewLightBaseShape.StringDirections`, and the shapes' own `Directions`, which have the same members.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum Direction {
    #[default]
    Clockwise,
    CounterClockwise,
}

impl Direction {
    pub fn name(self) -> &'static str {
        match self {
            Direction::Clockwise => "Clockwise",
            Direction::CounterClockwise => "CounterClockwise",
        }
    }

    pub fn parse(s: &str) -> Option<Direction> {
        match s.trim() {
            "Clockwise" => Some(Direction::Clockwise),
            "CounterClockwise" => Some(Direction::CounterClockwise),
            _ => None,
        }
    }
}

/// `PreviewPixelGrid.StringOrientations`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum StringOrientation {
    #[default]
    Vertical,
    Horizontal,
}

impl StringOrientation {
    pub fn name(self) -> &'static str {
        match self {
            StringOrientation::Vertical => "Vertical",
            StringOrientation::Horizontal => "Horizontal",
        }
    }

    pub fn parse(s: &str) -> Option<StringOrientation> {
        match s.trim() {
            "Vertical" => Some(StringOrientation::Vertical),
            "Horizontal" => Some(StringOrientation::Horizontal),
            _ => None,
        }
    }
}

/// The keyboard state `Control.ModifierKeys` gives the C#. Hosts on macOS decide what maps to `ctrl`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Modifiers {
    pub const NONE: Modifiers = Modifiers { ctrl: false, shift: false, alt: false };
    pub const CTRL: Modifiers = Modifiers { ctrl: true, shift: false, alt: false };
    pub const SHIFT: Modifiers = Modifiers { ctrl: false, shift: true, alt: false };

    /// `Control.ModifierKeys == Keys.Control`.
    #[inline]
    pub fn only_ctrl(self) -> bool {
        self.ctrl && !self.shift && !self.alt
    }

    /// `Control.ModifierKeys == Keys.Shift`.
    #[inline]
    pub fn only_shift(self) -> bool {
        self.shift && !self.ctrl && !self.alt
    }
}

/// Which point `_selectedPoint` refers to.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PointRef {
    /// An entry of `Shape::points`.
    Pt(usize),
    RotateHandle,
    RotationAxis,
    /// A point that belongs to nothing, like the `new PreviewPoint(0, 0)` a paste passes in, or the mouse
    /// point the Custom Prop hands back from `PointInSelectPoint`.
    Detached,
}

/// The argument of `SetSelectPoint`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SelectArg {
    /// `SetSelectPoint(null)`: about to move the shape.
    Null,
    /// `SetSelectPoint(new PreviewPoint(0, 0))`: the reset a paste uses.
    Reset,
    /// `SetSelectPoint(point)`: about to drag this handle.
    Point(PointRef),
}

/// A node in the element tree, as the shape constructors' `ElementNode selectedNode`.
#[derive(Copy, Clone)]
pub struct Sel<'a> {
    pub tree: &'a ElementTree,
    pub node: NodeId,
}

impl<'a> Sel<'a> {
    pub fn new(tree: &'a ElementTree, node: NodeId) -> Sel<'a> {
        Sel { tree, node }
    }

    pub fn from_uuid(tree: &'a ElementTree, id: Uuid) -> Option<Sel<'a>> {
        tree.find(&id).map(|node| Sel { tree, node })
    }

    #[inline]
    pub fn uuid(&self) -> Uuid {
        self.tree.node(self.node).id
    }

    #[inline]
    pub fn is_leaf(&self) -> bool {
        self.tree.is_leaf(self.node)
    }

    #[inline]
    pub fn has_element(&self) -> bool {
        self.tree.node(self.node).element_id.is_some()
    }

    pub fn children(&self) -> Vec<Sel<'a>> {
        let tree = self.tree;
        tree.node(self.node).children.iter().map(|c| Sel { tree, node: *c }).collect()
    }

    /// `PreviewTools.GetLeafNodes`.
    pub fn leaf_children(&self) -> Vec<Sel<'a>> {
        self.children().into_iter().filter(|c| c.is_leaf()).collect()
    }

    /// `PreviewTools.GetParentNodes`.
    pub fn parent_children(&self) -> Vec<Sel<'a>> {
        self.children().into_iter().filter(|c| !c.is_leaf()).collect()
    }

    /// `node.Count()`: the elements under this node, counting a repeated leaf each time.
    pub fn element_count(&self) -> usize {
        self.tree.element_count(self.node)
    }
}

/// Type-specific members. Points are in `Shape::points`.
#[derive(Clone, Debug)]
pub enum ShapeKind {
    Line { add_start_padding: bool, add_end_padding: bool, light_count: i32 },
    PolyLine { create_default_pixels: bool },
    Arch,
    Cane { arch_pixel_count: i32, line_pixel_count: i32, just_placed: bool },
    Rectangle { direction: Direction, lock_xy: bool },
    Ellipse,
    Triangle { bottom_right_start: Option<PreviewPoint> },
    Star { point_count: i32, pixel_count: i32, inside_size: i32, string_direction: Direction, xy_rotation: i32, pixels_per_point: i32 },
    StarBurst { direction: Direction, xy_rotation: i32, inner_circle_size: i32 },
    Net { pixel_spacing: i32, lock_xy: bool, initially_assigned_node: Option<Uuid>, loaded_pixels: Option<Vec<PreviewPixel>> },
    MegaTree { string_count: i32, top_height: i32, top_width: i32, base_height: i32, lights_per_string: i32, degrees: i32, degree_offset: i32 },
    MultiString { string_count: i32, input_elements: Option<Uuid> },
    Icicle { string_count: i32, vertical_spacing: i32, creating: bool, initial_node: Option<Uuid> },
    PixelGrid { string_count: i32, lights_per_string: i32, string_orientation: StringOrientation },
    Single,
    Flood,
    Custom { start_width: i32, start_height: i32 },
    CustomProp(Box<CustomPropData>),
    MovingHead(Box<MovingHeadData>),
    /// A shape type this port does not know, or a known one it could not read. Written back unchanged.
    Unknown(Box<XElement>),
}

/// A preview shape: `PreviewBaseShape`, `PreviewLightBaseShape` and the derived class in one struct.
#[derive(Clone, Debug)]
pub struct Shape {
    // PreviewBaseShape data members.
    pub(crate) name: Option<String>,
    pub(crate) use_common_location: bool,
    pub(crate) locked: bool,
    pub(crate) rotation_angle: i32,

    // PreviewBaseShape state.
    pub(crate) rotation_axis: Option<PreviewPoint>,
    pub(crate) rotate_handle: Option<PreviewPoint>,
    pub(crate) show_rotation: bool,
    pub(crate) selected: bool,
    pub(crate) select_points: Option<Vec<PointRef>>,
    pub(crate) selected_point: Option<PointRef>,
    pub(crate) detached: PreviewPoint,
    pub(crate) zoom_level: f64,
    pub(crate) creating: bool,
    pub(crate) background_alpha: i32,

    // PreviewLightBaseShape.
    pub(crate) pixels: Vec<PreviewPixel>,
    pub(crate) strings: Option<Vec<Shape>>,
    pub(crate) string_type: StringType,
    pub(crate) pixel_size: i32,
    pub(crate) connect_standard_strings: bool,

    // The derived class's PreviewPoint fields, and their copies taken by SetSelectPoint.
    pub(crate) points: Vec<PreviewPoint>,
    pub(crate) start_points: Vec<PreviewPoint>,
    /// Bit i set: `points[i]` is null in the C#, and is written as nil.
    pub(crate) point_nil_mask: u8,

    pub(crate) kind: ShapeKind,
    pub(crate) memo: Option<Box<XmlMemo>>,
}

// Point indices shared by the two and four cornered shapes.
pub(crate) const TL: usize = 0;
pub(crate) const BR: usize = 1;
pub(crate) const TR: usize = 2;
pub(crate) const BL: usize = 3;

impl Shape {
    /// The state after a C# constructor's field initialisers.
    pub(crate) fn constructed(kind: ShapeKind) -> Shape {
        Shape {
            name: None,
            use_common_location: false,
            locked: false,
            rotation_angle: 0,
            rotation_axis: None,
            rotate_handle: None,
            show_rotation: true,
            selected: false,
            select_points: None,
            selected_point: None,
            detached: PreviewPoint::default(),
            zoom_level: 1.0,
            creating: false,
            background_alpha: 0,
            pixels: Vec::new(),
            strings: None,
            string_type: StringType::Standard,
            pixel_size: 3,
            connect_standard_strings: false,
            points: Vec::new(),
            start_points: Vec::new(),
            point_nil_mask: 0,
            kind,
            memo: None,
        }
    }

    /// The state `DataContractSerializer` starts from: it runs neither constructors nor field initialisers.
    pub(crate) fn uninitialized(kind: ShapeKind) -> Shape {
        Shape { show_rotation: false, zoom_level: 0.0, pixel_size: 0, ..Shape::constructed(kind) }
    }

    /// A shape kept raw.
    pub(crate) fn unknown(raw: &XElement) -> Shape {
        Shape::uninitialized(ShapeKind::Unknown(Box::new(raw.clone())))
    }

    // ----- identity

    pub fn kind(&self) -> &ShapeKind {
        &self.kind
    }

    /// The shape's XML, as a display item's `Shape` member.
    pub fn to_xml(&self) -> XElement {
        self.to_xml_named("Shape")
    }

    /// Reads a `Shape` member. Anything it cannot understand stays raw.
    pub fn from_xml(e: &XElement) -> Shape {
        xml::shape_from_xml(e)
    }

    /// `TypeName`, the display name for the shape's type.
    pub fn class_name(&self) -> &'static str {
        match &self.kind {
            ShapeKind::Single => "PreviewSingle",
            ShapeKind::Line { .. } => "PreviewLine",
            ShapeKind::Rectangle { .. } => "PreviewRectangle",
            ShapeKind::Ellipse => "PreviewEllipse",
            ShapeKind::Triangle { .. } => "PreviewTriangle",
            ShapeKind::Flood => "PreviewFlood",
            ShapeKind::StarBurst { .. } => "PreviewStarBurst",
            ShapeKind::PixelGrid { .. } => "PreviewPixelGrid",
            ShapeKind::PolyLine { .. } => "PreviewPolyLine",
            ShapeKind::MultiString { .. } => "PreviewMultiString",
            ShapeKind::CustomProp(_) => "PreviewCustomProp",
            ShapeKind::Arch => "PreviewArch",
            ShapeKind::Cane { .. } => "PreviewCane",
            ShapeKind::Icicle { .. } => "PreviewIcicle",
            ShapeKind::MegaTree { .. } => "PreviewMegaTree",
            ShapeKind::Net { .. } => "PreviewNet",
            ShapeKind::Star { .. } => "PreviewStar",
            ShapeKind::MovingHead(_) => "PreviewMovingHead",
            ShapeKind::Custom { .. } => "PreviewCustom",
            ShapeKind::Unknown(_) => "DisplayItem",
        }
    }

    pub fn type_name(&self) -> &'static str {
        match &self.kind {
            ShapeKind::Line { .. } => "Line",
            ShapeKind::PolyLine { .. } => "Poly Line",
            ShapeKind::Arch => "Arch",
            ShapeKind::Cane { .. } => "Cane",
            ShapeKind::Rectangle { .. } => "Rectangle",
            ShapeKind::Ellipse => "Ellipse",
            ShapeKind::Triangle { .. } => "Triangle",
            ShapeKind::Star { .. } => "Star",
            ShapeKind::StarBurst { .. } => "Star Burst",
            ShapeKind::Net { .. } => "Net",
            ShapeKind::MegaTree { .. } => "Mega Tree",
            ShapeKind::MultiString { .. } => "Multi-String",
            ShapeKind::Icicle { .. } => "Icicle",
            ShapeKind::PixelGrid { .. } => "Pixel Grid",
            ShapeKind::Single => "Single",
            ShapeKind::Flood => "Flood",
            ShapeKind::Custom { .. } => "Custom Template",
            ShapeKind::CustomProp(_) => "Custom Prop",
            ShapeKind::MovingHead(_) => "Intelligent Fixture",
            ShapeKind::Unknown(_) => "Shape",
        }
    }

    /// `DisplayItem.IsLightShape()`. A shape of unknown type holds no lights this port can see.
    #[inline]
    pub fn is_light_shape(&self) -> bool {
        !matches!(self.kind, ShapeKind::MovingHead(_) | ShapeKind::Unknown(_))
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn locked(&self) -> bool {
        self.locked
    }

    pub fn set_locked(&mut self, locked: bool) {
        self.locked = locked;
    }

    pub fn use_common_location(&self) -> bool {
        self.use_common_location
    }

    pub fn rotation_angle(&self) -> i32 {
        self.rotation_angle
    }

    pub fn is_selected(&self) -> bool {
        self.selected
    }

    pub fn is_creating(&self) -> bool {
        self.creating
    }

    /// The geometry points, for a host that wants them. Which is which depends on the shape type.
    pub fn points(&self) -> &[PreviewPoint] {
        &self.points
    }

    /// `ZoomLevel` getter.
    #[inline]
    pub fn zoom(&self) -> f64 {
        if self.zoom_level <= 0.0 {
            1.0
        } else {
            self.zoom_level
        }
    }

    // ----- string type and pixel size

    /// `StringType` getter. Net and Flood are always Standard, and Multi-String is always Pixel.
    #[inline]
    pub fn string_type(&self) -> StringType {
        match self.kind {
            ShapeKind::Net { .. } | ShapeKind::Flood => StringType::Standard,
            ShapeKind::MultiString { .. } => StringType::Pixel,
            _ => self.string_type,
        }
    }

    /// `StringType` setter.
    pub fn set_string_type(&mut self, value: StringType) {
        match self.kind {
            ShapeKind::Net { .. } | ShapeKind::Flood => self.string_type = value,
            ShapeKind::MultiString { .. } => self.set_string_type_base(StringType::Pixel),
            ShapeKind::CustomProp(_) => {
                if value == StringType::Custom {
                    self.set_string_type_base(value);
                }
            }
            _ => self.set_string_type_base(value),
        }
    }

    fn set_string_type_base(&mut self, value: StringType) {
        self.string_type = value;
        if let Some(strings) = &mut self.strings {
            for line in strings {
                line.set_string_type(value);
            }
        }
    }

    /// `_isHighPrecision`.
    #[inline]
    pub(crate) fn is_high_precision(&self) -> bool {
        self.string_type == StringType::Custom && !matches!(self.kind, ShapeKind::Net { .. } | ShapeKind::Flood)
    }

    /// `PixelSize` getter.
    #[inline]
    pub fn pixel_size(&self) -> i32 {
        match &self.kind {
            ShapeKind::CustomProp(c) => c.pixel_size,
            _ => self.pixel_size,
        }
    }

    /// `PixelSize` setter.
    pub fn set_pixel_size(&mut self, value: i32) {
        match &mut self.kind {
            ShapeKind::Custom { .. } => {} // Do nothing, as the C# does.
            ShapeKind::CustomProp(c) => c.pixel_size = value,
            ShapeKind::Flood => {
                self.pixel_size = value;
                if let Some(p) = self.pixels.first_mut() {
                    p.size = 3;
                }
            }
            _ => {
                if value > 0 {
                    self.pixel_size = value;
                } else if self.pixel_size <= 0 {
                    // set a reasonable default.
                    self.pixel_size = 3;
                }
                self.resize_pixels();
            }
        }
    }

    /// `ResizePixels`.
    pub(crate) fn resize_pixels(&mut self) {
        match self.kind {
            ShapeKind::Custom { .. } => return, // Gotta override this. We don't want to do anything
            ShapeKind::CustomProp(_) | ShapeKind::MovingHead(_) | ShapeKind::Unknown(_) => return,
            _ => {}
        }
        let size = self.pixel_size;
        if let Some(strings) = self.strings.as_mut().filter(|s| !s.is_empty()) {
            for s in strings {
                s.set_pixel_size(size);
            }
        } else {
            for p in &mut self.pixels {
                p.size = size;
            }
        }
    }

    /// `MatchPixelSize`.
    pub fn match_pixel_size(&mut self, other: &Shape) {
        if let ShapeKind::CustomProp(c) = &mut self.kind {
            let Some(new_size) = other.first_pixel().map(|p| p.size) else { return };
            for p in c.prop_pixels.iter_mut().chain(c.view_pixels.iter_mut()) {
                p.size = new_size;
            }
        } else {
            self.set_pixel_size(other.pixel_size());
        }
    }

    /// `ResizePixelsBy`.
    pub fn resize_pixels_by(&mut self, value: i32) {
        if let ShapeKind::CustomProp(c) = &mut self.kind {
            for p in c.prop_pixels.iter_mut().chain(c.view_pixels.iter_mut()) {
                let new_size = p.size + value;
                p.size = if new_size > 0 { new_size } else { 1 };
            }
        } else {
            let new_size = self.pixel_size() + value;
            self.set_pixel_size(if new_size > 0 { new_size } else { 1 });
        }
    }

    // ----- pixels

    /// Runs `f` on every pixel of `Pixels`, in order, with no intermediate list.
    #[inline]
    pub fn for_each_pixel(&self, f: &mut impl FnMut(&PreviewPixel)) {
        match &self.kind {
            ShapeKind::CustomProp(c) => c.view_pixels.iter().for_each(f),
            ShapeKind::Rectangle { .. } | ShapeKind::Triangle { .. } | ShapeKind::StarBurst { .. } => {
                // These read each string's `_pixels` directly, and have no pixels without strings.
                if let Some(strings) = &self.strings {
                    for s in strings {
                        s.pixels.iter().for_each(&mut *f);
                    }
                }
            }
            ShapeKind::MovingHead(_) | ShapeKind::Unknown(_) => {}
            _ => match self.strings.as_ref().filter(|s| !s.is_empty()) {
                Some(strings) => {
                    for s in strings {
                        s.for_each_pixel(f);
                    }
                }
                None => self.pixels.iter().for_each(f),
            },
        }
    }

    /// Editing pixels through this does not update a scene: go through [`edit_shape`] in a session.
    ///
    /// [`edit_shape`]: crate::session::DesignSession::edit_shape
    pub fn for_each_pixel_mut(&mut self, f: &mut impl FnMut(&mut PreviewPixel)) {
        match &mut self.kind {
            ShapeKind::CustomProp(c) => c.view_pixels.iter_mut().for_each(f),
            ShapeKind::Rectangle { .. } | ShapeKind::Triangle { .. } | ShapeKind::StarBurst { .. } => {
                if let Some(strings) = &mut self.strings {
                    for s in strings {
                        s.pixels.iter_mut().for_each(&mut *f);
                    }
                }
            }
            ShapeKind::MovingHead(_) | ShapeKind::Unknown(_) => {}
            _ => match self.strings.as_mut().filter(|s| !s.is_empty()) {
                Some(strings) => {
                    for s in strings {
                        s.for_each_pixel_mut(f);
                    }
                }
                None => self.pixels.iter_mut().for_each(f),
            },
        }
    }

    /// `Pixels.Count`.
    pub fn pixel_count(&self) -> usize {
        match &self.kind {
            ShapeKind::CustomProp(c) => c.view_pixels.len(),
            ShapeKind::Rectangle { .. } | ShapeKind::Triangle { .. } | ShapeKind::StarBurst { .. } => {
                self.strings.as_ref().map_or(0, |ss| ss.iter().map(|s| s.pixels.len()).sum())
            }
            ShapeKind::MovingHead(_) | ShapeKind::Unknown(_) => 0,
            _ => match self.strings.as_ref().filter(|s| !s.is_empty()) {
                Some(strings) => strings.iter().map(Shape::pixel_count).sum(),
                None => self.pixels.len(),
            },
        }
    }

    /// A copy of `Pixels`, for inspection. Not for hot paths.
    pub fn pixels_vec(&self) -> Vec<PreviewPixel> {
        let mut out = Vec::with_capacity(self.pixel_count());
        self.for_each_pixel(&mut |p| out.push(p.clone()));
        out
    }

    /// `Pixels[0]`.
    pub fn first_pixel(&self) -> Option<&PreviewPixel> {
        match &self.kind {
            ShapeKind::CustomProp(c) => c.view_pixels.first(),
            ShapeKind::Rectangle { .. } | ShapeKind::Triangle { .. } | ShapeKind::StarBurst { .. } => {
                self.strings.as_ref()?.iter().find_map(|s| s.pixels.first())
            }
            ShapeKind::MovingHead(_) | ShapeKind::Unknown(_) => None,
            _ => match self.strings.as_ref().filter(|s| !s.is_empty()) {
                Some(strings) => strings.iter().find_map(Shape::first_pixel),
                None => self.pixels.first(),
            },
        }
    }

    /// `Strings` as the "Linked Elements" editor sees it: the string shapes, or the shape itself.
    pub fn strings(&self) -> Option<&[Shape]> {
        self.strings.as_deref()
    }

    /// `AddPixel(x, y)`.
    pub(crate) fn add_pixel(&mut self, x: i32, y: i32) -> &mut PreviewPixel {
        let size = self.pixel_size();
        self.pixels.push(PreviewPixel::new(x, y, 0, size));
        let last = self.pixels.len() - 1;
        &mut self.pixels[last]
    }

    /// `PreviewLightBaseShape.AddPixels`.
    pub(crate) fn add_pixels_for_node(&mut self, node: Option<Sel>, light_count: i32) {
        if self.pixels.is_empty() {
            // Just add the pixels, they will get laid out next
            for _ in 0..light_count {
                let p = self.add_pixel(10, 10);
                if let Some(n) = node.filter(|n| n.is_leaf()) {
                    p.node_id = n.uuid();
                }
            }
        }
    }

    /// `SetPixelZoom`. Unlike the transform below it truncates.
    pub(crate) fn set_pixel_zoom(&mut self) {
        let zoom = self.zoom();
        if zoom == 1.0 {
            return;
        }
        for p in &mut self.pixels {
            p.x = (p.x as f64 * zoom) as i32;
            p.y = (p.y as f64 * zoom) as i32;
        }
    }

    // ----- bounds

    /// `Left` getter.
    pub fn left(&self) -> i32 {
        let p = &self.points;
        match &self.kind {
            ShapeKind::Line { .. } | ShapeKind::Icicle { .. } => p[0].x.min(p[1].x),
            ShapeKind::Arch | ShapeKind::Ellipse | ShapeKind::Star { .. } | ShapeKind::StarBurst { .. } | ShapeKind::Cane { .. } => p[TL].x.min(p[BR].x),
            ShapeKind::Rectangle { .. } | ShapeKind::Net { .. } | ShapeKind::MovingHead(_) => p[TL].x.min(p[TR].x.min(p[BL].x.min(p[BR].x))),
            ShapeKind::PolyLine { .. } | ShapeKind::MultiString { .. } => p.iter().fold(i32::MAX, |m, q| m.min(q.x)),
            // Point 1 takes no part in a triangle's Left and Right. Deliberate, as in the C#.
            ShapeKind::Triangle { .. } => p[triangle::P2].x.min(p[triangle::P3].x),
            ShapeKind::MegaTree { .. } | ShapeKind::PixelGrid { .. } => p[TL].x,
            ShapeKind::Single | ShapeKind::Flood => p[0].x,
            ShapeKind::Custom { .. } => self.strings.as_deref().unwrap_or(&[]).iter().fold(i32::MAX, |m, s| m.min(s.left())),
            ShapeKind::CustomProp(c) => c.bounds.x as i32,
            ShapeKind::Unknown(_) => 0,
        }
    }

    /// `Top` getter.
    pub fn top(&self) -> i32 {
        let p = &self.points;
        match &self.kind {
            ShapeKind::Line { .. } | ShapeKind::Icicle { .. } => p[0].y.min(p[1].y),
            ShapeKind::Arch | ShapeKind::Ellipse | ShapeKind::Star { .. } | ShapeKind::StarBurst { .. } | ShapeKind::Cane { .. } => p[TL].y.min(p[BR].y),
            ShapeKind::Rectangle { .. } | ShapeKind::Net { .. } | ShapeKind::MovingHead(_) => p[TL].y.min(p[TR].y.min(p[BL].y.min(p[BR].y))),
            ShapeKind::PolyLine { .. } | ShapeKind::MultiString { .. } => p.iter().fold(i32::MAX, |m, q| m.min(q.y)),
            // Point 2 takes no part in a triangle's Top: the C# compares point 1 with itself. Deliberate.
            ShapeKind::Triangle { .. } => p[triangle::P1].y.min(p[triangle::P1].y.min(p[triangle::P3].y)),
            ShapeKind::MegaTree { .. } | ShapeKind::PixelGrid { .. } => p[TL].y,
            ShapeKind::Single | ShapeKind::Flood => p[0].y,
            ShapeKind::Custom { .. } => self.strings.as_deref().unwrap_or(&[]).iter().fold(i32::MAX, |m, s| m.min(s.top())),
            ShapeKind::CustomProp(c) => c.bounds.y as i32,
            ShapeKind::Unknown(_) => 0,
        }
    }

    /// `Right` getter.
    pub fn right(&self) -> i32 {
        let p = &self.points;
        match &self.kind {
            ShapeKind::Line { .. } | ShapeKind::Icicle { .. } => p[0].x.max(p[1].x),
            ShapeKind::Arch | ShapeKind::Ellipse | ShapeKind::Star { .. } | ShapeKind::Cane { .. } => p[TL].x.max(p[BR].x),
            ShapeKind::StarBurst { .. } => p[TR].x,
            ShapeKind::Rectangle { .. } | ShapeKind::Net { .. } | ShapeKind::MovingHead(_) => p[TL].x.max(p[TR].x.max(p[BL].x.max(p[BR].x))),
            ShapeKind::PolyLine { .. } | ShapeKind::MultiString { .. } => p.iter().fold(0, |m, q| m.max(q.x)),
            ShapeKind::Triangle { .. } => p[triangle::P2].x.max(p[triangle::P3].x),
            ShapeKind::MegaTree { .. } | ShapeKind::PixelGrid { .. } => p[BR].x,
            ShapeKind::Single | ShapeKind::Flood => p[0].x,
            ShapeKind::Custom { .. } => self.strings.as_deref().unwrap_or(&[]).iter().fold(0, |m, s| m.max(s.right())),
            ShapeKind::CustomProp(c) => c.bounds.right() as i32,
            ShapeKind::Unknown(_) => 0,
        }
    }

    /// `Bottom` getter.
    pub fn bottom(&self) -> i32 {
        let p = &self.points;
        match &self.kind {
            ShapeKind::Line { .. } | ShapeKind::Icicle { .. } => p[0].y.max(p[1].y),
            ShapeKind::Arch | ShapeKind::Ellipse | ShapeKind::Star { .. } | ShapeKind::Cane { .. } => p[TL].y.max(p[BR].y),
            ShapeKind::StarBurst { .. } => p[BR].y,
            ShapeKind::Rectangle { .. } | ShapeKind::Net { .. } | ShapeKind::MovingHead(_) => p[TL].y.max(p[TR].y.max(p[BL].y.max(p[BR].y))),
            ShapeKind::PolyLine { .. } | ShapeKind::MultiString { .. } => p.iter().fold(0, |m, q| m.max(q.y)),
            ShapeKind::Triangle { .. } => p[triangle::P1].y.max(p[triangle::P2].y.max(p[triangle::P3].y)),
            ShapeKind::MegaTree { .. } | ShapeKind::PixelGrid { .. } => p[BR].y,
            ShapeKind::Single | ShapeKind::Flood => p[0].y,
            ShapeKind::Custom { .. } => self.strings.as_deref().unwrap_or(&[]).iter().fold(0, |m, s| m.max(s.bottom())),
            ShapeKind::CustomProp(c) => c.bounds.bottom() as i32,
            ShapeKind::Unknown(_) => 0,
        }
    }

    /// `Center`.
    #[inline]
    pub fn center(&self) -> PreviewPoint {
        let (l, t) = (self.left(), self.top());
        let x = l.wrapping_add(self.right().wrapping_sub(l) / 2);
        let y = t.wrapping_add(self.bottom().wrapping_sub(t) / 2);
        PreviewPoint::new(x, y)
    }

    /// The four values `DisplayItem.Equals` compares.
    #[inline]
    pub(crate) fn bounds_key(&self) -> (i32, i32, i32, i32) {
        (self.left(), self.right(), self.top(), self.bottom())
    }

    fn translate_points(&mut self, dx: i32, dy: i32) {
        for p in &mut self.points {
            p.translate(dx, dy);
        }
    }

    /// Moves the given points.
    fn translate_some(&mut self, which: &[usize], dx: i32, dy: i32) {
        for i in which {
            if let Some(p) = self.points.get_mut(*i) {
                p.translate(dx, dy);
            }
        }
    }

    /// `Top` setter.
    pub fn set_top(&mut self, value: i32) {
        let delta = self.top().wrapping_sub(value);
        match &self.kind {
            ShapeKind::Line { .. } | ShapeKind::Icicle { .. } | ShapeKind::Arch | ShapeKind::Star { .. } | ShapeKind::Ellipse => {
                self.translate_some(&[0, 1], 0, delta.wrapping_neg());
            }
            ShapeKind::Cane { .. } => self.translate_some(&[TL, BR, cane::ARCH], 0, delta.wrapping_neg()),
            ShapeKind::Rectangle { .. } | ShapeKind::Net { .. } | ShapeKind::MovingHead(_) | ShapeKind::PolyLine { .. } | ShapeKind::MultiString { .. } => {
                self.translate_points(0, delta.wrapping_neg());
            }
            ShapeKind::Triangle { .. } => {
                use triangle::{P1, P2, P3};
                if self.points[P1].y < self.points[P2].y {
                    // Point 1 is the smallest
                    let d = self.points[P1].y.wrapping_sub(value);
                    self.points[P1].y = value;
                    self.points[P2].y = self.points[P2].y.wrapping_sub(d);
                    self.points[P3].y = self.points[P3].y.wrapping_sub(d);
                } else {
                    // Point 2 or 3 is the smallest. Point 3 is forced to the value, deliberately as in the C#.
                    let d = self.points[P2].y.wrapping_sub(value);
                    self.points[P1].y = self.points[P1].y.wrapping_sub(d);
                    self.points[P2].y = value;
                    self.points[P3].y = value;
                }
            }
            ShapeKind::StarBurst { .. } => {
                self.points[TL].y = value;
                self.points[TR].y = value;
                self.points[BL].y = self.points[BL].y.wrapping_sub(delta);
                self.points[BR].y = self.points[BR].y.wrapping_sub(delta);
            }
            ShapeKind::MegaTree { .. } => {
                self.points[BR].y = value.wrapping_add(self.points[BR].y.wrapping_sub(self.points[TL].y));
                self.points[TL].y = value;
            }
            ShapeKind::PixelGrid { .. } => {
                self.points[TL].y = value;
                self.points[TR].y = value;
                self.points[BR].y = self.points[BR].y.wrapping_sub(delta);
                self.points[BL].y = self.points[BL].y.wrapping_sub(delta);
            }
            // The Single's setter does not lay out: it moves on the next layout. Deliberate, as in the C#.
            ShapeKind::Single => {
                self.points[0].y = value;
                return;
            }
            // Throws NotImplementedException in C#. A flood has no bounds other than its point.
            ShapeKind::Flood => self.points[0].y = value,
            ShapeKind::Custom { .. } => {
                if let Some(strings) = &mut self.strings {
                    for s in strings {
                        let t = s.top().wrapping_sub(delta);
                        s.set_top(t);
                    }
                }
            }
            ShapeKind::CustomProp(c) => {
                let x = c.bounds.x as i32;
                self.move_to(x, value);
                return;
            }
            ShapeKind::Unknown(_) => return,
        }
        self.layout();
    }

    /// `Left` setter.
    pub fn set_left(&mut self, value: i32) {
        let delta = self.left().wrapping_sub(value);
        match &self.kind {
            ShapeKind::Line { .. } | ShapeKind::Icicle { .. } | ShapeKind::Arch | ShapeKind::Star { .. } => {
                self.translate_some(&[0, 1], delta.wrapping_neg(), 0);
            }
            ShapeKind::Ellipse => {
                self.translate_some(&[TL, BR], delta.wrapping_neg(), 0);
                // The C# ends with this whichever point was leftmost, which squashes a mirrored ellipse. Deliberate.
                self.points[TL].x = value;
            }
            ShapeKind::Cane { .. } => self.translate_some(&[TL, BR, cane::ARCH], delta.wrapping_neg(), 0),
            ShapeKind::Rectangle { .. } | ShapeKind::Net { .. } | ShapeKind::MovingHead(_) | ShapeKind::PolyLine { .. } | ShapeKind::MultiString { .. } => {
                self.translate_points(delta.wrapping_neg(), 0);
            }
            ShapeKind::Triangle { .. } => {
                use triangle::{P1, P2, P3};
                if self.points[P2].x < self.points[P3].x {
                    // Point 2 is the smallest. The delta is taken from point 1 and point 2 loses the value
                    // itself rather than the delta. Deliberate, as in the C#.
                    let d = self.points[P1].x.wrapping_sub(value);
                    self.points[P1].x = self.points[P1].x.wrapping_sub(d);
                    self.points[P2].x = self.points[P2].x.wrapping_sub(value);
                    self.points[P3].x = self.points[P3].x.wrapping_sub(d);
                } else {
                    // Point 3 is the smallest
                    let d = self.points[P3].x.wrapping_sub(value);
                    self.points[P1].x = self.points[P1].x.wrapping_sub(d);
                    self.points[P2].x = self.points[P2].x.wrapping_sub(d);
                    self.points[P3].x = value;
                }
            }
            ShapeKind::StarBurst { .. } => {
                self.points[TL].x = value;
                self.points[BL].x = value;
                self.points[TR].x = self.points[TR].x.wrapping_sub(delta);
                self.points[BR].x = self.points[BR].x.wrapping_sub(delta);
            }
            ShapeKind::MegaTree { .. } => {
                self.points[BR].x = value.wrapping_add(self.points[BR].x.wrapping_sub(self.points[TL].x));
                self.points[TL].x = value;
            }
            ShapeKind::PixelGrid { .. } => {
                self.points[TL].x = value;
                self.points[BL].x = value;
                self.points[TR].x = self.points[TR].x.wrapping_sub(delta);
                self.points[BR].x = self.points[BR].x.wrapping_sub(delta);
            }
            ShapeKind::Single => {
                self.points[0].x = value;
                return;
            }
            ShapeKind::Flood => self.points[0].x = value,
            ShapeKind::Custom { .. } => {
                if let Some(strings) = &mut self.strings {
                    for s in strings {
                        let l = s.left().wrapping_sub(delta);
                        s.set_left(l);
                    }
                }
            }
            ShapeKind::CustomProp(c) => {
                let y = c.bounds.y as i32;
                self.move_to(value, y);
                return;
            }
            ShapeKind::Unknown(_) => return,
        }
        self.layout();
    }

    // ----- the virtual methods

    /// `Layout`.
    pub fn layout(&mut self) {
        match &self.kind {
            ShapeKind::Line { .. } => self.layout_line(),
            ShapeKind::PolyLine { .. } => self.layout_poly_line(),
            ShapeKind::Arch => self.layout_arch(),
            ShapeKind::Cane { .. } => self.layout_cane(),
            ShapeKind::Rectangle { .. } => self.layout_rectangle(),
            ShapeKind::Ellipse => self.layout_ellipse(),
            ShapeKind::Triangle { .. } => self.layout_triangle(),
            ShapeKind::Star { .. } => self.layout_star(),
            ShapeKind::StarBurst { .. } => self.layout_star_burst(),
            ShapeKind::Net { .. } => self.layout_net(),
            ShapeKind::MegaTree { .. } => self.layout_mega_tree(),
            ShapeKind::MultiString { .. } => self.layout_multi_string(),
            ShapeKind::Icicle { .. } => self.layout_icicle(),
            ShapeKind::PixelGrid { .. } => self.layout_pixel_grid(),
            ShapeKind::Single => self.layout_single(),
            ShapeKind::Flood => self.layout_flood(),
            ShapeKind::Custom { .. } => self.layout_custom(),
            ShapeKind::CustomProp(_) => self.layout_custom_prop(),
            ShapeKind::MovingHead(_) | ShapeKind::Unknown(_) => {}
        }
    }

    /// `ZoomLevel` setter.
    pub fn set_zoom_level(&mut self, value: f64) {
        match &self.kind {
            ShapeKind::CustomProp(_) => {
                if value == self.zoom_level {
                    return;
                }
                self.zoom_level = if value <= 0.0 { 1.0 } else { value };
                self.layout();
            }
            ShapeKind::MovingHead(_) | ShapeKind::Unknown(_) => {
                self.zoom_level = value;
                self.layout();
            }
            _ => {
                self.zoom_level = value;
                if let Some(strings) = &mut self.strings {
                    for s in strings {
                        s.set_zoom_level(value);
                    }
                }
                self.layout();
            }
        }
    }

    /// `MouseMove`: the mouse is at (x, y), having moved (change_x, change_y) since the drag began.
    pub fn mouse_move(&mut self, x: i32, y: i32, change_x: i32, change_y: i32, m: Modifiers) {
        match &self.kind {
            ShapeKind::Line { .. } => self.mouse_move_line(x, y, change_x, change_y, m),
            ShapeKind::PolyLine { .. } => self.mouse_move_poly_line(x, y, change_x, change_y, m),
            ShapeKind::Arch => self.mouse_move_arch(x, y, change_x, change_y, m),
            ShapeKind::Cane { .. } => self.mouse_move_cane(x, y, change_x, change_y, m),
            ShapeKind::Rectangle { .. } => self.mouse_move_rectangle(x, y, change_x, change_y, m),
            ShapeKind::Ellipse => self.mouse_move_ellipse(x, y, change_x, change_y, m),
            ShapeKind::Triangle { .. } => self.mouse_move_triangle(x, y, change_x, change_y, m),
            ShapeKind::Star { .. } => self.mouse_move_star(x, y, change_x, change_y, m),
            ShapeKind::StarBurst { .. } => self.mouse_move_star_burst(x, y, change_x, change_y, m),
            ShapeKind::Net { .. } => self.mouse_move_net(x, y, change_x, change_y, m),
            ShapeKind::MegaTree { .. } => self.mouse_move_mega_tree(x, y, change_x, change_y, m),
            ShapeKind::MultiString { .. } => self.mouse_move_multi_string(x, y, change_x, change_y, m),
            ShapeKind::Icicle { .. } => self.mouse_move_icicle(x, y, change_x, change_y, m),
            ShapeKind::PixelGrid { .. } => self.mouse_move_pixel_grid(x, y, change_x, change_y, m),
            ShapeKind::Single => self.mouse_move_single(x, y, change_x, change_y),
            ShapeKind::Flood => self.mouse_move_flood(x, y, change_x, change_y),
            ShapeKind::Custom { .. } => self.mouse_move_custom(x, y, change_x, change_y, m),
            ShapeKind::CustomProp(_) => self.mouse_move_custom_prop(x, y, change_x, change_y, m),
            ShapeKind::MovingHead(_) => self.mouse_move_moving_head(x, y, change_x, change_y, m),
            ShapeKind::Unknown(_) => {}
        }
    }

    /// `MouseUp`. The coordinates are the raw event's, not scrolled, as in the C#. The tree is only for a
    /// Multi-String being drawn, whose next string comes from the selected element.
    pub fn mouse_up(&mut self, x: i32, y: i32, left_button: bool, tree: Option<&ElementTree>) {
        match &self.kind {
            ShapeKind::PolyLine { .. } => self.mouse_up_poly_line(x, y, left_button),
            ShapeKind::MultiString { .. } => self.mouse_up_multi_string(x, y, left_button, tree),
            ShapeKind::Icicle { .. } => {
                if let ShapeKind::Icicle { creating, .. } = &mut self.kind {
                    *creating = false;
                }
            }
            ShapeKind::Custom { .. } => self.select_drag_points(),
            _ => {}
        }
    }

    /// `SelectDragPoints`.
    pub fn select_drag_points(&mut self) {
        match &self.kind {
            ShapeKind::Line { .. } | ShapeKind::Icicle { .. } => {
                if self.points.len() >= 2 {
                    self.set_select_points(vec![PointRef::Pt(0), PointRef::Pt(1)]);
                }
            }
            ShapeKind::PolyLine { .. } | ShapeKind::MultiString { .. } => {
                let list = (0..self.points.len()).map(PointRef::Pt).collect();
                self.set_select_points(list);
            }
            ShapeKind::Arch => {
                self.sync_arch_corners();
                self.set_select_points(vec![PointRef::Pt(TL), PointRef::Pt(BR), PointRef::Pt(TR), PointRef::Pt(BL)]);
            }
            ShapeKind::Cane { .. } => self.set_select_points(vec![PointRef::Pt(BR), PointRef::Pt(TL), PointRef::Pt(cane::ARCH)]),
            ShapeKind::Rectangle { .. } | ShapeKind::StarBurst { .. } | ShapeKind::Net { .. } | ShapeKind::MovingHead(_) => {
                self.set_select_points(vec![PointRef::Pt(TL), PointRef::Pt(TR), PointRef::Pt(BL), PointRef::Pt(BR)]);
            }
            ShapeKind::Ellipse | ShapeKind::MegaTree { .. } => {
                // Both build new corner points each time, and the mega tree saves them from here on.
                let (tl, br) = (self.points[TL], self.points[BR]);
                self.points[TR] = PreviewPoint::typed(br.x, tl.y, PointType::SizeTopRight);
                self.points[BL] = PreviewPoint::typed(tl.x, br.y, PointType::SizeBottomLeft);
                self.point_nil_mask = 0;
                self.set_select_points(vec![PointRef::Pt(TL), PointRef::Pt(BR), PointRef::Pt(TR), PointRef::Pt(BL)]);
            }
            ShapeKind::Triangle { .. } => self.set_select_points(vec![PointRef::Pt(triangle::P1), PointRef::Pt(triangle::P2), PointRef::Pt(triangle::P3)]),
            ShapeKind::Star { .. } => self.set_select_points(vec![PointRef::Pt(BR), PointRef::Pt(TL)]),
            ShapeKind::PixelGrid { .. } => self.set_select_points(vec![PointRef::Pt(TL), PointRef::Pt(BR), PointRef::Pt(TR), PointRef::Pt(BL)]),
            ShapeKind::Single | ShapeKind::Flood => self.set_select_points(vec![PointRef::Pt(0)]),
            ShapeKind::Custom { .. } => self.select_drag_points_custom(),
            ShapeKind::CustomProp(_) => self.configure_drag_points(),
            ShapeKind::Unknown(_) => {}
        }
    }

    /// `Select`.
    pub fn select(&mut self, select_drag_points: bool) {
        if matches!(self.kind, ShapeKind::MultiString { .. }) {
            self.set_string_type(StringType::Pixel);
        }
        self.selected = true;
        if select_drag_points {
            self.select_drag_points();
        }
        match &self.kind {
            ShapeKind::PolyLine { .. } | ShapeKind::Rectangle { .. } | ShapeKind::Triangle { .. } | ShapeKind::Star { .. } | ShapeKind::StarBurst { .. } | ShapeKind::Net { .. } => {
                self.connect_standard_strings = true;
            }
            ShapeKind::Icicle { .. } => {
                self.connect_standard_strings = true;
                if let Some(strings) = &mut self.strings {
                    for line in strings {
                        line.connect_standard_strings = true;
                    }
                }
            }
            _ => {}
        }
    }

    /// `Deselect`.
    pub fn deselect(&mut self) {
        self.selected = false;
        if let ShapeKind::CustomProp(c) = &mut self.kind {
            c.selection_point = None;
            c.drag_points.clear();
        } else {
            self.select_points = None;
        }
    }

    /// `SetSelectPoint`.
    pub fn set_select_point(&mut self, arg: SelectArg) {
        let is_null = arg == SelectArg::Null;
        // `point.X == 0 && point.Y == 0`: a real handle that happens to be at the origin counts too.
        let is_zero = match arg {
            SelectArg::Null => false,
            SelectArg::Reset => true,
            SelectArg::Point(r) => {
                let p = self.point_of(r);
                p.x == 0 && p.y == 0
            }
        };
        let new_selected = match arg {
            SelectArg::Null => None,
            SelectArg::Reset => {
                self.detached = PreviewPoint::new(0, 0);
                Some(PointRef::Detached)
            }
            SelectArg::Point(r) => Some(r),
        };
        match &mut self.kind {
            // These capture only for a move and never reach the base method.
            ShapeKind::Line { .. } | ShapeKind::Single | ShapeKind::Flood => {
                if is_null {
                    self.capture_start_points();
                }
                self.selected_point = new_selected;
            }
            ShapeKind::MovingHead(_) => {
                self.set_moving_head_lock_xy(false);
                if is_null {
                    self.capture_start_points();
                }
                self.selected_point = new_selected;
            }
            ShapeKind::Custom { .. } => self.set_select_point_custom(new_selected),
            ShapeKind::CustomProp(_) => self.set_select_point_custom_prop(arg),
            ShapeKind::Unknown(_) => {}
            kind => {
                match kind {
                    ShapeKind::Cane { just_placed, .. } => *just_placed = false,
                    ShapeKind::Rectangle { lock_xy, .. } | ShapeKind::Net { lock_xy, .. } => *lock_xy = false,
                    _ => {}
                }
                if is_null || is_zero {
                    self.capture_start_points();
                }
                self.selected_point = new_selected;
                // PreviewBaseShape.SetSelectPoint: (0, 0) resets the rotation axis to the center.
                if is_zero {
                    self.reset_rotation_axis();
                }
            }
        }
    }

    fn capture_start_points(&mut self) {
        self.start_points.clear();
        self.start_points.extend_from_slice(&self.points);
    }

    /// `SelectDefaultSelectPoint`.
    pub fn select_default_select_point(&mut self) {
        match &mut self.kind {
            ShapeKind::Line { .. } | ShapeKind::Arch | ShapeKind::Ellipse | ShapeKind::Star { .. } | ShapeKind::StarBurst { .. } | ShapeKind::MegaTree { .. } | ShapeKind::PixelGrid { .. } | ShapeKind::Icicle { .. } => {
                self.selected_point = Some(PointRef::Pt(1));
            }
            ShapeKind::Cane { just_placed, .. } => {
                *just_placed = true;
                self.selected_point = Some(PointRef::Pt(BR));
            }
            ShapeKind::Rectangle { lock_xy, .. } | ShapeKind::Net { lock_xy, .. } => {
                *lock_xy = true;
                self.selected_point = Some(PointRef::Pt(BR));
            }
            ShapeKind::MovingHead(_) => {
                self.set_moving_head_lock_xy(true);
                self.selected_point = Some(PointRef::Pt(BR));
            }
            ShapeKind::Triangle { .. } => self.select_default_select_point_triangle(),
            ShapeKind::Single | ShapeKind::Flood => self.selected_point = Some(PointRef::Pt(0)),
            ShapeKind::Custom { .. } => self.selected_point = Some(PointRef::Pt(custom::TOP_LEFT)),
            // The second point (default) gets set in MouseDown
            ShapeKind::PolyLine { .. } | ShapeKind::MultiString { .. } | ShapeKind::CustomProp(_) | ShapeKind::Unknown(_) => {}
        }
    }

    /// `MoveTo`.
    pub fn move_to(&mut self, x: i32, y: i32) {
        match &self.kind {
            ShapeKind::Line { .. } | ShapeKind::Icicle { .. } => {
                let (l, t) = (self.points[0].x.min(self.points[1].x), self.points[0].y.min(self.points[1].y));
                self.translate_some(&[0, 1], x.wrapping_sub(l), y.wrapping_sub(t));
                self.layout();
            }
            ShapeKind::Arch => self.move_to_arch(x, y),
            ShapeKind::Ellipse => self.move_to_ellipse(x, y),
            ShapeKind::Cane { .. } => {
                let (dx, dy) = (x.wrapping_sub(self.points[TL].x), y.wrapping_sub(self.points[TL].y));
                self.translate_some(&[TL, BR, cane::ARCH], dx, dy);
                self.layout();
            }
            ShapeKind::Star { .. } => {
                let (dx, dy) = (x.wrapping_sub(self.points[TL].x), y.wrapping_sub(self.points[TL].y));
                self.translate_some(&[TL, BR], dx, dy);
                self.layout();
            }
            ShapeKind::Rectangle { .. } | ShapeKind::Net { .. } | ShapeKind::StarBurst { .. } | ShapeKind::MovingHead(_) => {
                let mut tl = (i32::MAX, i32::MAX);
                for p in &self.points[..4] {
                    tl = (tl.0.min(p.x), tl.1.min(p.y));
                }
                self.translate_points(x.wrapping_sub(tl.0), y.wrapping_sub(tl.1));
                self.layout();
            }
            ShapeKind::Triangle { .. } => {
                use triangle::{P1, P2, P3};
                let l = self.points[P1].x.min(self.points[P2].x.min(self.points[P3].x));
                let t = self.points[P1].y.min(self.points[P2].y.min(self.points[P3].y));
                self.translate_some(&[P1, P2, P3], x.wrapping_sub(l), y.wrapping_sub(t));
                self.layout();
            }
            ShapeKind::MegaTree { .. } | ShapeKind::PixelGrid { .. } => {
                let (tl, br) = (self.points[TL], self.points[BR]);
                let (dx, dy) = (x.wrapping_sub(tl.x.min(br.x)), y.wrapping_sub(tl.y.min(br.y)));
                self.translate_some(&[TL, BR], dx, dy);
                let (tl, br) = (self.points[TL], self.points[BR]);
                self.points[TR].x = br.x;
                self.points[TR].y = tl.y;
                self.points[BL].x = tl.x;
                self.points[BL].y = br.y;
                self.point_nil_mask = 0;
                self.layout();
            }
            ShapeKind::MultiString { .. } => {
                self.set_top(y);
                self.set_left(x);
                self.layout();
            }
            ShapeKind::PolyLine { .. } => {
                // The C# zooms a copy of the point and throws the result away, so x and y go straight through.
                self.set_top(y);
                self.set_left(x);
                self.layout();
            }
            ShapeKind::Single | ShapeKind::Flood => {
                self.points[0].x = x;
                self.points[0].y = y;
                self.layout();
            }
            ShapeKind::Custom { .. } => self.move_to_custom(x, y),
            ShapeKind::CustomProp(_) => self.move_to_custom_prop(x, y),
            ShapeKind::Unknown(_) => {}
        }
    }

    /// `Nudge`.
    pub fn nudge(&mut self, x: i32, y: i32) {
        let (l, t) = (self.left(), self.top());
        self.move_to(l.wrapping_add(x), t.wrapping_add(y));
    }

    /// `Resize`.
    pub fn resize(&mut self, aspect: f64) {
        match &self.kind {
            ShapeKind::Line { .. } | ShapeKind::Icicle { .. } | ShapeKind::Star { .. } | ShapeKind::MegaTree { .. } | ShapeKind::PixelGrid { .. } | ShapeKind::Single | ShapeKind::Flood | ShapeKind::Cane { .. } | ShapeKind::Rectangle { .. } | ShapeKind::StarBurst { .. } | ShapeKind::Net { .. } | ShapeKind::Triangle { .. } | ShapeKind::MovingHead(_) => {
                let n = self.resized_point_count();
                for p in &mut self.points[..n] {
                    p.x = (p.x as f64 * aspect) as i32;
                    p.y = (p.y as f64 * aspect) as i32;
                }
                self.layout();
            }
            ShapeKind::Arch | ShapeKind::Ellipse => {
                for p in &mut self.points[..2] {
                    p.x = (p.x as f64 * aspect) as i32;
                    p.y = (p.y as f64 * aspect) as i32;
                }
                if matches!(self.kind, ShapeKind::Arch) {
                    self.sync_arch_corners();
                }
                self.reset_rotation_axis();
                self.layout();
            }
            ShapeKind::PolyLine { .. } | ShapeKind::MultiString { .. } => {
                for p in &mut self.points {
                    p.x = to_int32(p.x as f64 * aspect);
                    p.y = to_int32(p.y as f64 * aspect);
                }
                self.layout();
            }
            ShapeKind::Custom { .. } => {
                if let Some(strings) = &mut self.strings {
                    for s in strings {
                        s.resize(aspect);
                    }
                }
                self.layout();
            }
            ShapeKind::CustomProp(_) => self.resize_custom_prop(aspect),
            ShapeKind::Unknown(_) => {}
        }
    }

    /// How many of `points` `Resize` and `ResizeFromOriginal` touch. The mega tree and grid leave their
    /// other two corners alone, deliberately as in the C#.
    fn resized_point_count(&self) -> usize {
        match &self.kind {
            ShapeKind::Line { .. } | ShapeKind::Icicle { .. } | ShapeKind::Star { .. } | ShapeKind::MegaTree { .. } | ShapeKind::PixelGrid { .. } | ShapeKind::Arch | ShapeKind::Ellipse => 2,
            ShapeKind::Single | ShapeKind::Flood => 1,
            ShapeKind::Cane { .. } | ShapeKind::Triangle { .. } => 3,
            ShapeKind::Rectangle { .. } | ShapeKind::StarBurst { .. } | ShapeKind::Net { .. } | ShapeKind::MovingHead(_) => 4,
            ShapeKind::PolyLine { .. } | ShapeKind::MultiString { .. } => self.points.len(),
            ShapeKind::Custom { .. } | ShapeKind::CustomProp(_) | ShapeKind::Unknown(_) => 0,
        }
    }

    /// `ResizeFromOriginal`. Custom and Custom Prop throw in C#, and do nothing here.
    pub fn resize_from_original(&mut self, aspect: f64) {
        if matches!(self.kind, ShapeKind::Custom { .. } | ShapeKind::CustomProp(_) | ShapeKind::Unknown(_)) {
            return;
        }
        let n = self.resized_point_count().min(self.start_points.len());
        for i in 0..n {
            let s = self.start_points[i];
            self.points[i].x = s.x;
            self.points[i].y = s.y;
        }
        self.resize(aspect);
    }

    /// `EndAddNew`.
    pub fn end_add_new(&mut self) {
        if matches!(self.kind, ShapeKind::MovingHead(_)) {
            self.end_add_new_moving_head();
        }
    }

    /// `OnMovePoint`.
    pub fn on_move_point(&mut self) {
        if matches!(self.kind, ShapeKind::MovingHead(_)) {
            self.on_move_point_moving_head();
        }
    }

    /// `Reconfigure`: links a new shape to the element a template just created.
    pub fn reconfigure(&mut self, node: Option<Sel>) {
        match &self.kind {
            ShapeKind::Line { .. } => self.reconfigure_line(node),
            ShapeKind::PolyLine { .. } => self.reconfigure_poly_line(node),
            ShapeKind::Arch => self.reconfigure_arch(node),
            ShapeKind::Cane { .. } => self.reconfigure_cane(node),
            ShapeKind::Rectangle { .. } => self.reconfigure_rectangle(node),
            ShapeKind::Ellipse => self.reconfigure_ellipse(node),
            ShapeKind::Triangle { .. } => self.reconfigure_triangle(node),
            ShapeKind::Star { .. } => self.reconfigure_star(node),
            ShapeKind::StarBurst { .. } => self.reconfigure_star_burst(node),
            ShapeKind::Net { .. } => self.reconfigure_net(node),
            ShapeKind::MegaTree { .. } => self.reconfigure_mega_tree(node),
            ShapeKind::Icicle { .. } => self.reconfigure_icicle(node),
            ShapeKind::PixelGrid { .. } => self.reconfigure_pixel_grid(node),
            ShapeKind::MovingHead(_) => self.reconfigure_moving_head(node),
            // PreviewBaseShape.Reconfigure does nothing, and these do not override it.
            ShapeKind::MultiString { .. } | ShapeKind::Single | ShapeKind::Flood | ShapeKind::Custom { .. } | ShapeKind::CustomProp(_) | ShapeKind::Unknown(_) => {}
        }
    }

    /// `Match`, the Match Properties button. `other` is the same type; a different one does nothing.
    pub fn match_shape(&mut self, other: &Shape) {
        if std::mem::discriminant(&self.kind) != std::mem::discriminant(&other.kind) {
            return;
        }
        match &self.kind {
            ShapeKind::Line { .. } => self.match_line(other),
            ShapeKind::PolyLine { .. } | ShapeKind::Unknown(_) => {} // the poly line's Match is commented out
            ShapeKind::Arch => self.match_arch(other),
            ShapeKind::Cane { .. } => self.match_cane(other),
            ShapeKind::Rectangle { .. } => self.match_rectangle(other),
            ShapeKind::Ellipse => self.match_ellipse(other),
            ShapeKind::Triangle { .. } => self.match_triangle(other),
            ShapeKind::Star { .. } => self.match_star(other),
            ShapeKind::StarBurst { .. } => self.match_star_burst(other),
            ShapeKind::Net { .. } => self.match_net(other),
            ShapeKind::MegaTree { .. } => self.match_mega_tree(other),
            ShapeKind::MultiString { .. } => {
                self.set_pixel_size(other.pixel_size());
                self.rotation_angle = other.rotation_angle;
                self.layout();
            }
            ShapeKind::Icicle { .. } => self.match_icicle(other),
            ShapeKind::PixelGrid { .. } => self.match_pixel_grid(other),
            ShapeKind::Single | ShapeKind::Flood | ShapeKind::Custom { .. } => {
                self.set_pixel_size(other.pixel_size());
                self.layout();
            }
            ShapeKind::CustomProp(_) => self.match_custom_prop(other),
            ShapeKind::MovingHead(_) => self.match_moving_head(other),
        }
    }

    // ----- hit testing, exact ports. The session reaches these only after shape-level bounds.

    /// The per-pixel rectangle `PointInShape` tests, when that is what the shape does:
    /// (x - 3, y - 3) to (x - 3 + width, y - 3 + width), inclusive on all sides.
    #[inline]
    pub(crate) fn pixel_hit_width(&self) -> Option<i32> {
        match &self.kind {
            ShapeKind::Line { .. } | ShapeKind::PolyLine { .. } | ShapeKind::Cane { .. } | ShapeKind::Star { .. } | ShapeKind::MultiString { .. } | ShapeKind::Icicle { .. } | ShapeKind::Ellipse => Some(SELECT_POINT_SIZE + self.pixel_size()),
            ShapeKind::Arch | ShapeKind::Rectangle { .. } | ShapeKind::Triangle { .. } | ShapeKind::StarBurst { .. } | ShapeKind::Net { .. } => Some(SELECT_POINT_SIZE),
            _ => None,
        }
    }

    /// Runs `f` on the pixels in order until it returns true, and says whether it did.
    pub(crate) fn any_pixel(&self, f: &mut impl FnMut(&PreviewPixel) -> bool) -> bool {
        match &self.kind {
            ShapeKind::CustomProp(c) => c.view_pixels.iter().any(f),
            ShapeKind::Rectangle { .. } | ShapeKind::Triangle { .. } | ShapeKind::StarBurst { .. } => {
                self.strings.as_deref().unwrap_or(&[]).iter().any(|s| s.pixels.iter().any(&mut *f))
            }
            ShapeKind::MovingHead(_) | ShapeKind::Unknown(_) => false,
            _ => match self.strings.as_ref().filter(|s| !s.is_empty()) {
                Some(strings) => strings.iter().any(|s| s.any_pixel(f)),
                None => self.pixels.iter().any(f),
            },
        }
    }

    fn pixels_hit(&self, x: i32, y: i32, width: i32) -> bool {
        let half = SELECT_POINT_SIZE / 2;
        self.any_pixel(&mut |p| {
            let (rx, ry) = (p.x.wrapping_sub(half), p.y.wrapping_sub(half));
            x >= rx && x <= rx.wrapping_add(width) && y >= ry && y <= ry.wrapping_add(width)
        })
    }

    /// `PointInShape`.
    pub fn point_in_shape(&self, point: PreviewPoint) -> bool {
        match &self.kind {
            ShapeKind::Ellipse => {
                let (x, y) = self.ellipse_hit_point(point);
                self.ellipse_pixels_hit(x, y)
            }
            ShapeKind::MegaTree { .. } | ShapeKind::PixelGrid { .. } => self.strings.as_deref().unwrap_or(&[]).iter().any(|l| l.point_in_shape(point)),
            ShapeKind::Single => self.point_in_shape_single(point),
            ShapeKind::Flood => {
                let (p1, size) = (self.points[0], self.pixel_size);
                let (rx, ry) = (p1.x.wrapping_sub(size / 2), p1.y.wrapping_sub(size / 2));
                point.x >= rx && point.x <= rx.wrapping_add(size) && point.y >= ry && point.y <= ry.wrapping_add(size)
            }
            ShapeKind::Custom { .. } => {
                let p = self.point_to_zoom_point(point);
                let (l, t) = (self.left(), self.top());
                Rect::new(l, t, self.right().wrapping_sub(l), self.bottom().wrapping_sub(t)).contains(p.x, p.y)
            }
            ShapeKind::CustomProp(_) => self.point_in_shape_custom_prop(point),
            ShapeKind::MovingHead(_) => self.point_in_shape_moving_head(point),
            ShapeKind::Unknown(_) => false,
            _ => match self.pixel_hit_width() {
                Some(w) => self.pixels_hit(point.x, point.y, w),
                None => false,
            },
        }
    }

    /// `ShapeInRect` for the light shapes: the last pixel looked at decides, and the loop stops at the first
    /// pixel outside when all must be in, or the first inside when any will do. No pixels at all is false.
    pub(crate) fn pixels_in_rect(&self, rect: Rect, all_in: bool) -> bool {
        let (x1, y1, x2, y2) = rect.normalized();
        let high_precision = self.is_high_precision();
        let mut result = false;
        self.any_pixel(&mut |p| {
            result = if high_precision {
                p.location.0 >= x1 as f64 && p.location.0 <= x2 as f64 && p.location.1 >= y1 as f64 && p.location.1 <= y2 as f64
            } else {
                p.x >= x1 && p.x <= x2 && p.y >= y1 && p.y <= y2
            };
            result != all_in
        });
        result
    }

    /// `ShapeInRect`.
    pub fn shape_in_rect(&self, rect: Rect, all_in: bool) -> bool {
        match &self.kind {
            ShapeKind::MovingHead(_) => self.shape_in_rect_moving_head(rect, all_in),
            ShapeKind::Unknown(_) => false,
            _ => self.pixels_in_rect(rect, all_in),
        }
    }

    /// `ShapeAllInRect`.
    pub fn shape_all_in_rect(&self, rect: Rect) -> bool {
        if matches!(self.kind, ShapeKind::CustomProp(_)) {
            return self.shape_all_in_rect_custom_prop(rect);
        }
        let p1 = self.point_to_zoom_point(PreviewPoint::new(rect.x, rect.y));
        let p2 = self.point_to_zoom_point(PreviewPoint::new(rect.x.wrapping_add(rect.width), rect.y.wrapping_add(rect.height)));
        let (x1, x2) = (p1.x.min(p2.x), p1.x.max(p2.x));
        let (y1, y2) = (p1.y.min(p2.y), p1.y.max(p2.y));
        self.top() >= y1 && self.bottom() <= y2 && self.left() >= x1 && self.right() <= x2
    }

    /// `PointToZoomPoint`.
    #[inline]
    pub fn point_to_zoom_point(&self, p: PreviewPoint) -> PreviewPoint {
        let zoom = self.zoom();
        // xDif = X - (int)(X / zoom), then X - xDif.
        PreviewPoint { x: (p.x as f64 / zoom) as i32, y: (p.y as f64 / zoom) as i32, point_type: p.point_type }
    }

    /// `PointToZoomPointAdd`.
    #[inline]
    pub fn point_to_zoom_point_add(&self, p: PreviewPoint) -> PreviewPoint {
        let zoom = self.zoom();
        PreviewPoint::new(to_int32(p.x as f64 * zoom), to_int32(p.y as f64 * zoom))
    }

    // ----- shared helpers of the MouseMove methods

    /// `changeX = Convert.ToInt32(start.X + changeX / ZoomLevel) - point.X`, the same for Y.
    pub(crate) fn moving_delta(&self, anchor: usize, change_x: i32, change_y: i32) -> (i32, i32) {
        let (Some(s), Some(p)) = (self.start_points.get(anchor), self.points.get(anchor)) else { return (0, 0) };
        let zoom = self.zoom();
        let dx = to_int32(s.x as f64 + change_x as f64 / zoom).wrapping_sub(p.x);
        let dy = to_int32(s.y as f64 + change_y as f64 / zoom).wrapping_sub(p.y);
        (dx, dy)
    }

    /// `PreviewBaseShape.MouseMove`.
    pub(crate) fn base_mouse_move(&mut self, x: i32, y: i32, change_x: i32, change_y: i32, m: Modifiers) {
        if let Some(sel) = self.selected_point {
            if self.point_of(sel).point_type == PointType::RotateHandle {
                let angle = self.calculate_rotation((x, y), self.zoom(), m.only_ctrl());
                self.set_rotation_angle(angle);
            }
        } else if let Some(axis) = &mut self.rotation_axis {
            axis.translate(change_x, change_y);
        }
    }

    pub(crate) fn point_of(&self, r: PointRef) -> PreviewPoint {
        match r {
            PointRef::Pt(i) => self.points.get(i).copied().unwrap_or_default(),
            PointRef::RotateHandle => self.rotate_handle.unwrap_or_default(),
            PointRef::RotationAxis => self.rotation_axis.unwrap_or_default(),
            PointRef::Detached => self.detached,
        }
    }

    pub(crate) fn set_point_xy(&mut self, r: PointRef, x: i32, y: i32) {
        let target = match r {
            PointRef::Pt(i) => self.points.get_mut(i),
            PointRef::RotateHandle => self.rotate_handle.as_mut(),
            PointRef::RotationAxis => self.rotation_axis.as_mut(),
            PointRef::Detached => Some(&mut self.detached),
        };
        if let Some(p) = target {
            p.x = x;
            p.y = y;
        }
    }

    pub(crate) fn translate_ref(&mut self, r: PointRef, dx: i32, dy: i32) {
        let p = self.point_of(r);
        self.set_point_xy(r, p.x.wrapping_add(dx), p.y.wrapping_add(dy));
    }

    /// `SetSelectPoints`: the rotation axis first, the rotate handle last.
    pub(crate) fn set_select_points(&mut self, mut list: Vec<PointRef>) {
        if self.rotate_handle.is_none() {
            let (c, t) = (self.center(), self.top());
            self.rotate_handle = Some(PreviewPoint::typed(c.x, t.wrapping_sub(10), PointType::RotateHandle));
        }
        if self.rotation_axis.is_none() {
            let c = self.center();
            self.rotation_axis = Some(c.with_type(PointType::RotationAxis));
        }
        list.push(PointRef::RotateHandle);
        list.insert(0, PointRef::RotationAxis);
        self.select_points = Some(list);
    }

    /// `PointInSelectPoint`.
    pub fn point_in_select_point(&mut self, point: PreviewPoint) -> Option<PointRef> {
        if matches!(self.kind, ShapeKind::CustomProp(_)) {
            return self.point_in_select_point_custom_prop(point);
        }
        let n = self.select_points.as_ref()?.len();
        let half = SELECT_POINT_SIZE / 2;
        let zoom = self.zoom();
        for i in 0..n {
            let Some(r) = self.select_points.as_ref().and_then(|l| l.get(i).copied()) else { break };
            // Disallow the rotation axis as a selectable point
            if r == PointRef::RotationAxis {
                continue;
            }
            let sp = self.point_of(r);
            let pp = self.transform_preview_point(sp, zoom, RotateTypes::Clockwise);
            if point.x >= pp.x.wrapping_sub(half) && point.y >= pp.y.wrapping_sub(half) && point.x <= pp.x.wrapping_add(half) && point.y <= pp.y.wrapping_add(half) {
                return Some(r);
            }
        }
        None
    }

    /// Whether a handle returned by `point_in_select_point` is the rotate handle.
    pub fn is_rotate_handle(&self, r: PointRef) -> bool {
        self.point_of(r).point_type == PointType::RotateHandle
    }

    /// The part of `DrawSelectPoints` that changes the shape: the rotate handle follows the top of the
    /// shape. It runs before a frame's overlay is read, as the C# does it while drawing.
    pub(crate) fn sync_rotate_handle(&mut self) {
        let Some(list) = self.select_points.as_ref() else { return };
        if list.is_empty() || !self.show_rotation {
            return;
        }
        let find = |t: PointType, list: &[PointRef], s: &Shape| list.iter().map(|r| s.point_of(*r)).find(|p| p.point_type == t);
        let list = list.clone();
        // A shape with no top left point throws here in C#.
        let Some(top) = find(PointType::SizeTopLeft, &list, self).map(|p| p.y) else { return };
        let bottom = find(PointType::SizeBottomRight, &list, self).map_or(i32::MAX, |p| p.y);
        let cx = self.center().x;
        let y = if top > bottom { self.bottom().wrapping_add(10) } else { self.top().wrapping_sub(10) };
        if let Some(h) = &mut self.rotate_handle {
            h.x = cx;
            h.y = y;
        }
    }
}
