//! Port of VixenModules.Effect.LineDance (LineDanceModule, LineDanceData and the Fan enums).

use crate::fixture_base::FixtureTargets;
use crate::generated::line_dance as gen;
use crate::spec::FuncId;
use crate::visual;
use std::collections::HashMap;
use vixen_core::value::RangeValue;
use vixen_core::IntentData;
use vixen_effects::gfx::Graphics;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    targets_changed: Some(targets_changed),
    force_visual: true,
    visual: Some(visual_representation),
    ported: true,
    ..gen::BASE
};

#[derive(Copy, Clone, PartialEq, Eq)]
enum FanMode {
    Synchronized,
    Stagger,
    Concurrent,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum CenterHandling {
    Centered,
    Left,
    Right,
}

/// The effect's settings plus the number of fixtures in the fan (`_renderNodes.Count`).
struct Fan {
    fan_mode: FanMode,
    from_edges: bool,
    invert_pan: bool,
    center: CenterHandling,
    hold_time: i32,
    pan_increment: i32,
    pan_speed: i32,
    pan_start_angle: i32,
    increment_angle: Curve,
    count: i32,
}

impl Fan {
    fn read(p: &Params, count: i32) -> Fan {
        Fan {
            fan_mode: match p.enum_name("FanMode") {
                "Stagger" => FanMode::Stagger,
                "Concurrent" => FanMode::Concurrent,
                _ => FanMode::Synchronized,
            },
            from_edges: p.enum_name("FanDirection") == "FanFromEdges",
            invert_pan: p.boolean("InvertPan"),
            center: match p.enum_name("CenterHandling") {
                "Left" => CenterHandling::Left,
                "Right" => CenterHandling::Right,
                _ => CenterHandling::Centered,
            },
            hold_time: p.int("HoldTime"),
            pan_increment: p.int("PanIncrement"),
            pan_speed: p.int("PanSpeed"),
            pan_start_angle: p.int("PanStartAngle"),
            increment_angle: p.curve("IncrementAngle"),
            count,
        }
    }

    /// `ExtraNodeOnLeft`.
    fn extra_node_on_left(&self) -> bool {
        self.count % 2 == 1 && self.center == CenterHandling::Left
    }

    /// `ExtraNodeOnRight`.
    fn extra_node_on_right(&self) -> bool {
        self.count % 2 == 1 && self.center == CenterHandling::Right
    }

    /// `GetOddBeam`.
    fn odd_beam(&self) -> i32 {
        if self.count % 2 == 1 && matches!(self.center, CenterHandling::Left | CenterHandling::Right) {
            1
        } else {
            0
        }
    }

    /// `CalculateMaxPanIncrement`.
    fn max_pan_increment(&self) -> f64 {
        (1.0 - self.pan_start_angle as f64 / 100.0) / (self.count / 2 + self.odd_beam()) as f64
    }

    /// `CalculatePanIncrement`: the slider as a share of 50, never zero.
    fn calc_pan_increment(&self) -> i32 {
        let v = ((self.pan_increment as f64 / 100.0) * 50.0) as i32;
        if v == 0 {
            1
        } else {
            v
        }
    }

    /// `CalculateStaggerWidth`. The divisor is at least 1 for the two or more fixtures a fan needs.
    fn stagger_width(&self) -> i32 {
        let d = self.count / 2 + self.odd_beam();
        if d == 0 {
            0
        } else {
            (100 - self.hold_time) / d
        }
    }

    /// `CreateStaggerCurve`.
    fn stagger_curve(&self, pan_increment: i32, x_position: i32, width: i32, last: bool) -> Curve {
        // The middle of the curve is zero
        const MIDDLE: f64 = 50.0;
        let top = MIDDLE + pan_increment as f64;
        let (x, w) = (x_position as f64, width as f64);
        if x_position == width {
            // The first node in the stagger fan
            Curve::from_points(vec![(0.0, MIDDLE), (w, top), (100.0, top)])
        } else if last {
            if self.hold_time == 0 {
                Curve::from_points(vec![(0.0, MIDDLE), (x - w, MIDDLE), (100.0, top)])
            } else {
                // Allows for a hold time pause at the end of the effect
                Curve::from_points(vec![
                    (0.0, MIDDLE),
                    (x - w, MIDDLE),
                    (100.0 - self.hold_time as f64, top),
                    (100.0, top),
                ])
            }
        } else {
            Curve::from_points(vec![(0.0, MIDDLE), (x - w, MIDDLE), (x, top), (100.0, top)])
        }
    }

    /// `CreateConcurrentCurve`.
    fn concurrent_curve(&self, pan_increment: i32, max_offset: f64, offset: f64) -> Curve {
        const MIDDLE: f64 = 50.0;
        let top = MIDDLE + pan_increment as f64;
        let pan_speed_offset = (100 - self.pan_speed) as f64 * (offset * pan_increment as f64) / (max_offset * pan_increment as f64);
        if self.pan_speed == 100 {
            return Curve::from_points(vec![(0.0, top), (100.0, top)]);
        }
        Curve::from_points(vec![(0.0, MIDDLE), (pan_speed_offset, top), (100.0, top)])
    }
}

/// `LimitAngle`.
fn limit_angle(angle: f64) -> f64 {
    if angle < 0.0 {
        0.0
    } else if angle > 1.0 {
        1.0
    } else {
        angle
    }
}

/// The nodes the fan renders on, each node's `(label, tag)` of its pan function, and the output.
struct Dance<'a, 'b> {
    ctx: &'a RenderCtx<'b>,
    fan: Fan,
    nodes: Vec<NodeId>,
    functions: HashMap<NodeId, (String, String)>,
    out: EffectIntents,
    failed: bool,
    max_increment_pan: f64,
    pan_start: f64,
}

impl Dance<'_, '_> {
    /// `RenderFanCurve`.
    fn render_fan_curve(&mut self, index: i32, curve: &Curve, initial_value: f64, offset: f64) {
        if self.failed {
            return;
        }
        // Curve.GetValue throws on malformed curves; preserve earlier intents but stop later nodes.
        if curve.points.len() < 2 {
            self.failed = true;
            return;
        }
        // The C# indexes `renderNodes[index]` directly; the loops below keep it in range.
        let Some(&node) = usize::try_from(index).ok().and_then(|i| self.nodes.get(i)) else { return };
        let Some((label, tag)) = self.functions.get(&node) else { return };
        let Some(element) = self.ctx.tree.node(node).element_id else { return };

        let mut curve = curve.clone();
        curve.sort_points();
        // HashSet<double> { 0.0, x..., 100.0 } in insertion order
        let mut point_list: Vec<f64> = vec![0.0];
        for &(x, _) in &curve.points {
            if !point_list.contains(&x) {
                point_list.push(x);
            }
        }
        if !point_list.contains(&100.0) {
            point_list.push(100.0);
        }

        // The middle of the curve represents zero
        const ZERO: f64 = 0.5;
        let identity = FuncId::Pan.to_core();
        let mut start_time = TimeSpan::ZERO;
        for i in 1..point_list.len() {
            let time_span = TimeSpan::from_ms(self.ctx.time_span.total_ms() * ((point_list[i] - point_list[i - 1]) / 100.0));
            let sv = curve.get_value(point_list[i - 1]) / 100.0;
            let sv = limit_angle(initial_value + (((sv - ZERO) / ZERO) * self.max_increment_pan) * offset);
            let ev = curve.get_value(point_list[i]) / 100.0;
            let ev = limit_angle(initial_value + (((ev - ZERO) / ZERO) * self.max_increment_pan) * offset);
            if self.ctx.cancelled() {
                return;
            }
            let intent = Intent::new(
                IntentData::Range {
                    start: RangeValue::new(identity, tag, sv, label),
                    end: RangeValue::new(identity, tag, ev, label),
                },
                time_span,
            );
            self.out.add_intent_for_element(element, intent, start_time);
            start_time += time_span;
        }
    }

    fn signed(&self, offset: f64) -> f64 {
        if self.fan.invert_pan {
            -offset
        } else {
            offset
        }
    }

    fn render_center_node(&mut self, center_index: i32) {
        let c = self.fan.increment_angle.clone();
        self.render_fan_curve(center_index, &c, self.pan_start, 0.0);
    }

    // ---- left half ----

    fn synchronized_left(&mut self, left_middle: i32) {
        let mut offset = 1.0;
        let curve = self.fan.increment_angle.clone();
        let mut left = left_middle;
        while left >= 0 {
            let o = self.signed(offset);
            self.render_fan_curve(left, &curve, self.pan_start, o);
            offset += 1.0;
            left -= 1;
        }
    }

    fn inner_stagger_left(&mut self, left_middle: i32) {
        let mut offset = 1;
        let mut left = left_middle;
        while left >= 0 {
            let width = self.fan.stagger_width();
            let c = self.fan.stagger_curve(self.fan.calc_pan_increment(), width * offset, width, left == 0 && !self.fan.extra_node_on_right());
            let o = self.signed(offset as f64);
            self.render_fan_curve(left, &c, self.pan_start, o);
            offset += 1;
            left -= 1;
        }
    }

    fn outer_stagger_left(&mut self, left_middle: i32) {
        let mut offset = 1;
        for left in 0..=left_middle {
            let width = self.fan.stagger_width();
            let c = self.fan.stagger_curve(
                self.fan.calc_pan_increment(),
                width * offset,
                width,
                left == left_middle && !self.fan.extra_node_on_right(),
            );
            // A moving head offset, since this moves from the outside in
            let render_offset = 1 + (left_middle + 1) - offset;
            let o = self.signed(render_offset as f64);
            self.render_fan_curve(left, &c, self.pan_start, o);
            offset += 1;
        }
    }

    fn concurrent_left(&mut self, left_middle: i32) {
        let mut offset = 1;
        let mut left = left_middle;
        while left >= 0 {
            let c = self.fan.concurrent_curve(self.fan.calc_pan_increment(), (1 + left_middle) as f64, offset as f64);
            let o = self.signed(offset as f64);
            self.render_fan_curve(left, &c, self.pan_start, o);
            offset += 1;
            left -= 1;
        }
    }

    // ---- right half ----

    fn synchronized_right(&mut self, right_middle: i32) {
        let mut offset = -1.0;
        let curve = self.fan.increment_angle.clone();
        for right in right_middle..self.fan.count {
            let o = self.signed(offset);
            self.render_fan_curve(right, &curve, self.pan_start, o);
            offset -= 1.0;
        }
    }

    fn inner_stagger_right(&mut self, right_middle: i32) {
        let mut offset = -1.0f64;
        for right in right_middle..self.fan.count {
            let width = self.fan.stagger_width();
            let c = self.fan.stagger_curve(
                self.fan.calc_pan_increment(),
                width * (offset.abs() as i32),
                width,
                right == self.fan.count - 1 && !self.fan.extra_node_on_left(),
            );
            let o = self.signed(offset);
            self.render_fan_curve(right, &c, self.pan_start, o);
            offset -= 1.0;
        }
    }

    fn outer_stagger_right(&mut self, right_middle: i32) {
        let mut offset = -1.0f64;
        let mut right = self.fan.count - 1;
        while right >= right_middle {
            let width = self.fan.stagger_width();
            let c = self.fan.stagger_curve(
                self.fan.calc_pan_increment(),
                width * (offset.abs() as i32),
                width,
                right == right_middle && !self.fan.extra_node_on_left(),
            );
            // A moving head offset, since this moves from the outside in
            let render_index = (-(right_middle as f64) - offset - 1.0) as i32;
            let o = self.signed(render_index as f64);
            self.render_fan_curve(right, &c, self.pan_start, o);
            offset -= 1.0;
            right -= 1;
        }
    }

    fn concurrent_right(&mut self, right_middle: i32) {
        let mut offset = -1.0f64;
        for right in right_middle..self.fan.count {
            let c = self.fan.concurrent_curve(self.fan.calc_pan_increment(), (self.fan.count - right_middle) as f64, offset.abs());
            let o = self.signed(offset);
            self.render_fan_curve(right, &c, self.pan_start, o);
            offset -= 1.0;
        }
    }

    /// `DoRendering` with one of the `Render*Fan` actions.
    fn do_rendering(&mut self, mode: FanMode, from_edges: bool) {
        let count = self.fan.count;
        // The node index for the left half of the fan, the right half and the center beam
        let mut left_middle = count / 2 - 1;
        let mut right_middle = left_middle + 1;
        let center_index = count / 2;
        let mut display_center = false;

        // An odd number of nodes needs a center beam
        if count % 2 == 1 {
            match self.fan.center {
                CenterHandling::Left => {
                    // The center node joins the left side logic and leaves the right
                    left_middle += 1;
                    right_middle += 1;
                }
                CenterHandling::Centered => {
                    right_middle += 1;
                    display_center = true;
                }
                CenterHandling::Right => {}
            }
        }

        match (mode, from_edges) {
            (FanMode::Synchronized, _) => self.synchronized_left(left_middle),
            (FanMode::Stagger, true) => self.outer_stagger_left(left_middle),
            (FanMode::Stagger, false) => self.inner_stagger_left(left_middle),
            (FanMode::Concurrent, _) => self.concurrent_left(left_middle),
        }
        if display_center {
            self.render_center_node(center_index);
        }
        match (mode, from_edges) {
            (FanMode::Synchronized, _) => self.synchronized_right(right_middle),
            (FanMode::Stagger, true) => self.outer_stagger_right(right_middle),
            (FanMode::Stagger, false) => self.inner_stagger_right(right_middle),
            (FanMode::Concurrent, _) => self.concurrent_right(right_middle),
        }
    }
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let tf = FixtureTargets::new(ctx);

    // UpdateFixtureCapabilities: every pan-capable fixture of every target, with its function's label and tag.
    let mut functions: HashMap<NodeId, (String, String)> = HashMap::new();
    let nodes = tf.render_nodes_for_identity_all_targets(FuncId::Pan, &mut functions);

    // _canPan: a fan needs at least two fixtures
    if nodes.len() <= 1 {
        return EffectIntents::new();
    }

    let fan = Fan::read(p, nodes.len() as i32);
    let mode = fan.fan_mode;
    let from_edges = fan.from_edges;
    let mut dance = Dance {
        ctx,
        max_increment_pan: fan.max_pan_increment(),
        pan_start: fan.pan_start_angle as f64 / 100.0,
        fan,
        nodes,
        functions,
        out: EffectIntents::new(),
        failed: false,
    };
    dance.do_rendering(mode, from_edges);
    dance.out
}

/// `CalculateStartAngle`: where a fan starts so that the fixtures can pan 360 degrees, as a share of the
/// pan range, or zero when the fixture cannot pan more than 360 degrees.
fn calculate_start_angle(tf: &FixtureTargets, first: NodeId) -> f64 {
    let Some(spec) = tf.spec(first) else { return 0.0 };
    let Some(pan) = spec.functions.iter().find(|f| f.identity == FuncId::Pan) else { return 0.0 };
    let total_degrees = (pan.rotation_limits.stop_position - pan.rotation_limits.start_position) as f64;
    if total_degrees > 360.0 {
        360.0 / total_degrees
    } else {
        0.0
    }
}

/// `_renderNodes`: every pan-capable fixture of every target.
fn render_nodes<'a>(t: &TargetInfo<'a>) -> (FixtureTargets<'a>, Vec<NodeId>) {
    let tf = FixtureTargets::from_info(*t);
    let mut functions: HashMap<NodeId, (String, String)> = HashMap::new();
    let nodes = tf.render_nodes_for_identity_all_targets(FuncId::Pan, &mut functions);
    (tf, nodes)
}

/// `TargetNodesChanged`: -1 asks for a default start angle from the first fixture's pan range. A loaded effect keeps what
/// was saved, -1 included.
fn targets_changed(p: &mut Params, t: &TargetInfo) {
    let (tf, nodes) = render_nodes(t);
    if nodes.len() > 1 && p.int("PanStartAngle") == -1 {
        p.set("PanStartAngle", ParamValue::Int((calculate_start_angle(&tf, nodes[0]) * 100.0) as i32));
    }
}

/// `UpdateAttributes`, which does nothing until the effect has been given targets.
fn visibility(p: &Params, t: &TargetInfo, v: &mut Visibility) {
    if t.targets.is_empty() {
        return;
    }
    let mode = p.enum_name("FanMode");
    v.set("CenterHandling", render_nodes(t).1.len() % 2 == 1);
    v.set("IncrementAngle", mode == "Synchronized");
    v.set("PanIncrement", mode != "Synchronized");
    v.set("PanStartAngle", p.boolean("AdvancedOverrides"));
    v.set("FanDirection", mode == "Stagger");
    v.set("PanSpeed", mode == "Concurrent");
    v.set("HoldTime", mode == "Stagger");
}

/// `GenerateVisualRepresentation`, which draws only when the targets can fan (`_canPan`).
fn visual_representation(p: &Params, t: &TargetInfo, g: &mut Graphics, rect: (i32, i32, i32, i32)) {
    let count = render_nodes(t).1.len() as i32;
    if count <= 1 {
        return;
    }
    let fan = Fan::read(p, count);
    match fan.fan_mode {
        FanMode::Stagger => {
            let mut nodes = fan.count / 2;
            if nodes == 0 {
                nodes = 1;
            }
            let width = fan.stagger_width();
            let curve = fan.stagger_curve(fan.calc_pan_increment(), width * nodes, width, false);
            visual::draw_curve(g, rect, Color::GREEN, "Fan - Stagger", 2, 1, &curve, 0);
        }
        FanMode::Concurrent => {
            let mut left_middle = fan.count / 2 - 1;
            if left_middle < 1 {
                left_middle = 1;
            }
            let curve = fan.concurrent_curve(fan.calc_pan_increment(), left_middle as f64, 1.0);
            visual::draw_curve(g, rect, Color::RED, "Fan - Concurrent", 2, 1, &curve, 0);
        }
        FanMode::Synchronized => {
            visual::draw_curve(g, rect, Color::WHITE, "Fan - Sync", 2, 1, &fan.increment_angle, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture_base::test_support::*;
    use vixen_effects::testing;

    fn set_enum(p: &mut Params, name: &str, v: &str) {
        p.set(name, ParamValue::Enum(v.to_string()));
    }

    /// The pan intents per head, in head order, as `(value at start, value at end)` of the first segment.
    fn pans(tree: &ElementTree, heads: NodeId, out: &EffectIntents) -> Vec<(f64, f64)> {
        tree.leaves(heads)
            .into_iter()
            .map(|h| {
                let e = tree.node(h).element_id.unwrap();
                let n = &out.map[&e][0];
                match &n.intent.data {
                    IntentData::Range { start, end } => (start.value, end.value),
                    other => panic!("{other:?}"),
                }
            })
            .collect()
    }

    fn setup(count: usize) -> (ElementTree, NodeId) {
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", count, &sample_spec());
        (tree, heads)
    }

    #[test]
    fn smoke() {
        // Plain elements have no fixtures, so no fan renders
        testing::smoke_test(&DESCRIPTOR);
    }

    #[test]
    fn a_single_fixture_cannot_fan() {
        let (tree, heads) = setup(1);
        assert!(testing::render_with(&DESCRIPTOR, &tree, &[heads], 1000, |_| {}).is_empty());
    }

    fn close(a: (f64, f64), b: (f64, f64)) -> bool {
        (a.0 - b.0).abs() < 1e-12 && (a.1 - b.1).abs() < 1e-12
    }

    #[test]
    fn synchronized_fan_spreads_the_heads_symmetrically_around_the_start() {
        let (tree, heads) = setup(4);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[heads], 1000, |p| {
            p.set("PanStartAngle", ParamValue::Int(0));
        });
        // IncrementAngle runs 20 -> 80, so (v - 0.5) / 0.5 is -0.6 at the start and +0.6 at the end.
        // maxIncrementPan = (1 - 0) / (4 / 2) = 0.5. Heads 0..3 have offsets +2, +1, -1, -2.
        // Negative results clamp to 0 and the rest scale with the offset.
        let v = pans(&tree, heads, &out);
        assert!(close(v[0], (0.0, 0.6)), "{:?}", v[0]);
        assert!(close(v[1], (0.0, 0.3)), "{:?}", v[1]);
        assert!(close(v[2], (0.3, 0.0)), "{:?}", v[2]);
        assert!(close(v[3], (0.6, 0.0)), "{:?}", v[3]);
    }

    #[test]
    fn invert_pan_mirrors_the_fan() {
        let (tree, heads) = setup(4);
        let normal = testing::render_with(&DESCRIPTOR, &tree, &[heads], 1000, |p| {
            p.set("PanStartAngle", ParamValue::Int(50));
        });
        let inverted = testing::render_with(&DESCRIPTOR, &tree, &[heads], 1000, |p| {
            p.set("PanStartAngle", ParamValue::Int(50));
            p.set("InvertPan", ParamValue::Bool(true));
        });
        let (a, b) = (pans(&tree, heads, &normal), pans(&tree, heads, &inverted));
        // PanStartAngle 50: initial value 0.5, so offsets of opposite sign land on opposite sides of it
        for i in 0..4 {
            assert!(((a[i].1 - 0.5) + (b[i].1 - 0.5)).abs() < 1e-12, "head {i}: {:?} vs {:?}", a[i], b[i]);
        }
    }

    #[test]
    fn center_beam_of_an_odd_fan_uses_offset_zero() {
        let (tree, heads) = setup(3);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[heads], 1000, |p| {
            p.set("PanStartAngle", ParamValue::Int(30));
        });
        let v = pans(&tree, heads, &out);
        // The middle head stays at the initial value for the whole effect
        assert_eq!(v[1], (0.3, 0.3));
        assert_ne!(v[0].0, v[0].1);
    }

    #[test]
    fn default_start_angle_comes_from_the_fixtures_pan_range() {
        // A fixture that pans 540 degrees starts the fan at 360/540 = 66%
        let mut spec = sample_spec();
        spec.functions.iter_mut().find(|f| f.name == "Pan").unwrap().rotation_limits =
            crate::spec::FixtureRotationLimits { start_position: 0, stop_position: 540 };
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 2, &spec);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[heads], 1000, |_| {});
        // maxIncrementPan = (1 - 0.66) / 1; the pan start is the 0.66 default
        let v = pans(&tree, heads, &out);
        let expected_start = limit_angle(0.66 + (((0.2 - 0.5) / 0.5) * 0.34) * 1.0);
        assert!((v[0].0 - expected_start).abs() < 1e-12, "{:?}", v[0]);
        let flat = testing::render_with(&DESCRIPTOR, &tree, &[heads], 1000, |p| {
            p.set("PanStartAngle", ParamValue::Int(0));
        });
        assert_ne!(pans(&tree, heads, &flat)[0], v[0]);
    }

    #[test]
    fn stagger_from_the_center_holds_the_outer_head_while_the_inner_one_moves() {
        let (tree, heads) = setup(4);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[heads], 1000, |p| {
            set_enum(p, "FanMode", "Stagger");
            set_enum(p, "FanDirection", "FanFromCenter");
            p.set("PanStartAngle", ParamValue::Int(0));
        });
        // width = 100 / 2 = 50 and panIncrement = 10, so the curves peak at 60: (60 - 50) / 50 = 0.2 of
        // maxIncrementPan 0.5, times the head's offset.
        let segments = |i: usize| {
            let e = tree.node(tree.leaves(heads)[i]).element_id.unwrap();
            out.map[&e]
                .iter()
                .map(|n| match &n.intent.data {
                    IntentData::Range { start, end } => (start.value, end.value),
                    other => panic!("{other:?}"),
                })
                .collect::<Vec<_>>()
        };
        // Head 1 is innermost (offset 1): moves during the first half, then holds
        let inner = segments(1);
        assert_eq!(inner.len(), 2);
        assert!(close(inner[0], (0.0, 0.1)), "{:?}", inner);
        assert!(close(inner[1], (0.1, 0.1)), "{:?}", inner);
        // Head 0 is outermost (offset 2): holds the start for the first half, then moves
        let outer = segments(0);
        assert_eq!(outer.len(), 2);
        assert!(close(outer[0], (0.0, 0.0)), "{:?}", outer);
        assert!(close(outer[1], (0.0, 0.2)), "{:?}", outer);
    }

    #[test]
    fn concurrent_at_full_speed_jumps_to_the_top() {
        let (tree, heads) = setup(2);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[heads], 1000, |p| {
            set_enum(p, "FanMode", "Concurrent");
            p.set("PanSpeed", ParamValue::Int(100));
            p.set("PanStartAngle", ParamValue::Int(0));
        });
        // CreateConcurrentCurve with PanSpeed 100 is flat at 50 + panIncrement (20% of 50 = 10)
        let v = pans(&tree, heads, &out);
        let expected = limit_angle((((0.6 - 0.5) / 0.5) * 1.0) * 1.0);
        assert!((v[0].0 - expected).abs() < 1e-12 && (v[0].1 - expected).abs() < 1e-12, "{:?}", v[0]);
    }

    #[test]
    fn pan_increment_never_rounds_to_zero() {
        let mut fan = Fan::read(&DESCRIPTOR.new_params(), 4);
        fan.pan_increment = 1;
        assert_eq!(fan.calc_pan_increment(), 1);
        fan.pan_increment = 100;
        assert_eq!(fan.calc_pan_increment(), 50);
        fan.pan_increment = 20;
        assert_eq!(fan.calc_pan_increment(), 10);
    }

    #[test]
    fn an_odd_fan_offers_center_handling() {
        for (count, shown) in [(2, false), (3, true), (4, false)] {
            let (tree, heads) = setup(count);
            let targets = [heads];
            let names = testing::visible_names(&DESCRIPTOR, &DESCRIPTOR.new_params(), &testing::target_info(&tree, &targets));
            assert_eq!(names.contains(&"CenterHandling"), shown, "{count}");
        }
    }

    #[test]
    fn only_assigning_targets_calculates_the_start_angle() {
        let mut spec = sample_spec();
        let pan = spec.functions.iter_mut().find(|f| f.identity == FuncId::Pan).unwrap();
        pan.rotation_limits = crate::spec::FixtureRotationLimits { start_position: 0, stop_position: 540 };
        let mut tree = ElementTree::new();
        let heads = fixtures(&mut tree, "Heads", 2, &spec);
        let targets = [heads];
        let t = testing::target_info(&tree, &targets);
        let mut p = DESCRIPTOR.new_params();
        p.set("PanStartAngle", ParamValue::Int(-1));
        let mut loaded = p.clone();
        DESCRIPTOR.load_data(&mut loaded, &t);
        assert_eq!(loaded.int("PanStartAngle"), -1);
        DESCRIPTOR.set_targets(&mut p, &t);
        assert_eq!(p.int("PanStartAngle"), 66);
    }

    #[test]
    fn visibility_depends_on_the_fan_mode() {
        let (tree, heads) = setup(2);
        let targets = [heads];
        let t = testing::target_info(&tree, &targets);
        let names = |p: &Params| -> Vec<&'static str> { testing::visible_names(&DESCRIPTOR, p, &t) };
        let mut p = DESCRIPTOR.new_params();
        let sync = names(&p);
        assert!(sync.contains(&"IncrementAngle") && !sync.contains(&"PanIncrement") && !sync.contains(&"HoldTime"));
        set_enum(&mut p, "FanMode", "Stagger");
        let stagger = names(&p);
        assert!(stagger.contains(&"HoldTime") && stagger.contains(&"FanDirection") && !stagger.contains(&"PanSpeed"));
        set_enum(&mut p, "FanMode", "Concurrent");
        assert!(names(&p).contains(&"PanSpeed"));
        assert!(!names(&p).contains(&"PanStartAngle"));
        p.set("AdvancedOverrides", ParamValue::Bool(true));
        assert!(names(&p).contains(&"PanStartAngle"));
    }

    #[test]
    fn thumbnails_draw_for_every_fan_mode() {
        for mode in ["Synchronized", "Stagger", "Concurrent"] {
            let mut p = DESCRIPTOR.new_params();
            set_enum(&mut p, "FanMode", mode);
            let mut g = Graphics::new(200, 40);
            let (tree, heads) = setup(4);
            let targets = [heads];
            (DESCRIPTOR.visual.unwrap())(&p, &testing::target_info(&tree, &targets), &mut g, (0, 0, 200, 40));
            assert!(crate::visual::has_ink(&g), "{mode}");
            // Nothing to fan: nothing on the timeline either.
            let mut g = Graphics::new(200, 40);
            (DESCRIPTOR.visual.unwrap())(&p, &TargetInfo::detached(), &mut g, (0, 0, 200, 40));
            assert!(!crate::visual::has_ink(&g), "{mode}");
        }
    }
}
