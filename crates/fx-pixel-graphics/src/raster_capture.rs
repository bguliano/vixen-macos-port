//! Explicit, ignored diagnostic generator. Calls production Picture helpers;
//! no replacement drawing algorithm or normal-build API is introduced.
use serde_json::{json, Value};
use std::{collections::HashSet, fs, io::Write, path::Path};
use vixen_core::Color;
use vixen_effects::gfx::{Bitmap, Graphics};

fn write_new(path: &Path, bytes: &[u8]) {
    let mut file = fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}

fn pixels(bitmap: &Bitmap) -> Vec<u8> {
    let mut out = Vec::new();
    for y in 0..bitmap.height() {
        for x in 0..bitmap.width() {
            let c = bitmap.get_pixel(x, y);
            out.extend_from_slice(&[c.a, c.r, c.g, c.b]);
        }
    }
    out
}

fn rgba_as_argb(bitmap: &Bitmap) -> Vec<u8> {
    bitmap.to_rgba().chunks_exact(4).flat_map(|p| [p[3], p[0], p[1], p[2]]).collect()
}

fn coords(value: &Value, size: usize) -> Vec<f32> {
    let values = value.as_array().unwrap();
    assert_eq!(values.len(), size);
    values.iter().map(|v| {
        let n = v.as_f64().unwrap();
        assert!(n.is_finite() && (-64.0..=64.0).contains(&n));
        n as f32
    }).collect()
}

fn primitive(case: &Value, w: i32, h: i32) -> Bitmap {
    let mut g = Graphics::new(w, h);
    match case["operation"].as_str().unwrap() {
        "line" => {
            let c = coords(&case["line"], 4);
            g.draw_line(Color::WHITE, 1.0, c[0], c[1], c[2], c[3]);
        }
        "fill-rectangle" | "rectangle" | "ellipse" => {
            let c = coords(&case["rect"], 4);
            match case["operation"].as_str().unwrap() {
                "fill-rectangle" => g.fill_rectangle(Color::WHITE, c[0], c[1], c[2], c[3]),
                "rectangle" => g.draw_rectangle(Color::WHITE, 1.0, c[0], c[1], c[2], c[3]),
                _ => g.draw_ellipse(Color::WHITE, 1.0, c[0], c[1], c[2], c[3]),
            }
        }
        "polygon" => {
            let c = coords(&case["points"], 10);
            let points: Vec<_> = c.chunks_exact(2).map(|p| (p[0], p[1])).collect();
            match case["fillMode"].as_str().unwrap() {
                "Alternate" => g.fill_polygon(Color::WHITE, &points),
                "Winding" => g.fill_polygon_winding(Color::WHITE, &points),
                _ => panic!("unknown polygon fill mode"),
            }
        }
        _ => panic!("unknown primitive"),
    }
    g.into_bitmap()
}

#[test]
#[ignore = "diagnostic generator: explicit VIXEN_RASTER_CASES and fresh VIXEN_RASTER_OUT required"]
fn capture_picture_cases() {
    let input = std::env::var_os("VIXEN_RASTER_CASES").expect("case file required");
    let output = std::env::var_os("VIXEN_RASTER_OUT").expect("fresh output required");
    let (input, output) = (Path::new(&input), Path::new(&output));
    assert!(input.is_absolute() && output.is_absolute() && !output.exists());
    assert!(fs::metadata(input).unwrap().len() <= 65_536);
    let cases: Value = serde_json::from_slice(&fs::read(input).unwrap()).unwrap();
    let schema = cases["schema"].as_u64().unwrap();
    assert!(schema == 1 || schema == 2);
    assert_eq!(cases["byteOrder"], "ARGB8-row-major-top-down");
    let rows = cases["cases"].as_array().unwrap();
    assert_eq!(rows.len(), if schema == 1 { 36 } else { 74 });
    let mut ids = HashSet::new();
    fs::create_dir(output).unwrap();
    fs::create_dir(output.join("cases")).unwrap();
    let mut reports = Vec::new();
    for case in rows {
        let id = case["id"].as_str().unwrap();
        assert!(!id.is_empty() && id.len() < 100 && id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-'));
        assert!(ids.insert(id));
        let w = case["width"].as_i64().unwrap() as i32;
        let h = case["height"].as_i64().unwrap() as i32;
        assert!((1..=64).contains(&w) && (1..=64).contains(&h));
        let alpha = case["alpha"].as_bool().unwrap();
        let family = case["family"].as_str().unwrap_or("picture");
        let mut rgba = Vec::new();
        let mut requested = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let a = if alpha { [0, 1, 127, 128, 254, 255][((y * w + x) % 6) as usize] } else { 255 };
                let (r, g, b) = (((71*x+31*y+17)%256) as u8, ((29*x+97*y+43)%256) as u8, ((113*x+19*y+89)%256) as u8);
                if family == "picture" {
                    rgba.extend_from_slice(&[r, g, b, a]);
                    requested.extend_from_slice(&[a, r, g, b]);
                } else {
                    rgba.extend_from_slice(&[0, 0, 0, 0]);
                    requested.extend_from_slice(&[0, 0, 0, 0]);
                }
            }
        }
        let bitmap = Bitmap::from_rgba(w, h, &rgba);
        let before = pixels(&bitmap);
        assert_eq!(before, rgba_as_argb(&bitmap));
        let rendered = if family == "primitive" {
            primitive(case, w, h)
        } else if family == "countdown" {
            let r = coords(&case["rect"], 4);
            assert!(r.iter().all(|v| v.fract() == 0.0));
            let rect = (r[0] as i32, r[1] as i32, r[2] as i32, r[3] as i32);
            let spinner = match case["operation"].as_str().unwrap() {
                "cross-hash" => None,
                "spinner" => Some((case["lineLength"].as_i64().unwrap() as i32, case["angle"].as_i64().unwrap() as i32)),
                _ => panic!("unknown CountDown operation"),
            };
            crate::count_down::raster_diagnostic(w, h, rect, spinner)
        } else { assert_eq!(family, "picture"); match case["operation"].as_str().unwrap() {
            "identity" => bitmap.clone(),
            "fit" => {
                let mw = case["maxWidth"].as_i64().unwrap();
                let mh = case["maxHeight"].as_i64().unwrap();
                assert!((1..=64).contains(&mw) && (1..=64).contains(&mh));
                crate::picture::scale_picture_image(&bitmap, mw as i32, mh as i32)
            }
            "scale" => {
                let scale = case["scale"].as_f64().unwrap();
                assert!(scale > 0.0 && scale <= 1.5);
                crate::picture::scale_image(&bitmap, scale)
            }
            "grayscale" => crate::picture::to_grayscale(&bitmap),
            other => panic!("unknown operation {other}"),
        }};
        assert!((1..=64).contains(&rendered.width()) && (1..=64).contains(&rendered.height()));
        let after = pixels(&rendered);
        assert_eq!(before, pixels(&bitmap), "input mutated");
        assert_eq!(after, rgba_as_argb(&rendered));
        let dir = output.join("cases").join(id);
        fs::create_dir(&dir).unwrap();
        for (name, bytes) in [
            ("input-requested.argb", requested), ("input-getpixel.argb", before.clone()),
            ("input-to-rgba.argb", rgba_as_argb(&bitmap)), ("output-getpixel.argb", after),
            ("output-to-rgba.argb", rgba_as_argb(&rendered)),
        ] { write_new(&dir.join(name), &bytes); }
        reports.push(json!({"schema":schema,"status":"rendered-unsealed","id":id,"input":case,
            "outputRepresentation":if family == "countdown" { "CountDown.Canvas-post-mask" } else { "Bitmap-straight-readback" },
            "byteOrder":"ARGB8-row-major-top-down","outputWidth":rendered.width(),"outputHeight":rendered.height(),
            "inputUnchanged":true,"inputReadbacksEqual":true,"outputReadbacksEqual":true,
            "inputPixelFormat":"tiny-skia-premultiplied-RGBA8; straight-ARGB8-readback",
            "outputPixelFormat":"tiny-skia-premultiplied-RGBA8; straight-ARGB8-readback"}));
    }
    write_new(&output.join("render.json"), &serde_json::to_vec_pretty(&json!({"schema":schema,"cases":reports})).unwrap());
}

#[path = "nominal_windows.rs"]
mod nominal_windows;
