//! Port of the geometry and data of Shapes/PreviewMovingHead.cs and PreviewMovingHeadPartial.cs. Its
//! drawing, beam math and intent handling are not here. The renderer gets the fixture rectangle and link.

use super::{Modifiers, PointRef, Sel, Shape, ShapeKind, BL, BR, TL, TR};
use crate::point::{PointType, PreviewPoint};
use crate::tools::{to_int32, Rect};
use crate::xml_util::DoubleText;
use uuid::Uuid;

/// `YesNoType`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum YesNo {
    #[default]
    No,
    Yes,
}

impl YesNo {
    pub fn name(self) -> &'static str {
        match self {
            YesNo::No => "No",
            YesNo::Yes => "Yes",
        }
    }

    pub fn parse(s: &str) -> Option<YesNo> {
        match s.trim() {
            "No" => Some(YesNo::No),
            "Yes" => Some(YesNo::Yes),
            _ => None,
        }
    }
}

/// `MountingPositionType`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum MountingPosition {
    #[default]
    Bottom,
    Top,
}

impl MountingPosition {
    pub fn name(self) -> &'static str {
        match self {
            MountingPosition::Bottom => "Bottom",
            MountingPosition::Top => "Top",
        }
    }

    pub fn parse(s: &str) -> Option<MountingPosition> {
        match s.trim() {
            "Bottom" => Some(MountingPosition::Bottom),
            "Top" => Some(MountingPosition::Top),
            _ => None,
        }
    }
}

pub(crate) const DEFAULT_PAN_START_POSITION: i32 = 0;
pub(crate) const DEFAULT_PAN_STOP_POSITION: i32 = 360;
pub(crate) const DEFAULT_TILT_START_POSITION: i32 = 0;
pub(crate) const DEFAULT_TILT_STOP_POSITION: i32 = 180;
pub(crate) const DEFAULT_BEAM_LENGTH: i32 = 100;
pub(crate) const DEFAULT_STROBE_RATE_MINIMUM: i32 = 1;
pub(crate) const DEFAULT_STROBE_RATE_MAXIMUM: i32 = 25;
pub(crate) const DEFAULT_MAX_STROBE_DURATION: i32 = 50;
pub(crate) const DEFAULT_MAX_PAN_TRAVEL_TIME: f64 = 4.6;
pub(crate) const DEFAULT_MAX_TILT_TRAVEL_TIME: f64 = 1.7;
pub(crate) const DEFAULT_MIN_COLOR_WHEEL_ROTATION_SPEED: f64 = 158.0;
pub(crate) const DEFAULT_MAX_COLOR_WHEEL_ROTATION_SPEED: f64 = 1.0;

/// The data members of a moving head. A newly linked shape copies pan, tilt and zoom constraints from
/// the Intelligent Fixture property; a saved shape retains its own editable constraints.
#[derive(Clone, Debug, Default)]
pub struct MovingHeadData {
    pub node_id: Uuid,
    pub min_color_wheel_rotation_speed: DoubleText,
    pub max_color_wheel_rotation_speed: DoubleText,
    pub max_pan_travel_time: DoubleText,
    pub max_tilt_travel_time: DoubleText,
    pub strobe_rate_minimum: i32,
    pub strobe_rate_maximum: i32,
    pub maximum_strobe_duration: i32,
    pub pan_start_position: i32,
    pub pan_stop_position: i32,
    pub tilt_start_position: i32,
    pub tilt_stop_position: i32,
    pub beam_length: i32,
    pub beam_transparency: i32,
    pub beam_width_multiplier: i32,
    pub zoom_narrow_to_wide: bool,
    pub show_legend: bool,
    pub invert_pan_direction: YesNo,
    pub invert_tilt_direction: YesNo,
    pub mounting_position: MountingPosition,
    pub(crate) ignore_mouse_move: bool,
    pub(crate) lock_xy: bool,
}

impl MovingHeadData {
    /// The zero values a deserialized fixture from before these members existed gets defaults for.
    pub(crate) fn apply_load_defaults(&mut self) {
        if self.strobe_rate_minimum == 0 {
            self.strobe_rate_minimum = DEFAULT_STROBE_RATE_MINIMUM;
        }
        if self.strobe_rate_maximum == 0 {
            self.strobe_rate_maximum = DEFAULT_STROBE_RATE_MAXIMUM;
        }
        if self.maximum_strobe_duration == 0 {
            self.maximum_strobe_duration = DEFAULT_MAX_STROBE_DURATION;
        }
        if self.max_pan_travel_time.value == 0.0 {
            self.max_pan_travel_time = DoubleText::new(DEFAULT_MAX_PAN_TRAVEL_TIME);
        }
        if self.max_tilt_travel_time.value == 0.0 {
            self.max_tilt_travel_time = DoubleText::new(DEFAULT_MAX_TILT_TRAVEL_TIME);
        }
        if self.min_color_wheel_rotation_speed.value == 0.0 {
            self.min_color_wheel_rotation_speed = DoubleText::new(DEFAULT_MIN_COLOR_WHEEL_ROTATION_SPEED);
        }
        if self.max_color_wheel_rotation_speed.value == 0.0 {
            self.max_color_wheel_rotation_speed = DoubleText::new(DEFAULT_MAX_COLOR_WHEEL_ROTATION_SPEED);
        }
    }
}

impl Shape {
    /// `new PreviewMovingHead(point1, selectedNode, zoomLevel)`.
    pub fn new_moving_head(point1: PreviewPoint, node: Option<Sel>, zoom_level: f64) -> Shape {
        let data = MovingHeadData {
            pan_start_position: DEFAULT_PAN_START_POSITION,
            pan_stop_position: DEFAULT_PAN_STOP_POSITION,
            tilt_start_position: DEFAULT_TILT_START_POSITION,
            tilt_stop_position: DEFAULT_TILT_STOP_POSITION,
            beam_length: DEFAULT_BEAM_LENGTH,
            // Default to partially transparent
            beam_transparency: 40,
            // Default the top beam width to 8 times larger than the base width
            beam_width_multiplier: 8,
            // Default to showing the legend
            show_legend: true,
            strobe_rate_minimum: DEFAULT_STROBE_RATE_MINIMUM,
            strobe_rate_maximum: DEFAULT_STROBE_RATE_MAXIMUM,
            min_color_wheel_rotation_speed: DoubleText::new(DEFAULT_MIN_COLOR_WHEEL_ROTATION_SPEED),
            max_color_wheel_rotation_speed: DoubleText::new(DEFAULT_MAX_COLOR_WHEEL_ROTATION_SPEED),
            maximum_strobe_duration: DEFAULT_MAX_STROBE_DURATION,
            max_pan_travel_time: DoubleText::new(DEFAULT_MAX_PAN_TRAVEL_TIME),
            max_tilt_travel_time: DoubleText::new(DEFAULT_MAX_TILT_TRAVEL_TIME),
            ..MovingHeadData::default()
        };
        let mut s = Shape::constructed(ShapeKind::MovingHead(Box::new(data)));
        s.zoom_level = zoom_level;
        let p = s.point_to_zoom_point(point1);
        s.points = vec![
            PreviewPoint::typed(p.x, p.y, PointType::SizeTopLeft),
            PreviewPoint::typed(p.x, p.y, PointType::SizeBottomRight),
            PreviewPoint::typed(p.x, p.y, PointType::SizeTopRight),
            PreviewPoint::typed(p.x, p.y, PointType::SizeBottomLeft),
        ];
        s.show_rotation = false;
        s.reconfigure_moving_head(node);
        s
    }

    pub(crate) fn reconfigure_moving_head(&mut self, node: Option<Sel>) {
        if let (ShapeKind::MovingHead(d), Some(n)) = (&mut self.kind, node) {
            d.node_id = n.uuid();
            d.pan_start_position = DEFAULT_PAN_START_POSITION;
            d.pan_stop_position = DEFAULT_PAN_STOP_POSITION;
            d.tilt_start_position = DEFAULT_TILT_START_POSITION;
            d.tilt_stop_position = DEFAULT_TILT_STOP_POSITION;
            d.beam_length = DEFAULT_BEAM_LENGTH;
            use vixen_core::element::{property_ids, PropertyData};
            if let Some(PropertyData::Unknown { raw_xml }) = n.tree.property(n.node, &property_ids::INTELLIGENT_FIXTURE).map(|p| &p.data) {
                if let Ok(xml) = vixen_xml::XElement::parse(raw_xml) {
                    let profile = if xml.local == "FixtureSpecification" { Some(&xml) } else { xml.child("FixtureSpecification") };
                    if let Some(functions) = profile.and_then(|p| p.child("FunctionDefinitions")) {
                        for identity in ["Pan", "Tilt", "Zoom"] {
                            let Some(function) = functions.elements().find(|f| f.child("FunctionIdentity").is_some_and(|v| v.text() == identity)) else { continue };
                            if identity == "Zoom" {
                                d.zoom_narrow_to_wide = function.child("ZoomType").is_none_or(|v| v.text() == "NarrowToWide");
                            } else if let Some(limits) = function.child("RotationLimits") {
                                let start = limits.child("StartPosition").and_then(|v| v.text().parse().ok()).unwrap_or(0);
                                let stop = limits.child("StopPosition").and_then(|v| v.text().parse().ok()).unwrap_or(360);
                                if identity == "Pan" { d.pan_start_position = start; d.pan_stop_position = stop; }
                                else { d.tilt_start_position = start; d.tilt_stop_position = stop; }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn moving_head(&self) -> Option<&MovingHeadData> {
        match &self.kind {
            ShapeKind::MovingHead(d) => Some(d),
            _ => None,
        }
    }

    pub(crate) fn set_moving_head_lock_xy(&mut self, v: bool) {
        if let ShapeKind::MovingHead(d) = &mut self.kind {
            d.lock_xy = v;
        }
    }

    pub(crate) fn set_moving_head_ignore_mouse(&mut self, v: bool) {
        if let ShapeKind::MovingHead(d) = &mut self.kind {
            d.ignore_mouse_move = v;
        }
    }

    pub(crate) fn mouse_move_moving_head(&mut self, x: i32, y: i32, change_x: i32, change_y: i32, m: Modifiers) {
        let ShapeKind::MovingHead(d) = &self.kind else { return };
        // If mouse move events are being honored then...
        if d.ignore_mouse_move {
            return;
        }
        let lock_xy = d.lock_xy;
        let point = self.point_to_zoom_point(PreviewPoint::new(x, y));
        if let Some(sel) = self.selected_point {
            self.set_point_xy(sel, point.x, point.y);
            if lock_xy || (sel == PointRef::Pt(BR) && m.only_ctrl()) {
                self.points[TR].x = point.x;
                self.points[BL].y = point.y;
            }
        } else if self.start_points.len() >= 4 {
            // If we get here, we're moving
            let zoom = self.zoom();
            for i in 0..4 {
                let s = self.start_points[i];
                let q = PreviewPoint::new(to_int32(s.x as f64 * zoom).wrapping_add(change_x), to_int32(s.y as f64 * zoom).wrapping_add(change_y));
                let q = self.point_to_zoom_point(q);
                self.points[i].x = q.x;
                self.points[i].y = q.y;
            }
        }
    }

    pub(crate) fn end_add_new_moving_head(&mut self) {
        // If the user created the shape right to left then...
        if self.points[BL].x > self.points[BR].x {
            // Swap the X coordinates
            let t = self.points[BL].x;
            self.points[BL].x = self.points[BR].x;
            self.points[BR].x = t;
        }
        // If the user created the shape right to left then...
        if self.points[TL].x > self.points[TR].x {
            // Swap the X coordinates
            let t = self.points[TL].x;
            self.points[TL].x = self.points[TR].x;
            self.points[TR].x = t;
        }
        // If the user created the shape bottom top then...
        if self.points[BL].y < self.points[TL].y {
            // Swap the top for the bottom
            self.points.swap(BL, TL);
            self.points.swap(BR, TR);
        }
        // Adjust the shape to form a square
        self.adjust_shape_to_square();
        // Turn on mouse move events
        self.set_moving_head_ignore_mouse(false);
    }

    pub(crate) fn on_move_point_moving_head(&mut self) {
        match self.selected_point {
            // Adjust the neighbouring points so we have a perfect square
            Some(PointRef::Pt(TL)) => {
                self.points[BL].x = self.points[TL].x;
                self.points[TR].y = self.points[TL].y;
            }
            Some(PointRef::Pt(BL)) => {
                self.points[TL].x = self.points[BL].x;
                self.points[BR].y = self.points[BL].y;
            }
            Some(PointRef::Pt(TR)) => {
                self.points[BR].x = self.points[TR].x;
                self.points[TL].y = self.points[TR].y;
            }
            Some(PointRef::Pt(BR)) => {
                self.points[TR].x = self.points[BR].x;
                self.points[BL].y = self.points[BR].y;
            }
            _ => {}
        }
        // Adjust the shape to form a square
        self.adjust_shape_to_square();
    }

    /// `AdjustShapeToSquare`.
    fn adjust_shape_to_square(&mut self) {
        let (l, t) = (self.left(), self.top());
        let w = self.right().wrapping_sub(l);
        let h = self.bottom().wrapping_sub(t);
        // If the height is greater than the width then...
        if h > w {
            // Adjust the height to match the smaller width
            self.points[BR].y = t.wrapping_add(w);
            self.points[BL].y = t.wrapping_add(w);
        } else {
            // Adjust the width to match the smaller height
            self.points[BR].x = l.wrapping_add(h);
            self.points[TR].x = l.wrapping_add(h);
        }
    }

    pub(crate) fn point_in_shape_moving_head(&self, point: PreviewPoint) -> bool {
        let zoom = self.zoom();
        let (x, y) = (point.x as f64, point.y as f64);
        x >= self.left() as f64 * zoom && x <= self.right() as f64 * zoom && y >= self.top() as f64 * zoom && y <= self.bottom() as f64 * zoom
    }

    pub(crate) fn shape_in_rect_moving_head(&self, rect: Rect, all_in: bool) -> bool {
        let (x1, y1, x2, y2) = rect.normalized();
        let (x1, y1, x2, y2) = (x1 as f64, y1 as f64, x2 as f64, y2 as f64);
        let zoom = self.zoom();
        let left = self.left() as f64 * zoom;
        let right = self.right() as f64 * zoom;
        let top = self.top() as f64 * zoom;
        let bottom = self.bottom() as f64 * zoom;
        if all_in {
            x1 <= left && x2 >= right && y1 <= top && y2 >= bottom
        } else {
            x1 <= right && x2 >= left && y1 <= bottom && y2 >= top
        }
    }

    pub(crate) fn match_moving_head(&mut self, other: &Shape) {
        let tl = self.points[TL];
        let o = &other.points;
        // The Y offsets are all taken from the other's top right, not its top left. Deliberate, as in the C#.
        self.points[TR].x = tl.x.wrapping_add(o[TR].x.wrapping_sub(o[TL].x));
        self.points[TR].y = tl.y.wrapping_add(o[TR].y.wrapping_sub(o[TR].y));
        self.points[BR].x = tl.x.wrapping_add(o[BR].x.wrapping_sub(o[TL].x));
        self.points[BR].y = tl.y.wrapping_add(o[BR].y.wrapping_sub(o[TR].y));
        self.points[BL].x = tl.x.wrapping_add(o[BL].x.wrapping_sub(o[TL].x));
        self.points[BL].y = tl.y.wrapping_add(o[BL].y.wrapping_sub(o[TR].y));
        self.layout();
    }

    /// `PreviewMovingHead.Validate`: the warning a property edit can raise, or None.
    pub fn validate_moving_head(&self, property: &str) -> Option<&'static str> {
        let d = self.moving_head()?;
        match property {
            "PanStartPosition" | "PanStopPosition" if d.pan_start_position > d.pan_stop_position => {
                Some("The 'Pan Start Position' should be less than the 'Pan Stop Position'.  Please correct the values.")
            }
            "TiltStartPosition" | "TiltStopPosition" if d.tilt_start_position > d.tilt_stop_position => {
                Some("The 'Tilt Start Position' should be less than the 'Tilt Stop Position'.  Please correct the values.")
            }
            "StrobeRateMinimum" | "StrobeRateMaximum" if d.strobe_rate_minimum > d.strobe_rate_maximum => {
                Some("The 'Strobe Rate Minimum' should be less than the 'Strobe Rate Maximum'.  Please correct the values.")
            }
            _ => None,
        }
    }
}
