//! Port of VixenModules.Effect.CustomValue.CustomValueModule.
//!
//! vixen-core has no string or multi-byte command value, so only the 8-bit value is a `Command` intent. The
//! Each value type maps to its `CommandValue` variant.

use crate::common::draw_device_label;
use crate::generated::custom_value as gen;
use vixen_core::value::CommandValue;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    force_visual: true,
    visual: Some(visual),
    ported: true,
    ..gen::BASE
};

fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    let t = p.enum_name("ValueType");
    v.set("Value8Bit", t == "_8Bit");
    v.set("Value16Bit", t == "_16Bit");
    v.set("Value32Bit", t == "_32Bit");
    v.set("Value64Bit", t == "_64Bit");
    v.set("StringValue", t == "String");
    v.set("ColorValue", t == "Color");
}

/// The 64-bit value is kept as the raw XML text of an unmodelled member.
fn value64(p: &Params) -> u64 {
    match p.get("Value64Bit") {
        ParamValue::Raw(s) | ParamValue::Str(s) => s.trim().parse().unwrap_or(0),
        ParamValue::Long(v) => *v as u64,
        ParamValue::Int(v) => *v as u64,
        _ => 0,
    }
}

fn value32(p: &Params) -> u32 {
    match p.get("Value32Bit") {
        ParamValue::Long(v) => *v as u32,
        ParamValue::Int(v) => *v as u32,
        _ => 0,
    }
}

/// `Color.ToString()` for a colour that came from a file: `Color [A=255, R=255, G=255, B=255]`.
fn color_text(c: Color) -> String {
    format!("Color [A={}, R={}, G={}, B={}]", c.a, c.r, c.g, c.b)
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let mut out = EffectIntents::new();
    let ts = ctx.time_span;
    let command = match p.enum_name("ValueType") {
        "_16Bit" => CommandValue::U16(p.int("Value16Bit") as u16),
        "_32Bit" => CommandValue::U32(value32(p) as u32),
        "_64Bit" => CommandValue::U64(value64(p) as u64),
        "Color" => CommandValue::Color(p.color("ColorValue")),
        "String" => CommandValue::Str(std::sync::Arc::from(p.string("StringValue"))),
        _ => CommandValue::U8(p.int("Value8Bit") as u8),
    };
    let data = IntentData::Command(command);
    for &node in ctx.targets {
        for leaf in ctx.tree.leaves(node) {
            if ctx.cancelled() {
                return EffectIntents::new();
            }
            if let Some(e) = ctx.tree.node(leaf).element_id {
                out.add_intent_for_element(e, Intent::new(data.clone(), ts), TimeSpan::ZERO);
            }
        }
    }
    out
}

/// `GenerateVisualRepresentation`.
fn visual(p: &Params, _t: &TargetInfo, g: &mut vixen_effects::gfx::Graphics, clip: (i32, i32, i32, i32)) {
    let value = match p.enum_name("ValueType") {
        "_16Bit" => (p.int("Value16Bit") as u16).to_string(),
        "_32Bit" => value32(p).to_string(),
        "_64Bit" => value64(p).to_string(),
        "Color" => color_text(p.color("ColorValue")),
        "String" => p.string("StringValue").to_string(),
        _ => (p.int("Value8Bit") as u8).to_string(),
    };
    draw_device_label(g, clip, &format!("Custom Value - {value}"), Color::DARK_GRAY);
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
    fn eight_bit_value_is_a_command_on_every_leaf() {
        let mut tree = ElementTree::new();
        let s = tree.add_string("S", 3);
        let out = testing::render_with(&DESCRIPTOR, &tree, &[s], 800, |p| {
            p.set("Value8Bit", ParamValue::Int(77));
        });
        assert_eq!(out.map.len(), 3);
        for nodes in out.map.values() {
            assert_eq!(nodes[0].start, TimeSpan::ZERO);
            assert_eq!(nodes[0].intent.span, TimeSpan::from_ms_i(800));
            assert_eq!(nodes[0].intent.state_at(TimeSpan::ZERO), StateValue::Command(CommandValue::U8(77)));
        }
    }

    #[test]
    fn wide_values_keep_their_exact_value() {
        let mut tree = ElementTree::new();
        let e = tree.add_element("E");
        let out = testing::render_with(&DESCRIPTOR, &tree, &[e], 100, |p| {
            p.set("ValueType", ParamValue::Enum("_64Bit".into()));
            p.set("Value64Bit", ParamValue::Raw("18446744073709551615".into()));
        });
        match out.map.values().next().unwrap()[0].intent.state_at(TimeSpan::ZERO) {
            StateValue::Command(c) => assert_eq!(c, CommandValue::U64(18446744073709551615)),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn thumbnail_is_drawn_and_visibility_tracks_the_type() {
        let mut g = vixen_effects::gfx::Graphics::new(120, 30);
        visual(&DESCRIPTOR.new_params(), &TargetInfo::detached(), &mut g, (0, 0, 120, 30));
        assert_eq!(g.bitmap().get_pixel(100, 25), Color::DARK_GRAY);
        let mut p = DESCRIPTOR.new_params();
        p.set("ValueType", ParamValue::Enum("String".into()));
        let mut v = Visibility::default();
        visibility(&p, &TargetInfo::detached(), &mut v);
        assert!(v.overrides.contains(&("StringValue", true)));
        assert!(v.overrides.contains(&("Value8Bit", false)));
    }
}
