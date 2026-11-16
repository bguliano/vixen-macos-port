//! The Effect Editor without the window: the panel's rows, what a commit does, curve and gradient gestures, the two editor dialogs and the
//! libraries they use. `docs/spec/effect-editor.md` is the specification, and F numbers below refer to it.
//!
//! The curve, gradient and colour parts are also what the shared controls in VixenKit run on, so they don't depend on a sequence.

pub mod curve_strip;
pub mod defaults;
pub mod dialogs;
pub mod drag;
pub mod function;
pub mod gradient_strip;
pub mod libraries;
pub mod numeric;
pub mod panel;
pub mod preview;
pub mod values;

/// The modifier keys held. ⌘ is upstream's Ctrl and ⌥ its Alt. "Exactly" means no other modifier is down; the difference matters per gesture.
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct Mods {
    pub shift: bool,
    pub command: bool,
    pub option: bool,
}

impl Mods {
    pub fn exactly_shift(self) -> bool {
        self.shift && !self.command && !self.option
    }

    pub fn exactly_command(self) -> bool {
        self.command && !self.shift && !self.option
    }

    pub fn exactly_option(self) -> bool {
        self.option && !self.shift && !self.command
    }

    pub fn none(self) -> bool {
        !self.shift && !self.command && !self.option
    }
}

/// The mouse cursor a strip asks for.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum StripCursor {
    Arrow,
    Cross,
    ResizeHorizontal,
    ResizeVertical,
    Hand,
}
