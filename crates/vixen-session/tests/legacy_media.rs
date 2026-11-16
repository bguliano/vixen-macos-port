//! Version 0 media migration through the real sequence-opening path. Every file
//! belongs to a private fixture; no original show or sound device is used.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};
use std::sync::atomic::AtomicBool;
use uuid::{uuid, Uuid};
use vixen_core::{ElementTree, TimeSpan};
use vixen_io::{load_sequence, new_sequence, MediaEntry, ModuleStore, SelectedTimingProvider};
use vixen_session::sequence::{SequenceSession, SessionEnvironment};
use vixen_xml::{XElement, XNode};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path =
            Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("legacy-media-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn environment(&self) -> SessionEnvironment {
        SessionEnvironment { video_cache: None,
            tree: Arc::new(ElementTree::new()),
            store: Arc::new(ModuleStore::new()),
            module_data_root: None,
            frame_ms: 25,
            media_dir: self.0.join("Profile/Media"),
        }
    }

    fn source(&self, parent: &str, sample: i16) -> PathBuf {
        let directory = self.0.join(parent);
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("Song & ü.wav");
        fs::write(&path, wav(sample)).unwrap();
        path
    }

    fn sequence(&self, source: &str, name: &str) -> PathBuf {
        let mut sequence = new_sequence();
        sequence.length = TimeSpan::from_ms_i(10);
        sequence.media.push(MediaEntry {
            type_id: uuid!("fe460392-3fab-4c63-99dd-d76c48354150"),
            instance_id: Uuid::new_v4(),
            file_name: String::new(),
            data: None,
        });
        sequence.selected_timing_provider = SelectedTimingProvider {
            provider_type: Some("Media".into()),
            source_name: Some(source.into()),
        };
        let mut root =
            XElement::parse(&vixen_io::sequence::sequence_to_document(&sequence).unwrap()).unwrap();
        root.set_attr("version", "0");
        let media = root
            .child_mut("_mediaSurrogates")
            .unwrap()
            .child_mut("MediaSurrogate")
            .unwrap();
        let file = media.child_mut("FileName").unwrap();
        file.local = "FilePath".into();
        file.children = vec![XNode::Text(source.into())];
        let path = self.0.join(name);
        fs::write(&path, root.to_document()).unwrap();
        path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn wav(sample: i16) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36_u32 + 160).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&1_u16.to_le_bytes()); // mono
    bytes.extend_from_slice(&8000_u32.to_le_bytes());
    bytes.extend_from_slice(&16000_u32.to_le_bytes());
    bytes.extend_from_slice(&2_u16.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&160_u32.to_le_bytes());
    for _ in 0..80 {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}

fn assert_audio(session: &SequenceSession, sample: f32) {
    let audio = session
        .audio()
        .expect("restored audio decodes without a sound device");
    assert_eq!(audio.sample_rate(), 8000);
    assert_eq!(audio.frames(), 80);
    assert!(audio.samples().iter().all(|&value| value == sample));
    assert!(!session.is_modified());
}

fn assert_only_media(directory: &Path) {
    let names: Vec<_> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(names, [std::ffi::OsString::from("Song & ü.wav")]);
}

#[test]
fn decoding_legacy_xml_is_read_only_and_keeps_the_migrated_names() {
    let fixture = Fixture::new();
    let source = fixture.source("Originals", 8192);
    let path = fixture.sequence(source.to_str().unwrap(), "Old.tim");
    let before = fs::read(&path).unwrap();
    let sequence = load_sequence(&path).unwrap();
    assert_eq!(sequence.source_version, 0);
    assert_eq!(sequence.media[0].file_name, "Song & ü.wav");
    assert_eq!(
        sequence.selected_timing_provider.source_name.as_deref(),
        Some("Song & ü.wav")
    );
    assert!(!fixture.environment().media_dir.exists());
    assert_eq!(fs::read(&source).unwrap(), wav(8192));
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 2);
}

#[test]
fn cancelled_open_does_not_read_or_restore_legacy_media() {
    let fixture = Fixture::new();
    let source = fixture.source("Originals", 8192);
    let path = fixture.sequence(source.to_str().unwrap(), "Old.tim");
    let before = fs::read(&path).unwrap();
    let env = fixture.environment();
    let error = SequenceSession::open_cancellable(&path, &env, &AtomicBool::new(true)).err().unwrap();
    assert!(error.to_string().contains("cancelled"));
    assert!(!env.media_dir.exists());
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(fs::read(&source).unwrap(), wav(8192));
    // Cancellation wins even when there is no file to open.
    let error = SequenceSession::open_cancellable(&fixture.0.join("missing.tim"), &env, &AtomicBool::new(true)).err().unwrap();
    assert!(error.to_string().contains("cancelled"));
}

#[test]
fn opening_restores_reachable_media_and_saving_a_copy_reopens_without_the_old_source() {
    let fixture = Fixture::new();
    let source = fixture.source("Originals", 8192);
    let path = fixture.sequence(source.to_str().unwrap(), "Old.tim");
    let before = fs::read(&path).unwrap();
    let env = fixture.environment();
    let mut session = SequenceSession::open(&path, &env).unwrap();
    assert_audio(&session, 0.25);
    assert_only_media(&env.media_dir);
    assert_eq!(
        fs::read(env.media_dir.join("Song & ü.wav")).unwrap(),
        wav(8192)
    );
    assert_eq!(fs::read(&source).unwrap(), wav(8192));
    assert_eq!(fs::read(&path).unwrap(), before);
    let saved = fixture.0.join("Saved.tim");
    session.save(Some(&saved)).unwrap();
    assert_eq!(fs::read(&path).unwrap(), before);
    fs::remove_file(&source).unwrap();
    let reopened = SequenceSession::open(&saved, &env).unwrap();
    assert_audio(&reopened, 0.25);
    assert_eq!(
        reopened.document().source_version,
        vixen_io::sequence::CURRENT_SEQUENCE_VERSION
    );
    assert_only_media(&env.media_dir);
}

#[test]
fn an_existing_profile_media_file_wins_without_being_overwritten() {
    let fixture = Fixture::new();
    let source = fixture.source("Originals", 8192);
    let path = fixture.sequence(source.to_str().unwrap(), "Old.tim");
    let env = fixture.environment();
    fs::create_dir_all(&env.media_dir).unwrap();
    let target = env.media_dir.join("Song & ü.wav");
    fs::write(&target, wav(-16384)).unwrap();
    let session = SequenceSession::open(&path, &env).unwrap();
    assert_audio(&session, -0.5);
    assert_eq!(fs::read(&target).unwrap(), wav(-16384));
    assert_eq!(fs::read(&source).unwrap(), wav(8192));
    assert_only_media(&env.media_dir);
}

#[test]
fn an_unavailable_windows_source_still_opens_and_retains_the_media_name() {
    let fixture = Fixture::new();
    let path = fixture.sequence(r"C:\Originals\Song & ü.wav", "Old.tim");
    let before = fs::read(&path).unwrap();
    let env = fixture.environment();
    let session = SequenceSession::open(&path, &env).unwrap();
    assert!(session.audio().is_none());
    assert!(!session.is_modified());
    assert_eq!(session.document().media[0].file_name, "Song & ü.wav");
    assert_eq!(
        session
            .document()
            .selected_timing_provider
            .source_name
            .as_deref(),
        Some("Song & ü.wav")
    );
    assert!(!env.media_dir.exists());
    assert_eq!(fs::read(&path).unwrap(), before);
}

#[test]
fn a_failed_copy_preserves_sources_removes_temporary_files_and_can_be_retried() {
    let fixture = Fixture::new();
    let source = fixture.source("Originals", 8192);
    let path = fixture.sequence(source.to_str().unwrap(), "Old.tim");
    let before = fs::read(&path).unwrap();
    let env = fixture.environment();
    let target = env.media_dir.join("Song & ü.wav");
    fs::create_dir_all(&target).unwrap();
    assert!(SequenceSession::open(&path, &env).is_err());
    assert!(target.is_dir());
    assert_only_media(&env.media_dir);
    assert_eq!(fs::read(&source).unwrap(), wav(8192));
    assert_eq!(fs::read(&path).unwrap(), before);
    fs::remove_dir(&target).unwrap();
    let session = SequenceSession::open(&path, &env).unwrap();
    assert_audio(&session, 0.25);
    assert_only_media(&env.media_dir);
}

#[test]
fn concurrent_restoration_keeps_one_complete_file_without_clobbering() {
    let fixture = Fixture::new();
    let first = fixture.source("First", 8192);
    let second = fixture.source("Second", -16384);
    let paths = [
        fixture.sequence(first.to_str().unwrap(), "First.tim"),
        fixture.sequence(second.to_str().unwrap(), "Second.tim"),
    ];
    let barrier = Barrier::new(2);
    let env = fixture.environment();
    std::thread::scope(|scope| {
        for path in &paths {
            let barrier = &barrier;
            let directory = &env.media_dir;
            scope.spawn(move || {
                let mut document = load_sequence(path).unwrap();
                barrier.wait();
                document.restore_migrated_media(directory).unwrap();
            });
        }
    });
    let actual = fs::read(env.media_dir.join("Song & ü.wav")).unwrap();
    assert!(actual == wav(8192) || actual == wav(-16384));
    assert_only_media(&env.media_dir);
    assert_eq!(fs::read(first).unwrap(), wav(8192));
    assert_eq!(fs::read(second).unwrap(), wav(-16384));
}
