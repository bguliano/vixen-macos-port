//! The waveform strip: F140 to F148. Peaks are computed once on a worker, and any zoom reads them in O(columns * log n).

use super::IRect;
use super::view::px;
use crate::timeline::model::Timeline;
use std::sync::Arc;
pub use vixen_audio::AudioClip;
use vixen_audio::Peaks;
use vixen_core::TimeSpan;

/// `MinimumHeight`. KEEP: upstream's comment says 40 and the constant is 30. The ruler's limit is 40.
pub const WAVEFORM_MIN_DRAG_Y: i32 = 30;
pub const DEFAULT_WAVEFORM_HEIGHT: i32 = 50;

/// `WaveformStyle`.
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum WaveformStyle {
    #[default]
    Half,
    Full,
}

/// The waveform's own state.
pub struct WaveformState {
    pub height: i32,
    pub locked: bool,
    pub style: WaveformStyle,
    /// Set once the worker has finished. Until then "Building waveform....." shows (F142).
    pub peaks: Option<Arc<Peaks>>,
    pub has_audio: bool,
    pub media_duration: TimeSpan,
    pub(crate) hover_strip: bool,
}

impl Default for WaveformState {
    fn default() -> Self {
        WaveformState {
            height: DEFAULT_WAVEFORM_HEIGHT,
            locked: false,
            style: WaveformStyle::Half,
            peaks: None,
            has_audio: false,
            media_duration: TimeSpan::ZERO,
            hover_strip: false,
        }
    }
}

/// One pass over the samples. Run it on a worker: F142's message shows until it's done.
pub fn build_peaks(clip: Arc<AudioClip>) -> Arc<Peaks> {
    Arc::new(Peaks::compute(clip))
}

/// `GetVisibleSampleRange`: the half-open range of columns to draw for a client clip.
pub fn visible_sample_range(samples: usize, media_duration: TimeSpan, visible_start: TimeSpan, tpp: TimeSpan, clip_left: i32, clip_right: i32) -> (usize, usize) {
    let start_px = px(visible_start, tpp).floor() as i64;
    let media_px = px(media_duration, tpp).floor() as i64;
    let maximum = (samples as i64).min(media_px).max(0);
    let start = (start_px + clip_left as i64 - 1).clamp(0, maximum);
    let end = (start_px + clip_right as i64 + 1).clamp(start, maximum);
    (start as usize, end as usize)
}

/// F141: one `(min, max)` per column over `[column * tpp, (column + 1) * tpp)`.
pub fn waveform_columns(peaks: &Peaks, first_column: i64, columns: usize, tpp: TimeSpan) -> Vec<(f32, f32)> {
    let start = TimeSpan(first_column * tpp.ticks());
    let end = TimeSpan((first_column + columns as i64) * tpp.ticks());
    peaks.range(start, end, columns)
}

/// `GetAlignmentInvalidationRectangle` for one time. None if it's out of the client area.
pub fn guide_dirty_rect(t: TimeSpan, visible_start: TimeSpan, tpp: TimeSpan, width: i32, height: i32) -> Option<IRect> {
    let x = px(t - visible_start, tpp).floor() as i32;
    let left = (x - 2).max(0);
    let right = (x + 3).min(width);
    (right > left).then(|| IRect::new(left, 0, right - left, height))
}

/// The same for several guides: the union of those in view.
pub fn guides_dirty_rect(times: &[TimeSpan], visible_start: TimeSpan, tpp: TimeSpan, width: i32, height: i32) -> Option<IRect> {
    let mut out: Option<IRect> = None;
    for t in times {
        if let Some(r) = guide_dirty_rect(*t, visible_start, tpp, width, height) {
            out = Some(out.map_or(r, |o| o.union(&r)));
        }
    }
    out
}

/// `workingHeight`, and the column's scale and baseline for a style.
pub fn waveform_metrics(height: i32, style: WaveformStyle) -> (i32, i32) {
    let working = height - 2 - height % 2;
    match style {
        WaveformStyle::Half => (working, working),
        WaveformStyle::Full => (working / 2, working / 2),
    }
}

/// One tile's columns as the strip paints them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WaveformFrame {
    pub building: bool,
    pub first_column: i32,
    pub top: i32,
    pub baseline: i32,
    pub full: bool,
    /// Height of each column above the baseline.
    pub highs: Vec<f32>,
    /// Depth of each column below the baseline, for Full only. Min is negative, so this is positive going down.
    pub lows: Vec<f32>,
}

impl Timeline {
    /// F141 for a span of canvas columns. Nothing past the media's end.
    pub fn waveform_frame(&self, first_column: i32, columns: i32) -> WaveformFrame {
        self.waveform_frame_at(first_column, columns, self.tpp())
    }

    /// The same at a given zoom. See `grid_frame_at`.
    pub fn waveform_frame_at(&self, first_column: i32, columns: i32, tpp: TimeSpan) -> WaveformFrame {
        let (top, baseline) = waveform_metrics(self.waveform.height, self.waveform.style);
        let full = self.waveform.style == WaveformStyle::Full;
        let mut frame = WaveformFrame { building: false, first_column, top, baseline, full, ..WaveformFrame::default() };
        let Some(peaks) = &self.waveform.peaks else {
            frame.building = self.waveform.has_audio;
            return frame;
        };
        let media_columns = px(self.waveform.media_duration, tpp).floor() as i64;
        let first = (first_column as i64).max(0);
        let last = (first_column as i64 + columns as i64).min(media_columns);
        if last <= first {
            return frame;
        }
        frame.first_column = first as i32;
        let values = waveform_columns(peaks, first, (last - first) as usize, tpp);
        frame.highs = values.iter().map(|(_, high)| high * top as f32).collect();
        if full {
            frame.lows = values.iter().map(|(low, _)| -low * top as f32).collect();
        }
        frame
    }

    /// F141, F142: while peaks build, or once the view is past the media's end, the strip shows no guides or cursor either.
    pub fn waveform_blank(&self) -> bool {
        self.waveform.has_audio && (self.waveform.peaks.is_none() || self.info.visible_start > self.waveform.media_duration)
    }

    /// `OnMouseMove`: the bottom strip is an up-down pointer and drags the height, otherwise a hand. F145.
    pub fn waveform_mouse_move(&mut self, y: i32, left_down: bool) -> super::CursorKind {
        let w = &mut self.waveform;
        if !w.locked && left_down && w.hover_strip && y > WAVEFORM_MIN_DRAG_Y {
            if w.height != y + 1 {
                w.height = y + 1;
                self.dirty.view = true;
                self.dirty.waveform = true;
            }
            return super::CursorKind::ResizeUpDown;
        }
        w.hover_strip = !w.locked && y <= w.height - 1 && y >= w.height - 6;
        if w.hover_strip {
            super::CursorKind::ResizeUpDown
        } else {
            super::CursorKind::Hand
        }
    }

    /// F146.
    pub fn waveform_double_click(&mut self) {
        if !self.waveform.locked && self.waveform.hover_strip {
            self.waveform.height = DEFAULT_WAVEFORM_HEIGHT;
            self.dirty.view = true;
            self.dirty.waveform = true;
        }
    }

    /// F147.
    pub fn set_waveform_locked(&mut self, locked: bool) {
        self.waveform.locked = locked;
    }

    /// F148.
    pub fn set_waveform_style(&mut self, style: WaveformStyle) {
        if self.waveform.style != style {
            self.waveform.style = style;
            self.dirty.waveform = true;
        }
    }

    /// F140.
    pub fn set_audio(&mut self, has_audio: bool, media_duration: TimeSpan) {
        self.waveform.has_audio = has_audio;
        self.waveform.media_duration = media_duration;
        if !has_audio {
            self.waveform.peaks = None;
        }
        self.dirty.view = true;
        self.dirty.waveform = true;
    }

    /// The worker has finished.
    pub fn set_peaks(&mut self, peaks: Option<Arc<Peaks>>) {
        self.waveform.peaks = peaks;
        self.dirty.waveform = true;
    }

    /// Height the strip takes in the layout: none with no audio.
    pub fn waveform_layout_height(&self) -> i32 {
        if self.waveform.has_audio {
            self.waveform.height
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TPP: TimeSpan = TimeSpan(10_000_000); // 1 s

    fn sec(v: i64) -> TimeSpan {
        TimeSpan::from_ms_i(v * 1000)
    }

    // WaveformAlignmentRenderingTests.cs
    #[test]
    fn guide_rectangles_are_narrow_and_clipped() {
        let start = sec(10);
        assert_eq!(guide_dirty_rect(sec(10), start, TPP, 100, 60), Some(IRect::new(0, 0, 3, 60)));
        assert_eq!(guide_dirty_rect(sec(60), start, TPP, 100, 60), Some(IRect::new(48, 0, 5, 60)));
        assert_eq!(guide_dirty_rect(sec(109), start, TPP, 100, 60), Some(IRect::new(97, 0, 3, 60)));
        assert_eq!(guide_dirty_rect(sec(113), start, TPP, 100, 60), None);
        assert_eq!(guides_dirty_rect(&[sec(10), sec(20)], start, TPP, 100, 60), Some(IRect::new(0, 0, 13, 60)));
        assert_eq!(guides_dirty_rect(&[], start, TPP, 100, 60), None);
    }

    #[test]
    fn sample_ranges_clip_to_the_media_and_the_samples() {
        let start = sec(10);
        let media = sec(150);
        assert_eq!(visible_sample_range(120, media, start, TPP, 0, 5), (9, 16));
        assert_eq!(visible_sample_range(120, media, start, TPP, 95, 100), (104, 111));
        assert_eq!(visible_sample_range(120, media, start, TPP, 200, 205), (120, 120));
        assert_eq!(visible_sample_range(120, media, start, TPP, 0, 100), (9, 111));
    }

    #[test]
    fn working_heights() {
        assert_eq!(waveform_metrics(50, WaveformStyle::Half), (48, 48));
        assert_eq!(waveform_metrics(50, WaveformStyle::Full), (24, 24));
        assert_eq!(waveform_metrics(51, WaveformStyle::Half), (48, 48));
    }

    // WaveformLockHeightTests.cs
    #[test]
    fn the_strip_drags_the_height_unless_locked() {
        let mut t = Timeline::new();
        assert!(!t.waveform.locked);
        // The bottom strip shows the resize pointer.
        assert_eq!(t.waveform_mouse_move(47, false), crate::timeline::CursorKind::ResizeUpDown);
        assert_eq!(t.waveform_mouse_move(20, false), crate::timeline::CursorKind::Hand);
        assert_eq!(t.waveform_mouse_move(47, false), crate::timeline::CursorKind::ResizeUpDown);
        t.waveform_mouse_move(70, true);
        assert_eq!(t.waveform.height, 71);
        // Too small to take.
        t.waveform_mouse_move(25, true);
        assert_eq!(t.waveform.height, 71);
    }

    #[test]
    fn a_lock_stops_the_drag_even_after_it_started_and_the_reset() {
        let mut t = Timeline::new();
        t.waveform_mouse_move(47, false);
        t.set_waveform_locked(true);
        t.waveform_mouse_move(70, true);
        assert_eq!(t.waveform.height, 50);
        assert_eq!(t.waveform_mouse_move(47, false), crate::timeline::CursorKind::Hand);
        t.waveform.height = 75;
        t.waveform_double_click();
        assert_eq!(t.waveform.height, 75);
        t.set_waveform_locked(false);
        t.waveform_mouse_move(74, false);
        t.waveform_double_click();
        assert_eq!(t.waveform.height, 50);
    }
}
