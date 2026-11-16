use super::children::{Children, Ranges};
use super::*;
use crate::{ElementModelType, StateDefinitionModel, StateItemModel};
use std::collections::HashSet;
use uuid::Uuid;

pub(super) fn assemble(
    name: &str,
    pixel: i32,
    geometry: Geometry,
    children: Children,
    now: &str,
) -> Result<Prop, PropError> {
    let width = geometry
        .nodes
        .values()
        .map(|v| v.0 as i64 + 7 + pixel as i64)
        .max()
        .unwrap_or(geometry.width as i64 + 14)
        .max(800);
    let height = geometry
        .nodes
        .values()
        .map(|v| v.1 as i64 + 7 + pixel as i64)
        .max()
        .unwrap_or(geometry.height as i64 + 14)
        .max(600);
    // The source bitmap allocator throws on extreme dimensions. Fail before attempting its huge allocation.
    if width > 100_000 || height > 100_000 || width * height > 64_000_000 {
        return Err(failed("The imported model's canvas is too large."));
    }
    let prefix = format!("{name} {{1}}");
    let mut prop = Prop::with_size(&prefix, width as u32, height as u32, "xModel Import", now);
    let model = group(
        &mut prop,
        &format!("{prefix} - Model"),
        None,
        ElementModelType::Model,
    );
    let mut lights = BTreeMap::new();
    for (order, (x, y)) in &geometry.nodes {
        let id = prop.add_light_node(
            Some(model),
            *x as f64 + 7.0,
            *y as f64 + 7.0,
            Some(*order),
            pixel,
            Some(&format!("{prefix} Px {order}")),
        )?;
        lights.insert(*order, id);
    }
    prop.physical.node_count = lights.len().to_string();
    let mut used = HashSet::new();
    let mut definitions = Vec::new();
    for state in &children.states {
        let mut definition_name = state.name.clone();
        let mut suffix = 2;
        while !used.insert(definition_name.clone()) {
            definition_name = format!("{} - {suffix}", state.name);
            suffix += 1;
        }
        let mut sorted: Vec<_> = state.items.iter().collect();
        sorted.sort_by_key(|v| v.index);
        let items = sorted
            .into_iter()
            .filter_map(|item| {
                let ids = resolve(item.ranges.as_ref()?, &lights);
                if ids.is_empty() {
                    return None;
                }
                Some(StateItemModel {
                    id: Uuid::new_v4(),
                    name: item.name.clone(),
                    color: item.color,
                    element_ids: ids,
                })
            })
            .collect::<Vec<_>>();
        if !items.is_empty() {
            definitions.push(StateDefinitionModel {
                id: Uuid::new_v4(),
                name: definition_name,
                description: String::new(),
                items,
            });
        }
    }
    prop.element_mut(model).unwrap().state_definitions = definitions;
    for (sub, lines) in &children.subs {
        if lines.is_empty() {
            continue;
        }
        let id = group(
            &mut prop,
            &format!("{prefix} - {sub}"),
            None,
            ElementModelType::SubModel,
        );
        let actual = prop.element(id).unwrap().name().to_string();
        let mut index = 1;
        for ranges in lines {
            if ranges.is_empty() {
                continue;
            }
            // This duplicated prefix in line-group names is deliberate upstream behavior.
            let target = if lines.len() > 1 {
                group(
                    &mut prop,
                    &format!("{prefix} - {sub} - {actual} {index}"),
                    Some(id),
                    ElementModelType::None,
                )
            } else {
                id
            };
            link(&mut prop, target, ranges, &lights);
            index += 1;
        }
    }
    if !children.faces.is_empty() {
        let parent = group(
            &mut prop,
            &format!("{prefix} - Faces "),
            None,
            ElementModelType::None,
        );
        for (face, items) in &children.faces {
            let id = group(
                &mut prop,
                &format!("{prefix} - {face} "),
                Some(parent),
                ElementModelType::FaceInfo,
            );
            for item in items {
                let child = group(
                    &mut prop,
                    &format!("{prefix} - {face} - {}", item.name),
                    Some(id),
                    ElementModelType::None,
                );
                prop.element_mut(child).unwrap().face = item.definition;
                link(&mut prop, child, &item.ranges, &lights);
            }
        }
    }
    if !children.states.is_empty() {
        let parent = group(
            &mut prop,
            &format!("{prefix} - States "),
            None,
            ElementModelType::None,
        );
        for state in &children.states {
            let id = group(
                &mut prop,
                &format!("{prefix} - {} ", state.name),
                Some(parent),
                ElementModelType::StateInfo,
            );
            for item in &state.items {
                let child = group(
                    &mut prop,
                    &format!(
                        "{prefix} - {} - S{} - {}",
                        state.name, item.index, item.name
                    ),
                    Some(id),
                    ElementModelType::None,
                );
                if let Some(ranges) = &item.ranges {
                    link(&mut prop, child, ranges, &lights);
                }
            }
        }
    }
    if !geometry.rings.is_empty()
        && !children.subs.iter().any(|(n, lines)| {
            n.eq_ignore_ascii_case("Circles") && matches_rings(lines, &geometry.rings)
        })
    {
        let parent = group(
            &mut prop,
            &format!("{prefix} - Circles"),
            None,
            ElementModelType::SubModel,
        );
        for (index, ring) in geometry.rings.iter().enumerate() {
            let id = group(
                &mut prop,
                &format!("{prefix} - Circle {}", index + 1),
                Some(parent),
                ElementModelType::SubModel,
            );
            for order in ring {
                if let Some(light) = lights.get(order) {
                    prop.add_to_parent(*light, id);
                }
            }
        }
    }
    Ok(prop)
}
fn group(prop: &mut Prop, name: &str, parent: Option<Uuid>, kind: ElementModelType) -> Uuid {
    let id = prop.create_node(name, parent, false);
    prop.element_mut(id).unwrap().model_type = kind;
    id
}
fn resolve(ranges: &Ranges, lights: &BTreeMap<i32, Uuid>) -> Vec<Uuid> {
    let mut ids = Vec::new();
    let mut seen = HashSet::new();
    // Visit existing keys, not potentially billions of absent integers in an input range.
    for &(a, b) in ranges {
        if a <= b {
            for (_, id) in lights.range(a..=b) {
                if seen.insert(*id) {
                    ids.push(*id);
                }
            }
        } else {
            for (_, id) in lights.range(b..=a).rev() {
                if seen.insert(*id) {
                    ids.push(*id);
                }
            }
        }
    }
    ids
}
fn link(prop: &mut Prop, parent: Uuid, ranges: &Ranges, lights: &BTreeMap<i32, Uuid>) {
    for id in resolve(ranges, lights) {
        prop.add_to_parent(id, parent);
    }
}
fn matches_rings(lines: &[Ranges], rings: &[Vec<i32>]) -> bool {
    if lines.len() != rings.len() {
        return false;
    }
    let mut remaining: Vec<_> = rings.iter().collect();
    for line in lines {
        let positive_count: u64 = line
            .iter()
            .map(|&(a, b)| {
                let lo = a.min(b).max(1) as i64;
                let hi = a.max(b) as i64;
                (hi - lo + 1).max(0) as u64
            })
            .sum();
        let mut matched = None;
        for (index, ring) in remaining.iter().enumerate() {
            if ring.len() as u64 != positive_count {
                continue;
            }
            let orders = line.iter().flat_map(|&(a, b)| {
                let lo = a.min(b).max(1) as i64;
                let hi = a.max(b) as i64;
                let count = (hi - lo + 1).max(0);
                (0..count).map(move |n| {
                    if a <= b {
                        (lo + n) as i32
                    } else {
                        (hi - n) as i32
                    }
                })
            });
            if orders.eq(ring.iter().copied()) {
                matched = Some(index);
                break;
            }
        }
        if let Some(index) = matched {
            remaining.remove(index);
        } else {
            return false;
        }
    }
    remaining.is_empty()
}
