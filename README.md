# Vixen for macOS: unofficial port (alpha)

> **This is an unofficial, community-made port by [bguliano](https://github.com/bguliano).**
> It is not made, endorsed or supported by the Vixen Lights team, and [Vixen Lights](https://github.com/VixenLights/Vixen) has no official macOS version. Please report problems with this port [here](https://github.com/bguliano/vixen-macos-port/issues), not to the Vixen team.

A native macOS version of the Vixen 3 light sequencer, with a Rust engine and a SwiftUI/AppKit interface. It reads and writes the same show files as Vixen 3.13 for Windows.

## Alpha status: please read first

This port is an **alpha**. It has **not been tested extensively**: almost all of the testing so far happened on one Mac, with one real show. Expect bugs, missing pieces and differences from Windows Vixen.

- **Back up your show folder before opening it with this app**, and preferably work on a copy.
- It runs only on **Apple Silicon Macs with macOS 26 or later**. Intel Macs aren't supported.
- See [current status](docs/STATUS.md) and [known differences from Windows Vixen](docs/DIFFERENCES.md).

## Download

Download the latest `.dmg` from [Releases](https://github.com/bguliano/vixen-macos-port/releases), open it, and drag **Vixen** onto the Applications shortcut. A `.zip` of the same app is there too. Release builds are signed and notarized by Apple, so they open normally.

## Build from source

You need:

1. **Xcode** 26 or later (free, from the App Store). Open it once to finish installing its components.
2. **Rust**, installed with the official installer from [rustup.rs](https://rustup.rs):

   ```sh
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

   Then open a new Terminal window. You don't need to choose a Rust version: the repository pins one, and rustup downloads it automatically on the first build.

Then download the source and build:

```sh
git clone --depth 1 --branch macos-port --recurse-submodules --shallow-submodules https://github.com/bguliano/vixen-macos-port.git
cd vixen-macos-port
scripts/build_app.sh release
```

The first build downloads the Rust dependencies and a checksum-verified FFmpeg source, then compiles everything. Expect it to take 5–15 minutes. The app is written to `build/Vixen.app`; you can open it with `open build/Vixen.app` or copy it to Applications. Don't run `swift build` directly in `app/`: the build script generates the Swift bindings first.

The `--depth 1` and `--shallow-submodules` options skip Vixen's full 15-year history, which otherwise makes the download several times larger. Leave them out if you want that history.

### Running the tests

Build the app once first (the audio tests use the FFmpeg it builds), then:

```sh
source ./env.sh
cargo test --workspace --locked
```

Some extra scripts in `scripts/` check the app's interface by driving it on screen. They need macOS accessibility permissions; read a script before running it.

## Repository layout

- `macos-port` (default branch): this port. `master` is an unchanged copy of the official Windows source.
- `upstream/Vixen`: a Git submodule pinned to the official Vixen commit this port follows.
- `app/`: the native Swift interface and app resources.
- `crates/`: the Rust engine, effects, media, file formats and app bindings.
- `tests/` and `tools/`: reference fixtures and the tools that generated them.
- `scripts/`: the app build and interface checks.
- `docs/`: specifications, known differences and development notes. Some notes refer to local evidence under `target/` that isn't in this repository.

## Credits

- **Vixen Lights** — the original Windows application, its show format and its effects — is the work of
  [the Vixen team](https://github.com/VixenLights/Vixen) and its contributors since 2011. Their commit history is kept
  in this repository, which is why they appear in its contributor list. They aren't involved in this edition and
  aren't responsible for it.
- **The macOS port** — the Rust engine and native Mac app that replace the Windows/.NET code — is by
  [Braden Guliano](https://github.com/bguliano).

## Licenses and attribution

Vixen is created by the Vixen team; its copyright and license are retained in [License.txt](License.txt) and the upstream submodule. This port also includes components under other licenses, including GPL-2.0-or-later analysis code, so the upstream license alone doesn't describe the combined app. Review [the component license inventory](docs/LICENSES.md) before redistributing builds.
