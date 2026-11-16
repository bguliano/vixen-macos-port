//! Show Sequence actions and the shared rendered-sequence cache (F63–F64).

use super::actions::{ActionRunner, Done, Finished, RunningAction};
use crate::app::AppCore;
use crate::logging;
use crate::sequence::SequenceSession;
use crate::shows::{self, ShowItem};
use parking_lot::{Condvar, Mutex};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use std::time::SystemTime;
use vixen_audio::AudioClip;
use vixen_core::TimeSpan;
use vixen_engine::{ContextPlayback, Schedule};

const LOGGER: &str = "VixenModules.App.Shows.Actions.SequenceAction";

struct PreparedSequence {
    _video_cache: Option<vixen_video::cache::CacheLease>,
    schedule: Arc<Schedule>,
    audio: Option<Arc<AudioClip>>,
    length: TimeSpan,
}

#[derive(Default)]
struct CacheEntry {
    modified: Option<SystemTime>,
    sequence: Weak<PreparedSequence>,
    loading: bool,
}

/// Only consumers own strong references. A stale version stays alive as long as its
/// running contexts need it; a later version never overwrites the old one's resources.
#[derive(Default)]
struct SequenceCache {
    entries: Mutex<HashMap<PathBuf, CacheEntry>>,
    changed: Condvar,
}

impl SequenceCache {
    fn load(
        &self,
        path: &Path,
        modified: SystemTime,
        load: impl FnOnce() -> Result<PreparedSequence, String>,
    ) -> Result<Arc<PreparedSequence>, String> {
        let mut entries = self.entries.lock();
        loop {
            let entry = entries.entry(path.to_path_buf()).or_default();
            if entry.loading {
                self.changed.wait(&mut entries);
                continue;
            }
            // Upstream reloads when the on-disk timestamp is newer, not merely different.
            if entry.modified.is_some_and(|old| modified <= old) {
                if let Some(sequence) = entry.sequence.upgrade() { return Ok(sequence); }
            }
            entry.loading = true;
            break;
        }
        drop(entries);
        let result = load().map(Arc::new);
        let mut entries = self.entries.lock();
        match &result {
            Ok(sequence) => {
                entries.insert(path.to_path_buf(), CacheEntry { modified: Some(modified), sequence: Arc::downgrade(sequence), loading: false });
            }
            Err(_) => { entries.remove(path); }
        }
        self.changed.notify_all();
        result
    }

    fn prune(&self) {
        self.entries.lock().retain(|_, entry| entry.loading || entry.sequence.strong_count() > 0);
    }
}

#[derive(Default)]
struct RunResources {
    released: bool,
    sequences: HashMap<PathBuf, (SystemTime, Arc<PreparedSequence>)>,
}

/// The scheduler creates a separate resource lease for each run, with one cache across
/// all runs. Identical show items in overlapping schedules therefore release independently.
pub struct SequenceRunner {
    core: Weak<AppCore>,
    cache: Arc<SequenceCache>,
    resources: Arc<Mutex<RunResources>>,
}

impl SequenceRunner {
    pub fn new(core: &Arc<AppCore>) -> Self {
        Self { core: Arc::downgrade(core), cache: Arc::new(SequenceCache::default()), resources: Arc::new(Mutex::new(RunResources::default())) }
    }

    fn shared(&self) -> Self {
        Self { core: self.core.clone(), cache: self.cache.clone(), resources: self.resources.clone() }
    }

    fn path(&self, item: &ShowItem) -> Option<PathBuf> {
        let core = self.core.upgrade()?;
        Some(shows::sequence_path(item, &core.paths.sequence_dir()))
    }

    fn prepare(&self, path: &Path) -> Result<Arc<PreparedSequence>, String> {
        let modified = std::fs::metadata(path).and_then(|m| m.modified()).map_err(|e| e.to_string())?;
        {
            let resources = self.resources.lock();
            if resources.released { return Err("Show has stopped.".into()); }
            if let Some((old, sequence)) = resources.sequences.get(path) {
                if modified <= *old { return Ok(sequence.clone()); }
            }
        }
        let core = self.core.upgrade().ok_or("The profile has closed.")?;
        let prepared = self.cache.load(path, modified, || {
            let mut session = SequenceSession::open(path, &core.session_environment()).map_err(|e| e.to_string())?;
            session.wait_for_renders();
            let problems = session.take_problems();
            if !problems.is_empty() { return Err(problems.join("\n")); }
            Ok(PreparedSequence { _video_cache: session.take_video_cache_lease(), schedule: session.schedule(), audio: session.audio().cloned(), length: session.length() })
        })?;
        let mut resources = self.resources.lock();
        if resources.released { return Err("Show has stopped.".into()); }
        resources.sequences.insert(path.to_path_buf(), (modified, prepared.clone()));
        Ok(prepared)
    }

    fn prepare_logged(&self, path: &Path) -> Option<Arc<PreparedSequence>> {
        match self.prepare(path) {
            Ok(sequence) => Some(sequence),
            Err(error) => {
                if !self.resources.lock().released {
                    logging::error(LOGGER, &format!("Could not pre-render sequence. Sequence will not play until the issue is corrected.{}; ", path.display()), Some(&error));
                }
                None
            }
        }
    }
}

impl ActionRunner for SequenceRunner {
    fn for_run(&self) -> Option<Arc<dyn ActionRunner>> {
        Some(Arc::new(Self { core: self.core.clone(), cache: self.cache.clone(), resources: Arc::new(Mutex::new(RunResources::default())) }))
    }

    fn needs_pre_process(&self, item: &ShowItem) -> bool {
        let Some(path) = self.path(item) else { return true };
        let Ok(modified) = std::fs::metadata(&path).and_then(|m| m.modified()) else { return true };
        self.resources.lock().sequences.get(&path).is_none_or(|(old, _)| modified > *old)
    }

    fn pre_process(&self, item: &ShowItem) {
        if let Some(path) = self.path(item) { self.prepare_logged(&path); }
    }

    fn execute(&self, item: &ShowItem, done: Done) -> Box<dyn RunningAction> {
        let Some(path) = self.path(item) else {
            done.complete();
            return Box::new(Finished);
        };
        let state = Arc::new(RunningState { cancelled: AtomicBool::new(false), context: Mutex::new(None), done });
        let worker = state.clone();
        let runner = self.shared();
        // Loading a changed sequence can take time. Never hold the scheduler's state lock
        // while decoding media/rendering, or Stop Now and the UI would be blocked.
        let result = std::thread::Builder::new().name("show-sequence".into()).spawn(move || {
            let prepared = runner.prepare_logged(&path);
            let context = {
                let mut slot = worker.context.lock();
                if worker.cancelled.load(Ordering::Acquire) {
                    None
                } else if let (Some(sequence), Some(core)) = (prepared, runner.core.upgrade()) {
                    let name = path.file_stem().unwrap_or_default().to_string_lossy();
                    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                    let context = core.playback.start_named_context(&name, &file_name, sequence.schedule.clone(), sequence.audio.clone(), sequence.length);
                    *slot = Some(context.clone());
                    Some(context)
                } else {
                    logging::error(LOGGER, &format!("Could not execute sequence. {}", path.display()), None);
                    None
                }
            };
            if let Some(context) = context { context.wait(); }
            worker.done.complete();
        });
        if let Err(error) = result {
            logging::error(LOGGER, "Could not execute sequence.", Some(&error.to_string()));
            state.done.complete();
        }
        Box::new(SequenceAction { state })
    }

    fn release(&self, _items: &[ShowItem]) {
        let mut resources = self.resources.lock();
        resources.released = true;
        resources.sequences.clear();
        drop(resources);
        self.cache.prune();
    }
}

struct RunningState {
    cancelled: AtomicBool,
    context: Mutex<Option<Arc<ContextPlayback>>>,
    done: Done,
}

struct SequenceAction { state: Arc<RunningState> }

impl RunningAction for SequenceAction {
    fn stop(&mut self) {
        let context = self.state.context.lock();
        self.state.cancelled.store(true, Ordering::Release);
        if let Some(context) = context.as_ref() { context.stop(); }
        drop(context);
        self.state.done.complete();
    }
}

impl Drop for SequenceAction {
    fn drop(&mut self) { self.stop(); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::time::Duration;
    use vixen_core::SequenceLayers;
    use vixen_engine::ElementMap;

    fn prepared(length: i64) -> PreparedSequence {
        PreparedSequence {
            _video_cache: None,
            schedule: Arc::new(Schedule::empty(Arc::new(ElementMap::default()), SequenceLayers::default().layers)),
            audio: None,
            length: TimeSpan::from_ms_i(length),
        }
    }

    #[test]
    fn overlapping_consumers_share_one_render_and_release_independently() {
        let cache = Arc::new(SequenceCache::default());
        let loads = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(std::sync::Barrier::new(8));
        let threads: Vec<_> = (0..8).map(|_| {
            let (cache, loads, barrier) = (cache.clone(), loads.clone(), barrier.clone());
            std::thread::spawn(move || {
                barrier.wait();
                cache.load(Path::new("same.tim"), SystemTime::UNIX_EPOCH, || {
                    loads.fetch_add(1, Ordering::SeqCst);
                    std::thread::sleep(Duration::from_millis(10));
                    Ok(prepared(100))
                }).unwrap()
            })
        }).collect();
        let mut copies: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
        assert_eq!(loads.load(Ordering::SeqCst), 1);
        assert!(copies.iter().all(|s| Arc::ptr_eq(s, &copies[0])));
        copies.pop();
        cache.prune();
        assert_eq!(cache.entries.lock().len(), 1);
        copies.clear();
        cache.prune();
        assert!(cache.entries.lock().is_empty());
    }

    #[test]
    fn newer_file_reloads_while_old_context_retains_its_version() {
        let cache = SequenceCache::default();
        let path = Path::new("edited.tim");
        let old = cache.load(path, SystemTime::UNIX_EPOCH, || Ok(prepared(100))).unwrap();
        let newer = SystemTime::UNIX_EPOCH + Duration::from_secs(1);
        let new = cache.load(path, newer, || Ok(prepared(200))).unwrap();
        assert!(!Arc::ptr_eq(&old, &new));
        assert_eq!(old.length, TimeSpan::from_ms_i(100));
        assert_eq!(new.length, TimeSpan::from_ms_i(200));
        let same = cache.load(path, newer, || panic!("already rendered")).unwrap();
        assert!(Arc::ptr_eq(&same, &new));
        drop(old);
        cache.prune();
        assert!(cache.entries.lock().contains_key(path));
    }

    #[test]
    fn failed_load_does_not_leave_a_loading_placeholder() {
        let cache = SequenceCache::default();
        let path = Path::new("missing.tim");
        assert!(cache.load(path, SystemTime::UNIX_EPOCH, || Err("broken".into())).is_err());
        assert!(!cache.entries.lock().contains_key(path));
        assert!(cache.load(path, SystemTime::UNIX_EPOCH, || Ok(prepared(100))).is_ok());
    }

    struct Profile { core: Arc<AppCore>, root: PathBuf }
    impl Profile {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("vixen-sequence-actions-{}", uuid::Uuid::new_v4()));
            let core = Arc::new(AppCore::open(root.clone(), true).unwrap());
            {
                let mut system = core.system.write();
                system.tree.add_element("Light");
                core.playback.replace_display(&system, Arc::new(ElementMap::from_tree(&system.tree)));
            }
            Self { core, root }
        }

        fn sequence(&self, name: &str, length: i64) -> ShowItem {
            let env = self.core.session_environment();
            let node = env.tree.node(env.tree.roots[0]).id;
            let mut sequence = SequenceSession::new_untitled(&env);
            sequence.set_length(TimeSpan::from_ms_i(length));
            sequence.add_new_effect(uuid::uuid!("32cff8e0-5b10-4466-a093-0d232c55aac0"), node, "Light", TimeSpan::ZERO, TimeSpan::from_ms_i(length)).unwrap();
            sequence.wait_for_renders();
            sequence.save(Some(&self.core.paths.sequence_dir().join(name))).unwrap();
            let mut item = ShowItem::new(crate::shows::ShowItemType::Sequential, name);
            item.action = crate::shows::ActionType::Sequence;
            item.sequence_file_name = Some(name.into());
            item
        }
    }
    impl Drop for Profile {
        fn drop(&mut self) {
            self.core.playback.close_execution();
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn completion() -> (Done, std::sync::mpsc::Receiver<()>) {
        let (tx, rx) = std::sync::mpsc::channel();
        (Done::new(move || { let _ = tx.send(()); }), rx)
    }

    #[test]
    fn real_sequence_actions_play_concurrently_finish_and_stop_without_taking_editor_transport() {
        let profile = Profile::new();
        let short = profile.sequence("short.tim", 150);
        let long = profile.sequence("long.tim", 2000);
        let runner = SequenceRunner::new(&profile.core);
        runner.pre_process(&short);
        runner.pre_process(&long);
        assert!(!runner.needs_pre_process(&short));
        let (done, ended) = completion();
        let mut first = runner.execute(&short, done);
        let (done, stopped) = completion();
        let mut second = runner.execute(&long, done);
        ended.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(stopped.try_recv().is_err(), "background is still running after sequential finishes");
        assert_eq!(profile.core.playback.transport(), crate::playback::Transport::Stopped);
        assert!(profile.core.playback.stats().frames > 1);
        assert!(!profile.core.playback.effect_preview_may_start());
        second.stop();
        stopped.recv_timeout(Duration::from_secs(1)).unwrap();
        first.stop();
        assert!(ended.try_recv().is_err(), "completion fires once even after Stop");
        runner.release(&[short, long]);
        assert!(runner.resources.lock().sequences.is_empty());
    }

    #[test]
    fn overlapping_runs_keep_shared_render_until_the_last_show_releases() {
        let profile = Profile::new();
        let item = profile.sequence("shared.tim", 100);
        let runner = SequenceRunner::new(&profile.core);
        let first = runner.for_run().unwrap();
        let second = runner.for_run().unwrap();
        first.pre_process(&item);
        second.pre_process(&item);
        assert_eq!(runner.cache.entries.lock().values().next().unwrap().sequence.strong_count(), 2);
        first.release(std::slice::from_ref(&item));
        assert!(!second.needs_pre_process(&item));
        assert_eq!(runner.cache.entries.lock().values().next().unwrap().sequence.strong_count(), 1);
        second.release(&[item]);
        assert!(runner.cache.entries.lock().is_empty());
    }

    #[test]
    fn edited_file_is_rendered_for_next_execution_without_changing_the_old_version() {
        let profile = Profile::new();
        let item = profile.sequence("edited.tim", 100);
        let runner = SequenceRunner::new(&profile.core);
        let path = runner.path(&item).unwrap();
        let old = runner.prepare(&path).unwrap();
        let previous = std::fs::metadata(&path).unwrap().modified().unwrap();
        profile.sequence("edited.tim", 250);
        let file = std::fs::File::options().write(true).open(&path).unwrap();
        file.set_times(std::fs::FileTimes::new().set_modified(previous + Duration::from_secs(2))).unwrap();
        assert!(runner.needs_pre_process(&item));
        let new = runner.prepare(&path).unwrap();
        assert_eq!(old.length, TimeSpan::from_ms_i(100));
        assert_eq!(new.length, TimeSpan::from_ms_i(250));
        assert!(!Arc::ptr_eq(&old, &new));
        assert!(!runner.needs_pre_process(&item));
    }

    #[test]
    fn missing_sequence_and_stop_before_start_both_complete() {
        let profile = Profile::new();
        let mut missing = ShowItem::new(crate::shows::ShowItemType::Sequential, "missing");
        missing.action = crate::shows::ActionType::Sequence;
        missing.sequence_file_name = Some("missing.tim".into());
        let runner = SequenceRunner::new(&profile.core);
        let (done, complete) = completion();
        let _failed = runner.execute(&missing, done);
        complete.recv_timeout(Duration::from_secs(2)).unwrap();
        let item = profile.sequence("cancel.tim", 2000);
        let (done, complete) = completion();
        let mut running = runner.execute(&item, done);
        running.stop();
        complete.recv_timeout(Duration::from_secs(1)).unwrap();
        runner.release(&[item]);
        std::thread::sleep(Duration::from_millis(50));
        assert!(profile.core.playback.effect_preview_may_start());
        assert!(runner.resources.lock().sequences.is_empty());
    }
}
