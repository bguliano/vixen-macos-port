//! Against the real show data on this machine, read only. Skips if it is not there. Point VIXEN_TEST_DATA
//! elsewhere to use another copy.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;
use vixen_core::element::{property_ids, PropertyData, PropertyInstance};
use vixen_core::{Color, ElementColorType, ElementTree, NodeId};
use vixen_preview::data::PreviewData;
use vixen_preview::scene::Scene;
use vixen_preview::session::DesignSession;
use vixen_preview::shape::{Modifiers, ShapeKind};
use vixen_xml::XElement;

fn data_root() -> Option<PathBuf> {
    let root = std::env::var("VIXEN_TEST_DATA").unwrap_or_else(|_| "/Users/developer/Documents/Vixen 3".to_string());
    let root = PathBuf::from(root);
    if root.join("SystemData/ModuleStore.xml").exists() {
        Some(root)
    } else {
        eprintln!("skipping: no test data at {}", root.display());
        None
    }
}

fn preview_modules(store: &XElement) -> Vec<&XElement> {
    fn walk<'a>(e: &'a XElement, out: &mut Vec<&'a XElement>) {
        if e.local == "Module" {
            if e.attr("dataModelType").is_some_and(|t| t.contains("VixenPreviewData")) {
                if let Some(d) = e.elements().next() {
                    out.push(d);
                }
            }
            return;
        }
        for c in e.elements() {
            walk(c, out);
        }
    }
    let mut out = Vec::new();
    walk(store, &mut out);
    out
}

/// The `NodeId` of each `PreviewPixel` under an element, in document order, directly from the XML.
fn xml_pixels(e: &XElement, out: &mut Vec<Option<Uuid>>) {
    if e.local == "PreviewPixel" {
        out.push(e.child("NodeId").map(|n| Uuid::parse_str(n.text().trim()).unwrap()));
        return;
    }
    for c in e.elements() {
        xml_pixels(c, out);
    }
}

/// Every place two trees of the same shape differ. `first_difference` stops at the first.
fn all_differences(a: &XElement, b: &XElement, path: String, out: &mut Vec<String>) {
    let path = format!("{path}/{}", a.local);
    if let Some(d) = shallow_difference(a, b) {
        out.push(format!("{path}: {d}"));
        return;
    }
    let (ae, be): (Vec<_>, Vec<_>) = (a.elements().collect(), b.elements().collect());
    if ae.is_empty() && be.is_empty() {
        if a.text().trim() != b.text().trim() {
            out.push(format!("{path}: {:?} vs {:?}", a.text().trim(), b.text().trim()));
        }
        return;
    }
    if ae.len() != be.len() {
        out.push(format!("{path}: {} children vs {}", ae.len(), be.len()));
        return;
    }
    for (x, y) in ae.iter().zip(be.iter()) {
        all_differences(x, y, path.clone(), out);
    }
}

fn shallow_difference(a: &XElement, b: &XElement) -> Option<String> {
    (a.local != b.local || a.ns != b.ns || a.nil != b.nil || a.xsi_type != b.xsi_type || a.attrs != b.attrs).then(|| format!("{}/{:?} vs {}/{:?}", a.local, a.xsi_type, b.local, b.xsi_type))
}

/// Load, save, compare, and everything else that must hold for any preview. `allowed` says which differences a
/// file is allowed to have from what this Vixen version would write.
fn check_previews(file: &str, expected_count: usize, allowed: impl Fn(&str) -> bool) -> usize {
    let Some(root) = data_root() else { return 0 };
    let path = root.join(file);
    if !path.exists() {
        eprintln!("skipping: no {}", path.display());
        return 0;
    }
    let store = XElement::from_file(&path).unwrap();
    let modules = preview_modules(&store);
    assert_eq!(modules.len(), expected_count, "{file}");
    let mut lights = 0;
    for m in modules {
        let data = PreviewData::from_xml(m).unwrap();

        // Saves back unchanged, apart from anything allowed.
        let out = data.to_xml();
        let mut differences = Vec::new();
        all_differences(m, &out, String::new(), &mut differences);
        let unexpected: Vec<&String> = differences.iter().filter(|d| !allowed(d)).collect();
        assert!(unexpected.is_empty(), "{file}: {unexpected:?}");
        if differences.is_empty() {
            assert_eq!(m.first_difference(&out, String::new()), None, "{file}");
        } else {
            eprintln!("{file}: {} differences, all allowed, such as {}", differences.len(), differences[0]);
        }

        // Saving what was saved changes nothing further.
        let again = PreviewData::from_xml(&out).unwrap();
        assert_eq!(out.first_difference(&again.to_xml(), String::new()), None, "{file} saved twice");

        // As text too, the way it would really reach a file.
        let reparsed = XElement::parse(&out.to_document()).unwrap();
        assert_eq!(out.first_difference(&reparsed, String::new()), None, "{file} through text");

        // Every light has the node the file says, in order, and the counts agree with a direct XML count.
        let mut direct = Vec::new();
        xml_pixels(m, &mut direct);
        assert_eq!(data.pixel_count(), direct.len(), "{file}");
        let scene = Scene::build(&data, None);
        assert_eq!(scene.light_count(), direct.len(), "{file}");
        for (i, want) in direct.iter().enumerate() {
            assert_eq!(scene.node_of_light(i), *want, "{file} light {i}");
        }
        lights += direct.len();

        // A session on it, with some clicks, still saves what it would have if nothing is edited.
        let mut session = DesignSession::new(data, None);
        for (x, y) in [(50, 50), (300, 200), (10, 400)] {
            session.mouse_down(x, y, Modifiers::NONE);
            session.mouse_up(x, y, Modifiers::NONE);
        }
        assert_eq!(out.first_difference(&session.data().to_xml(), String::new()), None, "{file} after selecting");
    }
    lights
}

#[test]
fn current_previews_round_trip_with_links_and_counts() {
    // Nothing at all may differ, and first_difference, the check the port's save relies on, finds nothing.
    let lights = check_previews("SystemData/ModuleStore.xml", 2, |_| false);
    eprintln!("ModuleStore.xml: {lights} lights");
}

#[test]
fn the_older_store_with_groups_and_a_rectangle_round_trips() {
    // An older Vixen wrote this store. It laid out a rectangle's top and bottom strings 2 in from the corners
    // where today's source says 1. Those points are derived data, and Vixen itself rewrites them on load.
    let lights = check_previews("SystemData/ModuleStore.xml.4", 5, |d| d.contains("_strings/PreviewBaseShape/_points/PreviewPoint/X"));
    eprintln!("ModuleStore.xml.4: {lights} lights");
}

/// The tree from SystemConfig.xml and each node's colour from ModuleStore.xml, enough for real discrete slots.
fn real_tree(root: &std::path::Path) -> (ElementTree, usize) {
    let config = XElement::from_file(&root.join("SystemData/SystemConfig.xml")).unwrap();
    let store = XElement::from_file(&root.join("SystemData/ModuleStore.xml")).unwrap();
    let mut colors: HashMap<String, (String, Option<String>, Color)> = HashMap::new();
    fn each_module<'a>(e: &'a XElement, f: &mut impl FnMut(&'a XElement)) {
        if e.local == "Module" {
            f(e);
            return;
        }
        for c in e.elements() {
            each_module(c, f);
        }
    }
    let mut sets: HashMap<String, Vec<Color>> = HashMap::new();
    each_module(&store, &mut |m| {
        let kind = m.attr("dataModelType").unwrap_or("");
        let Some(d) = m.elements().next() else { return };
        if kind.contains("Property.Color.ColorData") {
            let instance = m.attr("moduleInstance").unwrap_or("").to_string();
            let t = d.child_text("ElementColorType").unwrap_or_default();
            let set = d.child_text("ColorSetName").filter(|s| !s.is_empty());
            colors.insert(instance, (t, set, Color::WHITE));
        } else if kind.contains("ColorStaticData") {
            if let Some(list) = d.child("ColorSets") {
                for kv in list.elements() {
                    let (Some(key), Some(value)) = (kv.child("Key"), kv.child("Value")) else { continue };
                    // Only how many colours each set has matters here.
                    let n = value.child("_colors").map_or(0, |c| c.elements().count());
                    sets.insert(key.text().trim().to_string(), vec![Color::WHITE; n.max(1)].iter().enumerate().map(|(i, _)| Color::rgb(i as u8, 0, 0)).collect());
                }
            }
        }
    });
    let mut tree = ElementTree::new();
    tree.color_sets = sets;
    let nodes = config.child("Nodes").unwrap();
    fn add(tree: &mut ElementTree, e: &XElement, colors: &HashMap<String, (String, Option<String>, Color)>, parent: Option<NodeId>, discrete: &mut usize) {
        let id = Uuid::parse_str(e.attr("id").unwrap()).unwrap();
        let existing = tree.find(&id);
        let node = existing.unwrap_or_else(|| {
            let element = e.attr("channelId").and_then(|c| Uuid::parse_str(c).ok());
            tree.add_node(id, e.attr("name").unwrap_or(""), element)
        });
        if let Some(p) = parent {
            tree.add_child(p, node);
        }
        if existing.is_some() {
            return;
        }
        if let Some(props) = e.child("Properties") {
            for p in props.elements() {
                let Some((t, set, _)) = p.attr("instanceId").and_then(|i| colors.get(i)) else { continue };
                let color_type = match t.as_str() {
                    "SingleColor" => ElementColorType::SingleColor,
                    "MultipleDiscreteColors" => ElementColorType::MultipleDiscreteColors,
                    _ => ElementColorType::FullColor,
                };
                if color_type != ElementColorType::FullColor {
                    *discrete += 1;
                }
                tree.node_mut(node).properties.push(PropertyInstance {
                    type_id: property_ids::COLOR,
                    instance_id: Uuid::new_v4(),
                    data: PropertyData::Color { color_type, single_color: Color::RED, color_set_name: set.clone() },
                });
            }
        }
        for c in e.children_named("Node") {
            add(tree, c, colors, Some(node), discrete);
        }
    }
    let mut discrete = 0;
    for n in nodes.children_named("Node") {
        add(&mut tree, n, &colors, None, &mut discrete);
    }
    (tree, discrete)
}

#[test]
fn links_resolve_in_the_real_element_tree_with_discrete_slots() {
    let Some(root) = data_root() else { return };
    let (tree, discrete) = real_tree(&root);
    eprintln!("real tree: {} nodes, {} with discrete colours", tree.len(), discrete);
    assert!(tree.len() > 100);
    let store = XElement::from_file(&root.join("SystemData/ModuleStore.xml")).unwrap();
    let tree = Arc::new(tree);
    for m in preview_modules(&store) {
        let data = PreviewData::from_xml(m).unwrap();
        let scene = Scene::build(&data, Some(&tree));
        let mut direct = Vec::new();
        xml_pixels(m, &mut direct);

        // Walk the scene pixel by pixel: slot zero starts each pixel, and it shows the XML's node.
        let mut i = 0;
        for want in &direct {
            assert_eq!(scene.links[i].color_slot, 0);
            let count = scene.links[i].color_count as usize;
            let expected = want.and_then(|id| tree.find(&id)).map_or(1, |n| if tree.is_discrete(n) { tree.valid_colors(n, false).len().max(1) } else { 1 });
            assert_eq!(count, expected);
            for slot in 0..count {
                assert_eq!(scene.node_of_light(i + slot), *want);
                assert_eq!(scene.links[i + slot].color_slot, slot as u16);
            }
            i += count;
        }
        assert_eq!(i, scene.light_count());

        let (linked, resolved) = direct.iter().flatten().fold((0, 0), |(l, r), id| (l + 1, r + tree.find(id).is_some() as i32));
        eprintln!("preview: {} pixels, {} lights, {} linked, {} of those in the tree", direct.len(), scene.light_count(), linked, resolved);
        assert_eq!(linked, resolved, "every link in the real data goes to a node that exists");

        // Opening the setup window on it must not remove or move anything that real data has.
        let mut session = DesignSession::new(data, Some(Arc::clone(&tree)));
        let (_, report) = session.setup_open();
        assert_eq!(report.moved.len(), 0, "{report:?}");
        eprintln!("setup_open: removed {} shapes with no size", report.removed.len());
    }
}

#[test]
fn every_shape_in_the_real_data_has_geometry_where_the_file_implies() {
    let Some(root) = data_root() else { return };
    let store = XElement::from_file(&root.join("SystemData/ModuleStore.xml")).unwrap();
    for m in preview_modules(&store) {
        let data = PreviewData::from_xml(m).unwrap();
        let mut kinds: HashMap<&str, usize> = HashMap::new();
        for item in &data.display_items {
            *kinds.entry(item.shape.type_name()).or_insert(0) += 1;
            assert!(!matches!(item.shape.kind(), ShapeKind::Unknown(_)), "the real data has only known shapes");
            let (l, t, r, b) = (item.shape.left(), item.shape.top(), item.shape.right(), item.shape.bottom());
            // A poly line with one point never lays out: its lights stay at the origin, as in the C#.
            if r > l || b > t {
                let mut inside = true;
                item.shape.for_each_pixel(&mut |p| {
                    // Rounding and truncation can reach a little outside the bounds.
                    inside &= p.x >= l - 2 && p.x <= r + 2 && p.y >= t - 2 && p.y <= b + 2;
                });
                assert!(inside, "{} lights outside {l},{t},{r},{b}", item.shape.type_name());
            }
        }
        eprintln!("{kinds:?}");
    }
}
