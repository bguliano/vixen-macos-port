//! Port of VixenModules.Effect.Pulse.

use crate::generated::pulse as gen;
use vixen_effects::prelude::*;
use vixen_effects::pulse_renderer;

pub static DESCRIPTOR: EffectDescriptor =
    EffectDescriptor { render, validate: Some(validate), ported: true, ..gen::BASE };

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let curve = p.curve("LevelCurve");
    let gradient = p.gradient("ColorGradient");
    let has_discrete = ctx.has_discrete_colors();
    let mut out = EffectIntents::new();
    for &node in ctx.targets {
        if ctx.cancelled() {
            return out;
        }
        out = pulse_renderer::render_node(ctx.tree, node, &curve, &gradient, ctx.time_span, has_discrete, false);
    }
    out
}

/// `CheckForInvalidColorData`.
fn validate(p: &mut Params, valid: &[Color]) {
    if !valid.is_empty() {
        let in_gradient = p.gradient("ColorGradient").colors_in_gradient();
        if !in_gradient.iter().all(|c| valid.contains(c)) {
            p.set("ColorGradient", ParamValue::Gradient(ColorGradient::solid(valid[0])));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_effects::testing;
    use vixen_core::StateValue;

    #[test]
    fn smoke() {
        testing::smoke_test(&DESCRIPTOR);
        testing::assert_produces_light(&DESCRIPTOR);
    }

    #[test]
    fn default_curve_ramps_up() {
        let mut tree = ElementTree::new();
        let n = tree.add_element("E");
        let id = tree.node(n).element_id.unwrap();
        let out = testing::render_with(&DESCRIPTOR, &tree, &[n], 1000, |_| {});
        let node = &out.nodes_for(&id).unwrap()[0];
        match node.intent.state_at(TimeSpan::from_ms_i(500)) {
            StateValue::Lighting(v) => assert!((v.intensity - 0.5).abs() < 1e-9),
            other => panic!("{other:?}"),
        }
    }
}
