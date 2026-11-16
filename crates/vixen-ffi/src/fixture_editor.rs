//! Shared fixture editors used by the property dialogs and the setup wizard.
use crate::core::{FfiError, VixenCore};
use parking_lot::Mutex;
use std::sync::Arc;
use vixen_session::app::AppCore;
use vixen_session::setup::fixture::{self, FixtureProfile};
use vixen_session::setup::fixture_editor::{self as logic, ProfileEditor};

#[derive(uniffi::Record)]
pub struct FixtureChannelRow {
    pub id: u32,
    pub number: i32,
    pub name: String,
    pub function: String,
    pub incomplete: bool,
}
#[derive(uniffi::Record)]
pub struct FixtureFunctionRow {
    pub id: u32,
    pub name: String,
    pub function_type: String,
    pub tag: String,
    pub label: String,
    pub tags: Vec<String>,
}
#[derive(uniffi::Record)]
pub struct FixtureIndexRow {
    pub id: u32,
    pub name: String,
    pub start: String,
    pub end: String,
    pub use_curve: bool,
    pub tag: String,
    pub image: String,
    pub half: bool,
    pub color1: u32,
    pub color2: u32,
    pub exclude: bool,
}
#[derive(uniffi::Record)]
pub struct FixtureEditorState {
    pub name: String,
    pub manufacturer: String,
    pub revision: String,
    pub created_by: String,
    pub channels: Vec<FixtureChannelRow>,
    pub functions: Vec<FixtureFunctionRow>,
    pub function_names: Vec<String>,
    pub selected_function: Option<u32>,
    pub selected_channel: Option<u32>,
    pub selected_index: Option<u32>,
    pub panel: String,
    pub detail_title: String,
    pub indices: Vec<FixtureIndexRow>,
    pub show_images: bool,
    pub show_associated: bool,
    pub associated: String,
    pub prism_functions: Vec<String>,
    pub start: String,
    pub stop: String,
    pub zoom: String,
    pub rotation_tag: String,
    pub function_types: Vec<String>,
    pub index_tags: Vec<String>,
    pub errors: Vec<String>,
    pub function_errors: Vec<String>,
    pub channel_errors: Vec<String>,
    pub notice: Option<String>,
}
fn state_of(e: &ProfileEditor) -> FixtureEditorState {
    let mut names: Vec<_> = e
        .functions
        .iter()
        .filter_map(|f| f.value.name.clone())
        .collect();
    names.push("None".into());
    names.sort();
    let selected = e.selected.and_then(|i| e.functions.get(i));
    let panel = selected.map(|f| f.panel()).unwrap_or("none");
    let indices = selected
        .map(|f| {
            if panel == "wheel" {
                &f.wheel
            } else {
                &f.indices
            }
        })
        .map(|rows| {
            rows.iter()
                .enumerate()
                .map(|(i, r)| FixtureIndexRow {
                    id: i as u32,
                    name: r.name.clone(),
                    start: r.start.clone(),
                    end: r.end.clone(),
                    use_curve: r.use_curve,
                    tag: r.tag.clone(),
                    image: r.image.clone(),
                    half: r.half,
                    color1: r.color1.color().to_argb_u32(),
                    color2: r.color2.color().to_argb_u32(),
                    exclude: r.exclude,
                })
                .collect()
        })
        .unwrap_or_default();
    let text = |read: fn(&logic::FunctionDraft) -> String| selected.map(read).unwrap_or_default();
    FixtureEditorState {
        name: e.profile.name.clone().unwrap_or_default(),
        manufacturer: e.profile.manufacturer.clone().unwrap_or_default(),
        revision: e.profile.revision.clone().unwrap_or_default(),
        created_by: e.profile.created_by.clone().unwrap_or_default(),
        channels: e
            .profile
            .channels
            .iter()
            .enumerate()
            .map(|(i, c)| FixtureChannelRow {
                id: i as u32,
                number: c.channel_number,
                name: c.name.clone().unwrap_or_default(),
                function: c.function.clone().unwrap_or_else(|| "None".into()),
                incomplete: e
                    .functions
                    .iter()
                    .find(|f| f.value.name == c.function)
                    .is_some_and(|f| {
                        (f.panel() == "indexed" && f.indices.is_empty())
                            || (f.panel() == "wheel" && f.wheel.is_empty())
                    }),
            })
            .collect(),
        functions: e
            .functions
            .iter()
            .enumerate()
            .map(|(i, f)| FixtureFunctionRow {
                id: i as u32,
                name: f.value.name.clone().unwrap_or_default(),
                function_type: f.value.function_type.name().into(),
                tag: f.value.identity.name().into(),
                label: f.value.label.clone().unwrap_or_default(),
                tags: f.tags(),
            })
            .collect(),
        function_names: names,
        selected_function: e.selected.map(|i| i as u32),
        selected_channel: e.selected_channel.map(|i| i as u32),
        selected_index: e.selected_index.map(|i| i as u32),
        panel: panel.into(),
        detail_title: text(|f| {
            format!(
                "{} Details",
                f.value
                    .name
                    .as_deref()
                    .filter(|n| !n.is_empty())
                    .unwrap_or("<Function>")
            )
        }),
        indices,
        show_images: selected.is_some_and(|f| f.value.identity.name() == "Gobo"),
        show_associated: selected.is_some_and(|f| f.value.identity.name() == "OpenClosePrism"),
        associated: text(|f| f.value.associated_function_name.clone().unwrap_or_default()),
        prism_functions: e
            .functions
            .iter()
            .filter(|f| f.value.identity.name() == "Prism")
            .filter_map(|f| f.value.name.clone())
            .collect(),
        start: text(|f| f.start.clone()),
        stop: text(|f| f.stop.clone()),
        zoom: text(|f| format!("{:?}", f.value.zoom_type)),
        rotation_tag: text(|f| f.value.identity.name().into()),
        function_types: logic::FUNCTION_TYPES.iter().map(|s| (*s).into()).collect(),
        index_tags: logic::INDEX_TAGS.iter().map(|s| (*s).into()).collect(),
        errors: e.errors(),
        function_errors: e.functions_errors(),
        channel_errors: e.channel_errors(),
        notice: e.notice.clone(),
    }
}
#[derive(uniffi::Object)]
pub struct FixtureEditor {
    pub(crate) inner: Mutex<ProfileEditor>,
    core: Arc<AppCore>,
}
impl FixtureEditor {
    pub(crate) fn new(
        core: Arc<AppCore>,
        profile: FixtureProfile,
        selected: Option<&str>,
    ) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(ProfileEditor::new(profile, selected)),
            core,
        })
    }
    fn change(&self, f: impl FnOnce(&mut ProfileEditor)) -> FixtureEditorState {
        let mut e = self.inner.lock();
        f(&mut e);
        state_of(&e)
    }
}
#[uniffi::export]
impl FixtureEditor {
    pub fn state(&self) -> FixtureEditorState {
        state_of(&self.inner.lock())
    }
    pub fn set_profile_field(&self, field: String, value: String) -> FixtureEditorState {
        self.change(|e| e.set_profile_field(&field, value))
    }
    pub fn select_channel(&self, row: Option<u32>) -> FixtureEditorState {
        self.change(|e| e.selected_channel = row.map(|i| i as usize))
    }
    pub fn add_channel(&self) -> FixtureEditorState {
        self.change(ProfileEditor::add_channel)
    }
    pub fn remove_channel(&self, row: u32) -> FixtureEditorState {
        self.change(|e| e.remove_channel(row as usize))
    }
    pub fn set_channel_field(&self, row: u32, field: String, value: String) -> FixtureEditorState {
        self.change(|e| e.set_channel_field(row as usize, &field, value))
    }
    pub fn select_function(&self, row: Option<u32>) -> FixtureEditorState {
        self.change(|e| {
            e.try_select(row.map(|i| i as usize));
        })
    }
    pub fn add_function(&self) -> FixtureEditorState {
        self.change(ProfileEditor::add_function)
    }
    pub fn remove_function(&self, row: u32) -> FixtureEditorState {
        self.change(|e| e.remove_function(row as usize))
    }
    pub fn set_function_field(&self, row: u32, field: String, value: String) -> FixtureEditorState {
        self.change(|e| e.set_function_field(row as usize, &field, value))
    }
    pub fn select_index(&self, row: Option<u32>) -> FixtureEditorState {
        self.change(|e| e.selected_index = row.map(|i| i as usize))
    }
    pub fn add_index(&self) -> FixtureEditorState {
        self.change(ProfileEditor::add_index)
    }
    pub fn remove_index(&self, row: u32) -> FixtureEditorState {
        self.change(|e| e.remove_index(row as usize))
    }
    pub fn set_index_field(&self, row: u32, field: String, value: String) -> FixtureEditorState {
        self.change(|e| e.set_index_field(row as usize, &field, value))
    }
    pub fn edit_functions(&self, initial: Option<String>) -> Arc<FixtureEditor> {
        Self::new(
            Arc::clone(&self.core),
            self.inner.lock().profile(),
            initial.as_deref(),
        )
    }
    pub fn accept_functions(
        &self,
        editor: Arc<FixtureEditor>,
    ) -> Result<FixtureEditorState, FfiError> {
        let functions = {
            let other = editor.inner.lock();
            let errors = other.functions_errors();
            if !errors.is_empty() {
                return Err(FfiError::new(errors.join("\n")));
            }
            other.functions.clone()
        };
        Ok(self.change(|e| {
            e.functions = functions;
            e.reconcile_channels();
        }))
    }
    /// Library reads and writes run off the main actor. A property Cancel never mutates the library.
    pub fn profile_names(&self) -> Vec<String> {
        fixture::load_fixture_profiles(&self.core.paths.fixtures_dir())
            .profiles
            .into_iter()
            .map(|p| p.name_text().into())
            .collect()
    }
    pub fn load_profile(&self, name: String) -> Result<FixtureEditorState, FfiError> {
        let profile = fixture::load_fixture_profiles(&self.core.paths.fixtures_dir())
            .profiles
            .into_iter()
            .find(|p| p.name_text() == name)
            .ok_or_else(|| FfiError::new("That fixture profile no longer exists."))?;
        Ok(self.change(|e| *e = ProfileEditor::new(profile, None)))
    }
    pub fn save_library(&self) -> Result<(), FfiError> {
        let e = self.inner.lock();
        let errors = e.errors();
        if !errors.is_empty() {
            return Err(FfiError::new(errors.join("\n")));
        }
        fixture::save_fixture_profile(&self.core.paths.fixtures_dir(), &e.profile())
            .map_err(FfiError::new)
    }
    pub fn gobo_images(&self) -> Vec<String> {
        let mut names = vec![" ".into()];
        if let Ok(entries) = std::fs::read_dir(self.core.paths.fixtures_dir().join("Images")) {
            names.extend(
                entries
                    .flatten()
                    .filter(|e| e.path().is_file())
                    .map(|e| e.file_name().to_string_lossy().into_owned()),
            );
        }
        names[1..].sort();
        names
    }
    pub fn import_gobo_images(&self, paths: Vec<String>) -> Vec<String> {
        let dir = self.core.paths.fixtures_dir().join("Images");
        if std::fs::create_dir_all(&dir).is_ok() {
            for path in paths {
                let p = std::path::Path::new(&path);
                if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")) {
                    if let Some(name) = p.file_name() {
                        let _ = std::fs::copy(p, dir.join(name));
                    }
                }
            }
        }
        self.gobo_images()
    }
}
#[uniffi::export]
impl VixenCore {
    pub fn new_fixture_editor(&self) -> Arc<FixtureEditor> {
        FixtureEditor::new(Arc::clone(&self.inner), FixtureProfile::new_blank(), None)
    }
}

#[derive(uniffi::Object)]
pub struct FixtureDemo {
    inner: Mutex<vixen_session::setup::fixture_editor::FixtureDemo>,
}
#[uniffi::export]
impl FixtureDemo {
    #[uniffi::constructor]
    pub fn open(layer: u64, width: u32, height: u32, scale: f32) -> Result<Arc<Self>, FfiError> {
        // The native view retains the layer, and releases this object before dismantling the view.
        let demo = unsafe {
            vixen_session::setup::fixture_editor::FixtureDemo::open(layer, width, height, scale)
        }
        .map_err(FfiError::new)?;
        Ok(Arc::new(Self {
            inner: Mutex::new(demo),
        }))
    }
    pub fn resize(&self, width: u32, height: u32, scale: f32) {
        self.inner.lock().resize(width, height, scale);
    }
    pub fn set_pose(&self, pan: f32, tilt: f32, argb: u32) {
        self.inner.lock().set(pan, tilt, argb);
    }
    pub fn advance(&self, elapsed: f64, mode: String) {
        self.inner.lock().advance(elapsed, &mode);
    }
}
