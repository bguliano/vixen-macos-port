//! The timeline control without its windows: `Vixen.Common/Controls/TimeLineControl` and the parts of
//! `TimedSequenceEditorForm` that answer it. `docs/spec/timeline.md` is the specification; F and Q numbers below refer to it.
//!
//! Everything here is plain state and arithmetic, so it runs and is tested with no window. Swift forwards pointer events
//! and draws what comes back. The surfaces hold no copies of times or rows: paint, hit testing and rubber bands all read
//! the same per-row occurrence lists, which is the fix for upstream writing geometry onto effects during paint (Q3).

mod commands;
mod grid_input;
pub mod host;
pub mod marks;
pub mod md5;
pub mod model;
pub mod row_input;
pub mod rows;
pub mod ruler;
pub mod session_link;
pub mod snap;
pub mod stack;
pub mod thumb;
pub mod view;
pub mod waveform;

#[cfg(test)]
mod tests;

pub use commands::{AlignKind, AlignResult, ScrollEvent, Surface, ALIGNMENT_WARNING};
pub use grid_input::PointerEvent;
pub use host::TimelineHost;
pub use model::*;
pub use rows::*;
pub use stack::*;
pub use view::*;

pub use crate::sequence::EffectId as ElementId;
use vixen_core::TimeSpan;

/// One row occurrence: the same display element reached through two groups makes two rows. An index into the flat list.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct RowId(pub u32);

impl RowId {
    #[inline]
    pub fn idx(self) -> usize {
        self.0 as usize
    }
}

/// Modifier keys as upstream tests them. ⌘ is Ctrl, ⌥ is Alt. ⌃ is ignored for "exactly" tests: on the Mac it means right-click.
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct Mods {
    pub shift: bool,
    pub cmd: bool,
    pub opt: bool,
}

impl Mods {
    pub const NONE: Mods = Mods { shift: false, cmd: false, opt: false };
    pub const SHIFT: Mods = Mods { shift: true, cmd: false, opt: false };
    pub const CMD: Mods = Mods { shift: false, cmd: true, opt: false };
    pub const OPT: Mods = Mods { shift: false, cmd: false, opt: true };

    /// `ModifierKeys == ...`: these and nothing else.
    #[inline]
    pub fn exactly(self, shift: bool, cmd: bool, opt: bool) -> bool {
        self.shift == shift && self.cmd == cmd && self.opt == opt
    }

    #[inline]
    pub fn none(self) -> bool {
        self == Mods::NONE
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Button {
    Left,
    Right,
    Middle,
}

/// `System.Drawing.Rectangle`.
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct IRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl IRect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> IRect {
        IRect { x, y, w, h }
    }

    #[inline]
    pub fn right(&self) -> i32 {
        self.x.saturating_add(self.w)
    }

    #[inline]
    pub fn bottom(&self) -> i32 {
        self.y.saturating_add(self.h)
    }

    /// `Rectangle.Contains`: left and top inclusive, right and bottom exclusive.
    #[inline]
    pub fn contains(&self, p: (i32, i32)) -> bool {
        p.0 >= self.x && p.0 < self.right() && p.1 >= self.y && p.1 < self.bottom()
    }

    /// `Rectangle.IsEmpty`: all four are zero.
    #[inline]
    pub fn is_empty(&self) -> bool {
        *self == IRect::default()
    }

    /// `Rectangle.Size.IsEmpty`.
    #[inline]
    pub fn size_is_empty(&self) -> bool {
        self.w == 0 && self.h == 0
    }

    #[inline]
    pub fn intersects(&self, o: &IRect) -> bool {
        self.x < o.right() && o.x < self.right() && self.y < o.bottom() && o.y < self.bottom()
    }

    pub fn union(&self, o: &IRect) -> IRect {
        let x = self.x.min(o.x);
        let y = self.y.min(o.y);
        IRect { x, y, w: self.right().max(o.right()) - x, h: self.bottom().max(o.bottom()) - y }
    }

    /// `Util.RectangleFromPoints`.
    pub fn from_points(top_left: (i32, i32), bottom_right: (i32, i32)) -> IRect {
        IRect { x: top_left.0, y: top_left.1, w: bottom_right.0 - top_left.0, h: bottom_right.1 - top_left.1 }
    }
}

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum ResizeZone {
    #[default]
    None,
    Front,
    Back,
}

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum DragState {
    #[default]
    Normal,
    Waiting,
    Moving,
    Selecting,
    HResizing,
    Drawing,
}

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum CursorKind {
    #[default]
    Arrow,
    ResizeLeftRight,
    ResizeUpDown,
    Crosshair,
    Move,
    Hand,
}

/// `Element.StartTime` and `Duration`.
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct ElementTimes {
    pub start: TimeSpan,
    pub duration: TimeSpan,
}

impl ElementTimes {
    pub const fn new(start: TimeSpan, duration: TimeSpan) -> ElementTimes {
        ElementTimes { start, duration }
    }

    #[inline]
    pub fn end(&self) -> TimeSpan {
        self.start + self.duration
    }
}
