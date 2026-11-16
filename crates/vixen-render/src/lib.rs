//! GPU renderer for the preview and the display-design canvas.
//!
//! The original rasterises every light on the CPU, on the UI thread, and copies the whole window framebuffer
//! several times per frame. Here a frame costs an upload of the colours that changed plus at most four draw
//! calls, on a thread of its own, so neither the UI nor playback can stall it or be stalled by it.
//!
//! - [`Renderer`] does the drawing, into a `CAMetalLayer` or an offscreen texture.
//! - [`LiveRenderer`] runs a `Renderer` on its dedicated thread. This is what the app should use.
//!
//! No wgpu types appear in the API.
//!
//! # Coordinates
//!
//! Sizes passed to [`Renderer::for_layer`], [`Renderer::headless`] and [`Renderer::resize`] are physical
//! pixels, and `scale` is physical pixels per logical pixel (`backingScaleFactor`). A scene point lands at
//! `(scene * zoom + pan) * scale` physical pixels, so `pan` is in logical pixels.
//!
//! Scene coordinates name pixels, as they do in `FastPixel`: at zoom 1 and scale 1, a light at (10, 20) is
//! centred on the pixel at column 10, row 20. The background covers scene (0, 0) to (w, h), so that pixel is
//! also image pixel (10, 20).
//!
//! # Colours
//!
//! Every `u32` colour is RGBA8 with straight alpha, with its bytes in that order in memory, the same order as
//! the `rgba` byte slices and [`Renderer::read_pixels`]. Numerically that is `0xAABBGGRR`, not `0xRRGGBBAA`
//! or `vixen_core::Color::to_argb_u32`'s `0xAARRGGBB`. Use [`rgba`] to build one.
//!
//! Blending happens on the encoded values, not in linear light, because that is what `FastPixel.SetPixel` and
//! GDI+ do: a 50% red over black is 128, not 188.

mod colors;
mod fixture;
mod live;
mod mailbox;
mod objc;
mod renderer;

use std::fmt;
use std::time::Duration;

pub use live::{LiveRenderer, LiveStats};
pub use renderer::Renderer;
pub use fixture::{moving_head_beam_radii, BeamWidthModel, MovingHeadInstance, MovingHeadParams};

/// Builds a colour from its channels. See the [crate documentation](crate#colours) for the layout.
#[inline]
pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
    u32::from_le_bytes([r, g, b, a])
}

/// One light. `#[repr(C)]` three `f32`s, which is also its layout in the GPU buffer.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightGeom {
    /// Scene position of the centre of the light. `PreviewPixel.Bounds.X`: `FastPixel.DrawCircle` centres on
    /// the rectangle's corner, not its middle.
    pub x: f32,
    /// See [`LightGeom::x`].
    pub y: f32,
    /// `PreviewPixel.PixelSize`, in logical pixels on screen at any zoom: as in `PreviewPixel.Draw`, zoom moves
    /// a light but doesn't resize it.
    ///
    /// Like the original, 1 lights exactly one pixel, and otherwise the radius is `size / 2` rounded down, so
    /// 2 and 3 draw the same. Anything above 0 is visible, and 0 or less draws nothing.
    pub size: f32,
}

/// Editor graphics, drawn above the lights. Positions are in scene coordinates and follow the view. Widths and
/// sizes are in logical pixels and don't change with zoom. Text belongs to the Swift view on top of this one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Overlay {
    /// Shape outlines and rubber bands. Drawn first.
    pub lines: Vec<OverlayLine>,
    /// Selection handles. Drawn above lines.
    pub squares: Vec<OverlaySquare>,
    /// Filled discs, such as the rotate handle: `(center, radius, color)`, drawn above squares. The radius is
    /// the real radius, unlike `FastPixel.DrawCircle(X, Y, radius, color)`, which halves its argument: the
    /// original's `DrawCircle(x, y, SelectPointSize, …)` is radius 3 here.
    pub circles: Vec<([f32; 2], f32, u32)>,
}

/// A straight stroke from `a` to `b`, including both end points.
///
/// Horizontal and vertical lines stay crisp at any zoom and are always a whole number of pixels wide. Sloped
/// lines are anti-aliased.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct OverlayLine {
    pub a: [f32; 2],
    pub b: [f32; 2],
    pub color: u32,
    /// Stroke width. Never thinner than one physical pixel.
    pub width: f32,
}

/// A hollow square with a 1 logical pixel outline, as `FastPixel.DrawRectangle(X, Y, width, color)` draws a
/// selection handle: it spans `center - size / 2` to `center + size / 2`, with `size / 2` rounded down, so
/// `PreviewBaseShape.SelectPointSize` of 6 is 7 pixels across.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct OverlaySquare {
    pub center: [f32; 2],
    pub size: f32,
    pub color: u32,
}

/// What one call to [`Renderer::render`] did.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct FrameStats {
    /// CPU time on the calling thread, from this thread's CPU clock, so time blocked on the display or the GPU
    /// doesn't count.
    pub cpu_time: Duration,
    /// Elapsed time, including any wait for a drawable or for an earlier offscreen frame.
    pub wall_time: Duration,
    /// Never more than 4: background, fixtures, lights, overlay.
    pub draw_calls: u32,
    pub lights_drawn: u32,
    /// Lights whose colours were copied to the GPU for this frame. Changes are tracked in blocks of 64.
    pub lights_uploaded: u32,
    /// Set if no frame was drawn. Everything stays pending for the next call.
    pub skipped: Option<FrameSkip>,
}

/// Why [`Renderer::render`] drew nothing. None of these are failures.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FrameSkip {
    /// The target has no area.
    ZeroSize,
    /// The window hosting the layer isn't visible.
    Occluded,
    /// No drawable became free within a second.
    Timeout,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderError {
    /// No Metal adapter on this machine.
    NoAdapter,
    /// `for_layer` was given a null pointer.
    NullLayer,
    /// `for_layer` was given an object that isn't a `CAMetalLayer`, such as a view's default `CALayer`.
    NotAMetalLayer,
    /// The GPU device could not be created.
    Device(String),
    /// The surface could not be created or used.
    Surface(String),
    /// wgpu reported an error, or the device was lost.
    Gpu(String),
    /// A call was given something unusable, such as a pixel buffer of the wrong length. The call did nothing
    /// and the error surfaces from the next `render`.
    InvalidInput(String),
    /// The render thread could not be started.
    Thread(String),
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::NoAdapter => write!(f, "no Metal GPU adapter is available"),
            RenderError::NullLayer => write!(f, "the CAMetalLayer pointer is null"),
            RenderError::NotAMetalLayer => write!(f, "the layer is not a CAMetalLayer"),
            RenderError::Device(message) => write!(f, "could not create the GPU device: {message}"),
            RenderError::Surface(message) => write!(f, "surface error: {message}"),
            RenderError::Gpu(message) => write!(f, "GPU error: {message}"),
            RenderError::InvalidInput(message) => write!(f, "invalid input: {message}"),
            RenderError::Thread(message) => write!(f, "could not start the render thread: {message}"),
        }
    }
}

impl std::error::Error for RenderError {}
