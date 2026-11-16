//! Port of VixenModules.Effect.Candle.CandleModule.

use crate::generated::candle as gen;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor { render, validate: Some(validate), ported: true, ..gen::BASE };

/// `CheckForInvalidColorData`.
fn validate(p: &mut Params, valid: &[Color]) {
    if !valid.is_empty() && !valid.contains(&p.color("Color")) {
        p.set("Color", ParamValue::Color(valid[0]));
    }
}

struct Candle<'a> {
    ctx: &'a RenderCtx<'a>,
    rand: Rand,
    color: Color,
    has_discrete: bool,
    flicker_frequency: i32,
    // The C# `Percentage` and the generator maths are `float`.
    change_percentage: f32,
    min_level: f32,
    max_level: f32,
    frequency_deviation_cap: f32,
    change_deviation_cap: f32,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let mut out = EffectIntents::new();
    if ctx.targets.is_empty() {
        return out;
    }
    let mut c = Candle {
        ctx,
        rand: Rand::default(),
        color: p.color("Color"),
        has_discrete: ctx.has_discrete_colors(),
        flicker_frequency: p.int("FlickerFrequency"),
        change_percentage: p.double("ChangePercentage") as f32,
        min_level: p.double("MinLevel") as f32,
        max_level: p.double("MaxLevel") as f32,
        frequency_deviation_cap: p.double("FlickerFrequencyDeviationCap") as f32,
        change_deviation_cap: p.double("ChangePercentageDeviationCap") as f32,
    };
    // A negative frequency makes every state run backwards, and the C# loop never finishes.
    if c.flicker_frequency < 0 {
        return out;
    }
    let group = p.int("GroupLevel").max(1) as usize; // CandleData clamps this on load
    let mut nodes: Vec<NodeId> = Vec::new();
    for &t in ctx.targets {
        // GetLeafEnumerator().Distinct() per target
        nodes.extend(ctx.tree.distinct_leaves(t));
    }
    for block in nodes.chunks(group) {
        if ctx.cancelled() {
            return EffectIntents::new();
        }
        // An exception ends the render, and what was already built is what plays.
        if !c.render_block(block, &mut out) {
            break;
        }
    }
    out
}

impl Candle<'_> {
    /// `_RenderCandleOnElements`. False where the C# throws.
    fn render_block(&mut self, elements: &[NodeId], out: &mut EffectIntents) -> bool {
        let ctx = self.ctx;
        let ts = ctx.time_span;
        let mut start = TimeSpan::ZERO;
        let mut current_level = self.starting_level() as f64;
        while start < ts {
            let change = self.level_change();
            let mut next_level = current_level + change as f64;
            next_level = next_level.max(self.min_level as f64);
            next_level = next_level.min(self.max_level as f64);
            let mut state_length = self.state_length() as f64;
            // A frequency of 0 makes this infinite, or NaN if the deviation isn't positive. Infinite runs to the end of the
            // effect. `TimeSpan.FromMilliseconds(NaN)` throws. (`f64::min` would hide the NaN that `Math.Min` keeps.)
            if state_length.is_nan() {
                return false;
            }
            state_length = state_length.min((ts - start).total_ms());
            let mut length = TimeSpan::from_ms(state_length);
            if length == TimeSpan::ZERO {
                length = TimeSpan::from_ms(1.0);
            } else {
                let mut discrete: Option<Intent> = None;
                let mut full: Option<Intent> = None;
                for &element in elements {
                    // A childless group has no Element, so `.Element.Id` throws.
                    let Some(id) = ctx.tree.node(element).element_id else { return false };
                    if self.has_discrete && ctx.tree.is_discrete(element) {
                        let intent = discrete.get_or_insert_with(|| Intent::discrete(self.color, current_level, next_level, length));
                        out.add_intent_for_element(id, intent.clone(), start);
                    } else {
                        let intent =
                            full.get_or_insert_with(|| Intent::lighting(self.color, self.color, current_level, next_level, length));
                        out.add_intent_for_element(id, intent.clone(), start);
                    }
                }
            }
            start += length;
            current_level = next_level;
        }
        true
    }

    fn starting_level(&mut self) -> f32 {
        let mut level = self.rand.next_double() as f32;
        level = level.max(self.min_level);
        level.min(self.max_level)
    }

    fn state_length(&mut self) -> f32 {
        let state_length = 1000f32 / self.flicker_frequency as f32;
        let cap = (self.frequency_deviation_cap * 100.0) as i32;
        let deviation = self.rand.next_range(-cap, cap) as f32 * 0.01f32;
        state_length + state_length * deviation
    }

    fn level_change(&mut self) -> f32 {
        let change = self.change_percentage;
        let cap = (self.change_deviation_cap * 100.0) as i32;
        let deviation = self.rand.next_range(-cap, cap) as f32 * 0.01f32;
        let direction = self.rand.next_range(-1, 2);
        (change + change * deviation) * direction as f32
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
    fn levels_stay_within_bounds_and_cover_the_whole_effect() {
        let mut tree = ElementTree::new();
        let e = tree.add_element("E");
        let out = testing::render_with(&DESCRIPTOR, &tree, &[e], 3000, |p| {
            p.set("MinLevel", ParamValue::Double(0.3));
            p.set("MaxLevel", ParamValue::Double(0.7));
        });
        let nodes = out.nodes_for(&tree.node(e).element_id.unwrap()).unwrap();
        assert_eq!(nodes[0].start, TimeSpan::ZERO);
        assert!(nodes.last().unwrap().end >= TimeSpan::from_ms_i(2999));
        for n in nodes {
            assert!(n.start <= n.end);
            for t in [TimeSpan::ZERO, n.intent.span] {
                match n.intent.state_at(t) {
                    StateValue::Lighting(v) => {
                        assert!(v.intensity >= 0.3f32 as f64 - 1e-6 && v.intensity <= 0.7f32 as f64 + 1e-6, "{}", v.intensity)
                    }
                    o => panic!("{o:?}"),
                }
            }
        }
    }

    #[test]
    fn group_level_shares_intents_across_elements() {
        let mut tree = ElementTree::new();
        let s = tree.add_string("S", 4);
        let leaves = tree.distinct_leaves(s);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[s], 1000, |p| {
            p.set("GroupLevel", ParamValue::Int(2));
        });
        let times = |i: usize| -> Vec<i64> {
            out.nodes_for(&tree.node(leaves[i]).element_id.unwrap()).unwrap().iter().map(|n| n.start.ticks()).collect()
        };
        assert_eq!(times(0), times(1));
        assert_ne!(times(0), times(2));
    }

    #[test]
    fn zero_frequency_with_no_deviation_throws_and_stays_dark() {
        let mut tree = ElementTree::new();
        let e = tree.add_element("E");
        // Infinity plus infinity times zero is NaN, which `TimeSpan.FromMilliseconds` refuses.
        assert!(testing::render_with(&DESCRIPTOR, &tree, &[e], 1000, |p| {
            p.set("FlickerFrequency", ParamValue::Int(0));
            p.set("FlickerFrequencyDeviationCap", ParamValue::Double(0.0));
        }).is_empty());
    }

    #[test]
    fn zero_frequency_with_a_positive_deviation_is_one_fade_over_the_whole_effect() {
        let mut tree = ElementTree::new();
        let e = tree.add_element("E");
        let id = tree.node(e).element_id.unwrap();
        let mut lit = 0;
        for _ in 0..200 {
            let out = testing::render_with(&DESCRIPTOR, &tree, &[e], 1000, |p| {
                p.set("FlickerFrequency", ParamValue::Int(0));
            });
            if let Some(nodes) = out.nodes_for(&id) {
                lit += 1;
                assert_eq!(nodes.len(), 1);
                assert_eq!(nodes[0].intent.span, TimeSpan::from_ms_i(1000));
            }
        }
        assert!(lit > 0 && lit < 200, "{lit}");
    }

    #[test]
    fn a_negative_frequency_is_dark_not_a_hang() {
        let mut tree = ElementTree::new();
        let e = tree.add_element("E");
        assert!(testing::render_with(&DESCRIPTOR, &tree, &[e], 1000, |p| {
            p.set("FlickerFrequency", ParamValue::Int(-5));
        }).is_empty());
    }

    #[test]
    fn an_empty_group_stops_the_render_where_it_stands() {
        let mut tree = ElementTree::new();
        let group = tree.add_node(Uuid::new_v4(), "G", None);
        let a = tree.add_element("A");
        let empty = tree.add_node(Uuid::new_v4(), "Empty", None);
        let b = tree.add_element("B");
        tree.add_child(group, a);
        tree.add_child(group, empty);
        tree.add_child(group, b);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[group], 1000, |p| {
            p.set("GroupLevel", ParamValue::Int(10));
        });
        assert_eq!(out.nodes_for(&tree.node(a).element_id.unwrap()).unwrap().len(), 1);
        assert!(out.nodes_for(&tree.node(b).element_id.unwrap()).is_none());
    }
}
