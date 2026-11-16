//! The output windows of the Dummy Lighting and Debugging Values controllers (F46 to F50, F134), and messages from running controllers.

use super::{no_such, parse_id, DialogContext};
use crate::core::FfiError;
use parking_lot::Mutex;
use std::sync::Arc;
use uuid::Uuid;
use vixen_output::command::OutputCommand;
use vixen_output::ids;
use vixen_output::modules::dummy::DummyLightingView;
use vixen_output::modules::ControllerConfig;
use vixen_session::app::AppCore;
use vixen_session::dialogs::controllers as logic;
use vixen_session::dialogs::world::DialogWorld;

#[derive(uniffi::Enum, Copy, Clone, PartialEq, Eq, Debug)]
pub enum DialogMonitorKind {
    Dummy,
    Debug,
}

/// A controller with an output window, running now.
#[derive(uniffi::Record, Clone, PartialEq, Eq)]
pub struct DialogMonitor {
    pub controller: String,
    pub kind: DialogMonitorKind,
    pub title: String,
}

/// What the Dummy Lighting window draws, read once per display refresh.
#[derive(uniffi::Record, Clone)]
pub struct DialogDummyFrame {
    pub across: u32,
    pub down: u32,
    /// One RGBA per box, left to right and top to bottom.
    pub rgba: Vec<u8>,
    pub fps: String,
}

/// F46: the Dummy Lighting window's view of a running controller. Reads the latest frame, without going near the graph or the engine.
#[derive(uniffi::Object)]
pub struct DummyMonitor {
    view: Arc<Mutex<DummyLightingView>>,
}

#[uniffi::export]
impl DummyMonitor {
    /// F47 to F49.
    pub fn frame(&self) -> DialogDummyFrame {
        let view = self.view.lock();
        let grid = logic::dummy_grid_layout(view.outputs as u32, view.style);
        let mut rgba = Vec::new();
        logic::dummy_frame_rgba(view.style, grid, &view, &mut rgba);
        DialogDummyFrame { across: grid.across, down: grid.down, rgba, fps: logic::format_fps(view.fps) }
    }
}

/// F134: the Debugging Values window's view of a running controller.
#[derive(uniffi::Object)]
pub struct DebugMonitor {
    core: Arc<AppCore>,
    frame: Arc<Mutex<Vec<OutputCommand>>>,
}

#[uniffi::export]
impl DebugMonitor {
    /// The window's text for the latest frame. `None` means nothing to show, and the window keeps what it has.
    pub fn text(&self, verbose: bool) -> Option<String> {
        let time = self.core.playback.position().total_ms().max(0.0) as u64;
        let time = format!("{}:{:02}.{:03}", time / 60_000, (time % 60_000) / 1000, time % 1000);
        logic::debug_output_text(&time, &self.frame.lock(), verbose)
    }
}

/// Something a running controller wants the user to see (F90 to F92). `heading` means the scrolling message box.
#[derive(uniffi::Record, Clone, PartialEq, Eq)]
pub struct DialogControllerMessage {
    pub title: String,
    pub heading: Option<String>,
    pub text: String,
}

fn dummy_title(world: &dyn DialogWorld, id: &Uuid) -> String {
    match world.controller_config(id) {
        Some(ControllerConfig::DummyLighting(c)) => c.form_title.unwrap_or_default(),
        _ => String::new(),
    }
}

#[uniffi::export]
impl DialogContext {
    /// F46, F134: the controllers that should have a window open now. The window closes when one leaves this list.
    pub fn running_monitors(&self) -> Vec<DialogMonitor> {
        let running = vixen_session::dialogs::store::running_controllers(&self.core);
        self.world(|world| {
            world
                .controllers()
                .into_iter()
                .filter(|c| running.contains(&c.id))
                .filter_map(|c| match c.type_id {
                    ids::DUMMY_LIGHTING => Some(DialogMonitor {
                        controller: c.id.to_string(),
                        kind: DialogMonitorKind::Dummy,
                        title: dummy_title(world, &c.id),
                    }),
                    ids::DEBUG_CONTROLLER => Some(DialogMonitor {
                        controller: c.id.to_string(),
                        kind: DialogMonitorKind::Debug,
                        title: "Debug Controller Output".to_string(),
                    }),
                    _ => None,
                })
                .collect()
        })
    }

    pub fn dummy_monitor(&self, controller: String) -> Result<Arc<DummyMonitor>, FfiError> {
        let id = parse_id(&controller)?;
        let view = self.core.playback.with_output(|_, runner| runner.controller_index(&id).and_then(|i| runner.dummy_view(i)));
        Ok(Arc::new(DummyMonitor { view: view.ok_or_else(|| no_such("Dummy Lighting controller"))? }))
    }

    pub fn debug_monitor(&self, controller: String) -> Result<Arc<DebugMonitor>, FfiError> {
        let id = parse_id(&controller)?;
        let frame = self.core.playback.with_output(|_, runner| runner.controller_index(&id).and_then(|i| runner.debug_frame(i)));
        Ok(Arc::new(DebugMonitor {
            core: Arc::clone(&self.core),
            frame: frame.ok_or_else(|| no_such("Debugging Values controller"))?,
        }))
    }

    /// F90 to F92: what's waiting to be shown. Taking them clears them. The engine never waits on an alert.
    pub fn take_controller_messages(&self) -> Vec<DialogControllerMessage> {
        self.core.playback.with_output(|_, runner| {
            (0..runner.controller_count())
                .filter_map(|i| runner.shared(i))
                .flat_map(|shared| shared.take_messages())
                .map(|m| DialogControllerMessage { title: m.title, heading: m.heading, text: m.text })
                .collect()
        })
    }
}
