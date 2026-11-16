use super::element_mapper::ElementMapperDialog;
use crate::core::{FfiError, VixenCore};
use parking_lot::Mutex;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use vixen_session::{
    app::AppCore,
    sequence_package::{self as package, wizard::PackageWizard, Listener, RunResult},
};

#[derive(Clone, uniffi::Record)]
pub struct PackageRow {
    pub name: String,
    pub path: String,
    pub selected: bool,
}
#[derive(Clone, uniffi::Record)]
pub struct PackageView {
    pub importing: bool,
    pub stage: u32,
    pub can_next: bool,
    pub running: bool,
    pub rows: Vec<PackageRow>,
    pub output: String,
    pub include_audio: bool,
    pub input: String,
    pub map: String,
    pub can_create_map: bool,
    pub can_edit_map: bool,
    pub sequence_folder: String,
    pub final_text: String,
    pub completed: u32,
    pub skipped: u32,
    pub errors: Vec<String>,
}
#[uniffi::export(callback_interface)]
pub trait PackageListener: Send + Sync {
    fn progress(&self, task: String, task_percent: u32, overall_percent: u32);
    fn overwrite_package(&self, path: String) -> bool;
    fn sequence_destination(&self, name: String, path: String) -> Option<String>;
    fn warning(&self, message: String);
    fn finished(&self);
}
struct Forward(Box<dyn PackageListener>);
impl Listener for Forward {
    fn progress(&self, t: &str, p: u32, o: u32) {
        self.0.progress(t.into(), p, o);
    }
    fn overwrite_package(&self, p: &Path) -> bool {
        self.0.overwrite_package(p.to_string_lossy().into())
    }
    fn sequence_destination(&self, n: &str, p: &Path) -> Option<PathBuf> {
        self.0
            .sequence_destination(n.into(), p.to_string_lossy().into())
            .map(PathBuf::from)
    }
    fn warning(&self, m: &str) {
        self.0.warning(m.into());
    }
}
#[derive(uniffi::Object)]
pub struct PackageSession {
    core: Arc<AppCore>,
    state: Mutex<PackageWizard>,
    cancel: AtomicBool,
}
#[uniffi::export]
impl VixenCore {
    pub fn package_wizard(&self, importing: bool) -> Arc<PackageSession> {
        let state = PackageWizard::new(importing, &self.inner);
        Arc::new(PackageSession {
            core: Arc::clone(&self.inner),
            state: Mutex::new(state),
            cancel: AtomicBool::new(false),
        })
    }
}
#[uniffi::export]
impl PackageSession {
    pub fn view(&self) -> PackageView {
        let s = self.state.lock();
        let rows = if s.importing {
            s.index
                .as_ref()
                .map(|i| {
                    i.sequences
                        .iter()
                        .map(|r| PackageRow {
                            name: r.name.clone(),
                            path: String::new(),
                            selected: r.selected,
                        })
                        .collect()
                })
                .unwrap_or_default()
        } else {
            s.settings
                .sequences
                .iter()
                .map(|p| PackageRow {
                    name: Path::new(p)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into(),
                    path: p.clone(),
                    selected: true,
                })
                .collect()
        };
        PackageView {
            importing: s.importing,
            stage: s.stage,
            can_next: s.can_next(),
            running: s.running,
            rows,
            output: s.settings.output.clone(),
            include_audio: s.settings.audio,
            input: s.input.clone(),
            map: s.map.clone(),
            can_create_map: s.index.is_some(),
            can_edit_map: Path::new(&s.map).is_file(),
            sequence_folder: self.core.paths.sequence_dir().to_string_lossy().into(),
            final_text: s.final_text.clone(),
            completed: s.result.as_ref().map_or(0, |r| r.completed as u32),
            skipped: s.result.as_ref().map_or(0, |r| r.skipped as u32),
            errors: s
                .result
                .as_ref()
                .map(|r| r.errors.clone())
                .unwrap_or_default(),
        }
    }
    pub fn add_sequences(&self, paths: Vec<String>) -> bool {
        self.state.lock().add(paths)
    }
    pub fn add_all(&self) {
        self.state.lock().add_all();
    }
    pub fn remove_rows(&self, rows: Vec<u32>) {
        self.state
            .lock()
            .remove(&rows.into_iter().map(|n| n as usize).collect::<Vec<_>>());
    }
    pub fn set_selected(&self, rows: Vec<u32>, selected: bool) {
        self.state.lock().set_selected(
            &rows.into_iter().map(|n| n as usize).collect::<Vec<_>>(),
            selected,
        );
    }
    pub fn set_output(&self, path: String) {
        let mut s = self.state.lock();
        if !s.running {
            s.settings.output = path;
        }
    }
    pub fn set_audio(&self, include: bool) {
        let mut s = self.state.lock();
        if !s.running {
            s.settings.audio = include;
        }
    }
    pub fn set_input(&self, path: String) -> Result<(), FfiError> {
        self.state.lock().set_input(path).map_err(Into::into)
    }
    pub fn set_map(&self, path: String) {
        let mut s = self.state.lock();
        if !s.running {
            s.map = path;
        }
    }
    pub fn next(&self) -> bool {
        self.state.lock().next()
    }
    pub fn previous(&self) {
        self.state.lock().previous();
    }
    pub fn mapper(&self, edit: bool) -> Result<Arc<ElementMapperDialog>, FfiError> {
        Ok(ElementMapperDialog::from_state(
            self.state.lock().mapper(&self.core, edit)?,
        ))
    }
    pub fn start(self: Arc<Self>, listener: Box<dyn PackageListener>) -> Result<(), FfiError> {
        let (settings, input, map, selected, importing) = {
            let mut s = self.state.lock();
            if s.stage != 2 || s.running {
                return Err(FfiError::new("The package wizard is not ready to run."));
            }
            s.running = true;
            (
                s.settings.clone(),
                s.input.clone(),
                s.map.clone(),
                s.selected_names(),
                s.importing,
            )
        };
        self.cancel.store(false, Ordering::Release);
        let recover = Arc::clone(&self);
        std::thread::Builder::new()
            .name("vixen-package".into())
            .spawn(move || {
                let forward = Forward(listener);
                let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    if importing {
                        package::import(
                            &self.core,
                            Path::new(&input),
                            Path::new(&map),
                            &selected,
                            &self.cancel,
                            &forward,
                        )
                    } else {
                        package::export(&self.core, &settings, &self.cancel, &forward)
                    }
                }));
                let result = match run {
                    Ok(Ok(r)) => r,
                    Ok(Err(e)) => RunResult {
                        cancelled: self.cancel.load(Ordering::Acquire),
                        errors: if self.cancel.load(Ordering::Acquire) {
                            vec![]
                        } else {
                            vec![e.to_string()]
                        },
                        ..Default::default()
                    },
                    Err(_) => RunResult {
                        errors: vec!["The package operation failed unexpectedly.".into()],
                        ..Default::default()
                    },
                };
                self.state.lock().finish(result);
                forward.0.finished();
            })
            .map_err(|e| {
                recover.state.lock().running = false;
                FfiError::new(e.to_string())
            })?;
        Ok(())
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
    }
    /// Finish accepts the export working copy; Cancel still flushes existing module data.
    pub fn close(&self, accept: bool) -> Result<(), FfiError> {
        let s = self.state.lock();
        if s.running {
            return Err(FfiError::new("Wait for the package operation to stop."));
        }
        if accept && !s.importing {
            s.settings
                .store(&mut self.core.system.write().module_store, &self.core.paths);
        }
        drop(s);
        self.core
            .save_module_config()
            .map(|_| ())
            .map_err(Into::into)
    }
}
