mod common;
use common::*;
use std::sync::Arc;
use std::time::Duration;
use vixen_audio::*;
use vixen_core::TimeSpan;

fn setup(rate: u32, ch: u16, clip: Arc<AudioClip>) -> (AudioPlayer, FakeBackend) {
    let fake = FakeBackend::new(rate, ch);
    let p = AudioPlayer::with_backend(Box::new(fake.clone()), None);
    p.set_clip(clip);
    (p, fake)
}

fn ms(v: u64) -> Duration {
    Duration::from_millis(v)
}

#[test]
fn speed_one_output_is_bit_exact_stereo() {
    let clip = noise_clip(48000, 2, 48000 * 2, 3);
    let (p, fake) = setup(48000, 2, clip.clone());
    assert_eq!(p.state(), PlayerState::Stopped);
    // Not playing: silence.
    assert!(fake.pump(256).iter().all(|&v| v == 0.0));
    p.play();
    assert!(p.is_playing());
    let mut out = Vec::new();
    let sizes = [480usize, 333, 1024, 2500, 17, 4096, 1];
    let mut i = 0;
    while out.len() < clip.samples().len() + 20_000 {
        fake.advance_clock(ms(5));
        out.extend(fake.pump(sizes[i % sizes.len()]));
        i += 1;
    }
    let n = clip.samples().len();
    assert_eq!(&out[..n], clip.samples(), "speed 1.0 must be bit exact");
    assert!(out[n..].iter().all(|&v| v == 0.0));
    fake.advance_clock(ms(3000)); // queued output must actually reach the speakers
    assert!(p.has_ended());
    assert!(!p.is_playing());
    assert_eq!(p.position(), clip.duration());
    assert_eq!(p.state(), PlayerState::Stopped);
}

#[test]
fn mono_clip_on_stereo_device_duplicates_exactly() {
    let clip = noise_clip(48000, 1, 10_000, 4);
    let (p, fake) = setup(48000, 2, clip.clone());
    p.play();
    let out = fake.pump(10_000);
    for (i, f) in out.chunks(2).enumerate() {
        assert_eq!(f[0], clip.samples()[i]);
        assert_eq!(f[1], clip.samples()[i]);
    }
}

#[test]
fn volume_scales() {
    let clip = noise_clip(48000, 1, 4000, 4);
    let (p, fake) = setup(48000, 1, clip.clone());
    p.set_volume(0.5);
    p.play();
    fake.pump(1); // first buffer ramps from the initial gain
    p.seek(TimeSpan::ZERO);
    let out = fake.pump(2000);
    // After the first chunk the gain is settled.
    let out2 = fake.pump(1000);
    assert!(out.len() == 2000 && out2.len() == 1000);
    for (i, v) in out2.iter().enumerate() {
        assert!((v - clip.samples()[2000 + i] * 0.5).abs() < 1e-6);
    }
}

/// Simulated device: 1 ms clock steps, callback every 10 ms with jitter, poll at 1 kHz.
fn run_clock(p: &AudioPlayer, fake: &FakeBackend, total_ms: u64, mut on_ms: impl FnMut(u64, TimeSpan)) {
    let rate = fake.format().sample_rate as usize;
    let block = rate / 100;
    let mut rng = Lcg(42);
    let mut next_cb: i64 = 0;
    for t in 0..total_ms {
        if t as i64 >= next_cb {
            fake.pump(block);
            let jitter = (rng.next_f32() * 3.0) as i64;
            next_cb = t as i64 + 10 + jitter;
        }
        fake.advance_clock(ms(1));
        on_ms(t, p.position());
    }
}

#[test]
fn clock_is_monotonic_smooth_and_latency_adjusted() {
    let clip = sine_clip(48000, 1, 30.0, 440.0, 0.5);
    let (p, fake) = setup(48000, 2, clip);
    fake.set_latency(ms(20));
    p.play();
    let mut prev = TimeSpan::ZERO;
    let mut max_step = 0.0f64;
    let mut worst_err = 0.0f64;
    // Callback jitter above makes periods 10..12 ms, so the device runs a little slow relative to the
    // wall clock. Use the *frames actually rendered* as the truth for the media position.
    run_clock(&p, &fake, 3000, |t, pos| {
        assert!(pos >= prev, "position went backwards at {t} ms: {prev:?} -> {pos:?}");
        if t > 200 {
            max_step = max_step.max(pos.total_ms() - prev.total_ms());
            let truth = p.frames_rendered() as f64 / 48.0 - 20.0; // ms, minus latency
            worst_err = worst_err.max((pos.total_ms() - truth).abs());
        }
        prev = pos;
    });
    println!("max 1ms-poll step {max_step:.3} ms, worst error vs rendered frames {worst_err:.2} ms");
    assert!(max_step < 4.0, "jittery clock: step of {max_step} ms");
    assert!(worst_err < 20.0, "clock off by {worst_err} ms");
    assert!(prev.total_ms() > 2500.0);
}

#[test]
fn polling_at_100hz_sees_no_backward_steps_under_bursty_callbacks() {
    let clip = sine_clip(44100, 2, 30.0, 220.0, 0.5);
    let (p, fake) = setup(48000, 2, clip); // also exercises resampling
    fake.set_latency(ms(5));
    p.play();
    let mut rng = Lcg(9);
    let mut prev = TimeSpan::ZERO;
    for tick in 0..1000 {
        fake.advance_clock(ms(10));
        // 0, 1 or 2 callbacks of varying size per tick, as a bursty driver would.
        let n = (rng.next_f32().abs() * 2.99) as usize;
        for _ in 0..n {
            fake.pump(100 + (rng.next_f32().abs() * 700.0) as usize);
        }
        let pos = p.position();
        assert!(pos >= prev, "tick {tick}: {prev:?} -> {pos:?}");
        prev = pos;
    }
}

#[test]
fn pause_seek_stop_and_speed_move_the_clock() {
    let clip = sine_clip(48000, 1, 60.0, 330.0, 0.5);
    let (p, fake) = setup(48000, 2, clip);
    p.play();
    run_clock(&p, &fake, 1000, |_, _| {});
    let t1 = p.position().total_secs();
    assert!((t1 - 1.0).abs() < 0.03, "after 1 s at 1x: {t1}");

    p.pause();
    assert!(!p.is_playing());
    assert_eq!(p.state(), PlayerState::Paused);
    run_clock(&p, &fake, 50, |_, _| {});
    let frozen = p.position();
    run_clock(&p, &fake, 500, |_, pos| assert_eq!(pos, frozen));
    assert!(fake.pump(480).iter().all(|&v| v == 0.0));

    // seek while paused shows immediately, with no callback
    p.seek(TimeSpan::from_secs(20.0));
    assert_eq!(p.position(), TimeSpan::from_secs(20.0));
    run_clock(&p, &fake, 100, |_, pos| assert_eq!(pos, TimeSpan::from_secs(20.0)));

    p.play();
    run_clock(&p, &fake, 1000, |_, _| {});
    let t2 = p.position().total_secs();
    assert!((t2 - 21.0).abs() < 0.03, "seek then play 1 s: {t2}");

    // seek backwards while playing
    p.seek(TimeSpan::from_secs(5.0));
    let mut prev = p.position();
    assert_eq!(prev, TimeSpan::from_secs(5.0));
    run_clock(&p, &fake, 500, |_, pos| {
        assert!(pos >= prev);
        prev = pos;
    });
    assert!((prev.total_secs() - 5.5).abs() < 0.03, "{prev:?}");

    // speed 2.0: media time runs twice as fast
    p.set_speed(2.0);
    run_clock(&p, &fake, 50, |_, _| {});
    let a = p.position().total_secs();
    run_clock(&p, &fake, 1000, |_, _| {});
    let b = p.position().total_secs();
    assert!((b - a - 2.0).abs() < 0.06, "2x speed advanced {}", b - a);

    // speed 0.5
    p.set_speed(0.5);
    run_clock(&p, &fake, 50, |_, _| {});
    let a = p.position().total_secs();
    run_clock(&p, &fake, 2000, |_, _| {});
    let b = p.position().total_secs();
    assert!((b - a - 1.0).abs() < 0.05, "0.5x speed advanced {}", b - a);

    p.set_speed(100.0);
    assert_eq!(p.speed(), 4.0);
    p.set_speed(0.0);
    assert_eq!(p.speed(), 0.1);

    p.stop();
    assert_eq!(p.position(), TimeSpan::ZERO);
    assert_eq!(p.state(), PlayerState::Stopped);
    run_clock(&p, &fake, 100, |_, pos| assert_eq!(pos, TimeSpan::ZERO));
}

#[test]
fn speed_back_to_one_is_exact_again_and_slow_output_is_stretched() {
    let clip = sine_clip(48000, 1, 20.0, 300.0, 0.5);
    let (p, fake) = setup(48000, 1, clip.clone());
    p.seek(TimeSpan::from_secs(2.0));
    p.set_speed(0.5);
    p.play();
    let n = 48000;
    let out = fake.pump(n); // 1 s of output = 0.5 s of media
    assert!(out.iter().any(|&v| v.abs() > 0.3));
    let pos = p.position().total_secs();
    assert!((pos - 2.0).abs() < 0.05);
    fake.pump(1);
    p.set_speed(1.0);
    fake.pump(1);
    let at = p.position(); // near 2.5 s
    assert!((at.total_secs() - 2.5).abs() < 0.1, "{at:?}");
    // Now bypassed: exact samples from the current position onward.
    let start = (frames_for(&p) as usize).min(clip.frames() - 5000);
    p.seek(TimeSpan::from_secs(start as f64 / 48000.0));
    let out = fake.pump(4000);
    assert_eq!(&out[..], &clip.samples()[start..start + 4000]);
}

fn frames_for(p: &AudioPlayer) -> u64 {
    (p.position().total_secs() * 48000.0).round() as u64
}

#[test]
fn rate_mismatch_plays_at_the_right_pace() {
    let clip = sine_clip(44100, 2, 10.0, 441.0, 0.5);
    let (p, fake) = setup(48000, 2, clip);
    p.play();
    let mut total = 0;
    for _ in 0..100 {
        fake.advance_clock(ms(10));
        total += fake.pump(480).len() / 2;
    }
    assert_eq!(total, 48000);
    let pos = p.position().total_secs();
    assert!((pos - 1.0).abs() < 0.02, "{pos}");
}

#[test]
fn errors_are_reported_not_panicked() {
    // Unknown device at construction.
    let fake = FakeBackend::new(48000, 2);
    let p = AudioPlayer::with_backend(Box::new(fake.clone()), Some("Nope"));
    assert!(matches!(p.state(), PlayerState::Error(AudioError::NoDevice(_))));
    p.play();
    p.pause();
    p.seek(TimeSpan::from_secs(3.0));
    p.stop();
    assert_eq!(p.position(), TimeSpan::ZERO);

    // Open fails, then play() retries and recovers.
    let fake = FakeBackend::new(48000, 2);
    let p = AudioPlayer::with_backend(Box::new(fake.clone()), None);
    fake.fail_next_open(AudioError::Device("boom".into()));
    p.set_clip(sine_clip(48000, 2, 5.0, 440.0, 0.5));
    assert!(matches!(p.state(), PlayerState::Error(_)));
    assert!(!p.is_playing());
    p.play();
    assert!(p.state() == PlayerState::Playing, "{:?}", p.state());
    assert!(fake.pump(100).iter().any(|&v| v != 0.0));

    // Device vanishes while playing.
    fake.trigger_error(AudioError::Device("unplugged".into()));
    assert!(matches!(p.state(), PlayerState::Error(AudioError::Device(_))));
    assert!(!p.is_playing());
    let pos = p.position();
    assert!(pos <= p.duration());
    // Recover: position kept.
    p.seek(TimeSpan::from_secs(2.0));
    p.play();
    assert_eq!(p.state(), PlayerState::Playing);
    assert_eq!(fake.open_count(), 3);
    fake.pump(480);
    assert!(p.position() >= TimeSpan::from_secs(2.0));

    // set_device to a missing device fails cleanly.
    p.set_device(Some("Missing"));
    assert!(matches!(p.state(), PlayerState::Error(AudioError::NoDevice(_))));
    p.set_device(None);
    assert!(!matches!(p.state(), PlayerState::Error(_)));
}

#[test]
fn replacing_the_clip_resets_everything() {
    let (p, fake) = setup(48000, 2, sine_clip(48000, 2, 5.0, 440.0, 0.5));
    p.play();
    run_clock(&p, &fake, 500, |_, _| {});
    assert!(p.position() > TimeSpan::ZERO);
    p.set_clip(sine_clip(44100, 1, 3.0, 100.0, 0.5));
    assert_eq!(p.position(), TimeSpan::ZERO);
    assert!(!p.is_playing());
    assert_eq!(p.duration(), TimeSpan::from_ticks(30_000_000));
}

#[test]
fn clock_handle_reads_from_another_thread() {
    let (p, fake) = setup(48000, 2, sine_clip(48000, 2, 5.0, 440.0, 0.5));
    p.play();
    let h = p.clock_handle();
    fake.advance_clock(ms(10));
    fake.pump(480);
    let t = std::thread::spawn(move || h.position());
    let _ = t.join().unwrap();
}


/// Measure actual output rather than checking the speed setting: rate changes must move pitch,
/// tempo changes must preserve it, including when the device and clip sample rates differ.
fn output_frequency(samples: &[f32], channels: usize, channel: usize, rate: u32) -> f64 {
    let frames = samples.len() / channels;
    let mut crossings = Vec::new();
    for frame in frames / 4 + 1..frames * 3 / 4 {
        let a = samples[(frame - 1) * channels + channel];
        let b = samples[frame * channels + channel];
        if a < 0.0 && b >= 0.0 {
            crossings.push(frame as f64 - 1.0 + (-a / (b - a)) as f64);
        }
    }
    assert!(crossings.len() > 2, "output has too few crossings");
    (crossings.len() - 1) as f64 * rate as f64 / (crossings.last().unwrap() - crossings[0])
}

#[test]
fn rate_and_tempo_have_the_requested_pitch_and_clock_through_four_times() {
    for clip_rate in [44100, 48000] {
        for preserve_pitch in [false, true] {
            for speed in [0.1f32, 0.5, 1.0, 2.1, 4.0] {
                let clip = sine_clip(clip_rate, 2, 8.0, 330.0, 0.5);
                let (p, fake) = setup(48000, 2, clip);
                p.set_rate_mode(preserve_pitch);
                p.set_speed(speed);
                p.play();
                let out = fake.pump(48000);
                let expected = 330.0 * if preserve_pitch { 1.0 } else { speed as f64 };
                for channel in 0..2 {
                    let frequency = output_frequency(&out, 2, channel, 48000);
                    assert!((frequency / expected - 1.0).abs() < 0.02,
                        "rate={clip_rate} tempo={preserve_pitch} speed={speed}: frequency={frequency}, expected={expected}");
                }
                fake.advance_clock(Duration::from_secs(1));
                assert!((p.position().total_secs() - speed as f64).abs() < 0.002);
            }
        }
    }
}

#[test]
fn switching_audio_mode_waits_for_the_next_speed_assignment_without_jumping() {
    let (p, fake) = setup(48000, 1, sine_clip(48000, 1, 20.0, 300.0, 0.5));
    p.set_rate_mode(false);
    p.set_speed(2.0);
    p.play();
    let rate = fake.pump(24000);
    fake.advance_clock(Duration::from_millis(500));
    let before = p.position();
    assert!((output_frequency(&rate, 1, 0, 48000) - 600.0).abs() < 2.0);
    p.set_rate_mode(true);
    let still_rate = fake.pump(24000);
    fake.advance_clock(Duration::from_millis(500));
    assert!((output_frequency(&still_rate, 1, 0, 48000) - 600.0).abs() < 2.0);
    p.set_speed(2.0);
    let tempo = fake.pump(24000);
    fake.advance_clock(Duration::from_millis(500));
    assert!((output_frequency(&tempo, 1, 0, 48000) - 300.0).abs() < 6.0);
    assert!((p.position().total_secs() - before.total_secs() - 2.0).abs() < 0.01);
    p.set_rate_mode(false);
    p.set_speed(4.0);
    let fast = fake.pump(24000);
    assert!((output_frequency(&fast, 1, 0, 48000) - 1200.0).abs() < 2.0);
}

#[test]
fn rate_playback_ends_at_the_clip_boundary_and_seeks_from_a_fractional_position() {
    let clip = noise_clip(48000, 1, 960, 52);
    let (p, fake) = setup(48000, 1, clip.clone());
    p.set_rate_mode(false);
    p.set_speed(2.5);
    p.play();
    let out = fake.pump(500);
    assert_eq!(out[0], clip.samples()[0]);
    assert!((out[1] - (clip.samples()[2] + clip.samples()[3]) / 2.0).abs() < 1e-6);
    assert!(out[384..].iter().all(|&v| v == 0.0));
    fake.advance_clock(ms(8));
    assert!(p.has_ended());
    assert_eq!(p.position(), clip.duration());
    p.seek(TimeSpan::from_ms_i(5));
    p.play();
    assert_eq!(fake.pump(1)[0], clip.samples()[240]);
}

#[test]
fn eof_waits_for_the_last_audible_sample_and_preserves_the_latency_clock() {
    let (p, fake) = setup(48000, 1, noise_clip(48000, 1, 960, 71));
    fake.set_latency(ms(100));
    p.play();
    fake.pump(1440); // all 20ms of audio queued, plus silence
    assert!(!p.has_ended());
    assert!(p.is_playing());
    assert_eq!(p.position(), TimeSpan::ZERO);
    fake.advance_clock(ms(110));
    fake.pump(480); // another silent callback must not publish the end early
    assert!(!p.has_ended());
    assert!((p.position().total_ms() - 10.0).abs() < 0.05);
    fake.advance_clock(ms(10));
    assert!(p.has_ended());
    assert!(!p.is_playing());
    assert_eq!(p.position(), TimeSpan::from_ms_i(20));
    assert!(!p.clock_handle().is_playing());
    assert_eq!(p.state(), PlayerState::Stopped);
    p.seek(TimeSpan::from_ms_i(5));
    p.play();
    assert!(!p.has_ended());
    assert!(fake.pump(480).iter().any(|sample| *sample != 0.0));
}
