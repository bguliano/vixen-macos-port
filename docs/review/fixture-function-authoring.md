# Fixture function authoring — November 11, 2026

The catch-all Fixture effect now constructs an added function from its Functions
selection. Previously the generic record-list Add appended an unconfigured
`FixtureFunctionData`: its hidden function name was empty, so rendering skipped
it. Sequence loading also lost the nonserialized Functions selection.

The effect-local constructor follows upstream
`Effect/Fixture/FixtureEffect/FixtureFunctionExpando.cs:24–80`: name, identity,
type, first index and color-wheel values, white gradient and timeline color come
from the selected target capability. Range starts at zero; RGB/RGBW start at full
intensity, serialized in Range as `FixtureModule.UpdateModelData` does. Capability
lookup keeps the existing upstream first-target/first-matching-definition rule.
The panel has one exact `FixtureFunctionData` branch; all generic list behavior,
including multi-edit copying First's list and empty-list restrictions, is unchanged.

On ModuleData loading, the effect restores only the transient first-function
selection established by upstream target initialization. It preserves saved
function metadata and authored rows (including the existing index repair rule).
On a FunctionData edit, the effect snapshots the current target capabilities in
the same Undo transaction. Thus a profile changed since sequence save supplies
current index choices and UseCurve visibility to newly added rows; existing
authored row values remain unchanged. Undo restores both the list and snapshot.
Missing targets or an unavailable selection make Add a no-op rather than adding
an unnamed unusable row. This is a defensive handling difference from upstream's
blank expando fallback; it adds no Undo entry.

Evidence:

- `target/fixture-authoring-effects-tests.log`: all 78 fx-fixture tests pass.
  Three new regressions cover constructor fields for range/index/color-wheel/
  RGB/RGBW, load preservation, first-target/shared definitions, mixed supported
  and unsupported leaves, and unavailable selections.
- `target/fixture-authoring-panel-tests.log`: all 44 panel tests pass. Four new
  tests use the real picker commit and Add APIs, assert headers and selected index
  controls, verify rendered range/command/RGB intents on two fixture leaves,
  exercise Undo/Redo, and save/reopen a real `.tim` before adding another function.
  Reopen preserves authored data, shows the first function selected, and remains
  unmodified with no Undo history.
  The stale-profile test also checks current index choices and curve visibility,
  unchanged authored rows, and exact full-Params Undo/Redo restoration.

Independent read-only review confirmed the constructor mapping, target rules,
load preservation, narrow panel branch and atomic capability refresh with no
remaining blocker.

The Rust tests above are panel-model, renderer and file-boundary checks. The
separate native workflow now also passes **77 steps across two launches** on the
audited optimized executable SHA256
`4c98baee4fdbcab6d5571ea9c5fa104f50fd843fdaae1b8f8b7098eba32e42b7`:
`target/fixture-authoring-ui-retry/native/result.json`.

`fixture_authoring_fixture` creates two simulated heads with fixture properties,
zero controllers and one empty Fixture effect. `scripts/test_fixture_authoring_ui.py`
uses the actual Functions picker menu and Add button, then changes the index
from Star to Circle. Screenshots/OCR show the blue Gobo header and Star default,
Circle revealing its Range control, and the green RGB header with a white color
strip. Actual Edit-menu Undo/Redo removes/restores the RGB header. Timeline
snapshots report rendered=true, waiting=0 and failed=0 throughout.

Save clears modified state. A new process reopens with the first function Pan
selected, the authored Gobo/Circle and RGB rows intact, no Undo history and no
file rewrite. A separate read-only inspector verifies the saved fields and
rendered output on both leaves: Named8 command value 84/tag Gobo/label G/range
84–126/index type Custom, plus full-intensity white RGB. Each leaf has 80 intents.
The index type is distinct from the function identity Gobo.

The initial runner requested the literal Undo label instead of its contextual
title; that failed attempt is excluded. The corrected 57-step authoring run
completed before a 15-second cold OCR timeout. Its native artifacts were retained
and rechecked with the bounded 45-second helper, then the 20-step reopen run
completed. The test inspector was also corrected to assert the upstream Named8
command instead of a Range intent. `--resume` requires matching native scripts,
the same executable hash and a successful recorded process result; it reruns
assertions without replaying completed authoring actions.

Opening/focusing and state inspection use existing harness hooks. Authoring uses
posted Add/menu actions and actions from the actual opened picker menu; this is
not a claim about physical keyboard focus or menu tracking. No app or Rust
library changes were needed for this native verification, and no physical
fixture, controller, original show, network service or audio output was used.
