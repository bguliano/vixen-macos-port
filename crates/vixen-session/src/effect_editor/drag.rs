//! What a value drag carries (F67, F69): a curve, a gradient or a colour, as text for a private pasteboard type. The drag can't cross apps, so the format
//! only has to last while the app runs, and it keeps every field of the value.

use serde_json::{json, Value};
use vixen_core::gradient::{AlphaPoint, ColorPoint};
use vixen_core::color::Xyz;
use vixen_core::{Color, ColorGradient, Curve};

#[derive(Clone, Debug, PartialEq)]
pub enum DragPayload {
    Curve(Curve),
    Gradient(ColorGradient),
    Color(Color),
}

impl DragPayload {
    pub fn encode(&self) -> String {
        let value = match self {
            DragPayload::Curve(c) => json!({
                "curve": {
                    "points": c.points.iter().map(|p| [p.0, p.1]).collect::<Vec<_>>(),
                    "link": c.library_reference_name,
                    "current": c.is_current_library_curve,
                }
            }),
            DragPayload::Gradient(g) => json!({
                "gradient": {
                    "colors": g.colors().iter().map(|p| [p.color.x, p.color.y, p.color.z, p.position, p.focus]).collect::<Vec<_>>(),
                    "alphas": g.alphas().iter().map(|p| [p.alpha, p.position, p.focus]).collect::<Vec<_>>(),
                    "gamma": g.gammacorrected,
                    "title": g.title,
                    "link": g.library_reference_name,
                    "current": g.is_current_library_gradient,
                }
            }),
            DragPayload::Color(c) => json!({ "color": c.to_argb_u32() }),
        };
        value.to_string()
    }

    pub fn decode(text: &str) -> Option<DragPayload> {
        let value: Value = serde_json::from_str(text).ok()?;
        if let Some(c) = value.get("color") {
            return Some(DragPayload::Color(Color::from_argb_u32(c.as_u64()? as u32)));
        }
        if let Some(c) = value.get("curve") {
            let mut points = Vec::new();
            for p in c.get("points")?.as_array()? {
                let p = p.as_array()?;
                points.push((p.first()?.as_f64()?, p.get(1)?.as_f64()?));
            }
            let mut curve = Curve::from_points(points);
            curve.library_reference_name = c.get("link")?.as_str()?.to_string();
            curve.is_current_library_curve = c.get("current")?.as_bool()?;
            return Some(DragPayload::Curve(curve));
        }
        let g = value.get("gradient")?;
        let mut colors = Vec::new();
        for p in g.get("colors")?.as_array()? {
            let n = numbers(p)?;
            colors.push(ColorPoint::from_xyz(Xyz::new(*n.first()?, *n.get(1)?, *n.get(2)?), *n.get(4)?, *n.get(3)?));
        }
        let mut alphas = Vec::new();
        for p in g.get("alphas")?.as_array()? {
            let n = numbers(p)?;
            alphas.push(AlphaPoint { alpha: *n.first()?, position: *n.get(1)?, focus: *n.get(2)? });
        }
        let mut gradient = ColorGradient::from_parts(colors, alphas);
        gradient.gammacorrected = g.get("gamma")?.as_bool()?;
        gradient.title = g.get("title")?.as_str().map(str::to_string);
        gradient.library_reference_name = g.get("link")?.as_str()?.to_string();
        gradient.is_current_library_gradient = g.get("current")?.as_bool()?;
        Some(DragPayload::Gradient(gradient))
    }
}

fn numbers(v: &Value) -> Option<Vec<f64>> {
    v.as_array()?.iter().map(Value::as_f64).collect()
}

/// `LibraryDragPayloadFactory.Create`: a copy, marked not current, carrying the entry's name only when linking was asked for (F69).
pub fn library_curve_payload(curve: &Curve, name: &str, link: bool) -> DragPayload {
    let mut copy = curve.clone();
    copy.is_current_library_curve = false;
    // GUARD, F107: upstream throws on a blank name with the link on. Such an entry drags as an unlinked copy.
    copy.library_reference_name = if link && !name.trim().is_empty() { name.to_string() } else { String::new() };
    DragPayload::Curve(copy)
}

/// `LibraryDragPayloadFactory.Create`, for a gradient.
pub fn library_gradient_payload(gradient: &ColorGradient, name: &str, link: bool) -> DragPayload {
    let mut copy = gradient.clone();
    copy.is_current_library_gradient = false;
    copy.library_reference_name = if link && !name.trim().is_empty() { name.to_string() } else { String::new() };
    DragPayload::Gradient(copy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_round_trip() {
        let mut c = Curve::from_points(vec![(0.0, 12.5), (100.0, 33.0)]);
        c.library_reference_name = "L".into();
        let p = DragPayload::Curve(c);
        assert_eq!(DragPayload::decode(&p.encode()), Some(p));

        let mut g = ColorGradient::from_parts(
            vec![ColorPoint::new(Color::rgb(12, 34, 56), 0.25), ColorPoint::new(Color::WHITE, 1.0)],
            vec![AlphaPoint::new(255, 0.0), AlphaPoint::new(128, 1.0)],
        );
        g.is_current_library_gradient = true;
        g.title = Some("T".into());
        let p = DragPayload::Gradient(g);
        assert_eq!(DragPayload::decode(&p.encode()), Some(p));

        let p = DragPayload::Color(Color::argb(10, 20, 30, 40));
        assert_eq!(DragPayload::decode(&p.encode()), Some(p));
        assert_eq!(DragPayload::decode("{nope"), None);
    }

    #[test]
    fn library_payloads() {
        let mut c = Curve::flat(10.0);
        c.is_current_library_curve = true;
        let DragPayload::Curve(linked) = library_curve_payload(&c, "Ten", true) else { panic!() };
        assert_eq!(linked.library_reference_name, "Ten");
        assert!(!linked.is_current_library_curve);
        let DragPayload::Curve(plain) = library_curve_payload(&c, "Ten", false) else { panic!() };
        assert_eq!(plain.library_reference_name, "");
        let DragPayload::Curve(blank) = library_curve_payload(&c, "   ", true) else { panic!() };
        assert_eq!(blank.library_reference_name, "");
    }
}
