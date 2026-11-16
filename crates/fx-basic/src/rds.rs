//! Port of VixenModules.Effect.RDS.RDSModule. The command is a string command; vixen-core has none, so it is a
//! Port of the module. The command is a `CommandValue::Str`, whose text the matching controller reads.

use crate::common::draw_device_label;
use crate::generated::rds as gen;
use vixen_core::value::CommandValue;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor =
    EffectDescriptor { render, force_visual: true, visual: Some(visual), ported: true, ..gen::BASE };

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let mut out = EffectIntents::new();
    let value = CommandValue::Str(std::sync::Arc::from(format!("RDS|{}", p.string("Title"))));
    // The C# uses `node.Element.Id` on the target itself (no leaf walk), so a group target throws, and only the targets
    // before it keep their intents.
    for &node in ctx.targets {
        if ctx.cancelled() {
            return EffectIntents::new();
        }
        let Some(e) = ctx.tree.node(node).element_id else { return out };
        out.add_intent_for_element(e, Intent::new(IntentData::Command(value.clone()), ctx.time_span), TimeSpan::ZERO);
    }
    out
}

fn visual(p: &Params, _t: &TargetInfo, g: &mut vixen_effects::gfx::Graphics, clip: (i32, i32, i32, i32)) {
    draw_device_label(g, clip, &format!("RDS - {}", p.string("Title")), Color::DARK_GRAY);
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_core::StateValue;
    use vixen_effects::testing;

    #[test]
    fn smoke() {
        testing::smoke_test(&DESCRIPTOR);
    }

    #[test]
    fn sends_title_to_each_element_target() {
        let mut tree = ElementTree::new();
        let e = tree.add_element("E");
        let out = testing::render_with(&DESCRIPTOR, &tree, &[e], 500, |p| {
            p.set("Title", ParamValue::Str("Song".into()));
        });
        let n = &out.nodes_for(&tree.node(e).element_id.unwrap()).unwrap()[0];
        match n.intent.state_at(TimeSpan::ZERO) {
            StateValue::Command(CommandValue::Str(text)) => assert_eq!(&*text, "RDS|Song"),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn group_target_throws_in_csharp_so_stays_dark() {
        let mut tree = ElementTree::new();
        let s = tree.add_string("S", 3);
        assert!(testing::render_with(&DESCRIPTOR, &tree, &[s], 500, |_| {}).is_empty());
    }

    #[test]
    fn thumbnail_is_drawn() {
        let mut g = vixen_effects::gfx::Graphics::new(100, 30);
        visual(&DESCRIPTOR.new_params(), &TargetInfo::detached(), &mut g, (0, 0, 100, 30));
        assert_eq!(g.bitmap().get_pixel(90, 28), Color::DARK_GRAY);
    }
}
