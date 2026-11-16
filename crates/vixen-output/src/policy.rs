//! Port of Vixen.Core/Data/Evaluator, Combinator and Policy: how a list of intent states or commands
//! becomes the one command a controller output holds.

use crate::command::{CommandRef, OutputCommand};
use crate::dotnet::{byte_from_f64, ushort_from_f64};
use crate::flow::{Flow, StateRef};
use vixen_core::{Color, RgbValue};

/// `IEvaluator`: turns one intent state into a command.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Evaluator {
    /// `_8BitEvaluator`.
    Bits8,
    /// `_16BitEvaluator`.
    Bits16,
    /// `_32BitEvaluator`.
    Bits32,
    /// `_64BitEvaluator`.
    Bits64,
    /// `ColorEvaluator`.
    Color,
}

/// `ICombinator`: reduces commands to one.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Combinator {
    /// `_8BitHighestWinsCombinator`.
    HighestWins8,
    /// `_8BitAverageCombinator`.
    Average8,
    /// `_16BitHighestWinsCombinator`.
    HighestWins16,
    /// `_16BitAverageCombinator`.
    Average16,
    /// `_32BitHighestWinsCombinator`.
    HighestWins32,
    /// `_64BitHighestWinsCombinator`.
    HighestWins64,
    /// `NaiveColorCombinator`.
    NaiveColor,
    /// `DiscardExcessCombinator`: keeps the last command.
    DiscardExcess,
}

/// `ControllerDataPolicy`: the evaluator and combinator a controller module chooses.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct DataPolicy {
    pub evaluator: Evaluator,
    pub combinator: Combinator,
}

impl DataPolicy {
    /// GenericSerial, Renard, DMX USB Pro, OpenDMX, E1.31, DDP, Elexol and a monochrome DummyLighting.
    pub const HIGHEST_WINS_8BIT: DataPolicy = DataPolicy { evaluator: Evaluator::Bits8, combinator: Combinator::HighestWins8 };
    /// RDS, Launcher and Debug.
    pub const DISCARD_EXCESS_8BIT: DataPolicy = DataPolicy { evaluator: Evaluator::Bits8, combinator: Combinator::DiscardExcess };
    /// DummyLighting in `RGBMultiChannel`.
    pub const AVERAGE_8BIT: DataPolicy = DataPolicy { evaluator: Evaluator::Bits8, combinator: Combinator::Average8 };
    /// DummyLighting in `RGBSingleChannel`.
    pub const ONE_CHANNEL_COLOR: DataPolicy = DataPolicy { evaluator: Evaluator::Color, combinator: Combinator::NaiveColor };
}

/// A data policy with the state the C# object keeps between calls.
#[derive(Clone, Debug)]
pub(crate) struct PolicyState {
    policy: DataPolicy,
    /// `Evaluator.EvaluatorValue`. An evaluator's value survives calls that don't set it, and only
    /// `ColorEvaluator` has states it doesn't handle (ranges), so only that evaluator ever reads it back.
    evaluator_value: OutputCommand,
}

impl PolicyState {
    pub(crate) fn new(policy: DataPolicy) -> PolicyState {
        PolicyState { policy, evaluator_value: OutputCommand::None }
    }

    pub(crate) fn policy(&self) -> DataPolicy {
        self.policy
    }

    /// `DataFlowDataDispatchingDataPolicy.GenerateCommand`, guarded by the `o?.Value != null` check in
    /// `OutputController.Update`.
    #[inline]
    pub(crate) fn generate(&mut self, flow: Flow<'_>, out: &mut OutputCommand) {
        match flow {
            Flow::Null | Flow::NullIntents => *out = OutputCommand::None,
            // A single command passes straight through, whatever the policy.
            Flow::Command(c) => out.clone_from(c),
            Flow::Commands(list) => {
                let mut acc = OutputCommand::None;
                for c in list {
                    if let Some(c) = CommandRef::from_output(c) {
                        feed(self.policy.combinator, &mut acc, c);
                    }
                }
                *out = acc;
            }
            Flow::Intents(states) => {
                let mut acc = OutputCommand::None;
                states.for_each(|s| self.feed_state(&mut acc, s));
                *out = acc;
            }
        }
    }

    /// `Evaluator.Evaluate` for one state, then its result through the combinator.
    #[inline]
    fn feed_state(&mut self, acc: &mut OutputCommand, s: StateRef<'_>) {
        let combinator = self.policy.combinator;
        match self.policy.evaluator {
            Evaluator::Bits8 => {
                let c = match s {
                    StateRef::Lighting(l) => CommandRef::U8(byte_from_f64(255.0 * l.intensity)),
                    StateRef::Rgb(r) => CommandRef::U8(byte_from_f64(255.0 * r.intensity())),
                    StateRef::Discrete(d) => CommandRef::U8(byte_from_f64(255.0 * d.intensity)),
                    StateRef::Range(r) => CommandRef::U8(byte_from_f64(255.0 * r.value)),
                    StateRef::Command(c) => CommandRef::from_value(c),
                };
                feed(combinator, acc, c);
            }
            Evaluator::Bits16 => {
                let c = match s {
                    StateRef::Lighting(l) => CommandRef::U16(ushort_from_f64(65535.0 * l.intensity)),
                    StateRef::Rgb(r) => CommandRef::U16(ushort_from_f64(65535.0 * r.intensity())),
                    StateRef::Discrete(d) => CommandRef::U16(ushort_from_f64(65535.0 * d.intensity)),
                    StateRef::Range(r) => CommandRef::U16(ushort_from_f64(65535.0 * r.value)),
                    StateRef::Command(c) => CommandRef::from_value(c),
                };
                feed(combinator, acc, c);
            }
            Evaluator::Bits32 => {
                let m = u32::MAX as f64;
                let c = match s {
                    StateRef::Lighting(l) => CommandRef::U32((m * l.intensity) as u32),
                    StateRef::Rgb(r) => CommandRef::U32((m * r.intensity()) as u32),
                    StateRef::Discrete(d) => CommandRef::U32((m * d.intensity) as u32),
                    StateRef::Range(r) => CommandRef::U32((m * r.value) as u32),
                    StateRef::Command(c) => CommandRef::from_value(c),
                };
                feed(combinator, acc, c);
            }
            Evaluator::Bits64 => {
                let m = u64::MAX as f64;
                let c = match s {
                    StateRef::Lighting(l) => CommandRef::U64((m * l.intensity) as u64),
                    StateRef::Rgb(r) => CommandRef::U64((m * r.intensity()) as u64),
                    StateRef::Discrete(d) => CommandRef::U64((m * d.intensity) as u64),
                    StateRef::Range(r) => CommandRef::U64((m * r.value) as u64),
                    StateRef::Command(c) => CommandRef::from_value(c),
                };
                feed(combinator, acc, c);
            }
            Evaluator::Color => {
                match s {
                    StateRef::Rgb(r) => self.evaluator_value = OutputCommand::Color(r.full_color()),
                    StateRef::Lighting(l) => self.evaluator_value = OutputCommand::Color(l.full_color()),
                    StateRef::Discrete(d) => self.evaluator_value = OutputCommand::Color(d.full_color()),
                    StateRef::Command(c) => self.evaluator_value = c.into(),
                    // Not handled, so the previous state's command is returned again. In the C# a null
                    // one here throws in the combinator and that frame is lost; it is skipped instead.
                    StateRef::Range(_) => {}
                }
                if let Some(c) = CommandRef::from_output(&self.evaluator_value) {
                    feed(combinator, acc, c);
                }
            }
        }
    }
}

/// The combinators' `Handle` overloads. `acc` is `CombinatorValue`, null at the start of `Combine`.
///
/// The conversions look odd because they are: the first value goes through a command constructor, and C#
/// overload resolution sends a `ulong` to the `float` overload, while later values are cast straight to
/// `byte`, even in the 32-bit and 64-bit combinators.
#[inline]
pub(crate) fn feed(combinator: Combinator, acc: &mut OutputCommand, c: CommandRef<'_>) {
    let first = acc.is_none();
    match combinator {
        Combinator::HighestWins8 => {
            let v = match c {
                CommandRef::U8(v) => v,
                CommandRef::Color(color) => RgbValue::grayscale_level(color),
                CommandRef::U16(v) => v as u8,
                CommandRef::U32(v) => v as u8,
                CommandRef::U64(v) => {
                    if first {
                        (v as f32) as i32 as u8
                    } else {
                        v as u8
                    }
                }
                CommandRef::Str(_) => return,
            };
            *acc = OutputCommand::U8(if first { v } else { acc.byte().max(v) });
        }
        Combinator::Average8 => {
            if let CommandRef::U8(v) = c {
                *acc = OutputCommand::U8(if first { v } else { ((acc.byte() as i32 + v as i32) >> 1) as u8 });
            }
        }
        Combinator::HighestWins16 => {
            let prev = if let OutputCommand::U16(p) = acc { *p } else { 0 };
            let v = match c {
                CommandRef::U8(v) => v as u16,
                CommandRef::Color(color) => RgbValue::grayscale_level(color) as u16,
                CommandRef::U16(v) => v,
                CommandRef::U32(v) => {
                    if first {
                        (v as f32) as i32 as u16
                    } else {
                        v as u16
                    }
                }
                CommandRef::U64(v) => {
                    if first {
                        (v as f32) as i32 as u16
                    } else {
                        v as u16
                    }
                }
                CommandRef::Str(_) => return,
            };
            *acc = OutputCommand::U16(if first { v } else { prev.max(v) });
        }
        Combinator::Average16 => {
            if let CommandRef::U16(v) = c {
                let prev = if let OutputCommand::U16(p) = acc { *p } else { 0 };
                *acc = OutputCommand::U16(if first { v } else { ((prev as i32 + v as i32) >> 1) as u16 });
            }
        }
        Combinator::HighestWins32 => {
            let prev = if let OutputCommand::U32(p) = acc { *p } else { 0 };
            let v = match c {
                CommandRef::U8(v) => v as u32,
                CommandRef::Color(color) => RgbValue::grayscale_level(color) as u32,
                CommandRef::U32(v) => {
                    if first {
                        v
                    } else {
                        (v as u8) as u32
                    }
                }
                CommandRef::U64(v) => {
                    if first {
                        (v as f32 as f64) as u32
                    } else {
                        (v as u8) as u32
                    }
                }
                CommandRef::U16(v) => {
                    if first {
                        v as u32
                    } else {
                        (v as u8) as u32
                    }
                }
                CommandRef::Str(_) => return,
            };
            *acc = OutputCommand::U32(if first { v } else { prev.max(v) });
        }
        Combinator::HighestWins64 => {
            let prev = if let OutputCommand::U64(p) = acc { *p } else { 0 };
            let v = match c {
                CommandRef::U8(v) => v as u64,
                CommandRef::Color(color) => RgbValue::grayscale_level(color) as u64,
                CommandRef::U32(v) => {
                    if first {
                        v as u64
                    } else {
                        (v as u8) as u64
                    }
                }
                CommandRef::U64(v) => {
                    if first {
                        v
                    } else {
                        (v as u8) as u64
                    }
                }
                CommandRef::U16(v) => {
                    if first {
                        v as u64
                    } else {
                        (v as u8) as u64
                    }
                }
                CommandRef::Str(_) => return,
            };
            *acc = OutputCommand::U64(if first { v } else { prev.max(v) });
        }
        Combinator::NaiveColor => {
            if let CommandRef::Color(v2) = c {
                *acc = OutputCommand::Color(match acc {
                    OutputCommand::Color(v1) => merge_color_naively(*v1, v2),
                    _ => v2,
                });
            }
        }
        Combinator::DiscardExcess => *acc = c.to_owned(),
    }
}

fn merge_color_naively(v1: Color, v2: Color) -> Color {
    let a = v1.a.max(v2.a) as i32;
    let r = (v1.r as i32 + v2.r as i32) >> 1;
    let g = (v1.g as i32 + v2.g as i32) >> 1;
    let b = (v1.b as i32 + v2.b as i32) >> 1;
    Color::from_argb_i32(a, r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow::States;
    use std::sync::Arc;
    use vixen_core::{CommandValue, DiscreteValue, FunctionIdentity, LightingValue, RangeValue, StateValue};

    fn run(policy: DataPolicy, states: &[StateValue]) -> OutputCommand {
        let mut p = PolicyState::new(policy);
        let mut out = OutputCommand::None;
        p.generate(Flow::Intents(States::List(states)), &mut out);
        out
    }

    fn combine(combinator: Combinator, commands: &[OutputCommand]) -> OutputCommand {
        let mut acc = OutputCommand::None;
        for c in commands {
            feed(combinator, &mut acc, CommandRef::from_output(c).unwrap());
        }
        acc
    }

    #[test]
    fn evaluator_8bit_truncates() {
        let p = DataPolicy::HIGHEST_WINS_8BIT;
        let lit = |i| StateValue::Lighting(LightingValue::new(Color::RED, i));
        assert_eq!(run(p, &[lit(0.5)]), OutputCommand::U8(127));
        assert_eq!(run(p, &[lit(1.0)]), OutputCommand::U8(255));
        assert_eq!(run(p, &[StateValue::Rgb(RgbValue { r: 10, g: 200, b: 30 })]), OutputCommand::U8(200));
        assert_eq!(run(p, &[StateValue::Discrete(DiscreteValue::new(Color::BLUE, 0.25))]), OutputCommand::U8(63));
        assert_eq!(run(p, &[StateValue::Range(RangeValue::new(FunctionIdentity::Pan, "Pan", 0.2, ""))]), OutputCommand::U8(51));
    }

    #[test]
    fn empty_state_list_is_a_null_command() {
        assert_eq!(run(DataPolicy::HIGHEST_WINS_8BIT, &[]), OutputCommand::None);
        assert_eq!(run(DataPolicy::DISCARD_EXCESS_8BIT, &[]), OutputCommand::None);
    }

    #[test]
    fn highest_wins_takes_the_max() {
        let p = DataPolicy::HIGHEST_WINS_8BIT;
        let s = [
            StateValue::Discrete(DiscreteValue::new(Color::RED, 0.2)),
            StateValue::Discrete(DiscreteValue::new(Color::BLUE, 0.8)),
            StateValue::Discrete(DiscreteValue::new(Color::LIME, 0.4)),
        ];
        assert_eq!(run(p, &s), OutputCommand::U8(204));
    }

    #[test]
    fn discard_excess_keeps_the_last_command_of_any_type() {
        let text: Arc<str> = Arc::from("RDS|Hello");
        let s = [StateValue::Lighting(LightingValue::new(Color::RED, 1.0)), StateValue::Command(CommandValue::Str(text.clone()))];
        assert_eq!(run(DataPolicy::DISCARD_EXCESS_8BIT, &s), OutputCommand::Str(text));
    }

    #[test]
    fn highest_wins_ignores_strings() {
        let s = [StateValue::Command(CommandValue::Str(Arc::from("x")))];
        assert_eq!(run(DataPolicy::HIGHEST_WINS_8BIT, &s), OutputCommand::None);
    }

    #[test]
    fn highest_wins_8bit_narrows_wider_commands_to_the_low_byte() {
        // `new _8BitCommand(int)` is `(byte)value`, so 0x0180 becomes 0x80, not 255.
        assert_eq!(combine(Combinator::HighestWins8, &[OutputCommand::U16(0x0180)]), OutputCommand::U8(0x80));
        assert_eq!(combine(Combinator::HighestWins8, &[OutputCommand::U32(0x0001_0002)]), OutputCommand::U8(2));
        assert_eq!(combine(Combinator::HighestWins8, &[OutputCommand::U8(5), OutputCommand::U16(0x0103)]), OutputCommand::U8(5));
        // A first `ulong` takes the float overload, a later one is truncated directly.
        assert_eq!(combine(Combinator::HighestWins8, &[OutputCommand::U64(300)]), OutputCommand::U8(44));
        assert_eq!(combine(Combinator::HighestWins8, &[OutputCommand::U8(1), OutputCommand::U64(300)]), OutputCommand::U8(44));
        assert_eq!(combine(Combinator::HighestWins8, &[OutputCommand::U64(u64::MAX)]), OutputCommand::U8(255));
    }

    #[test]
    fn highest_wins_8bit_takes_grayscale_of_a_colour() {
        // 0.3 R + 0.59 G + 0.11 B, truncated.
        let c = OutputCommand::Color(Color::rgb(100, 100, 100));
        assert_eq!(combine(Combinator::HighestWins8, &[c]), OutputCommand::U8(100));
        let c = OutputCommand::Color(Color::rgb(255, 0, 0));
        assert_eq!(combine(Combinator::HighestWins8, &[c]), OutputCommand::U8(76));
    }

    #[test]
    fn average_8bit_shifts_at_each_step_and_ignores_other_types() {
        let list = [OutputCommand::U8(10), OutputCommand::U8(21), OutputCommand::U16(9000), OutputCommand::U8(100)];
        // (10 + 21) >> 1 = 15, then (15 + 100) >> 1 = 57.
        assert_eq!(combine(Combinator::Average8, &list), OutputCommand::U8(57));
    }

    #[test]
    fn wider_highest_wins_truncate_later_values_to_a_byte() {
        let list = [OutputCommand::U32(5), OutputCommand::U32(0x0000_0102)];
        assert_eq!(combine(Combinator::HighestWins32, &list), OutputCommand::U32(5));
        let list = [OutputCommand::U64(5), OutputCommand::U64(0x0102)];
        assert_eq!(combine(Combinator::HighestWins64, &list), OutputCommand::U64(5));
        let list = [OutputCommand::U16(5), OutputCommand::U16(0x0102)];
        assert_eq!(combine(Combinator::HighestWins16, &list), OutputCommand::U16(0x0102));
    }

    #[test]
    fn naive_colour_averages_channels_and_keeps_the_highest_alpha() {
        let list = [OutputCommand::Color(Color::argb(10, 100, 0, 255)), OutputCommand::Color(Color::argb(200, 50, 100, 0))];
        assert_eq!(combine(Combinator::NaiveColor, &list), OutputCommand::Color(Color::argb(200, 75, 50, 127)));
    }

    #[test]
    fn colour_evaluator_repeats_the_last_value_for_a_range_state() {
        let range = StateValue::Range(RangeValue::new(FunctionIdentity::Pan, "Pan", 0.5, ""));
        // Nothing to repeat yet.
        assert_eq!(run(DataPolicy::ONE_CHANNEL_COLOR, std::slice::from_ref(&range)), OutputCommand::None);
        let s = [StateValue::Rgb(RgbValue { r: 100, g: 0, b: 0 }), range];
        // (100,0,0) merged with itself.
        assert_eq!(run(DataPolicy::ONE_CHANNEL_COLOR, &s), OutputCommand::Color(Color::rgb(100, 0, 0)));
    }

    #[test]
    fn a_single_command_passes_through_any_policy() {
        let mut p = PolicyState::new(DataPolicy::HIGHEST_WINS_8BIT);
        let c = OutputCommand::U16(1234);
        let mut out = OutputCommand::None;
        p.generate(Flow::Command(&c), &mut out);
        assert_eq!(out, c);
        assert_eq!(out.byte(), 0);
    }

    #[test]
    fn null_flows_give_a_null_command() {
        let mut p = PolicyState::new(DataPolicy::HIGHEST_WINS_8BIT);
        let mut out = OutputCommand::U8(9);
        p.generate(Flow::Null, &mut out);
        assert_eq!(out, OutputCommand::None);
        out = OutputCommand::U8(9);
        p.generate(Flow::NullIntents, &mut out);
        assert_eq!(out, OutputCommand::None);
        p.generate(Flow::Commands(&[]), &mut out);
        assert_eq!(out, OutputCommand::None);
    }

    #[test]
    fn bits16_evaluator_scales_to_65535() {
        let p = DataPolicy { evaluator: Evaluator::Bits16, combinator: Combinator::HighestWins16 };
        assert_eq!(run(p, &[StateValue::Lighting(LightingValue::new(Color::RED, 0.5))]), OutputCommand::U16(32767));
    }
}
