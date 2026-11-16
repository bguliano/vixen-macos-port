//! Port of VixenModules.Effect.Twinkle.

use crate::common::validate_static_and_gradient;
use crate::generated::twinkle as gen;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    validate: Some(validate_static_and_gradient),
    targets_changed: Some(reset_depth_beyond_first_target),
    param_changed_ctx: Some(param_changed_ctx),
    ported: true,
    ..gen::BASE
};

/// The `MaximumLevel` setter ignores a value outside 0..1.
fn param_changed_ctx(c: &ParamChange, p: &mut Params) {
    if c.name == "MaximumLevel" && !(0.0..=1.0).contains(&p.double("MaximumLevel")) {
        p.set("MaximumLevel", c.old.clone());
    }
}

#[derive(Copy, Clone, PartialEq)]
enum Handling {
    Static,
    Whole,
    PerPulse,
    Across,
}

#[derive(Copy, Clone)]
struct ColorValue {
    c: Color,
    i: f64,
}

const NO_VALUE: ColorValue = ColorValue { c: Color::EMPTY, i: 0.0 };

/// `IndividualTwinkleDetails`.
struct TwinkleDetails {
    start: TimeSpan,
    duration: TimeSpan,
    curve_points: [f64; 3],
}

struct Twinkle<'a> {
    ctx: &'a RenderCtx<'a>,
    has_discrete: bool,
    handling: Handling,
    static_gradient: ColorGradient,
    gradient: ColorGradient,
    min_level: f64,
    max_level: f64,
    level_variation: i32,
    avg_pulse: i32,
    pulse_variation: i32,
    avg_coverage: i32,
    rand: Rand,
    /// `_colorValueSet`, in insertion order.
    sets: Vec<(Color, Vec<ColorValue>)>,
    frames: usize,
}

fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    let is_static = p.enum_name("ColorHandling") == "StaticColor";
    v.set("StaticColor", is_static);
    v.set("ColorGradient", !is_static);
    v.set("DepthOfEffect", p.boolean("IndividualElements"));
}

fn distinct(v: Vec<NodeId>) -> Vec<NodeId> {
    let mut seen = std::collections::HashSet::new();
    v.into_iter().filter(|n| seen.insert(*n)).collect()
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let mut out = EffectIntents::new();
    if ctx.targets.is_empty() || ctx.frame_ms <= 0 {
        return out;
    }
    let handling = match p.enum_name("ColorHandling") {
        "GradientThroughWholeEffect" => Handling::Whole,
        "GradientForEachPulse" => Handling::PerPulse,
        "ColorAcrossItems" => Handling::Across,
        _ => Handling::Static,
    };
    let individual = p.boolean("IndividualElements");
    let mut t = Twinkle {
        ctx,
        has_discrete: ctx.has_discrete_colors(),
        handling,
        static_gradient: ColorGradient::solid(p.color("StaticColor")),
        gradient: p.gradient("ColorGradient"),
        min_level: p.double("MinimumLevel"),
        max_level: p.double("MaximumLevel"),
        level_variation: p.int("LevelVariation"),
        avg_pulse: p.int("AveragePulseTime"),
        pulse_variation: p.int("PulseTimeVariation"),
        avg_coverage: p.int("AverageCoverage"),
        rand: Rand::default(),
        sets: Vec::new(),
        frames: (ctx.time_span.total_ms() / ctx.frame_ms as f64) as usize,
    };

    // GetNodesToRenderOn
    let depth = p.int("DepthOfEffect");
    let leaves = || distinct(ctx.targets.iter().flat_map(|&n| ctx.tree.leaves(n)).collect::<Vec<_>>());
    let mut nodes: Vec<NodeId>;
    if depth == 0 || !individual {
        nodes = leaves();
    } else {
        nodes = ctx.targets.to_vec();
        for _ in 0..depth {
            nodes = distinct(nodes.iter().flat_map(|&n| ctx.tree.node(n).children.iter().copied()).collect());
        }
    }
    if nodes.is_empty() {
        nodes = leaves();
    }

    let shared = if !individual { t.generate_twinkles() } else { None };
    if !individual && shared.is_none() {
        return out;
    }
    let total = nodes.len();
    for (i, &node) in nodes.iter().enumerate() {
        if ctx.cancelled() {
            return EffectIntents::new();
        }
        let discrete = t.has_discrete && ctx.tree.is_discrete(node);
        let intents = match &shared {
            Some(tw) => t.render_element(node, i as f64 / total as f64, discrete, tw),
            None => match t.generate_twinkles() {
                Some(tw) => t.render_element(node, i as f64 / total as f64, discrete, &tw),
                None => return EffectIntents::new(),
            },
        };
        // `StaticArrayIntent` refuses an empty array, so an effect shorter than one frame stops here. The nodes already
        // rendered stand, and this node's minimum-level pulse goes with it.
        let Some(intents) = intents else { return out };
        out.add(intents);
    }
    out
}

fn scale_all(curve: &Curve, f: impl Fn(f64, f64) -> f64) -> Curve {
    crate::common::scaled_curve(curve, f)
}

impl Twinkle<'_> {
    /// `GenerateTwinkleData`. None where the C# throws (a non-positive mean).
    fn generate_twinkles(&mut self) -> Option<Vec<TwinkleDetails>> {
        let ts = self.ctx.time_span;
        let mut result = Vec::new();
        let coverage = (if self.avg_coverage <= 0 { 1.0 } else { self.avg_coverage as f64 }) / 100.0;
        let mean = self.avg_pulse as f64 / coverage / 2.0;
        let max_between = self.avg_pulse as f64 / coverage;
        let max_var = ((self.pulse_variation as f64 / 100.0) * self.avg_pulse as f64) as i32;
        // RandomGenerator.GetExponential(mean) throws for mean <= 0.
        if mean <= 0.0 {
            return None;
        }
        let mut current = TimeSpan::ZERO;
        while current < ts {
            // The C# draws from a static Marsaglia generator; a per-render generator gives the same distribution.
            let u = 1.0 - self.rand.next_double();
            let mut next = mean * -u.ln();
            if next > max_between {
                next = max_between;
            }
            current += TimeSpan::from_ms(next);
            if current >= ts {
                break;
            }
            let dur_ms = self.rand.next_range(self.avg_pulse - max_var, self.avg_pulse + max_var + 1);
            let mut duration = TimeSpan::from_ms(dur_ms as f64);
            if current + duration > ts {
                if (ts - current).total_ms() >= (self.avg_pulse - max_var) as f64 {
                    duration = ts - current;
                } else {
                    break;
                }
            }
            let min_level = self.min_level;
            let max_level_variation = ((self.level_variation as f64 / 100.0) * (self.max_level - self.min_level)) as i32;
            let reduction = self.rand.next_range(0, max_level_variation);
            let max_level = self.max_level - reduction as f64;
            result.push(TwinkleDetails {
                start: current,
                duration,
                curve_points: [min_level * 100.0, max_level * 100.0, min_level * 100.0],
            });
        }
        Some(result)
    }

    fn min_pulse(&self, node: NodeId, value: f64, g: &ColorGradient, result: &mut EffectIntents) {
        let curve = crate::common::level_curve(value);
        let ts = self.ctx.time_span;
        result.add(vixen_effects::pulse_renderer::render_node(self.ctx.tree, node, &curve, g, ts, self.has_discrete, false));
    }

    fn render_element(&mut self, node: NodeId, position: f64, discrete: bool, twinkles: &[TwinkleDetails]) -> Option<EffectIntents> {
        let ctx = self.ctx;
        let mut result = EffectIntents::new();

        // The flat minimum level across the whole effect.
        if self.min_level > 0.0 {
            let min_v = self.min_level * 100.0;
            match self.handling {
                Handling::PerPulse => {
                    if discrete {
                        for (c, prop) in self.gradient.get_discrete_colors_and_proportions_at(0.0) {
                            self.min_pulse(node, min_v * prop as f64, &ColorGradient::solid(c), &mut result);
                        }
                    } else {
                        let g = ColorGradient::solid(self.gradient.get_color_at(0.0));
                        self.min_pulse(node, min_v, &g, &mut result);
                    }
                }
                Handling::Whole => self.min_pulse(node, min_v, &self.gradient, &mut result),
                Handling::Static => self.min_pulse(node, min_v, &self.static_gradient, &mut result),
                Handling::Across => {
                    if discrete {
                        for (c, prop) in self.gradient.get_discrete_colors_and_proportions_at(position) {
                            self.min_pulse(node, min_v * prop as f64, &ColorGradient::solid(c), &mut result);
                        }
                    } else {
                        let g = ColorGradient::solid(self.gradient.get_color_at(position));
                        self.min_pulse(node, min_v, &g, &mut result);
                    }
                }
            }
        }

        self.sets.clear();
        if !ctx.tree.is_discrete(node) {
            self.sets.push((Color::EMPTY, vec![NO_VALUE; self.frames]));
        }

        let gradient = self.gradient.clone();
        let static_gradient = self.static_gradient.clone();
        for tw in twinkles {
            let mut curve = Curve::from_points(vec![(0.0, tw.curve_points[0]), (50.0, tw.curve_points[1]), (100.0, tw.curve_points[2])]);
            match self.handling {
                Handling::PerPulse => self.segment(tw.start, tw.duration, &curve, &gradient, node),
                Handling::Whole => {
                    let start_pos = tw.start.ticks() as f64 / ctx.time_span.ticks() as f64;
                    let end_pos = (tw.start + tw.duration).ticks() as f64 / ctx.time_span.ticks() as f64;
                    if discrete {
                        let range = end_pos - start_pos;
                        if range <= 0.0 {
                            continue;
                        }
                        let Some(cg) = gradient.sub_gradient_with_discrete_colors(start_pos, end_pos) else { continue };
                        for color in cg.colors_in_gradient() {
                            // The C# rescales the previous colour's curve, so proportions compound. Ported as written.
                            curve = scale_all(&curve, |x, y| y * gradient.get_proportion_of_color_at(start_pos + (x / 100.0) * range, color));
                            self.segment(tw.start, tw.duration, &curve, &ColorGradient::solid(color), node);
                        }
                    } else if let Some(sub) = gradient.sub_gradient(start_pos, end_pos) {
                        self.segment(tw.start, tw.duration, &curve, &sub, node);
                    }
                }
                Handling::Static => self.segment(tw.start, tw.duration, &curve, &static_gradient, node),
                Handling::Across => {
                    if discrete {
                        for (c, prop) in gradient.get_discrete_colors_and_proportions_at(position) {
                            // Compounds across colours, as in the C#.
                            curve = scale_all(&curve, |_, y| y * prop as f64);
                            self.segment(tw.start, tw.duration, &curve, &ColorGradient::solid(c), node);
                        }
                    } else {
                        let g = ColorGradient::solid(gradient.get_color_at(position));
                        self.segment(tw.start, tw.duration, &curve, &g, node);
                    }
                }
            }
        }

        let leaves: Vec<NodeId> = if ctx.tree.is_leaf(node) { vec![node] } else { ctx.tree.leaves(node) };
        let frame = ctx.frame_timespan();
        if self.frames == 0 && !self.sets.is_empty() {
            return None;
        }
        for (key, values) in &self.sets {
            let intent = if *key == Color::EMPTY {
                let vals: Vec<LightingValue> = values.iter().map(|v| LightingValue::new(v.c, v.i)).collect();
                Intent::new(IntentData::ArrayLighting(FrameArray::from_vec(vals, frame)), ctx.time_span)
            } else {
                let vals: Vec<DiscreteValue> = values.iter().map(|v| DiscreteValue::new(v.c, v.i)).collect();
                Intent::new(IntentData::ArrayDiscrete(FrameArray::from_vec(vals, frame)), ctx.time_span)
            };
            for &leaf in &leaves {
                if let Some(e) = ctx.tree.node(leaf).element_id {
                    result.add_intent_for_element(e, intent.clone(), TimeSpan::ZERO);
                }
            }
        }
        Some(result)
    }

    /// `RenderPulseSegment(start, duration, curve, gradient, node)`.
    fn segment(&mut self, start: TimeSpan, duration: TimeSpan, curve: &Curve, cg: &ColorGradient, node: NodeId) {
        let ctx = self.ctx;
        if self.has_discrete && ctx.tree.is_discrete(node) {
            let in_gradient = cg.colors_in_gradient();
            let mut colors: Vec<Color> = Vec::new();
            for c in ctx.tree.valid_colors(node, false) {
                if in_gradient.contains(&c) && !colors.contains(&c) {
                    colors.push(c);
                }
            }
            for color in colors {
                let idx = match self.sets.iter().position(|s| s.0 == color) {
                    Some(i) => i,
                    None => {
                        self.sets.push((color, vec![NO_VALUE; self.frames]));
                        self.sets.len() - 1
                    }
                };
                fill_segment(&mut self.sets[idx].1, ctx.frame_ms, start, duration, curve, cg, Some(color));
            }
        } else if let Some(s) = self.sets.iter_mut().find(|s| s.0 == Color::EMPTY) {
            fill_segment(&mut s.1, ctx.frame_ms, start, duration, curve, cg, None);
        }
    }
}

/// `RenderPulseSegment(values, ...)`: keeps the brighter of the existing and the new value per frame.
fn fill_segment(values: &mut [ColorValue], frame_ms: i32, start: TimeSpan, duration: TimeSpan, c: &Curve, cg: &ColorGradient, filter: Option<Color>) {
    let intervals = duration.total_ms() / frame_ms as f64;
    // `(int)startTime.TotalMilliseconds / FrameTime` is an integer division.
    let start_offset = (start.total_ms() as i32) / frame_ms;
    let end_interval = start_offset as f64 + intervals;
    // A zero-length segment samples 0/0 in C#; there is nothing to draw.
    if intervals <= 0.0 || start_offset < 0 {
        return;
    }
    let mut i = start_offset;
    while i as f64 <= end_interval {
        if i as usize >= values.len() {
            break;
        }
        let current = values[i as usize];
        let sample = (i - start_offset) as f64 / intervals;
        match filter {
            Some(color) => {
                let intensity = cg.get_proportion_of_color_at(sample, color) * c.get_value(sample * 100.0) / 100.0;
                if current.i <= intensity {
                    values[i as usize] = ColorValue { c: color, i: intensity };
                }
            }
            None => {
                let intensity = c.get_value(sample * 100.0) / 100.0;
                if current.i <= intensity {
                    values[i as usize] = ColorValue { c: cg.get_color_at(sample), i: intensity };
                }
            }
        }
        i += 1;
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
    fn targets_reset_a_depth_too_deep_and_the_level_setter_rejects_out_of_range() {
        let mut tree = ElementTree::new();
        let s = tree.add_string("S", 3);
        let targets = [s];
        let t = testing::target_info(&tree, &targets);
        let mut p = DESCRIPTOR.new_params();
        p.set("DepthOfEffect", ParamValue::Int(2));
        let mut loaded = p.clone();
        DESCRIPTOR.load_data(&mut loaded, &t);
        assert_eq!(loaded.int("DepthOfEffect"), 2);
        DESCRIPTOR.set_targets(&mut p, &t);
        assert_eq!(p.int("DepthOfEffect"), 0);

        let before = p.double("MaximumLevel");
        DESCRIPTOR.set_param(&mut p, "MaximumLevel", ParamValue::Double(1.5), &t);
        assert_eq!(p.double("MaximumLevel"), before);
        DESCRIPTOR.set_param(&mut p, "MaximumLevel", ParamValue::Double(0.5), &t);
        assert_eq!(p.double("MaximumLevel"), 0.5);
    }

    #[test]
    fn renders_one_frame_array_per_leaf() {
        let mut tree = ElementTree::new();
        let s = tree.add_string("S", 3);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[s], 2000, |_| {});
        assert_eq!(out.map.len(), 3);
        for nodes in out.map.values() {
            match &nodes[0].intent.data {
                IntentData::ArrayLighting(a) => {
                    assert_eq!(a.len, 40);
                    assert_eq!(a.frame, TimeSpan::from_ms_i(50));
                }
                o => panic!("{o:?}"),
            }
            assert_eq!(nodes[0].intent.span, TimeSpan::from_ms_i(2000));
        }
    }

    #[test]
    fn minimum_level_keeps_the_background_lit() {
        let mut tree = ElementTree::new();
        let e = tree.add_element("E");
        let out = testing::render_with(&DESCRIPTOR, &tree, &[e], 1000, |p| {
            p.set("MinimumLevel", ParamValue::Double(0.25));
            p.set("AverageCoverage", ParamValue::Int(0)); // 1 % coverage: almost no twinkles
        });
        let nodes = out.nodes_for(&tree.node(e).element_id.unwrap()).unwrap();
        match nodes[0].intent.state_at(TimeSpan::from_ms_i(500)) {
            StateValue::Lighting(v) => assert!((v.intensity - 0.25).abs() < 1e-9, "{v:?}"),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn twinkles_peak_at_the_maximum_level_and_never_exceed_it() {
        let mut tree = ElementTree::new();
        let e = tree.add_element("E");
        let out = testing::render_with(&DESCRIPTOR, &tree, &[e], 4000, |p| {
            p.set("ColorHandling", ParamValue::Enum("StaticColor".into()));
            p.set("MaximumLevel", ParamValue::Double(0.8));
            p.set("LevelVariation", ParamValue::Int(0));
        });
        let nodes = out.nodes_for(&tree.node(e).element_id.unwrap()).unwrap();
        let IntentData::ArrayLighting(a) = &nodes[0].intent.data else { panic!() };
        let max = a.values().iter().map(|v| v.intensity).fold(0.0, f64::max);
        assert!(max <= 0.8 + 1e-9 && max > 0.5, "{max}");
    }

    #[test]
    fn visibility_follows_colour_handling() {
        let mut p = DESCRIPTOR.new_params();
        p.set("ColorHandling", ParamValue::Enum("StaticColor".into()));
        p.set("IndividualElements", ParamValue::Bool(false));
        let mut v = Visibility::default();
        visibility(&p, &TargetInfo::detached(), &mut v);
        assert!(v.overrides.contains(&("StaticColor", true)));
        assert!(v.overrides.contains(&("DepthOfEffect", false)));
    }
}
