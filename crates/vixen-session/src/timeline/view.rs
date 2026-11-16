//! Shared time state and the arithmetic of time, pixels, zooming and panning: F4 to F8, F11 to F17, F95. All pure.

use vixen_core::TimeSpan;

/// 10 ms per pixel (`TimeInfo.cs:8-11`).
pub const DEFAULT_TIME_PER_PIXEL: TimeSpan = TimeSpan(100_000);
/// Two minutes, until the sequence sets it.
pub const DEFAULT_TOTAL: TimeSpan = TimeSpan(120 * 10_000_000);
/// Zoom is rejected unless the new ticks per pixel are above this.
pub const ZOOM_FLOOR_TICKS: i64 = 2000;

/// `TimeInfo`: one per editor, shared by every surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeInfo {
    pub time_per_pixel: TimeSpan,
    pub visible_start: TimeSpan,
    pub total: TimeSpan,
    pub playback_start: Option<TimeSpan>,
    pub playback_end: Option<TimeSpan>,
    pub playback_current: Option<TimeSpan>,
}

impl Default for TimeInfo {
    fn default() -> Self {
        TimeInfo {
            time_per_pixel: DEFAULT_TIME_PER_PIXEL,
            visible_start: TimeSpan::ZERO,
            total: DEFAULT_TOTAL,
            playback_start: None,
            playback_end: None,
            playback_current: None,
        }
    }
}

/// `timeToPixels`. Upstream computes this in float32; at the zoom floor a one-hour sequence is 18 million pixels, where a
/// float32 step is 2 pixels and hit tests go wrong (FIX Q1). A zero `tpp` throws upstream; here it maps to 0.
#[inline]
pub fn px(t: TimeSpan, tpp: TimeSpan) -> f64 {
    if tpp.ticks() == 0 {
        return 0.0;
    }
    t.ticks() as f64 / tpp.ticks() as f64
}

/// `pixelsToTime`.
#[inline]
pub fn time_at(px: i32, tpp: TimeSpan) -> TimeSpan {
    TimeSpan(px as i64 * tpp.ticks())
}

/// `VisibleTimeSpan`.
#[inline]
pub fn visible_span(width_px: i32, tpp: TimeSpan) -> TimeSpan {
    time_at(width_px, tpp)
}

/// `VisibleTimeEnd`.
#[inline]
pub fn visible_end(info: &TimeInfo, width_px: i32) -> TimeSpan {
    (info.visible_start + visible_span(width_px, info.time_per_pixel)).min(info.total)
}

/// The `VisibleTimeStart` setter. Every view change goes through this.
#[inline]
pub fn clamp_visible_start(v: TimeSpan, total: TimeSpan, span: TimeSpan) -> TimeSpan {
    let v = v.max(TimeSpan::ZERO);
    if v > total - span {
        (total - span).max(TimeSpan::ZERO)
    } else {
        v
    }
}

/// `TimeSpan.Scale(double)`.
#[inline]
pub fn scale_span(t: TimeSpan, scale: f64) -> TimeSpan {
    TimeSpan((t.ticks() as f64 * scale) as i64)
}

/// `TryGetZoomTimePerPixel`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ZoomDecision {
    Rejected,
    Scaled(TimeSpan),
    ShowsTotal(TimeSpan),
}

pub fn zoom_decision(tpp: TimeSpan, span: TimeSpan, total: TimeSpan, grid_width: i32, scale: f64) -> ZoomDecision {
    if scale <= 0.0 || grid_width <= 0 {
        return ZoomDecision::Rejected;
    }
    let (new_tpp, shows_total) = if scale_span(span, scale) > total {
        // Integer division, so the full zoom-out span can be a few ticks short of the total. KEEP.
        (TimeSpan(total.ticks() / grid_width as i64), true)
    } else {
        (scale_span(tpp, scale), false)
    };
    if new_tpp.ticks() <= ZOOM_FLOOR_TICKS {
        return ZoomDecision::Rejected;
    }
    if shows_total {
        ZoomDecision::ShowsTotal(new_tpp)
    } else {
        ZoomDecision::Scaled(new_tpp)
    }
}

/// `TimelineControl.Zoom`: the left-edge time stays put. Returns the new time per pixel and start.
pub fn zoom(info: &TimeInfo, grid_width: i32, scale: f64) -> Option<(TimeSpan, TimeSpan)> {
    let span = visible_span(grid_width, info.time_per_pixel);
    match zoom_decision(info.time_per_pixel, span, info.total, grid_width, scale) {
        ZoomDecision::Rejected => None,
        ZoomDecision::ShowsTotal(tpp) => Some((tpp, TimeSpan::ZERO)),
        ZoomDecision::Scaled(tpp) => {
            let new_span = visible_span(grid_width, tpp);
            let mut start = info.visible_start;
            if (start + new_span).min(info.total) >= info.total {
                start = clamp_visible_start(info.total - new_span, info.total, new_span);
            }
            Some((tpp, start))
        }
    }
}

/// `TimelineControl.ZoomTime`: holds the time under the pointer steady. `pointer_offset_px` is the pointer's distance from
/// the splitter. KEEP: the fraction ignores the splitter's own width and uses the panel width including any scrollbar.
pub fn zoom_at(info: &TimeInfo, grid_width: i32, panel_width: i32, pointer_offset_px: i32, scale: f64) -> Option<(TimeSpan, TimeSpan)> {
    let span = visible_span(grid_width, info.time_per_pixel);
    match zoom_decision(info.time_per_pixel, span, info.total, grid_width, scale) {
        ZoomDecision::Rejected => None,
        ZoomDecision::ShowsTotal(tpp) => Some((tpp, TimeSpan::ZERO)),
        ZoomDecision::Scaled(tpp) => {
            if panel_width <= 0 {
                return None;
            }
            let new_span = visible_span(grid_width, tpp);
            let fraction = pointer_offset_px as f64 / panel_width as f64;
            let start = if scale > 1.0 {
                info.visible_start - time_at((px(new_span - span, tpp) * fraction) as i32, tpp)
            } else {
                info.visible_start + time_at((px(span - new_span, tpp) * fraction) as i32, tpp)
            };
            Some((tpp, clamp_visible_start(start, info.total, new_span)))
        }
    }
}

/// F11.
#[inline]
pub fn wheel_zoom_scale(delta: i32) -> f64 {
    1.0 - delta as f64 / 1200.0
}

/// `PanTimelineHorizontally`, unclamped.
#[inline]
pub fn wheel_pan(start: TimeSpan, span: TimeSpan, delta: i32) -> TimeSpan {
    start + scale_span(span, -(delta as f64 / 1200.0))
}

/// `PanTimelineHorizontallyFromNativeWheel`, unclamped. 10% whatever the size.
#[inline]
pub fn tilt_pan(start: TimeSpan, span: TimeSpan, delta: i32) -> TimeSpan {
    start + scale_span(span, if delta > 0 { 0.10 } else { -0.10 })
}

/// Integer division, so a notch of 120 is 40 points.
#[inline]
pub fn vertical_wheel_delta(delta: i32) -> i32 {
    -(delta / 3)
}

/// `OnPlaybackCurrentTimeChanged`: a page flip at 90% of the view. The caller clamps.
pub fn playback_follow(position: TimeSpan, start: TimeSpan, span: TimeSpan) -> Option<TimeSpan> {
    if position > start + scale_span(span, 0.9) || position < start {
        Some(position - scale_span(span, 0.1))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn ms(v: i64) -> TimeSpan {
        TimeSpan::from_ms_i(v)
    }

    fn sec(v: i64) -> TimeSpan {
        ms(v * 1000)
    }

    #[test]
    fn default_mapping() {
        let tpp = TimeInfo::default().time_per_pixel;
        assert_eq!(tpp.ticks(), 100_000);
        assert_eq!(time_at(10, tpp), ms(100));
        assert_eq!(px(sec(1), tpp), 100.0);
        assert_eq!(px(sec(1), TimeSpan::ZERO), 0.0);
    }

    #[test]
    fn clamping_visible_start() {
        assert_eq!(clamp_visible_start(ms(-5), sec(60), sec(10)), TimeSpan::ZERO);
        assert_eq!(clamp_visible_start(sec(55), sec(60), sec(10)), sec(50));
        assert_eq!(clamp_visible_start(sec(3), sec(5), sec(10)), TimeSpan::ZERO);
        assert_eq!(clamp_visible_start(sec(30), sec(5), sec(10)), TimeSpan::ZERO);
    }

    fn info(tpp: i64, start: TimeSpan, total: TimeSpan) -> TimeInfo {
        TimeInfo { time_per_pixel: TimeSpan(tpp), visible_start: start, total, ..TimeInfo::default() }
    }

    // TimelineZoomTests.cs
    #[test]
    fn zoom_below_the_floor_is_rejected_and_changes_nothing() {
        let i = info(2222, sec(1), sec(100));
        assert_eq!((2222.0f64 * 0.9) as i64, 1999);
        assert_eq!(zoom(&i, 1000, 0.9), None);
        assert_eq!(zoom_at(&i, 1000, 1000, 400, 0.9), None);
    }

    #[test]
    fn zoom_out_past_total_shows_everything() {
        let i = info(100_000, sec(1), sec(10));
        let want = Some((TimeSpan(sec(10).ticks() / 900), TimeSpan::ZERO));
        assert_eq!(zoom(&i, 900, 1.25), want);
        assert_eq!(zoom_at(&i, 900, 900, 100, 1.25), want);
    }

    #[test]
    fn zoom_at_moves_the_start_later_on_zoom_in() {
        let i = info(100_000, sec(10), sec(1000));
        let (tpp, start) = zoom_at(&i, 800, 800, 400, 0.9).unwrap();
        assert_eq!(tpp.ticks(), 90_000);
        assert!(start > sec(10), "{start:?}");
        // The time under the pointer stays where it was, to a pixel.
        let before = i.visible_start + time_at(400, i.time_per_pixel);
        let after = start + time_at(400, tpp);
        assert!((before - after).ticks().abs() <= tpp.ticks());
    }

    #[test]
    fn zoom_holds_the_left_edge_unless_it_would_pass_the_end() {
        let i = info(100_000, sec(10), sec(1000));
        assert_eq!(zoom(&i, 800, 0.8).unwrap().1, sec(10));
        let i = info(100_000, sec(990), sec(1000));
        let (tpp, start) = zoom(&i, 800, 1.25).unwrap();
        assert_eq!(start, sec(1000) - visible_span(800, tpp));
    }

    #[test]
    fn zero_scale_and_zero_width_are_rejected() {
        let i = info(100_000, sec(1), sec(100));
        assert_eq!(zoom(&i, 800, 0.0), None);
        assert_eq!(zoom(&i, 800, -1.0), None);
        assert_eq!(zoom(&i, 0, 0.5), None);
    }

    // MarksBarMouseWheelTests.cs
    #[test]
    fn wheel_formulas() {
        assert_eq!(wheel_zoom_scale(120), 0.9);
        assert_eq!(wheel_zoom_scale(-120), 1.1);
        let span = sec(100);
        assert_eq!(wheel_pan(sec(50), span, -120), sec(60));
        assert_eq!(wheel_pan(sec(50), span, 120), sec(40));
        assert_eq!(wheel_pan(sec(50), span, -30), sec(50) + ms(2500));
        assert_eq!(tilt_pan(sec(50), span, 120), sec(60));
        assert_eq!(tilt_pan(sec(50), span, -120), sec(40));
        assert_eq!(tilt_pan(sec(50), span, -1), sec(40));
        assert_eq!(vertical_wheel_delta(120), -40);
        assert_eq!(vertical_wheel_delta(-120), 40);
    }

    #[test]
    fn playback_follow_flips_at_ninety_percent() {
        let (start, span) = (TimeSpan::ZERO, ms(100));
        assert_eq!(playback_follow(ms(90), start, span), None);
        assert_eq!(playback_follow(ms(91), start, span), Some(ms(81)));
        assert_eq!(playback_follow(ms(40), ms(50), span), Some(ms(30)));
    }
}
