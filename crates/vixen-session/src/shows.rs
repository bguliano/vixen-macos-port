//! Shows (`VixenModules.App.Shows`): the data in `SystemData/ModuleStore.xml`, the Shows list and the show editor's rules. A show the
//! user hasn't changed goes back exactly as it was read, including members that have no editor now.

use std::path::{Path, PathBuf};
use uuid::Uuid;
use vixen_io::dcs::{format_bool, format_guid, parse_bool, parse_guid, windows_file_name};
use vixen_io::{ModuleEntry, ModuleStore};
use vixen_xml::XElement;

/// `ShowsDescriptor`'s type id. Its data is the module's static data, keyed by this.
pub const MODULE_TYPE: Uuid = uuid::uuid!("67a6dfdb-ddec-4a5f-a158-129421e2366f");
const DATA_MODEL_TYPE: &str = "VixenModules.App.Shows.ShowsData, Shows";
const ASSEMBLY_NAME: &str = "Module.App.Shows";
const NS: &str = "http://schemas.datacontract.org/2004/07/VixenModules.App.Shows";

pub const HELP_URL: &str = "https://www.vixenlights.com/docs/usage/scheduler/schedule-show-editor/";

/// `ShowItemType`, which is the editor's tab. `All` and `Input` have no tab.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ShowItemType {
    All,
    Startup,
    Background,
    Sequential,
    Input,
    Shutdown,
    /// Text the enum doesn't have, kept as written.
    Other(String),
}

impl ShowItemType {
    pub const TABS: [ShowItemType; 4] = [ShowItemType::Startup, ShowItemType::Background, ShowItemType::Sequential, ShowItemType::Shutdown];

    fn parse(text: &str) -> ShowItemType {
        match text {
            "All" => ShowItemType::All,
            "Startup" => ShowItemType::Startup,
            "Background" => ShowItemType::Background,
            "Sequential" => ShowItemType::Sequential,
            "Input" => ShowItemType::Input,
            "Shutdown" => ShowItemType::Shutdown,
            other => ShowItemType::Other(other.to_string()),
        }
    }

    pub fn text(&self) -> &str {
        match self {
            ShowItemType::All => "All",
            ShowItemType::Startup => "Startup",
            ShowItemType::Background => "Background",
            ShowItemType::Sequential => "Sequential",
            ShowItemType::Input => "Input",
            ShowItemType::Shutdown => "Shutdown",
            ShowItemType::Other(text) => text,
        }
    }

    /// The line under the list (F43).
    pub fn help(&self) -> &'static str {
        match self {
            ShowItemType::Startup => "These items are run once, in order, during startup.",
            ShowItemType::Background => "These items are always run in the background.",
            ShowItemType::Sequential => "These items are run, in order, and re-started while the show is running.",
            ShowItemType::Shutdown => "These items are run, in order, at the end of the show.",
            _ => "",
        }
    }
}

/// `ActionType`. GUARD: `Show` and `WebPage` are commented out of upstream's enum, so a file that has one fails its whole load; here the
/// text is kept, gets no editor, and runs as a pause of no length.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ActionType {
    Launch,
    Sequence,
    Pause,
    Other(String),
}

impl ActionType {
    /// The `Action:` list, in the enum's order.
    pub const CHOICES: [ActionType; 3] = [ActionType::Launch, ActionType::Sequence, ActionType::Pause];

    fn parse(text: &str) -> ActionType {
        match text {
            "Launch" => ActionType::Launch,
            "Sequence" => ActionType::Sequence,
            "Pause" => ActionType::Pause,
            other => ActionType::Other(other.to_string()),
        }
    }

    pub fn text(&self) -> &str {
        match self {
            ActionType::Launch => "Launch",
            ActionType::Sequence => "Sequence",
            ActionType::Pause => "Pause",
            ActionType::Other(text) => text,
        }
    }
}

/// `ShowItem`. Strings are `None` where the file has `i:nil` or leaves the member out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShowItem {
    pub item_type: ShowItemType,
    pub action: ActionType,
    pub name: Option<String>,
    /// Relative to `<data root>/Sequence`. One made on Windows may hold a subfolder with `\`.
    pub sequence_file_name: Option<String>,
    pub launch_program_name: Option<String>,
    pub launch_command_line: Option<String>,
    pub launch_show_command_window: bool,
    pub launch_wait_for_exit: bool,
    /// 1 on a new item, 0 on one stored before the member existed.
    pub pause_seconds: i32,
    /// The position on its own tab.
    pub item_order: i32,
    // No editor now; kept as stored.
    pub website_url: Option<String>,
    pub show_show_id: Uuid,
    pub show_stop_current_show: bool,
}

impl ShowItem {
    /// `new ShowItem(type, name, showId)`.
    pub fn new(item_type: ShowItemType, name: &str) -> ShowItem {
        ShowItem {
            item_type,
            action: ActionType::Launch,
            name: Some(name.to_string()),
            sequence_file_name: None,
            launch_program_name: None,
            launch_command_line: None,
            launch_show_command_window: false,
            launch_wait_for_exit: false,
            pause_seconds: 1,
            item_order: 0,
            website_url: None,
            show_show_id: Uuid::nil(),
            show_stop_current_show: false,
        }
    }

    pub fn name_text(&self) -> &str {
        self.name.as_deref().unwrap_or("")
    }

    /// `Path.GetFileName(SequencePath)`.
    pub fn sequence_file(&self) -> &str {
        windows_file_name(self.sequence_file_name.as_deref().unwrap_or(""))
    }

    /// What the spinner shows. GUARD: upstream's throws on a stored 0, the moment the action becomes Pause.
    pub fn pause_shown(&self) -> i32 {
        self.pause_seconds.clamp(PAUSE_MIN, PAUSE_MAX)
    }

    /// The name the action's editor gives the item (F48), or `None` for an action with no editor.
    pub fn editor_name(&self) -> Option<String> {
        match &self.action {
            ActionType::Sequence => Some(format!("Run sequence: {}", self.sequence_file())),
            ActionType::Launch => Some(format!("Launch: {}", self.launch_program_name.as_deref().unwrap_or(""))),
            ActionType::Pause => Some(format!("Pause for {}", self.pause_seconds)),
            ActionType::Other(_) => None,
        }
    }
}

pub const PAUSE_MIN: i32 = 1;
pub const PAUSE_MAX: i32 = 900;

/// `SequencePath`: the stored name under the Sequence folder, with a Windows subfolder's `\` read as a separator.
pub fn sequence_path(item: &ShowItem, sequence_dir: &Path) -> PathBuf {
    let mut path = sequence_dir.to_path_buf();
    for part in item.sequence_file_name.as_deref().unwrap_or("").split(['\\', '/']).filter(|p| !p.is_empty()) {
        path.push(part);
    }
    path
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PathError {
    /// Upstream's `ArgumentException`; its open dialog refuses such a file first.
    #[error("Path is not rooted in the profile sequence folder.")]
    OutsideSequenceFolder,
}

/// What `SequencePath`'s setter stores: the path relative to the Sequence folder.
pub fn relative_sequence_name(path: &Path, sequence_dir: &Path) -> Result<String, PathError> {
    let relative = path.strip_prefix(sequence_dir).map_err(|_| PathError::OutsideSequenceFolder)?;
    let parts: Vec<String> = relative.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
    if parts.is_empty() {
        return Err(PathError::OutsideSequenceFolder);
    }
    Ok(parts.join("/"))
}

/// `Show`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Show {
    pub id: Uuid,
    pub name: Option<String>,
    pub items: Vec<ShowItem>,
}

impl Show {
    pub fn name_text(&self) -> &str {
        self.name.as_deref().unwrap_or("")
    }

    /// `GetItems(type)`, in stored order.
    pub fn items_of(&self, item_type: &ShowItemType) -> Vec<&ShowItem> {
        self.items.iter().filter(|i| *item_type == ShowItemType::All || i.item_type == *item_type).collect()
    }

    /// A tab's items as a run takes them: by `ItemOrder`, ties in stored order.
    pub fn ordered(&self, item_type: &ShowItemType) -> Vec<&ShowItem> {
        let mut items = self.items_of(item_type);
        items.sort_by_key(|i| i.item_order);
        items
    }
}

/// `ShowsData.AddShow`: `New Show`, with a new id, at the end. Returns its position.
pub fn add_show(shows: &mut Vec<Show>) -> usize {
    shows.push(Show { id: Uuid::new_v4(), name: Some("New Show".to_string()), items: Vec::new() });
    shows.len() - 1
}

/// F40: an empty label is ignored. True when the name was taken.
pub fn rename_show(shows: &mut [Show], index: usize, name: &str) -> bool {
    match shows.get_mut(index) {
        Some(show) if !name.is_empty() => {
            show.name = Some(name.to_string());
            true
        }
        _ => false,
    }
}

/// F41's question.
pub fn delete_show_message(show: &Show) -> String {
    format!("Are you sure you want to delete the selected show?\n\n{}\n\nYou CANNOT undo this!", show.name_text())
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ShowError {
    #[error("You must have at least one sequential item defined for a valid show.")]
    NoSequentialItem,
}

impl ShowError {
    pub fn title(&self) -> &'static str {
        "Show Error"
    }
}

pub const CANCEL_TITLE: &str = "Cancel Show setup changes";
pub const CANCEL_MESSAGE: &str = "Are you sure you want to cancel? Any changes made to the show setup will be lost.";

/// `ShowEditorForm` over a copy of a show: nothing reaches the list until `finish` succeeds.
#[derive(Clone, Debug)]
pub struct ShowEditor {
    show: Show,
    sequence_dir: PathBuf,
    tab: ShowItemType,
    /// The selected row on the tab.
    selected: Option<usize>,
}

impl ShowEditor {
    /// Opens on Startup with nothing selected.
    pub fn new(show: Show, sequence_dir: PathBuf) -> ShowEditor {
        let mut editor = ShowEditor { show, sequence_dir, tab: ShowItemType::Startup, selected: None };
        editor.populate();
        editor
    }

    pub fn show(&self) -> &Show {
        &self.show
    }

    pub fn sequence_dir(&self) -> &Path {
        &self.sequence_dir
    }

    pub fn tab(&self) -> &ShowItemType {
        &self.tab
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// `textBoxShowName`. KEEP: a blank name is accepted.
    pub fn set_name(&mut self, name: &str) {
        self.show.name = Some(name.to_string());
    }

    /// `PopulateItemList`: every item sorted by `ItemOrder`. Upstream's sort isn't stable; this one is, so an untouched show keeps its
    /// stored order.
    fn populate(&mut self) {
        self.show.items.sort_by_key(|i| i.item_order);
    }

    /// Positions in `show.items` of the tab's rows, top to bottom.
    fn row_positions(&self) -> Vec<usize> {
        self.show.items.iter().enumerate().filter(|(_, i)| i.item_type == self.tab).map(|(n, _)| n).collect()
    }

    /// The tab's rows, top to bottom.
    pub fn rows(&self) -> Vec<&ShowItem> {
        self.show.items.iter().filter(|i| i.item_type == self.tab).collect()
    }

    pub fn selected_item(&self) -> Option<&ShowItem> {
        let position = *self.row_positions().get(self.selected?)?;
        self.show.items.get(position)
    }

    fn selected_item_mut(&mut self) -> Option<&mut ShowItem> {
        let position = *self.row_positions().get(self.selected?)?;
        self.show.items.get_mut(position)
    }

    /// F43: the tab's items with no selection.
    pub fn set_tab(&mut self, tab: ShowItemType) {
        self.tab = tab;
        self.selected = None;
        self.populate();
    }

    /// Selecting a row loads its action's editor, which gives the item that editor's name at once.
    pub fn select(&mut self, row: Option<usize>) {
        self.selected = row.filter(|r| *r < self.row_positions().len());
        self.rename_selected();
    }

    fn rename_selected(&mut self) {
        if let Some(item) = self.selected_item_mut() {
            if let Some(name) = item.editor_name() {
                item.name = Some(name);
            }
        }
    }

    /// `UpdateListViewItems`: the tab's rows numbered from 0.
    fn renumber(&mut self) {
        for (row, position) in self.row_positions().into_iter().enumerate() {
            self.show.items[position].item_order = row as i32;
        }
    }

    /// F44: a Sequence item at the end of the tab, selected.
    pub fn add_item(&mut self) {
        let mut item = ShowItem::new(self.tab.clone(), "New Item");
        item.action = ActionType::Sequence;
        self.show.items.push(item);
        self.selected = Some(self.row_positions().len() - 1);
        self.rename_selected();
        self.renumber();
    }

    /// F45: removes the selected row, then selects the one before it, else the one now in its place. GUARD: by the row's position,
    /// where upstream indexes the list by `ItemOrder` and throws on a tab stored with a gap in its numbers.
    pub fn delete_item(&mut self) {
        let Some(row) = self.selected else { return };
        let positions = self.row_positions();
        let Some(&position) = positions.get(row) else { return };
        self.show.items.remove(position);
        self.renumber();
        let remaining = positions.len() - 1;
        let next = if row > 0 {
            Some(row - 1)
        } else if remaining > 0 {
            Some(0)
        } else {
            None
        };
        self.select(next);
    }

    /// F46: the dragged row takes the target row's place. Dropping a row on itself does nothing.
    pub fn move_item(&mut self, from: usize, to: usize) {
        let positions = self.row_positions();
        if from == to || from >= positions.len() || to >= positions.len() {
            return;
        }
        let target = positions[to];
        let item = self.show.items.remove(positions[from]);
        self.show.items.insert(target, item);
        self.selected = Some(to);
        self.renumber();
    }

    /// F47: the other actions' fields stay on the item.
    pub fn set_action(&mut self, action: ActionType) {
        if let Some(item) = self.selected_item_mut() {
            item.action = action;
        }
        self.rename_selected();
    }

    /// The Sequence editor's browse: a file under the Sequence folder.
    pub fn set_sequence(&mut self, path: &Path) -> Result<(), PathError> {
        let name = relative_sequence_name(path, &self.sequence_dir)?;
        if let Some(item) = self.selected_item_mut() {
            item.sequence_file_name = Some(name);
        }
        self.rename_selected();
        Ok(())
    }

    /// `Program to Run:`, as typed.
    pub fn set_launch_program(&mut self, text: &str) {
        if let Some(item) = self.selected_item_mut() {
            item.launch_program_name = Some(text.to_string());
        }
        self.rename_selected();
    }

    pub fn set_launch_command_line(&mut self, text: &str) {
        if let Some(item) = self.selected_item_mut() {
            item.launch_command_line = Some(text.to_string());
        }
    }

    pub fn set_launch_show_command_window(&mut self, on: bool) {
        if let Some(item) = self.selected_item_mut() {
            item.launch_show_command_window = on;
        }
    }

    pub fn set_launch_wait_for_exit(&mut self, on: bool) {
        if let Some(item) = self.selected_item_mut() {
            item.launch_wait_for_exit = on;
        }
    }

    /// `Pause (seconds):`, 1 to 900.
    pub fn set_pause(&mut self, seconds: i32) {
        if let Some(item) = self.selected_item_mut() {
            item.pause_seconds = seconds.clamp(PAUSE_MIN, PAUSE_MAX);
        }
        self.rename_selected();
    }

    /// F48b, OK: renumbers the tab on screen and wants a Sequential item.
    pub fn finish(&mut self) -> Result<Show, ShowError> {
        self.renumber();
        if self.show.items_of(&ShowItemType::Sequential).is_empty() {
            return Err(ShowError::NoSequentialItem);
        }
        Ok(self.show.clone())
    }
}

// The data contract.

fn text_member(e: &XElement, name: &str) -> Option<String> {
    e.child(name).filter(|c| !c.nil).map(XElement::text)
}

fn bool_member(e: &XElement, name: &str) -> bool {
    e.child_text(name).and_then(|t| parse_bool(&t)).unwrap_or(false)
}

fn int_member(e: &XElement, name: &str) -> i32 {
    e.child_text(name).and_then(|t| t.trim().parse().ok()).unwrap_or(0)
}

fn decode_item(e: &XElement) -> ShowItem {
    ShowItem {
        item_type: ShowItemType::parse(e.child_text("ItemType").as_deref().unwrap_or("All")),
        action: ActionType::parse(e.child_text("Action").as_deref().unwrap_or("Launch")),
        name: text_member(e, "Name"),
        sequence_file_name: text_member(e, "Sequence_FileName"),
        launch_program_name: text_member(e, "Launch_ProgramName"),
        launch_command_line: text_member(e, "Launch_CommandLine"),
        launch_show_command_window: bool_member(e, "Launch_ShowCommandWindow"),
        launch_wait_for_exit: bool_member(e, "Launch_WaitForExit"),
        // A data-contract load doesn't run the initializer, so a missing member is 0.
        pause_seconds: int_member(e, "Pause_Seconds"),
        item_order: int_member(e, "ItemOrder"),
        website_url: text_member(e, "Website_URL"),
        show_show_id: e.child_text("Show_ShowID").and_then(|t| parse_guid(&t)).unwrap_or_default(),
        show_stop_current_show: bool_member(e, "Show_StopCurrentShow"),
    }
}

fn string_element(name: &str, value: &Option<String>) -> XElement {
    match value {
        Some(v) => XElement::with_text(NS, name, v.clone()),
        None => XElement::nil(NS, name),
    }
}

fn encode_item(item: &ShowItem) -> XElement {
    let mut e = XElement::new(NS, "ShowItem");
    // `EmitDefaultValue = false`.
    let optional = |e: &mut XElement, name: &str, value: &Option<String>| {
        if let Some(v) = value {
            e.push(XElement::with_text(NS, name, v.clone()));
        }
    };
    let flag = |e: &mut XElement, name: &str, value: bool| {
        if value {
            e.push(XElement::with_text(NS, name, format_bool(true)));
        }
    };
    e.push(XElement::with_text(NS, "Action", item.action.text()));
    e.push(XElement::with_text(NS, "ItemOrder", item.item_order.to_string()));
    e.push(XElement::with_text(NS, "ItemType", item.item_type.text()));
    optional(&mut e, "Launch_CommandLine", &item.launch_command_line);
    optional(&mut e, "Launch_ProgramName", &item.launch_program_name);
    flag(&mut e, "Launch_ShowCommandWindow", item.launch_show_command_window);
    flag(&mut e, "Launch_WaitForExit", item.launch_wait_for_exit);
    e.push(string_element("Name", &item.name));
    e.push(XElement::with_text(NS, "Pause_Seconds", item.pause_seconds.to_string()));
    e.push(string_element("Sequence_FileName", &item.sequence_file_name));
    if !item.show_show_id.is_nil() {
        e.push(XElement::with_text(NS, "Show_ShowID", format_guid(&item.show_show_id)));
    }
    flag(&mut e, "Show_StopCurrentShow", item.show_stop_current_show);
    optional(&mut e, "Website_URL", &item.website_url);
    e
}

fn decode_show(e: &XElement) -> Show {
    Show {
        // `ID`'s getter makes one for a show that has none.
        id: e.child_text("ID").and_then(|t| parse_guid(&t)).filter(|id| !id.is_nil()).unwrap_or_else(Uuid::new_v4),
        name: text_member(e, "Name"),
        items: e
            .child("Items")
            .map(|list| list.elements().filter(|i| i.local == "ShowItem").map(decode_item).collect())
            .unwrap_or_default(),
    }
}

fn encode_show(show: &Show) -> XElement {
    let mut e = XElement::new(NS, "Show");
    e.push(XElement::with_text(NS, "ID", format_guid(&show.id)));
    let mut items = XElement::new(NS, "Items");
    for item in &show.items {
        items.push(encode_item(item));
    }
    e.push(items);
    e.push(string_element("Name", &show.name));
    e
}

fn encode_data(shows: &[Show]) -> XElement {
    let mut data = XElement::new(NS, "ShowsData");
    data.push(XElement::with_text("", "ModuleInstanceId", format_guid(&MODULE_TYPE)));
    data.push(XElement::with_text("", "ModuleTypeId", format_guid(&MODULE_TYPE)));
    let mut list = XElement::new(NS, "Shows");
    for show in shows {
        list.push(encode_show(show));
    }
    data.push(list);
    data
}

fn decode_data(data: &XElement) -> Vec<Show> {
    data.child("Shows")
        .map(|list| list.elements().filter(|e| e.local == "Show").map(decode_show).collect())
        .unwrap_or_default()
}

/// The stored shows. None stored is an empty list.
pub fn load_shows(store: &ModuleStore) -> Vec<Show> {
    store.type_data(MODULE_TYPE).and_then(ModuleEntry::data).map(decode_data).unwrap_or_default()
}

/// Writes the shows into the store, keeping every value that didn't change exactly as it was written. Saving the file is the caller's
/// (`AppCore::save_module_config`).
pub fn save_shows(store: &mut ModuleStore, shows: &[Show]) {
    let new_plain = encode_data(shows);
    match store.type_data_mut(MODULE_TYPE) {
        Some(entry) => {
            let old_plain = encode_data(&entry.data().map(decode_data).unwrap_or_default());
            entry.merge_data(&old_plain, new_plain);
        }
        None => store.set_type_data(ModuleEntry::new(DATA_MODEL_TYPE, &MODULE_TYPE, &MODULE_TYPE, ASSEMBLY_NAME, new_plain)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEQUENCES: &str = "/data/Vixen 3/Sequence";

    fn item(tab: ShowItemType, order: i32, file: &str) -> ShowItem {
        let mut item = ShowItem::new(tab, "");
        item.action = ActionType::Sequence;
        item.item_order = order;
        item.sequence_file_name = Some(file.to_string());
        item.name = item.editor_name();
        item.pause_seconds = 0;
        item
    }

    fn owners_show() -> Show {
        Show {
            id: uuid::uuid!("e81726f8-9708-41df-b8de-f8c07b875f22"),
            name: Some("Christmas Lights 2018".to_string()),
            items: vec![
                item(ShowItemType::Shutdown, 0, "Zoom.tim"),
                item(ShowItemType::Sequential, 0, "Wizards in Winter.tim"),
                item(ShowItemType::Sequential, 1, "Christmas Eve Sarajevo.tim"),
                item(ShowItemType::Sequential, 2, "Carey Grinch.tim"),
            ],
        }
    }

    fn editor() -> ShowEditor {
        ShowEditor::new(owners_show(), PathBuf::from(SEQUENCES))
    }

    fn names(editor: &ShowEditor) -> Vec<String> {
        editor.rows().iter().map(|i| i.name_text().to_string()).collect()
    }

    #[test]
    fn a_new_store_gets_the_data_contract_shape() {
        let mut shows = Vec::new();
        let at = add_show(&mut shows);
        assert_eq!(shows[at].name_text(), "New Show");
        let mut launch = ShowItem::new(ShowItemType::Startup, "Launch: /bin/echo");
        launch.launch_program_name = Some("/bin/echo".to_string());
        launch.launch_wait_for_exit = true;
        shows[at].items.push(launch);

        let mut store = ModuleStore::new();
        save_shows(&mut store, &shows);
        let data = store.type_data(MODULE_TYPE).unwrap().data().unwrap();
        let item = data.child("Shows").unwrap().child("Show").unwrap().child("Items").unwrap().child("ShowItem").unwrap();
        let members: Vec<&str> = item.elements().map(|e| e.local.as_str()).collect();
        assert_eq!(
            members,
            ["Action", "ItemOrder", "ItemType", "Launch_ProgramName", "Launch_WaitForExit", "Name", "Pause_Seconds", "Sequence_FileName"]
        );
        assert!(item.child("Sequence_FileName").unwrap().nil);
        assert_eq!(load_shows(&store), shows);
    }

    #[test]
    fn members_with_no_editor_and_unknown_text_survive() {
        let mut show = owners_show();
        show.items[0].action = ActionType::Other("WebPage".to_string());
        show.items[0].website_url = Some("http://example.invalid/".to_string());
        show.items[1].show_show_id = Uuid::from_u128(7);
        show.items[1].show_stop_current_show = true;
        show.items[2].item_type = ShowItemType::Input;
        let mut store = ModuleStore::new();
        save_shows(&mut store, std::slice::from_ref(&show));
        assert_eq!(load_shows(&store), vec![show.clone()]);

        // An action with no editor keeps its name when selected.
        let mut editor = ShowEditor::new(show, PathBuf::from(SEQUENCES));
        editor.set_tab(ShowItemType::Shutdown);
        editor.select(Some(0));
        assert_eq!(editor.selected_item().unwrap().name_text(), "Run sequence: Zoom.tim");
    }

    #[test]
    fn tabs_list_their_own_items_in_order() {
        let mut editor = editor();
        assert!(editor.rows().is_empty() && editor.selected().is_none());
        assert_eq!(editor.tab().help(), "These items are run once, in order, during startup.");
        editor.set_tab(ShowItemType::Sequential);
        assert_eq!(
            names(&editor),
            ["Run sequence: Wizards in Winter.tim", "Run sequence: Christmas Eve Sarajevo.tim", "Run sequence: Carey Grinch.tim"]
        );
        editor.set_tab(ShowItemType::Shutdown);
        assert_eq!(names(&editor), ["Run sequence: Zoom.tim"]);
    }

    #[test]
    fn adding_makes_a_sequence_item_named_by_its_editor() {
        let mut editor = editor();
        editor.set_tab(ShowItemType::Sequential);
        editor.add_item();
        assert_eq!(editor.selected(), Some(3));
        let added = editor.selected_item().unwrap();
        assert_eq!((added.name_text(), added.item_order, added.pause_seconds), ("Run sequence: ", 3, 1));
        assert_eq!(added.action, ActionType::Sequence);

        editor.set_sequence(Path::new("/data/Vixen 3/Sequence/Snoopy Christmas.tim")).unwrap();
        assert_eq!(editor.selected_item().unwrap().name_text(), "Run sequence: Snoopy Christmas.tim");
        assert_eq!(editor.selected_item().unwrap().sequence_file_name.as_deref(), Some("Snoopy Christmas.tim"));
        assert_eq!(editor.set_sequence(Path::new("/elsewhere/Other.tim")), Err(PathError::OutsideSequenceFolder));
        editor.set_sequence(Path::new("/data/Vixen 3/Sequence/2019/Deep.tim")).unwrap();
        assert_eq!(editor.selected_item().unwrap().sequence_file_name.as_deref(), Some("2019/Deep.tim"));
        assert_eq!(editor.selected_item().unwrap().name_text(), "Run sequence: Deep.tim");
    }

    #[test]
    fn deleting_selects_the_row_before_or_the_one_in_its_place() {
        let mut editor = editor();
        editor.set_tab(ShowItemType::Sequential);
        editor.select(Some(1));
        editor.delete_item();
        assert_eq!(editor.selected(), Some(0));
        assert_eq!(names(&editor), ["Run sequence: Wizards in Winter.tim", "Run sequence: Carey Grinch.tim"]);
        assert_eq!(editor.rows().iter().map(|i| i.item_order).collect::<Vec<_>>(), [0, 1]);
        editor.delete_item();
        assert_eq!((editor.selected(), names(&editor)), (Some(0), vec!["Run sequence: Carey Grinch.tim".to_string()]));
        editor.delete_item();
        assert!(editor.selected().is_none() && editor.rows().is_empty());
        editor.delete_item();
        assert_eq!(editor.finish(), Err(ShowError::NoSequentialItem));
        // The other tab was left alone.
        assert_eq!(editor.show().items.len(), 1);
    }

    #[test]
    fn a_gap_in_the_stored_order_is_harmless() {
        let mut show = owners_show();
        show.items[3].item_order = 9;
        let mut editor = ShowEditor::new(show, PathBuf::from(SEQUENCES));
        editor.set_tab(ShowItemType::Sequential);
        editor.select(Some(2));
        editor.delete_item();
        assert_eq!(editor.selected(), Some(1));
        assert_eq!(editor.rows().len(), 2);
    }

    #[test]
    fn dragging_puts_the_row_at_the_targets_index() {
        let mut editor = editor();
        editor.set_tab(ShowItemType::Sequential);
        editor.move_item(0, 2);
        assert_eq!(
            names(&editor),
            ["Run sequence: Christmas Eve Sarajevo.tim", "Run sequence: Carey Grinch.tim", "Run sequence: Wizards in Winter.tim"]
        );
        assert_eq!(editor.selected(), Some(2));
        editor.move_item(2, 1);
        assert_eq!(
            names(&editor),
            ["Run sequence: Christmas Eve Sarajevo.tim", "Run sequence: Wizards in Winter.tim", "Run sequence: Carey Grinch.tim"]
        );
        assert_eq!(editor.rows().iter().map(|i| i.item_order).collect::<Vec<_>>(), [0, 1, 2]);
        editor.move_item(1, 1);
        editor.move_item(1, 7);
        assert_eq!(editor.rows()[1].name_text(), "Run sequence: Wizards in Winter.tim");
        // The Shutdown item is still the only one on its tab.
        let shutdown = editor.show().items_of(&ShowItemType::Shutdown);
        assert_eq!((shutdown.len(), shutdown[0].item_order), (1, 0));
    }

    #[test]
    fn changing_the_action_renames_and_keeps_the_other_fields() {
        let mut editor = editor();
        editor.set_tab(ShowItemType::Sequential);
        editor.select(Some(0));
        editor.set_action(ActionType::Pause);
        // The owner's items carry 0 seconds: shown as 1, stored only on a change.
        let paused = editor.selected_item().unwrap();
        assert_eq!((paused.name_text(), paused.pause_seconds, paused.pause_shown()), ("Pause for 0", 0, 1));
        editor.set_pause(5000);
        assert_eq!(editor.selected_item().unwrap().name_text(), "Pause for 900");
        editor.set_action(ActionType::Launch);
        assert_eq!(editor.selected_item().unwrap().name_text(), "Launch: ");
        editor.set_launch_program("/usr/bin/say");
        editor.set_launch_command_line("\"ho ho\" ho");
        editor.set_launch_wait_for_exit(true);
        let launch = editor.selected_item().unwrap();
        assert_eq!(launch.name_text(), "Launch: /usr/bin/say");
        assert_eq!(launch.sequence_file_name.as_deref(), Some("Wizards in Winter.tim"));
        assert_eq!(launch.pause_seconds, 900);
        editor.set_action(ActionType::Sequence);
        assert_eq!(editor.selected_item().unwrap().name_text(), "Run sequence: Wizards in Winter.tim");
    }

    #[test]
    fn a_windows_subfolder_resolves_under_the_sequence_folder() {
        let mut item = ShowItem::new(ShowItemType::Sequential, "");
        item.sequence_file_name = Some("2018\\Finale.tim".to_string());
        assert_eq!(sequence_path(&item, Path::new(SEQUENCES)), PathBuf::from("/data/Vixen 3/Sequence/2018/Finale.tim"));
        assert_eq!(item.sequence_file(), "Finale.tim");
    }

    #[test]
    fn the_list_adds_renames_and_asks_before_deleting() {
        let mut shows = vec![owners_show()];
        let at = add_show(&mut shows);
        assert_eq!((at, shows[1].items.len()), (1, 0));
        assert_ne!(shows[1].id, shows[0].id);
        assert!(!rename_show(&mut shows, 1, ""));
        assert!(rename_show(&mut shows, 1, "Halloween"));
        assert_eq!(shows[1].name_text(), "Halloween");
        assert_eq!(
            delete_show_message(&shows[0]),
            "Are you sure you want to delete the selected show?\n\nChristmas Lights 2018\n\nYou CANNOT undo this!"
        );
    }
}
