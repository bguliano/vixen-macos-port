//! The window's hotkeys, section D of the spec, as one table that's tested with no window: `OnKeyDown` in
//! `TimedSequenceEditorForm_Hotkeys.cs`. Menu shortcuts win first, as upstream's do, so ⌘S never gets here while Save is enabled.
//! ⌘ is upstream Ctrl and ⌥ is Alt.

/// Modifier keys. ⌃ is left out: on the Mac it means right-click.
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
    pub const SHIFT_CMD: Mods = Mods { shift: true, cmd: true, opt: false };

    #[inline]
    pub fn is_none(self) -> bool {
        self == Mods::NONE
    }

    /// `ModifierKeys == Keys.Shift`.
    #[inline]
    pub fn exactly_shift(self) -> bool {
        self == Mods::SHIFT
    }
}

/// The keys the window listens for.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Key {
    Home,
    End,
    PageUp,
    PageDown,
    Up,
    Down,
    Left,
    Right,
    Space,
    Escape,
    Minus,
    Equals,
    Delete,
    F5,
    F8,
    F9,
    /// A letter, lower case.
    Letter(char),
}

/// How an alignment key treats the other edge.
pub type Hold = bool;

/// What a key does. Each is a method on the window state or the timeline.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum HotkeyCommand {
    ScrollRowsToTop,
    ScrollToSequenceStart,
    ScrollRowsToBottom,
    ScrollToSequenceEnd,
    PageRowsUp,
    PageTimeBack,
    PageRowsDown,
    PageTimeForward,
    /// `MoveActiveRow(-1)`.
    ActiveRowUp,
    /// `MoveActiveRow(1)`.
    ActiveRowDown,
    /// Not running or paused: Play. Otherwise Stop.
    SpacePlayStop,
    /// Not running or paused: Play. Otherwise Pause.
    SpacePlayPause,
    /// `AddMarkAtTime(TimingSource.Position, false)`, when there's a timing source.
    AddMarkAtPlayPosition,
    NudgeSelectionBack,
    NudgeSelectionForward,
    AlignStart(Hold),
    AlignStartToNearestMark,
    AlignEnd(Hold),
    AlignEndToNearestMark,
    AlignBoth,
    AlignBothToNearestMark,
    /// Not while drawing an effect: deselect the Effects window, draw mode off, selection mode on.
    EscapeDrawMode,
    ZoomTimeOut,
    ZoomRowsOut,
    ZoomTimeIn,
    ZoomRowsIn,
    /// `ToggleSelectedRows(includeChildren)`.
    ToggleSelectedRows { include_children: bool },
    DistributeEqually,
    SplitAtPointer,
    AlignCenters,
    AlignDurations(Hold),
    AlignStartToEnd(Hold),
    AlignEndToStart(Hold),
    DistributeDialog,
    PlayPause,
    Stop,
    ToggleLoop,
    DeleteSelected,
}

/// `OnKeyDown`'s switch. Keys that don't exclude other modifiers still fire: Shift+D is D, ⇧⌘D is ⌘D, ⌥T is T, ⇧⌘T is ⌘T. ⌘E, ⌘B, ⌘U,
/// ⌘R and ⌘N have no menu binding, so they behave like the plain key with Shift off.
pub fn command_for_key(key: Key, mods: Mods) -> Option<HotkeyCommand> {
    use HotkeyCommand as C;
    Some(match key {
        Key::Home if mods.cmd => C::ScrollToSequenceStart,
        Key::Home => C::ScrollRowsToTop,
        Key::End if mods.cmd => C::ScrollToSequenceEnd,
        Key::End => C::ScrollRowsToBottom,
        Key::PageUp if mods.cmd => C::PageTimeBack,
        Key::PageUp => C::PageRowsUp,
        Key::PageDown if mods.cmd => C::PageTimeForward,
        Key::PageDown => C::PageRowsDown,
        Key::Up => return mods.is_none().then_some(C::ActiveRowUp),
        Key::Down => return mods.is_none().then_some(C::ActiveRowDown),
        Key::Space if mods.shift => C::AddMarkAtPlayPosition,
        // ⌘Space is Spotlight and ⌃Space switches the input source, so pause is ⌥Space. F192.
        Key::Space if mods.opt || mods.cmd => C::SpacePlayPause,
        Key::Space => C::SpacePlayStop,
        Key::Left => return mods.cmd.then_some(C::NudgeSelectionBack),
        Key::Right => return mods.cmd.then_some(C::NudgeSelectionForward),
        Key::Escape => C::EscapeDrawMode,
        Key::Minus if mods.cmd && mods.shift => C::ZoomRowsOut,
        Key::Minus => return mods.cmd.then_some(C::ZoomTimeOut),
        Key::Equals if mods.cmd && mods.shift => C::ZoomRowsIn,
        Key::Equals => return mods.cmd.then_some(C::ZoomTimeIn),
        Key::Delete => C::DeleteSelected,
        Key::F5 => C::PlayPause,
        Key::F8 => C::Stop,
        Key::F9 => C::ToggleLoop,
        Key::Letter(c) => return letter_command(c, mods),
    })
}

fn letter_command(c: char, mods: Mods) -> Option<HotkeyCommand> {
    use HotkeyCommand as C;
    Some(match c {
        's' if mods.shift && mods.cmd => C::AlignStartToNearestMark,
        's' => C::AlignStart(mods.shift),
        'e' if mods.shift && mods.cmd => C::AlignEndToNearestMark,
        'e' => C::AlignEnd(mods.shift),
        'b' if mods.shift && mods.cmd => C::AlignBothToNearestMark,
        'b' => C::AlignBoth,
        't' => C::ToggleSelectedRows { include_children: mods.cmd },
        'd' if mods.cmd => C::SplitAtPointer,
        'd' => C::DistributeEqually,
        'c' => C::AlignCenters,
        'u' => C::AlignDurations(mods.exactly_shift()),
        'r' => C::AlignStartToEnd(mods.exactly_shift()),
        'n' => C::AlignEndToStart(mods.exactly_shift()),
        'i' => C::DistributeDialog,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use HotkeyCommand as C;

    fn letter(c: char, mods: Mods) -> Option<C> {
        command_for_key(Key::Letter(c), mods)
    }

    #[test]
    fn the_table() {
        let rows: &[(Key, Mods, C)] = &[
            (Key::Home, Mods::NONE, C::ScrollRowsToTop),
            (Key::Home, Mods::CMD, C::ScrollToSequenceStart),
            (Key::End, Mods::NONE, C::ScrollRowsToBottom),
            (Key::End, Mods::CMD, C::ScrollToSequenceEnd),
            (Key::PageUp, Mods::NONE, C::PageRowsUp),
            (Key::PageUp, Mods::CMD, C::PageTimeBack),
            (Key::PageDown, Mods::NONE, C::PageRowsDown),
            (Key::PageDown, Mods::CMD, C::PageTimeForward),
            (Key::Up, Mods::NONE, C::ActiveRowUp),
            (Key::Down, Mods::NONE, C::ActiveRowDown),
            (Key::Space, Mods::NONE, C::SpacePlayStop),
            (Key::Space, Mods::OPT, C::SpacePlayPause),
            (Key::Space, Mods::SHIFT, C::AddMarkAtPlayPosition),
            (Key::Left, Mods::CMD, C::NudgeSelectionBack),
            (Key::Right, Mods::CMD, C::NudgeSelectionForward),
            (Key::Escape, Mods::NONE, C::EscapeDrawMode),
            (Key::Minus, Mods::CMD, C::ZoomTimeOut),
            (Key::Minus, Mods::SHIFT_CMD, C::ZoomRowsOut),
            (Key::Equals, Mods::CMD, C::ZoomTimeIn),
            (Key::Equals, Mods::SHIFT_CMD, C::ZoomRowsIn),
            (Key::Delete, Mods::NONE, C::DeleteSelected),
            (Key::F5, Mods::NONE, C::PlayPause),
            (Key::F8, Mods::NONE, C::Stop),
            (Key::F9, Mods::NONE, C::ToggleLoop),
        ];
        for (key, mods, expected) in rows {
            assert_eq!(command_for_key(*key, *mods), Some(*expected), "{key:?} {mods:?}");
        }
        let letters: &[(char, Mods, C)] = &[
            ('s', Mods::NONE, C::AlignStart(false)),
            ('s', Mods::SHIFT, C::AlignStart(true)),
            ('s', Mods::SHIFT_CMD, C::AlignStartToNearestMark),
            ('e', Mods::NONE, C::AlignEnd(false)),
            ('e', Mods::SHIFT, C::AlignEnd(true)),
            ('e', Mods::SHIFT_CMD, C::AlignEndToNearestMark),
            ('b', Mods::NONE, C::AlignBoth),
            ('b', Mods::SHIFT_CMD, C::AlignBothToNearestMark),
            ('t', Mods::NONE, C::ToggleSelectedRows { include_children: false }),
            ('t', Mods::CMD, C::ToggleSelectedRows { include_children: true }),
            ('d', Mods::NONE, C::DistributeEqually),
            ('d', Mods::CMD, C::SplitAtPointer),
            ('c', Mods::NONE, C::AlignCenters),
            ('u', Mods::NONE, C::AlignDurations(false)),
            ('u', Mods::SHIFT, C::AlignDurations(true)),
            ('r', Mods::NONE, C::AlignStartToEnd(false)),
            ('r', Mods::SHIFT, C::AlignStartToEnd(true)),
            ('n', Mods::NONE, C::AlignEndToStart(false)),
            ('n', Mods::SHIFT, C::AlignEndToStart(true)),
            ('i', Mods::NONE, C::DistributeDialog),
        ];
        for (c, mods, expected) in letters {
            assert_eq!(letter(*c, *mods), Some(*expected), "{c} {mods:?}");
        }
    }

    #[test]
    fn keys_that_do_not_exclude_other_modifiers_still_fire() {
        assert_eq!(letter('d', Mods::SHIFT), Some(C::DistributeEqually));
        assert_eq!(letter('d', Mods::SHIFT_CMD), Some(C::SplitAtPointer));
        assert_eq!(letter('t', Mods::OPT), Some(C::ToggleSelectedRows { include_children: false }));
        assert_eq!(letter('t', Mods::SHIFT_CMD), Some(C::ToggleSelectedRows { include_children: true }));
        assert_eq!(letter('b', Mods::SHIFT), Some(C::AlignBoth));
    }

    #[test]
    fn command_letters_with_no_menu_binding_act_like_the_plain_key() {
        assert_eq!(letter('e', Mods::CMD), Some(C::AlignEnd(false)));
        assert_eq!(letter('b', Mods::CMD), Some(C::AlignBoth));
        assert_eq!(letter('u', Mods::CMD), Some(C::AlignDurations(false)));
        assert_eq!(letter('r', Mods::CMD), Some(C::AlignStartToEnd(false)));
        assert_eq!(letter('n', Mods::CMD), Some(C::AlignEndToStart(false)));
        // "exactly Shift" tests are false with anything else down.
        assert_eq!(letter('u', Mods::SHIFT_CMD), Some(C::AlignDurations(false)));
    }

    #[test]
    fn arrows_need_their_exact_modifiers() {
        assert_eq!(command_for_key(Key::Up, Mods::SHIFT), None);
        assert_eq!(command_for_key(Key::Down, Mods::CMD), None);
        assert_eq!(command_for_key(Key::Left, Mods::NONE), None);
        assert_eq!(command_for_key(Key::Right, Mods::SHIFT), None);
        assert_eq!(command_for_key(Key::Minus, Mods::NONE), None);
        assert_eq!(command_for_key(Key::Equals, Mods::SHIFT), None);
    }

    #[test]
    fn other_letters_do_nothing() {
        assert_eq!(letter('q', Mods::NONE), None);
        assert_eq!(letter('z', Mods::CMD), None);
    }
}
