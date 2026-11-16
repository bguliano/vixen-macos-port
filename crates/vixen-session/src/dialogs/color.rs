//! The Color helper and the shared colour sets (F151 to F169).

use super::flow::{leaf_outputs_or_filters, FlowReference};
use super::properties::add_property;
use super::world::{distinct_leaves, DialogWorld, NodeKey};
use uuid::Uuid;
use vixen_core::element::{property_ids, ElementColorType, PropertyData};
use vixen_core::Color;
use vixen_output::dotnet::{known, DotNetColor};
use vixen_output::filters::{ColorBreakdownConfig, ColorBreakdownItem, FilterConfig};
use vixen_output::{ids, ModuleIds};

/// F158, in the picker's order.
pub const COLOR_ORDERS: [&str; 8] = ["RGB", "RBG", "GBR", "GRB", "BRG", "BGR", "RGBW", "GRWB"];

/// F153: `Color.RoyalBlue`.
pub const DEFAULT_SINGLE_COLOR: Color = Color::rgb(65, 105, 225);

/// What the sheet opens with. `preset_type` is `None` when no radio starts selected, which keeps OK off (F152).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColorHelperModel {
    pub set_names: Vec<String>,
    pub preset_type: Option<ElementColorType>,
    pub single_color: Color,
    pub set_name: Option<String>,
    /// F158: always RGB. Reconfiguring doesn't read the existing order, as upstream.
    pub color_order: String,
}

/// F3 and F6 preselect from one property, through `SetupColors`.
pub fn color_helper_model(world: &dyn DialogWorld, preset: Option<&PropertyData>) -> ColorHelperModel {
    let set_names = world.color_set_names();
    let mut model = ColorHelperModel {
        set_name: set_names.first().cloned(),
        set_names,
        preset_type: None,
        single_color: DEFAULT_SINGLE_COLOR,
        color_order: COLOR_ORDERS[0].to_string(),
    };
    if let Some(PropertyData::Color { color_type, single_color, color_set_name }) = preset {
        model.preset_type = Some(*color_type);
        match color_type {
            ElementColorType::SingleColor => model.single_color = *single_color,
            ElementColorType::MultipleDiscreteColors => {
                // F155: a set that's gone stays on the first one, and no name at all leaves nothing selected.
                match color_set_name {
                    Some(name) if model.set_names.contains(name) => model.set_name = Some(name.clone()),
                    Some(_) => {}
                    None => model.set_name = None,
                }
            }
            ElementColorType::FullColor => {}
        }
    }
    model
}

/// What OK asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColorRequest {
    pub color_type: ElementColorType,
    pub single_color: Color,
    pub set_name: Option<String>,
    pub color_order: String,
}

impl ColorRequest {
    /// F152, F155: OK needs a choice, and Multiple needs a set to choose.
    pub fn is_complete(&self) -> bool {
        self.color_type != ElementColorType::MultipleDiscreteColors || self.set_name.is_some()
    }
}

fn leaf_references(world: &dyn DialogWorld, leaves: &[NodeKey]) -> Vec<FlowReference> {
    let mut out = Vec::new();
    for leaf in leaves {
        if let Some(element) = world.node_element(*leaf) {
            leaf_outputs_or_filters(world, &element, Some(&ids::COLOR_BREAKDOWN), &mut out);
        }
    }
    out
}

/// Which of the two questions F159 and F160 have to ask before applying.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ColorPreflight {
    pub has_existing_properties: bool,
    pub has_existing_filters: bool,
}

pub fn color_helper_preflight(world: &dyn DialogWorld, nodes: &[NodeKey]) -> ColorPreflight {
    let leaves = distinct_leaves(world, nodes);
    let references = leaf_references(world, &leaves);
    ColorPreflight {
        has_existing_properties: leaves.iter().any(|l| world.properties(*l).iter().any(|p| p.type_id == property_ids::COLOR)),
        has_existing_filters: references.iter().any(|r| world.filter_type(&r.component) == Some(ids::COLOR_BREAKDOWN)),
    }
}

/// The counts F161 reports.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct ColorReport {
    pub properties_added: u32,
    pub properties_configured: u32,
    pub properties_skipped: u32,
    pub filters_added: u32,
    pub filters_configured: u32,
    pub filters_skipped: u32,
}

/// F161. Two spaces after each colon, as upstream writes it.
pub fn color_report_text(r: &ColorReport) -> String {
    format!(
        "Color Properties:  {} added, {} configured, {} skipped. Color Filters:  {} added, {} configured, {} skipped.",
        r.properties_added, r.properties_configured, r.properties_skipped, r.filters_added, r.filters_configured, r.filters_skipped
    )
}

/// `Color.Name` for a colour in a set: the known name, or lowercase ARGB hex for a value colour.
fn color_name(c: &DotNetColor) -> String {
    if c.is_known_color() {
        if let Some(name) = known::name(c.known_color) {
            return name.to_string();
        }
    }
    format!("{:08x}", c.to_argb_u32())
}

/// The items F160 gives a filter, and whether it mixes.
fn breakdown_items(world: &dyn DialogWorld, request: &ColorRequest) -> (Vec<ColorBreakdownItem>, bool) {
    match request.color_type {
        ElementColorType::FullColor => {
            let items = request
                .color_order
                .chars()
                .filter_map(|letter| match letter {
                    'R' => Some(ColorBreakdownItem::new(DotNetColor::RED, "Red")),
                    'G' => Some(ColorBreakdownItem::new(DotNetColor::LIME, "Green")),
                    'B' => Some(ColorBreakdownItem::new(DotNetColor::BLUE, "Blue")),
                    'W' => Some(ColorBreakdownItem::new(DotNetColor::WHITE, "White")),
                    _ => None,
                })
                .collect();
            (items, true)
        }
        ElementColorType::MultipleDiscreteColors => {
            let colors = request.set_name.as_deref().and_then(|n| world.color_set(n));
            let items = colors
                .map(|set| {
                    set.iter()
                        .map(|c| {
                            let color = DotNetColor::from_color(*c);
                            let name = color_name(&color);
                            ColorBreakdownItem { color, name }
                        })
                        .collect()
                })
                .unwrap_or_default();
            (items, false)
        }
        // The default name, "Unnamed".
        ElementColorType::SingleColor => {
            (vec![ColorBreakdownItem { color: DotNetColor::from_color(request.single_color), ..Default::default() }], false)
        }
    }
}

/// F159, F160. `overwrite_properties` and `overwrite_filters` are the answers to the two questions, which are only asked when
/// [`color_helper_preflight`] says they apply. Everything happens in one pass, for the 1,100 or so filters a real show has.
pub fn color_helper_apply(
    world: &mut dyn DialogWorld,
    nodes: &[NodeKey],
    request: &ColorRequest,
    overwrite_properties: bool,
    overwrite_filters: bool,
) -> ColorReport {
    let mut report = ColorReport::default();
    let leaves = distinct_leaves(world, nodes);

    // Single keeps no set name, Multiple no colour, and Full neither.
    let new_data = PropertyData::Color {
        color_type: request.color_type,
        single_color: if request.color_type == ElementColorType::SingleColor { request.single_color } else { Color::EMPTY },
        color_set_name: if request.color_type == ElementColorType::MultipleDiscreteColors { request.set_name.clone() } else { None },
    };
    for &leaf in &leaves {
        let existing = world.properties(leaf).iter().position(|p| p.type_id == property_ids::COLOR);
        let index = match existing {
            Some(_) if !overwrite_properties => {
                report.properties_skipped += 1;
                continue;
            }
            Some(index) => index,
            None => {
                report.properties_added += 1;
                add_property(world, leaf, &property_ids::COLOR)
            }
        };
        world.properties_mut(leaf)[index].data = new_data.clone();
        report.properties_configured += 1;
    }

    let references = leaf_references(world, &leaves);
    let (items, mix) = breakdown_items(world, request);
    for reference in references {
        let is_breakdown = world.filter_type(&reference.component) == Some(ids::COLOR_BREAKDOWN);
        let config = if is_breakdown {
            if !overwrite_filters {
                report.filters_skipped += 1;
                continue;
            }
            match world.filter_config(&reference.component) {
                Some(FilterConfig::ColorBreakdown(c)) => c,
                _ => {
                    report.filters_skipped += 1;
                    continue;
                }
            }
        } else {
            let instance = Uuid::new_v4();
            let fresh = ColorBreakdownConfig { ids: ModuleIds::new(ids::COLOR_BREAKDOWN, instance), ..Default::default() };
            world.add_filter(FilterConfig::ColorBreakdown(fresh.clone()));
            world.set_source(instance, reference.component, reference.output);
            report.filters_added += 1;
            fresh
        };
        // An existing filter's 16 bit setting stays.
        let updated = ColorBreakdownConfig { breakdown_items: items.clone(), mix_colors: mix, ..config };
        world.set_filter_config(FilterConfig::ColorBreakdown(updated));
        report.filters_configured += 1;
    }
    report
}

// ---- Colour sets ------------------------------------------------------------------------------------------------------------

/// Every set with its colours, in the order they were added.
pub fn ordered_color_sets(world: &dyn DialogWorld) -> Vec<(String, Vec<Color>)> {
    world.color_set_names().into_iter().filter_map(|name| world.color_set(&name).map(|c| (name, c))).collect()
}

/// F164. The prompt's OK is disabled while the name is empty, so this only refuses a name that exists.
pub fn color_set_add(world: &mut dyn DialogWorld, name: &str) -> Result<(), String> {
    if world.color_set(name).is_some() {
        return Err("Color Set already exists.".to_string());
    }
    let mut sets = ordered_color_sets(world);
    sets.push((name.to_string(), Vec::new()));
    world.set_color_sets(&sets);
    Ok(())
}

/// F165. Elements that use a removed set read as having no colours.
pub fn color_set_remove(world: &mut dyn DialogWorld, name: &str) -> Result<(), String> {
    let mut sets = ordered_color_sets(world);
    let count = sets.len();
    sets.retain(|(n, _)| n != name);
    if sets.len() == count {
        return Err("Error removing Color Set!".to_string());
    }
    world.set_color_sets(&sets);
    Ok(())
}

/// F168. A name that's new makes a new set and leaves the old one alone.
pub fn color_set_save(world: &mut dyn DialogWorld, name: &str, colors: &[Color]) -> Result<(), String> {
    if name.is_empty() {
        return Err("You must enter a name for the Color Set.".to_string());
    }
    let mut sets = ordered_color_sets(world);
    match sets.iter_mut().find(|(n, _)| n == name) {
        Some((_, existing)) => *existing = colors.to_vec(),
        None => sets.push((name.to_string(), colors.to_vec())),
    }
    world.set_color_sets(&sets);
    Ok(())
}

/// F169: a displayed set differs when its name is new, or its colours aren't the stored ones.
pub fn color_set_differs(world: &dyn DialogWorld, name: &str, colors: &[Color]) -> bool {
    match world.color_set(name) {
        Some(stored) => {
            stored.len() != colors.len() || stored.iter().zip(colors).any(|(a, b)| a.to_argb_u32() != b.to_argb_u32())
        }
        None => true,
    }
}

/// F166. Typing a blank name leaves the button as it was.
pub fn color_set_button_title(world: &dyn DialogWorld, name: &str, current: &str) -> String {
    if name.is_empty() {
        current.to_string()
    } else if world.color_set(name).is_some() {
        "Update Color Set".to_string()
    } else {
        "Make New Color Set".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::super::flow::test_support::*;
    use super::super::world::{property_data, SystemWorld};
    use super::*;
    use vixen_io::{color_sets, SystemData};
    use vixen_output::filters::FilterConfig;

    fn full(order: &str) -> ColorRequest {
        ColorRequest {
            color_type: ElementColorType::FullColor,
            single_color: Color::EMPTY,
            set_name: None,
            color_order: order.to_string(),
        }
    }

    fn only_filter(world: &SystemWorld<'_>, leaf: NodeKey) -> ColorBreakdownConfig {
        let element = world.node_element(leaf).unwrap();
        let destination = world.destinations(&element, 0)[0];
        match world.filter_config(&destination) {
            Some(FilterConfig::ColorBreakdown(c)) => c,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_report_reads_like_upstream() {
        let r = ColorReport {
            properties_added: 1,
            properties_configured: 2,
            properties_skipped: 3,
            filters_added: 4,
            filters_configured: 5,
            filters_skipped: 6,
        };
        assert_eq!(
            color_report_text(&r),
            "Color Properties:  1 added, 2 configured, 3 skipped. Color Filters:  4 added, 5 configured, 6 skipped."
        );
    }

    #[test]
    fn full_colour_grwb_mixes_one_item_per_letter() {
        let (mut system, root, leaves) = system_with_leaves(1);
        let mut world = SystemWorld::new(&mut system);
        let report = color_helper_apply(&mut world, &[root], &full("GRWB"), false, false);
        assert_eq!((report.properties_added, report.properties_configured, report.filters_added, report.filters_configured), (1, 1, 1, 1));
        let c = only_filter(&world, leaves[0]);
        assert!(c.mix_colors);
        let names: Vec<&str> = c.breakdown_items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, ["Green", "Red", "White", "Blue"]);
        assert_eq!(c.breakdown_items[0].color, DotNetColor::LIME);
        assert!(world.changes.structure);
    }

    #[test]
    fn multiple_with_the_default_rgb_set_names_items_by_hex() {
        let (mut system, root, leaves) = system_with_leaves(1);
        let mut world = SystemWorld::new(&mut system);
        let request = ColorRequest {
            color_type: ElementColorType::MultipleDiscreteColors,
            single_color: Color::EMPTY,
            set_name: Some("RGB".to_string()),
            color_order: "RGB".to_string(),
        };
        color_helper_apply(&mut world, &[root], &request, false, false);
        let c = only_filter(&world, leaves[0]);
        assert!(!c.mix_colors);
        let names: Vec<&str> = c.breakdown_items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, ["ffff0000", "ff00ff00", "ff0000ff"]);
        assert_eq!(
            property_data(&world, leaves[0], &property_ids::COLOR),
            Some(&PropertyData::Color {
                color_type: ElementColorType::MultipleDiscreteColors,
                single_color: Color::EMPTY,
                color_set_name: Some("RGB".to_string())
            })
        );
    }

    #[test]
    fn single_makes_one_unnamed_item() {
        let (mut system, root, leaves) = system_with_leaves(1);
        let mut world = SystemWorld::new(&mut system);
        let request = ColorRequest {
            color_type: ElementColorType::SingleColor,
            single_color: DEFAULT_SINGLE_COLOR,
            set_name: None,
            color_order: "RGB".to_string(),
        };
        color_helper_apply(&mut world, &[root], &request, false, false);
        let c = only_filter(&world, leaves[0]);
        assert_eq!(c.breakdown_items.len(), 1);
        assert_eq!(c.breakdown_items[0].name, "Unnamed");
        assert_eq!(c.breakdown_items[0].color.color(), DEFAULT_SINGLE_COLOR);
    }

    #[test]
    fn saying_no_to_the_questions_skips_what_exists() {
        let (mut system, root, leaves) = system_with_leaves(2);
        let mut world = SystemWorld::new(&mut system);
        color_helper_apply(&mut world, &[root], &full("RGB"), false, false);
        let preflight = color_helper_preflight(&world, &[root]);
        assert!(preflight.has_existing_properties && preflight.has_existing_filters);
        let before = only_filter(&world, leaves[0]);
        let report = color_helper_apply(&mut world, &[root], &full("BGR"), false, false);
        assert_eq!((report.properties_skipped, report.properties_configured), (2, 0));
        assert_eq!(report.filters_added, 0);
        assert_eq!(only_filter(&world, leaves[0]), before);
        // The free outputs mean each filter is listed once for itself and once per output, as upstream counts them.
        assert_eq!(report.filters_skipped, 8);
    }

    #[test]
    fn saying_yes_overwrites_and_keeps_16_bit() {
        let (mut system, root, leaves) = system_with_leaves(1);
        let mut world = SystemWorld::new(&mut system);
        color_helper_apply(&mut world, &[root], &full("RGB"), false, false);
        let mut c = only_filter(&world, leaves[0]);
        c.sixteen_bit = true;
        world.set_filter_config(FilterConfig::ColorBreakdown(c));
        let filters = world.system.filters.len();
        color_helper_apply(&mut world, &[root], &full("BGR"), true, true);
        assert_eq!(world.system.filters.len(), filters, "no new filters");
        let c = only_filter(&world, leaves[0]);
        assert_eq!(c.breakdown_items[0].name, "Blue");
        assert!(c.sixteen_bit);
    }

    #[test]
    fn nothing_existing_means_nothing_to_ask() {
        let (mut system, root, _) = system_with_leaves(2);
        let world = SystemWorld::new(&mut system);
        let p = color_helper_preflight(&world, &[root]);
        assert!(!p.has_existing_properties && !p.has_existing_filters);
    }

    #[test]
    fn a_patched_controller_output_gets_no_filter() {
        let (mut system, root, leaves) = system_with_leaves(1);
        let output = system.controllers[0].outputs[0].id;
        let mut world = SystemWorld::new(&mut system);
        let element = world.node_element(leaves[0]).unwrap();
        world.set_source(output, element, 0);
        let report = color_helper_apply(&mut world, &[root], &full("RGB"), false, false);
        assert_eq!((report.filters_added, report.filters_configured, report.filters_skipped), (0, 0, 0));
        assert!(world.system.filters.is_empty());
    }

    #[test]
    fn the_sheet_presets_from_one_property() {
        let (mut system, _, _) = system_with_leaves(1);
        let world = SystemWorld::new(&mut system);
        let m = color_helper_model(&world, None);
        assert_eq!(m.preset_type, None);
        assert_eq!(m.single_color, DEFAULT_SINGLE_COLOR);
        assert_eq!(m.set_name.as_deref(), Some("RGBW"));
        assert_eq!(m.color_order, "RGB");
        let preset = PropertyData::Color {
            color_type: ElementColorType::MultipleDiscreteColors,
            single_color: Color::EMPTY,
            color_set_name: Some("RGB".to_string()),
        };
        let m = color_helper_model(&world, Some(&preset));
        assert_eq!(m.preset_type, Some(ElementColorType::MultipleDiscreteColors));
        assert_eq!(m.set_name.as_deref(), Some("RGB"));
    }

    #[test]
    fn a_preset_set_that_is_gone_or_was_never_named() {
        let (mut system, _, _) = system_with_leaves(1);
        let world = SystemWorld::new(&mut system);
        let preset = |name: Option<&str>| PropertyData::Color {
            color_type: ElementColorType::MultipleDiscreteColors,
            single_color: Color::WHITE,
            color_set_name: name.map(str::to_string),
        };
        let first = world.color_set_names().first().cloned();
        assert_eq!(color_helper_model(&world, Some(&preset(Some("Long Gone")))).set_name, first);
        assert_eq!(color_helper_model(&world, Some(&preset(None))).set_name, None);
    }

    #[test]
    fn colour_sets_start_with_the_defaults_in_order() {
        let mut system = SystemData::default();
        let world = SystemWorld::new(&mut system);
        assert_eq!(world.color_set_names(), ["RGBW", "RGB"]);
    }

    #[test]
    fn saving_under_a_new_name_keeps_the_old_set() {
        let mut system = SystemData::default();
        let mut world = SystemWorld::new(&mut system);
        let rgb = world.color_set("RGB").unwrap();
        color_set_save(&mut world, "Warm", &[Color::ORANGE]).unwrap();
        assert_eq!(world.color_set_names(), ["RGBW", "RGB", "Warm"]);
        assert_eq!(world.color_set("RGB").unwrap(), rgb);
        assert_eq!(color_sets(&world.system.module_store).len(), 3);
        assert_eq!(color_set_save(&mut world, "", &[]).unwrap_err(), "You must enter a name for the Color Set.");
        // A later name goes after it, in the order they were added.
        color_set_save(&mut world, "Cool", &[Color::BLUE]).unwrap();
        assert_eq!(world.color_set_names(), ["RGBW", "RGB", "Warm", "Cool"]);
    }

    #[test]
    fn adding_a_name_that_exists_is_refused() {
        let mut system = SystemData::default();
        let mut world = SystemWorld::new(&mut system);
        assert_eq!(color_set_add(&mut world, "RGB").unwrap_err(), "Color Set already exists.");
        color_set_add(&mut world, "Empty").unwrap();
        assert!(world.color_set("Empty").unwrap().is_empty());
    }

    #[test]
    fn removing_keeps_the_rest_in_order() {
        let mut system = SystemData::default();
        let mut world = SystemWorld::new(&mut system);
        color_set_add(&mut world, "A").unwrap();
        color_set_remove(&mut world, "RGB").unwrap();
        assert_eq!(world.color_set_names(), ["RGBW", "A"]);
        assert_eq!(color_set_remove(&mut world, "RGB").unwrap_err(), "Error removing Color Set!");
    }

    #[test]
    fn differences_by_count_value_and_name() {
        let mut system = SystemData::default();
        let world = SystemWorld::new(&mut system);
        let rgb = world.color_set("RGB").unwrap();
        assert!(!color_set_differs(&world, "RGB", &rgb));
        assert!(color_set_differs(&world, "RGB", &rgb[..2]));
        assert!(color_set_differs(&world, "RGB", &[Color::RED, Color::GREEN, Color::BLUE]));
        assert!(color_set_differs(&world, "New", &rgb));
    }

    #[test]
    fn the_button_title_follows_the_name() {
        let mut system = SystemData::default();
        let world = SystemWorld::new(&mut system);
        assert_eq!(color_set_button_title(&world, "RGB", "x"), "Update Color Set");
        assert_eq!(color_set_button_title(&world, "Other", "x"), "Make New Color Set");
        assert_eq!(color_set_button_title(&world, "", "x"), "x");
    }
}
