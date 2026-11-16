# Vixen for macOS

A work-in-progress native macOS port of [Vixen Lights](https://github.com/VixenLights/Vixen), with a Rust engine and SwiftUI/AppKit interface. This is an independent port, not an official Vixen release.

The current priority is reliable everyday use on the maintainer's Mac: opening, editing, saving and reopening shows, preview, playback, and export. Crashes, lost changes, broken workflows, and noticeable performance or synchronization issues take priority over exact Windows pixel or audio-sample parity. The port is not yet declared fully verified or ready for public release.

## Upstream and repository layout

This public fork preserves the official Vixen commit history. The `macos-port` branch contains the native macOS port; `master` retains the original Windows source. The original Vixen source and its history are retained through the `upstream/Vixen` Git submodule, pinned to commit `e111ae41963abfa48b70f884445950e94e432e02`. Initialize it when cloning:

```sh
git clone --branch macos-port --recurse-submodules https://github.com/bguliano/vixen-macos-port.git
cd vixen-macos-port
```

- `app/`: native Swift interface and application resources.
- `crates/`: Rust engine, effects, media, file formats, and app bindings.
- `tests/` and `tools/`: reference fixtures and verification tools.
- `scripts/`: app builds and workflow checks.
- `docs/`: implementation specifications, differences, and continuation notes.

Local toolchains, app bundles, raw verification captures, screenshots, credentials, personal show/media directories, machine-access scripts and private session handoffs are not uploaded. Some development notes refer to local evidence under `target/`; those references describe historical checks and are not reproducible evidence included in this repository.

## Build

The current build targets Apple Silicon and macOS 26 or later. It requires Apple's development tools with Swift 6.2 or later, Python 3, and the Rust version pinned in `rust-toolchain.toml`. Intel support and notarized distribution are not claimed.

The build scripts use project-local Rust directories. After installing rustup through its official installer, initialize those directories in a Bash shell:

```sh
bash
export RUSTUP_HOME="$PWD/.tools/rustup"
export CARGO_HOME="$PWD/.tools/cargo"
rustup toolchain install 1.99.1 --profile minimal --component clippy --component rustfmt
mkdir -p "$CARGO_HOME/bin"
ln -s "$(command -v rustup)" "$CARGO_HOME/bin/rustup"
ln -s rustup "$CARGO_HOME/bin/cargo"
ln -s rustup "$CARGO_HOME/bin/rustc"
source ./env.sh
cargo fetch --locked
scripts/build_media_tools.sh --fetch
scripts/build_app.sh release
```

The initial fetches download dependencies and the checksum-pinned FFmpeg source. The app is written to `build/Vixen.app`. Build scripts generate Swift bindings automatically; do not run `swift build` directly in `app/`. Existing development machines with `.tools/` already initialized can start with `source ./env.sh`.

## Verification and status

```sh
# In Bash, with the project-local toolchain initialized:
source ./env.sh
cargo test --workspace --locked
```

Some integration checks require native macOS services, media helpers, foreground desktop access, or external hardware. Read the relevant script before running it. Always test with a private copy of a show.

The latest recorded workspace suite passed 3,549 tests, with zero failures and 10 ignored tests. This records a local checkpoint, not proof that all user workflows or hardware configurations work. See [current status](docs/STATUS.md), [known differences](docs/DIFFERENCES.md), and [UI guide](docs/UI_GUIDE.md).

Historical test helpers may refer to example paths under `/Users/developer`; configure those for your own private fixtures before using them. Private-fixture tests can skip when those files are absent, so a green test count alone does not establish real-show coverage. The public snapshot normalizes local paths and omits private orchestration records; the recorded local test results predate that packaging cleanup.

## Licenses and attribution

Vixen's copyright and license are retained in [License.txt](License.txt) and the upstream submodule. This project also includes components under other licenses, including GPL-2.0-or-later analysis code. The upstream license alone does not describe every component or the combined app's distribution requirements. Preserve all notices and review [the component license inventory](docs/LICENSES.md) before distributing builds or making a public release.
