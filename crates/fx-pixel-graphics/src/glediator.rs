//! Port of VixenModules.Effect.Glediator: plays back a Glediator recorder file (raw RGB frames).

use crate::generated::glediator as gen;
use std::io::{Read, Seek, SeekFrom};
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    param_changed: Some(param_changed),
    targets_changed: Some(crate::orientation_targets_changed),
    param_changed_ctx: Some(crate::orientation_param_changed),
    ported: true,
    ..gen::BASE
};

/// `UpdateMovementTypeAttribute`.
fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    let speed = p.enum_name("MovementType") == "Speed";
    v.set("Speed", speed);
    v.set("Iterations", !speed);
}

/// The `Orientation` setter also sets `StringOrientation`, whose getter and setter both use the same data.
fn param_changed(name: &str, p: &mut Params) {
    match name {
        "Orientation" => {
            let v = p.enum_name("Orientation").to_string();
            p.set("StringOrientation", ParamValue::Enum(v));
        }
        "StringOrientation" => {
            let v = p.enum_name("StringOrientation").to_string();
            p.set("Orientation", ParamValue::Enum(v));
        }
        _ => {}
    }
}

struct Glediator {
    movement_iterations: bool,
    speed: i32,
    iterations: i32,
    file_name: String,
    level_curve: Curve,
    path: Option<std::path::PathBuf>,
    // State
    speed_acc: f64,
    position: f64,
    file: Option<std::fs::File>,
    file_len: u64,
    seq_num_channels: usize,
    frame_buffer: Vec<u8>,
    failed: bool,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let file_name = p.string("FileName").to_string();
    // The C# `StringOrientation` is backed by the `Orientation` data member.
    let orientation = StringOrientation::from_name(p.enum_name("Orientation"));
    let positioning = TargetPositioning::from_name(p.enum_name("TargetPositioning"));
    let path = match (ctx.module_data_root, file_name.is_empty()) {
        (Some(root), false) => Some(root.join("Glediator").join(&file_name)),
        _ => None,
    };
    let mut e = Glediator {
        movement_iterations: p.enum_name("MovementType") != "Speed",
        speed: p.int("Speed"),
        iterations: p.int("Iterations"),
        file_name,
        level_curve: p.curve("LevelCurve"),
        path,
        speed_acc: 1.0,
        position: 0.0,
        file: None,
        file_len: 0,
        seq_num_channels: 0,
        frame_buffer: Vec::new(),
        failed: false,
    };
    let output = render_pixel(ctx, orientation, positioning, &mut e);
    // PixelEffectBase only publishes its arrays after the complete render succeeds.
    // A recording shorter than one frame throws in C#, leaving no intents (not black layers).
    if e.failed { EffectIntents::new() } else { output }
}

impl PixelEffect for Glediator {
    fn setup_render(&mut self, info: &PixelInfo) {
        if self.file_name.is_empty() {
            return;
        }
        self.file = None;
        if let Some(path) = &self.path {
            if let Ok(f) = std::fs::File::open(path) {
                self.file_len = f.metadata().map(|m| m.len()).unwrap_or(0);
                self.file = Some(f);
            }
        }
        self.seq_num_channels = (info.buffer_wi().max(0) as usize) * 3 * (info.buffer_ht().max(0) as usize);
        self.frame_buffer = vec![0u8; self.seq_num_channels];
    }

    fn cleanup_render(&mut self) {
        self.frame_buffer = Vec::new();
        self.file = None;
    }

    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        let Some(file) = self.file.as_mut() else { return };
        if self.seq_num_channels == 0 {
            return;
        }
        let level = self.level_curve.get_value(info.interval_position(frame) * 100.0) / 100.0;
        let seq_num_periods = (self.file_len / self.seq_num_channels as u64) as i64;

        if frame == 0 {
            self.position = if self.movement_iterations {
                seq_num_periods as f64 / (info.time_span.total_ms() / info.frame_time as f64) * self.iterations as f64
            } else {
                self.speed as f64
            };
        }

        self.speed_acc += self.position;
        // Match the caught C# divide-by-zero failure without unwinding the render worker.
        if seq_num_periods == 0 {
            self.failed = true;
            return;
        }
        let period = (self.speed_acc as i32 as i64) % seq_num_periods;
        let offset = self.seq_num_channels as u64 * period.max(0) as u64;
        if file.seek(SeekFrom::Start(offset)).is_err() {
            return;
        }
        let mut readcnt = 0usize;
        while readcnt < self.seq_num_channels {
            match file.read(&mut self.frame_buffer[readcnt..]) {
                Ok(0) | Err(_) => break,
                Ok(n) => readcnt += n,
            }
        }

        let (wi, ht) = (info.buffer_wi(), info.buffer_ht());
        let row = (wi as usize) * 3;
        if row == 0 {
            return;
        }
        let mut j = 0usize;
        while j + 2 < readcnt {
            let mut color = Color::rgb(self.frame_buffer[j], self.frame_buffer[j + 1], self.frame_buffer[j + 2]);
            let x = ((j % row) / 3) as i32;
            let y = (ht - 1) - (j / row) as i32;
            if level < 1.0 {
                let mut hsv = Hsv::from_color(color);
                hsv.set_v(hsv.v * level);
                color = hsv.to_color();
            }
            if x < wi && y < ht && y >= 0 {
                buf.set_pixel(x, y, color);
            }
            j += 3;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_effects::testing;

    #[test]
    fn orientation_follows_the_target_and_stays_one_value() {
        use vixen_core::element::{property_ids, Orientation, PropertyData, PropertyInstance};
        let mut tree = ElementTree::new();
        let plain = tree.add_string("Plain", 3);
        let turned = tree.add_string("Turned", 3);
        tree.node_mut(turned).properties.push(PropertyInstance {
            type_id: property_ids::ORIENTATION,
            instance_id: Uuid::new_v4(),
            data: PropertyData::Orientation(Orientation::Horizontal),
        });
        let mut p = DESCRIPTOR.new_params();
        p.set("Orientation", ParamValue::Enum("Vertical".into()));
        DESCRIPTOR.set_targets(&mut p, &testing::target_info(&tree, &[turned]));
        assert_eq!(p.enum_name("Orientation"), "Horizontal");
        assert_eq!(p.enum_name("StringOrientation"), "Horizontal");

        // No property on the target: what was stored stays, in both names.
        let mut p = DESCRIPTOR.new_params();
        p.set("Orientation", ParamValue::Enum("Horizontal".into()));
        DESCRIPTOR.set_targets(&mut p, &testing::target_info(&tree, &[plain]));
        assert_eq!(p.enum_name("StringOrientation"), "Horizontal");

        // Loading fills in the unsaved name from the saved one.
        let mut p = DESCRIPTOR.new_params();
        p.set("Orientation", ParamValue::Enum("Horizontal".into()));
        DESCRIPTOR.load_data(&mut p, &testing::target_info(&tree, &[plain]));
        assert_eq!(p.enum_name("StringOrientation"), "Horizontal");
    }

    #[test]
    fn smoke() {
        testing::smoke_test(&DESCRIPTOR);
    }

    #[test]
    fn missing_file_renders_nothing() {
        let mut tree = ElementTree::new();
        let m = testing::grid(&mut tree, "M", 4, 4);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[m], 1000, |p| {
            p.set("FileName", ParamValue::Str("nope.gled".into()));
        });
        assert!(crate::testutil::lit_count(&tree, m, &out, 0) == 0);
    }

    #[test]
    fn speed_mode_visibility() {
        let mut p = DESCRIPTOR.new_params();
        p.set("MovementType", ParamValue::Enum("Speed".to_string()));
        let visible: Vec<_> = DESCRIPTOR.visible_params(&p).into_iter().map(|i| DESCRIPTOR.params[i].name).collect();
        assert!(visible.contains(&"Speed"));
        assert!(!visible.contains(&"Iterations"));
    }

    #[test]
    fn short_recordings_emit_no_intents_that_could_mask_lower_layers() {
        let dir = std::env::temp_dir().join(format!("gled_short_{}", Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("Glediator")).unwrap();
        let mut tree = ElementTree::new();
        let grid = testing::grid(&mut tree, "Grid", 2, 2);
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let ctx = RenderCtx { video_cache: None,
            tree: &tree, targets: &[grid], start_time: TimeSpan::ZERO,
            time_span: TimeSpan::from_ms_i(100), frame_ms: 50, marks: &[], audio: None,
            module_data_root: Some(&dir), cancel: &cancel,
        };
        let mut p = DESCRIPTOR.new_params();
        p.set("FileName", ParamValue::Str("short.gled".into()));
        for size in [0, 1, 2, 11] {
            std::fs::write(dir.join("Glediator/short.gled"), vec![255; size]).unwrap();
            assert!((DESCRIPTOR.render)(&ctx, &p).map.is_empty(), "size {size}");
        }
        std::fs::write(dir.join("Glediator/short.gled"), vec![255; 12]).unwrap();
        assert!(!(DESCRIPTOR.render)(&ctx, &p).map.is_empty());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn plays_recorded_frames_bottom_up() {
        // 2x2 matrix, vertical orientation: x = string, y = pixel. File row 0 is the top (y = 1).
        let dir = std::env::temp_dir().join(format!("gled_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(dir.join("Glediator"));
        // period 0: row0 = (red, red), row1 = black. period 1: all green.
        let mut data = vec![0u8; 24];
        data[0] = 255;
        data[3] = 255;
        for i in 0..4 {
            data[12 + i * 3 + 1] = 255;
        }
        std::fs::write(dir.join("Glediator").join("t.gled"), &data).unwrap();

        let mut tree = ElementTree::new();
        let m = testing::grid(&mut tree, "M", 2, 2);
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let ctx = RenderCtx { video_cache: None,
            tree: &tree,
            targets: &[m],
            start_time: TimeSpan::ZERO,
            time_span: TimeSpan::from_ms_i(100),
            frame_ms: 50,
            marks: &[],
            audio: None,
            module_data_root: Some(&dir),
            cancel: &cancel,
        };
        let mut p = DESCRIPTOR.new_params();
        p.set("FileName", ParamValue::Str("t.gled".into()));
        p.set("MovementType", ParamValue::Enum("Speed".into()));
        p.set("Speed", ParamValue::Int(1));
        let out = (DESCRIPTOR.render)(&ctx, &p);
        // Frame 0: _speed = 1 + 1 = 2, period 0: top row (pixel 1) red, bottom row (pixel 0) black.
        assert_eq!(crate::testutil::color_at(&tree, m, &out, 0, 1, 0), (255, 0, 0));
        assert_eq!(crate::testutil::color_at(&tree, m, &out, 1, 1, 0), (255, 0, 0));
        assert_eq!(crate::testutil::color_at(&tree, m, &out, 0, 0, 0), (0, 0, 0));
        // Frame 1: period 1, all green.
        assert_eq!(crate::testutil::color_at(&tree, m, &out, 0, 0, 50), (0, 255, 0));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
