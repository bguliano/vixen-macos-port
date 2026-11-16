//! The libraries kept in the module store's type-level data: curves, gradients and colour sets.

use crate::dcs::{child_string, format_guid};
use crate::module_store::{ModuleEntry, ModuleStore};
use crate::values::{
    color_list_from_xml, color_list_to_xml, curve_from_xml, curve_to_xml, gradient_from_xml, gradient_to_xml,
};
use uuid::{uuid, Uuid};
use vixen_core::{Color, ColorGradient, Curve};
use vixen_xml::{XElement, XNode, ARRAYS_NS};

/// `CurveLibraryDescriptor.ModuleID`.
pub const CURVE_LIBRARY_TYPE_ID: Uuid = uuid!("4e258de2-7a75-4f0f-aa43-c8182e7f3400");
/// `ColorGradientLibraryDescriptor.ModuleID`.
pub const GRADIENT_LIBRARY_TYPE_ID: Uuid = uuid!("64f4ab26-3ed4-49a3-a004-23656ed0424a");
/// `ColorDescriptor.ModuleId`, whose type data holds the colour sets.
pub const COLOR_PROPERTY_TYPE_ID: Uuid = uuid!("bff34727-6b88-4f87-82b7-68424498c725");

const NS_CURVES: &str = "http://schemas.datacontract.org/2004/07/VixenModules.App.Curves";
const NS_GRADIENTS: &str = "http://schemas.datacontract.org/2004/07/VixenModules.App.ColorGradients";
const NS_COLOR: &str = "http://schemas.datacontract.org/2004/07/VixenModules.Property.Color";

// The element names DataContractSerializer gives these dictionaries' entries.
const CURVE_ITEM: &str = "KeyValueOfstringCurvesLZZwEMY";
const GRADIENT_ITEM: &str = "KeyValueOfstringColorGradientGbtyavQ7";
const COLOR_SET_ITEM: &str = "KeyValueOfstringArrayOfColor3odnvp_PE";

/// The named curves of `CurveLibraryStaticData`, in file order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CurveLibrary {
    pub curves: Vec<(String, Curve)>,
}

impl CurveLibrary {
    pub fn get(&self, name: &str) -> Option<&Curve> {
        self.curves.iter().find(|(n, _)| n == name).map(|(_, c)| c)
    }
}

/// The named gradients of `ColorGradientLibraryStaticData`, in file order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GradientLibrary {
    pub gradients: Vec<(String, ColorGradient)>,
}

impl GradientLibrary {
    pub fn get(&self, name: &str) -> Option<&ColorGradient> {
        self.gradients.iter().find(|(n, _)| n == name).map(|(_, g)| g)
    }
}

/// Reads the curve library. An absent or unreadable entry is an empty library, as in C#.
pub fn curve_library(store: &ModuleStore) -> CurveLibrary {
    let curves = library_items(store, CURVE_LIBRARY_TYPE_ID, curve_from_xml);
    CurveLibrary { curves }
}

/// Reads the gradient library.
pub fn gradient_library(store: &ModuleStore) -> GradientLibrary {
    let gradients = library_items(store, GRADIENT_LIBRARY_TYPE_ID, gradient_from_xml);
    GradientLibrary { gradients }
}

/// Reads the colour sets, in file order. Without an entry these are `ColorStaticData`'s constructor's two.
pub fn color_sets(store: &ModuleStore) -> Vec<(String, Vec<Color>)> {
    match store.type_data(COLOR_PROPERTY_TYPE_ID).and_then(|e| e.data()).and_then(|d| d.child("ColorSets")) {
        Some(sets) => sets
            .elements()
            .filter_map(|kv| {
                let name = child_string(kv, "Key")?;
                let colors = color_list_from_xml(kv.child("Value")?)?;
                Some((name, colors))
            })
            .collect(),
        None => default_color_sets(),
    }
}

fn default_color_sets() -> Vec<(String, Vec<Color>)> {
    let (r, g, b, w) = (Color::rgb(255, 0, 0), Color::rgb(0, 255, 0), Color::rgb(0, 0, 255), Color::rgb(255, 255, 255));
    vec![("RGBW".to_string(), vec![r, g, b, w]), ("RGB".to_string(), vec![r, g, b])]
}

fn library_items<T>(store: &ModuleStore, type_id: Uuid, decode: impl Fn(&XElement) -> Option<T>) -> Vec<(String, T)> {
    let Some(list) = store.type_data(type_id).and_then(|e| e.data()).and_then(|d| d.child("_library")) else {
        return Vec::new();
    };
    list.elements()
        .filter_map(|kv| {
            let name = child_string(kv, "Key")?;
            let value = decode(kv.child("Value")?)?;
            Some((name, value))
        })
        .collect()
}

/// The names in the LipSync map library, in file order: what LipSync's Phoneme Mapping row offers.
pub fn lip_sync_map_names(store: &ModuleStore) -> Vec<String> {
    let Some(list) = store
        .type_entries()
        .iter()
        .find_map(|e| e.data().filter(|d| d.local == "LipSyncMapStaticData"))
        .and_then(|d| d.child("_library"))
    else {
        return Vec::new();
    };
    list.elements().filter_map(|kv| child_string(kv, "Key")).collect()
}

/// Writes the curve library back. An entry whose curve has not changed keeps its exact XML.
pub fn set_curve_library(store: &mut ModuleStore, library: &CurveLibrary) {
    let data = library_data(store, CURVE_LIBRARY_TYPE_ID, NS_CURVES, "CurveLibraryStaticData");
    let items = library.curves.iter().map(|(name, curve)| {
        (
            name.as_str(),
            curve_from_xml as fn(&XElement) -> Option<Curve>,
            curve,
            curve_to_xml(ARRAYS_NS, "Value", curve),
        )
    });
    let data = rewrite_dictionary(data, NS_CURVES, "_library", CURVE_ITEM, items);
    store_library(
        store,
        CURVE_LIBRARY_TYPE_ID,
        "VixenModules.App.Curves.CurveLibraryStaticData, Curves",
        "Module.App.Curves",
        data,
    );
}

/// Writes the gradient library back. An entry whose gradient has not changed keeps its exact XML.
pub fn set_gradient_library(store: &mut ModuleStore, library: &GradientLibrary) {
    let data = library_data(store, GRADIENT_LIBRARY_TYPE_ID, NS_GRADIENTS, "ColorGradientLibraryStaticData");
    let items = library.gradients.iter().map(|(name, gradient)| {
        (
            name.as_str(),
            gradient_from_xml as fn(&XElement) -> Option<ColorGradient>,
            gradient,
            gradient_to_xml(ARRAYS_NS, "Value", gradient),
        )
    });
    let data = rewrite_dictionary(data, NS_GRADIENTS, "_library", GRADIENT_ITEM, items);
    store_library(
        store,
        GRADIENT_LIBRARY_TYPE_ID,
        "VixenModules.App.ColorGradients.ColorGradientLibraryStaticData, ColorGradients",
        "Module.App.ColorGradients",
        data,
    );
}

/// Writes the colour sets back. A set whose colours have not changed keeps its exact XML.
pub fn set_color_sets(store: &mut ModuleStore, sets: &[(String, Vec<Color>)]) {
    let data = library_data(store, COLOR_PROPERTY_TYPE_ID, NS_COLOR, "ColorStaticData");
    let items = sets.iter().map(|(name, colors)| {
        (
            name.as_str(),
            color_list_from_xml as fn(&XElement) -> Option<Vec<Color>>,
            colors,
            color_list_to_xml(ARRAYS_NS, "Value", colors),
        )
    });
    let data = rewrite_dictionary(data, NS_COLOR, "ColorSets", COLOR_SET_ITEM, items);
    store_library(
        store,
        COLOR_PROPERTY_TYPE_ID,
        "VixenModules.Property.Color.ColorStaticData, Color",
        "Module.Property.Color",
        data,
    );
}

fn library_data(store: &ModuleStore, type_id: Uuid, ns: &str, local: &str) -> XElement {
    if let Some(data) = store.type_data(type_id).and_then(|e| e.data()) {
        return data.clone();
    }
    let mut data = XElement::new(ns, local);
    let id = format_guid(&type_id);
    data.push(XElement::with_text("", "ModuleInstanceId", id.as_str()));
    data.push(XElement::with_text("", "ModuleTypeId", id.as_str()));
    data
}

fn store_library(store: &mut ModuleStore, type_id: Uuid, data_model_type: &str, assembly: &str, data: XElement) {
    match store.type_data_mut(type_id) {
        Some(entry) => entry.set_data(data),
        None => store.set_type_data(ModuleEntry::new(data_model_type, &type_id, &type_id, assembly, data)),
    }
}

/// Replaces a dictionary member's entries, reusing an old entry's XML when it still decodes to the same value.
fn rewrite_dictionary<'a, T: PartialEq + 'a>(
    mut data: XElement,
    ns: &str,
    member: &str,
    default_item: &str,
    items: impl Iterator<Item = (&'a str, fn(&XElement) -> Option<T>, &'a T, XElement)>,
) -> XElement {
    let old: Vec<XElement> = data.child(member).map(|m| m.elements().cloned().collect()).unwrap_or_default();
    let item_name = old.first().map_or(default_item, |e| e.local.as_str()).to_string();
    let mut items = items.peekable();
    // A file with no such member and nothing to put in it stays that way.
    if data.child(member).is_none() && items.peek().is_none() {
        return data;
    }
    let mut list = XElement::new(ns, member);
    for (name, decode, value, encoded) in items {
        let kept = old.iter().find(|kv| {
            child_string(kv, "Key").as_deref() == Some(name)
                && kv.child("Value").and_then(decode).as_ref() == Some(value)
        });
        match kept {
            Some(kv) => {
                list.push(kv.clone());
            }
            None => {
                let mut kv = XElement::new(ARRAYS_NS, &item_name);
                kv.push(XElement::with_text(ARRAYS_NS, "Key", name));
                kv.push(encoded);
                list.push(kv);
            }
        }
    }
    match data.children.iter().position(|n| matches!(n, XNode::Element(e) if e.local == member)) {
        Some(i) => data.children[i] = XNode::Element(list),
        None => crate::dcs::set_member(&mut data, list),
    }
    data
}

/// A `System.Drawing.Rectangle` member, as the libraries' `SelectorWindowBounds` is stored.
pub fn window_bounds(store: &ModuleStore, type_id: Uuid) -> Option<(i32, i32, i32, i32)> {
    let e = store.type_data(type_id)?.data()?.child("SelectorWindowBounds")?;
    let field = |n: &str| e.child_text(n)?.parse::<i32>().ok();
    Some((field("x")?, field("y")?, field("width")?, field("height")?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module_store::{module_store_from_xml, module_store_to_xml};
    use crate::values::{known_color_named, DotNetColor};
    use crate::xml::same_tree;

    fn store_with(data: XElement, type_id: Uuid, name: &str) -> ModuleStore {
        let mut store = ModuleStore::new();
        store.set_type_data(ModuleEntry::new(name, &type_id, &type_id, "A", data));
        store
    }

    fn curve_store() -> ModuleStore {
        let mut data = library_data(&ModuleStore::new(), CURVE_LIBRARY_TYPE_ID, NS_CURVES, "CurveLibraryStaticData");
        let mut list = XElement::new(NS_CURVES, "_library");
        for (name, curve) in [("Ramp", Curve::ramp_up()), ("Flat", Curve::flat100())] {
            let mut kv = XElement::new(ARRAYS_NS, CURVE_ITEM);
            kv.push(XElement::with_text(ARRAYS_NS, "Key", name));
            kv.push(curve_to_xml(ARRAYS_NS, "Value", &curve));
            list.push(kv);
        }
        data.push(list);
        store_with(data, CURVE_LIBRARY_TYPE_ID, "C")
    }

    #[test]
    fn curves_read_in_file_order() {
        let lib = curve_library(&curve_store());
        assert_eq!(lib.curves.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(), ["Ramp", "Flat"]);
        assert_eq!(lib.get("Flat"), Some(&Curve::flat100()));
        assert_eq!(lib.get("Nope"), None);
    }

    #[test]
    fn writing_an_unchanged_library_changes_nothing() {
        let mut store = curve_store();
        let before = module_store_to_xml(&store);
        let lib = curve_library(&store);
        set_curve_library(&mut store, &lib);
        assert!(same_tree(&before, &module_store_to_xml(&store)));
    }

    #[test]
    fn edits_are_written_and_read_back() {
        let mut store = curve_store();
        let mut lib = curve_library(&store);
        lib.curves[1].1 = Curve::flat(25.0);
        lib.curves.push(("New".to_string(), Curve::ramp_down()));
        set_curve_library(&mut store, &lib);
        let root = module_store_to_xml(&store);
        assert_eq!(curve_library(&module_store_from_xml(root).unwrap()), lib);
    }

    #[test]
    fn a_missing_library_is_empty_and_can_be_created() {
        let mut store = ModuleStore::new();
        assert!(curve_library(&store).curves.is_empty());
        assert!(gradient_library(&store).gradients.is_empty());
        let lib = GradientLibrary { gradients: vec![("Red".to_string(), ColorGradient::solid(Color::RED))] };
        set_gradient_library(&mut store, &lib);
        assert_eq!(gradient_library(&store), lib);
        let entry = store.type_data(GRADIENT_LIBRARY_TYPE_ID).unwrap();
        assert_eq!(entry.data().unwrap().local, "ColorGradientLibraryStaticData");
        assert_eq!(
            entry.data_model_type(),
            Some("VixenModules.App.ColorGradients.ColorGradientLibraryStaticData, ColorGradients")
        );
    }

    #[test]
    fn colour_sets_default_like_the_constructor() {
        let sets = color_sets(&ModuleStore::new());
        assert_eq!(sets[0].0, "RGBW");
        assert_eq!(sets[0].1.len(), 4);
        assert_eq!(sets[1].0, "RGB");
        assert_eq!(sets[1].1, vec![Color::rgb(255, 0, 0), Color::rgb(0, 255, 0), Color::rgb(0, 0, 255)]);
    }

    #[test]
    fn colour_sets_keep_known_colours_until_they_change() {
        let mut store = ModuleStore::new();
        set_color_sets(&mut store, &[("Warm".to_string(), vec![Color::rgb(255, 0, 0)])]);
        // Swap in a known colour, as a real file might have.
        let white =
            DotNetColor::from_known(known_color_named("White").unwrap()).to_xml(crate::values::NS_DRAWING, "Color");
        let entry = store.type_data_mut(COLOR_PROPERTY_TYPE_ID).unwrap();
        let sets = entry.data_mut().unwrap().child_mut("ColorSets").unwrap();
        let kv = sets.elements_mut().next().unwrap();
        let value = kv.child_mut("Value").unwrap();
        value.children = vec![XNode::Element(white)];

        let read = color_sets(&store);
        assert_eq!(read[0].1, vec![Color::WHITE]);
        set_color_sets(&mut store, &read);
        let e = store.type_data(COLOR_PROPERTY_TYPE_ID).unwrap().data().unwrap().child("ColorSets").unwrap();
        let color = e.elements().next().unwrap().child("Value").unwrap().child("Color").unwrap();
        assert_eq!(color.child_text("state").as_deref(), Some("1"));

        set_color_sets(&mut store, &[("Warm".to_string(), vec![Color::RED])]);
        let e = store.type_data(COLOR_PROPERTY_TYPE_ID).unwrap().data().unwrap().child("ColorSets").unwrap();
        let color = e.elements().next().unwrap().child("Value").unwrap().child("Color").unwrap();
        assert_eq!(color.child_text("state").as_deref(), Some("2"));
    }
}
