//! The preview model: every shape, its geometry and hit testing, and the design surface's behaviour.
//! Nothing here touches a pixel. This crate computes where lights are, and a renderer draws them.
//!
//! The original redraws the whole window on the CPU on every mouse move. Here a mouse event changes the
//! lights of the shape it moved and nothing else, and the renderer is told exactly which range that was:
//!
//! - [`PreviewData`] is the `VixenPreviewData` module as stored in `ModuleStore.xml`. Whatever it doesn't
//!   understand, unknown members and shape types included, is written back unchanged.
//! - [`Scene`] is the flat list of lights for the renderer: `lights[i]` is where a light is, `links[i]` is the
//!   element node it shows. Colours are not stored in it. Each shape owns one contiguous range.
//! - [`DesignSession`] is `VixenPreviewControl`'s design surface: mouse and keyboard events go in, and a
//!   [`Change`] comes out saying which light ranges moved and what else the host should refresh.
//!   [`DesignSession::overlay`] is everything drawn over the lights.
//!
//! # Coordinates
//!
//! A host drives the renderer's view with `zoom` from [`Scene::zoom`] and a pan of minus the session's
//! [`scroll`](DesignSession::scroll), which puts a light on the pixel the C# would draw it on and puts it under
//! the mouse events it receives, which are in view coordinates. A light's size doesn't change with zoom. The
//! [scene documentation](scene#coordinates) has the details.
//!
//! [`LightGeom`] has the same fields and layout as `vixen_render::LightGeom`, with (x, y) the centre of the
//! light. `FastPixel.DrawCircle` centres on the rectangle's corner, not its middle.
//!
//! # Hosting
//!
//! After each event, for every [`SceneDelta`] in [`Change::scene`]: send `scene.lights[delta.range]` to the
//! renderer, through [`LightGeom::as_floats`] if it has a type of its own, or the whole array if `delta.resized`.
//! Read the [`Overlay`] if [`Change::overlay_changed`] says so. If [`Change::light_colors_changed`] does too,
//! [`Overlay::fill_light_colors`] gives each light its edit colour in the renderer's colour format. A
//! [`LightRun::size_delta`](overlay::LightRun::size_delta) is the extra size of a pixel string's first pixel on
//! the design surface, which [`Scene::lights`] leaves out.
//!
//! # Differences from the C#
//!
//! The behaviour is the original's, quirks included, each with a comment where it is odd on purpose. An exception
//! in the C# becomes a no-op here. One thing is laid out differently: a discrete colour element has one light
//! per colour in its colour set, each with its own fixed slot, where the C# compacts the colours that are lit
//! into the first slots. So a lit colour can sit further along than the C# would put it.

pub mod custom_prop_builder;
pub mod data;
pub mod fixture_state;
pub mod hit;
pub mod link;
pub mod overlay;
pub mod pixel;
pub mod point;
pub mod scene;
pub mod session;
pub mod shape;
pub mod template;
pub mod tools;
pub mod xml_util;

pub use data::{DisplayItem, PreviewData, PreviewError, ShapeId, VIXEN_PREVIEW_MODULE_TYPE};
pub use link::{fixture_nodes, link_elements, ElementLinker, LinkRow};
pub use overlay::Overlay;
pub use scene::{LightGeom, LightLink, Scene, SceneDelta};
pub use session::{Change, Cursor, DesignSession, Key, MenuItem, MouseButton, Tool, ToolbarState, VerifyReport};
pub use shape::properties::{PropertyError, PropertyKind, PropertyValue, ShapeProperty};
pub use shape::{Modifiers, Shape, ShapeKind};
pub use template::{valid_template_name, TemplateStore};
