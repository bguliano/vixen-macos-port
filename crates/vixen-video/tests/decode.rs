//! Decodes the clips `tests/make_assets.swift` wrote.

use std::path::PathBuf;
use vixen_video::{video_frames, video_info, Frame, VideoInfo, VideoRequest};

fn asset(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/assets")
        .join(name)
}

fn pixel(frame: &Frame, x: i32, y: i32) -> (u8, u8, u8) {
    let i = ((y * frame.width + x) * 4) as usize;
    (frame.rgba[i], frame.rgba[i + 1], frame.rgba[i + 2])
}

/// Video compression leaves solid colours slightly off.
fn near(a: (u8, u8, u8), b: (u8, u8, u8)) -> bool {
    a.0.abs_diff(b.0) < 40 && a.1.abs_diff(b.1) < 40 && a.2.abs_diff(b.2) < 40
}

const RED: (u8, u8, u8) = (255, 0, 0);
const GREEN: (u8, u8, u8) = (0, 255, 0);
const BLUE: (u8, u8, u8) = (0, 0, 255);

#[test]
fn reports_size_and_length() {
    let info = video_info(&asset("colors.mov")).unwrap();
    assert_eq!((info.width, info.height), (160, 90));
    assert!((info.duration_seconds - 3.0).abs() < 0.05, "{info:?}");
}

#[test]
fn a_clip_that_displays_turned_reports_its_turned_size() {
    let VideoInfo { width, height, .. } = video_info(&asset("quarter_turn.mov")).unwrap();
    assert_eq!((width, height), (90, 160));
    let source = vixen_video::video_source_info_cancelable(&asset("quarter_turn.mov"), &std::sync::atomic::AtomicBool::new(false)).unwrap();
    assert_eq!((source.width, source.height), (160, 90));
    assert_eq!(source.duration_seconds, 1.0);
}

#[test]
fn a_missing_or_unreadable_file_gives_nothing() {
    assert!(video_info(&asset("nope.mov")).is_none());
    let request = VideoRequest {
        start_seconds: 0.0,
        duration_seconds: 1.0,
        fps: 20.0,
        width: 16,
        height: 9,
    };
    assert!(video_frames(&asset("nope.mov"), &request).is_none());
    assert!(video_info(&asset("../make_assets.swift")).is_none());
}

#[test]
fn frames_come_at_the_requested_rate_and_size() {
    let request = VideoRequest {
        start_seconds: 0.0,
        duration_seconds: 3.0,
        fps: 20.0,
        width: 16,
        height: 10,
    };
    let frames = video_frames(&asset("colors.mov"), &request).unwrap();
    assert_eq!(frames.len(), 60);
    assert_eq!((frames[0].width, frames[0].height), (16, 10));
    assert_eq!(frames[0].rgba.len(), 16 * 10 * 4);
    for (index, color) in [
        (1, RED),
        (17, RED),
        (22, GREEN),
        (37, GREEN),
        (42, BLUE),
        (58, BLUE),
    ] {
        assert!(
            near(pixel(&frames[index], 8, 5), color),
            "frame {index}: {:?}",
            pixel(&frames[index], 8, 5)
        );
    }
}

#[test]
fn a_rate_above_the_sources_repeats_frames_without_copying_them() {
    let request = VideoRequest {
        start_seconds: 0.0,
        duration_seconds: 1.0,
        fps: 100.0,
        width: 8,
        height: 4,
    };
    let frames = video_frames(&asset("colors.mov"), &request).unwrap();
    assert_eq!(frames.len(), 100);
    let distinct = frames
        .windows(2)
        .filter(|w| !std::sync::Arc::ptr_eq(&w[0], &w[1]))
        .count()
        + 1;
    assert!(
        (28..=32).contains(&distinct),
        "a 30 fps source gives about 30 different pictures, got {distinct}"
    );
}

#[test]
fn the_start_time_skips_into_the_video() {
    let request = VideoRequest {
        start_seconds: 1.0,
        duration_seconds: 1.0,
        fps: 10.0,
        width: 16,
        height: 10,
    };
    let frames = video_frames(&asset("colors.mov"), &request).unwrap();
    assert_eq!(frames.len(), 10);
    for (index, frame) in frames.iter().enumerate().skip(1).take(8) {
        assert!(
            near(pixel(frame, 8, 5), GREEN),
            "frame {index}: {:?}",
            pixel(frame, 8, 5)
        );
    }
}

#[test]
fn past_the_end_stops_early() {
    let request = VideoRequest {
        start_seconds: 2.5,
        duration_seconds: 2.0,
        fps: 10.0,
        width: 16,
        height: 10,
    };
    let frames = video_frames(&asset("colors.mov"), &request).unwrap();
    assert!((4..=7).contains(&frames.len()), "got {}", frames.len());
}

#[test]
fn a_clip_tagged_with_a_quarter_turn_comes_out_turned() {
    let request = VideoRequest {
        start_seconds: 0.0,
        duration_seconds: 0.5,
        fps: 10.0,
        width: 9,
        height: 16,
    };
    let frames = video_frames(&asset("quarter_turn.mov"), &request).unwrap();
    let f = &frames[2];
    assert_eq!((f.width, f.height), (9, 16));
    // Red on the left as stored is on top once turned a quarter clockwise, the same way AVFoundation's players show it.
    assert!(near(pixel(f, 4, 2), RED), "top {:?}", pixel(f, 4, 2));
    assert!(near(pixel(f, 4, 13), BLUE), "bottom {:?}", pixel(f, 4, 13));
}

#[test]
fn mpeg4_part2_avi_fallback_decodes_at_requested_size_rate_and_start() {
    // AVAssetReader reports metadata for this valid AVI but cannot decode its
    // MPEG-4 Part2 video on the audited macOS. The bundled software decoder can.
    let path = asset("colors.avi");
    let info = video_info(&path).unwrap();
    assert_eq!((info.width, info.height), (160, 90));
    let request = VideoRequest {
        start_seconds: 0.0,
        duration_seconds: 3.0,
        fps: 20.0,
        width: 16,
        height: 10,
    };
    let frames = video_frames(&path, &request).unwrap();
    assert_eq!(frames.len(), 60);
    for (index, color) in [
        (1, RED),
        (17, RED),
        (22, GREEN),
        (37, GREEN),
        (42, BLUE),
        (58, BLUE),
    ] {
        assert!(
            near(pixel(&frames[index], 8, 5), color),
            "frame{index}: {:?}",
            pixel(&frames[index], 8, 5)
        );
    }
    let frames = video_frames(
        &path,
        &VideoRequest {
            start_seconds: 1.0,
            duration_seconds: 1.0,
            fps: 10.0,
            ..request
        },
    )
    .unwrap();
    assert_eq!(frames.len(), 10);
    assert!(frames
        .iter()
        .skip(1)
        .take(8)
        .all(|frame| near(pixel(frame, 8, 5), GREEN)));
    let frames = video_frames(
        &path,
        &VideoRequest {
            start_seconds: 2.5,
            duration_seconds: 2.0,
            fps: 10.0,
            ..request
        },
    )
    .unwrap();
    assert!((4..=7).contains(&frames.len()), "{}", frames.len());
}

#[test]
fn software_fallback_honors_cancellation_and_consumer_abort_between_frames() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use vixen_video::video_frames_to;
    let path = asset("colors.avi");
    let request = VideoRequest {
        start_seconds: 0.0,
        duration_seconds: 2.0,
        fps: 1000.0,
        width: 128,
        height: 64,
    };
    for cancelled in [false, true] {
        let cancel = AtomicBool::new(false);
        let mut emitted = 0;
        let started = std::time::Instant::now();
        let result = video_frames_to(&path, &request, &cancel, |index, _| {
            assert_eq!(index, emitted);
            emitted += 1;
            if emitted == 3 {
                if cancelled {
                    cancel.store(true, Ordering::Relaxed);
                }
                return cancelled;
            }
            true
        });
        assert_eq!(result, None);
        assert_eq!(emitted, 3);
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
    }
    assert_eq!(
        video_frames(
            &path,
            &VideoRequest {
                duration_seconds: 1.0,
                fps: 10.0,
                ..request
            }
        )
        .unwrap()
        .len(),
        10
    );
}

#[test]
fn streaming_matches_vector_frames_indices_rotation_and_early_eof() {
    use std::sync::{atomic::AtomicBool, Arc};
    use vixen_video::video_frames_to;
    let cases = [
        (
            "colors.mov",
            VideoRequest {
                start_seconds: 0.0,
                duration_seconds: 1.0,
                fps: 100.0,
                width: 8,
                height: 4,
            },
        ),
        (
            "colors.mov",
            VideoRequest {
                start_seconds: 1.0,
                duration_seconds: 1.0,
                fps: 10.0,
                width: 16,
                height: 10,
            },
        ),
        (
            "colors.mov",
            VideoRequest {
                start_seconds: 2.5,
                duration_seconds: 2.0,
                fps: 10.0,
                width: 16,
                height: 10,
            },
        ),
        (
            "quarter_turn.mov",
            VideoRequest {
                start_seconds: 0.0,
                duration_seconds: 0.5,
                fps: 10.0,
                width: 9,
                height: 16,
            },
        ),
    ];
    for (name, request) in cases {
        let expected = video_frames(&asset(name), &request).unwrap();
        let mut streamed = Vec::new();
        let count = video_frames_to(
            &asset(name),
            &request,
            &AtomicBool::new(false),
            |index, frame| {
                assert_eq!(index, streamed.len());
                streamed.push(Arc::clone(frame));
                true
            },
        )
        .unwrap();
        assert_eq!(count, expected.len());
        assert_eq!(streamed, expected, "{name} {request:?}");
        let repeats = |frames: &[Arc<Frame>]| {
            frames
                .windows(2)
                .map(|pair| Arc::ptr_eq(&pair[0], &pair[1]))
                .collect::<Vec<_>>()
        };
        assert_eq!(repeats(&streamed), repeats(&expected));
    }
}

#[test]
fn streaming_keeps_only_current_frame_and_has_no_whole_clip_allocation_limit() {
    use std::sync::{atomic::AtomicBool, Arc, Weak};
    use vixen_video::video_frames_to;
    // 2,000 requested pictures would occupy2GiB if retained individually. The
    // production sink may write them one at a time; a short abort keeps this test small.
    let request = VideoRequest {
        start_seconds: 0.0,
        duration_seconds: 2.0,
        fps: 1000.0,
        width: 512,
        height: 512,
    };
    assert!(video_frames(&asset("colors.mov"), &request).is_none());
    let mut emitted = 0;
    let mut last: Weak<Frame> = Weak::new();
    let result = video_frames_to(
        &asset("colors.mov"),
        &request,
        &AtomicBool::new(false),
        |index, frame| {
            assert_eq!(index, emitted);
            assert_eq!(
                Arc::strong_count(frame),
                1,
                "streaming decoder retained output history"
            );
            last = Arc::downgrade(frame);
            emitted += 1;
            emitted < 3
        },
    );
    assert!(result.is_none());
    assert_eq!(emitted, 3);
    assert!(
        last.upgrade().is_none(),
        "aborted decoder retained its current frame"
    );
}

#[test]
fn streaming_observes_cancel_between_repeated_output_samples_and_can_restart() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use vixen_video::video_frames_to;
    let request = VideoRequest {
        start_seconds: 0.0,
        duration_seconds: 1.0,
        fps: 1000.0,
        width: 16,
        height: 10,
    };
    let cancel = AtomicBool::new(true);
    let mut emitted = 0;
    assert!(
        video_frames_to(&asset("colors.mov"), &request, &cancel, |_, _| {
            emitted += 1;
            true
        })
        .is_none()
    );
    assert_eq!(emitted, 0);
    cancel.store(false, Ordering::Relaxed);
    assert!(
        video_frames_to(&asset("colors.mov"), &request, &cancel, |_, _| {
            emitted += 1;
            if emitted == 3 {
                cancel.store(true, Ordering::Relaxed);
            }
            true
        })
        .is_none()
    );
    assert_eq!(emitted, 3);
    cancel.store(false, Ordering::Relaxed);
    emitted = 0;
    assert_eq!(
        video_frames_to(&asset("colors.mov"), &request, &cancel, |_, _| {
            emitted += 1;
            true
        }),
        Some(1000)
    );
    assert_eq!(emitted, 1000);
}

#[test]
fn malformed_stream_requests_do_not_call_the_sink_or_allocate_frame_sets() {
    use std::sync::atomic::AtomicBool;
    use vixen_video::video_frames_to;
    let valid = VideoRequest {
        start_seconds: 0.0,
        duration_seconds: 1.0,
        fps: 20.0,
        width: 16,
        height: 10,
    };
    for request in [
        VideoRequest {
            start_seconds: -1.0,
            ..valid
        },
        VideoRequest {
            start_seconds: f64::INFINITY,
            ..valid
        },
        VideoRequest {
            duration_seconds: 0.0,
            ..valid
        },
        VideoRequest {
            duration_seconds: f64::NAN,
            ..valid
        },
        VideoRequest {
            fps: f64::MAX,
            ..valid
        },
        VideoRequest {
            fps: 250_001.0,
            ..valid
        },
        VideoRequest {
            width: 4096,
            height: 4097,
            ..valid
        },
        VideoRequest {
            width: i32::MAX,
            height: i32::MAX,
            ..valid
        },
    ] {
        assert!(video_frames_to(
            &asset("colors.mov"),
            &request,
            &AtomicBool::new(false),
            |_, _| panic!("invalid request reached sink")
        )
        .is_none());
    }
}
