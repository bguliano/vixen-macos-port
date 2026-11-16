//! Persistent frame images and the lifetime of their owning effect instances.
//! A profile owns one manager; open sequences own leases, while render workers
//! only borrow sessions. Closing a lease is final even if a worker finishes later.
use crate::{video_frames_to_resettable, Frame, FrameEvent, VideoRequest};
use serde::{Deserialize, Serialize};
use siphasher::sip128::{Hasher128, SipHasher13};
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hasher;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};
use uuid::Uuid;

const VERSION: u32 = 1;
const OWNER: &[u8] = b"Vixen macOS video cache v1\n";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CacheOptions {
    pub clear_on_exit: bool,
    pub file_type: String,
}
impl Default for CacheOptions {
    fn default() -> Self {
        Self {
            clear_on_exit: true,
            file_type: "bmp".into(),
        }
    }
}

fn hash(text: &str) -> String {
    let mut hash = SipHasher13::new();
    hash.write(text.as_bytes());
    format!("{:032x}", hash.finish128().as_u128())
}
/// Like upstream this is disposable temporary data, never source media. Separate
/// profile roots prevent one profile's close/options from changing another's cache.
pub fn profile_cache_root(profile: &Path) -> PathBuf {
    let path = profile.canonicalize().unwrap_or_else(|_| profile.to_path_buf());
    std::env::temp_dir().join("Vixen").join("VideoEffect").join("mac-v1").join(hash(&path.to_string_lossy()))
}
struct State {
    options: CacheOptions,
    sessions: HashMap<Uuid, Weak<CacheSession>>,
    locks: HashMap<String, Weak<Mutex<()>>>,
    sets: HashMap<String, Weak<CachedFrames>>,
    pins: HashMap<String, usize>,
    retired: HashSet<String>,
    shutting_down: bool,
    inflight: HashMap<String, usize>,
}
pub struct VideoCache {
    root: PathBuf,
    state: Mutex<State>,
}
impl VideoCache {
    pub fn new(root: PathBuf, options: CacheOptions) -> Arc<Self> {
        Arc::new(Self {
            root,
            state: Mutex::new(State {
                options,
                sessions: HashMap::new(),
                locks: HashMap::new(),
                sets: HashMap::new(),
                pins: HashMap::new(),
                retired: HashSet::new(),
                shutting_down: false,
                inflight: HashMap::new(),
            }),
        })
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn set_options(&self, options: CacheOptions) {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).options = options;
    }
    pub fn options(&self) -> CacheOptions {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).options.clone()
    }
    pub fn session(self: &Arc<Self>) -> CacheLease {
        let session = Arc::new(CacheSession {
            cache: self.clone(),
            id: Uuid::new_v4(),
            closed: AtomicBool::new(false),
            effects: Mutex::new(HashMap::new()),
            live: Mutex::new(HashSet::new()),
            known: Mutex::new(HashSet::new()),
            sizes: Mutex::new(HashMap::new()),
        });
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.shutting_down {
            session.closed.store(true, Ordering::Release);
        }
        state.sessions.insert(session.id, Arc::downgrade(&session));
        CacheLease { session }
    }
    /// Application shutdown also closes non-editor sequences still retained by a
    /// scheduler/export owner. Workers cannot publish after this point.
    pub fn shutdown(&self) {
        let sessions = {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            state.shutting_down = true;
            state.sessions.values().filter_map(Weak::upgrade).collect::<Vec<_>>()
        };
        for session in sessions {
            session.close();
        }
    }
    fn collect(&self, state: &mut State, discover: bool) {
        if (!discover && state.retired.is_empty()) || !owned_dir(&self.root) {
            return;
        }
        let mut referenced: HashSet<String> = HashSet::new();
        state.sessions.retain(|_, weak| weak.strong_count() > 0);
        for session in state.sessions.values().filter_map(Weak::upgrade) {
            referenced.extend(session.effects.lock().unwrap_or_else(|e| e.into_inner()).values().cloned());
        }
        if let Ok(entries) = std::fs::read_dir(&self.root) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if let Some((id, key)) = name.split_once('.') {
                    if Uuid::parse_str(id).is_ok() && valid_key(key) && entry.file_type().is_ok_and(|t| t.is_file()) {
                        referenced.insert(key.to_string());
                    }
                }
            }
        }
        if let Ok(entries) = std::fs::read_dir(&self.root) {
            for entry in entries.flatten() {
                let key = entry.file_name().to_string_lossy().into_owned();
                // Only our hash-named directories with our manifest are owned cache data.
                if referenced.contains(&key) {
                    state.retired.remove(&key);
                    continue;
                }
                if valid_key(&key) && (discover || state.retired.contains(&key)) && entry.file_type().is_ok_and(|t| t.is_dir()) && owned_dir(&entry.path()) {
                    state.retired.insert(key.clone());
                    if !state.inflight.contains_key(&key) && !state.pins.contains_key(&key) && std::fs::remove_dir_all(entry.path()).is_ok() {
                        state.retired.remove(&key);
                    }
                }
            }
        }
        state.sets.retain(|_, weak| weak.strong_count() > 0);
        state.locks.retain(|_, lock| lock.strong_count() > 0);
    }
}
fn valid_key(key: &str) -> bool {
    key.len() == 32 && key.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The sequence/scheduler owns this value. A worker's Arc<CacheSession> cannot
/// postpone cleanup, and the manager's weak registry cannot keep a sequence alive.
pub struct CacheLease {
    session: Arc<CacheSession>,
}
impl CacheLease {
    pub fn session(&self) -> &Arc<CacheSession> {
        &self.session
    }
    pub fn set_live_effects(&self, ids: impl IntoIterator<Item = Uuid>) {
        self.session.set_live_effects(ids);
    }
    pub fn close(&self) {
        self.session.close();
    }
}
impl Drop for CacheLease {
    fn drop(&mut self) {
        self.session.close();
    }
}

#[derive(Clone, Copy)]
pub struct CacheContext<'a> {
    pub session: &'a CacheSession,
    pub instance: Uuid,
}
pub struct CacheSession {
    cache: Arc<VideoCache>,
    id: Uuid,
    closed: AtomicBool,
    effects: Mutex<HashMap<Uuid, String>>,
    live: Mutex<HashSet<Uuid>>,
    known: Mutex<HashSet<Uuid>>,
    sizes: Mutex<HashMap<Uuid, u64>>,
}
impl CacheSession {
    pub fn set_live_effects(&self, ids: impl IntoIterator<Item = Uuid>) {
        let ids: HashSet<Uuid> = ids.into_iter().collect();
        self.known.lock().unwrap_or_else(|e| e.into_inner()).extend(ids.iter().copied());
        *self.live.lock().unwrap_or_else(|e| e.into_inner()) = ids;
    }
    pub fn frames(&self, instance: Uuid, path: &Path, request: &VideoRequest, variant: &str, cancel: &AtomicBool) -> Option<Arc<CachedFrames>> {
        self.frames_with(instance, path, request, variant, cancel, |sink| video_frames_to_resettable(path, request, cancel, sink))
    }
    fn frames_with(
        &self,
        instance: Uuid,
        path: &Path,
        request: &VideoRequest,
        variant: &str,
        cancel: &AtomicBool,
        decode: impl FnOnce(&mut dyn FnMut(FrameEvent<'_>) -> bool) -> Option<usize>,
    ) -> Option<Arc<CachedFrames>> {
        if self.closed.load(Ordering::Acquire) || cancel.load(Ordering::Relaxed) {
            return None;
        }
        let options = self.cache.options();
        let extension = options.file_type.to_ascii_lowercase();
        let extension = extension.as_str();
        // The native picker offers BMP/PNG and preserves a legacy third value.
        // Honor other already-enabled image encoders (e.g. JPG/GIF/ICO), without
        // rewriting the setting or accepting path-like/unknown extensions.
        let format = image::ImageFormat::from_extension(extension).filter(image::ImageFormat::writing_enabled)?;
        let identity = identity(path, request, variant, &options.file_type)?;
        let key = hash(&identity);
        let lock = {
            let mut state = self.cache.state.lock().unwrap_or_else(|e| e.into_inner());
            let lock = state.locks.get(&key).and_then(Weak::upgrade).unwrap_or_else(|| Arc::new(Mutex::new(())));
            state.locks.insert(key.clone(), Arc::downgrade(&lock));
            *state.inflight.entry(key.clone()).or_default() += 1;
            lock
        };
        let _inflight = Inflight {
            cache: self.cache.clone(),
            key: key.clone(),
        };
        let _serial = lock.lock().unwrap_or_else(|e| e.into_inner());
        if self.closed.load(Ordering::Acquire) || cancel.load(Ordering::Relaxed) {
            return None;
        }
        let directory = self.cache.root.join(&key);
        let existing = self
            .cache
            .state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .sets
            .get(&key)
            .and_then(Weak::upgrade)
            .filter(|frames| frames.identity == identity && !frames.invalid.load(Ordering::Acquire));
        let mut staging = None;
        let manifest = if let Some(frames) = &existing {
            frames.manifest()
        } else if let Some(manifest) = read_manifest(&directory, &identity, extension, request) {
            manifest
        } else {
            ensure_root(&self.cache.root)?;
            let stage_path = self.cache.root.join(format!(".building-{}", Uuid::new_v4()));
            std::fs::create_dir(&stage_path).ok()?;
            let stage = Staging(stage_path);
            std::fs::write(stage.0.join(".owner"), OWNER).ok()?;
            let mut count = 0usize;
            let mut bytes = 0u64;
            let decoded = decode(&mut |event| {
                let (index, frame) = match event {
                    FrameEvent::Reset => {
                        // Only files created inside this unique, owned staging
                        // directory are removed when a decoder retries a codec.
                        for index in 0..count {
                            if std::fs::remove_file(stage.0.join(format!("{index:08}.{extension}"))).is_err() {
                                return false;
                            }
                        }
                        count = 0;
                        bytes = 0;
                        return true;
                    }
                    FrameEvent::Frame(index, frame) => (index, frame),
                };
                if self.closed.load(Ordering::Acquire) || cancel.load(Ordering::Relaxed) || index != count || frame.width != request.width || frame.height != request.height {
                    return false;
                }
                let path = stage.0.join(format!("{index:08}.{extension}"));
                if write_frame(&path, frame, format).is_none() {
                    return false;
                }
                let Ok(meta) = path.metadata() else {
                    return false;
                };
                let Some(total) = bytes.checked_add(meta.len()) else {
                    return false;
                };
                bytes = total;
                count += 1;
                true
            })?;
            if decoded != count || count == 0 || count > crate::frame_count(request)? {
                return None;
            }
            let manifest = Manifest {
                version: VERSION,
                identity: identity.clone(),
                extension: extension.into(),
                count,
                width: request.width,
                height: request.height,
                bytes,
            };
            std::fs::write(stage.0.join("manifest.json"), serde_json::to_vec(&manifest).ok()?).ok()?;
            staging = Some(stage);
            manifest
        };
        // Publication and close share this short lock. A closed lease cannot be
        // resurrected by a decode or disk reader which began before window close.
        let mut state = self.cache.state.lock().unwrap_or_else(|e| e.into_inner());
        if self.closed.load(Ordering::Acquire) || cancel.load(Ordering::Relaxed) || state.shutting_down {
            return None;
        }
        if let Some(stage) = &staging {
            if directory.exists() {
                if !owned_dir(&directory) {
                    return None;
                }
                std::fs::remove_dir_all(&directory).ok()?;
            }
            std::fs::rename(&stage.0, &directory).ok()?;
        }
        remove_pairings(&self.cache.root, instance);
        std::fs::write(self.cache.root.join(format!("{instance}.{key}")), []).ok()?;
        self.effects.lock().unwrap_or_else(|e| e.into_inner()).insert(instance, key.clone());
        self.known.lock().unwrap_or_else(|e| e.into_inner()).insert(instance);
        let frames = existing.unwrap_or_else(|| {
            *state.pins.entry(key.clone()).or_default() += 1;
            Arc::new(CachedFrames {
                cache: self.cache.clone(),
                key: key.clone(),
                directory,
                identity: manifest.identity,
                extension: manifest.extension,
                count: manifest.count,
                width: manifest.width,
                height: manifest.height,
                bytes: manifest.bytes,
                decoded: Mutex::new(VecDeque::new()),
                invalid: AtomicBool::new(false),
            })
        });
        state.sets.insert(key, Arc::downgrade(&frames));
        self.sizes.lock().unwrap_or_else(|e| e.into_inner()).insert(instance, frames.bytes);
        Some(frames)
    }
    fn close(&self) {
        let mut state = self.cache.state.lock().unwrap_or_else(|e| e.into_inner());
        if self.closed.swap(true, Ordering::AcqRel) {
            return;
        }
        let live = self.live.lock().unwrap_or_else(|e| e.into_inner());
        let effects = self.effects.lock().unwrap_or_else(|e| e.into_inner());
        let known = self.known.lock().unwrap_or_else(|e| e.into_inner());
        for instance in known.iter() {
            if state.options.clear_on_exit || !live.contains(instance) {
                remove_pairings(&self.cache.root, *instance);
            } else if let Some(key) = effects.get(instance) {
                remove_pairings(&self.cache.root, *instance);
                let _ = std::fs::write(self.cache.root.join(format!("{instance}.{key}")), []);
            }
        }
        drop(known);
        state.sessions.remove(&self.id);
        drop(effects);
        drop(live);
        self.cache.collect(&mut state, true);
    }
    pub fn cache_size(&self, instance: Uuid) -> u64 {
        self.sizes.lock().unwrap_or_else(|e| e.into_inner()).get(&instance).copied().unwrap_or(0)
    }
}
fn remove_pairings(root: &Path, instance: Uuid) {
    if !owned_dir(root) {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(root) {
        let prefix = format!("{instance}.");
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.strip_prefix(&prefix).is_some_and(valid_key) && entry.file_type().is_ok_and(|t| t.is_file()) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}
struct Inflight {
    cache: Arc<VideoCache>,
    key: String,
}
impl Drop for Inflight {
    fn drop(&mut self) {
        let mut state = self.cache.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(count) = state.inflight.get_mut(&self.key) {
            *count -= 1;
            if *count == 0 {
                state.inflight.remove(&self.key);
            }
        }
        self.cache.collect(&mut state, false);
    }
}
fn regular(path: &Path, max_bytes: u64) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|meta| meta.is_file() && meta.len() <= max_bytes)
}
fn owned_dir(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.is_dir())
        && regular(&path.join(".owner"), OWNER.len() as u64)
        && std::fs::read(path.join(".owner")).is_ok_and(|bytes| bytes == OWNER)
}
fn ensure_root(path: &Path) -> Option<()> {
    if owned_dir(path) {
        return Some(());
    }
    if path.exists() {
        if !std::fs::symlink_metadata(path).ok()?.is_dir() || std::fs::read_dir(path).ok()?.next().is_some() {
            return None;
        }
    } else {
        std::fs::create_dir_all(path).ok()?;
    }
    std::fs::write(path.join(".owner"), OWNER).ok()
}
struct Staging(PathBuf);
impl Drop for Staging {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[derive(Serialize, Deserialize)]
struct Manifest {
    version: u32,
    identity: String,
    extension: String,
    count: usize,
    width: i32,
    height: i32,
    bytes: u64,
}

/// Lazy frame images, with at most two decoded pictures resident. Holding this
/// value pins its folder through a render, even when its owning window closes.
pub struct CachedFrames {
    cache: Arc<VideoCache>,
    key: String,
    directory: PathBuf,
    identity: String,
    extension: String,
    count: usize,
    width: i32,
    height: i32,
    bytes: u64,
    decoded: Mutex<VecDeque<(usize, Arc<Frame>)>>,
    invalid: AtomicBool,
}
impl CachedFrames {
    pub fn len(&self) -> usize {
        self.count
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    pub fn encoded_bytes(&self) -> u64 {
        self.bytes
    }
    pub fn frame(&self, index: usize) -> Option<Arc<Frame>> {
        if index >= self.count {
            return None;
        }
        let mut decoded = self.decoded.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((_, frame)) = decoded.iter().find(|(i, _)| *i == index) {
            return Some(frame.clone());
        }
        let path = self.directory.join(format!("{index:08}.{}", self.extension));
        let load = || {
            if !regular(&path, 128 * 1024 * 1024) || image::image_dimensions(&path).ok()? != (self.width as u32, self.height as u32) {
                return None;
            }
            Some(image::open(path).ok()?.to_rgba8().into_raw())
        };
        let Some(rgba) = load() else {
            self.invalidate();
            return None;
        };
        let frame = Arc::new(Frame {
            width: self.width,
            height: self.height,
            rgba,
        });
        decoded.push_back((index, frame.clone()));
        while decoded.len() > 2 {
            decoded.pop_front();
        }
        Some(frame)
    }
    fn invalidate(&self) {
        self.invalid.store(true, Ordering::Release);
        let state = self.cache.state.lock().unwrap_or_else(|e| e.into_inner());
        let current = state.sets.get(&self.key).and_then(Weak::upgrade);
        if current.as_ref().is_some_and(|current| std::ptr::eq(Arc::as_ptr(current), self)) && owned_dir(&self.directory) {
            let _ = std::fs::remove_file(self.directory.join("manifest.json"));
        }
    }
    fn manifest(&self) -> Manifest {
        Manifest {
            version: VERSION,
            identity: self.identity.clone(),
            extension: self.extension.clone(),
            count: self.count,
            width: self.width,
            height: self.height,
            bytes: self.bytes,
        }
    }
}
impl Drop for CachedFrames {
    fn drop(&mut self) {
        let mut state = self.cache.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(count) = state.pins.get_mut(&self.key) {
            *count -= 1;
            if *count == 0 {
                state.pins.remove(&self.key);
            }
        }
        self.cache.collect(&mut state, false);
    }
}
fn identity(path: &Path, request: &VideoRequest, variant: &str, file_type: &str) -> Option<String> {
    crate::frame_count(request)?;
    if !request.start_seconds.is_finite()
        || !request.duration_seconds.is_finite()
        || !request.fps.is_finite()
        || request.start_seconds < 0.0
        || request.duration_seconds <= 0.0
        || request.fps <= 0.0
    {
        return None;
    }
    let path = path.canonicalize().ok()?;
    let meta = path.metadata().ok()?;
    let stamp = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_nanos();
    Some(
        serde_json::json!([
            VERSION,
            path.to_str()?,
            meta.len(),
            stamp,
            request.start_seconds.to_bits(),
            request.duration_seconds.to_bits(),
            request.fps.to_bits(),
            request.width,
            request.height,
            variant,
            file_type
        ])
        .to_string(),
    )
}
fn write_frame(path: &Path, frame: &Frame, format: image::ImageFormat) -> Option<()> {
    if format == image::ImageFormat::Jpeg {
        let rgb: Vec<u8> = frame.rgba.chunks_exact(4).flat_map(|pixel| pixel[..3].iter().copied()).collect();
        image::save_buffer_with_format(path, &rgb, frame.width as u32, frame.height as u32, image::ColorType::Rgb8, format).ok()
    } else {
        image::save_buffer_with_format(path, &frame.rgba, frame.width as u32, frame.height as u32, image::ColorType::Rgba8, format).ok()
    }
}
fn read_manifest(root: &Path, identity: &str, extension: &str, request: &VideoRequest) -> Option<Manifest> {
    if !owned_dir(root) || !regular(&root.join("manifest.json"), 65_536) {
        return None;
    }
    let manifest: Manifest = serde_json::from_slice(&std::fs::read(root.join("manifest.json")).ok()?).ok()?;
    let expected = crate::frame_count(request)?;
    if manifest.version != VERSION
        || manifest.identity != identity
        || manifest.extension != extension
        || manifest.count == 0
        || manifest.count > expected
        || manifest.width != request.width
        || manifest.height != request.height
    {
        return None;
    }
    let mut bytes = 0u64;
    for index in 0..manifest.count {
        let path = root.join(format!("{index:08}.{extension}"));
        if !regular(&path, 128 * 1024 * 1024) {
            return None;
        }
        bytes = bytes.checked_add(path.metadata().ok()?.len())?;
    }
    (bytes == manifest.bytes).then_some(manifest)
}
#[cfg(test)]
mod tests;
