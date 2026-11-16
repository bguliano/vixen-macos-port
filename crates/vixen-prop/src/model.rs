//! The prop document and its graph rules (`Model/*.cs`, `Services/PropModelServices.cs`).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use uuid::Uuid;
use vixen_core::Color;

/// `ElementModel.DefaultLightSize`.
pub const DEFAULT_LIGHT_SIZE: i32 = 3;
pub const DEFAULT_STATE_DEFINITION_NAME: &str = "State - 1";
pub const DEFAULT_STATE_ITEM_NAME: &str = "Item Name 1";

/// A failure with the short text a person sees and the detail for the log.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct PropError {
    pub message: String,
    pub detail: String,
}

impl PropError {
    pub fn new(message: &str, detail: impl Into<String>) -> PropError {
        PropError { message: message.to_string(), detail: detail.into() }
    }
}

macro_rules! named_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident => $display:expr),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
        pub enum $name { $($variant),+ }

        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            /// The C# member name, which is what files hold.
            pub fn name(self) -> &'static str {
                match self { $($name::$variant => stringify!($variant)),+ }
            }

            /// The `[Description]` text shown in pickers.
            pub fn display_name(self) -> &'static str {
                match self { $($name::$variant => $display),+ }
            }

            /// `Enum.TryParse` followed by `Enum.IsDefined`: a member name, or its number as text.
            pub fn parse(text: &str) -> Option<$name> {
                if let Some(found) = Self::ALL.iter().find(|v| v.name() == text) {
                    return Some(*found);
                }
                let index: usize = text.trim().parse().ok()?;
                Self::ALL.get(index).copied()
            }
        }
    };
}

named_enum!(ElementModelType {
    None => "None",
    Model => "Model",
    SubModel => "SubModel",
    FaceInfo => "FaceInfo",
    StateInfo => "StateInfo",
});

named_enum!(FaceComponent {
    None => "None",
    EyesClosed => "Eyes Closed",
    EyesOpen => "Eyes Open",
    EyesClosed2 => "Eyes Closed 2",
    EyesOpen2 => "Eyes Open 2",
    EyesClosed3 => "Eyes Closed 3",
    EyesOpen3 => "Eyes Open 3",
    Outlines => "Outline",
    Outlines2 => "Outline 2",
    AI => "AI",
    E => "E",
    FV => "FV",
    L => "L",
    MBP => "MBP",
    O => "O",
    U => "U",
    WQ => "WQ",
    ETC => "ETC",
    REST => "REST",
});

named_enum!(ColorMode {
    FullColor => "Full Color",
    Multiple => "Multiple Color",
    Single => "Single Color",
    Other => "Other",
});

named_enum!(
    /// What an element is, judged by how many lights it holds.
    ElementType {
    String => "Multiple Nodes",
    Node => "Single Node",
    Group => "Group",
});

/// `BulbTypeConverter`'s suggestions. The field takes any text.
pub const BULB_TYPE_SUGGESTIONS: &[&str] = &[
    "12mm Bullet",
    "12mm Square",
    "1x3 Rectangle",
    "PixaBulb",
    "Strip",
    "Generic Pixel",
    "M5",
    "M6",
    "C6",
    "C7",
    "C9",
    "G40",
];

#[derive(Clone, Debug, PartialEq)]
pub struct Light {
    pub id: Uuid,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub size: f64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FaceDefinition {
    pub component: FaceComponent,
    pub default_color: Color,
}

impl Default for FaceDefinition {
    fn default() -> Self {
        FaceDefinition { component: FaceComponent::None, default_color: Color::WHITE }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StateItemModel {
    pub id: Uuid,
    pub name: String,
    pub color: Color,
    pub element_ids: Vec<Uuid>,
}

impl StateItemModel {
    pub fn new(name: &str) -> StateItemModel {
        StateItemModel { id: Uuid::new_v4(), name: name.to_string(), color: Color::WHITE, element_ids: Vec::new() }
    }

    /// `StateItemModel.Normalize`.
    pub fn normalize(&mut self) {
        if self.id.is_nil() {
            self.id = Uuid::new_v4();
        }
        self.name = if self.name.trim().is_empty() { DEFAULT_STATE_ITEM_NAME.to_string() } else { self.name.trim().to_string() };
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StateDefinitionModel {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub items: Vec<StateItemModel>,
}

impl StateDefinitionModel {
    /// `StateDefinitionModel.CreateDefault`: one default item.
    pub fn create_default(name: &str) -> StateDefinitionModel {
        StateDefinitionModel {
            id: Uuid::new_v4(),
            name: name.to_string(),
            description: String::new(),
            items: vec![StateItemModel::new(DEFAULT_STATE_ITEM_NAME)],
        }
    }

    /// `StateDefinitionModel.Normalize`.
    pub fn normalize(&mut self) {
        if self.id.is_nil() {
            self.id = Uuid::new_v4();
        }
        self.name = if self.name.trim().is_empty() { DEFAULT_STATE_DEFINITION_NAME.to_string() } else { self.name.trim().to_string() };
        for item in &mut self.items {
            item.normalize();
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ElementModel {
    pub id: Uuid,
    pub state_property_id: Uuid,
    pub(crate) name: String,
    /// `_order`. Only a leaf reports it; see `Prop::order`.
    pub order: i32,
    /// `_lightSize`. A group reports its first leaf's instead; see `Prop::light_size`.
    pub(crate) light_size: i32,
    pub model_type: ElementModelType,
    pub face: FaceDefinition,
    pub state_definitions: Vec<StateDefinitionModel>,
    pub(crate) children: Vec<Uuid>,
    pub(crate) parents: Vec<Uuid>,
    pub(crate) lights: Vec<Light>,
}

impl ElementModel {
    pub fn new(name: &str) -> ElementModel {
        ElementModel {
            id: Uuid::new_v4(),
            state_property_id: Uuid::new_v4(),
            name: name.to_string(),
            order: 0,
            light_size: DEFAULT_LIGHT_SIZE,
            model_type: ElementModelType::None,
            face: FaceDefinition::default(),
            state_definitions: Vec::new(),
            children: Vec::new(),
            parents: Vec::new(),
            lights: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn children(&self) -> &[Uuid] {
        &self.children
    }

    pub fn parents(&self) -> &[Uuid] {
        &self.parents
    }

    pub fn lights(&self) -> &[Light] {
        &self.lights
    }

    pub fn raw_light_size(&self) -> i32 {
        self.light_size
    }

    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    pub fn is_light_node(&self) -> bool {
        self.children.is_empty() && !self.lights.is_empty()
    }

    /// No lights, so an empty leaf counts as a group.
    pub fn is_group_node(&self) -> bool {
        self.lights.is_empty()
    }

    pub fn is_root_node(&self) -> bool {
        self.parents.is_empty()
    }

    pub fn element_type(&self) -> ElementType {
        match self.lights.len() {
            0 => ElementType::Group,
            1 => ElementType::Node,
            _ => ElementType::String,
        }
    }

    /// `IsPhoneme`: the numbered eye and outline components count as phonemes upstream, and so here.
    pub fn is_phoneme(&self) -> bool {
        !matches!(
            self.face.component,
            FaceComponent::None | FaceComponent::EyesClosed | FaceComponent::EyesOpen | FaceComponent::Outlines
        )
    }

    /// `NormalizeStateModelData`.
    pub fn normalize_state_model_data(&mut self) {
        if self.state_property_id.is_nil() {
            self.state_property_id = Uuid::new_v4();
        }
        for definition in &mut self.state_definitions {
            definition.normalize();
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct VendorMetadata {
    pub name: String,
    pub website: String,
    pub contact: String,
    pub email: String,
    pub phone: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhysicalMetadata {
    pub height: String,
    pub width: String,
    pub depth: String,
    pub material: String,
    pub node_count: String,
    pub bulb_type: String,
    pub color_mode: ColorMode,
}

impl Default for PhysicalMetadata {
    fn default() -> Self {
        PhysicalMetadata {
            height: String::new(),
            width: String::new(),
            depth: String::new(),
            material: String::new(),
            node_count: String::new(),
            bulb_type: String::new(),
            color_mode: ColorMode::FullColor,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct InformationMetadata {
    pub notes: String,
}

/// The background picture, as the bytes of an image file plus its pixel size.
#[derive(Clone, Debug, PartialEq)]
pub struct PropImage {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl PropImage {
    pub fn is_jpeg(&self) -> bool {
        self.bytes.starts_with(&[0xFF, 0xD8, 0xFF])
    }

    /// A picture file's bytes, decoded once to be sure they are a picture.
    pub fn decode(bytes: Vec<u8>) -> Result<PropImage, String> {
        let decoded = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
        Ok(PropImage { width: decoded.width(), height: decoded.height(), bytes })
    }

    /// The solid black picture a new prop starts with.
    pub fn solid_black(width: u32, height: u32) -> PropImage {
        let width = width.max(1);
        let height = height.max(1);
        let pixels = image::RgbImage::new(width, height);
        PropImage { bytes: encode_jpeg(&image::DynamicImage::ImageRgb8(pixels)), width, height }
    }

    /// These bytes as a JPEG: unchanged when they already are one, so saving again and again doesn't wear the picture down.
    pub fn jpeg_bytes(&self) -> Result<Vec<u8>, String> {
        if self.is_jpeg() {
            return Ok(self.bytes.clone());
        }
        let decoded = image::load_from_memory(&self.bytes).map_err(|e| e.to_string())?;
        Ok(encode_jpeg(&decoded))
    }
}

fn encode_jpeg(picture: &image::DynamicImage) -> Vec<u8> {
    let rgb = picture.to_rgb8();
    let mut out = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90);
    if rgb.write_with_encoder(encoder).is_err() {
        out.clear();
    }
    out
}

/// Where a tree row sits: an element under one of its parents. The root has no parent.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RowRef {
    pub id: Uuid,
    pub parent: Option<Uuid>,
}

#[derive(Clone, Debug)]
pub struct Prop {
    pub id: Uuid,
    root: Uuid,
    elements: HashMap<Uuid, ElementModel>,
    /// How many elements carry each name, so making a name unique doesn't walk the whole prop.
    names: HashMap<String, u32>,
    pub prop_type: String,
    pub created_by: String,
    /// ISO 8601 text, kept exactly as a file gave it.
    pub creation_date: String,
    pub modified_date: String,
    pub opacity: f64,
    pub width: f64,
    pub height: f64,
    pub vendor: VendorMetadata,
    pub physical: PhysicalMetadata,
    pub information: InformationMetadata,
    pub image: Arc<PropImage>,
}

/// Everything a file holds about a prop besides its elements.
#[derive(Clone, Debug)]
pub struct PropHeader {
    pub id: Uuid,
    pub prop_type: String,
    pub created_by: String,
    pub creation_date: String,
    pub modified_date: String,
    pub opacity: f64,
    pub width: f64,
    pub height: f64,
    pub vendor: VendorMetadata,
    pub physical: PhysicalMetadata,
    pub information: InformationMetadata,
}

fn is_valid_canvas_dimension(value: f64) -> bool {
    value.is_finite() && (1.0..=100000.0).contains(&value)
}

impl Prop {
    /// `new Prop(name, 800, 600)`: a root named after the prop and a black 800 by 600 picture.
    pub fn new(name: &str, created_by: &str, now: &str) -> Prop {
        Prop::with_size(name, 800, 600, created_by, now)
    }

    pub fn with_size(name: &str, width: u32, height: u32, created_by: &str, now: &str) -> Prop {
        let root = ElementModel::new(name);
        let image = PropImage::solid_black(width, height);
        let mut prop = Prop {
            id: Uuid::new_v4(),
            root: root.id,
            elements: HashMap::new(),
            names: HashMap::new(),
            prop_type: String::new(),
            created_by: created_by.to_string(),
            creation_date: now.to_string(),
            modified_date: now.to_string(),
            opacity: 1.0,
            width: image.width as f64,
            height: image.height as f64,
            vendor: VendorMetadata::default(),
            physical: PhysicalMetadata::default(),
            information: InformationMetadata::default(),
            image: Arc::new(image),
        };
        prop.insert_element(root);
        prop
    }

    /// `Prop.Hydrate`: a prop put together from a file. `elements` must hold the root and be linked both ways already.
    pub fn from_parts(header: PropHeader, root: Uuid, elements: Vec<ElementModel>, image: PropImage) -> Prop {
        let mut prop = Prop {
            id: header.id,
            root,
            elements: HashMap::with_capacity(elements.len()),
            names: HashMap::new(),
            prop_type: header.prop_type,
            created_by: header.created_by,
            creation_date: header.creation_date,
            modified_date: header.modified_date,
            opacity: header.opacity,
            width: header.width,
            height: header.height,
            vendor: header.vendor,
            physical: header.physical,
            information: header.information,
            image: Arc::new(image),
        };
        for element in elements {
            prop.insert_element(element);
        }
        if !prop.elements.contains_key(&root) {
            let mut stand_in = ElementModel::new("");
            stand_in.id = root;
            prop.insert_element(stand_in);
        }
        // The `Image` setter: a canvas size that isn't usable gives way to the picture's.
        if !(is_valid_canvas_dimension(prop.width) && is_valid_canvas_dimension(prop.height)) {
            prop.width = prop.image.width as f64;
            prop.height = prop.image.height as f64;
        }
        prop
    }

    pub fn header(&self) -> PropHeader {
        PropHeader {
            id: self.id,
            prop_type: self.prop_type.clone(),
            created_by: self.created_by.clone(),
            creation_date: self.creation_date.clone(),
            modified_date: self.modified_date.clone(),
            opacity: self.opacity,
            width: self.width,
            height: self.height,
            vendor: self.vendor.clone(),
            physical: self.physical.clone(),
            information: self.information.clone(),
        }
    }

    // ----- Reading -----

    pub fn root_id(&self) -> Uuid {
        self.root
    }

    pub fn root(&self) -> &ElementModel {
        &self.elements[&self.root]
    }

    /// The prop's name is its root element's name.
    pub fn name(&self) -> &str {
        &self.root().name
    }

    pub fn element(&self, id: Uuid) -> Option<&ElementModel> {
        self.elements.get(&id)
    }

    pub fn contains(&self, id: Uuid) -> bool {
        self.elements.contains_key(&id)
    }

    pub fn element_count(&self) -> usize {
        self.elements.len()
    }

    pub fn elements(&self) -> impl Iterator<Item = &ElementModel> {
        self.elements.values()
    }

    /// For fields with no rule attached: face, model type, state definitions, order. Names, links and lights go through `Prop`.
    pub fn element_mut(&mut self, id: Uuid) -> Option<&mut ElementModel> {
        self.elements.get_mut(&id)
    }

    pub fn is_leaf(&self, id: Uuid) -> bool {
        self.element(id).is_some_and(|e| e.is_leaf())
    }

    pub fn is_light_node(&self, id: Uuid) -> bool {
        self.element(id).is_some_and(|e| e.is_light_node())
    }

    pub fn is_group_node(&self, id: Uuid) -> bool {
        self.element(id).is_some_and(|e| e.is_group_node())
    }

    pub fn is_root_node(&self, id: Uuid) -> bool {
        self.element(id).is_some_and(|e| e.is_root_node())
    }

    pub fn can_add_group_nodes(&self, id: Uuid) -> bool {
        let Some(e) = self.element(id) else { return false };
        e.is_group_node() && (e.children.is_empty() || e.children.iter().any(|c| self.is_group_node(*c)))
    }

    pub fn can_add_leaf_nodes(&self, id: Uuid) -> bool {
        let Some(e) = self.element(id) else { return false };
        e.is_group_node() && (e.children.is_empty() || e.children.iter().any(|c| self.is_leaf(*c)))
    }

    pub fn can_add_light_nodes(&self, id: Uuid) -> bool {
        let Some(e) = self.element(id) else { return false };
        !e.is_root_node() && (e.children.is_empty() || e.is_light_node() || !e.children.iter().any(|c| self.is_group_node(*c)))
    }

    /// `Order`: a leaf's own, and -1 for anything with children.
    pub fn order(&self, id: Uuid) -> i32 {
        match self.element(id) {
            Some(e) if e.is_leaf() => e.order,
            _ => -1,
        }
    }

    /// `LightSize`: a leaf's own, and for a group its first leaf's.
    pub fn light_size(&self, id: Uuid) -> i32 {
        let Some(e) = self.element(id) else { return DEFAULT_LIGHT_SIZE };
        if e.is_leaf() {
            return e.light_size;
        }
        self.first_leaf(id).and_then(|leaf| self.element(leaf)).map_or(e.light_size, |leaf| leaf.light_size)
    }

    fn first_leaf(&self, id: Uuid) -> Option<Uuid> {
        let mut current = id;
        // A well-formed prop has no cycles, but a walk that can't end must not hang the editor.
        for _ in 0..=self.elements.len() {
            let e = self.element(current)?;
            match e.children.first() {
                None => return Some(current),
                Some(first) => current = *first,
            }
        }
        None
    }

    /// `GetNodeEnumerator`: this element and everything below it, parents first. A shared element appears once per place.
    pub fn node_enumerator(&self, id: Uuid) -> Vec<Uuid> {
        let mut out = Vec::new();
        let mut stack = vec![id];
        let limit = self.walk_limit();
        while let Some(current) = stack.pop() {
            if out.len() >= limit {
                break;
            }
            if let Some(e) = self.element(current) {
                out.push(current);
                stack.extend(e.children.iter().rev());
            }
        }
        out
    }

    /// `GetLeafEnumerator`: the leaves at or below this element, in tree order, with repeats for shared ones.
    pub fn leaf_enumerator(&self, id: Uuid) -> Vec<Uuid> {
        self.node_enumerator(id).into_iter().filter(|n| self.is_leaf(*n)).collect()
    }

    /// The leaves at or below this element, each once, in tree order.
    pub fn distinct_leaves(&self, id: Uuid) -> Vec<Uuid> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        let mut stack = vec![id];
        while let Some(current) = stack.pop() {
            if !seen.insert(current) {
                continue;
            }
            if let Some(e) = self.element(current) {
                if e.is_leaf() {
                    out.push(current);
                } else {
                    stack.extend(e.children.iter().rev());
                }
            }
        }
        out
    }

    /// Every element reachable from the root, each once, parents before children by level (the order a file lists them in).
    pub fn breadth_first(&self) -> Vec<Uuid> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        let mut queue = std::collections::VecDeque::from([self.root]);
        while let Some(current) = queue.pop_front() {
            if !seen.insert(current) {
                continue;
            }
            if let Some(e) = self.element(current) {
                out.push(current);
                queue.extend(e.children.iter().copied());
            }
        }
        out
    }

    fn walk_limit(&self) -> usize {
        // A shared element is listed once per place, so a walk can be longer than the element count, but a cycle would make it endless.
        self.elements.len().saturating_mul(64).max(1024)
    }

    /// Whether `descendant` is `ancestor` or sits anywhere below it.
    pub fn is_same_or_descendant(&self, ancestor: Uuid, descendant: Uuid) -> bool {
        let mut seen = HashSet::new();
        let mut stack = vec![ancestor];
        while let Some(current) = stack.pop() {
            if current == descendant {
                return true;
            }
            if !seen.insert(current) {
                continue;
            }
            if let Some(e) = self.element(current) {
                stack.extend(e.children.iter().copied());
            }
        }
        false
    }

    /// The lights of every light node, each element once.
    pub fn lights(&self) -> impl Iterator<Item = (Uuid, &Light)> {
        self.elements.values().flat_map(|e| e.lights.iter().map(move |l| (e.id, l)))
    }

    pub(crate) fn lights_mut(&mut self) -> impl Iterator<Item = &mut Light> {
        self.elements.values_mut().flat_map(|e| e.lights.iter_mut())
    }

    pub fn light_count(&self) -> usize {
        self.elements.values().map(|e| e.lights.len()).sum()
    }

    /// Which element holds each light.
    pub fn light_owners(&self) -> HashMap<Uuid, Uuid> {
        self.lights().map(|(owner, light)| (light.id, owner)).collect()
    }

    // ----- Names -----

    fn name_count(&self, name: &str) -> u32 {
        self.names.get(name).copied().unwrap_or(0)
    }

    /// `IsNameDuplicated`: more than one element has it.
    pub fn is_name_duplicated(&self, name: &str) -> bool {
        self.name_count(name) > 1
    }

    /// `Uniquify`: the name itself when no element other than `rename_model` has it, else "name - n" counting from `order_index`.
    pub fn uniquify(&self, name: &str, order_index: i32, rename_model: Option<Uuid>) -> String {
        let own = rename_model.and_then(|id| self.element(id)).is_some_and(|e| e.name == name);
        if self.name_count(name) <= u32::from(own) {
            return name.to_string();
        }
        let mut counter = order_index;
        loop {
            // The check inside the loop doesn't set the renamed element aside. Upstream's doesn't either.
            let candidate = format!("{name} - {counter}");
            if self.name_count(&candidate) == 0 {
                return candidate;
            }
            counter = counter.saturating_add(1);
        }
    }

    /// Gives an element exactly this name. Callers that want it unique pass it through `uniquify` first.
    pub fn set_element_name(&mut self, id: Uuid, name: &str) {
        let Some(e) = self.elements.get_mut(&id) else { return };
        if e.name == name {
            return;
        }
        let old = std::mem::replace(&mut e.name, name.to_string());
        self.forget_name(&old);
        *self.names.entry(name.to_string()).or_insert(0) += 1;
    }

    pub fn set_name(&mut self, name: &str) {
        self.set_element_name(self.root, name);
    }

    /// Rename as the Rename command does: unique among the other elements. Returns the name it ended up with.
    pub fn rename(&mut self, id: Uuid, name: &str) -> String {
        let unique = self.uniquify(name, 2, Some(id));
        self.set_element_name(id, &unique);
        unique
    }

    fn forget_name(&mut self, name: &str) {
        if let Some(count) = self.names.get_mut(name) {
            *count -= 1;
            if *count == 0 {
                self.names.remove(name);
            }
        }
    }

    // ----- Building -----

    pub(crate) fn insert_element(&mut self, element: ElementModel) {
        *self.names.entry(element.name.clone()).or_insert(0) += 1;
        if let Some(old) = self.elements.insert(element.id, element) {
            self.forget_name(&old.name);
        }
    }

    fn drop_element(&mut self, id: Uuid) -> Option<ElementModel> {
        let removed = self.elements.remove(&id)?;
        self.forget_name(&removed.name);
        Some(removed)
    }

    /// `GetNextOrder`: one past the highest order any leaf has, and 1 when none is above zero.
    pub fn next_order(&self) -> i32 {
        let max = self.elements.values().filter(|e| e.is_leaf()).map(|e| e.order).max().unwrap_or(0);
        if max <= 0 {
            1
        } else {
            max.saturating_add(1)
        }
    }

    /// `OrderExists`.
    pub fn order_exists(&self, order: i32) -> bool {
        self.elements.values().any(|e| e.is_leaf() && e.order == order)
    }

    /// `CreateNode(name, parent, oneBasedNaming)`: a new child, under the root when no parent is given.
    pub fn create_node(&mut self, name: &str, parent: Option<Uuid>, one_based_naming: bool) -> Uuid {
        let unique = self.uniquify(name, if one_based_naming { 1 } else { 2 }, None);
        self.create_child(ElementModel::new(&unique), parent)
    }

    fn create_child(&mut self, mut element: ElementModel, parent: Option<Uuid>) -> Uuid {
        let parent = parent.filter(|p| self.contains(*p)).unwrap_or(self.root);
        let id = element.id;
        element.parents.push(parent);
        self.insert_element(element);
        if let Some(p) = self.elements.get_mut(&parent) {
            p.children.push(id);
        }
        id
    }

    /// `AddLight`: onto the target when it is a light node, otherwise as a new light node. Returns the element that got the light.
    pub fn add_light(&mut self, target: Option<Uuid>, x: f64, y: f64, size: i32) -> Result<Uuid, PropError> {
        if let Some(t) = target.and_then(|t| self.element(t)) {
            if !t.is_group_node() && !t.parents.is_empty() {
                let id = t.id;
                self.add_light_to_target(id, x, y);
                return Ok(id);
            }
        }
        self.add_light_node(target, x, y, None, size, None)
    }

    /// `AddLightNode`: a new one-light element under the target, or under the first group that can take one.
    pub fn add_light_node(
        &mut self,
        target: Option<Uuid>,
        x: f64,
        y: f64,
        order: Option<i32>,
        size: i32,
        node_name: Option<&str>,
    ) -> Result<Uuid, PropError> {
        let target = match target.filter(|t| self.contains(*t) && *t != self.root) {
            None => self.find_or_create_target_group_for_light(),
            Some(t) => {
                if !self.is_group_node(t) {
                    return Err(PropError::new("Cannot add light node to leaf element with lights.", ""));
                }
                t
            }
        };
        let order = order.unwrap_or_else(|| self.next_order());
        let base = node_name.map(str::to_string).unwrap_or_else(|| self.elements[&target].name.clone());
        let name = self.uniquify(&base, 1, None);
        let mut element = ElementModel::new(&name);
        element.order = order;
        let id = self.create_child(element, Some(target));
        self.set_light_size(id, size);
        self.add_light_to_target(id, x, y);
        Ok(id)
    }

    /// `FindOrCreateTargetGroupForLight`. Upstream takes the first element that can hold light nodes, which can be a light node
    /// itself and then ends up with both lights and children. Only a group is accepted here, and one is made when there is none.
    fn find_or_create_target_group_for_light(&mut self) -> Uuid {
        if !self.root().is_leaf() {
            let found = self
                .node_enumerator(self.root)
                .into_iter()
                .find(|id| self.can_add_light_nodes(*id) && self.is_group_node(*id));
            if let Some(found) = found {
                return found;
            }
        }
        let name = format!("{} Group", self.root().name);
        self.create_node(&name, None, false)
    }

    /// `AddLightToTarget`: another light on an element, at that element's own light size.
    fn add_light_to_target(&mut self, id: Uuid, x: f64, y: f64) -> Option<Uuid> {
        let size = self.light_size(id) as f64;
        let e = self.elements.get_mut(&id)?;
        if !e.is_leaf() {
            return None;
        }
        let light = Light { id: Uuid::new_v4(), x, y, z: 0.0, size };
        let light_id = light.id;
        e.lights.push(light);
        Some(light_id)
    }

    /// Puts ready-made lights on a leaf, as the file readers and paste do.
    pub(crate) fn push_lights(&mut self, id: Uuid, lights: Vec<Light>) {
        if let Some(e) = self.elements.get_mut(&id) {
            if e.is_leaf() {
                e.lights.extend(lights);
            }
        }
    }

    /// The `LightSize` setter: nothing when unchanged, at least 1, then pushed down to the lights or the children.
    pub fn set_light_size(&mut self, id: Uuid, value: i32) {
        self.set_light_size_bounded(id, value, 0);
    }

    fn set_light_size_bounded(&mut self, id: Uuid, value: i32, depth: usize) {
        let Some(e) = self.elements.get_mut(&id) else { return };
        if value == e.light_size || depth > 4096 {
            return;
        }
        let size = if value > 0 { value } else { 1 };
        e.light_size = size;
        if e.is_leaf() {
            for light in &mut e.lights {
                light.size = size as f64;
            }
        } else {
            for child in e.children.clone() {
                self.set_light_size_bounded(child, size, depth + 1);
            }
        }
    }

    // ----- Links -----

    /// `AddToParent`: one more parent for an existing element.
    pub fn add_to_parent(&mut self, model: Uuid, parent: Uuid) {
        let at = self.element(parent).map_or(0, |p| p.children.len());
        self.insert_to_parent(model, parent, at);
    }

    /// `InsertToParent`. Refused when it would make an element its own ancestor, which upstream allows and then cannot walk.
    pub fn insert_to_parent(&mut self, model: Uuid, parent: Uuid, index: usize) -> bool {
        if !self.contains(model) || !self.contains(parent) || self.is_same_or_descendant(model, parent) {
            return false;
        }
        if let Some(p) = self.elements.get_mut(&parent) {
            let at = index.min(p.children.len());
            p.children.insert(at, model);
        }
        if let Some(m) = self.elements.get_mut(&model) {
            m.parents.push(parent);
        }
        true
    }

    /// `RemoveFromParent`: unlink, and when that was the last parent drop the element and everything it alone held.
    /// Upstream leaves the orphaned descendants in its index as lightless ghosts; here they go too.
    pub fn remove_from_parent(&mut self, model: Uuid, parent: Uuid) {
        if model == self.root {
            return;
        }
        let mut pending = vec![(model, parent)];
        while let Some((model, parent)) = pending.pop() {
            if let Some(p) = self.elements.get_mut(&parent) {
                if let Some(at) = p.children.iter().position(|c| *c == model) {
                    p.children.remove(at);
                }
            }
            let Some(m) = self.elements.get_mut(&model) else { continue };
            if let Some(at) = m.parents.iter().position(|p| *p == parent) {
                m.parents.remove(at);
            }
            if m.parents.is_empty() {
                if let Some(removed) = self.drop_element(model) {
                    pending.extend(removed.children.iter().map(|child| (*child, model)));
                }
            }
        }
    }

    /// `RemoveLight` for many lights at once. An element left with none is removed from every parent.
    pub fn remove_lights(&mut self, lights: &HashSet<Uuid>) {
        let mut emptied = Vec::new();
        for e in self.elements.values_mut() {
            if e.lights.is_empty() || !e.is_leaf() {
                continue;
            }
            let before = e.lights.len();
            e.lights.retain(|l| !lights.contains(&l.id));
            if e.lights.is_empty() && before > 0 {
                emptied.push(e.id);
            }
        }
        for id in emptied {
            let parents = self.element(id).map(|e| e.parents.clone()).unwrap_or_default();
            for parent in parents {
                self.remove_from_parent(id, parent);
            }
        }
    }

    /// `TryMoveWithinParent`: moves children of one parent, kept in the order given, to start at `insertion_index`.
    pub fn try_move_within_parent(&mut self, parent: Uuid, models: &[Uuid], insertion_index: usize) -> bool {
        let Some(p) = self.elements.get_mut(&parent) else { return false };
        if insertion_index > p.children.len() || models.is_empty() {
            return false;
        }
        let mut seen = HashSet::new();
        if !models.iter().all(|m| seen.insert(*m) && p.children.contains(m)) {
            return false;
        }
        let mut cursor = insertion_index;
        for model in models {
            let Some(old_index) = p.children.iter().position(|c| c == model) else { return false };
            if old_index < cursor {
                cursor -= 1;
            }
            let destination = cursor.min(p.children.len() - 1);
            if old_index != destination {
                let moved = p.children.remove(old_index);
                p.children.insert(destination, moved);
            }
            cursor = destination + 1;
        }
        true
    }

    // ----- Picture -----

    /// `SetImage`: the canvas always takes the picture's pixel size. Lights stay where they are.
    pub fn set_image(&mut self, image: PropImage) {
        self.width = image.width as f64;
        self.height = image.height as f64;
        self.image = Arc::new(image);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prop() -> Prop {
        Prop::new("Prop", "tester", "2026-01-01T00:00:00")
    }

    #[test]
    fn new_prop_defaults() {
        let p = prop();
        assert_eq!(p.name(), "Prop");
        assert_eq!((p.width, p.height, p.opacity), (800.0, 600.0, 1.0));
        assert_eq!((p.image.width, p.image.height), (800, 600));
        assert!(p.image.is_jpeg());
        assert_eq!(p.creation_date, p.modified_date);
        assert!(p.root().is_leaf() && p.root().is_group_node() && p.root().is_root_node());
        assert_eq!(p.root().raw_light_size(), 3);
        assert_eq!(p.root().face, FaceDefinition { component: FaceComponent::None, default_color: Color::WHITE });
    }

    #[test]
    fn uniquify_counts_from_the_order_index() {
        let mut p = prop();
        let a = p.create_node("Arm", None, false);
        assert_eq!(p.element(a).unwrap().name(), "Arm");
        let b = p.create_node("Arm", None, false);
        assert_eq!(p.element(b).unwrap().name(), "Arm - 2");
        let c = p.create_node("Arm", None, true);
        assert_eq!(p.element(c).unwrap().name(), "Arm - 1");
        assert_eq!(p.uniquify("Arm", 2, None), "Arm - 3");
        assert_eq!(p.uniquify("Arm", 2, Some(a)), "Arm", "the renamed element doesn't clash with itself");
        assert_eq!(p.uniquify("Arm", 2, Some(b)), "Arm - 3");
        assert_eq!(p.uniquify("Leg", 2, None), "Leg");
        assert!(!p.is_name_duplicated("Arm"));
        p.set_element_name(b, "Arm");
        assert!(p.is_name_duplicated("Arm"));
        assert_eq!(p.rename(b, "Arm"), "Arm - 2");
    }

    #[test]
    fn first_light_makes_a_group_named_after_the_root() {
        let mut p = prop();
        let node = p.add_light(None, 10.0, 20.0, 3).unwrap();
        let group = p.root().children()[0];
        assert_eq!(p.element(group).unwrap().name(), "Prop Group");
        let e = p.element(node).unwrap();
        assert_eq!(e.name(), "Prop Group - 1");
        assert_eq!(e.parents(), &[group]);
        assert_eq!(e.order, 1);
        assert_eq!(e.lights().len(), 1);
        assert_eq!((e.lights()[0].x, e.lights()[0].y, e.lights()[0].size), (10.0, 20.0, 3.0));
        assert_eq!(e.element_type(), ElementType::Node);

        let second = p.add_light(None, 1.0, 1.0, 5).unwrap();
        let e2 = p.element(second).unwrap();
        assert_eq!(e2.name(), "Prop Group - 2");
        assert_eq!(e2.order, 2);
        assert_eq!(e2.lights()[0].size, 5.0);
        assert_eq!(p.next_order(), 3);
    }

    #[test]
    fn a_light_on_a_light_node_makes_a_string_at_its_own_size() {
        let mut p = prop();
        let node = p.add_light(None, 0.0, 0.0, 7).unwrap();
        let same = p.add_light(Some(node), 5.0, 5.0, 3).unwrap();
        assert_eq!(same, node);
        let e = p.element(node).unwrap();
        assert_eq!(e.element_type(), ElementType::String);
        assert_eq!(e.lights()[1].size, 7.0, "the element's size, not the default passed in");
        assert_eq!(p.element_count(), 3);
    }

    #[test]
    fn a_light_into_a_chosen_group_is_named_after_it() {
        let mut p = prop();
        let arm = p.create_node("Arm", None, false);
        let n1 = p.add_light(Some(arm), 0.0, 0.0, 3).unwrap();
        let n2 = p.add_light(Some(arm), 0.0, 0.0, 3).unwrap();
        assert_eq!(p.element(n1).unwrap().name(), "Arm - 1");
        assert_eq!(p.element(n2).unwrap().name(), "Arm - 2");
        assert_eq!(p.element(arm).unwrap().children(), &[n1, n2]);
    }

    #[test]
    fn with_no_group_free_a_new_one_is_made_rather_than_nesting_under_a_light_node() {
        let mut p = prop();
        let node = p.add_light(None, 0.0, 0.0, 3).unwrap();
        let group = p.root().children()[0];
        // Move the light node straight under the root and drop the group.
        p.add_to_parent(node, p.root_id());
        p.remove_from_parent(node, group);
        p.remove_from_parent(group, p.root_id());
        assert_eq!(p.root().children(), &[node]);
        let fresh = p.add_light(None, 1.0, 1.0, 3).unwrap();
        assert!(p.element(node).unwrap().children().is_empty());
        let parent = p.element(fresh).unwrap().parents()[0];
        assert_eq!(p.element(parent).unwrap().name(), "Prop Group");
    }

    #[test]
    fn type_rules() {
        let mut p = prop();
        let root = p.root_id();
        let group = p.create_node("G", None, false);
        assert!(p.can_add_group_nodes(root) && p.can_add_group_nodes(group));
        assert!(p.can_add_leaf_nodes(group));
        assert!(!p.can_add_light_nodes(root), "never the root");
        assert!(p.can_add_light_nodes(group));
        let node = p.add_light(Some(group), 0.0, 0.0, 3).unwrap();
        assert!(p.can_add_light_nodes(node) && !p.can_add_group_nodes(node));
        assert!(!p.can_add_group_nodes(group), "its children are light nodes");
        assert!(p.can_add_leaf_nodes(group));
        assert_eq!(p.order(group), -1);
        assert_eq!(p.order(node), 1);
        let sub = p.create_node("Sub", Some(root), false);
        let inner = p.create_node("Inner", Some(sub), false);
        assert!(p.can_add_light_nodes(inner));
        assert!(!p.can_add_light_nodes(sub), "a child of it is a group");
    }

    #[test]
    fn light_size_reads_the_first_leaf_and_writes_down_the_tree() {
        let mut p = prop();
        let group = p.create_node("G", None, false);
        let a = p.add_light(Some(group), 0.0, 0.0, 4).unwrap();
        let b = p.add_light(Some(group), 0.0, 0.0, 6).unwrap();
        assert_eq!(p.light_size(group), 4);
        p.set_light_size(group, 9);
        assert_eq!((p.light_size(a), p.light_size(b)), (9, 9));
        assert_eq!(p.element(b).unwrap().lights()[0].size, 9.0);
        p.set_light_size(a, 0);
        assert_eq!(p.light_size(a), 1, "never below 1");
        p.set_light_size(a, -5);
        assert_eq!(p.light_size(a), 1);
    }

    #[test]
    fn removing_the_last_parent_drops_the_whole_orphaned_branch() {
        let mut p = prop();
        let group = p.create_node("G", None, false);
        let node = p.add_light(Some(group), 0.0, 0.0, 3).unwrap();
        let other = p.create_node("Other", None, false);
        let shared = p.add_light(Some(group), 0.0, 0.0, 3).unwrap();
        p.add_to_parent(shared, other);
        p.remove_from_parent(group, p.root_id());
        assert!(!p.contains(group) && !p.contains(node));
        assert!(p.contains(shared), "it still has another parent");
        assert_eq!(p.element(shared).unwrap().parents(), &[other]);
        assert_eq!(p.uniquify("G - 1", 2, None), "G - 1", "no ghost keeps the old name taken");
        assert_eq!(p.element_count(), 3);
    }

    #[test]
    fn removing_lights_removes_emptied_elements_everywhere() {
        let mut p = prop();
        let g1 = p.create_node("G1", None, false);
        let g2 = p.create_node("G2", None, false);
        let node = p.add_light(Some(g1), 0.0, 0.0, 3).unwrap();
        p.add_light(Some(node), 1.0, 1.0, 3).unwrap();
        p.add_to_parent(node, g2);
        let lights: Vec<Uuid> = p.element(node).unwrap().lights().iter().map(|l| l.id).collect();
        p.remove_lights(&HashSet::from([lights[0]]));
        assert_eq!(p.element(node).unwrap().lights().len(), 1);
        p.remove_lights(&HashSet::from([lights[1]]));
        assert!(!p.contains(node));
        assert!(p.element(g1).unwrap().children().is_empty() && p.element(g2).unwrap().children().is_empty());
    }

    // Vixen.Tests/App/CustomPropEditor/ElementTreeDragDropTests.cs
    fn five() -> (Prop, Uuid, Vec<Uuid>) {
        let mut p = prop();
        let parent = p.create_node("Parent", None, false);
        let kids = (0..5).map(|i| p.create_node(&format!("Child {i}"), Some(parent), false)).collect();
        (p, parent, kids)
    }

    fn order(p: &Prop, parent: Uuid, kids: &[Uuid]) -> Vec<usize> {
        p.element(parent).unwrap().children().iter().map(|c| kids.iter().position(|k| k == c).unwrap()).collect()
    }

    #[test]
    fn move_within_parent_keeps_the_given_order() {
        let (mut p, parent, k) = five();
        assert!(p.try_move_within_parent(parent, &[k[0], k[1]], 4));
        assert_eq!(order(&p, parent, &k), [2, 3, 0, 1, 4]);

        let (mut p, parent, k) = five();
        assert!(p.try_move_within_parent(parent, &[k[3], k[4]], 1));
        assert_eq!(order(&p, parent, &k), [0, 3, 4, 1, 2]);

        let (mut p, parent, k) = five();
        assert!(p.try_move_within_parent(parent, &[k[1], k[3]], 5));
        assert_eq!(order(&p, parent, &k), [0, 2, 4, 1, 3]);

        let (mut p, parent, k) = five();
        assert!(p.try_move_within_parent(parent, &[k[4], k[0]], 2));
        assert_eq!(order(&p, parent, &k), [1, 4, 0, 2, 3]);
    }

    #[test]
    fn move_within_parent_rejects_bad_requests() {
        let (mut p, parent, k) = five();
        let stranger = p.create_node("Stranger", None, false);
        assert!(!p.try_move_within_parent(parent, &[], 0));
        assert!(!p.try_move_within_parent(parent, &[k[0]], 6));
        assert!(!p.try_move_within_parent(parent, &[k[0], k[0]], 1));
        assert!(!p.try_move_within_parent(parent, &[stranger], 1));
        assert!(!p.try_move_within_parent(Uuid::new_v4(), &[k[0]], 1));
        assert_eq!(order(&p, parent, &k), [0, 1, 2, 3, 4]);
    }

    #[test]
    fn linking_never_makes_a_cycle() {
        let mut p = prop();
        let a = p.create_node("A", None, false);
        let b = p.create_node("B", Some(a), false);
        assert!(!p.insert_to_parent(a, b, 0));
        assert!(!p.insert_to_parent(a, a, 0));
        assert!(p.element(b).unwrap().children().is_empty());
    }

    #[test]
    fn enum_names_round_trip_and_numbers_parse() {
        for f in FaceComponent::ALL {
            assert_eq!(FaceComponent::parse(f.name()), Some(*f));
        }
        assert_eq!(FaceComponent::Outlines.display_name(), "Outline");
        assert_eq!(ElementModelType::parse("2"), Some(ElementModelType::SubModel));
        assert_eq!(ElementModelType::parse("model"), None, "names are case-sensitive");
        assert_eq!(ColorMode::parse("9"), None);
    }
}
