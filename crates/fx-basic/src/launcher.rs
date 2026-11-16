//! Port of the module. The command is a `CommandValue::Str`, whose text the matching controller reads.

use crate::common::draw_device_label;
use crate::generated::launcher as gen;
use vixen_core::value::CommandValue;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor =
    EffectDescriptor { render, force_visual: true, visual: Some(visual), ported: true, ..gen::BASE };

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let mut out = EffectIntents::new();
    let text = format!("Launcher|{},{}", p.string("Executable"), p.string("Arguments"));
    let value = CommandValue::Str(std::sync::Arc::from(text.as_str()));
    for &node in ctx.targets {
        for leaf in ctx.tree.leaves(node) {
            if ctx.cancelled() {
                return EffectIntents::new();
            }
            if let Some(e) = ctx.tree.node(leaf).element_id {
                out.add_intent_for_element(e, Intent::new(IntentData::Command(value.clone()), ctx.time_span), TimeSpan::ZERO);
            }
        }
    }
    out
}

fn visual(p: &Params, _t: &TargetInfo, g: &mut vixen_effects::gfx::Graphics, clip: (i32, i32, i32, i32)) {
    // System.Drawing.Color.DarkBlue
    draw_device_label(g, clip, &format!("Launcher - {}", p.string("Description")), Color::rgb(0, 0, 139));
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
    fn command_text_joins_executable_and_arguments() {
        let mut tree = ElementTree::new();
        let e = tree.add_element("E");
        let out = testing::render_with(&DESCRIPTOR, &tree, &[e], 500, |p| {
            p.set("Executable", ParamValue::Str("/bin/echo".into()));
            p.set("Arguments", ParamValue::Str("hi".into()));
        });
        let n = &out.map.values().next().unwrap()[0];
        assert_eq!(n.intent.span, TimeSpan::from_ms_i(500));
        match n.intent.state_at(TimeSpan::ZERO) {
            StateValue::Command(CommandValue::Str(text)) => assert_eq!(&*text, "Launcher|/bin/echo,hi"),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn thumbnail_is_dark_blue() {
        let mut g = vixen_effects::gfx::Graphics::new(100, 30);
        visual(&DESCRIPTOR.new_params(), &TargetInfo::detached(), &mut g, (0, 0, 100, 30));
        assert_eq!(g.bitmap().get_pixel(90, 28), Color::rgb(0, 0, 139));
    }
}
