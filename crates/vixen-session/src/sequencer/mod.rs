//! The sequencer window without the window: `TimedSequenceEditorForm`'s state and commands. `docs/spec/sequencer-shell.md` is the specification,
//! and F numbers below refer to it. Swift forwards menu items, toolbar buttons and keys here, and shows what comes back.

pub mod clipboard;
mod commands;
pub mod dialogs;
pub mod drops;
pub mod effect_ops;
pub mod effect_props;
pub mod find;
pub mod grid;
pub mod keys;
pub mod layers;
pub mod libraries;
pub mod lifecycle;
pub mod panels;
pub mod papagayo;
pub mod prefs;
pub mod time_box;
pub mod toolbars;
pub mod transport;

pub mod state;

pub use commands::*;

pub use state::*;
