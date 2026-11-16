//! Port of Vixen.Modules/Controller/LauncherController: a string command starts a process.

use super::{ConfigMismatch, ControllerConfig, ModuleShared, ModuleStatus, OutputModule};
use crate::command::OutputCommand;
use crate::ids;
use crate::policy::DataPolicy;
use crate::xmlutil::{self, bool_el, data_root, ModuleIds};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;
use uuid::Uuid;
use vixen_xml::{contract_ns, XElement};

const CLR_NS: &str = "VixenModules.Output.LauncherController";

/// `Data`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LauncherConfig {
    pub ids: ModuleIds,
    pub hide_launched_windows: bool,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        LauncherConfig { ids: ModuleIds::new(ids::LAUNCHER, Uuid::nil()), hide_launched_windows: false }
    }
}

impl LauncherConfig {
    pub fn from_xml(e: &XElement) -> LauncherConfig {
        LauncherConfig { ids: ModuleIds::read(e), hide_launched_windows: xmlutil::get_bool(e, "HideLaunchedWindows") }
    }

    pub fn to_xml(&self) -> XElement {
        let ns = contract_ns(CLR_NS);
        let mut e = data_root(&ns, "Data", &self.ids);
        e.push(bool_el(&ns, "HideLaunchedWindows", self.hide_launched_windows));
        e
    }
}

/// How `ProcessStartInfo.Arguments` is split on Unix: spaces and tabs separate, double quotes group, and a backslash is only
/// special in front of a double quote.
pub fn parse_arguments(arguments: &str) -> Vec<String> {
    let chars: Vec<char> = arguments.chars().collect();
    let mut results = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        while i < chars.len() && (chars[i] == ' ' || chars[i] == '\t') {
            i += 1;
        }
        if i == chars.len() {
            break;
        }
        results.push(next_argument(&chars, &mut i));
    }
    results
}

fn next_argument(chars: &[char], i: &mut usize) -> String {
    let mut current = String::new();
    let mut in_quotes = false;
    while *i < chars.len() {
        let mut backslashes = 0;
        while *i < chars.len() && chars[*i] == '\\' {
            *i += 1;
            backslashes += 1;
        }
        if backslashes > 0 {
            if *i >= chars.len() || chars[*i] != '"' {
                current.extend(std::iter::repeat_n('\\', backslashes));
            } else {
                current.extend(std::iter::repeat_n('\\', backslashes / 2));
                if backslashes % 2 != 0 {
                    current.push('"');
                    *i += 1;
                }
            }
            continue;
        }
        let c = chars[*i];
        if c == '"' {
            if in_quotes && *i < chars.len() - 1 && chars[*i + 1] == '"' {
                current.push('"');
                *i += 1;
            } else {
                in_quotes = !in_quotes;
            }
            *i += 1;
            continue;
        }
        if (c == ' ' || c == '\t') && !in_quotes {
            break;
        }
        current.push(c);
        *i += 1;
    }
    current
}

/// `Module.Launch`: starts the process in the background and waits for it there. A missing file is only logged, so a path saved
/// on Windows reports an error here and nothing else happens.
pub(crate) fn launch(hide_windows: bool, executable: &str, arguments: &str, shared: &Arc<ModuleShared>) {
    if !Path::new(executable).is_file() {
        shared.record_error(format!("File Not found to Launch: [{executable}]"));
        return;
    }
    let mut command = Command::new(executable);
    if !arguments.trim().is_empty() {
        command.args(parse_arguments(arguments));
    }
    // `CreateNoWindow`. A process here has no console window to hide, but it can be kept off the app's streams.
    if hide_windows {
        command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    }
    let on_thread = shared.clone();
    let executable = executable.to_string();
    let started = std::thread::Builder::new().name("launcher".to_string()).spawn(move || {
        if let Err(e) = command.spawn().and_then(|mut child| child.wait()) {
            on_thread.record_error(format!("{executable}: {e}"));
        }
    });
    if let Err(e) = started {
        shared.record_error(format!("Couldn't start the launcher's thread: {e}"));
    }
}

/// The per-output memory both modules keep, so a command only fires when it changes.
#[derive(Clone, Debug, Default)]
pub(crate) struct LastCommands {
    values: Vec<Option<Arc<str>>>,
}

impl LastCommands {
    /// `_lastCommandValues[index] = value`, returning what was there.
    #[inline]
    pub(crate) fn replace(&mut self, index: usize, value: Option<Arc<str>>) -> Option<Arc<str>> {
        if index >= self.values.len() {
            self.values.resize(index + 1, None);
        }
        std::mem::replace(&mut self.values[index], value)
    }
}

/// `value.Split('|')[1].Split(',')`, taking `[0]` and `[1]`. `None` where the C# would throw.
pub(crate) fn launcher_arguments(value: &str) -> Option<(&str, &str)> {
    let mut parts = value.split('|');
    parts.next()?;
    let mut args = parts.next()?.split(',');
    Some((args.next()?, args.next()?))
}

/// `Module` in the Launcher controller.
pub struct LauncherModule {
    config: LauncherConfig,
    shared: Arc<ModuleShared>,
    output_count: usize,
    last: LastCommands,
}

impl LauncherModule {
    pub fn new(config: LauncherConfig) -> LauncherModule {
        LauncherModule { config, shared: ModuleShared::new(), output_count: 0, last: LastCommands::default() }
    }
}

impl OutputModule for LauncherModule {
    fn type_id(&self) -> Uuid {
        ids::LAUNCHER
    }

    fn start(&mut self) {
        self.shared.set_status(ModuleStatus::Running);
    }

    fn stop(&mut self) {
        self.shared.set_status(ModuleStatus::Stopped);
    }

    fn update(&mut self, commands: &[OutputCommand]) {
        for (index, command) in commands.iter().enumerate() {
            let OutputCommand::Str(value) = command else {
                self.last.replace(index, None);
                continue;
            };
            let last = self.last.replace(index, Some(value.clone()));
            let command_type = value.split('|').next().unwrap_or("");
            if command_type.eq_ignore_ascii_case("LAUNCHER") {
                if last.as_deref() == Some(&**value) {
                    continue;
                }
                match launcher_arguments(value) {
                    Some((executable, arguments)) => {
                        launch(self.config.hide_launched_windows, executable, arguments, &self.shared)
                    }
                    // The index is out of range in the C#. The exception ends this frame's loop.
                    None => {
                        self.shared.record_error(format!("Malformed launcher command: {value}"));
                        return;
                    }
                }
            }
        }
    }

    fn output_count(&self) -> usize {
        self.output_count
    }

    fn set_output_count(&mut self, count: usize) {
        self.output_count = count;
    }

    fn config(&self) -> ControllerConfig {
        ControllerConfig::Launcher(self.config.clone())
    }

    fn set_config(&mut self, config: ControllerConfig) -> Result<(), ConfigMismatch> {
        let ControllerConfig::Launcher(config) = config else {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments_split_like_process_start_info() {
        assert_eq!(parse_arguments("a b\tc"), vec!["a", "b", "c"]);
        assert_eq!(parse_arguments(r#""a b" c"#), vec!["a b", "c"]);
        assert_eq!(parse_arguments(r#"a\b"#), vec![r"a\b"]);
        assert_eq!(parse_arguments(r#"a\"b"#), vec![r#"a"b"#]);
        assert_eq!(parse_arguments(r#"a\\"b c""#), vec![r"a\b c"]);
        assert_eq!(parse_arguments(r#""a""b""#), vec![r#"a"b"#]);
        assert!(parse_arguments("   ").is_empty());
    }

    #[test]
    fn command_text_splits_on_pipe_then_comma() {
        assert_eq!(launcher_arguments("Launcher|/bin/echo,hello"), Some(("/bin/echo", "hello")));
        assert_eq!(launcher_arguments("Launcher|/bin/echo"), None);
        assert_eq!(launcher_arguments("Launcher"), None);
    }

    #[test]
    fn a_windows_path_only_records_an_error() {
        let mut m = LauncherModule::new(LauncherConfig::default());
        m.set_output_count(1);
        m.start();
        m.update(&[OutputCommand::Str(Arc::from(r"Launcher|C:\Program Files\app.exe,/x"))]);
        assert!(m.shared().last_error().unwrap().contains("File Not found"));
        // Not again for the same command.
        let before = m.shared().error_count();
        m.update(&[OutputCommand::Str(Arc::from(r"Launcher|C:\Program Files\app.exe,/x"))]);
        assert_eq!(m.shared().error_count(), before);
    }

    #[test]
    fn malformed_command_ends_the_frame_without_retrying() {
        let mut m = LauncherModule::new(LauncherConfig::default());
        m.start();
        let bad = OutputCommand::Str(Arc::from("LAUNCHER"));
        m.update(&[bad.clone()]);
        assert_eq!(m.shared().error_count(), 1);
        // `_lastCommandValues` was set before the exception, so the same text is now a repeat.
        m.update(&[bad]);
        assert_eq!(m.shared().error_count(), 1);
    }

    #[test]
    fn other_commands_and_clearing_are_handled() {
        let mut m = LauncherModule::new(LauncherConfig::default());
        m.start();
        m.update(&[OutputCommand::Str(Arc::from("Other|x")), OutputCommand::U8(1), OutputCommand::None]);
        assert_eq!(m.shared().error_count(), 0);
        let cmd = OutputCommand::Str(Arc::from("Launcher|/no/such/file,x"));
        m.update(&[cmd.clone()]);
        assert_eq!(m.shared().error_count(), 1);
        // A frame without the command clears the memory, so it fires again after.
        m.update(&[OutputCommand::None]);
        m.update(&[cmd]);
        assert_eq!(m.shared().error_count(), 2);
    }
}
