//! Helpers shared by the effect tests: rendering onto a grid and reading the frames back.

use vixen_effects::prelude::*;
use vixen_effects::testing;

/// A grid of `strings` strings of `pixels` pixels.
pub struct Grid {
    pub tree: ElementTree,
    pub root: NodeId,
    pub strings: usize,
    pub pixels: usize,
}

impl Grid {
    pub fn new(strings: usize, pixels: usize) -> Grid {
        let mut tree = ElementTree::new();
        let root = testing::grid(&mut tree, "Grid", strings, pixels);
        Grid { tree, root, strings, pixels }
    }

    /// Renders with horizontal strings (the data default) and the given edits, and reads the frames.
    /// Each string is a row, so x runs along a string and y picks the string.
    pub fn render(&self, d: &'static EffectDescriptor, ms: i64, edit: impl FnOnce(&mut Params)) -> Frames {
        self.frames(&testing::render_with(d, &self.tree, &[self.root], ms, edit))
    }

    pub fn frames(&self, out: &EffectIntents) -> Frames {
        let elements = self.tree.distinct_elements(self.root);
        let mut columns: Vec<Vec<RgbValue>> = Vec::with_capacity(elements.len());
        for id in &elements {
            let Some(nodes) = out.nodes_for(id) else { return Frames::dark() };
            match &nodes[0].intent.data {
                IntentData::ArrayRgb(a) => columns.push(a.values().to_vec()),
                other => panic!("expected an RGB array, got {other:?}"),
            }
        }
        let frame_count = columns.first().map_or(0, |c| c.len());
        Frames { wi: self.pixels, ht: self.strings, frame_count, columns }
    }
}

/// Rendered pixels, by frame and position. (0, 0) is lower left, as in `PixelFrameBuffer`.
pub struct Frames {
    pub wi: usize,
    pub ht: usize,
    pub frame_count: usize,
    columns: Vec<Vec<RgbValue>>,
}

impl Frames {
    fn dark() -> Frames {
        Frames { wi: 0, ht: 0, frame_count: 0, columns: Vec::new() }
    }

    pub fn is_dark(&self) -> bool {
        self.columns.iter().all(|c| c.iter().all(|v| *v == RgbValue::BLACK))
    }

    /// True when the effect rendered no intents at all, as when the C# throws.
    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }

    pub fn at(&self, frame: usize, x: usize, y: usize) -> RgbValue {
        self.columns[y * self.wi + x][frame]
    }

    /// Whether the pixel at this frame is lit.
    pub fn lit(&self, frame: usize, x: usize, y: usize) -> bool {
        self.at(frame, x, y) != RgbValue::BLACK
    }

    /// The lit pixels of a frame.
    pub fn lit_pixels(&self, frame: usize) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for y in 0..self.ht {
            for x in 0..self.wi {
                if self.lit(frame, x, y) {
                    out.push((x, y));
                }
            }
        }
        out
    }

    /// One frame drawn as text, top row first, for readable test failures.
    pub fn picture(&self, frame: usize) -> String {
        let mut s = String::new();
        for y in (0..self.ht).rev() {
            for x in 0..self.wi {
                s.push(if self.lit(frame, x, y) { '#' } else { '.' });
            }
            s.push('\n');
        }
        s
    }
}

pub fn rgb(r: u8, g: u8, b: u8) -> RgbValue {
    RgbValue { r, g, b }
}

/// A `PixelInfo` for `strings` strings of `pixels` pixels with horizontal strings, as the framework builds it.
pub fn info(strings: usize, pixels: usize, ms: i64) -> PixelInfo {
    struct Grab(Option<PixelInfo>);
    impl PixelEffect for Grab {
        fn setup_render(&mut self, info: &PixelInfo) {
            self.0 = Some(info.clone());
        }
        fn render_effect(&mut self, _f: i32, _i: &PixelInfo, _b: &mut PixelFrameBuffer) {}
    }
    let grid = Grid::new(strings, pixels);
    let targets = [grid.root];
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let ctx = RenderCtx { video_cache: None,
        tree: &grid.tree,
        targets: &targets,
        start_time: TimeSpan::ZERO,
        time_span: TimeSpan::from_ms_i(ms),
        frame_ms: 50,
        marks: &[],
        audio: None,
        module_data_root: None,
        cancel: &cancel,
    };
    let mut g = Grab(None);
    render_pixel(&ctx, StringOrientation::Horizontal, TargetPositioning::Strings, &mut g);
    g.0.expect("setup_render ran")
}

/// Whether a table's members are in DataContractSerializer order: base classes first, each class alphabetical.
pub fn in_contract_order(params: &[ParamDesc]) -> bool {
    params.windows(2).all(|w| (w[0].xml_level, w[0].xml) < (w[1].xml_level, w[1].xml))
}
