//! Port of Vixen.Modules/Controller/DebugController. The Windows form becomes text the Swift side can show.

use super::{ConfigMismatch, ControllerConfig, ModuleShared, ModuleStatus, OutputModule};
use crate::command::OutputCommand;
use crate::ids;
use crate::policy::DataPolicy;
use crate::xmlutil::{self, ModuleIds};
use parking_lot::Mutex;
use std::sync::Arc;
use uuid::Uuid;
use vixen_xml::XElement;

/// The debug controller has no data class, so nothing is stored for it and this has no XML.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugConfig {
    pub ids: ModuleIds,
}

impl Default for DebugConfig {
    fn default() -> Self {
        DebugConfig { ids: ModuleIds::new(ids::DEBUG_CONTROLLER, Uuid::nil()) }
    }
}

impl DebugConfig {
    pub fn from_xml(e: &XElement) -> DebugConfig {
        DebugConfig { ids: xmlutil::ModuleIds::read(e) }
    }
}

/// `DebugControllerOutputForm.UpdateState`'s text: a line per output, with quiet outputs hidden unless `verbose`.
pub fn format_commands(time: &str, commands: &[OutputCommand], verbose: bool) -> String {
    let mut text = String::new();
    for (i, command) in commands.iter().enumerate() {
        let (value, show_if_quiet) = match command {
            OutputCommand::None => ("null".to_string(), false),
            OutputCommand::U8(v) => (format!("8-bit: {v}"), true),
            OutputCommand::U16(v) => (format!("16-bit: {v}"), true),
            OutputCommand::U32(v) => (format!("32-bit: {v}"), true),
            OutputCommand::U64(v) => (format!("64-bit: {v}"), true),
            // `Color.ToString()`, which is all a command's colour can be by the time it gets here.
            OutputCommand::Color(c) => (format!("Color: Color [A={}, R={}, G={}, B={}]", c.a, c.r, c.g, c.b), true),
            OutputCommand::Str(s) => (format!("String: {s}"), true),
        };
        if verbose || show_if_quiet {
            text.push_str(&format!("[{time}] [{}]: {value}\n", i + 1));
        }
    }
    if !text.is_empty() {
        text.push('\n');
    }
    text
}

/// `DebugControllerModule`.
pub struct DebugModule {
    config: DebugConfig,
    shared: Arc<ModuleShared>,
    output_count: usize,
    latest: Arc<Mutex<Vec<OutputCommand>>>,
}

impl DebugModule {
    pub fn new(config: DebugConfig) -> DebugModule {
        DebugModule { config, shared: ModuleShared::new(), output_count: 0, latest: Arc::new(Mutex::new(Vec::new())) }
    }

    /// The latest frame. Pass it to [`format_commands`] to show it like the form does.
    pub fn latest(&self) -> Arc<Mutex<Vec<OutputCommand>>> {
        self.latest.clone()
    }
}

impl OutputModule for DebugModule {
    fn type_id(&self) -> Uuid {
        ids::DEBUG_CONTROLLER
    }

    fn start(&mut self) {
        self.shared.set_status(ModuleStatus::Running);
    }

    fn stop(&mut self) {
        self.shared.set_status(ModuleStatus::Stopped);
    }

    fn update(&mut self, commands: &[OutputCommand]) {
        let mut latest = self.latest.lock();
        if latest.len() == commands.len() {
            latest.clone_from_slice(commands);
        } else {
            *latest = commands.to_vec();
        }
    }

    fn output_count(&self) -> usize {
        self.output_count
    }

    fn set_output_count(&mut self, count: usize) {
        self.output_count = count;
    }

    fn config(&self) -> ControllerConfig {
        ControllerConfig::Debug(self.config.clone())
    }

    fn set_config(&mut self, config: ControllerConfig) -> Result<(), ConfigMismatch> {
        let ControllerConfig::Debug(config) = config else {
            return Err(ConfigMismatch);
        };
        self.config = config;
        Ok(())
    }

    fn data_policy(&self) -> DataPolicy {
        DataPolicy::DISCARD_EXCESS_8BIT
    }

    fn shared(&self) -> &Arc<ModuleShared> {
        &self.shared
    }

    fn debug_frame(&self) -> Option<Arc<Mutex<Vec<OutputCommand>>>> {
        Some(self.latest.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_core::Color;

    #[test]
    fn text_matches_the_form() {
        let c = [OutputCommand::U8(5), OutputCommand::None, OutputCommand::U16(300), OutputCommand::Str(Arc::from("hi"))];
        assert_eq!(format_commands("t", &c, false), "[t] [1]: 8-bit: 5\n[t] [3]: 16-bit: 300\n[t] [4]: String: hi\n\n");
        assert!(format_commands("t", &c, true).contains("[t] [2]: null\n"));
        assert_eq!(
            format_commands("t", &[OutputCommand::Color(Color::RED)], false),
            "[t] [1]: Color: Color [A=255, R=255, G=0, B=0]\n\n"
        );
    }

    #[test]
    fn quiet_outputs_give_no_text() {
        assert_eq!(format_commands("t", &[OutputCommand::None], false), "");
    }

    #[test]
    fn keeps_the_latest_frame() {
        let mut m = DebugModule::new(DebugConfig::default());
        let latest = m.latest();
        m.start();
        m.update(&[OutputCommand::U8(1)]);
        m.update(&[OutputCommand::U8(2), OutputCommand::U64(9)]);
        assert_eq!(*latest.lock(), vec![OutputCommand::U8(2), OutputCommand::U64(9)]);
        m.stop();
    }
}
