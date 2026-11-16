mod common;
use common::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use vixen_audio::{AudioClip, Peaks};
use vixen_core::TimeSpan;
use vixen_effects::framework::AudioSource;

#[test]
fn advertised_mp2_decodes_without_mp3_delay_trimming() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/assets/tone.mp2");
    let clip = AudioClip::load(&path).unwrap();
    assert_eq!((clip.channels(), clip.sample_rate()), (2, 44100));
    // Ten MPEG Layer II frames; the MP3-only 528-sample delay must not apply.
    assert_eq!(clip.frames(), 10 * 1152);
    let mono = clip.mono();
    let peak = mono.iter().copied().map(f32::abs).fold(0.0f32, f32::max);
    assert!((0.07..0.11).contains(&peak), "peak {peak}");
    let crossings = mono[2000..9000].windows(2).filter(|s| s[0] <= 0.0 && s[1] > 0.0).count();
    assert!((68..=71).contains(&crossings), "440 Hz tone crossed zero {crossings} times");
}

#[test]
fn advertised_formats_missing_from_the_primary_decoder_use_the_bundled_fallback() {
    for (name, channels, rate) in [
        ("tone.wma", 2, 44100), ("tone.avi", 2, 44100),
        ("tone.m2ts", 2, 44100), ("tone.ac3", 2, 44100),
        ("tone.ec3", 2, 44100), ("tone.adpcm.wav", 2, 44100),
        ("tone.mono.wma", 1, 22050),
    ] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/assets").join(name);
        let clip = AudioClip::load(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!((clip.channels(), clip.sample_rate()), (channels, rate), "{name}");
        assert!((0.18..0.37).contains(&clip.duration().total_secs()), "{name}: {} frames", clip.frames());
        assert!(clip.samples().iter().all(|s| s.is_finite()), "{name}");
        let peak = clip.samples().iter().copied().map(f32::abs).fold(0.0f32, f32::max);
        assert!((0.05..0.15).contains(&peak), "{name}: peak {peak}");
    }
}

#[test]
fn a_live_bundled_audio_decode_cancels_after_its_first_pcm_chunk() {
    use std::ffi::OsString;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;
    use vixen_media_tools::{stream, Error, Tool};

    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/assets/tone.wma");
    let mut args: Vec<OsString> = ["-stream_loop", "-1", "-threads", "2", "-i"]
        .into_iter().map(OsString::from).collect();
    args.push(path.into_os_string());
    args.extend(["-map", "0:a:0", "-vn", "-sn", "-dn", "-acodec", "pcm_f32le", "-f", "f32le", "pipe:1"]
        .into_iter().map(OsString::from));
    let cancel = AtomicBool::new(false);
    let mut decoded = Vec::new();
    let start = Instant::now();
    let error = stream(Tool::Ffmpeg, &args, &cancel, Some(Duration::from_secs(5)), |bytes| {
        // Looping the tiny real media keeps the decoder alive without a large fixture,
        // sleeps or a test-only production hook. Cancellation occurs after actual PCM.
        decoded.extend_from_slice(bytes);
        cancel.store(true, Ordering::Release);
        Ok(())
    }).unwrap_err();
    assert!(matches!(error, Error::Cancelled), "{error}");
    assert!(start.elapsed() < Duration::from_secs(5));
    assert!(decoded.len() >= 8, "the decoder produced real stereo PCM before cancellation");
    let samples: Vec<f32> = decoded.chunks_exact(4).map(|b| f32::from_le_bytes(b.try_into().unwrap())).collect();
    assert!(samples.iter().all(|s| s.is_finite()));
    assert!(samples.iter().any(|s| s.abs() > 0.01), "decoded tone, not just a running process");
}

#[test]
fn cancelled_audio_load_never_enters_either_decoder() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/assets/tone.wma");
    assert!(matches!(AudioClip::load_cancellable(&path, &std::sync::atomic::AtomicBool::new(true)), Err(vixen_audio::AudioError::Cancelled)));
}

#[test]
fn wav_i16_round_trip_is_exact() {
    let dir = tmp_dir("wav_i16");
    for (ch, rate) in [(1u16, 22050u32), (2, 44100), (2, 48000)] {
        let frames = rate as usize * 2; // exactly two seconds
        let mut data = Vec::new();
        let mut r = Lcg(7 + ch as u64);
        for _ in 0..frames * ch as usize {
            data.push((r.next_f32() * 32767.0) as i16);
        }
        data[0] = i16::MIN;
        data[1] = i16::MAX;
        let p = dir.join(format!("t_{ch}_{rate}.wav"));
        write_wav_i16(&p, ch, rate, &data);
        let clip = AudioClip::load(&p).unwrap();
        assert_eq!(clip.channels(), ch);
        assert_eq!(clip.sample_rate(), rate);
        assert_eq!(clip.frames(), frames);
        assert_eq!(clip.duration(), TimeSpan::from_ticks(20_000_000));
        let expect: Vec<f32> = data.iter().map(|&v| v as f32 / 32768.0).collect();
        assert_eq!(clip.samples(), &expect[..]);
    }
}

#[test]
fn wav_f32_round_trip_is_exact() {
    let dir = tmp_dir("wav_f32");
    let mut r = Lcg(99);
    let data: Vec<f32> = (0..44100 * 2 * 3).map(|_| r.next_f32()).collect();
    let p = dir.join("t.wav");
    write_wav_f32(&p, 2, 44100, &data);
    let clip = AudioClip::load(&p).unwrap();
    assert_eq!(clip.samples(), &data[..]);
    assert_eq!(clip.duration(), TimeSpan::from_ticks(30_000_000));
}

#[test]
fn mono_view_is_cached_average() {
    let clip = AudioClip::from_interleaved(vec![1.0, 0.0, 0.5, -0.5], 2, 100);
    let m = clip.mono_view();
    assert_eq!(m.samples(), &[0.5, 0.0]);
    assert_eq!(m.sample_rate(), 100);
    assert_eq!(m.duration(), TimeSpan::from_ticks(200_000));
    assert!(std::ptr::eq(clip.mono().as_ptr(), clip.mono().as_ptr()));
    let a: &dyn AudioSource = &clip;
    assert_eq!(a.samples().len(), 2);
}

#[test]
fn load_errors_are_values() {
    assert!(AudioClip::load(Path::new("/definitely/not/here.mp3")).is_err());
    let dir = tmp_dir("bad");
    let p = dir.join("garbage.mp3");
    std::fs::write(&p, vec![0x55u8; 5000]).unwrap();
    assert!(AudioClip::load(&p).is_err());
}

fn brute(clip: &AudioClip, f0: usize, f1: usize) -> (f32, f32) {
    let ch = clip.channels() as usize;
    let total = clip.frames();
    if f0 >= total {
        return (0.0, 0.0);
    }
    let f1 = f1.min(total);
    let s = &clip.samples()[f0 * ch..f1 * ch];
    (s.iter().cloned().fold(f32::INFINITY, f32::min), s.iter().cloned().fold(f32::NEG_INFINITY, f32::max))
}

#[test]
fn peaks_match_brute_force_at_many_zooms() {
    for (ch, frames) in [(1u16, 100_003usize), (2, 220_501), (3, 70_000)] {
        let clip = noise_clip(44100, ch, frames, 5 + ch as u64);
        let peaks = Peaks::compute(clip.clone());
        let dur = clip.duration().total_secs();
        let mut r = Lcg(1234);
        let mut cases: Vec<(f64, f64, usize)> = vec![
            (0.0, dur, 1),
            (0.0, dur, 7),
            (0.0, dur, 1000),
            (0.0, dur, 20_000),
            (0.0, dur, 300_000), // more buckets than frames
            (dur * 0.5, dur * 1.5, 500), // runs past the end
            (dur + 1.0, dur + 2.0, 10),  // entirely outside
        ];
        for _ in 0..200 {
            let a = (r.next_f32().abs() as f64) * dur;
            let len = (r.next_f32().abs() as f64).powi(3) * dur + 0.0005;
            let b = (a + len).min(dur);
            let buckets = 1 + (r.next_f32().abs() * 900.0) as usize;
            cases.push((a, b, buckets));
        }
        for (a, b, buckets) in cases {
            let (st, en) = (TimeSpan::from_secs(a), TimeSpan::from_secs(b));
            let got = peaks.range(st, en, buckets);
            assert_eq!(got.len(), buckets);
            let (s, e) = peaks.frame_range(st, en);
            for (i, g) in got.iter().enumerate() {
                let (f0, f1) = Peaks::bucket_frames(s, e, buckets, i);
                assert_eq!(*g, brute(&clip, f0, f1), "ch={ch} range=({a},{b}) buckets={buckets} i={i}");
            }
        }
    }
}

#[test]
fn peaks_of_sine_have_right_shape_and_are_fast() {
    let clip = sine_clip(44100, 2, 600.0, 440.0, 0.5);
    let t = Instant::now();
    let peaks = Peaks::compute(clip.clone());
    let build = t.elapsed();
    let t = Instant::now();
    let mut n = 0;
    for z in 0..200 {
        let len = 600.0 / (1.0 + z as f64);
        n += peaks.range(TimeSpan::ZERO, TimeSpan::from_secs(len), 1920).len();
    }
    println!("peaks build {:?} (10 min stereo), 200 redraws x 1920 buckets {:?}", build, t.elapsed());
    assert_eq!(n, 200 * 1920);
    let v = peaks.range(TimeSpan::ZERO, TimeSpan::from_secs(600.0), 100);
    for (mn, mx) in v {
        assert!((mx - 0.5).abs() < 1e-3 && (mn + 0.5).abs() < 1e-3);
    }
}

#[test]
fn cancelled_peaks_return_none() {
    let clip = noise_clip(8000, 1, 8000 * 4096 / 8 * 4, 1);
    let c = std::sync::atomic::AtomicBool::new(true);
    assert!(Peaks::compute_cancellable(clip, &c).is_none());
}

fn data_dir() -> Option<PathBuf> {
    let root = std::env::var("VIXEN_TEST_DATA")
        .unwrap_or_else(|_| "/Users/developer/Documents/Vixen 3".to_string());
    let p = PathBuf::from(root).join("Media");
    p.is_dir().then_some(p)
}

#[test]
fn every_real_media_file_decodes() {
    let Some(dir) = data_dir() else {
        eprintln!("skipping: no test data");
        return;
    };
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            matches!(
                p.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref(),
                Some("mp3" | "m4a" | "aac" | "wav" | "flac" | "ogg" | "aif" | "aiff")
            )
        })
        .collect();
    files.sort();
    assert!(!files.is_empty());
    let mut largest: Option<(u64, String, f64)> = None;
    for f in &files {
        let size = std::fs::metadata(f).unwrap().len();
        let t = Instant::now();
        let clip = AudioClip::load(f).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
        let dt = t.elapsed().as_secs_f64();
        let dur = clip.duration().total_secs();
        let declared = clip.declared_duration().map(|d| d.total_secs());
        println!(
            "{:<52} {:>7.1} KB {:>2}ch {}Hz {:>7.2}s declared={:?} concealed={} decode={:.3}s ({:.0}x realtime)",
            f.file_name().unwrap().to_string_lossy(),
            size as f64 / 1024.0,
            clip.channels(),
            clip.sample_rate(),
            dur,
            declared.map(|d| (d * 100.0).round() / 100.0),
            clip.concealed_frames(),
            dt,
            dur / dt
        );
        assert!(dur > 1.0, "{} too short", f.display());
        if let Some(d) = declared {
            assert!((d - dur).abs() <= 0.1, "{}: decoded {dur}s vs declared {d}s", f.display());
        }
        let peaks = Peaks::compute(Arc::new(clip));
        let v = peaks.range(TimeSpan::ZERO, peaks.clip().duration(), 2000);
        let loud = v.iter().map(|(a, b)| a.abs().max(b.abs())).fold(0.0f32, f32::max);
        let active = v.iter().filter(|(a, b)| b - a > 0.01).count();
        assert!(loud > 0.05 && active > 1000, "{} looks silent (peak {loud}, active {active})", f.display());
        assert!(loud <= 4.0);
        if largest.as_ref().map(|l| size > l.0).unwrap_or(true) {
            largest = Some((size, f.file_name().unwrap().to_string_lossy().into_owned(), dt));
        }
    }
    let l = largest.unwrap();
    println!("largest file: {} ({:.1} MB) decoded in {:.3}s", l.1, l.0 as f64 / 1e6, l.2);
}
