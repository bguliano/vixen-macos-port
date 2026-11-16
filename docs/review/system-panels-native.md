# Actual macOS Color and Font panels

Status: actual native Color and Font panel edits and normal closes pass. No production behavior was changed in this follow-up.

The private native host invokes the actual compiled `ColorPanelSession` and `FontPanelSession` from the pinned fixed Kit inputs at `target/color-session-native-fixed-02/prepared.json`. It opens no Core, show, media or output devices. The external helper uses observed Accessibility controls in that owned process; it does not supply chosen values through `testSetColor`, `testSetFont` or direct session callbacks.

## Existing evidence and limits

`target/system-panels-native-02/events.json` shows that confirming the actual RGB hexadecimal field changes blue to red, emits red live state, and commits red once on normal Color-panel close. The actual Font Size field accepts 18, but the host's 90-second watchdog expires before normal Font-panel close. The Arial 18 callback appears after termination begins. This is evidence of the native font change, **not** successful normal close or complete session verification.

The Font Size Accessibility value was initially blank. That alone does not establish an initialization defect. The replacement host records `NSFontManager.shared.selectedFont` immediately after `FontPanelSession.begin`, so actual initial selection can be compared with the panel's AX field value.

## Prepared replacement

- Host: `scripts/system_panel_host.swift`, with a 180-second watchdog and read-only native close notifications.
- Driver: `scripts/system_panel_driver.swift`, restricted to one explicitly observed window for each action. Large font list/table subtrees are pruned; action results contain the selected control rather than repeatedly dumping hundreds of font rows. It retains executable hash/PID/private-domain checks, existing-AX-permission checks, enabled-control checks, foreground guards and guarded prior-app restoration.
- Runner: `scripts/check_system_panels.py`, bounded to 150 seconds before cleanup, with separate 16-second driver deadlines and 5-second callback waits. It requires normal native closes and callbacks **before** the explicit Finish button; any watchdog event fails acceptance.
- Prepared bundle/helper and provenance: `target/system-panels-native-final/prepared.json`. The Kit module is reconstructed from its pinned matching snapshot, alongside the pinned Kit/Bridge/FFI objects; no shared app build is overwritten.

The runner will use the actual blue RGB field, confirm red through its advertised `AXConfirm` action, close Color, open Font, verify the initial Arial 12 selection, confirm size 18 through the observed Font Size combo box, close Font, and finish the host. Acceptance requires one red color commit, one Arial 18 font commit, both native close notifications, normal host termination, no watchdog, and restoration of the preceding foreground app. The earlier failed runs remain intact.

Run only in the root-coordinated desktop slot:

```sh
python3 scripts/check_system_panels.py --out target/system-panels-native-final --run-prepared
```


## Completed normal-close verification

`target/system-panels-native-final-v2/result.json` passes in **5.13 seconds** under the user's unattended desktop authorization, with root holding the foreground slot exclusively. The host uses pinned production Kit code; no test-set method or direct commit callback supplies the chosen values.

- Color: actual Color Sliders/RGB mode, hexadecimal blue `0000FF` changed to `FF0000` through the field's native AXConfirm action. Red live state appears, then native Close produces `color-ended`, one red commit and `color-window-closed`.
- Font: the actual Font Size combo box confirms 18. Native Close produces one Arial 18 callback (not bold/italic), followed by `font-window-closed`. Both occur before the explicit Finish button and normal application termination.
- No watchdog event occurs. The host exits0, all helper processes exit, and the preceding Chrome process is restored.

At initial Font open the AX Size value is blank, while the read-only `NSFontManager.selectedFont` observation is ArialMT, family Arial, size12. The blank AX value alone is not treated as evidence that the session initialized to the wrong font; visual initial-field rendering is not claimed by this observation.

The first replacement attempt at `target/system-panels-native-final` safely failed in1.90 seconds because RGB mode selection returned from AXPress before the panel finished updating. It stopped, terminated its owned host and restored Chrome. Restoring a short 250ms post-press UI settle fixed the test driver's timing; the earlier attempt remains intact. Target-window traversal and pruned font lists reduce the former90-second timeout to a five-second successful run without changing production session behavior.

This covers native changed-color and changed-font acceptance and normal close. No-op/reverted-value semantics retain the separate compiled-callback evidence; they were not added to this foreground run.
