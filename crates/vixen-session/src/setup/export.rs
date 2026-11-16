//! Export Element Tree (F53): the `.v3e` JSON that `ElementTreeWriter` writes through Newtonsoft: indented, camelCase, nulls and empty lists left out. Its root is the
//! invisible Root node. A multi-parent node appears once per parent.

use super::tree::{Nid, SetupTree};
use std::path::Path;
use uuid::Uuid;

fn escape(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn indent(depth: usize, out: &mut String) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn member(depth: usize, name: &str, value: &str, out: &mut String) {
    indent(depth, out);
    out.push('"');
    out.push_str(name);
    out.push_str("\": ");
    escape(value, out);
}

fn write_children(tree: &SetupTree, children: &[Nid], depth: usize, out: &mut String) {
    if children.is_empty() {
        return;
    }
    out.push_str(",\n");
    indent(depth, out);
    out.push_str("\"children\": [\n");
    for (i, c) in children.iter().enumerate() {
        write_node(tree, *c, depth + 1, out);
        if i + 1 < children.len() {
            out.push(',');
        }
        out.push('\n');
    }
    indent(depth, out);
    out.push(']');
}

fn write_node(tree: &SetupTree, node: Nid, depth: usize, out: &mut String) {
    let n = tree.node(node);
    indent(depth, out);
    out.push_str("{\n");
    member(depth + 1, "id", &n.id.to_string(), out);
    out.push_str(",\n");
    member(depth + 1, "name", &n.name, out);
    if let Some(element) = n.element_id {
        out.push_str(",\n");
        indent(depth + 1, out);
        out.push_str("\"element\": {\n");
        member(depth + 2, "id", &element.to_string(), out);
        out.push_str(",\n");
        member(depth + 2, "name", &n.name, out);
        out.push('\n');
        indent(depth + 1, out);
        out.push('}');
    }
    write_children(tree, &n.children, depth + 1, out);
    out.push('\n');
    indent(depth, out);
    out.push('}');
}

/// The whole tree as `ExportElementNodeProxy` writes it.
pub fn element_tree_json(tree: &SetupTree, root_id: Uuid) -> String {
    let mut out = String::with_capacity(tree.len() * 96);
    out.push_str("{\n");
    member(1, "id", &root_id.to_string(), &mut out);
    out.push_str(",\n");
    member(1, "name", "Root", &mut out);
    write_children(tree, &tree.roots, 1, &mut out);
    out.push_str("\n}");
    out
}

/// F53. Whatever is selected, this exports the whole tree.
pub fn write_element_tree(tree: &SetupTree, path: &Path) -> Result<(), crate::SessionError> {
    let text = element_tree_json(tree, Uuid::new_v4());
    vixen_io::file::write_atomic(path, text.as_bytes()).map_err(crate::SessionError::Io)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shape_is_what_the_writer_makes() {
        let mut t = SetupTree::new();
        let g = t.create_node(None, Some("Arch \"A\""), false, true).0;
        let leaf = t.create_node(Some(g), Some("Arch Px-1"), true, true).0;
        let root = Uuid::from_u128(1);
        let text = element_tree_json(&t, root);
        let (gid, lid, eid) = (t.node(g).id, t.node(leaf).id, t.node(leaf).element_id.unwrap());
        let expected = format!(
            "{{\n  \"id\": \"{root}\",\n  \"name\": \"Root\",\n  \"children\": [\n    {{\n      \"id\": \"{gid}\",\n      \"name\": \"Arch \\\"A\\\"\",\n      \"children\": [\n        {{\n          \"id\": \"{lid}\",\n          \"name\": \"Arch Px-1\",\n          \"element\": {{\n            \"id\": \"{eid}\",\n            \"name\": \"Arch Px-1\"\n          }}\n        }}\n      ]\n    }}\n  ]\n}}"
        );
        assert_eq!(text, expected);
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["children"][0]["children"][0]["element"]["id"], eid.to_string());
    }

    #[test]
    fn an_empty_tree_has_no_children_member_and_a_shared_node_is_written_per_parent() {
        let t = SetupTree::new();
        assert_eq!(element_tree_json(&t, Uuid::nil()), format!("{{\n  \"id\": \"{}\",\n  \"name\": \"Root\"\n}}", Uuid::nil()));

        let mut t = SetupTree::new();
        let a = t.create_node(None, Some("A"), false, true).0;
        let b = t.create_node(None, Some("B"), false, true).0;
        let n = t.create_node(Some(a), Some("N"), true, true).0;
        t.add_child_to_parent(n, Some(b), None);
        let parsed: serde_json::Value = serde_json::from_str(&element_tree_json(&t, Uuid::nil())).unwrap();
        assert_eq!(parsed["children"][0]["children"][0]["name"], "N");
        assert_eq!(parsed["children"][1]["children"][0]["name"], "N");
    }
}
