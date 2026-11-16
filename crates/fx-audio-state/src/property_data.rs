//! Typed views of the Face and State element properties.
//!
//! `vixen_core` keeps these modules' data as `PropertyData::Unknown { raw_xml }`. The structs here are parsed
//! lazily from that XML (`DataContractSerializer` output) the first time an effect needs them. Parsing is
//! tolerant: namespaces and prefixes are ignored, missing members keep the C# constructor defaults, and
//! a document that cannot be parsed counts as "no such property".

use std::collections::HashMap;
use vixen_core::element::PropertyData;
use vixen_core::{Color, ElementTree, NodeId, Uuid};

/// `FaceDescriptor.ModuleId`.
pub const FACE_ID: Uuid = uuid::uuid!("eda73851-8350-4d3f-9df2-0d3b34e70bb6");
/// `StateDescriptor.ModuleId`.
pub const STATE_ID: Uuid = uuid::uuid!("35b893c3-d10c-4359-82bc-929c0762e809");

/// `FaceComponent`.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum FaceComponent {
    Mouth,
    Outlines,
    EyesOpen,
    EyesClosed,
}

impl FaceComponent {
    fn from_name(name: &str) -> Option<FaceComponent> {
        match name.trim() {
            "Mouth" => Some(FaceComponent::Mouth),
            "Outlines" => Some(FaceComponent::Outlines),
            "EyesOpen" => Some(FaceComponent::EyesOpen),
            "EyesClosed" => Some(FaceComponent::EyesClosed),
            _ => None,
        }
    }
}

/// `FaceData` / `FaceModule`.
#[derive(Clone, Debug, PartialEq)]
pub struct FaceData {
    /// `PhonemeList`, an `OrdinalIgnoreCase` dictionary: keys are stored lower-cased.
    pub phoneme_list: HashMap<String, bool>,
    pub face_components: HashMap<FaceComponent, bool>,
    pub default_color: Color,
}

impl Default for FaceData {
    fn default() -> Self {
        FaceData { phoneme_list: HashMap::new(), face_components: HashMap::new(), default_color: Color::WHITE }
    }
}

impl FaceData {
    /// `FaceModule.PhonemeState`.
    pub fn phoneme_state(&self, name: &str) -> bool {
        self.phoneme_list.get(&name.to_ascii_lowercase()).copied().unwrap_or(false)
    }

    /// `FaceModule.IsFaceComponentType`.
    pub fn is_face_component(&self, c: FaceComponent) -> bool {
        self.face_components.get(&c).copied().unwrap_or(false)
    }

    /// `FaceModule.ConfiguredColorAndIntensity`: always intensity 1.
    pub fn configured_color_and_intensity(&self) -> (f64, Color) {
        (1.0, self.default_color)
    }

    /// Reads the Face property off a node, if it has one.
    pub fn for_node(tree: &ElementTree, node: NodeId) -> Option<FaceData> {
        match &tree.property(node, &FACE_ID)?.data {
            PropertyData::Unknown { raw_xml } => parse_face(raw_xml),
            _ => None,
        }
    }
}

/// `StateItemData`.
#[derive(Clone, Debug, PartialEq)]
pub struct StateItemData {
    pub id: Uuid,
    pub name: String,
    pub color: Color,
    pub element_node_ids: Vec<Uuid>,
}

/// `StateDefinitionData`.
#[derive(Clone, Debug, PartialEq)]
pub struct StateDefinitionData {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub items: Vec<StateItemData>,
}

/// `StateData` of the State property.
#[derive(Clone, Debug, PartialEq)]
pub struct StatePropertyData {
    pub id: Uuid,
    pub definitions: Vec<StateDefinitionData>,
}

impl StatePropertyData {
    /// Reads the State property off a node, if it has one.
    pub fn for_node(tree: &ElementTree, node: NodeId) -> Option<StatePropertyData> {
        match &tree.property(node, &STATE_ID)?.data {
            PropertyData::Unknown { raw_xml } => parse_state(raw_xml),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// XML

/// Strips namespace prefixes from element and attribute names, and drops `xmlns` declarations, for fragments
/// whose prefixes are declared on an ancestor that was cut off.
fn strip_prefixes(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    let mut in_tag = false;
    let mut quote: Option<char> = None;
    let mut name = String::new();
    let mut at_name_start = false;
    while let Some(c) = chars.next() {
        if let Some(q) = quote {
            out.push(c);
            if c == q {
                quote = None;
            }
            continue;
        }
        if !in_tag {
            out.push(c);
            if c == '<' && !matches!(chars.peek(), Some('!') | Some('?')) {
                in_tag = true;
                at_name_start = true;
                if chars.peek() == Some(&'/') {
                    out.push(chars.next().unwrap());
                }
            }
            continue;
        }
        // inside a tag
        if c == '"' || c == '\'' {
            quote = Some(c);
            out.push(c);
            continue;
        }
        if c == '>' {
            in_tag = false;
            name.clear();
            out.push(c);
            continue;
        }
        if c.is_whitespace() {
            at_name_start = true;
            name.clear();
            out.push(c);
            continue;
        }
        if at_name_start {
            name.push(c);
            if c == ':' {
                // drop the prefix collected so far
                let kept = out.len() - (name.len() - 1);
                out.truncate(kept);
                name.clear();
                continue;
            }
            if c == '=' || c == '/' {
                at_name_start = false;
            }
        }
        out.push(c);
    }
    // `xmlns="..."` and `xmlns:p="..."` attributes: remove, they are meaningless once prefixes are gone
    let mut result = String::with_capacity(out.len());
    let mut rest = out.as_str();
    while let Some(i) = rest.find("xmlns") {
        let (head, tail) = rest.split_at(i);
        result.push_str(head);
        // skip to the closing quote of the value
        match tail.find('=').and_then(|eq| {
            let after = &tail[eq + 1..];
            let q = after.chars().next()?;
            let end = after[1..].find(q)?;
            Some(eq + 1 + 1 + end + 1)
        }) {
            Some(skip) => rest = &tail[skip..],
            None => {
                result.push_str("xmlns");
                rest = &tail[5..];
            }
        }
    }
    result.push_str(rest);
    result
}

fn with_doc<T>(raw: &str, f: impl Fn(roxmltree::Node) -> Option<T>) -> Option<T> {
    if raw.trim().is_empty() {
        return None;
    }
    for text in [raw.to_string(), format!("<r>{raw}</r>")] {
        if let Ok(doc) = roxmltree::Document::parse(&text) {
            return f(doc.root_element());
        }
    }
    let stripped = strip_prefixes(raw);
    for text in [stripped.clone(), format!("<r>{stripped}</r>")] {
        if let Ok(doc) = roxmltree::Document::parse(&text) {
            return f(doc.root_element());
        }
    }
    None
}

fn child<'a, 'b>(n: roxmltree::Node<'a, 'b>, name: &str) -> Option<roxmltree::Node<'a, 'b>> {
    n.children().find(|c| c.is_element() && c.tag_name().name() == name)
}

/// The element `name` is looked up on the root itself or among its descendants, so the document may be the
/// `ModuleData` element, the data element itself, or a fragment wrapped in a synthetic root.
fn find_data<'a, 'b>(root: roxmltree::Node<'a, 'b>, has: &str) -> roxmltree::Node<'a, 'b> {
    if child(root, has).is_some() {
        return root;
    }
    root.descendants().find(|n| n.is_element() && child(*n, has).is_some()).unwrap_or(root)
}

fn text_of(n: Option<roxmltree::Node>) -> Option<String> {
    n.map(|n| n.text().unwrap_or("").trim().to_string())
}

fn parse_guid(n: Option<roxmltree::Node>) -> Option<Uuid> {
    text_of(n).and_then(|t| Uuid::parse_str(&t).ok())
}

/// `System.Drawing.Color` as a data contract: `name`, `value`, `knownColor`, `state`.
fn parse_color(n: roxmltree::Node) -> Option<Color> {
    let state: i64 = text_of(child(n, "state")).and_then(|t| t.parse().ok()).unwrap_or(0);
    let value: i64 = text_of(child(n, "value")).and_then(|t| t.parse().ok()).unwrap_or(0);
    let name = text_of(child(n, "name")).unwrap_or_default();
    // Color.Value gives explicit ARGB precedence, then the numeric KnownColor identity.
    // Named .NET colors normally serialize with name=nil and value=0.
    if state & 2 != 0 {
        return Some(Color::from_argb_u32(value as u32));
    }
    if state & 1 != 0 {
        let known = text_of(child(n, "knownColor")).and_then(|t| t.parse().ok());
        if let Some(argb) = known.and_then(vixen_core::known_color::known_color_argb) {
            return Some(Color::from_argb_u32(argb));
        }
    }
    if let Some(known) = vixen_core::known_color::known_color_named(&name) {
        return vixen_core::known_color::known_color_argb(known).map(Color::from_argb_u32);
    }
    // Empty and unknown named colors have ARGB 0 in System.Drawing.
    child(n, "state").map(|_| Color::EMPTY)
}

fn parse_bool(t: &str) -> bool {
    matches!(t.trim(), "true" | "True" | "1")
}

/// A `Dictionary<K, bool>` data contract: `KeyValueOf...` entries with `Key` and `Value`.
fn parse_dictionary(n: roxmltree::Node) -> Vec<(String, bool)> {
    n.children()
        .filter(|e| e.is_element())
        .filter_map(|entry| {
            let k = child(entry, "Key")?.text().unwrap_or("").to_string();
            let v = parse_bool(&text_of(child(entry, "Value")).unwrap_or_default());
            Some((k, v))
        })
        .collect()
}

/// Parses the `FaceData` XML.
pub fn parse_face(raw: &str) -> Option<FaceData> {
    with_doc(raw, |root| {
        let data = find_data(root, "PhonemeList");
        let mut face = FaceData::default();
        if let Some(l) = child(data, "PhonemeList") {
            for (k, v) in parse_dictionary(l) {
                face.phoneme_list.insert(k.to_ascii_lowercase(), v);
            }
        }
        if let Some(c) = child(data, "FaceComponents") {
            for (k, v) in parse_dictionary(c) {
                if let Some(fc) = FaceComponent::from_name(&k) {
                    face.face_components.insert(fc, v);
                }
            }
        }
        if let Some(c) = child(data, "DefaultColor") {
            if let Some(color) = parse_color(c) {
                face.default_color = color;
            }
        }
        Some(face)
    })
}

/// Parses the State property's `StateData` XML, applying `Normalize` where it fixes missing members.
pub fn parse_state(raw: &str) -> Option<StatePropertyData> {
    with_doc(raw, |root| {
        let data = find_data(root, "StateDefinitions");
        let mut out = StatePropertyData { id: parse_guid(child(data, "Id")).unwrap_or(Uuid::nil()), definitions: Vec::new() };
        if let Some(defs) = child(data, "StateDefinitions") {
            for d in defs.children().filter(|c| c.is_element()) {
                let name = text_of(child(d, "Name")).unwrap_or_default();
                let mut def = StateDefinitionData {
                    id: parse_guid(child(d, "Id")).unwrap_or(Uuid::nil()),
                    // Normalize: blank names become "State - 1" (`StateNamingRules`)
                    name: if name.trim().is_empty() { "State - 1".to_string() } else { name.trim().to_string() },
                    description: text_of(child(d, "Description")).unwrap_or_default(),
                    items: Vec::new(),
                };
                if let Some(items) = child(d, "Items") {
                    for i in items.children().filter(|c| c.is_element()) {
                        let name = match child(i, "Name") {
                            Some(n) => n.text().unwrap_or("").to_string(),
                            None => "State Item - 1".to_string(),
                        };
                        let ids = child(i, "ElementNodeIds")
                            .map(|l| l.children().filter(|c| c.is_element()).filter_map(|g| Uuid::parse_str(g.text()?.trim()).ok()).collect())
                            .unwrap_or_default();
                        def.items.push(StateItemData {
                            id: parse_guid(child(i, "Id")).unwrap_or(Uuid::nil()),
                            name,
                            color: child(i, "Color").and_then(parse_color).unwrap_or(Color::WHITE),
                            element_node_ids: ids,
                        });
                    }
                }
                out.definitions.push(def);
            }
        }
        Some(out)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FACE: &str = r#"<FaceData xmlns="http://schemas.datacontract.org/2004/07/VixenModules.Property.Face" xmlns:i="http://www.w3.org/2001/XMLSchema-instance"><DefaultColor xmlns:d2p1="http://schemas.datacontract.org/2004/07/System.Drawing"><d2p1:knownColor>0</d2p1:knownColor><d2p1:name i:nil="true"/><d2p1:state>2</d2p1:state><d2p1:value>4294901760</d2p1:value></DefaultColor><FaceComponents xmlns:d2p1="http://schemas.microsoft.com/2003/10/Serialization/Arrays"><d2p1:KeyValueOfFaceComponentboolean><d2p1:Key>Mouth</d2p1:Key><d2p1:Value>true</d2p1:Value></d2p1:KeyValueOfFaceComponentboolean><d2p1:KeyValueOfFaceComponentboolean><d2p1:Key>EyesOpen</d2p1:Key><d2p1:Value>false</d2p1:Value></d2p1:KeyValueOfFaceComponentboolean></FaceComponents><PhonemeList xmlns:d2p1="http://schemas.microsoft.com/2003/10/Serialization/Arrays"><d2p1:KeyValueOfstringboolean><d2p1:Key>AI</d2p1:Key><d2p1:Value>true</d2p1:Value></d2p1:KeyValueOfstringboolean><d2p1:KeyValueOfstringboolean><d2p1:Key>REST</d2p1:Key><d2p1:Value>false</d2p1:Value></d2p1:KeyValueOfstringboolean></PhonemeList></FaceData>"#;

    #[test]
    fn parses_face_data() {
        let f = parse_face(FACE).unwrap();
        assert!(f.phoneme_state("ai") && f.phoneme_state("AI"));
        assert!(!f.phoneme_state("REST") && !f.phoneme_state("O"));
        assert!(f.is_face_component(FaceComponent::Mouth) && !f.is_face_component(FaceComponent::EyesOpen));
        assert_eq!(f.default_color, Color::RED);
    }

    #[test]
    fn parses_fragments_with_cut_off_prefixes() {
        let frag = r#"<PhonemeList><d2p1:KeyValueOfstringboolean><d2p1:Key>E</d2p1:Key><d2p1:Value>true</d2p1:Value></d2p1:KeyValueOfstringboolean></PhonemeList><DefaultColor><d2p1:name>Lime</d2p1:name><d2p1:state>9</d2p1:state><d2p1:value>0</d2p1:value></DefaultColor>"#;
        let f = parse_face(frag).unwrap();
        assert!(f.phoneme_state("E"));
        assert_eq!(f.default_color, Color::LIME);
    }

    #[test]
    fn parses_state_data() {
        let xml = r#"<StateData xmlns="http://schemas.datacontract.org/2004/07/VixenModules.Property.State" xmlns:i="http://www.w3.org/2001/XMLSchema-instance"><Id>11111111-1111-1111-1111-111111111111</Id><StateDefinitions><StateDefinitionData><Description/><Id>22222222-2222-2222-2222-222222222222</Id><Items><StateItemData><Color xmlns:d5p1="http://schemas.datacontract.org/2004/07/System.Drawing"><d5p1:knownColor>0</d5p1:knownColor><d5p1:name i:nil="true"/><d5p1:state>2</d5p1:state><d5p1:value>4278190335</d5p1:value></Color><ElementNodeIds xmlns:d6p1="http://schemas.microsoft.com/2003/10/Serialization/Arrays"><d6p1:guid>33333333-3333-3333-3333-333333333333</d6p1:guid></ElementNodeIds><Id>44444444-4444-4444-4444-444444444444</Id><Name>On</Name></StateItemData></Items><Name>Gate</Name></StateDefinitionData></StateDefinitions></StateData>"#;
        let s = parse_state(xml).unwrap();
        assert_eq!(s.definitions.len(), 1);
        let d = &s.definitions[0];
        assert_eq!(d.name, "Gate");
        assert_eq!(d.items[0].name, "On");
        assert_eq!(d.items[0].color, Color::BLUE);
        assert_eq!(d.items[0].element_node_ids.len(), 1);
    }

    #[test]
    fn known_color_ids_and_distinct_phoneme_keys_survive_deserialization() {
        let raw = r#"<FaceData xmlns:i="http://www.w3.org/2001/XMLSchema-instance"><DefaultColor><knownColor>141</knownColor><name i:nil="true"/><state>1</state><value>0</value></DefaultColor><PhonemeList><Entry><Key>AI</Key><Value>true</Value></Entry><Entry><Key> AI </Key><Value>false</Value></Entry></PhonemeList></FaceData>"#;
        let face = parse_face(raw).unwrap();
        assert_eq!(face.default_color, Color::RED);
        assert!(face.phoneme_state("ai"));
        assert!(!face.phoneme_state(" AI "));
        assert_eq!(face.phoneme_list.len(), 2);
    }

    #[test]
    fn garbage_is_none() {
        assert!(parse_face("<<<").is_none());
        assert!(parse_state("").is_none());
    }
}
