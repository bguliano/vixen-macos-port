//! Port of the data half of Shapes/PreviewPixel.cs. Drawing and intent handling live elsewhere.

use crate::point::PreviewPoint;
use crate::xml_util::{el_bool, el_i32, el_text, fmt_double, parse_double, read_bool, read_i32, Malformed, Parse, SHAPES_NS, WINDOWS_NS};
use uuid::Uuid;
use vixen_xml::XElement;

/// A pixel the plain writer could not reproduce exactly, kept so an untouched one goes back unchanged.
#[derive(Clone, Debug)]
pub struct PixelMemo {
    raw: XElement,
    node_id: Uuid,
    high_precision: bool,
    location: (f64, f64),
    size: i32,
}

/// `PreviewPixel`. In the file only `NodeId`, and for high precision pixels `IsHighPrecision`, `Location`
/// and `PixelSize`, are stored. X, Y and Z are computed by the shape's layout and are never saved.
#[derive(Clone, Debug, Default)]
pub struct PreviewPixel {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub size: i32,
    /// `NodeId`. Nil is `Guid.Empty`, an unlinked pixel.
    pub node_id: Uuid,
    pub high_precision: bool,
    /// `Location`, meaningful when `high_precision` is set.
    pub location: (f64, f64),
    memo: Option<Box<PixelMemo>>,
}

impl PreviewPixel {
    /// `new PreviewPixel(x, y, z, pixelSize)`.
    #[inline]
    pub fn new(x: i32, y: i32, z: i32, size: i32) -> PreviewPixel {
        PreviewPixel { x, y, z, size, ..PreviewPixel::default() }
    }

    /// `new PreviewPixel(location, pixelSize)`.
    pub fn new_high_precision(location: (f64, f64), size: i32) -> PreviewPixel {
        PreviewPixel { high_precision: true, location, size, ..PreviewPixel::default() }
    }

    /// `pixel.Point`.
    #[inline]
    pub fn point(&self) -> PreviewPoint {
        if self.high_precision {
            PreviewPoint::new(self.location.0 as i32, self.location.1 as i32)
        } else {
            PreviewPoint::new(self.x, self.y)
        }
    }

    /// Where the light is drawn. High precision pixels use `Location` as the OpenGL preview does.
    #[inline]
    pub fn draw_position(&self) -> (f32, f32) {
        if self.high_precision {
            (self.location.0 as f32, self.location.1 as f32)
        } else {
            (self.x as f32, self.y as f32)
        }
    }

    pub(crate) fn from_xml(e: &XElement) -> Parse<PreviewPixel> {
        if e.nil || e.local != "PreviewPixel" {
            return Err(Malformed);
        }
        let mut p = PreviewPixel::default();
        let mut canonical = e.attrs.is_empty() && e.xsi_type.is_none();
        let mut last_rank = 0;
        let mut size_present = false;
        for c in e.elements() {
            let rank = match c.local.as_str() {
                "IsHighPrecision" => {
                    p.high_precision = read_bool(c)?;
                    canonical &= p.high_precision;
                    1
                }
                "Location" => {
                    let (x, xt) = location_part(c, "_x")?;
                    let (y, yt) = location_part(c, "_y")?;
                    canonical &= c.elements().count() == 2 && fmt_double(x) == xt && fmt_double(y) == yt;
                    p.location = (x, y);
                    canonical &= x != 0.0 || y != 0.0;
                    2
                }
                "NodeId" => {
                    let t = c.text();
                    let t = t.trim();
                    p.node_id = Uuid::parse_str(t).map_err(|_| Malformed)?;
                    canonical &= !p.node_id.is_nil() && t.len() == 36 && !t.bytes().any(|b| b.is_ascii_uppercase());
                    3
                }
                "PixelSize" => {
                    p.size = read_i32(c)?;
                    size_present = true;
                    canonical &= p.size != 0;
                    4
                }
                _ => {
                    canonical = false;
                    continue;
                }
            };
            canonical &= rank > last_rank;
            last_rank = rank;
        }
        // Vixen zeroes PixelSize on a normal pixel while saving, so one in the file is foreign.
        canonical &= !size_present || p.high_precision;
        canonical &= p.high_precision || p.location == (0.0, 0.0);
        if !canonical {
            p.memo = Some(Box::new(PixelMemo {
                raw: e.clone(),
                node_id: p.node_id,
                high_precision: p.high_precision,
                location: p.location,
                size: p.size,
            }));
        }
        Ok(p)
    }

    pub(crate) fn to_xml(&self) -> XElement {
        if let Some(m) = &self.memo {
            let same_location = self.location.0.to_bits() == m.location.0.to_bits() && self.location.1.to_bits() == m.location.1.to_bits();
            if m.node_id == self.node_id
                && m.high_precision == self.high_precision
                && same_location
                && (!self.high_precision || m.size == self.size)
            {
                return m.raw.clone();
            }
        }
        let mut e = XElement::new(SHAPES_NS, "PreviewPixel");
        if self.high_precision {
            e.push(el_bool(SHAPES_NS, "IsHighPrecision", true));
            if self.location != (0.0, 0.0) {
                let mut l = XElement::new(SHAPES_NS, "Location");
                l.push(el_text(WINDOWS_NS, "_x", fmt_double(self.location.0)));
                l.push(el_text(WINDOWS_NS, "_y", fmt_double(self.location.1)));
                e.push(l);
            }
        }
        if !self.node_id.is_nil() {
            e.push(el_text(SHAPES_NS, "NodeId", self.node_id.hyphenated().to_string()));
        }
        if self.high_precision && self.size != 0 {
            e.push(el_i32(SHAPES_NS, "PixelSize", self.size));
        }
        if let Some(m) = &self.memo {
            // A linked or moved pixel still keeps the members this port does not know.
            for c in m.raw.elements() {
                if !matches!(c.local.as_str(), "IsHighPrecision" | "Location" | "NodeId" | "PixelSize") {
                    e.push(c.clone());
                }
            }
        }
        e
    }
}

fn location_part(loc: &XElement, name: &str) -> Parse<(f64, String)> {
    let c = loc.child(name).ok_or(Malformed)?;
    let t = c.text();
    let t = t.trim().to_string();
    Ok((parse_double(&t).ok_or(Malformed)?, t))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> XElement {
        XElement::parse(s).unwrap()
    }

    const NS: &str = r#"xmlns="http://schemas.datacontract.org/2004/07/VixenModules.Preview.VixenPreview.Shapes""#;

    #[test]
    fn plain_pixel_round_trips_without_a_memo() {
        let e = parse(&format!(r#"<PreviewPixel {NS}><NodeId>1940775e-9694-44e5-9e5d-100a57177018</NodeId></PreviewPixel>"#));
        let p = PreviewPixel::from_xml(&e).unwrap();
        assert!(p.memo.is_none());
        assert_eq!(p.node_id.to_string(), "1940775e-9694-44e5-9e5d-100a57177018");
        assert!(e.semantic_eq(&p.to_xml()));
    }

    #[test]
    fn high_precision_pixel_round_trips() {
        let e = parse(&format!(
            r#"<PreviewPixel {NS}><IsHighPrecision>true</IsHighPrecision><Location xmlns:b="http://schemas.datacontract.org/2004/07/System.Windows"><b:_x>12.5</b:_x><b:_y>1E-05</b:_y></Location><PixelSize>4</PixelSize></PreviewPixel>"#
        ));
        let p = PreviewPixel::from_xml(&e).unwrap();
        assert!(p.memo.is_none());
        assert!(p.high_precision);
        assert_eq!(p.location, (12.5, 0.00001));
        assert_eq!(p.size, 4);
        assert!(e.semantic_eq(&p.to_xml()));
    }

    #[test]
    fn foreign_members_are_preserved_even_after_a_relink() {
        let e = parse(&format!(r#"<PreviewPixel {NS}><Future>7</Future><PixelSize>9</PixelSize></PreviewPixel>"#));
        let mut p = PreviewPixel::from_xml(&e).unwrap();
        assert!(e.semantic_eq(&p.to_xml()));
        p.node_id = Uuid::from_u128(5);
        let out = p.to_xml();
        assert!(out.child("Future").is_some());
        assert!(out.child("NodeId").is_some());
    }
}
