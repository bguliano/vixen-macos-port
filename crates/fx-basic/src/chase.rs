//! Port of VixenModules.Effect.Chase.

use crate::generated::chase as gen;
use vixen_effects::prelude::*;
use vixen_effects::pulse_renderer;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    validate: Some(validate),
    targets_changed: Some(targets_changed),
    param_changed_ctx: Some(param_changed_ctx),
    ported: true,
    ..gen::BASE
};

#[derive(Copy, Clone, PartialEq)]
enum ColorHandling {
    StaticColor,
    GradientThroughWholeEffect,
    GradientForEachPulse,
    ColorAcrossItems,
}

struct Chase<'a> {
    ctx: &'a RenderCtx<'a>,
    has_discrete: bool,
    color_handling: ColorHandling,
    static_gradient: ColorGradient,
    gradient: ColorGradient,
    pulse_curve: Curve,
    chase_movement: Curve,
    pulse_overlap: i32,
    default_level: f64,
    enable_default_level: bool,
    extend_start: bool,
    extend_end: bool,
    out: EffectIntents,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    if ctx.targets.is_empty() {
        return EffectIntents::new();
    }
    let color_handling = match p.enum_name("ColorHandling") {
        "GradientThroughWholeEffect" | "GradientThruEffect" => ColorHandling::GradientThroughWholeEffect,
        "GradientForEachPulse" => ColorHandling::GradientForEachPulse,
        "ColorAcrossItems" => ColorHandling::ColorAcrossItems,
        _ => ColorHandling::StaticColor,
    };
    let mut c = Chase {
        ctx,
        has_discrete: ctx.has_discrete_colors(),
        color_handling,
        static_gradient: ColorGradient::solid(p.color("StaticColor")),
        gradient: p.gradient("ColorGradient"),
        pulse_curve: p.curve("PulseCurve"),
        chase_movement: p.curve("ChaseMovement"),
        pulse_overlap: p.int("PulseOverlap"),
        default_level: p.double("DefaultLevel"),
        enable_default_level: p.boolean("EnableDefaultLevel"),
        extend_start: p.boolean("ExtendPulseToStart"),
        extend_end: p.boolean("ExtendPulseToEnd"),
        out: EffectIntents::new(),
    };

    let depth_of_effect = p.int("DepthOfEffect");
    if p.enum_name("TargetNodeHandling") != "Individual" {
        if ctx.targets.len() == 1 {
            let nodes = ctx.tree.nodes_at_depth(ctx.targets[0], depth_of_effect);
            c.do_rendering(&nodes);
        } else {
            c.do_rendering(ctx.targets);
        }
    } else if ctx.targets.len() == 1 {
        for n in ctx.tree.nodes_at_depth(ctx.targets[0], depth_of_effect) {
            let nodes = ctx.tree.nodes_at_depth(n, depth_of_effect);
            c.do_rendering(&nodes);
        }
    } else {
        for &n in ctx.targets {
            let nodes = ctx.tree.nodes_at_depth(n, depth_of_effect);
            c.do_rendering(&nodes);
        }
    }
    let mut out = c.out.restrict(TimeSpan::ZERO, ctx.time_span);
    out.map.retain(|_, v| !v.is_empty());
    out
}

fn visibility(p: &Params, t: &TargetInfo, v: &mut Visibility) {
    let is_static = p.enum_name("ColorHandling") == "StaticColor";
    v.set("StaticColor", is_static);
    v.set("ColorGradient", !is_static);
    v.set("DefaultLevel", p.boolean("EnableDefaultLevel"));
    // UpdateTargetingAttributes
    let depth = t.determine_depth();
    v.set("TargetNodeHandling", t.targets.len() > 1 || depth > 2);
    v.set("DepthOfEffect", depth > 2);
}

/// `TargetNodesChanged`, past the colour check `set_targets` runs. `UpdateTargetingAttributes` adds nothing here.
fn targets_changed(p: &mut Params, t: &TargetInfo) {
    if t.targets.is_empty() {
        return;
    }
    force_group_when_shallow(p, t);
    if t.targets.len() > 1 {
        p.set("DepthOfEffect", ParamValue::Int(0));
    } else {
        reset_depth_beyond_first_target(p, t);
    }
}

/// `depth < 3 && TargetNodeHandling == Individual`, from `UpdateTargetingAttributes`.
fn force_group_when_shallow(p: &mut Params, t: &TargetInfo) {
    if t.determine_depth() < 3 {
        p.set("TargetNodeHandling", ParamValue::Enum("Group".to_string()));
    }
}

fn param_changed_ctx(c: &ParamChange, p: &mut Params) {
    match c.name {
        // `ModuleData` calls InitAllAttributes, so a saved Individual on a shallow target loads as Group. It leaves
        // DepthOfEffect as saved: only TargetNodesChanged resets that.
        MODULE_DATA => force_group_when_shallow(p, c.info),
        // The `StaticColor` getter calls CheckForInvalidColorData, so a colour the targets can't show doesn't last. The
        // property grid only reads it while it is visible, so a gradient mode keeps the gradient it was given.
        _ if p.enum_name("ColorHandling") == "StaticColor" => validate(p, &c.info.valid_colors()),
        _ => {}
    }
}

/// `CheckForInvalidColorData`.
fn validate(p: &mut Params, valid: &[Color]) {
    if valid.is_empty() {
        return;
    }
    let in_gradient = p.gradient("ColorGradient").colors_in_gradient();
    if !valid.contains(&p.color("StaticColor")) || !in_gradient.iter().all(|c| valid.contains(c)) {
        p.set("ColorGradient", ParamValue::Gradient(ColorGradient::solid(valid[0])));
        p.set("StaticColor", ParamValue::Color(valid[0]));
    }
}

fn level_curve(level: f64) -> Curve {
    Curve::from_points(vec![(0.0, level), (100.0, level)])
}

impl Chase<'_> {
    /// `result.FirstOrDefault().Value`: the intents of the first element the render produced.
    fn first_element_intents(&self, target: NodeId, result: &EffectIntents) -> Vec<IntentNode> {
        for leaf in self.ctx.tree.distinct_leaves(target) {
            if let Some(e) = self.ctx.tree.node(leaf).element_id {
                if let Some(v) = result.nodes_for(&e) {
                    return v.clone();
                }
            }
        }
        Vec::new()
    }

    fn add_start(&mut self, target: NodeId, n: &IntentNode, g: Option<&ColorGradient>) {
        let r = pulse_renderer::starting_static_pulse(self.ctx.tree, target, Some(n), self.has_discrete, g);
        self.out.add(r);
    }

    fn add_end(&mut self, target: NodeId, n: &IntentNode, g: Option<&ColorGradient>) {
        let r = pulse_renderer::extended_static_pulse(self.ctx.tree, target, Some(n), self.ctx.time_span, self.has_discrete, g);
        self.out.add(r);
    }

    /// The Extend start / add / extend end sequence shared by most colour modes, applied to every
    /// intent of the first element (`each`) or just the first / last one (`single`).
    fn add_with_extensions(&mut self, target: NodeId, result: EffectIntents, g_start: Option<&ColorGradient>, g_end: Option<&ColorGradient>, single: bool) {
        let first = self.first_element_intents(target, &result);
        if self.extend_start && !result.is_empty() {
            if single {
                if let Some(n) = first.first() {
                    self.add_start(target, n, g_start);
                }
            } else {
                for n in &first {
                    self.add_start(target, n, g_start);
                }
            }
        }
        let had = !result.is_empty();
        self.out.add(result);
        if self.extend_end && had {
            if single {
                if let Some(n) = first.last() {
                    self.add_end(target, n, g_end);
                }
            } else {
                for n in &first {
                    self.add_end(target, n, g_end);
                }
            }
        }
    }

    fn do_rendering(&mut self, render_nodes: &[NodeId]) {
        let ctx = self.ctx;
        let count = render_nodes.len();
        if count == 0 {
            return;
        }
        let time_span = ctx.time_span;

        if self.enable_default_level {
            let level = self.default_level * 100.0;
            for (i, &target) in render_nodes.iter().enumerate() {
                if ctx.cancelled() {
                    return;
                }
                let discrete_colors = ctx.tree.is_discrete(target);
                let tree = ctx.tree;
                let hd = self.has_discrete;
                match self.color_handling {
                    ColorHandling::GradientForEachPulse => {
                        let r = pulse_renderer::render_node(tree, target, &level_curve(level), &self.static_gradient, time_span, hd, true);
                        self.out.add(r);
                    }
                    ColorHandling::GradientThroughWholeEffect => {
                        let r = pulse_renderer::render_node(tree, target, &level_curve(level), &self.gradient, time_span, hd, true);
                        self.out.add(r);
                    }
                    ColorHandling::StaticColor => {
                        let r = pulse_renderer::render_node(tree, target, &level_curve(level), &self.static_gradient, time_span, hd, true);
                        self.out.add(r);
                    }
                    ColorHandling::ColorAcrossItems => {
                        let pos = i as f64 / count as f64;
                        if discrete_colors {
                            for (color, proportion) in self.gradient.get_discrete_colors_and_proportions_at(pos) {
                                if ctx.cancelled() {
                                    return;
                                }
                                let v = level * proportion as f64;
                                let r = pulse_renderer::render_node(tree, target, &level_curve(v), &ColorGradient::solid(color), time_span, hd, true);
                                self.out.add(r);
                            }
                        } else {
                            let g = ColorGradient::solid(self.gradient.get_color_at(pos));
                            let r = pulse_renderer::render_node(tree, target, &level_curve(level), &g, time_span, hd, true);
                            self.out.add(r);
                        }
                    }
                }
            }
        }

        let total_ms = time_span.total_ms();
        let mut chase_time = TimeSpan::from_ms(total_ms - self.pulse_overlap as f64);
        if chase_time.total_ms() <= 0.0 {
            chase_time = TimeSpan::from_ms(1.0);
        }
        let mut sample_ms = chase_time.total_ms() / count as f64 / 2.0;
        if sample_ms < 0.25 {
            sample_ms = 0.25;
        } else if sample_ms > 2.0 {
            sample_ms = 2.0;
        }
        let increment = TimeSpan::from_ticks((sample_ms * 10_000.0) as i64).max(TimeSpan::from_ticks(1));

        let (Some(&(first_x, _)), Some(&(last_x, _))) = (self.chase_movement.points.first(), self.chase_movement.points.last()) else {
            return;
        };
        let mut start_time = TimeSpan::ZERO;
        let mut end_time = time_span;
        if first_x > 0.0 {
            start_time = TimeSpan::from_ticks((time_span.ticks() as f64 * first_x / 100.0).round_ties_even() as i64);
        }
        if last_x < 100.0 {
            end_time = TimeSpan::from_ticks((time_span.ticks() as f64 * last_x / 100.0).round_ties_even() as i64);
        }

        let mut last_node: Option<NodeId> = None;
        let mut last_start = TimeSpan::ZERO;
        let mut current = start_time;
        while current <= end_time {
            if ctx.cancelled() {
                return;
            }
            let pct = (current.ticks() as f64 / chase_time.ticks() as f64) * 100.0;
            let pos = self.chase_movement.get_value(pct);
            let mut idx = ((pos / 100.0) * count as f64) as i32;
            if idx == count as i32 && idx > 0 {
                idx -= 1;
            }
            if idx >= count as i32 || idx < 0 {
                current += increment;
                continue;
            }
            let node = render_nodes[idx as usize];
            if Some(node) == last_node {
                current += increment;
                continue;
            }
            if let Some(prev) = last_node {
                let duration = current - last_start + TimeSpan::from_ms(self.pulse_overlap as f64);
                self.generate_pulse(prev, last_start, duration, pos);
            }
            last_node = Some(node);
            last_start = current;
            if pct >= 100.0 {
                break;
            }
            current += increment;
        }
        if let Some(n) = last_node {
            let last_pos = self.chase_movement.get_value(last_x);
            self.generate_pulse(n, last_start, end_time - last_start, last_pos);
        }
    }

    fn generate_pulse(&mut self, target: NodeId, start: TimeSpan, duration: TimeSpan, pos: f64) {
        let ctx = self.ctx;
        let tree = ctx.tree;
        let hd = self.has_discrete;
        match self.color_handling {
            ColorHandling::GradientForEachPulse => {
                let mut r = pulse_renderer::render_node(tree, target, &self.pulse_curve, &self.gradient, duration, hd, false);
                r.offset_all(start);
                self.add_with_extensions(target, r, None, None, false);
            }
            ColorHandling::GradientThroughWholeEffect => {
                let total = ctx.time_span.ticks() as f64;
                let mut sp = start.ticks() as f64 / total;
                let mut ep = (start + duration).ticks() as f64 / total;
                if sp < 0.0 {
                    sp = 0.0;
                }
                if ep > 1.0 {
                    ep = 1.0;
                }
                if hd {
                    let range = ep - sp;
                    if range <= 0.0 {
                        return;
                    }
                    let Some(cg) = self.gradient.sub_gradient_with_discrete_colors(sp, ep) else { return };
                    for color in cg.colors_in_gradient() {
                        let mut curve = self.pulse_curve.clone();
                        for pt in curve.points.iter_mut() {
                            let rel = sp + (pt.0 / 100.0) * range;
                            pt.1 *= self.gradient.get_proportion_of_color_at(rel, color);
                        }
                        let g = ColorGradient::solid(color);
                        let mut r = pulse_renderer::render_node(tree, target, &curve, &g, duration, hd, false);
                        r.offset_all(start);
                        let first = self.first_element_intents(target, &r);
                        if self.extend_start && !r.is_empty() {
                            for n in &first {
                                self.add_start(target, n, Some(&g));
                            }
                        }
                        let had = !r.is_empty();
                        if had {
                            self.out.add(r);
                        }
                        if self.extend_end && had {
                            for n in &first {
                                self.add_end(target, n, Some(&g));
                            }
                        }
                    }
                } else {
                    let Some(sub) = self.gradient.sub_gradient(sp, ep) else { return };
                    let mut r = pulse_renderer::render_node(tree, target, &self.pulse_curve, &sub, duration, hd, false);
                    r.offset_all(start);
                    let gs = self.gradient.sub_gradient(0.0, sp);
                    let ge = self.gradient.sub_gradient(ep, 1.0);
                    self.add_with_extensions(target, r, gs.as_ref(), ge.as_ref(), false);
                }
            }
            ColorHandling::StaticColor => {
                let mut r = pulse_renderer::render_node(tree, target, &self.pulse_curve, &self.static_gradient, duration, hd, false);
                r.offset_all(start);
                self.add_with_extensions(target, r, None, None, true);
            }
            ColorHandling::ColorAcrossItems => {
                if hd {
                    for (color, proportion) in self.gradient.get_discrete_colors_and_proportions_at(pos / 100.0) {
                        let mut curve = self.pulse_curve.clone();
                        for pt in curve.points.iter_mut() {
                            pt.1 *= proportion as f64;
                        }
                        let g = ColorGradient::solid(color);
                        let mut r = pulse_renderer::render_node(tree, target, &curve, &g, duration, hd, false);
                        r.offset_all(start);
                        let first = self.first_element_intents(target, &r);
                        if self.extend_start && !r.is_empty() {
                            for n in &first {
                                self.add_start(target, n, Some(&g));
                            }
                        }
                        let had = !r.is_empty();
                        if had {
                            self.out.add(r);
                        }
                        if self.extend_end && had {
                            for n in &first {
                                self.add_end(target, n, Some(&g));
                            }
                        }
                    }
                } else {
                    let g = ColorGradient::solid(self.gradient.get_color_at(pos / 100.0));
                    let mut r = pulse_renderer::render_node(tree, target, &self.pulse_curve, &g, duration, hd, false);
                    r.offset_all(start);
                    self.add_with_extensions(target, r, None, None, false);
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
    fn targets_decide_what_the_editor_offers() {
        let mut tree = ElementTree::new();
        let string = tree.add_string("S", 4);
        let second = tree.add_string("S2", 4);
        let deep = testing::deep_grid(&mut tree, "D", 2, 2, 3);
        let p = DESCRIPTOR.new_params();
        let shown = |t: &[NodeId], name: &str| {
            testing::visible_names(&DESCRIPTOR, &p, &testing::target_info(&tree, t)).contains(&name)
        };
        assert!(!shown(&[string], "TargetNodeHandling"));
        assert!(!shown(&[string], "DepthOfEffect"));
        assert!(shown(&[string, second], "TargetNodeHandling"));
        assert!(!shown(&[string, second], "DepthOfEffect"));
        assert!(shown(&[deep], "TargetNodeHandling"));
        assert!(shown(&[deep], "DepthOfEffect"));
        // No targets: DetermineDepth is int.MaxValue.
        assert!(shown(&[], "TargetNodeHandling"));
        assert!(shown(&[], "DepthOfEffect"));
    }

    #[test]
    fn assigning_targets_normalises_but_loading_keeps_the_saved_depth() {
        let mut tree = ElementTree::new();
        let string = tree.add_string("S", 4);
        let second = tree.add_string("S2", 4);
        let edited = || {
            let mut p = DESCRIPTOR.new_params();
            p.set("TargetNodeHandling", ParamValue::Enum("Individual".into()));
            p.set("DepthOfEffect", ParamValue::Int(5));
            p
        };

        let mut assigned = edited();
        DESCRIPTOR.set_targets(&mut assigned, &testing::target_info(&tree, &[string]));
        assert_eq!(assigned.enum_name("TargetNodeHandling"), "Group");
        assert_eq!(assigned.int("DepthOfEffect"), 0);

        let mut loaded = edited();
        DESCRIPTOR.load_data(&mut loaded, &testing::target_info(&tree, &[string]));
        assert_eq!(loaded.enum_name("TargetNodeHandling"), "Group");
        assert_eq!(loaded.int("DepthOfEffect"), 5);

        let deep = testing::deep_grid(&mut tree, "D", 2, 2, 3);
        let mut loaded = edited();
        DESCRIPTOR.load_data(&mut loaded, &testing::target_info(&tree, &[deep]));
        assert_eq!(loaded.enum_name("TargetNodeHandling"), "Individual");

        let mut many = edited();
        p_set_depth(&mut many, 1);
        DESCRIPTOR.set_targets(&mut many, &testing::target_info(&tree, &[deep, second]));
        assert_eq!(many.int("DepthOfEffect"), 0);
    }

    fn p_set_depth(p: &mut Params, depth: i32) {
        p.set("DepthOfEffect", ParamValue::Int(depth));
    }

    #[test]
    fn setting_a_colour_the_targets_cannot_show_corrects_it() {
        let mut tree = ElementTree::new();
        let s = tree.add_string("S", 2);
        testing::make_discrete(&mut tree, s, Color::RED);
        let mut p = DESCRIPTOR.new_params();
        let targets = [s];
        let t = testing::target_info(&tree, &targets);
        DESCRIPTOR.set_targets(&mut p, &t);
        assert_eq!(p.color("StaticColor"), Color::RED);
        DESCRIPTOR.set_param(&mut p, "StaticColor", ParamValue::Color(Color::BLUE), &t);
        assert_eq!(p.color("StaticColor"), Color::RED);
    }

    #[test]
    fn pulses_walk_across_elements_in_order() {
        let mut tree = ElementTree::new();
        let s = tree.add_string("S", 4);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[s], 4000, |_| {});
        let leaves = tree.distinct_leaves(s);
        let starts: Vec<f64> = leaves
            .iter()
            .map(|l| out.nodes_for(&tree.node(*l).element_id.unwrap()).unwrap()[0].start.total_ms())
            .collect();
        for w in starts.windows(2) {
            assert!(w[1] > w[0], "{starts:?}");
        }
        assert!(starts[0].abs() < 1.0);
        assert!((starts[1] - 1000.0).abs() < 5.0, "{starts:?}");
    }

    #[test]
    fn visibility_follows_colour_handling() {
        let mut p = DESCRIPTOR.new_params();
        p.set("ColorHandling", ParamValue::Enum("GradientForEachPulse".into()));
        let mut v = Visibility::default();
        visibility(&p, &TargetInfo::detached(), &mut v);
        assert!(v.overrides.contains(&("StaticColor", false)));
        assert!(v.overrides.contains(&("ColorGradient", true)));
    }
}
