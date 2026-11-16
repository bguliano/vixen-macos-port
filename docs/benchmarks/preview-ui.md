# Preview windows: measured speed

The owner remembers Vixen's previews and display design as slow and choppy. These are the numbers for the Mac port's live preview
window and Preview Configuration window, taken in the real app with a copy of the real show (`target/show-preview`).

**Conditions.** Apple M5 Pro MacBook Pro, built-in 120 Hz display at scale 2. Release build (`VIXEN_BUILD_ID=preview-rel
VIXEN_AREAS=preview,admin scripts/build_app.sh release`), muted, no output hardware. **All numbers were taken under load**: other
workers were compiling the whole time, with load averages between 10 and 25 during the runs below and up to 169 at other times
(`uptime` is logged next to each run in the scripts' output). On an idle machine expect the same or better.

**How.** UI harness scripts drive the app (`docs/UI_GUIDE.md`) with this area's own steps (`app/Sources/VixenPreview/PreviewHarness.swift`):
`measureLive` reads the renderer's counters and frame gaps over a stretch of real playback, and samples the main thread with a 5 ms tick to
catch any stall. `measureDrag` drives the same FFI calls the canvas makes, one mouse move every 1/120 s, and times each call on the main
thread. The window being measured is brought on screen for the run, because an occluded Metal layer draws nothing, which is correct.

## Live preview: the 2022 tree during a real sequence

"Christmas Tree 2022 (OpenGL)": 2,309 lights, 2,307 of them linked, 1,126 nodes, camera view, background image, window 1058 x 595.
The engine runs at 100 frames a second. A frame whose colours for this preview don't differ from the last one isn't sent to the renderer at all.

| Passage | Engine frames | Frames where this preview's colours changed | Drawn | Replaced before showing | Gaps between drawn frames | Render thread CPU per frame | Process CPU |
|---|---|---|---|---|---|---|---|
| Carol of the Bells (Lindsey Stirling), from 0:40, 10 s, run 1 | 1001 | 831 | 808 (80.8 fps) | 23 | p50 10.0 ms | 0.255 ms | 14.5% |
| The same, run 2 | 1000 | 826 | 773 (77.3 fps) | 53 | p50 10.0 ms | 0.208 ms | 13.6% |
| Wizards in Winter, from 0:30, 10 s | 1000 | 48 | 48 | 0 | p50 200 ms (the beat) | 0.25 to 0.51 ms | 2.7 to 4.3% |
| After playback stops, 3 s | 0 | 0 | 0 | 0 | | | 1.7 to 1.8% |

"Drawn" can't exceed the changes, and the gaps' p99 (130 ms in the busy passage) is the music's pauses, when nothing changes, not pacing.

- **Every change is drawn.** Where colours change on every engine frame, the window draws at the engine's 100 Hz, and the few frames
  "replaced before showing" are pairs that landed inside one 8.3 ms display refresh, where only the newer can show. Where the show
  changes on the beat, the window draws on the beat and nothing in between.
- **The main thread is never held up.** The 5 ms tick on the main thread came back at p99 6.1 to 6.2 ms during the busy passage, so a
  stall of even a few milliseconds would have shown. Its worst was 7.7 ms in one run and 25.6 ms once in the other, with the machine
  compiling. Swift reads the renderer once a second, for the status bar.
- **Process CPU** is the whole app: the engine evaluating every effect at 100 Hz, the colour lookup, the renderer and the harness's own
  5 ms tick. It is 1.7% idle.
- **When nothing changes nothing draws**, and a hidden window draws nothing either (the renderer's `Occluded` skip).
- Scanning twelve passages from seven songs found the 2022 tree's colours changing on 0 to 100% of engine frames: 0 for two passages
  that don't use this tree, 5 to 15% for beat-driven ones, 40 to 100% for chases and fades. The task's "about 65 changes a second" is
  an average over a song of this kind.

Earlier runs of the same passages in a debug build gave the same picture (Carol of the Bells (Lindsey Stirling) from 0:40: 549 drawn of
598 changes in 6 s, gaps p50 10.1 ms, p99 15.6 ms, with a release compile running alongside).

## Opening

| What | Time |
|---|---|
| Live window, everything slow (XML, scene, background decode) | off the main thread in `prepare_live_preview`; the window opens after it |
| Designer on the 2022 preview, the FFI open: session, scene, renderer, repair | 2.5 to 3.3 ms on the main thread |
| Designer, the 2022 background image decoded | 5.2 to 6.1 ms, off the main thread, after the window shows |
| Designer window, from its SwiftUI root appearing to the first frame requested, 2022 preview | 94 to 139 ms |
| Designer window, the same, synthetic 100,000-light preview | 90 to 108 ms |

The window time is mostly SwiftUI building the window, the tool band's 40 icon buttons and the split view, and waiting for the canvas
to be in its window. Upstream decodes and scans the background twice on the UI thread at open (spec hazards 17 and 19); here it's once,
in the background, with the lights already showing over black.

## Dragging in the designer

Main thread time is per mouse event: the FFI call, the session's work, and sending the moved lights to the render thread. The display
refreshes every 8.3 ms, so anything well under that keeps dragging at display rate. "Drawn" is frames the renderer presented during the
5 s drag; mouse moves came at about 107 a second.

| Preview | What was dragged | Main thread per move: p50, p99, max | Mouse down / up | Drawn | Gaps: p50, p99, max |
|---|---|---|---|---|---|
| 2022 (2,309 lights) | one shape, by its end handle (a resize) | 0.036, 0.045, 0.070 ms | 0.02 / 0.20 ms | 94 fps | 10.5, 15.6, 17.1 ms |
| 2022 | all 15 shapes, every light | 0.053, 0.064, 0.085 ms | 0.06 / 0.18 ms | 94 fps | 9.8, 15.6, 18.1 ms |
| 2022 | a rubber band across the whole view | 0.038, 0.056, 0.106 ms | 0.01 / 0.18 ms | 95 fps | 9.8, 16.3, 27.7 ms |
| Synthetic (99,200 lights in 100 mega trees) | one mega tree, 992 lights | 0.047, 0.055, 0.104 ms | 0.03 / 0.26 ms | 90 fps | 11.3, 15.3, 16.8 ms |
| Synthetic | all 100 shapes, all 99,200 lights | 0.46, 0.61, 0.78 ms | 1.2 / 1.0 ms | 95 fps | 9.9, 15.4, 16.5 ms |
| Synthetic | a rubber band across the whole view | 0.041, 0.102, 0.134 ms | 0.01 / 1.1 ms | 92 fps | 10.9, 15.1, 18.6 ms |

The renderer's CPU per frame was 0.20 to 0.27 ms throughout, and process CPU 9 to 17%. Two earlier runs, one with the machine busier,
drew 80 to 103 fps, and put the 100,000-light drag at 0.52 to 1.0 ms p50 and 0.66 to 1.6 ms p99 per move. Mouse moves arrive about 107
times a second from the harness, out of step with the 120 Hz display, so two sometimes land in one refresh and only the newer shows:
that's why "drawn" is a little under the moves. Nothing in a drag touches SwiftUI: the tool band and panels only update when the
selection, the tool or the undo stack change, and the Properties panel refreshes once on mouse up (spec hazards 14 and 22).

The synthetic preview is made by `cargo run -p vixen-session --example synthetic_preview -- <copy of a show>`.

## Editing elements from the designer's tree

Each edit in the Elements tree is made in a working copy of the display off the main thread, the copy then replaces the running display
(nothing is saved), and the designer reloads its tree and drops links to elements that are gone. Real show: 1,136 nodes in `SystemConfig.xml`, the 2019
preview open with 977 linked lights. Two harness runs, pass 2, with other workers compiling on the machine.

| Edit | Until the running display has it | Until the tree shows it |
|---|---|---|
| Drop Relays/Star onto Relays/Plug 11 (converts it to a group) | 42 to 51 ms | 44 to 54 ms |
| Sort the 17 children of Relays | 21 to 39 ms | 23 to 41 ms |
| Create Group from one element | 31 to 35 ms | 37 to 44 ms |
| Paste as New into Light Strip L (554 children) | 36 to 37 ms | 44 to 51 ms |
| Add Multiple, 5 elements | 32 ms | 44 ms |

None of it is on the main thread except the tree reload at the end, which is 2 to 15 ms. The cost is mostly rebuilding the running
display from the copy, so it grows with the whole show and not with the size of the edit: fine at this size, and the first thing to look
at if a show with tens of thousands of elements makes tree edits feel slow.

## What upstream did, for comparison

Upstream redraws the whole canvas on the CPU on every mouse move of a drag, copying the background bitmap and rasterising every light
(spec hazard 1), and its live forms rasterise every light on the UI thread per frame (hazards 10 and 11). There are no upstream numbers
here, because nothing here runs Windows.

## Not measured

- **Pinch zoom and two-finger panning** with a real trackpad: the harness can't make gesture events. Wheel zoom and the code paths
  pinch uses are covered.
- **Real mouse input**: drags go through the same FFI calls the canvas makes, but from the harness, so event delivery from the window
  server isn't in the numbers. The computer-use tool drew and selected shapes in the real window as a check that the path works.
- **The flat view's live numbers** on the 2019 tree: it was shown and played (relays only, so it rarely changes), but not measured.
- **An idle machine**: there was never one.
- **Upstream on Windows**, for a side by side.
