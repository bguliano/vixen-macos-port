//! Port of VixenModules.Effect.Tree.

use crate::generated::tree as gen;
use vixen_effects::prelude::*;

// The C# constructor never calls `EnableTargetPositioning`, has no `UpdateAttributes` and has no
// `RenderEffectByLocation`, so there is no visibility rule and no location support.
pub static DESCRIPTOR: EffectDescriptor = EffectDescriptor {
    render,
    ported: true,
    ..gen::BASE
};

#[derive(Copy, Clone, PartialEq)]
enum BranchDirection {
    Up,
    Down,
    Left,
    Right,
    UpRight,
    UpLeft,
    DownRight,
    DownLeft,
    Alternate,
    None,
}

#[derive(Copy, Clone, PartialEq)]
enum TreeColorType {
    Static,
    Twinkle,
    Alternate,
    AlternatePixel,
}

struct Tree {
    direction: BranchDirection,
    color_type: TreeColorType,
    colors: Vec<ColorGradient>,
    background_color: ColorGradient,
    background_level_curve: Curve,
    blend_curve: Curve,
    level_curve: Curve,
    branches: i32,
    speed: i32,
    toggle_blend: bool,
    tree_width: i32,
    x_limit: i32,
    branch_color: i32,
    branch: i32,
    m: i32,
    color_idx: i32,
    row: i32,
    rand: Rand,
}

fn render(ctx: &RenderCtx, p: &Params) -> EffectIntents {
    // Indexing the empty list throws in the C#, and a pixel effect that throws has no intents at all.
    if p.gradients("Colors").is_empty() {
        return EffectIntents::new();
    }
    let mut e = Tree {
        direction: match p.enum_name("BranchDirection") {
            "Down" => BranchDirection::Down,
            "Left" => BranchDirection::Left,
            "Right" => BranchDirection::Right,
            "UpRight" => BranchDirection::UpRight,
            "UpLeft" => BranchDirection::UpLeft,
            "DownRight" => BranchDirection::DownRight,
            "DownLeft" => BranchDirection::DownLeft,
            "Alternate" => BranchDirection::Alternate,
            "None" => BranchDirection::None,
            _ => BranchDirection::Up,
        },
        color_type: match p.enum_name("ColorType") {
            "Twinkle" => TreeColorType::Twinkle,
            "Alternate" => TreeColorType::Alternate,
            "AlternatePixel" => TreeColorType::AlternatePixel,
            _ => TreeColorType::Static,
        },
        colors: p.gradients("Colors"),
        background_color: p.gradient("BackgroundColor"),
        background_level_curve: p.curve("BackgroundLevelCurve"),
        blend_curve: p.curve("BlendCurve"),
        level_curve: p.curve("LevelCurve"),
        branches: p.int("Branches"),
        speed: p.int("Speed"),
        toggle_blend: p.boolean("ToggleBlend"),
        tree_width: 0,
        x_limit: 0,
        branch_color: 0,
        branch: 0,
        m: 0,
        color_idx: 0,
        row: 0,
        rand: Rand::default(),
    };
    render_pixel_effect(ctx, p, &mut e)
}

impl Tree {
    /// The `Branch` property setter: a changed branch bumps the branch colour.
    fn set_branch(&mut self, value: i32) {
        if self.branch != value {
            self.branch_color += 1;
        }
        self.branch = value;
    }

    /// The `M` property setter: arriving at 3 steps the colour index, wrapping at the colour count.
    fn set_m(&mut self, value: i32) {
        if self.m != value && value == 3 {
            self.color_idx += 1;
            if self.color_idx == self.colors.len() as i32 {
                self.color_idx = 0;
            }
        }
        self.m = value;
    }

    /// `Colors[idx].GetColorAt(pos)`; `None` is the C# index exception.
    fn color_at(&self, idx: i32, pos: f64) -> Option<Color> {
        if idx < 0 {
            return None;
        }
        self.colors.get(idx as usize).map(|g| g.get_color_at(pos))
    }

    /// The 6-strand pattern shared by every branch direction.
    #[inline]
    fn pattern(&self) -> bool {
        let (row, m) = (self.row, self.m);
        (row == 3 && (m == 0 || m == 5)) || (row == 2 && (m == 1 || m == 4)) || (row == 1 && (m == 2 || m == 3))
    }

    /// Whether the pixel at `x` is part of the tree for the current branch direction.
    fn in_tree(&self, x: i32, wi: i32, b: i32) -> bool {
        let pattern = self.pattern();
        match self.direction {
            BranchDirection::UpRight | BranchDirection::DownRight | BranchDirection::UpLeft | BranchDirection::DownLeft => {
                self.branch <= b && x <= self.tree_width && pattern
            }
            BranchDirection::Up | BranchDirection::Down => self.branch <= b && pattern,
            BranchDirection::Left => (wi - x) <= self.tree_width && pattern,
            BranchDirection::Alternate => {
                if self.branch % 2 != 0 {
                    (wi - x) <= self.tree_width && pattern
                } else {
                    x <= self.tree_width && pattern
                }
            }
            BranchDirection::Right => x <= self.tree_width && pattern,
            BranchDirection::None => pattern,
        }
    }
}

impl PixelEffect for Tree {
    fn setup_render(&mut self, info: &PixelInfo) {
        self.branch_color = 0;
        self.tree_width = 0;
        self.x_limit = info.buffer_wi();
    }

    fn render_effect(&mut self, frame: i32, info: &PixelInfo, buf: &mut PixelFrameBuffer) {
        let (wi, ht) = (info.buffer_wi(), info.buffer_ht());
        let pos = info.interval_position(frame);
        let interval_pos_factor = pos * 100.0;
        let level = self.level_curve.get_value(interval_pos_factor) / 100.0;
        let blend_level = self.blend_curve.get_value(interval_pos_factor) / 100.0;
        let background_level = self.background_level_curve.get_value(interval_pos_factor) / 100.0;
        let mut background_hsv = Hsv::from_color(self.background_color.get_color_at(pos));
        // `FrameTimespan.Milliseconds` is the millisecond component, 0..999.
        let cycle_len = (frame as f64 * self.speed as f64 * ((info.frame_time % 1000) as f64 / 50.0)) as i32;
        let mut pixels_per_branch = (0.5 + ht as f64 / self.branches as f64) as i32;
        if pixels_per_branch == 0 {
            pixels_per_branch = 1;
        }

        if frame == self.x_limit {
            self.tree_width = 0;
            self.x_limit += wi;
        }
        self.tree_width += 1;
        // The C# takes `% Branches` below, which throws for zero.
        if self.branches == 0 || wi <= 0 {
            return;
        }

        for y in 0..ht {
            let mut modulo = y % pixels_per_branch;
            if modulo == 0 {
                modulo = pixels_per_branch;
            }
            let v: f32 = if self.toggle_blend {
                // Fade between branches.
                (1.0 - (1.0 * modulo as f64 / pixels_per_branch as f64) * (1.0 - blend_level)) as f32
            } else {
                ((1.0 * modulo as f64 / pixels_per_branch as f64) * blend_level) as f32
            };
            background_hsv.set_v(background_level * v as f64);
            self.set_branch((y - 1) / pixels_per_branch);
            if self.branch_color >= self.colors.len() as i32 || self.branch == 0 {
                self.branch_color = 0;
            }
            // row 0 is the bottom of a branch, row 1 one above that.
            self.row = pixels_per_branch - modulo;
            let b = cycle_len / wi % self.branches;

            for x in 0..wi {
                let mut col = background_hsv.to_color();
                self.set_m(x % 6);
                match self.color_type {
                    TreeColorType::Twinkle => self.color_idx = self.rand.next_range(0, self.colors.len() as i32),
                    TreeColorType::AlternatePixel => {
                        if self.colors.is_empty() {
                            return;
                        }
                        self.color_idx = x % self.colors.len() as i32;
                    }
                    TreeColorType::Static => self.color_idx = self.branch_color,
                    TreeColorType::Alternate => {}
                }
                if self.in_tree(x, wi, b) {
                    match self.color_at(self.color_idx, pos) {
                        Some(c) => col = c,
                        None => return,
                    }
                }
                if level < 1.0 {
                    let mut hsv = Hsv::from_color(col);
                    hsv.set_v(hsv.v * level);
                    col = hsv.to_color();
                }
                match self.direction {
                    BranchDirection::Down | BranchDirection::DownRight => buf.set_pixel(x, ht - y, col),
                    BranchDirection::UpLeft => buf.set_pixel(wi - x, y, col),
                    BranchDirection::DownLeft => buf.set_pixel(wi - x, ht - y, col),
                    _ => buf.set_pixel(x, y, col),
                }
            }
            if self.colors.is_empty() {
                return;
            }
            self.color_idx = self.branch % self.colors.len() as i32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_effects::testing;

    #[test]
    fn smoke() {
        testing::smoke_test(&DESCRIPTOR);
        testing::assert_produces_light(&DESCRIPTOR);
    }

    fn tree(direction: BranchDirection) -> Tree {
        Tree {
            direction,
            color_type: TreeColorType::Static,
            colors: vec![ColorGradient::solid(Color::rgb(0, 255, 0)), ColorGradient::solid(Color::rgb(0, 0, 255))],
            background_color: ColorGradient::solid(Color::BLACK),
            background_level_curve: Curve::flat(0.0),
            blend_curve: Curve::flat(70.0),
            level_curve: Curve::flat100(),
            branches: 2,
            speed: 10,
            toggle_blend: true,
            tree_width: 0,
            x_limit: 0,
            branch_color: 0,
            branch: 0,
            m: 0,
            color_idx: 0,
            row: 0,
            rand: Rand::seeded(5),
        }
    }

    #[test]
    fn m_setter_steps_the_colour_index_on_reaching_three() {
        let mut t = tree(BranchDirection::Up);
        t.set_m(2);
        assert_eq!(t.color_idx, 0);
        t.set_m(3);
        assert_eq!(t.color_idx, 1);
        t.set_m(3); // unchanged, so no step
        assert_eq!(t.color_idx, 1);
        t.set_m(0);
        t.set_m(3);
        assert_eq!(t.color_idx, 0); // wraps at the colour count of 2
    }

    #[test]
    fn branch_setter_bumps_the_branch_colour_only_on_change() {
        let mut t = tree(BranchDirection::Up);
        t.set_branch(0);
        assert_eq!(t.branch_color, 0);
        t.set_branch(1);
        t.set_branch(1);
        assert_eq!(t.branch_color, 1);
    }

    #[test]
    fn first_frame_draws_only_the_bottom_branch_pattern() {
        // 6 wide, 4 high, 2 branches -> 2 pixels per branch. Frame 0 has cycle 0, so b = 0.
        let info = crate::test_util::info(6, 4, 1000);
        let mut t = tree(BranchDirection::Up);
        t.setup_render(&info);
        let mut buf = PixelFrameBuffer::new(6, 4);
        buf.clear_buffer();
        t.render_effect(0, &info, &mut buf);
        // y = 1: mod 1, row 1 -> columns with m = 2 or 3 (x = 2, 3). Branch (1-1)/2 = 0 <= b.
        assert_eq!(buf.get_color_at(2, 1), Color::rgb(0, 255, 0));
        assert_eq!(buf.get_color_at(3, 1), Color::rgb(0, 255, 0));
        assert_ne!(buf.get_color_at(0, 1), Color::rgb(0, 255, 0));
        // y = 2 is mod 2 (the top of the branch... the last pixel), row 0, so no tree pixels.
        assert_ne!(buf.get_color_at(2, 2), Color::rgb(0, 255, 0));
    }
}
