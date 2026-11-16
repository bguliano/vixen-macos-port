//! Port of `VixenModules.Effect.Pulse.PulseRenderer`, which the basic effects (Pulse, Chase, Spin,
//! Alternating, Wipe, and others) use to turn a level curve and a gradient into intents.

use vixen_core::{Color, ColorGradient, Curve, EffectIntents, ElementTree, Intent, IntentData, IntentNode, NodeId, TimeSpan, Uuid};

/// `PulseRenderer.RenderNode(node, levelCurve, colorGradient, duration, isDiscrete, allowZeroIntensity)`.
pub fn render_node(
    tree: &ElementTree,
    node: NodeId,
    level_curve: &Curve,
    gradient: &ColorGradient,
    duration: TimeSpan,
    is_discrete: bool,
    allow_zero_intensity: bool,
) -> EffectIntents {
    let points = significant_points(level_curve, gradient);
    // `_GetAllSignificantDataPoints` sorts the curve's own point list, so every later read sees it sorted.
    let sorted_curve;
    let level_curve = if level_curve.points.windows(2).all(|w| w[0].0 <= w[1].0) {
        level_curve
    } else {
        let mut c = level_curve.clone();
        c.sort_points();
        sorted_curve = c;
        &sorted_curve
    };
    let mut data = EffectIntents::new();
    for leaf in tree.distinct_leaves(node) {
        let Some(element) = tree.node(leaf).element_id else { continue };
        if is_discrete && tree.is_tree_discrete(leaf) {
            let in_gradient = gradient.colors_in_gradient();
            for color in tree.valid_colors(leaf, false).into_iter().filter(|c| in_gradient.contains(c)) {
                add_intents(element, &points, level_curve, gradient, duration, &mut data, allow_zero_intensity, Some(color));
            }
        } else {
            add_intents(element, &points, level_curve, gradient, duration, &mut data, allow_zero_intensity, None);
        }
    }
    data
}

#[allow(clippy::too_many_arguments)]
fn add_intents(
    element: Uuid,
    points: &[f64],
    level_curve: &Curve,
    gradient: &ColorGradient,
    duration: TimeSpan,
    data: &mut EffectIntents,
    allow_zero: bool,
    color: Option<Color>,
) {
    let mut last_position = points[0];
    let mut last_end = TimeSpan::ZERO;
    for &position in &points[1..] {
        let start_time = last_end;
        let span = TimeSpan::from_ms(duration.total_ms() * (position - last_position));
        match color {
            None => {
                let start_color = gradient.get_color_at(last_position);
                let end_color = gradient.get_color_at(position);
                let start_i = level_curve.get_value(last_position * 100.0) / 100.0;
                let end_i = level_curve.get_value(position * 100.0) / 100.0;
                if allow_zero || !(start_i == 0.0 && end_i == 0.0) {
                    data.add_intent_for_element(element, Intent::lighting(start_color, end_color, start_i, end_i, span), start_time);
                }
            }
            Some(c) => {
                let start_i = gradient.get_proportion_of_color_at(last_position, c) * level_curve.get_value(last_position * 100.0) / 100.0;
                let end_i = gradient.get_proportion_of_color_at(position, c) * level_curve.get_value(position * 100.0) / 100.0;
                if allow_zero || !(start_i == 0.0 && end_i == 0.0) {
                    data.add_intent_for_element(element, Intent::discrete(c, start_i, end_i, span), start_time);
                }
            }
        }
        last_position = position;
        last_end = start_time + span;
    }
}

/// `_GetAllSignificantDataPoints` with no colour filter: every curve and gradient stop, sorted and distinct.
pub fn significant_points(level_curve: &Curve, gradient: &ColorGradient) -> Vec<f64> {
    let mut sorted = level_curve.clone();
    sorted.sort_points();
    let mut points = vec![0.0];
    for (x, _) in &sorted.points {
        points.push(*x / 100.0);
    }
    for cp in gradient.sorted_colors() {
        points.push(cp.position);
    }
    points.push(1.0);
    points.sort_by(f64::total_cmp);
    points.dedup();
    points
}

/// `PulseRenderer.RenderNode(node, level, color, duration)`: a constant level on one element.
pub fn render_node_static(tree: &ElementTree, node: NodeId, level: f64, color: Color, duration: TimeSpan) -> EffectIntents {
    let mut data = EffectIntents::new();
    if let Some(element) = tree.node(node).element_id {
        if level > 0.0 {
            let span = TimeSpan::from_ms(duration.total_ms());
            data.add_intent_for_element(element, Intent::lighting(color, color, level, level, span), TimeSpan::ZERO);
        }
    }
    data
}

/// `GenerateExtendedStaticPulse(target, intentNode, timeSpan, hasDiscrete, gradient)`: holds the last
/// value of an intent until the effect ends.
pub fn extended_static_pulse(
    tree: &ElementTree,
    target: NodeId,
    intent_node: Option<&IntentNode>,
    time_span: TimeSpan,
    has_discrete: bool,
    gradient: Option<&ColorGradient>,
) -> EffectIntents {
    let Some(n) = intent_node else { return EffectIntents::new() };
    if n.end >= time_span {
        return EffectIntents::new();
    }
    let end = match &n.intent.data {
        IntentData::Lighting { end, .. } if end.intensity > 0.0 => Some((end.intensity, end.color)),
        IntentData::Discrete { end, .. } if end.intensity > 0.0 => Some((end.intensity, end.color)),
        _ => None,
    };
    let Some((intensity, color)) = end else { return EffectIntents::new() };
    let curve = Curve::flat(intensity * 100.0);
    let solid = ColorGradient::solid(color);
    let g = gradient.unwrap_or(&solid);
    let mut result = render_node(tree, target, &curve, g, time_span - n.end, has_discrete, false);
    result.offset_all(n.end);
    result
}

/// `GenerateStartingStaticPulse`: holds the first value of an intent from time zero until it starts.
pub fn starting_static_pulse(
    tree: &ElementTree,
    target: NodeId,
    intent_node: Option<&IntentNode>,
    has_discrete: bool,
    gradient: Option<&ColorGradient>,
) -> EffectIntents {
    let Some(n) = intent_node else { return EffectIntents::new() };
    if n.start <= TimeSpan::ZERO {
        return EffectIntents::new();
    }
    let start = match &n.intent.data {
        IntentData::Lighting { start, .. } if start.intensity > 0.0 => Some((start.intensity, start.color)),
        IntentData::Discrete { start, .. } if start.intensity > 0.0 => Some((start.intensity, start.color)),
        _ => None,
    };
    let Some((intensity, color)) = start else { return EffectIntents::new() };
    let curve = Curve::flat(intensity * 100.0);
    let solid = ColorGradient::solid(color);
    let g = gradient.unwrap_or(&solid);
    render_node(tree, target, &curve, g, n.start, has_discrete, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_core::{ColorPoint, StateValue};
    use vixen_core::gradient::AlphaPoint;

    #[test]
    fn splits_at_every_significant_point() {
        let mut t = ElementTree::new();
        let n = t.add_element("E");
        let g = ColorGradient::from_parts(
            vec![ColorPoint::new(Color::RED, 0.0), ColorPoint::new(Color::BLUE, 0.5), ColorPoint::new(Color::RED, 1.0)],
            vec![AlphaPoint::new(255, 0.0), AlphaPoint::new(255, 1.0)],
        );
        let c = Curve::flat100();
        let out = render_node(&t, n, &c, &g, TimeSpan::from_ms_i(1000), false, false);
        let nodes = out.nodes_for(&t.node(n).element_id.unwrap()).unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].start, TimeSpan::ZERO);
        assert_eq!(nodes[1].start, TimeSpan::from_ms_i(500));
        assert_eq!(nodes[1].end, TimeSpan::from_ms_i(1000));
    }

    #[test]
    fn zero_intensity_segments_are_skipped_unless_allowed() {
        let mut t = ElementTree::new();
        let n = t.add_element("E");
        let g = ColorGradient::solid(Color::WHITE);
        let c = Curve::flat(0.0);
        assert!(render_node(&t, n, &c, &g, TimeSpan::from_ms_i(100), false, false).map.is_empty());
        assert!(!render_node(&t, n, &c, &g, TimeSpan::from_ms_i(100), false, true).map.is_empty());
    }

    #[test]
    fn extended_pulse_holds_the_final_value() {
        let mut t = ElementTree::new();
        let n = t.add_element("E");
        let e = t.node(n).element_id.unwrap();
        let g = ColorGradient::solid(Color::RED);
        let c = Curve::flat100();
        let first = render_node(&t, n, &c, &g, TimeSpan::from_ms_i(100), false, false);
        let node = &first.nodes_for(&e).unwrap()[0];
        let ext = extended_static_pulse(&t, n, Some(node), TimeSpan::from_ms_i(300), false, None);
        let tail = &ext.nodes_for(&e).unwrap()[0];
        assert_eq!(tail.start, TimeSpan::from_ms_i(100));
        assert_eq!(tail.end, TimeSpan::from_ms_i(300));
        match tail.intent.state_at(TimeSpan::from_ms_i(50)) {
            StateValue::Lighting(v) => {
                assert_eq!(v.color, Color::RED);
                assert_eq!(v.intensity, 1.0);
            }
            other => panic!("{other:?}"),
        }
    }
}
