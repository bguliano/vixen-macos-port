//! Full-byte native Windows regressions for nominal strokes and CountDown.
//! The fixture records archive hashes and entries; `expectedARGBHex` preserves
//! every channel. No production renderer reads these values or case identifiers.
use super::{coords, pixels, primitive, rgba_as_argb};
use serde_json::Value;
use vixen_core::Color;
use vixen_effects::gfx::Graphics;

#[test]
fn nominal_strokes_match_all_native_argb_bytes() {
    let fixture: Value = serde_json::from_str(include_str!("../../../tests/golden/nominal-strokes-windows.json")).unwrap();
    assert_eq!(fixture["schema"], 1);
    assert_eq!(fixture["byteOrder"], "ARGB8-row-major-top-down");
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 73);
    let mut ids = std::collections::HashSet::new();
    for row in rows {
        let id = row["id"].as_str().unwrap();
        assert!(ids.insert(id));
        let case = &row["input"];
        assert_eq!(case["id"], id);
        let (w, h) = (case["width"].as_i64().unwrap() as i32, case["height"].as_i64().unwrap() as i32);
        assert!((1..=64).contains(&w) && (1..=64).contains(&h));
        let actual = match case["family"].as_str().unwrap() {
            "primitive" => primitive(case, w, h),
            "countdown" => {
                let r = coords(&case["rect"], 4);
                assert!(r.iter().all(|v| v.fract() == 0.0));
                let rect = (r[0] as i32, r[1] as i32, r[2] as i32, r[3] as i32);
                let spinner = match case["operation"].as_str().unwrap() {
                    "cross-hash" => None,
                    "spinner" => Some((case["lineLength"].as_i64().unwrap() as i32, case["angle"].as_i64().unwrap() as i32)),
                    _ => panic!("unknown CountDown operation"),
                };
                crate::count_down::raster_diagnostic(w, h, rect, spinner)
            }
            "native-flat-path" => {
                let bits = case["pointBits"].as_array().unwrap();
                let types = case["pointTypes"].as_array().unwrap();
                assert_eq!(bits.len(), types.len() * 2);
                assert!(types.last().unwrap().as_u64().unwrap() & 128 != 0);
                let points: Vec<_> = bits.chunks_exact(2).map(|p| {
                    (f32::from_bits(p[0].as_i64().unwrap() as u32), f32::from_bits(p[1].as_i64().unwrap() as u32))
                }).collect();
                assert!(points.iter().all(|&(x, y)| x.is_finite() && y.is_finite()));
                assert_eq!(points.first(), points.last());
                let mut g = Graphics::new(w, h);
                g.draw_lines(Color::WHITE, 1.0, &points);
                g.into_bitmap()
            }
            _ => panic!("unknown native case family"),
        };
        let hex = row["expectedARGBHex"].as_str().unwrap();
        assert_eq!(hex.len(), w as usize * h as usize * 8);
        let expected: Vec<_> = hex.as_bytes().chunks_exact(2).map(|p| {
            u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap()
        }).collect();
        assert_eq!((actual.width(), actual.height()), (w, h), "{id}: dimensions");
        let bytes = pixels(&actual);
        assert_eq!(bytes, expected, "{id}: every native ARGB byte");
        assert_eq!(bytes, rgba_as_argb(&actual), "{id}: alternate readback");
    }
}
