# Prop tree and Order system-input verification

Acceptance checkpoint, 2026-11-12 UTC. Two physical runs passed all **15 drags / 293 steps**. Independent read-only audits pass **157 checks**. No production code changed for this verification.

The existing `target/resume-ui/prop-canvas` 31-step tree-provider and 27-step Order-delegate suites call the production provider/delegate directly. The new runs add actual WindowServer drag start, hover and release evidence, including insertion, Command-reverse and a kind-compatible cycle. F34–F37 now have bounded physical acceptance alongside their earlier source/backend coverage.

`scripts/foreground_prop_drag.swift` and `scripts/test_prop_drag_foreground.py` ran nine independent physical cases using the existing synthetic `canvas.prp`: root-to-child, ancestor-to-descendant, incompatible group-to-light-group and Option-copy invalid drops; a single light move; dragging a selected row in a two-row selection; dragging an unselected row after selecting two others; single-row and multi-row Order reorder. Each case reopened the clean private fixture, used actual CG pointer events, read the actual AX selection and saved a separate package. Four invalid drops remained clean; five successful moves became dirty with the expected Move/Reorder undo name. The descendant target also rejects the source's node kind, so that case does not independently isolate the cycle-check branch.

Only open/save/layout and read-only observation hooks are permitted. No tree/drop/order-delegate or selection mutation hooks occur in the script. Physical source/destination points derive from AX rows, not guessed coordinates. The helper verifies the unique executable/PID, key/main window, actual WindowServer frame, frontmost process and topmost owner at every event. It settles and reads back the pointer before mouse-down, uses a threshold-crossing drag path and bounds all waits. Unexpected AX structure or focus loss stops the attempt with diagnostics.

The private clone is `target/private-app-prop-drag/Vixen.app`, bundle identifier `com.vixenlights.verification.9d83a80a0dea4ce3a4beb2025b13327f`. Its source is the standard debug `build/Vixen.app`, SHA-256 `001c6d37e19bb0617f5e6ad128b9419b9d55388ef8fc2eff012651dd3e4e1632`; clone SHA-256 `4d6095359eeba923bba08af511c6efec2b4d840c636979b3c48afe2d36407776`. Code/data digest matches after identifier/signature changes. Each run copies a controller-free generated profile, disables web/execution/controllers, mutes audio and uses private settings. Original profile/prop files and owner preference hashes are checked read-only before/after. This is a synthetic package test, not compatibility evidence for a Windows-authored `.prp`.

Local preparation checks pass in `target/prop-drag-preparation-check.json`: nine cases, 175 harness steps, and no prohibited mutation hook. Swift helper compiles; its only warnings are the existing deprecated activation-option usage also present in earlier foreground helpers.

Offscreen calibration:

- `target/prop-drag-layout`: restricted-terminal launch exited -6 before harness output; retained, no acceptance.
- `target/prop-drag-layout-02`: elevated native-app launch exited 0, all 15 layout steps passed, and the 1800×1400 screenshot (900×700 points) was inspected. The six rows, group indentation and three lights are correctly visible. Seven independent checks pass in `layout-acceptance.json`, including unchanged source/profile/preferences.
- The original `layout-02/result.json` is preserved with its failed postcheck: it incorrectly required normal profile-lock removal from `UITestHarness.finish`, which explicitly calls `exit(0)`. The corrected drag runner records the retained isolated lock and makes no normal-shutdown claim. No app rerun was used to repair that bookkeeping result. Native normal Quit was verified separately in the file-panel work.

Physical result: `target/prop-drag-foreground/result.json`; app/driver both exited 0 with no forced cleanup, and the pointer/previous application were restored. The immutable prepared run pins helper/source/runner/app/case/script hashes, uses nine-second observation intervals, a 135-second helper deadline and a 145-second process deadline. Its helper SHA-256 is `7e476db7632edc1d3283515225453cdb7b921ce39245793af82f585c3d9e5086`; source SHA-256 `123a2a43d6c31bf06467d5dd45dc33845909c09e792707d1b94c9f31bec084b9`.

Independent read-only audit: `target/prop-drag-independent-check.py` / `target/prop-drag-independent-review.json`, **83 checks passed**. It independently reconstructs expected graphs and order numbers rather than calling the runner's expected-value function, checks actual saved JSON except modified timestamp, verifies background JPEG bytes unchanged, validates nine package hashes and rechecks original profile/prop, prepared inputs, executable/helper/source and owner preference hashes. All 175 raw results align with their pinned input script; all 15 pointer readbacks match calibrated coordinates. Retained result SHA-256: `d91258bee1e2424a4f593c0ea5f46cfaa28406033a7d7d4fff7365c23d0b9cc5`.

Observed changes from clean `Source=[Pixel 1, Pixel 2]`, `Target=[Pixel 3]`:

| Case | Actual saved result |
| --- | --- |
| Four invalid drops | Graph/order unchanged; dirty false |
| Single tree move | Source `[Pixel 2]`; Target `[Pixel 3, Pixel 1]` |
| Drag selected Pixel 2 after selecting Pixel 1+2 | Source empty; Target `[Pixel 3, Pixel 1, Pixel 2]`; both remain selected |
| Drag unselected Pixel 3 after selecting Pixel 1+2 | Source `[Pixel 1, Pixel 2, Pixel 3]`; Target empty; only Pixel 3 selected |
| Order single and selected pair | Both produce `[Pixel 2, Pixel 3, Pixel 1]` with exact orders 1, 2, 3; graph and identities unchanged |

Tree insertion before/after and Command-reverse were outside the initial nine cases; the second run below closes those gaps. Foreign-window/cross-list rejection and arbitrary graph shapes remain outside these physical case sets. The prior provider/delegate and Rust suites retain their separate coverage; this report does not relabel them as physical input. The retained isolated lock is expected from UI_TEST's immediate exit and makes no normal-shutdown claim.

Separate edge acceptance: `scripts/foreground_prop_drag_edges.swift`, `scripts/test_prop_drag_edges_foreground.py` and `target/prop-drag-edges` preserve the accepted nine-case sources unchanged. All **six drags / 118 steps pass**, with both processes exiting 0, no forced cleanup and prior application/pointer restored. The helper adds Command after drag initiation so mouse-down does not toggle the prepared two-row selection.

| Case | Actual saved result |
| --- | --- |
| Pixel 2 before Pixel 1, same parent | Source `[Pixel 2, Pixel 1]`; Target unchanged |
| Pixel 1 after Pixel 2, same parent | Source `[Pixel 2, Pixel 1]`; Target unchanged |
| Pixel 3 before Pixel 1, other parent | Source `[Pixel 3, Pixel 1, Pixel 2]`; Target empty |
| Pixel 3 after Pixel 2, other parent | Source `[Pixel 1, Pixel 2, Pixel 3]`; Target empty |
| Selected Pixel 1+2 with Command | Source empty; Target `[Pixel 3, Pixel 2, Pixel 1]`; both moved nodes remain selected |
| Branch onto its child Inner | Entire graph/order unchanged, dirty false, no Undo action |

The fresh eight-node fixture adds `Branch→Inner`. `model.rs:257` makes no-light nodes groups, and `:559` allows group children in empty Inner; `editor.rs:921` rejects Branch→Inner because Inner is a descendant. This reaches a cycle-only target without breaking valid kind invariants. Five successful moves each register Move undo and dirty=true; the cycle remains clean. The reverse and cycle screenshots were inspected. Exact saved graph comparisons also retain all node/light/state identities, coordinates, metadata, order numbers and background JPEG bytes.

`target/prop-drag-edges-independent-check.py` / `target/prop-drag-edges-independent-review.json` pass **74 independent checks** against the actual package files, raw 118-step results, six foreground records and eight exact pointer readbacks. Original profile/prop, fixture, app/helper/source and owner preference hashes all match their pinned evidence. Result SHA-256: `a106834367f045e248cf86af632fff96784bd3903ef3693cc0881b8c08251afd`; helper SHA-256: `efdcef94b809fff1a1b298e2e3be7409c0f2df1e407314794a03f98cd86289d8`; prepared SHA-256: `06a41a247922e73359ad3b974827028bcf91a9c59041b03462f90b51b4c26141`.

Remaining physical cases are broader boundary checks: cross-window/cross-list rejection, mixed-kind multi-selection, a same-parent/other-parent mixed insertion source, aliases/already-linked target rules, large-tree autoscroll and drag cancellation. They were not exercised by these runs. Existing provider/Rust checks remain the evidence for their covered rules; no new defect was found or new native run required by this audit. Real Windows-authored `.prp` compatibility remains a separate file-format verification gap.
