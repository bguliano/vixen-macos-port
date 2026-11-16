use super::*;
use std::sync::atomic::AtomicUsize;

struct Fixture {
    root: PathBuf,
    source: PathBuf,
    cache: Arc<VideoCache>,
    request: VideoRequest,
}
impl Fixture {
    fn new(clear: bool, format: &str) -> Self {
        let root = std::env::temp_dir().join(format!("vixen-video-cache-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.mov");
        std::fs::write(&source, b"fixture source").unwrap();
        let cache = VideoCache::new(
            root.join("cache"),
            CacheOptions {
                clear_on_exit: clear,
                file_type: format.into(),
            },
        );
        Self {
            root,
            source,
            cache,
            request: VideoRequest {
                start_seconds: 0.0,
                duration_seconds: 1.0,
                fps: 4.0,
                width: 8,
                height: 4,
            },
        }
    }
    fn lease(&self, instance: Uuid) -> CacheLease {
        let lease = self.cache.session();
        lease.set_live_effects([instance]);
        lease
    }
    fn build(&self, lease: &CacheLease, instance: Uuid) -> Arc<CachedFrames> {
        lease
            .session()
            .frames_with(instance, &self.source, &self.request, "variant", &AtomicBool::new(false), fake)
            .unwrap()
    }
    fn folders(&self) -> Vec<PathBuf> {
        std::fs::read_dir(self.cache.root())
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| valid_key(&entry.file_name().to_string_lossy()))
            .map(|entry| entry.path())
            .collect()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn fake(sink: &mut dyn FnMut(FrameEvent<'_>) -> bool) -> Option<usize> {
    for i in 0..4 {
        let frame = Arc::new(Frame {
            width: 8,
            height: 4,
            rgba: [i as u8 * 60, 24, 128, 255].repeat(32),
        });
        if !sink(FrameEvent::Frame(i, &frame)) {
            return None;
        }
    }
    Some(4)
}
fn missing(_: &mut dyn FnMut(FrameEvent<'_>) -> bool) -> Option<usize> {
    panic!("retained frames must be read from disk")
}

#[test]
fn bmp_and_png_are_real_images_reused_after_a_fresh_manager_reopens() {
    for format in ["bmp", "png"] {
        let f = Fixture::new(false, format);
        let id = Uuid::new_v4();
        let lease = f.lease(id);
        let frames = f.build(&lease, id);
        assert_eq!(frames.len(), 4);
        assert_eq!(frames.frame(2).unwrap().rgba[0], 120);
        let folder = f.folders().pop().unwrap();
        let bytes = std::fs::read(folder.join(format!("00000000.{format}"))).unwrap();
        assert!(if format == "bmp" {
            bytes.starts_with(b"BM")
        } else {
            bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        });
        let actual_size: u64 = (0..4).map(|i| folder.join(format!("{i:08}.{format}")).metadata().unwrap().len()).sum();
        assert_eq!(frames.encoded_bytes(), actual_size);
        assert_eq!(lease.session().cache_size(id), actual_size);
        drop(frames);
        drop(lease);
        assert!(folder.exists());
        let reopened = VideoCache::new(f.cache.root().to_path_buf(), f.cache.options());
        let lease = reopened.session();
        lease.set_live_effects([id]);
        let frames = lease.session().frames_with(id, &f.source, &f.request, "variant", &AtomicBool::new(false), missing).unwrap();
        assert_eq!(frames.frame(3).unwrap().rgba[0], 180);
        for i in 0..4 {
            frames.frame(i).unwrap();
        }
        assert_eq!(frames.decoded.lock().unwrap().len(), 2, "only two images remain decoded");
    }
}

#[test]
fn clear_deleted_and_undo_lifetimes_keep_frames_until_the_last_reader_releases() {
    let f = Fixture::new(true, "bmp");
    let id = Uuid::new_v4();
    let lease = f.lease(id);
    let frames = f.build(&lease, id);
    let folder = frames.directory.clone();
    lease.set_live_effects([]);
    assert!(folder.exists(), "delete preserves frames for Undo");
    lease.set_live_effects([id]);
    assert_eq!(frames.frame(1).unwrap().rgba[0], 60);
    lease.close();
    assert!(folder.exists(), "a render reader pins its file set through close");
    assert_eq!(frames.frame(2).unwrap().rgba[0], 120);
    drop(frames);
    assert!(!folder.exists());

    let f = Fixture::new(false, "png");
    let lease = f.lease(id);
    let frames = f.build(&lease, id);
    let folder = frames.directory.clone();
    drop(frames);
    lease.set_live_effects([]);
    assert!(folder.exists());
    drop(lease);
    assert!(!folder.exists(), "deleted effect is unpaired even with retention enabled");
}

#[test]
fn an_unrenderable_reopened_effect_still_removes_old_pairings_on_delete_or_clear() {
    for clear in [false, true] {
        let f = Fixture::new(false, "bmp");
        let id = Uuid::new_v4();
        let lease = f.lease(id);
        let frames = f.build(&lease, id);
        let folder = frames.directory.clone();
        drop(frames);
        drop(lease);
        let lease = f.lease(id);
        if clear {
            f.cache.set_options(CacheOptions::default());
        } else {
            lease.set_live_effects([]);
        }
        // No decode in the second session: missing source/invalid effect still owns its old pairing.
        drop(lease);
        assert!(!folder.exists());
    }
}

#[test]
fn settings_and_source_changes_invalidate_and_unknown_types_stay_unchanged() {
    let f = Fixture::new(false, "bmp");
    let id = Uuid::new_v4();
    let lease = f.lease(id);
    let first = f.build(&lease, id);
    let first_key = first.key.clone();
    drop(first);
    f.cache.set_options(CacheOptions {
        clear_on_exit: false,
        file_type: "png".into(),
    });
    let second = f.build(&lease, id);
    assert_ne!(first_key, second.key);
    let second_key = second.key.clone();
    drop(second);
    std::fs::write(&f.source, b"a different source size").unwrap();
    let third = f.build(&lease, id);
    assert_ne!(second_key, third.key);
    drop(third);
    f.cache.set_options(CacheOptions {
        clear_on_exit: false,
        file_type: "not-a-codec".into(),
    });
    assert!(lease
        .session()
        .frames_with(id, &f.source, &f.request, "variant", &AtomicBool::new(false), missing)
        .is_none());
    assert_eq!(f.cache.options().file_type, "not-a-codec");
    f.cache.set_options(CacheOptions::default());
    drop(lease);
    assert!(f.folders().is_empty());
}

#[test]
fn concurrent_windows_share_one_build_and_closing_one_does_not_remove_the_other() {
    let f = Fixture::new(true, "png");
    let a = f.lease(Uuid::nil());
    let b = f.lease(Uuid::nil());
    let calls = AtomicUsize::new(0);
    let build = |lease: &CacheLease| {
        lease
            .session()
            .frames_with(Uuid::nil(), &f.source, &f.request, "variant", &AtomicBool::new(false), |sink| {
                calls.fetch_add(1, Ordering::Relaxed);
                fake(sink)
            })
            .unwrap()
    };
    let (first, second) = std::thread::scope(|scope| {
        let one = scope.spawn(|| build(&a));
        let two = scope.spawn(|| build(&b));
        (one.join().unwrap(), two.join().unwrap())
    });
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert!(Arc::ptr_eq(&first, &second));
    let folder = first.directory.clone();
    drop(first);
    drop(a);
    assert!(folder.exists());
    assert_eq!(second.frame(0).unwrap().rgba[2], 128);
    drop(second);
    assert!(folder.exists());
    drop(b);
    assert!(!folder.exists());
}

#[test]
fn close_or_shutdown_during_decode_never_publishes_a_late_cache() {
    for shutdown in [false, true] {
        let f = Fixture::new(false, "bmp");
        let id = Uuid::new_v4();
        let lease = f.lease(id);
        let session = lease.session().clone();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let (go_tx, go_rx) = std::sync::mpsc::channel();
        std::thread::scope(|scope| {
            let source = &f.source;
            let request = &f.request;
            let worker = scope.spawn(move || {
                session.frames_with(id, source, request, "variant", &AtomicBool::new(false), |sink| {
                    ready_tx.send(()).unwrap();
                    go_rx.recv().unwrap();
                    fake(sink)
                })
            });
            ready_rx.recv().unwrap();
            if shutdown {
                f.cache.shutdown();
            } else {
                lease.close();
            }
            go_tx.send(()).unwrap();
            assert!(worker.join().unwrap().is_none());
        });
        assert!(f.folders().is_empty());
        assert!(std::fs::read_dir(f.cache.root()).unwrap().flatten().all(|e| e.file_name() == ".owner"));
    }
}

#[test]
fn corrupt_manifests_and_symlink_frames_do_not_escape_owned_cache_data() {
    let f = Fixture::new(false, "bmp");
    let id = Uuid::new_v4();
    let lease = f.lease(id);
    let frames = f.build(&lease, id);
    let folder = frames.directory.clone();
    drop(frames);
    std::fs::write(folder.join("manifest.json"), b"bad").unwrap();
    let rebuilt = f.build(&lease, id);
    assert_eq!(rebuilt.len(), 4);
    drop(rebuilt);
    let file = folder.join("00000000.bmp");
    std::fs::remove_file(&file).unwrap();
    std::os::unix::fs::symlink(&f.source, &file).unwrap();
    let rebuilt = f.build(&lease, id);
    assert_eq!(rebuilt.frame(0).unwrap().rgba[2], 128);
    assert_eq!(std::fs::read(&f.source).unwrap(), b"fixture source");
    let untouched = f.root.join("untouched");
    std::fs::create_dir(&untouched).unwrap();
    std::fs::write(untouched.join("important"), b"keep").unwrap();
    let other = VideoCache::new(f.root.join("linked"), CacheOptions::default());
    std::os::unix::fs::symlink(&untouched, other.root()).unwrap();
    let other_lease = other.session();
    assert!(other_lease
        .session()
        .frames_with(id, &f.source, &f.request, "variant", &AtomicBool::new(false), fake)
        .is_none());
    other.shutdown();
    assert_eq!(std::fs::read(untouched.join("important")).unwrap(), b"keep");
}

#[test]
fn long_cache_streams_are_not_limited_by_the_legacy_vector_budget() {
    let f = Fixture::new(false, "png");
    let id = Uuid::new_v4();
    let lease = f.lease(id);
    let request = VideoRequest {
        width: 800,
        height: 600,
        duration_seconds: 30.0,
        fps: 20.0,
        ..f.request
    };
    // More than1GiB of decoded frames, but only one input frame and two lazy
    // output frames resident. PNG encodes this synthetic constant image compactly.
    let frame = Arc::new(Frame {
        width: request.width,
        height: request.height,
        rgba: [10, 20, 30, 255].repeat(800 * 600),
    });
    let frames = lease
        .session()
        .frames_with(id, &f.source, &request, "long", &AtomicBool::new(false), |sink| {
            for i in 0..600 {
                if !sink(FrameEvent::Frame(i, &frame)) {
                    return None;
                }
            }
            Some(600)
        })
        .unwrap();
    assert_eq!(frames.len(), 600);
    assert_eq!(frames.frame(599).unwrap().rgba[0], 10);
    assert_eq!(frames.decoded.lock().unwrap().len(), 1);
}

#[test]
fn in_flight_disk_reader_protects_last_pairing_while_another_window_closes() {
    let f = Fixture::new(true, "bmp");
    let id = Uuid::new_v4();
    let first = f.lease(id);
    let frames = f.build(&first, id);
    let folder = frames.directory.clone();
    let key = frames.key.clone();
    drop(frames);
    let second = f.lease(id);
    let lock = Arc::new(Mutex::new(()));
    f.cache.state.lock().unwrap().locks.insert(key.clone(), Arc::downgrade(&lock));
    let held = lock.lock().unwrap();
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| {
            second
                .session()
                .frames_with(id, &f.source, &f.request, "variant", &AtomicBool::new(false), missing)
                .unwrap()
        });
        let start = std::time::Instant::now();
        while !f.cache.state.lock().unwrap().inflight.contains_key(&key) {
            assert!(start.elapsed() < std::time::Duration::from_secs(2));
            std::thread::yield_now();
        }
        first.close();
        assert!(folder.exists());
        drop(held);
        let frames = worker.join().unwrap();
        assert_eq!(frames.frame(3).unwrap().rgba[0], 180);
        drop(frames);
    });
    assert!(folder.exists());
    second.close();
    assert!(!folder.exists());
}

#[test]
fn same_size_corrupted_image_invalidates_the_set_and_next_render_rebuilds() {
    let f = Fixture::new(false, "png");
    let id = Uuid::new_v4();
    let lease = f.lease(id);
    let frames = f.build(&lease, id);
    let path = frames.directory.join("00000000.png");
    let bytes = path.metadata().unwrap().len() as usize;
    std::fs::write(&path, vec![0; bytes]).unwrap();
    assert!(frames.frame(0).is_none());
    let replacement = f.build(&lease, id);
    assert_eq!(replacement.frame(0).unwrap().rgba[2], 128);
    assert!(!Arc::ptr_eq(&frames, &replacement));
}

#[test]
fn decoder_retry_resets_only_partial_staging_frames_before_atomic_publish() {
    let f = Fixture::new(false, "bmp");
    let id = Uuid::new_v4();
    let lease = f.lease(id);
    let old = Arc::new(Frame {
        width: 8,
        height: 4,
        rgba: [255, 0, 0, 255].repeat(32),
    });
    let frames = lease
        .session()
        .frames_with(id, &f.source, &f.request, "retry", &AtomicBool::new(false), |sink| {
            assert!(sink(FrameEvent::Frame(0, &old)));
            assert!(sink(FrameEvent::Frame(1, &old)));
            assert!(sink(FrameEvent::Reset));
            fake(sink)
        })
        .unwrap();
    assert_eq!(frames.len(), 4);
    assert_eq!(frames.frame(0).unwrap().rgba[0], 0);
    assert_eq!(frames.frame(3).unwrap().rgba[0], 180);
    assert_eq!(
        frames.encoded_bytes(),
        (0..4).map(|i| frames.directory.join(format!("{i:08}.bmp")).metadata().unwrap().len()).sum::<u64>()
    );
}

#[test]
fn legacy_stored_image_types_use_available_encoders_without_rewriting_the_choice() {
    for format in ["jpg", "GIF", "ico"] {
        let f = Fixture::new(false, format);
        let id = Uuid::new_v4();
        let lease = f.lease(id);
        let frames = lease
            .session()
            .frames_with(id, &f.source, &f.request, "variant", &AtomicBool::new(false), fake)
            .unwrap_or_else(|| panic!("could not encode stored {format}"));
        assert_eq!(f.cache.options().file_type, format);
        assert_eq!(frames.frame(0).unwrap().width, 8);
        let first = frames.directory.join(format!("00000000.{}", format.to_ascii_lowercase()));
        assert_eq!(
            image::guess_format(&std::fs::read(first).unwrap()).unwrap(),
            image::ImageFormat::from_extension(format).unwrap()
        );
    }
}
