# Normal Open/Save panels — verified native flow

Current acceptance: `target/native-panels-menu-only/acceptance.json` passes all 16 retained-run checks. Real normal-app Open Cancel, Unicode Open, Save As Cancel, Unicode Save As, close/reopen and the observed Quit menu all work on private app `030460815c17fae87f9233b62b4fd1715ee0c835481ed0481473247cc77444bf`. The app and helper exit normally, the profile lock is removed, the prior foreground app is restored, and the original private sequence plus owner settings/preferences remain unchanged. Saved effect/target/layer/data-model sections match both the prepared JSON and source XML exactly; saved bytes remain identical through reopen and quit.

The original runner stopped only in postprocessing because Python XML attributes were tuples, while their prepared JSON representation had become lists. The canonicalizer now consistently uses lists. Original `result.json`, input manifests and native evidence are preserved; a separate read-only acceptance report rechecks all final conditions without relaunching the app. The user’s Vorssaint hold-to-quit setting explains the earlier tapped Command-Q; the accepted run uses the normal Quit menu and makes no shortcut claim. Earlier attempts and preparation below remain historical.

The existing UI harness substitutes Save As paths. This separate check deliberately uses a normal app launch with no `VIXEN_UI_TEST` or `VIXEN_TEST_SAVE_AS`, so a successful future run will exercise the real AppKit file panels.

The first normal-app runs observed real, owned AppKit Open and Save sheets. Open Cancel, opening the Unicode source, and Save As Cancel pass; final Save As/reopen acceptance remains pending. No UI_TEST or scripted file-panel substitution was used. Both attempts preserved the original private sequence and the owner’s settings/preference hashes, cleaned up only their own app, and restored the previous foreground app.

## Private prepared artifacts

- App: `target/private-app-native-panels/Vixen.app`, bundle `com.vixenlights.verification.cdb5be4810bd44cdba318d255cbf9dcb`, executable SHA `030460815c17fae87f9233b62b4fd1715ee0c835481ed0481473247cc77444bf`. It contains the implemented normal-mode `VIXEN_SETTINGS_DIR` override.
- Fixture: `target/native-panels-prepared/prepared.json`. `native_panel_fixture.rs` now records the persisted effect instance UUID separately from the window-local numeric EffectId. The source is `Panel Source Café.tim`, containing one two-second Set Level effect in a three-second sequence, one element, no controllers or media, and an explicitly disabled web server.
- Driver: `scripts/foreground_native_panels.swift`, compiled to `target/foreground-native-panels` without warnings. It uses AppKit/Accessibility/CGEvent and SHA256; no model hooks, clipboard changes, or coordinate guesses.
- Runner: `scripts/test_native_panels.py`, syntax-checked. Current prepared run: `target/native-panels-directory/prepared.json`, using fresh fixture `target/native-panels-directory-fixture/prepared.json` and helper `target/foreground-native-panels-directory`. Earlier preparation directories and failed runs remain historical.

The fixture build completed with only the pre-existing `butterfly.rs` unused-assignment warning. The normal app's unique Cocoa preference domain and separate Rust settings directory are both required. Private data and settings remain inside the fixture directory; the real preferences and show are never copied over or deleted.

## Intended native flow

The driver will observe and cancel Open; reopen the actual Unicode source through Open's Go To field; cancel Save As; save to `Saved sequences é/Native Saved Café.tim`; close the saved editor; reopen that destination through Open; then use normal Quit. It checks the focused document's real AXDocument URL, source hash at each cancellation and save, live saved-file publication, and unchanged saved bytes after reopening. After exit, the runner compares the persisted effect/target/layer/data-model XML against the original fixture and checks normal profile-lock removal.

The runner validates the clone's binary/code hashes, strict code signature, fixture hashes, no-output flags, disabled web server, private paths and helper/source hashes before launch. It snapshots the owner's existing Settings.xml and Cocoa preference file read-only immediately before/after the run. Any change prevents acceptance; it does not restore or delete owner data.

The helper captures the previous foreground PID before launch. Before every key or AX press it checks the owned executable/bundle, actual foreground app, and owned key/main windows. Only observed native `AXSheet` structures with unique Open/Save and Cancel buttons qualify. Filename identification uses the actual observed source filename before navigation; the same owned field must remain present afterward. The observed Where control must identify the intended private destination before the filename is committed. A missing, ambiguous, disabled, or different-process element stops and records an AX dump. In particular, unknown XPC-owned file-panel views are not automatically trusted; their ownership must be audited before support is added. No system authorization dialog is answered.

Overall input work is bounded by 75 seconds and each AX request by 0.3 seconds. The parent gives the driver 165 seconds, then cleans up only its own process groups; unsuccessful runs cannot count as normal Quit. The previous app is restored. No foreground activation or input occurs during ordinary preparation.

## Future run

Use only during a freshly agreed exclusive desktop interval, with no other Vixen instance:

```sh
python3 scripts/test_native_panels.py \
  --prepared target/native-panels-directory-fixture/prepared.json \
  --helper target/foreground-native-panels-directory \
  --out target/native-panels-directory \
  --run-prepared
```

Do not reuse the fixture after an attempted launch. Retain `foreground.json`, the bounded AX snapshots, process logs, source/saved hashes and `result.json` on failure. The first actual native-panel run may expose an AX structure assumption; that is a diagnostic result, not evidence of successful native-panel behavior.

## Observed normal-app attempts

`target/native-panels-foreground-reviewed/result.json` stopped before input because the directly launched app did not activate itself. The helper now waits for Launch Services registration and explicitly requests activation only after checking its owned executable and private bundle identifier.

`target/native-panels-activate/result.json` then reached both real native panels. It cancelled Open, opened `Panel Source Café.tim` through the real Go To field, cancelled Save As, and reopened Save As. The app's AXDocument and displayed title use decomposed Unicode; comparison now normalizes paths while retaining exact file hashes. No source modification or saved output occurred.

The second attempt stopped safely before Save because the panel interpreted the Go To directory path without a trailing slash as a proposed filename: Where showed the parent `native-panels-activate-fixture`, and the name became `Saved sequences é.tim`. This is a driver navigation error, not evidence of an application Save failure. The corrected helper preserves the observed name field, enters the directory with an explicit trailing slash, requires the observed Where value to match the private destination, then enters and commits the intended filename. Final Save/reopen/normal Quit remain unverified until that complete native run passes. The retained AX dumps document these exact observations.

The corrected helper compiles; the local SDK emits only an existing deprecated activation-option warning. It has not yet been run. All failed fixtures retain their locks and diagnostics and will not be reused. The runner never restores a third-party foreground app over unexpected user activity.

## Native Unicode Save/reopen and owner keyboard setting

The directory correction ran in `target/native-panels-directory`. Open Cancel, Unicode Open, Save As Cancel, Unicode Save As, closing the saved editor and reopening it all completed with real normal-app native panels. `file-panel-review.json` independently compares every selected TIM XML section (effect, target, layer, data models, marks and media): all match the original fixture. The source hash, both owner-preference hashes and live saved-file bytes after reopening are unchanged. This establishes that file-panel flow on the exact `03046081…44bf` private app.

The original combined runner still reports failure because it tapped Command-Q and then had to clean up its owned app. The owner subsequently confirmed Vorssaint requires holding Command-Q. The tap was therefore incompatible with the host keyboard setting; it is not evidence of a Vixen shutdown bug. No production change is indicated. Normal Quit-menu exit and lock cleanup remain to be observed separately. The independent audit is `docs/review/native-panel-quit.md`.

The current prepared run is `target/native-panels-menu-only`, using untouched fixture `target/native-panels-quit-menu-fixture/prepared.json` and helper `target/foreground-native-panels-menu-only`. It repeats the normal file flow and invokes only the observed, owned, enabled Quit Vixen menu item. It records AXEdited/menu metadata, preserves unexpected sheets, and never changes the owner’s hold-to-quit utility. It waits at most 15 seconds for normal exit; failure captures a one-second sample of only its own child before bounded cleanup. The separately prepared fallback-key diagnostic was never run.
