use super::*;

#[test]
fn source_duration_matches_ffmpeg_dump_hundredths_including_half_up_carry() {
    // FFmpeg9.0.2 dump.c adds5000 integer microseconds, then prints centiseconds.
    for (source, expected) in [(1.0049, 1.0), (1.005, 1.01), (1.015, 1.02), (59.995, 60.0), (3.0, 3.0)] {
        assert_eq!(source_duration_seconds(source), expected);
    }
}

#[test]
fn successful_native_output_never_calls_fallback_or_resets() {
    let frame = Arc::new(Frame {
        width: 1,
        height: 1,
        rgba: vec![1, 2, 3, 255],
    });
    let mut indices = Vec::new();
    let result = decode_with_fallback(
        &AtomicBool::new(false),
        |event| match event {
            FrameEvent::Frame(index, _) => {
                indices.push(index);
                true
            }
            FrameEvent::Reset => panic!("successful native decode must not reset"),
        },
        |sink| {
            assert!(sink(0, &frame));
            Some(1)
        },
        |_| panic!("successful native decode must not retry"),
    );
    assert_eq!(result, Some(1));
    assert_eq!(indices, [0]);
}

#[test]
fn partial_native_failure_resets_before_fallback_and_never_mixes_frames() {
    let old = Arc::new(Frame {
        width: 1,
        height: 1,
        rgba: vec![1; 4],
    });
    let new = Arc::new(Frame {
        width: 1,
        height: 1,
        rgba: vec![2; 4],
    });
    let mut frames = Vec::new();
    let mut resets = 0;
    let result = decode_with_fallback(
        &AtomicBool::new(false),
        |event| {
            match event {
                FrameEvent::Reset => {
                    frames.clear();
                    resets += 1;
                }
                FrameEvent::Frame(index, frame) => {
                    assert_eq!(index, frames.len());
                    frames.push(frame.clone());
                }
            }
            true
        },
        |sink| {
            assert!(sink(0, &old));
            assert!(sink(1, &old));
            None
        },
        |sink| {
            assert!(sink(0, &new));
            Some(1)
        },
    );
    assert_eq!(result, Some(1));
    assert_eq!(resets, 1);
    assert_eq!(frames, [new]);
}

#[test]
fn abort_cancel_and_unresettable_partial_output_never_start_a_fallback() {
    let frame = Arc::new(Frame {
        width: 1,
        height: 1,
        rgba: vec![1; 4],
    });
    for mode in 0..3 {
        let cancel = AtomicBool::new(false);
        let result = decode_with_fallback(
            &cancel,
            |event| match event {
                FrameEvent::Reset => false,
                FrameEvent::Frame(_, _) => mode != 0,
            },
            |sink| {
                sink(0, &frame);
                if mode == 1 {
                    cancel.store(true, Ordering::Relaxed);
                }
                None
            },
            |_| panic!("abort, cancellation, or rejected reset must not retry"),
        );
        assert_eq!(result, None);
    }
    let result = decode_with_fallback(
        &AtomicBool::new(true),
        |_| panic!("cancelled"),
        |_| panic!("cancelled"),
        |_| panic!("cancelled"),
    );
    assert_eq!(result, None);
}

#[test]
fn native_open_failure_retries_without_requesting_a_reset() {
    let frame = Arc::new(Frame {
        width: 1,
        height: 1,
        rgba: vec![1; 4],
    });
    let result = decode_with_fallback(
        &AtomicBool::new(false),
        |event| match event {
            FrameEvent::Reset => panic!("no accepted frames to reset"),
            FrameEvent::Frame(index, _) => index == 0,
        },
        |_| None,
        |sink| {
            assert!(sink(0, &frame));
            Some(1)
        },
    );
    assert_eq!(result, Some(1));
}

#[test]
fn supported_native_decode_is_byte_identical_through_the_fallback_entrypoint() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/assets/colors.mov");
    let request = VideoRequest {
        start_seconds: 0.5,
        duration_seconds: 1.0,
        fps: 20.0,
        width: 16,
        height: 10,
    };
    let cancel = AtomicBool::new(false);
    let mut native = Vec::new();
    native_frames_to(&path, &request, &cancel, |_, frame| {
        native.push(frame.clone());
        true
    })
    .unwrap();
    assert_eq!(video_frames(&path, &request).unwrap(), native);
}
