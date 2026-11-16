# Test preferences and clean-user gates

The existing harness isolates profile data only when its caller supplies private
`VIXEN_DATA`. Rust global settings/layouts use `VIXEN_UI_TEST_OUT` when both
`VIXEN_UI_TEST` and `VIXEN_UI_TEST_OUT` exist. This does not isolate Cocoa's
application preferences: all ordinary built/copied bundles have the same
`com.vixenlights.vixen3.mac` identifier, and current native runners inherit the
owner's home rather than selecting a separate preferences domain.

A read-only snapshot of that domain is retained at
`target/preference-isolation-review/current-defaults-observation.json`. It contains
explicit test-scene window keys, native color/file-panel preferences, and negative
frame origins (sequencer `-1400,-768`, dialogs-test `-940,-651`). The snapshot proves
persisted shared state exists; it does not attribute every value to a particular
run, or prove an owner launch will open offscreen. SwiftUI restoration is disabled
for the main scenes, and OS frame clamping may alter a saved location. No current
owner default was cleared, replaced, imported or rewritten in this audit.

The historical blanket `defaults delete com.vixenlights.vixen3.mac` instruction in
RESUME should not be treated as a required cleanup action. It also discards native
panel/color preferences and all other values in that domain. Its premise is now
a verification question; it is not authorization to delete the owner's settings.

## Actual persistence boundaries

- `VixenKit/UITestHarness.swift:54` clears an NSWindow's frame autosave name only
  when moving it offscreen. The initial400ms wait and the early already-offscreen
  return are outside that protection. `hideWindows` skips NSPanel and windows kept
  onscreen. This is a movement guard, not full preference isolation.
- `VixenSequencer/SequencerModel.swift:123` assigns `TimedSequenceEditorForm` even
  in tests. `SequencerContent.swift:453` gives detached NSPanel windows their own
  autosave names; those panels are excluded by `hideWindows`.
- `VixenPreview/PreviewHarness.swift:514` places a visible designer/live window
  without disabling Cocoa frame autosave. Its onscreen exception bypasses the
  normal movement guard. Native color/file-panel state can also be remembered
  independently of frame movement.
- `VixenApp/VixenApp.swift:60` registers restoration/unknown-argument defaults.
  Registration supplies defaults; it does not create a private domain.
  Scene `.restorationBehavior(.disabled)` says scenes do not reopen automatically;
  its interaction with existing geometry keys has not been established here.
- Preview designer's explicit profile bounds restore/save are suppressed during
  harness runs (`DesignerModel.swift:291,960`). Live preview bounds save through
  `ProfileSettings`, and Scheduler status location saves to the private profile.
  Those stores are separate from Cocoa's scene frame preferences.

Thus a harness run cannot prove ordinary Preview geometry restoration merely
because it passes: that specific restore/save path is disabled in the harness.
Conversely, existing negative Cocoa coordinates alone do not prove broken launch.

## Narrow settings override

`vixen-io::settings_dir()` now recognizes `VIXEN_SETTINGS_DIR`. Precedence is:

1. Active UI harness plus its output directory (existing behavior).
2. Explicit `VIXEN_SETTINGS_DIR`, for non-harness private runs too.
3. The existing home-based application support directory/fallback.

All consumers of the shared resolver, including Settings.xml and global layouts,
receive the override. It must be set before process startup, because the shared
store is initialized once. Callers must validate an absolute private path. Empty
or relative explicit values are honored as supplied rather than silently falling
back to the owner's directory; the resolver is not the caller's path validator.
No `HOME` or process-global environment is modified by the tests.

All18 focused settings tests pass in `target/settings-directory-override-tests.log`.
Five new tests cover default/fallback behavior, UI-output precedence, non-harness
selection, empty/relative no-fallback behavior and a real private save/reopen that
leaves a separate fake-owner sentinel unchanged. Independent review found no
remaining issue with this narrow resolver change.

## Reusable verification setup

A centralized runner preparation utility is preferable to adding frame guards to
each view. Root's `scripts/prepare_private_test_app.py` copies the audited bundle,
assigns a unique `com.vixenlights.verification.*` identifier, re-signs only that
copy and verifies normalized executable code/data equality plus unchanged source
resources. This isolates the Cocoa application domain for ordinary panels as
well as harness windows. Rust isolation still requires UI_TEST+OUT or the new
absolute private settings override; a private CFBundleIdentifier alone is not
sufficient. `VIXEN_DATA` and muted output remain independently required.

Prepared frame cases are in `target/preference-isolation-review/restore-seeds.json`.
The next bounded runtime check should compare a fresh private domain with seeded
negative and normal Administration frame strings, observing the actual initial
frame before any relocation hook. Run without the harness to reach ordinary
restore and file-panel code, with a generated controller-free profile, private
settings, disabled web server, `no_execution`, `no_controllers` and mute. Compare
the production domain hash before/after. No native launch or defaults import was
performed for this preparation.

This can resolve whether saved geometry affects ordinary startup on the current
host. It still does not establish Finder/LaunchServices delivery, a disposable
macOS user, Documents permission grant/denial, Gatekeeper/notarization, or macOS26
runtime behavior. Those remaining gates should stay separate.

## Verified build checkpoint

The full workspace passes3,523 tests with0 failures and9 ignored after the
settings override (`target/workspace-settings-isolation-result.json`; log
`target/progress-workspace-settings-isolation.log`). The standard debug build
succeeds and its bundle audit passes. Root's preparation utility created
`target/private-app-native-panels/Vixen.app` with unique identifier
`com.vixenlights.verification.cdb5be4810bd44cdba318d255cbf9dcb`.
Its audit and code/data equality checks pass; the source bundle remains unchanged
by cloning. `target/settings-isolation-debug-preparation.json` records both full
executable hashes, the normalized code digest and exact launch preconditions.
Neither bundle was launched here; release and portable packs were not changed.
