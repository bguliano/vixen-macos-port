//! The frame thread: reads the clock, evaluates the schedule and feeds the sinks, on its own high-priority
//! thread so UI work can never make the lights stutter.

use crate::schedule::{ElementMap, EvalScratch, FrameMixer, Schedule};
use parking_lot::{Condvar, Mutex, RwLock};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Weak};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use vixen_core::{CombinedState, TimeSpan};

/// Where playback time comes from: the audio device while there is music, otherwise a monotonic clock.
pub trait TimeSource: Send + Sync {
    fn position(&self) -> TimeSpan;
    fn play(&self);
    fn pause(&self);
    fn seek(&self, to: TimeSpan);
    /// Media clocks can end (or lose their device) before the sequence's nominal end.
    fn finished(&self) -> bool { false }
}

/// A clock for sequences with no audio. `speed` follows the sequencer's speed control.
pub struct InternalClock {
    state: Mutex<ClockState>,
}

struct ClockState {
    base: Instant,
    offset: TimeSpan,
    running: bool,
    speed: f64,
}

impl Default for InternalClock {
    fn default() -> Self {
        InternalClock { state: Mutex::new(ClockState { base: Instant::now(), offset: TimeSpan::ZERO, running: false, speed: 1.0 }) }
    }
}

impl InternalClock {
    pub fn set_speed(&self, speed: f64) {
        let mut s = self.state.lock();
        s.offset = Self::now(&s);
        s.base = Instant::now();
        s.speed = speed;
    }

    fn now(s: &ClockState) -> TimeSpan {
        if s.running {
            s.offset + TimeSpan::from_secs(s.base.elapsed().as_secs_f64() * s.speed)
        } else {
            s.offset
        }
    }
}

impl TimeSource for InternalClock {
    fn position(&self) -> TimeSpan {
        Self::now(&self.state.lock())
    }

    fn play(&self) {
        let mut s = self.state.lock();
        if !s.running {
            s.base = Instant::now();
            s.running = true;
        }
    }

    fn pause(&self) {
        let mut s = self.state.lock();
        s.offset = Self::now(&s);
        s.running = false;
    }

    fn seek(&self, to: TimeSpan) {
        let mut s = self.state.lock();
        s.offset = to;
        s.base = Instant::now();
    }
}

/// Receives each evaluated frame. Controllers and previews implement this. A sink runs on the frame thread,
/// so it must hand slow work off (`vixen_output::ControllerRunner` and `vixen_render::LiveRenderer` do).
pub trait FrameSink: Send {
    fn frame(&mut self, time: TimeSpan, states: &[CombinedState], any_state: bool);
}

/// Playback state as the UI sees it.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Transport {
    Stopped,
    Playing,
    Paused,
}

/// Timing measurements for the Instrumentation window and for tests.
#[derive(Copy, Clone, Debug, Default)]
pub struct EngineStats {
    pub frames: u64,
    /// Frames that started later than one interval after they should have.
    pub late_frames: u64,
    pub last_frame_us: u64,
    pub max_frame_us: u64,
    /// Whole frames since the last `reset_timings`.
    pub frame_timing: TimingTotals,
    /// The schedule's evaluation alone, without what the sinks do with the result.
    pub evaluate_timing: TimingTotals,
}

/// Minimum, maximum and total of a measurement, for an Instrumentation row. `samples` of 0 means none yet.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct TimingTotals {
    pub min_us: u64,
    pub max_us: u64,
    pub total_us: u64,
    pub samples: u64,
}

#[derive(Debug)]
struct Timing {
    min_us: AtomicU64,
    max_us: AtomicU64,
    total_us: AtomicU64,
    samples: AtomicU64,
}

impl Timing {
    fn new() -> Timing {
        Timing {
            min_us: AtomicU64::new(u64::MAX),
            max_us: AtomicU64::new(0),
            total_us: AtomicU64::new(0),
            samples: AtomicU64::new(0),
        }
    }

    fn add(&self, us: u64) {
        self.min_us.fetch_min(us, Ordering::Relaxed);
        self.max_us.fetch_max(us, Ordering::Relaxed);
        self.total_us.fetch_add(us, Ordering::Relaxed);
        self.samples.fetch_add(1, Ordering::Relaxed);
    }

    fn totals(&self) -> TimingTotals {
        TimingTotals {
            min_us: self.min_us.load(Ordering::Relaxed),
            max_us: self.max_us.load(Ordering::Relaxed),
            total_us: self.total_us.load(Ordering::Relaxed),
            samples: self.samples.load(Ordering::Relaxed),
        }
    }

    fn reset(&self) {
        self.min_us.store(u64::MAX, Ordering::Relaxed);
        self.max_us.store(0, Ordering::Relaxed);
        self.total_us.store(0, Ordering::Relaxed);
        self.samples.store(0, Ordering::Relaxed);
    }
}

struct PlayRange {
    start: TimeSpan,
    end: TimeSpan,
    looping: bool,
}

struct Shared {
    elements: RwLock<Option<Arc<ElementMap>>>,
    schedule: RwLock<Option<Arc<Schedule>>>,
    source: RwLock<Option<Arc<dyn TimeSource>>>,
    sinks: Mutex<Vec<Box<dyn FrameSink>>>,
    control: Mutex<Control>,
    wake: Condvar,
    position: AtomicI64,
    interval_us: AtomicI64,
    frames: AtomicU64,
    late_frames: AtomicU64,
    last_frame_us: AtomicU64,
    max_frame_us: AtomicU64,
    frame_timing: Timing,
    evaluate_timing: Timing,
    /// Set by the frame thread when playback reached the end, cleared when the UI reads it.
    finished: AtomicBool,
}

struct Control {
    contexts: Vec<Arc<ContextState>>,
    generation: u64,
    transport: Transport,
    range: PlayRange,
    shutdown: bool,
    /// A single evaluation requested while not playing, for scrubbing or after an edit.
    pending_refresh: bool,
    /// The sinks must be told once that everything is off.
    needs_clear: bool,
}

struct ContextState {
    schedule: RwLock<Arc<Schedule>>,
    source: Arc<dyn TimeSource>,
    end: TimeSpan,
    finished: Mutex<bool>,
    paused: AtomicBool,
    sequence: bool,
    wake: Condvar,
}

impl ContextState {
    fn advances_frames(&self) -> bool {
        !self.paused.load(Ordering::Acquire) && (self.sequence || !self.schedule.read().is_empty())
    }

    fn finish(&self) {
        let mut finished = self.finished.lock();
        if !*finished {
            self.source.pause();
            *finished = true;
            self.wake.notify_all();
        }
    }
}

/// One independently timed sequence. Stopping or dropping it leaves the editor and
/// other scheduled contexts alone. Its waiter is notified on natural end or shutdown.
pub struct ContextPlayback {
    shared: Weak<Shared>,
    state: Arc<ContextState>,
}

impl ContextPlayback {
    pub fn is_finished(&self) -> bool { *self.state.finished.lock() }

    pub fn position(&self) -> TimeSpan { self.state.source.position() }

    pub fn transport(&self) -> Transport {
        if self.is_finished() { Transport::Stopped }
        else if self.state.paused.load(Ordering::Acquire) { Transport::Paused }
        else { Transport::Playing }
    }

    pub fn pause(&self) {
        let finished = self.state.finished.lock();
        if !*finished {
            self.state.source.pause();
            self.state.paused.store(true, Ordering::Release);
        }
        drop(finished);
        self.refresh();
    }

    pub fn resume(&self) {
        let finished = self.state.finished.lock();
        if !*finished {
            self.state.source.play();
            self.state.paused.store(false, Ordering::Release);
        }
        drop(finished);
        self.refresh();
    }

    /// Live contexts replace their immutable schedule without restarting their clock.
    pub fn set_schedule(&self, schedule: Arc<Schedule>) {
        let finished = self.state.finished.lock();
        if !*finished { *self.state.schedule.write() = schedule; }
        drop(finished);
        self.refresh();
    }

    fn refresh(&self) {
        if let Some(shared) = self.shared.upgrade() {
            let mut control = shared.control.lock();
            control.pending_refresh = true;
            shared.wake.notify_all();
        }
    }

    pub fn wait(&self) {
        let mut finished = self.state.finished.lock();
        while !*finished { self.state.wake.wait(&mut finished); }
    }

    pub fn stop(&self) {
        self.state.finish();
        if let Some(shared) = self.shared.upgrade() {
            let mut control = shared.control.lock();
            control.needs_clear = true;
            shared.wake.notify_all();
        }
    }
}

impl Drop for ContextPlayback {
    fn drop(&mut self) { self.stop(); }
}

/// Owns the frame thread.
pub struct Engine {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl Engine {
    /// `interval` is `VixenSystem.DefaultUpdateInterval`.
    pub fn new(interval: Duration) -> Engine {
        let shared = Arc::new(Shared {
            elements: RwLock::new(None),
            schedule: RwLock::new(None),
            source: RwLock::new(None),
            sinks: Mutex::new(Vec::new()),
            control: Mutex::new(Control {
                contexts: Vec::new(),
                generation: 0,
                transport: Transport::Stopped,
                range: PlayRange { start: TimeSpan::ZERO, end: TimeSpan::MAX, looping: false },
                shutdown: false,
                pending_refresh: false,
                needs_clear: false,
            }),
            wake: Condvar::new(),
            position: AtomicI64::new(0),
            interval_us: AtomicI64::new(interval.as_micros() as i64),
            frames: AtomicU64::new(0),
            late_frames: AtomicU64::new(0),
            last_frame_us: AtomicU64::new(0),
            max_frame_us: AtomicU64::new(0),
            frame_timing: Timing::new(),
            evaluate_timing: Timing::new(),
            finished: AtomicBool::new(false),
        });
        let thread_shared = shared.clone();
        let thread = std::thread::Builder::new()
            .name("vixen-frames".to_string())
            .spawn(move || frame_thread(thread_shared))
            .expect("the frame thread must start");
        Engine { shared, thread: Some(thread) }
    }

    pub fn set_interval(&self, interval: Duration) {
        self.shared.interval_us.store(interval.as_micros().max(1) as i64, Ordering::Relaxed);
    }

    /// Publishes a schedule. The frame thread picks it up on its next frame, and a paused view refreshes.
    pub fn set_schedule(&self, schedule: Arc<Schedule>) {
        if self.shared.elements.read().is_none() {
            *self.shared.elements.write() = Some(schedule.elements().clone());
        }
        *self.shared.schedule.write() = Some(schedule);
        self.refresh();
    }

    pub fn schedule(&self) -> Option<Arc<Schedule>> {
        self.shared.schedule.read().clone()
    }

    pub fn set_source(&self, source: Arc<dyn TimeSource>) {
        let mut control = self.shared.control.lock();
        control.generation = control.generation.wrapping_add(1);
        *self.shared.source.write() = Some(source);
    }

    /// Canonical output order, independent of which context is currently playing.
    pub fn set_elements(&self, elements: Arc<ElementMap>) {
        *self.shared.elements.write() = Some(elements);
    }

    pub fn start_context(&self, schedule: Arc<Schedule>, source: Arc<dyn TimeSource>, end: TimeSpan) -> ContextPlayback {
        self.start_context_kind(schedule, source, end, true)
    }

    pub fn start_live_context(&self, schedule: Arc<Schedule>) -> ContextPlayback {
        self.start_context_kind(schedule, Arc::new(InternalClock::default()), TimeSpan::MAX, false)
    }

    fn start_context_kind(&self, schedule: Arc<Schedule>, source: Arc<dyn TimeSource>, end: TimeSpan, sequence: bool) -> ContextPlayback {
        if self.shared.elements.read().is_none() {
            *self.shared.elements.write() = Some(schedule.elements().clone());
        }
        let state = Arc::new(ContextState { schedule: RwLock::new(schedule), source, end, finished: Mutex::new(false), paused: AtomicBool::new(false), sequence, wake: Condvar::new() });
        let mut control = self.shared.control.lock();
        state.source.seek(TimeSpan::ZERO);
        state.source.play();
        control.contexts.push(state.clone());
        self.shared.wake.notify_all();
        ContextPlayback { shared: Arc::downgrade(&self.shared), state }
    }

    pub fn has_contexts(&self) -> bool {
        self.shared.control.lock().contexts.iter().any(|c| !*c.finished.lock())
    }

    pub fn has_sequence_contexts(&self) -> bool {
        self.shared.control.lock().contexts.iter().any(|c| c.sequence && !*c.finished.lock())
    }

    /// Closing execution or replacing a display ends every context and wakes its owner.
    pub fn stop_all(&self) {
        self.stop();
        let mut control = self.shared.control.lock();
        for context in control.contexts.drain(..) { context.finish(); }
        control.needs_clear = true;
        self.shared.wake.notify_all();
    }

    pub fn add_sink(&self, sink: Box<dyn FrameSink>) {
        self.shared.sinks.lock().push(sink);
    }

    pub fn clear_sinks(&self) {
        self.shared.sinks.lock().clear();
    }

    pub fn transport(&self) -> Transport {
        self.shared.control.lock().transport
    }

    /// Plays from `start` to `end`, the C# `Play(startTime, endTime)` and `PlayLoop`.
    pub fn play(&self, start: TimeSpan, end: TimeSpan, looping: bool) {
        let mut c = self.shared.control.lock();
        let source = self.shared.source.read().clone();
        if let Some(s) = &source {
            s.seek(start);
            s.play();
        }
        self.shared.position.store(start.ticks(), Ordering::Relaxed);
        self.shared.finished.store(false, Ordering::Relaxed);
        c.range = PlayRange { start, end, looping };
        c.generation = c.generation.wrapping_add(1);
        c.transport = Transport::Playing;
        self.shared.wake.notify_all();
    }

    pub fn pause(&self) {
        let mut c = self.shared.control.lock();
        if c.transport == Transport::Playing {
            if let Some(s) = self.shared.source.read().as_ref() {
                s.pause();
            }
            c.transport = Transport::Paused;
            c.generation = c.generation.wrapping_add(1);
        }
    }

    pub fn resume(&self) {
        let mut c = self.shared.control.lock();
        if c.transport == Transport::Paused {
            if let Some(s) = self.shared.source.read().as_ref() {
                s.play();
            }
            c.transport = Transport::Playing;
            c.generation = c.generation.wrapping_add(1);
            self.shared.wake.notify_all();
        }
    }

    pub fn stop(&self) {
        let mut c = self.shared.control.lock();
        if let Some(s) = self.shared.source.read().as_ref() {
            s.pause();
        }
        c.transport = Transport::Stopped;
        c.generation = c.generation.wrapping_add(1);
        c.needs_clear = true;
        self.shared.wake.notify_all();
    }

    /// Moves the play position. While paused or stopped this shows that instant once.
    pub fn seek(&self, to: TimeSpan) {
        let mut c = self.shared.control.lock();
        c.generation = c.generation.wrapping_add(1);
        if let Some(s) = self.shared.source.read().as_ref() {
            s.seek(to);
        }
        self.shared.position.store(to.ticks(), Ordering::Relaxed);
        if c.transport != Transport::Playing { c.pending_refresh = true; }
        self.shared.wake.notify_all();
    }

    /// Asks for one evaluation at the current position when not playing.
    pub fn refresh(&self) {
        let mut c = self.shared.control.lock();
        if c.transport != Transport::Playing {
            c.pending_refresh = true;
            self.shared.wake.notify_all();
        }
    }

    /// The position of the most recent frame, without locks.
    #[inline]
    pub fn position(&self) -> TimeSpan {
        TimeSpan(self.shared.position.load(Ordering::Relaxed))
    }

    /// True once, after playback runs off the end of its range.
    pub fn take_finished(&self) -> bool {
        self.shared.finished.swap(false, Ordering::Relaxed)
    }

    pub fn stats(&self) -> EngineStats {
        EngineStats {
            frames: self.shared.frames.load(Ordering::Relaxed),
            late_frames: self.shared.late_frames.load(Ordering::Relaxed),
            last_frame_us: self.shared.last_frame_us.load(Ordering::Relaxed),
            max_frame_us: self.shared.max_frame_us.load(Ordering::Relaxed),
            frame_timing: self.shared.frame_timing.totals(),
            evaluate_timing: self.shared.evaluate_timing.totals(),
        }
    }

    /// Starts `frame_timing` and `evaluate_timing` over.
    pub fn reset_timings(&self) {
        self.shared.frame_timing.reset();
        self.shared.evaluate_timing.reset();
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.stop_all();
        {
            let mut c = self.shared.control.lock();
            c.shutdown = true;
            self.shared.wake.notify_all();
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn set_realtime_priority() {
    // The highest non-privileged class macOS has. The frame thread does little work, so this doesn't hog a core.
    unsafe {
        libc::pthread_set_qos_class_self_np(libc::qos_class_t::QOS_CLASS_USER_INTERACTIVE, 0);
    }
}

fn frame_thread(shared: Arc<Shared>) {
    set_realtime_priority();
    let mut scratch = EvalScratch::default();
    let mut states: Vec<CombinedState> = Vec::new();
    let mut next_tick = Instant::now();
    let mut contexts = Vec::new();
    let mut inputs = Vec::new();
    let mut mixer = FrameMixer::default();
    let mut generation_seen = 0;
    let mut advanced = false;

    loop {
        // Release the previous frame's references before the idle wait. In particular,
        // a naturally ended context must not keep its audio stream open until another show.
        contexts.clear();
        inputs.clear();
        // Idle until there is work. An idle engine uses no CPU.
        let (playing, paused, one_shot, clear, range_end, range_start, looping, generation, source) = {
            let mut c = shared.control.lock();
            loop {
                if c.shutdown {
                    return;
                }
                c.contexts.retain(|context| !*context.finished.lock());
                if c.transport == Transport::Playing || c.pending_refresh || c.needs_clear || c.contexts.iter().any(|context| context.advances_frames()) {
                    break;
                }
                shared.wake.wait(&mut c);
            }
            let playing = c.transport == Transport::Playing;
            let one_shot = std::mem::take(&mut c.pending_refresh);
            let clear = std::mem::take(&mut c.needs_clear);
            contexts.clear();
            contexts.extend(c.contexts.iter().cloned());
            (playing, c.transport == Transport::Paused, one_shot, clear, c.range.end, c.range.start, c.range.looping, c.generation, shared.source.read().clone())
        };

        if generation != generation_seen {
            generation_seen = generation;
            advanced = false;
        }

        let advancing = playing || contexts.iter().any(|context| context.advances_frames());
        if !advancing {
            next_tick = Instant::now();
        }

        let started = Instant::now();
        let mut time = TimeSpan(shared.position.load(Ordering::Relaxed));
        if playing {
            let mut source_finished = false;
            if let Some(source) = source.as_ref() {
                time = source.position();
                source_finished = source.finished();
            }
            // Audio clocks clamp to their duration. Waiting for a value strictly beyond
            // it would leave an audio-backed sequence playing forever at its last frame.
            advanced |= time > range_start;
            // Reading an external media clock can overlap a new Play/Seek/Stop. Only
            // this generation may change its transport, clock or completion flag.
            let mut c = shared.control.lock();
            if c.generation != generation || c.transport != Transport::Playing { continue; }
            // Some upstream media sources return zero on end. Only treat that as end
            // after progress, so the initial zero (and a fresh loop) is not a completion.
            if (advanced && range_end >= range_start && (time >= range_end || time == TimeSpan::ZERO)) || source_finished {
                if looping {
                    if let Some(source) = source.as_ref() {
                        source.pause();
                        source.seek(range_start);
                        source.play();
                    }
                    time = range_start;
                    advanced = false;
                } else {
                    // Off the end: stop, and clear the lights on the next pass.
                    if let Some(source) = source.as_ref() {
                        source.pause();
                    }
                    c.transport = Transport::Stopped;
                    c.needs_clear = true;
                    shared.finished.store(true, Ordering::Relaxed);
                    continue;
                }
            }
            shared.position.store(time.ticks(), Ordering::Relaxed);
        }

        inputs.clear();
        if playing || paused || (one_shot && !clear) {
            if let Some(schedule) = shared.schedule.read().as_ref() {
                inputs.push((schedule.clone(), time));
            }
        }
        for context in &contexts {
            if *context.finished.lock() { continue; }
            let at = context.source.position();
            if at >= context.end || context.source.finished() {
                context.finish();
            } else {
                inputs.push((context.schedule.read().clone(), at));
            }
        }
        let evaluate_started = Instant::now();
        let any = if let Some(elements) = shared.elements.read().as_ref() {
            mixer.evaluate(elements, &inputs, &mut scratch, &mut states)
        } else { false };
        shared.evaluate_timing.add(evaluate_started.elapsed().as_micros() as u64);

        if advancing || one_shot || clear {
            let frame_time = if playing || paused { time } else { inputs.first().map_or(time, |(_, at)| *at) };
            for sink in shared.sinks.lock().iter_mut() {
                sink.frame(frame_time, &states, any);
            }
        }

        let took = started.elapsed().as_micros() as u64;
        shared.frames.fetch_add(1, Ordering::Relaxed);
        shared.last_frame_us.store(took, Ordering::Relaxed);
        shared.max_frame_us.fetch_max(took, Ordering::Relaxed);
        shared.frame_timing.add(took);

        if advancing {
            let interval = Duration::from_micros(shared.interval_us.load(Ordering::Relaxed).max(1) as u64);
            next_tick += interval;
            let now = Instant::now();
            if now > next_tick + interval {
                // Fell behind by more than a frame: count it and resynchronise rather than rushing to catch up.
                shared.late_frames.fetch_add(1, Ordering::Relaxed);
                next_tick = now + interval;
            }
            precise_sleep_until(next_tick);
        }
    }
}

/// Sleeps most of the way and spins the last stretch, because `thread::sleep` alone can overshoot by a
/// millisecond or more, which would show up as jitter on a 10 ms frame.
fn precise_sleep_until(deadline: Instant) {
    const SPIN: Duration = Duration::from_micros(600);
    let now = Instant::now();
    if deadline <= now {
        return;
    }
    let remaining = deadline - now;
    if remaining > SPIN {
        std::thread::sleep(remaining - SPIN);
    }
    while Instant::now() < deadline {
        std::hint::spin_loop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::{ElementMap, ScheduledEffect};
    use std::sync::atomic::AtomicUsize;
    use vixen_core::{Color, EffectIntents, Intent, SequenceLayers, Uuid};

    #[derive(Default)]
    struct Recorded {
        frames: Mutex<Vec<(TimeSpan, bool)>>,
        calls: AtomicUsize,
    }

    struct Recorder(Arc<Recorded>);

    impl FrameSink for Recorder {
        fn frame(&mut self, time: TimeSpan, _states: &[CombinedState], any_state: bool) {
            self.0.calls.fetch_add(1, Ordering::Relaxed);
            self.0.frames.lock().push((time, any_state));
        }
    }

    fn lit_schedule(ms: i64) -> Arc<Schedule> {
        let id = Uuid::new_v4();
        let mut e = EffectIntents::new();
        e.add_intent_for_element(id, Intent::static_lighting(Color::RED, 1.0, TimeSpan::from_ms_i(ms)), TimeSpan::ZERO);
        let effect = ScheduledEffect {
            id: Uuid::new_v4(),
            start: TimeSpan::ZERO,
            span: TimeSpan::from_ms_i(ms),
            layer: 0,
            intents: Arc::new(e),
        };
        Arc::new(Schedule::build(Arc::new(ElementMap::from_ids([id])), SequenceLayers::default().layers, &[effect]))
    }

    fn engine(recorded: &Arc<Recorded>) -> Engine {
        let e = Engine::new(Duration::from_millis(5));
        e.set_source(Arc::new(InternalClock::default()));
        e.set_schedule(lit_schedule(10_000));
        e.add_sink(Box::new(Recorder(recorded.clone())));
        e
    }

    fn wait_until(mut f: impl FnMut() -> bool) -> bool {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(5) {
            if f() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        false
    }

    #[test]
    fn playing_delivers_frames_with_advancing_time() {
        let r = Arc::new(Recorded::default());
        let e = engine(&r);
        e.play(TimeSpan::ZERO, TimeSpan::from_ms_i(10_000), false);
        assert!(wait_until(|| r.calls.load(Ordering::Relaxed) >= 10));
        e.stop();
        let frames = r.frames.lock();
        assert!(frames.iter().take(10).all(|f| f.1), "the effect is active");
        assert!(frames.windows(2).take(9).all(|w| w[1].0 >= w[0].0), "time never moves backwards");
    }

    #[test]
    fn reaching_the_end_stops_and_clears() {
        let r = Arc::new(Recorded::default());
        let e = engine(&r);
        e.play(TimeSpan::ZERO, TimeSpan::from_ms_i(30), false);
        assert!(wait_until(|| e.transport() == Transport::Stopped));
        assert!(e.take_finished());
        assert!(!e.take_finished(), "the flag reports once");
        assert!(wait_until(|| r.frames.lock().last().is_some_and(|f| !f.1)), "a final all-off frame goes out");
    }

    /// Like an audio clock, this source holds at its duration and needs Play after end.
    #[derive(Default)]
    struct ClampedClock {
        position: AtomicI64,
        plays: AtomicUsize,
        pauses: AtomicUsize,
    }

    impl TimeSource for ClampedClock {
        fn position(&self) -> TimeSpan { TimeSpan(self.position.load(Ordering::Relaxed)) }
        fn play(&self) { self.plays.fetch_add(1, Ordering::Relaxed); }
        fn pause(&self) { self.pauses.fetch_add(1, Ordering::Relaxed); }
        fn seek(&self, to: TimeSpan) { self.position.store(to.ticks(), Ordering::Relaxed); }
    }

    #[test]
    fn clock_at_exact_end_finishes_and_clears() {
        let r = Arc::new(Recorded::default());
        let e = engine(&r);
        let source = Arc::new(ClampedClock::default());
        e.set_source(source.clone());
        e.play(TimeSpan::ZERO, TimeSpan::from_ms_i(30), false);
        assert!(wait_until(|| r.frames.lock().iter().any(|f| f.1)));
        source.seek(TimeSpan::from_ms_i(30));
        assert!(wait_until(|| e.transport() == Transport::Stopped));
        assert!(e.take_finished());
        assert!(wait_until(|| r.frames.lock().last().is_some_and(|f| !f.1)));
    }

    #[test]
    fn looping_at_exact_end_restarts_source_at_nonzero_start() {
        let r = Arc::new(Recorded::default());
        let e = engine(&r);
        let source = Arc::new(ClampedClock::default());
        e.set_source(source.clone());
        e.play(TimeSpan::from_ms_i(100), TimeSpan::from_ms_i(130), true);
        source.seek(TimeSpan::from_ms_i(130));
        assert!(wait_until(|| source.plays.load(Ordering::Relaxed) >= 2));
        assert_eq!(source.position(), TimeSpan::from_ms_i(100));
        assert_eq!(source.pauses.load(Ordering::Relaxed), 1);
        assert_eq!(e.transport(), Transport::Playing);
        assert!(!e.take_finished());
        e.stop();
    }

    #[test]
    fn zero_clock_ends_only_after_progress_and_a_new_play_resets_that_history() {
        let r = Arc::new(Recorded::default());
        let e = engine(&r);
        let clock = Arc::new(ClampedClock::default());
        e.set_source(clock.clone());
        e.play(TimeSpan::ZERO, TimeSpan::from_ms_i(1000), false);
        assert!(wait_until(|| r.calls.load(Ordering::Relaxed) >= 2));
        assert_eq!(e.transport(), Transport::Playing);
        clock.seek(TimeSpan::from_ms_i(100));
        assert!(wait_until(|| e.position() == TimeSpan::from_ms_i(100)));
        clock.seek(TimeSpan::ZERO);
        assert!(wait_until(|| e.transport() == Transport::Stopped));
        assert!(e.take_finished());
        let before = r.calls.load(Ordering::Relaxed);
        e.play(TimeSpan::ZERO, TimeSpan::from_ms_i(1000), false);
        assert!(wait_until(|| r.calls.load(Ordering::Relaxed) >= before + 2));
        assert_eq!(e.transport(), Transport::Playing);
        e.stop();
    }

    struct StateRecorder(Arc<Mutex<Vec<CombinedState>>>);
    impl FrameSink for StateRecorder {
        fn frame(&mut self, _: TimeSpan, states: &[CombinedState], _: bool) {
            *self.0.lock() = states.to_vec();
        }
    }

    fn context_schedule(elements: Arc<ElementMap>, color: Color) -> Arc<Schedule> {
        let mut intents = EffectIntents::new();
        intents.add_intent_for_element(elements.ids[0], Intent::static_lighting(color, 1.0, TimeSpan::from_ms_i(1000)), TimeSpan::ZERO);
        Arc::new(Schedule::build(elements, SequenceLayers::default().layers, &[
            ScheduledEffect { id: Uuid::new_v4(), start: TimeSpan::ZERO, span: TimeSpan::from_ms_i(1000), layer: 0, intents: Arc::new(intents) }
        ]))
    }

    fn displayed(states: &Mutex<Vec<CombinedState>>) -> Option<Color> {
        states.lock().first().and_then(|s| s.rgb.map(|v| v.full_color()).or_else(|| s.lighting.map(|v| v.full_color())))
    }

    #[test]
    fn scheduled_context_mixes_with_editor_and_survives_editor_stop() {
        let e = Engine::new(Duration::from_millis(2));
        let elements = Arc::new(ElementMap::from_ids([Uuid::new_v4()]));
        let primary = Arc::new(ClampedClock::default());
        e.set_source(primary);
        e.set_schedule(context_schedule(elements.clone(), Color::RED));
        let states = Arc::new(Mutex::new(Vec::new()));
        e.add_sink(Box::new(StateRecorder(states.clone())));
        e.play(TimeSpan::ZERO, TimeSpan::from_ms_i(1000), false);
        let background_clock = Arc::new(ClampedClock::default());
        let background = e.start_context(context_schedule(elements, Color::BLUE), background_clock.clone(), TimeSpan::from_ms_i(1000));
        assert!(wait_until(|| displayed(&states) == Some(Color::rgb(255, 0, 255))));
        e.stop();
        assert!(wait_until(|| displayed(&states) == Some(Color::BLUE)));
        assert!(!background.is_finished());
        background_clock.seek(TimeSpan::from_ms_i(1000));
        assert!(wait_until(|| background.is_finished()));
        assert!(wait_until(|| displayed(&states).is_none()));
        assert!(!e.take_finished(), "a background completion does not finish the editor");
    }

    #[test]
    fn an_old_clock_completion_cannot_stop_or_rewind_a_new_play() {
        struct HeldClock {
            first: AtomicBool,
            entered: std::sync::mpsc::SyncSender<()>,
            release: Mutex<std::sync::mpsc::Receiver<()>>,
        }
        impl TimeSource for HeldClock {
            fn position(&self) -> TimeSpan {
                if self.first.swap(false, Ordering::SeqCst) {
                    self.entered.send(()).unwrap();
                    self.release.lock().recv().unwrap();
                }
                TimeSpan::from_ms_i(200)
            }
            fn play(&self) {}
            fn pause(&self) {}
            fn seek(&self, _: TimeSpan) {}
        }
        for looping in [false, true] {
            let (entered, entered_rx) = std::sync::mpsc::sync_channel(1);
            let (release, release_rx) = std::sync::mpsc::sync_channel(1);
            let recorded = Arc::new(Recorded::default());
            let e = engine(&recorded);
            e.set_source(Arc::new(HeldClock { first: AtomicBool::new(true), entered, release: Mutex::new(release_rx) }));
            e.play(TimeSpan::ZERO, TimeSpan::from_ms_i(100), looping);
            entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            let current = Arc::new(ClampedClock::default());
            e.set_source(current.clone());
            e.play(TimeSpan::from_ms_i(500), TimeSpan::from_ms_i(5000), false);
            release.send(()).unwrap();
            assert!(wait_until(|| recorded.calls.load(Ordering::Relaxed) >= 2));
            assert_eq!(e.transport(), Transport::Playing);
            assert_eq!(current.position(), TimeSpan::from_ms_i(500));
            assert_eq!(current.pauses.load(Ordering::Relaxed), 0);
            assert!(!e.take_finished());
        }
    }

    #[test]
    fn empty_live_contexts_park_but_wake_for_new_effects_and_sequences_still_end() {
        let e = Engine::new(Duration::from_millis(2));
        let recorded = Arc::new(Recorded::default());
        e.add_sink(Box::new(Recorder(recorded.clone())));
        let elements = Arc::new(ElementMap::from_ids([Uuid::new_v4()]));
        let empty = Arc::new(Schedule::empty(elements.clone(), SequenceLayers::default().layers));
        let live = e.start_live_context(empty.clone());
        std::thread::sleep(Duration::from_millis(15));
        let parked = recorded.calls.load(Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(25));
        assert_eq!(recorded.calls.load(Ordering::Relaxed), parked);
        assert_eq!(live.transport(), Transport::Playing);
        live.set_schedule(context_schedule(elements, Color::RED));
        assert!(wait_until(|| recorded.calls.load(Ordering::Relaxed) >= parked + 3));
        assert!(recorded.frames.lock().last().unwrap().1);
        live.set_schedule(empty.clone());
        assert!(wait_until(|| recorded.frames.lock().last().is_some_and(|f| !f.1)));
        std::thread::sleep(Duration::from_millis(15));
        let cleared = recorded.calls.load(Ordering::Relaxed);
        std::thread::sleep(Duration::from_millis(25));
        assert_eq!(recorded.calls.load(Ordering::Relaxed), cleared);
        let sequence = e.start_context(empty, Arc::new(InternalClock::default()), TimeSpan::from_ms_i(20));
        assert!(wait_until(|| sequence.is_finished()));
        assert!(!live.is_finished());
    }

    #[test]
    fn live_schedule_changes_and_paused_contexts_keep_independent_state() {
        let e = Engine::new(Duration::from_millis(2));
        let elements = Arc::new(ElementMap::from_ids([Uuid::new_v4()]));
        let states = Arc::new(Mutex::new(Vec::new()));
        e.add_sink(Box::new(StateRecorder(states.clone())));
        let context = e.start_live_context(context_schedule(elements.clone(), Color::RED));
        assert!(!e.has_sequence_contexts());
        assert!(wait_until(|| displayed(&states) == Some(Color::RED)));
        context.pause();
        let at = context.position();
        std::thread::sleep(Duration::from_millis(10));
        assert_eq!(context.transport(), Transport::Paused);
        assert_eq!(context.position(), at);
        context.set_schedule(context_schedule(elements, Color::BLUE));
        assert!(wait_until(|| displayed(&states) == Some(Color::BLUE)));
        context.resume();
        assert!(wait_until(|| context.position() > at));
        context.stop();
        context.resume();
        assert_eq!(context.transport(), Transport::Stopped);
        assert!(wait_until(|| displayed(&states).is_none()));
    }

    #[test]
    fn contexts_stop_independently_and_execution_shutdown_wakes_waiters() {
        let e = Engine::new(Duration::from_millis(2));
        let elements = Arc::new(ElementMap::from_ids([Uuid::new_v4()]));
        let states = Arc::new(Mutex::new(Vec::new()));
        e.add_sink(Box::new(StateRecorder(states.clone())));
        let red = e.start_context(context_schedule(elements.clone(), Color::RED), Arc::new(ClampedClock::default()), TimeSpan::from_ms_i(1000));
        let blue = Arc::new(e.start_context(context_schedule(elements, Color::BLUE), Arc::new(ClampedClock::default()), TimeSpan::from_ms_i(1000)));
        assert!(wait_until(|| displayed(&states) == Some(Color::rgb(255, 0, 255))));
        drop(red);
        assert!(wait_until(|| displayed(&states) == Some(Color::BLUE)));
        let waiting = blue.clone();
        let waiter = std::thread::spawn(move || waiting.wait());
        e.stop_all();
        waiter.join().unwrap();
        assert!(blue.is_finished());
        assert!(!e.has_contexts());
        assert!(wait_until(|| displayed(&states).is_none()));
    }

    #[test]
    fn engine_drop_finishes_outstanding_contexts() {
        let e = Engine::new(Duration::from_millis(2));
        let schedule = lit_schedule(1000);
        let source = Arc::new(ClampedClock::default());
        let context = e.start_context(schedule, source.clone(), TimeSpan::from_ms_i(1000));
        assert!(!context.is_finished());
        drop(e);
        assert!(context.is_finished());
        assert_eq!(source.pauses.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn looping_wraps_back_to_the_start() {
        let r = Arc::new(Recorded::default());
        let e = engine(&r);
        // set_schedule publishes an idle preview at zero. Drain it before this
        // test measures the loop's range; it is not a frame from the new play.
        assert!(wait_until(|| r.calls.load(Ordering::Relaxed) > 0));
        r.frames.lock().clear();
        e.play(TimeSpan::from_ms_i(100), TimeSpan::from_ms_i(130), true);
        assert!(wait_until(|| {
            let f = r.frames.lock();
            f.windows(2).any(|w| w[1].0 < w[0].0)
        }), "time should jump back at the loop point");
        assert_eq!(e.transport(), Transport::Playing);
        assert!(r.frames.lock().iter().all(|f| f.0 >= TimeSpan::from_ms_i(100)));
        e.stop();
    }

    #[test]
    fn pause_stops_frames_and_resume_continues() {
        let r = Arc::new(Recorded::default());
        let e = engine(&r);
        e.play(TimeSpan::ZERO, TimeSpan::from_ms_i(10_000), false);
        assert!(wait_until(|| r.calls.load(Ordering::Relaxed) >= 3));
        e.pause();
        std::thread::sleep(Duration::from_millis(20));
        let held = r.calls.load(Ordering::Relaxed);
        let position = e.position();
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(r.calls.load(Ordering::Relaxed), held, "no frames while paused");
        assert_eq!(e.position(), position);
        e.resume();
        assert!(wait_until(|| r.calls.load(Ordering::Relaxed) > held));
        assert!(e.position() >= position);
        e.stop();
    }

    #[test]
    fn seeking_while_stopped_shows_one_frame_there() {
        let r = Arc::new(Recorded::default());
        let e = engine(&r);
        assert!(wait_until(|| r.calls.load(Ordering::Relaxed) >= 1), "publishing a schedule refreshes once");
        let before = r.calls.load(Ordering::Relaxed);
        e.seek(TimeSpan::from_ms_i(500));
        assert!(wait_until(|| r.calls.load(Ordering::Relaxed) == before + 1));
        std::thread::sleep(Duration::from_millis(20));
        assert_eq!(r.calls.load(Ordering::Relaxed), before + 1, "exactly one frame");
        assert_eq!(r.frames.lock().last().unwrap().0, TimeSpan::from_ms_i(500));
    }

    #[test]
    fn dropping_the_engine_ends_its_thread() {
        let r = Arc::new(Recorded::default());
        let e = engine(&r);
        e.play(TimeSpan::ZERO, TimeSpan::from_ms_i(10_000), true);
        drop(e);
    }

    #[test]
    fn internal_clock_tracks_speed_and_pause() {
        let c = InternalClock::default();
        c.seek(TimeSpan::from_ms_i(1000));
        assert_eq!(c.position(), TimeSpan::from_ms_i(1000), "a stopped clock holds still");
        c.play();
        std::thread::sleep(Duration::from_millis(30));
        let p = c.position();
        assert!(p > TimeSpan::from_ms_i(1020) && p < TimeSpan::from_ms_i(1200), "{p:?}");
        c.pause();
        let held = c.position();
        std::thread::sleep(Duration::from_millis(10));
        assert_eq!(c.position(), held);
    }
}
