# Portable macOS verification pack

This pack contains the arm64 Vixen app, private controller-free fixtures, local media assets and verification scripts. Its exact release hash and existing evidence are recorded in `manifest.json`. A successful run verifies only the OS and workflows recorded in its result; it does not certify the whole port.

## Requirements

Use an Apple-silicon Mac with a logged-in graphical session, Python 3.11 or newer, and Apple's `codesign`, `lipo` and `otool` tools. No Rust toolchain, third-party Python packages, Swift compilation, downloads or paid services are required on the destination. The screenshot OCR helper is already compiled and signed for macOS 26.

Keep the extracted pack unchanged and preserve executable permissions. The runner checks every packaged file hash before launch and afterward. Handle normal macOS trust or permission decisions yourself. The app is ad-hoc signed; this pack does not bypass Gatekeeper or establish Developer ID/notarization support.

## Run the checks

On an actual macOS 26 installation, replace the example pack path with your extracted folder:

```sh
python3 '/path/to/pack/scripts/run_second_mac_pack.py' \
  --require-macos26 --out '/private/tmp/vixen26-results-new'
```

The output folder must be new and outside the pack. This mode refuses an unsupported architecture or a different macOS version before copying or launching the app.

For a smoke test on another supported macOS version:

```sh
python3 '/path/to/pack/scripts/run_second_mac_pack.py' \
  --host-smoke --out '/private/tmp/vixen-host-results-new'
```

Host smoke always reports `minimumOSVerified=false`, including when run on macOS 26. Wait for the final success or error message; the runner prints each workflow as it starts. Each workflow has a 180-second timeout. Preserve the entire results folder if a check fails.

The results include `result.json`, three workflow logs, screenshots and private test data. `result.json` records the actual OS/build, architecture, Python version, canonical/private app hashes and passed workflows. Return the results folder, preferably compressed as a ZIP.

## What runs

The three workflows cover 164 native harness steps:

- Relocated startup, resources and legacy WMA waveform: 26 steps. A separate native-launch-helper check verifies its private process handshake, Stop and descendant cleanup.
- Two-editor playback ownership, pause and resume: 47 steps.
- Video selection, MOV/AVI rendering, metadata, Save/reopen and retained cache: 91 steps.

The startup 26 steps are included in 164, not added again. These are muted offscreen app workflows, not physical keyboard/mouse tests.

The runner copies the app into its results folder and gives that copy a unique Cocoa preference identifier. It verifies that the identifier/signature changes leave executable code/data identical. Every Vixen launch uses private Rust settings, a disposable profile and muted output. The web listener and external update transport are suppressed. No original show, audio output, controller or external device is used. Media/cache paths are resolved on the destination.

These checks do not establish physical gestures, audible synchronization, real controllers, Finder launch, Documents permission prompts, clean-user behavior, distribution trust, Windows file interoperability or any untested OS version. The source checkout tracks these separately in `docs/REMAINING_VERIFICATION.md`.

## Maintainer staging

From the source checkout, after accepting the current release checkpoint:

```sh
python3 scripts/prepare_second_mac_pack.py --out target/second-mac-pack
```

Staging verifies the release against `target/completion-checkpoint.json`, uses the existing `video_cache_fixture` generator, and compiles only the standalone OCR helper. Its signature, arm64 architecture, minimum macOS 26 deployment target and system dependencies are checked. Use a new staging directory. Historical release evidence remains in the source project's progress and acceptance records; it is not attributed to a new executable.
