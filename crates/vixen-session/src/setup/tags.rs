//! Element tags: `ElementTagService`, `BuiltInElementTags` and the tag parts of `ElementTree`.

use super::tree::{BitSet, Nid};
use super::SetupSession;
use std::collections::HashMap;
use uuid::{uuid, Uuid};
use vixen_io::TagDefinition;

const DEPRECATED: Uuid = uuid!("753115da-e27a-4261-a136-8222ccc3f22e");
const HIDDEN: Uuid = uuid!("2f21bfcb-07a3-4cac-bdc5-4b6e37773c04");
const PROP: Uuid = uuid!("6db1852d-71d8-449d-afa5-193a6def26a0");

/// `BuiltInElementTags.All`, in order.
pub fn built_in_tags() -> Vec<TagDefinition> {
    let tag = |id, key: &str, name: &str, color: &str, sort_order| TagDefinition {
        id,
        key: key.to_string(),
        name: name.to_string(),
        is_built_in: true,
        sort_order,
        description: None,
        display_color: Some(color.to_string()),
    };
    vec![
        tag(DEPRECATED, "deprecated", "Deprecated", "#FF0000", 0),
        tag(HIDDEN, "hidden", "Hidden", "#000000", 1),
        tag(PROP, "prop", "Prop", "#0000FF", 2),
    ]
}

/// `LoadSystemConfig` seeds any built-in tag the file doesn't have. A file from before tags gets them in memory only, until something else makes a save happen.
pub fn seeded_tags(from_file: &[TagDefinition]) -> Vec<TagDefinition> {
    let mut out = from_file.to_vec();
    for tag in built_in_tags() {
        if !out.iter().any(|t| t.id == tag.id) {
            out.push(tag);
        }
    }
    out
}

/// `ElementTagService.GetAll`: by `SortOrder`, then name ignoring case.
pub fn sorted_tags(tags: &[TagDefinition]) -> Vec<TagDefinition> {
    let mut out = tags.to_vec();
    out.sort_by_cached_key(|t| (t.sort_order, t.name.to_lowercase()));
    out
}

/// `ColorTranslator.FromHtml` for tag colours: hex or case-insensitive .NET colour names.
pub fn parse_html_color(text: &str) -> Option<u32> {
    let text = text.trim();
    if let Some(hex) = text.strip_prefix('#') {
        let value = u32::from_str_radix(hex, 16).ok()?;
        return match hex.len() {
            6 => Some(0xFF00_0000 | value),
            8 => Some(value),
            3 => {
                let (r, g, b) = ((value >> 8) & 0xF, (value >> 4) & 0xF, value & 0xF);
                Some(0xFF00_0000 | (r * 17) << 16 | (g * 17) << 8 | (b * 17))
            }
            _ => None,
        };
    }
    let name = if text.eq_ignore_ascii_case("grey") { "Gray" } else { text };
    vixen_output::dotnet::known::by_name(name).and_then(vixen_output::dotnet::known::argb)
}

/// Tag colours resolved once, so a row's fetch does one hash lookup per tag instead of a list search and a colour parse per paint.
#[derive(Clone, Debug, Default)]
pub struct TagTable {
    entries: HashMap<Uuid, (i32, u32)>,
}

impl TagTable {
    pub fn new(tags: &[TagDefinition]) -> TagTable {
        let mut entries = HashMap::with_capacity(tags.len());
        for t in tags {
            if let Some(color) = t.display_color.as_deref().filter(|c| !c.is_empty()).and_then(parse_html_color) {
                entries.insert(t.id, (t.sort_order, color));
            }
        }
        TagTable { entries }
    }

    /// F25: one colour per tag having a `DisplayColor`, by `SortOrder`. Unknown tag ids are skipped.
    pub fn colors_of(&self, tags: &[Uuid]) -> Vec<u32> {
        if tags.is_empty() {
            return Vec::new();
        }
        let mut found: Vec<(i32, u32)> = tags.iter().filter_map(|t| self.entries.get(t).copied()).collect();
        found.sort_by_key(|f| f.0);
        found.into_iter().map(|f| f.1).collect()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TagCheck {
    Off,
    On,
    Mixed,
}

/// A row of the Tags submenu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagMenuItem {
    pub id: Uuid,
    pub name: String,
    pub color: Option<u32>,
    pub check: TagCheck,
}

/// A row of Manage Tag Colors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagColorRow {
    pub id: Uuid,
    pub name: String,
    /// White when the stored colour is bad or empty (`TagColorItem`).
    pub color: u32,
}

fn built_in_color_rows(tags: &[TagDefinition]) -> Vec<TagColorRow> {
    sorted_tags(tags).into_iter().filter(|t| t.is_built_in).map(|t| {
        let color = t.display_color.as_deref().and_then(parse_html_color).unwrap_or(0xFFFF_FFFF);
        TagColorRow { id: t.id, name: t.name, color }
    }).collect()
}

impl crate::app::AppCore {
    /// The sequencer opens the same built-in color editor without opening Display Setup.
    pub fn element_tag_colors(&self) -> Vec<TagColorRow> {
        built_in_color_rows(&seeded_tags(&self.system.read().tags))
    }

    /// `saveOnClose: true`: save only tag definitions, without rebuilding the display or
    /// stopping playback. Failed writes restore the live tags and report the error.
    pub fn save_element_tag_colors(&self, colors: &[(Uuid, u32)]) -> Result<(), crate::SessionError> {
        let _saving = crate::app::Saving::begin(&self.saving_system)
            .ok_or_else(|| crate::SessionError::Other("System configuration is already being saved. Please try again.".into()))?;
        let mut system = self.system.write();
        let mut tags = seeded_tags(&system.tags);
        for (id, color) in colors {
            if let Some(tag) = tags.iter_mut().find(|t| t.id == *id && t.is_built_in) {
                tag.display_color = Some(format!("#{:06X}", color & 0x00FF_FFFF));
            }
        }
        let previous = std::mem::replace(&mut system.tags, tags);
        if let Err(error) = vixen_io::system::save_system_config_file(&system, &self.paths) {
            system.tags = previous;
            return Err(error.into());
        }
        drop(system);
        self.display.tag_colors_changed();
        Ok(())
    }
}

impl SetupSession {
    /// F54: every tag in service order, with its check state over the selection.
    pub fn tag_menu(&self, selection: &[String]) -> Vec<TagMenuItem> {
        let nodes = self.nodes_of(selection);
        sorted_tags(&self.tags)
            .into_iter()
            .map(|t| {
                let tagged = nodes.iter().filter(|n| self.tree.node(**n).tags.contains(&t.id)).count();
                let check = if tagged == 0 {
                    TagCheck::Off
                } else if tagged == nodes.len() {
                    TagCheck::On
                } else {
                    TagCheck::Mixed
                };
                let color = t.display_color.as_deref().and_then(parse_html_color);
                TagMenuItem { id: t.id, name: t.name, color, check }
            })
            .collect()
    }

    /// F55: fully checked removes it from every target. Off or mixed adds it to every target. Returns the nodes to repaint.
    pub fn toggle_tag(&mut self, selection: &[String], tag: Uuid, cascade: bool) -> Vec<Nid> {
        let nodes = self.nodes_of(selection);
        let all_on = !nodes.is_empty() && nodes.iter().all(|n| self.tree.node(*n).tags.contains(&tag));
        let mut seen = BitSet::new(self.tree.slots());
        let mut targets = Vec::new();
        for n in nodes {
            if cascade {
                for d in self.tree.descendants(n) {
                    if seen.insert(d.0 as usize) {
                        targets.push(d);
                    }
                }
            } else if seen.insert(n.0 as usize) {
                targets.push(n);
            }
        }
        for t in &targets {
            let list = &mut self.tree.node_mut(*t).tags;
            if all_on {
                list.retain(|x| *x != tag);
            } else if !list.contains(&tag) {
                list.push(tag);
            }
        }
        if !targets.is_empty() {
            self.dirty.tree = true;
        }
        targets
    }

    /// F132: the built-in tags in `SortOrder`.
    pub fn tag_colors(&self) -> Vec<TagColorRow> {
        built_in_color_rows(&self.tags)
    }

    /// F132's Save: each colour as `#RRGGBB`, reaching the file with Display Setup's OK. A colour equal to what's there changes nothing.
    pub fn set_tag_colors(&mut self, colors: &[(Uuid, u32)]) {
        let mut changed = false;
        for (id, argb) in colors {
            let text = format!("#{:06X}", argb & 0x00FF_FFFF);
            if let Some(t) = self.tags.iter_mut().find(|t| t.id == *id) {
                if t.display_color.as_deref() != Some(text.as_str()) {
                    t.display_color = Some(text);
                    changed = true;
                }
            }
        }
        if changed {
            self.tag_table = TagTable::new(&self.tags);
            self.dirty.tags = true;
        }
    }

    /// `CreateUserTag`. Names are trimmed, must not be blank and must be unique ignoring case.
    pub fn create_user_tag(&mut self, name: &str) -> Result<Uuid, String> {
        let name = self.check_tag_name(name, None)?;
        let id = Uuid::new_v4();
        let sort_order = self.tags.iter().map(|t| t.sort_order).max().unwrap_or(0) + 1;
        self.tags.push(TagDefinition { id, key: String::new(), name, is_built_in: false, sort_order, description: None, display_color: None });
        self.tag_table = TagTable::new(&self.tags);
        self.dirty.tags = true;
        Ok(id)
    }

    /// `RenameUserTag`. A built-in can't be renamed.
    pub fn rename_user_tag(&mut self, id: Uuid, name: &str) -> Result<(), String> {
        let name = self.check_tag_name(name, Some(id))?;
        let tag = self.tags.iter_mut().find(|t| t.id == id).ok_or_else(|| "No such tag.".to_string())?;
        if tag.is_built_in {
            return Err("Built-in tags cannot be renamed.".to_string());
        }
        tag.name = name;
        self.dirty.tags = true;
        Ok(())
    }

    /// `DeleteUserTag`: from the catalog and from every node. A built-in can't be deleted.
    pub fn delete_user_tag(&mut self, id: Uuid) -> Result<(), String> {
        let tag = self.tags.iter().find(|t| t.id == id).ok_or_else(|| "No such tag.".to_string())?;
        if tag.is_built_in {
            return Err("Built-in tags cannot be deleted.".to_string());
        }
        self.tags.retain(|t| t.id != id);
        let nodes: Vec<Nid> = self.tree.live_nodes().map(|(n, _)| n).collect();
        for n in nodes {
            self.tree.node_mut(n).tags.retain(|t| *t != id);
        }
        self.tag_table = TagTable::new(&self.tags);
        self.dirty.tags = true;
        self.dirty.tree = true;
        Ok(())
    }

    fn check_tag_name(&self, name: &str, ignoring: Option<Uuid>) -> Result<String, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Tag name cannot be blank.".to_string());
        }
        let folded = name.to_lowercase();
        if self.tags.iter().any(|t| Some(t.id) != ignoring && t.name.to_lowercase() == folded) {
            return Err(format!("A tag named \"{name}\" already exists."));
        }
        Ok(name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use super::*;

    #[test]
    fn built_ins_are_seeded_in_order() {
        let s = session(&[], 0);
        let names: Vec<String> = s.tag_colors().into_iter().map(|t| t.name).collect();
        assert_eq!(names, ["Deprecated", "Hidden", "Prop"]);
        assert_eq!(s.tag_colors()[2].color, 0xFF00_00FF);
        assert!(!s.is_dirty(), "seeding alone is not an edit");
    }

    #[test]
    fn colours_parse_like_from_html() {
        assert_eq!(parse_html_color("#FF8000"), Some(0xFFFF_8000));
        assert_eq!(parse_html_color("#F80"), Some(0xFFFF_8800));
        assert_eq!(parse_html_color("Blue"), Some(0xFF00_00FF));
        assert_eq!(parse_html_color(" lightpink "), Some(0xFFFF_B6C1));
        assert_eq!(parse_html_color("SteelBlue"), Some(0xFF46_82B4));
        assert_eq!(parse_html_color("Transparent"), Some(0x00FF_FFFF));
        assert_eq!(parse_html_color("grey"), Some(0xFF80_8080));
        assert_eq!(parse_html_color("nonsense"), None);
        assert_eq!(parse_html_color("#12"), None);
    }

    #[test]
    fn named_colours_reach_tree_dots_and_the_tag_editor() {
        let mut tags = built_in_tags();
        tags[0].display_color = Some("LightPink".into());
        tags[1].display_color = Some("sTeElBlUe".into());
        assert_eq!(TagTable::new(&tags).colors_of(&[HIDDEN, DEPRECATED]), [0xFFFF_B6C1, 0xFF46_82B4]);
        let rows = built_in_color_rows(&tags);
        assert_eq!(rows[0].color, 0xFFFF_B6C1);
        assert_eq!(rows[1].color, 0xFF46_82B4);
    }

    #[test]
    fn toggling_follows_the_three_states() {
        let mut s = session(&["A", "B"], 0);
        let a = s.row_path_of(s.tree.live_nodes().find(|(_, n)| n.name == "A").unwrap().0);
        let b = s.row_path_of(s.tree.live_nodes().find(|(_, n)| n.name == "B").unwrap().0);
        let prop = s.tag_menu(&[a.clone()]).into_iter().find(|t| t.name == "Prop").unwrap();
        assert_eq!(prop.check, TagCheck::Off);

        s.toggle_tag(&[a.clone()], prop.id, false);
        let state = |s: &SetupSession, rows: &[String]| s.tag_menu(rows).into_iter().find(|t| t.id == prop.id).unwrap().check;
        assert_eq!(state(&s, &[a.clone()]), TagCheck::On);
        assert_eq!(state(&s, &[a.clone(), b.clone()]), TagCheck::Mixed);
        assert_eq!(s.element_rows(None)[0].tag_colors, [0xFF00_00FF]);

        // Mixed adds to all.
        s.toggle_tag(&[a.clone(), b.clone()], prop.id, false);
        assert_eq!(state(&s, &[a.clone(), b.clone()]), TagCheck::On);
        // Fully on removes from all.
        s.toggle_tag(&[a.clone(), b.clone()], prop.id, false);
        assert_eq!(state(&s, &[a, b]), TagCheck::Off);
    }

    #[test]
    fn cascade_reaches_descendants_once() {
        let mut s = session(&["G", "N"], 0);
        let g = s.row_path_of(s.tree.live_nodes().find(|(_, n)| n.name == "G").unwrap().0);
        let n = s.row_path_of(s.tree.live_nodes().find(|(_, n)| n.name == "N").unwrap().0);
        s.drop_rows(&[n.clone()], Some(&g), None, false);
        let tag = s.tags[0].id;
        let touched = s.toggle_tag(&[g.clone()], tag, true);
        assert_eq!(touched.len(), 2);
        assert_eq!(s.element_rows(Some(&g))[0].tag_colors.len(), 1);
    }

    #[test]
    fn dot_colours_are_sorted_and_unknown_ids_skipped() {
        let s = session(&["A"], 0);
        let hidden = s.tags.iter().find(|t| t.name == "Hidden").unwrap().id;
        let deprecated = s.tags.iter().find(|t| t.name == "Deprecated").unwrap().id;
        assert_eq!(s.tag_table.colors_of(&[hidden, Uuid::new_v4(), deprecated]), [0xFFFF_0000, 0xFF00_0000]);
    }

    #[test]
    fn setting_colours_is_staged_in_the_session() {
        let mut s = session(&[], 0);
        let rows = s.tag_colors();
        s.set_tag_colors(&[(rows[0].id, rows[0].color)]);
        assert!(!s.dirty.tags, "the same colour changes nothing");
        s.set_tag_colors(&[(rows[0].id, 0xFF12_3456)]);
        assert!(s.dirty.tags);
        assert_eq!(s.tags.iter().find(|t| t.id == rows[0].id).unwrap().display_color.as_deref(), Some("#123456"));
    }

    #[test]
    fn standalone_manager_saves_colors_without_stopping_playback_or_changing_user_tags() {
        let root = std::env::temp_dir().join(format!("vixen-tag-manager-{}", Uuid::new_v4()));
        let core = crate::app::AppCore::open(root.clone(), true).unwrap();
        let mut user = built_in_tags()[0].clone();
        user.id = Uuid::new_v4();
        user.is_built_in = false;
        user.name = "Custom".into();
        core.system.write().tags.push(user.clone());
        let rows = core.element_tag_colors();
        assert_eq!(rows.len(), 3);
        let schedule = std::sync::Arc::new(vixen_engine::Schedule::empty(
            core.playback.elements().clone(), vixen_core::SequenceLayers::default().layers));
        let context = core.playback.start_sequence_context(schedule, None, vixen_core::TimeSpan::from_ms_i(10000));
        let revision = core.display.revision();
        core.save_element_tag_colors(&[(rows[0].id, 0xAA12_3456), (user.id, 0xFFFF_FFFF)]).unwrap();
        assert!(!context.is_finished(), "a color edit must not rebuild/stop playback");
        assert_eq!(core.display.revision(), revision + 1);
        let loaded = vixen_io::load_system(&core.paths).unwrap();
        assert_eq!(loaded.tags.iter().find(|t| t.id == rows[0].id).unwrap().display_color.as_deref(), Some("#123456"));
        assert_eq!(loaded.tags.iter().find(|t| t.id == user.id), Some(&user));
        context.stop();
        drop(core);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn standalone_manager_rolls_back_live_colors_when_saving_fails() {
        let root = std::env::temp_dir().join(format!("vixen-tag-manager-failure-{}", Uuid::new_v4()));
        let core = crate::app::AppCore::open(root.clone(), true).unwrap();
        let previous = core.system.read().tags.clone();
        let rows = core.element_tag_colors();
        std::fs::create_dir(root.join("SystemData/SystemConfig.xml")).unwrap();
        assert!(core.save_element_tag_colors(&[(rows[0].id, 0xFF12_3456)]).is_err());
        assert_eq!(core.system.read().tags, previous);
        assert_eq!(core.display.revision(), 0);
        drop(core);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn user_tags_follow_the_service_rules() {
        let mut s = session(&["A"], 0);
        assert_eq!(s.create_user_tag("  ").unwrap_err(), "Tag name cannot be blank.");
        let id = s.create_user_tag(" Mine ").unwrap();
        assert_eq!(s.create_user_tag("mine").unwrap_err(), "A tag named \"mine\" already exists.");
        s.rename_user_tag(id, "Yours").unwrap();
        let built_in = s.tags[0].id;
        assert!(s.rename_user_tag(built_in, "X").is_err());
        assert!(s.delete_user_tag(built_in).is_err());
        let a = s.row_path_of(s.tree.live_nodes().next().unwrap().0);
        s.toggle_tag(&[a.clone()], id, false);
        s.delete_user_tag(id).unwrap();
        assert!(s.tree.node(s.tree.resolve(&a).unwrap()[0]).tags.is_empty());
    }
}
