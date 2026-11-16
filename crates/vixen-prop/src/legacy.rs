//! Legacy `.prp` files: a LiteDB v4 data file read as raw pages and BSON, never opened as a database and never written.
//!
//! Ported from `Persistence/Legacy/LiteDb5021/LegacyLiteDbRawReaderPrototype.cs`, `LegacyBsonDocumentReader.cs` and
//! `Persistence/LegacyLiteDbPropReader.cs`. The page layout comes from LiteDB 5.0.21: see `THIRD-PARTY-NOTICES.md`.

use std::collections::{HashMap, HashSet};

use uuid::Uuid;
use vixen_core::Color;

use crate::model::*;

const PAGE_SIZE: usize = 4096;
const PAGE_HEADER: usize = 25;
const MAXIMUM_VISITED_PAGES: usize = 1_000_000;
const HEADER_INFO: &[u8] = b"** This is a LiteDB file **";
const NO_PAGE: u32 = u32::MAX;
// Upstream recurses without a limit and would run out of stack on a hostile file.
const MAXIMUM_DEPTH: usize = 256;

fn failed(detail: impl Into<String>) -> PropError {
    PropError::new("The legacy prop file could not be read.", detail)
}

pub fn is_v4_header(header: &[u8]) -> bool {
    header.len() >= 53 && &header[25..25 + HEADER_INFO.len()] == HEADER_INFO && header[52] == 7
}

// ----- Pages -----

struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], PropError> {
        let end = self.position.checked_add(count).filter(|end| *end <= self.bytes.len());
        let end = end.ok_or_else(|| failed("The legacy page is malformed."))?;
        let slice = &self.bytes[self.position..end];
        self.position = end;
        Ok(slice)
    }

    fn skip(&mut self, count: usize) -> Result<(), PropError> {
        self.take(count).map(|_| ())
    }

    fn u8(&mut self) -> Result<u8, PropError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, PropError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> Result<u32, PropError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn string(&mut self) -> Result<String, PropError> {
        let length = self.u32()? as usize;
        Ok(String::from_utf8_lossy(self.take(length)?).into_owned())
    }
}

struct IndexNode {
    page_id: u32,
    index: u16,
    previous_page_id: u32,
    next_page_id: u32,
}

struct DataFile<'a> {
    bytes: &'a [u8],
    collections: HashMap<String, u32>,
}

impl<'a> DataFile<'a> {
    fn open(bytes: &'a [u8]) -> Result<Self, PropError> {
        let page = page_of(bytes, 0)?;
        if !is_v4_header(page) {
            return Err(failed("The file is not a supported LiteDB v4 data file."));
        }
        if page[85..101].iter().any(|b| *b != 0) {
            return Err(failed("Encrypted LiteDB v4 Custom Prop files are not supported by this proof of concept."));
        }
        let mut cursor = Cursor { bytes: page, position: 101 };
        let count = cursor.u8()?;
        let mut collections = HashMap::new();
        for _ in 0..count {
            let name = cursor.string()?;
            let page_id = cursor.u32()?;
            if collections.insert(name, page_id).is_some() {
                return Err(failed("The legacy file lists a collection twice."));
            }
        }
        Ok(DataFile { bytes, collections })
    }

    fn page(&self, page_id: u32, expected: u8, purpose: &str) -> Result<&'a [u8], PropError> {
        let page = page_of(self.bytes, page_id)?;
        if page[4] != expected {
            return Err(failed(format!("The legacy {purpose} page has an unexpected type.")));
        }
        Ok(page)
    }

    fn read_collection(&self, collection: &str) -> Result<Vec<Vec<u8>>, PropError> {
        let Some(&collection_page_id) = self.collections.get(collection) else {
            return Err(failed(format!("The legacy file does not contain the {collection} collection.")));
        };
        let page = self.page(collection_page_id, 2, collection)?;
        let mut cursor = Cursor { bytes: page, position: PAGE_HEADER };
        cursor.string()?;
        cursor.skip(12)?;
        let head = index_head_page_id(&mut cursor)?;

        let mut documents = Vec::new();
        for index_page_id in self.index_pages(head)? {
            let page = self.page(index_page_id, 3, collection)?;
            let mut cursor = Cursor { bytes: page, position: PAGE_HEADER };
            for _ in 0..item_count(page) {
                let node = index_node(&mut cursor)?;
                if node.page_id != NO_PAGE {
                    documents.push(self.data_block(&node)?);
                }
            }
        }
        Ok(documents)
    }

    fn index_pages(&self, start: u32) -> Result<Vec<u32>, PropError> {
        let mut pending = vec![start];
        let mut visited = HashSet::new();
        let mut order = Vec::new();
        while let Some(page_id) = pending.pop() {
            if visited.len() >= MAXIMUM_VISITED_PAGES {
                return Err(failed("The legacy index page limit was exceeded."));
            }
            if !visited.insert(page_id) {
                continue;
            }
            let page = self.page(page_id, 3, "index")?;
            let mut cursor = Cursor { bytes: page, position: PAGE_HEADER };
            for _ in 0..item_count(page) {
                let node = index_node(&mut cursor)?;
                for linked in [node.previous_page_id, node.next_page_id] {
                    if linked != NO_PAGE && !visited.contains(&linked) {
                        pending.push(linked);
                    }
                }
            }
            order.push(page_id);
        }
        Ok(order)
    }

    fn data_block(&self, address: &IndexNode) -> Result<Vec<u8>, PropError> {
        let page = self.page(address.page_id, 4, "data")?;
        let mut cursor = Cursor { bytes: page, position: PAGE_HEADER };
        for _ in 0..item_count(page) {
            let index = cursor.u16()?;
            let extend_page_id = cursor.u32()?;
            let length = cursor.u16()? as usize;
            let data = cursor.take(length)?;
            if index == address.index {
                return if extend_page_id == NO_PAGE { Ok(data.to_vec()) } else { self.extend_data(extend_page_id) };
            }
        }
        Err(failed("The legacy data block is missing."))
    }

    fn extend_data(&self, mut page_id: u32) -> Result<Vec<u8>, PropError> {
        let mut output = Vec::new();
        let mut visited = HashSet::new();
        while page_id != NO_PAGE {
            if !visited.insert(page_id) || visited.len() > MAXIMUM_VISITED_PAGES {
                return Err(failed("The legacy extend page chain is invalid."));
            }
            let page = self.page(page_id, 5, "extend")?;
            let length = item_count(page) as usize;
            let data = page.get(PAGE_HEADER..PAGE_HEADER + length).ok_or_else(|| failed("The legacy page is malformed."))?;
            output.extend_from_slice(data);
            page_id = u32::from_le_bytes([page[9], page[10], page[11], page[12]]);
        }
        Ok(output)
    }
}

fn page_of(bytes: &[u8], page_id: u32) -> Result<&[u8], PropError> {
    let offset = page_id as usize * PAGE_SIZE;
    if bytes.len() < PAGE_SIZE || offset > bytes.len() - PAGE_SIZE {
        return Err(failed("The legacy page points outside the data file."));
    }
    Ok(&bytes[offset..offset + PAGE_SIZE])
}

fn item_count(page: &[u8]) -> u16 {
    u16::from_le_bytes([page[13], page[14]])
}

fn index_head_page_id(cursor: &mut Cursor) -> Result<u32, PropError> {
    for i in 0..16 {
        let field = cursor.string()?;
        cursor.skip(1)?;
        let head = cursor.u32()?;
        cursor.skip(12)?;
        if i == 0 && !field.is_empty() {
            return Ok(head);
        }
    }
    Err(failed("The legacy props collection has no primary index."))
}

fn index_node(cursor: &mut Cursor) -> Result<IndexNode, PropError> {
    cursor.u16()?;
    let levels = cursor.u8()? as usize;
    cursor.skip(13)?;
    let key_length = cursor.u16()? as usize;
    cursor.skip(1 + key_length)?;
    let page_id = cursor.u32()?;
    let index = cursor.u16()?;
    let previous_page_id = cursor.u32()?;
    cursor.skip(2)?;
    let next_page_id = cursor.u32()?;
    cursor.skip(2)?;
    // Upstream's `(levels - 1) * 12` goes negative for a zero-level node and steps back; no real node has zero levels.
    cursor.skip(levels.saturating_sub(1) * 12)?;
    Ok(IndexNode { page_id, index, previous_page_id, next_page_id })
}

// ----- BSON -----

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Bson {
    Double(f64),
    String(String),
    Document(Document),
    Array(Vec<Bson>),
    /// Binary data, and the raw bytes of an ObjectId or a Decimal, which upstream also hands back as a byte array.
    Bytes(Vec<u8>),
    Boolean(bool),
    Int32(i32),
    Int64(i64),
    Null,
}

pub(crate) type Document = Vec<(String, Bson)>;

fn field<'a>(document: &'a Document, name: &str) -> Option<&'a Bson> {
    document.iter().find(|(key, _)| key == name).map(|(_, value)| value)
}

struct BsonReader<'a> {
    source: &'a [u8],
    position: usize,
}

impl<'a> BsonReader<'a> {
    fn bytes(&mut self, count: i64) -> Result<&'a [u8], PropError> {
        if count < 0 || count as usize > self.source.len() - self.position {
            return Err(failed("The BSON value extends beyond its source data."));
        }
        let slice = &self.source[self.position..self.position + count as usize];
        self.position += count as usize;
        Ok(slice)
    }

    fn byte(&mut self) -> Result<u8, PropError> {
        let value = *self.source.get(self.position).ok_or_else(|| failed("The BSON value is truncated."))?;
        self.position += 1;
        Ok(value)
    }

    fn int32(&mut self) -> Result<i32, PropError> {
        if self.source.len() - self.position < 4 {
            return Err(failed("The BSON Int32 value is truncated."));
        }
        let b = &self.source[self.position..self.position + 4];
        self.position += 4;
        Ok(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn int64(&mut self) -> Result<i64, PropError> {
        if self.source.len() - self.position < 8 {
            return Err(failed("The BSON Int64 value is truncated."));
        }
        let mut b = [0u8; 8];
        b.copy_from_slice(&self.source[self.position..self.position + 8]);
        self.position += 8;
        Ok(i64::from_le_bytes(b))
    }

    fn cstring(&mut self) -> Result<String, PropError> {
        let start = self.position;
        while self.position < self.source.len() && self.source[self.position] != 0 {
            self.position += 1;
        }
        if self.position == self.source.len() {
            return Err(failed("The BSON field name terminator is missing."));
        }
        let value = String::from_utf8_lossy(&self.source[start..self.position]).into_owned();
        self.position += 1;
        Ok(value)
    }

    fn string(&mut self) -> Result<String, PropError> {
        let length = self.int32()?;
        if length <= 0 {
            return Err(failed("The BSON string length is invalid."));
        }
        let bytes = self.bytes(length as i64)?;
        if bytes[bytes.len() - 1] != 0 {
            return Err(failed("The BSON string terminator is invalid."));
        }
        Ok(String::from_utf8_lossy(&bytes[..bytes.len() - 1]).into_owned())
    }

    fn document(&mut self, depth: usize) -> Result<Document, PropError> {
        if depth > MAXIMUM_DEPTH {
            return Err(failed("The BSON document is nested too deeply."));
        }
        let length = self.int32()?;
        if length < 5 {
            return Err(failed("The BSON document length is invalid."));
        }
        let end = self.position + length as usize - 4;
        if end > self.source.len() {
            return Err(failed("The BSON document extends beyond its source data."));
        }
        let mut values: Document = Vec::new();
        while self.position < end - 1 {
            let kind = self.byte()?;
            let name = self.cstring()?;
            if name == "_type" {
                return Err(failed("Legacy prop data contains forbidden _type metadata."));
            }
            if field(&values, &name).is_some() {
                return Err(failed("The BSON document repeats a field."));
            }
            let value = self.value(kind, depth)?;
            values.push((name, value));
        }
        if self.position != end - 1 || self.byte()? != 0 {
            return Err(failed("The BSON document terminator is invalid."));
        }
        Ok(values)
    }

    fn value(&mut self, kind: u8, depth: usize) -> Result<Bson, PropError> {
        Ok(match kind {
            0x01 => Bson::Double(f64::from_bits(self.int64()? as u64)),
            0x02 => Bson::String(self.string()?),
            0x03 => Bson::Document(self.document(depth + 1)?),
            0x04 => {
                let mut keyed = Vec::new();
                for (key, value) in self.document(depth + 1)? {
                    let index: i32 = key.trim().parse().map_err(|_| failed("The BSON array index is invalid."))?;
                    keyed.push((index, value));
                }
                keyed.sort_by_key(|(index, _)| *index);
                Bson::Array(keyed.into_iter().map(|(_, value)| value).collect())
            }
            0x05 => {
                let length = self.int32()?;
                self.byte()?;
                Bson::Bytes(self.bytes(length as i64)?.to_vec())
            }
            0x07 => Bson::Bytes(self.bytes(12)?.to_vec()),
            0x08 => match self.byte()? {
                0 => Bson::Boolean(false),
                1 => Bson::Boolean(true),
                _ => return Err(failed("The BSON Boolean value is invalid.")),
            },
            0x09 | 0x12 => Bson::Int64(self.int64()?),
            0x0A | 0x7F | 0xFF => Bson::Null,
            0x10 => Bson::Int32(self.int32()?),
            0x13 => Bson::Bytes(self.bytes(16)?.to_vec()),
            other => return Err(failed(format!("Unsupported BSON type 0x{other:02X}."))),
        })
    }
}

pub(crate) fn read_bson(bytes: &[u8]) -> Result<Document, PropError> {
    let mut reader = BsonReader { source: bytes, position: 0 };
    let document = reader.document(0)?;
    if reader.position != bytes.len() {
        return Err(failed("The BSON document contains trailing data."));
    }
    Ok(document)
}

// ----- The prop -----

fn text(document: &Document, name: &str, fallback: &str) -> String {
    match field(document, name) {
        Some(Bson::String(value)) => value.clone(),
        _ => fallback.to_string(),
    }
}

fn number(document: &Document, name: &str, fallback: f64) -> f64 {
    match field(document, name) {
        Some(Bson::Int32(value)) => *value as f64,
        Some(Bson::Int64(value)) => *value as f64,
        Some(Bson::Double(value)) if value.is_finite() => *value,
        _ => fallback,
    }
}

fn guid_of(value: Option<&Bson>) -> Option<Uuid> {
    match value {
        // `new Guid(byte[])`: the first three fields are little-endian.
        Some(Bson::Bytes(bytes)) if bytes.len() == 16 => {
            let mut raw = [0u8; 16];
            raw.copy_from_slice(bytes);
            Some(Uuid::from_bytes_le(raw))
        }
        Some(Bson::String(value)) => Uuid::parse_str(value.trim()).ok(),
        _ => None,
    }
}

fn identifier(document: &Document) -> Uuid {
    guid_of(field(document, "_id")).or_else(|| guid_of(field(document, "Id"))).unwrap_or_else(Uuid::new_v4)
}

fn documents<'a>(document: &'a Document, name: &str) -> impl Iterator<Item = &'a Document> {
    let values: &[Bson] = match field(document, name) {
        Some(Bson::Array(values)) => values,
        _ => &[],
    };
    values.iter().filter_map(|value| match value {
        Bson::Document(inner) => Some(inner),
        _ => None,
    })
}

fn legacy_color(value: &str) -> Color {
    let parts: Vec<Option<u8>> = value.split(',').map(|part| part.trim().parse::<u8>().ok()).collect();
    match parts.as_slice() {
        [Some(a), Some(r), Some(g), Some(b)] => Color::argb(*a, *r, *g, *b),
        _ => Color::WHITE,
    }
}

#[derive(Default)]
struct Mapper {
    elements: Vec<ElementModel>,
    index: HashMap<Uuid, usize>,
    active: HashSet<Uuid>,
}

impl Mapper {
    fn element(&mut self, source: &Document) -> Uuid {
        let id = identifier(source);
        if self.index.contains_key(&id) {
            return id;
        }
        let mut element = ElementModel::new(&text(source, "Name", ""));
        element.id = id;
        element.state_property_id = guid_of(field(source, "StatePropertyId")).unwrap_or_else(Uuid::new_v4);
        element.order = number(source, "Order", 0.0) as i32;
        element.light_size = (number(source, "LightSize", DEFAULT_LIGHT_SIZE as f64) as i32).max(1);
        element.model_type = ElementModelType::parse(&text(source, "ModelType", "None")).unwrap_or(ElementModelType::None);
        let face = match field(source, "FaceDefinition") {
            Some(Bson::Document(face)) => face.clone(),
            _ => Vec::new(),
        };
        element.face = FaceDefinition {
            component: FaceComponent::parse(&text(&face, "FaceComponent", "None")).unwrap_or(FaceComponent::None),
            default_color: legacy_color(&text(&face, "DefaultColor", "255,255,255,255")),
        };
        for light in documents(source, "Lights") {
            element.lights.push(Light {
                id: guid_of(field(light, "Id")).unwrap_or_else(Uuid::new_v4),
                x: number(light, "X", 0.0),
                y: number(light, "Y", 0.0),
                z: number(light, "Z", 0.0),
                size: number(light, "Size", element.light_size as f64),
            });
        }
        authored_states(source, &mut element);
        if element.state_definitions.is_empty() {
            legacy_states(source, &mut element);
        }
        element.normalize_state_model_data();

        let at = self.elements.len();
        self.index.insert(id, at);
        self.elements.push(element);
        self.active.insert(id);
        for child in documents(source, "Children") {
            let child_id = self.element(child);
            // A child that repeats one of its own ancestors' ids would loop every walk of the tree, and one listed twice
            // under a parent could never be saved. Upstream links both; they are left out here.
            if self.active.contains(&child_id) || self.elements[at].children.contains(&child_id) {
                continue;
            }
            self.elements[at].children.push(child_id);
            let child_at = self.index[&child_id];
            if !self.elements[child_at].parents.contains(&id) {
                self.elements[child_at].parents.push(id);
            }
        }
        self.active.remove(&id);
        id
    }
}

fn authored_states(source: &Document, element: &mut ElementModel) {
    for definition in documents(source, "StateDefinitionModels") {
        let items = documents(definition, "Items")
            .map(|item| StateItemModel {
                id: identifier(item),
                name: text(item, "Name", DEFAULT_STATE_ITEM_NAME),
                color: legacy_color(&text(item, "Color", "255,255,255,255")),
                element_ids: match field(item, "ElementModelIds") {
                    Some(Bson::Array(values)) => values
                        .iter()
                        .filter_map(|value| match value {
                            Bson::Bytes(bytes) if bytes.len() == 16 => guid_of(Some(value)),
                            _ => None,
                        })
                        .filter(|id| !id.is_nil())
                        .collect(),
                    _ => Vec::new(),
                },
            })
            .collect();
        element.state_definitions.push(StateDefinitionModel {
            id: identifier(definition),
            name: text(definition, "Name", DEFAULT_STATE_DEFINITION_NAME),
            description: text(definition, "Description", ""),
            items,
        });
    }
}

/// The older shape: one row per item, each naming the definition it belongs to.
fn legacy_states(source: &Document, element: &mut ElementModel) {
    let mut rows: Vec<&Document> = Vec::new();
    if let Some(Bson::Document(row)) = field(source, "StateDefinition") {
        rows.push(row);
    }
    rows.extend(documents(source, "StateDefinitions"));
    let mut groups: Vec<(String, Vec<StateItemModel>)> = Vec::new();
    for row in rows {
        let key = text(row, "StateDefinitionName", DEFAULT_STATE_DEFINITION_NAME);
        let item = StateItemModel::new(&text(row, "Name", DEFAULT_STATE_ITEM_NAME));
        match groups.iter_mut().find(|(name, _)| *name == key) {
            Some((_, items)) => items.push(item),
            None => groups.push((key, vec![item])),
        }
    }
    for (name, items) in groups {
        element.state_definitions.push(StateDefinitionModel { id: Uuid::new_v4(), name, description: String::new(), items });
    }
}

fn read_image(file: &DataFile) -> Result<Vec<u8>, PropError> {
    let mut file_id = None;
    for raw in file.read_collection("_files")? {
        if let Some(Bson::String(id)) = field(&read_bson(&raw)?, "_id") {
            if id == "$/image/background.jpg" || id == "$/image/background.png" {
                file_id = Some(id.clone());
                break;
            }
        }
    }
    let Some(file_id) = file_id else {
        return Err(failed("The legacy Custom Prop background image is missing."));
    };
    let prefix = format!("{file_id}\\");
    let mut chunks: Vec<(i32, Document)> = Vec::new();
    for raw in file.read_collection("_chunks")? {
        let document = read_bson(&raw)?;
        let number = match field(&document, "_id") {
            Some(Bson::String(id)) => id.strip_prefix(&prefix).and_then(|rest| rest.trim().parse::<i32>().ok()),
            _ => None,
        };
        if let Some(number) = number {
            chunks.push((number, document));
        }
    }
    chunks.sort_by_key(|(number, _)| *number);
    let mut output = Vec::new();
    for (_, chunk) in &chunks {
        match field(chunk, "data") {
            Some(Bson::Bytes(data)) => output.extend_from_slice(data),
            _ => return Err(failed("The legacy Custom Prop background image chunks are invalid.")),
        }
    }
    if chunks.is_empty() {
        return Err(failed("The legacy Custom Prop background image chunks are invalid."));
    }
    Ok(output)
}

/// `LegacyLiteDbPropReader.ReadAsync`. The file holds no dates, so both are `now`.
pub fn read_legacy(bytes: &[u8], now: &str) -> Result<Prop, PropError> {
    let file = DataFile::open(bytes)?;
    let mut props = file.read_collection("props")?;
    if props.len() != 1 {
        return Err(failed("A legacy Custom Prop file must contain exactly one props document."));
    }
    let values = read_bson(&props.remove(0))?;
    let image = PropImage::decode(read_image(&file)?)
        .map_err(|e| PropError::new("The legacy prop image is invalid.", format!("The image decoder rejected the legacy image. {e}")))?;

    let mut mapper = Mapper::default();
    let root = match field(&values, "RootNode") {
        Some(Bson::Document(root)) => mapper.element(root),
        _ => {
            let element = ElementModel::new(&text(&values, "Name", "Legacy Custom Prop"));
            let id = element.id;
            mapper.elements.push(element);
            id
        }
    };
    let root_name = mapper.elements.iter().find(|e| e.id == root).map(|e| e.name().to_string()).unwrap_or_default();
    let header = PropHeader {
        id: identifier(&values),
        prop_type: text(&values, "Type", ""),
        created_by: text(&values, "CreatedBy", ""),
        creation_date: now.to_string(),
        modified_date: now.to_string(),
        opacity: number(&values, "Opacity", 1.0),
        width: number(&values, "Width", image.width as f64),
        height: number(&values, "Height", image.height as f64),
        vendor: VendorMetadata::default(),
        physical: PhysicalMetadata::default(),
        information: InformationMetadata::default(),
    };
    let mut prop = Prop::from_parts(header, root, mapper.elements, image);
    prop.set_name(&text(&values, "Name", &root_name));
    Ok(prop)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::file::{self, PropSourceFormat};

    // ----- A BSON writer and a LiteDB v4 page writer, enough to build the files the reader is for -----

    pub(crate) fn bson(fields: &[(&str, Bson)]) -> Vec<u8> {
        let mut body = Vec::new();
        for (name, value) in fields {
            write_value(&mut body, name, value);
        }
        let mut out = ((body.len() + 5) as i32).to_le_bytes().to_vec();
        out.extend(body);
        out.push(0);
        out
    }

    fn write_value(out: &mut Vec<u8>, name: &str, value: &Bson) {
        let kind = match value {
            Bson::Double(_) => 0x01,
            Bson::String(_) => 0x02,
            Bson::Document(_) => 0x03,
            Bson::Array(_) => 0x04,
            Bson::Bytes(_) => 0x05,
            Bson::Boolean(_) => 0x08,
            Bson::Null => 0x0A,
            Bson::Int32(_) => 0x10,
            Bson::Int64(_) => 0x12,
        };
        out.push(kind);
        out.extend(name.as_bytes());
        out.push(0);
        match value {
            Bson::Double(v) => out.extend(v.to_le_bytes()),
            Bson::String(v) => {
                out.extend(((v.len() + 1) as i32).to_le_bytes());
                out.extend(v.as_bytes());
                out.push(0);
            }
            Bson::Document(fields) => {
                let borrowed: Vec<(&str, Bson)> = fields.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
                out.extend(bson(&borrowed));
            }
            Bson::Array(values) => {
                let keys: Vec<String> = (0..values.len()).map(|i| i.to_string()).collect();
                let borrowed: Vec<(&str, Bson)> = keys.iter().map(String::as_str).zip(values.iter().cloned()).collect();
                out.extend(bson(&borrowed));
            }
            Bson::Bytes(v) => {
                out.extend((v.len() as i32).to_le_bytes());
                out.push(0);
                out.extend(v);
            }
            Bson::Boolean(v) => out.push(*v as u8),
            Bson::Null => {}
            Bson::Int32(v) => out.extend(v.to_le_bytes()),
            Bson::Int64(v) => out.extend(v.to_le_bytes()),
        }
    }

    pub(crate) fn doc(fields: &[(&str, Bson)]) -> Bson {
        Bson::Document(fields.iter().map(|(k, v)| (k.to_string(), v.clone())).collect())
    }

    pub(crate) fn text_of(value: &str) -> Bson {
        Bson::String(value.to_string())
    }

    pub(crate) fn guid_bytes(id: Uuid) -> Bson {
        Bson::Bytes(id.to_bytes_le().to_vec())
    }

    #[derive(Default)]
    pub(crate) struct FileBuilder {
        pages: Vec<Vec<u8>>,
        collections: Vec<(String, u32)>,
    }

    impl FileBuilder {
        pub(crate) fn new() -> Self {
            FileBuilder { pages: vec![vec![0u8; PAGE_SIZE]], collections: Vec::new() }
        }

        fn page(&mut self, kind: u8) -> usize {
            let mut page = vec![0u8; PAGE_SIZE];
            page[4] = kind;
            self.pages.push(page);
            self.pages.len() - 1
        }

        fn data(&mut self, document: &[u8]) -> (u32, u16) {
            let index = 7u16;
            let mut extend = NO_PAGE;
            let inline: &[u8] = if document.len() > 3000 {
                let chunks: Vec<&[u8]> = document.chunks(PAGE_SIZE - PAGE_HEADER).collect();
                let ids: Vec<usize> = chunks.iter().map(|_| self.page(5)).collect();
                for (i, chunk) in chunks.iter().enumerate() {
                    let next = ids.get(i + 1).map_or(NO_PAGE, |id| *id as u32);
                    let page = &mut self.pages[ids[i]];
                    page[9..13].copy_from_slice(&next.to_le_bytes());
                    page[13..15].copy_from_slice(&(chunk.len() as u16).to_le_bytes());
                    page[PAGE_HEADER..PAGE_HEADER + chunk.len()].copy_from_slice(chunk);
                }
                extend = ids[0] as u32;
                &[]
            } else {
                document
            };
            let id = self.page(4);
            let page = &mut self.pages[id];
            // One other block first, so the reader has to find the right one by its index.
            page[13..15].copy_from_slice(&2u16.to_le_bytes());
            let mut block = Vec::new();
            block.extend(3u16.to_le_bytes());
            block.extend(NO_PAGE.to_le_bytes());
            block.extend(2u16.to_le_bytes());
            block.extend([0xAA, 0xBB]);
            block.extend(index.to_le_bytes());
            block.extend(extend.to_le_bytes());
            block.extend((inline.len() as u16).to_le_bytes());
            block.extend(inline);
            page[PAGE_HEADER..PAGE_HEADER + block.len()].copy_from_slice(&block);
            (id as u32, index)
        }

        fn node(levels: u8, key: &[u8], block: (u32, u16), previous: u32, next: u32) -> Vec<u8> {
            let mut out = Vec::new();
            out.extend(0u16.to_le_bytes());
            out.push(levels);
            out.extend([0u8; 13]);
            out.extend((key.len() as u16).to_le_bytes());
            out.push(0);
            out.extend(key);
            out.extend(block.0.to_le_bytes());
            out.extend(block.1.to_le_bytes());
            out.extend(previous.to_le_bytes());
            out.extend([0u8; 2]);
            out.extend(next.to_le_bytes());
            out.extend([0u8; 2]);
            out.extend(vec![0u8; (levels as usize - 1) * 12]);
            out
        }

        /// One collection: a head index page holding the head node and the first document, and a second index page,
        /// reached only through a node's next-page link, holding the rest.
        pub(crate) fn collection(&mut self, name: &str, documents: &[Vec<u8>]) {
            let blocks: Vec<(u32, u16)> = documents.iter().map(|d| self.data(d)).collect();
            let head = self.page(3);
            let second = self.page(3);
            let (first, rest) = blocks.split_at(blocks.len().min(1));
            let mut head_nodes = Self::node(3, b"", (NO_PAGE, 0), NO_PAGE, second as u32);
            for block in first {
                head_nodes.extend(Self::node(1, b"key", *block, NO_PAGE, NO_PAGE));
            }
            self.pages[head][13..15].copy_from_slice(&((1 + first.len()) as u16).to_le_bytes());
            self.pages[head][PAGE_HEADER..PAGE_HEADER + head_nodes.len()].copy_from_slice(&head_nodes);
            let mut rest_nodes = Vec::new();
            for block in rest {
                rest_nodes.extend(Self::node(2, b"another key", *block, head as u32, NO_PAGE));
            }
            self.pages[second][13..15].copy_from_slice(&(rest.len() as u16).to_le_bytes());
            self.pages[second][PAGE_HEADER..PAGE_HEADER + rest_nodes.len()].copy_from_slice(&rest_nodes);

            let id = self.page(2);
            let mut body = Vec::new();
            body.extend((name.len() as u32).to_le_bytes());
            body.extend(name.as_bytes());
            body.extend([0u8; 12]);
            for slot in 0..16 {
                let field: &[u8] = if slot == 0 { b"_id" } else { b"" };
                body.extend((field.len() as u32).to_le_bytes());
                body.extend(field);
                body.push(0);
                body.extend((head as u32).to_le_bytes());
                body.extend([0u8; 12]);
            }
            self.pages[id][PAGE_HEADER..PAGE_HEADER + body.len()].copy_from_slice(&body);
            self.collections.push((name.to_string(), id as u32));
        }

        pub(crate) fn image(&mut self, file_id: &str, bytes: &[u8]) {
            self.collection("_files", &[bson(&[("_id", text_of("$/other")),]), bson(&[("_id", text_of(file_id)), ("length", Bson::Int64(bytes.len() as i64))])]);
            let size = bytes.len().div_ceil(3).max(1);
            let mut chunks: Vec<Vec<u8>> = bytes
                .chunks(size)
                .enumerate()
                .map(|(i, chunk)| bson(&[("_id", text_of(&format!("{file_id}\\{i}"))), ("data", Bson::Bytes(chunk.to_vec()))]))
                .collect();
            // Stored out of order, with another file's chunk among them.
            chunks.reverse();
            chunks.push(bson(&[("_id", text_of("$/other\\0")), ("data", Bson::Bytes(vec![1, 2, 3]))]));
            self.collection("_chunks", &chunks);
        }

        pub(crate) fn finish(mut self) -> Vec<u8> {
            let header = &mut self.pages[0];
            header[4] = 1;
            header[25..25 + HEADER_INFO.len()].copy_from_slice(HEADER_INFO);
            header[52] = 7;
            let mut body = vec![self.collections.len() as u8];
            for (name, page) in &self.collections {
                body.extend((name.len() as u32).to_le_bytes());
                body.extend(name.as_bytes());
                body.extend(page.to_le_bytes());
            }
            header[101..101 + body.len()].copy_from_slice(&body);
            self.pages.concat()
        }
    }

    pub(crate) struct Ids {
        pub prop: Uuid,
        pub root: Uuid,
        pub group: Uuid,
        pub shared: Uuid,
        pub single: Uuid,
        pub light: Uuid,
    }

    pub(crate) fn ids() -> Ids {
        Ids {
            prop: Uuid::parse_str("00112233-4455-6677-8899-aabbccddeeff").unwrap(),
            root: Uuid::from_u128(1),
            group: Uuid::from_u128(2),
            shared: Uuid::from_u128(3),
            single: Uuid::from_u128(4),
            light: Uuid::from_u128(5),
        }
    }

    pub(crate) fn sample_document() -> Vec<u8> {
        let ids = ids();
        let shared = doc(&[
            ("_id", guid_bytes(ids.shared)),
            ("Name", text_of("Shared")),
            ("Order", Bson::Int32(2)),
            ("LightSize", Bson::Int32(5)),
            (
                "Lights",
                Bson::Array(vec![
                    doc(&[("Id", guid_bytes(ids.light)), ("X", Bson::Double(12.5)), ("Y", Bson::Int32(40)), ("Size", Bson::Double(6.0))]),
                    doc(&[("X", Bson::Int64(7)), ("Y", Bson::Double(f64::NAN))]),
                ]),
            ),
            ("FaceDefinition", doc(&[("FaceComponent", text_of("Outlines")), ("DefaultColor", text_of("255,10,20,30"))])),
            ("ModelType", text_of("FaceInfo")),
        ]);
        let single = doc(&[
            ("Id", text_of(&ids.single.to_string())),
            ("Name", text_of("Single")),
            ("Order", Bson::Double(1.9)),
            ("Lights", Bson::Array(vec![doc(&[("X", Bson::Int32(1)), ("Y", Bson::Int32(2))])])),
            (
                "StateDefinitions",
                Bson::Array(vec![
                    doc(&[("StateDefinitionName", text_of("Doors")), ("Name", text_of("Open"))]),
                    doc(&[("Name", text_of("Loose"))]),
                    doc(&[("StateDefinitionName", text_of("Doors")), ("Name", text_of("Closed"))]),
                ]),
            ),
        ]);
        let group = doc(&[
            ("_id", guid_bytes(ids.group)),
            ("Name", text_of("Group")),
            ("ModelType", text_of("9")),
            // The second copy of Shared is a duplicate under this parent, and the copy of the root would be a cycle.
            ("Children", Bson::Array(vec![shared.clone(), single, shared.clone(), doc(&[("_id", guid_bytes(ids.root))])])),
            (
                "StateDefinitionModels",
                Bson::Array(vec![doc(&[
                    ("Name", text_of("Windows")),
                    ("Description", text_of("authored")),
                    (
                        "Items",
                        Bson::Array(vec![doc(&[
                            ("Name", text_of("Lit")),
                            ("Color", text_of("255,0,128,255")),
                            ("ElementModelIds", Bson::Array(vec![guid_bytes(ids.shared), guid_bytes(Uuid::nil()), text_of("no")])),
                        ])]),
                    ),
                ])]),
            ),
            ("StateDefinition", doc(&[("Name", text_of("ignored because authored states exist"))])),
        ]);
        let root = doc(&[("_id", guid_bytes(ids.root)), ("Name", text_of("Root Name")), ("Children", Bson::Array(vec![group, shared]))]);
        bson(&[
            ("_id", guid_bytes(ids.prop)),
            ("Name", text_of("Old Star")),
            ("Type", text_of("Star")),
            ("CreatedBy", text_of("someone")),
            ("Opacity", Bson::Double(0.25)),
            ("Width", Bson::Int32(640)),
            ("RootNode", root),
            ("Padding", text_of(&"x".repeat(9000))),
            ("SelectedModelIds", Bson::Null),
            ("Flag", Bson::Boolean(true)),
        ])
    }

    pub(crate) fn sample_file() -> Vec<u8> {
        let mut builder = FileBuilder::new();
        builder.collection("props", &[sample_document()]);
        builder.image("$/image/background.png", &png(32, 16));
        builder.finish()
    }

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut out = Vec::new();
        image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(width, height, image::Rgb([9, 99, 199])))
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .unwrap();
        out
    }

    #[test]
    fn a_legacy_file_reads_into_a_prop() {
        let ids = ids();
        let bytes = sample_file();
        assert!(is_v4_header(&bytes));
        let loaded = file::load_prop_bytes(&bytes, "2026-11-09T10:00:00.0000000-05:00").unwrap();
        assert_eq!(loaded.format, PropSourceFormat::LegacyLiteDbV4);
        let prop = loaded.prop;
        assert_eq!(prop.id, ids.prop, "a 16-byte binary is a .NET Guid, little-endian in its first three fields");
        assert_eq!(prop.name(), "Old Star", "the prop's name replaces the root's");
        assert_eq!((prop.prop_type.as_str(), prop.created_by.as_str()), ("Star", "someone"));
        assert_eq!(prop.creation_date, "2026-11-09T10:00:00.0000000-05:00");
        assert_eq!((prop.opacity, prop.width, prop.height), (0.25, 640.0, 16.0), "a missing height is the picture's");
        assert_eq!((prop.image.width, prop.image.height), (32, 16));

        assert_eq!(prop.root_id(), ids.root);
        assert_eq!(prop.root().children(), [ids.group, ids.shared]);
        let group = prop.element(ids.group).unwrap();
        assert_eq!(group.children(), [ids.shared, ids.single], "no duplicate child and no link back to the root");
        assert_eq!(group.model_type, ElementModelType::None, "an undefined enum number falls back");
        let mut parents = prop.element(ids.shared).unwrap().parents().to_vec();
        parents.sort();
        assert_eq!(parents, [ids.root, ids.group]);

        let shared = prop.element(ids.shared).unwrap();
        assert_eq!((shared.order, shared.raw_light_size(), shared.model_type), (2, 5, ElementModelType::FaceInfo));
        assert_eq!(shared.face, FaceDefinition { component: FaceComponent::Outlines, default_color: Color::argb(255, 10, 20, 30) });
        let lights = shared.lights();
        assert_eq!((lights[0].id, lights[0].x, lights[0].y, lights[0].size), (ids.light, 12.5, 40.0, 6.0));
        assert_eq!((lights[1].x, lights[1].y, lights[1].size), (7.0, 0.0, 5.0), "NaN falls back, and a light takes its element's size");

        let single = prop.element(ids.single).unwrap();
        assert_eq!(single.order, 1, "a double order truncates");
        let states: Vec<(&str, Vec<&str>)> =
            single.state_definitions.iter().map(|d| (d.name.as_str(), d.items.iter().map(|i| i.name.as_str()).collect())).collect();
        assert_eq!(states, [("Doors", vec!["Open", "Closed"]), ("State - 1", vec!["Loose"])]);

        assert_eq!(group.state_definitions.len(), 1, "authored states win over the older rows");
        let item = &group.state_definitions[0].items[0];
        assert_eq!((group.state_definitions[0].name.as_str(), group.state_definitions[0].description.as_str()), ("Windows", "authored"));
        assert_eq!((item.name.as_str(), item.color, item.element_ids.as_slice()), ("Lit", Color::argb(255, 0, 128, 255), &[ids.shared][..]));
        assert_eq!(prop.light_count(), 3);
    }

    #[test]
    fn a_legacy_prop_saves_as_a_package_and_keeps_the_original_once() {
        let dir = file::tests::scratch("legacy");
        let path = dir.join("Old Star.prp");
        let original = sample_file();
        std::fs::write(&path, &original).unwrap();
        let loaded = file::load_prop(&path, "2026-11-09T10:00:00").unwrap();

        let elsewhere = dir.join("Copy.prp");
        assert_eq!(file::save_prop(&loaded.prop, &elsewhere, Some(&path)).unwrap().legacy_backup, None, "saving elsewhere backs nothing up");
        assert_eq!(std::fs::read(&path).unwrap(), original);

        let backup = file::legacy_backup_path(&path);
        assert!(backup.to_string_lossy().ends_with("Old Star.prp.legacy-litedb.bak"));
        let outcome = file::save_prop(&loaded.prop, &path, Some(&path)).unwrap();
        assert_eq!(outcome.legacy_backup.as_deref(), Some(backup.as_path()));
        assert_eq!(std::fs::read(&backup).unwrap(), original, "the backup is the untouched original");
        let again = file::load_prop(&path, "now").unwrap();
        assert_eq!(again.format, PropSourceFormat::Package);
        assert_eq!(again.prop.id, loaded.prop.id);
        assert_eq!(again.prop.element_count(), loaded.prop.element_count());
        assert!(again.prop.image.is_jpeg());

        // The editor forgets the legacy source after that save; even if it didn't, an existing v4 backup is left alone.
        assert_eq!(file::save_prop(&again.prop, &path, Some(&path)).unwrap().legacy_backup, None);
        assert_eq!(std::fs::read(&backup).unwrap(), original);

        std::fs::write(&backup, b"something else entirely").unwrap();
        let error = file::save_prop(&again.prop, &path, Some(&path)).unwrap_err();
        assert_eq!(error.detail, "The existing legacy backup is not a LiteDB v4 file and will not be overwritten.");
        let _ = std::fs::remove_dir_all(dir);
    }

    fn detail(bytes: &[u8]) -> String {
        read_legacy(bytes, "now").unwrap_err().detail
    }

    #[test]
    fn broken_legacy_files_fail_cleanly() {
        let image = png(4, 4);

        let mut none = FileBuilder::new();
        none.image("$/image/background.png", &image);
        assert_eq!(detail(&none.finish()), "The legacy file does not contain the props collection.");

        let mut two = FileBuilder::new();
        two.collection("props", &[sample_document(), sample_document()]);
        two.image("$/image/background.png", &image);
        assert_eq!(detail(&two.finish()), "A legacy Custom Prop file must contain exactly one props document.");

        let mut no_image = FileBuilder::new();
        no_image.collection("props", &[sample_document()]);
        no_image.image("$/image/elsewhere.png", &image);
        assert_eq!(detail(&no_image.finish()), "The legacy Custom Prop background image is missing.");

        let mut bad_image = FileBuilder::new();
        bad_image.collection("props", &[sample_document()]);
        bad_image.image("$/image/background.jpg", b"not a picture");
        let error = read_legacy(&bad_image.finish(), "now").unwrap_err();
        assert_eq!(error.message, "The legacy prop image is invalid.");

        let mut typed = FileBuilder::new();
        typed.collection("props", &[bson(&[("_type", text_of("Some.Type, Assembly"))])]);
        typed.image("$/image/background.png", &image);
        assert_eq!(detail(&typed.finish()), "Legacy prop data contains forbidden _type metadata.");

        let mut encrypted = sample_file();
        encrypted[90] = 1;
        assert_eq!(detail(&encrypted), "Encrypted LiteDB v4 Custom Prop files are not supported by this proof of concept.");

        let mut wrong_type = sample_file();
        let props_page = {
            let file = DataFile::open(&wrong_type).unwrap();
            file.collections["props"] as usize
        };
        wrong_type[props_page * PAGE_SIZE + 4] = 9;
        assert_eq!(detail(&wrong_type), "The legacy props page has an unexpected type.");

        let mut short = sample_file();
        short.truncate(PAGE_SIZE * 2 + 100);
        assert_eq!(detail(&short), "The legacy page points outside the data file.");

        let mut header_only = sample_file();
        header_only.truncate(60);
        assert!(is_v4_header(&header_only));
        assert_eq!(detail(&header_only), "The legacy page points outside the data file.");
    }

    #[test]
    fn a_prop_with_no_root_node_gets_one() {
        let mut builder = FileBuilder::new();
        builder.collection("props", &[bson(&[("Name", text_of("Bare"))])]);
        builder.image("$/image/background.jpg", &PropImage::solid_black(8, 6).bytes);
        let prop = read_legacy(&builder.finish(), "now").unwrap();
        assert_eq!((prop.name(), prop.element_count(), prop.width, prop.height, prop.opacity), ("Bare", 1, 8.0, 6.0, 1.0));
    }

    #[test]
    fn bson_faults_are_named() {
        let fault = |bytes: &[u8]| read_bson(bytes).unwrap_err().detail;
        assert_eq!(fault(&[4, 0, 0, 0]), "The BSON document length is invalid.");
        assert_eq!(fault(&[9, 0, 0, 0, 0]), "The BSON document extends beyond its source data.");
        assert_eq!(fault(&[5, 0, 0, 0, 0, 0]), "The BSON document contains trailing data.");
        assert_eq!(fault(&[5, 0, 0, 0, 1]), "The BSON document terminator is invalid.");
        assert_eq!(fault(&[8, 0, 0, 0, 0x08, b'a', 0, 2]), "The BSON Boolean value is invalid.");
        assert_eq!(fault(&[8, 0, 0, 0, 0x06, b'a', 0, 0]), "Unsupported BSON type 0x06.");
        assert_eq!(fault(&[12, 0, 0, 0, 0x02, b'a', 0, 0, 0, 0, 0, 0]), "The BSON string length is invalid.");
        let mut deep = bson(&[]);
        for _ in 0..400 {
            deep = bson(&[("a", Bson::Document(read_bson(&deep).unwrap()))].as_slice()).to_vec();
            if read_bson(&deep).is_err() {
                break;
            }
        }
        assert_eq!(fault(&deep), "The BSON document is nested too deeply.");
        assert_eq!(read_bson(&bson(&[("a", Bson::Array(vec![Bson::Int32(1), Bson::Null]))])).unwrap(), vec![("a".to_string(), Bson::Array(vec![Bson::Int32(1), Bson::Null]))]);
    }
}
