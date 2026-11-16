fn main() {
    println!("cargo:rerun-if-changed=native/launch_helper.m");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let out =
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("vixen-launch-helper");
    let arch = match std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("aarch64") => "arm64",
        Ok("x86_64") => "x86_64",
        _ => panic!("unsupported macOS architecture"),
    };
    let status = std::process::Command::new("/usr/bin/clang")
        .args([
            "-fobjc-arc",
            "-fblocks",
            "-O2",
            "-Wall",
            "-Wextra",
            "-arch",
            arch,
            "-mmacosx-version-min=26.0",
            "-framework",
            "Foundation",
            "-framework",
            "AppKit",
            "native/launch_helper.m",
            "-o",
        ])
        .arg(&out)
        .status()
        .expect("compile native launch helper");
    assert!(status.success(), "native launch helper compilation failed");
    println!("cargo:rustc-env=VIXEN_LAUNCH_HELPER={}", out.display());
}
