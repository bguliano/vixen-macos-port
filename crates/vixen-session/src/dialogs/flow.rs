//! Walking the data flow patches, as the Color and Dimming Curve helpers do.

use super::world::DialogWorld;
use uuid::Uuid;

/// A component's output, as `DataFlowComponentReference`. `output` is -1 for a filter found by its type, as upstream's walks yield.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FlowReference {
    pub component: Uuid,
    pub output: i32,
}

/// `_FindLeafOutputsOrBreakdownFilters` and `_FindLeafOutputsOrDimmingCurveFilters`: every free output below `component`, and every
/// filter of type `found_type` on the way, with -1 as its output. A controller output yields nothing.
pub fn leaf_outputs_or_filters(
    world: &dyn DialogWorld,
    component: &Uuid,
    found_type: Option<&Uuid>,
    out: &mut Vec<FlowReference>,
) {
    let mut path = Vec::new();
    walk(world, component, found_type, out, &mut path);
}

fn walk(
    world: &dyn DialogWorld,
    component: &Uuid,
    found_type: Option<&Uuid>,
    out: &mut Vec<FlowReference>,
    path: &mut Vec<Uuid>,
) {
    // A loop could only come from a damaged file, and following it would never end.
    if path.contains(component) {
        return;
    }
    if let (Some(found), Some(actual)) = (found_type, world.filter_type(component)) {
        if *found == actual {
            out.push(FlowReference { component: *component, output: -1 });
        }
    }
    path.push(*component);
    for i in 0..world.output_count(component) as i32 {
        let children = world.destinations(component, i);
        if children.is_empty() {
            out.push(FlowReference { component: *component, output: i });
        } else {
            for child in &children {
                walk(world, child, found_type, out, path);
            }
        }
    }
    path.pop();
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::super::world::NodeKey;
    use uuid::Uuid;
    use vixen_core::element::NodeId;
    use vixen_io::{Channel, Controller, ControllerOutput, SystemData};
    use vixen_output::ids;

    pub fn key(node: NodeId) -> NodeKey {
        NodeKey(node.0)
    }

    /// A system with `count` leaf elements under a root group, each with a channel, and a controller with `count` outputs that aren't
    /// patched to anything.
    pub fn system_with_leaves(count: usize) -> (SystemData, NodeKey, Vec<NodeKey>) {
        let mut system = SystemData::default();
        let root = system.tree.add_string("Group", count);
        let leaves = system.tree.leaves(root);
        for leaf in &leaves {
            let n = system.tree.node(*leaf);
            if let Some(id) = n.element_id {
                system.channels.push(Channel { id, name: n.name.clone() });
            }
        }
        system.controllers.push(Controller {
            name: "C".to_string(),
            hardware_type_id: ids::DUMMY_LIGHTING,
            hardware_instance_id: Uuid::new_v4(),
            id: Uuid::new_v4(),
            outputs: (0..count)
                .map(|i| ControllerOutput { name: format!("Output {}", i + 1), id: Uuid::new_v4() })
                .collect(),
        });
        (system, key(root), leaves.into_iter().map(key).collect())
    }

}

#[cfg(test)]
mod tests {
    use super::super::world::{DialogWorld, SystemWorld};
    use super::test_support::*;
    use super::*;
    use vixen_output::filters::{ColorBreakdownConfig, FilterConfig};
    use vixen_output::ids;

    #[test]
    fn a_free_element_output_is_its_own_leaf_and_a_patched_controller_output_is_not() {
        let (mut system, _, leaves) = system_with_leaves(2);
        let output = system.controllers[0].outputs[0].id;
        let mut world = SystemWorld::new(&mut system);
        let a = world.node_element(leaves[0]).unwrap();
        let b = world.node_element(leaves[1]).unwrap();
        world.set_source(output, b, 0);
        let mut out = Vec::new();
        leaf_outputs_or_filters(&world, &a, None, &mut out);
        assert_eq!(out, [FlowReference { component: a, output: 0 }]);
        out.clear();
        leaf_outputs_or_filters(&world, &b, None, &mut out);
        assert!(out.is_empty(), "a dead end has nothing to attach to");
    }

    #[test]
    fn a_breakdown_with_free_outputs_counts_once_and_once_per_free_output() {
        let (mut system, _, leaves) = system_with_leaves(1);
        let output = system.controllers[0].outputs[0].id;
        let mut world = SystemWorld::new(&mut system);
        let element = world.node_element(leaves[0]).unwrap();
        let instance = Uuid::new_v4();
        let mut config = ColorBreakdownConfig { ids: vixen_output::ModuleIds::new(ids::COLOR_BREAKDOWN, instance), ..Default::default() };
        config.breakdown_items = ColorBreakdownConfig::template("RGB").unwrap();
        world.add_filter(FilterConfig::ColorBreakdown(config));
        world.set_source(instance, element, 0);
        // One of three outputs is patched.
        world.set_source(output, instance, 1);
        let mut out = Vec::new();
        leaf_outputs_or_filters(&world, &element, Some(&ids::COLOR_BREAKDOWN), &mut out);
        assert_eq!(
            out,
            [
                FlowReference { component: instance, output: -1 },
                FlowReference { component: instance, output: 0 },
                FlowReference { component: instance, output: 2 },
            ]
        );
    }

    #[test]
    fn set_source_replaces_and_reset_leaves_an_empty_patch() {
        let (mut system, _, leaves) = system_with_leaves(2);
        let output = system.controllers[0].outputs[0].id;
        let (a, b);
        {
            let mut world = SystemWorld::new(&mut system);
            a = world.node_element(leaves[0]).unwrap();
            b = world.node_element(leaves[1]).unwrap();
            // Asked once, so the index exists while it changes.
            assert!(world.destinations(&a, 0).is_empty());
            world.set_source(output, a, 0);
            world.set_source(output, b, 0);
            assert!(world.destinations(&a, 0).is_empty());
            assert_eq!(world.destinations(&b, 0), [output]);
            world.reset_source(&output);
            assert!(world.destinations(&b, 0).is_empty());
        }
        assert_eq!(system.patches.len(), 1);
        assert_eq!(system.patches[0].source_id, None);
    }
}
