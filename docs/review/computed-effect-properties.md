# Computed effect property audit — November 11, 2026

This audit traced upstream read-only properties and the port's visible properties
with no XML member through loading, placement and the effect panel. It did not
infer authoring behavior from rendering test counts or launch an app.

The bounded inventory covers all four upstream `[ReadOnly(true)]` effect
properties, all generated `Editor::Label` rows, and all nine visible `xml: ""`
properties. The two PixelEffectBase counters are explicitly non-browsable.
Video Length and Cache Size are the only visible read-only labels. The remaining
computed properties are Pattern's two Brick aliases, Fixture's Functions choice,
Picture/Video/Glediator orientation aliases, State's two choices, and Morph's
polygon-editor entry point. Fireworks' additional orientation property is hidden.

Concrete gaps identified were Video Length (separate Video metadata work),
Pattern's loaded aliases, and Fixture's loaded selection/new-row construction
(separate Fixture work). Picture/Video/Glediator orientations and State labels
have explicit ModuleData hydration; Morph's entry point reaches the registered
Tools polygon editor. This is a bounded source-path audit, not a physical UI
verification claim for every effect property.

## Pattern fix and evidence

Upstream `Pattern.cs`'s `BrickHeight` getter returns `_data.WeaveThickness`, and
`BrickColors` returns `_data.HorizontalColors`. Its ModuleData assignment replaces
the data object; getters immediately reflect the replacement. The port kept
separate nonserialized aliases, initialized to 5/red, and synchronized them only
through interactive setters. Sequence and saved-default decoding skipped those
setters, so the panel showed 5/red despite rendering stored values such as 37/blue.

Pattern now copies just those two canonical values into their UI aliases on
ModuleData loading and target placement. Placement covers saved defaults and
pasted data. No central loader or generic setter replay changed; in particular,
AdvancedSizing's setter is not run and cannot overwrite separately saved widths.

Evidence:

- `crates/vixen-session/tests/pattern_aliases.rs` exercises actual sequence reopen,
  NBFX saved-default reload/placement, and `panel_model`. Both tests fail before
  the fix with the stale slider value 5, then pass with 37 and the saved blue
  gradient. They preserve independent AdvancedSizing fields and verify that a
  sequence save remains byte-identical. Logs: `target/pattern-alias-before.log`
  and `target/pattern-alias-after.log`.
- The unchanged C# Pattern oracle still matches 742 cases / 6,406,396 frame values
  exactly, with zero mismatches (`target/pattern-alias-golden.log`). This verifies
  rendering preservation independently of the new panel tests.

Reproduce locally:

```sh
source env.sh
cargo test -p vixen-session --test pattern_aliases --jobs 3
cargo test -p vixen-effect-registry --test golden pattern --jobs 3 -- --nocapture
```
