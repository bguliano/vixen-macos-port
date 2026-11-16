//! The Effect Preview switch's loop, F6: the selection plays over and over from a live context, with nothing cleared between loops, so an effect longer than the loop keeps playing under the next one.
//! The timer is here, on its own thread, so a stalled main thread can't starve it or double it. An edit doesn't restart anything: the next loop plays the new render.

use crate::playback::ShowPlayback;
use crate::sequence::{EffectEntry, EffectId, SequenceSession};
use parking_lot::{Condvar, Mutex};
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;
use vixen_core::TimeSpan;
use vixen_engine::schedule::{Schedule, ScheduledEffect};

/// A new loop's effects go on the schedule this long before they start, so that a busy machine doesn't clip their beginnings.
const LEAD: Duration = Duration::from_millis(100);

/// F6, steps 2 and 3: copies placed at `start - startOffset`, and the loop's length, the last-starting effect's end minus `startOffset`. `None` when there's nothing to play or the length is zero (GUARD).
pub fn loop_copies(s: &SequenceSession, effects: &[EffectId]) -> Option<(Vec<ScheduledEffect>, TimeSpan)> {
    let mut entries: Vec<&EffectEntry> = effects.iter().filter_map(|id| s.effect(*id)).collect();
    entries.sort_by_key(|e| e.node.start);
    let offset = entries.first()?.node.start;
    let duration = entries.last()?.end() - offset;
    if duration <= TimeSpan::ZERO {
        return None;
    }
    let copies = entries
        .iter()
        .map(|e| {
            let mut copy = s.scheduled(e);
            copy.id = Uuid::new_v4();
            copy.start = e.node.start - offset;
            copy
        })
        .collect();
    Some((copies, duration))
}

fn schedule_of(s: &SequenceSession, copies: &[ScheduledEffect]) -> Arc<Schedule> {
    // The context mixes through the sequence's layers.
    Arc::new(Schedule::build(Arc::clone(s.elements()), s.document().layers.layers.clone(), copies))
}

struct Shared {
    stop: Mutex<bool>,
    wake: Condvar,
}

/// A running preview. Dropping it or calling `stop` ends the loop and clears what it was showing, without waiting for anything: this runs on every selection change (hazard 14).
pub struct EffectPreview {
    shared: Arc<Shared>,
    /// The claim on the engine.
    claim: u64,
    playback: Arc<ShowPlayback>,
    session: Arc<Mutex<SequenceSession>>,
}

impl EffectPreview {
    /// F6's `PreviewPlay`. `None` when there's nothing to play, or the engine isn't free: real playback or Export has it.
    pub fn start(playback: Arc<ShowPlayback>, session: Arc<Mutex<SequenceSession>>, effects: Vec<EffectId>) -> Option<EffectPreview> {
        if !playback.effect_preview_may_start() {
            return None;
        }
        let (first, mut duration, schedule) = {
            let s = session.lock();
            let (copies, duration) = loop_copies(&s, &effects)?;
            let schedule = schedule_of(&s, &copies);
            (copies, duration, schedule)
        };
        // Finished renders of the sequence mustn't replace the preview's schedule.
        session.lock().set_schedule_listener(None);
        let Some(claim) = playback.begin_effect_preview(schedule) else {
            playback.attach_session(&mut session.lock());
            return None;
        };
        let started = Instant::now();
        let shared = Arc::new(Shared { stop: Mutex::new(false), wake: Condvar::new() });
        {
            let shared = Arc::clone(&shared);
            let session = Arc::clone(&session);
            let playback = Arc::clone(&playback);
            let _ = std::thread::Builder::new().name("vixen-effect-preview".to_string()).spawn(move || {
                let mut live = first;
                let mut next_start = duration;
                loop {
                    let wait = Duration::from_secs_f64(next_start.total_secs().max(0.0)).saturating_sub(LEAD).saturating_sub(started.elapsed());
                    {
                        let mut stop = shared.stop.lock();
                        if !*stop && !wait.is_zero() {
                            shared.wake.wait_for(&mut stop, wait);
                        }
                        if *stop {
                            return;
                        }
                    }
                    let s = session.lock();
                    // An edit can leave nothing to play this time round: the loop waits and looks again.
                    let Some((copies, length)) = loop_copies(&s, &effects) else {
                        drop(s);
                        next_start = next_start + TimeSpan::from_ms(250.0);
                        continue;
                    };
                    live.retain(|c| c.start + c.span > next_start);
                    live.extend(copies.into_iter().map(|mut c| {
                        c.start = c.start + next_start;
                        c
                    }));
                    let schedule = schedule_of(&s, &live);
                    drop(s);
                    if !playback.update_effect_preview(claim, schedule) {
                        return;
                    }
                    duration = length;
                    next_start = next_start + duration;
                }
            });
        }
        Some(EffectPreview { shared, claim, playback, session })
    }

    /// False once something else has taken the engine or stopped it, so the panel knows to bring the preview back when it's free.
    pub fn is_playing(&self) -> bool {
        self.playback.effect_preview_is_playing(self.claim)
    }

    /// F6's `PreviewStop`: the timer stops, and what's lit clears.
    pub fn stop(self) {}
}

impl Drop for EffectPreview {
    fn drop(&mut self) {
        *self.shared.stop.lock() = true;
        self.shared.wake.notify_all();
        if self.playback.end_effect_preview(self.claim) {
            self.playback.attach_session(&mut self.session.lock());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequence::SessionEnvironment;
    use vixen_core::ElementTree;
    use vixen_io::ModuleStore;

    fn session() -> (SequenceSession, Uuid) {
        let mut tree = ElementTree::new();
        let string = tree.add_string("Arch", 3);
        let target = tree.node(string).id;
        let env = SessionEnvironment { video_cache: None,
            tree: Arc::new(tree),
            store: Arc::new(ModuleStore::new()),
            module_data_root: None,
            frame_ms: 50,
            media_dir: std::env::temp_dir(),
        };
        (SequenceSession::new_untitled(&env), target)
    }

    fn pulse() -> Uuid {
        vixen_effect_registry::all().iter().find(|d| d.name == "Pulse").unwrap().type_id
    }

    #[test]
    fn the_loop_runs_to_the_last_starting_effects_end() {
        let (mut s, target) = session();
        let ms = TimeSpan::from_ms;
        let long = s.add_new_effect(pulse(), target, "Arch", ms(1000.0), ms(5000.0)).unwrap();
        let short = s.add_new_effect(pulse(), target, "Arch", ms(2000.0), ms(1000.0)).unwrap();
        let (copies, duration) = loop_copies(&s, &[short, long]).unwrap();
        assert_eq!(duration, ms(2000.0));
        // Ordered by start, and the first starts at 0.
        assert_eq!(copies[0].start, TimeSpan::ZERO);
        assert_eq!(copies[0].span, ms(5000.0));
        assert_eq!(copies[1].start, ms(1000.0));
    }

    #[test]
    fn nothing_to_play_and_no_length_are_refused() {
        let (mut s, target) = session();
        assert!(loop_copies(&s, &[]).is_none());
        let zero = s.add_new_effect(pulse(), target, "Arch", TimeSpan::ZERO, TimeSpan::ZERO).unwrap();
        assert!(loop_copies(&s, &[zero]).is_none());
    }

    fn playback_and_session() -> (Arc<ShowPlayback>, Arc<Mutex<SequenceSession>>, EffectId, EffectId) {
        let mut system = vixen_io::SystemData::default();
        let mut tree = ElementTree::new();
        let string = tree.add_string("Arch", 3);
        let target = tree.node(string).id;
        system.tree = tree.clone();
        system.default_update_interval = 10;
        let tree = Arc::new(tree);
        let elements = Arc::new(vixen_engine::schedule::ElementMap::from_tree(&tree));
        let playback = Arc::new(ShowPlayback::new(&system, elements, true));
        let env = SessionEnvironment { video_cache: None, tree, store: Arc::new(ModuleStore::new()), module_data_root: None, frame_ms: 25, media_dir: std::env::temp_dir() };
        let mut s = SequenceSession::new_untitled(&env);
        let ms = TimeSpan::from_ms;
        let long = s.add_new_effect(pulse(), target, "Arch", ms(0.0), ms(10_000.0)).unwrap();
        let short = s.add_new_effect(pulse(), target, "Arch", ms(1000.0), ms(1000.0)).unwrap();
        s.wait_for_renders();
        (playback, Arc::new(Mutex::new(s)), long, short)
    }

    #[test]
    fn test_62_the_loop_is_the_last_starting_effects_end_and_the_first_plays_on() {
        let (_playback, session, long, short) = playback_and_session();
        let (copies, duration) = loop_copies(&session.lock(), &[long, short]).unwrap();
        assert_eq!(duration, TimeSpan::from_ms(2000.0));
        // The long one isn't cut off at the loop's end: nothing clears between loops.
        assert_eq!(copies[0].span, TimeSpan::from_ms(10_000.0));
    }

    #[test]
    fn test_63_stopping_doesnt_wait() {
        let (playback, session, long, short) = playback_and_session();
        let preview = EffectPreview::start(Arc::clone(&playback), Arc::clone(&session), vec![long, short]).unwrap();
        assert!(preview.is_playing());
        let begun = Instant::now();
        drop(preview);
        assert!(begun.elapsed() < Duration::from_millis(100));
        assert_eq!(playback.transport(), vixen_engine::Transport::Stopped);
    }

    #[test]
    fn the_windows_see_a_stopped_transport_while_the_preview_plays() {
        let (playback, session, long, short) = playback_and_session();
        let preview = EffectPreview::start(Arc::clone(&playback), Arc::clone(&session), vec![long, short]).unwrap();
        assert_eq!(playback.transport(), vixen_engine::Transport::Stopped);
        assert_eq!(playback.position(), TimeSpan::ZERO);
        // Their Stop and Pause are for someone else.
        playback.stop();
        playback.pause();
        assert!(preview.is_playing());
        drop(preview);
    }

    #[test]
    fn real_playback_takes_the_engine_and_the_preview_can_come_back_after_it() {
        let (playback, session, long, short) = playback_and_session();
        let preview = EffectPreview::start(Arc::clone(&playback), Arc::clone(&session), vec![long, short]).unwrap();
        playback.play(&mut session.lock(), TimeSpan::ZERO, TimeSpan::from_ms(10_000.0), false);
        assert!(!preview.is_playing());
        assert_eq!(playback.transport(), vixen_engine::Transport::Playing);
        // A preview can't start while it plays, and dropping the old one leaves it alone.
        assert!(EffectPreview::start(Arc::clone(&playback), Arc::clone(&session), vec![long]).is_none());
        drop(preview);
        assert_eq!(playback.transport(), vixen_engine::Transport::Playing);
        playback.stop();
        let again = EffectPreview::start(Arc::clone(&playback), Arc::clone(&session), vec![long]);
        assert!(again.is_some_and(|p| p.is_playing()));
    }

    #[test]
    fn a_hold_stops_the_preview_and_keeps_it_off_until_let_go() {
        let (playback, session, long, short) = playback_and_session();
        let preview = EffectPreview::start(Arc::clone(&playback), Arc::clone(&session), vec![long, short]).unwrap();
        playback.hold_effect_preview(true);
        assert!(!preview.is_playing());
        assert!(!playback.effect_preview_may_start());
        assert!(EffectPreview::start(Arc::clone(&playback), Arc::clone(&session), vec![long]).is_none());
        playback.hold_effect_preview(false);
        assert!(EffectPreview::start(Arc::clone(&playback), Arc::clone(&session), vec![long]).is_some());
    }
}
