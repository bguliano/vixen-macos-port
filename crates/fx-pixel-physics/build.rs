//! Compiles LiquidFun (`vendor/Box2D`, copied from upstream's `Vixen.Common/Box2D`) and the extern "C" shim
//! that stands in for the C++/CLI `LiquidFunWrapper` (`shim/liquid_shim.cpp`).
//!
//! The files and defines follow the Release configuration of upstream's `Box2D.vcxproj`. It builds everything
//! except `b2ParticleAssembly.cpp`, which is NEON only and was not copied. `NDEBUG` and `DEBUG=0` compile
//! `b2Assert` out, as a failed assert would abort the process and take a running show with it.

use std::path::{Path, PathBuf};

fn cpp_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir).expect("vendor directory").map(|e| e.expect("entry").path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            cpp_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "cpp") {
            out.push(path);
        }
    }
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=vendor");
    println!("cargo:rerun-if-changed=shim");

    let mut files = Vec::new();
    cpp_files(Path::new("vendor/Box2D"), &mut files);
    assert!(files.len() > 50, "the LiquidFun sources are missing from vendor/Box2D");

    cc::Build::new()
        .cpp(true)
        .std("c++11")
        .include("vendor")
        .files(&files)
        .file("shim/liquid_shim.cpp")
        .define("NDEBUG", None)
        .define("DEBUG", "0")
        .define("LIQUIDFUN_EXTERNAL_LANGUAGE_API", "1")
        // Physics stays usable in debug builds. Tests step thousands of particles.
        .opt_level(2)
        // MSVC on x64 evaluates float32 strictly and never fuses a multiply and an add. Clang would on
        // arm64, which moves every particle slightly, so fusing is turned off to keep the same arithmetic.
        .flag("-ffp-contract=off")
        .warnings(false)
        .compile("liquidfun");
}
