//! The Color Library's palette, colour collections, and the library files: `Form_ColorLibrary`, `ColorCollectionLibrary_Form`, and the export and import of `Form_CurveLibrary` and
//! `Form_GradientLibrary`. F341 to F374. Curves and gradients themselves live in the module store, through `effect_editor::libraries`.

use std::path::{Path, PathBuf};
use vixen_core::{Color, ColorGradient, Curve};
use vixen_io::file::write_atomic;
use vixen_io::values::{color_list_from_xml, color_list_to_xml, curve_from_xml, curve_to_xml, gradient_from_xml, gradient_to_xml};
use vixen_io::IoError;
use vixen_xml::XElement;

const NS_DRAWING: &str = "http://schemas.datacontract.org/2004/07/System.Drawing";
const NS_TIMED: &str = "http://schemas.datacontract.org/2004/07/VixenModules.Sequence.Timed";
const NS_ARRAYS: &str = "http://schemas.microsoft.com/2003/10/Serialization/Arrays";
const CURVE_ITEM: &str = "KeyValueOfstringCurvesLZZwEMY";
const GRADIENT_ITEM: &str = "KeyValueOfstringColorGradientGbtyavQ7";

fn io_error(path: &Path, source: std::io::Error) -> IoError {
    IoError::Io { path: path.to_path_buf(), source }
}

fn format_error(path: &Path, message: &str) -> IoError {
    IoError::Format { context: path.display().to_string(), message: message.to_string() }
}

fn write_document(path: &Path, root: &XElement) -> Result<(), IoError> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| io_error(dir, e))?;
    }
    write_atomic(path, root.to_document().as_bytes())
}

// ------------------------------------------------------------------------------------------------------------------
// Colours
// ------------------------------------------------------------------------------------------------------------------

pub fn palette_path(dir: &Path) -> PathBuf {
    dir.join("ColorPalette.xml")
}

/// `Load_ColorPaletteFile`: a missing or bad file is an empty palette.
pub fn load_palette(dir: &Path) -> Vec<Color> {
    XElement::from_file(&palette_path(dir)).ok().and_then(|root| color_list_from_xml(&root)).unwrap_or_default()
}

/// `Save_ColorPaletteFile`: on every change. `List<Color>`.
pub fn save_palette(dir: &Path, colors: &[Color]) -> Result<(), IoError> {
    write_document(&palette_path(dir), &color_list_to_xml(NS_DRAWING, "ArrayOfColor", colors))
}

/// `.vfc`: the same document. F349.
pub fn read_vfc(path: &Path) -> Result<Vec<Color>, IoError> {
    let root = XElement::from_file(path).map_err(|e| format_error(path, &e.to_string()))?;
    if root.local != "ArrayOfColor" {
        return Err(format_error(path, "not a list of colours"));
    }
    color_list_from_xml(&root).ok_or_else(|| format_error(path, "not a list of colours"))
}

pub fn write_vfc(path: &Path, colors: &[Color]) -> Result<(), IoError> {
    write_document(path, &color_list_to_xml(NS_DRAWING, "ArrayOfColor", colors))
}

/// `RemoveColors`: the first match per value, since duplicates are allowed.
pub fn remove_colors(colors: &mut Vec<Color>, selected: &[Color]) {
    for color in selected {
        if let Some(at) = colors.iter().position(|c| c == color) {
            colors.remove(at);
        }
    }
}

// ------------------------------------------------------------------------------------------------------------------
// Colour collections
// ------------------------------------------------------------------------------------------------------------------

/// `ColorCollection`. Name equality ignores case.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColorCollection {
    pub name: String,
    pub description: String,
    pub colors: Vec<Color>,
}

pub fn collections_path(dir: &Path) -> PathBuf {
    dir.join("ColorCollections.xml")
}

fn collections_to_xml(list: &[ColorCollection]) -> XElement {
    let mut root = XElement::new(NS_TIMED, "ArrayOfColorCollection");
    for c in list {
        let mut item = XElement::new(NS_TIMED, "ColorCollection");
        item.push(color_list_to_xml(NS_TIMED, "Color", &c.colors));
        item.push(XElement::with_text(NS_TIMED, "Description", c.description.clone()));
        item.push(XElement::with_text(NS_TIMED, "Name", c.name.clone()));
        root.push(item);
    }
    root
}

fn collections_from_xml(root: &XElement) -> Option<Vec<ColorCollection>> {
    if root.local != "ArrayOfColorCollection" {
        return None;
    }
    let mut list = Vec::new();
    for item in root.elements() {
        list.push(ColorCollection {
            name: item.child_text("Name").unwrap_or_default(),
            description: item.child_text("Description").unwrap_or_default(),
            colors: item.child("Color").and_then(color_list_from_xml).unwrap_or_default(),
        });
    }
    Some(list)
}

/// `LoadColorCollections`. A corrupt file loads as empty, where upstream throws at load (GUARD).
pub fn load_color_collections(dir: &Path) -> Vec<ColorCollection> {
    XElement::from_file(&collections_path(dir)).ok().and_then(|root| collections_from_xml(&root)).unwrap_or_default()
}

pub fn save_color_collections(dir: &Path, list: &[ColorCollection]) -> Result<(), IoError> {
    write_document(&collections_path(dir), &collections_to_xml(list))
}

/// `.vcc`. F372. Upstream has no error handling here (GUARD: an error comes back to show).
pub fn read_vcc(path: &Path) -> Result<Vec<ColorCollection>, IoError> {
    let root = XElement::from_file(path).map_err(|e| format_error(path, &e.to_string()))?;
    collections_from_xml(&root).ok_or_else(|| format_error(path, "not a list of colour collections"))
}

pub fn write_vcc(path: &Path, list: &[ColorCollection]) -> Result<(), IoError> {
    write_document(path, &collections_to_xml(list))
}

// ------------------------------------------------------------------------------------------------------------------
// Curve and gradient library files
// ------------------------------------------------------------------------------------------------------------------

/// `.vcl`: `Dictionary<string, Curve>`. F359, F360.
pub fn read_vcl(path: &Path) -> Result<Vec<(String, Curve)>, IoError> {
    read_dictionary(path, curve_from_xml)
}

pub fn write_vcl(path: &Path, items: &[(String, Curve)]) -> Result<(), IoError> {
    write_dictionary(path, CURVE_ITEM, items, |c| curve_to_xml(NS_ARRAYS, "Value", c))
}

/// `.vgl`: `Dictionary<string, ColorGradient>`. F363.
pub fn read_vgl(path: &Path) -> Result<Vec<(String, ColorGradient)>, IoError> {
    read_dictionary(path, gradient_from_xml)
}

pub fn write_vgl(path: &Path, items: &[(String, ColorGradient)]) -> Result<(), IoError> {
    write_dictionary(path, GRADIENT_ITEM, items, |g| gradient_to_xml(NS_ARRAYS, "Value", g))
}

fn read_dictionary<T>(path: &Path, decode: impl Fn(&XElement) -> Option<T>) -> Result<Vec<(String, T)>, IoError> {
    let root = XElement::from_file(path).map_err(|e| format_error(path, &e.to_string()))?;
    if !root.local.starts_with("ArrayOfKeyValueOfstring") {
        return Err(format_error(path, "not a library file"));
    }
    let mut items = Vec::new();
    for entry in root.elements() {
        let name = entry.child_text("Key").ok_or_else(|| format_error(path, "an entry has no name"))?;
        let value = entry.child("Value").and_then(&decode).ok_or_else(|| format_error(path, "an entry isn't readable"))?;
        items.push((name, value));
    }
    Ok(items)
}

fn write_dictionary<T>(path: &Path, item: &str, items: &[(String, T)], encode: impl Fn(&T) -> XElement) -> Result<(), IoError> {
    let mut root = XElement::new(NS_ARRAYS, &format!("ArrayOf{item}"));
    for (name, value) in items {
        let mut entry = XElement::new(NS_ARRAYS, item);
        entry.push(XElement::with_text(NS_ARRAYS, "Key", name.clone()));
        entry.push(encode(value));
        root.push(entry);
    }
    write_document(path, &root)
}

/// A new collection's outcome. F366.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NewCollection {
    /// Blank: "Please enter a name." and ask again.
    Blank,
    /// That name's taken, ignoring case: ask whether to overwrite.
    Duplicate(String),
    Added,
}

/// `btnNewCollection_Click`'s test, and the addition when there's no clash.
pub fn add_collection(list: &mut Vec<ColorCollection>, name: &str, overwrite: bool) -> NewCollection {
    if name.trim().is_empty() {
        return NewCollection::Blank;
    }
    let existing = list.iter().position(|c| c.name.eq_ignore_ascii_case(name));
    if let Some(at) = existing {
        if !overwrite {
            return NewCollection::Duplicate(name.to_string());
        }
        list[at] = ColorCollection { name: name.to_string(), description: String::new(), colors: Vec::new() };
    } else {
        list.push(ColorCollection { name: name.to_string(), description: String::new(), colors: Vec::new() });
    }
    NewCollection::Added
}

/// `ColorCollectionLibrary_Form`'s working copy: edits stay here until OK. F364 to F374.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollectionEditor {
    pub list: Vec<ColorCollection>,
    pub selected: Option<usize>,
    /// `_colorValue`: where the next picker starts.
    pub last_color: Color,
    pub dirty: bool,
}

impl CollectionEditor {
    pub fn new(list: Vec<ColorCollection>) -> CollectionEditor {
        CollectionEditor { list, selected: None, last_color: Color::rgb(255, 255, 255), dirty: false }
    }

    pub fn select(&mut self, name: &str) {
        self.selected = self.list.iter().position(|c| c.name == name);
    }

    pub fn new_collection(&mut self, name: &str, overwrite: bool) -> NewCollection {
        let outcome = add_collection(&mut self.list, name, overwrite);
        if outcome == NewCollection::Added {
            self.dirty = true;
            self.selected = self.list.iter().position(|c| c.name.eq_ignore_ascii_case(name));
        }
        outcome
    }

    pub fn delete_selected(&mut self) {
        if let Some(at) = self.selected.take() {
            self.list.remove(at);
            self.dirty = true;
        }
    }

    pub fn add_color(&mut self, color: Color) {
        self.last_color = color;
        if let Some(c) = self.selected.and_then(|at| self.list.get_mut(at)) {
            c.colors.push(color);
            self.dirty = true;
        }
    }

    pub fn remove_colors(&mut self, colors: &[Color]) {
        if let Some(c) = self.selected.and_then(|at| self.list.get_mut(at)) {
            remove_colors(&mut c.colors, colors);
            self.dirty = true;
        }
    }

    /// F371: the first selected, to the target's place.
    pub fn move_color(&mut self, from: usize, to: usize) {
        if let Some(c) = self.selected.and_then(|at| self.list.get_mut(at)) {
            if from < c.colors.len() && to < c.colors.len() && from != to {
                let color = c.colors.remove(from);
                c.colors.insert(to, color);
                self.dirty = true;
            }
        }
    }

    pub fn set_description(&mut self, text: &str) {
        if let Some(c) = self.selected.and_then(|at| self.list.get_mut(at)) {
            if c.description != text {
                c.description = text.to_string();
                self.dirty = true;
            }
        }
    }

    /// The names that need F372's question.
    pub fn import_clashes(&self, incoming: &[ColorCollection]) -> Vec<String> {
        incoming.iter().filter(|c| self.list.iter().any(|e| e.name.eq_ignore_ascii_case(&c.name))).map(|c| c.name.clone()).collect()
    }

    /// F372: `overwrite` are the clashing names answered Yes; the rest stay as they are. Returns how many came in.
    pub fn import(&mut self, incoming: Vec<ColorCollection>, overwrite: &[String]) -> usize {
        let mut count = 0;
        for collection in incoming {
            match self.list.iter().position(|e| e.name.eq_ignore_ascii_case(&collection.name)) {
                Some(at) => {
                    if overwrite.contains(&collection.name) {
                        self.list[at] = collection;
                        count += 1;
                    }
                }
                None => {
                    self.list.push(collection);
                    count += 1;
                }
            }
        }
        if count > 0 {
            self.dirty = true;
        }
        count
    }
}

/// F347's Move branch, `listViewColors_DragDrop`: the reorder the curve and gradient lists use, by position since a colour can repeat.
pub fn moved_by_position<T: Clone>(list: &[T], selected: &[usize], target: Option<usize>) -> Vec<T> {
    let order: Vec<String> = (0..list.len()).map(|i| i.to_string()).collect();
    let selected: Vec<String> = selected.iter().filter(|i| **i < list.len()).map(usize::to_string).collect();
    let target = target.filter(|t| *t < list.len()).map(|t| t.to_string());
    crate::effect_editor::libraries::moved_names(&order, &selected, target.as_deref())
        .iter()
        .filter_map(|name| name.parse::<usize>().ok().map(|i| list[i].clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vixen-libs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn palette_round_trips_and_a_bad_file_is_empty() {
        let dir = temp("palette");
        assert!(load_palette(&dir).is_empty());
        let colors = vec![Color::rgb(255, 0, 0), Color::rgb(0, 0, 255), Color::rgb(255, 0, 0)];
        save_palette(&dir, &colors).unwrap();
        assert_eq!(load_palette(&dir), colors);
        std::fs::write(palette_path(&dir), "junk").unwrap();
        assert!(load_palette(&dir).is_empty());
    }

    #[test]
    fn reads_a_real_shape_palette() {
        let dir = temp("real");
        let text = r#"<ArrayOfColor xmlns:i="http://www.w3.org/2001/XMLSchema-instance" xmlns="http://schemas.datacontract.org/2004/07/System.Drawing"><Color><knownColor>0</knownColor><name i:nil="true"/><state>2</state><value>4294901760</value></Color></ArrayOfColor>"#;
        std::fs::write(palette_path(&dir), text).unwrap();
        assert_eq!(load_palette(&dir), vec![Color::rgb(255, 0, 0)]);
    }

    #[test]
    fn vfc_refuses_other_data() {
        let dir = temp("vfc");
        let path = dir.join("a.vfc");
        write_vfc(&path, &[Color::rgb(1, 2, 3)]).unwrap();
        assert_eq!(read_vfc(&path).unwrap(), vec![Color::rgb(1, 2, 3)]);
        std::fs::write(&path, "<Other/>").unwrap();
        assert!(read_vfc(&path).is_err());
        std::fs::write(&path, "nonsense").unwrap();
        assert!(read_vfc(&path).is_err());
    }

    #[test]
    fn remove_takes_the_first_match_per_value() {
        let (r, b) = (Color::rgb(255, 0, 0), Color::rgb(0, 0, 255));
        let mut colors = vec![r, b, r];
        remove_colors(&mut colors, &[r]);
        assert_eq!(colors, vec![b, r]);
    }

    #[test]
    fn collections_round_trip_and_corrupt_loads_empty() {
        let dir = temp("collections");
        let list = vec![ColorCollection { name: "Xmas".into(), description: "Red and green".into(), colors: vec![Color::rgb(255, 0, 0), Color::rgb(0, 255, 0)] }];
        save_color_collections(&dir, &list).unwrap();
        assert_eq!(load_color_collections(&dir), list);
        std::fs::write(collections_path(&dir), "junk").unwrap();
        assert!(load_color_collections(&dir).is_empty());
        let path = dir.join("a.vcc");
        write_vcc(&path, &list).unwrap();
        assert_eq!(read_vcc(&path).unwrap(), list);
    }

    #[test]
    fn new_collection_rules() {
        let mut list = Vec::new();
        assert_eq!(add_collection(&mut list, " ", false), NewCollection::Blank);
        assert_eq!(add_collection(&mut list, "Xmas", false), NewCollection::Added);
        list[0].colors.push(Color::rgb(1, 1, 1));
        assert_eq!(add_collection(&mut list, "XMAS", false), NewCollection::Duplicate("XMAS".to_string()));
        assert_eq!(add_collection(&mut list, "XMAS", true), NewCollection::Added);
        assert_eq!(list.len(), 1);
        assert!(list[0].colors.is_empty());
    }

    #[test]
    fn library_files_round_trip() {
        let dir = temp("vcl");
        let curves = vec![("Ramp".to_string(), Curve::ramp_up()), ("Flat".to_string(), Curve::flat100())];
        let path = dir.join("a.vcl");
        write_vcl(&path, &curves).unwrap();
        let back = read_vcl(&path).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].0, "Ramp");
        assert_eq!(back[1].1.points, curves[1].1.points);
        std::fs::write(&path, "<ArrayOfColor/>").unwrap();
        assert!(read_vcl(&path).is_err());

        let gradients = vec![("Red".to_string(), ColorGradient::solid(Color::rgb(255, 0, 0)))];
        let path = dir.join("a.vgl");
        write_vgl(&path, &gradients).unwrap();
        let back = read_vgl(&path).unwrap();
        assert_eq!(back[0].0, "Red");
        assert_eq!(back[0].1.get_color_at(0.5), Color::rgb(255, 0, 0));
    }

    #[test]
    fn the_editor_works_on_its_own_copy() {
        let start = vec![ColorCollection { name: "Xmas".into(), description: String::new(), colors: vec![Color::rgb(255, 0, 0)] }];
        let mut e = CollectionEditor::new(start.clone());
        assert!(!e.dirty);
        e.select("Xmas");
        e.add_color(Color::rgb(0, 255, 0));
        e.add_color(Color::rgb(0, 0, 255));
        assert!(e.dirty);
        assert_eq!(e.last_color, Color::rgb(0, 0, 255));
        e.move_color(2, 0);
        assert_eq!(e.list[0].colors[0], Color::rgb(0, 0, 255));
        e.remove_colors(&[Color::rgb(255, 0, 0)]);
        assert_eq!(e.list[0].colors.len(), 2);
        e.set_description("two");
        assert_eq!(e.list[0].description, "two");
        assert_eq!(start[0].colors.len(), 1, "the original isn't touched");

        assert_eq!(e.new_collection("xmas", false), NewCollection::Duplicate("xmas".to_string()));
        assert_eq!(e.new_collection("New", false), NewCollection::Added);
        assert_eq!(e.selected, Some(1));
        e.delete_selected();
        assert_eq!(e.list.len(), 1);
        assert_eq!(e.selected, None);

        let incoming = vec![
            ColorCollection { name: "XMAS".into(), description: "in".into(), colors: vec![] },
            ColorCollection { name: "Other".into(), description: String::new(), colors: vec![] },
        ];
        assert_eq!(e.import_clashes(&incoming), ["XMAS"]);
        assert_eq!(e.clone().import(incoming.clone(), &[]), 1);
        assert_eq!(e.import(incoming, &["XMAS".to_string()]), 2);
        assert_eq!(e.list[0].description, "in");
    }

    #[test]
    fn colours_move_by_position_like_the_named_lists() {
        // Rightward: each lands at the target's old index, so the group ends after it, in order. Leftward: before it, the reversal keeping the order.
        assert_eq!(moved_by_position(&[1, 2, 3, 4, 5], &[0, 1], Some(3)), vec![3, 4, 1, 2, 5]);
        assert_eq!(moved_by_position(&[1, 2, 3, 4, 5], &[3, 4], Some(1)), vec![1, 4, 5, 2, 3]);
        // No item under the pointer: to the end. Duplicates stay apart.
        assert_eq!(moved_by_position(&[7, 7, 9], &[0], None), vec![7, 9, 7]);
        assert_eq!(moved_by_position(&[1, 2, 3], &[9], Some(0)), vec![1, 2, 3]);
    }
}
