//! Reading and writing shapes in `DataContractSerializer`'s form, and the `[OnDeserialized]` chain.
//!
//! `DataContractSerializer` runs no constructors or field initialisers, then sets members in file order,
//! then calls the `[OnDeserialized]` methods from the base class down. `rehydrate` is those methods.
//! Member order is the serializer's: base class first, ordinal by name within each class.

use super::custom_prop::CustomPropData;
use super::moving_head::{MountingPosition, MovingHeadData, YesNo};
use super::{cane, custom, triangle, Direction, Shape, ShapeKind, StringOrientation, StringType, BL, BR, TL, TR};
use crate::pixel::PreviewPixel;
use crate::point::{PointType, PreviewPoint};
use crate::xml_util::{el_bool, el_i32, el_text, read_bool, read_i32, DoubleText, Malformed, Member, Parse, XmlMemo, SHAPES_NS};
use uuid::Uuid;
use vixen_xml::{XElement, XNode};

/// Reads a shape, keeping it raw if its type is unknown or a known type is malformed.
pub(crate) fn shape_from_xml(el: &XElement) -> Shape {
    parse_shape(el).unwrap_or_else(|_| Shape::unknown(el))
}

fn blank_kind(type_name: &str) -> Option<ShapeKind> {
    Some(match type_name {
        "PreviewLine" => ShapeKind::Line { add_start_padding: false, add_end_padding: false, light_count: 0 },
        "PreviewPolyLine" => ShapeKind::PolyLine { create_default_pixels: false },
        "PreviewArch" => ShapeKind::Arch,
        "PreviewCane" => ShapeKind::Cane { arch_pixel_count: 0, line_pixel_count: 0, just_placed: false },
        "PreviewRectangle" => ShapeKind::Rectangle { direction: Direction::Clockwise, lock_xy: false },
        "PreviewEllipse" => ShapeKind::Ellipse,
        "PreviewTriangle" => ShapeKind::Triangle { bottom_right_start: None },
        "PreviewStar" => ShapeKind::Star { point_count: 0, pixel_count: 0, inside_size: 0, string_direction: Direction::Clockwise, xy_rotation: 0, pixels_per_point: 0 },
        "PreviewStarBurst" => ShapeKind::StarBurst { direction: Direction::Clockwise, xy_rotation: 0, inner_circle_size: 0 },
        "PreviewNet" => ShapeKind::Net { pixel_spacing: 0, lock_xy: false, initially_assigned_node: None, loaded_pixels: None },
        "PreviewMegaTree" => ShapeKind::MegaTree { string_count: 0, top_height: 0, top_width: 0, base_height: 0, lights_per_string: 0, degrees: 0, degree_offset: 0 },
        "PreviewMultiString" => ShapeKind::MultiString { string_count: 0, input_elements: None },
        "PreviewIcicle" => ShapeKind::Icicle { string_count: 0, vertical_spacing: 0, creating: false, initial_node: None },
        "PreviewPixelGrid" => ShapeKind::PixelGrid { string_count: 0, lights_per_string: 0, string_orientation: StringOrientation::Vertical },
        "PreviewSingle" => ShapeKind::Single,
        "PreviewFlood" => ShapeKind::Flood,
        "PreviewCustom" => ShapeKind::Custom { start_width: 0, start_height: 0 },
        "PreviewCustomProp" => ShapeKind::CustomProp(Box::default()),
        "PreviewMovingHead" => ShapeKind::MovingHead(Box::default()),
        _ => return None,
    })
}

/// How many entries `Shape::points` has before reading, and which must be in the file.
fn point_layout(kind: &ShapeKind) -> (usize, u8) {
    let all = |n: u8| (1u8 << n) - 1;
    match kind {
        ShapeKind::Arch | ShapeKind::Ellipse => (4, all(2)),
        ShapeKind::Cane { .. } => (3, all(3)),
        ShapeKind::Rectangle { .. } | ShapeKind::StarBurst { .. } | ShapeKind::Net { .. } | ShapeKind::MovingHead(_) => (4, all(4)),
        ShapeKind::Triangle { .. } => (4, all(3)),
        ShapeKind::Star { .. } => (2, all(2)),
        ShapeKind::MegaTree { .. } => (4, all(2)),
        // The top right is rebuilt by its getter, so only it may be missing.
        ShapeKind::PixelGrid { .. } => (4, (1 << TL) | (1 << BR) | (1 << BL)),
        ShapeKind::Single => (1, 1),
        ShapeKind::Flood => (1, 0),
        ShapeKind::Custom { .. } => (2, 0),
        _ => (0, 0),
    }
}

/// `(element name, index in Shape::points)` for the shapes whose points are separate members.
fn point_members(kind: &ShapeKind) -> &'static [(&'static str, usize)] {
    match kind {
        ShapeKind::Arch | ShapeKind::Ellipse => &[("_bottomRight", BR), ("_topLeft", TL)],
        ShapeKind::Cane { .. } => &[("_archLeftPoint", cane::ARCH), ("_bottomRightPoint", BR), ("_topLeftPoint", TL)],
        ShapeKind::Rectangle { .. } | ShapeKind::StarBurst { .. } | ShapeKind::Net { .. } | ShapeKind::MegaTree { .. } | ShapeKind::PixelGrid { .. } | ShapeKind::MovingHead(_) => {
            &[("_bottomLeft", BL), ("_bottomRight", BR), ("_topLeft", TL), ("_topRight", TR)]
        }
        ShapeKind::Triangle { .. } => &[("_point1", triangle::P1), ("_point2", triangle::P2), ("_point3", triangle::P3)],
        ShapeKind::Star { .. } => &[("_bottomRightPoint", BR), ("_topLeftPoint", TL)],
        ShapeKind::Single => &[("p1", 0)],
        ShapeKind::Flood => &[("_p1", 0)],
        ShapeKind::Custom { .. } => &[("_topLeft", custom::TOP_LEFT)],
        _ => &[],
    }
}

fn read_pixels(e: &XElement) -> Parse<Vec<PreviewPixel>> {
    let mut out = Vec::with_capacity(e.children.len());
    for c in e.elements() {
        out.push(PreviewPixel::from_xml(c)?);
    }
    Ok(out)
}

fn read_points(e: &XElement) -> Parse<Vec<PreviewPoint>> {
    let mut out = Vec::with_capacity(e.children.len());
    for c in e.elements() {
        if c.local != "PreviewPoint" {
            return Err(Malformed);
        }
        out.push(PreviewPoint::from_xml(c)?);
    }
    Ok(out)
}

fn parse_shape(el: &XElement) -> Parse<Shape> {
    if el.nil {
        return Err(Malformed);
    }
    let (ns, type_name) = el.xsi_type.as_ref().ok_or(Malformed)?;
    if ns != SHAPES_NS {
        return Err(Malformed);
    }
    let kind = blank_kind(type_name).ok_or(Malformed)?;
    let (point_count, required) = point_layout(&kind);
    let mut s = Shape::uninitialized(kind);
    s.points = vec![PreviewPoint::default(); point_count];
    let mut memo = XmlMemo::with_attrs(&el.attrs);
    let mut present = 0u8;
    for e in el.elements() {
        // A second member of the same name, or one in another namespace, is not ours to interpret.
        let known = e.ns == SHAPES_NS && !memo.has_known(&e.local) && s.read_member(e, &mut memo, &mut present)?;
        if !known {
            memo.extra(e);
        }
    }
    if present & required != required {
        return Err(Malformed);
    }
    match &s.kind {
        ShapeKind::Line { .. } | ShapeKind::Icicle { .. } if s.points.len() < 2 => return Err(Malformed),
        ShapeKind::PolyLine { .. } | ShapeKind::MultiString { .. } if s.points.is_empty() => return Err(Malformed),
        _ => {}
    }
    if matches!(s.kind, ShapeKind::Flood) && present & 1 == 0 {
        // Layout: a flood from a file with no point goes here.
        s.points[0] = PreviewPoint::new(100, 100);
    }
    s.memo = Some(Box::new(memo));
    s.rehydrate();
    Ok(s)
}

impl Shape {
    /// Reads one member. False means it is not a member of this shape, so it stays raw.
    fn read_member(&mut self, e: &XElement, memo: &mut XmlMemo, present: &mut u8) -> Parse<bool> {
        let local = e.local.as_str();
        // PreviewBaseShape
        match local {
            "Locked" if !e.nil => {
                self.locked = read_bool(e)?;
                memo.known_default("Locked", !self.locked);
                return Ok(true);
            }
            "Name" if !e.nil => {
                self.name = Some(e.text());
                memo.known("Name");
                return Ok(true);
            }
            "RotationAngle" if !e.nil => {
                let v = read_i32(e)?;
                // The setter ignores anything outside this range.
                if (-360..=360).contains(&v) {
                    self.rotation_angle = v;
                }
                memo.known_default("RotationAngle", v == 0);
                return Ok(true);
            }
            "UseCommonLocation" if !e.nil => {
                self.use_common_location = read_bool(e)?;
                memo.known_default("UseCommonLocation", !self.use_common_location);
                return Ok(true);
            }
            _ => {}
        }
        // PreviewLightBaseShape
        if self.is_light_shape() {
            match local {
                "PixelSize" if !e.nil => {
                    let v = read_i32(e)?;
                    match &mut self.kind {
                        ShapeKind::Custom { .. } | ShapeKind::Flood => self.pixel_size = v,
                        ShapeKind::CustomProp(c) => c.pixel_size = v,
                        _ => self.pixel_size = if v > 0 { v } else { 3 },
                    }
                    memo.known("PixelSize");
                    return Ok(true);
                }
                "Pixels" if !e.nil => {
                    self.pixels = read_pixels(e)?;
                    memo.known("Pixels");
                    return Ok(true);
                }
                "StringType" if !e.nil => {
                    self.string_type = StringType::parse(&e.text()).ok_or(Malformed)?;
                    memo.known("StringType");
                    return Ok(true);
                }
                "_strings" if !e.nil => {
                    self.strings = Some(e.elements().map(shape_from_xml).collect());
                    memo.known_default("_strings", false);
                    return Ok(true);
                }
                _ => {}
            }
        }
        // The derived class's points
        if let Some((name, index)) = point_members(&self.kind).iter().find(|(n, _)| *n == local) {
            if e.nil {
                self.point_nil_mask |= 1 << *index;
                memo.known_default(name, true);
            } else {
                self.points[*index] = PreviewPoint::from_xml(e)?;
                memo.known(name);
            }
            *present |= 1 << *index;
            return Ok(true);
        }
        if e.nil {
            return Ok(false);
        }
        // The rest of the derived class
        let name: Option<&'static str> = match &mut self.kind {
            ShapeKind::Line { .. } | ShapeKind::PolyLine { .. } => match local {
                "_points" => {
                    self.points = read_points(e)?;
                    Some("_points")
                }
                _ => None,
            },
            ShapeKind::Cane { arch_pixel_count, line_pixel_count, .. } => match local {
                "_archPixelCount" => {
                    *arch_pixel_count = read_i32(e)?;
                    Some("_archPixelCount")
                }
                "_linePixelCount" => {
                    *line_pixel_count = read_i32(e)?;
                    Some("_linePixelCount")
                }
                _ => None,
            },
            ShapeKind::Rectangle { direction, .. } => match local {
                "Direction" => {
                    *direction = Direction::parse(&e.text()).ok_or(Malformed)?;
                    Some("Direction")
                }
                _ => None,
            },
            ShapeKind::Star { point_count, pixel_count, inside_size, string_direction, xy_rotation, .. } => match local {
                "_XYRotation" => {
                    *xy_rotation = read_i32(e)?;
                    Some("_XYRotation")
                }
                "_insideSize" => {
                    *inside_size = read_i32(e)?;
                    Some("_insideSize")
                }
                "_pixelCount" => {
                    *pixel_count = read_i32(e)?;
                    Some("_pixelCount")
                }
                "_pointCount" => {
                    *point_count = read_i32(e)?;
                    Some("_pointCount")
                }
                "_stringDirection" => {
                    *string_direction = Direction::parse(&e.text()).ok_or(Malformed)?;
                    Some("_stringDirection")
                }
                _ => None,
            },
            ShapeKind::StarBurst { direction, xy_rotation, inner_circle_size } => match local {
                "Direction" => {
                    *direction = Direction::parse(&e.text()).ok_or(Malformed)?;
                    Some("Direction")
                }
                "_XYRotation" => {
                    *xy_rotation = read_i32(e)?;
                    Some("_XYRotation")
                }
                "_innerCircleSize" => {
                    *inner_circle_size = read_i32(e)?;
                    Some("_innerCircleSize")
                }
                _ => None,
            },
            ShapeKind::Net { pixel_spacing, .. } => match local {
                "_pixelSpacing" => {
                    *pixel_spacing = read_i32(e)?;
                    Some("_pixelSpacing")
                }
                _ => None,
            },
            ShapeKind::MegaTree { string_count, top_height, top_width, base_height, lights_per_string, degrees, degree_offset } => {
                let (slot, name): (Option<&mut i32>, &'static str) = match local {
                    "_baseHeight" => (Some(base_height), "_baseHeight"),
                    "_degreeOffset" => (Some(degree_offset), "_degreeOffset"),
                    "_degrees" => (Some(degrees), "_degrees"),
                    "_lightsPerString" => (Some(lights_per_string), "_lightsPerString"),
                    "_stringCount" => (Some(string_count), "_stringCount"),
                    "_topHeight" => (Some(top_height), "_topHeight"),
                    "_topWidth" => (Some(top_width), "_topWidth"),
                    _ => (None, ""),
                };
                match slot {
                    Some(v) => {
                        *v = read_i32(e)?;
                        Some(name)
                    }
                    None => None,
                }
            }
            ShapeKind::MultiString { string_count, .. } => match local {
                "_points" => {
                    self.points = read_points(e)?;
                    Some("_points")
                }
                "_stringCount" => {
                    *string_count = read_i32(e)?;
                    Some("_stringCount")
                }
                _ => None,
            },
            ShapeKind::Icicle { string_count, vertical_spacing, .. } => match local {
                "_points" => {
                    self.points = read_points(e)?;
                    Some("_points")
                }
                "_stringCount" => {
                    *string_count = read_i32(e)?;
                    Some("_stringCount")
                }
                "_verticalSpacing" => {
                    *vertical_spacing = read_i32(e)?;
                    Some("_verticalSpacing")
                }
                _ => None,
            },
            ShapeKind::PixelGrid { string_count, lights_per_string, string_orientation } => match local {
                "_lightsPerString" => {
                    *lights_per_string = read_i32(e)?;
                    Some("_lightsPerString")
                }
                "_stringCount" => {
                    *string_count = read_i32(e)?;
                    Some("_stringCount")
                }
                "_stringOrientation" => {
                    *string_orientation = StringOrientation::parse(&e.text()).ok_or(Malformed)?;
                    Some("_stringOrientation")
                }
                _ => None,
            },
            ShapeKind::CustomProp(c) => match local {
                "PropPixels" => {
                    c.prop_pixels = read_pixels(e)?;
                    Some("PropPixels")
                }
                _ => None,
            },
            ShapeKind::MovingHead(d) => return read_moving_head_member(d, e, memo),
            _ => None,
        };
        match name {
            Some(n) => {
                memo.known(n);
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// The `[OnDeserialized]` chain: `PreviewBaseShape`, then `PreviewLightBaseShape`, then the derived class.
    pub(crate) fn rehydrate(&mut self) {
        // PreviewBaseShape.InitializeNonDataMembers
        let c = self.center();
        let top = self.top();
        self.rotation_axis = Some(c.with_type(PointType::RotationAxis));
        self.rotate_handle = Some(PreviewPoint::typed(c.x, top.wrapping_sub(10), PointType::RotateHandle));
        self.show_rotation = true;
        // PreviewLightBaseShape.OnDeserialized
        self.resize_pixels();
        // The derived class
        match &mut self.kind {
            ShapeKind::Line { .. } => {
                self.points[0].point_type = PointType::SizeTopLeft;
                self.points[1].point_type = PointType::SizeBottomRight;
            }
            ShapeKind::PolyLine { .. } | ShapeKind::MultiString { .. } => {
                for p in &mut self.points {
                    p.point_type = PointType::Size;
                }
                self.points[0].point_type = PointType::SizeTopLeft;
            }
            ShapeKind::Arch => {
                self.points[TL].point_type = PointType::SizeTopLeft;
                self.points[BR].point_type = PointType::SizeBottomRight;
                self.sync_arch_corners();
            }
            ShapeKind::Cane { .. } => {
                self.points[TL].point_type = PointType::SizeTopLeft;
                self.points[BR].point_type = PointType::SizeBottomRight;
                self.points[cane::ARCH].point_type = PointType::Size;
            }
            ShapeKind::Rectangle { .. } | ShapeKind::PixelGrid { .. } | ShapeKind::MovingHead(_) => {
                self.points[TL].point_type = PointType::SizeTopLeft;
                self.points[BR].point_type = PointType::SizeBottomRight;
                self.points[TR].point_type = PointType::SizeTopRight;
                self.points[BL].point_type = PointType::SizeBottomLeft;
            }
            ShapeKind::Ellipse | ShapeKind::Star { .. } | ShapeKind::StarBurst { .. } | ShapeKind::MegaTree { .. } => {
                // These leave any other corner's type alone: None until something rebuilds it.
                self.points[TL].point_type = PointType::SizeTopLeft;
                self.points[BR].point_type = PointType::SizeBottomRight;
            }
            ShapeKind::Triangle { .. } => {
                self.points[triangle::P1].point_type = PointType::SizeTopLeft;
                self.points[triangle::P2].point_type = PointType::SizeBottomLeft;
                self.points[triangle::P3].point_type = PointType::SizeBottomRight;
            }
            ShapeKind::Net { pixel_spacing, .. } => {
                if *pixel_spacing == 0 {
                    *pixel_spacing = 8;
                }
                self.points[TL].point_type = PointType::SizeTopLeft;
                self.points[BR].point_type = PointType::SizeBottomRight;
            }
            ShapeKind::Icicle { .. } => {
                self.points[0].point_type = PointType::SizeTopLeft;
                self.points[1].point_type = PointType::SizeTopRight;
            }
            ShapeKind::Single | ShapeKind::Custom { .. } => self.show_rotation = false,
            ShapeKind::Flood => self.string_type = StringType::Standard,
            ShapeKind::CustomProp(_) => {
                self.string_type = StringType::Custom;
                self.zoom_level = 1.0;
            }
            ShapeKind::Unknown(_) => {}
        }
        if let ShapeKind::MovingHead(d) = &mut self.kind {
            d.apply_load_defaults();
            self.show_rotation = false;
        }
        // The net's OnDeserialized does not lay out. The previews do it as soon as they open.
        if matches!(self.kind, ShapeKind::Net { .. }) {
            self.layout_net_after_load();
        } else {
            self.layout();
        }
    }

    /// What a shape would be after serializing and deserializing it, which is how the C# restores shapes
    /// on undo. Selection and drag state go, along with anything else that is not a data member.
    pub(crate) fn roundtrip_state(&mut self) {
        if let Some(strings) = &mut self.strings {
            for s in strings {
                s.roundtrip_state();
            }
        }
        self.rotation_axis = None;
        self.rotate_handle = None;
        self.selected = false;
        self.select_points = None;
        self.selected_point = None;
        self.detached = PreviewPoint::default();
        self.zoom_level = 0.0;
        self.creating = false;
        self.background_alpha = 0;
        self.connect_standard_strings = false;
        self.start_points.clear();
        for p in &mut self.pixels {
            p.z = 0;
        }
        match &mut self.kind {
            ShapeKind::Line { add_start_padding, add_end_padding, light_count } => {
                *add_start_padding = false;
                *add_end_padding = false;
                *light_count = 0;
            }
            ShapeKind::PolyLine { create_default_pixels } => *create_default_pixels = false,
            ShapeKind::Cane { just_placed, .. } => *just_placed = false,
            ShapeKind::Rectangle { lock_xy, .. } => *lock_xy = false,
            ShapeKind::Triangle { bottom_right_start } => *bottom_right_start = None,
            ShapeKind::Net { lock_xy, initially_assigned_node, loaded_pixels, .. } => {
                *lock_xy = false;
                *initially_assigned_node = None;
                *loaded_pixels = None;
            }
            ShapeKind::MultiString { input_elements, .. } => *input_elements = None,
            ShapeKind::Icicle { creating, initial_node, .. } => {
                *creating = false;
                *initial_node = None;
            }
            ShapeKind::Custom { start_width, start_height } => {
                *start_width = 0;
                *start_height = 0;
            }
            ShapeKind::CustomProp(c) => {
                let CustomPropData { drag_points, p1_start, selection_point, .. } = &mut **c;
                drag_points.clear();
                *p1_start = None;
                *selection_point = None;
            }
            ShapeKind::MovingHead(d) => {
                d.ignore_mouse_move = false;
                d.lock_xy = false;
            }
            _ => {}
        }
        if matches!(self.kind, ShapeKind::Unknown(_)) {
            return;
        }
        self.rehydrate();
    }

    /// The members, in `DataContractSerializer` order.
    fn known_members(&self) -> Vec<Member> {
        let ns = SHAPES_NS;
        let mut m: Vec<Member> = Vec::with_capacity(16);
        // PreviewBaseShape
        m.push(Member::optional("Locked", el_bool(ns, "Locked", self.locked), self.locked));
        m.push(Member::optional("Name", el_text(ns, "Name", self.name.clone().unwrap_or_default()), self.name.is_some()));
        m.push(Member::optional("RotationAngle", el_i32(ns, "RotationAngle", self.rotation_angle), self.rotation_angle != 0));
        m.push(Member::optional("UseCommonLocation", el_bool(ns, "UseCommonLocation", self.use_common_location), self.use_common_location));
        // PreviewLightBaseShape
        if self.is_light_shape() {
            m.push(Member::new("PixelSize", el_i32(ns, "PixelSize", self.pixel_size())));
            let stored: &[PreviewPixel] = match &self.kind {
                ShapeKind::Net { loaded_pixels: Some(p), .. } => p,
                _ => &self.pixels,
            };
            m.push(Member::new("Pixels", pixels_element("Pixels", stored)));
            m.push(Member::new("StringType", el_text(ns, "StringType", self.string_type().name())));
            let strings = self.strings.as_ref().map(|ss| {
                let mut e = XElement::new(ns, "_strings");
                for s in ss {
                    e.push(s.to_xml_named("PreviewBaseShape"));
                }
                e
            });
            let emit = strings.is_some();
            m.push(Member::optional("_strings", strings.unwrap_or_else(|| XElement::new(ns, "_strings")), emit));
        }
        // The derived class
        let point_el = |s: &Shape, name: &'static str, i: usize| -> Member {
            if s.point_nil_mask & (1 << i) != 0 {
                // Nil in the C#, where only a shape read without this member has it missing altogether.
                Member::optional(name, XElement::nil(ns, name), s.memo.is_none())
            } else {
                Member::new(name, s.points[i].to_xml(name))
            }
        };
        match &self.kind {
            ShapeKind::Line { .. } | ShapeKind::PolyLine { .. } => m.push(Member::new("_points", points_element(&self.points))),
            ShapeKind::Arch | ShapeKind::Ellipse => {
                m.push(point_el(self, "_bottomRight", BR));
                m.push(point_el(self, "_topLeft", TL));
            }
            ShapeKind::Cane { arch_pixel_count, line_pixel_count, .. } => {
                m.push(point_el(self, "_archLeftPoint", cane::ARCH));
                m.push(Member::new("_archPixelCount", el_i32(ns, "_archPixelCount", *arch_pixel_count)));
                m.push(point_el(self, "_bottomRightPoint", BR));
                m.push(Member::new("_linePixelCount", el_i32(ns, "_linePixelCount", *line_pixel_count)));
                m.push(point_el(self, "_topLeftPoint", TL));
            }
            ShapeKind::Rectangle { direction, .. } => {
                m.push(Member::new("Direction", el_text(ns, "Direction", direction.name())));
                m.push(point_el(self, "_bottomLeft", BL));
                m.push(point_el(self, "_bottomRight", BR));
                m.push(point_el(self, "_topLeft", TL));
                m.push(point_el(self, "_topRight", TR));
            }
            ShapeKind::Triangle { .. } => {
                m.push(point_el(self, "_point1", triangle::P1));
                m.push(point_el(self, "_point2", triangle::P2));
                m.push(point_el(self, "_point3", triangle::P3));
            }
            ShapeKind::Star { point_count, pixel_count, inside_size, string_direction, xy_rotation, .. } => {
                m.push(Member::new("_XYRotation", el_i32(ns, "_XYRotation", *xy_rotation)));
                m.push(point_el(self, "_bottomRightPoint", BR));
                m.push(Member::new("_insideSize", el_i32(ns, "_insideSize", *inside_size)));
                m.push(Member::new("_pixelCount", el_i32(ns, "_pixelCount", *pixel_count)));
                m.push(Member::new("_pointCount", el_i32(ns, "_pointCount", *point_count)));
                m.push(Member::new("_stringDirection", el_text(ns, "_stringDirection", string_direction.name())));
                m.push(point_el(self, "_topLeftPoint", TL));
            }
            ShapeKind::StarBurst { direction, xy_rotation, inner_circle_size } => {
                m.push(Member::new("Direction", el_text(ns, "Direction", direction.name())));
                m.push(Member::new("_XYRotation", el_i32(ns, "_XYRotation", *xy_rotation)));
                m.push(point_el(self, "_bottomLeft", BL));
                m.push(point_el(self, "_bottomRight", BR));
                m.push(Member::new("_innerCircleSize", el_i32(ns, "_innerCircleSize", *inner_circle_size)));
                m.push(point_el(self, "_topLeft", TL));
                m.push(point_el(self, "_topRight", TR));
            }
            ShapeKind::Net { pixel_spacing, .. } => {
                m.push(point_el(self, "_bottomLeft", BL));
                m.push(point_el(self, "_bottomRight", BR));
                m.push(Member::new("_pixelSpacing", el_i32(ns, "_pixelSpacing", *pixel_spacing)));
                m.push(point_el(self, "_topLeft", TL));
                m.push(point_el(self, "_topRight", TR));
            }
            ShapeKind::MegaTree { string_count, top_height, top_width, base_height, lights_per_string, degrees, degree_offset } => {
                m.push(Member::new("_baseHeight", el_i32(ns, "_baseHeight", *base_height)));
                m.push(point_el(self, "_bottomLeft", BL));
                m.push(point_el(self, "_bottomRight", BR));
                m.push(Member::new("_degreeOffset", el_i32(ns, "_degreeOffset", *degree_offset)));
                m.push(Member::new("_degrees", el_i32(ns, "_degrees", *degrees)));
                m.push(Member::new("_lightsPerString", el_i32(ns, "_lightsPerString", *lights_per_string)));
                m.push(Member::new("_stringCount", el_i32(ns, "_stringCount", *string_count)));
                m.push(Member::new("_topHeight", el_i32(ns, "_topHeight", *top_height)));
                m.push(point_el(self, "_topLeft", TL));
                m.push(point_el(self, "_topRight", TR));
                m.push(Member::new("_topWidth", el_i32(ns, "_topWidth", *top_width)));
            }
            ShapeKind::MultiString { string_count, .. } => {
                m.push(Member::new("_points", points_element(&self.points)));
                m.push(Member::new("_stringCount", el_i32(ns, "_stringCount", *string_count)));
            }
            ShapeKind::Icicle { string_count, vertical_spacing, .. } => {
                m.push(Member::new("_points", points_element(&self.points)));
                m.push(Member::new("_stringCount", el_i32(ns, "_stringCount", *string_count)));
                m.push(Member::new("_verticalSpacing", el_i32(ns, "_verticalSpacing", *vertical_spacing)));
            }
            ShapeKind::PixelGrid { string_count, lights_per_string, string_orientation } => {
                m.push(point_el(self, "_bottomLeft", BL));
                m.push(point_el(self, "_bottomRight", BR));
                m.push(Member::new("_lightsPerString", el_i32(ns, "_lightsPerString", *lights_per_string)));
                m.push(Member::new("_stringCount", el_i32(ns, "_stringCount", *string_count)));
                m.push(Member::new("_stringOrientation", el_text(ns, "_stringOrientation", string_orientation.name())));
                m.push(point_el(self, "_topLeft", TL));
                m.push(point_el(self, "_topRight", TR));
            }
            ShapeKind::Single => m.push(point_el(self, "p1", 0)),
            ShapeKind::Flood => m.push(point_el(self, "_p1", 0)),
            ShapeKind::Custom { .. } => m.push(point_el(self, "_topLeft", custom::TOP_LEFT)),
            ShapeKind::CustomProp(c) => m.push(Member::new("PropPixels", pixels_element("PropPixels", &c.prop_pixels))),
            ShapeKind::MovingHead(d) => {
                moving_head_members(d, &mut m);
                m.push(point_el(self, "_bottomLeft", BL));
                m.push(point_el(self, "_bottomRight", BR));
                m.push(point_el(self, "_topLeft", TL));
                m.push(point_el(self, "_topRight", TR));
            }
            ShapeKind::Unknown(_) => {}
        }
        m
    }

    /// `type_name` for `i:type`.
    fn xml_type_name(&self) -> &'static str {
        match &self.kind {
            ShapeKind::Line { .. } => "PreviewLine",
            ShapeKind::PolyLine { .. } => "PreviewPolyLine",
            ShapeKind::Arch => "PreviewArch",
            ShapeKind::Cane { .. } => "PreviewCane",
            ShapeKind::Rectangle { .. } => "PreviewRectangle",
            ShapeKind::Ellipse => "PreviewEllipse",
            ShapeKind::Triangle { .. } => "PreviewTriangle",
            ShapeKind::Star { .. } => "PreviewStar",
            ShapeKind::StarBurst { .. } => "PreviewStarBurst",
            ShapeKind::Net { .. } => "PreviewNet",
            ShapeKind::MegaTree { .. } => "PreviewMegaTree",
            ShapeKind::MultiString { .. } => "PreviewMultiString",
            ShapeKind::Icicle { .. } => "PreviewIcicle",
            ShapeKind::PixelGrid { .. } => "PreviewPixelGrid",
            ShapeKind::Single => "PreviewSingle",
            ShapeKind::Flood => "PreviewFlood",
            ShapeKind::Custom { .. } => "PreviewCustom",
            ShapeKind::CustomProp(_) => "PreviewCustomProp",
            ShapeKind::MovingHead(_) => "PreviewMovingHead",
            ShapeKind::Unknown(_) => "",
        }
    }

    /// Writes this shape as an element called `local`, usually `Shape` or `PreviewBaseShape`.
    pub(crate) fn to_xml_named(&self, local: &str) -> XElement {
        if let ShapeKind::Unknown(raw) = &self.kind {
            let mut e = (**raw).clone();
            e.local = local.to_string();
            e.ns = SHAPES_NS.to_string();
            return e;
        }
        let mut e = XElement::new(SHAPES_NS, local);
        e.xsi_type = Some((SHAPES_NS.to_string(), self.xml_type_name().to_string()));
        crate::xml_util::write_members(&mut e, self.known_members(), self.memo.as_deref());
        e
    }
}

fn pixels_element(local: &str, pixels: &[PreviewPixel]) -> XElement {
    let mut e = XElement::new(SHAPES_NS, local);
    e.children.reserve(pixels.len());
    for p in pixels {
        e.children.push(XNode::Element(p.to_xml()));
    }
    e
}

fn points_element(points: &[PreviewPoint]) -> XElement {
    let mut e = XElement::new(SHAPES_NS, "_points");
    for p in points {
        e.push(p.to_xml("PreviewPoint"));
    }
    e
}

fn read_moving_head_member(d: &mut MovingHeadData, e: &XElement, memo: &mut XmlMemo) -> Parse<bool> {
    macro_rules! int {
        ($field:ident, $name:literal) => {{
            d.$field = read_i32(e)?;
            memo.known_default($name, d.$field == 0);
        }};
    }
    macro_rules! dbl {
        ($field:ident, $name:literal) => {{
            d.$field = DoubleText::parse(&e.text())?;
            memo.known_default($name, d.$field.value == 0.0);
        }};
    }
    match e.local.as_str() {
        "BeamLength" => int!(beam_length, "BeamLength"),
        "BeamTransparency" => int!(beam_transparency, "BeamTransparency"),
        "BeamWidthMultiplier" => int!(beam_width_multiplier, "BeamWidthMultiplier"),
        "InvertPanDirection" => {
            d.invert_pan_direction = YesNo::parse(&e.text()).ok_or(Malformed)?;
            memo.known_default("InvertPanDirection", d.invert_pan_direction == YesNo::No);
        }
        "InvertTiltDirection" => {
            d.invert_tilt_direction = YesNo::parse(&e.text()).ok_or(Malformed)?;
            memo.known_default("InvertTiltDirection", d.invert_tilt_direction == YesNo::No);
        }
        "MaxColorWheelRotationSpeed" => dbl!(max_color_wheel_rotation_speed, "MaxColorWheelRotationSpeed"),
        "MaxPanTravelTime" => dbl!(max_pan_travel_time, "MaxPanTravelTime"),
        "MaxTiltTravelTime" => dbl!(max_tilt_travel_time, "MaxTiltTravelTime"),
        "MaximumStrobeDuration" => int!(maximum_strobe_duration, "MaximumStrobeDuration"),
        "MinColorWheelRotationSpeed" => dbl!(min_color_wheel_rotation_speed, "MinColorWheelRotationSpeed"),
        "MountingPosition" => {
            d.mounting_position = MountingPosition::parse(&e.text()).ok_or(Malformed)?;
            memo.known_default("MountingPosition", d.mounting_position == MountingPosition::Bottom);
        }
        "NodeId" => {
            d.node_id = Uuid::parse_str(e.text().trim()).map_err(|_| Malformed)?;
            memo.known_default("NodeId", d.node_id.is_nil());
        }
        "PanStartPosition" => int!(pan_start_position, "PanStartPosition"),
        "PanStopPosition" => int!(pan_stop_position, "PanStopPosition"),
        "ShowLegend" => {
            d.show_legend = read_bool(e)?;
            memo.known_default("ShowLegend", !d.show_legend);
        }
        "StrobeRateMaximum" => int!(strobe_rate_maximum, "StrobeRateMaximum"),
        "StrobeRateMinimum" => int!(strobe_rate_minimum, "StrobeRateMinimum"),
        "TiltStartPosition" => int!(tilt_start_position, "TiltStartPosition"),
        "TiltStopPosition" => int!(tilt_stop_position, "TiltStopPosition"),
        "ZoomNarrowToWide" => {
            d.zoom_narrow_to_wide = read_bool(e)?;
            memo.known_default("ZoomNarrowToWide", !d.zoom_narrow_to_wide);
        }
        _ => return Ok(false),
    }
    Ok(true)
}

fn moving_head_members(d: &MovingHeadData, m: &mut Vec<Member>) {
    let ns = SHAPES_NS;
    let int = |m: &mut Vec<Member>, name: &'static str, v: i32| m.push(Member::optional(name, el_i32(ns, name, v), v != 0));
    let dbl = |m: &mut Vec<Member>, name: &'static str, v: &DoubleText| m.push(Member::optional(name, el_text(ns, name, v.text()), v.value != 0.0));
    int(m, "BeamLength", d.beam_length);
    int(m, "BeamTransparency", d.beam_transparency);
    int(m, "BeamWidthMultiplier", d.beam_width_multiplier);
    m.push(Member::optional("InvertPanDirection", el_text(ns, "InvertPanDirection", d.invert_pan_direction.name()), d.invert_pan_direction != YesNo::No));
    m.push(Member::optional("InvertTiltDirection", el_text(ns, "InvertTiltDirection", d.invert_tilt_direction.name()), d.invert_tilt_direction != YesNo::No));
    dbl(m, "MaxColorWheelRotationSpeed", &d.max_color_wheel_rotation_speed);
    dbl(m, "MaxPanTravelTime", &d.max_pan_travel_time);
    dbl(m, "MaxTiltTravelTime", &d.max_tilt_travel_time);
    int(m, "MaximumStrobeDuration", d.maximum_strobe_duration);
    dbl(m, "MinColorWheelRotationSpeed", &d.min_color_wheel_rotation_speed);
    m.push(Member::optional("MountingPosition", el_text(ns, "MountingPosition", d.mounting_position.name()), d.mounting_position != MountingPosition::Bottom));
    m.push(Member::optional("NodeId", el_text(ns, "NodeId", d.node_id.hyphenated().to_string()), !d.node_id.is_nil()));
    int(m, "PanStartPosition", d.pan_start_position);
    int(m, "PanStopPosition", d.pan_stop_position);
    m.push(Member::optional("ShowLegend", el_bool(ns, "ShowLegend", d.show_legend), d.show_legend));
    int(m, "StrobeRateMaximum", d.strobe_rate_maximum);
    int(m, "StrobeRateMinimum", d.strobe_rate_minimum);
    int(m, "TiltStartPosition", d.tilt_start_position);
    int(m, "TiltStopPosition", d.tilt_stop_position);
    m.push(Member::optional("ZoomNarrowToWide", el_bool(ns, "ZoomNarrowToWide", d.zoom_narrow_to_wide), d.zoom_narrow_to_wide));
}
