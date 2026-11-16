//! Port of VixenModules.Effect.Alternating.

use crate::generated::alternating as gen;
use vixen_effects::prelude::*;
use vixen_effects::pulse_renderer;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    validate: Some(validate),
    targets_changed: Some(reset_depth_beyond_first_target),
    param_changed_ctx: Some(param_changed_ctx),
    ported: true,
    ..gen::BASE
};

fn param_changed_ctx(c: &ParamChange, p: &mut Params) {
    let active = p.enum_name("AlternatingMode") == "MarkCollection";
    match c.name {
        "Colors" => validate(p, &c.info.valid_colors()),
        "GroupLevel" => {
            let group = p.int("GroupLevel");
            p.set("GroupLevel", ParamValue::Int(if group > 0 { group } else { 1 }));
        }
        // The setter calls ActivateMarkCollectionSelections.
        MARK_COLLECTIONS | "AlternatingMode" => normalize_mark_selection(p, c.info, active, None, true),
        _ => {}
    }
}

fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    let time_mode = p.enum_name("AlternatingMode") == "TimeInterval";
    let is_static = p.boolean("EnableStatic");
    v.set("MarkCollectionId", !time_mode);
    // InitAllAttributes writes Interval from EnableStatic and then from the mode, so on load and after a mode change the
    // mode alone decides. Only the moment after EnableStatic is toggled, when its setter wrote last, can't be told from here.
    v.set("Interval", time_mode);
    v.set("EnableStatic", time_mode);
    v.set("IntervalSkipCount", !is_static);
    v.set("DepthOfEffect", p.boolean("EnableDepth"));
}

/// `CheckForInvalidColorData`.
pub fn validate(p: &mut Params, valid: &[Color]) {
    if valid.is_empty() {
        return;
    }
    let mut colors = p.gradient_levels("Colors");
    let mut changed = false;
    for glp in colors.iter_mut() {
        if !glp.color_gradient.colors_in_gradient().iter().all(|c| valid.contains(c)) {
            glp.color_gradient = ColorGradient::solid(valid[0]);
            changed = true;
        }
    }
    if changed {
        p.set("Colors", ParamValue::GradientLevels(colors));
    }
}

fn distinct(v: Vec<NodeId>) -> Vec<NodeId> {
    let mut seen = std::collections::HashSet::new();
    v.into_iter().filter(|n| seen.insert(*n)).collect()
}

/// `GetNodesToRenderOn`.
fn nodes_to_render_on(ctx: &RenderCtx, enable_depth: bool, depth_of_effect: i32) -> Vec<NodeId> {
    // `TargetNodes.SelectMany(x => x.GetLeafEnumerator().Distinct())`: distinct within a target, not across them.
    let leaves = || ctx.targets.iter().flat_map(|&t| ctx.tree.distinct_leaves(t)).collect::<Vec<_>>();
    let mut nodes: Vec<NodeId> = ctx.targets.to_vec();
    if !enable_depth {
        nodes = leaves();
    } else {
        for _ in 0..depth_of_effect {
            nodes = distinct(nodes.iter().flat_map(|&n| ctx.tree.node(n).children.iter().copied()).collect());
        }
    }
    if nodes.is_empty() {
        nodes = leaves();
    }
    nodes
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let mut out = EffectIntents::new();
    if ctx.targets.is_empty() {
        return out;
    }
    let colors = p.gradient_levels("Colors");
    let color_count = colors.len() as i32;
    // The setter clamps to 1, but a file's value isn't clamped: `index / 0` throws, and a negative size groups by its size.
    let group = p.int("GroupLevel").unsigned_abs() as usize;
    if group == 0 {
        return out;
    }
    let skip = p.int("IntervalSkipCount");
    let mark_mode = p.enum_name("AlternatingMode") == "MarkCollection";
    let enable_static = p.boolean("EnableStatic");
    let interval_ms = p.int("Interval");
    let has_discrete = ctx.has_discrete_colors();
    // Colors[...] and `% colorCount` throw in C# with an empty list.
    if color_count == 0 {
        return out;
    }

    let nodes = nodes_to_render_on(ctx, p.boolean("EnableDepth"), p.int("DepthOfEffect"));
    let time_span = ctx.time_span;
    let mut interval_time = time_span;
    let mut mark_interval: Vec<TimeSpan> = Vec::new();
    let mut intervals = 1;
    if mark_mode {
        let id = p.guid("MarkCollectionId");
        if let Some(mc) = crate::common::selected_mark_collection(ctx, id) {
            mark_interval.extend(mc.marks_inclusive_of_time(ctx.start_time, ctx.start_time + time_span).iter().map(|m| m.start_time - ctx.start_time));
        }
        mark_interval.push(time_span);
        intervals = mark_interval.len();
    } else if !enable_static {
        // Math.Ceiling(total / 0) overflows Convert.ToInt32 in C#.
        if interval_ms <= 0 {
            return out;
        }
        intervals = (time_span.total_ms() / interval_ms as f64).ceil().round_ties_even() as usize;
        if intervals >= 1 {
            interval_time = TimeSpan::from_ms(interval_ms as f64);
        }
    }

    let mut item = 0i32;
    let mut start_offset = 0i32;
    let mut start_time = TimeSpan::ZERO;
    for i in 0..intervals {
        if ctx.cancelled() {
            return out;
        }
        if mark_mode {
            interval_time = mark_interval[i] - start_time;
        }
        for chunk in nodes.chunks(group) {
            // A negative skip makes a negative index, and `Colors[-1]` throws, which leaves the effect dark.
            let Some(glp) = usize::try_from(item).ok().and_then(|i| colors.get(i)) else { return EffectIntents::new() };
            for &node in chunk {
                render_element(ctx, glp, start_time, interval_time - TimeSpan::from_ms(1.0), node, has_discrete, &mut out);
            }
            item = (item + 1) % color_count;
        }
        start_offset = (skip + start_offset) % color_count;
        item = start_offset;
        start_time = if mark_mode { mark_interval[i] } else { start_time + interval_time };
    }
    out
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
    use vixen_core::StateValue;
    use vixen_effects::testing;

    #[test]
    fn smoke() {
        testing::smoke_test(&DESCRIPTOR);
        testing::assert_produces_light(&DESCRIPTOR);
    }

    #[test]
    fn a_saved_depth_too_deep_renders_on_leaves_until_targets_are_assigned() {
        let mut tree = ElementTree::new();
        let s = tree.add_string("S", 4);
        let leaves = tree.distinct_leaves(s);
        let edit = |p: &mut Params| {
            p.set("EnableDepth", ParamValue::Bool(true));
            p.set("DepthOfEffect", ParamValue::Int(5));
        };
        // Loaded: the depth stays, walks off the bottom of the tree and falls back to the leaves, which alternate.
        let loaded = testing::render_loaded(&DESCRIPTOR, &tree, &[s], 1000, edit);
        assert_ne!(colour_at(&loaded, &tree, leaves[0], 100), colour_at(&loaded, &tree, leaves[1], 100));
        // Assigned: TargetNodesChanged resets it to 0, so the one node rendered on is the target itself.
        let assigned = testing::render_with(&DESCRIPTOR, &tree, &[s], 1000, edit);
        assert_eq!(colour_at(&assigned, &tree, leaves[0], 100), colour_at(&assigned, &tree, leaves[1], 100));
    }

    #[test]
    fn mark_selection_follows_the_mode_and_the_collections() {
        let mut tree = ElementTree::new();
        let s = tree.add_string("S", 2);
        let marks = [MarkCollection::default(), MarkCollection::default()];
        let targets = [s];
        let t = TargetInfo { marks: &marks, ..testing::target_info(&tree, &targets) };
        let mut p = DESCRIPTOR.new_params();
        DESCRIPTOR.set_marks(&mut p, &t);
        assert!(p.guid("MarkCollectionId").is_nil(), "an inactive selection is left alone");
        DESCRIPTOR.set_param(&mut p, "AlternatingMode", ParamValue::Enum("MarkCollection".into()), &t);
        assert_eq!(p.guid("MarkCollectionId"), marks[0].id);
        p.set("MarkCollectionId", ParamValue::Guid(marks[1].id));
        DESCRIPTOR.set_marks(&mut p, &t);
        assert_eq!(p.guid("MarkCollectionId"), marks[1].id);
        let one = TargetInfo { marks: &marks[..1], ..t };
        DESCRIPTOR.set_marks(&mut p, &one);
        assert_eq!(p.guid("MarkCollectionId"), marks[0].id, "a removed collection falls back to the first");
    }

    #[test]
    fn group_level_setter_clamps() {
        let t = TargetInfo::detached();
        let mut p = DESCRIPTOR.new_params();
        DESCRIPTOR.set_param(&mut p, "GroupLevel", ParamValue::Int(0), &t);
        assert_eq!(p.int("GroupLevel"), 1);
        DESCRIPTOR.set_param(&mut p, "GroupLevel", ParamValue::Int(3), &t);
        assert_eq!(p.int("GroupLevel"), 3);
    }

    fn colour_at(out: &EffectIntents, tree: &ElementTree, leaf: NodeId, ms: i64) -> Option<Color> {
        let nodes = out.nodes_for(&tree.node(leaf).element_id.unwrap())?;
        let n = nodes.iter().find(|n| n.start <= TimeSpan::from_ms_i(ms) && TimeSpan::from_ms_i(ms) <= n.end)?;
        match n.intent.state_at(TimeSpan::from_ms_i(ms) - n.start) {
            StateValue::Lighting(v) => Some(v.color),
            _ => None,
        }
    }

    #[test]
    fn colours_alternate_across_elements_and_intervals() {
        let mut tree = ElementTree::new();
        let s = tree.add_string("S", 2);
        let leaves = tree.distinct_leaves(s);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[s], 1000, |_| {});
        // Interval 500, skip 1: element 0 is red then lime; element 1 is lime then red.
        assert_eq!(colour_at(&out, &tree, leaves[0], 100), Some(Color::RED));
        assert_eq!(colour_at(&out, &tree, leaves[1], 100), Some(Color::LIME));
        assert_eq!(colour_at(&out, &tree, leaves[0], 600), Some(Color::LIME));
        assert_eq!(colour_at(&out, &tree, leaves[1], 600), Some(Color::RED));
    }

    #[test]
    fn a_saved_group_level_is_not_clamped() {
        let mut tree = ElementTree::new();
        let s = tree.add_string("S", 4);
        let leaves = tree.distinct_leaves(s);
        // Zero divides by zero in the C#, so the effect stays dark.
        let out = testing::render_loaded(&DESCRIPTOR, &tree, &[s], 1000, |p| {
            p.set("GroupLevel", ParamValue::Int(0));
        });
        assert!(out.is_empty());
        // A negative size still groups by its size.
        let out = testing::render_loaded(&DESCRIPTOR, &tree, &[s], 1000, |p| {
            p.set("GroupLevel", ParamValue::Int(-2));
        });
        assert_eq!(colour_at(&out, &tree, leaves[0], 100), colour_at(&out, &tree, leaves[1], 100));
        assert_eq!(colour_at(&out, &tree, leaves[2], 100), Some(Color::LIME));
    }

    #[test]
    fn a_negative_skip_leaves_the_effect_dark() {
        let mut tree = ElementTree::new();
        let s = tree.add_string("S", 2);
        let out = testing::render_loaded(&DESCRIPTOR, &tree, &[s], 1500, |p| {
            p.set("IntervalSkipCount", ParamValue::Int(-1));
        });
        assert!(out.is_empty());
    }

    #[test]
    fn leaves_are_distinct_within_a_target_only() {
        let mut tree = ElementTree::new();
        let x = tree.add_element("X");
        let y = tree.add_element("Y");
        let z = tree.add_element("Z");
        let a = tree.add_node(Uuid::new_v4(), "A", None);
        let b = tree.add_node(Uuid::new_v4(), "B", None);
        tree.add_child(a, x);
        tree.add_child(a, y);
        tree.add_child(b, x);
        tree.add_child(b, z);
        // X, Y, X, Z, so Z gets the colour after the second X, and X is lit twice.
        let out = testing::render_loaded(&DESCRIPTOR, &tree, &[a, b], 1000, |p| {
            p.set("EnableStatic", ParamValue::Bool(true));
        });
        assert_eq!(colour_at(&out, &tree, y, 100), Some(Color::LIME));
        assert_eq!(colour_at(&out, &tree, z, 100), Some(Color::LIME));
        assert_eq!(out.nodes_for(&tree.node(x).element_id.unwrap()).unwrap().len(), 2);
    }

    #[test]
    fn group_level_keeps_neighbours_together() {
        let mut tree = ElementTree::new();
        let s = tree.add_string("S", 4);
        let leaves = tree.distinct_leaves(s);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[s], 1000, |p| {
            p.set("GroupLevel", ParamValue::Int(2));
        });
        assert_eq!(colour_at(&out, &tree, leaves[0], 100), colour_at(&out, &tree, leaves[1], 100));
        assert_eq!(colour_at(&out, &tree, leaves[2], 100), Some(Color::LIME));
    }

    #[test]
    fn mark_mode_splits_at_marks() {
        let mut tree = ElementTree::new();
        let e = tree.add_element("E");
        let mut mc = MarkCollection::default();
        mc.add_mark(Mark::new(TimeSpan::from_ms_i(300)));
        let marks = [mc.clone()];
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let ctx = RenderCtx { video_cache: None,
            tree: &tree,
            targets: &[e],
            start_time: TimeSpan::ZERO,
            time_span: TimeSpan::from_ms_i(1000),
            frame_ms: 50,
            marks: &marks,
            audio: None,
            module_data_root: None,
            cancel: &cancel,
        };
        let mut p = DESCRIPTOR.new_params();
        p.set("AlternatingMode", ParamValue::Enum("MarkCollection".into()));
        p.set("MarkCollectionId", ParamValue::Guid(mc.id));
        let out = render(&ctx, &p);
        let nodes = out.nodes_for(&tree.node(e).element_id.unwrap()).unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[1].start, TimeSpan::from_ms_i(300));
    }

    #[test]
    fn static_hides_the_skip_count_and_the_mode_decides_the_interval() {
        let mut p = DESCRIPTOR.new_params();
        p.set("EnableStatic", ParamValue::Bool(true));
        let mut v = Visibility::default();
        visibility(&p, &TargetInfo::detached(), &mut v);
        assert!(v.overrides.contains(&("IntervalSkipCount", false)));
        assert!(v.overrides.contains(&("Interval", true)));
        p.set("AlternatingMode", ParamValue::Enum("MarkCollection".into()));
        let mut v = Visibility::default();
        visibility(&p, &TargetInfo::detached(), &mut v);
        assert!(v.overrides.contains(&("Interval", false)));
    }
}
