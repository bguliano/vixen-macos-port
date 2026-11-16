//! Streaming ACN (E1.31) setup. The draft stays in Rust, since Add Multiple can make 63,999 rows.

use super::{no_such, parse_id, DialogContext};
use crate::core::FfiError;
use parking_lot::Mutex;
use std::sync::Arc;
use uuid::Uuid;
use vixen_output::modules::ControllerConfig;
use vixen_output::net;
use vixen_session::dialogs::e131 as logic;
use vixen_session::dialogs::e131::{E131Destination, E131Draft, E131OkStep, E131Peer, UniverseColumn};
use vixen_session::dialogs::world::DialogWorld;

#[derive(uniffi::Enum, Copy, Clone, PartialEq, Eq, Debug)]
pub enum DialogUniverseColumn {
    Start,
    Active,
    Universe,
    Size,
}

impl From<DialogUniverseColumn> for UniverseColumn {
    fn from(c: DialogUniverseColumn) -> Self {
        match c {
            DialogUniverseColumn::Start => UniverseColumn::Start,
            DialogUniverseColumn::Active => UniverseColumn::Active,
            DialogUniverseColumn::Universe => UniverseColumn::Universe,
            DialogUniverseColumn::Size => UniverseColumn::Size,
        }
    }
}

#[derive(uniffi::Record, Clone, Copy, PartialEq, Eq)]
pub struct DialogUniverseRow {
    pub start: i32,
    pub active: bool,
    pub universe: i32,
    pub size: i32,
}

#[derive(uniffi::Record, Clone, PartialEq, Eq)]
pub struct DialogE131Options {
    pub manual: bool,
    pub warnings: bool,
    pub statistics: bool,
    pub blind: bool,
    pub priority: i32,
    pub repeat_text: String,
    pub suppress_text: String,
}

#[derive(uniffi::Record, Clone, PartialEq, Eq)]
pub struct DialogDestination {
    pub text: String,
    pub is_multicast: bool,
    /// The address, for a unicast entry.
    pub address: Option<String>,
}

#[derive(uniffi::Record, Clone, PartialEq, Eq)]
pub struct DialogDestinationList {
    pub entries: Vec<DialogDestination>,
    pub selected: Option<u32>,
    /// Show this first, when a saved interface isn't on this Mac.
    pub missing_interface_alert: Option<String>,
}

#[derive(uniffi::Enum, Clone, PartialEq, Eq)]
pub enum DialogE131Step {
    /// "Save Anyway" goes on, "Go Back" stays.
    Warn { title: String, text: String },
    /// "Review Settings" stays, "Continue" goes on.
    ReviewOrContinue { text: String },
    /// OK stays.
    Block { title: String, text: String },
}

/// One open E1.31 sheet. While it's open the controller sends nothing (F51), and [`E131Session::close`] lets it carry on.
#[derive(uniffi::Object)]
pub struct E131Session {
    context: Arc<DialogContext>,
    controller: Uuid,
    original: vixen_output::modules::E131Config,
    state: Mutex<(E131Draft, logic::E131DestinationList)>,
}

fn peers(world: &dyn DialogWorld, excluding: &Uuid) -> Vec<E131Peer> {
    world
        .controllers()
        .into_iter()
        .filter(|c| c.running && c.id != *excluding)
        .filter_map(|c| match world.controller_config(&c.id)? {
            ControllerConfig::E131(config) => Some(E131Peer::from_config(&config)),
            _ => None,
        })
        .collect()
}

fn destination_list(saved: &E131Destination) -> logic::E131DestinationList {
    let choices = net::multicast_choices(saved.multicast.as_deref());
    let primary = net::primary_interface_id();
    logic::e131_destination_list(&choices, &logic::known_unicasts(), saved, primary.as_deref())
}

#[uniffi::export]
impl DialogContext {
    /// Opens the sheet's draft, and pauses the controller until it's closed.
    pub fn open_e131(self: Arc<Self>, controller: String) -> Result<Arc<E131Session>, FfiError> {
        let id = parse_id(&controller)?;
        let (original, draft) = self.world(|world| {
            let info = world.controllers().into_iter().find(|c| c.id == id).ok_or_else(|| no_such("controller"))?;
            let ControllerConfig::E131(config) = world.controller_config(&id).ok_or_else(|| no_such("controller"))? else {
                return Err(no_such("E1.31 controller"));
            };
            // The table fills from every controller's saved destination.
            for c in world.controllers() {
                if let Some(ControllerConfig::E131(other)) = world.controller_config(&c.id) {
                    if let Some(unicast) = other.unicast.as_deref().filter(|u| !u.is_empty()) {
                        logic::remember_unicast(unicast);
                    }
                }
            }
            let draft = logic::e131_draft(&config, &info.name, info.output_count);
            Ok((config, draft))
        })?;
        let saved = E131Destination { unicast: original.unicast.clone(), multicast: original.multicast.clone() };
        let list = destination_list(&saved);
        self.core.playback.with_output(|_, runner| {
            if let Some(i) = runner.controller_index(&id) {
                runner.set_paused(i, true);
            }
        });
        Ok(Arc::new(E131Session { context: self, controller: id, original, state: Mutex::new((draft, list)) }))
    }

    /// F79.
    pub fn e131_repeat_is_valid(&self, text: String) -> bool {
        logic::e131_validate_repeat(&text).is_some()
    }

    /// F80.
    pub fn e131_suppress_is_valid(&self, text: String) -> bool {
        logic::e131_validate_suppress(&text).is_some()
    }

    /// F57, F58: whether New Unicast Destination's OK is available.
    pub fn unicast_text_is_valid(&self, text: String, is_ip: bool) -> bool {
        if is_ip {
            logic::unicast_ip_is_valid(&text)
        } else {
            logic::unicast_name_is_valid(&text)
        }
    }
}

impl E131Session {
    fn current_destination(&self) -> E131Destination {
        let state = self.state.lock();
        let list = &state.1;
        list.selected.and_then(|i| list.entries.get(i)).map_or_else(E131Destination::default, |e| E131Destination {
            unicast: e.unicast.clone(),
            multicast: e.multicast.clone(),
        })
    }

    fn rebuild_destinations(&self, select_unicast: Option<&str>) {
        let saved = match select_unicast {
            Some(address) => E131Destination { unicast: Some(address.to_string()), multicast: None },
            None => self.current_destination(),
        };
        let mut list = destination_list(&saved);
        // The alert has had its chance, when the sheet opened.
        list.missing_interface = false;
        self.state.lock().1 = list;
    }
}

#[uniffi::export]
impl E131Session {
    pub fn title(&self) -> String {
        self.state.lock().0.title.clone()
    }

    pub fn rows(&self) -> Vec<DialogUniverseRow> {
        self.state
            .lock()
            .0
            .rows
            .iter()
            .map(|r| DialogUniverseRow { start: r.start, active: r.active, universe: r.universe, size: r.size })
            .collect()
    }

    pub fn options(&self) -> DialogE131Options {
        let state = self.state.lock();
        let d = &state.0;
        DialogE131Options {
            manual: d.manual,
            warnings: d.warnings,
            statistics: d.statistics,
            blind: d.blind,
            priority: d.priority,
            repeat_text: d.repeat_text.clone(),
            suppress_text: d.suppress_text.clone(),
        }
    }

    pub fn set_options(&self, options: DialogE131Options) {
        let mut state = self.state.lock();
        let d = &mut state.0;
        d.set_manual(options.manual);
        d.warnings = options.warnings;
        d.statistics = options.statistics;
        d.blind = options.blind;
        d.priority = options.priority;
        d.repeat_text = options.repeat_text;
        d.suppress_text = options.suppress_text;
    }

    /// F66: a cell's new text. The result is the alert's text when it can't be used.
    pub fn set_cell(&self, row: u32, column: DialogUniverseColumn, text: String) -> Option<String> {
        self.state.lock().0.set_cell(row as usize, column.into(), &text).err()
    }

    /// F63.
    pub fn set_active(&self, row: u32, active: bool) {
        if let Some(r) = self.state.lock().0.rows.get_mut(row as usize) {
            r.active = active;
        }
    }

    /// F69. The result is an error's text.
    pub fn add_universe(&self) -> Option<String> {
        let mut state = self.state.lock();
        let manual = state.0.manual;
        logic::e131_add_universe(&mut state.0.rows, manual).err()
    }

    /// F70: how many more universes fit.
    pub fn max_universes_to_add(&self) -> i32 {
        logic::e131_max_to_add(&self.state.lock().0.rows)
    }

    /// F70, in one operation.
    pub fn add_universes(&self, count: i32) {
        let mut state = self.state.lock();
        let manual = state.0.manual;
        logic::e131_add_universes(&mut state.0.rows, count, manual);
    }

    /// F68, F71.
    pub fn delete_rows(&self, rows: Vec<u32>) {
        let indices: Vec<usize> = rows.iter().map(|r| *r as usize).collect();
        let mut state = self.state.lock();
        let manual = state.0.manual;
        logic::e131_delete_rows(&mut state.0.rows, &indices, manual);
    }

    /// F72.
    pub fn delete_all_rows(&self) {
        logic::e131_delete_all(&mut self.state.lock().0.rows);
    }

    /// F67. The result is the warning to show.
    pub fn sort(&self, column: DialogUniverseColumn, ascending: bool) -> Option<String> {
        self.state.lock().0.sort(column.into(), ascending)
    }

    pub fn destinations(&self) -> DialogDestinationList {
        let state = self.state.lock();
        let list = &state.1;
        DialogDestinationList {
            entries: list
                .entries
                .iter()
                .map(|e| DialogDestination { text: e.text.clone(), is_multicast: e.is_multicast(), address: e.unicast.clone() })
                .collect(),
            selected: list.selected.map(|i| i as u32),
            missing_interface_alert: list.missing_interface.then(|| logic::MISSING_INTERFACE_TEXT.to_string()),
        }
    }

    pub fn select_destination(&self, index: Option<u32>) {
        self.state.lock().1.selected = index.map(|i| i as usize);
    }

    /// F54. Either way, the address ends up selected. The result is the alert's text.
    pub fn add_unicast(&self, text: String) -> Option<String> {
        let text = text.trim().to_string();
        let error = logic::add_known_unicast(&text).err();
        self.rebuild_destinations(Some(&text));
        error
    }

    /// F55. The first entry ends up selected. The result is the alert's text.
    pub fn remove_selected_unicast(&self) -> Option<String> {
        let destination = self.current_destination();
        let Some(address) = destination.unicast else {
            return destination.multicast.map(|_| logic::MULTICAST_NOT_REMOVABLE.to_string());
        };
        let others = self.context.world(|world| peers(world, &self.controller));
        if let Err(error) = logic::remove_known_unicast(&address, &others) {
            return Some(error);
        }
        self.rebuild_destinations(None);
        let mut state = self.state.lock();
        state.1.selected = (!state.1.entries.is_empty()).then_some(0);
        None
    }

    /// F83 to F87, for the sheet to go through one at a time.
    pub fn ok_steps(&self) -> Vec<DialogE131Step> {
        let others = self.context.world(|world| peers(world, &self.controller));
        let destination = self.current_destination();
        let state = self.state.lock();
        logic::e131_ok_steps(&state.0.rows, &destination, state.0.output_count, &others)
            .into_iter()
            .map(|step| match step {
                E131OkStep::Warn { title, text } => DialogE131Step::Warn { title, text },
                E131OkStep::ReviewOrContinue { text } => DialogE131Step::ReviewOrContinue { text },
                E131OkStep::Block { title, text } => DialogE131Step::Block { title, text },
            })
            .collect()
    }

    /// F88. The controller restarts only if it was running.
    pub fn apply(&self) {
        let destination = self.current_destination();
        let (rows, options, output_count) = {
            let state = self.state.lock();
            (state.0.rows.clone(), state.0.options(), state.0.output_count)
        };
        let mut config = self.original.clone();
        logic::e131_apply(&mut config, &rows, &options, &destination);
        let names = logic::e131_output_names(&rows, destination.unicast.as_deref(), output_count);
        self.context.world(|world| {
            world.set_output_names(&self.controller, &names);
            world.set_controller_config(&self.controller, ControllerConfig::E131(config));
        });
    }

    /// The sheet is gone, so the controller can carry on.
    pub fn close(&self) {
        self.context.core.playback.with_output(|_, runner| {
            if let Some(i) = runner.controller_index(&self.controller) {
                runner.set_paused(i, false);
            }
        });
    }
}
