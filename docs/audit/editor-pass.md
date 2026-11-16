# Editor pass: the four unverified audits

`docs/audit/{set_level,alternating,candle,butterfly}.unverified.json` came from a run whose verifiers all failed. Each
finding below was checked against the C# and the Rust by reading them. Fixes have a test beside the code unless the entry
says otherwise. No finding was individually compared against the running C# in this pass, except where one quotes the
original audit's own run. The golden test, which does compare render output with the running C#, passes with no mismatches
for all four effects after these fixes. I did not check which of the findings its cases reach, and it doesn't cover the
editor's own behaviour.

Status means:

- **fixed**: it was real and it is now fixed.
- **fixed earlier**: it was real, and the shared API this pass was built on already covers it.
- **wrong**: the finding does not hold.
- **left**: it is real and not changed, for the reason given.

| Finding | Status | Note |
|---|---|---|
| set_level-1 | fixed earlier | `vixen-session`'s `apply_load_rules` and `EffectDescriptor::set_targets` call `validate`. `apply_load_rules` now goes through `load_data`. |
| set_level-2 | left | Known-colour identity. See [below](#net-color-equality). Not changed, as instructed. |
| set_level-3 | fixed | A childless group makes `.Element.Id` throw, so the effect stays dark. `set_level.rs` now returns nothing for that target. |
| set_level-4 | fixed | `EffectDescriptor.information` holds the text, and the checker compares it with the C#. |
| alternating-1 | fixed earlier | As set_level-1. |
| alternating-2 | fixed | The depth reset moved to `targets_changed`. A loaded effect keeps its saved depth, walks off the tree and falls back to the leaves. Tested both ways. |
| alternating-3 | fixed | Timing Source uses plain `[DisplayName]` and `[Description]`, which the generator didn't read. The checker reads both, and the table now has the row. |
| alternating-4 | fixed, with a remainder | `Interval` now follows the mode alone, which is the state after a load and after a mode change. The one state left: right after toggling Static Effect, the C# has `Interval = !EnableStatic` until the next load or mode change. That depends on which setter wrote last, and `VisibilityFn` is a function of the current values. |
| alternating-5 | fixed | `GroupLevel` and `Interval` are `Editor::Number`. |
| alternating-6 | fixed | Every row of a category now has the category's order, the first the grid discovers, and `visible_params` breaks ties on display name. See [Ordering](#ordering). |
| alternating-7 | fixed | The checker reads `[ExcludeFromEffectDefault]` from the data member, where `EffectDefaultScrubber` looks. |
| alternating-8 | fixed | `bool_labels`. |
| alternating-9 | fixed | As set_level-4. |
| alternating-10 | fixed | Mark selection normalises through `param_changed_ctx` (`MARK_COLLECTIONS` and the mode setter) with `normalized_mark_selection`. Depth writes are in `targets_changed`. Drop-down choices come from `EffectDescriptor::selection_values`. |
| alternating-11 | fixed | A saved `GroupLevel` is no longer clamped at render: 0 leaves the effect dark, a negative size groups by its size. The clamp is in the setter, as in the C#. Dissolve read it the same way and is fixed too. |
| alternating-12 | fixed | A truncating `%`, and a negative colour index leaves the effect dark. |
| alternating-13 | fixed later | `Mark::apply_minimum_duration`, applied by `vixen-io`'s mark reader. |
| alternating-14 | fixed | Leaves are distinct within a target, not across targets. |
| candle-1 | left | Known-colour identity. See [below](#net-color-equality). Not changed, as instructed. |
| candle-2 | fixed earlier | As set_level-1. |
| candle-3 | fixed | A frequency of 0 now does what the C# does: an infinite state runs to the end of the effect, and a NaN state stops the render there. A negative frequency stays dark because the C# loop never finishes. |
| candle-4 | fixed | A leaf with no element stops the render where it stands, keeping what was built. |
| candle-5 | fixed | New `Editor::Percent`: a 0..1 value on a 0..100 slider that doesn't snap, which is how `Percentage` differs from `Level`. The kind stays `Double` because the XML is a float. |
| candle-6 | fixed | `GroupLevel` is `Editor::Number`. |
| candle-7 | fixed | Ties break on display name. |
| candle-8 | fixed | As set_level-4. |
| butterfly-1 | fixed earlier | `EffectDescriptor::set_param` runs the `TargetPositioning` setter's rule. A test now pins it for Butterfly. |
| butterfly-2 | fixed earlier | `EffectDescriptor::set_targets` follows the first target's Orientation, and `load_data` leaves a saved value alone. A test now pins it for Butterfly. |
| butterfly-3 | fixed later | `crates/vixen-session/src/library_links.rs` brings linked curves and gradients up to date at load, before each render, and when a library changes. |
| butterfly-4 | fixed | `wrapping_mul`, as the unchecked C# does. |
| butterfly-5 | left, in part | The location case is fixed: a leaf with no element leaves a location render dark. The others are left: a curve under two points or a null member in a damaged file throws in the C# and renders something in the Rust. Fixing those means every effect checking its members before rendering, for files no working Vixen writes. |
| butterfly-6 | fixed | As set_level-4. |

None of the 32 findings turned out to be wrong.

## .NET Color equality

**Decision reaffirmed November 11, 2026:** the owner confirmed value-based core
colors. The original State oracle now demonstrates a named Blue fallback to red
upstream while value-only blue stays blue; the port intentionally stays blue in
both cases. See `docs/DIFFERENCES.md` and `tools/golden/states/README.md`. This is
resolved; the historical source-reading discussion below is preserved.

These are set_level-2 and candle-1. `vixen_core::Color` compares ARGB channels only. `System.Drawing.Color.Equals` also
compares the known-colour state and name, so `Color.White != Color.FromArgb(255, 255, 255)`. This difference is deliberate
and stays, as instructed: colour identity is not being added to the core type.

My reading of the code, which I did not check against a running C# in this pass:

- `ColorModule.getValidColorsForElementNode` adds the property's colours to a `HashSet<Color>` unchanged
  (`Property/Color/ColorProperty.cs:174, :178`).
- `ColorSetupHelper` stores its panel's default, the known `Color.RoyalBlue` (`ColorSetupHelper.cs:26-27`). Colours from the
  built-in sets are `FromArgb` (`ColorStaticData.cs:21-24`).

So there are two cases, and they go different ways.

**The element's colours are named.** Colours an effect computes are unnamed: `RGB.ToArgb` builds them with `FromArgb`, and
gradients give theirs through `ColorPoint`. So nothing computed can match a named entry:

- Set Level's `validColors.Contains(_data.color.ToArgb())` is false, so it replaces the colour with `validColors.First()`,
  stores that as an `RGB`, and reads it back unnamed.
- `colors.Contains(Color)` at `SetLevel.cs:139` is then false as well, so the leaf is skipped.
- The gradient effects intersect the leaf's colours with `GetColorsInGradient()` and get nothing.

So upstream goes dark on such an element and the Rust lights it. That agrees with your reading. The owner's show doesn't
reach this: the original audit found all 10 SingleColor properties and both colour sets saved with `state=2`, which is
unnamed.

**The effect's colour is named and the element's set isn't.** This is a new effect holding a default like
`Color.White` (`CandleData.cs:17`, and Chase's, Spin's and Twinkle's `StaticColor` work the same way). On a set holding the
same ARGB, such as RGBW:

- `validColors.Contains(_data.Color)` is false, so upstream replaces the colour with the set's first entry, which is red
  for RGBW. The Rust finds white in the set and keeps it.
- Only effects still holding a named colour are affected. Once corrected, or once saved with `FromArgb`, both agree.

## Ordering

The brief asked for ties in declaration order. The grid doesn't do that, so I followed the grid:

- `Editor/EffectEditor/PropertyEditing/PropertyItemComparer.cs` sorts properties by `[PropertyOrder]`, then
  `string.Compare(DisplayName, DisplayName, ignoreCase)`.
- `CategoryItemComparer.cs` sorts categories by order, then name ignoring case.
- `CollectCategories` creates each category from the first of its properties it meets. So a category's order is the first
  one discovered, and any other order declared on the same category is ignored.

`visible_params_for` and `scripts/check_effect_metadata.py` both follow that. `string.Compare` is culture-sensitive and the
port compares lower-cased text. The two differ only on punctuation, which these labels barely use.
