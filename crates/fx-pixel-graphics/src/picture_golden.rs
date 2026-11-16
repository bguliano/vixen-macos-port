//! Original Picture sampling/movement on supplied pixels; no decode, resize or GDI oracle claim.
use super::*;
use flate2::read::GzDecoder;
use serde_json::Value;
use std::{fs::File, sync::atomic::AtomicBool};
use vixen_effects::testing;

struct Probe {
    effect: Picture,
    frames: Vec<Vec<u32>>,
    coords: Vec<(i32, i32)>,
}
impl PixelEffect for Probe {
    fn render_effect_by_location(
        &mut self,
        frames: i32,
        info: &PixelInfo,
        buf: &mut PixelLocationFrameBuffer,
    ) {
        self.effect.render_effect_by_location(frames, info, buf);
        for frame in 0..frames {
            buf.current_frame = frame;
            self.frames.push(
                self.coords
                    .iter()
                    .map(|&(x, y)| buf.get_color_at(x, y).to_argb_u32())
                    .collect(),
            );
        }
    }
    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        self.effect.render_effect(frame, info, buf);
        let mut pixels = Vec::new();
        for x in 0..info.buffer_wi() {
            for y in 0..info.buffer_ht() {
                pixels.push(buf.get_color_at(x, y).to_argb_u32());
            }
        }
        self.frames.push(pixels);
    }
}
#[test]
fn original_picture_movement_and_sampling_of_supplied_pixels() {
    let path = std::env::var_os("PICTURE_GOLDEN")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/golden/picture-sampling.json.gz")
        });
    let fixture: Value =
        serde_json::from_reader(GzDecoder::new(File::open(path).unwrap())).unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    let mut pixels_compared = 0;
    let mut mismatches = 0;
    for case in cases {
        let integer = |key: &str| case[key].as_i64().unwrap() as i32;
        let boolean = |key: &str| case[key].as_bool().unwrap();
        let horizontal = boolean("horizontal");
        let (w, h) = (integer("width"), integer("height"));
        let (strings, lights) = if horizontal { (h, w) } else { (w, h) };
        let mut tree = ElementTree::new();
        let target = testing::grid(&mut tree, "Sampling", strings as usize, lights as usize);
        let targets = [target];
        let locations = boolean("locations");
        let coords: Vec<(i32, i32)> = case["coords"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| (v[0].as_i64().unwrap() as i32, v[1].as_i64().unwrap() as i32))
            .collect();
        if locations {
            use vixen_core::element::{property_ids, PropertyData, PropertyInstance};
            for (leaf, &(x, y)) in tree.distinct_leaves(target).into_iter().zip(&coords) {
                tree.node_mut(leaf).properties.push(PropertyInstance {
                    type_id: property_ids::LOCATION,
                    instance_id: Uuid::new_v4(),
                    data: PropertyData::Location { x, y, z: 0 },
                });
            }
        }
        let cancel = AtomicBool::new(false);
        let ctx = RenderCtx { video_cache: None,
            tree: &tree,
            targets: &targets,
            start_time: TimeSpan::ZERO,
            time_span: TimeSpan::from_ms_i(integer("ms") as i64),
            frame_ms: integer("frameMs"),
            marks: &[],
            audio: None,
            module_data_root: None,
            cancel: &cancel,
        };
        let mut p = DESCRIPTOR.new_params();
        p.set(
            "Type",
            ParamValue::Enum(case["type"].as_str().unwrap().into()),
        );
        p.set(
            "Orientation",
            ParamValue::Enum(if horizontal { "Horizontal" } else { "Vertical" }.into()),
        );
        p.set(
            "ColorEffect",
            ParamValue::Enum(
                if boolean("custom") {
                    "CustomColor"
                } else {
                    "None"
                }
                .into(),
            ),
        );
        p.set(
            "TargetPositioning",
            ParamValue::Enum(if locations { "Locations" } else { "Strings" }.into()),
        );
        p.set("CenterStop", ParamValue::Bool(boolean("stop")));
        for (name, key) in [
            ("Direction", "direction"),
            ("Speed", "speed"),
            ("MovementRate", "rate"),
        ] {
            p.set(name, ParamValue::Int(integer(key)));
        }
        for (name, key) in [
            ("LevelCurve", "level"),
            ("IncreaseBrightnessCurve", "brightness"),
            ("XOffsetCurve", "xoffset"),
            ("YOffsetCurve", "yoffset"),
        ] {
            let value = integer(key) as f64;
            p.set(
                name,
                ParamValue::Curve(Curve::from_points(vec![(0.0, value), (100.0, value)])),
            );
        }
        let mut effect = from_params(&ctx, &p);
        let (iw, ih) = (integer("iw"), integer("ih"));
        let mut image = Bitmap::new(iw, ih);
        for y in 0..ih {
            for x in 0..iw {
                image.set_pixel(
                    x,
                    y,
                    Color::argb(
                        255,
                        (x * 71 + y * 29) as u8,
                        (x * 13 + y * 53) as u8,
                        (x * 31 + y * 19) as u8,
                    ),
                );
            }
        }
        effect.fp = Some(image);
        let mut probe = Probe {
            effect,
            frames: Vec::new(),
            coords,
        };
        render_pixel(
            &ctx,
            StringOrientation::from_name(p.enum_name("Orientation")),
            if locations {
                TargetPositioning::Locations
            } else {
                TargetPositioning::Strings
            },
            &mut probe,
        );
        let expected = case["frames"].as_array().unwrap();
        assert_eq!(expected.len(), probe.frames.len());
        for (frame, (want, got)) in expected.iter().zip(&probe.frames).enumerate() {
            let want = want.as_array().unwrap();
            assert_eq!(want.len(), got.len());
            for (pixel, (want, got)) in want.iter().zip(got).enumerate() {
                pixels_compared += 1;
                if want.as_u64().unwrap() as u32 != *got {
                    mismatches += 1;
                    if mismatches <= 20 {
                        eprintln!("{} {w}x{h}/{iw}x{ih} horizontal={horizontal} locations={locations} custom={} stop={} dir={} frame={frame} pixel={pixel}: C#={:08x} Rust={got:08x}",case["type"],case["custom"],case["stop"],case["direction"],want.as_u64().unwrap());
                    }
                }
            }
        }
    }
    println!(
        "{} cases / {pixels_compared} pixel values / {mismatches} mismatches",
        cases.len()
    );
    assert_eq!(mismatches, 0);
}
