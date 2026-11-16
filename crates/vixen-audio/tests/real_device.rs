//! Touches the real audio system. Ignored by default; never run in normal test runs.
#[test]
#[ignore]
fn lists_real_output_devices() {
    println!("{:?}", vixen_audio::output_devices());
}
