//! Non-raster contract from unchanged C# CountDown helpers; see tools/golden/countdown.
use super::*;
use flate2::read::GzDecoder;
use serde_json::Value;
use std::{fs::File, sync::atomic::AtomicBool};
use vixen_effects::testing;

struct Probe {
    effect: CountDown,
    frames: Vec<Value>,
}
impl PixelEffect for Probe {
    fn setup_render(&mut self, info: &PixelInfo) {
        self.effect.setup_render(info);
    }
    fn render_effect(&mut self, frame: i32, info: &PixelInfo, _: &mut PixelFrameBuffer) {
        self.effect.set_fade_position_level(frame, info);
        self.frames.push(serde_json::json!({
            "frame": frame, "text": self.effect.text,
            "position": self.effect.direction_position.to_string(), "fade": self.effect.fade.to_string(),
            "iteration": self.effect.iteration, "previous": self.effect.previous_number,
            "direction": self.effect.direction, "size": self.effect.size_adjust,
            "angle": self.effect.radial_line_angle(frame, info),
        }));
    }
}

#[test]
fn original_countdown_timing_fades_directions_and_spinner_angles() {
    let path = std::env::var_os("COUNTDOWN_GOLDEN")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/golden/countdown-math.json.gz")
        });
    let fixture: Value =
        serde_json::from_reader(GzDecoder::new(File::open(path).unwrap())).unwrap();
    let mut tree = ElementTree::new();
    let matrix = testing::grid(&mut tree, "Math probe", 1, 1);
    let targets = [matrix];
    let cancel = AtomicBool::new(false);
    let mut mismatches = 0;
    let mut frames_compared = 0;
    let cases = fixture["cases"].as_array().unwrap();
    for case in cases {
        let mut p = DESCRIPTOR.new_params();
        for (name, key) in [
            ("CountDownType", "type"),
            ("CountDownFade", "fade"),
            ("Direction", "direction"),
        ] {
            p.set(name, ParamValue::Enum(case[key].as_str().unwrap().into()));
        }
        p.set(
            "TimeFormat",
            ParamValue::Enum(
                if case["minutes"].as_bool().unwrap() {
                    "Minutes"
                } else {
                    "Seconds"
                }
                .into(),
            ),
        );
        p.set(
            "CountDownTime",
            ParamValue::Str(case["count"].as_str().unwrap().into()),
        );
        p.set(
            "Speed",
            ParamValue::Int(case["speed"].as_i64().unwrap() as i32),
        );
        p.set(
            "PerIteration",
            ParamValue::Bool(case["per"].as_bool().unwrap()),
        );
        let ctx = RenderCtx { video_cache: None,
            tree: &tree,
            targets: &targets,
            start_time: TimeSpan::ZERO,
            time_span: TimeSpan::from_ms_i(case["ms"].as_i64().unwrap()),
            frame_ms: case["frameMs"].as_i64().unwrap() as i32,
            marks: &[],
            audio: None,
            module_data_root: None,
            cancel: &cancel,
        };
        fastrand::seed(case["seed"].as_u64().unwrap());
        let mut probe = Probe {
            effect: from_params(&p),
            frames: Vec::new(),
        };
        render_pixel_effect(&ctx, &p, &mut probe);
        let expected = case["frames"].as_array().unwrap();
        assert_eq!(
            expected.len(),
            probe.frames.len(),
            "{} frame count",
            case["label"]
        );
        for (expected, actual) in expected.iter().zip(&probe.frames) {
            frames_compared += 1;
            for key in [
                "frame",
                "text",
                "iteration",
                "previous",
                "direction",
                "size",
                "angle",
                "position",
                "fade",
            ] {
                let matches = if ["position", "fade"].contains(&key) {
                    let a: f64 = actual[key].as_str().unwrap().parse().unwrap();
                    let e: f64 = expected[key].as_str().unwrap().parse().unwrap();
                    (a - e).abs() < 1e-12 || a == e || (a.is_nan() && e.is_nan())
                } else {
                    expected[key] == actual[key]
                };
                if !matches {
                    mismatches += 1;
                    if mismatches <= 20 {
                        eprintln!(
                            "{} frame={} {key}: C#={} Rust={}",
                            case["label"], actual["frame"], expected[key], actual[key]
                        );
                    }
                }
            }
        }
    }
    println!(
        "{} cases / {frames_compared} frames / {mismatches} mismatches",
        cases.len()
    );
    assert_eq!(mismatches, 0);
}
