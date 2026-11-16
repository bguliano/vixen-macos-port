//! Port of Shapes/PreviewPoint.cs and PreviewDoublePoint.cs.

use crate::xml_util::{el_i32, read_i32, Malformed, Parse, SHAPES_NS};
use vixen_xml::XElement;

/// `PreviewPoint.PointTypes`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum PointType {
    #[default]
    None,
    Size,
    SkewNS,
    SkewWE,
    RotateHandle,
    SizeTopLeft,
    SizeTopRight,
    SizeBottomLeft,
    SizeBottomRight,
    RotationAxis,
}

/// `PreviewPoint`. Only X and Y are data members. The type is rebuilt by each shape on load.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct PreviewPoint {
    pub x: i32,
    pub y: i32,
    pub point_type: PointType,
}

impl PreviewPoint {
    #[inline]
    pub const fn new(x: i32, y: i32) -> PreviewPoint {
        PreviewPoint { x, y, point_type: PointType::None }
    }

    #[inline]
    pub const fn typed(x: i32, y: i32, point_type: PointType) -> PreviewPoint {
        PreviewPoint { x, y, point_type }
    }

    #[inline]
    pub fn with_type(self, point_type: PointType) -> PreviewPoint {
        PreviewPoint { point_type, ..self }
    }

    #[inline]
    pub fn translate(&mut self, dx: i32, dy: i32) {
        self.x = self.x.wrapping_add(dx);
        self.y = self.y.wrapping_add(dy);
    }

    pub(crate) fn from_xml(e: &XElement) -> Parse<PreviewPoint> {
        if e.nil {
            return Err(Malformed);
        }
        let mut p = PreviewPoint::default();
        for c in e.elements() {
            match c.local.as_str() {
                "X" => p.x = read_i32(c)?,
                "Y" => p.y = read_i32(c)?,
                _ => return Err(Malformed),
            }
        }
        Ok(p)
    }

    pub(crate) fn to_xml(self, local: &str) -> XElement {
        let mut e = XElement::new(SHAPES_NS, local);
        e.push(el_i32(SHAPES_NS, "X", self.x));
        e.push(el_i32(SHAPES_NS, "Y", self.y));
        e
    }
}

/// `PreviewDoublePoint`, used by the Custom Prop's drag points.
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct PreviewDoublePoint {
    pub x: f64,
    pub y: f64,
    pub point_type: PointType,
}

impl PreviewDoublePoint {
    pub const fn typed(x: f64, y: f64, point_type: PointType) -> PreviewDoublePoint {
        PreviewDoublePoint { x, y, point_type }
    }
}
