# Building the SwiftUI windows

The owner's brief, in their words: *"feel free to modernize slightly. it doesn't have to look identical as long
as the buttons are where they typically are and the functionality is the same. if there are modern SwiftUI best
practices or Apple-isms that should be used, do so, as this is a mac native software."*

So: **same functionality, same layout regions, native Mac behaviour.** They know Vixen well and have used it for
years, so anything they relied on must still be there and easy to find.

## What to keep from the original

Every window has a spec in `docs/reference/forms/<upstream path>.json`, generated from the WinForms designer files:
every control, its label, tooltip, position, grouping, menu structure and event handler name. The handlers' logic is
in the matching `.cs` file under `upstream/Vixen/src`. WPF windows have `.xaml` and `.xaml.cs` files there instead.

- Keep every feature, option and label wording. Don't drop a control because it looks obscure.
- Keep each control in the same region of the window and the same order, so muscle memory still works.
- Reference screenshots of the real app are on https://www.vixenlights.com/docs/ (for example `started/qs-preview-setup/`,
  `started/qs-controller-setup/`, `started/qs-first-sequence/`, `usage/...`). `upstream/Vixen/Assets/Editor.png` shows the sequencer.
- Only leave out what truly has no meaning on a Mac (Windows-only paths, for example). Record each omission.

For each window you finish, write `docs/parity/<window>.md`: a table of upstream controls and menu items, each marked
done (with where it lives now) or omitted (with why). A control missing from the table counts as not done.

## What to change: Mac conventions

| Original | On the Mac |
|---|---|
| In-window menu strip | The menu bar, with `.commands`. Same menus and items, plus the standard App, File, Edit, Window and Help behaviour |
| Ctrl shortcuts | ⌘ equivalents. Keep single-key shortcuts as they are. Check `TimedSequenceEditorForm_Hotkeys.cs` |
| Options / Preferences dialogs | `Settings` scene (⌘,) |
| Modal dialog windows | Sheets on the owning window where a dialog belongs to it; separate windows for tools you work alongside |
| MessageBoxForm | `.alert` / `.confirmationDialog`, with verbs on the buttons ("Delete", not "Yes") |
| Open / Save dialogs | `.fileImporter`, `.fileExporter`, or `NSOpenPanel` / `NSSavePanel`. Drag and drop files where it makes sense |
| Vixen's colour picker | `ColorPicker` / the system colour panel, except where discrete element colours restrict the choice |
| Segoe UI | The system font. Monospaced digits (`.monospacedDigit()`) for times and counters |
| Hand-drawn dark theme | Native controls in dark appearance (the app forces it by default, and timeline canvases use `Theme` colours) |
| Tool button icons | SF Symbols where one clearly means the same thing. Keep Vixen's own icons for effects (`Resources/effects/*.png`) and anything distinctive |
| Dock panels | Right-side panels with the same tabs and order, toggled from the View menu, each detachable to a floating panel |
| Windows scrollbars, mouse wheel | Trackpad gestures work everywhere: two-finger scroll, pinch to zoom on canvases |
| Ctrl-click to multi-select | ⌘-click and ⇧-click, as elsewhere on the Mac |
| Tooltips | `.help("…")` with the original tooltip text |

Also: right-click context menus keep the same items (`.contextMenu`), Undo and Redo use the window's `UndoManager` so the Edit
menu names the action, destructive actions use `role: .destructive`, and VoiceOver labels come free by using real controls.

## Modern SwiftUI, concretely

- Swift 6 language mode. Modules here default to `MainActor`; mark background work `nonisolated` or `@concurrent` and don't touch the UI from it.
- State in `@Observable` classes, passed with `@Environment` or `@Bindable`. No `ObservableObject`, no singletons beyond the app model.
- `NavigationSplitView`, `HSplitView`, `Inspector`, `Table`, `Form` (`.formStyle(.grouped)` in settings), `GroupBox`, `DisclosureGroup`, `LabeledContent`, `ContentUnavailableView`, `Grid` for aligned dialog fields.
- Multiple windows are scenes: `Window` / `WindowGroup(id:for:)` with `openWindow`. Documents go through `DocumentGroup`.
- Don't use AnyView or GeometryReader where a layout container does the job. No fixed pixel sizes except where the original's proportion matters. Windows must resize sensibly.

## Performance rules

The original's slowness is why the owner chose Rust. The UI must never bring it back.

- **Canvases and big lists are AppKit**, wrapped in `NSViewRepresentable`: the timeline, element trees (`NSOutlineView`, which is virtualized; 1,000+ rows is the norm and 50,000 must work), the preview. Never put thousands of rows in a SwiftUI `List`, and never use a SwiftUI `ForEach` per pixel or per effect.
- The preview is drawn by Rust on its own thread into a `CAMetalLayer`. Swift only forwards events to it.
- Never poll Rust faster than the display. Read playback position with `TimelineView(.animation)` or a display link, and move the play cursor as a separate `CALayer` so the grid doesn't redraw every frame.
- Fetch from Rust in bulk and cache in Swift; don't call across FFI per row per draw. Rendering, sorting and searching happen in Rust, off the main thread.
- A drag must never block on disk or re-render whole documents.

## Project structure

```
app/Package.swift
app/Sources/VixenKit         Shared: Theme, tree view, curve and gradient views, AppModel, WindowID, UI test harness
app/Sources/VixenApp         @main: puts the app together from whichever area modules a build has
app/Sources/Vixen<Area>      One module per window area: Admin, Dialogs, Setup, Preview, Timeline, EffectEditor, Marks, Sequencer, Scheduler, Tools
crates/vixen-ffi/src/core.rs The application object, VixenCore
crates/vixen-ffi/src/<area>.rs  The Rust side of each area's API (UniFFI), one Cargo feature per area
crates/vixen-session         What windows do, without the windows: sequences, undo, playback, timeline logic
```

The generated bindings aren't in `app/`. Each build generates its own, in its build folder.

An area's module exports a `Scene` named after it (`AdministrationScene`, `SetupScene`, `PreviewScene`, `SequencerScene`, `SchedulerScene`, `ToolsScene`) with an `init(model: AppModel)`. `VixenApp` includes it with `#if canImport`. **An area may use VixenKit and VixenBridge, and never another area**, so it can be built alone. The one exception: `VixenSequencer` assembles the window from `VixenTimeline`, `VixenEffectEditor` and `VixenMarks`. Each of those three also exports a small test scene (`TimelineTestScene`, `EffectEditorTestScene`, `MarksTestScene`) that the app uses when `VixenSequencer` isn't in the build, so it can be developed alone. `VixenSetup` may likewise use `VixenDialogs`, the property, filter and controller setup dialogs, whose test scene is `DialogsTestScene`. Areas open each other's windows by id, with `openWindow(id:)` and the constants in `WindowID`. Anything two areas need goes in VixenKit; add a new file rather than changing a shared one if you can.

The Rust side works the same way: `core.rs` and `sequence.rs` (the `SequencerSession` object: open, save, render status, transport, undo) are shared, and each `<area>.rs` adds its own `#[uniffi::export] impl` blocks or objects and depends on nothing but those.

Swift stays thin: no business logic in Swift. If behaviour exists in the C# non-UI code, it belongs in Rust and is exposed through
`vixen-ffi`. Validation, undo payloads, sorting, patching algorithms and file handling all live in Rust, so they can be tested without a window.

What's there to build on, in `crates/vixen-session`:

- `app::AppCore`: the open profile. `VixenCore.inner` is one. `session_environment()` gives what a sequence needs.
- `sequence::SequenceSession`: one open sequence, with stable `EffectId`s, background rendering (`render_all_dirty`, `apply_finished_renders`), the schedule, and save. Its edit methods record nothing; call `record_undo` with one `undo::UndoAction` for each user action.
- `playback::ShowPlayback` (`AppCore.playback`): play, pause, seek, position, execution open and close. One sequence plays at a time.
- `compare` and `tests/real_show.rs`: the whole pipeline against the owner's real exports. If that test starts failing, you've broken something real.

## Building and testing

```bash
source ./env.sh
scripts/build_app.sh debug        # Rust + bindings + Swift + bundle at build/Vixen.app
```

**Building alongside other workers:** give your build a name and list only the areas you need, so someone else's half-finished area can't break your build, and yours can't break theirs:

```bash
VIXEN_BUILD_ID=setup VIXEN_AREAS=setup,admin scripts/build_app.sh debug
```

That builds everything into `target/build-setup/` and the app is `target/build-setup/Vixen.app`. The sequencer window alone is the area `shell`; its parts `timeline`, `effecteditor` and `marks` can be added to the list, and `sequencer` means all four. The first build of a new id takes a few minutes; later ones take seconds. `admin` gives you the main window to start from. Keep your own Rust and Swift compiling between edits anyway: `crates/vixen-session` and `VixenKit` are shared.

### The UI test harness

The app can drive itself: no screen control, no focus stealing, no windows in the way. Set `VIXEN_UI_TEST` to a JSON script:

```bash
VIXEN_UI_TEST=/path/to/script.json VIXEN_UI_TEST_OUT=/path/to/out VIXEN_DATA=/path/to/profile \
  target/build-setup/Vixen.app/Contents/MacOS/Vixen
```

```json
{ "steps": [
  { "action": "window", "id": "display-setup" },
  { "action": "wait", "seconds": 0.5 },
  { "action": "click", "label": "Patch Elements" },
  { "action": "menu", "path": ["File", "Save"] },
  { "action": "snapshot", "name": "display-setup" },
  { "action": "dump", "name": "display-setup-controls" },
  { "action": "quit" }
] }
```

`menus` writes the menu bar as text, with each item's shortcut, whether it's off, and whether it's hidden. `selectRow` with a `label` selects the table or list row showing that text, because a test window is never key and ignores a first click on a table. `closeWindow` closes the current window as its close button would. Alerts are reachable: `dump` lists their text and buttons, and `click` presses them. An alert's text is light on transparent, so its snapshot looks mostly blank. Under test the app writes what it would hand to another app, such as a browser address, to `notes.txt`.

`popup` (`x`, `y`, `label`, and `button: "right"` for a context menu, and `path` to walk submenus first) clicks a pop-up or menu button at those window coordinates and chooses by title (`name` can instead locate its currently displayed text, with optional `index`), and `editCell` (`x`, `y`) starts editing a table cell. A plain click can't do either here: SwiftUI draws its own pop-ups, and a test window never takes a first click on a table.

`snapshot` writes `<name>.png` of the key window and `dump` writes the window's accessibility tree as text, to `VIXEN_UI_TEST_OUT`. **Read those PNGs
with your Read tool and look at them.** "It compiled" doesn't mean it works: open every window you build, exercise its controls, and check the picture against
the reference. Results go to `VIXEN_UI_TEST_OUT/result.json`, with any step that failed.

Real data for testing is in `/Users/developer/Documents/Vixen 3`, which is the owner's real show and is READ-ONLY. Never point the app directly at
it. A copy for tests is at `target/show-copy` (its `Media` is a link back to the original, so only read it). Copy that folder to your own under `target/` if your test saves
anything, and pass it as `VIXEN_DATA`.

Set `VIXEN_UI_TEST_TRACE=1` to log each step before and after it runs when diagnosing a stalled script. The in-process accessibility reader handles both plain and attributed text returned by SwiftUI; some offscreen icon controls are still absent from that tree, so use posted native pointer events and verify their resulting state when label lookup cannot reach them.

Tests must not make sound or send anything to hardware or the network. The harness sets `VIXEN_MUTE=1`, and output controllers stay stopped unless a test starts them.
