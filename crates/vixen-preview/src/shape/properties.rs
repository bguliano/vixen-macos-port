//! The property grid: `[Category]`, `[DisplayName]`, `[Description]`, `[Browsable]` and `[ReadOnly]` from each
//! shape class, and what each C# setter does.
//!
//! A public property with no attributes at all still shows in a PropertyGrid, under Misc by its C# name, and
//! one with no setter shows read only. A few shapes have some, and those are kept.

use super::moving_head::{MountingPosition, YesNo};
use super::{cane, triangle, Direction, Modifiers, Shape, ShapeKind, StringOrientation, StringType, BL, BR, TL, TR};
use crate::point::PreviewPoint;
use crate::xml_util::DoubleText;
use std::fmt;
use uuid::Uuid;

/// What kind of editor a property needs.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PropertyKind {
    Int,
    Bool,
    Text,
    Double,
    /// `System.Drawing.Point`, shown as X and Y.
    Point,
    /// One of these member names.
    Enum(&'static [&'static str]),
    /// `Strings`, "Linked Elements": edited through [`crate::link::ElementLinker`].
    LinkedElements,
    /// A moving head's "Linked Element": one node.
    LinkedElement,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PropertyValue {
    Int(i32),
    Bool(bool),
    Text(String),
    Double(f64),
    Point { x: i32, y: i32 },
    Enum(String),
    /// How many strings the linker will show.
    LinkedElements { strings: usize },
    LinkedElement(Option<Uuid>),
}

/// One row of the property grid.
#[derive(Clone, Debug, PartialEq)]
pub struct ShapeProperty {
    /// `[Category]`, or "Misc" for a property with none.
    pub category: &'static str,
    /// `[DisplayName]`, or the C# name for a property with none. "Botom Left" is misspelled in the C# too.
    pub name: &'static str,
    /// The C# property name.
    pub id: &'static str,
    /// `[Description]`, or empty.
    pub description: &'static str,
    pub kind: PropertyKind,
    pub value: PropertyValue,
    pub read_only: bool,
    /// `[PropertyOrder]`, zero if it has none: rows sort by category, then this, then the C# name.
    pub order: i32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PropertyError {
    UnknownShape,
    UnknownProperty,
    ReadOnly,
    /// The value is not what this property holds.
    WrongType,
    /// "Linked Elements" goes through [`crate::link::ElementLinker`].
    UseLinker,
}

impl fmt::Display for PropertyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            PropertyError::UnknownShape => "there is no such shape",
            PropertyError::UnknownProperty => "the shape has no such property",
            PropertyError::ReadOnly => "the property is read only",
            PropertyError::WrongType => "the value is not of the property's type",
            PropertyError::UseLinker => "linked elements are edited through the element linker",
        };
        write!(f, "{s}")
    }
}

impl std::error::Error for PropertyError {}

const STRING_TYPES: &[&str] = &["Standard", "Pixel", "Custom"];
const DIRECTIONS: &[&str] = &["Clockwise", "CounterClockwise"];
const ORIENTATIONS: &[&str] = &["Vertical", "Horizontal"];
const YES_NO: &[&str] = &["No", "Yes"];
const MOUNTING: &[&str] = &["Bottom", "Top"];

struct Rows(Vec<ShapeProperty>);

impl Rows {
    fn add(&mut self, category: &'static str, name: &'static str, id: &'static str, description: &'static str, kind: PropertyKind, value: PropertyValue) {
        self.0.push(ShapeProperty { category, name, id, description, kind, value, read_only: false, order: 0 });
    }

    fn int(&mut self, category: &'static str, name: &'static str, id: &'static str, description: &'static str, v: i32) {
        self.add(category, name, id, description, PropertyKind::Int, PropertyValue::Int(v));
    }

    fn point(&mut self, name: &'static str, id: &'static str, description: &'static str, p: PreviewPoint) {
        self.add("Position", name, id, description, PropertyKind::Point, PropertyValue::Point { x: p.x, y: p.y });
    }

    fn enumeration(&mut self, category: &'static str, name: &'static str, id: &'static str, description: &'static str, members: &'static [&'static str], v: &str) {
        self.add(category, name, id, description, PropertyKind::Enum(members), PropertyValue::Enum(v.to_string()));
    }

    /// A property with no attributes and no setter.
    fn misc_read_only(&mut self, id: &'static str, v: i32) {
        self.0.push(ShapeProperty { category: "Misc", name: id, id, description: "", kind: PropertyKind::Int, value: PropertyValue::Int(v), read_only: true, order: 0 });
    }
}

impl Shape {
    /// The rows in the order a PropertyGrid shows them. Categorized: by category, then display name ignoring case.
    /// Alphabetical: by display name ignoring case. `PropertySorter` and `[PropertyOrder]` don't decide this: the grid
    /// keeps its default `CategorizedAlphabetical`, which sorts the converter's list again by display name.
    pub fn properties_for_grid(&self, alphabetical: bool) -> Vec<ShapeProperty> {
        let mut rows = self.properties();
        if alphabetical {
            rows.sort_by_cached_key(|r| r.name.to_lowercase());
        } else {
            rows.sort_by_cached_key(|r| (r.category.to_lowercase(), r.name.to_lowercase()));
        }
        rows
    }

    fn point_at(&self, i: usize) -> PreviewPoint {
        self.points.get(i).copied().unwrap_or_default()
    }

    fn string_len(&self, i: usize) -> i32 {
        self.strings.as_deref().and_then(|s| s.get(i)).map_or(0, Shape::pixel_count) as i32
    }

    /// The rows the PropertyGrid shows for this shape, in its categorized order.
    pub fn properties(&self) -> Vec<ShapeProperty> {
        let mut r = Rows(Vec::with_capacity(16));
        if matches!(self.kind, ShapeKind::Unknown(_)) {
            return r.0;
        }
        // PreviewBaseShape
        r.add(
            "Settings",
            "Name",
            "Name",
            "The name of this string. Used in templates to distinguish various strings.",
            PropertyKind::Text,
            PropertyValue::Text(self.name.clone().unwrap_or_default()),
        );
        r.add(
            "Location",
            "Common Location",
            "UseCommonLocation",
            "Uses a common location for all lights based on the center of the prop.",
            PropertyKind::Bool,
            PropertyValue::Bool(self.use_common_location),
        );
        r.add("Control", "Locked", "Locked", "Locks out access to the Prop. Use Shift key in combination with the mouse to override.", PropertyKind::Bool, PropertyValue::Bool(self.locked));
        r.0.push(ShapeProperty {
            category: "Position",
            name: "Rotation Angle",
            id: "RotationAngle",
            description: "Rotates the prop by the specified degrees. A positive value rotates in a clockwise direction and a negative value rotates in a counter-clockwise direction.",
            kind: PropertyKind::Int,
            value: PropertyValue::Int(self.rotation_angle),
            read_only: false,
            order: 99,
        });
        // PreviewLightBaseShape
        if self.is_light_shape() {
            let hides_string_type = matches!(self.kind, ShapeKind::Net { .. } | ShapeKind::MultiString { .. } | ShapeKind::Flood);
            if !hides_string_type {
                r.enumeration("Settings", "String Type", "StringType", "", STRING_TYPES, self.string_type().name());
                if matches!(self.kind, ShapeKind::CustomProp(_)) {
                    if let Some(last) = r.0.last_mut() {
                        last.read_only = true;
                    }
                }
            }
            if !matches!(self.kind, ShapeKind::Custom { .. } | ShapeKind::CustomProp(_)) {
                r.int("Settings", "Light Size", "PixelSize", "The size of the light point on the preview.", self.pixel_size());
            }
            if !matches!(self.kind, ShapeKind::Custom { .. }) {
                let strings = match &self.strings {
                    Some(s) => s.len(),
                    None => 1,
                };
                r.add("Settings", "Linked Elements", "Strings", "", PropertyKind::LinkedElements, PropertyValue::LinkedElements { strings });
            }
        }
        // The derived class
        match &self.kind {
            ShapeKind::Line { .. } => {
                r.point("Point 1", "Point1", "Lines are defined by 2 points. This is point 1.", self.point_at(0));
                r.point("Point 2", "Point2", "Lines are defined by 2 points. This is point 2.", self.point_at(1));
                r.int("Settings", "Light Count", "PixelCount", "Number of pixels or lights in the string.", self.pixels.len() as i32);
            }
            ShapeKind::PolyLine { .. } => {
                r.int("Settings", "Light Count", "PixelCount", "Number of pixels or lights in the string.", self.pixels.len() as i32);
            }
            ShapeKind::Arch => {
                r.point("Top Left", "TopLeft", "An arch is defined by a 2 points of a rectangle. This is point 1.", self.point_at(TL));
                r.point("Bottom Right", "BottomRight", "An arch is defined by a 2 points of a rectangle. This is point 2.", self.point_at(BR));
                r.int("Size", "Width", "Width", "An arch is defined by a 2 points of a rectangle. This is the width of those points.", self.arch_width());
                r.int("Size", "Height", "Height", "An arch is defined by a 2 points of a rectangle. This is the height of those points.", self.arch_height());
                r.int("Settings", "Light Count", "PixelCount", "Number of pixels or lights in the arch.", self.pixels.len() as i32);
            }
            ShapeKind::Cane { arch_pixel_count, line_pixel_count, .. } => {
                r.int("Settings", "Line Light Count", "LinePixelCount", "The number of lights in the vertical string of the candy cane.", *line_pixel_count);
                r.int("Settings", "Arch Light Count", "ArchPixelCount", "The number of lights in the arch of the candy cane.", *arch_pixel_count);
                r.point("Bottom Right", "BottomRight", "The bottom right point of bounding box of the candy cane.", self.point_at(BR));
                r.point("Top Left", "TopLeft", "The top left point of the bounding box of the candy cane.", self.point_at(TL));
                r.point("Arch Start", "ArchLeft", "The point on the line where the arch starts.", self.point_at(cane::ARCH));
            }
            ShapeKind::Rectangle { direction, .. } => {
                r.point("Top Left", "TopLeftPoint", "Rectangles are defined by 4 points. This is point 1.", self.point_at(TL));
                r.point("Top Right", "TopRightPoint", "Rectangles are defined by 4 points. This is point 2.", self.point_at(TR));
                r.point("Bottom Right", "BottomRightPoint", "Rectangles are defined by 4 points. This is point 3.", self.point_at(BR));
                r.point("Botom Left", "BottomLeftPoint", "Rectangles are defined by 4 points. This is point 4.", self.point_at(BL));
                r.int("Settings", "String 1 Light Count", "LightCountString1", "Number of pixels or lights in string 1 of the rectangle.", self.string_len(0));
                r.int("Settings", "String 2 Light Count", "LightCountString2", "Number of pixels or lights in string 2 of the rectangle.", self.string_len(1));
                r.int("Settings", "String 3 Light Count", "LightCountString3", "Number of pixels or lights in string 3 of the rectangle.", self.string_len(2));
                r.int("Settings", "String 4 Light Count", "LightCountString4", "Number of pixels or lights in string 4 of the rectangle.", self.string_len(3));
                r.enumeration("Settings", "Direction", "Direction", "Wrap direction.", DIRECTIONS, direction.name());
                r.misc_read_only("PixelCount", self.pixel_count() as i32);
            }
            ShapeKind::Ellipse => {
                r.point("Top Left", "TopLeft", "An ellipse is defined by a 2 points of a ellipse. This is point 1.", self.point_at(TL));
                r.point("Bottom Right", "BottomRight", "An ellipse is defined by a 2 points of a ellipse. This is point 2.", self.point_at(BR));
                r.int("Settings", "Light Count", "PixelCount", "Number of pixels or lights in the ellipse.", self.pixels.len() as i32);
            }
            ShapeKind::Triangle { .. } => {
                r.point("Point 1", "Point1", "An triangle is defined by a 3 points of a rectangle. This is point 1.", self.point_at(triangle::P1));
                r.point("Point 2", "Point2", "An triangle is defined by a 3 points of a rectangle. This is point 2.", self.point_at(triangle::P2));
                r.point("Point 3", "Point3", "An triangle is defined by a 3 points of a rectangle. This is point 3.", self.point_at(triangle::P3));
                r.int("Settings", "String 1 Light Count", "LightCountString1", "Number of pixels or lights in string 1 of the triangle.", self.string_len(0));
                r.int("Settings", "String 2 Light Count", "LightCountString2", "Number of pixels or lights in string 2 of the triangle.", self.string_len(1));
                r.int("Settings", "String 3 Light Count", "LightCountString3", "Number of pixels or lights in string 3 of the triangle.", self.string_len(2));
                r.misc_read_only("Width", self.triangle_width());
                r.misc_read_only("Height", self.triangle_height());
            }
            ShapeKind::Star { point_count, pixel_count, inside_size, string_direction, xy_rotation, .. } => {
                r.enumeration("Settings", "String Direction", "StringDirection", "Do the lights rotate around the star clockwise or counter-clockwise?", DIRECTIONS, string_direction.name());
                r.int("Settings", "Light Count", "PixelCount", "The number of lights in the star.", *pixel_count);
                r.point("Bottom Right", "BottomRight", "The bottom right point of bounding box of the star.", self.point_at(BR));
                r.point("Top Left", "TopLeft", "The top left point of the bounding box of the star.", self.point_at(TL));
                r.int("Settings", "XY Rotation", "XYRotation", "The prop can be rotated about the Z axis in the XY plane. This is the rotation angle.", *xy_rotation);
                r.int("Settings", "Inside Size", "InsideSize", "The size, in percent of the inside star diameter in relation to the outside diameter.", *inside_size);
                r.int("Settings", "Star Points", "PointCount", "The number of points on the star.", *point_count);
            }
            ShapeKind::StarBurst { direction, xy_rotation, inner_circle_size } => {
                r.point("Top Left", "TopLeftPoint", "Star Bursts are defined by 2 points. This is point 1.", self.point_at(TL));
                r.point("Bottom Right", "BottomRightPoint", "Star Bursts are defined by 2 points. This is point 2.", self.point_at(BR));
                r.int("Settings", "XY Rotation", "XYRotation", "The prop can be rotated about the Z axis in the XY plane. This is the rotation angle.", *xy_rotation);
                r.int("Settings", "Light Count per Spoke", "LightCount", "Number of pixels or lights in each spoke of the star burst.", self.string_len(0));
                r.enumeration("Settings", "Direction", "Direction", "Wrap direction.", DIRECTIONS, direction.name());
                r.int("Settings", "Inner Circle Size", "InnerCircleSize", "The strings on each spoke attach to the middle of the star burst. This is the size of the inner circle.", *inner_circle_size);
                r.int("Misc", "StringCount", "StringCount", "", self.strings.as_ref().map_or(0, Vec::len) as i32);
                r.misc_read_only("PixelCount", self.pixel_count() as i32);
            }
            ShapeKind::Net { pixel_spacing, .. } => {
                r.point("Top Left", "TopLeftPoint", "Nets are defined by 4 points. This is point 1.", self.point_at(TL));
                r.point("Top Right", "TopRightPoint", "Nets are defined by 4 points. This is point 2.", self.point_at(TR));
                r.point("Bottom Right", "BottomRightPoint", "Nets are defined by 4 points. This is point 3.", self.point_at(BR));
                r.point("Botom Left", "BottomLeftPoint", "Nets are defined by 4 points. This is point 4.", self.point_at(BL));
                r.int("Settings", "Light Spacing", "PixelSpacing", "This is the spacing between each light in the net.", *pixel_spacing);
            }
            ShapeKind::MegaTree { string_count, top_height, top_width, base_height, lights_per_string, degrees, degree_offset } => {
                r.int("Settings", "Top Height", "TopHeight", "Height of Mega Tree top.", *top_height);
                r.int("Settings", "Top Width", "TopWidth", "Width of Mega Tree top.", *top_width);
                r.int("Settings", "Base Height", "BaseHeight", "Height of Mega Tree base.", *base_height);
                r.int("Settings", "String Coverage Degrees", "Degrees", "Coverage in degrees that strings go around Mega Tree.", *degrees);
                r.int("Settings", "Degree Offset", "DegreeOffset", "Degree offset can be used to center the Mega Tree.", *degree_offset);
                r.int("Settings", "Lights Per String", "LightsPerString", "The number of lights on each string of the Mega Tree", *lights_per_string);
                r.int("Settings", "String Count", "StringCount", "Number of strings on Mega Tree", *string_count);
                r.point("Top Left", "TopLeft", "Upper left point of Mega Tree.", self.point_at(TL));
                r.point("Bottom Right", "BottomRight", "Lower right point of Mega Tree.", self.point_at(BR));
            }
            ShapeKind::MultiString { .. } => {}
            ShapeKind::Icicle { string_count, vertical_spacing, .. } => {
                r.int("Settings", "Vertical Spacing", "VerticalSpacing", "This is the spacing between each light in a vertical icicle strand.", (*vertical_spacing).max(2));
                r.point("Point 1", "Point1", "Icicles are defined by 2 points. This is point 1.", self.point_at(0));
                r.point("Point 2", "Point2", "Icicles are defined by 2 points. This is point 2.", self.point_at(1));
                r.int("Settings", "Icicle Count", "StringCount", "Number of icicles in the string.", *string_count);
            }
            ShapeKind::PixelGrid { string_count, lights_per_string, string_orientation } => {
                r.int("Settings", "Lights Per String", "LightsPerString", "The number of lights on each string of the Pixel Grid", *lights_per_string);
                r.int("Settings", "String Count", "StringCount", "Number of strings on Pixel Grid", *string_count);
                r.point("Bottom Right", "BottomRight", "Lower right point of Pixel Grid.", self.point_at(BR));
                r.point("Top Left", "TopLeft", "Upper left point of Pixel Grid.", self.point_at(TL));
                r.enumeration("Settings", "String Orientation", "StringOrientation", "Orientation of strings in pixel grid. Vertical is Up/Down, Horizontal is Left/Right", ORIENTATIONS, string_orientation.name());
            }
            ShapeKind::Single => r.point("Position", "Point1", "An point is defined by a point on the screen.", self.point_at(0)),
            ShapeKind::Flood => r.point("Point 1", "Point1", "Spot lights are defined by 1 point.", self.point_at(0)),
            ShapeKind::Custom { .. } => {
                r.point("Position", "TopLeft", "Top, left position of this custom object.", PreviewPoint::new(self.left(), self.top()));
            }
            ShapeKind::CustomProp(c) => {
                let mut ordered = |name, id, order, value| r.0.push(ShapeProperty { category: "Position", name, id, description: "", kind: PropertyKind::Int, value, read_only: false, order });
                ordered("X", "Left", 1, PropertyValue::Int(c.bounds.x as i32));
                ordered("Y", "Top", 2, PropertyValue::Int(c.bounds.y as i32));
                r.0.push(ShapeProperty { category: "Position", name: "Top Left", id: "TopLeft", description: "", kind: PropertyKind::Point, value: PropertyValue::Point { x: c.bounds.x as i32, y: c.bounds.y as i32 }, read_only: false, order: 3 });
                r.0.push(ShapeProperty { category: "Position", name: "Bottom Right", id: "BottomRight", description: "", kind: PropertyKind::Point, value: PropertyValue::Point { x: c.bounds.right() as i32, y: c.bounds.bottom() as i32 }, read_only: false, order: 4 });
            }
            ShapeKind::MovingHead(d) => {
                r.point("Top Left", "TopLeftPoint", "Rectangles are defined by 4 points. This is point 1.", self.point_at(TL));
                r.point("Top Right", "TopRightPoint", "Rectangles are defined by 4 points. This is point 2.", self.point_at(TR));
                r.point("Bottom Right", "BottomRightPoint", "Rectangles are defined by 4 points. This is point 3.", self.point_at(BR));
                r.point("Botom Left", "BottomLeftPoint", "Rectangles are defined by 4 points. This is point 4.", self.point_at(BL));
                r.add(
                    "Settings",
                    "Linked Element",
                    "Node",
                    "Determines the Intelligent Fixture element the graphic is linked to.  Selecting the \u{2018}\u{2026}\u{2018} button allows you to pick an Intelligent Fixture element.",
                    PropertyKind::LinkedElement,
                    PropertyValue::LinkedElement((!d.node_id.is_nil()).then_some(d.node_id)),
                );
                let double = |r: &mut Rows, cat, name, id, desc, v: &DoubleText| r.add(cat, name, id, desc, PropertyKind::Double, PropertyValue::Double(v.value));
                double(&mut r, "Color Wheel", "Color Wheel Rotation Speed Minimum (s)", "MinColorWheelRotationSpeed", "The time it takes the color wheel to make a complete rotation in seconds.", &d.min_color_wheel_rotation_speed);
                double(&mut r, "Color Wheel", "Color Wheel Rotation Speed Maximum (s)", "MaxColorWheelRotationSpeed", "The time it takes the color wheel to make a complete rotation in seconds.", &d.max_color_wheel_rotation_speed);
                double(&mut r, "Pan", "Maximum Pan Travel Time (s)", "MaxPanTravelTime", "The time it takes of the intelligent fixture to pan from the starting position to the maximum stop position.", &d.max_pan_travel_time);
                double(&mut r, "Tilt", "Maximum Tilt Travel Time (s)", "MaxTiltTravelTime", "The time it takes of the intelligent fixture to tilt from the starting position to the maximum stop position.", &d.max_tilt_travel_time);
                r.int("Strobe", "Minimum Strobe Rate (ms)", "StrobeRateMinimum", "The strobe rate minimum (in Hz).", d.strobe_rate_minimum);
                r.int("Strobe", "Maximum Strobe Rate (Hz)", "StrobeRateMaximum", "The strobe rate maximum (in Hz).", d.strobe_rate_maximum);
                r.int("Strobe", "Maximum Strobe Duration (ms)", "MaximumStrobeDuration", "The maximum strobe duration in ms.", d.maximum_strobe_duration);
                r.int("Pan", "Pan Start Position (Degrees)", "PanStartPosition", "The pan starting point angle (in degrees).", d.pan_start_position);
                r.int("Pan", "Pan Stop Position (Degrees)", "PanStopPosition", "The pan stopping point angle (in degrees).", d.pan_stop_position);
                r.int("Tilt", "Tilt Start Position (Degrees)", "TiltStartPosition", "The tilt starting point angle (in degrees).", d.tilt_start_position);
                r.int("Tilt", "Tilt Stop Position (Degrees)", "TiltStopPosition", "The tilt stopping point angle (in degrees).", d.tilt_stop_position);
                r.int("Beam", "Beam Length", "BeamLength", "Length of the beam as a percentage of the background height.", d.beam_length);
                r.int("Beam", "Beam Transparency (%)", "BeamTransparency", "Determines the transparency of the light beam. 0% is completely opaque.  100% is completely transparent.  Note at 100% transparency the beam is not visible.", d.beam_transparency);
                r.int("Beam", "Beam Width Multiplier", "BeamWidthMultiplier", "Beam Width Multiplier", d.beam_width_multiplier);
                r.add("Settings", "Zoom Narrow To Wide", "ZoomNarrowToWide", "Indicates whether the fixture zooms from a narrow beam to a wide beam or vice-versa", PropertyKind::Bool, PropertyValue::Bool(d.zoom_narrow_to_wide));
                r.add("Settings", "Show Legend", "ShowLegend", "When true enables a legend that will show a function label and the corresponding channel value.  The legend is only applicable to index and range functions that were populated with a Preview Legend character.", PropertyKind::Bool, PropertyValue::Bool(d.show_legend));
                r.enumeration("Pan", "Invert Pan Direction", "InvertPanDirection", "Changes the start point of the pan by 180 degrees and inverts the direction of movement.  This setting is often used with 'Top' (upside down) mounting position.", YES_NO, d.invert_pan_direction.name());
                r.enumeration("Tilt", "Invert Tilt Direction", "InvertTiltDirection", "Swaps the start position with the stop position and inverts the direction of movement.  This setting is often used with 'Top' (upside down) mounting position.", YES_NO, d.invert_tilt_direction.name());
                r.enumeration("Settings", "Mounting Position", "MountingPosition", "Selects the mounting position of the fixture.  This property allows for simulating the fixture being mounted upside down.", MOUNTING, d.mounting_position.name());
            }
            ShapeKind::Unknown(_) => {}
        }
        // PropertySorter: by order, then by property name. The grid's categories are alphabetical.
        r.0.sort_by(|a, b| a.category.cmp(b.category).then(a.order.cmp(&b.order)).then(a.id.cmp(b.id)));
        r.0
    }

    /// A property edit, by display name or C# name. Ok(Some(text)) is a warning to show.
    pub fn set_property(&mut self, name: &str, value: PropertyValue, draw_shape: &str) -> Result<Option<&'static str>, PropertyError> {
        let row = self.properties().into_iter().find(|p| p.name == name || p.id == name).ok_or(PropertyError::UnknownProperty)?;
        if row.read_only {
            return Err(PropertyError::ReadOnly);
        }
        if row.kind == PropertyKind::LinkedElements {
            return Err(PropertyError::UseLinker);
        }
        let id = row.id;
        match (row.kind, value) {
            (PropertyKind::Text, PropertyValue::Text(t)) => {
                // Name: `string.IsNullOrEmpty(value) ? VixenPreviewSetup3.DrawShape : value`
                self.name = Some(if t.is_empty() { draw_shape.to_string() } else { t });
            }
            (PropertyKind::Bool, PropertyValue::Bool(b)) => self.set_bool_property(id, b),
            (PropertyKind::Int, PropertyValue::Int(v)) => self.set_int_property(id, v),
            (PropertyKind::Point, PropertyValue::Point { x, y }) => self.set_point_property(id, x, y),
            (PropertyKind::Double, PropertyValue::Double(v)) => {
                if let ShapeKind::MovingHead(d) = &mut self.kind {
                    let v = DoubleText::new(v);
                    match id {
                        "MinColorWheelRotationSpeed" => d.min_color_wheel_rotation_speed = v,
                        "MaxColorWheelRotationSpeed" => d.max_color_wheel_rotation_speed = v,
                        "MaxPanTravelTime" => d.max_pan_travel_time = v,
                        "MaxTiltTravelTime" => d.max_tilt_travel_time = v,
                        _ => return Err(PropertyError::UnknownProperty),
                    }
                }
            }
            (PropertyKind::Enum(members), PropertyValue::Enum(e)) => {
                if !members.contains(&e.as_str()) {
                    return Err(PropertyError::WrongType);
                }
                self.set_enum_property(id, &e);
            }
            (PropertyKind::LinkedElement, PropertyValue::LinkedElement(n)) => {
                if let ShapeKind::MovingHead(d) = &mut self.kind {
                    d.node_id = n.unwrap_or_else(Uuid::nil);
                }
            }
            _ => return Err(PropertyError::WrongType),
        }
        Ok(self.validate_moving_head(id))
    }

    fn set_bool_property(&mut self, id: &str, v: bool) {
        match id {
            "UseCommonLocation" => self.use_common_location = v,
            "Locked" => self.locked = v,
            _ => {
                if let ShapeKind::MovingHead(d) = &mut self.kind {
                    match id {
                        "ZoomNarrowToWide" => d.zoom_narrow_to_wide = v,
                        "ShowLegend" => d.show_legend = v,
                        _ => {}
                    }
                }
            }
        }
    }

    fn set_int_property(&mut self, id: &str, v: i32) {
        match (id, &mut self.kind) {
            ("RotationAngle", _) => self.set_rotation_angle(v),
            ("PixelSize", _) => self.set_pixel_size(v),
            ("PixelCount", ShapeKind::Line { .. } | ShapeKind::Arch | ShapeKind::Ellipse) => self.set_line_pixel_count(v),
            ("PixelCount", ShapeKind::PolyLine { .. }) => self.set_poly_line_pixel_count(v),
            ("PixelCount", ShapeKind::Star { .. }) => self.set_star_pixel_count(v),
            ("Width", ShapeKind::Arch) => self.set_arch_width(v),
            ("Height", ShapeKind::Arch) => self.set_arch_height(v),
            ("LinePixelCount", _) => self.set_cane_line_pixel_count(v),
            ("ArchPixelCount", _) => self.set_cane_arch_pixel_count(v),
            ("LightCountString1", _) => self.set_string_light_count(0, v),
            ("LightCountString2", _) => self.set_string_light_count(1, v),
            ("LightCountString3", _) => self.set_string_light_count(2, v),
            ("LightCountString4", _) => self.set_string_light_count(3, v),
            ("XYRotation", ShapeKind::Star { xy_rotation, .. } | ShapeKind::StarBurst { xy_rotation, .. }) => {
                *xy_rotation = v;
                self.layout();
            }
            ("InsideSize", ShapeKind::Star { inside_size, .. }) => {
                *inside_size = v;
                self.layout();
            }
            ("PointCount", ShapeKind::Star { point_count, .. }) => {
                *point_count = v;
                self.layout();
            }
            ("LightCount", ShapeKind::StarBurst { .. }) => self.set_star_burst_light_count(v),
            ("InnerCircleSize", ShapeKind::StarBurst { inner_circle_size, .. }) => {
                *inner_circle_size = v;
                self.layout();
            }
            ("StringCount", ShapeKind::StarBurst { .. }) => self.set_star_burst_string_count(v),
            ("PixelSpacing", ShapeKind::Net { pixel_spacing, .. }) => {
                *pixel_spacing = v;
                self.layout();
            }
            ("TopHeight", ShapeKind::MegaTree { top_height, .. }) => {
                *top_height = v;
                self.layout();
            }
            ("TopWidth", ShapeKind::MegaTree { top_width, .. }) => {
                *top_width = v;
                self.layout();
            }
            ("BaseHeight", ShapeKind::MegaTree { base_height, .. }) => {
                *base_height = v;
                self.layout();
            }
            ("Degrees", ShapeKind::MegaTree { degrees, .. }) => {
                *degrees = v;
                self.layout();
            }
            ("DegreeOffset", ShapeKind::MegaTree { degree_offset, .. }) => {
                *degree_offset = v;
                self.layout();
            }
            ("LightsPerString", ShapeKind::MegaTree { .. }) => self.set_mega_tree_lights_per_string(v),
            ("StringCount", ShapeKind::MegaTree { .. }) => self.set_mega_tree_string_count(v),
            ("VerticalSpacing", ShapeKind::Icicle { vertical_spacing, .. }) => {
                *vertical_spacing = v;
                self.layout();
            }
            ("StringCount", ShapeKind::Icicle { .. }) => self.set_icicle_string_count(v),
            ("LightsPerString", ShapeKind::PixelGrid { .. }) => self.set_pixel_grid_lights_per_string(v),
            ("StringCount", ShapeKind::PixelGrid { .. }) => self.set_pixel_grid_string_count(v),
            ("Left", ShapeKind::CustomProp(_)) => self.set_left(v),
            ("Top", ShapeKind::CustomProp(_)) => self.set_top(v),
            (_, ShapeKind::MovingHead(d)) => match id {
                "StrobeRateMinimum" => d.strobe_rate_minimum = v,
                "StrobeRateMaximum" => d.strobe_rate_maximum = v,
                "MaximumStrobeDuration" => d.maximum_strobe_duration = v,
                "PanStartPosition" => d.pan_start_position = v,
                "PanStopPosition" => d.pan_stop_position = v,
                "TiltStartPosition" => d.tilt_start_position = v,
                "TiltStopPosition" => d.tilt_stop_position = v,
                "BeamLength" => d.beam_length = v,
                "BeamTransparency" => d.beam_transparency = v,
                "BeamWidthMultiplier" => d.beam_width_multiplier = v,
                _ => {}
            },
            _ => {}
        }
    }

    fn set_point_property(&mut self, id: &str, x: i32, y: i32) {
        let p = PreviewPoint::new(x, y);
        match (&self.kind, id) {
            (ShapeKind::Line { .. } | ShapeKind::Icicle { .. }, "Point1") => self.set_point_setter(0, p),
            (ShapeKind::Line { .. } | ShapeKind::Icicle { .. }, "Point2") => self.set_point_setter(1, p),
            (ShapeKind::Triangle { .. }, "Point1") => self.set_point_setter(triangle::P1, p),
            (ShapeKind::Triangle { .. }, "Point2") => self.set_point_setter(triangle::P2, p),
            (ShapeKind::Triangle { .. }, "Point3") => self.set_point_setter(triangle::P3, p),
            (ShapeKind::Arch, "TopLeft" | "BottomRight") => {
                let i = if id == "TopLeft" { TL } else { BR };
                self.points[i].x = x;
                self.points[i].y = y;
                self.sync_arch_corners();
                self.reset_rotation_axis();
                self.layout();
            }
            (ShapeKind::Cane { .. }, "ArchLeft") => self.set_point_setter(cane::ARCH, p),
            (ShapeKind::Single | ShapeKind::Flood, "Point1") => {
                // These setters leave the rotation axis alone.
                self.points[0].x = x;
                self.points[0].y = y;
                self.layout();
            }
            (ShapeKind::Custom { .. }, "TopLeft") => self.move_to(x, y),
            (ShapeKind::CustomProp(_), "TopLeft") => self.set_custom_prop_corner(0, x, y, Modifiers::NONE),
            (ShapeKind::CustomProp(_), "BottomRight") => self.set_custom_prop_corner(2, x, y, Modifiers::NONE),
            (ShapeKind::MovingHead(_), _) => {
                // These setters leave the rotation axis alone, and a moving head has no lights to lay out.
                let i = four_corner_index(id);
                if let Some(q) = i.and_then(|i| self.points.get_mut(i)) {
                    q.x = x;
                    q.y = y;
                }
            }
            (_, _) => {
                if let Some(i) = four_corner_index(id) {
                    self.set_point_setter(i, p);
                }
            }
        }
    }

    fn set_enum_property(&mut self, id: &str, e: &str) {
        match (id, &mut self.kind) {
            ("StringType", _) => {
                if let Some(t) = StringType::parse(e) {
                    self.set_string_type(t);
                }
            }
            ("Direction", ShapeKind::Rectangle { direction, .. }) => {
                if let Some(d) = Direction::parse(e) {
                    *direction = d;
                    self.layout();
                }
            }
            // The star burst's Direction is a plain auto property, so nothing lays out.
            ("Direction", ShapeKind::StarBurst { direction, .. }) => {
                if let Some(d) = Direction::parse(e) {
                    *direction = d;
                }
            }
            ("StringDirection", ShapeKind::Star { string_direction, .. }) => {
                if let Some(d) = Direction::parse(e) {
                    *string_direction = d;
                    self.layout();
                }
            }
            ("StringOrientation", ShapeKind::PixelGrid { string_orientation, .. }) => {
                if let Some(o) = StringOrientation::parse(e) {
                    *string_orientation = o;
                    self.layout();
                }
            }
            ("InvertPanDirection", ShapeKind::MovingHead(d)) => d.invert_pan_direction = YesNo::parse(e).unwrap_or_default(),
            ("InvertTiltDirection", ShapeKind::MovingHead(d)) => d.invert_tilt_direction = YesNo::parse(e).unwrap_or_default(),
            ("MountingPosition", ShapeKind::MovingHead(d)) => d.mounting_position = MountingPosition::parse(e).unwrap_or_default(),
            _ => {}
        }
    }
}

/// The points of the shapes that name theirs `TopLeft` or `TopLeftPoint`.
fn four_corner_index(id: &str) -> Option<usize> {
    match id {
        "TopLeft" | "TopLeftPoint" => Some(TL),
        "BottomRight" | "BottomRightPoint" => Some(BR),
        "TopRightPoint" => Some(TR),
        "BottomLeftPoint" => Some(BL),
        _ => None,
    }
}
