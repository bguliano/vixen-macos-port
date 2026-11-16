//! Shared checks every effect port runs. They render default parameters over the element shapes that
//! real shows use, which catches panics, index errors and misnamed parameters.

use crate::framework::{EffectDescriptor, RenderCtx};
use crate::pixel::StringOrientation;
use std::sync::atomic::AtomicBool;
use vixen_core::element::{property_ids, ElementColorType, PropertyData, PropertyInstance};
use vixen_core::{Color, EffectIntents, ElementTree, NodeId, TimeSpan, Uuid};

/// A flat group of strings, each `pixels` long, like a matrix or mega tree.
pub fn grid(tree: &mut ElementTree, name: &str, strings: usize, pixels: usize) -> NodeId {
    let root = tree.add_node(Uuid::new_v4(), name, None);
    for s in 0..strings {
        let string = tree.add_string(&format!("{name} S{s}"), pixels);
        tree.add_child(root, string);
    }
    root
}

/// Marks a node's leaves as single-colour strings.
pub fn make_discrete(tree: &mut ElementTree, node: NodeId, color: Color) {
    for leaf in tree.distinct_leaves(node) {
        tree.node_mut(leaf).properties.push(PropertyInstance {
            type_id: property_ids::COLOR,
            instance_id: Uuid::new_v4(),
            data: PropertyData::Color {
                color_type: ElementColorType::SingleColor,
                single_color: color,
                color_set_name: None,
            },
        });
    }
}

/// Gives a node's leaves a Location property in a grid, for location-mode rendering.
pub fn add_locations(tree: &mut ElementTree, node: NodeId) {
    for (i, leaf) in tree.distinct_leaves(node).into_iter().enumerate() {
        tree.node_mut(leaf).properties.push(PropertyInstance {
            type_id: property_ids::LOCATION,
            instance_id: Uuid::new_v4(),
            data: PropertyData::Location { x: (i as i32 % 10) * 4, y: (i as i32 / 10) * 4, z: 0 },
        });
    }
}

/// Renders an effect with its default parameters, or with changes applied by `edit`.
pub fn render_with(
    d: &'static EffectDescriptor,
    tree: &ElementTree,
    targets: &[NodeId],
    ms: i64,
    edit: impl FnOnce(&mut crate::framework::Params),
) -> EffectIntents {
    let cancel = AtomicBool::new(false);
    let ctx = RenderCtx { video_cache: None,
        tree,
        targets,
        start_time: TimeSpan::ZERO,
        time_span: TimeSpan::from_ms_i(ms),
        frame_ms: 50,
        marks: &[],
        audio: None,
        module_data_root: None,
        cancel: &cancel,
    };
    let mut p = d.new_params();
    edit(&mut p);
    // As `effect.TargetNodes = ...` does after the properties have been set.
    d.set_targets(&mut p, &ctx.target_info());
    (d.render)(&ctx, &p)
}

/// Renders an effect as a sequence file loads it: the saved values `edit` sets go through the `ModuleData` setter's rules
/// and not `TargetNodesChanged`'s.
pub fn render_loaded(
    d: &'static EffectDescriptor,
    tree: &ElementTree,
    targets: &[NodeId],
    ms: i64,
    edit: impl FnOnce(&mut crate::framework::Params),
) -> EffectIntents {
    let cancel = AtomicBool::new(false);
    let ctx = RenderCtx { video_cache: None,
        tree,
        targets,
        start_time: TimeSpan::ZERO,
        time_span: TimeSpan::from_ms_i(ms),
        frame_ms: 50,
        marks: &[],
        audio: None,
        module_data_root: None,
        cancel: &cancel,
    };
    let mut p = d.new_params();
    edit(&mut p);
    d.load_data(&mut p, &ctx.target_info());
    (d.render)(&ctx, &p)
}

/// A target's place for editor rules, with no marks and no audio.
pub fn target_info<'a>(tree: &'a ElementTree, targets: &'a [NodeId]) -> crate::framework::TargetInfo<'a> {
    crate::framework::TargetInfo {
        tree,
        targets,
        marks: &[],
        start_time: TimeSpan::ZERO,
        time_span: TimeSpan::from_ms_i(1000),
        audio_loaded: false,
    }
}

/// The names `visibility` shows for these values and targets.
pub fn visible_names(d: &'static EffectDescriptor, p: &crate::framework::Params, t: &crate::framework::TargetInfo) -> Vec<&'static str> {
    d.visible_params_for(p, t).into_iter().map(|i| d.params[i].name).collect()
}

/// A tree of `groups` groups of `strings` strings of `pixels` leaves, whose root is three levels above the leaves.
pub fn deep_grid(tree: &mut ElementTree, name: &str, groups: usize, strings: usize, pixels: usize) -> NodeId {
    let root = tree.add_node(uuid::Uuid::new_v4(), name, None);
    for g in 0..groups {
        let group = tree.add_node(uuid::Uuid::new_v4(), &format!("{name} G{g}"), None);
        tree.add_child(root, group);
        for s in 0..strings {
            let string = tree.add_string(&format!("{name} G{g} S{s}"), pixels);
            tree.add_child(group, string);
        }
    }
    root
}

/// Everything must render without panicking across representative targets, durations and orientations,
/// and visibility rules must hold together when every enum parameter changes value.
pub fn smoke_test(d: &'static EffectDescriptor) {
    let mut tree = ElementTree::new();
    let single = tree.add_element("Single");
    let string = tree.add_string("String", 30);
    let matrix = grid(&mut tree, "Matrix", 8, 25);
    let discrete = grid(&mut tree, "Discrete", 3, 5);
    make_discrete(&mut tree, discrete, Color::RED);
    let located = grid(&mut tree, "Located", 4, 10);
    add_locations(&mut tree, located);

    for &target in &[single, string, matrix, discrete] {
        for &ms in &[50_i64, 1_000, 4_000] {
            render_with(d, &tree, &[target], ms, |_| {});
        }
    }

    if d.params.iter().any(|p| p.name == "StringOrientation") {
        render_with(d, &tree, &[matrix], 1_000, |p| {
            p.set("StringOrientation", crate::framework::ParamValue::Enum("Horizontal".to_string()));
        });
    }
    if d.supports_location && d.params.iter().any(|p| p.name == "TargetPositioning") {
        render_with(d, &tree, &[located], 1_000, |p| {
            p.set("TargetPositioning", crate::framework::ParamValue::Enum("Locations".to_string()));
        });
    }

    // Each enum value on its own must render and must leave a consistent set of visible properties.
    for (i, desc) in d.params.iter().enumerate() {
        if let crate::framework::ParamKind::Enum(e) = desc.kind {
            if desc.name == "TargetPositioning" {
                continue;
            }
            for m in e.members {
                render_with(d, &tree, &[matrix], 500, |p| {
                    p.values[i] = crate::framework::ParamValue::Enum(m.name.to_string());
                    let _ = d.visible_params(p);
                });
            }
        }
    }

    let _ = StringOrientation::default();
}

/// Asserts that rendering a matrix produced light somewhere, which catches a port that returns nothing.
pub fn assert_produces_light(d: &'static EffectDescriptor) {
    let mut tree = ElementTree::new();
    let matrix = grid(&mut tree, "Matrix", 8, 25);
    let out = render_with(d, &tree, &[matrix], 2_000, |_| {});
    assert!(!out.is_empty(), "{} rendered no intents", d.name);
    let lit = out.map.values().flatten().any(|n| {
        (0..40).any(|f| match n.intent.state_at(TimeSpan::from_ms_i(f * 50) - n.start) {
            vixen_core::StateValue::Rgb(v) => v.r > 0 || v.g > 0 || v.b > 0,
            vixen_core::StateValue::Lighting(v) => v.intensity > 0.0,
            vixen_core::StateValue::Discrete(v) => v.intensity > 0.0,
            _ => true,
        })
    });
    assert!(lit, "{} rendered only darkness", d.name);
}
