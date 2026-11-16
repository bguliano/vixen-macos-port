//! The real-time heart of the app: turns rendered effects into a per-element schedule, evaluates it at
//! each frame and hands the result to whatever consumes it (controllers, previews).
//!
//! This replaces Vixen's `ContextBase` / `IntentStateBuilder` / `Element.Update` path. Semantics match, but
//! the structure doesn't: Vixen walks every active effect's whole intent dictionary each frame, boxing state
//! objects and locking a shared dictionary per intent. Here each element has a time-sorted schedule that is
//! searched directly, and a frame allocates nothing.

pub mod playback;
pub mod render;
pub mod schedule;

pub use playback::{ContextPlayback, Engine, EngineStats, FrameSink, InternalClock, TimeSource, TimingTotals, Transport};
pub use render::{render_all, render_effect, EffectJob, RenderEnv, RenderProgress};
pub use schedule::{ElementMap, EvalScratch, Schedule, ScheduledEffect};
