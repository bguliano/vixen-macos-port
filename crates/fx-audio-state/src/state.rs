//! Port of VixenModules.Effect.State: StateRenderPlanner, StateRenderSegmentCoalescer, StateMarkParser,
//! StateDefinitionDiscovery and the effect itself.

use crate::generated::state as gen;
use crate::property_data::{StateDefinitionData, StateItemData, StatePropertyData};
use crate::visual::{adjusted_font, FONT_NAME};
use std::collections::{HashMap, HashSet};
use vixen_effects::gfx::Graphics;
use vixen_effects::prelude::*;

pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    visibility: Some(visibility),
    targets_changed: Some(targets_changed),
    param_changed_ctx: Some(param_changed_ctx),
    // `ForceGenerateVisualRepresentation => _data.ShowEffectVisual`.
    force_visual_when: Some(|p| p.boolean("ShowEffectVisual")),
    visual: Some(visual),
    selection: Some(selection),
    ported: true,
    ..gen::BASE
};

const MAX_ITERATIONS: i32 = 5000;

/// `SetRenderSourceBrowsables`.
fn visibility(p: &Params, _t: &TargetInfo, v: &mut Visibility) {
    let source = p.enum_name("RenderSource");
    let iterate = p.enum_name("PlaybackMode") == "Iterate";
    v.set("StateItem", source == "StateItem");
    v.set("MarkCollectionId", source == "MarkCollection");
    v.set("CustomStateItems", source == "Custom");
    v.set("Iterations", iterate);
    v.set("CycleOffset", iterate);
    v.set("CycleIndividually", source == "Custom" && iterate);
}

// ---------------------------------------------------------------------------------------------------------
// Discovery

struct DiscoveredStateDefinition {
    state_definition_id: Uuid,
    state_property_id: Uuid,
    owner_element_id: Uuid,
    owner_element_name: String,
    display_label: String,
    definition: StateDefinitionData,
}

/// `StateDefinitionDiscovery.Discover`: every definition of the State properties on the targets and
/// everything below them, each node visited once.
fn discover(ctx: &RenderCtx) -> Vec<DiscoveredStateDefinition> {
    discover_in(ctx.tree, ctx.targets)
}

fn discover_in(tree: &ElementTree, targets: &[NodeId]) -> Vec<DiscoveredStateDefinition> {
    let mut out = Vec::new();
    let mut visited: HashSet<Uuid> = HashSet::new();
    for &target in targets {
        for node in tree.descendants(target) {
            let element = tree.node(node);
            if !visited.insert(element.id) {
                continue;
            }
            if let Some(state) = StatePropertyData::for_node(tree, node) {
                for d in state.definitions {
                    out.push(DiscoveredStateDefinition {
                        state_definition_id: d.id,
                        state_property_id: state.id,
                        owner_element_id: element.id,
                        owner_element_name: element.name.clone(),
                        display_label: d.name.clone(),
                        definition: d,
                    });
                }
            }
        }
    }
    apply_display_labels(&mut out);
    out
}

/// `ApplyDisplayLabels`: a name used twice gets its owner in brackets, and a label still used twice its short id.
fn apply_display_labels(definitions: &mut [DiscoveredStateDefinition]) {
    let count = |labels: Vec<&str>, label: &str| labels.iter().filter(|l| **l == label).count();
    let names: Vec<String> = definitions.iter().map(|d| d.definition.name.clone()).collect();
    for d in definitions.iter_mut() {
        if count(names.iter().map(|s| s.as_str()).collect(), &d.definition.name) > 1 {
            d.display_label = format!("{} ({})", d.definition.name, d.owner_element_name);
        }
    }
    let labels: Vec<String> = definitions.iter().map(|d| d.display_label.clone()).collect();
    for d in definitions.iter_mut() {
        if count(labels.iter().map(|s| s.as_str()).collect(), &d.display_label) > 1 {
            d.display_label = format!("{} [{}]", d.display_label, short_id(d.state_definition_id));
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// Editor rules

/// The `StateDefinition` getter.
fn state_definition_label(p: &Params, discovered: &[DiscoveredStateDefinition]) -> String {
    let id = p.guid("SelectedStateDefinitionId");
    match discovered.iter().find(|d| d.state_definition_id == id) {
        Some(d) => d.display_label.clone(),
        None if id.is_nil() => "<No States Available>".to_string(),
        None => format!("<Missing State: {}>", short_id(id)),
    }
}

/// The `StateItem` getter.
fn state_item_label(p: &Params, discovered: &[DiscoveredStateDefinition]) -> String {
    let id = p.guid("SelectedStateDefinitionId");
    let Some(definition) = discovered.iter().find(|d| d.state_definition_id == id).filter(|d| !d.definition.items.is_empty()) else {
        return "<No State Items Available>".to_string();
    };
    let item = p.guid("SelectedStateItemId");
    if item.is_nil() {
        return "<All>".to_string();
    }
    match definition.definition.items.iter().find(|i| i.id == item) {
        Some(i) => i.name.clone(),
        None => format!("<Missing State Item: {}>", short_id(item)),
    }
}

/// `StateDefinition` and `StateItem` are worked out by their getters in the C#. Here they are kept up to date in the values.
fn store_labels(p: &mut Params, discovered: &[DiscoveredStateDefinition]) {
    p.set("StateDefinition", ParamValue::Str(state_definition_label(p, discovered)));
    p.set("StateItem", ParamValue::Str(state_item_label(p, discovered)));
}

/// `SelectStateDefinition`.
fn select_state_definition(p: &mut Params, d: &DiscoveredStateDefinition, previous_item_name: Option<&str>) {
    p.set("SelectedStateDefinitionId", ParamValue::Guid(d.state_definition_id));
    p.set("StatePropertyId", ParamValue::Guid(d.state_property_id));
    p.set("StateOwnerElementId", ParamValue::Guid(d.owner_element_id));
    if !d.definition.items.is_empty() {
        let matching = previous_item_name.and_then(|name| d.definition.items.iter().find(|i| i.name == name));
        p.set("SelectedStateItemId", ParamValue::Guid(matching.map_or(Uuid::nil(), |i| i.id)));
    }
}

/// `RefreshStateDefinitionSelection`. A loaded effect keeps its saved selection until its targets change.
fn targets_changed(p: &mut Params, t: &TargetInfo) {
    let discovered = discover_in(t.tree, t.targets);
    let id = p.guid("SelectedStateDefinitionId");
    if id.is_nil() {
        if let Some(first) = discovered.first() {
            select_state_definition(p, first, None);
        }
    } else if let Some(selected) = discovered.iter().find(|d| d.state_definition_id == id) {
        p.set("StatePropertyId", ParamValue::Guid(selected.state_property_id));
        p.set("StateOwnerElementId", ParamValue::Guid(selected.owner_element_id));
    }
    store_labels(p, &discovered);
}

/// `EnsureCustomStateItemRequired`: Custom needs a row, so an empty list gets one for the first item of the definition.
fn ensure_custom_state_item_required(p: &mut Params, discovered: &[DiscoveredStateDefinition]) {
    if p.enum_name("RenderSource") != "Custom" || !p.records("CustomStateItems").is_empty() {
        return;
    }
    let id = p.guid("SelectedStateDefinitionId");
    let Some(first) = discovered.iter().find(|d| d.state_definition_id == id).and_then(|d| d.definition.items.first()) else { return };
    let mut row = Params::defaults(&gen::CUSTOM_STATE_ITEM_PARAMS);
    row.set("StateItemId", ParamValue::Guid(first.id));
    row.set("Color", ParamValue::Color(first.color));
    p.set("CustomStateItems", ParamValue::RecordList(vec![row]));
}

/// `NormalizeCustomStateItemsForDefault`: a set of distinct items, in order.
fn normalize_custom_items_for_default(p: &mut Params) {
    let mut seen: HashSet<Uuid> = HashSet::new();
    let rows: Vec<Params> = p
        .records("CustomStateItems")
        .iter()
        .filter(|r| {
            let id = r.guid("StateItemId");
            !id.is_nil() && seen.insert(id)
        })
        .cloned()
        .collect();
    p.set("CustomStateItems", ParamValue::RecordList(rows));
}

/// `CreateCustomStateItemLabels`: an item's name, told apart from its namesakes by what it is assigned to, then by number,
/// then by its id.
fn custom_item_labels(tree: &ElementTree, targets: &[NodeId], items: &[StateItemData]) -> Vec<(Uuid, String)> {
    let mut names_by_id: Vec<(Uuid, String)> = Vec::new();
    for &target in targets {
        for node in tree.descendants(target) {
            let element = tree.node(node);
            if !names_by_id.iter().any(|(id, _)| *id == element.id) {
                names_by_id.push((element.id, element.name.clone()));
            }
        }
    }
    let summary = |item: &StateItemData| match item.element_node_ids.first() {
        None => "No assignments".to_string(),
        Some(first) => {
            let name = names_by_id.iter().find(|(id, _)| id == first).map_or_else(|| short_id(*first), |(_, n)| n.clone());
            if item.element_node_ids.len() == 1 {
                name
            } else {
                format!("{name} + {} more", item.element_node_ids.len() - 1)
            }
        }
    };
    let count = |labels: &[(Uuid, String)], label: &str| labels.iter().filter(|(_, l)| l == label).count();
    let mut labels: Vec<(Uuid, String)> = items
        .iter()
        .map(|item| {
            let repeated = items.iter().filter(|o| o.name == item.name).count() > 1;
            (item.id, if repeated { format!("{} ({})", item.name, summary(item)) } else { item.name.clone() })
        })
        .collect();
    // AddOrdinalToDuplicateLabels
    let proposed = labels.clone();
    let mut ordinals: Vec<(String, i32)> = Vec::new();
    for (_, label) in labels.iter_mut() {
        if count(&proposed, label) > 1 {
            let ordinal = match ordinals.iter_mut().find(|(l, _)| l == label) {
                Some((_, n)) => {
                    *n += 1;
                    *n
                }
                None => {
                    ordinals.push((label.clone(), 1));
                    1
                }
            };
            *label = match label.strip_suffix(')') {
                Some(open) => format!("{open}, {ordinal})"),
                None => format!("{label} ({ordinal})"),
            };
        }
    }
    // AddShortIdToDuplicateLabels
    let numbered = labels.clone();
    for (id, label) in labels.iter_mut() {
        if count(&numbered, label) > 1 {
            *label = format!("{label} [{}]", short_id(*id));
        }
    }
    labels
}

/// The State drop downs: `GetStateDefinitionOptions`, `GetStateItemOptions` and `GetCustomStateItemOptions`.
fn selection(name: &str, row: Option<&Params>, p: &Params, t: &TargetInfo) -> Option<Vec<SelectionValue>> {
    let discovered = discover_in(t.tree, t.targets);
    let selected_id = p.guid("SelectedStateDefinitionId");
    let selected = discovered.iter().find(|d| d.state_definition_id == selected_id);
    let items = selected.map_or(&[][..], |d| &d.definition.items[..]);
    match name {
        "StateDefinition" => {
            let mut options: Vec<String> = discovered.iter().map(|d| d.display_label.clone()).collect();
            if !selected_id.is_nil() && selected.is_none() {
                options.insert(0, format!("<Missing State: {}>", short_id(selected_id)));
            }
            if options.is_empty() {
                options.push("<No States Available>".to_string());
            }
            Some(options.into_iter().map(SelectionValue::text).collect())
        }
        "StateItem" => {
            if items.is_empty() {
                return Some(vec![SelectionValue::text("<No State Items Available>")]);
            }
            let mut options = vec!["<All>".to_string()];
            let item = p.guid("SelectedStateItemId");
            if !item.is_nil() && !items.iter().any(|i| i.id == item) {
                options.push(format!("<Missing State Item: {}>", short_id(item)));
            }
            options.extend(unique_names(items).into_iter().map(|n| n.to_string()));
            Some(options.into_iter().map(SelectionValue::text).collect())
        }
        "StateItemId" => {
            let row = row?;
            if items.is_empty() {
                return Some(vec![SelectionValue { value: ParamValue::Guid(Uuid::nil()), label: "<No State Items Available>".to_string() }]);
            }
            let current = row.guid("StateItemId");
            let iterate = p.enum_name("PlaybackMode") == "Iterate";
            let labels = custom_item_labels(t.tree, t.targets, items);
            let choice = |id: Uuid, label: String| SelectionValue { value: ParamValue::Guid(id), label };
            // In Default playback an item already on another row isn't offered. Rows have no identity here, so "another row"
            // is any row holding the item when this row doesn't.
            let taken = |id: Uuid| id != current && p.records("CustomStateItems").iter().any(|r| r.guid("StateItemId") == id);
            let mut options: Vec<SelectionValue> = Vec::new();
            if iterate {
                options.push(choice(Uuid::nil(), "<None>".to_string()));
            }
            options.extend(labels.iter().filter(|(id, _)| iterate || !taken(*id)).map(|(id, label)| choice(*id, label.clone())));
            if !current.is_nil() && !labels.iter().any(|(id, _)| *id == current) {
                options.insert(usize::from(iterate), choice(current, format!("<Missing State Item: {}>", short_id(current))));
            }
            Some(options)
        }
        _ => None,
    }
}

/// `GetCustomStateItemValidColors`: the discrete colours of the elements assigned to the row's item, in the effect's own targets. No item means full colour.
pub fn custom_item_valid_colors(row: &Params, p: &Params, t: &TargetInfo) -> Vec<Color> {
    let id = row.guid("StateItemId");
    if id.is_nil() {
        return Vec::new();
    }
    let discovered = discover_in(t.tree, t.targets);
    let selected_id = p.guid("SelectedStateDefinitionId");
    let Some(item) = discovered.iter().find(|d| d.state_definition_id == selected_id).and_then(|d| d.definition.items.iter().find(|i| i.id == id)) else {
        return Vec::new();
    };
    let mut colors = Vec::new();
    for &target in t.targets {
        for node in t.tree.descendants(target) {
            if item.element_node_ids.contains(&t.tree.node(node).id) {
                for color in t.tree.valid_colors(node, true) {
                    if !colors.contains(&color) {
                        colors.push(color);
                    }
                }
            }
        }
    }
    colors
}

fn param_changed_ctx(c: &ParamChange, p: &mut Params) {
    use vixen_core::marks::MarkCollectionType;
    let t = c.info;
    let normalize_marks = |p: &mut Params| {
        normalize_mark_selection(p, t, p.enum_name("RenderSource") == "MarkCollection", Some(MarkCollectionType::State), true);
    };
    match c.name {
        MODULE_DATA => store_labels(p, &discover_in(t.tree, t.targets)),
        MARK_COLLECTIONS => normalize_marks(p),
        "RenderSource" => {
            if c.old != p.get("RenderSource") {
                ensure_custom_state_item_required(p, &discover_in(t.tree, t.targets));
                normalize_marks(p);
            }
        }
        "PlaybackMode" => {
            if c.old != p.get("PlaybackMode") && p.enum_name("PlaybackMode") == "Default" {
                normalize_custom_items_for_default(p);
            }
        }
        // An edited row reaches this as the whole list. `CustomStateItem.StateItem`'s setter gives the row the item's colour, `<None>` keeps the colour, and
        // `CreateNewCustomStateItem` gives a new row the first item no other row uses.
        "CustomStateItems" => {
            let discovered = discover_in(t.tree, t.targets);
            let selected_id = p.guid("SelectedStateDefinitionId");
            let items: Vec<StateItemData> =
                discovered.iter().find(|d| d.state_definition_id == selected_id).map_or_else(Vec::new, |d| d.definition.items.clone());
            let old: Vec<Params> = if let ParamValue::RecordList(old) = c.old { old.clone() } else { Vec::new() };
            let Some(i) = p.index_of("CustomStateItems") else { return };
            let ParamValue::RecordList(rows) = &mut p.values[i] else { return };
            let used: Vec<Uuid> = rows.iter().map(|r| r.guid("StateItemId")).collect();
            for (index, row) in rows.iter_mut().enumerate() {
                let id = row.guid("StateItemId");
                match old.get(index) {
                    Some(before) if before.guid("StateItemId") != id => {
                        if let Some(item) = items.iter().find(|item| item.id == id) {
                            row.set("Color", ParamValue::Color(item.color));
                        }
                    }
                    None if id.is_nil() => {
                        if let Some(item) = items.iter().find(|item| !used.contains(&item.id)) {
                            row.set("StateItemId", ParamValue::Guid(item.id));
                            row.set("Color", ParamValue::Color(item.color));
                        }
                    }
                    _ => {}
                }
            }
        }
        "Iterations" => {
            let iterations = normalize_iterations(p.int("Iterations"));
            p.set("Iterations", ParamValue::Int(iterations));
        }
        // The setter takes a label. One that isn't offered is ignored.
        "StateDefinition" => {
            let discovered = discover_in(t.tree, t.targets);
            let previous_id = p.guid("SelectedStateDefinitionId");
            let label = p.string("StateDefinition").to_string();
            if let Some(selected) = discovered.iter().find(|d| d.display_label == label) {
                let previous_name = discovered
                    .iter()
                    .find(|d| d.state_definition_id == previous_id)
                    .and_then(|d| d.definition.items.iter().find(|i| i.id == p.guid("SelectedStateItemId")))
                    .map(|i| i.name.clone());
                select_state_definition(p, selected, previous_name.as_deref());
                if previous_id != selected.state_definition_id {
                    p.set("CustomStateItems", ParamValue::RecordList(Vec::new()));
                    ensure_custom_state_item_required(p, &discovered);
                }
            }
            store_labels(p, &discovered);
        }
        "StateItem" => {
            let discovered = discover_in(t.tree, t.targets);
            let label = p.string("StateItem").to_string();
            if label == "<All>" {
                p.set("SelectedStateItemId", ParamValue::Guid(Uuid::nil()));
            } else {
                let id = p.guid("SelectedStateDefinitionId");
                let item = discovered
                    .iter()
                    .find(|d| d.state_definition_id == id)
                    .and_then(|d| d.definition.items.iter().find(|i| i.name == label))
                    .map(|i| i.id);
                if let Some(item) = item {
                    p.set("SelectedStateItemId", ParamValue::Guid(item));
                }
            }
            store_labels(p, &discovered);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------------------------------------
// Planner

#[derive(Copy, Clone, PartialEq)]
enum PlaybackMode {
    Default,
    Iterate,
}

struct Interval<'a> {
    item: &'a StateItemData,
    start: TimeSpan,
    duration: TimeSpan,
    color_override: Option<Color>,
}

/// `StateData.NormalizeIterations`.
fn normalize_iterations(i: i32) -> i32 {
    i.clamp(1, MAX_ITERATIONS)
}

fn normalize_cycle_offset(cycle_offset: i32, slot_count: i64) -> i64 {
    if cycle_offset > 0 {
        cycle_offset as i64 % slot_count
    } else {
        0
    }
}

fn offset_slot_index(output_index: i64, slot_count: i64, normalized_cycle_offset: i64) -> usize {
    (((output_index % slot_count) + normalized_cycle_offset) % slot_count) as usize
}

/// `GetIntervalDuration`: equal slices, the last one takes the remainder.
fn interval_duration(effect_duration: TimeSpan, interval_count: i64, index: i64, interval_start: TimeSpan) -> TimeSpan {
    if index == interval_count - 1 {
        effect_duration - interval_start
    } else {
        TimeSpan::from_ticks(effect_duration.ticks() / interval_count)
    }
}

fn unique_names(items: &[StateItemData]) -> Vec<&str> {
    let mut names: Vec<&str> = Vec::new();
    for i in items {
        if !names.contains(&i.name.as_str()) {
            names.push(&i.name);
        }
    }
    names
}

fn state_item_intervals<'a>(
    definition: Option<&'a StateDefinitionData>,
    selected_state_item_id: Uuid,
    playback: PlaybackMode,
    iterations: i32,
    cycle_offset: i32,
    effect_duration: TimeSpan,
) -> Vec<Interval<'a>> {
    let Some(definition) = definition else { return Vec::new() };
    if effect_duration <= TimeSpan::ZERO || definition.items.is_empty() {
        return Vec::new();
    }
    let items = &definition.items;
    let whole = |item: &'a StateItemData| Interval { item, start: TimeSpan::ZERO, duration: effect_duration, color_override: None };
    if !selected_state_item_id.is_nil() {
        // CreateSelectedItemIntervals: every item sharing the selected item's name
        let Some(selected) = items.iter().find(|i| i.id == selected_state_item_id) else { return Vec::new() };
        return items.iter().filter(|i| i.name == selected.name).map(whole).collect();
    }
    match playback {
        PlaybackMode::Iterate => iterated_intervals(items, iterations, cycle_offset, effect_duration),
        PlaybackMode::Default => items.iter().map(whole).collect(),
    }
}

fn iterated_intervals<'a>(items: &'a [StateItemData], iterations: i32, cycle_offset: i32, effect_duration: TimeSpan) -> Vec<Interval<'a>> {
    let names = unique_names(items);
    if names.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut interval_start = TimeSpan::ZERO;
    let offset = normalize_cycle_offset(cycle_offset, names.len() as i64);
    let count = names.len() as i64 * normalize_iterations(iterations) as i64;
    for index in 0..count {
        let duration = interval_duration(effect_duration, count, index, interval_start);
        let name = names[offset_slot_index(index, names.len() as i64, offset)];
        for item in items.iter().filter(|i| i.name == name) {
            out.push(Interval { item, start: interval_start, duration, color_override: None });
        }
        interval_start += duration;
    }
    out
}

/// `StateMarkParser.ParseStateItemNames`.
fn parse_state_item_names(mark_text: &str) -> Vec<String> {
    mark_text.split(',').map(|s| s.trim().to_string()).collect()
}

#[allow(clippy::too_many_arguments)]
fn mark_collection_intervals<'a>(
    definition: Option<&'a StateDefinitionData>,
    marks: &[&Mark],
    playback: PlaybackMode,
    iterations: i32,
    cycle_offset: i32,
    effect_start: TimeSpan,
    effect_duration: TimeSpan,
) -> Vec<Interval<'a>> {
    let Some(definition) = definition else { return Vec::new() };
    if effect_duration <= TimeSpan::ZERO || definition.items.is_empty() {
        return Vec::new();
    }
    // GroupBy(Name) keeps first-appearance order inside each group; lookups are by name.
    let mut groups: HashMap<&str, Vec<&'a StateItemData>> = HashMap::new();
    for item in &definition.items {
        groups.entry(item.name.as_str()).or_default().push(item);
    }
    let effect_end = effect_start + effect_duration;
    let mut out = Vec::new();
    for mark in marks {
        let interval_start = mark.start_time.max(effect_start) - effect_start;
        let interval_end = mark.end_time().min(effect_end) - effect_start;
        let interval_duration_ = interval_end - interval_start;
        if interval_duration_ <= TimeSpan::ZERO {
            continue;
        }
        let names = parse_state_item_names(&mark.text);
        if playback == PlaybackMode::Iterate {
            if names.is_empty() {
                continue;
            }
            let mut segment_start = interval_start;
            let offset = normalize_cycle_offset(cycle_offset, names.len() as i64);
            let count = names.len() as i64 * normalize_iterations(iterations) as i64;
            for index in 0..count {
                let dur = interval_duration(interval_duration_, count, index, segment_start - interval_start);
                let name = &names[offset_slot_index(index, names.len() as i64, offset)];
                if !name.is_empty() {
                    if let Some(items) = groups.get(name.as_str()) {
                        for item in items {
                            out.push(Interval { item, start: segment_start, duration: dur, color_override: None });
                        }
                    }
                }
                segment_start += dur;
            }
        } else {
            let mut recognized: HashSet<&str> = HashSet::new();
            for name in &names {
                if name.is_empty() || !recognized.insert(name.as_str()) {
                    continue;
                }
                if let Some(items) = groups.get(name.as_str()) {
                    for item in items {
                        out.push(Interval { item, start: interval_start, duration: interval_duration_, color_override: None });
                    }
                }
            }
        }
    }
    out
}

struct CustomRow {
    state_item_id: Uuid,
    color: Color,
}

fn custom_intervals<'a>(
    definition: Option<&'a StateDefinitionData>,
    rows: &[CustomRow],
    playback: PlaybackMode,
    iterations: i32,
    cycle_individually: bool,
    cycle_offset: i32,
    effect_duration: TimeSpan,
) -> Vec<Interval<'a>> {
    let Some(definition) = definition else { return Vec::new() };
    if effect_duration <= TimeSpan::ZERO || rows.is_empty() {
        return Vec::new();
    }
    // ToDictionary(first by id)
    let mut by_id: HashMap<Uuid, &'a StateItemData> = HashMap::new();
    for item in &definition.items {
        by_id.entry(item.id).or_insert(item);
    }
    if playback != PlaybackMode::Iterate {
        let mut out = Vec::new();
        let mut rendered: HashSet<Uuid> = HashSet::new();
        for row in rows {
            if row.state_item_id.is_nil() || !rendered.insert(row.state_item_id) {
                continue;
            }
            let Some(item) = by_id.get(&row.state_item_id) else { continue };
            out.push(Interval { item, start: TimeSpan::ZERO, duration: effect_duration, color_override: Some(row.color) });
        }
        return out;
    }

    let mut out = Vec::new();
    let mut interval_start = TimeSpan::ZERO;
    if !cycle_individually {
        // CreateCustomStateItemGroups: consecutive rows with the same key form one slot.
        let key = |r: &CustomRow| -> String {
            if r.state_item_id.is_nil() {
                "None:".to_string()
            } else {
                match by_id.get(&r.state_item_id) {
                    Some(i) => format!("Name:{}", i.name),
                    None => format!("Missing:{}", r.state_item_id.simple()),
                }
            }
        };
        let mut groups: Vec<Vec<&CustomRow>> = Vec::new();
        let mut current: Vec<&CustomRow> = Vec::new();
        let mut current_key: Option<String> = None;
        for r in rows {
            let k = key(r);
            if !current.is_empty() && current_key.as_deref() != Some(k.as_str()) {
                groups.push(std::mem::take(&mut current));
            }
            current.push(r);
            current_key = Some(k);
        }
        if !current.is_empty() {
            groups.push(current);
        }
        if groups.is_empty() {
            return Vec::new();
        }
        let offset = normalize_cycle_offset(cycle_offset, groups.len() as i64);
        let count = groups.len() as i64 * normalize_iterations(iterations) as i64;
        for index in 0..count {
            let duration = interval_duration(effect_duration, count, index, interval_start);
            for r in &groups[offset_slot_index(index, groups.len() as i64, offset)] {
                if !r.state_item_id.is_nil() {
                    if let Some(item) = by_id.get(&r.state_item_id) {
                        out.push(Interval { item, start: interval_start, duration, color_override: Some(r.color) });
                    }
                }
            }
            interval_start += duration;
        }
        return out;
    }

    let offset = normalize_cycle_offset(cycle_offset, rows.len() as i64);
    let count = rows.len() as i64 * normalize_iterations(iterations) as i64;
    for index in 0..count {
        let duration = interval_duration(effect_duration, count, index, interval_start);
        let r = &rows[offset_slot_index(index, rows.len() as i64, offset)];
        if !r.state_item_id.is_nil() {
            if let Some(item) = by_id.get(&r.state_item_id) {
                out.push(Interval { item, start: interval_start, duration, color_override: Some(r.color) });
            }
        }
        interval_start += duration;
    }
    out
}

// ---------------------------------------------------------------------------------------------------------
// Effect

struct Segment {
    state_item_id: Uuid,
    leaf: NodeId,
    element_id: Uuid,
    color: Color,
    start: TimeSpan,
    duration: TimeSpan,
}

/// `StateRenderSegmentCoalescer.Coalesce`: back-to-back segments of one item on one element in one colour
/// merge. Only neighbours in the sequence are considered.
fn coalesce(segments: impl Iterator<Item = Segment>) -> Vec<Segment> {
    let mut out: Vec<Segment> = Vec::new();
    for s in segments {
        if let Some(last) = out.last_mut() {
            if last.state_item_id == s.state_item_id
                && last.element_id == s.element_id
                && last.color == s.color
                && last.start + last.duration == s.start
            {
                last.duration += s.duration;
                continue;
            }
        }
        out.push(s);
    }
    out
}

/// `State.ResolveStateItemColor`.
fn resolve_color(ctx: &RenderCtx, leaf: NodeId, state_item_color: Color) -> Option<Color> {
    if !ctx.is_element_discrete(leaf) {
        return Some(state_item_color);
    }
    let valid = ctx.tree.valid_colors(leaf, false);
    if valid.is_empty() {
        return None;
    }
    Some(if valid.contains(&state_item_color) { state_item_color } else { valid[0] })
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    let mut out = EffectIntents::new();
    let discovered = discover(ctx);
    let selected_id = p.guid("SelectedStateDefinitionId");
    let selected = discovered.iter().find(|d| d.state_definition_id == selected_id).map(|d| &d.definition);

    let playback = if p.enum_name("PlaybackMode") == "Iterate" { PlaybackMode::Iterate } else { PlaybackMode::Default };
    let iterations = normalize_iterations(p.int("Iterations"));
    let cycle_offset = p.int("CycleOffset");
    let intervals = match p.enum_name("RenderSource") {
        "Custom" => {
            let rows: Vec<CustomRow> = p
                .records("CustomStateItems")
                .iter()
                .map(|r| CustomRow { state_item_id: r.guid("StateItemId"), color: r.color("Color") })
                .collect();
            custom_intervals(selected, &rows, playback, iterations, p.boolean("CycleIndividually"), cycle_offset, ctx.time_span)
        }
        "MarkCollection" => match ctx.mark_collection(p.guid("MarkCollectionId")) {
            None => Vec::new(),
            Some(mc) => {
                let marks = mc.marks_inclusive_of_time(ctx.start_time, ctx.start_time + ctx.time_span);
                mark_collection_intervals(selected, &marks, playback, iterations, cycle_offset, ctx.start_time, ctx.time_span)
            }
        },
        _ => state_item_intervals(selected, p.guid("SelectedStateItemId"), playback, iterations, cycle_offset, ctx.time_span),
    };

    // Target scope: the targets and everything below them, the first node for each id.
    let mut scope: HashMap<Uuid, NodeId> = HashMap::new();
    for &t in ctx.targets {
        for n in ctx.tree.descendants(t) {
            scope.entry(ctx.tree.node(n).id).or_insert(n);
        }
    }

    let segments = intervals.iter().flat_map(|interval| {
        let mut v: Vec<Segment> = Vec::new();
        if interval.duration <= TimeSpan::ZERO {
            return v;
        }
        for node_id in &interval.item.element_node_ids {
            let Some(&assigned) = scope.get(node_id) else { continue };
            for leaf in ctx.tree.leaves(assigned) {
                let Some(element_id) = ctx.tree.node(leaf).element_id else { continue };
                let Some(color) = resolve_color(ctx, leaf, interval.color_override.unwrap_or(interval.item.color)) else { continue };
                v.push(Segment {
                    state_item_id: interval.item.id,
                    leaf,
                    element_id,
                    color,
                    start: interval.start,
                    duration: interval.duration,
                });
            }
        }
        v
    });

    let has_discrete = ctx.has_discrete_colors();
    for s in coalesce(segments) {
        let intent = ctx.create_intent(has_discrete, s.leaf, s.color, 1.0, s.duration);
        out.add_intent_for_element(s.element_id, intent, s.start);
    }
    out
}

// ---------------------------------------------------------------------------------------------------------
// Thumbnail

fn short_id(id: Uuid) -> String {
    id.simple().to_string()[..8].to_string()
}

/// `GetVisualRepresentationText`.
fn visual_text(p: &Params, t: &TargetInfo) -> String {
    let discovered = discover_in(t.tree, t.targets);
    let text = format!("State - {}", state_definition_label(p, &discovered));
    match p.enum_name("RenderSource") {
        "Custom" => {
            let grouped = p.enum_name("PlaybackMode") == "Iterate" && !p.boolean("CycleIndividually");
            if grouped {
                format!("{text} - Custom Group")
            } else {
                format!("{text} - Custom")
            }
        }
        "StateItem" => format!("{text} - {}", state_item_label(p, &discovered)),
        "MarkCollection" => format!("{text} - Marks"),
        _ => text,
    }
}

fn visual(p: &Params, t: &TargetInfo, g: &mut Graphics, rect: (i32, i32, i32, i32)) {
    let (x, _y, w, h) = rect;
    let text = visual_text(p, t);
    let font = adjusted_font(&text, w, h, 48);
    let _ = FONT_NAME;
    g.fill_rectangle(Color::DARK_GRAY, rect.0 as f32, rect.1 as f32, w as f32, h as f32);
    g.draw_string(&text, &font, Color::WHITE, (x + 2) as f32, 2.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_core::element::{PropertyData, PropertyInstance};
    use vixen_core::marks::MarkCollectionType;
    use vixen_effects::testing;

    const STATE_PROPERTY: Uuid = crate::property_data::STATE_ID;

    /// A tree with a "Prop" node holding a State property of one definition ("Gate") with items
    /// "Open" (red, assigned to leaf A) and "Closed" (blue, assigned to leaf B).
    struct Fixture {
        tree: ElementTree,
        prop: NodeId,
        a: NodeId,
        b: NodeId,
        def_id: Uuid,
        open_id: Uuid,
        closed_id: Uuid,
    }

    fn fixture() -> Fixture {
        let mut tree = ElementTree::new();
        let prop = tree.add_node(Uuid::new_v4(), "Prop", None);
        let a = tree.add_element("A");
        let b = tree.add_element("B");
        tree.add_child(prop, a);
        tree.add_child(prop, b);
        let (def_id, open_id, closed_id) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let a_id = tree.node(a).id;
        let b_id = tree.node(b).id;
        let colour = |argb: u32| format!("<Color><state>2</state><value>{argb}</value></Color>");
        let xml = format!(
            "<StateData><Id>{}</Id><StateDefinitions><StateDefinitionData><Id>{def_id}</Id><Name>Gate</Name><Items>\
             <StateItemData><Id>{open_id}</Id><Name>Open</Name>{}<ElementNodeIds><guid>{a_id}</guid></ElementNodeIds></StateItemData>\
             <StateItemData><Id>{closed_id}</Id><Name>Closed</Name>{}<ElementNodeIds><guid>{b_id}</guid></ElementNodeIds></StateItemData>\
             </Items></StateDefinitionData></StateDefinitions></StateData>",
            Uuid::new_v4(),
            colour(0xFFFF0000),
            colour(0xFF0000FF)
        );
        tree.node_mut(prop).properties.push(PropertyInstance {
            type_id: STATE_PROPERTY,
            instance_id: Uuid::new_v4(),
            data: PropertyData::Unknown { raw_xml: xml },
        });
        Fixture { tree, prop, a, b, def_id, open_id, closed_id }
    }

    #[test]
    fn custom_row_takes_the_items_colour_and_a_new_row_the_first_free_item() {
        let f = fixture();
        let mut p = DESCRIPTOR.new_params();
        p.set("SelectedStateDefinitionId", ParamValue::Guid(f.def_id));
        let targets = [f.prop];
        let info = TargetInfo { tree: &f.tree, targets: &targets, marks: &[], start_time: TimeSpan::ZERO, time_span: TimeSpan::from_ms(1000.0), audio_loaded: false };
        let red = Color::rgb(255, 0, 0);
        let blue = Color::rgb(0, 0, 255);
        let mut row = Params::defaults(&gen::CUSTOM_STATE_ITEM_PARAMS);
        row.set("StateItemId", ParamValue::Guid(f.open_id));
        row.set("Color", ParamValue::Color(red));
        DESCRIPTOR.set_param(&mut p, "CustomStateItems", ParamValue::RecordList(vec![row.clone()]), &info);
        // Choosing an item on a row gives the row that item's colour.
        let mut changed = row.clone();
        changed.set("StateItemId", ParamValue::Guid(f.closed_id));
        DESCRIPTOR.set_param(&mut p, "CustomStateItems", ParamValue::RecordList(vec![changed]), &info);
        assert_eq!(p.records("CustomStateItems")[0].color("Color"), blue);
        // `<None>` keeps the colour.
        let mut none = p.records("CustomStateItems")[0].clone();
        none.set("StateItemId", ParamValue::Guid(Uuid::nil()));
        DESCRIPTOR.set_param(&mut p, "CustomStateItems", ParamValue::RecordList(vec![none]), &info);
        assert_eq!(p.records("CustomStateItems")[0].color("Color"), blue);
        // A new row takes the first item no other row uses.
        let mut rows = p.records("CustomStateItems").to_vec();
        rows[0].set("StateItemId", ParamValue::Guid(f.open_id));
        rows.push(Params::defaults(&gen::CUSTOM_STATE_ITEM_PARAMS));
        DESCRIPTOR.set_param(&mut p, "CustomStateItems", ParamValue::RecordList(rows), &info);
        assert_eq!(p.records("CustomStateItems")[1].guid("StateItemId"), f.closed_id);
        assert_eq!(p.records("CustomStateItems")[1].color("Color"), blue);
    }

    #[test]
    fn custom_rows_discrete_colours_come_from_the_items_elements() {
        let f = fixture();
        let mut p = DESCRIPTOR.new_params();
        p.set("SelectedStateDefinitionId", ParamValue::Guid(f.def_id));
        let targets = [f.prop];
        let info = TargetInfo { tree: &f.tree, targets: &targets, marks: &[], start_time: TimeSpan::ZERO, time_span: TimeSpan::from_ms(1000.0), audio_loaded: false };
        let mut row = Params::defaults(&gen::CUSTOM_STATE_ITEM_PARAMS);
        assert!(custom_item_valid_colors(&row, &p, &info).is_empty());
        row.set("StateItemId", ParamValue::Guid(f.open_id));
        // Leaf A has no colour property: full colour.
        assert!(custom_item_valid_colors(&row, &p, &info).is_empty());
    }

    fn lighting(i: &IntentNode) -> (Color, f64) {
        match i.intent.state_at(TimeSpan::ZERO) {
            vixen_core::StateValue::Lighting(v) => (v.color, v.intensity),
            other => panic!("{other:?}"),
        }
    }

    fn render_state(f: &Fixture, ms: i64, marks: &[MarkCollection], edit: impl FnOnce(&mut Params)) -> EffectIntents {
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let targets = [f.prop];
        let ctx = RenderCtx { video_cache: None,
            tree: &f.tree,
            targets: &targets,
            start_time: TimeSpan::from_ms_i(1000),
            time_span: TimeSpan::from_ms_i(ms),
            frame_ms: 50,
            marks,
            audio: None,
            module_data_root: None,
            cancel: &cancel,
        };
        let mut p = DESCRIPTOR.new_params();
        p.set("SelectedStateDefinitionId", ParamValue::Guid(f.def_id));
        edit(&mut p);
        render(&ctx, &p)
    }

    #[test]
    fn smoke() {
        testing::smoke_test(&DESCRIPTOR);
        // The default has no State property to render: the C# renders nothing, so light is checked below.
    }

    #[test]
    fn default_playback_lights_every_item_for_the_whole_effect() {
        let f = fixture();
        let out = render_state(&f, 2000, &[], |_| {});
        let ea = f.tree.node(f.a).element_id.unwrap();
        let eb = f.tree.node(f.b).element_id.unwrap();
        let na = out.nodes_for(&ea).unwrap();
        assert_eq!(na.len(), 1);
        assert_eq!(lighting(&na[0]), (Color::RED, 1.0));
        assert_eq!(na[0].start, TimeSpan::ZERO);
        assert_eq!(na[0].intent.span, TimeSpan::from_ms_i(2000));
        assert_eq!(lighting(&out.nodes_for(&eb).unwrap()[0]).0, Color::BLUE);
    }

    #[test]
    fn selecting_a_state_item_lights_only_that_name() {
        let f = fixture();
        let out = render_state(&f, 1000, &[], |p| {
            p.set("SelectedStateItemId", ParamValue::Guid(f.closed_id));
        });
        assert!(out.nodes_for(&f.tree.node(f.a).element_id.unwrap()).is_none());
        assert!(out.nodes_for(&f.tree.node(f.b).element_id.unwrap()).is_some());
        // an id the definition does not have renders nothing
        let none = render_state(&f, 1000, &[], |p| {
            p.set("SelectedStateItemId", ParamValue::Guid(Uuid::new_v4()));
        });
        assert!(none.is_empty());
    }

    #[test]
    fn cycle_mode_splits_time_between_item_names_with_offset() {
        let f = fixture();
        let cycle = |p: &mut Params| {
            p.set("PlaybackMode", ParamValue::Enum("Iterate".into()));
        };
        let out = render_state(&f, 1000, &[], cycle);
        let na = out.nodes_for(&f.tree.node(f.a).element_id.unwrap()).unwrap();
        let nb = out.nodes_for(&f.tree.node(f.b).element_id.unwrap()).unwrap();
        // Open first (0..500), Closed second (500..1000)
        assert_eq!((na[0].start, na[0].intent.span), (TimeSpan::ZERO, TimeSpan::from_ms_i(500)));
        assert_eq!((nb[0].start, nb[0].intent.span), (TimeSpan::from_ms_i(500), TimeSpan::from_ms_i(500)));
        // two iterations: four slots of 250 ms, alternating, so A has two intents
        let out = render_state(&f, 1000, &[], |p| {
            cycle(p);
            p.set("Iterations", ParamValue::Int(2));
        });
        let na = out.nodes_for(&f.tree.node(f.a).element_id.unwrap()).unwrap();
        assert_eq!(na.len(), 2);
        assert_eq!(na[1].start, TimeSpan::from_ms_i(500));
        // cycle offset 1 starts with Closed
        let out = render_state(&f, 1000, &[], |p| {
            cycle(p);
            p.set("CycleOffset", ParamValue::Int(1));
        });
        let nb = out.nodes_for(&f.tree.node(f.b).element_id.unwrap()).unwrap();
        assert_eq!(nb[0].start, TimeSpan::ZERO);
    }

    #[test]
    fn mark_collection_names_drive_the_items() {
        let f = fixture();
        let mut mc = MarkCollection::default();
        mc.collection_type = MarkCollectionType::Generic;
        // effect is 1000..3000 ms
        let mut m1 = Mark::new(TimeSpan::from_ms_i(1500));
        m1.duration = TimeSpan::from_ms_i(500);
        m1.text = " Open , Missing ".into();
        let mut m2 = Mark::new(TimeSpan::from_ms_i(2800));
        m2.duration = TimeSpan::from_ms_i(1000); // runs past the end: clipped to 3000
        m2.text = "Closed".into();
        mc.add_mark(m1);
        mc.add_mark(m2);
        let id = mc.id;
        let marks = [mc];
        let out = render_state(&f, 2000, &marks, |p| {
            p.set("RenderSource", ParamValue::Enum("MarkCollection".into()));
            p.set("MarkCollectionId", ParamValue::Guid(id));
        });
        let na = out.nodes_for(&f.tree.node(f.a).element_id.unwrap()).unwrap();
        assert_eq!((na[0].start, na[0].intent.span), (TimeSpan::from_ms_i(500), TimeSpan::from_ms_i(500)));
        let nb = out.nodes_for(&f.tree.node(f.b).element_id.unwrap()).unwrap();
        assert_eq!((nb[0].start, nb[0].intent.span), (TimeSpan::from_ms_i(1800), TimeSpan::from_ms_i(200)));
        // no collection selected: nothing
        assert!(render_state(&f, 2000, &marks, |p| {
            p.set("RenderSource", ParamValue::Enum("MarkCollection".into()));
        })
        .is_empty());
    }

    #[test]
    fn custom_rows_override_colour_and_skip_duplicates() {
        let f = fixture();
        let row = |id: Uuid, c: Color| {
            let mut r = Params::defaults(&gen::CUSTOM_STATE_ITEM_PARAMS);
            r.set("StateItemId", ParamValue::Guid(id));
            r.set("Color", ParamValue::Color(c));
            r
        };
        let out = render_state(&f, 1000, &[], |p| {
            p.set("RenderSource", ParamValue::Enum("Custom".into()));
            p.set(
                "CustomStateItems",
                ParamValue::RecordList(vec![row(f.open_id, Color::LIME), row(f.open_id, Color::YELLOW), row(Uuid::nil(), Color::RED)]),
            );
        });
        let na = out.nodes_for(&f.tree.node(f.a).element_id.unwrap()).unwrap();
        assert_eq!(na.len(), 1);
        assert_eq!(lighting(&na[0]).0, Color::LIME);
        assert!(out.nodes_for(&f.tree.node(f.b).element_id.unwrap()).is_none());
    }

    #[test]
    fn grouped_cycle_merges_consecutive_rows_of_one_name() {
        let f = fixture();
        let row = |id: Uuid| {
            let mut r = Params::defaults(&gen::CUSTOM_STATE_ITEM_PARAMS);
            r.set("StateItemId", ParamValue::Guid(id));
            r
        };
        // Open, Open, Closed with CycleIndividually off is two slots; on, three.
        let rows = vec![row(f.open_id), row(f.open_id), row(f.closed_id)];
        let mk = |individually: bool| {
            let rows = rows.clone();
            render_state(&f, 900, &[], move |p| {
                p.set("RenderSource", ParamValue::Enum("Custom".into()));
                p.set("PlaybackMode", ParamValue::Enum("Iterate".into()));
                p.set("CycleIndividually", ParamValue::Bool(individually));
                p.set("CustomStateItems", ParamValue::RecordList(rows));
            })
        };
        let grouped = mk(false);
        let na = grouped.nodes_for(&f.tree.node(f.a).element_id.unwrap()).unwrap();
        // both Open rows share slot 0 (0..450). They are not back to back, so each keeps its own intent, as
        // in the C# (the coalescer only joins a segment to the one ending where it starts).
        assert_eq!(na.len(), 2);
        assert!(na.iter().all(|n| n.start == TimeSpan::ZERO && n.intent.span == TimeSpan::from_ms_i(450)));
        let individual = mk(true);
        let na = individual.nodes_for(&f.tree.node(f.a).element_id.unwrap()).unwrap();
        // slots 0 and 1 are both Open: coalesced into 0..600
        assert_eq!(na.len(), 1);
        assert_eq!(na[0].intent.span, TimeSpan::from_ms_i(600));
    }

    #[test]
    fn discrete_leaf_uses_its_valid_colour() {
        let mut f = fixture();
        testing::make_discrete(&mut f.tree, f.prop, Color::LIME);
        let out = render_state(&f, 1000, &[], |_| {});
        let na = out.nodes_for(&f.tree.node(f.a).element_id.unwrap()).unwrap();
        // Red is not a valid colour of a single-colour lime leaf: falls back to its first valid colour.
        assert!(matches!(na[0].intent.state_at(TimeSpan::ZERO), vixen_core::StateValue::Discrete(v) if v.color == Color::LIME));
    }

    #[test]
    fn visibility_rules_and_visual() {
        let mut p = DESCRIPTOR.new_params();
        let names = |p: &Params| -> Vec<&'static str> {
            DESCRIPTOR.visible_params(p).iter().map(|i| DESCRIPTOR.params[*i].name).collect()
        };
        let n = names(&p);
        assert!(n.contains(&"StateItem") && !n.contains(&"Iterations") && !n.contains(&"CustomStateItems"));
        p.set("RenderSource", ParamValue::Enum("Custom".into()));
        p.set("PlaybackMode", ParamValue::Enum("Iterate".into()));
        p.set("CycleIndividually", ParamValue::Bool(false));
        let n = names(&p);
        assert!(n.contains(&"CustomStateItems") && n.contains(&"CycleIndividually") && n.contains(&"Iterations"));

        assert_eq!(visual_text(&p, &TargetInfo::detached()), "State - <No States Available> - Custom Group");
        p.set("RenderSource", ParamValue::Enum("StateItem".into()));
        assert_eq!(visual_text(&p, &TargetInfo::detached()), "State - <No States Available> - <No State Items Available>");
        let f = fixture();
        let targets = [f.prop];
        let t = testing::target_info(&f.tree, &targets);
        DESCRIPTOR.set_targets(&mut p, &t);
        assert_eq!(visual_text(&p, &t), "State - Gate - <All>");

        let mut g = Graphics::new(200, 40);
        visual(&p, &t, &mut g, (0, 0, 200, 40));
        assert_eq!(g.bitmap().get_pixel(198, 38), Color::DARK_GRAY);
    }

    #[test]
    fn the_visual_is_only_forced_when_show_effect_visual_is_on() {
        let mut p = DESCRIPTOR.new_params();
        assert!(DESCRIPTOR.forces_visual(&p));
        p.set("ShowEffectVisual", ParamValue::Bool(false));
        assert!(!DESCRIPTOR.forces_visual(&p));
    }

    #[test]
    fn targets_choose_a_definition_and_loading_leaves_the_saved_one() {
        let f = fixture();
        let targets = [f.prop];
        let t = testing::target_info(&f.tree, &targets);
        let mut p = DESCRIPTOR.new_params();
        let mut loaded = p.clone();
        DESCRIPTOR.load_data(&mut loaded, &t);
        assert!(loaded.guid("SelectedStateDefinitionId").is_nil());
        DESCRIPTOR.set_targets(&mut p, &t);
        assert!(!p.guid("SelectedStateDefinitionId").is_nil());
        assert_eq!(p.string("StateDefinition"), "Gate");
        assert_eq!(p.string("StateItem"), "<All>");
    }

    #[test]
    fn labels_select_by_name_and_custom_gets_a_first_row() {
        let f = fixture();
        let targets = [f.prop];
        let t = testing::target_info(&f.tree, &targets);
        let mut p = DESCRIPTOR.new_params();
        DESCRIPTOR.set_targets(&mut p, &t);
        DESCRIPTOR.set_param(&mut p, "StateItem", ParamValue::Str("Open".into()), &t);
        assert!(!p.guid("SelectedStateItemId").is_nil());
        assert_eq!(p.string("StateItem"), "Open");
        DESCRIPTOR.set_param(&mut p, "StateItem", ParamValue::Str("<All>".into()), &t);
        assert!(p.guid("SelectedStateItemId").is_nil());
        DESCRIPTOR.set_param(&mut p, "StateItem", ParamValue::Str("Not a state".into()), &t);
        assert_eq!(p.string("StateItem"), "<All>");

        DESCRIPTOR.set_param(&mut p, "RenderSource", ParamValue::Enum("Custom".into()), &t);
        assert_eq!(p.records("CustomStateItems").len(), 1);

        DESCRIPTOR.set_param(&mut p, "Iterations", ParamValue::Int(0), &t);
        assert_eq!(p.int("Iterations"), 1);
    }

    #[test]
    fn the_drop_downs_come_from_the_targets() {
        let f = fixture();
        let targets = [f.prop];
        let t = testing::target_info(&f.tree, &targets);
        let mut p = DESCRIPTOR.new_params();
        let labels = |v: Vec<SelectionValue>| v.into_iter().map(|c| c.label).collect::<Vec<_>>();
        assert_eq!(labels(DESCRIPTOR.selection_values("StateDefinition", None, &p, &TargetInfo::detached())), ["<No States Available>"]);
        DESCRIPTOR.set_targets(&mut p, &t);
        assert_eq!(labels(DESCRIPTOR.selection_values("StateDefinition", None, &p, &t)), ["Gate"]);
        assert_eq!(labels(DESCRIPTOR.selection_values("StateItem", None, &p, &t)), ["<All>", "Open", "Closed"]);

        let mut row = Params::defaults(&gen::CUSTOM_STATE_ITEM_PARAMS);
        row.set("StateItemId", ParamValue::Guid(f.open_id));
        let mut other = row.clone();
        other.set("StateItemId", ParamValue::Guid(f.closed_id));
        p.set("CustomStateItems", ParamValue::RecordList(vec![row.clone(), other]));
        // In Default playback the other row's item isn't offered.
        assert_eq!(labels(DESCRIPTOR.selection_values("StateItemId", Some(&row), &p, &t)), ["Open"]);
        p.set("PlaybackMode", ParamValue::Enum("Iterate".into()));
        assert_eq!(labels(DESCRIPTOR.selection_values("StateItemId", Some(&row), &p, &t)), ["<None>", "Open", "Closed"]);
    }

    #[test]
    fn items_with_the_same_name_are_told_apart() {
        let item = |name: &str, ids: Vec<Uuid>| StateItemData { id: Uuid::new_v4(), name: name.to_string(), color: Color::RED, element_node_ids: ids };
        let tree = ElementTree::new();
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        let items = [item("On", vec![a]), item("On", vec![b, a]), item("On", Vec::new()), item("Off", Vec::new())];
        let labels = custom_item_labels(&tree, &[], &items);
        assert_eq!(labels[0].1, format!("On ({})", short_id(a)));
        assert_eq!(labels[1].1, format!("On ({} + 1 more)", short_id(b)));
        assert_eq!(labels[2].1, "On (No assignments)");
        assert_eq!(labels[3].1, "Off");
    }

    #[test]
    fn mark_selection_prefers_a_state_collection() {
        let f = fixture();
        let targets = [f.prop];
        let mut generic = MarkCollection::default();
        generic.collection_type = MarkCollectionType::Generic;
        let mut states = MarkCollection::default();
        states.collection_type = MarkCollectionType::State;
        let marks = [generic, states];
        let t = TargetInfo { marks: &marks, ..testing::target_info(&f.tree, &targets) };
        let mut p = DESCRIPTOR.new_params();
        DESCRIPTOR.set_param(&mut p, "RenderSource", ParamValue::Enum("MarkCollection".into()), &t);
        assert_eq!(p.guid("MarkCollectionId"), marks[1].id);
    }

    #[test]
    fn defaults() {
        let p = DESCRIPTOR.new_params();
        assert_eq!(p.int("Iterations"), 1);
        assert_eq!(p.records("CustomStateItems").len(), 0);
    }

    #[test]
    fn robust_without_properties_or_duration() {
        let f = fixture();
        assert!(render_state(&f, 0, &[], |_| {}).is_empty());
        assert!(render_state(&f, 1000, &[], |p| {
            p.set("SelectedStateDefinitionId", ParamValue::Guid(Uuid::nil()));
        })
        .is_empty());
    }
}
