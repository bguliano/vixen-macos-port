use std::borrow::Cow;
use std::collections::VecDeque;
use std::ffi::c_void;
use std::mem::size_of;
use std::num::NonZeroU64;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytemuck::{Pod, Zeroable};
use parking_lot::Mutex;

use crate::colors::ColorShadow;
use crate::objc::{self, AutoreleasePool};
use crate::{FrameSkip, FrameStats, LightGeom, MovingHeadInstance, Overlay, RenderError};
use crate::fixture::{moving_head_mesh, FixtureVertex};

/// The same format for the layer and for offscreen, so tests exercise the pipelines that get presented, and
/// not sRGB, so blending stays in encoded values.
const TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;
const BACKGROUND_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// Offscreen has no display to pace it, so this keeps a tight loop from queueing GPU work without limit.
const MAX_OFFSCREEN_FRAMES_IN_FLIGHT: usize = 2;
const GPU_WAIT_LIMIT: Duration = Duration::from_secs(5);

/// Three drawables, `CAMetalLayer`'s default and the most it allows. Measured on a 144 Hz display with the machine
/// busy, two stalled the render thread outright every few runs, for two seconds in the worst one, where three kept
/// going. The extra drawable costs a frame of latency only while frames outrun the display.
const LAYER_FRAME_LATENCY: u32 = 2;

const BYTES_PER_PIXEL: u32 = 4;

/// Reused upload memory. Larger uploads get a chunk of their own, which is reused the same way.
const STAGING_CHUNK_BYTES: u64 = 1 << 20;

/// Must match `View` in shaders.wgsl.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct ViewUniform {
    pan: [f32; 2],
    viewport: [f32; 2],
    bg_size: [f32; 2],
    zoom: f32,
    scale: f32,
    bg_dim: f32,
    /// 0 for the flat rule. Otherwise lights are `gl_PointSize` points scaled by this.
    light_scale: f32,
    /// 1 when the window shows through unlit areas, and a light is opaque, with its colour already over black.
    opaque_lights: f32,
    camera_depth: f32,
}

const KIND_LINE: u32 = 0;
const KIND_SQUARE: u32 = 1;
const KIND_CIRCLE: u32 = 2;

/// Lines, squares and circles share one instance layout, so the overlay is one draw call.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct OverlayInstance {
    p0: [f32; 2],
    p1: [f32; 2],
    color: u32,
    param: f32,
    kind: u32,
}

/// A GPU buffer that only ever grows. Changes between frames only ever pay for the data, not for a buffer.
struct GrowBuffer {
    buffer: wgpu::Buffer,
    label: &'static str,
}

impl GrowBuffer {
    fn new(device: &wgpu::Device, label: &'static str) -> GrowBuffer {
        GrowBuffer { buffer: instance_buffer(device, label, 4096), label }
    }

    /// True if the buffer had to be replaced, which loses its contents.
    fn ensure(&mut self, device: &wgpu::Device, bytes: u64) -> bool {
        if bytes <= self.buffer.size() {
            return false;
        }
        let size = bytes.next_power_of_two().min(device.limits().max_buffer_size).max(bytes);
        self.buffer = instance_buffer(device, self.label, size);
        true
    }
}

fn instance_buffer(device: &wgpu::Device, label: &'static str, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

struct Offscreen {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

struct LayerTarget {
    surface: wgpu::Surface<'static>,
    layer: LayerPtr,
    usage: wgpu::TextureUsages,
    needs_configure: bool,
    transparent: bool,
    /// The last drawable's pixels, for tests that verify what really reached a drawable.
    capture: Option<Capture>,
}

/// The surface holds a retained reference to the layer for as long as this exists.
struct LayerPtr(*mut c_void);
// SAFETY: only used to call property setters that `wgpu::Surface::configure` also calls from this thread.
unsafe impl Send for LayerPtr {}

struct Capture {
    buffer: wgpu::Buffer,
    width: u32,
    height: u32,
    has_frame: bool,
}

enum Target {
    /// `None` while the size is zero.
    Offscreen(Option<Offscreen>),
    Layer(Box<LayerTarget>),
}

struct Background {
    bind_group: wgpu::BindGroup,
}

// Only ever lives for one call, so boxing the frame would just add an allocation to every frame.
#[allow(clippy::large_enum_variant)]
pub(crate) enum Acquired {
    Frame(Frame),
    Skipped(FrameSkip),
}

pub(crate) struct Frame {
    view: wgpu::TextureView,
    texture: wgpu::Texture,
    drawable: Option<wgpu::SurfaceTexture>,
}

pub(crate) fn thread_cpu_time() -> Duration {
    let mut time = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    // SAFETY: `time` is a valid timespec, and this clock id is always supported on Apple platforms.
    unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut time) };
    Duration::new(time.tv_sec as u64, time.tv_nsec as u32)
}

fn write_staged(
    belt: &mut wgpu::util::StagingBelt,
    encoder: &mut wgpu::CommandEncoder,
    target: &wgpu::Buffer,
    offset: u64,
    bytes: &[u8],
) {
    if let Some(size) = NonZeroU64::new(bytes.len() as u64) {
        belt.write_buffer(encoder, target, offset, size).copy_from_slice(bytes);
    }
}

/// Draws lights, a background and editor graphics. See the [crate documentation](crate) for the conventions.
///
/// Setters take effect at the next [`render`](Renderer::render) and cost little until then, so calling one
/// several times between frames only uploads the last value.
pub struct Renderer {
    adapter_name: String,
    device: wgpu::Device,
    queue: wgpu::Queue,
    target: Target,
    width: u32,
    height: u32,
    scale: f32,

    background_pipeline: wgpu::RenderPipeline,
    mipmap_pipeline: wgpu::RenderPipeline,
    light_pipeline: wgpu::RenderPipeline,
    overlay_pipeline: wgpu::RenderPipeline,
    fixture_pipeline: wgpu::RenderPipeline,
    depth_view: Option<wgpu::TextureView>,
    view_buffer: wgpu::Buffer,
    view_bind_group: wgpu::BindGroup,
    background_layout: wgpu::BindGroupLayout,
    background_sampler: wgpu::Sampler,
    belt: wgpu::util::StagingBelt,
    in_flight: VecDeque<wgpu::SubmissionIndex>,
    max_lights: usize,
    max_texture_size: u32,

    geometry: GrowBuffer,
    color_buffer: GrowBuffer,
    overlay_buffer: GrowBuffer,
    fixture_buffer: GrowBuffer,
    fixtures: Vec<MovingHeadInstance>,
    fixture_vertices: Vec<FixtureVertex>,
    fixtures_dirty: bool,
    fixture_count: u32,
    camera_depth: f32,
    fixture_reference: [f32; 2],

    light_count: u32,
    colors: ColorShadow,
    overlay: Vec<OverlayInstance>,
    overlay_count: u32,
    background: Option<Background>,
    background_size: [f32; 2],
    background_alpha: u8,
    light_scale: f32,
    transparent: bool,
    zoom: f32,
    pan: [f32; 2],

    view_dirty: bool,
    overlay_dirty: bool,
    /// Something changed that needs a new frame without needing an upload here.
    frame_dirty: bool,

    errors: Arc<Mutex<Option<RenderError>>>,
}

const _: () = {
    const fn assert_send<T: Send>() {}
    assert_send::<Renderer>();
};

impl Renderer {
    /// Renders into a `CAMetalLayer` that Swift owns and keeps on screen. Sizes are physical pixels.
    ///
    /// Hosting the layer and setting its `contentsScale` stay with Swift. The renderer sets its device, pixel
    /// format, drawable size and colour space.
    ///
    /// Before each frame wgpu walks up from the layer to the first layer with a delegate, and sends that delegate
    /// `window` and the window `occlusionState`, from the thread rendering. So that delegate must be an `NSView`,
    /// or anything else that answers `window`, or the message raises an exception. Those two messages are off the
    /// main thread by wgpu's design, so Xcode's Main Thread Checker may flag them.
    ///
    /// # Safety
    /// `metal_layer` must be null or point to a live Objective-C object. wgpu retains the layer, so it may be
    /// released on the Swift side while this renderer exists.
    pub unsafe fn for_layer(
        metal_layer: *mut c_void,
        width_px: u32,
        height_px: u32,
        scale: f32,
    ) -> Result<Renderer, RenderError> {
        // SAFETY: passed on from the caller.
        unsafe { Self::new_for_layer(metal_layer, width_px, height_px, scale, wgpu::TextureUsages::empty()) }
    }

    /// [`for_layer`](Renderer::for_layer), with each drawable also copied so that
    /// [`read_pixels`](Renderer::read_pixels) can verify it. Makes the drawables slower: this is for tests.
    ///
    /// # Safety
    /// As for `for_layer`.
    #[doc(hidden)]
    pub unsafe fn for_layer_readable(
        metal_layer: *mut c_void,
        width_px: u32,
        height_px: u32,
        scale: f32,
    ) -> Result<Renderer, RenderError> {
        // SAFETY: passed on from the caller.
        unsafe { Self::new_for_layer(metal_layer, width_px, height_px, scale, wgpu::TextureUsages::COPY_SRC) }
    }

    unsafe fn new_for_layer(
        metal_layer: *mut c_void,
        width_px: u32,
        height_px: u32,
        scale: f32,
        extra_usage: wgpu::TextureUsages,
    ) -> Result<Renderer, RenderError> {
        if metal_layer.is_null() {
            return Err(RenderError::NullLayer);
        }
        // SAFETY: the caller promises a live object.
        if !unsafe { objc::is_metal_layer(metal_layer) } {
            return Err(RenderError::NotAMetalLayer);
        }

        let instance = new_instance();
        // SAFETY: checked above to be a CAMetalLayer, and the surface retains it.
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(metal_layer))
        }
        .map_err(|error| RenderError::Surface(error.to_string()))?;

        let (adapter, device, queue) = request_gpu(&instance, Some(&surface))?;
        if !surface.get_capabilities(&adapter).formats.contains(&TARGET_FORMAT) {
            return Err(RenderError::Surface(format!("{TARGET_FORMAT:?} is not offered by this layer")));
        }

        let target = Target::Layer(Box::new(LayerTarget {
            surface,
            layer: LayerPtr(metal_layer),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | extra_usage,
            needs_configure: true,
            transparent: false,
            capture: None,
        }));
        Ok(Self::assemble(adapter, device, queue, target, width_px, height_px, scale))
    }

    /// Renders to an offscreen texture, for tests and for exporting images. Sizes are physical pixels.
    pub fn headless(width: u32, height: u32, scale: f32) -> Result<Renderer, RenderError> {
        let instance = new_instance();
        let (adapter, device, queue) = request_gpu(&instance, None)?;
        Ok(Self::assemble(adapter, device, queue, Target::Offscreen(None), width, height, scale))
    }

    fn assemble(
        adapter: wgpu::Adapter,
        device: wgpu::Device,
        queue: wgpu::Queue,
        target: Target,
        width: u32,
        height: u32,
        scale: f32,
    ) -> Renderer {
        let errors = Arc::new(Mutex::new(None));
        {
            let errors = errors.clone();
            device.on_uncaptured_error(Arc::new(move |error: wgpu::Error| {
                errors.lock().get_or_insert_with(|| RenderError::Gpu(error.to_string()));
            }));
        }
        {
            let errors = errors.clone();
            device.set_device_lost_callback(move |reason, message| {
                if reason != wgpu::DeviceLostReason::Destroyed {
                    errors.lock().get_or_insert_with(|| RenderError::Gpu(format!("device lost: {message}")));
                }
            });
        }

        let view_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("view"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(size_of::<ViewUniform>() as u64),
                },
                count: None,
            }],
        });
        let background_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("background"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let view_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("view"),
            size: size_of::<ViewUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let view_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("view"),
            layout: &view_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: view_buffer.as_entire_binding() }],
        });

        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders.wgsl"));
        let scene_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene"),
            bind_group_layouts: &[Some(&view_layout)],
            immediate_size: 0,
        });
        let textured_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("background"),
            bind_group_layouts: &[Some(&view_layout), Some(&background_layout)],
            immediate_size: 0,
        });

        // The mipmap pass binds only the image, in the group the shader declares it in.
        let mipmap_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mipmap"),
            bind_group_layouts: &[None, Some(&background_layout)],
            immediate_size: 0,
        });

        let pipeline = |label, layout, vertex: &str, fragment: &str, format, buffers: &[Option<wgpu::VertexBufferLayout>], blend| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(vertex),
                    compilation_options: Default::default(),
                    buffers,
                },
                primitive: wgpu::PrimitiveState {
                    topology: if label == "fixtures" { wgpu::PrimitiveTopology::TriangleList } else { wgpu::PrimitiveTopology::TriangleStrip },
                    ..Default::default()
                },
                depth_stencil: (label != "mipmap").then_some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(label == "fixtures" || label == "lights"),
                    depth_compare: Some(if label == "fixtures" || label == "lights" { wgpu::CompareFunction::LessEqual } else { wgpu::CompareFunction::Always }),
                    stencil: Default::default(), bias: Default::default(),
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fragment),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };

        let background_pipeline =
            pipeline("background", &textured_layout, "background_vs", "background_fs", TARGET_FORMAT, &[], None);
        let mipmap_pipeline =
            pipeline("mipmap", &mipmap_layout, "mipmap_vs", "mipmap_fs", BACKGROUND_FORMAT, &[], None);
        let light_pipeline = pipeline(
            "lights",
            &scene_layout,
            "light_vs",
            "light_fs",
            TARGET_FORMAT,
            &[
                Some(wgpu::VertexBufferLayout {
                    array_stride: size_of::<LightGeom>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3],
                }),
                Some(wgpu::VertexBufferLayout {
                    array_stride: size_of::<u32>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![1 => Unorm8x4],
                }),
            ],
            Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
        );
        let overlay_pipeline = pipeline(
            "overlay",
            &scene_layout,
            "overlay_vs",
            "overlay_fs",
            TARGET_FORMAT,
            &[Some(wgpu::VertexBufferLayout {
                array_stride: size_of::<OverlayInstance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![
                    0 => Float32x2, 1 => Float32x2, 2 => Unorm8x4, 3 => Float32, 4 => Uint32
                ],
            })],
            Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
        );

        let fixture_pipeline = pipeline(
            "fixtures", &scene_layout, "fixture_vs", "fixture_fs", TARGET_FORMAT,
            &[Some(wgpu::VertexBufferLayout {
                array_stride: size_of::<FixtureVertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32, 4 => Float32x4, 5 => Float32],
            })], Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
        );

        let background_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("background"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        let limits = device.limits();
        let max_lights = (limits.max_buffer_size / size_of::<LightGeom>() as u64).min(u32::MAX as u64) as usize;

        let mut renderer = Renderer {
            adapter_name: adapter.get_info().name,
            belt: wgpu::util::StagingBelt::new(device.clone(), STAGING_CHUNK_BYTES),
            geometry: GrowBuffer::new(&device, "light geometry"),
            color_buffer: GrowBuffer::new(&device, "light colors"),
            overlay_buffer: GrowBuffer::new(&device, "overlay"),
            fixture_buffer: GrowBuffer::new(&device, "fixtures"),
            fixtures: Vec::new(), fixture_vertices: Vec::new(), fixtures_dirty: false, fixture_count: 0,
            camera_depth: 0.0, fixture_reference: [0.0, 0.0], depth_view: None, fixture_pipeline,
            device,
            queue,
            target,
            width: 0,
            height: 0,
            scale: 1.0,
            background_pipeline,
            mipmap_pipeline,
            light_pipeline,
            overlay_pipeline,
            view_buffer,
            view_bind_group,
            background_layout,
            background_sampler,
            in_flight: VecDeque::new(),
            max_lights,
            max_texture_size: limits.max_texture_dimension_2d,
            light_count: 0,
            colors: ColorShadow::default(),
            overlay: Vec::new(),
            overlay_count: 0,
            background: None,
            background_size: [0.0, 0.0],
            background_alpha: 255,
            light_scale: 0.0,
            transparent: false,
            zoom: 1.0,
            pan: [0.0, 0.0],
            view_dirty: true,
            overlay_dirty: false,
            frame_dirty: true,
            errors,
        };
        renderer.resize(width, height, scale);
        renderer
    }

    /// The GPU in use.
    pub fn adapter_name(&self) -> &str {
        &self.adapter_name
    }

    /// Sets the target size in physical pixels, and physical pixels per logical pixel. Either size may be zero,
    /// in which case [`render`](Renderer::render) skips until there is something to draw on.
    pub fn resize(&mut self, width_px: u32, height_px: u32, scale: f32) {
        let width = width_px.min(self.max_texture_size);
        let height = height_px.min(self.max_texture_size);
        let scale = if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
        let size_changed = width != self.width || height != self.height;
        if size_changed || scale != self.scale { self.fixtures_dirty = !self.fixtures.is_empty(); }
        if !size_changed && scale == self.scale {
            return;
        }
        (self.width, self.height, self.scale) = (width, height, scale);
        self.view_dirty = true;
        self.frame_dirty = true;
        if !size_changed {
            return;
        }

        self.depth_view = (width > 0 && height > 0).then(|| self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("preview depth"), size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[],
        }).create_view(&Default::default()));

        match &mut self.target {
            Target::Offscreen(offscreen) => {
                *offscreen = (width > 0 && height > 0).then(|| {
                    let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("offscreen target"),
                        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: TARGET_FORMAT,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                        view_formats: &[],
                    });
                    let view = texture.create_view(&Default::default());
                    Offscreen { texture, view }
                });
            }
            Target::Layer(layer) => layer.needs_configure = true,
        }
    }

    /// Sets the image, `w * h * 4` bytes of straight-alpha RGBA, top row first, with its corner at scene (0, 0).
    /// One image pixel is one scene unit. Transparent parts show black, as in the original.
    ///
    /// Empty images clear the background. A buffer of the wrong length also clears it, and `render` reports
    /// [`RenderError::InvalidInput`].
    pub fn set_background(&mut self, rgba: &[u8], w: u32, h: u32) {
        let expected = (w as usize).checked_mul(h as usize).and_then(|pixels| pixels.checked_mul(4));
        if expected != Some(rgba.len()) {
            self.report(RenderError::InvalidInput(format!(
                "set_background: {} bytes is not a {w} x {h} RGBA image",
                rgba.len()
            )));
            self.clear_background();
            return;
        }
        if w == 0 || h == 0 {
            self.clear_background();
            return;
        }

        // One past the GPU's limit costs some detail rather than the whole image.
        let shrink = w.max(h).div_ceil(self.max_texture_size).max(1);
        let (width, height) = (w / shrink, h / shrink);
        let image = shrink_nearest(rgba, w, h, shrink);
        let image = composite_over_black(image);

        let mip_level_count = u32::BITS - width.max(height).leading_zeros();
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("background"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: BACKGROUND_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        self.queue.write_texture(
            texture.as_image_copy(),
            &image,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * BYTES_PER_PIXEL),
                rows_per_image: None,
            },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        self.generate_mipmaps(&texture, mip_level_count);

        let view = texture.create_view(&Default::default());
        self.background = Some(Background { bind_group: self.background_bind_group(&view) });
        self.background_size = [w as f32, h as f32];
        self.view_dirty = true;
    }

    fn background_bind_group(&self, view: &wgpu::TextureView) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("background"),
            layout: &self.background_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view) },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.background_sampler),
                },
            ],
        })
    }

    fn generate_mipmaps(&self, texture: &wgpu::Texture, levels: u32) {
        let level_view = |level| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                base_mip_level: level,
                mip_level_count: Some(1),
                ..Default::default()
            })
        };
        let mut encoder = self.device.create_command_encoder(&Default::default());
        for level in 1..levels {
            let source = self.background_bind_group(&level_view(level - 1));
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("mipmap"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &level_view(level),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.mipmap_pipeline);
            pass.set_bind_group(1, &source, &[]);
            pass.draw(0..4, 0..1);
        }
        self.queue.submit([encoder.finish()]);
    }

    /// How far the image shows: 255 is the image as it is, 0 is black. `BackgroundAlpha` in the original, which
    /// dims the image over black (`GDIControl.CreateAlphaBackground`). This costs nothing, where the original
    /// rebuilds a bitmap.
    pub fn set_background_alpha(&mut self, alpha: u8) {
        if alpha != self.background_alpha {
            self.background_alpha = alpha;
            self.view_dirty = true;
        }
    }

    pub fn clear_background(&mut self) {
        if self.background.take().is_some() {
            self.frame_dirty = true;
        }
    }

    /// Replaces every light, keeping the colours of those that still exist. New lights are transparent until
    /// [`set_light_colors`](Renderer::set_light_colors) gives them a colour. For one shape moving, use
    /// [`update_light_range`](Renderer::update_light_range).
    pub fn set_lights(&mut self, lights: &[LightGeom]) {
        let lights = &lights[..lights.len().min(self.max_lights)];
        let old_count = self.light_count as usize;
        let new_count = lights.len();

        self.geometry.ensure(&self.device, size_of_val(lights) as u64);
        if !lights.is_empty() {
            self.queue.write_buffer(&self.geometry.buffer, 0, bytemuck::cast_slice(lights));
        }

        self.colors.ensure_len(new_count);
        if self.color_buffer.ensure(&self.device, (new_count * size_of::<u32>()) as u64) {
            self.colors.mark(0..new_count);
        } else if new_count > old_count {
            self.colors.mark(old_count..new_count);
        }

        self.light_count = new_count as u32;
        self.frame_dirty = true;
    }

    /// Replaces lights `start..start + lights.len()`. Anything past the last light is ignored.
    pub fn update_light_range(&mut self, start: usize, lights: &[LightGeom]) {
        let count = self.light_count as usize;
        if start >= count {
            return;
        }
        let lights = &lights[..lights.len().min(count - start)];
        if !lights.is_empty() {
            let offset = (start * size_of::<LightGeom>()) as u64;
            self.queue.write_buffer(&self.geometry.buffer, offset, bytemuck::cast_slice(lights));
            self.frame_dirty = true;
        }
    }

    /// Sets every light's colour: RGBA8, straight alpha, [`rgba`](crate::rgba). `alpha == 0` draws nothing.
    /// Lights past the end of a short array go transparent.
    ///
    /// This is the per-frame path. Only colours that differ from the last call reach the GPU, so unchanged
    /// lights cost a compare and changed lights cost a copy.
    pub fn set_light_colors(&mut self, colors: &[u32]) {
        self.colors.set_all(colors);
    }

    /// Reuses the common fixture geometry for live previews and fixture editors.
    pub fn set_moving_heads(&mut self, heads: Vec<MovingHeadInstance>) {
        if heads != self.fixtures {
            self.fixtures = heads;
            self.fixtures_dirty = true;
        }
    }

    pub fn set_camera_depth(&mut self, depth: f32) {
        let depth = if depth.is_finite() { depth.max(0.0) } else { 0.0 };
        if self.camera_depth != depth { self.camera_depth = depth; self.view_dirty = true; }
    }

    /// Background dimensions, or the original form dimensions when there is no picture. A negative
    /// `max_beam_length` on an instance asks the renderer to derive its beam length from this reference.
    pub fn set_fixture_reference(&mut self, width: f32, height: f32) {
        let size = [width.max(1.0),height.max(1.0)];
        if size != self.fixture_reference { self.fixture_reference = size; self.fixtures_dirty = !self.fixtures.is_empty(); }
    }

    /// Replaces the editor graphics. Drawn above the lights at their own width, whatever the zoom.
    pub fn set_overlay(&mut self, overlay: Overlay) {
        self.overlay.clear();
        self.overlay.reserve(overlay.lines.len() + overlay.squares.len() + overlay.circles.len());
        self.overlay.extend(overlay.lines.iter().map(|line| OverlayInstance {
            p0: line.a,
            p1: line.b,
            color: line.color,
            param: line.width,
            kind: KIND_LINE,
        }));
        self.overlay.extend(overlay.squares.iter().map(|square| OverlayInstance {
            p0: square.center,
            p1: square.center,
            color: square.color,
            param: square.size,
            kind: KIND_SQUARE,
        }));
        self.overlay.extend(overlay.circles.iter().map(|&(center, radius, color)| OverlayInstance {
            p0: center,
            p1: center,
            color,
            param: radius,
            kind: KIND_CIRCLE,
        }));
        self.overlay_dirty = true;
    }

    /// Scene to pixels: `(scene * zoom + pan) * scale`, with `pan` in logical pixels. Lights keep their size at
    /// any zoom. Values that aren't finite are ignored.
    pub fn set_view(&mut self, zoom: f32, pan_x: f32, pan_y: f32) {
        if !(zoom.is_finite() && pan_x.is_finite() && pan_y.is_finite()) {
            return;
        }
        if zoom != self.zoom || [pan_x, pan_y] != self.pan {
            self.zoom = zoom;
            self.pan = [pan_x, pan_y];
            self.view_dirty = true;
        }
    }

    /// How big lights are, for the camera view. 0 is the design surface's and the flat view's rule, `FastPixel.DrawCircle`
    /// at any zoom. Above 0 a light is an OpenGL point: `size * scale` across, or 1 when `size` is under 1.
    pub fn set_light_scale(&mut self, scale: f32) {
        let scale = if scale.is_finite() && scale > 0.0 { scale } else { 0.0 };
        if scale != self.light_scale {
            self.light_scale = scale;
            self.view_dirty = true;
        }
    }

    /// Whether what's behind the window shows through unlit areas: clears to no alpha and skips the background. A light is
    /// opaque however dim, with its colour over black, as it is upstream over its near-black colour key.
    pub fn set_transparent(&mut self, on: bool) {
        if on == self.transparent {
            return;
        }
        self.transparent = on;
        self.view_dirty = true;
        self.frame_dirty = true;
        if let Target::Layer(layer) = &mut self.target {
            layer.transparent = on;
            layer.needs_configure = true;
        }
    }

    /// Draws a frame and, for a layer, presents it. Waits for a free drawable first, which is what paces it to
    /// the display; that wait doesn't count in [`FrameStats::cpu_time`].
    pub fn render(&mut self) -> Result<FrameStats, RenderError> {
        let _pool = AutoreleasePool::new();
        let cpu_start = thread_cpu_time();
        let wall_start = Instant::now();

        let mut stats = match self.acquire()? {
            Acquired::Frame(frame) => self.draw(frame),
            Acquired::Skipped(reason) => FrameStats { skipped: Some(reason), ..Default::default() },
        };
        stats.cpu_time = thread_cpu_time().saturating_sub(cpu_start);
        stats.wall_time = wall_start.elapsed();

        self.take_error().map_or(Ok(stats), Err)
    }

    /// The last rendered frame as tightly packed RGBA, top row first, `width_px * height_px * 4` bytes. Empty if
    /// the size is zero, or for a layer. Waits for the GPU, so this is for tests and exporting images.
    pub fn read_pixels(&self) -> Vec<u8> {
        match &self.target {
            Target::Offscreen(None) => Vec::new(),
            Target::Offscreen(Some(offscreen)) => {
                let (padded_row, size) = readback_layout(self.width, self.height);
                let buffer = readback_buffer(&self.device, size);
                let mut encoder = self.device.create_command_encoder(&Default::default());
                copy_to_readback(&mut encoder, &offscreen.texture, &buffer, padded_row, self.width, self.height);
                self.queue.submit([encoder.finish()]);
                self.read_back(&buffer, padded_row, self.width, self.height)
            }
            Target::Layer(layer) => match &layer.capture {
                Some(capture) if capture.has_frame => {
                    let (padded_row, _) = readback_layout(capture.width, capture.height);
                    let pixels = self.read_back(&capture.buffer, padded_row, capture.width, capture.height);
                    capture.buffer.unmap();
                    pixels
                }
                _ => Vec::new(),
            },
        }
    }

    fn read_back(&self, buffer: &wgpu::Buffer, padded_row: u32, width: u32, height: u32) -> Vec<u8> {
        buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        let wait = wgpu::PollType::Wait { submission_index: None, timeout: Some(GPU_WAIT_LIMIT) };
        if self.device.poll(wait).is_err() {
            return Vec::new();
        }
        let Ok(mapped) = buffer.slice(..).get_mapped_range() else {
            return Vec::new();
        };
        let row_bytes = (width * BYTES_PER_PIXEL) as usize;
        let mut pixels = Vec::with_capacity(row_bytes * height as usize);
        for row in mapped.chunks_exact(padded_row as usize).take(height as usize) {
            pixels.extend_from_slice(&row[..row_bytes]);
        }
        drop(mapped);
        for pixel in pixels.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2); // The target is BGRA.
        }
        pixels
    }

    /// Blocks until the GPU has finished everything submitted so far.
    pub fn wait_for_gpu(&mut self) {
        let wait = wgpu::PollType::Wait { submission_index: None, timeout: Some(GPU_WAIT_LIMIT) };
        let _ = self.device.poll(wait);
        self.in_flight.clear();
    }

    /// Whether a frame would show anything the last one didn't.
    pub(crate) fn needs_render(&self) -> bool {
        self.frame_dirty
            || self.view_dirty
            || self.overlay_dirty
            || self.fixtures_dirty
            || self.colors.is_dirty_below(self.light_count as usize)
    }

    pub(crate) fn request_redraw(&mut self) {
        self.frame_dirty = true;
    }

    pub(crate) fn take_error(&mut self) -> Option<RenderError> {
        self.errors.lock().take()
    }

    fn report(&self, error: RenderError) {
        self.errors.lock().get_or_insert(error);
    }

    /// Gets something to draw on. For a layer this is where the thread blocks until a drawable is free.
    pub(crate) fn acquire(&mut self) -> Result<Acquired, RenderError> {
        if self.width == 0 || self.height == 0 {
            return Ok(Acquired::Skipped(FrameSkip::ZeroSize));
        }
        match &mut self.target {
            Target::Offscreen(offscreen) => {
                let Some(offscreen) = offscreen else {
                    return Ok(Acquired::Skipped(FrameSkip::ZeroSize));
                };
                while self.in_flight.len() >= MAX_OFFSCREEN_FRAMES_IN_FLIGHT {
                    let oldest = self.in_flight.pop_front();
                    let wait = wgpu::PollType::Wait { submission_index: oldest, timeout: Some(GPU_WAIT_LIMIT) };
                    let _ = self.device.poll(wait);
                }
                Ok(Acquired::Frame(Frame {
                    view: offscreen.view.clone(),
                    texture: offscreen.texture.clone(),
                    drawable: None,
                }))
            }
            Target::Layer(layer) => {
                if layer.needs_configure {
                    layer.configure(&self.device, self.width, self.height);
                }
                let mut status = layer.surface.get_current_texture();
                if matches!(status, wgpu::CurrentSurfaceTexture::Outdated) {
                    layer.configure(&self.device, self.width, self.height);
                    status = layer.surface.get_current_texture();
                }
                match status {
                    wgpu::CurrentSurfaceTexture::Success(drawable)
                    | wgpu::CurrentSurfaceTexture::Suboptimal(drawable) => {
                        let texture = drawable.texture.clone();
                        Ok(Acquired::Frame(Frame {
                            view: texture.create_view(&Default::default()),
                            texture,
                            drawable: Some(drawable),
                        }))
                    }
                    wgpu::CurrentSurfaceTexture::Occluded => Ok(Acquired::Skipped(FrameSkip::Occluded)),
                    wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Outdated => {
                        Ok(Acquired::Skipped(FrameSkip::Timeout))
                    }
                    wgpu::CurrentSurfaceTexture::Lost => {
                        Err(RenderError::Surface("the surface was lost".to_string()))
                    }
                    wgpu::CurrentSurfaceTexture::Validation => {
                        Err(self.take_error().unwrap_or_else(|| {
                            RenderError::Surface("the surface reported a validation error".to_string())
                        }))
                    }
                }
            }
        }
    }

    /// Uploads what changed, draws, submits and presents. Times are left for the caller.
    pub(crate) fn draw(&mut self, frame: Frame) -> FrameStats {
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let lights_uploaded = self.upload(&mut encoder);

        let mut draw_calls = 0;
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(if self.transparent { wgpu::Color::TRANSPARENT } else { wgpu::Color::BLACK }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: self.depth_view.as_ref().map(|view| wgpu::RenderPassDepthStencilAttachment {
                    view, depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Discard }), stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.view_bind_group, &[]);

            if let (Some(background), true) = (&self.background, self.background_alpha > 0 && !self.transparent) {
                pass.set_pipeline(&self.background_pipeline);
                pass.set_bind_group(1, &background.bind_group, &[]);
                pass.draw(0..4, 0..1);
                draw_calls += 1;
            }
            if self.fixture_count > 0 {
                pass.set_pipeline(&self.fixture_pipeline);
                pass.set_vertex_buffer(0, self.fixture_buffer.buffer.slice(..));
                pass.draw(0..self.fixture_count, 0..1);
                draw_calls += 1;
            }
            if self.light_count > 0 {
                pass.set_pipeline(&self.light_pipeline);
                pass.set_vertex_buffer(0, self.geometry.buffer.slice(..));
                pass.set_vertex_buffer(1, self.color_buffer.buffer.slice(..));
                pass.draw(0..4, 0..self.light_count);
                draw_calls += 1;
            }
            if self.overlay_count > 0 {
                pass.set_pipeline(&self.overlay_pipeline);
                pass.set_vertex_buffer(0, self.overlay_buffer.buffer.slice(..));
                pass.draw(0..4, 0..self.overlay_count);
                draw_calls += 1;
            }
        }

        if let Target::Layer(layer) = &mut self.target {
            if layer.usage.contains(wgpu::TextureUsages::COPY_SRC) {
                layer.capture_frame(&self.device, &mut encoder, &frame.texture, self.width, self.height);
            }
        }

        self.belt.finish();
        let submission = self.queue.submit([encoder.finish()]);
        self.belt.recall();

        match frame.drawable {
            Some(drawable) => self.queue.present(drawable),
            None => self.in_flight.push_back(submission),
        }

        self.frame_dirty = false;
        FrameStats {
            draw_calls,
            lights_drawn: self.light_count,
            lights_uploaded,
            ..Default::default()
        }
    }

    fn upload(&mut self, encoder: &mut wgpu::CommandEncoder) -> u32 {
        if self.view_dirty {
            let uniform = ViewUniform {
                pan: self.pan,
                viewport: [self.width as f32, self.height as f32],
                bg_size: self.background_size,
                zoom: self.zoom,
                scale: self.scale,
                bg_dim: self.background_alpha as f32 / 255.0,
                light_scale: self.light_scale,
                opaque_lights: if self.transparent { 1.0 } else { 0.0 },
                camera_depth: self.camera_depth,
            };
            write_staged(&mut self.belt, encoder, &self.view_buffer, 0, bytemuck::bytes_of(&uniform));
            self.view_dirty = false;
        }

        if self.fixtures_dirty {
            let client = [self.width as f32/self.scale,self.height as f32/self.scale];
            let reference = if self.fixture_reference[0] > 0.0 { self.fixture_reference } else { client };
            let size_scale = (client[0]/reference[0].max(1.0)+client[1]/reference[1].max(1.0))/2.0;
            let maximum = (client[0].max(client[1])/size_scale.max(f32::EPSILON)).round();
            let mut heads = self.fixtures.clone();
            for head in &mut heads { if head.params.max_beam_length < 0.0 { head.params.max_beam_length = maximum; } }
            self.fixture_vertices = moving_head_mesh(&heads);
            self.fixture_buffer.ensure(&self.device, size_of_val(self.fixture_vertices.as_slice()) as u64);
            write_staged(&mut self.belt, encoder, &self.fixture_buffer.buffer, 0, bytemuck::cast_slice(&self.fixture_vertices));
            self.fixture_count = self.fixture_vertices.len() as u32;
            self.fixtures_dirty = false;
        }

        if self.overlay_dirty {
            self.overlay_buffer.ensure(&self.device, size_of_val(self.overlay.as_slice()) as u64);
            let bytes = bytemuck::cast_slice(&self.overlay);
            write_staged(&mut self.belt, encoder, &self.overlay_buffer.buffer, 0, bytes);
            self.overlay_count = self.overlay.len() as u32;
            self.overlay_dirty = false;
        }

        let mut uploaded = 0;
        let (belt, target) = (&mut self.belt, &self.color_buffer.buffer);
        self.colors.drain(self.light_count as usize, |start, values| {
            let offset = (start * size_of::<u32>()) as u64;
            write_staged(belt, encoder, target, offset, bytemuck::cast_slice(values));
            uploaded += values.len() as u32;
        });
        uploaded
    }
}

impl LayerTarget {
    fn configure(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.surface.configure(
            device,
            &wgpu::SurfaceConfiguration {
                usage: self.usage,
                format: TARGET_FORMAT,
                color_space: wgpu::SurfaceColorSpace::Srgb,
                width,
                height,
                present_mode: wgpu::PresentMode::Fifo,
                desired_maximum_frame_latency: LAYER_FRAME_LATENCY,
                // On Metal this is what turns off the layer's `opaque`. The colours written are premultiplied either way.
                alpha_mode: if self.transparent { wgpu::CompositeAlphaMode::PostMultiplied } else { wgpu::CompositeAlphaMode::Opaque },
                view_formats: Vec::new(),
            },
        );
        // SAFETY: checked to be a CAMetalLayer when this target was made, and retained by the surface since.
        unsafe { objc::apply_layer_settings(self.layer.0) };
        self.needs_configure = false;
    }

    fn capture_frame(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::Texture,
        width: u32,
        height: u32,
    ) {
        let (padded_row, size) = readback_layout(width, height);
        let capture = match &mut self.capture {
            Some(capture) if capture.width == width && capture.height == height => capture,
            slot => slot.insert(Capture {
                buffer: readback_buffer(device, size),
                width,
                height,
                has_frame: false,
            }),
        };
        copy_to_readback(encoder, texture, &capture.buffer, padded_row, width, height);
        capture.has_frame = true;
    }
}

fn new_instance() -> wgpu::Instance {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = wgpu::Backends::METAL;
    wgpu::Instance::new(descriptor)
}

fn request_gpu(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'static>>,
) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue), RenderError> {
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: surface,
        ..Default::default()
    }))
    .map_err(|_| RenderError::NoAdapter)?;

    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("vixen-render"),
        required_features: wgpu::Features::empty(),
        required_limits: adapter.limits(),
        experimental_features: Default::default(),
        memory_hints: wgpu::MemoryHints::Performance,
        trace: wgpu::Trace::Off,
    }))
    .map_err(|error| RenderError::Device(error.to_string()))?;
    Ok((adapter, device, queue))
}

/// Picks every `factor`th pixel.
fn shrink_nearest(rgba: &[u8], w: u32, h: u32, factor: u32) -> Cow<'_, [u8]> {
    if factor <= 1 {
        return Cow::Borrowed(rgba);
    }
    let (w, h, factor) = (w as usize, h as usize, factor as usize);
    let mut out = Vec::with_capacity((w / factor) * (h / factor) * 4);
    for y in (0..h / factor).map(|y| y * factor) {
        for x in (0..w / factor).map(|x| x * factor) {
            let at = (y * w + x) * 4;
            out.extend_from_slice(&rgba[at..at + 4]);
        }
    }
    Cow::Owned(out)
}

/// `CreateAlphaBackground` fills with black and then draws the image, so any transparency in it shows black.
/// Doing that here means the texture is opaque, and filtering it can't bleed colour from hidden pixels.
fn composite_over_black(rgba: Cow<'_, [u8]>) -> Cow<'_, [u8]> {
    if rgba.as_chunks::<4>().0.iter().all(|pixel| pixel[3] == 255) {
        return rgba;
    }
    let mut out = rgba.into_owned();
    for pixel in out.as_chunks_mut::<4>().0 {
        let alpha = pixel[3] as u32;
        for channel in &mut pixel[..3] {
            *channel = ((*channel as u32 * alpha + 127) / 255) as u8;
        }
        pixel[3] = 255;
    }
    Cow::Owned(out)
}

fn readback_layout(width: u32, height: u32) -> (u32, u64) {
    let padded_row = (width * BYTES_PER_PIXEL).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    (padded_row, padded_row as u64 * height as u64)
}

fn readback_buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    })
}

fn copy_to_readback(
    encoder: &mut wgpu::CommandEncoder,
    texture: &wgpu::Texture,
    buffer: &wgpu::Buffer,
    padded_row: u32,
    width: u32,
    height: u32,
) {
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_row),
                rows_per_image: None,
            },
        },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );
}
