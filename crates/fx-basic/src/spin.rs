//! Port of VixenModules.Effect.Spin.

use crate::common::*;
use crate::generated::spin as gen;
use vixen_effects::prelude::*;
use vixen_effects::pulse_renderer;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    validate: Some(validate_static_and_gradient),
    targets_changed: Some(targets_changed),
    param_changed_ctx: Some(param_changed_ctx),
    ported: true,
    ..gen::BASE
};

#[derive(Copy, Clone, PartialEq)]
enum Handling {
    Static,
    Whole,
    PerPulse,
    Across,
}

struct Spin<'a> {
    ctx: &'a RenderCtx<'a>,
    has_discrete: bool,
    handling: Handling,
    static_gradient: ColorGradient,
    gradient: ColorGradient,
    pulse_curve: Curve,
    speed_format: String,
    pulse_format: String,
    revolution_count: f64,
    revolution_frequency: f64,
    revolution_time: i32,
    pulse_overlap: i32,
    pulse_time: i32,
    pulse_percentage: i32,
    default_level: f64,
    enable_default_level: bool,
    reverse: bool,
    out: EffectIntents,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let handling = match p.enum_name("ColorHandling") {
        "GradientThroughWholeEffect" => Handling::Whole,
        "GradientForEachPulse" => Handling::PerPulse,
        "ColorAcrossItems" => Handling::Across,
        _ => Handling::Static,
    };
    let mut s = Spin {
        ctx,
        has_discrete: ctx.has_discrete_colors(),
        handling,
        static_gradient: ColorGradient::solid(p.color("StaticColor")),
        gradient: p.gradient("ColorGradient"),
        pulse_curve: p.curve("PulseCurve"),
        speed_format: p.enum_name("SpeedFormat").to_string(),
        pulse_format: p.enum_name("PulseLengthFormat").to_string(),
        revolution_count: p.double("RevolutionCount"),
        revolution_frequency: p.double("RevolutionFrequency"),
        revolution_time: p.int("RevolutionTime"),
        pulse_overlap: p.int("PulseOverlap"),
        pulse_time: p.int("PulseTime"),
        pulse_percentage: p.int("PulsePercentage"),
        default_level: p.double("DefaultLevel"),
        enable_default_level: p.boolean("EnableDefaultLevel"),
        reverse: p.boolean("ReverseSpin"),
        out: EffectIntents::new(),
    };
    for set in target_sets(ctx, p.enum_name("TargetNodeHandling") == "Individual", p.int("DepthOfEffect")) {
        if !s.do_rendering(&set) {
            break;
        }
    }
    let mut out = s.out.restrict(TimeSpan::ZERO, ctx.time_span);
    out.map.retain(|_, v| !v.is_empty());
    out
}

/// `TargetNodesChanged`, past the colour check `set_targets` runs.
fn targets_changed(p: &mut Params, t: &TargetInfo) {
    if t.targets.len() > 1 {
        p.set("DepthOfEffect", ParamValue::Int(0));
    } else {
        reset_depth_beyond_first_target(p, t);
    }
    force_group_when_shallow(p, t);
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
        _ if p.enum_name("ColorHandling") == "StaticColor" => validate_static_and_gradient(p, &c.info.valid_colors()),
        _ => {}
    }
}

fn visibility(p: &Params, t: &TargetInfo, v: &mut Visibility) {
    let is_static = p.enum_name("ColorHandling") == "StaticColor";
    v.set("StaticColor", is_static);
    v.set("ColorGradient", !is_static);
    v.set("DefaultLevel", p.boolean("EnableDefaultLevel"));
    let sf = p.enum_name("SpeedFormat");
    v.set("RevolutionCount", sf == "RevolutionCount");
    v.set("RevolutionTime", sf == "FixedTime");
    v.set("RevolutionFrequency", sf == "RevolutionFrequency");
    let pf = p.enum_name("PulseLengthFormat");
    v.set("PulseTime", pf == "FixedTime");
    v.set("PulseOverlap", pf == "FixedTime");
    v.set("PulsePercentage", pf == "PercentageOfRevolution");
    // UpdateTargetingAttributes
    let depth = t.determine_depth();
    v.set("TargetNodeHandling", t.targets.len() > 1 || depth > 2);
    v.set("DepthOfEffect", depth > 2);
}

impl Spin<'_> {
    /// `DoRendering`. Returns false when cancelled.
    fn do_rendering(&mut self, nodes: &[NodeId]) -> bool {
        let ctx = self.ctx;
        let tree = ctx.tree;
        let hd = self.has_discrete;
        let count = nodes.len();
        if count == 0 {
            return true;
        }
        let ts = ctx.time_span;

        if self.enable_default_level {
            let level = self.default_level * 100.0;
            for (i, &target) in nodes.iter().enumerate() {
                if ctx.cancelled() {
                    return false;
                }
                let discrete = tree.is_discrete(target);
                match self.handling {
                    Handling::PerPulse | Handling::Static => {
                        let r = pulse_renderer::render_node(tree, target, &level_curve(level), &self.static_gradient, ts, hd, true);
                        self.out.add(r);
                    }
                    Handling::Whole => {
                        let r = pulse_renderer::render_node(tree, target, &level_curve(level), &self.gradient, ts, hd, true);
                        self.out.add(r);
                    }
                    Handling::Across => {
                        let pos = i as f64 / count as f64;
                        if discrete {
                            for (c, prop) in self.gradient.get_discrete_colors_and_proportions_at(pos) {
                                let v = level * prop as f64;
                                let r = pulse_renderer::render_node(tree, target, &level_curve(v), &ColorGradient::solid(c), ts, hd, true);
                                self.out.add(r);
                            }
                        } else {
                            let g = ColorGradient::solid(self.gradient.get_color_at(pos));
                            let r = pulse_renderer::render_node(tree, target, &level_curve(level), &g, ts, hd, true);
                            self.out.add(r);
                        }
                    }
                }
            }
        }

        let total_ms = ts.total_ms();
        let mut pulse_constant = 0.0;
        let mut pulse_fractional = 0.0;
        match self.pulse_format.as_str() {
            "FixedTime" => {
                if self.pulse_overlap == 0 {
                    pulse_constant = self.pulse_time as f64;
                } else {
                    pulse_constant = total_ms * (1.0 / count as f64) + self.pulse_overlap as f64;
                    if pulse_constant >= total_ms {
                        pulse_constant = total_ms - 1.0;
                    }
                }
            }
            "PercentageOfRevolution" => pulse_fractional = self.pulse_percentage as f64 / 100.0,
            "EvenlyDistributedAcrossSegments" => pulse_fractional = 1.0 / count as f64,
            _ => {}
        }
        let interpolation_offset = 1.0 / count as f64;

        let mut rev_ms = 0.0;
        match self.speed_format.as_str() {
            "RevolutionCount" => {
                let mut t = self.revolution_count + pulse_fractional - interpolation_offset;
                if t <= 0.0 {
                    t = if self.revolution_count > 0.0 { self.revolution_count } else { 1.0 };
                }
                rev_ms = (total_ms - pulse_constant) / t;
            }
            "RevolutionFrequency" => rev_ms = (1.0 / self.revolution_frequency) * 1000.0,
            "FixedTime" => rev_ms = self.revolution_time as f64,
            _ => {}
        }
        let pulse_ms = pulse_constant + rev_ms * pulse_fractional;
        let rev_span = TimeSpan::from_ms(rev_ms);
        let pulse_span = TimeSpan::from_ms(pulse_ms);
        // A zero-length revolution is a divide by zero in C#: the effect stays dark.
        if rev_span.ticks() <= 0 || !rev_ms.is_finite() {
            return true;
        }

        let movement = if self.reverse {
            Curve::from_points(vec![(0.0, 100.0), (100.0, 0.0)])
        } else {
            Curve::from_points(vec![(0.0, 0.0), (100.0, 100.0)])
        };

        let mut sample_ms = rev_span.total_ms() / count as f64 / 2.0;
        if sample_ms < 0.25 {
            sample_ms = 0.25;
        } else if sample_ms > 2.0 {
            sample_ms = 2.0;
        }
        let increment = TimeSpan::from_ticks((sample_ms * 10_000.0) as i64).max(TimeSpan::from_ticks(1));

        let mut last: Option<NodeId> = None;
        let mut current = TimeSpan::ZERO;
        let limit = ts - pulse_span + increment;
        while current <= limit {
            if ctx.cancelled() {
                return false;
            }
            let pct = (current.ticks() % rev_span.ticks()) as f64 / rev_span.ticks() as f64 * 100.0;
            let pos = movement.get_value(pct);
            let mut idx = ((pos / 100.0) * count as f64) as i32;
            if idx == count as i32 {
                idx -= 1;
            }
            if idx >= count as i32 || idx < 0 {
                current += increment;
                continue;
            }
            let node = nodes[idx as usize];
            if Some(node) == last {
                current += increment;
                continue;
            }
            let discrete = tree.is_discrete(node);
            match self.handling {
                Handling::PerPulse => {
                    let mut r = pulse_renderer::render_node(tree, node, &self.pulse_curve, &self.gradient, pulse_span, hd, false);
                    r.offset_all(current);
                    self.out.add(r);
                }
                Handling::Whole => {
                    let start_pos = current.ticks() as f64 / ts.ticks() as f64;
                    let mut end_pos = 1.0;
                    if ts - current >= pulse_span {
                        end_pos = (current + pulse_span).ticks() as f64 / ts.ticks() as f64;
                    }
                    if discrete {
                        let range = end_pos - start_pos;
                        if range > 0.0 {
                            if let Some(cg) = self.gradient.sub_gradient_with_discrete_colors(start_pos, end_pos) {
                                for color in cg.colors_in_gradient() {
                                    if ctx.cancelled() {
                                        return false;
                                    }
                                    let curve = scaled_curve(&self.pulse_curve, |x, y| {
                                        y * self.gradient.get_proportion_of_color_at(start_pos + (x / 100.0) * range, color)
                                    });
                                    let mut r = pulse_renderer::render_node(tree, node, &curve, &ColorGradient::solid(color), pulse_span, hd, false);
                                    r.offset_all(current);
                                    self.out.add(r);
                                }
                            }
                        }
                    } else if let Some(sub) = self.gradient.sub_gradient(start_pos, end_pos) {
                        let mut r = pulse_renderer::render_node(tree, node, &self.pulse_curve, &sub, pulse_span, hd, false);
                        r.offset_all(current);
                        self.out.add(r);
                    }
                }
                Handling::Static => {
                    let mut r = pulse_renderer::render_node(tree, node, &self.pulse_curve, &self.static_gradient, pulse_span, hd, false);
                    r.offset_all(current);
                    self.out.add(r);
                }
                Handling::Across => {
                    if discrete {
                        for (c, prop) in self.gradient.get_discrete_colors_and_proportions_at(pos / 100.0) {
                            if ctx.cancelled() {
                                return false;
                            }
                            let curve = scaled_curve(&self.pulse_curve, |_, y| y * prop as f64);
                            let mut r = pulse_renderer::render_node(tree, node, &curve, &ColorGradient::solid(c), pulse_span, hd, false);
                            r.offset_all(current);
                            self.out.add(r);
                        }
                    } else {
                        let g = ColorGradient::solid(self.gradient.get_color_at(pos / 100.0));
                        let mut r = pulse_renderer::render_node(tree, node, &self.pulse_curve, &g, pulse_span, hd, false);
                        r.offset_all(current);
                        self.out.add(r);
                    }
                }
            }
            last = Some(node);
            current += increment;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_core::StateValue;
    use vixen_effects::testing;

    #[test]
    fn smoke() {
        testing::smoke_test(&DESCRIPTOR);
        testing::assert_produces_light(&DESCRIPTOR);
    }

    #[test]
    fn targets_decide_handling_and_depth() {
        let mut tree = ElementTree::new();
        let string = tree.add_string("S", 4);
        let second = tree.add_string("S2", 4);
        let deep = testing::deep_grid(&mut tree, "D", 2, 2, 3);
        let mut p = DESCRIPTOR.new_params();
        let names = testing::visible_names(&DESCRIPTOR, &p, &testing::target_info(&tree, &[string]));
        assert!(!names.contains(&"TargetNodeHandling") && !names.contains(&"DepthOfEffect"));
        let names = testing::visible_names(&DESCRIPTOR, &p, &testing::target_info(&tree, &[string, second]));
        assert!(names.contains(&"TargetNodeHandling") && !names.contains(&"DepthOfEffect"));
        let names = testing::visible_names(&DESCRIPTOR, &p, &testing::target_info(&tree, &[deep]));
        assert!(names.contains(&"TargetNodeHandling") && names.contains(&"DepthOfEffect"));

        p.set("TargetNodeHandling", ParamValue::Enum("Individual".into()));
        p.set("DepthOfEffect", ParamValue::Int(5));
        let mut loaded = p.clone();
        DESCRIPTOR.load_data(&mut loaded, &testing::target_info(&tree, &[string]));
        assert_eq!(loaded.enum_name("TargetNodeHandling"), "Group");
        assert_eq!(loaded.int("DepthOfEffect"), 5);
        DESCRIPTOR.set_targets(&mut p, &testing::target_info(&tree, &[string]));
        assert_eq!(p.enum_name("TargetNodeHandling"), "Group");
        assert_eq!(p.int("DepthOfEffect"), 0);
    }

    #[test]
    fn first_pulse_is_on_the_first_element_and_reverse_flips_it() {
        let mut tree = ElementTree::new();
        let s = tree.add_string("S", 4);
        let leaves = tree.distinct_leaves(s);
        let first_start = |out: &EffectIntents, i: usize| {
            out.nodes_for(&tree.node(leaves[i]).element_id.unwrap()).map(|n| n[0].start.total_ms())
        };
        let fwd = testing::render_with(&DESCRIPTOR, &tree, &[s], 3000, |_| {});
        assert_eq!(first_start(&fwd, 0), Some(0.0));
        assert!(first_start(&fwd, 3).unwrap() > 0.0);
        let rev = testing::render_with(&DESCRIPTOR, &tree, &[s], 3000, |p| {
            p.set("ReverseSpin", ParamValue::Bool(true));
        });
        assert_eq!(first_start(&rev, 3), Some(0.0));
        assert!(first_start(&rev, 0).unwrap() > 0.0);
    }

    #[test]
    fn fixed_time_revolution_repeats() {
        let mut tree = ElementTree::new();
        let s = tree.add_string("S", 2);
        let leaves = tree.distinct_leaves(s);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[s], 2000, |p| {
            p.set("SpeedFormat", ParamValue::Enum("FixedTime".into()));
            p.set("RevolutionTime", ParamValue::Int(500));
            p.set("PulseLengthFormat", ParamValue::Enum("FixedTime".into()));
            p.set("PulseTime", ParamValue::Int(100));
            p.set("PulseCurve", ParamValue::Curve(Curve::flat100()));
        });
        let nodes = out.nodes_for(&tree.node(leaves[0]).element_id.unwrap()).unwrap();
        assert!(nodes.len() >= 3, "{}", nodes.len());
        match nodes[0].intent.state_at(TimeSpan::from_ms_i(10)) {
            StateValue::Lighting(v) => assert_eq!(v.intensity, 1.0),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn visibility_follows_speed_format() {
        let mut p = DESCRIPTOR.new_params();
        p.set("SpeedFormat", ParamValue::Enum("FixedTime".into()));
        let mut v = Visibility::default();
        visibility(&p, &TargetInfo::detached(), &mut v);
        assert!(v.overrides.contains(&("RevolutionTime", true)));
        assert!(v.overrides.contains(&("RevolutionCount", false)));
    }
}
