//! The ruler: ticks, labels and their arithmetic, and its input. F97 to F116.

use super::grid_input::PointerEvent;
use super::host::TimelineHost;
use super::marks::mark_label_undo;
use super::model::{HostRequest, InputOutcome, Timeline};
use super::view::{px, time_at, visible_end, TimeInfo};
use super::{Button, CursorKind, Mods};
use uuid::Uuid;
use vixen_core::time::{TICKS_PER_DAY, TICKS_PER_HOUR, TICKS_PER_MIN, TICKS_PER_MS, TICKS_PER_SEC};
use vixen_core::{Color, MarkCollection, TimeSpan};

pub const DEFAULT_RULER_HEIGHT: i32 = 50;
/// `maxDxForClick`.
const MAX_DX_FOR_CLICK: i32 = 2;
/// `minPxBetweenTimeLabels`.
const MIN_PX_BETWEEN_LABELS: f64 = 10.0;
/// F107: a fixed time window, not pixels. KEEP Q21.
const MARK_HIT_WINDOW: TimeSpan = TimeSpan::from_ms_i(20);
/// F113: the height doesn't follow below this.
const RULER_MIN_DRAG_Y: i32 = 40;

/// `m_MinorTick` and `m_minorTicksPerMajor`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct RulerScale {
    pub minor: TimeSpan,
    pub minor_per_major: i64,
}

impl RulerScale {
    #[inline]
    pub fn major(&self) -> TimeSpan {
        TimeSpan(self.minor.ticks() * self.minor_per_major)
    }
}

/// `recalculate`, adapted upstream from Audacity's ruler. At least 10 pixels between minor ticks. Exact f64 arithmetic in this order
/// matters at the boundaries.
pub fn ruler_scale(tpp: TimeSpan) -> RulerScale {
    let t = time_at(10, tpp);
    let seconds = t.ticks() as f64 / TICKS_PER_SEC as f64;
    if seconds > 0.05 {
        let minutes = t.ticks() as f64 / TICKS_PER_MIN as f64;
        let hours = t.ticks() as f64 / TICKS_PER_HOUR as f64;
        let days = t.ticks() as f64 / TICKS_PER_DAY as f64;
        let ms = |v: i64| TimeSpan(v * TICKS_PER_MS);
        let s = |v: i64| TimeSpan(v * TICKS_PER_SEC);
        let m = |v: i64| TimeSpan(v * TICKS_PER_MIN);
        let h = |v: i64| TimeSpan(v * TICKS_PER_HOUR);
        let (minor, minor_per_major) = if seconds < 0.1 {
            (ms(100), 5)
        } else if seconds < 0.25 {
            (ms(250), 4)
        } else if seconds < 0.5 {
            (ms(500), 4)
        } else if seconds < 1.0 {
            (s(1), 5)
        } else if seconds < 5.0 {
            (s(5), 6)
        } else if seconds < 10.0 {
            (s(10), 6)
        } else if seconds < 15.0 {
            (s(15), 4)
        } else if seconds < 30.0 {
            (s(30), 4)
        } else if minutes < 1.0 {
            (m(1), 5)
        } else if minutes < 5.0 {
            (m(5), 3)
        } else if minutes < 10.0 {
            (m(10), 3)
        } else if minutes < 15.0 {
            (m(15), 4)
        } else if minutes < 30.0 {
            (m(30), 2)
        } else if hours < 1.0 {
            (h(1), 6)
        } else if hours < 6.0 {
            (h(6), 4)
        } else if days < 1.0 {
            (TimeSpan(TICKS_PER_DAY), 7)
        } else {
            (TimeSpan(TICKS_PER_DAY * 7), 1)
        };
        return RulerScale { minor, minor_per_major };
    }
    // Fractional seconds.
    let of = |d: f64| TimeSpan((TICKS_PER_MS as f64 * 1000.0 * d) as i64);
    let mut d = 0.000001f64;
    loop {
        if seconds < d {
            return RulerScale { minor: of(d), minor_per_major: 5 };
        }
        d *= 5.0;
        if seconds < d {
            return RulerScale { minor: of(d), minor_per_major: 5 };
        }
        d *= 5.0;
        if seconds < d {
            return RulerScale { minor: of(d), minor_per_major: 4 };
        }
        d *= 4.0;
    }
}

/// Components, not totals, as `TimeSpan.ToString` gives them.
#[derive(Copy, Clone)]
struct Parts {
    hours: i64,
    minutes: i64,
    seconds: i64,
    /// Ticks below a second.
    fraction: i64,
}

fn parts(t: TimeSpan) -> Parts {
    let ticks = t.ticks().abs();
    Parts {
        hours: (ticks / TICKS_PER_HOUR) % 24,
        minutes: (ticks / TICKS_PER_MIN) % 60,
        seconds: (ticks / TICKS_PER_SEC) % 60,
        fraction: ticks % TICKS_PER_SEC,
    }
}

/// `labelString`: F99's table. Fractions are truncated, not rounded.
pub fn ruler_label(t: TimeSpan, minor: TimeSpan) -> String {
    let ms = |v: i64| TimeSpan(v * TICKS_PER_MS);
    let hour = TimeSpan(TICKS_PER_HOUR);
    let minute = TimeSpan(TICKS_PER_MIN);
    let second = TimeSpan(TICKS_PER_SEC);
    if minor >= hour {
        let p = parts(TimeSpan((t.ticks() / TICKS_PER_HOUR) * TICKS_PER_HOUR));
        return format!("{}:{:02}", p.hours, p.minutes);
    }
    if minor >= minute || minor >= second {
        let whole = if minor >= minute { TICKS_PER_MIN } else { TICKS_PER_SEC };
        let t = TimeSpan((t.ticks() / whole) * whole);
        let p = parts(t);
        return if t >= hour {
            format!("{}:{:02}:{:02}", p.hours, p.minutes, p.seconds)
        } else {
            format!("{}:{:02}", p.minutes, p.seconds)
        };
    }
    let p = parts(t);
    let fraction = if minor >= ms(100) {
        format!("{}", p.fraction / (TICKS_PER_SEC / 10))
    } else if minor >= ms(10) {
        format!("{:02}", p.fraction / (TICKS_PER_SEC / 100))
    } else if minor >= ms(1) {
        format!("{:03}", p.fraction / (TICKS_PER_SEC / 1000))
    } else {
        format!("{:06}", p.fraction / (TICKS_PER_SEC / 1_000_000))
    };
    if t >= hour {
        format!("{}:{:02}:{:02}.{fraction}", p.hours, p.minutes, p.seconds)
    } else if t >= minute {
        format!("{}:{:02}.{fraction}", p.minutes, p.seconds)
    } else {
        format!("{}.{fraction}", p.seconds)
    }
}

/// "It is the first multiple of interval greater than start."
#[inline]
pub fn first_tick_px(start_px: f64, step_px: f64) -> f64 {
    start_px - start_px % step_px + step_px
}

/// F99's drawn interval: the major tick repeated often enough that labels of the end label's width don't touch.
pub fn label_interval(major: TimeSpan, tpp: TimeSpan, end_label_width: f64) -> TimeSpan {
    let major_px = px(major, tpp);
    if major_px <= 0.0 {
        return major;
    }
    let n = ((end_label_width + MIN_PX_BETWEEN_LABELS) / major_px) as i64 + 1;
    TimeSpan(major.ticks() * n)
}

/// The times labels are drawn at in a view. Swift measures and applies the overlap rule, or `ruler_frame` does with the measured metrics.
pub fn label_times(visible_start: TimeSpan, visible_end: TimeSpan, major: TimeSpan, tpp: TimeSpan, end_label_width: f64) -> Vec<TimeSpan> {
    let interval = label_interval(major, tpp, end_label_width);
    let mut out = Vec::new();
    if interval.ticks() <= 0 {
        return out;
    }
    let mut t = TimeSpan(visible_start.ticks() - visible_start.ticks() % interval.ticks() + interval.ticks());
    while t <= visible_end {
        out.push(t);
        t += interval;
    }
    out
}

/// F110: the time beside the pointer while a mark drags.
pub fn mark_drag_label(t: TimeSpan) -> String {
    let p = parts(t);
    let millis = p.fraction / TICKS_PER_MS;
    if p.minutes >= 10 {
        format!("{:02}:{:02}.{millis:03}", p.minutes, p.seconds)
    } else if p.minutes >= 1 {
        format!("{}:{:02}.{millis:03}", p.minutes, p.seconds)
    } else if p.seconds >= 10 {
        format!("{:02}.{millis:03}", p.seconds)
    } else {
        format!("{}.{millis:03}", p.seconds)
    }
}

/// F107: unlocked collections that are visible, highest level first; marks within ±20 ms of a time.
pub fn marks_at(collections: &[MarkCollection], t: TimeSpan) -> Vec<Uuid> {
    let mut order: Vec<&MarkCollection> =
        collections.iter().filter(|c| (c.show_grid_lines || c.show_marks) && !c.is_locked).collect();
    // Stable, as `OrderByDescending` is.
    order.sort_by_key(|c| std::cmp::Reverse(c.level));
    let mut out = Vec::new();
    for c in order {
        let from = c.marks.partition_point(|m| m.start_time < t - MARK_HIT_WINDOW);
        for m in &c.marks[from..] {
            if m.start_time > t + MARK_HIT_WINDOW {
                break;
            }
            out.push(m.id);
        }
    }
    out
}

/// Glyph widths for the label font, measured once by Swift. Digits are monospaced, so any label's width follows from these, and a view
/// change needs no text measurement across the bindings.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct LabelMetrics {
    pub digit: f64,
    pub colon: f64,
    pub dot: f64,
}

impl Default for LabelMetrics {
    fn default() -> Self {
        LabelMetrics { digit: 8.0, colon: 4.0, dot: 4.0 }
    }
}

impl LabelMetrics {
    pub fn width(&self, label: &str) -> f64 {
        label
            .chars()
            .map(|c| match c {
                ':' => self.colon,
                '.' => self.dot,
                _ => self.digit,
            })
            .sum()
    }
}

/// A line for one mark on the ruler. F106.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct RulerMarkLine {
    pub x: f64,
    pub color: Color,
    pub width: i32,
    pub dash: i32,
}

/// F98, F99, F104, F106 for a span of ruler columns.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RulerFrame {
    pub major_ticks: Vec<f64>,
    pub minor_ticks: Vec<f64>,
    /// Centre x and text.
    pub labels: Vec<(f64, String)>,
    pub mark_lines: Vec<RulerMarkLine>,
    /// `px(TotalTime)`, where the bottom line ends.
    pub total_px: f64,
    pub range_start: Option<i32>,
    pub range_end: Option<i32>,
}

/// `Ruler.MouseState`.
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub(crate) enum RulerMouse {
    #[default]
    Normal,
    DragWait,
    Dragging,
    DraggingMark,
    Resize,
}

/// The ruler's own state.
pub struct RulerState {
    pub height: i32,
    pub locked: bool,
    pub metrics: LabelMetrics,
    pub(crate) mouse: RulerMouse,
    pub(crate) down_x: i32,
    pub(crate) button: Option<Button>,
    pub(crate) down_mark: Option<Uuid>,
    pub(crate) drag_last: TimeSpan,
    /// The pointer is over the strip. `Cursor == Cursors.HSplit`.
    pub(crate) strip: bool,
    pub(crate) cursor: CursorKind,
    /// `markTime` and the pointer's canvas x, while a mark drags.
    pub drag_label: Option<(f64, String)>,
    /// `_mPrevPlaybackStart`, `_mPrevPlaybackEnd`: what comes back when playback ends.
    pub(crate) remembered: (Option<TimeSpan>, Option<TimeSpan>),
    /// The drawn interval the tiles were painted with.
    pub(crate) painted_interval: TimeSpan,
}

impl Default for RulerState {
    fn default() -> Self {
        RulerState {
            height: DEFAULT_RULER_HEIGHT,
            locked: false,
            metrics: LabelMetrics::default(),
            mouse: RulerMouse::Normal,
            down_x: 0,
            button: None,
            down_mark: None,
            drag_last: TimeSpan::ZERO,
            strip: false,
            cursor: CursorKind::Hand,
            drag_label: None,
            remembered: (None, None),
            painted_interval: TimeSpan::ZERO,
        }
    }
}

impl Timeline {
    /// `PixelsToTime(e.X) + VisibleTimeStart`. KEEP: not clamped to the sequence.
    #[inline]
    fn ruler_time(&self, x: i32) -> TimeSpan {
        self.time(x) + self.info.visible_start
    }

    /// The interval labels repeat at for the current view. Depends on how wide the end time's label is, so it can change on a scroll.
    pub fn current_label_interval(&self) -> TimeSpan {
        self.label_interval_at(self.tpp())
    }

    fn label_interval_at(&self, tpp: TimeSpan) -> TimeSpan {
        let scale = ruler_scale(tpp);
        let info = TimeInfo { time_per_pixel: tpp, ..self.info };
        let width = self.marks.ruler.metrics.width(&ruler_label(visible_end(&info, self.grid_width), scale.minor));
        label_interval(scale.major(), tpp, width)
    }

    /// F98 and F99 for a span of the canvas. Upstream starts strictly after the visible start and stops at the visible end; a tile covers
    /// more than that, and only what's in view shows.
    pub fn ruler_frame(&self, x: i32, width: i32) -> RulerFrame {
        self.ruler_frame_at(x, width, self.tpp())
    }

    /// The same at a given zoom. See `grid_frame_at`.
    pub fn ruler_frame_at(&self, x: i32, width: i32, tpp: TimeSpan) -> RulerFrame {
        let scale = ruler_scale(tpp);
        let total_px = px(self.info.total, tpp);
        let mut frame = RulerFrame { total_px, ..RulerFrame::default() };
        let (left, right) = (x as f64, (x + width) as f64);
        let ticks = |step: TimeSpan, out: &mut Vec<f64>| {
            let step_px = px(step, tpp);
            if step_px <= 0.0 {
                return;
            }
            let mut k = (left / step_px).floor().max(1.0);
            loop {
                let tick = k * step_px;
                if tick > right + 1.0 || tick > total_px {
                    break;
                }
                out.push(tick);
                k += 1.0;
            }
        };
        ticks(scale.major(), &mut frame.major_ticks);
        ticks(scale.minor, &mut frame.minor_ticks);

        let metrics = self.marks.ruler.metrics;
        let interval = self.label_interval_at(tpp);
        if interval.ticks() > 0 {
            let interval_px = px(interval, tpp);
            // From one label before the span, so a label straddling its edge is drawn in both tiles.
            let mut k = ((left / interval_px).floor() - 1.0).max(1.0) as i64;
            let mut last = 0.0f64;
            loop {
                let t = TimeSpan(interval.ticks() * k);
                let centre = px(t, tpp);
                if t > self.info.total {
                    break;
                }
                let text = ruler_label(t, scale.minor);
                let half = metrics.width(&text) / 2.0;
                if centre - half > right {
                    break;
                }
                if last + MIN_PX_BETWEEN_LABELS + half < centre {
                    last = (centre + half).trunc();
                    if centre + half >= left {
                        frame.labels.push((centre, text));
                    }
                }
                k += 1;
            }
        }

        let (t0, t1) = (time_at(x - 3, tpp), time_at(x + width + 3, tpp));
        for c in &self.marks.collections {
            if !(c.show_grid_lines || c.show_marks) {
                continue;
            }
            let from = c.marks.partition_point(|m| m.start_time < t0);
            for m in &c.marks[from..] {
                if m.start_time > t1 {
                    break;
                }
                frame.mark_lines.push(RulerMarkLine {
                    x: px(m.start_time, tpp),
                    color: c.decorator.color,
                    // KEEP Q20: bold is overwritten per mark, so it has no effect here.
                    width: if self.marks.is_selected(m.id) { 3 } else { 1 },
                    // GUARD: Level 0 makes an invalid dash pattern upstream.
                    dash: if c.decorator.is_solid_line { 0 } else { c.level.max(1) },
                });
            }
        }
        frame.range_start = self.info.playback_start.map(|t| px(t, tpp) as i32);
        frame.range_end = self.info.playback_end.map(|t| px(t, tpp) as i32);
        frame
    }

    /// `Ruler.OnMouseDown`. F100, F109, F113. Any button can press a mark.
    pub fn ruler_mouse_down(&mut self, ev: PointerEvent) -> InputOutcome {
        self.refresh_view_stacks();
        self.marks.ruler.button = ev.button;
        self.marks.ruler.down_x = ev.x;
        self.ruler_update_strip(ev.y);
        let hits = marks_at(&self.marks.collections, self.ruler_time(ev.x));
        if let Some(first) = hits.first().copied() {
            if !ev.mods.exactly(false, true, false) {
                self.marks_clear_selected();
            }
            if self.marks.is_selected(first) {
                self.marks_deselect(first);
            } else {
                self.marks_select(first);
            }
            self.marks.ruler.down_mark = Some(first);
            self.marks.ruler.drag_last = self.marks.mark(first).map_or(TimeSpan::ZERO, |m| m.start_time);
            self.marks.ruler.mouse = RulerMouse::DraggingMark;
            self.marks.save_selected_times();
            // FIX Q48: the line shows from the press, and the time beside it only from the first move.
            self.marks.ruler.drag_label = Some((px(self.ruler_time(ev.x), self.tpp()), String::new()));
            self.dirty.overlay = true;
        } else if !self.marks.ruler.locked && self.marks.ruler.strip {
            self.marks.ruler.mouse = RulerMouse::Resize;
        } else {
            self.marks_clear_selected();
            self.marks.ruler.mouse = RulerMouse::DragWait;
        }
        self.finish_ruler()
    }

    /// `Ruler.OnMouseMove`.
    pub fn ruler_mouse_move(&mut self, ev: PointerEvent) -> InputOutcome {
        self.refresh_view_stacks();
        if self.marks.ruler.button == Some(Button::Left) {
            match self.marks.ruler.mouse {
                RulerMouse::Normal => {}
                RulerMouse::DragWait => {
                    if (ev.x - self.marks.ruler.down_x).abs() > MAX_DX_FOR_CLICK {
                        self.marks.ruler.mouse = RulerMouse::Dragging;
                        // `timelineControl_RulerBeginDragTimeRange`.
                        self.marks.ruler.remembered = (self.info.playback_start, self.info.playback_end);
                        self.ruler_drag_range(ev.x);
                    }
                }
                RulerMouse::Dragging => self.ruler_drag_range(ev.x),
                RulerMouse::DraggingMark => self.ruler_drag_mark(ev.x),
                RulerMouse::Resize => {
                    // The lock is tested at press and here too.
                    if !self.marks.ruler.locked && ev.y > RULER_MIN_DRAG_Y && self.marks.ruler.height != ev.y + 1 {
                        self.marks.ruler.height = ev.y + 1;
                        self.dirty.view = true;
                        self.dirty.ruler = true;
                    }
                }
            }
        } else {
            let hits = marks_at(&self.marks.collections, self.ruler_time(ev.x));
            if let Some(time) = hits.first().and_then(|id| self.marks.mark(*id)).map(|m| m.start_time) {
                self.marks.ruler.cursor = CursorKind::ResizeLeftRight;
                self.marks.ruler.strip = false;
                self.set_guides(vec![time]);
            } else if !self.marks.ruler.locked && ev.y <= self.marks.ruler.height - 1 && ev.y >= self.marks.ruler.height - 6 {
                self.marks.ruler.cursor = CursorKind::ResizeUpDown;
                self.marks.ruler.strip = true;
            } else {
                self.marks.ruler.cursor = CursorKind::Hand;
                self.marks.ruler.strip = false;
                self.set_guides(Vec::new());
            }
        }
        self.finish_ruler()
    }

    fn ruler_update_strip(&mut self, y: i32) {
        let r = &mut self.marks.ruler;
        r.strip = !r.locked && y <= r.height - 1 && y >= r.height - 6;
    }

    /// F102: the lower and higher of press and pointer, start clamped at 0 and end at the total. The pointer may leave the control, with no
    /// auto-scroll.
    fn ruler_drag_range(&mut self, x: i32) {
        let down = self.marks.ruler.down_x;
        let (a, b) = (down.min(x), down.max(x));
        let start = self.ruler_time(a).max(TimeSpan::ZERO);
        let end = self.ruler_time(b).min(self.info.total);
        self.info.playback_start = Some(start);
        self.info.playback_end = Some(end);
        self.dirty.ruler = true;
    }

    /// F110. KEEP Q22: no clamp, no snap, so a start can go negative, and the first move jumps the pressed mark's start onto the pointer.
    fn ruler_drag_mark(&mut self, x: i32) {
        let new_time = self.ruler_time(x);
        let diff = new_time - self.marks.ruler.drag_last;
        self.marks.ruler.drag_last = new_time;
        let selected = self.marks.selected.clone();
        self.marks_offset_starts(&selected, diff);
        self.marks.ruler.drag_label = Some((px(new_time, self.tpp()), mark_drag_label(new_time)));
        self.marks_moving(&selected);
        if let Some(down) = self.marks.ruler.down_mark {
            let shows_bar = self.marks.collection_of(down).is_some_and(|c| c.show_marks);
            if let Some(m) = self.marks.mark(down) {
                let times = if shows_bar { vec![m.start_time, m.end_time()] } else { vec![m.start_time] };
                self.set_guides(times);
            }
        }
        self.dirty.ruler = true;
        self.dirty.marks_bar = true;
        self.dirty.overlay = true;
    }

    /// `Ruler.OnMouseUp`.
    pub fn ruler_mouse_up(&mut self, ev: PointerEvent, host: &mut dyn TimelineHost) -> InputOutcome {
        self.refresh_view_stacks();
        match self.marks.ruler.button {
            Some(Button::Left) => match self.marks.ruler.mouse {
                RulerMouse::DragWait => self.ruler_clicked(self.ruler_time(ev.x), ev.mods, host),
                RulerMouse::Dragging => self.ruler_range_dragged(ev.mods),
                RulerMouse::DraggingMark => {
                    if !self.marks.selected.is_empty() && ev.x != self.marks.ruler.down_x {
                        self.marks_moved(super::marks::MarkMove::Move, host);
                        self.set_guides(Vec::new());
                    } else {
                        // KEEP Q43: dragged away and back to the press X, the first move's jump stays, with no undo entry. And with no movement at
                        // all, the guides stay until the next hover.
                        if !self.marks.moving_ids_empty() {
                            host.commit_marks_quietly(&self.marks.collections);
                        }
                        self.marks.move_info = None;
                        self.marks_lift_end();
                    }
                }
                RulerMouse::Normal | RulerMouse::Resize => {}
            },
            Some(Button::Right) => {
                let hits = marks_at(&self.marks.collections, self.ruler_time(ev.x));
                self.marks.move_info = None;
                if !hits.is_empty() {
                    if ev.x == self.marks.ruler.down_x {
                        self.requests.push(HostRequest::MarkMenu);
                    }
                } else {
                    self.ruler_right_clicked(self.ruler_time(ev.x), ev.mods);
                }
            }
            _ => {}
        }
        self.marks.ruler.mouse = RulerMouse::Normal;
        self.marks.ruler.button = None;
        if self.marks.ruler.drag_label.take().is_some() {
            self.dirty.overlay = true;
        }
        self.dirty.ruler = true;
        self.finish_ruler()
    }

    /// `timelineControl_RulerClicked` for the left button. F100, F101.
    fn ruler_clicked(&mut self, time: TimeSpan, mods: Mods, host: &mut dyn TimelineHost) {
        if mods.cmd {
            self.marks.ruler.remembered = (self.info.playback_start, self.info.playback_end);
        } else {
            self.marks.ruler.remembered = (Some(time), None);
        }
        self.info.playback_start = Some(time);
        self.info.playback_end = None;
        host.mark_modified();
        if mods.cmd {
            self.requests.push(HostRequest::PlayRange { start: time, end: None });
        } else {
            self.set_cursor(time);
        }
    }

    /// F111's four forms.
    fn ruler_right_clicked(&mut self, time: TimeSpan, mods: Mods) {
        let fill_gap = mods.exactly(false, true, false) || mods.exactly(true, true, false);
        let prompt = mods.exactly(true, false, false) || mods.exactly(true, true, false);
        if prompt {
            self.requests.push(HostRequest::PromptMarkLabel { time, fill_gap });
        } else {
            self.requests.push(HostRequest::AddMark { time, fill_gap, label: None });
        }
    }

    /// `timelineControl_TimeRangeDragged`. F102, F103.
    fn ruler_range_dragged(&mut self, mods: Mods) {
        if mods.cmd {
            if let (Some(start), Some(end)) = (self.info.playback_start, self.info.playback_end) {
                self.requests.push(HostRequest::PlayRange { start, end: Some(end) });
            }
        } else {
            self.marks.ruler.remembered = (self.info.playback_start, self.info.playback_end);
        }
    }

    /// `OnMouseDoubleClick`. F114.
    pub fn ruler_double_click(&mut self) -> InputOutcome {
        if !self.marks.ruler.locked && self.marks.ruler.strip {
            self.marks.ruler.height = DEFAULT_RULER_HEIGHT;
            self.dirty.view = true;
            self.dirty.ruler = true;
        }
        self.finish_ruler()
    }

    /// `OnMouseEnter` and `OnMouseLeave`.
    pub fn ruler_mouse_exited(&mut self) -> InputOutcome {
        self.marks.ruler.cursor = CursorKind::Hand;
        self.marks.ruler.strip = false;
        self.set_guides(Vec::new());
        self.finish_ruler()
    }

    fn finish_ruler(&mut self) -> InputOutcome {
        let cursor = self.marks.ruler.cursor;
        let mut outcome = self.finish();
        outcome.cursor = cursor;
        outcome
    }

    /// F115.
    pub fn set_ruler_locked(&mut self, locked: bool) {
        self.marks.ruler.locked = locked;
    }

    /// The range that comes back when playback ends: what the sequencer's Play From and range commands read.
    pub fn remembered_playback_range(&self) -> (Option<TimeSpan>, Option<TimeSpan>) {
        self.marks.ruler.remembered
    }

    /// F104, as `context_ContextEnded` restores it: the remembered range comes back when playback ends.
    pub fn playback_ended(&mut self) {
        self.info.playback_start = self.marks.ruler.remembered.0;
        self.info.playback_end = self.marks.ruler.remembered.1;
        self.info.playback_current = None;
        self.dirty.ruler = true;
    }

    /// F105's Start button.
    pub fn playback_range_to_start(&mut self) {
        self.info.playback_start = Some(TimeSpan::ZERO);
        self.marks.ruler.remembered.0 = Some(TimeSpan::ZERO);
        self.set_visible_start(TimeSpan::ZERO);
        self.dirty.ruler = true;
    }

    /// F105's End button.
    pub fn playback_range_to_end(&mut self) {
        let total = self.info.total;
        self.info.playback_end = Some(total);
        self.marks.ruler.remembered.1 = Some(total);
        self.set_visible_start(total - self.span());
        self.dirty.ruler = true;
    }

    /// Height the strip takes in the layout.
    pub fn ruler_height(&self) -> i32 {
        self.marks.ruler.height
    }
}

/// The undo description for marks moved or resized, re-exported for the bindings.
pub fn marks_move_label(kind: super::marks::MarkMove, count: usize) -> String {
    mark_label_undo(kind, count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vixen_core::marks::Mark;

    fn ms(v: f64) -> TimeSpan {
        TimeSpan::from_ms(v)
    }

    #[test]
    fn scale_table() {
        let check = |tpp_ms: f64, minor_ms: f64, per: i64| {
            let s = ruler_scale(ms(tpp_ms));
            assert_eq!((s.minor, s.minor_per_major), (ms(minor_ms), per), "tpp {tpp_ms}");
        };
        check(0.5, 10.0, 5);
        check(1.0, 50.0, 5);
        check(2.0, 50.0, 5);
        check(5.0, 250.0, 4);
        check(10.0, 250.0, 4);
        check(20.0, 250.0, 4);
        check(25.0, 500.0, 4);
        check(50.0, 1000.0, 5);
        check(100.0, 5000.0, 6);
        check(250.0, 5000.0, 6);
        check(500.0, 10_000.0, 6);
        check(1000.0, 15_000.0, 4);
        check(5000.0, 60_000.0, 5);
        let s = ruler_scale(TimeSpan(2001));
        assert_eq!((s.minor, s.minor_per_major), (ms(2.5), 4));
    }

    #[test]
    fn labels_follow_the_table() {
        let quarter = ms(250.0);
        assert_eq!(ruler_label(ms(5250.0), quarter), "5.2");
        assert_eq!(ruler_label(ms(65_500.0), quarter), "1:05.5");
        assert_eq!(ruler_label(ms(3_661_000.0), quarter), "1:01:01.0");
        assert_eq!(ruler_label(ms(65_000.0), ms(5000.0)), "1:05");
        assert_eq!(ruler_label(ms(5_400_000.0), TimeSpan(TICKS_PER_HOUR)), "1:00");
        assert_eq!(ruler_label(ms(3_660_000.0), TimeSpan(TICKS_PER_MIN)), "1:01:00");
        assert_eq!(ruler_label(ms(1234.0), ms(50.0)), "1.23");
        assert_eq!(ruler_label(ms(1234.0), ms(5.0)), "1.234");
        assert_eq!(ruler_label(ms(1234.5), ms(0.5)), "1.234500");
    }

    #[test]
    fn first_tick_is_strictly_after_the_start() {
        assert_eq!(first_tick_px(0.0, 100.0), 100.0);
        assert_eq!(first_tick_px(100.0, 100.0), 200.0);
        assert_eq!(first_tick_px(150.0, 100.0), 200.0);
    }

    #[test]
    fn label_times_skip_enough_majors_to_fit() {
        // 10 ms per pixel, major 1 s is 100 px. A label 95 wide needs (95 + 10) / 100 + 1 = 2 majors.
        let tpp = ms(10.0);
        let times = label_times(ms(500.0), ms(5000.0), ms(1000.0), tpp, 95.0);
        assert_eq!(times, [ms(2000.0), ms(4000.0)]);
        let times = label_times(ms(500.0), ms(3000.0), ms(1000.0), tpp, 50.0);
        assert_eq!(times, [ms(1000.0), ms(2000.0), ms(3000.0)]);
    }

    #[test]
    fn mark_drag_labels_have_four_forms() {
        assert_eq!(mark_drag_label(ms(725_250.0)), "12:05.250");
        assert_eq!(mark_drag_label(ms(65_250.0)), "1:05.250");
        assert_eq!(mark_drag_label(ms(12_250.0)), "12.250");
        assert_eq!(mark_drag_label(ms(5250.0)), "5.250");
    }

    fn collection(level: i32, times: &[f64]) -> MarkCollection {
        let mut c = MarkCollection { level, show_grid_lines: true, ..MarkCollection::default() };
        for t in times {
            c.add_mark(Mark::new(ms(*t)));
        }
        c
    }

    #[test]
    fn marks_at_is_a_fixed_window_and_highest_level_first() {
        let low = collection(1, &[1000.0]);
        let high = collection(5, &[1010.0]);
        let (l, h) = (low.marks[0].id, high.marks[0].id);
        let cs = vec![low, high];
        assert_eq!(marks_at(&cs, ms(1005.0)), [h, l]);
        assert_eq!(marks_at(&cs, ms(1020.0)), [h, l]);
        assert_eq!(marks_at(&cs, ms(1021.0)), [h]);
        assert_eq!(marks_at(&cs, ms(979.0)), Vec::<Uuid>::new());
        assert_eq!(marks_at(&cs, ms(980.0)), [l]);
        let mut locked = cs.clone();
        locked[1].is_locked = true;
        assert_eq!(marks_at(&locked, ms(1005.0)), [l]);
        let mut hidden = cs;
        hidden[0].show_grid_lines = false;
        hidden[0].show_marks = false;
        assert_eq!(marks_at(&hidden, ms(1005.0)), [h]);
    }

    fn timeline() -> Timeline {
        let mut t = Timeline::new();
        t.info.time_per_pixel = ms(100.0);
        t.info.total = ms(60_000.0);
        t.set_viewport(100, 100, 100);
        t.marks.ruler.height = 60;
        t
    }

    fn at(x: i32, y: i32, button: Option<Button>) -> PointerEvent {
        PointerEvent::new(x, y, button, Mods::NONE)
    }

    // RulerLockHeightTests.cs
    #[test]
    fn the_strip_shows_a_resize_pointer_unless_locked() {
        let mut t = timeline();
        assert!(!t.marks.ruler.locked);
        assert_eq!(t.ruler_mouse_move(at(10, 55, None)).cursor, CursorKind::ResizeUpDown);
        t.set_ruler_locked(true);
        assert_eq!(t.ruler_mouse_move(at(10, 55, None)).cursor, CursorKind::Hand);
    }

    #[test]
    fn a_lock_set_after_the_drag_starts_stops_the_height() {
        let mut t = timeline();
        t.ruler_mouse_move(at(10, 55, None));
        t.ruler_mouse_down(at(10, 55, Some(Button::Left)));
        t.set_ruler_locked(true);
        t.ruler_mouse_move(at(10, 80, Some(Button::Left)));
        assert_eq!(t.ruler_height(), 60);
    }

    #[test]
    fn dragging_the_strip_follows_above_the_minimum_and_double_click_resets() {
        let mut t = timeline();
        t.ruler_mouse_move(at(10, 55, None));
        t.ruler_mouse_down(at(10, 55, Some(Button::Left)));
        t.ruler_mouse_move(at(10, 80, Some(Button::Left)));
        assert_eq!(t.ruler_height(), 81);
        t.ruler_mouse_move(at(10, 40, Some(Button::Left)));
        assert_eq!(t.ruler_height(), 81);
        let mut host = crate::timeline::tests::RecordingHost::default();
        t.ruler_mouse_up(at(10, 80, Some(Button::Left)), &mut host);
        t.ruler_mouse_move(at(10, 79, None));
        t.ruler_double_click();
        assert_eq!(t.ruler_height(), 50);
    }

    #[test]
    fn a_double_click_on_the_strip_does_nothing_when_locked() {
        let mut t = timeline();
        t.marks.ruler.height = 75;
        t.ruler_mouse_move(at(10, 74, None));
        t.set_ruler_locked(true);
        t.ruler_double_click();
        assert_eq!(t.ruler_height(), 75);
    }

    #[test]
    fn a_click_moves_the_cursor_and_sets_the_range_start() {
        let mut t = timeline();
        let mut host = crate::timeline::tests::RecordingHost::default();
        t.ruler_mouse_down(at(10, 10, Some(Button::Left)));
        // Two points is still a click, three is a drag.
        t.ruler_mouse_move(at(12, 10, Some(Button::Left)));
        t.ruler_mouse_up(at(12, 10, Some(Button::Left)), &mut host);
        assert_eq!(t.cursor, ms(1200.0));
        assert_eq!((t.info.playback_start, t.info.playback_end), (Some(ms(1200.0)), None));
        assert!(host.modified);
    }

    #[test]
    fn dragging_selects_the_same_range_either_way_and_clamps() {
        let mut t = timeline();
        let mut host = crate::timeline::tests::RecordingHost::default();
        t.ruler_mouse_down(at(50, 10, Some(Button::Left)));
        t.ruler_mouse_move(at(53, 10, Some(Button::Left)));
        t.ruler_mouse_move(at(20, 10, Some(Button::Left)));
        assert_eq!((t.info.playback_start, t.info.playback_end), (Some(ms(2000.0)), Some(ms(5000.0))));
        t.ruler_mouse_move(at(80, 10, Some(Button::Left)));
        assert_eq!((t.info.playback_start, t.info.playback_end), (Some(ms(5000.0)), Some(ms(8000.0))));
        t.ruler_mouse_move(at(-30, 10, Some(Button::Left)));
        assert_eq!(t.info.playback_start, Some(TimeSpan::ZERO));
        t.ruler_mouse_up(at(-30, 10, Some(Button::Left)), &mut host);
        assert_eq!(t.marks.ruler.remembered.0, Some(TimeSpan::ZERO));
        assert_eq!(t.cursor, TimeSpan::ZERO);
    }

    #[test]
    fn command_click_plays_and_restores_the_range_afterwards() {
        let mut t = timeline();
        let mut host = crate::timeline::tests::RecordingHost::default();
        t.info.playback_start = Some(ms(100.0));
        t.info.playback_end = Some(ms(200.0));
        let down = PointerEvent::new(10, 10, Some(Button::Left), Mods::CMD);
        t.ruler_mouse_down(down);
        let out = t.ruler_mouse_up(down, &mut host);
        assert_eq!(out.requests, [HostRequest::PlayRange { start: ms(1000.0), end: None }]);
        assert_eq!(t.cursor, TimeSpan::ZERO);
        t.playback_ended();
        assert_eq!((t.info.playback_start, t.info.playback_end), (Some(ms(100.0)), Some(ms(200.0))));
    }
}
