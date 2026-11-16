// All drawing is a triangle strip of four vertices per instance, so a frame is at most three draw calls
// however many lights there are. Everything runs in physical pixels with the origin at the top left.
//
// Colour math stays in the same encoded byte space as FastPixel and GDI+: textures and targets are plain
// Unorm, never sRGB, so a 50% blend lands where the original's does.

struct View {
    pan: vec2<f32>,      // logical pixels
    viewport: vec2<f32>, // physical pixels
    bg_size: vec2<f32>,  // scene units
    zoom: f32,
    scale: f32,          // physical pixels per logical pixel
    bg_dim: f32,         // BackgroundAlpha / 255
    light_scale: f32,    // 0 for FastPixel's rule, otherwise OpenGL points scaled by this
    opaque_lights: f32,  // 1 when unlit areas show through: a dim light stays an opaque dark disc
    camera_depth: f32,   // 0 for a flat view; perspective distance for moving-head volumes
};

@group(0) @binding(0) var<uniform> view: View;

// Anything below this radius, in physical pixels, can lose its whole lit area between pixel centres, so it
// is placed on the pixel grid instead of being blurred across it.
const SMALL_RADIUS: f32 = 2.0;

// Keeps a position that is an integer give or take float error on the integer's side of a floor().
const SNAP_EPSILON: f32 = 1.0 / 256.0;

const OFF_SCREEN: vec4<f32> = vec4<f32>(2.0, 2.0, 2.0, 1.0);

const KIND_LINE: u32 = 0u;
const KIND_SQUARE: u32 = 1u;

// Strip order: (-1,-1), (1,-1), (-1,1), (1,1).
fn corner(index: u32) -> vec2<f32> {
    return vec2<f32>(f32(index & 1u), f32((index >> 1u) & 1u)) * 2.0 - 1.0;
}

fn clip_from_px(p: vec2<f32>) -> vec4<f32> {
    let ndc = p / view.viewport * 2.0 - 1.0;
    var depth = 0.5;
    if (view.camera_depth > 0.0) { depth = (6000.0 / 5999.0) * (1.0 - 1.0 / view.camera_depth); }
    return vec4<f32>(ndc.x, -ndc.y, depth, 1.0);
}

// Scene coordinates name pixels, as FastPixel's do: at zoom 1 a point at (10, 20) is the pixel whose corner
// is at (10, 20), so its centre is half a logical pixel further on.
fn px_from_scene(p: vec2<f32>) -> vec2<f32> {
    return (p * view.zoom + view.pan + 0.5) * view.scale;
}

// Moves `center` so that a shape `span` pixels across starts on a pixel boundary. The floor matches the
// truncation in PreviewPixel.Draw when the zoomed position is fractional.
fn snap(center: vec2<f32>, span: f32) -> vec2<f32> {
    return floor(center - span * 0.5 + SNAP_EPSILON) + span * 0.5;
}

struct Disc {
    center: vec2<f32>,
    radius: f32,
    ramp: f32, // width of the anti-aliased edge
};

fn make_disc(center: vec2<f32>, radius: f32) -> Disc {
    if (radius >= SMALL_RADIUS) {
        return Disc(center, radius, 1.0);
    }
    // A tighter edge keeps the pixels a small light covers at full colour. Two pixels across sits on a pixel
    // corner, which takes a radius of 1 to reach the middle of all four pixels around it.
    let span = max(floor(radius * 2.0 + 0.5), 1.0);
    return Disc(snap(center, span), max(radius, 0.5 * min(span, 2.0)), 0.5);
}

fn disc_coverage(local: vec2<f32>, radius: f32, inverse_ramp: f32) -> f32 {
    return clamp((radius - length(local)) * inverse_ramp + 0.5, 0.0, 1.0);
}

// ---------------------------------------------------------------------------------------------------------
// Background: the image over black, dimmed. Scene (0,0)-(w,h) at any zoom, as DrawImage draws it.
// ---------------------------------------------------------------------------------------------------------

@group(1) @binding(0) var bg_texture: texture_2d<f32>;
@group(1) @binding(1) var bg_sampler: sampler;

struct BackgroundOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn background_vs(@builtin(vertex_index) index: u32) -> BackgroundOut {
    let uv = corner(index) * 0.5 + 0.5;
    var out: BackgroundOut;
    out.position = clip_from_px((uv * view.bg_size * view.zoom + view.pan) * view.scale);
    out.uv = uv;
    return out;
}

@fragment
fn background_fs(in: BackgroundOut) -> @location(0) vec4<f32> {
    // The texture holds the image already composited over black.
    return vec4<f32>(textureSample(bg_texture, bg_sampler, in.uv).rgb * view.bg_dim, 1.0);
}

// Mipmaps: each level is the level above it, filtered, so a zoomed-out view doesn't skip most of the image.
@vertex
fn mipmap_vs(@builtin(vertex_index) index: u32) -> BackgroundOut {
    let c = corner(index);
    var out: BackgroundOut;
    out.position = vec4<f32>(c.x, -c.y, 0.0, 1.0);
    out.uv = c * 0.5 + 0.5;
    return out;
}

@fragment
fn mipmap_fs(in: BackgroundOut) -> @location(0) vec4<f32> {
    return textureSampleLevel(bg_texture, bg_sampler, in.uv, 0.0);
}

// ---------------------------------------------------------------------------------------------------------
// Lights: position zooms, size does not (PreviewPixel.Draw).
// ---------------------------------------------------------------------------------------------------------

struct LightOut {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) @interpolate(flat) edge: vec2<f32>, // radius, 1 / ramp
    @location(2) @interpolate(flat) color: vec4<f32>, // premultiplied
};

@vertex
fn light_vs(
    @builtin(vertex_index) index: u32,
    @location(0) geom: vec3<f32>, // x, y, size
    @location(1) color: vec4<f32>,
) -> LightOut {
    var out: LightOut;
    // Written so a NaN size is rejected as well.
    if (color.a <= 0.0 || !(geom.z > 0.0)) {
        out.position = OFF_SCREEN;
        return out;
    }

    // FastPixel.DrawCircle lights every pixel within Width / 2 (integer division) of the centre, and exactly
    // one pixel for Width 1. A quarter pixel over keeps those pixels fully covered.
    var radius = max(floor(geom.z * 0.5) + 0.25, 0.5) * view.scale;
    if (view.light_scale > 0.0) {
        // The OpenGL form's gl_PointSize: a diameter, which a size under 1 keeps at 1.
        let point = select(geom.z * view.light_scale, 1.0, geom.z < 1.0);
        radius = max(point * 0.5, 0.5) * view.scale;
    }
    let disc = make_disc(px_from_scene(geom.xy), radius);
    let local = corner(index) * (disc.radius + 0.5 * disc.ramp);

    out.position = clip_from_px(disc.center + local);
    out.local = local;
    out.edge = vec2<f32>(disc.radius, 1.0 / disc.ramp);
    out.color = vec4<f32>(color.rgb * color.a, select(color.a, 1.0, view.opaque_lights > 0.0));
    return out;
}

@fragment
fn light_fs(in: LightOut) -> @location(0) vec4<f32> {
    return in.color * disc_coverage(in.local, in.edge.x, in.edge.y);
}

// ---------------------------------------------------------------------------------------------------------
// Overlay: positions follow the view, strokes and handle sizes stay the same on screen.
// ---------------------------------------------------------------------------------------------------------

struct OverlayOut {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    // Line: half length, half width. Square: half side, half stroke. Circle: radius, 1 / ramp.
    @location(1) @interpolate(flat) shape: vec2<f32>,
    @location(2) @interpolate(flat) color: vec4<f32>, // premultiplied
    @location(3) @interpolate(flat) kind: u32,
};

@vertex
fn overlay_vs(
    @builtin(vertex_index) index: u32,
    @location(0) p0: vec2<f32>,
    @location(1) p1: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) param: f32,
    @location(4) kind: u32,
) -> OverlayOut {
    var out: OverlayOut;
    if (color.a <= 0.0) {
        out.position = OFF_SCREEN;
        return out;
    }
    out.color = vec4<f32>(color.rgb * color.a, color.a);
    out.kind = kind;

    let c = corner(index);
    if (kind == KIND_LINE) {
        let stroke = max(round(param * view.scale), 1.0);
        var a = px_from_scene(p0);
        var b = px_from_scene(p1);
        // Rubber bands and outlines stay crisp at any zoom, with corners that meet.
        if (p0.x == p1.x || p0.y == p1.y) {
            a = snap(a, stroke);
            b = snap(b, stroke);
        }
        let along = b - a;
        let length_px = length(along);
        let dir = select(vec2<f32>(1.0, 0.0), along / length_px, length_px > 0.0);
        let normal = vec2<f32>(-dir.y, dir.x);
        // Projecting caps: DrawLine includes both end pixels, and joined segments close their corners.
        let half_size = vec2<f32>(length_px + stroke, stroke) * 0.5;
        let local = c * (half_size + 0.5);
        out.position = clip_from_px((a + b) * 0.5 + dir * local.x + normal * local.y);
        out.local = local;
        out.shape = half_size;
    } else if (kind == KIND_SQUARE) {
        // FastPixel.DrawRectangle(X, Y, width, color): a 1 pixel outline from X - width / 2 to X + width / 2.
        let stroke = max(round(view.scale), 1.0);
        let half_side = max(floor(param * 0.5), 0.0) * view.scale;
        let outer = half_side + stroke * 0.5;
        let local = c * (outer + 0.5);
        out.position = clip_from_px(snap(px_from_scene(p0), outer * 2.0) + local);
        out.local = local;
        out.shape = vec2<f32>(half_side, stroke * 0.5);
    } else {
        if (!(param > 0.0)) {
            out.position = OFF_SCREEN;
            return out;
        }
        let disc = make_disc(px_from_scene(p0), max(param + 0.25, 0.5) * view.scale);
        let local = c * (disc.radius + 0.5 * disc.ramp);
        out.position = clip_from_px(disc.center + local);
        out.local = local;
        out.shape = vec2<f32>(disc.radius, 1.0 / disc.ramp);
    }
    return out;
}

@fragment
fn overlay_fs(in: OverlayOut) -> @location(0) vec4<f32> {
    var coverage: f32;
    if (in.kind == KIND_LINE) {
        let edge = clamp(in.shape - abs(in.local) + 0.5, vec2<f32>(0.0), vec2<f32>(1.0));
        coverage = edge.x * edge.y;
    } else if (in.kind == KIND_SQUARE) {
        let from_outline = abs(max(abs(in.local.x), abs(in.local.y)) - in.shape.x);
        coverage = clamp(in.shape.y - from_outline + 0.5, 0.0, 1.0);
    } else {
        coverage = disc_coverage(in.local, in.shape.x, in.shape.y);
    }
    return in.color * coverage;
}

// Moving heads retain their actual Z. Plane-zero props share the same camera/depth projection.
struct FixtureOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) body: f32,
    @location(4) viewport: vec4<f32>,
    @location(5) local_camera: f32,
};
@vertex
fn fixture_vs(@location(0) p: vec3<f32>, @location(1) normal: vec3<f32>,
              @location(2) color: vec4<f32>, @location(3) body: f32,
              @location(4) viewport: vec4<f32>, @location(5) local_camera: f32) -> FixtureOut {
    let plane = (p.xy * view.zoom + view.pan) * view.scale;
    let ndc = plane / view.viewport * 2.0 - 1.0;
    let distance = view.camera_depth - p.z;
    var out: FixtureOut;
    out.clip = vec4<f32>(ndc.x * view.camera_depth, -ndc.y * view.camera_depth,
        (6000.0 / 5999.0) * (distance - 1.0), distance);
    if (local_camera > 0.0) {
        let center = viewport.xy + viewport.zw * 0.5;
        let factor = local_camera / (local_camera - p.z);
        let projected = center + (p.xy - center) * factor;
        let clip = clip_from_px(px_from_scene(projected));
        out.clip = vec4<f32>(clip.xy, 0.5-p.z/(local_camera*4.0), 1.0) / factor;
    } else if (view.camera_depth <= 0.0) { out.clip = OFF_SCREEN; }
    out.position = p;
    out.normal = normal;
    out.color = color;
    out.body = body;
    out.viewport = viewport;
    out.local_camera = local_camera;
    return out;
}
@fragment
fn fixture_fs(in: FixtureOut) -> @location(0) vec4<f32> {
    var camera = vec3<f32>((view.viewport / view.scale * 0.5 - view.pan) / view.zoom, view.camera_depth);
    if (in.local_camera > 0.0) {
        let near = px_from_scene(in.viewport.xy);
        let far = px_from_scene(in.viewport.xy + in.viewport.zw);
        if (any(in.clip.xy < near) || any(in.clip.xy >= far)) { discard; }
        camera = vec3<f32>(in.viewport.xy + in.viewport.zw * 0.5, in.local_camera);
    }
    let light_dir = normalize(camera - in.position);
    let normal = normalize(in.normal);
    let diffuse = max(dot(normal, light_dir), 0.0);
    var color = in.color.rgb;
    var specular = vec3<f32>(0.0);
    if (in.body > 0.5) {
        color *= view.bg_dim;
        specular = 0.2 * pow(max(dot(light_dir, reflect(-light_dir, normal)), 0.0), 32.0) * color;
    }
    let lit = (vec3<f32>(0.3 + diffuse) + specular) * color;
    return vec4<f32>(lit * in.color.a, in.color.a);
}
