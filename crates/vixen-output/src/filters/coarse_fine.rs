//! Port of Vixen.Modules/OutputFilter/CoarseFineBreakdown: a 16-bit value in, high and low bytes out.

use crate::command::OutputCommand;
use crate::dotnet::ushort_from_f64;
use crate::flow::{StateRef, States};
use crate::ids;
use crate::xmlutil::{self, bool_el, data_root, num_el, ModuleIds};
use vixen_xml::{contract_ns, XElement};

const CLR_NS: &str = "VixenModules.OutputFilter.CoarseFineBreakdown";

/// `CoarseFineBreakdownData`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoarseFineConfig {
    pub ids: ModuleIds,
    pub enable_default_value_mapping: bool,
    pub resting_coarse_value: u8,
    pub resting_fine_value: u8,
}

impl Default for CoarseFineConfig {
    fn default() -> Self {
        CoarseFineConfig {
            ids: ModuleIds::new(ids::COARSE_FINE_BREAKDOWN, Default::default()),
            enable_default_value_mapping: false,
            resting_coarse_value: 0,
            resting_fine_value: 0,
        }
    }
}

impl CoarseFineConfig {
    pub fn from_xml(e: &XElement) -> CoarseFineConfig {
        CoarseFineConfig {
            ids: ModuleIds::read(e),
            enable_default_value_mapping: xmlutil::get_bool(e, "EnableDefaultValueMapping"),
            resting_coarse_value: xmlutil::get_u8(e, "RestingCoarseValue"),
            resting_fine_value: xmlutil::get_u8(e, "RestingFineValue"),
        }
    }

    pub fn to_xml(&self) -> XElement {
        let ns = contract_ns(CLR_NS);
        let mut e = data_root(&ns, "CoarseFineBreakdownData", &self.ids);
        e.push(bool_el(&ns, "EnableDefaultValueMapping", self.enable_default_value_mapping));
        e.push(num_el(&ns, "RestingCoarseValue", self.resting_coarse_value));
        e.push(num_el(&ns, "RestingFineValue", self.resting_fine_value));
        e
    }
}

/// `CoarseFineBreakdownModule` with its two outputs. Output 0 is the high byte, output 1 the low byte.
#[derive(Clone, Debug)]
pub(crate) struct CoarseFineRt {
    enable_default: bool,
    resting: u16,
    high: Vec<OutputCommand>,
    low: Vec<OutputCommand>,
}

impl CoarseFineRt {
    pub(crate) fn new(cfg: &CoarseFineConfig) -> CoarseFineRt {
        let mut rt = CoarseFineRt {
            enable_default: cfg.enable_default_value_mapping,
            resting: ((cfg.resting_coarse_value as u16) << 8) | cfg.resting_fine_value as u16,
            high: Vec::with_capacity(2),
            low: Vec::with_capacity(2),
        };
        rt.handle_default();
        rt
    }

    #[inline]
    fn handle(&mut self, canonical: u16) {
        self.high.push(OutputCommand::U8((canonical >> 8) as u8));
        self.low.push(OutputCommand::U8((canonical & 0xFF) as u8));
    }

    #[inline]
    fn handle_default(&mut self) {
        if self.enable_default {
            self.handle(self.resting);
        }
    }

    /// `ProcessInputData(IntentsDataFlowData)`. `None` is a null list. Any range intent counts here: the
    /// tag is the upstream filter's job.
    pub(crate) fn update_intents(&mut self, states: Option<States<'_>>) {
        self.high.clear();
        self.low.clear();
        let mut has_output = false;
        if let Some(states) = states {
            states.for_each(|s| {
                if let StateRef::Range(r) = s {
                    self.handle(ushort_from_f64(r.value * 65535.0));
                    has_output = true;
                }
            });
        }
        if !has_output {
            self.handle_default();
        }
    }

    /// `ProcessInputData(CommandDataFlowData)`: only a 16-bit command is split.
    pub(crate) fn update_command(&mut self, command: &OutputCommand) {
        self.high.clear();
        self.low.clear();
        if let OutputCommand::U16(v) = command {
            self.handle(*v);
            return;
        }
        self.handle_default();
    }

    #[inline]
    pub(crate) fn output(&self, index: usize) -> Option<&[OutputCommand]> {
        match index {
            0 => Some(&self.high),
            1 => Some(&self.low),
            _ => None,
        }
    }
}

/// The expected values come from upstream `CoarseFineBreakdownModuleTests`.
#[cfg(test)]
mod tests {
    use super::*;
    use vixen_core::{CombinedState, FunctionIdentity, RangeValue, StateValue};

    const RESTING: (u8, u8) = (0xA5, 0x3C);

    fn data(enable: bool, coarse: u8, fine: u8) -> CoarseFineConfig {
        CoarseFineConfig {
            enable_default_value_mapping: enable,
            resting_coarse_value: coarse,
            resting_fine_value: fine,
            ..Default::default()
        }
    }

    /// `Assert.Single` on both outputs.
    fn output_bytes(rt: &CoarseFineRt) -> (u8, u8) {
        assert_eq!(rt.high.len(), 1);
        assert_eq!(rt.low.len(), 1);
        (rt.high[0].byte(), rt.low[0].byte())
    }

    fn process_command(cfg: &CoarseFineConfig, value: u16) -> (u8, u8) {
        let mut rt = CoarseFineRt::new(cfg);
        rt.update_command(&OutputCommand::U16(value));
        output_bytes(&rt)
    }

    fn process_missing_command(cfg: &CoarseFineConfig) -> (u8, u8) {
        let mut rt = CoarseFineRt::new(cfg);
        rt.update_command(&OutputCommand::None);
        output_bytes(&rt)
    }

    fn process_range(cfg: &CoarseFineConfig, value: f64) -> (u8, u8) {
        let mut rt = CoarseFineRt::new(cfg);
        let list = [StateValue::Range(RangeValue::new(FunctionIdentity::Pan, "Pan", value, ""))];
        rt.update_intents(Some(States::List(&list)));
        output_bytes(&rt)
    }

    #[test]
    fn disabled_mapping_preserves_legacy_command_splitting() {
        let cfg = data(false, RESTING.0, RESTING.1);
        assert_eq!(process_command(&cfg, 0x1234).0, 0x12);
        assert_eq!(process_command(&cfg, 0x1234).1, 0x34);
        assert_eq!(process_command(&cfg, 0).0, 0);
        assert_eq!(process_command(&cfg, 0).1, 0);
    }

    #[test]
    fn disabled_mapping_leaves_missing_values_without_an_output() {
        let mut rt = CoarseFineRt::new(&data(false, RESTING.0, RESTING.1));
        rt.update_command(&OutputCommand::None);
        assert!(rt.high.is_empty());
        assert!(rt.low.is_empty());
    }

    #[test]
    fn enabled_mapping_uses_resting_bytes_when_command_value_is_missing() {
        let cfg = data(true, RESTING.0, RESTING.1);
        assert_eq!(process_missing_command(&cfg), RESTING);
        assert_eq!(process_command(&cfg, 0), (0x00, 0x00));
        assert_eq!(process_command(&cfg, 0x1234), (0x12, 0x34));
    }

    #[test]
    fn enabled_mapping_initializes_outputs_with_resting_bytes() {
        let rt = CoarseFineRt::new(&data(true, RESTING.0, RESTING.1));
        assert_eq!(output_bytes(&rt), RESTING);
    }

    #[test]
    fn enabled_mapping_uses_resting_bytes_when_no_intent_produces_an_output() {
        let cfg = data(true, RESTING.0, RESTING.1);
        let mut rt = CoarseFineRt::new(&cfg);
        rt.update_intents(None);
        assert_eq!(output_bytes(&rt), RESTING);
        assert_eq!(process_range(&cfg, 0.5), (0x7F, 0xFF));
    }

    #[test]
    fn range_value_uses_existing_truncation_when_an_input_is_present() {
        assert_eq!(process_range(&CoarseFineConfig::default(), 0.5), (0x7F, 0xFF));
    }

    #[test]
    fn enabled_mapping_handles_input_and_resting_boundaries() {
        for (input, coarse, fine) in [(0u16, 0u8, 0u8), (u16::MAX, u8::MAX, u8::MAX)] {
            let cfg = data(true, coarse, fine);
            assert_eq!(process_missing_command(&cfg), (coarse, fine));
            assert_eq!(process_command(&cfg, input), ((input >> 8) as u8, (input & 0xFF) as u8));
        }
    }

    #[test]
    fn programmatic_setters_rebuild_outputs_with_updated_configuration() {
        // The setters rebuild the outputs, which here means building a new runtime from the config.
        let mut cfg = CoarseFineConfig::default();
        cfg.resting_coarse_value = 0xA5;
        cfg.resting_fine_value = 0x3C;
        cfg.enable_default_value_mapping = true;
        assert_eq!(output_bytes(&CoarseFineRt::new(&cfg)), RESTING);
        cfg.resting_fine_value = 0x5E;
        assert_eq!(output_bytes(&CoarseFineRt::new(&cfg)), (0xA5, 0x5E));
    }

    #[test]
    fn clone_preserves_independent_default_mapping_configuration() {
        let mut original = data(true, 0xA5, 0x3C);
        let clone = original.clone();
        original.enable_default_value_mapping = false;
        original.resting_coarse_value = 0;
        original.resting_fine_value = 0;
        assert_ne!(original, clone);
        assert!(clone.enable_default_value_mapping);
        assert_eq!(clone.resting_coarse_value, 0xA5);
        assert_eq!(clone.resting_fine_value, 0x3C);
    }

    #[test]
    fn only_a_16_bit_command_is_split() {
        let mut rt = CoarseFineRt::new(&data(false, 0, 0));
        rt.update_command(&OutputCommand::U8(200));
        assert!(rt.high.is_empty());
    }

    #[test]
    fn every_range_state_adds_a_value_and_other_states_add_none() {
        let mut rt = CoarseFineRt::new(&data(false, 0, 0));
        let mut s = CombinedState::default();
        s.other.push(StateValue::Range(RangeValue::new(FunctionIdentity::Pan, "Pan", 1.0, "")));
        s.other.push(StateValue::Range(RangeValue::new(FunctionIdentity::Tilt, "Tilt", 0.0, "")));
        s.rgb = Some(vixen_core::RgbValue { r: 255, g: 255, b: 255 });
        rt.update_intents(Some(States::Element(&s)));
        assert_eq!(rt.high, vec![OutputCommand::U8(255), OutputCommand::U8(0)]);
        assert_eq!(rt.low, vec![OutputCommand::U8(255), OutputCommand::U8(0)]);
    }
}
