//! TimedSequenceMapper's ElementMap and editor state. The dialog edits a private map;
//! only its accepted pairs are applied to a sequence. Profiles are never changed here.
use crate::SessionError;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;
use vixen_core::ElementTree;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mapping {
    pub source_id: Uuid,
    pub source_name: String,
    pub target_id: Option<Uuid>,
    pub target_name: String,
}

#[derive(Clone, Debug)]
pub struct TreeRow {
    pub path: String,
    pub id: Uuid,
    pub name: String,
    pub target_name: String,
    pub child_count: usize,
    pub active: bool,
}

pub struct ElementMapState {
    pub id: Uuid,
    pub name: String,
    pub path: Option<PathBuf>,
    pub modified: bool,
    pub mappings: Vec<Mapping>,
    index: HashMap<Uuid, usize>,
    sources: Vec<(Uuid, String)>,
    active_sources: HashSet<Uuid>,
    source_tree: Option<Value>,
    destination: Arc<ElementTree>,
}

fn fail(message: impl Into<String>) -> SessionError {
    SessionError::Other(message.into())
}
fn guid(v: &Value, key: &str) -> Result<Uuid, SessionError> {
    v.get(key)
        .and_then(Value::as_str)
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| fail(format!("Missing or invalid {key}.")))
}
fn children(v: &Value) -> &[Value] {
    v.get("children")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}
fn tree_node<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut node = root;
    for part in path.split('/').filter(|p| !p.is_empty()) {
        node = children(node).get(part.parse::<usize>().ok()?)?;
    }
    Some(node)
}
fn validate_tree(root: &Value) -> Result<(), SessionError> {
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        guid(node, "id")?;
        if !node.get("name").is_some_and(Value::is_string) {
            return Err(fail("Element tree contains an invalid name."));
        }
        if let Some(c) = node.get("children") {
            if !c.is_array() {
                return Err(fail("Element tree children must be an array."));
            }
        }
        pending.extend(children(node));
    }
    Ok(())
}

impl ElementMapState {
    pub fn new(name: String, sources: Vec<(Uuid, String)>, destination: Arc<ElementTree>) -> Self {
        let mut state = Self {
            id: Uuid::new_v4(),
            name,
            path: None,
            modified: true,
            mappings: vec![],
            index: HashMap::new(),
            active_sources: sources.iter().map(|(id, _)| *id).collect(),
            sources,
            source_tree: None,
            destination,
        };
        state.add_missing_sources();
        state
    }
    fn reindex(&mut self) {
        crate::collation::sort_by_name(&mut self.mappings, |m| &m.source_name);
        self.index = self
            .mappings
            .iter()
            .enumerate()
            .map(|(i, m)| (m.source_id, i))
            .collect();
    }
    pub fn reset(&mut self, name: String) {
        self.id = Uuid::new_v4();
        self.name = name;
        self.path = None;
        self.source_tree = None;
        self.mappings.clear();
        self.index.clear();
        self.add_missing_sources();
        self.modified = true;
    }
    pub fn missing_sources(&self) -> usize {
        self.sources
            .iter()
            .filter(|(id, _)| !self.index.contains_key(id))
            .count()
    }
    pub fn add_missing_sources(&mut self) {
        for (id, name) in &self.sources {
            if self.index.contains_key(id) {
                continue;
            }
            self.index.insert(*id, self.mappings.len());
            self.mappings.push(Mapping {
                source_id: *id,
                source_name: name.clone(),
                target_id: None,
                target_name: String::new(),
            });
            self.modified = true;
        }
        self.reindex();
    }
    pub fn has_source_tree(&self) -> bool {
        self.source_tree.is_some()
    }
    pub fn load(&mut self, path: &Path) -> Result<(), SessionError> {
        let text = std::fs::read_to_string(path).map_err(|e| fail(e.to_string()))?;
        self.load_json(&text)?;
        self.name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        self.path = Some(path.to_owned());
        Ok(())
    }
    pub fn load_json(&mut self, text: &str) -> Result<(), SessionError> {
        let data: Value = serde_json::from_str(text).map_err(|e| fail(e.to_string()))?;
        let id = guid(&data, "id")?;
        let name = data
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("New Map")
            .to_owned();
        let source_tree = data.get("sourceTree").filter(|v| !v.is_null()).cloned();
        if let Some(tree) = &source_tree {
            validate_tree(tree)?;
        }
        let mut mappings = vec![];
        let mut seen = HashSet::new();
        if let Some(entries) = data.get("elementMappings") {
            let entries = entries
                .as_array()
                .ok_or_else(|| fail("Element mappings must be an array."))?;
            for entry in entries {
                let source_id = guid(entry, "sourceId")?;
                if !seen.insert(source_id) {
                    crate::logging::warn(
                        "VixenModules.App.TimedSequenceMapper",
                        "Repeated source id in element map; keeping the first entry.",
                    );
                    continue;
                }
                let target_id = match entry.get("targetId").filter(|v| !v.is_null()) {
                    Some(_) => Some(guid(entry, "targetId")?).filter(|id| !id.is_nil()),
                    None => None,
                };
                mappings.push(Mapping {
                    source_id,
                    source_name: entry
                        .get("sourceName")
                        .and_then(Value::as_str)
                        .unwrap_or("Unknown")
                        .into(),
                    target_id,
                    target_name: entry
                        .get("targetName")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .into(),
                });
            }
        }
        // A malformed file cannot replace the current editor contents.
        self.id = id;
        self.name = name;
        self.mappings = mappings;
        self.source_tree = source_tree;
        self.path = None;
        self.modified = false;
        self.reindex();
        Ok(())
    }
    pub fn json(&self) -> Value {
        let entries: Vec<_> = self.mappings.iter().map(|m| {
            let mut entry=json!({"sourceName":m.source_name,"sourceId":m.source_id.to_string(),"targetName":m.target_name,"isDirty":true});
            if let Some(id)=m.target_id { entry["targetId"]=json!(id.to_string()); }
            entry
        }).collect();
        let mut data = json!({"id":self.id.to_string(),"name":self.name,"isDirty":true});
        if !entries.is_empty() {
            data["elementMappings"] = json!(entries);
        }
        if let Some(tree) = &self.source_tree {
            data["sourceTree"] = tree.clone();
        }
        data
    }
    pub fn save(&mut self, path: &Path) -> Result<(), SessionError> {
        let name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let mut data = self.json();
        data["name"] = json!(name);
        let bytes = serde_json::to_vec_pretty(&data).map_err(|e| fail(e.to_string()))?;
        vixen_io::file::write_atomic(path, &bytes)?;
        self.name = name;
        self.path = Some(path.to_owned());
        self.modified = false;
        Ok(())
    }
    pub fn load_tree(&mut self, path: &Path) -> Result<(), SessionError> {
        let text = std::fs::read_to_string(path).map_err(|e| fail(e.to_string()))?;
        self.load_tree_json(&text)
    }
    pub fn load_tree_json(&mut self, text: &str) -> Result<(), SessionError> {
        let tree: Value = serde_json::from_str(&text).map_err(|e| fail(e.to_string()))?;
        validate_tree(&tree)?;
        if children(&tree).is_empty() {
            return Err(fail("The element tree contains no elements."));
        }
        self.source_tree = Some(tree);
        self.modified = true;
        Ok(())
    }
    pub fn assign(&mut self, source: Uuid, target: Option<Uuid>) -> Result<(), SessionError> {
        // SourceTreeViewModel.ClearMapping only clears an existing map. An inactive
        // source-tree row with no mapping must stay absent, and leave dirty state alone.
        if target.is_none() && !self.index.contains_key(&source) {
            return Ok(());
        }
        let target_name = match target {
            Some(id) => self
                .destination
                .find(&id)
                .map(|n| self.destination.node(n).name.clone())
                .ok_or_else(|| fail("The destination element is no longer available."))?,
            None => String::new(),
        };
        if !self.index.contains_key(&source) {
            let mut pending = self.source_tree.iter().collect::<Vec<_>>();
            let mut name = None;
            while let Some(n) = pending.pop() {
                if guid(n, "id").ok() == Some(source) {
                    name = n.get("name").and_then(Value::as_str).map(str::to_owned);
                    break;
                }
                pending.extend(children(n));
            }
            let name = name.ok_or_else(|| fail("The source element is not in this map."))?;
            self.mappings.push(Mapping {
                source_id: source,
                source_name: name,
                target_id: None,
                target_name: String::new(),
            });
            self.reindex();
        }
        let row = &mut self.mappings[*self.index.get(&source).unwrap()];
        if row.target_id != target || row.target_name != target_name {
            row.target_id = target;
            row.target_name = target_name;
            self.modified = true;
        }
        Ok(())
    }
    pub fn accepted_pairs(&self) -> Vec<(Uuid, Uuid)> {
        self.mappings
            .iter()
            .filter_map(|m| {
                m.target_id
                    .filter(|id| self.destination.find(id).is_some())
                    .map(|id| (m.source_id, id))
            })
            .collect()
    }
    pub fn tree_rows(&self, source: bool, parent: Option<&str>) -> Vec<TreeRow> {
        let base = parent.unwrap_or("");
        if source {
            let Some(root) = self.source_tree.as_ref() else {
                return vec![];
            };
            let Some(node) = tree_node(root, base) else {
                return vec![];
            };
            children(node)
                .iter()
                .enumerate()
                .filter_map(|(i, n)| {
                    let id = guid(n, "id").ok()?;
                    Some(TreeRow {
                        path: format!("{base}/{i}"),
                        id,
                        name: n["name"].as_str().unwrap_or_default().into(),
                        target_name: self
                            .index
                            .get(&id)
                            .map(|i| self.mappings[*i].target_name.clone())
                            .unwrap_or_default(),
                        child_count: children(n).len(),
                        active: self.active_sources.contains(&id),
                    })
                })
                .collect()
        } else {
            let mut nodes = &self.destination.roots;
            for part in base.split('/').filter(|p| !p.is_empty()) {
                let Some(node) = part.parse::<usize>().ok().and_then(|i| nodes.get(i)) else {
                    return vec![];
                };
                nodes = &self.destination.node(*node).children;
            }
            nodes
                .iter()
                .enumerate()
                .map(|(i, id)| {
                    let n = self.destination.node(*id);
                    TreeRow {
                        path: format!("{base}/{i}"),
                        id: n.id,
                        name: n.name.clone(),
                        target_name: String::new(),
                        child_count: n.children.len(),
                        active: false,
                    }
                })
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fifty_thousand_sources_use_lazy_tree_rows() {
        let start = std::time::Instant::now();
        let sources = (1..=50_000)
            .map(|n| (Uuid::from_u128(n), format!("Source {n:05}")))
            .collect();
        let mut m = ElementMapState::new("Large".into(), sources, Arc::new(ElementTree::new()));
        let groups = (0..500).map(|g| json!({"id":Uuid::from_u128(100_000+g).to_string(),"name":format!("Group {g}"),"children":(1..=100).map(|n|json!({"id":Uuid::from_u128(g*100+n).to_string(),"name":format!("Source {}",g*100+n)})).collect::<Vec<_>>()})).collect::<Vec<_>>();
        let tree = json!({"id":Uuid::nil().to_string(),"name":"Root","children":groups});
        m.load_tree_json(&tree.to_string()).unwrap();
        assert_eq!(m.mappings.len(), 50_000);
        let groups = m.tree_rows(true, None);
        assert_eq!(groups.len(), 500);
        for group in groups {
            let rows = m.tree_rows(true, Some(&group.path));
            assert_eq!(rows.len(), 100);
            assert!(rows.iter().all(|row| row.active));
        }
        eprintln!(
            "50,000 mapper sources + tree parse and all 500 child batches: {:?}",
            start.elapsed()
        );
    }
    fn fixture() -> ElementMapState {
        let mut tree = ElementTree::new();
        tree.add_node(Uuid::from_u128(3), "Destination", Some(Uuid::from_u128(4)));
        ElementMapState::new(
            "Sequence".into(),
            vec![
                (Uuid::from_u128(1), "B".into()),
                (Uuid::from_u128(2), "A".into()),
            ],
            Arc::new(tree),
        )
    }
    #[test]
    fn map_clear_and_subset_do_not_change_profile() {
        let mut m = fixture();
        assert_eq!(m.mappings[0].source_name, "A");
        m.assign(Uuid::from_u128(1), Some(Uuid::from_u128(3)))
            .unwrap();
        assert_eq!(
            m.accepted_pairs(),
            vec![(Uuid::from_u128(1), Uuid::from_u128(3))]
        );
        m.modified = false;
        m.assign(Uuid::from_u128(1), None).unwrap();
        assert!(m.modified);
        assert!(m.accepted_pairs().is_empty());
        assert_eq!(
            m.destination.node(m.destination.roots[0]).name,
            "Destination"
        );
        assert!(m
            .assign(Uuid::from_u128(1), Some(Uuid::from_u128(999)))
            .is_err());
    }
    #[test]
    fn windows_shape_roundtrip_and_failed_load_is_transactional() {
        let mut m = fixture();
        m.assign(Uuid::from_u128(2), Some(Uuid::from_u128(3)))
            .unwrap();
        let json = m.json();
        assert!(json["elementMappings"][1].get("targetId").is_none());
        m.load_json(&json.to_string()).unwrap();
        assert!(!m.modified);
        assert_eq!(m.accepted_pairs().len(), 1);
        let before = m.json();
        assert!(m.load_json(r#"{"id":"bad"}"#).is_err());
        assert_eq!(before, m.json());
    }
    #[test]
    fn duplicate_ids_keep_first_and_missing_sources_can_be_added() {
        let mut m = fixture();
        let mut data = m.json();
        let mut duplicate = data["elementMappings"][0].clone();
        duplicate["sourceName"] = json!("Wrong");
        data["elementMappings"] = json!([data["elementMappings"][0].clone(), duplicate]);
        m.load_json(&data.to_string()).unwrap();
        assert_eq!(m.mappings.len(), 1);
        assert_eq!(m.mappings[0].source_name, "A");
        assert_eq!(m.missing_sources(), 1);
        m.add_missing_sources();
        assert_eq!(m.mappings.len(), 2);
        assert!(m.modified);
    }
    #[test]
    fn saved_map_is_readable_and_failed_save_retains_dirty_state() {
        let mut m = fixture();
        let dir = std::env::temp_dir().join(format!("vixen-map-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("Saved.vmap");
        m.assign(Uuid::from_u128(1), Some(Uuid::from_u128(3)))
            .unwrap();
        m.save(&path).unwrap();
        assert!(!m.modified);
        assert_eq!(m.name, "Saved");
        let expected = m.accepted_pairs();
        m.reset("Other".into());
        m.load(&path).unwrap();
        assert_eq!(m.accepted_pairs(), expected);
        m.assign(Uuid::from_u128(1), None).unwrap();
        assert!(m.save(&dir).is_err());
        assert!(m.modified);
        assert_eq!(m.path.as_deref(), Some(path.as_path()));
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn source_tree_maps_inactive_nodes_and_loading_marks_modified() {
        let mut m = fixture();
        let dir = std::env::temp_dir().join(format!("vixen-tree-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("Tree.v3e");
        let tree = json!({"id":Uuid::new_v4().to_string(),"name":"Root","children":[{"id":Uuid::from_u128(9).to_string(),"name":"Other"}]});
        std::fs::write(&path, tree.to_string()).unwrap();
        m.modified = false;
        m.load_tree(&path).unwrap();
        assert!(m.modified);
        let rows = m.tree_rows(true, None);
        assert_eq!(rows.len(), 1);
        assert!(!rows[0].active);
        m.modified = false;
        let count = m.mappings.len();
        m.assign(rows[0].id, None).unwrap();
        assert_eq!(m.mappings.len(), count);
        assert!(!m.modified);
        m.assign(rows[0].id, Some(Uuid::from_u128(3))).unwrap();
        assert_eq!(m.accepted_pairs().len(), 1);
        assert_eq!(m.json()["sourceTree"], tree);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
