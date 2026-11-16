//! Plays a file through the real audio device at zero volume, and compares the device's clock with wall time.
use std::sync::Arc;
use std::time::{Duration, Instant};
use vixen_audio::{AudioClip, AudioPlayer};
use vixen_core::TimeSpan;

fn main() {
    let path = std::env::args().nth(1).expect("audio file");
    let t = Instant::now();
    let clip = Arc::new(AudioClip::load(std::path::Path::new(&path)).expect("decode"));
    println!("decoded {} in {:.0} ms: {} Hz, {} channels, {:.1} s", path, t.elapsed().as_secs_f64() * 1000.0, clip.sample_rate(), clip.channels(), clip.duration().total_ms() / 1000.0);
    let player = AudioPlayer::new(None);
    player.set_clip(clip);
    // Silence: the callback still runs and the clock still advances.
    player.set_volume(0.0);
    println!("devices: {:?}", vixen_audio::output_devices().iter().map(|d| d.to_string()).collect::<Vec<_>>());
    player.seek(TimeSpan::from_ms_i(30_000));
    let start_pos = player.position();
    player.play();
    let start = Instant::now();
    let mut worst = 0f64;
    let mut last = player.position();
    let mut stalls = 0;
    for i in 0..30 {
        std::thread::sleep(Duration::from_millis(100));
        let p = player.position();
        let wall = start.elapsed().as_secs_f64() * 1000.0;
        let audio = p.total_ms() - start_pos.total_ms();
        worst = worst.max((audio - wall).abs());
        if p == last {
            stalls += 1;
        }
        last = p;
        if i % 10 == 9 {
            println!("t={wall:.0} ms  audio advanced {audio:.0} ms  diff {:.1} ms  state {:?}", audio - wall, player.state());
        }
    }
    println!("worst drift {worst:.1} ms, stalled samples {stalls}, frames rendered {}, format {:?}, error {:?}", player.frames_rendered(), player.stream_format(), player.last_error());
    player.seek(TimeSpan::from_ms_i(60_000));
    std::thread::sleep(Duration::from_millis(200));
    println!("after seek to 60 s: position {:.0} ms", player.position().total_ms());
    player.pause();
    let a = player.position();
    std::thread::sleep(Duration::from_millis(200));
    println!("paused: moved {:.1} ms", player.position().total_ms() - a.total_ms());
    player.stop();
    println!("stopped at {:.0} ms", player.position().total_ms());
}
