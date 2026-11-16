//! Port of VixenModules.Effect.Fire.

use crate::generated::fire as gen;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    targets_changed: Some(normalize_targeting),
    param_changed_ctx: Some(param_changed_ctx),
    supports_location: true,
    selection: Some(selection),
    ported: true,
    ..gen::BASE
};

/// `UpdateStringOrientationAttributes`, `EnableTargetPositioning` and `UpdateTargetingAttributes`.
fn visibility(p: &Params, t: &TargetInfo, v: &mut Visibility) {
    v.set("StringOrientation", p.enum_name("TargetPositioning") == "Strings");
    v.set("TargetPositioning", true);
    let depth = t.determine_depth();
    v.set("TargetNodeHandling", !t.targets.is_empty() && (t.targets.len() > 1 || depth > 2));
    v.set("DepthOfEffect", p.enum_name("TargetNodeHandling") == "Individual" && t.targets.len() == 1 && depth > 2);
}

/// `FireTargetElementDepthConverter`: only the depths between the target and its leaves.
fn selection(name: &str, _row: Option<&Params>, _p: &Params, t: &TargetInfo) -> Option<Vec<SelectionValue>> {
    (name == "DepthOfEffect").then(|| depth_choices(1, t.targets.first().map_or(0, |n| t.tree.max_child_depth(*n)) - 1))
}

/// The data half of `UpdateTargetingAttributes`, which unlike Wipe's has no guard for an effect with no targets yet.
fn normalize_targeting(p: &mut Params, t: &TargetInfo) {
    let depth = t.determine_depth();
    let handling_visible = !t.targets.is_empty() && (t.targets.len() > 1 || depth > 2);
    if !handling_visible && p.enum_name("TargetNodeHandling") == "Individual" {
        p.set("TargetNodeHandling", ParamValue::Enum("Group".to_string()));
    }
    let individual = p.enum_name("TargetNodeHandling") == "Individual";
    if t.targets.len() > 1 || !individual {
        p.set("DepthOfEffect", ParamValue::Int(0));
    } else {
        // IsUsefulIntermediateDepth, GetFirstUsefulIntermediateDepth
        let selected = p.int("DepthOfEffect");
        if !(selected > 0 && selected < depth - 1) {
            p.set("DepthOfEffect", ParamValue::Int(if depth > 2 { 1 } else { 0 }));
        }
    }
}

/// `UpdateTargetingAttributes` also runs from the `ModuleData` setter, which is how a saved value reaches it on load, and
/// from the two setters that change what it normalises.
fn param_changed_ctx(c: &ParamChange, p: &mut Params) {
    if matches!(c.name, MODULE_DATA | "TargetNodeHandling" | "DepthOfEffect") {
        normalize_targeting(p, c.info);
    }
}

#[derive(Copy, Clone, PartialEq)]
enum FireDirection {
    Bottom,
    Top,
    Left,
    Right,
}

struct FrameState {
    level: f64,
    hue_shift: f64,
    step: i32,
}

struct Fire {
    location: FireDirection,
    height: Curve,
    level_curve: Curve,
    hue_shift_curve: Curve,
    palette: Vec<Hsv>,
    fire_buffer: Vec<i32>,
    rand: Rand,
    /// The render groups worked out from the targets in `render`, which has the element tree.
    groups: Option<Vec<Vec<NodeId>>>,
}

/// `FirePalette`: 100 reds from black to bright red, then 100 hues from red to yellow.
fn build_palette() -> Vec<Hsv> {
    let mut palette = Vec::with_capacity(200);
    let mut hsv = Hsv::default();
    hsv.set_h(0.0);
    hsv.set_s(1.0);
    for i in 0..100 {
        hsv.set_v((i as f32 / 100.0f32) as f64);
        palette.push(hsv);
    }
    hsv.set_v(1.0);
    for _ in 0..100 {
        palette.push(hsv);
        hsv.set_h(hsv.h + 0.00166666f32 as f64);
    }
    palette
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    // GetRenderGroups. The handling and depth were normalised when the targets or the data were assigned.
    let targets = ctx.targets;
    let individual = p.enum_name("TargetNodeHandling") == "Individual";
    let depth_of_effect = p.int("DepthOfEffect");

    let groups = if !individual {
        None
    } else if targets.len() > 1 {
        let mut seen: Vec<NodeId> = Vec::new();
        for t in targets {
            if !seen.contains(t) {
                seen.push(*t);
            }
        }
        Some(seen.into_iter().map(|t| vec![t]).collect())
    } else if let Some(&target) = targets.first() {
        Some(ctx.tree.nodes_at_depth(target, depth_of_effect).into_iter().map(|n| vec![n]).collect())
    } else {
        Some(Vec::new())
    };

    let mut e = Fire {
        location: match p.enum_name("Location") {
            "Top" => FireDirection::Top,
            "Left" => FireDirection::Left,
            "Right" => FireDirection::Right,
            _ => FireDirection::Bottom,
        },
        height: p.curve("Height"),
        level_curve: p.curve("LevelCurve"),
        hue_shift_curve: p.curve("HueShiftCurve"),
        palette: build_palette(),
        fire_buffer: Vec::new(),
        rand: Rand::default(),
        groups,
    };
    render_pixel_effect(ctx, p, &mut e)
}

impl Fire {
    /// `GetFireBuffer`: -1 outside the simulation.
    #[inline]
    fn get_fire_buffer(&self, x: i32, y: i32, max_wi: i32, max_ht: i32) -> i32 {
        if x >= 0 && x < max_wi && y >= 0 && y < max_ht {
            self.fire_buffer.get(y as usize * max_wi as usize + x as usize).copied().unwrap_or(-1)
        } else {
            -1
        }
    }

    fn simulation_dimensions(&self, info: &PixelInfo) -> (i32, i32) {
        if matches!(self.location, FireDirection::Left | FireDirection::Right) {
            (info.buffer_ht(), info.buffer_wi())
        } else {
            (info.buffer_wi(), info.buffer_ht())
        }
    }

    fn simulation_coordinate(&self, info: &PixelInfo, out_x: i32, out_y: i32) -> (i32, i32) {
        match self.location {
            FireDirection::Bottom => (out_x, out_y),
            FireDirection::Top => (out_x, info.buffer_ht() - out_y - 1),
            FireDirection::Left => (out_y, out_x),
            FireDirection::Right => (out_y, info.buffer_wi() - out_x - 1),
        }
    }

    fn frame_state(&self, frame: i32, max_ht: i32, info: &PixelInfo) -> FrameState {
        let interval_pos_factor = info.interval_position(frame) * 100.0;
        let mut effective_height = self.height.get_value(interval_pos_factor) as i32;
        if effective_height <= 0 {
            effective_height = 1;
        }
        FrameState {
            level: self.level_curve.get_value(interval_pos_factor) / 100.0,
            hue_shift: scale_curve_to_value(self.hue_shift_curve.get_value(interval_pos_factor), 100.0, 0.0),
            step: 255 * 100 / max_ht.max(1) / effective_height,
        }
    }

    fn generate_fire_buffer(&mut self, max_wi: i32, max_ht: i32, step: i32) {
        let count = self.palette.len() as i32;
        for x in 0..max_wi {
            let r = if x % 2 == 0 { 190 + (self.rand.next() % 10) } else { 100 + (self.rand.next() % 50) };
            self.fire_buffer[x as usize] = r;
        }
        for y in 1..max_ht {
            for x in 0..max_wi {
                let v1 = self.get_fire_buffer(x - 1, y - 1, max_wi, max_ht);
                let v2 = self.get_fire_buffer(x + 1, y - 1, max_wi, max_ht);
                let v3 = self.get_fire_buffer(x, y - 1, max_wi, max_ht);
                // v4 reads the same cell as v3 on purpose; the C# counts it twice.
                let v4 = v3;
                let mut n = 0;
                let mut sum = 0;
                for v in [v1, v2, v3, v4] {
                    if v >= 0 {
                        sum += v;
                        n += 1;
                    }
                }
                let mut new_index = if n > 0 { sum / n } else { 0 };
                if new_index > 0 {
                    new_index += if self.rand.next() % 100 < 20 { step } else { -step };
                    if new_index < 0 {
                        new_index = 0;
                    }
                    if new_index >= count {
                        new_index = count - 1;
                    }
                }
                self.fire_buffer[y as usize * max_wi as usize + x as usize] = new_index;
            }
        }
    }

    fn fire_color(&self, x: i32, y: i32, max_wi: i32, max_ht: i32, fs: &FrameState) -> Option<Hsv> {
        let color_index = self.get_fire_buffer(x, y, max_wi, max_ht);
        if color_index == 0 {
            return None;
        }
        // The C# would throw for -1 (outside the simulation); stay dark instead.
        let mut hsv = *self.palette.get(color_index as usize)?;
        if fs.hue_shift > 0.0 {
            hsv.set_h(hsv.h + fs.hue_shift / 100.0f32 as f64);
        }
        hsv.set_v(hsv.v * fs.level);
        Some(hsv)
    }
}

impl PixelEffect for Fire {
    fn render_groups(&self, targets: &[NodeId]) -> Vec<Vec<NodeId>> {
        match &self.groups {
            Some(g) => g.clone(),
            None => vec![targets.to_vec()],
        }
    }

    fn setup_render(&mut self, info: &PixelInfo) {
        self.fire_buffer = vec![0; (info.buffer_wi().max(0) as usize) * (info.buffer_ht().max(0) as usize)];
    }

    fn cleanup_render(&mut self) {
        self.fire_buffer = Vec::new();
    }

    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        let (max_wi, max_ht) = self.simulation_dimensions(info);
        if max_wi <= 0 || max_ht <= 0 {
            return;
        }
        let fs = self.frame_state(frame, max_ht, info);
        self.generate_fire_buffer(max_wi, max_ht, fs.step);
        for y in 0..max_ht {
            for x in 0..max_wi {
                let Some(hsv) = self.fire_color(x, y, max_wi, max_ht, &fs) else { continue };
                let mut xp = x;
                let mut yp = y;
                if matches!(self.location, FireDirection::Top | FireDirection::Right) {
                    yp = max_ht - y - 1;
                }
                if matches!(self.location, FireDirection::Left | FireDirection::Right) {
                    std::mem::swap(&mut xp, &mut yp);
                }
                buf.set_pixel_hsv(xp, yp, hsv);
            }
        }
    }

    fn render_effect_by_location(&mut self, num_frames: i32, info: &PixelInfo, buf: &mut PixelLocationFrameBuffer) {
        if num_frames <= 0 || info.buffer_wi() <= 0 || info.buffer_ht() <= 0 {
            return;
        }
        let (max_wi, max_ht) = self.simulation_dimensions(info);
        let locs = buf.element_locations().to_vec();
        for frame in 0..num_frames {
            buf.current_frame = frame;
            let fs = self.frame_state(frame, max_ht, info);
            self.generate_fire_buffer(max_wi, max_ht, fs.step);
            for l in &locs {
                let out_x = l.x - info.buffer_wi_offset();
                let mut out_y = ((info.buffer_ht_offset() - l.y) + (info.buffer_ht() - 1 + info.buffer_ht_offset())).abs();
                out_y -= info.buffer_ht_offset();
                let (sx, sy) = self.simulation_coordinate(info, out_x, out_y);
                if let Some(hsv) = self.fire_color(sx, sy, max_wi, max_ht, &fs) {
                    buf.set_pixel_hsv(l.x, l.y, hsv);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_effects::testing;

    #[test]
    fn smoke() {
        testing::smoke_test(&DESCRIPTOR);
        testing::assert_produces_light(&DESCRIPTOR);
    }

    #[test]
    fn targeting_follows_the_targets_on_load_and_on_assignment() {
        let mut tree = ElementTree::new();
        let string = tree.add_string("S", 4);
        let second = tree.add_string("S2", 4);
        let deep = testing::deep_grid(&mut tree, "D", 2, 2, 3);
        let individual = |depth: i32| {
            let mut p = DESCRIPTOR.new_params();
            p.set("TargetNodeHandling", ParamValue::Enum("Individual".into()));
            p.set("DepthOfEffect", ParamValue::Int(depth));
            p
        };
        // A shallow target has nothing to choose between, whether loaded or newly assigned.
        let mut p = individual(1);
        DESCRIPTOR.load_data(&mut p, &testing::target_info(&tree, &[string]));
        assert_eq!((p.enum_name("TargetNodeHandling"), p.int("DepthOfEffect")), ("Group", 0));
        // A deep one keeps a useful depth, and replaces any other with the first useful one.
        let mut p = individual(2);
        DESCRIPTOR.load_data(&mut p, &testing::target_info(&tree, &[deep]));
        assert_eq!((p.enum_name("TargetNodeHandling"), p.int("DepthOfEffect")), ("Individual", 2));
        let mut p = individual(3);
        DESCRIPTOR.set_targets(&mut p, &testing::target_info(&tree, &[deep]));
        assert_eq!(p.int("DepthOfEffect"), 1);
        // The setters run it too, and without Wipe's guard an effect with no targets falls back to Group.
        let mut p = individual(2);
        DESCRIPTOR.set_param(&mut p, "DepthOfEffect", ParamValue::Int(2), &TargetInfo::detached());
        assert_eq!((p.enum_name("TargetNodeHandling"), p.int("DepthOfEffect")), ("Group", 0));

        // What the editor offers.
        let shown = |p: &Params, targets: &[NodeId]| testing::visible_names(&DESCRIPTOR, p, &testing::target_info(&tree, targets));
        let p = individual(1);
        assert!(!shown(&p, &[string]).contains(&"TargetNodeHandling"));
        assert!(shown(&p, &[string, second]).contains(&"TargetNodeHandling"));
        assert!(shown(&p, &[deep]).contains(&"DepthOfEffect"));
        assert!(!shown(&p, &[deep, second]).contains(&"DepthOfEffect"));
    }

    #[test]
    fn palette_runs_black_to_red_to_yellow() {
        let p = build_palette();
        assert_eq!(p.len(), 200);
        assert_eq!(p[0].to_color(), Color::BLACK);
        assert_eq!(p[99].v, (99.0f32 / 100.0f32) as f64);
        assert_eq!(p[100].to_color(), Color::RED);
        // Hue climbs 0.00166666 per step, so the last entry is almost (but not quite) yellow.
        let last = p[199].to_color();
        assert!(last.r == 255 && last.g > 240);
    }

    #[test]
    fn step_scales_with_height() {
        let e = Fire {
            location: FireDirection::Bottom,
            height: Curve::flat(50.0),
            level_curve: Curve::flat100(),
            hue_shift_curve: Curve::flat(0.0),
            palette: build_palette(),
            fire_buffer: Vec::new(),
            rand: Rand::default(),
            groups: None,
        };
        let info = crate::test_util::info(2, 10, 1000);
        // 255 * 100 / 20 / 50 = 25
        assert_eq!(e.frame_state(0, 20, &info).step, 25);
    }

    #[test]
    fn individual_handling_renders_each_target() {
        let mut tree = ElementTree::new();
        let a = tree.add_string("A", 10);
        let b = tree.add_string("B", 10);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[a, b], 500, |p| {
            p.set("TargetNodeHandling", ParamValue::Enum("Individual".to_string()));
        });
        assert_eq!(out.map.len(), 20);
    }
}
