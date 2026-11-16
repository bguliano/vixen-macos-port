//! `PropertyDiscovery`, F313: which of an effect's properties a colour, gradient or curve can land on. The effect's parameter tables and its visibility rules replace reflection.

use vixen_effects::framework::{EffectDescriptor, ParamKind, ParamValue, Params, TargetInfo, Visibility};

/// The property types drops care about.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum PropType {
    Color,
    Gradient,
    GradientList,
    GradientLevels,
    Curve,
}

/// `ColorTypes`.
pub const COLOR_TYPES: &[PropType] = &[PropType::Color];
/// `GradientTypes`.
pub const GRADIENT_TYPES: &[PropType] = &[PropType::Gradient, PropType::GradientList, PropType::GradientLevels];
/// `AllColorTypes`.
pub const ALL_COLOR_TYPES: &[PropType] = &[PropType::Gradient, PropType::GradientList, PropType::GradientLevels, PropType::Color];
/// `CurveTypes`.
pub const CURVE_TYPES: &[PropType] = &[PropType::Curve, PropType::GradientLevels];

/// `PropertyMetaData`: a browsable property, reached directly or through an item of a collection.
#[derive(Clone, PartialEq, Debug)]
pub struct EffectProp {
    pub name: &'static str,
    pub label: &'static str,
    pub kind: PropType,
    /// The collection property and item index it belongs to: `PropertyOwnerMetaData`'s `CollectionIndex`.
    pub owner: Option<(&'static str, usize)>,
    /// `OwnerDisplayName`, for grouping in the picker.
    pub owner_label: Option<&'static str>,
}

impl EffectProp {
    pub fn is_list(&self) -> bool {
        matches!(self.kind, PropType::GradientList | PropType::GradientLevels)
    }

    /// The value now on an effect.
    pub fn value<'a>(&self, params: &'a Params) -> Option<&'a ParamValue> {
        match self.owner {
            None => params.index_of(self.name).map(|i| &params.values[i]),
            Some((list, index)) => {
                let ParamValue::RecordList(items) = params.get(list) else { return None };
                let item = items.get(index)?;
                item.index_of(self.name).map(|i| &item.values[i])
            }
        }
    }
}

fn prop_type(kind: &ParamKind) -> Option<PropType> {
    match kind {
        // `Rgb` is how Set Level stores its `Color` property.
        ParamKind::Color | ParamKind::Rgb => Some(PropType::Color),
        ParamKind::Gradient => Some(PropType::Gradient),
        ParamKind::GradientList => Some(PropType::GradientList),
        ParamKind::GradientLevels => Some(PropType::GradientLevels),
        ParamKind::Curve => Some(PropType::Curve),
        _ => None,
    }
}

/// `CollectProperties`: visible properties of these types, in table order, going into each item of a collection. A property in the set isn't gone into further.
pub fn collect_props(descriptor: &EffectDescriptor, params: &Params, info: &TargetInfo, types: &[PropType]) -> Vec<EffectProp> {
    let mut visible = descriptor.visible_params_for(params, info);
    // In table order rather than the editor's display order, as `TypeDescriptor.GetProperties` gives them.
    visible.sort_unstable();
    let mut out = Vec::new();
    for index in visible {
        let desc = &descriptor.params[index];
        if let Some(kind) = prop_type(&desc.kind) {
            if types.contains(&kind) {
                out.push(EffectProp { name: desc.name, label: desc.label, kind, owner: None, owner_label: None });
                continue;
            }
        }
        if let (ParamKind::RecordList(record), ParamValue::RecordList(items)) = (&desc.kind, &params.values[index]) {
            for (item_index, item) in items.iter().enumerate() {
                let mut visibility = Visibility::default();
                if let Some(f) = record.visibility {
                    f(item, params, &mut visibility);
                }
                for inner in record.params.iter() {
                    if !visibility.is_visible(inner) {
                        continue;
                    }
                    if let Some(kind) = prop_type(&inner.kind) {
                        if types.contains(&kind) {
                            out.push(EffectProp { name: inner.name, label: inner.label, kind, owner: Some((desc.name, item_index)), owner_label: Some(desc.label) });
                        }
                    }
                }
            }
        }
    }
    out
}

/// `ContainsTypes`.
pub fn contains_types(descriptor: &EffectDescriptor, params: &Params, info: &TargetInfo, types: &[PropType]) -> bool {
    !collect_props(descriptor, params, info, types).is_empty()
}

/// `SupportsColor`'s list: every colour property.
pub fn color_properties(descriptor: &EffectDescriptor, params: &Params, info: &TargetInfo) -> Vec<EffectProp> {
    collect_props(descriptor, params, info, ALL_COLOR_TYPES)
}

/// Writes a value to the property, in `params` directly when it's inside a collection. Returns the value to give the session's setter when it's a top-level one.
pub fn set_nested(params: &mut Params, prop: &EffectProp, value: ParamValue) -> bool {
    let Some((list, index)) = prop.owner else { return false };
    let Some(at) = params.index_of(list) else { return false };
    let ParamValue::RecordList(items) = &mut params.values[at] else { return false };
    let Some(item) = items.get_mut(index) else { return false };
    if item.index_of(prop.name).is_none() {
        return false;
    }
    item.set(prop.name, value);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_core::{ElementTree, Uuid};

    fn info(_tree: &ElementTree) -> TargetInfo<'static> {
        TargetInfo::detached()
    }

    fn descriptor(name: &str) -> &'static EffectDescriptor {
        vixen_effect_registry::all().iter().copied().find(|d| d.name == name).unwrap()
    }

    #[test]
    fn set_level_takes_a_colour_and_a_pixel_effect_takes_gradients() {
        let tree = ElementTree::new();
        let set_level = descriptor("Set Level");
        let props = collect_props(set_level, &set_level.new_params(), &info(&tree), COLOR_TYPES);
        assert_eq!(props.len(), 1);
        assert_eq!(props[0].kind, PropType::Color);
        assert!(!contains_types(set_level, &set_level.new_params(), &info(&tree), GRADIENT_TYPES));

        let butterfly = descriptor("Butterfly");
        let props = collect_props(butterfly, &butterfly.new_params(), &info(&tree), ALL_COLOR_TYPES);
        assert!(!props.is_empty());
        assert!(props.iter().all(|p| p.kind != PropType::Color));
        assert!(contains_types(butterfly, &butterfly.new_params(), &info(&tree), GRADIENT_TYPES));
    }

    #[test]
    fn a_collection_is_gone_into_item_by_item() {
        let tree = ElementTree::new();
        let wave = descriptor("Wave");
        let params = wave.new_params();
        let props = collect_props(wave, &params, &info(&tree), ALL_COLOR_TYPES);
        let nested: Vec<&EffectProp> = props.iter().filter(|p| p.owner.is_some()).collect();
        assert!(!nested.is_empty(), "the wave forms hold colours");
        for p in &nested {
            assert!(p.value(&params).is_some());
        }
        let first = nested[0];
        let mut changed = params.clone();
        assert!(set_nested(&mut changed, first, first.value(&params).unwrap().clone()));
        assert!(changed == params);
    }

    #[test]
    fn unknown_type_ids_have_no_descriptor_to_ask() {
        assert!(vixen_effect_registry::by_type_id(&Uuid::nil()).is_none());
    }
}
