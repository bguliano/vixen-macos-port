//! The value a controller output holds after the data policy has run: C#'s `ICommand`, or null.

use std::sync::Arc;
use vixen_core::{Color, CommandValue};

/// One output's command for a frame.
///
/// `None` is a null `ICommand`: nothing is patched to the output, or nothing produced a value. Every
/// byte-oriented controller treats it the same as an explicit 0.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum OutputCommand {
    #[default]
    None,
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    Color(Color),
    Str(Arc<str>),
}

impl OutputCommand {
    /// What the byte-oriented controllers read: the value of an `_8BitCommand`, and 0 for anything else.
    /// A 16-bit command at a controller output really does send 0, in every module.
    #[inline]
    pub fn byte(&self) -> u8 {
        match self {
            OutputCommand::U8(v) => *v,
            _ => 0,
        }
    }

    #[inline]
    pub fn is_none(&self) -> bool {
        matches!(self, OutputCommand::None)
    }

    /// The text of a `StringCommand`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            OutputCommand::Str(s) => Some(s),
            _ => None,
        }
    }
}

impl From<&CommandValue> for OutputCommand {
    /// A `Named8BitCommand` is an `_8BitCommand` as far as any controller is concerned.
    fn from(v: &CommandValue) -> OutputCommand {
        match v {
            CommandValue::U8(v) => OutputCommand::U8(*v),
            CommandValue::U16(v) => OutputCommand::U16(*v),
            CommandValue::U32(v) => OutputCommand::U32(*v),
            CommandValue::U64(v) => OutputCommand::U64(*v),
            CommandValue::Color(c) => OutputCommand::Color(*c),
            CommandValue::Str(s) => OutputCommand::Str(s.clone()),
            CommandValue::Named8(n) => OutputCommand::U8(n.value),
        }
    }
}

/// A borrowed command, for dispatching without touching a reference count.
#[derive(Copy, Clone, Debug)]
pub(crate) enum CommandRef<'a> {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    Color(Color),
    Str(&'a Arc<str>),
}

impl<'a> CommandRef<'a> {
    #[inline]
    pub(crate) fn from_value(v: &'a CommandValue) -> CommandRef<'a> {
        match v {
            CommandValue::U8(v) => CommandRef::U8(*v),
            CommandValue::U16(v) => CommandRef::U16(*v),
            CommandValue::U32(v) => CommandRef::U32(*v),
            CommandValue::U64(v) => CommandRef::U64(*v),
            CommandValue::Color(c) => CommandRef::Color(*c),
            CommandValue::Str(s) => CommandRef::Str(s),
            CommandValue::Named8(n) => CommandRef::U8(n.value),
        }
    }

    /// `None` for a null command.
    #[inline]
    pub(crate) fn from_output(c: &'a OutputCommand) -> Option<CommandRef<'a>> {
        match c {
            OutputCommand::None => None,
            OutputCommand::U8(v) => Some(CommandRef::U8(*v)),
            OutputCommand::U16(v) => Some(CommandRef::U16(*v)),
            OutputCommand::U32(v) => Some(CommandRef::U32(*v)),
            OutputCommand::U64(v) => Some(CommandRef::U64(*v)),
            OutputCommand::Color(c) => Some(CommandRef::Color(*c)),
            OutputCommand::Str(s) => Some(CommandRef::Str(s)),
        }
    }

    #[inline]
    pub(crate) fn to_owned(self) -> OutputCommand {
        match self {
            CommandRef::U8(v) => OutputCommand::U8(v),
            CommandRef::U16(v) => OutputCommand::U16(v),
            CommandRef::U32(v) => OutputCommand::U32(v),
            CommandRef::U64(v) => OutputCommand::U64(v),
            CommandRef::Color(c) => OutputCommand::Color(c),
            CommandRef::Str(s) => OutputCommand::Str(s.clone()),
        }
    }
}
