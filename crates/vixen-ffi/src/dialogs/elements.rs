//! Element property setup and the setup helpers.

use super::filters::DialogCurve;
use super::{find_nodes, no_such, parse_id, DialogContext};
use crate::core::FfiError;
use parking_lot::Mutex;
use std::sync::Arc;
use uuid::Uuid;
use vixen_core::element::{property_ids, ElementColorType, PropertyData};
use vixen_core::Color;
use vixen_session::dialogs::color as color_logic;
use vixen_session::dialogs::dimming as dimming_logic;
use vixen_session::dialogs::face as face_logic;
use vixen_session::dialogs::order as order_logic;
use vixen_session::dialogs::properties as logic;
use vixen_session::dialogs::world::{element_count, property_data, NodeKey};

#[derive(uniffi::Enum, Copy, Clone, PartialEq, Eq, Debug)]
pub enum DialogPropertyKind {
    Color,
    Location,
    Orientation,
    Order,
    Grid,
    Face,
    State,
    IntelligentFixture,
    None,
}

impl From<logic::PropertySetupKind> for DialogPropertyKind {
    fn from(k: logic::PropertySetupKind) -> Self {
        use logic::PropertySetupKind as K;
        match k {
            K::Color => DialogPropertyKind::Color,
            K::Location => DialogPropertyKind::Location,
            K::Orientation => DialogPropertyKind::Orientation,
            K::Order => DialogPropertyKind::Order,
            K::Grid => DialogPropertyKind::Grid,
            K::Face => DialogPropertyKind::Face,
            K::State => DialogPropertyKind::State,
            K::IntelligentFixture => DialogPropertyKind::IntelligentFixture,
            K::None => DialogPropertyKind::None,
        }
    }
}

/// The setup helpers, in the combo's order (F4).
#[derive(uniffi::Enum, Copy, Clone, PartialEq, Eq, Debug)]
pub enum DialogSetupHelper {
    ColorHandling,
    DimmingCurve,
    FaceMapping,
    Orientation,
    PatchingOrder,
    StateMapping,
}

#[derive(uniffi::Record, Clone, Copy, PartialEq, Eq)]
pub struct DialogLocation {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(uniffi::Record, Clone, Copy, PartialEq, Eq)]
pub struct DialogGrid {
    pub width: i32,
    pub height: i32,
    /// The number of elements under the node, which `width * height` has to match.
    pub required: i32,
}

#[derive(uniffi::Enum, Copy, Clone, PartialEq, Eq, Debug)]
pub enum DialogColorType {
    SingleColor,
    MultipleDiscreteColors,
    FullColor,
}

impl From<ElementColorType> for DialogColorType {
    fn from(t: ElementColorType) -> Self {
        match t {
            ElementColorType::SingleColor => DialogColorType::SingleColor,
            ElementColorType::MultipleDiscreteColors => DialogColorType::MultipleDiscreteColors,
            ElementColorType::FullColor => DialogColorType::FullColor,
        }
    }
}

impl From<DialogColorType> for ElementColorType {
    fn from(t: DialogColorType) -> Self {
        match t {
            DialogColorType::SingleColor => ElementColorType::SingleColor,
            DialogColorType::MultipleDiscreteColors => ElementColorType::MultipleDiscreteColors,
            DialogColorType::FullColor => ElementColorType::FullColor,
        }
    }
}

/// What the Color Configuration sheet opens with.
#[derive(uniffi::Record, Clone)]
pub struct DialogColorHelper {
    pub set_names: Vec<String>,
    /// `None` starts with no radio selected, which keeps OK off.
    pub preset_type: Option<DialogColorType>,
    pub single_color: u32,
    pub set_name: Option<String>,
    pub color_orders: Vec<String>,
    pub color_order: String,
}

#[derive(uniffi::Record, Clone)]
pub struct DialogColorRequest {
    pub color_type: DialogColorType,
    pub single_color: u32,
    pub set_name: Option<String>,
    pub color_order: String,
}

impl From<&DialogColorRequest> for color_logic::ColorRequest {
    fn from(r: &DialogColorRequest) -> Self {
        color_logic::ColorRequest {
            color_type: r.color_type.into(),
            single_color: Color::from_argb_u32(r.single_color),
            set_name: r.set_name.clone(),
            color_order: r.color_order.clone(),
        }
    }
}

/// Which of F159's and F160's questions apply.
#[derive(uniffi::Record, Clone, Copy)]
pub struct DialogColorPreflight {
    pub has_existing_properties: bool,
    pub has_existing_filters: bool,
}

#[derive(uniffi::Record, Clone, PartialEq, Eq)]
pub struct DialogColorSet {
    pub name: String,
    pub colors: Vec<u32>,
}

#[derive(uniffi::Enum, Copy, Clone, PartialEq, Eq, Debug)]
pub enum DialogExistingCurve {
    DoNothing,
    UpdateExisting,
    AddInsertAfterElement,
    AddNew,
}

impl From<DialogExistingCurve> for dimming_logic::ExistingBehaviour {
    fn from(b: DialogExistingCurve) -> Self {
        match b {
            DialogExistingCurve::DoNothing => dimming_logic::ExistingBehaviour::DoNothing,
            DialogExistingCurve::UpdateExisting => dimming_logic::ExistingBehaviour::UpdateExisting,
            DialogExistingCurve::AddInsertAfterElement => dimming_logic::ExistingBehaviour::AddInsertAfterElement,
            DialogExistingCurve::AddNew => dimming_logic::ExistingBehaviour::AddNew,
        }
    }
}

#[derive(uniffi::Record, Clone, PartialEq, Eq)]
pub struct DialogFaceRow {
    pub node: String,
    pub name: String,
    pub phonemes: Vec<bool>,
    pub components: Vec<bool>,
    pub color: u32,
}

impl From<&face_logic::FaceRow> for DialogFaceRow {
    fn from(r: &face_logic::FaceRow) -> Self {
        DialogFaceRow {
            node: r.node.to_string(),
            name: r.name.clone(),
            phonemes: r.phonemes.clone(),
            components: r.components.clone(),
            color: r.color.to_argb_u32(),
        }
    }
}

#[derive(uniffi::Record, Clone)]
pub struct DialogFaceColumn {
    pub key: String,
    pub title: String,
}

/// F171's list. It stays in Rust, where Reverse and Zig Zag work on the whole array.
#[derive(uniffi::Object)]
pub struct OrderSession {
    context: Arc<DialogContext>,
    rows: Mutex<Vec<order_logic::OrderRow>>,
}

#[derive(uniffi::Record, Clone)]
pub struct DialogOrderBounds {
    pub default_value: i32,
    pub minimum: i32,
    pub maximum: i32,
}

#[uniffi::export]
impl OrderSession {
    /// The list's names, in order. A row's number is its position plus one.
    pub fn names(&self) -> Vec<String> {
        self.rows.lock().iter().map(|r| r.name.clone()).collect()
    }

    /// F172.
    pub fn move_rows(&self, selected: Vec<u32>, to: u32) {
        let selected: Vec<usize> = selected.iter().map(|i| *i as usize).collect();
        order_logic::order_move(&mut self.rows.lock(), &selected, to as usize);
    }

    /// F175.
    pub fn reverse(&self, selected: Vec<u32>) {
        let selected: Vec<usize> = selected.iter().map(|i| *i as usize).collect();
        order_logic::order_reverse(&mut self.rows.lock(), &selected);
    }

    /// F176: the prompt's numbers.
    pub fn zigzag_bounds(&self, selected_count: u32) -> DialogOrderBounds {
        let (default_value, minimum, maximum) = order_logic::order_zigzag_bounds(selected_count as usize);
        DialogOrderBounds { default_value, minimum, maximum }
    }

    /// F176. The result is the alert's text.
    pub fn zigzag(&self, selected: Vec<u32>, every: i32) -> Option<String> {
        let selected: Vec<usize> = selected.iter().map(|i| *i as usize).collect();
        order_logic::order_zigzag(&mut self.rows.lock(), &selected, every).err()
    }

    /// F177.
    pub fn apply(&self) {
        let rows = self.rows.lock().clone();
        self.context.world(|world| order_logic::order_helper_apply(world, &rows));
    }
}

fn argb_list(colors: &[Color]) -> Vec<u32> {
    colors.iter().map(|c| c.to_argb_u32()).collect()
}

fn colors_from(argb: &[u32]) -> Vec<Color> {
    argb.iter().map(|c| Color::from_argb_u32(*c)).collect()
}

#[uniffi::export]
impl DialogContext {
    /// F3: where a property's Configure goes.
    pub fn property_kind(&self, type_id: String) -> Result<DialogPropertyKind, FfiError> {
        Ok(logic::property_setup_kind(&parse_id(&type_id)?).into())
    }

    /// F4.
    pub fn setup_helper_names(&self) -> Vec<String> {
        logic::SETUP_HELPERS.iter().map(|s| s.to_string()).collect()
    }

    // ---- Location ------------------------------------------------------------------------------------------------------------

    /// F181, F182.
    pub fn location(&self, node: String) -> Result<DialogLocation, FfiError> {
        let id = parse_id(&node)?;
        self.world(|world| {
            let node = world.find_node(&id).ok_or_else(|| no_such("element"))?;
            let (x, y, z) = logic::location_values(property_data(world, node, &property_ids::LOCATION));
            Ok(DialogLocation { x, y, z })
        })
    }

    /// F3, F182. `selection` is every selected element, which take a copy if they have the property.
    pub fn apply_location(&self, node: String, selection: Vec<String>, location: DialogLocation) -> Result<(), FfiError> {
        let data = PropertyData::Location {
            x: logic::clamp_location(location.x as i64),
            y: logic::clamp_location(location.y as i64),
            z: logic::clamp_location(location.z as i64),
        };
        self.apply_to_selection(&node, &selection, &property_ids::LOCATION, &data)
    }

    // ---- Orientation ---------------------------------------------------------------------------------------------------------

    /// F179: `Horizontal` or `Vertical`, in the combo's order.
    pub fn orientations(&self) -> Vec<String> {
        logic::ORIENTATIONS.iter().map(|s| s.to_string()).collect()
    }

    /// F179, for a property's own Configure.
    pub fn orientation(&self, node: String) -> Result<String, FfiError> {
        let id = parse_id(&node)?;
        self.world(|world| {
            let node = world.find_node(&id).ok_or_else(|| no_such("element"))?;
            Ok(match property_data(world, node, &property_ids::ORIENTATION) {
                Some(PropertyData::Orientation(o)) => logic::orientation_name(*o).to_string(),
                _ => "Vertical".to_string(),
            })
        })
    }

    /// F3.
    pub fn apply_orientation(&self, node: String, selection: Vec<String>, value: String) -> Result<(), FfiError> {
        let data = PropertyData::Orientation(logic::orientation_from_name(&value));
        self.apply_to_selection(&node, &selection, &property_ids::ORIENTATION, &data)
    }

    /// F180: the helper sets or adds the property on the selected nodes themselves.
    pub fn apply_orientation_helper(&self, nodes: Vec<String>, value: String) {
        self.world(|world| {
            let nodes = find_nodes(world, &nodes);
            logic::orientation_helper_apply(world, &nodes, logic::orientation_from_name(&value));
        });
    }

    // ---- Order ---------------------------------------------------------------------------------------------------------------

    /// F178.
    pub fn order_value(&self, node: String) -> Result<i32, FfiError> {
        let id = parse_id(&node)?;
        self.world(|world| {
            let node = world.find_node(&id).ok_or_else(|| no_such("element"))?;
            Ok(match property_data(world, node, &property_ids::ORDER) {
                Some(PropertyData::Order { order }) => *order,
                _ => 0,
            })
        })
    }

    /// F3, F178.
    pub fn apply_order_value(&self, node: String, selection: Vec<String>, value: i32) -> Result<(), FfiError> {
        let data = PropertyData::Order { order: value.max(0) };
        self.apply_to_selection(&node, &selection, &property_ids::ORDER, &data)
    }

    /// F170, F171.
    pub fn open_order_helper(self: Arc<Self>, nodes: Vec<String>) -> Arc<OrderSession> {
        let rows = self.world(|world| {
            let nodes = find_nodes(world, &nodes);
            order_logic::order_helper_rows(world, &nodes)
        });
        Arc::new(OrderSession { context: self, rows: Mutex::new(rows) })
    }

    // ---- Grid ----------------------------------------------------------------------------------------------------------------

    /// F183.
    pub fn grid(&self, node: String) -> Result<DialogGrid, FfiError> {
        let id = parse_id(&node)?;
        self.world(|world| {
            let node = world.find_node(&id).ok_or_else(|| no_such("element"))?;
            let required = element_count(world, node) as i32;
            let (width, height) = logic::grid_values(property_data(world, node, &property_ids::GRID), required);
            Ok(DialogGrid { width, height, required })
        })
    }

    /// F183's red message. `None` means OK is available.
    pub fn grid_check(&self, width: i32, height: i32, required: i32) -> Option<String> {
        logic::grid_check(width, height, required)
    }

    /// F3, F183.
    pub fn apply_grid(&self, node: String, selection: Vec<String>, width: i32, height: i32) -> Result<(), FfiError> {
        let id = parse_id(&node)?;
        self.world(|world| {
            let node = world.find_node(&id).ok_or_else(|| no_such("element"))?;
            let selected = find_nodes(world, &selection);
            logic::grid_apply(world, node, &selected, width, height);
            Ok(())
        })
    }

    // ---- Color helper --------------------------------------------------------------------------------------------------------

    /// F151 to F158. `preset` is the node whose Color property the sheet starts from, for a property's Configure (F3).
    pub fn color_helper(&self, preset: Option<String>) -> DialogColorHelper {
        self.world(|world| {
            let data = preset
                .and_then(|id| Uuid::parse_str(&id).ok())
                .and_then(|id| world.find_node(&id))
                .and_then(|node| property_data(world, node, &property_ids::COLOR).cloned());
            let m = color_logic::color_helper_model(world, data.as_ref());
            DialogColorHelper {
                set_names: m.set_names,
                preset_type: m.preset_type.map(Into::into),
                single_color: m.single_color.to_argb_u32(),
                set_name: m.set_name,
                color_orders: color_logic::COLOR_ORDERS.iter().map(|o| o.to_string()).collect(),
                color_order: m.color_order,
            }
        })
    }

    /// F159, F160: which questions apply.
    pub fn color_helper_preflight(&self, nodes: Vec<String>) -> DialogColorPreflight {
        self.world(|world| {
            let nodes = find_nodes(world, &nodes);
            let p = color_logic::color_helper_preflight(world, &nodes);
            DialogColorPreflight { has_existing_properties: p.has_existing_properties, has_existing_filters: p.has_existing_filters }
        })
    }

    /// F159 to F161. Takes a moment over a whole show, so call it off the main thread. Returns the report's text.
    pub fn apply_color_helper(
        &self,
        nodes: Vec<String>,
        request: DialogColorRequest,
        overwrite_properties: bool,
        overwrite_filters: bool,
    ) -> String {
        self.world(|world| {
            let nodes = find_nodes(world, &nodes);
            let report = color_logic::color_helper_apply(world, &nodes, &(&request).into(), overwrite_properties, overwrite_filters);
            color_logic::color_report_text(&report)
        })
    }

    // ---- Colour sets ---------------------------------------------------------------------------------------------------------

    /// F162.
    pub fn color_sets(&self) -> Vec<DialogColorSet> {
        self.world(|world| {
            color_logic::ordered_color_sets(world)
                .into_iter()
                .map(|(name, colors)| DialogColorSet { name, colors: argb_list(&colors) })
                .collect()
        })
    }

    /// F164. The result is the alert's text.
    pub fn add_color_set(&self, name: String) -> Option<String> {
        self.world(|world| color_logic::color_set_add(world, &name).err())
    }

    /// F165. The result is the alert's text.
    pub fn remove_color_set(&self, name: String) -> Option<String> {
        self.world(|world| color_logic::color_set_remove(world, &name).err())
    }

    /// F168. The result is the alert's text.
    pub fn save_color_set(&self, name: String, colors: Vec<u32>) -> Option<String> {
        self.world(|world| color_logic::color_set_save(world, &name, &colors_from(&colors)).err())
    }

    /// F169.
    pub fn color_set_differs(&self, name: String, colors: Vec<u32>) -> bool {
        self.world(|world| color_logic::color_set_differs(world, &name, &colors_from(&colors)))
    }

    /// F166.
    pub fn color_set_button_title(&self, name: String, current: String) -> String {
        self.world(|world| color_logic::color_set_button_title(world, &name, &current))
    }

    // ---- Dimming Curve helper --------------------------------------------------------------------------------------------------

    /// F244. Returns the report's text.
    pub fn apply_dimming_helper(&self, nodes: Vec<String>, curve: DialogCurve, existing: DialogExistingCurve) -> String {
        self.world(|world| {
            let nodes = find_nodes(world, &nodes);
            let report = dimming_logic::dimming_helper_apply(world, &nodes, &(&curve).into(), existing.into());
            dimming_logic::dimming_report_text(&report)
        })
    }

    /// F243.
    pub fn patching_help_url(&self) -> String {
        dimming_logic::PATCHING_HELP_URL.to_string()
    }

    // ---- Face ----------------------------------------------------------------------------------------------------------------

    /// F185.
    pub fn face_phonemes(&self) -> Vec<String> {
        face_logic::PHONEMES.iter().map(|s| s.to_string()).collect()
    }

    /// F186.
    pub fn face_components(&self) -> Vec<DialogFaceColumn> {
        face_logic::COMPONENTS
            .iter()
            .map(|(key, title)| DialogFaceColumn { key: key.to_string(), title: title.to_string() })
            .collect()
    }

    /// F184 to F186.
    pub fn face_rows(&self, nodes: Vec<String>) -> Vec<DialogFaceRow> {
        self.world(|world| {
            let nodes = find_nodes(world, &nodes);
            face_logic::face_helper_model(world, &nodes).iter().map(Into::into).collect()
        })
    }

    /// F187: the new value for every selected checkbox cell.
    pub fn face_toggle(&self, cells: Vec<bool>) -> bool {
        face_logic::face_toggle(&cells)
    }

    /// F188: empty means the full colour picker.
    pub fn face_color_choices(&self, nodes: Vec<String>) -> Vec<u32> {
        self.world(|world| {
            let nodes = find_nodes(world, &nodes);
            argb_list(&face_logic::face_color_choices(world, &nodes))
        })
    }

    /// F190.
    pub fn apply_face(&self, rows: Vec<DialogFaceRow>) {
        let rows: Vec<face_logic::FaceRow> = rows
            .iter()
            .filter_map(|r| {
                Some(face_logic::FaceRow {
                    node: Uuid::parse_str(&r.node).ok()?,
                    name: r.name.clone(),
                    phonemes: r.phonemes.clone(),
                    components: r.components.clone(),
                    color: Color::from_argb_u32(r.color),
                })
            })
            .collect();
        self.world(|world| face_logic::face_helper_apply(world, &rows));
    }
}

impl DialogContext {
    fn apply_to_selection(&self, node: &str, selection: &[String], type_id: &Uuid, data: &PropertyData) -> Result<(), FfiError> {
        let id = parse_id(node)?;
        self.world(|world| {
            let node: NodeKey = world.find_node(&id).ok_or_else(|| no_such("element"))?;
            let selected = find_nodes(world, selection);
            logic::apply_to_selection(world, node, &selected, type_id, data);
            Ok(())
        })
    }
}
