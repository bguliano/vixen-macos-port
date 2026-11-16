//! The Dimming Curve helper (F238 to F245).

use super::filters::{curve_data_from_draft, CurveDraft};
use super::flow::{leaf_outputs_or_filters, FlowReference};
use super::world::{distinct_leaves, DialogWorld, NodeKey};
use uuid::Uuid;
use vixen_output::filters::{DimmingCurveConfig, FilterConfig};
use vixen_output::{ids, ModuleIds};

/// F242's radios.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ExistingBehaviour {
    DoNothing,
    UpdateExisting,
    AddInsertAfterElement,
    AddNew,
}

/// F243: `VixenHelp.HelpStrings.Patching`.
pub const PATCHING_HELP_URL: &str = "https://www.vixenlights.com/docs/usage/display-setup/display-patching/";

/// The counts F244 reports.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct DimmingReport {
    pub created: u32,
    pub configured: u32,
    pub skipped: u32,
}

/// F244.
pub fn dimming_report_text(r: &DimmingReport) -> String {
    format!("{} Dimming Curves created, {} configured, and {} skipped.", r.created, r.configured, r.skipped)
}

/// A new filter with its own copy of the curve. Upstream shares one curve object between all of them.
fn new_filter(world: &mut dyn DialogWorld, draft: &CurveDraft) -> Uuid {
    let instance = Uuid::new_v4();
    let config = DimmingCurveConfig {
        ids: ModuleIds::new(ids::DIMMING_CURVE, instance),
        curve: curve_data_from_draft(draft, None),
    };
    world.add_filter(FilterConfig::DimmingCurve(config));
    instance
}

/// F244.
pub fn dimming_helper_apply(
    world: &mut dyn DialogWorld,
    nodes: &[NodeKey],
    draft: &CurveDraft,
    behaviour: ExistingBehaviour,
) -> DimmingReport {
    let mut report = DimmingReport::default();
    for leaf in distinct_leaves(world, nodes) {
        let Some(element) = world.node_element(leaf) else {
            continue;
        };
        if behaviour == ExistingBehaviour::AddInsertAfterElement {
            let destinations = world.destinations(&element, 0);
            let filter = new_filter(world, draft);
            world.set_source(filter, element, 0);
            for destination in destinations {
                world.reset_source(&destination);
                world.set_source(destination, filter, 0);
            }
            report.created += 1;
            report.configured += 1;
            continue;
        }
        // Adding regardless ignores existing curves and always goes to the outputs.
        let found = (behaviour != ExistingBehaviour::AddNew).then_some(&ids::DIMMING_CURVE);
        let mut references: Vec<FlowReference> = Vec::new();
        leaf_outputs_or_filters(world, &element, found, &mut references);
        for reference in references {
            let mut output = reference.output;
            if world.filter_type(&reference.component) == Some(ids::DIMMING_CURVE) {
                match behaviour {
                    ExistingBehaviour::DoNothing => {
                        report.skipped += 1;
                        continue;
                    }
                    ExistingBehaviour::UpdateExisting => {
                        if let Some(FilterConfig::DimmingCurve(mut existing)) = world.filter_config(&reference.component) {
                            existing.curve = curve_data_from_draft(draft, Some(&existing.curve));
                            world.set_filter_config(FilterConfig::DimmingCurve(existing));
                        }
                        report.configured += 1;
                        continue;
                    }
                    ExistingBehaviour::AddNew => output = 0,
                    ExistingBehaviour::AddInsertAfterElement => {}
                }
            }
            let filter = new_filter(world, draft);
            world.set_source(filter, reference.component, output);
            report.created += 1;
            report.configured += 1;
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::super::filters::default_curve_draft;
    use super::super::flow::test_support::*;
    use super::super::world::SystemWorld;
    use super::*;

    fn half() -> CurveDraft {
        CurveDraft { points: vec![(0.0, 0.0), (100.0, 50.0)], ..default_curve_draft() }
    }

    fn curve_of(world: &dyn DialogWorld, instance: &Uuid) -> DimmingCurveConfig {
        match world.filter_config(instance) {
            Some(FilterConfig::DimmingCurve(c)) => c,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_report_reads_like_upstream() {
        let r = DimmingReport { created: 3, configured: 4, skipped: 5 };
        assert_eq!(dimming_report_text(&r), "3 Dimming Curves created, 4 configured, and 5 skipped.");
    }

    #[test]
    fn a_free_output_gets_a_new_filter_with_its_own_curve() {
        let (mut system, root, leaves) = system_with_leaves(2);
        let mut world = SystemWorld::new(&mut system);
        let report = dimming_helper_apply(&mut world, &[root], &half(), ExistingBehaviour::DoNothing);
        assert_eq!(report, DimmingReport { created: 2, configured: 2, skipped: 0 });
        let a = world.node_element(leaves[0]).unwrap();
        let filter = world.destinations(&a, 0)[0];
        assert_eq!(curve_of(&world, &filter).curve.points[1].y, 50.0);
        assert_eq!(world.system.filters.len(), 2);
        assert_ne!(world.system.filters[0].instance_id, world.system.filters[1].instance_id);
    }

    #[test]
    fn leave_alone_skips_an_existing_curve() {
        let (mut system, root, _) = system_with_leaves(1);
        let mut world = SystemWorld::new(&mut system);
        dimming_helper_apply(&mut world, &[root], &half(), ExistingBehaviour::DoNothing);
        let report = dimming_helper_apply(&mut world, &[root], &default_curve_draft(), ExistingBehaviour::DoNothing);
        // Listed once for itself and once for its free output, as upstream counts it.
        assert_eq!(report, DimmingReport { created: 0, configured: 0, skipped: 2 });
        assert_eq!(world.system.filters.len(), 1);
        let filter = world.system.filters[0].instance_id;
        assert_eq!(curve_of(&world, &filter).curve.points[1].y, 50.0);
    }

    #[test]
    fn update_stores_the_new_curve() {
        let (mut system, root, _) = system_with_leaves(1);
        let mut world = SystemWorld::new(&mut system);
        dimming_helper_apply(&mut world, &[root], &default_curve_draft(), ExistingBehaviour::DoNothing);
        let report = dimming_helper_apply(&mut world, &[root], &half(), ExistingBehaviour::UpdateExisting);
        assert_eq!(report, DimmingReport { created: 0, configured: 2, skipped: 0 });
        assert_eq!(world.system.filters.len(), 1);
        let filter = world.system.filters[0].instance_id;
        assert_eq!(curve_of(&world, &filter).curve.points[1].y, 50.0);
    }

    #[test]
    fn add_new_puts_another_filter_after_the_existing_one() {
        let (mut system, root, _) = system_with_leaves(1);
        let mut world = SystemWorld::new(&mut system);
        dimming_helper_apply(&mut world, &[root], &default_curve_draft(), ExistingBehaviour::DoNothing);
        let first = world.system.filters[0].instance_id;
        let report = dimming_helper_apply(&mut world, &[root], &half(), ExistingBehaviour::AddNew);
        assert_eq!(report, DimmingReport { created: 1, configured: 1, skipped: 0 });
        assert_eq!(world.system.filters.len(), 2);
        let second = world.system.filters[1].instance_id;
        assert_eq!(world.destinations(&first, 0), [second]);
    }

    #[test]
    fn insert_moves_what_followed_the_element_behind_the_new_filter() {
        let (mut system, root, leaves) = system_with_leaves(1);
        let output = system.controllers[0].outputs[0].id;
        let mut world = SystemWorld::new(&mut system);
        let element = world.node_element(leaves[0]).unwrap();
        world.set_source(output, element, 0);
        let report = dimming_helper_apply(&mut world, &[root], &half(), ExistingBehaviour::AddInsertAfterElement);
        assert_eq!(report, DimmingReport { created: 1, configured: 1, skipped: 0 });
        let filter = world.system.filters[0].instance_id;
        assert_eq!(world.destinations(&element, 0), [filter]);
        assert_eq!(world.destinations(&filter, 0), [output]);
    }
}
