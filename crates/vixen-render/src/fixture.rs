//! Shared moving-head volumes from `FixtureGraphics/OpenGL`. Coordinates returned to the renderer are
//! scene X/right, Y/down, Z/toward the camera. The editor chooses its own tilt offset and beam-width rule.

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BeamWidthModel {
    PreviewMultiplier(f32),
    EditorTopPlusBottom,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MovingHeadParams {
    pub pan_deg: f32,
    pub tilt_deg: f32,
    /// Every port view passes 180 for bottom mounting and 0 for top (owner-confirmed OpenGL convention).
    pub tilt_offset_deg: f32,
    pub mounted_top: bool,
    pub max_beam_length: f32,
    pub beam_length_pct: f32,
    pub focus: f32,
    pub intensity: f32,
    pub on: bool,
    /// Straight RGBA8, in the renderer's byte order.
    pub color_left: u32,
    pub color_right: u32,
    pub beam_alpha: f32,
    pub beam_width: BeamWidthModel,
    /// The preview's opaque black lens when off; the editor can omit it.
    pub black_stub: bool,
    /// Draw through a local 45-degree camera clipped to the instance's square, on a flat editor canvas.
    pub thumbnail: bool,
}

impl Default for MovingHeadParams {
    fn default() -> Self {
        Self {
            pan_deg: 0.0,
            tilt_deg: 0.0,
            tilt_offset_deg: 180.0,
            mounted_top: false,
            max_beam_length: 100.0,
            beam_length_pct: 1.0,
            focus: 100.0,
            intensity: 100.0,
            on: true,
            color_left: crate::rgba(255, 255, 0, 255),
            color_right: crate::rgba(255, 255, 0, 255),
            beam_alpha: 1.0,
            beam_width: BeamWidthModel::PreviewMultiplier(8.0),
            black_stub: true,
            thumbnail: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MovingHeadInstance {
    /// Centre of the drawing rectangle, in scene coordinates.
    pub position: [f32; 2],
    pub length: f32,
    pub params: MovingHeadParams,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct FixtureVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
    /// Body takes BackgroundAlpha and its tinted specular highlight; beams don't.
    pub body: f32,
    pub viewport: [f32; 4],
    pub local_camera: f32,
}

type V3 = [f32; 3];
fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn rotate_x(p: V3, a: f32) -> V3 {
    let (s, c) = a.sin_cos();
    [p[0], p[1] * c - p[2] * s, p[1] * s + p[2] * c]
}
fn rotate_y(p: V3, a: f32) -> V3 {
    let (s, c) = a.sin_cos();
    [p[0] * c + p[2] * s, p[1], -p[0] * s + p[2] * c]
}

struct Mesh<'a> {
    out: &'a mut Vec<FixtureVertex>,
    color: [f32; 4],
    body: f32,
}
impl Mesh<'_> {
    fn tri(&mut self, a: V3, b: V3, c: V3, transform: &impl Fn(V3) -> V3) {
        let [a, b, c] = [transform(a), transform(b), transform(c)];
        let n = cross(sub(b, a), sub(c, a));
        let size = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2])
            .sqrt()
            .max(f32::EPSILON);
        let n = [n[0] / size, n[1] / size, n[2] / size];
        for position in [a, b, c] {
            self.out.push(FixtureVertex {
                position,
                normal: n,
                color: self.color,
                body: self.body,
                viewport: [0.0; 4],
                local_camera: 0.0,
            });
        }
    }
    fn quad(&mut self, a: V3, b: V3, c: V3, d: V3, t: &impl Fn(V3) -> V3) {
        self.tri(a, b, c, t);
        self.tri(a, c, d, t);
    }
    fn box_volume(&mut self, w: f32, h: f32, base: f32, top: f32, t: &impl Fn(V3) -> V3) {
        let p = [
            [-w, -h, base],
            [w, -h, base],
            [w, h, top],
            [-w, h, top],
            [-w, -h, -base],
            [w, -h, -base],
            [w, h, -top],
            [-w, h, -top],
        ];
        // Scene Y is reflected by the final transform. Reverse each world-space face accordingly.
        for [a, b, c, d] in [
            [0, 3, 2, 1],
            [4, 5, 6, 7],
            [3, 7, 6, 2],
            [0, 1, 5, 4],
            [1, 2, 6, 5],
            [0, 4, 7, 3],
        ] {
            self.quad(p[a], p[b], p[c], p[d], t);
        }
    }
    fn cylinder(
        &mut self,
        length: f32,
        r0: f32,
        r1: f32,
        offset: f32,
        panels: impl Iterator<Item = usize>,
        caps: bool,
        t: &impl Fn(V3) -> V3,
    ) {
        // Keep the original 20 panels with divisor 19 (the last repeats the first).
        for i in panels {
            let a = i as f32 / 19.0 * std::f32::consts::TAU;
            let b = (i + 1) as f32 / 19.0 * std::f32::consts::TAU;
            let p = [a.cos() * r0, -offset, a.sin() * r0];
            let q = [b.cos() * r0, -offset, b.sin() * r0];
            let r = [b.cos() * r1, length - offset, b.sin() * r1];
            let s = [a.cos() * r1, length - offset, a.sin() * r1];
            self.quad(p, s, r, q, t);
            if caps {
                self.tri(p, q, [0.0, -offset, 0.0], t);
                // Upstream's top cap centre uses +offset, even when the rim is at length-offset.
                self.tri(s, [0.0, offset, 0.0], r, t);
            }
        }
    }
}

/// Returns bottom and top beam radii. The two callers intentionally have different focus formulas.
pub fn moving_head_beam_radii(length: f32, focus: f32, model: BeamWidthModel) -> (f32, f32) {
    let bottom = length * 0.1;
    (
        bottom,
        match model {
            BeamWidthModel::PreviewMultiplier(m) => m * bottom * focus / 100.0,
            BeamWidthModel::EditorTopPlusBottom => bottom + 8.0 * bottom * focus / 100.0,
        },
    )
}

pub(crate) fn moving_head_mesh(instances: &[MovingHeadInstance]) -> Vec<FixtureVertex> {
    let mut out = Vec::with_capacity(instances.len() * 900);
    // Bodies before beams, as MovingHeadRenderStrategy does. Depth resolves the separate volumes.
    for body in [true, false] {
        for instance in instances {
            let p = &instance.params;
            let l = instance.length * if p.thumbnail { 0.5 } else { 1.0 };
            if !l.is_finite() || l <= 0.0 {
                continue;
            }
            let sign = if p.mounted_top { -1.0 } else { 1.0 };
            let pan = p.pan_deg.to_radians();
            let tilt = (p.tilt_deg + p.tilt_offset_deg).to_radians();
            let start = out.len();
            let world = |v: V3| {
                [
                    instance.position[0] + v[0],
                    instance.position[1] - v[1],
                    v[2] - if p.thumbnail { 0.0 } else { 0.9 * l },
                ]
            };
            let group = |v: V3| world(add(rotate_y(v, pan), [0.0, -sign * 0.36 * l, 0.0]));
            let head = |v: V3| group(add(rotate_x(v, tilt), [0.0, sign * 0.45 * l, 0.0]));
            if body {
                let mut m = Mesh {
                    out: &mut out,
                    color: [128.0 / 255.0, 128.0 / 255.0, 128.0 / 255.0, 1.0],
                    body: 1.0,
                };
                m.box_volume(0.45 * l, 0.07 * l, 0.45 * l, 0.45 * l, &|v| {
                    world(add(v, [0.0, -sign * 0.43 * l, 0.0]))
                });
                let (b, t) = if p.mounted_top {
                    (0.1 * l, 0.2 * l)
                } else {
                    (0.2 * l, 0.1 * l)
                };
                for x in [-0.252 * l, 0.252 * l] {
                    m.box_volume(0.05 * l, 0.3 * l, b, t, &|v| {
                        group(add(v, [x, sign * 0.3 * l, 0.0]))
                    });
                }
                m.cylinder(
                    0.504 * l,
                    0.07 * l,
                    0.07 * l,
                    0.252 * l,
                    0..20,
                    false,
                    &|v| group([v[1], v[0] + sign * 0.45 * l, v[2]]),
                );
                m.cylinder(0.6 * l, 0.18 * l, 0.18 * l, 0.198 * l, 0..20, true, &head);
            } else {
                let lit = p.on && p.color_left.to_le_bytes()[3] != 0;
                if !lit && !p.black_stub {
                    continue;
                }
                let (r0, r1) = moving_head_beam_radii(l, p.focus, p.beam_width);
                for left in [true, false] {
                    let bytes = if left { p.color_left } else { p.color_right }.to_le_bytes();
                    let factor = if lit {
                        p.intensity.clamp(0.0, 100.0) / 100.0 / 255.0
                    } else {
                        0.0
                    };
                    let color = [
                        bytes[0] as f32 * factor,
                        bytes[1] as f32 * factor,
                        bytes[2] as f32 * factor,
                        if lit {
                            p.beam_alpha.clamp(0.0, 1.0)
                        } else {
                            1.0
                        },
                    ];
                    let mut m = Mesh {
                        out: &mut out,
                        color,
                        body: 0.0,
                    };
                    m.cylinder(
                        if lit {
                            p.max_beam_length * p.beam_length_pct / 100.0
                        } else {
                            0.5
                        },
                        r0,
                        if lit { r1 } else { r0 },
                        0.0,
                        (0..20).filter(|i| (5..15).contains(i) == left),
                        true,
                        &|v| head(add(v, [0.0, 0.402 * l, 0.0])),
                    );
                }
            }
            if p.thumbnail {
                for v in &mut out[start..] {
                    v.viewport = [instance.position[0]-l,instance.position[1]-l,2.0*l,2.0*l];
                    v.local_camera = l / (std::f32::consts::PI / 8.0).tan();
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preview_and_editor_focus_rules_remain_distinct() {
        assert_eq!(
            moving_head_beam_radii(10.0, 0.0, BeamWidthModel::PreviewMultiplier(8.0)),
            (1.0, 0.0)
        );
        assert_eq!(
            moving_head_beam_radii(10.0, 0.0, BeamWidthModel::EditorTopPlusBottom),
            (1.0, 1.0)
        );
        assert_eq!(
            moving_head_beam_radii(10.0, 100.0, BeamWidthModel::EditorTopPlusBottom),
            (1.0, 9.0)
        );
    }
    #[test]
    fn caller_controls_tilt_convention_and_off_stub() {
        let mut head = MovingHeadInstance {
            position: [100.0, 100.0],
            length: 20.0,
            params: MovingHeadParams {
                beam_length_pct: 100.0,
                ..Default::default()
            },
        };
        let mesh = moving_head_mesh(&[head.clone()]);
        assert!(mesh
            .iter()
            .filter(|v| v.body == 0.0)
            .any(|v| v.position[1] > 190.0));
        head.params.tilt_offset_deg = 0.0;
        let mesh = moving_head_mesh(&[head.clone()]);
        assert!(mesh
            .iter()
            .filter(|v| v.body == 0.0)
            .any(|v| v.position[1] < 0.0));
        head.params.on = false;
        let mesh = moving_head_mesh(&[head.clone()]);
        assert!(mesh
            .iter()
            .filter(|v| v.body == 0.0)
            .all(|v| v.color == [0.0, 0.0, 0.0, 1.0]));
        head.params.black_stub = false;
        assert!(moving_head_mesh(&[head]).iter().all(|v| v.body == 1.0));
    }
}
