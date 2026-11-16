//! The Custom Prop Editor's API: one object per window, holding that window's prop and its editor state.

mod states;
mod preferences;
mod scale;
mod wire;
mod polygon;
mod xmodel;
mod vendor;

use crate::core::FfiError;
use parking_lot::Mutex;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;
use vixen_prop::{
    cleanse_name, new_prop_name, parse_path, path_text, AdornerHandle, Axis, CanvasMode, Edge, ElementModelType, FaceComponent, Modifiers, PanelTab,
    PropClip, PropEditor, PropEditorSettings, PropError, PropField, RowPath, SizeChange,
};

impl From<PropError> for FfiError {
    fn from(e: PropError) -> FfiError {
        FfiError::new(if e.detail.is_empty() { e.message } else { format!("{}\n{}", e.message, e.detail) })
    }
}

#[derive(uniffi::Enum, Copy, Clone, PartialEq, Eq)]
pub enum PropEdMode {
    Draw,
    Select,
}

#[derive(uniffi::Enum, Copy, Clone, PartialEq, Eq)]
pub enum PropEdTab {
    Layout,
    Order,
    State,
}

#[derive(uniffi::Enum, Copy, Clone, PartialEq, Eq)]
pub enum PropEdEdge {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(uniffi::Enum, Copy, Clone, PartialEq, Eq)]
pub enum PropEdAxis {
    Horizontal,
    Vertical,
}

#[derive(uniffi::Enum, Copy, Clone, PartialEq, Eq)]
pub enum PropEdSizeChange {
    Increase,
    Decrease,
    Match,
}

#[derive(uniffi::Enum, Copy, Clone, PartialEq, Eq)]
pub enum PropEdHandle {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
    Center,
    Rotate,
}

#[derive(uniffi::Enum, Copy, Clone, PartialEq, Eq)]
pub enum PropEdField {
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

fn handle_out(handle: AdornerHandle) -> PropEdHandle {
    match handle {
        AdornerHandle::TopLeft => PropEdHandle::TopLeft,
        AdornerHandle::Top => PropEdHandle::Top,
        AdornerHandle::TopRight => PropEdHandle::TopRight,
        AdornerHandle::Right => PropEdHandle::Right,
        AdornerHandle::BottomRight => PropEdHandle::BottomRight,
        AdornerHandle::Bottom => PropEdHandle::Bottom,
        AdornerHandle::BottomLeft => PropEdHandle::BottomLeft,
        AdornerHandle::Left => PropEdHandle::Left,
        AdornerHandle::Center => PropEdHandle::Center,
        AdornerHandle::Rotate => PropEdHandle::Rotate,
    }
}

fn field_in(field: PropEdField) -> PropField {
    match field {
        PropEdField::Name => PropField::Name,
        PropEdField::Type => PropField::Type,
        PropEdField::CreatedBy => PropField::CreatedBy,
        PropEdField::PhysicalHeight => PropField::PhysicalHeight,
        PropEdField::PhysicalWidth => PropField::PhysicalWidth,
        PropEdField::PhysicalDepth => PropField::PhysicalDepth,
        PropEdField::Material => PropField::Material,
        PropEdField::NodeCount => PropField::NodeCount,
        PropEdField::BulbType => PropField::BulbType,
        PropEdField::ColorMode => PropField::ColorMode,
        PropEdField::VendorName => PropField::VendorName,
        PropEdField::VendorWebsite => PropField::VendorWebsite,
        PropEdField::VendorContact => PropField::VendorContact,
        PropEdField::VendorEmail => PropField::VendorEmail,
        PropEdField::VendorPhone => PropField::VendorPhone,
        PropEdField::Notes => PropField::Notes,
    }
}

/// Everything the window's chrome shows, read after each change.
#[derive(uniffi::Record, Clone)]
pub struct PropEdState {
    pub revision: u64,
    pub dirty: bool,
    pub file_path: Option<String>,
    pub prop_name: String,
    /// The prop's name with what a file name can't hold taken out: the Save As suggestion and the close prompt's name.
    pub file_name: String,
    pub mode: PropEdMode,
    pub tab: PropEdTab,
    pub canvas_width: f64,
    pub canvas_height: f64,
    pub opacity: f64,
    /// Changes when the background picture does, so the window decodes it only then.
    pub image_token: u64,
    pub undo_name: Option<String>,
    pub redo_name: Option<String>,
    pub status: String,
    pub pointer_text: String,
    pub light_color: u32,
    pub selected_light_color: u32,
    pub creation_date: String,
    pub modified_date: String,
    pub has_light_selection: bool,
    pub can_group: bool,
    pub can_create_empty_group: bool,
    pub can_copy: bool,
    pub can_delete_elements: bool,
    pub can_increase_light_size: bool,
    pub can_decrease_light_size: bool,
    pub can_match_light_size: bool,
    pub tree_selection_count: u32,
    pub order_selection_count: u32,
}

#[derive(uniffi::Record, Clone)]
pub struct PropEdTreeRow {
    /// The ids from the root down to this row, since one element can sit under several parents.
    pub path: String,
    pub id: String,
    pub name: String,
    pub depth: u32,
    pub is_root: bool,
    pub is_group: bool,
    pub child_count: u32,
    pub light_count: u32,
}

#[derive(uniffi::Record, Clone)]
pub struct PropEdOrderRow {
    pub id: String,
    pub order: i32,
    pub name: String,
}

#[derive(uniffi::Record, Clone)]
pub struct PropEdLight {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub selected: bool,
    pub state_color: Option<u32>,
}

#[derive(uniffi::Record, Clone)]
pub struct PropEdHandlePoint {
    pub handle: PropEdHandle,
    pub x: f64,
    pub y: f64,
}

/// The box around the selected lights, in canvas units. `angle` is in degrees about the centre.
#[derive(uniffi::Record, Clone)]
pub struct PropEdAdorner {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub center_x: f64,
    pub center_y: f64,
    pub angle: f64,
    pub handle_size: f64,
    pub handles: Vec<PropEdHandlePoint>,
}

#[derive(uniffi::Record, Clone)]
pub struct PropEdRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(uniffi::Record, Clone)]
pub struct PropEdCanvas {
    pub lights: Vec<PropEdLight>,
    pub adorner: Option<PropEdAdorner>,
    pub band: Option<PropEdRect>,
}

#[derive(uniffi::Record, Clone)]
pub struct PropEdElementInfo {
    pub id: String,
    pub name: String,
    pub face_component: String,
    pub face_color: String,
    pub model_type: String,
    pub children: u32,
    pub lights: u32,
    pub light_size: String,
    pub error: Option<String>,
}

#[derive(uniffi::Record, Clone)]
pub struct PropEdChoice {
    pub value: String,
    pub label: String,
}

#[derive(uniffi::Record, Clone)]
pub struct PropEdSaved {
    /// False when the prop has no file yet, so the window must ask where to save.
    pub saved: bool,
    pub legacy_backup: Option<String>,
}

fn now_text() -> String {
    let now = vixen_io::datetime::LocalDateTime::now();
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}", now.year, now.month, now.day, now.hour, now.minute, now.second, now.millisecond)
}

fn settings() -> PropEditorSettings {
    PropEditorSettings { user_name: std::env::var("USER").unwrap_or_default(), ..PropEditorSettings::default() }
}

fn ids(texts: &[String]) -> Vec<Uuid> {
    texts.iter().filter_map(|t| Uuid::parse_str(t).ok()).collect()
}

fn paths(texts: &[String]) -> Vec<RowPath> {
    texts.iter().filter_map(|t| parse_path(t)).collect()
}

fn id(text: &str) -> Result<Uuid, FfiError> {
    Uuid::parse_str(text).map_err(|_| FfiError::new("That element no longer exists."))
}

fn clip(json: &str) -> Option<PropClip> {
    PropClip::from_json(json).ok()
}

fn modifiers(command: bool, shift: bool) -> Modifiers {
    Modifiers { command, shift }
}

/// `NewProp`'s name rule: what New's prompt turns the typed text into.
#[uniffi::export]
pub fn prop_ed_new_name(typed: String) -> String {
    new_prop_name(&typed)
}

#[uniffi::export]
pub fn prop_ed_face_components() -> Vec<PropEdChoice> {
    FaceComponent::ALL.iter().map(|c| PropEdChoice { value: c.name().to_string(), label: c.display_name().to_string() }).collect()
}

#[uniffi::export]
pub fn prop_ed_model_types() -> Vec<PropEdChoice> {
    ElementModelType::ALL.iter().map(|c| PropEdChoice { value: c.name().to_string(), label: c.display_name().to_string() }).collect()
}

#[uniffi::export]
pub fn prop_ed_color_modes() -> Vec<PropEdChoice> {
    vixen_prop::ColorMode::ALL.iter().map(|c| PropEdChoice { value: c.name().to_string(), label: c.display_name().to_string() }).collect()
}

/// One Custom Prop Editor window's document.
#[derive(uniffi::Object)]
pub struct CustomPropEditor {
    inner: Mutex<PropEditor>,
}

#[uniffi::export]
impl CustomPropEditor {
    /// A new prop called `name`, already run through the New prompt's name rule by the caller when it came from there.
    #[uniffi::constructor]
    pub fn new(name: String) -> Arc<CustomPropEditor> {
        Arc::new(CustomPropEditor { inner: Mutex::new(PropEditor::new_document(&name, settings(), &now_text())) })
    }

    #[uniffi::constructor]
    pub fn open(path: String) -> Result<Arc<CustomPropEditor>, FfiError> {
        let editor = PropEditor::open(Path::new(&path), settings(), &now_text())?;
        Ok(Arc::new(CustomPropEditor { inner: Mutex::new(editor) }))
    }

    pub fn state(&self) -> PropEdState {
        let e = self.inner.lock();
        let prop = e.prop();
        PropEdState {
            revision: e.revision(),
            dirty: e.is_dirty(),
            file_path: e.file_path().map(|p| p.to_string_lossy().into_owned()),
            prop_name: prop.name().to_string(),
            file_name: cleanse_name(prop.name()),
            mode: match e.mode() {
                CanvasMode::Draw => PropEdMode::Draw,
                CanvasMode::Select => PropEdMode::Select,
            },
            tab: match e.tab() {
                PanelTab::Layout => PropEdTab::Layout,
                PanelTab::Order => PropEdTab::Order,
                PanelTab::State => PropEdTab::State,
            },
            canvas_width: prop.width,
            canvas_height: prop.height,
            opacity: prop.opacity,
            image_token: Arc::as_ptr(&prop.image) as usize as u64,
            undo_name: e.undo_name().map(str::to_string),
            redo_name: e.redo_name().map(str::to_string),
            status: e.status().to_string(),
            pointer_text: e.pointer_text(),
            light_color: e.settings().light_color.to_argb_u32(),
            selected_light_color: e.settings().selected_light_color.to_argb_u32(),
            creation_date: prop.creation_date.clone(),
            modified_date: prop.modified_date.clone(),
            has_light_selection: e.has_light_selection(),
            can_group: e.can_group(),
            can_create_empty_group: e.can_create_empty_group(),
            can_copy: e.can_copy(),
            can_delete_elements: e.can_delete_elements(),
            can_increase_light_size: e.can_change_light_size(SizeChange::Increase),
            can_decrease_light_size: e.can_change_light_size(SizeChange::Decrease),
            can_match_light_size: e.can_change_light_size(SizeChange::Match),
            tree_selection_count: e.tree_selection().len() as u32,
            order_selection_count: e.order_selection().len() as u32,
        }
    }

    pub fn save(&self) -> Result<PropEdSaved, FfiError> {
        let outcome = self.inner.lock().save(&now_text())?;
        Ok(match outcome {
            Some(o) => PropEdSaved { saved: true, legacy_backup: o.legacy_backup.map(|p| p.to_string_lossy().into_owned()) },
            None => PropEdSaved { saved: false, legacy_backup: None },
        })
    }

    pub fn save_as(&self, path: String) -> Result<PropEdSaved, FfiError> {
        let outcome = self.inner.lock().save_as(Path::new(&path), &now_text())?;
        Ok(PropEdSaved { saved: true, legacy_backup: outcome.legacy_backup.map(|p| p.to_string_lossy().into_owned()) })
    }

    pub fn undo(&self) -> bool {
        self.inner.lock().undo()
    }

    pub fn redo(&self) -> bool {
        self.inner.lock().redo()
    }

    pub fn set_status(&self, text: String) {
        self.inner.lock().set_status(&text);
    }

    pub fn tree_rows(&self) -> Vec<PropEdTreeRow> {
        self.inner
            .lock()
            .tree_rows()
            .into_iter()
            .map(|r| PropEdTreeRow {
                path: path_text(&r.path),
                id: r.id.simple().to_string(),
                name: r.name,
                depth: r.depth as u32,
                is_root: r.is_root,
                is_group: r.is_group,
                child_count: r.child_count as u32,
                light_count: r.light_count as u32,
            })
            .collect()
    }

    /// The rows a command wants opened, handed over once.
    pub fn take_expand_requests(&self) -> Vec<String> {
        self.inner.lock().take_expand_requests().iter().map(|p| path_text(p)).collect()
    }

    pub fn tree_selection(&self) -> Vec<String> {
        self.inner.lock().tree_selection().iter().map(|p| path_text(p)).collect()
    }

    pub fn order_selection(&self) -> Vec<String> {
        self.inner.lock().order_selection().iter().map(|i| i.simple().to_string()).collect()
    }

    pub fn set_tree_selection(&self, paths_text: Vec<String>) {
        self.inner.lock().set_tree_selection(paths(&paths_text));
    }

    pub fn set_order_selection(&self, ids_text: Vec<String>) {
        self.inner.lock().set_order_selection(ids(&ids_text));
    }

    pub fn set_tab(&self, tab: PropEdTab) {
        self.inner.lock().set_tab(match tab {
            PropEdTab::Layout => PanelTab::Layout,
            PropEdTab::Order => PanelTab::Order,
            PropEdTab::State => PanelTab::State,
        });
    }

    pub fn set_mode(&self, mode: PropEdMode) {
        self.inner.lock().set_mode(match mode {
            PropEdMode::Draw => CanvasMode::Draw,
            PropEdMode::Select => CanvasMode::Select,
        });
    }

    pub fn copy_to_new_group(&self, name: String) -> bool {
        self.inner.lock().copy_to_new_group(&name)
    }

    pub fn move_to_new_group(&self, name: String) -> bool {
        self.inner.lock().move_to_new_group(&name)
    }

    pub fn suggested_empty_group_name(&self) -> String {
        self.inner.lock().suggested_empty_group_name()
    }

    pub fn create_empty_group(&self, name: String) -> bool {
        self.inner.lock().create_empty_group(&name)
    }

    /// The copied rows as the text the pasteboard holds, so a paste works in another window.
    pub fn copy(&self) -> Option<String> {
        self.inner.lock().copy().map(|c| c.to_json())
    }

    pub fn cut(&self) -> Option<String> {
        self.inner.lock().cut().map(|c| c.to_json())
    }

    pub fn can_paste(&self, clip_json: String) -> bool {
        clip(&clip_json).is_some_and(|c| self.inner.lock().can_paste(&c))
    }

    pub fn can_paste_as_new(&self, clip_json: String) -> bool {
        clip(&clip_json).is_some_and(|c| self.inner.lock().can_paste_as_new(&c))
    }

    pub fn paste(&self, clip_json: String) -> bool {
        clip(&clip_json).is_some_and(|c| self.inner.lock().paste(&c))
    }

    pub fn suggested_paste_name(&self) -> String {
        self.inner.lock().suggested_paste_name()
    }

    pub fn paste_as_new(&self, clip_json: String, name: String) -> bool {
        clip(&clip_json).is_some_and(|c| self.inner.lock().paste_as_new(&c, &name))
    }

    pub fn delete_selected_elements(&self) -> bool {
        self.inner.lock().delete_selected_elements()
    }

    /// The name the element ended up with, which differs from `name` when that one was taken.
    pub fn rename(&self, id_text: String, name: String) -> Result<Option<String>, FfiError> {
        Ok(self.inner.lock().rename(id(&id_text)?, &name))
    }

    pub fn rename_many(&self, ids_text: Vec<String>, names: Vec<String>) -> bool {
        self.inner.lock().rename_many(&ids(&ids_text), &names)
    }

    /// `parent` is the path of the row dropped on or between whose children; `index` is the child position, absent for a drop on the row.
    pub fn can_drop(&self, dragged: Vec<String>, parent: String, index: Option<u32>) -> bool {
        let Some(parent) = parse_path(&parent) else { return false };
        self.inner.lock().can_drop(&paths(&dragged), &parent, index.map(|i| i as usize))
    }

    pub fn drop_rows(&self, dragged: Vec<String>, parent: String, index: Option<u32>, reverse: bool) -> bool {
        let Some(parent) = parse_path(&parent) else { return false };
        self.inner.lock().drop_rows(&paths(&dragged), &parent, index.map(|i| i as usize), reverse)
    }

    pub fn order_rows(&self) -> Vec<PropEdOrderRow> {
        self.inner.lock().order_rows().into_iter().map(|r| PropEdOrderRow { id: r.id.simple().to_string(), order: r.order, name: r.name }).collect()
    }

    pub fn order_drop(&self, dragged: Vec<String>, index: u32) -> bool {
        self.inner.lock().order_drop(&ids(&dragged), index as usize)
    }

    pub fn order_reverse(&self) -> bool {
        self.inner.lock().order_reverse()
    }

    pub fn prop_field(&self, field: PropEdField) -> String {
        self.inner.lock().prop_field(field_in(field))
    }

    /// False when the text can't be that field's value, so the box should show the stored one again.
    pub fn set_prop_field(&self, field: PropEdField, value: String) -> bool {
        self.inner.lock().set_prop_field(field_in(field), &value)
    }

    pub fn element_info(&self) -> Option<PropEdElementInfo> {
        self.inner.lock().element_info().map(|i| PropEdElementInfo {
            id: i.id.simple().to_string(),
            name: i.name,
            face_component: i.face_component.name().to_string(),
            face_color: i.face_color,
            model_type: i.model_type.name().to_string(),
            children: i.children as u32,
            lights: i.lights as u32,
            light_size: i.light_size,
            error: i.error,
        })
    }

    pub fn set_element_name(&self, id_text: String, name: String) -> Result<bool, FfiError> {
        Ok(self.inner.lock().set_element_name(id(&id_text)?, &name))
    }

    pub fn set_face_component(&self, id_text: String, component: String) -> Result<bool, FfiError> {
        let component = FaceComponent::parse(&component).ok_or_else(|| FfiError::new("That face component isn't one of the choices."))?;
        Ok(self.inner.lock().set_face_component(id(&id_text)?, component))
    }

    pub fn set_face_color(&self, id_text: String, hex: String) -> Result<bool, FfiError> {
        Ok(self.inner.lock().set_face_color(id(&id_text)?, &hex))
    }

    pub fn set_model_type(&self, id_text: String, model_type: String) -> Result<bool, FfiError> {
        let model_type = ElementModelType::parse(&model_type).ok_or_else(|| FfiError::new("That model type isn't one of the choices."))?;
        Ok(self.inner.lock().set_model_type(id(&id_text)?, model_type))
    }

    pub fn assign_background(&self, path: String) -> Result<(), FfiError> {
        Ok(self.inner.lock().assign_background(Path::new(&path))?)
    }

    pub fn set_opacity(&self, opacity: f64) -> bool {
        self.inner.lock().set_opacity(opacity)
    }

    pub fn scale_background(&self, width: i32, height: i32, scale_lights: bool) -> Result<(), FfiError> {
        Ok(self.inner.lock().scale_background(width, height, scale_lights)?)
    }

    /// The background picture as the bytes of an image file.
    pub fn background_image(&self) -> Vec<u8> {
        self.inner.lock().prop().image.bytes.clone()
    }

    pub fn background_pixel_width(&self) -> u32 {
        self.inner.lock().prop().image.width
    }

    pub fn background_pixel_height(&self) -> u32 {
        self.inner.lock().prop().image.height
    }

    /// What the canvas draws, in one call so a drag costs one crossing per frame.
    pub fn canvas(&self) -> PropEdCanvas {
        let e = self.inner.lock();
        let colors = e.state_preview_colors();
        PropEdCanvas {
            lights: e.light_dots().into_iter().map(|d| PropEdLight { x: d.x as f32, y: d.y as f32, size: d.size as f32, selected: d.selected, state_color: colors.as_ref().map(|c| c.get(&d.owner).copied().unwrap_or(e.settings().state_preview_base_color).to_argb_u32()) }).collect(),
            adorner: e.adorner().map(|a| PropEdAdorner {
                x: a.x,
                y: a.y,
                width: a.width,
                height: a.height,
                center_x: a.center_x,
                center_y: a.center_y,
                angle: a.angle,
                handle_size: vixen_prop::canvas::HANDLE_SIZE,
                handles: a.handles.into_iter().map(|(h, x, y)| PropEdHandlePoint { handle: handle_out(h), x, y }).collect(),
            }),
            band: e.band().map(|(x, y, width, height)| PropEdRect { x, y, width, height }),
        }
    }

    pub fn pointer_moved(&self, x: f64, y: f64) {
        self.inner.lock().pointer_moved(x, y);
    }

    pub fn pointer_exited(&self) {
        self.inner.lock().pointer_exited();
    }

    /// Which handle of the selection box is under the pointer, for the cursor.
    pub fn handle_at(&self, x: f64, y: f64) -> Option<PropEdHandle> {
        self.inner.lock().handle_at(x, y).map(handle_out)
    }

    pub fn pointer_down(&self, x: f64, y: f64, command: bool, shift: bool) {
        self.inner.lock().pointer_down(x, y, modifiers(command, shift));
    }

    pub fn pointer_dragged(&self, x: f64, y: f64, command: bool, shift: bool) {
        self.inner.lock().pointer_dragged(x, y, modifiers(command, shift));
    }

    pub fn pointer_up(&self) {
        self.inner.lock().pointer_up();
    }

    pub fn align(&self, edge: PropEdEdge) -> bool {
        self.inner.lock().align(match edge {
            PropEdEdge::Left => Edge::Left,
            PropEdEdge::Right => Edge::Right,
            PropEdEdge::Top => Edge::Top,
            PropEdEdge::Bottom => Edge::Bottom,
        })
    }

    pub fn distribute(&self, axis: PropEdAxis) -> bool {
        self.inner.lock().distribute(axis_in(axis))
    }

    pub fn flip(&self, axis: PropEdAxis) -> bool {
        self.inner.lock().flip(axis_in(axis))
    }

    pub fn change_light_size(&self, change: PropEdSizeChange) -> bool {
        self.inner.lock().change_light_size(match change {
            PropEdSizeChange::Increase => SizeChange::Increase,
            PropEdSizeChange::Decrease => SizeChange::Decrease,
            PropEdSizeChange::Match => SizeChange::Match,
        })
    }

    pub fn delete_selected_lights(&self) -> bool {
        self.inner.lock().delete_selected_lights()
    }
}

fn axis_in(axis: PropEdAxis) -> Axis {
    match axis {
        PropEdAxis::Horizontal => Axis::Horizontal,
        PropEdAxis::Vertical => Axis::Vertical,
    }
}

#[uniffi::export]
pub fn prop_ed_bulb_types() -> Vec<String> {
    vixen_prop::BULB_TYPE_SUGGESTIONS.iter().map(|s| (*s).to_owned()).collect()
}
