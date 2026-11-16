//! Effect modules: border, count_down, glediator, picture, shapes, snowflakes, text, video. Generated tables live in `generated`.

pub mod generated;
pub mod border;
pub mod count_down;
pub mod glediator;
pub mod picture;
pub mod shapes;
pub mod snowflakes;
pub mod text;
pub mod video;
pub(crate) mod svgdoc;
pub(crate) mod textdraw;
pub(crate) mod shape_assets;

#[cfg(test)]
mod raster_capture;

use vixen_effects::framework::EffectDescriptor;

pub fn descriptors() -> Vec<&'static EffectDescriptor> {
    vec![&border::DESCRIPTOR, &count_down::DESCRIPTOR, &glediator::DESCRIPTOR, &picture::DESCRIPTOR, &shapes::DESCRIPTOR, &snowflakes::DESCRIPTOR, &text::DESCRIPTOR, &video::DESCRIPTOR, ]
}

#[cfg(test)]
pub(crate) mod testutil {
    use vixen_effects::prelude::*;

    /// The colour of leaf `pixel` of string `string` of a `testing::grid` group at `ms`, black when unlit.
    pub fn color_at(tree: &ElementTree, group: NodeId, out: &EffectIntents, string: usize, pixel: usize, ms: i64) -> (u8, u8, u8) {
        let strings = &tree.node(group).children;
        let leaves = &tree.node(strings[string]).children;
        let id = tree.node(leaves[pixel]).element_id.unwrap();
        let Some(nodes) = out.nodes_for(&id) else { return (0, 0, 0) };
        for n in nodes {
            let rel = TimeSpan::from_ms_i(ms) - n.start;
            if let vixen_core::StateValue::Rgb(v) = n.intent.state_at(rel) {
                return (v.r, v.g, v.b);
            }
        }
        (0, 0, 0)
    }

    pub fn lit_count(tree: &ElementTree, group: NodeId, out: &EffectIntents, ms: i64) -> usize {
        let mut n = 0;
        let strings = tree.node(group).children.clone();
        for (s, st) in strings.iter().enumerate() {
            for p in 0..tree.node(*st).children.len() {
                let c = color_at(tree, group, out, s, p, ms);
                if c != (0, 0, 0) {
                    n += 1;
                }
            }
        }
        n
    }
}

/// `Orientation` and `StringOrientation` are one data member in Picture, Video and Glediator, and `EffectDescriptor`'s
/// generic target rule only knows the second. This is the rest of `TargetNodesChanged` for them.
pub(crate) fn orientation_targets_changed(p: &mut vixen_effects::framework::Params, t: &vixen_effects::framework::TargetInfo) {
    let followed_target = p.enum_name("TargetPositioning") != "Locations" && t.targets.first().and_then(|n| t.tree.orientation(*n)).is_some();
    if followed_target {
        copy_param(p, "StringOrientation", "Orientation");
    } else {
        copy_param(p, "Orientation", "StringOrientation");
    }
}

/// The same member's `ModuleData` and `TargetPositioning` setter rules, for `param_changed_ctx`.
pub(crate) fn orientation_param_changed(c: &vixen_effects::framework::ParamChange, p: &mut vixen_effects::framework::Params) {
    match c.name {
        vixen_effects::framework::MODULE_DATA => copy_param(p, "Orientation", "StringOrientation"),
        "TargetPositioning" => {
            if p.enum_name("TargetPositioning") == "Locations" {
                copy_param(p, "StringOrientation", "Orientation");
            } else {
                orientation_targets_changed(p, c.info);
            }
        }
        _ => {}
    }
}

fn copy_param(p: &mut vixen_effects::framework::Params, from: &str, to: &str) {
    let value = p.get(from).clone();
    p.set(to, value);
}
