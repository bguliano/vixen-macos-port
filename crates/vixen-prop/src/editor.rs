//! One Custom Prop Editor window without the window: the document, the three selections and how they follow each
//! other, the element panel's commands, and undo. The canvas side is in `canvas.rs`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use uuid::Uuid;
use vixen_core::Color;

use crate::canvas::{Adorner, Drag};
use crate::clip::PropClip;
use crate::file::{load_prop, save_prop, PropSourceFormat, SaveOutcome};
use crate::model::*;

/// A tree row: the element ids from the root down to the row. One element shows once under each of its parents.
pub type RowPath = Vec<Uuid>;

pub fn path_text(path: &[Uuid]) -> String {
    path.iter().map(|id| id.simple().to_string()).collect::<Vec<_>>().join("/")
}

pub fn parse_path(text: &str) -> Option<RowPath> {
    text.split('/').map(|part| Uuid::parse_str(part).ok()).collect()
}

const UNDO_LIMIT: usize = 100;
const TREE_ROW_LIMIT: usize = 200_000;

#[derive(Clone, Debug, PartialEq)]
pub struct PropEditorSettings {
    pub light_color: Color,
    pub selected_light_color: Color,
    pub state_preview_base_color: Color,
    pub default_light_size: i32,
    pub user_name: String,
}

impl Default for PropEditorSettings {
    fn default() -> Self {
        PropEditorSettings {
            light_color: Color::WHITE,
            selected_light_color: Color::rgb(255, 105, 180),
            state_preview_base_color: Color::rgb(25, 25, 25),
            default_light_size: DEFAULT_LIGHT_SIZE,
            user_name: String::new(),
        }
    }
}

impl PropEditorSettings {
    /// `Configuration.DefaultLightSize`: anything below 1 becomes 3.
    pub fn light_size(&self) -> i32 {
        if self.default_light_size < 1 {
            DEFAULT_LIGHT_SIZE
        } else {
            self.default_light_size
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CanvasMode {
    Draw,
    Select,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PanelTab {
    Layout,
    Order,
    State,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TreeRow {
    pub path: RowPath,
    pub id: Uuid,
    pub name: String,
    pub depth: usize,
    pub is_root: bool,
    pub is_group: bool,
    pub child_count: usize,
    pub light_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OrderRow {
    pub id: Uuid,
    pub order: i32,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ElementInfo {
    pub id: Uuid,
    pub name: String,
    pub face_component: FaceComponent,
    pub face_color: String,
    pub model_type: ElementModelType,
    pub children: usize,
    pub lights: usize,
    pub light_size: String,
    pub error: Option<String>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PropField {
    Name,
    Type,
    CreatedBy,
    PhysicalHeight,
    PhysicalWidth,
    PhysicalDepth,
    Material,
    NodeCount,
    BulbType,
    ColorMode,
    VendorName,
    VendorWebsite,
    VendorContact,
    VendorEmail,
    VendorPhone,
    Notes,
}

#[derive(Clone)]
struct Snapshot {
    name: String,
    prop: Prop,
    tree_selection: Vec<RowPath>,
    order_selection: Vec<Uuid>,
    light_selection: Vec<Uuid>,
    revision: u64,
}

pub struct PropEditor {
    pub(crate) prop: Prop,
    path: Option<PathBuf>,
    legacy_source: Option<PathBuf>,
    pub(crate) settings: PropEditorSettings,
    pub(crate) mode: CanvasMode,
    pub(crate) tab: PanelTab,
    pub(crate) tree_selection: Vec<RowPath>,
    pub(crate) order_selection: Vec<Uuid>,
    pub(crate) light_selection: Vec<Uuid>,
    expand_requests: Vec<RowPath>,
    pub(crate) adorner: Option<Adorner>,
    pub(crate) drag: Option<Drag>,
    pub(crate) pointer: Option<(f64, f64)>,
    revision: u64,
    saved_revision: u64,
    next_revision: u64,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    status: String,
    pub(crate) selected_definition: Option<Uuid>,
    pub(crate) selected_state_items: Vec<Uuid>,
    pub(crate) state_drag: Option<((f64, f64), (f64, f64), bool)>,
}

/// `NewPropCommand`: a name without a `{n}` token gets " {1}", which the preview replaces with the prop's number.
pub fn new_prop_name(typed: &str) -> String {
    if has_token(typed) {
        typed.to_string()
    } else {
        format!("{typed} {{1}}")
    }
}

fn token_end(bytes: &[u8], open: usize) -> Option<usize> {
    let mut at = open + 1;
    while at < bytes.len() && bytes[at].is_ascii_digit() {
        at += 1;
    }
    (at > open + 1 && at < bytes.len() && bytes[at] == b'}').then_some(at)
}

fn has_token(text: &str) -> bool {
    let bytes = text.as_bytes();
    (0..bytes.len()).any(|i| bytes[i] == b'{' && token_end(bytes, i).is_some())
}

/// `CleanseNameString`: drops every `{digits}` token, trims, and collapses runs of spaces.
pub fn cleanse_name(name: &str) -> String {
    let bytes = name.as_bytes();
    let mut kept = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            if let Some(end) = token_end(bytes, i) {
                i = end + 1;
                continue;
            }
        }
        kept.push(bytes[i]);
        i += 1;
    }
    let text = String::from_utf8_lossy(&kept);
    let mut out = String::new();
    let mut spaces = 0;
    for c in text.trim().chars() {
        if c == ' ' {
            spaces += 1;
            if spaces >= 2 {
                continue;
            }
        } else {
            spaces = 0;
        }
        out.push(c);
    }
    out
}

fn parse_hex(text: &str) -> Option<Color> {
    let digits = text.strip_prefix('#')?;
    if !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let full = match digits.len() {
        3 => digits.chars().flat_map(|c| [c, c]).collect::<String>(),
        6 => digits.to_string(),
        _ => return None,
    };
    let value = u32::from_str_radix(&full, 16).ok()?;
    Some(Color::rgb((value >> 16) as u8, (value >> 8) as u8, value as u8))
}

impl PropEditor {
    pub fn new_document(name: &str, settings: PropEditorSettings, now: &str) -> PropEditor {
        let prop = Prop::new(name, &settings.user_name, now);
        PropEditor::with_prop(prop, None, None, settings)
    }

    /// Imported documents have no source path and must be saved, with selection mode active.
    pub fn imported_document(prop: Prop, settings: PropEditorSettings) -> PropEditor {
        let mut editor = Self::with_prop(prop, None, None, settings);
        editor.mode = CanvasMode::Select;
        editor.revision = 1;
        editor.next_revision = 2;
        editor
    }

    fn with_prop(prop: Prop, path: Option<PathBuf>, legacy_source: Option<PathBuf>, settings: PropEditorSettings) -> PropEditor {
        PropEditor {
            prop,
            path,
            legacy_source,
            settings,
            mode: CanvasMode::Draw,
            tab: PanelTab::Layout,
            tree_selection: Vec::new(),
            order_selection: Vec::new(),
            light_selection: Vec::new(),
            expand_requests: Vec::new(),
            adorner: None,
            drag: None,
            pointer: None,
            revision: 0,
            saved_revision: 0,
            next_revision: 1,
            undo: Vec::new(),
            redo: Vec::new(),
            status: String::new(),
            selected_definition: None,
            selected_state_items: Vec::new(),
            state_drag: None,
        }
    }

    /// `LoadPropFromPathAsync`. A failure leaves nothing changed, since the caller keeps its current editor.
    pub fn open(path: &Path, settings: PropEditorSettings, now: &str) -> Result<PropEditor, PropError> {
        let loaded = load_prop(path, now)?;
        let legacy = (loaded.format == PropSourceFormat::LegacyLiteDbV4).then(|| path.to_path_buf());
        Ok(PropEditor::with_prop(loaded.prop, Some(path.to_path_buf()), legacy, settings))
    }

    pub fn prop(&self) -> &Prop {
        &self.prop
    }

    pub fn settings(&self) -> &PropEditorSettings {
        &self.settings
    }

    /// OrderedPointsToSvg: each distinct light element contributes all its lights with the same order label.
    pub fn wire_points(&self) -> Vec<((i32, i32), i32)> {
        let mut nodes: Vec<_> = self.prop.elements().filter(|e| !e.lights().is_empty()).collect();
        nodes.sort_by_key(|e| e.order);
        nodes.into_iter().flat_map(|e| e.lights().iter().map(move |l| ((l.x as i32, l.y as i32), e.order))).collect()
    }

    pub fn set_settings(&mut self, settings: PropEditorSettings) {
        self.settings = settings;
    }

    pub fn file_path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn is_dirty(&self) -> bool {
        self.revision != self.saved_revision
    }

    /// Changes whenever the document does, so a window knows when to fetch again.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn set_status(&mut self, text: &str) {
        self.status = text.to_string();
    }

    /// The Save As panel's suggested file name.
    pub fn suggested_file_name(&self) -> String {
        cleanse_name(self.prop.name())
    }

    /// `SaveModelCommand` with a known path. `Ok(None)` means there is no path yet and the caller must ask for one.
    pub fn save(&mut self, now: &str) -> Result<Option<SaveOutcome>, PropError> {
        match self.path.clone() {
            Some(path) => self.save_as(&path, now).map(Some),
            None => Ok(None),
        }
    }

    pub fn save_as(&mut self, path: &Path, now: &str) -> Result<SaveOutcome, PropError> {
        let errors = self.state_validation();
        if !errors.is_empty() {
            return Err(PropError::new("Invalid State Definitions", errors.join("\n")));
        }
        let before = self.prop.modified_date.clone();
        self.prop.modified_date = now.to_string();
        match save_prop(&self.prop, path, self.legacy_source.as_deref()) {
            Ok(outcome) => {
                self.path = Some(path.to_path_buf());
                // Once written as a package the legacy original is no longer what the next save would replace.
                self.legacy_source = None;
                self.saved_revision = self.revision;
                Ok(outcome)
            }
            Err(e) => {
                self.prop.modified_date = before;
                Err(e)
            }
        }
    }

    // ----- Undo -----

    fn snapshot(&self, name: &str) -> Snapshot {
        Snapshot {
            name: name.to_string(),
            prop: self.prop.clone(),
            tree_selection: self.tree_selection.clone(),
            order_selection: self.order_selection.clone(),
            light_selection: self.light_selection.clone(),
            revision: self.revision,
        }
    }

    /// Call before changing the document: records where to come back to and marks it changed.
    pub(crate) fn begin(&mut self, name: &str) {
        let snapshot = self.snapshot(name);
        self.undo.push(snapshot);
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.revision = self.next_revision;
        self.next_revision += 1;
    }

    /// Takes back a `begin` whose change turned out not to happen.
    pub(crate) fn abandon(&mut self) {
        if let Some(snapshot) = self.undo.pop() {
            self.restore(snapshot);
        }
    }

    fn restore(&mut self, snapshot: Snapshot) {
        self.prop = snapshot.prop;
        self.tree_selection = snapshot.tree_selection;
        self.order_selection = snapshot.order_selection;
        self.light_selection = snapshot.light_selection;
        self.revision = snapshot.revision;
        self.drag = None;
        self.state_drag = None;
        self.rebuild_adorner();
    }

    pub fn undo_name(&self) -> Option<&str> {
        self.undo.last().map(|s| s.name.as_str())
    }

    pub fn redo_name(&self) -> Option<&str> {
        self.redo.last().map(|s| s.name.as_str())
    }

    pub fn undo(&mut self) -> bool {
        let Some(snapshot) = self.undo.pop() else { return false };
        self.redo.push(self.snapshot(&snapshot.name));
        self.restore(snapshot);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(snapshot) = self.redo.pop() else { return false };
        self.undo.push(self.snapshot(&snapshot.name));
        self.restore(snapshot);
        true
    }

    // ----- Rows -----

    pub fn tree_rows(&self) -> Vec<TreeRow> {
        let mut rows = Vec::new();
        let mut stack = vec![vec![self.prop.root_id()]];
        while let Some(path) = stack.pop() {
            if rows.len() >= TREE_ROW_LIMIT {
                break;
            }
            let Some(e) = path.last().and_then(|id| self.prop.element(*id)) else { continue };
            for child in e.children().iter().rev() {
                // An element can't be its own ancestor, and a row that says so would never end.
                if !path.contains(child) {
                    let mut next = path.clone();
                    next.push(*child);
                    stack.push(next);
                }
            }
            rows.push(TreeRow {
                id: e.id,
                name: e.name().to_string(),
                depth: path.len() - 1,
                is_root: e.is_root_node(),
                is_group: e.is_group_node(),
                child_count: e.children().len(),
                light_count: e.lights().len(),
                path,
            });
        }
        rows
    }

    pub(crate) fn path_exists(&self, path: &[Uuid]) -> bool {
        if path.first() != Some(&self.prop.root_id()) {
            return false;
        }
        path.windows(2).all(|pair| self.prop.element(pair[0]).is_some_and(|p| p.children().contains(&pair[1])))
    }

    /// Every row an element shows on.
    pub(crate) fn paths_to(&self, id: Uuid) -> Vec<RowPath> {
        let mut out = Vec::new();
        let mut stack = vec![vec![id]];
        while let Some(reversed) = stack.pop() {
            if out.len() >= 4096 {
                break;
            }
            let Some(e) = reversed.last().and_then(|top| self.prop.element(*top)) else { continue };
            if e.is_root_node() {
                out.push(reversed.iter().rev().copied().collect());
                continue;
            }
            for parent in e.parents() {
                if !reversed.contains(parent) {
                    let mut next = reversed.clone();
                    next.push(*parent);
                    stack.push(next);
                }
            }
        }
        out
    }

    fn prune_selections(&mut self) {
        let valid: Vec<RowPath> = self.tree_selection.iter().filter(|p| self.path_exists(p)).cloned().collect();
        self.tree_selection = valid;
        let prop = &self.prop;
        self.order_selection.retain(|id| prop.is_light_node(*id));
        let lights: HashSet<Uuid> = prop.lights().map(|(_, l)| l.id).collect();
        self.light_selection.retain(|id| lights.contains(id));
    }

    pub fn take_expand_requests(&mut self) -> Vec<RowPath> {
        std::mem::take(&mut self.expand_requests)
    }

    // ----- Selection -----

    pub fn tree_selection(&self) -> &[RowPath] {
        &self.tree_selection
    }

    pub fn order_selection(&self) -> &[Uuid] {
        &self.order_selection
    }

    pub fn light_selection(&self) -> &[Uuid] {
        &self.light_selection
    }

    pub fn tab(&self) -> PanelTab {
        self.tab
    }

    /// Selecting the Layout tab clears the tree's selection and the Order tab its own; the State tab clears nothing.
    pub fn set_tab(&mut self, tab: PanelTab) {
        if self.tab == tab {
            return;
        }
        self.tab = tab;
        self.state_drag = None;
        match tab {
            PanelTab::Layout => self.set_tree_selection(Vec::new()),
            PanelTab::Order => self.set_order_selection(Vec::new()),
            PanelTab::State => {}
        }
        self.rebuild_adorner();
    }

    fn leaf_lights(&self, element: Uuid) -> Vec<Uuid> {
        self.prop
            .leaf_enumerator(element)
            .into_iter()
            .filter_map(|leaf| self.prop.element(leaf))
            .flat_map(|leaf| leaf.lights().iter().map(|l| l.id))
            .collect()
    }

    pub(crate) fn add_lights_quietly(&mut self, lights: Vec<Uuid>) {
        let mut have: HashSet<Uuid> = self.light_selection.iter().copied().collect();
        for light in lights {
            if have.insert(light) {
                self.light_selection.push(light);
            }
        }
    }

    fn remove_lights_quietly(&mut self, lights: &[Uuid]) {
        let gone: HashSet<Uuid> = lights.iter().copied().collect();
        self.light_selection.retain(|l| !gone.contains(l));
    }

    /// The tree's selection as the window reports it. The lights under rows that left it are deselected, those under
    /// rows that joined it selected, and an emptied selection deselects every light.
    pub fn set_tree_selection(&mut self, paths: Vec<RowPath>) {
        let paths: Vec<RowPath> = paths.into_iter().filter(|p| self.path_exists(p)).collect();
        let removed: Vec<Uuid> = self.tree_selection.iter().filter(|p| !paths.contains(p)).filter_map(|p| p.last().copied()).collect();
        let added: Vec<Uuid> = paths.iter().filter(|p| !self.tree_selection.contains(p)).filter_map(|p| p.last().copied()).collect();
        if removed.is_empty() && added.is_empty() {
            return;
        }
        self.tree_selection = paths;
        self.follow_panel_selection(&removed, &added, self.tree_selection.is_empty());
    }

    pub fn set_order_selection(&mut self, ids: Vec<Uuid>) {
        let ids: Vec<Uuid> = ids.into_iter().filter(|id| self.prop.is_light_node(*id)).collect();
        let removed: Vec<Uuid> = self.order_selection.iter().filter(|id| !ids.contains(id)).copied().collect();
        let added: Vec<Uuid> = ids.iter().filter(|id| !self.order_selection.contains(id)).copied().collect();
        if removed.is_empty() && added.is_empty() {
            return;
        }
        self.order_selection = ids;
        self.follow_panel_selection(&removed, &added, self.order_selection.is_empty());
    }

    fn follow_panel_selection(&mut self, removed: &[Uuid], added: &[Uuid], emptied: bool) {
        if emptied {
            self.light_selection.clear();
        } else {
            for element in removed {
                let lights = self.leaf_lights(*element);
                self.remove_lights_quietly(&lights);
            }
            for element in added {
                let lights = self.leaf_lights(*element);
                self.add_lights_quietly(lights);
            }
        }
        self.rebuild_adorner();
    }

    fn select_rows_of(&mut self, elements: &[Uuid]) {
        for element in elements {
            for path in self.paths_to(*element) {
                if !self.tree_selection.contains(&path) {
                    // SelectModels expands every ancestor so a canvas-selected
                    // light remains visible even when its group was collapsed.
                    if path.len() > 1 {
                        let parent = path[..path.len() - 1].to_vec();
                        if !self.expand_requests.contains(&parent) { self.expand_requests.push(parent); }
                    }
                    self.tree_selection.push(path);
                }
            }
        }
    }

    /// Lights selected on the canvas: every tree row of their elements joins the tree's selection.
    pub(crate) fn canvas_select(&mut self, lights: &[Uuid]) {
        let have: HashSet<Uuid> = self.light_selection.iter().copied().collect();
        let fresh: Vec<Uuid> = lights.iter().filter(|l| !have.contains(l)).copied().collect();
        if fresh.is_empty() {
            return;
        }
        let owners = self.prop.light_owners();
        let mut elements = Vec::new();
        for light in &fresh {
            if let Some(owner) = owners.get(light) {
                if !elements.contains(owner) {
                    elements.push(*owner);
                }
            }
        }
        self.add_lights_quietly(fresh);
        self.select_rows_of(&elements);
    }

    /// Lights deselected on the canvas: their elements' rows leave the tree's selection, even when other lights of
    /// the same element stay selected, as upstream.
    pub(crate) fn canvas_deselect(&mut self, lights: &[Uuid]) {
        let have: HashSet<Uuid> = self.light_selection.iter().copied().collect();
        let leaving: Vec<Uuid> = lights.iter().filter(|l| have.contains(l)).copied().collect();
        if leaving.is_empty() {
            return;
        }
        let owners = self.prop.light_owners();
        let elements: HashSet<Uuid> = leaving.iter().filter_map(|l| owners.get(l).copied()).collect();
        self.remove_lights_quietly(&leaving);
        self.tree_selection.retain(|path| path.last().is_none_or(|id| !elements.contains(id)));
    }

    pub(crate) fn canvas_clear(&mut self) {
        if self.light_selection.is_empty() && self.tree_selection.is_empty() && self.order_selection.is_empty() {
            return;
        }
        self.light_selection.clear();
        self.tree_selection.clear();
        self.order_selection.clear();
    }

    /// Replaces the tree's selection with these elements' rows (first row of each) and lets the lights follow.
    fn select_elements_in_tree(&mut self, elements: &[Uuid], under: Option<Uuid>) {
        let mut rows = Vec::new();
        for element in elements {
            let paths = self.paths_to(*element);
            let chosen = paths
                .iter()
                .find(|p| under.is_none_or(|parent| p.len() >= 2 && p[p.len() - 2] == parent))
                .or(paths.first())
                .cloned();
            if let Some(path) = chosen {
                if !rows.contains(&path) {
                    rows.push(path);
                }
            }
        }
        self.tree_selection.clear();
        self.light_selection.clear();
        self.set_tree_selection(rows);
    }

    fn selected_elements(&self) -> Vec<Uuid> {
        let mut out = Vec::new();
        for id in self.tree_selection.iter().filter_map(|p| p.last()) {
            if !out.contains(id) {
                out.push(*id);
            }
        }
        out
    }

    fn row_parent(path: &[Uuid]) -> Option<Uuid> {
        (path.len() >= 2).then(|| path[path.len() - 2])
    }

    pub(crate) fn single_selected(&self) -> Option<&RowPath> {
        match self.tree_selection.as_slice() {
            [only] => Some(only),
            _ => None,
        }
    }

    fn selection_has_root(&self) -> bool {
        self.tree_selection.iter().any(|p| p.len() < 2)
    }

    // ----- Group commands -----

    /// `CanGroup`: something selected, not the root, all of one element type.
    pub fn can_group(&self) -> bool {
        let mut types = self.tree_selection.iter().filter_map(|p| p.last()).filter_map(|id| self.prop.element(*id)).map(|e| e.element_type());
        let Some(first) = types.next() else { return false };
        !self.selection_has_root() && types.all(|t| t == first)
    }

    /// "Copy to New Group": a new group under the root that also holds the selected elements.
    pub fn copy_to_new_group(&mut self, name: &str) -> bool {
        if !self.can_group() {
            return false;
        }
        let elements = self.selected_elements();
        self.begin("Copy to New Group");
        self.canvas_clear();
        let group = self.prop.create_node(name, None, false);
        for element in elements {
            self.prop.add_to_parent(element, group);
        }
        self.after_change();
        true
    }

    /// "Move to New Group": the new group goes where the selection was when it was a parent's whole content, else under the root.
    pub fn move_to_new_group(&mut self, name: &str) -> bool {
        if !self.can_group() {
            return false;
        }
        let rows = self.tree_selection.clone();
        let selected: HashSet<Uuid> = self.selected_elements().into_iter().collect();
        let first_parent = rows.first().and_then(|p| Self::row_parent(p));
        let whole_parent = first_parent
            .and_then(|p| self.prop.element(p))
            .filter(|p| p.children().iter().all(|c| selected.contains(c)))
            .map(|p| p.id);
        let parent_path: Option<RowPath> = whole_parent.and_then(|_| rows.first().map(|p| p[..p.len() - 1].to_vec()));
        self.begin("Move to New Group");
        let group = self.prop.create_node(name, whole_parent, false);
        for row in &rows {
            let (Some(element), Some(parent)) = (row.last().copied(), Self::row_parent(row)) else { continue };
            if !self.prop.contains(element) {
                continue;
            }
            self.prop.add_to_parent(element, group);
            self.prop.remove_from_parent(element, parent);
        }
        self.after_change();
        let mut group_path = parent_path.unwrap_or_else(|| vec![self.prop.root_id()]);
        group_path.push(group);
        self.tree_selection.clear();
        self.light_selection.clear();
        self.expand_requests.push(group_path.clone());
        self.set_tree_selection(vec![group_path]);
        true
    }

    /// "Create Empty Group": one group row selected whose children, if any, are all groups.
    pub fn can_create_empty_group(&self) -> bool {
        let Some(e) = self.single_selected().and_then(|p| p.last()).and_then(|id| self.prop.element(*id)) else { return false };
        e.is_group_node() && e.children().iter().all(|c| self.prop.is_group_node(*c))
    }

    pub fn suggested_empty_group_name(&self) -> String {
        self.single_selected()
            .and_then(|p| p.last())
            .and_then(|id| self.prop.element(*id))
            .map(|e| self.prop.uniquify(e.name(), 2, None))
            .unwrap_or_default()
    }

    pub fn create_empty_group(&mut self, name: &str) -> bool {
        if !self.can_create_empty_group() {
            return false;
        }
        let Some(row) = self.single_selected().cloned() else { return false };
        self.begin("Create Empty Group");
        self.prop.create_node(name, row.last().copied(), false);
        self.expand_requests.push(row);
        self.after_change();
        true
    }

    // ----- Clipboard -----

    pub fn can_copy(&self) -> bool {
        self.can_group()
    }

    pub fn copy(&self) -> Option<PropClip> {
        if !self.can_copy() {
            return None;
        }
        PropClip::capture(&self.prop, &self.selected_elements())
    }

    /// `Cut`: the clip, then each selected row leaves its parent. An element with no parent left is gone, lights and all.
    pub fn cut(&mut self) -> Option<PropClip> {
        let clip = self.copy()?;
        let rows = self.tree_selection.clone();
        self.begin("Cut");
        self.canvas_clear();
        for row in rows {
            if let (Some(element), Some(parent)) = (row.last().copied(), Self::row_parent(&row)) {
                self.prop.remove_from_parent(element, parent);
            }
        }
        self.after_change();
        Some(clip)
    }

    pub fn can_paste(&self, clip: &PropClip) -> bool {
        self.single_selected().and_then(|p| p.last()).is_some_and(|target| clip.can_paste(&self.prop, *target))
    }

    pub fn can_paste_as_new(&self, clip: &PropClip) -> bool {
        self.single_selected().and_then(|p| p.last()).is_some_and(|target| clip.can_paste_as_new(&self.prop, *target))
    }

    pub fn paste(&mut self, clip: &PropClip) -> bool {
        if !self.can_paste(clip) {
            return false;
        }
        let Some(row) = self.single_selected().cloned() else { return false };
        let Some(target) = row.last().copied() else { return false };
        self.begin("Paste");
        clip.paste(&mut self.prop, target);
        self.after_change();
        self.expand_requests.push(row.clone());
        self.tree_selection.clear();
        self.light_selection.clear();
        self.set_tree_selection(vec![row]);
        true
    }

    /// The name Paste as New offers: the target's, made unique.
    pub fn suggested_paste_name(&self) -> String {
        self.suggested_empty_group_name()
    }

    pub fn paste_as_new(&mut self, clip: &PropClip, name: &str) -> bool {
        if !self.can_paste_as_new(clip) {
            return false;
        }
        let Some(row) = self.single_selected().cloned() else { return false };
        let Some(target) = row.last().copied() else { return false };
        self.begin("Paste as New");
        let created = clip.paste_as_new(&mut self.prop, target, name);
        self.after_change();
        self.expand_requests.push(row);
        self.select_elements_in_tree(&created, Some(target));
        true
    }

    pub fn can_delete_elements(&self) -> bool {
        !self.tree_selection.is_empty() && !self.selection_has_root()
    }

    /// The context menu's Delete: each selected row leaves its parent. No confirmation, as upstream.
    pub fn delete_selected_elements(&mut self) -> bool {
        if !self.can_delete_elements() {
            return false;
        }
        let rows = self.tree_selection.clone();
        self.begin("Delete");
        for row in rows {
            if let (Some(element), Some(parent)) = (row.last().copied(), Self::row_parent(&row)) {
                self.prop.remove_from_parent(element, parent);
            }
        }
        self.light_selection.clear();
        self.after_change();
        true
    }

    // ----- Names -----

    /// Rename from the prompt or the inline editor: unique among the other elements. Returns the name it got.
    pub fn rename(&mut self, id: Uuid, name: &str) -> Option<String> {
        let current = self.prop.element(id)?.name().to_string();
        let unique = self.prop.uniquify(name, 2, Some(id));
        if unique == current {
            return Some(current);
        }
        self.begin("Rename");
        self.prop.set_element_name(id, &unique);
        Some(unique)
    }

    /// The pattern and find/replace renamers: `names[i]` for `ids[i]`, each made unique. Stops when names run out.
    pub fn rename_many(&mut self, ids: &[Uuid], names: &[String]) -> bool {
        if ids.is_empty() || names.is_empty() {
            return false;
        }
        self.begin("Rename");
        for (id, name) in ids.iter().zip(names) {
            self.prop.rename(*id, name);
        }
        true
    }

    // ----- Tree drag and drop -----

    fn dragged_kind(&self, dragged: &[RowPath]) -> Option<bool> {
        let mut groups = None;
        for row in dragged {
            let e = self.prop.element(*row.last()?)?;
            let is_group = e.is_group_node();
            if groups.is_some_and(|g| g != is_group) {
                return None;
            }
            groups = Some(is_group);
        }
        groups
    }

    /// `CanAcceptData`. `index` None is a drop on the row itself; otherwise the place among the parent's children.
    pub fn can_drop(&self, dragged: &[RowPath], parent: &[Uuid], index: Option<usize>) -> bool {
        if dragged.is_empty() || !self.path_exists(parent) || dragged.iter().any(|row| row.len() < 2 || !self.path_exists(row)) {
            return false;
        }
        let Some(groups) = self.dragged_kind(dragged) else { return false };
        let Some(target) = parent.last().copied() else { return false };
        let Some(t) = self.prop.element(target) else { return false };
        let elements: Vec<Uuid> = dragged.iter().filter_map(|row| row.last().copied()).collect();
        if elements.iter().any(|e| self.prop.is_same_or_descendant(*e, target)) {
            return false;
        }
        match index {
            None => {
                if !t.is_group_node() || (t.is_root_node() && !groups) {
                    return false;
                }
                if groups && !self.prop.can_add_group_nodes(target) {
                    return false;
                }
                if !groups && !self.prop.can_add_leaf_nodes(target) {
                    return false;
                }
                !elements.iter().any(|e| t.children().contains(e))
            }
            Some(index) => {
                if index > t.children().len() {
                    return false;
                }
                // Stricter than upstream, which let a row dropped beside another put lights and groups under one parent.
                if groups && !self.prop.can_add_group_nodes(target) {
                    return false;
                }
                if !groups && !self.prop.can_add_light_nodes(target) {
                    return false;
                }
                let inside = dragged.iter().filter(|row| Self::row_parent(row) == Some(target)).count();
                if inside == dragged.len() {
                    return true;
                }
                inside == 0 && !elements.iter().any(|e| t.children().contains(e))
            }
        }
    }

    /// `Drop`. Only ever a move. `reverse` is the Control key, which drops the dragged rows in reverse order.
    pub fn drop_rows(&mut self, dragged: &[RowPath], parent: &[Uuid], index: Option<usize>, reverse: bool) -> bool {
        if !self.can_drop(dragged, parent, index) {
            return false;
        }
        let Some(target) = parent.last().copied() else { return false };
        let mut rows: Vec<RowPath> = dragged.to_vec();
        if reverse {
            rows.reverse();
        }
        let elements: Vec<Uuid> = rows.iter().filter_map(|row| row.last().copied()).collect();
        let all_inside = rows.iter().all(|row| Self::row_parent(row) == Some(target));
        self.begin("Move");
        match index {
            Some(index) if all_inside => {
                if !self.prop.try_move_within_parent(target, &elements, index) {
                    self.abandon();
                    return false;
                }
            }
            _ => {
                let mut at = index.unwrap_or(usize::MAX);
                for row in &rows {
                    let (Some(element), Some(old)) = (row.last().copied(), Self::row_parent(row)) else { continue };
                    if old == target || !self.prop.contains(element) {
                        continue;
                    }
                    if self.prop.insert_to_parent(element, target, at) {
                        at = at.saturating_add(1);
                        self.prop.remove_from_parent(element, old);
                    }
                }
            }
        }
        self.after_change();
        self.expand_requests.push(parent.to_vec());
        let moved: Vec<RowPath> = elements
            .iter()
            .map(|e| {
                let mut path = parent.to_vec();
                path.push(*e);
                path
            })
            .collect();
        self.tree_selection.clear();
        self.light_selection.clear();
        self.set_tree_selection(moved);
        true
    }

    // ----- Order tab -----

    fn ordered_light_nodes(&self) -> Vec<Uuid> {
        let mut leaves: Vec<Uuid> = self.prop.distinct_leaves(self.prop.root_id()).into_iter().filter(|id| self.prop.is_light_node(*id)).collect();
        leaves.sort_by_key(|id| self.prop.order(*id));
        leaves
    }

    pub fn order_rows(&self) -> Vec<OrderRow> {
        self.ordered_light_nodes()
            .into_iter()
            .filter_map(|id| self.prop.element(id))
            .map(|e| OrderRow { id: e.id, order: e.order, name: e.name().to_string() })
            .collect()
    }

    fn renumber(&mut self, list: &[Uuid]) {
        for (i, id) in list.iter().enumerate() {
            if let Some(e) = self.prop.element_mut(*id) {
                e.order = i as i32 + 1;
            }
        }
    }

    /// A drop in the Order tab: the dragged rows, in list order, move to `index` and the whole list is renumbered 1..n.
    pub fn order_drop(&mut self, dragged: &[Uuid], index: usize) -> bool {
        let list = self.ordered_light_nodes();
        let moving: Vec<Uuid> = list.iter().filter(|id| dragged.contains(id)).copied().collect();
        if moving.is_empty() || index > list.len() {
            return false;
        }
        let before = list.iter().take(index).filter(|id| moving.contains(id)).count();
        let mut rest: Vec<Uuid> = list.iter().filter(|id| !moving.contains(id)).copied().collect();
        let at = (index - before).min(rest.len());
        rest.splice(at..at, moving);
        self.begin("Reorder");
        self.renumber(&rest);
        true
    }

    /// "Reverse": the selected rows swap places among themselves, then the list is renumbered.
    /// Upstream indexes by `Order - 1`, which runs off the list once deletions leave gaps; positions are used here.
    pub fn order_reverse(&mut self) -> bool {
        let mut list = self.ordered_light_nodes();
        let positions: Vec<usize> = list.iter().enumerate().filter(|(_, id)| self.order_selection.contains(id)).map(|(i, _)| i).collect();
        if positions.is_empty() {
            return false;
        }
        let picked: Vec<Uuid> = positions.iter().map(|i| list[*i]).collect();
        for (slot, id) in positions.iter().zip(picked.iter().rev()) {
            list[*slot] = *id;
        }
        self.begin("Reverse");
        self.renumber(&list);
        true
    }

    // ----- Prop Info and Element Info -----

    pub fn prop_field(&self, field: PropField) -> String {
        let p = &self.prop;
        match field {
            PropField::Name => p.name().to_string(),
            PropField::Type => p.prop_type.clone(),
            PropField::CreatedBy => p.created_by.clone(),
            PropField::PhysicalHeight => p.physical.height.clone(),
            PropField::PhysicalWidth => p.physical.width.clone(),
            PropField::PhysicalDepth => p.physical.depth.clone(),
            PropField::Material => p.physical.material.clone(),
            PropField::NodeCount => p.physical.node_count.clone(),
            PropField::BulbType => p.physical.bulb_type.clone(),
            PropField::ColorMode => p.physical.color_mode.name().to_string(),
            PropField::VendorName => p.vendor.name.clone(),
            PropField::VendorWebsite => p.vendor.website.clone(),
            PropField::VendorContact => p.vendor.contact.clone(),
            PropField::VendorEmail => p.vendor.email.clone(),
            PropField::VendorPhone => p.vendor.phone.clone(),
            PropField::Notes => p.information.notes.clone(),
        }
    }

    pub fn set_prop_field(&mut self, field: PropField, value: &str) -> bool {
        if self.prop_field(field) == value {
            return false;
        }
        let color_mode = if field == PropField::ColorMode {
            match ColorMode::parse(value) {
                Some(mode) => Some(mode),
                None => return false,
            }
        } else {
            None
        };
        self.begin("Change Prop Info");
        let p = &mut self.prop;
        let text = value.to_string();
        match field {
            PropField::Name => p.set_name(value),
            PropField::Type => p.prop_type = text,
            PropField::CreatedBy => p.created_by = text,
            PropField::PhysicalHeight => p.physical.height = text,
            PropField::PhysicalWidth => p.physical.width = text,
            PropField::PhysicalDepth => p.physical.depth = text,
            PropField::Material => p.physical.material = text,
            PropField::NodeCount => p.physical.node_count = text,
            PropField::BulbType => p.physical.bulb_type = text,
            PropField::ColorMode => p.physical.color_mode = color_mode.unwrap_or(p.physical.color_mode),
            PropField::VendorName => p.vendor.name = text,
            PropField::VendorWebsite => p.vendor.website = text,
            PropField::VendorContact => p.vendor.contact = text,
            PropField::VendorEmail => p.vendor.email = text,
            PropField::VendorPhone => p.vendor.phone = text,
            PropField::Notes => p.information.notes = text,
        }
        true
    }

    /// The Light Size line of Element Info: the size, "multiple sizes", or nothing when there are no lights below.
    fn light_size_text(&self, id: Uuid) -> String {
        let Some(e) = self.prop.element(id) else { return String::new() };
        if e.is_light_node() {
            return e.raw_light_size().to_string();
        }
        let mut found = 0;
        for leaf in self.prop.leaf_enumerator(id) {
            let Some(l) = self.prop.element(leaf) else { continue };
            if !l.is_light_node() {
                continue;
            }
            let size = l.raw_light_size();
            if found == 0 {
                found = size;
            } else if size != found {
                return "multiple sizes".to_string();
            }
        }
        if found == 0 {
            String::new()
        } else {
            found.to_string()
        }
    }

    /// Element Info shows only while exactly one tree row is selected.
    pub fn element_info(&self) -> Option<ElementInfo> {
        let id = *self.single_selected()?.last()?;
        let e = self.prop.element(id)?;
        let error = if e.name().is_empty() {
            Some("Name can not be empty".to_string())
        } else if self.prop.is_name_duplicated(e.name()) {
            Some("Duplicate name".to_string())
        } else {
            None
        };
        let color = e.face.default_color;
        Some(ElementInfo {
            id,
            name: e.name().to_string(),
            face_component: e.face.component,
            face_color: if e.face.component == FaceComponent::None { String::new() } else { format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b) },
            model_type: e.model_type,
            children: e.children().len(),
            lights: self.prop.leaf_enumerator(id).into_iter().filter(|leaf| self.prop.is_light_node(*leaf)).count(),
            light_size: self.light_size_text(id),
            error,
        })
    }

    /// The Element Info name box: taken as typed, not made unique. A blank or repeated name shows as an error instead.
    pub fn set_element_name(&mut self, id: Uuid, name: &str) -> bool {
        if self.prop.element(id).is_none_or(|e| e.name() == name) {
            return false;
        }
        self.begin("Rename");
        self.prop.set_element_name(id, name);
        true
    }

    pub fn set_face_component(&mut self, id: Uuid, component: FaceComponent) -> bool {
        if self.prop.element(id).is_none_or(|e| e.face.component == component) {
            return false;
        }
        self.begin("Change Face Component");
        if let Some(e) = self.prop.element_mut(id) {
            e.face.component = component;
        }
        true
    }

    /// A face colour as "#RGB" or "#RRGGBB". Anything else stores the empty colour, as upstream.
    pub fn set_face_color(&mut self, id: Uuid, hex: &str) -> bool {
        let color = parse_hex(hex).unwrap_or(Color::EMPTY);
        if self.prop.element(id).is_none_or(|e| e.face.default_color == color) {
            return false;
        }
        self.begin("Change Face Color");
        if let Some(e) = self.prop.element_mut(id) {
            e.face.default_color = color;
        }
        true
    }

    pub fn set_model_type(&mut self, id: Uuid, model_type: ElementModelType) -> bool {
        if self.prop.element(id).is_none_or(|e| e.model_type == model_type) {
            return false;
        }
        // PropStateModelResolver: a Model is a group (or an empty root), and only one is explicit.
        if model_type == ElementModelType::Model && self.prop.element(id).is_some_and(|e| e.is_leaf() && !(id == self.prop.root_id() && e.children().is_empty())) {
            return false;
        }
        self.begin("Change Model Type");
        if model_type == ElementModelType::Model {
            for other in self.prop.breadth_first() {
                if let Some(e) = self.prop.element_mut(other) {
                    if e.model_type == ElementModelType::Model { e.model_type = ElementModelType::None; }
                }
            }
        }
        if let Some(e) = self.prop.element_mut(id) {
            e.model_type = model_type;
        }
        true
    }

    // ----- Background -----

    /// `SetImage`: the canvas takes the picture's size and the lights stay where they are.
    pub fn assign_background(&mut self, path: &Path) -> Result<(), PropError> {
        let bytes = std::fs::read(path).map_err(|e| PropError::new("The picture could not be read.", format!("{}: {e}", path.display())))?;
        let image = PropImage::decode(bytes).map_err(|e| PropError::new("The picture could not be read.", e))?;
        self.begin("Assign Background");
        self.prop.set_image(image);
        self.rebuild_adorner();
        Ok(())
    }

    pub fn set_opacity(&mut self, opacity: f64) -> bool {
        if !opacity.is_finite() || self.prop.opacity == opacity {
            return false;
        }
        self.begin("Change Background Opacity");
        self.prop.opacity = opacity.clamp(0.0, 1.0);
        true
    }

    /// `ApplyBackgroundImageScale`: a new canvas size, with each light's position scaled along when asked.
    pub fn scale_background(&mut self, target_width: i32, target_height: i32, scale_lights: bool) -> Result<(), PropError> {
        if !(1..=100_000).contains(&target_width) || !(1..=100_000).contains(&target_height) {
            return Err(PropError::new("The target canvas dimensions must be between 1 and 100,000 pixels.", "target out of range"));
        }
        let (width, height) = (self.prop.width, self.prop.height);
        if !(width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0) {
            return Err(PropError::new("The current canvas dimensions must be finite positive values.", "current size invalid"));
        }
        self.begin("Scale Background Image");
        let (sx, sy) = (target_width as f64 / width, target_height as f64 / height);
        self.prop.width = target_width as f64;
        self.prop.height = target_height as f64;
        if scale_lights {
            for light in self.prop.lights_mut() {
                light.x *= sx;
                light.y *= sy;
            }
        }
        self.rebuild_adorner();
        Ok(())
    }

    /// After a change to the element graph: drop selections that point at nothing and refresh the adorner.
    pub(crate) fn after_change(&mut self) {
        let alive: HashSet<_> = self.prop.elements().map(|e| e.id).collect();
        for id in self.prop.breadth_first() {
            if let Some(element) = self.prop.element_mut(id) {
                for definition in &mut element.state_definitions {
                    for item in &mut definition.items { item.element_ids.retain(|id| alive.contains(id)); }
                }
            }
        }
        self.prune_selections();
        self.rebuild_adorner();
    }

    pub(crate) fn request_expand(&mut self, path: RowPath) {
        self.expand_requests.push(path);
    }

    pub(crate) fn add_tree_rows(&mut self, rows: Vec<RowPath>) {
        let mut all = self.tree_selection.clone();
        for row in rows {
            if !all.contains(&row) {
                all.push(row);
            }
        }
        self.set_tree_selection(all);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: &str = "2024-01-01T00:00:00";

    pub(crate) fn editor_with_lights(count: usize) -> (PropEditor, Uuid, Vec<Uuid>) {
        let mut editor = PropEditor::new_document("Star {1}", PropEditorSettings::default(), NOW);
        let mut nodes = Vec::new();
        for i in 0..count {
            nodes.push(editor.prop.add_light(None, 10.0 + 20.0 * i as f64, 10.0, 3).unwrap());
        }
        let group = editor.prop.element(nodes[0]).unwrap().parents()[0];
        (editor, group, nodes)
    }

    fn row(editor: &PropEditor, ids: &[Uuid]) -> RowPath {
        let mut path = vec![editor.prop.root_id()];
        path.extend_from_slice(ids);
        path
    }

    #[test]
    fn wire_points_keep_multi_light_order_once_and_truncate_coordinates() {
        let (mut editor, group, nodes) = editor_with_lights(2);
        let other = editor.prop.create_node("Shared", None, false);
        editor.prop.add_to_parent(nodes[0], other);
        editor.prop.add_light(Some(nodes[0]), 25.9, -4.9, 3).unwrap();
        editor.prop.element_mut(nodes[0]).unwrap().order = 8;
        editor.prop.element_mut(nodes[1]).unwrap().order = 2;
        assert_eq!(editor.prop.element(group).unwrap().children().len(), 2);
        assert_eq!(editor.wire_points(), vec![((30,10),2),((10,10),8),((25,-4),8)]);
    }

    #[test]
    fn names_get_a_token_and_lose_it_for_files() {
        assert_eq!(new_prop_name("Tree"), "Tree {1}");
        assert_eq!(new_prop_name("Tree {12}"), "Tree {12}");
        assert_eq!(new_prop_name(""), " {1}");
        assert_eq!(new_prop_name("Tree {x}"), "Tree {x} {1}");
        assert_eq!(cleanse_name("  Big   Star {1} "), "Big Star");
        assert_eq!(cleanse_name("A {1} B"), "A B");
        assert_eq!(cleanse_name("A{}B"), "A{}B");
    }

    #[test]
    fn paths_round_trip_as_text() {
        let path = vec![Uuid::new_v4(), Uuid::new_v4()];
        assert_eq!(parse_path(&path_text(&path)), Some(path));
        assert_eq!(parse_path("nope"), None);
    }

    #[test]
    fn tree_rows_list_the_root_then_groups_then_lights() {
        let (editor, group, nodes) = editor_with_lights(2);
        let rows = editor.tree_rows();
        assert_eq!(rows.len(), 4);
        assert!(rows[0].is_root);
        assert_eq!(rows[1].id, group);
        assert_eq!(rows[1].name, "Star {1} Group");
        assert_eq!(rows[2].path, row(&editor, &[group, nodes[0]]));
        assert_eq!(rows[2].depth, 2);
        assert_eq!(rows[2].light_count, 1);
        assert!(!rows[2].is_group);
    }

    #[test]
    fn tree_selection_selects_the_lights_below_and_back() {
        let (mut editor, group, nodes) = editor_with_lights(3);
        editor.set_tree_selection(vec![row(&editor, &[group])]);
        assert_eq!(editor.light_selection().len(), 3);
        editor.set_tree_selection(vec![row(&editor, &[group, nodes[1]])]);
        let light = editor.prop.element(nodes[1]).unwrap().lights()[0].id;
        assert_eq!(editor.light_selection(), &[light]);
        editor.set_tree_selection(Vec::new());
        assert!(editor.light_selection().is_empty());
    }

    #[test]
    fn a_light_picked_on_the_canvas_selects_every_row_of_its_element() {
        let (mut editor, group, nodes) = editor_with_lights(2);
        let other = editor.prop.create_node("Other", None, false);
        editor.prop.add_to_parent(nodes[0], other);
        let light = editor.prop.element(nodes[0]).unwrap().lights()[0].id;
        editor.take_expand_requests();
        editor.canvas_select(&[light]);
        assert_eq!(editor.tree_selection().len(), 2);
        assert!(editor.tree_selection().contains(&row(&editor, &[group, nodes[0]])));
        assert!(editor.tree_selection().contains(&row(&editor, &[other, nodes[0]])));
        let expanded = editor.take_expand_requests();
        assert_eq!(expanded.len(), 2);
        assert!(expanded.contains(&row(&editor, &[group])) && expanded.contains(&row(&editor, &[other])));
        editor.canvas_deselect(&[light]);
        assert!(editor.tree_selection().is_empty());
    }

    #[test]
    fn undo_and_redo_restore_the_document_and_the_dirty_state() {
        let (mut editor, group, _) = editor_with_lights(2);
        assert!(!editor.is_dirty());
        editor.set_tree_selection(vec![row(&editor, &[group])]);
        assert!(editor.create_empty_group("Nope") == false, "a group of lights takes no groups");
        editor.set_tree_selection(vec![vec![editor.prop.root_id()]]);
        assert!(editor.create_empty_group("Arms"));
        assert!(editor.is_dirty());
        assert_eq!(editor.undo_name(), Some("Create Empty Group"));
        let count = editor.prop.element_count();
        assert!(editor.undo());
        assert!(!editor.is_dirty());
        assert_eq!(editor.prop.element_count(), count - 1);
        assert_eq!(editor.redo_name(), Some("Create Empty Group"));
        assert!(editor.redo());
        assert!(editor.is_dirty());
        assert_eq!(editor.prop.element_count(), count);
    }

    #[test]
    fn saving_clears_dirty_and_undo_past_it_is_dirty_again() {
        let (mut editor, _, nodes) = editor_with_lights(1);
        let dir = crate::file::tests::scratch("editor-save");
        let path = dir.join("Star.prp");
        assert_eq!(editor.save(NOW).unwrap(), None, "no path yet");
        editor.rename(nodes[0], "First");
        editor.save_as(&path, "2025-02-02T00:00:00").unwrap();
        assert!(!editor.is_dirty());
        assert_eq!(editor.prop.modified_date, "2025-02-02T00:00:00");
        editor.undo();
        assert!(editor.is_dirty());
        let again = PropEditor::open(&path, PropEditorSettings::default(), NOW).unwrap();
        assert_eq!(again.prop.element(nodes[0]).unwrap().name(), "First");
        assert_eq!(again.file_path(), Some(path.as_path()));
        assert!(!again.is_dirty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rename_keeps_an_unchanged_name_and_numbers_a_clash() {
        let (mut editor, _, nodes) = editor_with_lights(2);
        let first = editor.prop.element(nodes[0]).unwrap().name().to_string();
        let second = editor.prop.element(nodes[1]).unwrap().name().to_string();
        assert_eq!(editor.rename(nodes[0], &first), Some(first.clone()));
        assert!(!editor.is_dirty());
        assert_eq!(editor.rename(nodes[0], &second), Some(format!("{second} - 2")));
    }

    #[test]
    fn move_to_new_group_takes_the_place_of_a_fully_selected_parent() {
        let (mut editor, group, nodes) = editor_with_lights(3);
        editor.set_tree_selection(vec![row(&editor, &[group, nodes[0]]), row(&editor, &[group, nodes[1]])]);
        assert!(editor.can_group());
        assert!(editor.move_to_new_group("Pair"));
        let root = editor.prop.root_id();
        let pair = *editor.prop.element(root).unwrap().children().last().unwrap();
        assert_eq!(editor.prop.element(pair).unwrap().name(), "Pair");
        assert_eq!(editor.prop.element(pair).unwrap().children(), &[nodes[0], nodes[1]]);
        assert_eq!(editor.prop.element(group).unwrap().children(), &[nodes[2]]);
        assert_eq!(editor.tree_selection(), &[row(&editor, &[pair])]);
        assert_eq!(editor.light_selection().len(), 2);
        assert_eq!(editor.take_expand_requests(), vec![row(&editor, &[pair])]);
    }

    #[test]
    fn copy_to_new_group_links_without_removing() {
        let (mut editor, group, nodes) = editor_with_lights(2);
        editor.set_tree_selection(vec![row(&editor, &[group, nodes[0]])]);
        assert!(editor.copy_to_new_group("Extra"));
        assert_eq!(editor.prop.element(nodes[0]).unwrap().parents().len(), 2);
        assert!(editor.tree_selection().is_empty());
        assert_eq!(editor.prop.light_count(), 2);
    }

    #[test]
    fn cut_then_paste_moves_an_element_and_keeps_its_light() {
        let (mut editor, group, nodes) = editor_with_lights(2);
        let other = editor.prop.create_node("Other", None, false);
        editor.set_tree_selection(vec![row(&editor, &[group, nodes[0]])]);
        let clip = editor.cut().unwrap();
        assert!(!editor.prop.contains(nodes[0]));
        assert!(editor.tree_selection().is_empty());
        editor.set_tree_selection(vec![row(&editor, &[other])]);
        assert!(editor.can_paste(&clip));
        assert!(editor.paste(&clip));
        assert_eq!(editor.prop.element(nodes[0]).unwrap().parents(), &[other]);
        assert_eq!(editor.prop.light_count(), 2);
        assert_eq!(editor.light_selection().len(), 1, "the target row is selected, so its light is");
    }

    #[test]
    fn delete_from_the_menu_removes_rows_but_never_the_root() {
        let (mut editor, group, nodes) = editor_with_lights(2);
        editor.set_tree_selection(vec![vec![editor.prop.root_id()]]);
        assert!(!editor.delete_selected_elements());
        editor.set_tree_selection(vec![row(&editor, &[group, nodes[1]])]);
        assert!(editor.delete_selected_elements());
        assert!(!editor.prop.contains(nodes[1]));
        assert!(editor.tree_selection().is_empty());
        assert!(editor.light_selection().is_empty());
    }

    #[test]
    fn drop_rules_follow_the_kinds() {
        let (mut editor, group, nodes) = editor_with_lights(3);
        let empty = editor.prop.create_node("Empty", None, false);
        let root = vec![editor.prop.root_id()];
        let light_row = row(&editor, &[group, nodes[0]]);
        let group_row = row(&editor, &[group]);
        let empty_row = row(&editor, &[empty]);
        assert!(!editor.can_drop(&[light_row.clone()], &root, None), "lights never sit under the root");
        assert!(!editor.can_drop(&[light_row.clone()], &root, Some(0)));
        assert!(editor.can_drop(&[light_row.clone()], &empty_row, None));
        assert!(!editor.can_drop(&[light_row.clone()], &group_row, None), "already its parent");
        assert!(editor.can_drop(&[light_row.clone()], &group_row, Some(3)), "reorder inside its parent");
        assert!(!editor.can_drop(&[light_row.clone(), group_row.clone()], &empty_row, None), "mixed kinds");
        assert!(!editor.can_drop(&[group_row.clone()], &group_row, None), "onto itself");
        assert!(!editor.can_drop(&[group_row.clone()], &light_row, None));
        assert!(editor.can_drop(&[group_row.clone()], &empty_row, None));
        assert!(editor.can_drop(&[empty_row.clone()], &root, Some(0)));
        assert!(!editor.can_drop(&[root.clone()], &empty_row, None), "the root does not move");
    }

    #[test]
    fn drop_inside_a_parent_reorders_and_onto_a_group_moves() {
        let (mut editor, group, nodes) = editor_with_lights(3);
        let group_row = row(&editor, &[group]);
        assert!(editor.drop_rows(&[row(&editor, &[group, nodes[0]])], &group_row, Some(3), false));
        assert_eq!(editor.prop.element(group).unwrap().children(), &[nodes[1], nodes[2], nodes[0]]);
        assert_eq!(editor.tree_selection(), &[row(&editor, &[group, nodes[0]])]);
        let empty = editor.prop.create_node("Empty", None, false);
        let empty_row = row(&editor, &[empty]);
        let rows = vec![row(&editor, &[group, nodes[1]]), row(&editor, &[group, nodes[2]])];
        assert!(editor.drop_rows(&rows, &empty_row, None, true));
        assert_eq!(editor.prop.element(empty).unwrap().children(), &[nodes[2], nodes[1]]);
        assert_eq!(editor.prop.element(group).unwrap().children(), &[nodes[0]]);
        assert_eq!(editor.light_selection().len(), 2);
        assert!(editor.undo());
        assert_eq!(editor.prop.element(group).unwrap().children(), &[nodes[1], nodes[2], nodes[0]]);
    }

    #[test]
    fn order_tab_drop_and_reverse_renumber() {
        let (mut editor, _, nodes) = editor_with_lights(4);
        assert_eq!(editor.order_rows().iter().map(|r| r.order).collect::<Vec<_>>(), vec![1, 2, 3, 4]);
        assert!(editor.order_drop(&[nodes[0]], 4));
        assert_eq!(editor.order_rows().iter().map(|r| r.id).collect::<Vec<_>>(), vec![nodes[1], nodes[2], nodes[3], nodes[0]]);
        assert_eq!(editor.prop.order(nodes[0]), 4);
        editor.set_order_selection(vec![nodes[1], nodes[0]]);
        assert_eq!(editor.light_selection().len(), 2);
        assert!(editor.order_reverse());
        assert_eq!(editor.order_rows().iter().map(|r| r.id).collect::<Vec<_>>(), vec![nodes[0], nodes[2], nodes[3], nodes[1]]);
    }

    #[test]
    fn reverse_works_after_deletions_left_gaps() {
        let (mut editor, group, nodes) = editor_with_lights(4);
        editor.prop.remove_from_parent(nodes[1], group);
        editor.prop.remove_from_parent(nodes[2], group);
        editor.prop.element_mut(nodes[3]).unwrap().order = 10;
        editor.set_order_selection(vec![nodes[0], nodes[3]]);
        assert!(editor.order_reverse());
        assert_eq!(editor.order_rows(), vec![
            OrderRow { id: nodes[3], order: 1, name: editor.prop.element(nodes[3]).unwrap().name().to_string() },
            OrderRow { id: nodes[0], order: 2, name: editor.prop.element(nodes[0]).unwrap().name().to_string() },
        ]);
    }

    #[test]
    fn element_info_reports_counts_sizes_and_name_errors() {
        let (mut editor, group, nodes) = editor_with_lights(2);
        editor.set_tree_selection(vec![row(&editor, &[group])]);
        let info = editor.element_info().unwrap();
        assert_eq!((info.children, info.lights, info.light_size.as_str()), (2, 2, "3"));
        assert_eq!(info.face_color, "");
        editor.prop.set_light_size(nodes[1], 5);
        assert_eq!(editor.element_info().unwrap().light_size, "multiple sizes");
        editor.set_tree_selection(vec![row(&editor, &[group, nodes[0]])]);
        let other = editor.prop.element(nodes[1]).unwrap().name().to_string();
        assert!(editor.set_element_name(nodes[0], &other));
        assert_eq!(editor.element_info().unwrap().error.as_deref(), Some("Duplicate name"));
        assert!(editor.set_element_name(nodes[0], ""));
        assert_eq!(editor.element_info().unwrap().error.as_deref(), Some("Name can not be empty"));
        assert!(editor.set_face_component(nodes[0], FaceComponent::EyesOpen));
        assert!(editor.set_face_color(nodes[0], "#abc"));
        assert_eq!(editor.element_info().unwrap().face_color, "#AABBCC");
        assert!(editor.set_face_color(nodes[0], "blue"));
        assert_eq!(editor.prop.element(nodes[0]).unwrap().face.default_color, Color::EMPTY);
    }

    #[test]
    fn model_role_transition_preserves_authored_states_and_undo_reverts_demotion() {
        let (mut editor, source, nodes) = editor_with_lights(2);
        let target = editor.prop.create_node("Other", None, false);
        editor.prop.add_to_parent(nodes[1], target);
        let revision = editor.revision();
        assert!(!editor.set_model_type(nodes[0], ElementModelType::Model));
        assert_eq!(editor.revision(), revision);
        assert!(editor.set_model_type(source, ElementModelType::Model));
        editor.add_state_definition("Door").unwrap();
        editor.assign_state_elements(&[nodes[0]], false, false);
        let authored = editor.state_definitions().to_vec();
        assert!(editor.set_model_type(target, ElementModelType::Model));
        assert_eq!(editor.prop.element(source).unwrap().model_type, ElementModelType::None);
        assert_eq!(editor.prop.element(target).unwrap().model_type, ElementModelType::Model);
        assert!(editor.state_definitions().is_empty());
        assert_eq!(editor.prop.element(source).unwrap().state_definitions, authored);
        assert!(editor.undo());
        assert_eq!(editor.prop.element(source).unwrap().model_type, ElementModelType::Model);
        assert_eq!(editor.prop.element(target).unwrap().model_type, ElementModelType::None);
        assert_eq!(editor.state_definitions(), authored);
        assert!(editor.set_model_type(nodes[0], ElementModelType::SubModel));
        assert_eq!(editor.state_definitions(), authored);
        let mut empty = PropEditor::new_document("Empty", PropEditorSettings::default(), NOW);
        assert!(empty.set_model_type(empty.prop.root_id(), ElementModelType::Model));
    }

    #[test]
    fn prop_fields_read_and_write() {
        let (mut editor, _, _) = editor_with_lights(1);
        assert_eq!(editor.prop_field(PropField::Name), "Star {1}");
        assert!(editor.set_prop_field(PropField::Type, "Christmas"));
        assert!(!editor.set_prop_field(PropField::Type, "Christmas"));
        assert!(editor.set_prop_field(PropField::Notes, "hello"));
        assert!(!editor.set_prop_field(PropField::ColorMode, "nonsense"));
        assert_eq!(editor.prop.prop_type, "Christmas");
        assert_eq!(editor.undo_name(), Some("Change Prop Info"));
    }

    #[test]
    fn scaling_the_background_moves_lights_when_asked() {
        let (mut editor, _, nodes) = editor_with_lights(1);
        assert!(editor.scale_background(0, 10, true).is_err());
        assert!(editor.scale_background(100_001, 10, true).is_err());
        editor.scale_background(1600, 300, true).unwrap();
        let light = &editor.prop.element(nodes[0]).unwrap().lights()[0];
        assert_eq!((light.x, light.y), (20.0, 5.0));
        assert_eq!((editor.prop.width, editor.prop.height), (1600.0, 300.0));
        editor.scale_background(800, 600, false).unwrap();
        let light = &editor.prop.element(nodes[0]).unwrap().lights()[0];
        assert_eq!((light.x, light.y), (20.0, 5.0));
    }
}
