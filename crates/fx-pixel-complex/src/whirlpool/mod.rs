//! Port of VixenModules.Effect.Whirlpool. The effect lays out a grid of whirls (`whirl.rs`), advances them
//! through repeating iterations and renders them into the frame buffer (or a virtual one for locations).
//!
//! The effect-level Whirl properties (WhirlMode, Thickness, ...) are only a way to set every whirl at once in
//! the C#, so `param_changed` copies them into each `WhirlData` record, and `render` reads the records.

mod whirl;

use crate::generated::whirlpool as gen;
use vixen_effects::prelude::*;
use whirl::Whirl;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    param_changed: Some(param_changed),
    param_changed_ctx: Some(param_changed_ctx),
    supports_location: true,
    ported: true,
    ..gen::BASE
};

/// `UpdateAllAttributes`.
fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    let locations = p.enum_name("TargetPositioning") == "Locations";
    v.set("StringOrientation", !locations);
    v.set("TargetPositioning", true);
    v.set("RenderScaleFactor", locations);

    let color_mode = p.enum_name("ColorMode");
    v.set("SingleColor", color_mode == "GradientOverTime");
    v.set("BandLength", color_mode == "Bands");
    v.set("Colors", color_mode == "RectangularRings" || color_mode == "Bands");
    for leg in ["LeftColor", "RightColor", "TopColor", "BottomColor"] {
        v.set(leg, color_mode == "LegColors");
    }

    // The whirl list is only shown for individual configuration.
    v.set("WhirlData", p.boolean("IndividualConfiguration"));
    v.set("Show3D", p.int("Thickness") > 1);
    let panels = p.int("Rows") > 1 || p.int("Columns") > 1;
    v.set("IndividualConfiguration", panels);
    v.set("PanelSpacing", panels);

    let mode = p.enum_name("WhirlMode");
    v.set("TailLength", mode == "Meteor");
    v.set("Rotation", mode != "SymmetricalWhirls");
    v.set("StartLocation", mode != "SymmetricalWhirls");
    v.set("Inverted", mode != "SymmetricalWhirls");
    v.set("ColorMode", mode != "Meteor");
}

/// `Whirl.InitAllAttributes`, for an editor showing one whirl.
pub(crate) fn whirl_visibility(whirl: &Params, _effect: &Params, v: &mut Visibility) {
    let color_mode = whirl.enum_name("ColorMode");
    v.set("SingleColor", color_mode == "GradientOverTime");
    v.set("BandLength", color_mode == "Bands");
    v.set("Colors", color_mode == "RectangularRings" || color_mode == "Bands");
    for leg in ["LeftColor", "RightColor", "TopColor", "BottomColor"] {
        v.set(leg, color_mode == "LegColors");
    }
    // CalculateThickness: at least one pixel.
    let thickness = (whirl.int("Thickness") as f64 / 100.0 * whirl.int("Width").max(whirl.int("Height")) as f64) as i32;
    v.set("Show3D", thickness.max(1) > 1);
    let mode = whirl.enum_name("WhirlMode");
    v.set("TailLength", mode == "Meteor");
    v.set("Rotation", mode != "SymmetricalWhirls");
    v.set("StartLocation", mode != "SymmetricalWhirls");
    v.set("ReverseDraw", mode != "SymmetricalWhirls");
    v.set("ColorMode", mode != "Meteor");
}

/// Effect-level property -> `WhirlData` member, as the `UpdateXxx` property setters broadcast them.
const BROADCAST: [(&str, &str); 17] = [
    ("WhirlMode", "WhirlMode"),
    ("TailLength", "TailLength"),
    ("Rotation", "Rotation"),
    ("StartLocation", "StartLocation"),
    ("WhirlDirection", "WhirlDirection"),
    ("Inverted", "ReverseDraw"),
    ("Spacing", "Spacing"),
    ("Thickness", "Thickness"),
    ("Show3D", "Show3D"),
    ("ColorMode", "ColorMode"),
    ("SingleColor", "SingleColor"),
    ("BandLength", "BandLength"),
    ("Colors", "Colors"),
    ("LeftColor", "LeftColor"),
    ("RightColor", "RightColor"),
    ("TopColor", "TopColor"),
    ("BottomColor", "BottomColor"),
];

fn broadcast(p: &mut Params, effect_name: &str, whirl_name: &str) {
    let value = p.get(effect_name).clone();
    if let Some(i) = p.index_of("WhirlData") {
        if let ParamValue::RecordList(list) = &mut p.values[i] {
            for whirl in list.iter_mut() {
                whirl.set(whirl_name, value.clone());
            }
        }
    }
}

/// `Whirl.WhirlMode`'s setter, for an edited row, which reaches this as the whole list: becoming a Meteor forces the single colour mode.
fn param_changed_ctx(c: &ParamChange, p: &mut Params) {
    if c.name != "WhirlData" {
        return;
    }
    let old: &[Params] = if let ParamValue::RecordList(old) = c.old { old } else { &[] };
    let Some(i) = p.index_of("WhirlData") else { return };
    let ParamValue::RecordList(rows) = &mut p.values[i] else { return };
    for (index, row) in rows.iter_mut().enumerate() {
        let was_meteor = old.get(index).is_some_and(|o| o.enum_name("WhirlMode") == "Meteor");
        if !was_meteor && row.enum_name("WhirlMode") == "Meteor" {
            row.set("ColorMode", ParamValue::Enum("GradientOverTime".to_string()));
        }
    }
}

/// The side effects of the property setters: effect-level whirl settings are copied to every whirl, a
/// Meteor forces the single colour mode, and Rows and Columns resize the whirl list (`UpdateGridPanels`).
fn param_changed(name: &str, p: &mut Params) {
    if let Some((effect_name, whirl_name)) = BROADCAST.iter().find(|(n, _)| *n == name) {
        broadcast(p, effect_name, whirl_name);
        if name == "WhirlMode" && p.enum_name("WhirlMode") == "Meteor" {
            p.set("ColorMode", ParamValue::Enum("GradientOverTime".to_string()));
            broadcast(p, "ColorMode", "ColorMode");
        }
    } else if name == "Rows" || name == "Columns" {
        let panels = (p.int("Rows").max(0) as usize) * (p.int("Columns").max(0) as usize);
        if let Some(i) = p.index_of("WhirlData") {
            if let ParamValue::RecordList(list) = &mut p.values[i] {
                list.truncate(panels);
                while list.len() < panels {
                    match list.last().cloned() {
                        Some(last) => list.push(last),
                        None => list.push(Params::defaults(&gen::WHIRL_PARAMS)),
                    }
                }
            }
        }
    }
}

struct Whirlpool {
    whirls: Vec<Whirl>,
    rows: i32,
    columns: i32,
    panel_spacing: i32,
    x_scale: i32,
    y_scale: i32,
    x_offset_curve: Curve,
    y_offset_curve: Curve,
    level_curve: Curve,
    iterations: i32,
    pause_at_end: i32,
    render_scale_factor: i32,
    locations: bool,

    buffer_ht: i32,
    buffer_wi: i32,
    scaled_buffer_ht: i32,
    scaled_buffer_wi: i32,
    frames_per_iteration: i32,
    iteration_frame: i32,
    /// False when the C# would throw (no panels, zero iterations): nothing is drawn.
    valid: bool,
    virtual_fb: Option<PixelFrameBuffer>,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let mut e = Whirlpool {
        whirls: p.records("WhirlData").iter().map(Whirl::from_record).collect(),
        rows: p.int("Rows"),
        columns: p.int("Columns"),
        panel_spacing: p.int("PanelSpacing"),
        x_scale: p.int("XScale"),
        y_scale: p.int("YScale"),
        x_offset_curve: p.curve("XOffsetCurve"),
        y_offset_curve: p.curve("YOffsetCurve"),
        level_curve: p.curve("LevelCurve"),
        iterations: p.int("Iterations"),
        pause_at_end: p.int("PauseAtEnd"),
        render_scale_factor: p.int("RenderScaleFactor"),
        locations: TargetPositioning::from_name(p.enum_name("TargetPositioning")) == TargetPositioning::Locations,
        buffer_ht: 0,
        buffer_wi: 0,
        scaled_buffer_ht: 0,
        scaled_buffer_wi: 0,
        frames_per_iteration: 0,
        iteration_frame: 0,
        valid: true,
        virtual_fb: None,
    };
    let intents = render_pixel_effect(ctx, p, &mut e);
    // An exception in a pixel effect means `_elementData` is never assigned, so the whole effect is dark.
    if e.valid {
        intents
    } else {
        EffectIntents::new()
    }
}

impl Whirlpool {
    /// `UpdateGridPanels`: matches the whirl count to the grid and lays the panels out.
    fn update_grid_panels(&mut self, buffer_wi: i32, buffer_ht: i32) {
        if self.rows <= 0 || self.columns <= 0 {
            self.whirls.clear();
            self.valid = false;
            return;
        }
        let panels = (self.rows as i64 * self.columns as i64).min(10_000) as usize;
        self.whirls.truncate(panels);
        while self.whirls.len() < panels {
            let next = match self.whirls.last() {
                Some(last) => last.clone(),
                None => Whirl::new(),
            };
            self.whirls.push(next);
        }
        let spacing = (self.panel_spacing as f64 / 100.0 * buffer_wi.max(buffer_ht) as f64) as i32;
        let panel_width = (self.scaled_buffer_wi - self.columns * spacing) / self.columns;
        let panel_height = (self.scaled_buffer_ht - self.rows * spacing) / self.rows;
        let mut cell = 0usize;
        for r in 0..self.rows {
            for c in 0..self.columns {
                let w = &mut self.whirls[cell];
                w.width = panel_width;
                w.height = panel_height;
                w.x = c * panel_width + spacing * c;
                w.y = r * panel_height + spacing * r;
                cell += 1;
            }
        }
    }

    fn x_offset(&self, interval_pos: f64) -> i32 {
        let half = (self.buffer_wi as f64 / 2.0) as i32;
        let neg = (-(self.buffer_wi as f64) / 2.0) as i32;
        scale_curve_to_value(self.x_offset_curve.get_value(interval_pos), half as f64, neg as f64).round_ties_even() as i32
    }

    fn y_offset(&self, interval_pos: f64) -> i32 {
        let half = (self.buffer_ht as f64 / 2.0) as i32;
        let neg = (-(self.buffer_ht as f64) / 2.0) as i32;
        scale_curve_to_value(self.y_offset_curve.get_value(interval_pos), half as f64, neg as f64).round_ties_even() as i32
    }

    fn render_frame(&mut self, frame: i32, info: &PixelInfo, fb: &mut PixelFrameBuffer) {
        if !self.valid {
            return;
        }
        self.iteration_frame += 1;
        if self.iteration_frame > self.frames_per_iteration {
            self.iteration_frame = 0;
            for w in &mut self.whirls {
                w.reset_pixel_number();
            }
        }
        let interval_pos = info.interval_position(frame);
        let factor = interval_pos * 100.0;
        let level = self.level_curve.get_value(factor) / 100.0;
        let (xo, yo) = (self.x_offset(factor), self.y_offset(factor));
        let it = self.iteration_frame;
        for w in &mut self.whirls {
            if w.enabled {
                w.x_offset = xo;
                w.y_offset = yo;
                w.intensity = level;
                w.render_effect(it, fb, interval_pos);
                if w.threw {
                    self.valid = false;
                    return;
                }
            }
        }
    }
}

impl PixelEffect for Whirlpool {
    fn setup_render(&mut self, info: &PixelInfo) {
        let (bw, bh) = (info.buffer_wi(), info.buffer_ht());
        self.valid = true;
        if self.locations {
            // The C# divides by the scale factor, which throws for zero.
            let f = self.render_scale_factor.max(1);
            self.buffer_wi = bw / f;
            self.buffer_ht = bh / f;
            // The C# flags this rounding as probably wrong; it is kept.
            if bh % f != 0 {
                self.buffer_ht += 1;
            }
            if bw % f != 0 {
                self.buffer_wi += 1;
            }
        } else {
            self.buffer_wi = bw;
            self.buffer_ht = bh;
        }
        self.scaled_buffer_wi = self.buffer_wi * self.x_scale / 100;
        self.scaled_buffer_ht = self.buffer_ht * self.y_scale / 100;
        self.update_grid_panels(bw, bh);
        self.iteration_frame = 0;
        if self.iterations <= 0 {
            self.valid = false;
            return;
        }
        self.frames_per_iteration = info.number_frames() / self.iterations;
        for w in &mut self.whirls {
            // The C# assigns PauseAtEnd after SetupRender, so a freshly loaded effect ignores it until its second render, and an
            // edit to it shows only on the render after next. Applying it first gives what those later renders settle on.
            w.pause_at_end = self.pause_at_end;
            w.setup_render(self.frames_per_iteration);
            if w.threw {
                self.valid = false;
                return;
            }
        }
    }

    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        self.render_frame(frame, info, buf);
    }

    fn render_effect_by_location(&mut self, num_frames: i32, info: &PixelInfo, buf: &mut PixelLocationFrameBuffer) {
        let locations = buf.element_locations().to_vec();
        let scale = self.render_scale_factor.max(1);
        let buffer_ht = info.buffer_ht();
        for frame in 0..num_frames {
            buf.current_frame = frame;
            let mut virtual_fb = match self.virtual_fb.take() {
                Some(mut v) if v.width() == self.buffer_wi && v.height() == self.buffer_ht => {
                    v.fill(Color::EMPTY);
                    v
                }
                _ => PixelFrameBuffer::new(self.buffer_wi, self.buffer_ht),
            };
            self.render_frame(frame, info, &mut virtual_fb);
            for l in &locations {
                // Flip and offset the coordinates so the location version acts like the string version.
                let mut y = ((info.buffer_ht_offset() - l.y) + (buffer_ht - 1 + info.buffer_ht_offset())).abs();
                y -= info.buffer_ht_offset();
                let x = l.x - info.buffer_wi_offset();
                let (sx, sy) = (x / scale, y / scale);
                // C# indexes unchecked and throws outside the virtual buffer; those pixels stay dark here.
                if sx >= 0 && sy >= 0 && sx < virtual_fb.width() && sy < virtual_fb.height() {
                    buf.set_pixel(l.x, l.y, virtual_fb.get_color_at(sx, sy));
                }
            }
            self.virtual_fb = Some(virtual_fb);
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

    fn set_enum(p: &mut Params, name: &str, v: &str) {
        p.set(name, ParamValue::Enum(v.to_string()));
    }

    fn set_whirl(p: &mut Params, edit: impl Fn(&mut Params)) {
        if let Some(i) = p.index_of("WhirlData") {
            if let ParamValue::RecordList(list) = &mut p.values[i] {
                for w in list.iter_mut() {
                    edit(w);
                }
            }
        }
    }

    #[test]
    fn defaults_have_one_whirl_like_whirlpool_data() {
        let p = DESCRIPTOR.new_params();
        let whirls = p.records("WhirlData");
        assert_eq!(whirls.len(), 1);
        assert!(whirls[0].boolean("Enabled"));
        assert_eq!(whirls[0].int("Thickness"), 2);
        assert_eq!(whirls[0].gradients("Colors").len(), 2);
    }

    #[test]
    fn every_mode_corner_rotation_and_direction_renders() {
        let mut tree = ElementTree::new();
        let g = testing::grid(&mut tree, "G", 13, 19);
        let single = tree.add_string("S", 1);
        let located = testing::grid(&mut tree, "L", 4, 10);
        testing::add_locations(&mut tree, located);
        for mode in ["RecurrentWhirls", "SymmetricalWhirls", "Meteor"] {
            for start in ["TopLeft", "BottomLeft", "TopRight", "BottomRight"] {
                for rot in ["Clockwise", "CounterClockwise"] {
                    for dir in ["In", "Out", "InAndOut"] {
                        for (rev, color) in [(false, "GradientOverTime"), (true, "Bands"), (false, "RectangularRings"), (true, "LegColors")] {
                            let edit = |p: &mut Params| {
                                set_whirl(p, |w| {
                                    set_enum(w, "WhirlMode", mode);
                                    set_enum(w, "StartLocation", start);
                                    set_enum(w, "Rotation", rot);
                                    set_enum(w, "WhirlDirection", dir);
                                    set_enum(w, "ColorMode", color);
                                    w.set("ReverseDraw", ParamValue::Bool(rev));
                                    w.set("Show3D", ParamValue::Bool(rev));
                                });
                            };
                            for target in [g, single] {
                                testing::render_with(&DESCRIPTOR, &tree, &[target], 600, edit);
                            }
                        }
                    }
                }
            }
        }
        testing::render_with(&DESCRIPTOR, &tree, &[located], 600, |p| set_enum(p, "TargetPositioning", "Locations"));
        // A grid of panels, no whirls, zero iterations and 100% pause must not fail.
        testing::render_with(&DESCRIPTOR, &tree, &[g], 800, |p| {
            p.set("Rows", ParamValue::Int(3));
            p.set("Columns", ParamValue::Int(2));
            p.set("PanelSpacing", ParamValue::Int(10));
        });
        testing::render_with(&DESCRIPTOR, &tree, &[g], 800, |p| {
            p.set("WhirlData", ParamValue::RecordList(Vec::new()));
        });
        testing::render_with(&DESCRIPTOR, &tree, &[g], 800, |p| {
            p.set("Iterations", ParamValue::Int(0));
        });
        testing::render_with(&DESCRIPTOR, &tree, &[g], 800, |p| {
            p.set("PauseAtEnd", ParamValue::Int(100));
        });
        testing::render_with(&DESCRIPTOR, &tree, &[g], 800, |p| {
            p.set("Iterations", ParamValue::Int(10));
        });
    }

    /// Lit pixels (anything non-black) of one element at a time, as (string, pixel) from the last frame.
    fn last_frame(out: &EffectIntents, tree: &ElementTree, target: NodeId, ms: i64, strings: usize, pixels: usize) -> Vec<Vec<bool>> {
        let ids = tree.distinct_elements(target);
        let mut grid = vec![vec![false; pixels]; strings];
        for (i, id) in ids.iter().enumerate() {
            let n = &out.nodes_for(id).unwrap()[0];
            let lit = match n.intent.state_at(TimeSpan::from_ms_i(ms - 50)) {
                vixen_core::StateValue::Rgb(v) => v.r > 0 || v.g > 0 || v.b > 0,
                _ => false,
            };
            grid[i / pixels][i % pixels] = lit;
        }
        grid
    }

    /// A 10x10 Recurrent whirl with 1 pixel thickness and spacing is a spiral: the outer arm runs along the
    /// first pixel of every string, the last string and the last pixel of every string, and the arm that
    /// closes the loop stops one thickness plus spacing short, leaving the gap at pixel 8 of the first string.
    /// The next arm starts two pixels in, with a gap pixel between the arms.
    #[test]
    fn finished_whirl_is_a_spiral_with_gaps() {
        let mut tree = ElementTree::new();
        let g = testing::grid(&mut tree, "G", 10, 10);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[g], 2000, |p| {
            p.set("PauseAtEnd", ParamValue::Int(20));
            set_whirl(p, |w| {
                w.set("Thickness", ParamValue::Int(10));
                w.set("Spacing", ParamValue::Int(10));
            });
        });
        let grid = last_frame(&out, &tree, g, 2000, 10, 10);
        for i in 0..10 {
            assert!(grid[i][0] && grid[i][9] && grid[9][i], "outer arm pixel {i}");
        }
        for i in 0..8 {
            assert!(grid[0][i], "closing arm pixel {i}");
        }
        assert!(!grid[0][8] && grid[0][9]);
        // The pixels just inside the outer arm are the spacing.
        for i in 1..9 {
            assert!(!grid[i][1] || i == 1 || i == 8, "gap pixel {i}");
        }
        assert!(!grid[8][2] && grid[2][2]);
    }

    #[test]
    fn effect_level_settings_are_copied_to_every_whirl() {
        let mut p = DESCRIPTOR.new_params();
        p.set("Rows", ParamValue::Int(2));
        param_changed("Rows", &mut p);
        assert_eq!(p.records("WhirlData").len(), 2);
        p.set("Thickness", ParamValue::Int(9));
        param_changed("Thickness", &mut p);
        assert!(p.records("WhirlData").iter().all(|w| w.int("Thickness") == 9));
        set_enum(&mut p, "WhirlMode", "Meteor");
        param_changed("WhirlMode", &mut p);
        assert!(p.records("WhirlData").iter().all(|w| w.enum_name("WhirlMode") == "Meteor" && w.enum_name("ColorMode") == "GradientOverTime"));
        p.set("Inverted", ParamValue::Bool(true));
        param_changed("Inverted", &mut p);
        assert!(p.records("WhirlData").iter().all(|w| w.boolean("ReverseDraw")));
    }

    #[test]
    fn a_whirl_shows_what_its_modes_use() {
        let shown = |w: &Params, name: &'static str| {
            let mut v = Visibility::default();
            (gen::WHIRL_DATA.visibility.unwrap())(w, w, &mut v);
            v.overrides.iter().find(|o| o.0 == name).map(|o| o.1).unwrap()
        };
        let mut w = Params::defaults(&gen::WHIRL_PARAMS);
        set_enum(&mut w, "ColorMode", "Bands");
        assert!(shown(&w, "BandLength") && shown(&w, "Colors") && !shown(&w, "SingleColor") && !shown(&w, "LeftColor"));
        set_enum(&mut w, "WhirlMode", "SymmetricalWhirls");
        assert!(!shown(&w, "Rotation") && !shown(&w, "ReverseDraw"));
        // 10% of a 20 pixel whirl is two pixels thick, enough for 3D. One pixel is not.
        w.set("Thickness", ParamValue::Int(10));
        w.set("Width", ParamValue::Int(20));
        assert!(shown(&w, "Show3D"));
        w.set("Width", ParamValue::Int(10));
        assert!(!shown(&w, "Show3D"));
    }

    #[test]
    fn visibility_follows_color_mode_whirl_mode_and_grid() {
        let names = |p: &Params| -> Vec<&'static str> { DESCRIPTOR.visible_params(p).into_iter().map(|i| DESCRIPTOR.params[i].name).collect() };
        let mut p = DESCRIPTOR.new_params();
        let v = names(&p);
        assert!(v.contains(&"SingleColor") && v.contains(&"Show3D") && v.contains(&"Rotation"));
        assert!(!v.contains(&"TailLength") && !v.contains(&"IndividualConfiguration") && !v.contains(&"WhirlData"));
        set_enum(&mut p, "WhirlMode", "SymmetricalWhirls");
        set_enum(&mut p, "ColorMode", "Bands");
        p.set("Columns", ParamValue::Int(2));
        p.set("Thickness", ParamValue::Int(1));
        let v = names(&p);
        assert!(!v.contains(&"Rotation") && !v.contains(&"SingleColor") && !v.contains(&"Show3D"));
        assert!(v.contains(&"BandLength") && v.contains(&"Colors") && v.contains(&"IndividualConfiguration"));
    }
}
