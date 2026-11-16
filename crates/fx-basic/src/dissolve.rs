//! Port of VixenModules.Effect.Dissolve.

use crate::generated::dissolve as gen;
use vixen_effects::prelude::*;
use vixen_effects::pulse_renderer;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    // Dissolve's CheckForInvalidColorData is the same rule as Alternating's.
    validate: Some(crate::alternating::validate),
    targets_changed: Some(normalize_depth),
    param_changed_ctx: Some(param_changed_ctx),
    ported: true,
    ..gen::BASE
};

/// `NormalizeDepthOfEffect`, from `TargetNodesChanged` and the `DepthOfEffect` setter. Loading leaves the saved depth alone.
fn normalize_depth(p: &mut Params, t: &TargetInfo) {
    if t.targets.is_empty() {
        return;
    }
    let maximum = t.targets.iter().map(|n| t.tree.max_child_depth(*n)).min().unwrap_or(0);
    let depth = p.int("DepthOfEffect");
    if depth < 0 || depth > maximum - 1 {
        p.set("DepthOfEffect", ParamValue::Int(0));
    }
}

fn param_changed_ctx(c: &ParamChange, p: &mut Params) {
    let active = p.enum_name("DissolveMode") == "MarkCollection";
    match c.name {
        "Colors" => crate::alternating::validate(p, &c.info.valid_colors()),
        "DepthOfEffect" => normalize_depth(p, c.info),
        "GroupLevel" | "StartingNode" => {
            let v = p.int(c.name);
            p.set(c.name, ParamValue::Int(if v > 0 { v } else { 1 }));
        }
        // The setter calls ActivateMarkCollectionSelections.
        MARK_COLLECTIONS | "DissolveMode" => normalize_mark_selection(p, c.info, active, None, true),
        _ => {}
    }
}

const TIME_INTERVAL: i32 = 25;

#[derive(Copy, Clone, PartialEq)]
enum MarkType {
    PerMark,
    PerMarkFill,
    PerMarkDissolve,
    MarkLabelValue,
    MarkLabelPixels,
}

#[derive(Clone, Copy)]
struct TempClass {
    element_index: usize,
    color_index: usize,
}

#[derive(Clone, Copy)]
struct DissolveClass {
    element_index: usize,
    color_index: usize,
    start: TimeSpan,
    end: TimeSpan,
    duration: TimeSpan,
}

struct Dissolve<'a> {
    ctx: &'a RenderCtx<'a>,
    colors: Vec<GradientLevelPair>,
    mark_mode: bool,
    mark_type: MarkType,
    color_per_step: bool,
    random_color: bool,
    group_colors: bool,
    random_dissolve: bool,
    flip: bool,
    both: bool,
    together: bool,
    starting_node: i32,
    curve: Curve,
    rand: Rand,
    total_nodes: usize,
    nodes: Vec<TempClass>,
    temp_nodes: Vec<TempClass>,
    elements: Vec<DissolveClass>,
    render_elements: Vec<DissolveClass>,
}

fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    let mark_mode = p.enum_name("DissolveMode") == "MarkCollection";
    let mt = p.enum_name("DissolveMarkType");
    let per_mark = mt == "PerMark";
    let fill_or_dissolve = mt == "PerMarkFill" || mt == "PerMarkDissolve";
    let random = p.boolean("RandomDissolve");
    let both = p.boolean("BothDirections");
    let cps = p.boolean("ColorPerStep");
    v.set("MarkCollectionId", mark_mode);
    v.set("DissolveCurve", per_mark || !mark_mode);
    v.set("DissolveMarkType", mark_mode);
    v.set("DissolveFlip", !random);
    v.set("StartingNode", !random);
    v.set("BothDirections", !random);
    v.set("DirectionsTogether", !random && both);
    v.set("GroupLevel", !mark_mode || !cps || !fill_or_dissolve);
    v.set("EnableDepth", !cps || !fill_or_dissolve);
    v.set("DepthOfEffect", p.boolean("EnableDepth"));
}

fn distinct(v: Vec<NodeId>) -> Vec<NodeId> {
    let mut seen = std::collections::HashSet::new();
    v.into_iter().filter(|n| seen.insert(*n)).collect()
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    if ctx.targets.is_empty() {
        return EffectIntents::new();
    }
    let colors = p.gradient_levels("Colors");
    // Colors[...] and `% colorCount` throw in C# for an empty list.
    if colors.is_empty() {
        return EffectIntents::new();
    }
    let mark_type = match p.enum_name("DissolveMarkType") {
        "PerMarkFill" => MarkType::PerMarkFill,
        "PerMarkDissolve" => MarkType::PerMarkDissolve,
        "MarkLabelValue" => MarkType::MarkLabelValue,
        "MarkLabelPixels" => MarkType::MarkLabelPixels,
        _ => MarkType::PerMark,
    };
    let mark_mode = p.enum_name("DissolveMode") == "MarkCollection";
    let mut d = Dissolve {
        ctx,
        colors,
        mark_mode,
        mark_type,
        color_per_step: p.boolean("ColorPerStep"),
        random_color: p.boolean("RandomColor"),
        group_colors: p.boolean("GroupColors"),
        random_dissolve: p.boolean("RandomDissolve"),
        flip: p.boolean("DissolveFlip"),
        both: p.boolean("BothDirections"),
        together: p.boolean("DirectionsTogether"),
        starting_node: p.int("StartingNode").max(1),
        curve: p.curve("DissolveCurve"),
        rand: Rand::default(),
        total_nodes: 0,
        nodes: Vec::new(),
        temp_nodes: Vec::new(),
        elements: Vec::new(),
        render_elements: Vec::new(),
    };
    // The setter clamps to 1, but a file's value isn't clamped: `index / 0` throws, and a negative size groups by its size.
    let group_level = p.int("GroupLevel").unsigned_abs() as usize;
    if group_level == 0 {
        return EffectIntents::new();
    }

    let depth = p.int("DepthOfEffect");
    let leaves = || distinct(ctx.targets.iter().flat_map(|&n| ctx.tree.leaves(n)).collect::<Vec<_>>());
    let mut render_nodes: Vec<NodeId>;
    if !p.boolean("EnableDepth") || depth == 0 {
        render_nodes = leaves();
    } else {
        render_nodes = ctx.targets.to_vec();
        for _ in 0..depth {
            render_nodes = distinct(render_nodes.iter().flat_map(|&n| ctx.tree.node(n).children.iter().copied()).collect());
        }
    }
    if render_nodes.is_empty() {
        render_nodes = leaves();
    }

    // Marks in the effect's time range.
    let mut marks: Option<Vec<(TimeSpan, String)>> = None;
    if mark_mode {
        if let Some(mc) = crate::common::selected_mark_collection(ctx, p.guid("MarkCollectionId")) {
            marks = Some(
                mc.marks_inclusive_of_time(ctx.start_time, ctx.start_time + ctx.time_span)
                    .iter()
                    .map(|m| (m.start_time - ctx.start_time, m.text.clone()))
                    .collect(),
            );
        }
    }

    let per_count_grouping = mark_mode
        && matches!(mark_type, MarkType::PerMarkFill | MarkType::PerMarkDissolve)
        && d.color_per_step;
    let per_group = if per_count_grouping {
        let node_count = render_nodes.len() as i32;
        let marks_per_count = match &marks {
            Some(m) if m.len() > 1 => m.len() as i32 - 1,
            _ => 1,
        };
        if d.together && d.both {
            node_count / marks_per_count / 2
        } else {
            node_count / marks_per_count
        }
    } else {
        group_level as i32
    };
    // The C# divides by this, which throws at zero.
    if per_group <= 0 {
        return EffectIntents::new();
    }
    let groups: Vec<&[NodeId]> = render_nodes.chunks(per_group as usize).collect();
    d.total_nodes = groups.len();
    d.initialize(1);
    d.render_node(&groups, marks.as_deref())
}

impl Dissolve<'_> {
    /// `InitializeDissolveClasses`.
    fn initialize(&mut self, intervals: i32) {
        self.nodes.clear();
        self.temp_nodes.clear();
        let color_count = self.colors.len();
        let total = self.total_nodes as i32;
        let mut starting = self.starting_node - 1;
        let mut backwards = starting - 1;
        let mut element_index: Vec<usize> = (0..self.total_nodes).collect();
        let mut color_index = 0usize;
        for x in 0..total {
            let current: i32;
            if self.random_dissolve {
                let idx = self.rand.next_range(0, element_index.len() as i32) as usize;
                current = element_index.remove(idx.min(element_index.len().saturating_sub(1))) as i32;
            } else {
                if starting < 0 {
                    starting = 0;
                }
                if backwards < 0 {
                    backwards = total - 1;
                }
                if self.both {
                    if x % 2 == 0 {
                        current = backwards % total;
                        backwards -= 1;
                    } else {
                        current = starting % total;
                        starting += 1;
                    }
                } else {
                    current = starting % total;
                    starting += 1;
                }
            }
            if self.mark_type == MarkType::PerMark && self.color_per_step && self.mark_mode {
                if self.random_color {
                    if x == 0 {
                        color_index = self.rand.next_range(0, color_count as i32) as usize;
                    }
                } else {
                    color_index = intervals.rem_euclid(color_count as i32) as usize;
                }
            } else {
                color_index = if self.random_color {
                    self.rand.next_range(0, color_count as i32) as usize
                } else {
                    current as usize % color_count
                };
            }
            self.nodes.push(TempClass { element_index: current as usize, color_index });
        }
        if !self.flip {
            self.nodes.reverse();
        }
    }

    fn copy_elements(&mut self) {
        self.render_elements.append(&mut self.elements);
    }

    fn render_node(&mut self, groups: &[&[NodeId]], marks: Option<&[(TimeSpan, String)]>) -> EffectIntents {
        let ctx = self.ctx;
        let ts = ctx.time_span;
        let mut out = EffectIntents::new();
        let mut interval_time = ts;
        let mut mark_interval: Vec<TimeSpan> = Vec::new();
        let mut mark_text: Vec<&str> = Vec::new();
        let pixel_count = self.nodes.len() as i32;
        let mut start_time = TimeSpan::ZERO;
        let intervals: i32;
        let mut interval = 1.0f64;
        let mut total_pixel_count = 0i32;
        let (mark_mode, mt) = (self.mark_mode, self.mark_type);

        if mark_mode {
            if let Some(m) = marks {
                for (t, text) in m {
                    mark_interval.push(*t);
                    mark_text.push(text);
                }
            }
            mark_interval.push(ts);
            let mut n = mark_interval.len() as i32;
            if mt != MarkType::PerMark {
                n -= 1;
            }
            intervals = n;
        } else {
            intervals = (ts.total_ms() / TIME_INTERVAL as f64).ceil().round_ties_even() as i32;
            if intervals >= 1 {
                interval_time = TimeSpan::from_ms(TIME_INTERVAL as f64);
            }
        }
        let mut end_time = ts;
        let mut pixels: i32;

        for i in 0..intervals {
            if ctx.cancelled() {
                return EffectIntents::new();
            }
            if mark_mode && mt == MarkType::PerMark {
                interval = (mark_interval[i as usize] - start_time).total_ms() / TIME_INTERVAL as f64;
                end_time = mark_interval[i as usize];
            }
            let mut j = 0;
            while (j as f64) < interval {
                let position = if !mark_mode || mt != MarkType::PerMark {
                    100.0 / intervals as f64 * i as f64
                } else {
                    100.0 / interval * j as f64
                };
                if mark_mode && mt != MarkType::PerMark {
                    match mt {
                        MarkType::MarkLabelValue => {
                            let pct: i32 = mark_text.get(i as usize).and_then(|t| t.trim().parse().ok()).unwrap_or(0);
                            pixels = (pixel_count as f64 * pct as f64 / 100.0).ceil() as i32 - total_pixel_count;
                        }
                        MarkType::MarkLabelPixels => {
                            let px: i32 = mark_text.get(i as usize).and_then(|t| t.trim().parse().ok()).unwrap_or(0);
                            pixels = (px as f64).ceil() as i32 - total_pixel_count;
                        }
                        MarkType::PerMarkFill => {
                            pixels = if !self.color_per_step {
                                (pixel_count as f64 / (intervals - 1) as f64 * (i + 1) as f64).ceil() as i32 - total_pixel_count
                            } else {
                                1
                            };
                        }
                        _ => {
                            // PerMarkDissolve
                            if self.color_per_step {
                                pixels = if i == 0 { marks.map(|m| m.len() as i32).unwrap_or(0) - 1 } else { -1 };
                            } else {
                                pixels = (pixel_count as f64 / (intervals - 1) as f64 * (intervals - i - 1) as f64).ceil() as i32
                                    - total_pixel_count;
                            }
                        }
                    }
                } else {
                    pixels = (pixel_count as f64 * self.curve.get_value(position) / 100.0).ceil() as i32 - total_pixel_count;
                }

                let mut together = 1;
                let mut both_together = 1;
                if self.color_per_step {
                    if mark_mode {
                        match mt {
                            MarkType::PerMark if self.both && self.together => {
                                together = 2;
                                both_together = 2;
                            }
                            MarkType::PerMarkFill | MarkType::PerMarkDissolve => {
                                together = 1;
                                both_together = if self.both && !self.random_dissolve && self.together { 2 } else { together };
                            }
                            _ => {}
                        }
                        pixels /= together;
                    } else if !self.both {
                        together = 1;
                        both_together = 1;
                        pixels /= together;
                    } else {
                        together = if !self.together && self.both { 1 } else { 2 };
                        both_together = together;
                        pixels /= together;
                    }
                } else {
                    together = if !self.together && self.both { 1 } else { 2 };
                    both_together = together;
                    pixels /= together;
                }

                if pixels >= 0 {
                    for _ in 0..pixels {
                        // The remaining steps would all break out of the inner loop.
                        if self.nodes.is_empty() {
                            break;
                        }
                        for _ in 0..both_together {
                            if self.nodes.is_empty() {
                                break;
                            }
                            total_pixel_count += 1;
                            let n = self.nodes[0];
                            self.elements.push(DissolveClass {
                                start: start_time,
                                end: end_time,
                                duration: end_time - start_time,
                                color_index: n.color_index,
                                element_index: n.element_index,
                            });
                            if self.random_dissolve {
                                self.temp_nodes.push(n);
                            } else {
                                self.temp_nodes.insert(0, n);
                            }
                            self.nodes.remove(0);
                        }
                    }
                } else {
                    for _ in 0..(-(pixels as i64)) {
                        if self.temp_nodes.is_empty() {
                            break;
                        }
                        for _ in 0..both_together {
                            if self.temp_nodes.is_empty() {
                                break;
                            }
                            total_pixel_count -= 1;
                            let t0 = self.temp_nodes[0];
                            for ii in 0..self.elements.len() {
                                let e = self.elements[ii];
                                if e.element_index == t0.element_index && e.start + e.duration > start_time {
                                    // The element is finished: move it to the render list.
                                    self.render_elements.push(DissolveClass {
                                        duration: start_time - e.start - TimeSpan::from_ms(1.0),
                                        start: e.start,
                                        end: e.end,
                                        color_index: e.color_index,
                                        element_index: e.element_index,
                                    });
                                    self.elements.remove(ii);
                                    break;
                                }
                            }
                            if self.random_dissolve {
                                self.nodes.push(t0);
                            } else {
                                self.nodes.insert(0, t0);
                            }
                            self.temp_nodes.remove(0);
                        }
                    }
                }
                if mark_mode && mt == MarkType::PerMark {
                    start_time += TimeSpan::from_ms(TIME_INTERVAL as f64);
                }
                j += 1;
            }
            start_time = if !mark_mode {
                start_time + interval_time
            } else if mt != MarkType::PerMark {
                mark_interval[i as usize + 1]
            } else {
                mark_interval[i as usize]
            };
            if mark_mode && mt == MarkType::PerMark {
                self.initialize(i);
                total_pixel_count = 0;
                self.copy_elements();
            }
        }
        self.copy_elements();

        let has_discrete = ctx.has_discrete_colors();
        let render_elements = std::mem::take(&mut self.render_elements);
        for dn in &render_elements {
            let Some(group) = groups.get(dn.element_index) else { continue };
            for &element in group.iter() {
                if self.group_colors {
                    for glp in &self.colors {
                        render_element(ctx, glp, dn.start, dn.duration, element, has_discrete, &mut out);
                    }
                } else if let Some(glp) = self.colors.get(dn.color_index) {
                    render_element(ctx, glp, dn.start, dn.duration, element, has_discrete, &mut out);
                }
            }
        }
        out
    }
}

fn render_element(ctx: &RenderCtx, glp: &GradientLevelPair, start: TimeSpan, interval: TimeSpan, node: NodeId, hd: bool, out: &mut EffectIntents) {
    if interval <= TimeSpan::ZERO {
        return;
    }
    let mut r = pulse_renderer::render_node(ctx.tree, node, &glp.curve, &glp.color_gradient, interval, hd, false);
    r.offset_all(start);
    out.add(r);
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
    fn depth_is_normalised_when_assigned_and_kept_when_loaded() {
        let mut tree = ElementTree::new();
        let s = tree.add_string("S", 4);
        let targets = [s];
        let t = testing::target_info(&tree, &targets);
        let mut p = DESCRIPTOR.new_params();
        p.set("DepthOfEffect", ParamValue::Int(7));
        let mut loaded = p.clone();
        DESCRIPTOR.load_data(&mut loaded, &t);
        assert_eq!(loaded.int("DepthOfEffect"), 7);
        DESCRIPTOR.set_targets(&mut p, &t);
        assert_eq!(p.int("DepthOfEffect"), 0);
        DESCRIPTOR.set_param(&mut p, "DepthOfEffect", ParamValue::Int(1), &t);
        assert_eq!(p.int("DepthOfEffect"), 1);
        DESCRIPTOR.set_param(&mut p, "DepthOfEffect", ParamValue::Int(2), &t);
        assert_eq!(p.int("DepthOfEffect"), 0);
        DESCRIPTOR.set_param(&mut p, "GroupLevel", ParamValue::Int(-2), &t);
        assert_eq!(p.int("GroupLevel"), 1);
        DESCRIPTOR.set_param(&mut p, "StartingNode", ParamValue::Int(0), &t);
        assert_eq!(p.int("StartingNode"), 1);
    }

    #[test]
    fn every_pixel_gets_lit_for_part_of_the_effect() {
        let mut tree = ElementTree::new();
        let s = tree.add_string("S", 10);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[s], 2000, |_| {});
        assert_eq!(out.map.len(), 10);
    }

    #[test]
    fn sequential_dissolve_removes_pixels_in_order() {
        let mut tree = ElementTree::new();
        let s = tree.add_string("S", 8);
        let leaves = tree.distinct_leaves(s);
        // Default curve falls from 100 % to 0 %: all pixels start lit and drop out in order.
        let out = testing::render_with(&DESCRIPTOR, &tree, &[s], 2000, |p| {
            p.set("RandomDissolve", ParamValue::Bool(false));
            p.set("RandomColor", ParamValue::Bool(false));
        });
        let ends: Vec<f64> = leaves
            .iter()
            .map(|l| out.nodes_for(&tree.node(*l).element_id.unwrap()).unwrap().iter().map(|n| n.end.total_ms()).fold(0.0, f64::max))
            .collect();
        for w in ends.windows(2) {
            assert!(w[0] != w[1] || w[0] >= 1999.0, "{ends:?}");
        }
        let min = ends.iter().cloned().fold(f64::MAX, f64::min);
        let max = ends.iter().cloned().fold(0.0, f64::max);
        assert!(min < max, "{ends:?}");
    }

    #[test]
    fn visibility_hides_sequence_options_when_random() {
        let p = DESCRIPTOR.new_params();
        let mut v = Visibility::default();
        visibility(&p, &TargetInfo::detached(), &mut v);
        assert!(v.overrides.contains(&("StartingNode", false)));
        assert!(v.overrides.contains(&("DissolveMarkType", false)));
    }
}
