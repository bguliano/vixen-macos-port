//! What the element property, output filter and controller setup dialogs do, without the windows. See
//! docs/spec/property-filter-controller-setup.md; each function names the F-number it implements.
//!
//! Dialogs fetch a draft and apply it in one call on OK, so Cancel never has anything to undo. Applying changes the profile in
//! memory. It reaches the disk when Display Setup saves, or through [`crate::app::AppCore`] in any other caller.

pub mod color;
pub mod controllers;
pub mod dimming;
pub mod e131;
pub mod face;
pub mod filters;
pub mod flow;
pub mod layers;
pub mod order;
pub mod properties;
pub mod serial;
pub mod setup_world;
pub mod state;
pub mod state_preview;
pub mod store;
pub mod world;
