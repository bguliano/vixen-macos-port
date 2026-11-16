//! State-tab bulk view and commands; document state remains in vixen-prop.
use super::*;
use vixen_core::Color;

#[derive(uniffi::Record, Clone)]
pub struct PropEdStateDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
}
#[derive(uniffi::Record, Clone)]
pub struct PropEdStateItem {
    pub id: String,
    pub name: String,
    pub color: u32,
    pub color_hex: String,
    pub count: u32,
    pub selected: bool,
}
#[derive(uniffi::Record, Clone)]
pub struct PropEdStates {
    pub definitions: Vec<PropEdStateDefinition>,
    pub selected_definition: Option<String>,
    pub items: Vec<PropEdStateItem>,
    pub validation: Vec<String>,
    pub suggested_name: String,
    pub can_move_up: bool,
    pub can_move_down: bool,
}

fn parse_color(text: &str) -> Option<Color> {
    let text = text.trim();
    if let Some(hex) = text.strip_prefix('#') {
        if !hex.chars().all(|c| c.is_ascii_hexdigit()) { return None }
        let full = match hex.len() { 3 => hex.chars().flat_map(|c| [c,c]).collect(), 6 | 8 => hex.to_string(), _ => return None };
        let value = u32::from_str_radix(&full, 16).ok()?;
        return Some(Color::from_argb_u32(if full.len() == 6 { 0xff00_0000 | value } else { value }));
    }
    // CSS aliases absent from System.Drawing's KnownColor table.
    let name = text.to_lowercase().replace("grey", "gray");
    if name == "rebeccapurple" { return Some(Color::rgb(102, 51, 153)) }
    vixen_output::dotnet::known::by_name(&name).and_then(vixen_output::dotnet::known::argb).map(Color::from_argb_u32)
}

#[uniffi::export]
impl CustomPropEditor {
    pub fn states(&self) -> PropEdStates {
        let e = self.inner.lock(); let selected = e.state_selected_items();
        let at = if selected.len() == 1 { e.state_items().iter().position(|i| i.id == selected[0]) } else { None };
        PropEdStates {
            definitions: e.state_definitions().iter().map(|d| PropEdStateDefinition { id: d.id.to_string(), name: d.name.clone(), description: d.description.clone() }).collect(),
            selected_definition: e.selected_state_definition().map(|id| id.to_string()),
            items: e.state_items().iter().map(|i| PropEdStateItem {
                id: i.id.to_string(), name: i.name.clone(), color: i.color.to_argb_u32(), color_hex: format!("#{:02X}{:02X}{:02X}", i.color.r, i.color.g, i.color.b),
                count: i.element_ids.iter().filter(|id| e.prop().contains(**id)).collect::<std::collections::HashSet<_>>().len() as u32, selected: selected.contains(&i.id),
            }).collect(),
            validation: e.state_validation(), suggested_name: e.next_state_definition_name(),
            can_move_up: at.is_some_and(|at| at > 0), can_move_down: at.is_some_and(|at| at + 1 < e.state_items().len()),
        }
    }
    pub fn select_state_definition(&self, id: Option<String>) { self.inner.lock().select_state_definition(id.and_then(|s| Uuid::parse_str(&s).ok())); }
    pub fn select_state_items(&self, ids: Vec<String>) { self.inner.lock().select_state_items(ids.iter().filter_map(|s| Uuid::parse_str(s).ok()).collect()); }
    pub fn add_state_definition(&self, name: String) -> Result<(), FfiError> { self.inner.lock().add_state_definition(&name)?; Ok(()) }
    pub fn rename_state_definition(&self, name: String) -> Result<bool, FfiError> { Ok(self.inner.lock().rename_state_definition(&name)?) }
    pub fn copy_state_definition(&self, name: String) -> Result<bool, FfiError> { Ok(self.inner.lock().copy_state_definition(&name)?) }
    pub fn delete_state_definition(&self) -> bool { self.inner.lock().delete_state_definition() }
    pub fn set_state_description(&self, text: String) -> bool { self.inner.lock().set_state_description(&text) }
    pub fn add_state_item(&self, copy: bool) -> bool { self.inner.lock().add_state_item(copy) }
    pub fn remove_state_items(&self) -> bool { self.inner.lock().remove_state_items() }
    pub fn move_state_item(&self, down: bool) -> bool { self.inner.lock().move_state_item(down) }
    pub fn edit_state_item(&self, id: String, name: Option<String>, color_text: Option<String>) -> bool {
        let Ok(id) = Uuid::parse_str(&id) else { return false };
        let mut e = self.inner.lock(); let Some(item) = e.state_items().iter().find(|i| i.id == id).cloned() else { return false };
        let color = match color_text { Some(s) => match parse_color(&s) { Some(c) => c, None => return false }, None => item.color };
        e.edit_state_item(id, name.as_deref().unwrap_or(&item.name), color)
    }
    pub fn sort_state_items(&self, column: String, ascending: bool) -> bool {
        let mut e = self.inner.lock(); let mut items = e.state_items().to_vec();
        let direction = |order: std::cmp::Ordering| if ascending { order } else { order.reverse() };
        match column.as_str() {
            "name" => items.sort_by(|a,b| direction(a.name.to_lowercase().cmp(&b.name.to_lowercase()))),
            "color" => items.sort_by(|a,b| direction((a.color.to_argb_u32() & 0xffffff).cmp(&(b.color.to_argb_u32() & 0xffffff)))),
            "count" => { let count = |i: &vixen_prop::StateItemModel| i.element_ids.iter().filter(|id| e.prop().contains(**id)).collect::<std::collections::HashSet<_>>().len(); items.sort_by(|a,b| direction(count(a).cmp(&count(b)))); },
            _ => return false,
        }
        e.reorder_state_items(&items.iter().map(|i| i.id).collect::<Vec<_>>())
    }
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn state_color_formats_and_invalid_edits() {
        assert_eq!(parse_color("#123"), Some(Color::rgb(17,34,51)));
        assert_eq!(parse_color("#80112233"), Some(Color::argb(128,17,34,51)));
        assert_eq!(parse_color("LightGrey"), Some(Color::rgb(211,211,211)));
        assert_eq!(parse_color("blue"), Some(Color::BLUE));
        assert_eq!(parse_color("#nope"), None);
        let editor = CustomPropEditor::new("test".into()); editor.add_state_definition("State".into()).unwrap();
        let before = editor.state().revision; let item = editor.states().items[0].clone();
        assert!(!editor.edit_state_item(item.id, None, Some("garbage".into())));
        assert_eq!(editor.state().revision, before);
    }
}
