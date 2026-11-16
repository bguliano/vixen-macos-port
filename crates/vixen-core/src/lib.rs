//! Vixen core: exact Rust ports of the math and data model in Vixen.Core and the
//! Curves / ColorGradients app modules.
//!
//! Everything here must stay numerically faithful to the C#: truncating casts, banker's
//! rounding and float precision are all deliberate, because they decide the colours that
//! reach the lights. See docs/PORTING_GUIDE.md.

pub mod color;
pub mod curve;
pub mod element;
pub mod gradient;
pub mod intent;
pub mod known_color;
pub mod layer;
pub mod marks;
pub mod time;
pub mod value;

pub use color::{Color, Hsv, Rgb, Xyz};
pub use curve::Curve;
pub use element::{ElementColorType, ElementTree, NodeId, Orientation, PropertyData, PropertyInstance};
pub use gradient::{AlphaPoint, ColorGradient, ColorPoint, GradientLevelPair};
pub use intent::{EffectIntents, FrameArray, Intent, IntentData, IntentNode, StateValue};
pub use layer::{CombinedState, Layer, LayerType, LayeredState, MixingFilter, SequenceLayers, StateCombinator};
pub use marks::{Mark, MarkCollection};
pub use time::TimeSpan;
pub use uuid::Uuid;
pub use value::{CommandValue, DiscreteValue, FixtureIndexType, FunctionIdentity, LightingValue, NamedCommand, RangeValue, RgbValue};
