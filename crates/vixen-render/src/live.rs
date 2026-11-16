//! Runs a [`Renderer`] on a thread of its own.
//!
//! Nothing on this thread can stall the UI or playback, because they never wait for it: colours go through a
//! lock-free mailbox and everything else through an unbounded channel. It blocks in only two places, and both
//! are its own business: waiting for work, and waiting for a drawable, which is what paces it to the display.
//! With nothing changing it stays parked, so an idle preview uses no CPU and no GPU.

use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::thread::{self, JoinHandle, Thread};
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use crate::mailbox::{Mailbox, MailboxReader};
use crate::objc::AutoreleasePool;
use crate::renderer::{thread_cpu_time, Acquired};
use crate::{FrameSkip, LightGeom, MovingHeadInstance, Overlay, RenderError, Renderer};

/// How often to look again while the window is hidden or no drawable is free, if a change is waiting. An
/// up-to-date frame doesn't come back here at all.
const SKIPPED_FRAME_RETRY: Duration = Duration::from_millis(100);
const FAILED_FRAME_RETRY: Duration = Duration::from_secs(1);

enum Command {
    SetBackground { rgba: Vec<u8>, w: u32, h: u32 },
    SetBackgroundAlpha(u8),
    ClearBackground,
    SetLights(Vec<LightGeom>),
    UpdateLightRange { start: usize, lights: Vec<LightGeom> },
    SetOverlay(Overlay),
    SetMovingHeads(Vec<MovingHeadInstance>),
    SetCameraDepth(f32),
    SetFixtureReference([f32; 2]),
    SetLightScale(f32),
    SetTransparent(bool),
    SetView { zoom: f32, pan_x: f32, pan_y: f32 },
    Resize { width_px: u32, height_px: u32, scale: f32 },
    Redraw,
    ReadPixels(Sender<Vec<u8>>),
    Quit,
}

/// What [`LiveRenderer::stats`] reports.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LiveStats {
    pub frames_rendered: u64,
    /// CPU time of the last frame on the render thread, from taking its colours to presenting it.
    pub last_frame_cpu: Duration,
    /// Published colour frames that were replaced by a newer one before the render thread got to them.
    pub frames_dropped: u64,
    /// The most recent failure, if there has been one. Rendering keeps trying.
    pub last_error: Option<String>,
    /// Frames not drawn because the window wasn't visible.
    pub frames_occluded: u64,
    /// Frames not drawn because no drawable came free in time.
    pub frames_timed_out: u64,
    /// Published colour frames the render thread took, whether or not any colour differed. Far more of these than
    /// `frames_rendered` means the show isn't changing much, and that the thread is keeping up.
    pub colors_taken: u64,
}

/// How many frame gaps [`LiveRenderer::recent_frame_gaps`] remembers.
const FRAME_GAP_HISTORY: usize = 512;

struct Shared {
    frames_rendered: AtomicU64,
    last_frame_cpu_nanos: AtomicU64,
    frames_occluded: AtomicU64,
    frames_timed_out: AtomicU64,
    colors_taken: AtomicU64,
    last_error: Mutex<Option<String>>,
    frame_gaps_micros: [AtomicU32; FRAME_GAP_HISTORY],
}

/// A [`Renderer`] on a dedicated thread, at `QOS_CLASS_USER_INTERACTIVE`.
///
/// Every method returns right away, from any thread, whatever the renderer is doing. Changes apply in the order
/// they are made, and the thread draws as soon as a drawable is free once something has changed.
pub struct LiveRenderer {
    commands: Sender<Command>,
    mailbox: Arc<Mailbox>,
    shared: Arc<Shared>,
    waker: Thread,
    thread: Option<JoinHandle<()>>,
}

impl LiveRenderer {
    /// Starts the render thread. It draws a first frame right away, so the layer doesn't start out blank.
    pub fn new(renderer: Renderer) -> Result<LiveRenderer, RenderError> {
        let (commands, receiver) = mpsc::channel();
        let (mailbox, reader) = Mailbox::new();
        let shared = Arc::new(Shared {
            frames_rendered: AtomicU64::new(0),
            last_frame_cpu_nanos: AtomicU64::new(0),
            frames_occluded: AtomicU64::new(0),
            frames_timed_out: AtomicU64::new(0),
            colors_taken: AtomicU64::new(0),
            last_error: Mutex::new(None),
            frame_gaps_micros: std::array::from_fn(|_| AtomicU32::new(0)),
        });

        let thread = {
            let shared = shared.clone();
            thread::Builder::new()
                .name("vixen-render".to_string())
                .spawn(move || run(renderer, receiver, reader, shared))
                .map_err(|error| RenderError::Thread(error.to_string()))?
        };
        Ok(LiveRenderer { commands, mailbox, shared, waker: thread.thread().clone(), thread: Some(thread) })
    }

    /// [`Renderer::for_layer`] on its own thread.
    ///
    /// # Safety
    /// As for `Renderer::for_layer`.
    pub unsafe fn for_layer(
        metal_layer: *mut c_void,
        width_px: u32,
        height_px: u32,
        scale: f32,
    ) -> Result<LiveRenderer, RenderError> {
        // SAFETY: passed on from the caller.
        Self::new(unsafe { Renderer::for_layer(metal_layer, width_px, height_px, scale) }?)
    }

    /// [`LiveRenderer::for_layer`], with each drawable also copied, so [`read_pixels`](Self::read_pixels) can see what really reached
    /// the layer. Makes the drawables slower: this is for tests.
    ///
    /// # Safety
    /// As for `Renderer::for_layer`.
    #[doc(hidden)]
    pub unsafe fn for_layer_readable(metal_layer: *mut c_void, width_px: u32, height_px: u32, scale: f32) -> Result<LiveRenderer, RenderError> {
        // SAFETY: passed on from the caller.
        Self::new(unsafe { Renderer::for_layer_readable(metal_layer, width_px, height_px, scale) }?)
    }

    /// [`Renderer::headless`] on its own thread.
    pub fn headless(width: u32, height: u32, scale: f32) -> Result<LiveRenderer, RenderError> {
        Self::new(Renderer::headless(width, height, scale)?)
    }

    /// Makes `colors` the newest frame. Copies them and never blocks or waits on the GPU, however far behind the
    /// render thread is: a frame it didn't get to is replaced and counted in
    /// [`LiveStats::frames_dropped`]. See [`Renderer::set_light_colors`] for the format.
    pub fn publish_colors(&self, colors: &[u32]) {
        self.mailbox.publish(colors);
        self.waker.unpark();
    }

    /// See [`Renderer::set_lights`].
    pub fn set_lights(&self, lights: &[LightGeom]) {
        self.send(Command::SetLights(lights.to_vec()));
    }

    /// See [`Renderer::update_light_range`].
    pub fn update_light_range(&self, start: usize, lights: &[LightGeom]) {
        self.send(Command::UpdateLightRange { start, lights: lights.to_vec() });
    }

    /// [`update_light_range`](Self::update_light_range) taking a list the caller has already built, so a drag copies its
    /// lights once.
    pub fn update_light_range_vec(&self, start: usize, lights: Vec<LightGeom>) {
        self.send(Command::UpdateLightRange { start, lights });
    }

    /// See [`Renderer::set_background`].
    pub fn set_background(&self, rgba: &[u8], w: u32, h: u32) {
        self.send(Command::SetBackground { rgba: rgba.to_vec(), w, h });
    }

    /// See [`Renderer::set_background_alpha`].
    pub fn set_background_alpha(&self, alpha: u8) {
        self.send(Command::SetBackgroundAlpha(alpha));
    }

    /// See [`Renderer::clear_background`].
    pub fn clear_background(&self) {
        self.send(Command::ClearBackground);
    }

    /// See [`Renderer::set_overlay`].
    pub fn set_overlay(&self, overlay: Overlay) {
        self.send(Command::SetOverlay(overlay));
    }

    pub fn set_moving_heads(&self, heads: Vec<MovingHeadInstance>) {
        self.send(Command::SetMovingHeads(heads));
    }

    /// Perspective distance; 0 returns to the flat canvas.
    pub fn set_camera_depth(&self, depth: f32) {
        self.send(Command::SetCameraDepth(depth));
    }

    pub fn set_fixture_reference(&self, width: f32, height: f32) {
        self.send(Command::SetFixtureReference([width,height]));
    }

    /// See [`Renderer::set_view`].
    pub fn set_view(&self, zoom: f32, pan_x: f32, pan_y: f32) {
        self.send(Command::SetView { zoom, pan_x, pan_y });
    }

    /// See [`Renderer::set_light_scale`].
    pub fn set_light_scale(&self, scale: f32) {
        self.send(Command::SetLightScale(scale));
    }

    /// See [`Renderer::set_transparent`].
    pub fn set_transparent(&self, on: bool) {
        self.send(Command::SetTransparent(on));
    }

    /// See [`Renderer::resize`].
    pub fn resize(&self, width_px: u32, height_px: u32, scale: f32) {
        self.send(Command::Resize { width_px, height_px, scale });
    }

    /// Draws a frame even if nothing changed. Worth calling when the window comes back on screen.
    pub fn request_redraw(&self) {
        self.send(Command::Redraw);
    }

    /// Everything so far, drawn and read back, as [`Renderer::read_pixels`]. This one does wait, for the thread
    /// and the GPU, so it is for tests and exporting images and must not be called on the UI thread.
    pub fn read_pixels(&self) -> Vec<u8> {
        let (reply, pixels) = mpsc::channel();
        self.send(Command::ReadPixels(reply));
        pixels.recv().unwrap_or_default()
    }

    /// The time between each of the last few hundred frames and the one before, oldest first. Includes the times nothing
    /// changed, so whoever reads this looks at a stretch when something was. For measuring pacing.
    pub fn recent_frame_gaps(&self) -> Vec<Duration> {
        let rendered = self.shared.frames_rendered.load(Ordering::Relaxed) as usize;
        let count = rendered.saturating_sub(1).min(FRAME_GAP_HISTORY);
        (0..count)
            .map(|i| {
                let slot = (rendered - count + i) % FRAME_GAP_HISTORY;
                Duration::from_micros(self.shared.frame_gaps_micros[slot].load(Ordering::Relaxed) as u64)
            })
            .collect()
    }

    /// Current counters. Lock-free apart from a brief lock for the error text, which only a failure contends.
    pub fn stats(&self) -> LiveStats {
        LiveStats {
            frames_rendered: self.shared.frames_rendered.load(Ordering::Relaxed),
            last_frame_cpu: Duration::from_nanos(self.shared.last_frame_cpu_nanos.load(Ordering::Relaxed)),
            frames_dropped: self.mailbox.dropped(),
            last_error: self.shared.last_error.lock().clone(),
            frames_occluded: self.shared.frames_occluded.load(Ordering::Relaxed),
            frames_timed_out: self.shared.frames_timed_out.load(Ordering::Relaxed),
            colors_taken: self.shared.colors_taken.load(Ordering::Relaxed),
        }
    }

    fn send(&self, command: Command) {
        // Fails only if the thread has gone, and then there is nothing left to tell.
        let _ = self.commands.send(command);
        self.waker.unpark();
    }
}

impl Drop for LiveRenderer {
    /// Waits for the thread. That takes at most about a second, the longest `nextDrawable` can block.
    fn drop(&mut self) {
        self.send(Command::Quit);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Everything the thread has to do with its queue of commands.
struct CommandQueue {
    receiver: Receiver<Command>,
    readers: Vec<Sender<Vec<u8>>>,
    /// A resize that arrived with a drawable in hand, which is already the old size.
    deferred_resize: Option<(u32, u32, f32)>,
}

impl CommandQueue {
    /// Applies everything waiting. False means it's time to stop.
    fn drain(&mut self, renderer: &mut Renderer, holding_drawable: bool) -> bool {
        if !holding_drawable {
            if let Some((width_px, height_px, scale)) = self.deferred_resize.take() {
                renderer.resize(width_px, height_px, scale);
            }
        }
        loop {
            match self.receiver.try_recv() {
                Ok(Command::Quit) | Err(TryRecvError::Disconnected) => return false,
                Ok(Command::ReadPixels(reply)) => self.readers.push(reply),
                Ok(Command::Resize { width_px, height_px, scale }) if holding_drawable => {
                    self.deferred_resize = Some((width_px, height_px, scale));
                }
                Ok(command) => apply(renderer, command),
                Err(TryRecvError::Empty) => return true,
            }
        }
    }
}

fn run(mut renderer: Renderer, receiver: Receiver<Command>, mut colors: MailboxReader, shared: Arc<Shared>) {
    // SAFETY: only affects the calling thread. On failure it just runs at the default class.
    unsafe { libc::pthread_set_qos_class_self_np(libc::qos_class_t::QOS_CLASS_USER_INTERACTIVE, 0) };

    let mut commands = CommandQueue { receiver, readers: Vec::new(), deferred_resize: None };
    let mut retry_at = None;
    let mut undrawn_frame = false;
    let mut last_frame_at: Option<Instant> = None;
    loop {
        let pool = AutoreleasePool::new();

        if !commands.drain(&mut renderer, false) {
            return;
        }

        // A publish wakes this thread even when the layer can't take a frame yet, and there is nothing to do
        // about it until then, not even copying colours that will be replaced.
        if let Some(at) = retry_at {
            let now = Instant::now();
            if now < at && commands.readers.is_empty() {
                thread::park_timeout(at - now);
                continue;
            }
            retry_at = None;
        }

        let cpu_start = thread_cpu_time();
        if take_newest_colors(&mut colors, &mut renderer, &mut undrawn_frame) {
            shared.colors_taken.fetch_add(1, Ordering::Relaxed);
        }
        if renderer.needs_render() {
            match renderer.acquire() {
                Ok(Acquired::Frame(frame)) => {
                    // Getting the drawable can take most of a frame. Draw the newest of everything now, so a
                    // drag or a pan that arrived during the wait shows in this frame and not the next.
                    if !commands.drain(&mut renderer, true) {
                        return;
                    }
                    if take_newest_colors(&mut colors, &mut renderer, &mut undrawn_frame) {
                        shared.colors_taken.fetch_add(1, Ordering::Relaxed);
                    }
                    renderer.draw(frame);
                    undrawn_frame = false;
                    let cpu = thread_cpu_time().saturating_sub(cpu_start);
                    let now = Instant::now();
                    if let Some(last) = last_frame_at.replace(now) {
                        let frame = shared.frames_rendered.load(Ordering::Relaxed) as usize;
                        let gap = now.duration_since(last).as_micros().min(u32::MAX as u128) as u32;
                        shared.frame_gaps_micros[frame % FRAME_GAP_HISTORY].store(gap, Ordering::Relaxed);
                    }
                    shared.frames_rendered.fetch_add(1, Ordering::Relaxed);
                    shared.last_frame_cpu_nanos.store(cpu.as_nanos() as u64, Ordering::Relaxed);
                }
                // Parks below until a resize, which is a command and so wakes this thread.
                Ok(Acquired::Skipped(FrameSkip::ZeroSize)) => {}
                Ok(Acquired::Skipped(skip @ (FrameSkip::Occluded | FrameSkip::Timeout))) => {
                    let counter = if skip == FrameSkip::Occluded { &shared.frames_occluded } else { &shared.frames_timed_out };
                    counter.fetch_add(1, Ordering::Relaxed);
                    retry_at = Some(Instant::now() + SKIPPED_FRAME_RETRY);
                }
                Err(error) => {
                    *shared.last_error.lock() = Some(error.to_string());
                    retry_at = Some(Instant::now() + FAILED_FRAME_RETRY);
                }
            }
        }
        if let Some(error) = renderer.take_error() {
            *shared.last_error.lock() = Some(error.to_string());
        }

        // A deferred resize came before these reads, and goes round again first, with nothing needed to wake it.
        if commands.deferred_resize.is_some() {
            continue;
        }

        for reply in commands.readers.drain(..) {
            let _ = reply.send(renderer.read_pixels());
        }

        // So the last frame's objects aren't held for as long as it stays idle.
        drop(pool);

        // With the wake-up token, a change that arrived since the checks above returns from this immediately.
        if retry_at.is_none() {
            thread::park();
        }
    }
}

/// True if there was a frame to take.
fn take_newest_colors(colors: &mut MailboxReader, renderer: &mut Renderer, undrawn_frame: &mut bool) -> bool {
    if let Some(frame) = colors.take() {
        renderer.set_light_colors(frame);
        if std::mem::replace(undrawn_frame, true) {
            colors.count_dropped();
        }
        true
    } else {
        false
    }
}

fn apply(renderer: &mut Renderer, command: Command) {
    match command {
        Command::SetBackground { rgba, w, h } => renderer.set_background(&rgba, w, h),
        Command::SetBackgroundAlpha(alpha) => renderer.set_background_alpha(alpha),
        Command::ClearBackground => renderer.clear_background(),
        Command::SetLights(lights) => renderer.set_lights(&lights),
        Command::UpdateLightRange { start, lights } => renderer.update_light_range(start, &lights),
        Command::SetOverlay(overlay) => renderer.set_overlay(overlay),
        Command::SetMovingHeads(heads) => renderer.set_moving_heads(heads),
        Command::SetCameraDepth(depth) => renderer.set_camera_depth(depth),
        Command::SetFixtureReference(size) => renderer.set_fixture_reference(size[0],size[1]),
        Command::SetLightScale(scale) => renderer.set_light_scale(scale),
        Command::SetTransparent(on) => renderer.set_transparent(on),
        Command::SetView { zoom, pan_x, pan_y } => renderer.set_view(zoom, pan_x, pan_y),
        Command::Resize { width_px, height_px, scale } => renderer.resize(width_px, height_px, scale),
        Command::Redraw => renderer.request_redraw(),
        Command::ReadPixels(_) | Command::Quit => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::thread::JoinHandleExt;

    #[test]
    fn the_render_thread_runs_at_user_interactive() {
        let live = match LiveRenderer::headless(8, 8, 1.0) {
            Ok(live) => live,
            Err(RenderError::NoAdapter) if std::env::var_os("VIXEN_RENDER_REQUIRE_GPU").is_none() => return,
            Err(error) => panic!("{error}"),
        };
        // Answering this means the thread is past setting its class.
        live.read_pixels();

        let mut class = libc::qos_class_t::QOS_CLASS_UNSPECIFIED;
        let mut priority = 0;
        let pthread = live.thread.as_ref().unwrap().as_pthread_t();
        // SAFETY: the thread is alive, because `live` is, and both out-pointers are valid.
        assert_eq!(unsafe { libc::pthread_get_qos_class_np(pthread, &mut class, &mut priority) }, 0);
        assert_eq!(class as u32, libc::qos_class_t::QOS_CLASS_USER_INTERACTIVE as u32);
    }
}
