//! The Intelligent Fixture wizard's API. The wizard's state lives in Rust: each call returns all the window shows, and is quick enough for the main thread.
//! Only opening the wizard and finishing it touch the disk, so those two go off it.

use super::{edit_result, DisplaySetupSession, SetupEditResult, SetupTemplateProblem};
use crate::core::{FfiError, VixenCore};
use crate::fixture_editor::FixtureEditor;
use crate::kit::{CurveValue, KitPoint};
use parking_lot::Mutex;
use std::sync::Arc;
use vixen_session::app::AppCore;
use vixen_session::setup::fixture::{
    add_profiles_in_tree, check_fixture_choices, create_fixture_elements, load_fixture_profiles,
    save_fixture_profile, wizard_summary, ColorSupport, DimmingCurveSelection, FixtureWizard,
    WizardPage, INVALID_PROFILE_TITLE,
};

#[derive(uniffi::Enum, Copy, Clone, PartialEq, Eq)]
pub enum SetupFixturePage {
    SelectProfile,
    EditFunctions,
    EditChannels,
    ColorSupport,
    Automation,
    DimmingCurves,
    Grouping,
    Summary,
}

impl From<WizardPage> for SetupFixturePage {
    fn from(p: WizardPage) -> SetupFixturePage {
        match p {
            WizardPage::SelectProfile => SetupFixturePage::SelectProfile,
            WizardPage::EditFunctions => SetupFixturePage::EditFunctions,
            WizardPage::EditChannels => SetupFixturePage::EditChannels,
            WizardPage::ColorSupport => SetupFixturePage::ColorSupport,
            WizardPage::Automation => SetupFixturePage::Automation,
            WizardPage::DimmingCurves => SetupFixturePage::DimmingCurves,
            WizardPage::Grouping => SetupFixturePage::Grouping,
            WizardPage::Summary => SetupFixturePage::Summary,
        }
    }
}

impl From<SetupFixturePage> for WizardPage {
    fn from(p: SetupFixturePage) -> WizardPage {
        match p {
            SetupFixturePage::SelectProfile => WizardPage::SelectProfile,
            SetupFixturePage::EditFunctions => WizardPage::EditFunctions,
            SetupFixturePage::EditChannels => WizardPage::EditChannels,
            SetupFixturePage::ColorSupport => WizardPage::ColorSupport,
            SetupFixturePage::Automation => WizardPage::Automation,
            SetupFixturePage::DimmingCurves => WizardPage::DimmingCurves,
            SetupFixturePage::Grouping => WizardPage::Grouping,
            SetupFixturePage::Summary => WizardPage::Summary,
        }
    }
}

#[derive(uniffi::Enum, Copy, Clone, PartialEq, Eq)]
pub enum SetupFixtureColorSupport {
    NoColorSupport,
    ColorMixing,
    ColorWheel,
}

impl From<ColorSupport> for SetupFixtureColorSupport {
    fn from(c: ColorSupport) -> SetupFixtureColorSupport {
        match c {
            ColorSupport::None => SetupFixtureColorSupport::NoColorSupport,
            ColorSupport::Mixing => SetupFixtureColorSupport::ColorMixing,
            ColorSupport::Wheel => SetupFixtureColorSupport::ColorWheel,
        }
    }
}

impl From<SetupFixtureColorSupport> for ColorSupport {
    fn from(c: SetupFixtureColorSupport) -> ColorSupport {
        match c {
            SetupFixtureColorSupport::NoColorSupport => ColorSupport::None,
            SetupFixtureColorSupport::ColorMixing => ColorSupport::Mixing,
            SetupFixtureColorSupport::ColorWheel => ColorSupport::Wheel,
        }
    }
}

#[derive(uniffi::Enum, Copy, Clone, PartialEq, Eq)]
pub enum SetupFixtureDimming {
    NoDimmingCurve,
    OneDimmingCurvePerFixture,
    OneDimmingCurvePerColor,
}

impl From<DimmingCurveSelection> for SetupFixtureDimming {
    fn from(d: DimmingCurveSelection) -> SetupFixtureDimming {
        match d {
            DimmingCurveSelection::NoDimmingCurve => SetupFixtureDimming::NoDimmingCurve,
            DimmingCurveSelection::OneDimmingCurvePerFixture => {
                SetupFixtureDimming::OneDimmingCurvePerFixture
            }
            DimmingCurveSelection::OneDimmingCurvePerColor => {
                SetupFixtureDimming::OneDimmingCurvePerColor
            }
        }
    }
}

impl From<SetupFixtureDimming> for DimmingCurveSelection {
    fn from(d: SetupFixtureDimming) -> DimmingCurveSelection {
        match d {
            SetupFixtureDimming::NoDimmingCurve => DimmingCurveSelection::NoDimmingCurve,
            SetupFixtureDimming::OneDimmingCurvePerFixture => {
                DimmingCurveSelection::OneDimmingCurvePerFixture
            }
            SetupFixtureDimming::OneDimmingCurvePerColor => {
                DimmingCurveSelection::OneDimmingCurvePerColor
            }
        }
    }
}

/// A row of the step sidebar.
#[derive(uniffi::Record)]
pub struct SetupFixtureStep {
    pub page: SetupFixturePage,
    pub title: String,
    pub visited: bool,
    /// Quick navigation may go there now. The current page counts, so its row doesn't dim while it has a problem.
    pub can_go: bool,
}

/// A line of the Summary page.
#[derive(uniffi::Record)]
pub struct SetupFixtureSummaryRow {
    pub title: String,
    pub text: String,
}

/// All the wizard shows.
#[derive(uniffi::Record)]
pub struct SetupFixtureWizardState {
    pub page: SetupFixturePage,
    pub title: String,
    pub description: String,
    pub help_url: String,
    pub steps: Vec<SetupFixtureStep>,
    pub can_go_back: bool,
    pub can_go_next: bool,
    pub can_finish: bool,
    /// The validation bar.
    pub problem: Option<String>,

    // Select Profile
    pub profile_names: Vec<String>,
    pub existing_enabled: bool,
    pub select_existing: bool,
    pub selected_fixture: Option<String>,
    pub profile_name: String,
    pub manufacturer: String,
    pub created_by: String,
    pub revision: String,

    // Color Support
    pub color_support: SetupFixtureColorSupport,

    // Automation
    pub auto_shutter: bool,
    pub auto_color_wheel: bool,
    pub auto_dimmer: bool,
    pub auto_prism: bool,
    pub enable_shutter: bool,
    pub enable_color_wheel: bool,
    pub enable_dimmer: bool,
    pub enable_prism: bool,

    // Dimming Curves
    pub dimming: SetupFixtureDimming,
    pub per_color_enabled: bool,

    // Grouping
    pub count: u32,
    pub can_remove_fixture: bool,
    pub prefix: String,
    pub create_group: bool,
    pub group_name: String,
    /// The fixtures the preview shows, under `group_name` when `create_group` is on.
    pub preview_fixtures: Vec<String>,

    // Summary
    pub summary: Vec<SetupFixtureSummaryRow>,
}

fn state_of(w: &FixtureWizard) -> SetupFixtureWizardState {
    let page = w.page();
    let skip_group = usize::from(w.create_group);
    SetupFixtureWizardState {
        page: page.into(),
        title: page.title().to_string(),
        description: page.description().to_string(),
        help_url: page.help_url(),
        steps: WizardPage::ALL
            .iter()
            .map(|p| SetupFixtureStep {
                page: (*p).into(),
                title: p.title().to_string(),
                visited: w.has_visited(*p),
                can_go: *p == page || w.can_go_to(*p),
            })
            .collect(),
        can_go_back: w.can_go_back(),
        can_go_next: w.can_go_next(),
        can_finish: w.can_finish(),
        problem: w.problem().map(str::to_string),
        profile_names: w.profile_names(),
        existing_enabled: w.existing_enabled,
        select_existing: w.select_existing,
        selected_fixture: w.selected_fixture.clone(),
        profile_name: w.profile.name.clone().unwrap_or_default(),
        manufacturer: w.profile.manufacturer.clone().unwrap_or_default(),
        created_by: w.profile.created_by.clone().unwrap_or_default(),
        revision: w.profile.revision.clone().unwrap_or_default(),
        color_support: w.color_support.into(),
        auto_shutter: w.automation.shutter,
        auto_color_wheel: w.automation.color_wheel,
        auto_dimmer: w.automation.dimmer,
        auto_prism: w.automation.prism,
        enable_shutter: w.automation_enabled.shutter,
        enable_color_wheel: w.automation_enabled.color_wheel,
        enable_dimmer: w.automation_enabled.dimmer,
        enable_prism: w.automation_enabled.prism,
        dimming: w.dimming.into(),
        per_color_enabled: w.per_color_enabled,
        count: w.count,
        can_remove_fixture: w.count > 1,
        prefix: w.prefix.clone(),
        create_group: w.create_group,
        group_name: w.group_name.clone(),
        preview_fixtures: w.preview_names().split_off(skip_group),
        // Only that page shows it, and this runs per keystroke elsewhere.
        summary: if page == WizardPage::Summary {
            wizard_summary(&w.choices())
                .into_iter()
                .map(|(title, text)| SetupFixtureSummaryRow { title, text })
                .collect()
        } else {
            Vec::new()
        },
    }
}

/// One run of the wizard.
#[derive(uniffi::Object)]
pub struct SetupFixtureWizard {
    inner: Mutex<FixtureWizard>,
    warnings: Vec<String>,
    core: Arc<AppCore>,
    editor: Mutex<Option<Arc<FixtureEditor>>>,
    overwrite_approved: Mutex<Option<String>>,
}

impl SetupFixtureWizard {
    fn change(&self, f: impl FnOnce(&mut FixtureWizard)) -> SetupFixtureWizardState {
        let mut w = self.inner.lock();
        if matches!(
            w.page(),
            WizardPage::EditFunctions | WizardPage::EditChannels
        ) {
            if let Some(editor) = self.editor.lock().as_ref() {
                let e = editor.inner.lock();
                w.editor_errors = if w.page() == WizardPage::EditFunctions {
                    e.functions_errors()
                } else {
                    e.errors()
                };
                if w.editor_errors.is_empty() {
                    w.profile = e.profile();
                }
            }
        }
        let previous = w.page();
        f(&mut w);
        if previous == WizardPage::EditFunctions && w.page() == WizardPage::EditChannels {
            if let Some(editor) = self.editor.lock().as_ref() {
                let mut e = editor.inner.lock();
                e.reconcile_channels();
                w.profile = e.profile();
                w.editor_errors = e.errors();
            }
        }
        if w.page() == WizardPage::SelectProfile {
            *self.editor.lock() = None;
            w.editor_errors.clear();
        }
        state_of(&w)
    }
}

#[uniffi::export]
impl SetupFixtureWizard {
    pub fn state(&self) -> SetupFixtureWizardState {
        self.change(|_| {})
    }

    pub fn requires_profile_overwrite(&self) -> bool {
        let w = self.inner.lock();
        !w.select_existing
            && self
                .core
                .paths
                .fixtures_dir()
                .join(w.profile.file_name())
                .exists()
            && self.overwrite_approved.lock().as_deref() != Some(w.profile.name_text())
    }
    pub fn approve_profile_overwrite(&self) {
        let name = self.inner.lock().profile.name_text().to_string();
        *self.overwrite_approved.lock() = Some(name);
    }

    pub fn profile_editor(&self) -> Arc<FixtureEditor> {
        let w = self.inner.lock();
        let mut editor = self.editor.lock();
        Arc::clone(editor.get_or_insert_with(|| {
            FixtureEditor::new(Arc::clone(&self.core), w.profile.clone(), Some("Pan"))
        }))
    }

    /// One alert per profile file left out because its name was taken.
    pub fn load_warnings(&self) -> Vec<SetupTemplateProblem> {
        self.warnings
            .iter()
            .map(|m| SetupTemplateProblem {
                title: INVALID_PROFILE_TITLE.to_string(),
                message: m.clone(),
            })
            .collect()
    }

    // ---- Navigation ----------------------------------------------------------------------------------------------------

    pub fn go_next(&self) -> SetupFixtureWizardState {
        self.change(FixtureWizard::go_next)
    }

    pub fn go_back(&self) -> SetupFixtureWizardState {
        self.change(FixtureWizard::go_back)
    }

    pub fn go_to(&self, page: SetupFixturePage) -> SetupFixtureWizardState {
        self.change(|w| w.go_to(page.into()))
    }

    // ---- Select Profile ------------------------------------------------------------------------------------------------

    pub fn choose_existing(&self) -> SetupFixtureWizardState {
        self.change(FixtureWizard::choose_existing)
    }

    pub fn choose_create_new(&self) -> SetupFixtureWizardState {
        self.change(FixtureWizard::choose_create_new)
    }

    pub fn select_profile(&self, name: String) -> SetupFixtureWizardState {
        self.change(|w| w.select_profile(&name))
    }

    pub fn set_profile_name(&self, text: String) -> SetupFixtureWizardState {
        self.change(|w| w.profile.name = Some(text))
    }

    pub fn set_manufacturer(&self, text: String) -> SetupFixtureWizardState {
        self.change(|w| w.profile.manufacturer = Some(text))
    }

    pub fn set_created_by(&self, text: String) -> SetupFixtureWizardState {
        self.change(|w| w.profile.created_by = Some(text))
    }

    pub fn set_revision(&self, text: String) -> SetupFixtureWizardState {
        self.change(|w| w.profile.revision = Some(text))
    }

    // ---- Color Support, Automation, Dimming Curves -------------------------------------------------------------------

    pub fn set_color_support(&self, support: SetupFixtureColorSupport) -> SetupFixtureWizardState {
        self.change(|w| w.color_support = support.into())
    }

    pub fn set_auto_shutter(&self, on: bool) -> SetupFixtureWizardState {
        self.change(|w| w.set_auto_shutter(on))
    }

    pub fn set_auto_color_wheel(&self, on: bool) -> SetupFixtureWizardState {
        self.change(|w| w.set_auto_color_wheel(on))
    }

    pub fn set_auto_dimmer(&self, on: bool) -> SetupFixtureWizardState {
        self.change(|w| w.set_auto_dimmer(on))
    }

    pub fn set_auto_prism(&self, on: bool) -> SetupFixtureWizardState {
        self.change(|w| w.set_auto_prism(on))
    }

    pub fn set_dimming(&self, selection: SetupFixtureDimming) -> SetupFixtureWizardState {
        self.change(|w| w.set_dimming(selection.into()))
    }

    pub fn vixen_core(&self) -> Arc<VixenCore> {
        Arc::new(VixenCore { inner: Arc::clone(&self.core) })
    }

    pub fn dimming_curve(&self) -> CurveValue {
        let wizard = self.inner.lock();
        CurveValue {
            points: wizard.curve.points.iter().map(|p| KitPoint { x: p.x, y: p.y }).collect(),
            library_name: wizard.curve.library_reference_name.clone().unwrap_or_default(),
            is_current_library: wizard.curve.is_current_library_curve,
        }
    }

    pub fn set_dimming_curve(&self, curve: CurveValue) -> SetupFixtureWizardState {
        self.change(|w| {
            let draft = vixen_session::dialogs::filters::CurveDraft {
                points: curve.points.iter().map(|p| (p.x, p.y)).collect(),
                library_reference_name: curve.library_name,
                is_current_library_curve: curve.is_current_library,
            };
            w.curve = vixen_session::dialogs::filters::curve_data_from_draft(&draft, Some(&w.curve));
        })
    }

    // ---- Grouping ------------------------------------------------------------------------------------------------------

    pub fn add_fixture(&self) -> SetupFixtureWizardState {
        self.change(FixtureWizard::add_fixture)
    }

    pub fn remove_fixture(&self) -> SetupFixtureWizardState {
        self.change(FixtureWizard::remove_fixture)
    }

    pub fn set_prefix(&self, text: String) -> SetupFixtureWizardState {
        self.change(|w| w.prefix = text)
    }

    pub fn set_create_group(&self, on: bool) -> SetupFixtureWizardState {
        self.change(|w| w.create_group = on)
    }

    pub fn set_group_name(&self, text: String) -> SetupFixtureWizardState {
        self.change(|w| w.group_name = text)
    }
}

#[uniffi::export]
impl DisplaySetupSession {
    /// F139: a new wizard on Select Profile. Reads the profile folder, so call it off the main thread.
    pub fn open_fixture_wizard(&self) -> Arc<SetupFixtureWizard> {
        let mut profiles = load_fixture_profiles(&self.core.paths.fixtures_dir());
        add_profiles_in_tree(&mut profiles, &self.inner.read());
        Arc::new(SetupFixtureWizard {
            inner: Mutex::new(FixtureWizard::new(profiles.profiles)),
            warnings: profiles.warnings,
            core: Arc::clone(&self.core),
            editor: Mutex::new(None),
            overwrite_approved: Mutex::new(None),
        })
    }

    /// F110, for Finish: saves the profile, which Display Setup's Cancel doesn't undo, then makes the elements. Off the main thread.
    pub fn create_fixture_elements(
        &self,
        wizard: Arc<SetupFixtureWizard>,
    ) -> Result<SetupEditResult, FfiError> {
        if wizard.requires_profile_overwrite() {
            return Err(FfiError::new(
                "Confirm replacing the existing fixture profile before finishing.",
            ));
        }
        let choices = wizard.inner.lock().choices();
        let errors =
            vixen_session::setup::fixture_editor::ProfileEditor::new(choices.profile.clone(), None)
                .errors();
        if !errors.is_empty() {
            return Err(FfiError::new(errors.join("\n")));
        }
        if choices.count == 0 {
            return Err(FfiError::new("At least one fixture is required."));
        }
        check_fixture_choices(&choices).map_err(FfiError::new)?;
        save_fixture_profile(&self.core.paths.fixtures_dir(), &choices.profile)
            .map_err(FfiError::new)?;
        let mut s = self.inner.write();
        let result = create_fixture_elements(&mut s, &choices).map_err(FfiError::new)?;
        Ok(edit_result(&s, result.edit))
    }
}
