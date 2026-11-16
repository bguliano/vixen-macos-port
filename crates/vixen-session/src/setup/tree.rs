//! The Display Setup working copy of the element tree: `NodeManager`, `ElementNode` and the row logic of `ElementTree`.
//!
//! `vixen_core::ElementTree` is shared read-only with render threads and has no way to remove a node, so edits happen here, in
//! an arena whose ids stay valid for the whole session (deleted slots are tombstones). `to_element_tree` builds the shared form
//! when the window closes with OK.

use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use vixen_core::element::{ElementTree, NodeId, PropertyInstance};

/// A node's slot in the arena. Never reused within a session, so a row path stays meaningful after edits.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Nid(pub u32);

#[derive(Clone, Debug)]
pub struct TreeNode {
    pub id: Uuid,
    pub name: String,
    pub element_id: Option<Uuid>,
    pub children: Vec<Nid>,
    /// Real parents only. Being a child of the invisible `RootNode` is `at_root`.
    pub parents: Vec<Nid>,
    pub at_root: bool,
    /// How many of `parents` come before the root in `ElementNode.Parents`, which lists them in the order the links were made.
    /// Only meaningful with `at_root`. A loaded node has the root last, because `Nodes.AddNodes` runs after the whole file is read.
    pub parents_before_root: u32,
    pub properties: Vec<PropertyInstance>,
    pub tags: Vec<Uuid>,
    pub alive: bool,
}

impl TreeNode {
    fn drop_parent(&mut self, parent: Nid) {
        if let Some(i) = self.parents.iter().position(|p| *p == parent) {
            self.parents.remove(i);
            if (i as u32) < self.parents_before_root {
                self.parents_before_root -= 1;
            }
        }
    }
}

/// What a tree edit means for the data flow graph and the module store. The session applies these.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TreeEffects {
    pub removed_elements: Vec<Uuid>,
    pub added_elements: Vec<(Uuid, Nid)>,
    pub removed_nodes: Vec<Nid>,
    /// `(type id, instance id)` of properties whose data goes with a removed node.
    pub removed_properties: Vec<(Uuid, Uuid)>,
}

impl TreeEffects {
    pub fn merge(&mut self, other: TreeEffects) {
        self.removed_elements.extend(other.removed_elements);
        self.added_elements.extend(other.added_elements);
        self.removed_nodes.extend(other.removed_nodes);
        self.removed_properties.extend(other.removed_properties);
    }
}

/// A tree row: the nodes from a root down to the row's own node. A node under two parents has two rows.
pub type RowPath = Vec<Nid>;

/// Parses `"12/345/6789"`. Row ids are arena slots rather than the GUIDs: they are only meaningful within a session, and short
/// enough that Swift doesn't allocate for them.
pub fn parse_path(text: &str) -> Option<RowPath> {
    if text.is_empty() {
        return None;
    }
    text.split('/').map(|p| p.parse().ok().map(Nid)).collect()
}

pub fn format_path(path: &[Nid]) -> String {
    let mut out = String::with_capacity(path.len() * 6);
    for (i, n) in path.iter().enumerate() {
        if i > 0 {
            out.push('/');
        }
        push_u32(&mut out, n.0);
    }
    out
}

/// `parent_path/child`, in one allocation, because a big group makes one of these per row.
pub fn child_path(parent: Option<&str>, child: Nid) -> String {
    let mut out = String::with_capacity(parent.map_or(0, str::len) + 11);
    if let Some(p) = parent {
        out.push_str(p);
        out.push('/');
    }
    push_u32(&mut out, child.0);
    out
}

fn push_u32(out: &mut String, value: u32) {
    use std::fmt::Write as _;
    let _ = write!(out, "{value}");
}

/// The working element tree.
#[derive(Clone, Debug, Default)]
pub struct SetupTree {
    nodes: Vec<TreeNode>,
    by_id: HashMap<Uuid, Nid>,
    by_element: HashMap<Uuid, Nid>,
    pub roots: Vec<Nid>,
    /// How many live nodes have each name. `_Uniquify` scans all nodes per attempt, which made adding many nodes quadratic.
    name_counts: HashMap<String, u32>,
    /// Where the suffix search for a base name can resume. Dropped whenever a name goes away, so the lowest free suffix is still found.
    suffix_hints: HashMap<String, u32>,
    live: usize,
}

impl SetupTree {
    pub fn new() -> SetupTree {
        let mut t = SetupTree::default();
        // `NodeManager._instances` holds the invisible root, so "Root" is taken.
        t.name_counts.insert("Root".to_string(), 1);
        t
    }

    pub fn from_element_tree(source: &ElementTree) -> SetupTree {
        let mut t = SetupTree::new();
        let map: Vec<Nid> = (0..source.len() as u32).map(Nid).collect();
        t.nodes.reserve(source.len());
        for (id, n) in source.all_nodes() {
            let nid = map[id.0 as usize];
            t.nodes.push(TreeNode {
                id: n.id,
                name: n.name.clone(),
                element_id: n.element_id,
                children: n.children.iter().map(|c| map[c.0 as usize]).collect(),
                parents: n.parents.iter().map(|c| map[c.0 as usize]).collect(),
                at_root: false,
                parents_before_root: n.parents.len() as u32,
                properties: n.properties.clone(),
                tags: n.tags.clone(),
                alive: true,
            });
            t.by_id.insert(n.id, nid);
            if let Some(e) = n.element_id {
                t.by_element.insert(e, nid);
            }
            *t.name_counts.entry(n.name.clone()).or_insert(0) += 1;
        }
        t.live = t.nodes.len();
        t.roots = source.roots.iter().map(|r| map[r.0 as usize]).collect();
        for r in t.roots.clone() {
            t.nodes[r.0 as usize].at_root = true;
        }
        t
    }

    /// The shared, read-only form, with arena order the depth-first order `read_node` gives, so a reload makes the same tree.
    pub fn to_element_tree(&self, color_sets: HashMap<String, Vec<vixen_core::Color>>) -> ElementTree {
        let mut out = ElementTree::new();
        out.color_sets = color_sets;
        let mut map: HashMap<Nid, NodeId> = HashMap::with_capacity(self.live);
        let mut stack: Vec<(Nid, usize)> = Vec::new();
        for &root in &self.roots {
            self.copy_subtree(root, &mut out, &mut map, &mut stack);
        }
        // `add_node` makes every node a root, and `add_child` is quadratic on a wide group, so the links go in directly.
        for (&nid, &id) in &map {
            let n = self.node(nid);
            let target = out.node_mut(id);
            target.children = n.children.iter().filter_map(|c| map.get(c).copied()).collect();
            target.parents = n.parents.iter().filter_map(|c| map.get(c).copied()).collect();
        }
        out.roots = self.roots.iter().filter_map(|r| map.get(r).copied()).collect();
        out
    }

    fn copy_subtree(
        &self,
        root: Nid,
        out: &mut ElementTree,
        map: &mut HashMap<Nid, NodeId>,
        stack: &mut Vec<(Nid, usize)>,
    ) {
        let add = |nid: Nid, out: &mut ElementTree, map: &mut HashMap<Nid, NodeId>| -> bool {
            if map.contains_key(&nid) {
                return false;
            }
            let n = self.node(nid);
            let id = out.add_node(n.id, &n.name, n.element_id);
            let target = out.node_mut(id);
            target.properties = n.properties.clone();
            target.tags = n.tags.clone();
            map.insert(nid, id);
            true
        };
        if !add(root, out, map) {
            return;
        }
        stack.clear();
        stack.push((root, 0));
        while let Some(&(nid, next)) = stack.last() {
            match self.node(nid).children.get(next) {
                Some(&child) => {
                    stack.last_mut().expect("a frame").1 += 1;
                    if add(child, out, map) {
                        stack.push((child, 0));
                    }
                }
                None => {
                    stack.pop();
                }
            }
        }
    }

    #[inline]
    pub fn node(&self, id: Nid) -> &TreeNode {
        &self.nodes[id.0 as usize]
    }

    #[inline]
    pub fn node_mut(&mut self, id: Nid) -> &mut TreeNode {
        &mut self.nodes[id.0 as usize]
    }

    /// False for a slot out of range or one whose node was deleted, which is what a stale row path from the window can name.
    #[inline]
    pub fn is_live(&self, id: Nid) -> bool {
        self.nodes.get(id.0 as usize).is_some_and(|n| n.alive)
    }

    /// How many slots there are, live or not: the size a bitset over nodes needs.
    #[inline]
    pub fn slots(&self) -> usize {
        self.nodes.len()
    }

    /// How many nodes are alive.
    #[inline]
    pub fn len(&self) -> usize {
        self.live
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.live == 0
    }

    pub fn live_nodes(&self) -> impl Iterator<Item = (Nid, &TreeNode)> {
        self.nodes.iter().enumerate().filter(|(_, n)| n.alive).map(|(i, n)| (Nid(i as u32), n))
    }

    pub fn find(&self, id: &Uuid) -> Option<Nid> {
        self.by_id.get(id).copied()
    }

    pub fn find_by_element(&self, element: &Uuid) -> Option<Nid> {
        self.by_element.get(element).copied()
    }

    #[inline]
    pub fn is_leaf(&self, id: Nid) -> bool {
        self.node(id).children.is_empty()
    }

    /// A row path, checked against the tree as it is now: every step really is a child of the one before.
    pub fn resolve(&self, text: &str) -> Option<RowPath> {
        let path = parse_path(text)?;
        self.check_path(&path).then_some(path)
    }

    pub fn check_path(&self, path: &[Nid]) -> bool {
        let Some(&first) = path.first() else { return false };
        if !self.is_live(first) || !self.node(first).at_root {
            return false;
        }
        path.windows(2).all(|w| self.is_live(w[1]) && self.node(w[1]).parents.contains(&w[0]))
    }

    /// `ElementNode.Parents`, with the root as `None`.
    pub fn parent_list(&self, id: Nid) -> impl Iterator<Item = Option<Nid>> + '_ {
        let n = self.node(id);
        n.at_root.then_some(None).into_iter().chain(n.parents.iter().map(|p| Some(*p)))
    }

    // ---- Names ----------------------------------------------------------------------------------------------------

    fn count_name(&mut self, name: &str) {
        *self.name_counts.entry(name.to_string()).or_insert(0) += 1;
    }

    fn uncount_name(&mut self, name: &str) {
        if let Some(c) = self.name_counts.get_mut(name) {
            *c -= 1;
            if *c == 0 {
                self.name_counts.remove(name);
            }
        }
        self.suffix_hints.clear();
    }

    fn name_used(&self, name: &str, ignoring: Option<&str>) -> bool {
        let count = self.name_counts.get(name).copied().unwrap_or(0);
        count > u32::from(ignoring == Some(name))
    }

    /// `_Uniquify`: `name` if free, otherwise `"{name} - 2"`, `"{name} - 3"`, and so on. `ignoring` is a node's own current name
    /// on a rename: upstream counts the node itself, so renaming to the same name gave "X - 2" (F138, fixed here).
    pub fn uniquify(&mut self, name: &str, ignoring: Option<Nid>) -> String {
        let own = ignoring.map(|n| self.node(n).name.clone());
        if !self.name_used(name, own.as_deref()) {
            return name.to_string();
        }
        let mut counter = self.suffix_hints.get(name).copied().unwrap_or(2);
        loop {
            let candidate = format!("{name} - {counter}");
            if !self.name_used(&candidate, own.as_deref()) {
                self.suffix_hints.insert(name.to_string(), counter);
                return candidate;
            }
            counter += 1;
        }
    }

    /// Every name in use, for `NamingUtilities.Uniquify`'s set.
    pub fn name_set(&self) -> HashSet<String> {
        self.name_counts.keys().cloned().collect()
    }

    // ---- Creating ---------------------------------------------------------------------------------------------------

    /// `ElementNodeService.CreateSingle`. A `None` name is "Unnamed". The new node goes at the end of `parent`, or of the root.
    pub fn create_node(
        &mut self,
        parent: Option<Nid>,
        name: Option<&str>,
        with_element: bool,
        uniquify: bool,
    ) -> (Nid, TreeEffects) {
        let name = name.unwrap_or("Unnamed");
        let name = if uniquify { self.uniquify(name, None) } else { name.to_string() };
        let mut effects = TreeEffects::default();
        let id = Nid(self.nodes.len() as u32);
        let node_id = Uuid::new_v4();
        let element_id = with_element.then(Uuid::new_v4);
        self.nodes.push(TreeNode {
            id: node_id,
            name: name.clone(),
            element_id,
            children: Vec::new(),
            parents: Vec::new(),
            at_root: false,
            parents_before_root: 0,
            properties: Vec::new(),
            tags: Vec::new(),
            alive: true,
        });
        self.live += 1;
        self.by_id.insert(node_id, id);
        self.count_name(&name);
        if let Some(e) = element_id {
            self.by_element.insert(e, id);
            effects.added_elements.push((e, id));
        }
        // A new node can't be there already, so this skips the search `AddChild` does.
        match parent {
            None => {
                self.roots.push(id);
                self.nodes[id.0 as usize].at_root = true;
            }
            Some(p) => {
                effects.merge(self.make_group(p));
                self.nodes[p.0 as usize].children.push(id);
                self.nodes[id.0 as usize].parents.push(p);
            }
        }
        (id, effects)
    }

    /// "if an item is a group (or is becoming one), it can't have an output element anymore".
    fn make_group(&mut self, parent: Nid) -> TreeEffects {
        let mut effects = TreeEffects::default();
        if let Some(element) = self.nodes[parent.0 as usize].element_id.take() {
            self.by_element.remove(&element);
            effects.removed_elements.push(element);
        }
        effects
    }

    // ---- Links ------------------------------------------------------------------------------------------------------

    /// `AddChildToParent`: a link, so the child also stays where it was. A child already there stays put, as in `AddChild`.
    pub fn add_child_to_parent(&mut self, child: Nid, parent: Option<Nid>, index: Option<usize>) -> TreeEffects {
        match parent {
            None => {
                if !self.roots.contains(&child) {
                    match index {
                        Some(i) => self.roots.insert(i.min(self.roots.len()), child),
                        None => self.roots.push(child),
                    }
                    let c = &mut self.nodes[child.0 as usize];
                    c.at_root = true;
                    c.parents_before_root = c.parents.len() as u32;
                }
                TreeEffects::default()
            }
            Some(p) => {
                let effects = self.make_group(p);
                let list = &mut self.nodes[p.0 as usize].children;
                if !list.contains(&child) {
                    match index {
                        Some(i) => list.insert(i.min(list.len()), child),
                        None => list.push(child),
                    }
                    let parents = &mut self.nodes[child.0 as usize].parents;
                    if !parents.contains(&p) {
                        parents.push(p);
                    }
                }
                effects
            }
        }
    }

    /// `NodeManager.RemoveNode`: `parent` loses this link. An emptied group becomes a leaf with a brand new element. With `cleanup`,
    /// a node with no links left takes its element and properties with it and unlinks its children, which survive if they have other parents.
    pub fn remove_node(&mut self, node: Nid, parent: Option<Nid>, cleanup: bool) -> TreeEffects {
        let mut effects = TreeEffects::default();
        self.unlink(node, parent, cleanup, &mut effects);
        if let Some(p) = parent {
            let parent_node = &self.nodes[p.0 as usize];
            if parent_node.alive && parent_node.children.is_empty() && parent_node.element_id.is_none() {
                let element = Uuid::new_v4();
                self.nodes[p.0 as usize].element_id = Some(element);
                self.by_element.insert(element, p);
                effects.added_elements.push((element, p));
            }
        }
        effects
    }

    fn unlink(&mut self, node: Nid, parent: Option<Nid>, cleanup: bool, effects: &mut TreeEffects) {
        match parent {
            None => {
                self.roots.retain(|r| *r != node);
                self.nodes[node.0 as usize].at_root = false;
            }
            Some(p) => {
                self.nodes[p.0 as usize].children.retain(|c| *c != node);
                self.nodes[node.0 as usize].drop_parent(p);
            }
        }
        let n = &self.nodes[node.0 as usize];
        if !(cleanup && n.parents.is_empty() && !n.at_root && n.alive) {
            return;
        }
        // An iterative walk: a very deep chain must not run out of stack, and a whole group goes at once.
        let mut stack = vec![node];
        while let Some(current) = stack.pop() {
            let children = std::mem::take(&mut self.nodes[current.0 as usize].children);
            for child in children {
                self.nodes[child.0 as usize].drop_parent(current);
                let c = &self.nodes[child.0 as usize];
                if c.parents.is_empty() && !c.at_root && c.alive {
                    stack.push(child);
                }
            }
            self.forget(current, effects);
        }
    }

    fn forget(&mut self, node: Nid, effects: &mut TreeEffects) {
        let n = &mut self.nodes[node.0 as usize];
        n.alive = false;
        let name = std::mem::take(&mut n.name);
        let id = n.id;
        let element = n.element_id.take();
        let properties = std::mem::take(&mut n.properties);
        self.live -= 1;
        self.by_id.remove(&id);
        if let Some(e) = element {
            self.by_element.remove(&e);
            effects.removed_elements.push(e);
        }
        effects.removed_properties.extend(properties.iter().map(|p| (p.type_id, p.instance_id)));
        effects.removed_nodes.push(node);
        self.uncount_name(&name);
    }

    /// `NodeManager.MoveNode`. Within one parent nothing transient happens: upstream's throwaway element on a lone child is
    /// never seen, so it isn't made.
    pub fn move_node(&mut self, node: Nid, new_parent: Option<Nid>, old_parent: Option<Nid>, index: Option<usize>) -> TreeEffects {
        if new_parent == old_parent {
            let list = match new_parent {
                None => &mut self.roots,
                Some(p) => &mut self.nodes[p.0 as usize].children,
            };
            let Some(current) = list.iter().position(|c| *c == node) else { return TreeEffects::default() };
            let mut target = index;
            if let Some(i) = target {
                if i > current {
                    target = Some(i - 1);
                }
            }
            list.remove(current);
            match target {
                Some(i) => list.insert(i.min(list.len()), node),
                None => list.push(node),
            }
            self.relink_last(node, new_parent);
            return TreeEffects::default();
        }
        let mut effects = self.remove_node(node, old_parent, false);
        effects.merge(self.add_child_to_parent(node, new_parent, index));
        effects
    }

    /// A `MoveNode` inside one parent removes the link and makes it again, so it ends up last in `Parents`.
    fn relink_last(&mut self, node: Nid, parent: Option<Nid>) {
        let n = &mut self.nodes[node.0 as usize];
        match parent {
            None => n.parents_before_root = n.parents.len() as u32,
            Some(p) => {
                n.drop_parent(p);
                n.parents.push(p);
            }
        }
    }

    /// `RenameNode`: no change when the name is the node's own. Otherwise unique against the rest of the system.
    pub fn rename_node(&mut self, node: Nid, name: &str) -> bool {
        if self.node(node).name == name {
            return false;
        }
        let unique = self.uniquify(name, Some(node));
        let old = std::mem::replace(&mut self.nodes[node.0 as usize].name, unique.clone());
        self.count_name(&unique);
        self.uncount_name(&old);
        true
    }

    // ---- Rules ------------------------------------------------------------------------------------------------------

    /// `InvalidChildren`: the node, its direct children, and all its ancestors through every parent. A visited-set walk, because
    /// `GetAllParentNodes` repeats itself exponentially on a diamond.
    pub fn invalid_children(&self, node: Nid) -> HashSet<Nid> {
        let n = self.node(node);
        let mut out: HashSet<Nid> = HashSet::with_capacity(n.children.len() + 8);
        out.insert(node);
        out.extend(n.children.iter().copied());
        let mut stack: Vec<Nid> = n.parents.clone();
        while let Some(p) = stack.pop() {
            if out.insert(p) {
                stack.extend(self.node(p).parents.iter().copied());
            }
        }
        out
    }

    /// `InvalidRootNodes`: the current roots.
    pub fn invalid_root_nodes(&self) -> HashSet<Nid> {
        self.roots.iter().copied().collect()
    }

    /// `CanPaste(asNew)` (F36). `clipboard` is `None` when nothing has been copied.
    pub fn can_paste(&self, clipboard: Option<&[Nid]>, destination: Option<Nid>, as_new: bool) -> bool {
        let Some(clipboard) = clipboard else { return false };
        let invalid = match (destination, as_new) {
            (Some(d), _) => self.invalid_children(d),
            (None, false) => self.invalid_root_nodes(),
            (None, true) => HashSet::new(),
        };
        !clipboard.iter().any(|n| invalid.contains(n))
    }

    /// `CheckAndPromptIfNodeWillLosePatches`' first half: is it a leaf with an element? The session adds "and a destination".
    pub fn element_of_leaf(&self, node: Nid) -> Option<Uuid> {
        let n = self.node(node);
        if n.children.is_empty() {
            n.element_id
        } else {
            None
        }
    }

    /// `PopulateNodeTree(elements)`: each node's row through `Parents.FirstOrDefault()` (F27). Upstream calls this logic flawed: it
    /// decides which row of a multi-parent node gets highlighted. The climb stops at a node whose first parent is the root.
    pub fn first_parent_path(&self, node: Nid) -> RowPath {
        let mut path = vec![node];
        let mut seen: HashSet<Nid> = HashSet::new();
        seen.insert(node);
        let mut current = node;
        loop {
            let n = self.node(current);
            if n.at_root && n.parents_before_root == 0 {
                break;
            }
            match n.parents.first() {
                Some(&p) if seen.insert(p) => {
                    path.push(p);
                    current = p;
                }
                _ => break,
            }
        }
        path.reverse();
        path
    }

    /// F72: every selected row has one parent, and their positions are consecutive in display order.
    pub fn can_reverse(&self, rows: &[RowPath]) -> bool {
        if rows.len() < 2 {
            return false;
        }
        let parent = parent_of(&rows[0]);
        if rows.iter().any(|r| parent_of(r) != parent) {
            return false;
        }
        let list = self.child_list(parent);
        let positions: HashMap<Nid, usize> = list.iter().enumerate().map(|(i, n)| (*n, i)).collect();
        let mut indexes: Vec<usize> = Vec::with_capacity(rows.len());
        for r in rows {
            match r.last().and_then(|n| positions.get(n)) {
                Some(&i) => indexes.push(i),
                None => return false,
            }
        }
        indexes.sort_unstable();
        indexes.windows(2).all(|w| w[1] == w[0] + 1)
    }

    /// F50: each later row in turn goes to the first row's position, which reverses the run.
    pub fn reverse_rows(&mut self, rows: &[RowPath]) -> bool {
        if !self.can_reverse(rows) {
            return false;
        }
        let parent = parent_of(&rows[0]);
        let selected: HashSet<Nid> = rows.iter().filter_map(|r| r.last().copied()).collect();
        let list = match parent {
            None => &mut self.roots,
            Some(p) => &mut self.nodes[p.0 as usize].children,
        };
        let Some(start) = list.iter().position(|n| selected.contains(n)) else { return false };
        list[start..start + selected.len()].reverse();
        // Every row but the first selected one went through `MoveNode`.
        let moved: Vec<Nid> = list[start..start + selected.len() - 1].to_vec();
        for node in moved {
            self.relink_last(node, parent);
        }
        true
    }

    pub fn child_list(&self, parent: Option<Nid>) -> &[Nid] {
        match parent {
            None => &self.roots,
            Some(p) => &self.node(p).children,
        }
    }

    /// F51: `OrderBy(n => n.Name)`. .NET's default comparison is culture-aware: letters compare without case first, lower before upper to
    /// break ties, and digits by character, so "Item 10" sorts before "Item 2".
    pub fn sort_children(&mut self, group: Nid) {
        let mut list = std::mem::take(&mut self.nodes[group.0 as usize].children);
        list.sort_by_cached_key(|c| culture_key(&self.node(*c).name));
        // Each child goes through `MoveNode`, whether or not it changes place.
        for child in &list {
            self.relink_last(*child, Some(group));
        }
        self.nodes[group.0 as usize].children = list;
    }

    // ---- Enumerators -------------------------------------------------------------------------------------------------

    /// `GetLeafEnumerator()` appended to `out`: depth first in child order, once per path, as the C# does.
    pub fn collect_leaves(&self, node: Nid, out: &mut Vec<Nid>) {
        let mut stack = vec![(node, 0usize)];
        if self.is_leaf(node) {
            out.push(node);
            return;
        }
        while let Some(&(nid, next)) = stack.last() {
            match self.node(nid).children.get(next) {
                Some(&child) => {
                    stack.last_mut().expect("a frame").1 += 1;
                    if self.is_leaf(child) {
                        out.push(child);
                    } else {
                        stack.push((child, 0));
                    }
                }
                None => {
                    stack.pop();
                }
            }
        }
    }

    pub fn distinct_leaves(&self, nodes: impl IntoIterator<Item = Nid>) -> Vec<Nid> {
        let mut seen = BitSet::new(self.slots());
        let mut out = Vec::new();
        let mut scratch = Vec::new();
        for n in nodes {
            scratch.clear();
            self.collect_leaves(n, &mut scratch);
            for &leaf in &scratch {
                if seen.insert(leaf.0 as usize) {
                    out.push(leaf);
                }
            }
        }
        out
    }

    /// `GetNodeEnumerator()`: this node and all below it, depth first, once per path.
    pub fn descendants(&self, node: Nid) -> Vec<Nid> {
        let mut out = vec![node];
        let mut stack: Vec<(Nid, usize)> = vec![(node, 0)];
        while let Some(&(nid, next)) = stack.last() {
            match self.node(nid).children.get(next) {
                Some(&child) => {
                    stack.last_mut().expect("a frame").1 += 1;
                    out.push(child);
                    stack.push((child, 0));
                }
                None => {
                    stack.pop();
                }
            }
        }
        out
    }
}

/// The parent of a row: `None` for a root row.
#[inline]
pub fn parent_of(path: &[Nid]) -> Option<Nid> {
    path.len().checked_sub(2).map(|i| path[i])
}

/// A sort key that matches .NET's default comparison closely enough for names: case folded, with the original as a tie break where lower
/// comes before upper.
fn culture_key(name: &str) -> (String, String) {
    let folded = name.to_lowercase();
    let tie: String = name.chars().map(|c| if c.is_uppercase() { 'U' } else { 'L' }).collect();
    (folded, tie)
}

/// A fixed-size set of small integers, for walks over the arena.
#[derive(Clone, Debug)]
pub struct BitSet {
    words: Vec<u64>,
}

impl BitSet {
    pub fn new(size: usize) -> BitSet {
        BitSet { words: vec![0; size / 64 + 1] }
    }

    /// True if it wasn't there before.
    #[inline]
    pub fn insert(&mut self, i: usize) -> bool {
        let (w, b) = (i / 64, 1u64 << (i % 64));
        if self.words[w] & b == 0 {
            self.words[w] |= b;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn contains(&self, i: usize) -> bool {
        self.words[i / 64] & (1u64 << (i % 64)) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(t: &SetupTree, list: &[Nid]) -> Vec<String> {
        list.iter().map(|n| t.node(*n).name.clone()).collect()
    }

    fn leaf(t: &mut SetupTree, parent: Option<Nid>, name: &str) -> Nid {
        t.create_node(parent, Some(name), true, true).0
    }

    fn group(t: &mut SetupTree, parent: Option<Nid>, name: &str) -> Nid {
        t.create_node(parent, Some(name), false, true).0
    }

    /// `[A,B,C,D]` at the root.
    fn abcd(t: &mut SetupTree) -> [Nid; 4] {
        [leaf(t, None, "A"), leaf(t, None, "B"), leaf(t, None, "C"), leaf(t, None, "D")]
    }

    #[test]
    fn uniquify_uses_the_next_free_suffix() {
        let mut t = SetupTree::new();
        assert_eq!(t.uniquify("foo", None), "foo");
        leaf(&mut t, None, "foo");
        assert_eq!(t.uniquify("foo", None), "foo - 2");
        leaf(&mut t, None, "foo");
        assert_eq!(t.uniquify("foo", None), "foo - 3");
        // Case matters.
        assert_eq!(t.uniquify("Foo", None), "Foo");
    }

    #[test]
    fn the_lowest_free_suffix_comes_back_after_a_removal() {
        let mut t = SetupTree::new();
        leaf(&mut t, None, "X");
        let second = leaf(&mut t, None, "X");
        leaf(&mut t, None, "X");
        assert_eq!(t.node(second).name, "X - 2");
        t.remove_node(second, None, true);
        assert_eq!(t.uniquify("X", None), "X - 2");
    }

    #[test]
    fn root_is_taken_by_the_invisible_root() {
        let mut t = SetupTree::new();
        assert_eq!(t.uniquify("Root", None), "Root - 2");
    }

    #[test]
    fn create_node_with_no_name_is_unnamed() {
        let mut t = SetupTree::new();
        let a = t.create_node(None, None, true, true).0;
        let b = t.create_node(None, None, true, true).0;
        assert_eq!(names(&t, &[a, b]), ["Unnamed", "Unnamed - 2"]);
    }

    #[test]
    fn rename_to_own_name_changes_nothing_and_a_taken_name_is_uniquified() {
        let mut t = SetupTree::new();
        let a = leaf(&mut t, None, "A");
        let _b = leaf(&mut t, None, "B");
        assert!(!t.rename_node(a, "A"));
        assert_eq!(t.node(a).name, "A");
        assert!(t.rename_node(a, "B"));
        assert_eq!(t.node(a).name, "B - 2");
        // The old name is free again.
        assert_eq!(t.uniquify("A", None), "A");
    }

    #[test]
    fn moves_in_the_root() {
        let mut t = SetupTree::new();
        let [a, b, c, d] = abcd(&mut t);
        // Drag A, B to below D: index 4, which stays put as each moves later.
        t.move_node(a, None, None, Some(4));
        t.move_node(b, None, None, Some(4));
        assert_eq!(t.roots, [c, d, a, b]);

        let mut t = SetupTree::new();
        let [a, b, c, d] = abcd(&mut t);
        // Drag C, D to above A: index 0, then 1, as each moves earlier.
        t.move_node(c, None, None, Some(0));
        t.move_node(d, None, None, Some(1));
        assert_eq!(t.roots, [c, d, a, b]);

        let mut t = SetupTree::new();
        let [a, b, c, d] = abcd(&mut t);
        // Drag A to below B.
        t.move_node(a, None, None, Some(2));
        assert_eq!(t.roots, [b, a, c, d]);
        let _ = d;
    }

    #[test]
    fn a_leaf_parent_loses_its_element_when_it_gets_a_child() {
        let mut t = SetupTree::new();
        let a = leaf(&mut t, None, "A");
        let b = leaf(&mut t, None, "B");
        let element = t.node(a).element_id.unwrap();
        let effects = t.add_child_to_parent(b, Some(a), None);
        assert_eq!(effects.removed_elements, vec![element]);
        assert_eq!(t.node(a).element_id, None);
        assert_eq!(t.find_by_element(&element), None);
        assert_eq!(t.node(b).parents, vec![a]);
        // It stays at the root too: this is a link.
        assert!(t.node(b).at_root);
    }

    #[test]
    fn a_node_with_another_parent_only_loses_one_link() {
        let mut t = SetupTree::new();
        let g1 = group(&mut t, None, "G1");
        let g2 = group(&mut t, None, "G2");
        let n = leaf(&mut t, Some(g1), "N");
        t.add_child_to_parent(n, Some(g2), None);
        let element = t.node(n).element_id;
        let effects = t.remove_node(n, Some(g1), true);
        assert!(t.is_live(n));
        assert_eq!(t.node(n).element_id, element);
        assert_eq!(t.node(n).parents, vec![g2]);
        assert!(effects.removed_nodes.is_empty());
    }

    #[test]
    fn the_last_link_takes_the_node_its_element_and_properties() {
        let mut t = SetupTree::new();
        let g = group(&mut t, None, "G");
        let keep = group(&mut t, None, "Keep");
        let shared = leaf(&mut t, Some(g), "Shared");
        let only = leaf(&mut t, Some(g), "Only");
        t.add_child_to_parent(shared, Some(keep), None);
        let prop = PropertyInstance {
            type_id: vixen_core::element::property_ids::ORDER,
            instance_id: Uuid::new_v4(),
            data: vixen_core::element::PropertyData::Order { order: 1 },
        };
        t.node_mut(only).properties.push(prop.clone());
        let only_element = t.node(only).element_id.unwrap();

        let effects = t.remove_node(g, None, true);
        assert!(!t.is_live(g));
        assert!(!t.is_live(only));
        assert!(t.is_live(shared));
        assert_eq!(t.node(shared).parents, vec![keep]);
        assert_eq!(effects.removed_elements, vec![only_element]);
        assert_eq!(effects.removed_properties, vec![(prop.type_id, prop.instance_id)]);
        assert_eq!(t.find(&Uuid::new_v4()), None);
        // Their names are free again.
        assert_eq!(t.uniquify("Only", None), "Only");
    }

    #[test]
    fn an_emptied_group_becomes_a_leaf_with_a_new_element() {
        let mut t = SetupTree::new();
        let g = group(&mut t, None, "G");
        let n = leaf(&mut t, Some(g), "N");
        let effects = t.remove_node(n, Some(g), true);
        let element = t.node(g).element_id.expect("a new element");
        assert_eq!(effects.added_elements, vec![(element, g)]);
        assert_eq!(t.find_by_element(&element), Some(g));
        assert!(t.is_leaf(g));
    }

    #[test]
    fn moving_the_only_child_out_leaves_a_leaf_behind_but_within_its_parent_changes_nothing() {
        let mut t = SetupTree::new();
        let g = group(&mut t, None, "G");
        let n = leaf(&mut t, Some(g), "N");
        let effects = t.move_node(n, Some(g), Some(g), Some(0));
        assert_eq!(effects, TreeEffects::default());
        assert_eq!(t.node(g).element_id, None);
        let effects = t.move_node(n, None, Some(g), None);
        assert_eq!(effects.added_elements.len(), 1);
        assert!(t.node(n).at_root);
    }

    #[test]
    fn invalid_children_is_self_children_and_all_ancestors_but_not_grandchildren() {
        let mut t = SetupTree::new();
        let top = group(&mut t, None, "Top");
        let other = group(&mut t, None, "Other");
        let mid = group(&mut t, Some(top), "Mid");
        t.add_child_to_parent(mid, Some(other), None);
        let child = group(&mut t, Some(mid), "Child");
        let grandchild = leaf(&mut t, Some(child), "Grandchild");
        let invalid = t.invalid_children(mid);
        for n in [mid, child, top, other] {
            assert!(invalid.contains(&n));
        }
        assert!(!invalid.contains(&grandchild));
    }

    #[test]
    fn can_paste_follows_f36() {
        let mut t = SetupTree::new();
        let g = group(&mut t, None, "G");
        let n = leaf(&mut t, Some(g), "N");
        let free = leaf(&mut t, None, "Free");
        assert!(!t.can_paste(None, Some(g), false));
        assert!(t.can_paste(Some(&[]), Some(g), false));
        // Already a child, or a root when pasting at the root.
        assert!(!t.can_paste(Some(&[n]), Some(g), false));
        assert!(!t.can_paste(Some(&[free]), None, false));
        // Into a group it isn't in.
        assert!(t.can_paste(Some(&[free]), Some(g), false));
        // Pasting as new at the root can't clash with anything.
        assert!(t.can_paste(Some(&[free]), None, true));
        // Into itself is refused even as new.
        assert!(!t.can_paste(Some(&[g]), Some(g), true));
    }

    #[test]
    fn reverse_needs_one_parent_and_consecutive_rows() {
        let mut t = SetupTree::new();
        let [a, b, c, d] = abcd(&mut t);
        let row = |n: Nid| vec![n];
        assert!(t.can_reverse(&[row(b), row(c), row(d)]));
        assert!(!t.can_reverse(&[row(a), row(c)]));
        assert!(!t.can_reverse(&[row(a)]));
        assert!(t.reverse_rows(&[row(b), row(c), row(d)]));
        assert_eq!(t.roots, [a, d, c, b]);

        let g = group(&mut t, None, "G");
        let n = leaf(&mut t, Some(g), "N");
        assert!(!t.can_reverse(&[row(a), vec![g, n]]));
    }

    #[test]
    fn sort_is_not_natural_order() {
        let mut t = SetupTree::new();
        let g = group(&mut t, None, "G");
        for n in ["Item 2", "Item 10", "item 1", "Beta", "alpha"] {
            leaf(&mut t, Some(g), n);
        }
        t.sort_children(g);
        let list = t.node(g).children.clone();
        assert_eq!(names(&t, &list), ["alpha", "Beta", "item 1", "Item 10", "Item 2"]);
    }

    #[test]
    fn paths_round_trip_and_stale_ones_are_refused() {
        let mut t = SetupTree::new();
        let g = group(&mut t, None, "G");
        let n = leaf(&mut t, Some(g), "N");
        let text = format_path(&[g, n]);
        assert_eq!(t.resolve(&text), Some(vec![g, n]));
        assert_eq!(child_path(Some(&format_path(&[g])), n), text);
        assert_eq!(child_path(None, g), format_path(&[g]));
        assert_eq!(t.resolve("not/a/path"), None);
        assert_eq!(t.resolve(&format_path(&[n])), None, "not a root");
        t.remove_node(n, Some(g), true);
        assert_eq!(t.resolve(&text), None);
    }

    #[test]
    fn first_parent_path_climbs_first_parents() {
        let mut t = SetupTree::new();
        let a = group(&mut t, None, "A");
        let b = group(&mut t, None, "B");
        let n = leaf(&mut t, Some(a), "N");
        t.add_child_to_parent(n, Some(b), None);
        assert_eq!(t.first_parent_path(n), vec![a, n]);
        assert_eq!(t.first_parent_path(b), vec![b]);
    }

    #[test]
    fn first_parent_path_follows_the_order_the_links_were_made() {
        // ElementTree.cs:243-254 with GroupNode.cs:43-48, :78-84 and NodeManager.cs:37-53.
        let mut t = SetupTree::new();
        let a = group(&mut t, None, "A");
        let b = group(&mut t, None, "B");
        // Made at the root, then linked into a group: the root is still the first parent.
        let n = leaf(&mut t, None, "N");
        t.add_child_to_parent(n, Some(a), None);
        assert_eq!(t.first_parent_path(n), vec![n]);
        // Off the root and back on: now the group came first.
        t.remove_node(n, None, false);
        t.add_child_to_parent(n, None, None);
        assert_eq!(t.first_parent_path(n), vec![a, n]);
        // Losing the group in front of the root makes the root first again.
        t.add_child_to_parent(n, Some(b), None);
        t.remove_node(n, Some(a), false);
        assert_eq!(t.first_parent_path(n), vec![n]);

        // A move inside one parent removes the link and makes it again, last.
        let m = leaf(&mut t, Some(a), "M");
        let other = leaf(&mut t, Some(a), "Other");
        t.add_child_to_parent(m, Some(b), None);
        assert_eq!(t.first_parent_path(m), vec![a, m]);
        t.move_node(m, Some(a), Some(a), None);
        assert_eq!(t.node(a).children, vec![other, m]);
        assert_eq!(t.first_parent_path(m), vec![b, m]);

        // A node the file lists at the top and in a group has the root last.
        let mut source = ElementTree::new();
        let g = source.add_node(Uuid::new_v4(), "G", None);
        let x = source.add_node(Uuid::new_v4(), "X", Some(Uuid::new_v4()));
        source.add_child(g, x);
        source.roots = vec![x, g];
        let loaded = SetupTree::from_element_tree(&source);
        assert_eq!(loaded.first_parent_path(Nid(x.0)), vec![Nid(g.0), Nid(x.0)]);
    }

    #[test]
    fn reverse_and_sort_remake_the_links_of_the_rows_they_move() {
        let mut t = SetupTree::new();
        let g = group(&mut t, None, "G");
        let h = group(&mut t, None, "H");
        let b = leaf(&mut t, Some(g), "B");
        let a = leaf(&mut t, Some(g), "A");
        let c = leaf(&mut t, Some(g), "C");
        for n in [a, b, c] {
            t.add_child_to_parent(n, Some(h), None);
        }
        // Reverse B, A under G: A goes through `MoveNode` and B, the first selected, doesn't.
        assert!(t.reverse_rows(&[vec![g, b], vec![g, a]]));
        assert_eq!(t.node(g).children, [a, b, c]);
        assert_eq!(t.first_parent_path(a), vec![h, a]);
        assert_eq!(t.first_parent_path(b), vec![g, b]);
        assert_eq!(t.first_parent_path(c), vec![g, c]);
        // Sort moves every child, in place or not.
        t.sort_children(g);
        assert_eq!(t.node(g).children, [a, b, c]);
        for n in [a, b, c] {
            assert_eq!(t.first_parent_path(n), vec![h, n]);
        }
    }

    #[test]
    fn leaves_repeat_once_per_path_and_distinct_keeps_first_seen() {
        let mut t = SetupTree::new();
        let a = group(&mut t, None, "A");
        let b = group(&mut t, None, "B");
        let x = leaf(&mut t, Some(a), "X");
        let y = leaf(&mut t, Some(b), "Y");
        t.add_child_to_parent(x, Some(b), None);
        let mut out = Vec::new();
        t.collect_leaves(a, &mut out);
        t.collect_leaves(b, &mut out);
        assert_eq!(out, [x, y, x]);
        assert_eq!(t.distinct_leaves([b, a]), [y, x]);
    }

    #[test]
    fn converting_back_keeps_order_links_and_ids() {
        let mut t = SetupTree::new();
        let g = group(&mut t, None, "G");
        let a = leaf(&mut t, Some(g), "A");
        let b = leaf(&mut t, Some(g), "B");
        let r = leaf(&mut t, None, "R");
        t.add_child_to_parent(a, None, None);
        let removed = leaf(&mut t, None, "Gone");
        t.remove_node(removed, None, true);

        let out = t.to_element_tree(HashMap::new());
        assert_eq!(out.len(), 4);
        let names: Vec<_> = out.roots.iter().map(|n| out.node(*n).name.clone()).collect();
        assert_eq!(names, ["G", "R", "A"]);
        let gid = out.find(&t.node(g).id).unwrap();
        let kids: Vec<_> = out.node(gid).children.iter().map(|n| out.node(*n).name.clone()).collect();
        assert_eq!(kids, ["A", "B"]);
        assert_eq!(out.node(out.find(&t.node(a).id).unwrap()).parents, vec![gid]);
        assert_eq!(out.node(out.find(&t.node(b).id).unwrap()).element_id, t.node(b).element_id);
        // Arena order is depth first, as a load makes it.
        let order: Vec<_> = out.all_nodes().map(|(_, n)| n.name.clone()).collect();
        assert_eq!(order, ["G", "A", "B", "R"]);
        let _ = r;

        let back = SetupTree::from_element_tree(&out);
        assert_eq!(back.len(), 4);
        assert_eq!(back.roots.len(), 3);
    }
}
