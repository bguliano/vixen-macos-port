//! The data flowing between components: the port of Vixen.Core/Data/Flow without the allocation. A frame's
//! `IDataFlowData` objects become borrowed views over storage the graph already owns.

use crate::command::OutputCommand;
use vixen_core::{CombinedState, CommandValue, DiscreteValue, LightingValue, RangeValue, RgbValue, StateValue};

/// One `IIntentState`, borrowed.
#[derive(Copy, Clone, Debug)]
pub(crate) enum StateRef<'a> {
    Lighting(LightingValue),
    Rgb(RgbValue),
    Discrete(DiscreteValue),
    Range(&'a RangeValue),
    Command(&'a CommandValue),
}

impl<'a> From<&'a StateValue> for StateRef<'a> {
    #[inline]
    fn from(v: &'a StateValue) -> StateRef<'a> {
        match v {
            StateValue::Lighting(l) => StateRef::Lighting(*l),
            StateValue::Rgb(r) => StateRef::Rgb(*r),
            StateValue::Discrete(d) => StateRef::Discrete(*d),
            StateValue::Range(r) => StateRef::Range(r),
            StateValue::Command(c) => StateRef::Command(c),
        }
    }
}

/// A list of intent states.
#[derive(Copy, Clone)]
pub(crate) enum States<'a> {
    /// An element's own state.
    Element(&'a CombinedState),
    /// An element whose discrete values a dimming curve may have changed in place this frame.
    Overlay(&'a CombinedState, &'a [DiscreteValue]),
    /// A filter's output list.
    List(&'a [StateValue]),
}

impl<'a> States<'a> {
    /// Visits the states in the order `LayeredStateCombinator.Combine` lists them: fixture range and command
    /// values, which pass through during the sort, then the mix, then the discrete colours. A lone
    /// lighting state only ever appears on its own.
    #[inline]
    pub(crate) fn for_each(self, mut f: impl FnMut(StateRef<'a>)) {
        match self {
            States::Element(cs) => {
                for s in cs.other.iter() {
                    f(s.into());
                }
                if let Some(l) = cs.lighting {
                    f(StateRef::Lighting(l));
                }
                if let Some(r) = cs.rgb {
                    f(StateRef::Rgb(r));
                }
                for d in cs.discrete.iter() {
                    f(StateRef::Discrete(*d));
                }
            }
            States::Overlay(cs, discrete) => {
                for s in cs.other.iter() {
                    f(s.into());
                }
                if let Some(l) = cs.lighting {
                    f(StateRef::Lighting(l));
                }
                if let Some(r) = cs.rgb {
                    f(StateRef::Rgb(r));
                }
                for d in discrete {
                    f(StateRef::Discrete(*d));
                }
            }
            States::List(list) => {
                for s in list {
                    f(s.into());
                }
            }
        }
    }

    #[inline]
    pub(crate) fn is_empty(self) -> bool {
        match self {
            States::Element(cs) | States::Overlay(cs, _) => cs.is_empty(),
            States::List(list) => list.is_empty(),
        }
    }
}

/// `DataFlowType`: what a component takes in or gives out, which the patching view uses to decide what can connect to what.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DataFlowType {
    None,
    SingleCommand,
    MultipleCommands,
    MultipleIntents,
    SingleIntent,
    SingleCommandOrIntent,
    MultipleCommandOrIntent,
}

/// What reading a component output's `Data` returns.
#[derive(Copy, Clone)]
pub(crate) enum Flow<'a> {
    /// `Data` itself is null: a dimming curve that hasn't run yet. Filters skip this.
    Null,
    /// An `IntentsDataFlowData` whose `Value` is null: a dimming curve that was given no states.
    NullIntents,
    /// `IntentsDataFlowData`.
    Intents(States<'a>),
    /// `CommandDataFlowData`.
    Command(&'a OutputCommand),
    /// `CommandsDataFlowData`.
    Commands(&'a [OutputCommand]),
}
