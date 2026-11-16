//! Port of VixenPreviewData.cs and the data half of Shapes/DisplayItem.cs.

use crate::hit::HitCache;
use crate::overlay::ItemRuns;
use crate::shape::xml::shape_from_xml;
use crate::shape::Shape;
use crate::xml_util::{el_bool, el_i32, el_text, read_bool, read_i32, write_members, DoubleText, Member, XmlMemo, MEDIA3D_NS, PREVIEW_NS, SHAPES_NS};
use std::fmt;
use uuid::Uuid;
use vixen_xml::XElement;

/// `moduleType` of a `VixenPreviewData` module in a ModuleStore.
pub const VIXEN_PREVIEW_MODULE_TYPE: Uuid = uuid::uuid!("f43c31bc-cfcd-4dbc-adb7-d349c9ec623d");

/// A display item's identity in this session. Not saved. The C# compares items by bounds or reference.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ShapeId(pub u32);

/// `DisplayItem`.
#[derive(Clone, Debug)]
pub struct DisplayItem {
    pub id: ShapeId,
    pub shape: Shape,
    pub(crate) memo: Option<Box<XmlMemo>>,
    /// Shape-level bounds for hit testing, rebuilt on demand after the shape changes.
    pub(crate) hit: Option<Box<HitCache>>,
    /// The shape's edit colours, rebuilt on demand after its links or state change.
    pub(crate) runs: Option<Box<ItemRuns>>,
}

impl DisplayItem {
    pub fn new(id: ShapeId, shape: Shape) -> DisplayItem {
        DisplayItem { id, shape, memo: None, hit: None, runs: None }
    }

    /// Reads a `<DisplayItem>`.
    pub(crate) fn from_xml(e: &XElement, id: ShapeId) -> Option<DisplayItem> {
        let mut memo = XmlMemo::with_attrs(&e.attrs);
        let mut shape = None;
        for c in e.elements() {
            if c.local == "Shape" && c.ns == SHAPES_NS && shape.is_none() {
                shape = Some(shape_from_xml(c));
                memo.known("Shape");
            } else {
                memo.extra(c);
            }
        }
        Some(DisplayItem { id, shape: shape?, memo: Some(Box::new(memo)), hit: None, runs: None })
    }

    /// Writes a `<DisplayItem>`.
    pub(crate) fn to_xml(&self) -> XElement {
        let mut e = XElement::new(SHAPES_NS, "DisplayItem");
        write_members(&mut e, vec![Member::new("Shape", self.shape.to_xml_named("Shape"))], self.memo.as_deref());
        e
    }

    /// `PreviewTools.SerializeToString(displayItem)`: the text of a template file.
    pub(crate) fn to_xml_document(&self) -> String {
        self.to_xml().to_fragment()
    }
}

/// Why a preview could not be read at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewError {
    /// The element is not a `VixenPreviewData` or a `<Module>` holding one.
    NotPreviewData(String),
    /// A member of `VixenPreviewData` itself could not be read.
    BadMember(&'static str),
    /// Text that was supposed to be preview XML could not be parsed.
    Xml(String),
}

impl fmt::Display for PreviewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PreviewError::NotPreviewData(n) => write!(f, "<{n}> is not a VixenPreviewData"),
            PreviewError::BadMember(m) => write!(f, "VixenPreviewData member {m} could not be read"),
            PreviewError::Xml(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for PreviewError {}

/// `System.Windows.Media.Media3D.Vector3D`, the location offset.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Vector3D {
    pub x: DoubleText,
    pub y: DoubleText,
    pub z: DoubleText,
}

/// `VixenPreviewData`: one preview's saved state, as stored in a ModuleStore `<Module>` entry.
#[derive(Clone, Debug)]
pub struct PreviewData {
    pub module_instance_id: Option<String>,
    pub module_type_id: Option<String>,
    pub background_alpha: i32,
    pub background_file_name: Option<String>,
    pub top: i32,
    pub left: i32,
    pub setup_top: i32,
    pub setup_left: i32,
    pub setup_width: i32,
    pub setup_height: i32,
    pub width: i32,
    pub height: i32,
    pub save_locations: bool,
    pub location_offset: Vector3D,
    pub keep_insert_mode: bool,
    pub display_items: Vec<DisplayItem>,
    pub use_open_gl: bool,
    next_id: u32,
    memo: Option<Box<XmlMemo>>,
}

impl Default for PreviewData {
    /// A new preview, with the C# defaults.
    fn default() -> PreviewData {
        PreviewData {
            module_instance_id: None,
            module_type_id: None,
            background_alpha: 255,
            background_file_name: None,
            top: 0,
            left: 0,
            setup_top: 0,
            setup_left: 0,
            setup_width: 0,
            setup_height: 0,
            width: 0,
            height: 0,
            save_locations: true,
            location_offset: Vector3D::default(),
            keep_insert_mode: false,
            display_items: Vec::new(),
            use_open_gl: false,
            next_id: 1,
            memo: None,
        }
    }
}

impl PreviewData {
    pub fn new() -> PreviewData {
        PreviewData::default()
    }

    /// Reads a `VixenPreviewData` element, or a ModuleStore `<Module>` that holds one.
    pub fn from_xml(element: &XElement) -> Result<PreviewData, PreviewError> {
        let e = if element.local == "Module" {
            element.elements().find(|c| c.local == "VixenPreviewData").ok_or_else(|| PreviewError::NotPreviewData(element.local.clone()))?
        } else {
            element
        };
        if e.local != "VixenPreviewData" {
            return Err(PreviewError::NotPreviewData(e.local.clone()));
        }
        // The serializer starts from zeros, not the C# initialisers, so a member absent from the file
        // stays zero rather than taking the default a new preview gets.
        let mut d = PreviewData { background_alpha: 0, save_locations: false, ..PreviewData::default() };
        let mut memo = XmlMemo::with_attrs(&e.attrs);
        let ints = |c: &XElement, name: &'static str| read_i32(c).map_err(|_| PreviewError::BadMember(name));
        let bools = |c: &XElement, name: &'static str| read_bool(c).map_err(|_| PreviewError::BadMember(name));
        for c in e.elements() {
            let local = c.local.as_str();
            if memo.has_known(local) {
                memo.extra(c);
                continue;
            }
            let known: Option<&'static str> = match local {
                "ModuleInstanceId" if c.ns.is_empty() => {
                    d.module_instance_id = Some(c.text());
                    Some("ModuleInstanceId")
                }
                "ModuleTypeId" if c.ns.is_empty() => {
                    d.module_type_id = Some(c.text());
                    Some("ModuleTypeId")
                }
                _ if c.ns != PREVIEW_NS => None,
                "BackgroundAlpha" => {
                    d.background_alpha = ints(c, "BackgroundAlpha")?;
                    Some("BackgroundAlpha")
                }
                "BackgroundFileName" => {
                    d.background_file_name = if c.nil { None } else { Some(c.text()) };
                    Some("BackgroundFileName")
                }
                "DisplayItems" if !c.nil => {
                    for item in c.elements() {
                        let id = ShapeId(d.next_id);
                        match DisplayItem::from_xml(item, id) {
                            Some(i) => {
                                d.next_id += 1;
                                d.display_items.push(i);
                            }
                            // An item with no shape at all: nothing to show, but it must survive a save.
                            None => {
                                d.next_id += 1;
                                let mut i = DisplayItem::new(id, Shape::unknown(item));
                                let mut m = XmlMemo::default();
                                m.extra(item);
                                i.memo = Some(Box::new(m));
                                d.display_items.push(i);
                            }
                        }
                    }
                    Some("DisplayItems")
                }
                "Height" => {
                    d.height = ints(c, "Height")?;
                    Some("Height")
                }
                "KeepInsertMode" => {
                    d.keep_insert_mode = bools(c, "KeepInsertMode")?;
                    Some("KeepInsertMode")
                }
                "Left" => {
                    d.left = ints(c, "Left")?;
                    Some("Left")
                }
                "LocationOffset" if !c.nil => {
                    let part = |n: &str| -> Result<DoubleText, PreviewError> {
                        match c.child(n) {
                            Some(p) => DoubleText::parse(&p.text()).map_err(|_| PreviewError::BadMember("LocationOffset")),
                            None => Err(PreviewError::BadMember("LocationOffset")),
                        }
                    };
                    d.location_offset = Vector3D { x: part("_x")?, y: part("_y")?, z: part("_z")? };
                    Some("LocationOffset")
                }
                "SaveLocations" => {
                    d.save_locations = bools(c, "SaveLocations")?;
                    Some("SaveLocations")
                }
                "SetupHeight" => {
                    d.setup_height = ints(c, "SetupHeight")?;
                    Some("SetupHeight")
                }
                "SetupLeft" => {
                    d.setup_left = ints(c, "SetupLeft")?;
                    Some("SetupLeft")
                }
                "SetupTop" => {
                    d.setup_top = ints(c, "SetupTop")?;
                    Some("SetupTop")
                }
                "SetupWidth" => {
                    d.setup_width = ints(c, "SetupWidth")?;
                    Some("SetupWidth")
                }
                "Top" => {
                    d.top = ints(c, "Top")?;
                    Some("Top")
                }
                "UseOpenGL" => {
                    d.use_open_gl = bools(c, "UseOpenGL")?;
                    Some("UseOpenGL")
                }
                "Width" => {
                    d.width = ints(c, "Width")?;
                    Some("Width")
                }
                _ => None,
            };
            match known {
                Some(name) => memo.known(name),
                None => memo.extra(c),
            }
        }
        d.memo = Some(Box::new(memo));
        Ok(d)
    }

    /// Writes the `VixenPreviewData` element.
    pub fn to_xml(&self) -> XElement {
        let ns = PREVIEW_NS;
        let mut e = XElement::new(ns, "VixenPreviewData");
        let mut m: Vec<Member> = Vec::with_capacity(17);
        // ModuleDataModelBase's members come first and have no namespace.
        if let Some(id) = &self.module_instance_id {
            m.push(Member::new("ModuleInstanceId", el_text("", "ModuleInstanceId", id.clone())));
        }
        if let Some(id) = &self.module_type_id {
            m.push(Member::new("ModuleTypeId", el_text("", "ModuleTypeId", id.clone())));
        }
        m.push(Member::new("BackgroundAlpha", el_i32(ns, "BackgroundAlpha", self.background_alpha)));
        m.push(Member::new(
            "BackgroundFileName",
            match &self.background_file_name {
                Some(name) => el_text(ns, "BackgroundFileName", name.clone()),
                None => XElement::nil(ns, "BackgroundFileName"),
            },
        ));
        let mut items = XElement::new(ns, "DisplayItems");
        items.children.reserve(self.display_items.len());
        for item in &self.display_items {
            items.push(item.to_xml());
        }
        m.push(Member::new("DisplayItems", items));
        m.push(Member::new("Height", el_i32(ns, "Height", self.height)));
        m.push(Member::new("KeepInsertMode", el_bool(ns, "KeepInsertMode", self.keep_insert_mode)));
        m.push(Member::new("Left", el_i32(ns, "Left", self.left)));
        let mut offset = XElement::new(ns, "LocationOffset");
        offset.push(el_text(MEDIA3D_NS, "_x", self.location_offset.x.text()));
        offset.push(el_text(MEDIA3D_NS, "_y", self.location_offset.y.text()));
        offset.push(el_text(MEDIA3D_NS, "_z", self.location_offset.z.text()));
        m.push(Member::new("LocationOffset", offset));
        m.push(Member::new("SaveLocations", el_bool(ns, "SaveLocations", self.save_locations)));
        m.push(Member::new("SetupHeight", el_i32(ns, "SetupHeight", self.setup_height)));
        m.push(Member::new("SetupLeft", el_i32(ns, "SetupLeft", self.setup_left)));
        m.push(Member::new("SetupTop", el_i32(ns, "SetupTop", self.setup_top)));
        m.push(Member::new("SetupWidth", el_i32(ns, "SetupWidth", self.setup_width)));
        m.push(Member::new("Top", el_i32(ns, "Top", self.top)));
        m.push(Member::new("UseOpenGL", el_bool(ns, "UseOpenGL", self.use_open_gl)));
        m.push(Member::new("Width", el_i32(ns, "Width", self.width)));
        write_members(&mut e, m, self.memo.as_deref());
        e
    }

    pub(crate) fn alloc_id(&mut self) -> ShapeId {
        let id = ShapeId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Adds a shape, as `AddDisplayItem` does. Returns its id.
    pub fn add_shape(&mut self, shape: Shape) -> ShapeId {
        let id = self.alloc_id();
        self.display_items.push(DisplayItem::new(id, shape));
        id
    }

    pub fn item(&self, id: ShapeId) -> Option<&DisplayItem> {
        self.display_items.iter().find(|i| i.id == id)
    }

    pub fn item_mut(&mut self, id: ShapeId) -> Option<&mut DisplayItem> {
        self.display_items.iter_mut().find(|i| i.id == id)
    }

    /// `PixelCount`: all the lights in the preview, one per pixel however many colours it has.
    pub fn pixel_count(&self) -> usize {
        self.display_items.iter().map(|i| i.shape.pixel_count()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NS: &str = r#"xmlns="http://schemas.datacontract.org/2004/07/VixenModules.Preview.VixenPreview" xmlns:i="http://www.w3.org/2001/XMLSchema-instance" xmlns:a="http://schemas.datacontract.org/2004/07/VixenModules.Preview.VixenPreview.Shapes""#;

    fn round_trip(xml: &str) -> PreviewData {
        let e = XElement::parse(xml).unwrap();
        let d = PreviewData::from_xml(&e).unwrap();
        let out = d.to_xml();
        assert_eq!(e.first_difference(&out, String::new()), None, "{}", out.to_fragment());
        d
    }

    #[test]
    fn unknown_members_and_shape_types_survive_a_save() {
        let xml = format!(
            r#"<VixenPreviewData {NS}>
                <ModuleInstanceId xmlns="">c1645f1c-4ea6-44c4-a406-8f6b74d157e7</ModuleInstanceId>
                <ModuleTypeId xmlns="">f43c31bc-cfcd-4dbc-adb7-d349c9ec623d</ModuleTypeId>
                <BackgroundAlpha>71</BackgroundAlpha>
                <BackgroundFileName i:nil="true" />
                <FutureRootMember>x</FutureRootMember>
                <DisplayItems>
                  <a:DisplayItem>
                    <a:Shape i:type="a:PreviewHologram"><a:Beams>7</a:Beams></a:Shape>
                  </a:DisplayItem>
                  <a:DisplayItem>
                    <a:Future>y</a:Future>
                    <a:Shape i:type="a:PreviewLine">
                      <a:Locked>false</a:Locked>
                      <a:PixelSize>3</a:PixelSize>
                      <a:Pixels>
                        <a:PreviewPixel><a:WhatIsThis>1</a:WhatIsThis></a:PreviewPixel>
                        <a:PreviewPixel />
                      </a:Pixels>
                      <a:NewMember><a:Nested>2</a:Nested></a:NewMember>
                      <a:StringType>Pixel</a:StringType>
                      <a:_points>
                        <a:PreviewPoint><a:X>1</a:X><a:Y>2</a:Y></a:PreviewPoint>
                        <a:PreviewPoint><a:X>3</a:X><a:Y>4</a:Y></a:PreviewPoint>
                      </a:_points>
                    </a:Shape>
                  </a:DisplayItem>
                </DisplayItems>
                <Height>0</Height>
                <KeepInsertMode>false</KeepInsertMode>
                <Left>0</Left>
                <LocationOffset xmlns:b="http://schemas.datacontract.org/2004/07/System.Windows.Media.Media3D">
                  <b:_x>1.50</b:_x><b:_y>0</b:_y><b:_z>1E-05</b:_z>
                </LocationOffset>
                <SaveLocations>true</SaveLocations>
                <SetupHeight>1048</SetupHeight><SetupLeft>-8</SetupLeft><SetupTop>-8</SetupTop><SetupWidth>1936</SetupWidth>
                <Top>0</Top><UseOpenGL>true</UseOpenGL><Width>0</Width>
                <AnotherFutureMember />
            </VixenPreviewData>"#
        );
        let d = round_trip(&xml);
        assert_eq!(d.background_alpha, 71);
        assert_eq!(d.background_file_name, None);
        assert_eq!(d.display_items.len(), 2);
        assert_eq!(d.display_items[0].shape.pixel_count(), 0);
        assert_eq!(d.display_items[1].shape.pixel_count(), 2);
        assert_eq!(d.location_offset.x.value, 1.5);
    }

    #[test]
    fn a_new_preview_writes_every_member_in_serializer_order() {
        let e = PreviewData::new().to_xml();
        let names: Vec<&str> = e.elements().map(|c| c.local.as_str()).collect();
        assert_eq!(
            names,
            ["BackgroundAlpha", "BackgroundFileName", "DisplayItems", "Height", "KeepInsertMode", "Left", "LocationOffset", "SaveLocations", "SetupHeight", "SetupLeft", "SetupTop", "SetupWidth", "Top", "UseOpenGL", "Width"]
        );
        let back = PreviewData::from_xml(&e).unwrap();
        assert_eq!(e.first_difference(&back.to_xml(), String::new()), None);
    }

    #[test]
    fn a_module_element_is_accepted_and_anything_else_is_not() {
        let xml = format!(r#"<Module moduleType="x"><VixenPreviewData {NS}><BackgroundAlpha>5</BackgroundAlpha></VixenPreviewData></Module>"#);
        let d = PreviewData::from_xml(&XElement::parse(&xml).unwrap()).unwrap();
        assert_eq!(d.background_alpha, 5);
        assert!(PreviewData::from_xml(&XElement::parse("<Other/>").unwrap()).is_err());
    }

    #[test]
    fn a_known_shape_with_missing_points_stays_raw() {
        let xml = format!(
            r#"<VixenPreviewData {NS}><DisplayItems><a:DisplayItem><a:Shape i:type="a:PreviewArch"><a:PixelSize>3</a:PixelSize><a:Pixels /><a:StringType>Standard</a:StringType></a:Shape></a:DisplayItem></DisplayItems></VixenPreviewData>"#
        );
        let d = round_trip(&xml);
        assert!(!d.display_items[0].shape.is_light_shape());
    }
}
