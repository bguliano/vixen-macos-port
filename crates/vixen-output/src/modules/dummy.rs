//! Port of Vixen.Modules/Controller/DummyLighting. The Windows form becomes a snapshot the Swift side can draw.

use super::{ConfigMismatch, ControllerConfig, ModuleShared, ModuleStatus, OutputModule};
use crate::command::OutputCommand;
use crate::ids;
use crate::policy::DataPolicy;
use crate::xmlutil::{self, data_root, opt_text_el, text_el, ModuleIds};
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;
use vixen_core::Color;
use vixen_xml::{contract_ns, XElement};

const CLR_NS: &str = "VixenModules.Output.DummyLighting";

/// `RenderStyle`.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum RenderStyle {
    /// Each output is a single monochrome output.
    #[default]
    Monochrome,
    /// Every 3 outputs is a single RGB output.
    RGBMultiChannel,
    /// Every output is a single RGB output.
    RGBSingleChannel,
}

impl RenderStyle {
    pub fn name(self) -> &'static str {
        match self {
            RenderStyle::Monochrome => "Monochrome",
            RenderStyle::RGBMultiChannel => "RGBMultiChannel",
            RenderStyle::RGBSingleChannel => "RGBSingleChannel",
        }
    }

    pub fn from_name(name: &str) -> RenderStyle {
        match name {
            "RGBMultiChannel" => RenderStyle::RGBMultiChannel,
            "RGBSingleChannel" => RenderStyle::RGBSingleChannel,
            _ => RenderStyle::Monochrome,
        }
    }

    /// `_SetDataPolicy`.
    pub fn data_policy(self) -> DataPolicy {
        match self {
            RenderStyle::Monochrome => DataPolicy::HIGHEST_WINS_8BIT,
            RenderStyle::RGBSingleChannel => DataPolicy::ONE_CHANNEL_COLOR,
            RenderStyle::RGBMultiChannel => DataPolicy::AVERAGE_8BIT,
        }
    }
}

/// `DummyLightingData`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DummyLightingConfig {
    pub ids: ModuleIds,
    pub form_title: Option<String>,
    pub render_style: RenderStyle,
}

impl Default for DummyLightingConfig {
    fn default() -> Self {
        DummyLightingConfig {
            ids: ModuleIds::new(ids::DUMMY_LIGHTING, Uuid::nil()),
            form_title: Some("Dummy Lighting Controller".to_string()),
            render_style: RenderStyle::Monochrome,
        }
    }
}

impl DummyLightingConfig {
    pub fn from_xml(e: &XElement) -> DummyLightingConfig {
        DummyLightingConfig {
            ids: ModuleIds::read(e),
            form_title: xmlutil::get_opt_string(e, "FormTitle"),
            render_style: RenderStyle::from_name(&e.child_text("RenderStyle").unwrap_or_default()),
        }
    }

    pub fn to_xml(&self) -> XElement {
        let ns = contract_ns(CLR_NS);
        let mut e = data_root(&ns, "DummyLightingData", &self.ids);
        e.push(opt_text_el(&ns, "FormTitle", self.form_title.as_deref()));
        e.push(text_el(&ns, "RenderStyle", self.render_style.name()));
        e
    }
}

/// What the output form shows. `values` is empty in `RGBSingleChannel`, and `colors` is empty otherwise.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DummyLightingView {
    pub fps: f64,
    pub frames: u64,
    pub values: Vec<u8>,
    pub colors: Vec<Color>,
    /// `OutputCount`, which sets how the boxes lay out.
    pub outputs: usize,
    pub style: RenderStyle,
}

/// `DummyLighting`.
pub struct DummyLightingModule {
    config: DummyLightingConfig,
    shared: Arc<ModuleShared>,
    output_count: usize,
    view: Arc<Mutex<DummyLightingView>>,
    update_count: u64,
    started: Option<Instant>,
}

impl DummyLightingModule {
    pub fn new(config: DummyLightingConfig) -> DummyLightingModule {
        DummyLightingModule {
            config,
            shared: ModuleShared::new(),
            output_count: 0,
            view: Arc::new(Mutex::new(DummyLightingView::default())),
            update_count: 0,
            started: None,
        }
    }

    /// The form's state, for whatever draws it.
    pub fn view(&self) -> Arc<Mutex<DummyLightingView>> {
        self.view.clone()
    }
}

impl OutputModule for DummyLightingModule {
    fn type_id(&self) -> Uuid {
        ids::DUMMY_LIGHTING
    }

    fn start(&mut self) {
        self.update_count = 0;
        self.started = None;
        *self.view.lock() = DummyLightingView { outputs: self.output_count, style: self.config.render_style, ..Default::default() };
        self.shared.set_status(ModuleStatus::Running);
    }

    fn stop(&mut self) {
        self.shared.set_status(ModuleStatus::Stopped);
    }

    fn update(&mut self, commands: &[OutputCommand]) {
        if self.update_count == 0 {
            self.started = Some(Instant::now());
        }
        self.update_count += 1;
        let elapsed_ms = self.started.map_or(0.0, |s| s.elapsed().as_secs_f64() * 1000.0);
        let mut view = self.view.lock();
        view.fps = 1000.0 * (self.update_count as f64 / elapsed_ms);
        view.frames = self.update_count;
        view.outputs = self.output_count;
        view.style = self.config.render_style;
        match self.config.render_style {
            RenderStyle::Monochrome | RenderStyle::RGBMultiChannel => {
                view.colors.clear();
                view.values.resize(commands.len(), 0);
                for (slot, c) in view.values.iter_mut().zip(commands) {
                    *slot = c.byte();
                }
            }
            RenderStyle::RGBSingleChannel => {
                view.values.clear();
                view.colors.resize(commands.len(), Color::TRANSPARENT);
                for (slot, c) in view.colors.iter_mut().zip(commands) {
                    // `CommandHandler.Reset` sets `Color.Transparent`, and only a colour command changes it.
                    *slot = if let OutputCommand::Color(color) = c { *color } else { Color::TRANSPARENT };
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
        ControllerConfig::DummyLighting(self.config.clone())
    }

    fn set_config(&mut self, config: ControllerConfig) -> Result<(), ConfigMismatch> {
        let ControllerConfig::DummyLighting(config) = config else {
            return Err(ConfigMismatch);
        };
        self.config = config;
        Ok(())
    }

    fn data_policy(&self) -> DataPolicy {
        self.config.render_style.data_policy()
    }

    fn shared(&self) -> &Arc<ModuleShared> {
        &self.shared
    }

    fn dummy_view(&self) -> Option<Arc<Mutex<DummyLightingView>>> {
        Some(self.view.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monochrome_view_holds_the_bytes() {
        let mut m = DummyLightingModule::new(DummyLightingConfig::default());
        let view = m.view();
        m.start();
        assert_eq!(m.status(), ModuleStatus::Running);
        m.update(&[OutputCommand::U8(10), OutputCommand::None, OutputCommand::U16(3)]);
        let v = view.lock();
        assert_eq!(v.values, vec![10, 0, 0]);
        assert!(v.colors.is_empty());
        assert_eq!(v.frames, 1);
    }

    #[test]
    fn single_channel_view_holds_colours() {
        let mut m =
            DummyLightingModule::new(DummyLightingConfig { render_style: RenderStyle::RGBSingleChannel, ..Default::default() });
        let view = m.view();
        m.start();
        m.update(&[OutputCommand::Color(Color::RED), OutputCommand::U8(1)]);
        let v = view.lock();
        assert_eq!(v.colors, vec![Color::RED, Color::TRANSPARENT]);
        assert!(v.values.is_empty());
    }

    #[test]
    fn a_new_config_has_the_default_title() {
        assert_eq!(DummyLightingConfig::default().form_title.as_deref(), Some("Dummy Lighting Controller"));
    }
}
