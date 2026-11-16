# Layer row drag verification (F62)

**Passed:**59 native steps and five real OS pointer drags on the isolated release-code clone. F62 is verified within the row-drag cases below.

`target/layer-drag-foreground/result.json` records normal save/reopen, exact UUID/filter/settings preservation, descending levels, clean/modified states, no added Undo and stopped transport. The owned app and helper exited, the prior app/pointer were restored, and the fixture source hashes remained unchanged. The app SHA is b5e6f8a2…825d4 (unique preference-domain clone of releasefd5c); no product Layer changes were needed.

The upstream contract is `LayerEditor/Views/LayerEditorView.xaml.cs:96–151`
and `LayerEditorLayerService.MoveLayer`: a non-default row moves to the target
row's index; the Default target clamps to the preceding index. Same-index,
Default-source and outside-row drops do nothing. Valid changes mark Modified
and recompute descending levels, without adding Undo.

`crates/vixen-session/examples/layer_drag_fixture.rs` creates a new
controller-free profile and a 10-second empty sequence. Its four layers are
Zebra / Alpha / Middle / Default, with distinct UUIDs and mixing-filter instance
IDs. Mask And Fill and Luma Key carry nondefault settings. The generator saves
and reopens through the normal Rust session to check those values.

`scripts/test_layer_drag_foreground.py` copies that fixture and requires a
unique-bundle-ID app prepared by `scripts/prepare_private_test_app.py`. This
separates Cocoa frame preferences as well as the harness's Rust settings and
profile. It pins source, app, clone-manifest, helper and script hashes.
`scripts/foreground_layer_drag.swift` checks the exact PID/executable, actual
foreground/key/main ownership, and WindowServer ownership before every OS
pointer event. It obtains all four row bounds through external accessibility,
and restores the previous frontmost app and pointer on completion or error.

The accepted 59-step case exercised these five real pointer drags in order:

1. Zebra within its own row: unchanged and clean.
2. Default onto Zebra: unchanged and clean.
3. Alpha onto blank space in the layer scroll area: unchanged and clean.
4. Middle onto Zebra: Middle / Zebra / Alpha / Default, Modified.
5. Zebra onto Default: Middle / Alpha / Zebra / Default, Modified.

It then uses the normal Sequence Save menu, closes and reopens the sequence,
and checks native row order, clean state, no Undo and stopped transport. Saved
XML must retain the original UUIDs, filter instance IDs and full settings, with
levels 3 / 2 / 1 / 0. No layer mutation hook or new app instrumentation is used.

Preparation evidence on release code `fd5c974c03f8b424966ddbecff143e98a24ee692c4bbe57a2f8abb185b0895b5`:

- `target/private-app-layer-drag/private-app.json`: signed private app SHA
  `b5e6f8a2249c57bf078b4690e40947b45e498c470ff3dc18668f4ef0f6c825d4`;
  all bytes outside code-signing metadata match the audited source app.
- `target/layer-drag-layout-probe/result.json`: 13 background harness steps,
  exit 0; `ui/layers.png` shows the four collapsed rows and blank drop area.
- `target/layer-drag-external-ax/foreground.json`: read-only external AX sees
  all four full-width `AXDisclosureTriangle` rows, each 28 points high at a
  30-point pitch, inside the Layers `AXScrollArea`; foreground unchanged.
  The private app and inspector both exited 0; 12 harness steps passed.
  The in-process accessibility dump omits these SwiftUI children, so it was
  insufficient to establish external accessibility availability on its own.
- `target/layer-drag-foreground/prepared.json`: frozen provenance for the accepted foreground run.

The completed run was (use a fresh prepared output for future reproduction):

```sh
python3 scripts/test_layer_drag_foreground.py \
  --app target/private-app-layer-drag/Vixen.app \
  --profile target/layer-drag-source \
  --out target/layer-drag-foreground --run-prepared
```

Do not launch it without the exclusive foreground slot. The driver is bounded
to 85 seconds, with an outer process deadline of 95 seconds; its normal prepared
schedule is about 50 seconds. Unknown/missing accessibility geometry or lost
window ownership stops input and remains a failed verification, not a pass.
