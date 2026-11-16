//! Helpers shared by the effect tests.

use vixen_effects::prelude::*;

/// A `PixelInfo` for `strings` strings of `pixels` pixels over `ms` milliseconds, 50 ms frames, as the
/// framework builds it for string rendering.
pub fn info(strings: usize, pixels: usize, ms: i64) -> PixelInfo {
    struct Grab(Option<PixelInfo>);
    impl PixelEffect for Grab {
        fn setup_render(&mut self, info: &PixelInfo) {
            self.0 = Some(info.clone());
        }
        fn render_effect(&mut self, _f: i32, _i: &PixelInfo, _b: &mut PixelFrameBuffer) {}
    }
    let mut tree = ElementTree::new();
    let root = vixen_effects::testing::grid(&mut tree, "G", strings, pixels);
    let targets = [root];
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let ctx = RenderCtx { video_cache: None,
        tree: &tree,
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
    render_pixel(&ctx, StringOrientation::Vertical, TargetPositioning::Strings, &mut g);
    g.0.expect("setup_render ran")
}
