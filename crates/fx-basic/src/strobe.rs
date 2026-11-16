//! Port of VixenModules.Effect.Strobe.

use crate::generated::strobe as gen;
use vixen_effects::prelude::*;
use vixen_effects::pulse_renderer;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    validate: Some(validate),
    param_changed_ctx: Some(param_changed_ctx),
    ported: true,
    ..gen::BASE
};

fn param_changed_ctx(c: &ParamChange, p: &mut Params) {
    if matches!(c.name, MARK_COLLECTIONS | "StrobeSource") {
        // The setter calls ActivateMarkCollectionSelections.
        normalize_mark_selection(p, c.info, p.enum_name("StrobeSource") != "TimeInterval", None, true);
    }
}

#[derive(Copy, Clone, PartialEq)]
enum Source {
    TimeInterval,
    LabelValue,
    LabelDuration,
}

/// `StrobeClass`.
struct StrobeClass {
    cycle_time: i32,
    start: TimeSpan,
}

fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    let time = p.enum_name("StrobeSource") == "TimeInterval";
    let advanced = p.enum_name("StrobeMode") == "Advanced";
    v.set("MarkCollectionId", !time);
    v.set("OnTimeCurve", advanced);
    v.set("CycleVariationCurve", advanced && time);
    v.set("CycleTime", time);
}

/// `CheckForInvalidColorData`.
fn validate(p: &mut Params, valid: &[Color]) {
    if valid.is_empty() {
        return;
    }
    if !p.gradient("Colors").colors_in_gradient().iter().all(|c| valid.contains(c)) {
        p.set("Colors", ParamValue::Gradient(ColorGradient::solid(valid[0])));
    }
}

/// `GetMarkLabelValue`.
fn mark_label_value(text: &str) -> i32 {
    text.trim().parse().unwrap_or(150)
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let mut out = EffectIntents::new();
    if ctx.targets.is_empty() {
        return out;
    }
    let source = match p.enum_name("StrobeSource") {
        "MarkCollection" => Source::LabelValue,
        "MarkCollectionLabelDuration" => Source::LabelDuration,
        _ => Source::TimeInterval,
    };
    let simple = p.enum_name("StrobeMode") != "Advanced";
    let cycle_time = p.int("CycleTime");
    let cycle_curve = p.curve("CycleVariationCurve");
    let on_curve = p.curve("OnTimeCurve");
    let glp = GradientLevelPair::new(p.gradient("Colors"), p.curve("IntensityCurve"));
    let has_discrete = ctx.has_discrete_colors();
    let ts = ctx.time_span;

    let mut seen = std::collections::HashSet::new();
    let elements: Vec<NodeId> =
        ctx.targets.iter().flat_map(|&t| ctx.tree.leaves(t)).filter(|n| seen.insert(*n)).collect();

    // CalculateInterval
    let calc_interval = |pos: f64| -> f64 {
        let v = scale_curve_to_value(cycle_curve.get_value(pos), cycle_time as f64, 1.0) as i32 as f64;
        if v < 1.0 {
            1.0
        } else {
            v
        }
    };

    // SetupMarks
    let mut classes: Vec<StrobeClass> = Vec::new();
    if source != Source::TimeInterval {
        let Some(mc) = crate::common::selected_mark_collection(ctx, p.guid("MarkCollectionId")) else { return out };
        let mut first = true;
        for mark in mc.marks_inclusive_of_time(ctx.start_time, ctx.start_time + ts) {
            let cycle = |m: &Mark| -> i32 {
                if source == Source::LabelDuration {
                    m.duration.total_ms() as i32
                } else {
                    mark_label_value(&m.text)
                }
            };
            if first && mark.start_time > ctx.start_time {
                classes.push(StrobeClass { start: TimeSpan::ZERO, cycle_time: cycle(mark) });
                first = false;
            }
            if mark.start_time < ctx.start_time {
                continue;
            }
            classes.push(StrobeClass { cycle_time: cycle(mark), start: mark.start_time - ctx.start_time });
            first = false;
        }
        if let Some(last) = classes.last() {
            let end = StrobeClass { cycle_time: last.cycle_time, start: ts };
            classes.push(end);
        }
        if classes.is_empty() {
            return out;
        }
    }

    let mut start_time = TimeSpan::ZERO;
    let mut interval_pos = ctx.time_interval_position(start_time) * 100.0;
    let mut i = 0usize;
    let mut interval = if source != Source::TimeInterval {
        classes[0].cycle_time as f64
    } else if simple {
        cycle_time as f64
    } else {
        calc_interval(interval_pos)
    };
    let on_ms = |interval: f64, pos: f64| interval * if simple { 0.5 } else { on_curve.get_value(pos) / 100.0 };
    let mut interval_time = TimeSpan::from_ms(interval);
    let mut on_duration = TimeSpan::from_ms(on_ms(interval, interval_pos));

    loop {
        if ctx.cancelled() {
            return EffectIntents::new();
        }
        // A zero interval never advances in C# and hangs the render.
        if interval_time.ticks() <= 0 {
            return EffectIntents::new();
        }
        if source != Source::TimeInterval {
            // An out-of-range index throws in C#, which leaves the effect dark.
            let Some(next) = classes.get(i + 1) else { return EffectIntents::new() };
            if (on_duration + start_time).total_ms() > next.start.total_ms() {
                on_duration = TimeSpan::from_ms((next.start.total_ms() - 1.0 - start_time.total_ms()) / 2.0);
            }
        }
        for &element in &elements {
            render_element(ctx, &glp, start_time, on_duration - TimeSpan::from_ms(1.0), element, has_discrete, &mut out);
        }
        start_time += interval_time;
        interval_pos = ctx.time_interval_position(start_time) * 100.0;
        if source != Source::TimeInterval {
            let Some(next) = classes.get(i + 1) else { return EffectIntents::new() };
            if next.start.total_ms() < start_time.total_ms() {
                i += 1;
                start_time = classes[i].start;
                if i > classes.len() - 1 {
                    break;
                }
            }
            interval = classes[i].cycle_time as f64;
        } else {
            interval = if simple { cycle_time as f64 } else { calc_interval(interval_pos) };
        }
        on_duration = TimeSpan::from_ms(on_ms(interval, interval_pos));
        interval_time = TimeSpan::from_ms(interval);
        if start_time.total_ms() >= ts.total_ms() {
            break;
        }
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
    use vixen_effects::testing;

    #[test]
    fn smoke() {
        testing::smoke_test(&DESCRIPTOR);
        testing::assert_produces_light(&DESCRIPTOR);
    }

    #[test]
    fn choosing_a_mark_source_selects_a_collection() {
        let mut tree = ElementTree::new();
        let s = tree.add_string("S", 2);
        let marks = [MarkCollection::default()];
        let targets = [s];
        let t = TargetInfo { marks: &marks, ..testing::target_info(&tree, &targets) };
        let mut p = DESCRIPTOR.new_params();
        DESCRIPTOR.set_param(&mut p, "StrobeSource", ParamValue::Enum("MarkCollection".into()), &t);
        assert_eq!(p.guid("MarkCollectionId"), marks[0].id);
    }

    #[test]
    fn simple_mode_flashes_every_cycle_for_half_a_cycle() {
        let mut tree = ElementTree::new();
        let e = tree.add_element("E");
        let out = testing::render_with(&DESCRIPTOR, &tree, &[e], 1000, |p| {
            p.set("CycleTime", ParamValue::Int(200));
        });
        let nodes = out.nodes_for(&tree.node(e).element_id.unwrap()).unwrap();
        // The default intensity curve has three points, so each flash is two intents.
        let starts: Vec<f64> = nodes.iter().map(|n| n.start.total_ms()).collect();
        assert_eq!(nodes.len(), 10, "{starts:?}");
        assert_eq!(starts[0], 0.0);
        assert_eq!(starts[2], 200.0);
        // On time is 100 ms less the 1 ms the C# trims off.
        assert!((nodes[1].end.total_ms() - 99.0).abs() < 1e-6, "{}", nodes[1].end.total_ms());
    }

    #[test]
    fn zero_cycle_time_does_not_hang() {
        let mut tree = ElementTree::new();
        let e = tree.add_element("E");
        let out = testing::render_with(&DESCRIPTOR, &tree, &[e], 1000, |p| {
            p.set("CycleTime", ParamValue::Int(0));
        });
        assert!(out.is_empty());
    }

    #[test]
    fn mark_label_value_sets_the_cycle_and_defaults_to_150() {
        assert_eq!(mark_label_value("80"), 80);
        assert_eq!(mark_label_value("fast"), 150);
    }

    #[test]
    fn visibility_hides_cycle_time_for_marks() {
        let mut p = DESCRIPTOR.new_params();
        p.set("StrobeSource", ParamValue::Enum("MarkCollection".into()));
        let mut v = Visibility::default();
        visibility(&p, &TargetInfo::detached(), &mut v);
        assert!(v.overrides.contains(&("CycleTime", false)));
        assert!(v.overrides.contains(&("MarkCollectionId", true)));
    }
}
