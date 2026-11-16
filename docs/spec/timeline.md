# Timeline Control

## Scope

Citation shorthand, used everywhere below:

| Prefix | Path |
|---|---|
| (bare file name, for example `Grid.cs:123`) | `upstream/Vixen/src/Vixen.Common/Controls/TimeLineControl/` |
| `LM/` | `upstream/Vixen/src/Vixen.Common/Controls/TimeLineControl/LabeledMarks/` |
| `TSE/` | `upstream/Vixen/src/Vixen.Modules/Editor/TimedSequenceEditor/` |
| `Form.cs` | `TSE/TimedSequenceEditorForm.cs` |
| `Hotkeys.cs`, `Menu.cs`, `Toolstrip.cs`, `ContextMenu.cs` | `TSE/TimedSequenceEditorForm_<name>.cs` |
| `Tests/` | `upstream/Vixen/src/Vixen.Tests/Sequencer/` |
| `Marks/` | `upstream/Vixen/src/Vixen.Modules/App/Marks/` |

**Read in full, in scope (`Vixen.Common/Controls/TimeLineControl/`):**

| File | Lines | File | Lines |
|---|---|---|---|
| Grid.cs | 3316 | TimeInfo.cs | 111 |
| Grid_Mouse.cs | 1219 | EventArgs.cs | 163 |
| MarksBar.cs | 1340 | TimeLineGlobalStateManager.cs | 47 |
| TimelineControl.cs | 1291 | ExtensionMethods.cs | 47 |
| Ruler.cs | 985 | DragState.cs | 22 |
| Row.cs | 818 | AlignmentEventArgs.cs | 17 |
| Element.cs | 602 | ElementMoveType.cs | 15 |
| Waveform.cs | 383 | ResizeZone.cs | 14 |
| TimelineControlBase.cs | 259 | BlockingCollectionExtensions.cs (Microsoft helper) | 205 |
| RowList.cs | 256 | DelegateBasedObserver.cs (Microsoft helper) | 30 |
| MarkRow.cs | 250 | TimelineControl.resx (only the stock schema comment, no resources) | 119 |
| RowLabel.cs | 238 | | |

`LabeledMarks/`: TimeLineGlobalEventManager.cs 97, MarksSelectionManager.cs 97, MarkTimeInfo.cs 36, MarksTextChangedEventArgs.cs 31, MarksMoveResizeInfo.cs 25, PhonemeBreakdownEventArgs.cs 23, MarksBulkChangeInfo.cs 22, MarksMovedEventArgs.cs 22, SelectedMarkMoveEventArgs.cs 20, PlayRangeEventArgs.cs 15, MarksPastedEventArgs.cs 14, MarksDeletedEventArgs.cs 13, MarksMovingEventArgs.cs 13, MarkSnapPointRegistration.cs 12.

`Vixen.Tests/Sequencer/` (all 26 files, 4235 lines, read in full). Those that exercise this area: GridAlignmentNullReferenceTests 64, GridCloneDragDuplicateRowsTests 263, GridMarkSnapPointTests 376, MarksBarAutoScrollTests 127, MarksBarMouseWheelTests 160, RowEventScopeTests 55, RulerLockHeightTests 126, TimelineActiveRowNavigationTests 121, TimelineControlTestCollection 9, TimelineCursorSelectionTests 156, TimelineGridDrawingTests 184, TimelineZoomTests 68, WaveformAlignmentRenderingTests 172, WaveformLockHeightTests 117. The rest do not touch the timeline control: LayerEditorLayerServiceTests 221, LayerEditorViewModelTests 150, LayerImportExportServiceTests 420, LayerMixingFilterDataJsonSerializationTests 126, LibraryDragPayloadFactoryTests 105, MarkCollectionImportCommitterTests 164, MarkCollectionImportSelectionViewModelTests 119, MarkCollectionNameServiceTests 176, MarkCollectionTypeTests 59 (pins `MarkCollectionType.State = 4`, relevant to F133), PangolinBeyondMarkImportTests 227, SequenceExecutorLifecycleTests 455, SequenceExecutorTestCollection 15.

`upstream/Vixen/docs/reviews/`: marks-bar-drag-performance-remediation-plan.md 315 (read in full; it is the source of the current incremental snap and clip-aware waveform design). vix-4009-range-selection-order-design-legacy.md 44 (read in full; it concerns `MultiSelectTreeview` range order in Display Setup and says nothing about the ruler range or this control).

**Read partly, to pin the contract with the host form. Not owned by this spec:** `Form.cs` lines 340-579, 855-1014, 1310-1549, 1570-1704, 2152-2395, 2820-3209, 3255-3395, 3400-3479, 3566-3615, 4050-4100, 4162-4180, 4190-4232, 5500-5560, 5578-5592, 5810-5840, 5996-6019, 6218-6232, 6300-6312 (of 6396). `Hotkeys.cs` (334, all). `Menu.cs` lines 150-339 (of 828). `Toolstrip.cs` lines 1590-1620 and 1850-1865 (of 2189). `ContextMenu.cs` lines 1-150, plus a listing of item names to the end (of 493). `TSE/TimedSequenceElement.cs` 54, `TSE/TimedSequenceRowLabel.cs` 95, `TSE/Forms/Form_Grid.cs` 36, `TSE/Undo/ElementsTimeChangedUndoAction.cs` 87, `TSE/Undo/MarksTimeChangedUndoAction.cs` 65, `TSE/EffectRasterizer.cs` (head; already ported to `crates/vixen-session/src/thumbnail.rs`). `Marks/Mark.cs` 105, `Marks/MarkCollection.cs` 385, `Marks/MarkDecorator.cs` 72. `Vixen.Core/Marks/IMark.cs`, `IMarkCollection.cs`, `IMarkDecorator.cs`, `IMarkType.cs`. `Vixen.Modules/Media/Audio/Audio.cs` lines 60-175 and `SampleProviders/MaxPeakProvider.cs`, `PeakProvider.cs`, `Sample.cs`. `Vixen.Common/Controls/Theme/ThemeColorTable.cs` (timeline colours), `Scaling/ScalingTools.cs`, `TextDialog.cs`. Plans `upstream/Vixen/docs/plans/sequencer/vix-3983-*` (requirements in full, plan head), `vix-4001-*` and `vix-1931-*` (heads). `upstream/Vixen/Assets/Editor.png`.

**Also read by the audit of this spec, to check contact points:** `Form.cs` lines 3622-3653, 5436-5511, 5636-5695. `Toolstrip.cs` lines 1425-1450. `ContextMenu.cs` lines 296-392. `TSE/TimedSequenceEditorForm.Designer.cs` (menu trees at the `DropDownItems.AddRange` calls, and every `ShortcutKeys` line). `TSE/Forms/Form_Effects.cs` lines 170-210. `TSE/Forms/FindEffectForm.cs` lines 95-112, 217-252. `TSE/Undo/MarksAddedRemovedUndoAction.cs`, `MarksRemovedUndoAction.cs`, `MarksBulkChangeUndoAction.cs`. `Vixen.Application/VixenApplication.cs` lines 849-866. `Vixen.Modules/Sequence/Timed/TimedSequenceData.cs` lines 15-50, `RowSetting.cs`. `Vixen.Common/Controls/MessageBoxForm.cs` lines 30-60, its designer's button results. `Vixen.Common/Controls/TextDialog.cs`. `Vixen.Core/Extensions/CollectionExtensions.cs` lines 30-55. `upstream/Vixen/docs/plans/sequencer/vix-3936-*`, `vix-3940-*`, `vix-4010-*` (heads). `docs/reference/forms/Vixen.Modules__Editor__TimedSequenceEditor__TimedSequenceEditorForm.json` and `..._Forms__Form_Grid.json`. `docs/spec/sequencer-shell.md`, for what it already owns. `crates/vixen-core/src/marks.rs`, `crates/vixen-io/src/sequence.rs` (mark sections), `crates/vixen-audio/src/peaks.rs`, and the signatures in `crates/vixen-session/src/timeline/`.

**Not read fully, in scope:** nothing. **Out of scope and not read:** `TSE/IntentRasterizer.cs`, the mark collection docker (`TSE/Forms/WPF/MarksDocker/*`, except its Weight menu values), the rest of the editor form. The sequencer window, its menus, toolbars and the effect context menu belong to the sequencer spec. This spec names their contact points.

**Numbering is frozen.** `crates/vixen-session/src/timeline/` already cites these F and Q numbers in its comments. New entries take new numbers at the end of the list; never renumber.

## Features

### Conventions

**Modifier keys.** Every ⌘ below is upstream Ctrl, ⌥ is upstream Alt, ⇧ is Shift (`docs/UI_GUIDE.md`). Upstream's tests come in two kinds and the difference matters: "holding" means the key is among those down (`HasFlag`), "exactly" means no other modifier is down (`ModifierKeys ==`). ⌃ is not an upstream modifier here: ⌃-click means right-click on the Mac, and ⌃ is ignored for all "exactly" tests. Mask to ⇧⌘⌥ and read the flags from the event.

**Units.** One upstream pixel is one point. `ScalingTools.GetScaleFactor()` (DPI / 96) is constant 1 on the Mac, so `32 * scale` is 32. Pointer locations are floored to whole points before any logic, because upstream's are integers. Times are `vixen_core::TimeSpan`, 100 ns ticks.
**Font sizes are the exception.** `new Font(family, n)` is `n` typographic points, which at 96 DPI is `n * 4/3` pixels, and upstream pixels are our points. So keep proportions by multiplying those sizes by 4/3: the row label's 12 is 16, the hover info's 7 is 9.33, the ruler's mark-drag 8 is 10.67, the "Building waveform" 10 is 13.33, and the control font (`SystemFonts.MessageBoxFont`, 9 pt, used for mark text, `TimelineControlBase.cs:34`) is 12. The ruler's label font is created with `GraphicsUnit.Pixel` (`Ruler.cs:269`), so it needs no conversion.

**Coordinates.** *Client*: top-left origin of the surface. *Canvas*: grid client plus scroll, so `canvas.x = client.x + (int)px(VisibleTimeStart)` and `canvas.y = client.y + VerticalOffset` (`Grid_Mouse.cs:36-42`, `Grid.cs:179`, `Grid.cs:257`).

**Terms.** *Effect* is upstream `Element`. *Row* is one occurrence of a display element in the flattened tree. *Occurrence* is an effect in one particular row. *Last row* is `Element.Row`, the row the effect was most recently added to (`Row.cs:673`, `Element.cs:126-129`).

**Treatment tags for odd behaviour** (`docs/PORTING_GUIDE.md` says port quirks as written, except crashes). **KEEP**: port as written and comment that it is deliberate. **GUARD**: upstream throws or crashes; do not panic, do the stated safe thing. **FIX**: a deliberate divergence, listed for the owner's veto in the quirk register at the end of this section. Most FIX items exist because upstream computes behaviour in `OnPaint`.

**Entry format.** *Do* what the user does. *Does* the exact behaviour. *Rules* preconditions and enabling. *Default*. *Undo*. *Persist*. A missing line means none.

### A. Panel, viewport and time mapping

**F1. Timeline panel.**
*Does:* the control is hosted in the dock tab `Form_Grid`, titled "Timeline" (`TSE/Forms/Form_Grid.cs`, reference `docs/reference/forms/Vixen.Modules__Editor__TimedSequenceEditor__Forms__Form_Grid.json`, which also sets `CloseButton: false`). The title becomes `Timeline - <audio file name>` when audio is set and returns to `Timeline` when audio is removed (`Form.cs:1774`, `Form.cs:1879`, `Form.cs:1993`).
*Rules:* while floating, the tab forwards key presses to the editor's key handler (`TSE/Forms/Form_Grid.cs:14-19`, `TSE/Forms/Form_Grid.cs:31-34`). Mac: the panel is the main area of the sequencer window, not a closable panel. The control has a 1 px `FixedSingle` border (`Form_Grid` reference JSON). View > "Timeline Window" belongs to the sequencer spec (its F76).

**F2. Splitter between row list and time panels.**
*Do:* drag the vertical divider.
*Does:* left width stays fixed when the window resizes (`FixedPanel.Panel1`), minimum 100 (`TimelineControl.cs:140-147`).
*Default:* 200 (`TimelineControl.cs:18`). Restore rule: use the saved distance if it is above 0 and the control is wider than it, otherwise the default if the control is wider than that (`Form.cs:548-561`).
*Persist:* profile setting `SplitterDistance`, written on close (`Form.cs:5835`).

**F3. Stacked regions.**
*Does:* right side, top to bottom: Waveform, Ruler, Marks bar, Grid (filling the rest) (`TimelineControl.cs:191-228`; WinForms docks the last-added control outermost, and `upstream/Vixen/Assets/Editor.png` confirms the order). Waveform is absent when there is no audio (F140). Left side: a blank backdrop of `TimeLinePanel1BackColor` (40,40,40) above the row list; the row list's top is kept level with the grid's top and its height equals the grid's client height (`TimelineControl.cs:168`, `TimelineControl.cs:856-857`, `TimelineControl.cs:1269-1270`).
*Default:* Waveform height 50, Ruler height 50 (`TimelineControl.cs:215`, `TimelineControl.cs:225`). Marks bar starts at 50 and is replaced by its computed height (`TimelineControl.cs:204`, F118).

**F4. Shared time state.**
*Does:* one `TimeInfo` is shared by all surfaces: `TimePerPixel`, `VisibleTimeStart`, `TotalTime`, `PlaybackStartTime?`, `PlaybackEndTime?`, `PlaybackCurrentTime?` (`TimeInfo.cs:12-18`). Setting a value to itself does nothing and raises nothing (`TimeInfo.cs:26-27`). The cursor position, mark selection and timeline-wide events are per-editor objects keyed by an instance id (`TimeLineGlobalStateManager.cs:17-28`, `LM/MarksSelectionManager.cs:19-30`, `LM/TimeLineGlobalEventManager.cs:26-37`). On the Mac these are fields of one per-document object in Rust, not global dictionaries.
*Default:* `TimePerPixel` 100000 ticks (10 ms per pixel), `VisibleTimeStart` 0, `TotalTime` 2 minutes until the sequence sets it (`TimeInfo.cs:8-11`, `TimelineControl.cs:61-67`).

**F5. Time to pixel mapping.**
*Does:*
- `px(t) = t.ticks / TimePerPixel.ticks`, computed in float32 upstream (`TimelineControlBase.cs:144-150`). **FIX Q1:** use f64. At the 2000-tick zoom floor a one-hour sequence is 18 million pixels, where a float32 step is 2 pixels and hit tests go wrong.
- `time(px) = px * TimePerPixel.ticks`, integer px (`TimelineControlBase.cs:40-43`).
- Grid: pointer to canvas as in Conventions. Painting translates by the integer scroll position (`Grid.cs:3173`).
- Ruler: `time = time(client.x) + VisibleTimeStart` (`Ruler.cs:467`). Painting translates by `-px(VisibleTimeStart)` as a float (`Ruler.cs:92`).
- Marks bar: `canvas.x = client.x + (int)px(VisibleTimeStart)`, no vertical scroll (`MarksBar.cs:1140-1147`). Painting as the ruler (`MarksBar.cs:1176`).
- Waveform: painting as the ruler (`Waveform.cs:295`).
*Rules:* `TimePerPixel` of zero throws upstream (`TimelineControlBase.cs:146-147`). **GUARD:** treat as a no-op view, return 0.

**F6. Visible range and clamping.**
*Does:* `VisibleTimeSpan = ClientSize.Width * TimePerPixel` (`TimelineControlBase.cs:120-123`); the composite control uses the grid's (`TimelineControl.cs:262-265`). `VisibleTimeEnd = min(start + span, TotalTime)` (`TimelineControlBase.cs:129-133`). Setting `VisibleTimeStart`: below 0 becomes 0; above `TotalTime - span` becomes `max(TotalTime - span, 0)` (`TimelineControlBase.cs:51-64`). All view changes go through this setter. On resize, if `start + span > TotalTime` the start is re-set to `TotalTime - span` (`Grid.cs:788-789`).
*Note:* Ruler, Marks bar and Waveform compute span from their own width, so with a permanent vertical scroller their end time runs slightly later than the grid's. Use the grid width for all commands, and each surface's width only to decide what it draws.
*Persist:* `VisibleTimeStart` and `TimePerPixel` are copied to the sequence on every change and marked as editor state (`Form.cs:2368-2384`), saved with the `.tim`, restored on load when above zero (`Form.cs:1515-1523`).

**F7. Horizontal scroll bar.**
*Do:* drag it.
*Does:* canvas width is `(int)px(TotalTime)` (`Grid.cs:206`). Scrolling by the bar sets `VisibleTimeStart = time(scrollX)` so the bar snaps time to whole pixels (`Grid.cs:757-780`). Setting `VisibleTimeStart` moves the bar (`Grid.cs:177-181`). Changes of zoom or total time rerun the same mapping (`Grid.cs:183-214`).
*Rules:* the bar exists only while the canvas is wider than the viewport (`AutoScroll = true`, `Grid.cs:81`). The vertical bar in F8 likewise. A shown bar takes client space, which changes `VisibleTimeSpan`. Mac: an overlay `NSScroller` takes none, so the view is a few points wider than upstream's.
**FIX Q46:** upstream's re-clamp here tests `VisibleTimeEnd > TotalTime` (`Grid.cs:208`), which can never be true because the end is capped at the total. So shortening the sequence leaves the start past the end until the next scroll or resize. On the Mac run the start through the F6 clamp whenever zoom or total time changes, which is what the test was for.

**F8. Vertical scroll bar and vertical offset.**
*Do:* drag it, or wheel (F11).
*Does:* canvas height is the sum of visible row heights (`Grid.cs:2568-2578`, `Grid.cs:2589`). Setting the offset: below 0 becomes 0; above `canvasHeight - clientHeight` becomes that; unchanged does nothing; otherwise it scrolls and raises `VerticalOffsetChanged` (`Grid.cs:255-272`), which sets the row list's offset and forces an immediate repaint of the whole control (`TimelineControl.cs:852-864`). The same event also fires on every grid resize (`Grid.cs:792`). After any expand, collapse or height change, re-clamp the offset and re-sync the row list (`TimelineControl.cs:880-885`).
*Rules:* the two clamps run in that order, so when the rows are shorter than the viewport the upper limit is negative. Upstream passes the negative value on and the framework holds the scroll at 0, raising the event anyway. **GUARD:** clamp to `max(canvasHeight - clientHeight, 0)`, offset stays 0.

**F9. Vertical gridlines.**
*Does:* every `GridlineInterval` (fixed at 1 second, `Grid.cs:87`), a dashed line in `MajorGridlineColor` (black, `TimeLineGridColor`, `Grid.cs:86`) over the viewport height. First line is the first multiple of the interval strictly after the visible start, so none is drawn at exactly the left edge (`Grid.cs:2718-2734`).
*Note:* see Performance hazards for the sub-pixel case.

**F10. Grid paint order.**
*Does:* back to front: gridlines, row fills and separators, mark lines, effects, hover info, rubber band, draw box, edit cursor and playback line, resize indicator (`Grid.cs:3178-3189`). So gridlines lie under row highlights, mark lines lie over row highlights and under effects. Nothing is painted while the sequence is loading (`Grid.cs:3171`), from `SequenceLoading = true` at the start of a load until the element load finishes (`Form.cs:1454`, `Form.cs:1489`). The grid background is `TimeLineEffectsBackColor` (68,68,68) (`Grid.cs:88`, `app/Sources/VixenKit/Theme.swift`). A paint failure stops the pass at that point (F151).

### B. Wheel, trackpad and zoom

**F11. Wheel over grid, ruler, waveform, row list and the backdrop above it.**
*Does:* these route to the composite control's handler (`Grid_Mouse.cs:289-294` suppresses the grid's own scrolling). Checked in this order (`TimelineControl.cs:1144-1171`):

| Holding | Action |
|---|---|
| ⌘ and ⌥ | Row under pointer changes height (F32) |
| ⌘ and ⇧ | All rows change height: delta below 0 is scale 0.8, otherwise 1.25 (F31) |
| ⌘ | Zoom time: `scale = 1.0 - delta / 1200.0`; through F16 if "Zoom Under Mouse Position" is on, otherwise F15 (`TimelineControl.cs:1217-1228`) |
| ⇧ | Pan: `VisibleTimeStart += VisibleTimeSpan * -(delta / 1200.0)`, 10% per notch (`TimelineControl.cs:1206-1209`) |
| none, or ⌥ alone | Vertical: `VerticalOffset += -(delta / 3)`, integer division, 40 per notch |

⌘⌥⇧ matches the first row. A notch is delta 120. Positive delta is wheel away from the user, so it zooms in and pans left.
*Mac:* for a line-based wheel use `delta = scrollingDeltaY * 120` (lines). macOS turns a wheel with ⇧ held into horizontal deltas, so with ⇧ take whichever axis is non-zero. Sign is `scrollingDeltaY` directly, which respects natural scrolling. For precise deltas see F14.

**F12. Wheel over the marks bar.**
*Does:* only navigation (`TimelineControl.cs:1173-1177`, `TimelineControl.cs:1189-1199`). Holding ⇧ (with anything else) pans with the F11 formula. Holding ⌘ without ⌥ and without ⇧ zooms with the F11 formula, using the pointer translated to the composite's coordinates. Anything else does nothing: plain wheel does not scroll rows, ⌘⌥ does not change a row height (`Tests/MarksBarMouseWheelTests.cs:73-98`, `upstream/Vixen/docs/plans/sequencer/vix-3983-marks-bar-wheel-navigation-requirements.md` lines 13-16).
*Note:* WinForms may also forward an unhandled wheel to the parent, which cannot be confirmed from this source. Implement the written requirement above.

**F13. Horizontal wheel (tilt).**
*Does:* over the grid and marks bar only: `VisibleTimeStart += VisibleTimeSpan * (delta > 0 ? 0.10 : -0.10)`, regardless of size (`Grid_Mouse.cs:296-299`, `TimelineControl.cs:1179-1182`, `TimelineControl.cs:1211-1215`). Ruler and waveform have no handler. Mac: tilt arrives as line-based `scrollingDeltaX`. Handle it on all time surfaces, since the guide requires gestures to work everywhere.

**F14. Trackpad scroll (new, Mac only).**
*Do:* two-finger scroll anywhere over the timeline.
*Does:* precise deltas (`hasPreciseScrollingDeltas`). No modifier or ⌥ alone: vertical delta moves `VerticalOffset` one point per point; horizontal delta moves `VisibleTimeStart` by `time(delta)`. ⇧: either axis pans by `time(delta)`. ⌘, ⌘⇧, ⌘⌥: convert to a wheel delta with `delta = scrollingDeltaY * 10` and use the F11 formulas, applying row-height steps only when the accumulated delta passes 120 either way, because F31 and F32 scale per step. Momentum events are accepted; clamps stop them. Over the marks bar, apply F12's restrictions to the same mapping, except that plain horizontal scroll pans.

**F15. Zoom time.**
*Do:* menu View > Zoom Time In / Out, toolbar TIME buttons, ⌘= / ⌘-, numpad ⌘+ / ⌘-, or ⌘-wheel.
*Does:* menu and keys use scale 0.8 to zoom in and 1.25 to zoom out (`Menu.cs:301-309`, `Hotkeys.cs:199-211`); wheel uses `1 - delta/1200`. The menu's own shortcuts are the numpad keys, shown as "Ctrl+ +" and "Ctrl+ -" (`TSE/TimedSequenceEditorForm.Designer.cs:810-811`, `TSE/TimedSequenceEditorForm.Designer.cs:819-820`); the main-row keys come through the hotkey handler. Decision shared with F16 (`TimelineControl.cs:448-466`):
1. `scale <= 0`: rejected.
2. If `VisibleTimeSpan * scale > TotalTime`: new `TimePerPixel = TotalTime.ticks / grid.Width` (integer division), and it "shows total".
3. Otherwise new `TimePerPixel = TimePerPixel * scale` (`(long)(ticks * scale)`).
4. Rejected unless new ticks are above 2000.

If accepted (`TimelineControl.cs:396-412`): apply it; if showing total, `VisibleTimeStart = 0`; else if `VisibleTimeEnd >= TotalTime`, `VisibleTimeStart = TotalTime - VisibleTimeSpan`. Otherwise the left-edge time stays put.
*Rules:* **GUARD** `grid.Width` of 0: reject. **KEEP:** the full-zoom-out span can be a few ticks short of `TotalTime` because of the integer division (plan vix-4001 notes 9:59.9999148 for ten minutes).
*Default:* 10 ms per pixel. *Persist:* see F6.

**F16. Zoom under the pointer.**
*Do:* ⌘-wheel with View > "Zoom Under Mouse Position" on.
*Does:* same decision as F15. If rejected, nothing changes at all. If showing total, start 0. Otherwise (`TimelineControl.cs:423-446`), with `f = (pointerX - splitterDistance) / panel2Width` where pointerX is in composite coordinates: zoom in moves start later by `time((int)(px(oldSpan - newSpan) * f))`, zoom out moves it earlier by `time((int)(px(newSpan - oldSpan) * f))`, both using the new `TimePerPixel`. This holds the time under the pointer steady. **KEEP:** the fraction ignores the splitter's own width and uses the panel width including any scrollbar.
*Default:* option off. *Persist:* profile setting `ZoomUnderMousePosition` (`Form.cs:369`, `Menu.cs:321-324`).

**F17. Pinch to zoom (new, Mac only).**
*Do:* pinch over grid, ruler, waveform or marks bar.
*Does:* `magnify(with:)` gives `scale = 1 / (1 + magnification)` per event, applied through the F16 anchor always, whatever the menu option, because a pinch is expected to centre on the fingers. Limits are F15's. While the gesture runs, scale existing content with a layer transform and commit `TimePerPixel` at most every 100 ms and at gesture end (see Performance hazards). With ⇧ held, pinch scales all rows through F31 at each accumulated factor of 1.25 or 0.8.

### C. Rows

**F18. Row tree.**
*Does:* each display element node makes one row, recursively, depth first, with its children below it; a node with several parents makes several rows (`Form.cs:4194-4226`). Each row's tag is its node and `_elementNodeToRows` maps a node to all of its rows. Root rows are visible at creation; children take the parent's `TreeOpen` (`Form.cs:4200-4203`, `Row.cs:758-763`). New rows take the current default height (`Form.cs:4198`). On load, saved settings are applied to each row in order: `TreeOpen`, then `Height`, then `Visible` (`Form.cs:1364-1378`, F20, F39).

**F19. Expand and collapse.**
*Do:* click the expander.
*Does:* toggles `TreeOpen` and recomputes children's visibility (`RowLabel.cs:99-103`, `Row.cs:455-459`, `Row.cs:181-206`), then re-lays out rows and re-clamps the scroll (F8).
*Rules:* the expander exists only when `HasExpandableChildRows`: at least one direct child passes the visibility filter (`Row.cs:293`). No selection change. The handler is `OnMouseClick`, which WinForms does not raise for the second press of a double-click (it raises the double-click event, which `RowLabel` does not handle; framework behaviour, not visible in this source). So a fast double-click toggles once, and the same goes for a double-click on the label (F25). Mac: act only when `clickCount` is odd.
*Persist:* expanded rows are saved with the sequence (see Data and persistence).

**F20. Row visibility rules.**
*Does:* `Visible` stores a requested flag. Effective visible = `requested && filter(row)`. Children get `Visible = effective && TreeOpen`, cascading. Label visibility follows; the label height is re-synced when a row becomes visible (`Row.cs:232-240`, `Row.cs:295-313`). `TreeOpen` never sets children directly, so a hidden parent cannot leak "visible" onto a filtered child (`Row.cs:191-200`). Assigning a filter propagates to descendants, recomputes, and collapses the row if it no longer has an expandable child (`Row.cs:262-275`). A height set while invisible is stored without notifying anyone (`Row.cs:38-51`). Events are per row and do not reach other rows (`Tests/RowEventScopeTests.cs`).

**F21. Row label appearance.**
*Does:* background `TimeLineLabelBackColor` (68,68,68); inset 1 px border in `TimeLineGridColor` (black) on rectangle `(0, -1, width-1, height)` (`RowLabel.cs:186-218`). Icon area: `(0,0,ToggleTreeButtonWidth,h)` if the row has expandable children or is a root, otherwise half that width. Label area is the rest (`RowLabel.cs:200-210`). Labels are indented by `ParentDepth * ToggleTreeButtonWidth` and sized to the remaining width (`RowList.cs:189-197`). In the sequencer `ToggleTreeButtonWidth` is the icon width plus 8, so 24 (`TSE/TimedSequenceRowLabel.cs:22`).
Text: name, left aligned, vertically centred, no wrap, ellipsis, in a box 6 narrower than the label area (`RowLabel.cs:220-227`). Font size `min(12, (int)(height * 0.4))` (`RowLabel.cs:178-182`). Colour `TimeLineForeColor` (235) (`RowLabel.cs:188`).
Expander icon: plus or minus (`bullet_toggle_plus_16px`, `bullet_toggle_minus_16px`), centred in the icon area, with a yellow rectangle and "active" artwork (`..._active_16px`) when F23 applies (`TSE/TimedSequenceRowLabel.cs:14-17`, `TSE/TimedSequenceRowLabel.cs:28-50`). Mac: SF Symbol or a disclosure glyph, plus the yellow backing.
Tag dots: for each tag of the row's node with a display colour, sorted by `SortOrder`, an 8 pt circle spaced 3, starting 6 after the measured text, vertically centred (`TSE/TimedSequenceRowLabel.cs:55-94`).

**F22. Tree connector lines.**
*Does:* on the row list background, in dotted `DottedLineColor` (235): a horizontal stub from `((depth - 0.5) * inset, midY)` to `(depth * inset, midY)` for each indented row, and a vertical line up to the bottom of the nearest ancestor-level row when the branch ends (`RowList.cs:200-252`). **GUARD:** upstream's arrays hold 20 levels and overflow beyond that. No limit on the Mac.
Exact walk, over visible labels in flat order, with `top` starting at `-VerticalOffset`, `lastDepth` 0, `inset` the toggle width:
1. For `i` from `depth + 1` to `lastDepth`: if `needs[i]`, draw `bottoms[i]` up to `(bottoms[i].x, tops[i - 1])`, clear `needs[i]`.
2. If `depth > 0`: `start = ((int)((depth - 0.5) * inset), (int)(top + h * 0.5))`, draw to `(depth * inset, start.y)`, `bottoms[depth] = start`, `needs[depth] = true`.
3. `lastDepth = depth`, `top += h`, `tops[depth] = top`.

After the last label, for every `i >= 1` with `needs[i]`, draw the same vertical line. So the vertical runs from the last child's stub up to the bottom of the parent-level row above it. The gutter shows the panel's (40,40,40).

**F23. Highlight rows with effects.**
*Do:* View > "Highlight Rows With Effects".
*Does:* a global flag (`Grid.cs:283-287`, `RowLabel.cs:36`). When on: label text is `ActiveColor` (yellow) if the row has any effect (`RowLabel.cs:188`, `Row.cs:25`); the expander gets its active look if any descendant row has an effect (`Row.cs:92-95`, `TSE/TimedSequenceRowLabel.cs:32-48`). Adding or removing an effect refreshes the row's label and its ancestors' (`Row.cs:341-348`, `Row.cs:679`, `Row.cs:703`).
*Default:* off. *Persist:* profile `HighlightActiveElements` (`Form.cs:370`).

**F24. Show Hidden Rows.**
*Do:* toggle View > "Show Hidden Rows", tooltip "Show rows tagged Hidden for this session. Resets to off the next time a sequence is opened."
*Does:* host assigns a filter to every root: rows whose node has the built-in Hidden tag are hidden unless the toggle is on, then re-lays out and refreshes labels (`Form.cs:2302-2306`, `Form.cs:2345-2360`). Also reapplied after any tag change (`Form.cs:2312-2315`).
*Default:* off, reset at each load (`Form.cs:1382-1384`). *Persist:* none, session only.

**F25. Click a row label: select the row and its effects.**
*Do:* click the label (or the icon area of a row with no expander), plain, with ⌘ or with ⇧.
*Does:* a click in the resize bar is ignored: inside the label area with `y > labelHeight - 4`, which on whole points is the bottom 3 (`RowLabel.cs:83-89`, `RowLabel.cs:95-96`). Row: holding ⌘ toggles `Selected`, otherwise `Selected = true` (`Row.cs:476-486`). Then the grid (`Grid.cs:613-744`):
- Holding ⌘, or holding ⇧ with no selected rows: if the row is now selected, select its effects and make it the first selected row; if not, clear the first row when no row remains selected and deselect its effects.
- ⇧ otherwise: for rows from the first selected row to this one in flat order, either direction, each row visible at that moment becomes selected with its effects selected. If either index is missing, behave as the case above. Rows selected outside the range stay selected.
- No modifier: clear all selected effects, clear other selected rows, clear active rows, select this row's effects, make it the first selected row.
"Select its effects" with Drag Box Filter on assigns each effect `Selected = (type in filter)`, so non-matching effects in that row are deselected.
Selection changed is raised at the end, once (upstream raises it several times on the way, from the clear calls; subscribers only read the final state). The edit cursor does not move.
**KEEP Q37:** the first selected row is a plain field that `ClearSelectedRows` does not reset (`Grid.cs:324`, `Grid.cs:854-862`). After an effect click has cleared the row selection, ⇧-click on a row still ranges from the old first row. Since the row was set `Selected` just before, "⇧ with no selected rows" can't actually occur.
*Mac:* the row list is a custom view; see SwiftUI and AppKit structure.

**F26. Row highlight painting.**
*Does:* over the viewport width only: a selected row is filled with `SelectionColor` ARGB(100,40,100,160) and outlined in `SelectionBorder`; an active row gets the same fill with no outline; both applied means two fills. A separator in `RowSeparatorColor` (black) at `bottom - 1` (`Grid.cs:2688-2716`, `Grid.cs:85`, `Grid.cs:89`). The row label has no selected or active look of its own (`RowLabel.cs:184-235`), so these show only in the grid. **KEEP:** `SelectionBorder` is blue, but a CAD rubber band turns it green and it stays so until the next rubber-band update (`Grid.cs:1809`).

**F27. Active row.**
*Does:* a flag separate from selection, normally one row; "first with the flag" wins (`Row.cs:334`, `Grid.cs:329-332`). Set by clicks on effects and background (F46, F49, F54, F51). ⌘-clicks and a release that collapses a selection add the flag without clearing others (`Grid_Mouse.cs:146-162`, `Grid_Mouse.cs:259-260`), so two rows can show it. It takes part in two target rules, and they differ:
- Paste: `SelectedRow ?? ActiveRow ?? TopVisibleRow` (`Menu.cs:172`, `Form.cs:5331`), and the paste time is 0 if that row is selected, otherwise the edit cursor (`Menu.cs:173`).
- Edit > Add Effect: `ActiveRow ?? SelectedRow`, and nothing happens if neither exists (`Form.cs:1276-1281`).

`TopVisibleRow` is the row at canvas y = `VerticalOffset` (`Grid.cs:347-350`). `SelectedRow` is the first selected in flat order (`TimelineControl.cs:302-306`).

**F28. Arrow keys move the active row.**
*Do:* ↑ or ↓ with no modifiers.
*Does:* moves the active flag one visible row, no wrap, leaves selection alone, scrolls just enough to show it (top aligned above, bottom aligned below) (`Grid.cs:884-906`, `Grid.cs:908-922`, `Hotkeys.cs:96-110`). With no active row, or none visible, it does nothing and returns false. These keys are claimed before child controls see them (`Hotkeys.cs:313-331`).
*Tests:* `Tests/TimelineActiveRowNavigationTests.cs`.

**F29. Toggle selected rows.**
*Do:* T, or ⌘T for the whole subtree.
*Does:* for each selected row, or the active row if none is selected: toggle `TreeOpen`; with ⌘ set the row and all descendants to the opposite of the row's current state. Re-layout once at the end (`Grid.cs:2637-2671`, `Row.cs:208-218`, `Hotkeys.cs:212-214`). Only ⌘ is tested, so ⇧T and ⌥T behave as T. The key runs during a drag too.

**F30. Drag a row's height.**
*Do:* left-press in the resize bar of the label area (`y > labelHeight - 4`, F25; not the icon area) and drag. Pointer is `HSplit` over the bar and arrow elsewhere, updated on every move, up-down resize on the Mac.
*Does:* each move adds the y delta to the row height, floor 10 (`RowLabel.cs:109-119`, `RowLabel.cs:142-156`, `Row.cs:38-51`, `Row.cs:461-464`). The grid and row list relayout live (`Grid.cs:804-808`, `RowList.cs:77-80`). On release, every row with the same **name** takes that height (`RowLabel.cs:126-131`, `TimelineControl.cs:892-905`). **KEEP:** this matches by display name, not by node.
*Persist:* marks editor state changed (`Form.cs:2294-2300`); see Data and persistence for what is saved.

**F31. Zoom all rows.**
*Do:* View > Zoom Rows In / Out (1.25 / 0.8, `Menu.cs:311-319`), toolbar ROW buttons, ⌘⇧= / ⌘⇧- (`Hotkeys.cs:199-211`), numpad ⌘⇧+ / ⌘⇧- (the menu's own shortcuts, `TSE/TimedSequenceEditorForm.Designer.cs:841-842`, `TSE/TimedSequenceEditorForm.Designer.cs:851-852`), ⌘⇧-wheel.
*Does:* `rowHeight = (int)(rowHeight * scale)`; for each row, hidden ones too, skip it if `row.Height * scale > grid.Height`, otherwise `row.Height = (int)(row.Height * scale)` with floor 10. Handlers and grid resize are off during the loop, then one layout (`TimelineControl.cs:468-490`). **GUARD:** upstream calls `Rows.ElementAt(0)` afterwards and throws with no rows. `scale <= 0` is ignored.
**KEEP Q36:** the floor of 10 is on rows, not on `rowHeight`. Repeated zoom out takes `rowHeight` 32, 25, 20, 16, 12, 9, 7, 5, 4, 3, 2, 1, 0, while the rows stop at 10, and from 0 it can never grow again until F35 or a reload. A saved `DefaultRowHeight` of 0 means 32 at the next load.
*Persist:* `rowHeight` as `DefaultRowHeight`.

**F32. Zoom the row under the pointer.**
*Do:* ⌘⌥-wheel.
*Does:* scale 0.8 if delta is below 0, else 1.25, applied once to that row with `(int)` truncation and floor 10 (`TimelineControl.cs:1231-1255`). Comment states the intent "adjusts the row height under the cursor" (`TimelineControl.cs:1149`). **FIX Q2:** upstream's hit test starts at `ruler.Height + waveform.Height`, ignoring the marks bar and the vertical scroll, and keeps the last hit row in a field when nothing matches. Use the row at the pointer's canvas y; do nothing if none.

**F33. Row label context menu.**
*Do:* right-click a row label.
*Does:* if the pointer is in the label area, or in the icon area of a row with no expander, the row label click runs with the current modifiers, then the menu opens (`RowLabel.cs:132-139`). Item order: Collapse All Groups, Reset All Rows to Default Height, Reset Selected and Child rows to Default Height, separator, Tags (`TimelineControl.cs:912-935`).
*Note:* WinForms also raises its click event on right-button release, so the label click runs twice in upstream (`RowLabel.cs:91-107` then `RowLabel.cs:121-140`), and right-clicking an expander also toggles it (framework behaviour, not visible in this source). A plain right-click therefore collapses a multi-selection to the clicked row, ⇧-right-click extends the range, and ⌘-right-click toggles twice with no net change, which keeps the selection (**KEEP**, and see the open question on Finder-style behaviour in the register).

**F34. Collapse All Groups.**
*Does:* every row with `TreeOpen` closes (`TimelineControl.cs:1110-1120`). Same action as View > "Collapse All Element Groups" (`Menu.cs:331-334`).

**F35. Reset All Rows to Default Height.**
*Does:* `rowHeight = 32`, every row set to it, one layout (`TimelineControl.cs:1091-1108`). Same as View > "Reset Row Height to Default" (`Menu.cs:326-329`). Same empty-rows **GUARD** as F31.

**F36. Reset Selected and Child rows to Default Height.**
*Does:* first selected row and all descendants set to the current `rowHeight`, which F31 may have changed (`TimelineControl.cs:1122-1141`). **GUARD:** no selected row throws upstream (right-click on an expander). Disable the item then.

**F37. Tags submenu.**
*Do:* right-click a row label, Tags.
*Does:* hint line "Ctrl+Click to cascade tag" (Mac "⌘-Click to cascade tag"), separator, one item per tag definition with a checkmark state and a colour dot at the right if the tag has a colour, separator, "Manage Tags..." (`TimelineControl.cs:942-985`, `TimelineControl.cs:1003-1010`). State over the selected rows' nodes: none tagged unchecked, all tagged checked, otherwise mixed (`TimelineControl.cs:1012-1024`). Click: if checked, remove from the targets; otherwise add. Holding ⌘: targets include all descendants of each selected node (`TimelineControl.cs:970-971`, `TimelineControl.cs:1034-1075`). Refreshes affected labels, raises tags changed (F24), and saves system config.
*Rules:* Tags is enabled only when at least one selected row has a node (`TimelineControl.cs:960-961`). Mac: use `NSMenuItem` state `.mixed` for the third state, and read `NSEvent.modifierFlags` in the action for ⌘.
*Persist:* system config, immediately. *Undo:* none.

**F38. Manage Tags...**
*Does:* asks the host to open the element tag manager; if it saved, row labels refresh (`TimelineControl.cs:982-983`, `Form.cs:2320-2326`). The window belongs to another spec.

### D. Effects: geometry and look

**F39. Effect rectangle.**
*Does:* computed per occurrence (`Grid.cs:2956-3003`), with row height `H`:
- `h = stackCount != 0 ? (H - 1) / stackCount : H - 1` (integer).
- `y = rowTop + h * stackIndex`, `rowTopOffset = h * stackIndex`.
- If `h == 0`, too many lanes: red border, and `h = H`. The y values stay as computed with 0. Upstream reads `H` here from the effect's last row rather than the painted row (`Grid.cs:2972`); part of **FIX Q3**, use the painted row.
- Clipped width: start at or after visible start and end before visible end: `(int)px(duration)`. Start at or after visible start otherwise: `(int)(px(visEnd) - px(start))`. Start before visible start and end at or before visible end: `(int)(px(end) - px(visStart))`. Otherwise `(int)(px(visEnd) - px(visStart))`.
- Width at or below 0: not drawn, and `DisplayRect` keeps its previous value (`Grid.cs:2997`).
- `x = floor(px(max(start, visStart)))`.
- Whole-effect width passed to the thumbnail: `(int)px(duration)`.

Effects are drawn when `end >= visStart`, stopping at the first with `start > visEnd`, in row order (`Grid.cs:3054-3077`).
**FIX Q3:** upstream writes these values onto the shared `Element` during paint, and hit testing, rubber band and tooltips read them back. Compute them in a pure per-row layout function, for rows on demand, and let paint and hit testing both read that. The numbers must match the formulas above.

**F40. Stacking overlapping effects.**
*Does:* per row, for the visible time range, before drawing (`Row.cs:509-538`):
1. Walk the row's sorted list. Skip effects with `end < rangeStart`; stop at the first with `start > rangeEnd`.
2. Build a cluster from effect `m` (`Row.cs:560-597`): forward from `m`, add while `next.start < coverEnd`, where `coverEnd` starts as `m.end` and grows to the max end seen, stopping at the first failure. Backward from `m` through all earlier effects with no early stop, insert at the front if `prev.end > coverStart`, where `coverStart` starts as `m.start` and shrinks. Backward additions do not extend `coverEnd`.
3. One effect: index 0, count 1. Otherwise assign lanes first-fit in cluster order (`Row.cs:540-558`): place in the first lane whose last effect's end is at or before this start, else open a new lane. Everyone gets `count = lanes`, `index = lane`.
4. Advance `i` past the cluster: `i += cluster.Count - cluster.IndexOf(m) - 1`.

Row order is by start then end (`Element.cs:307-314`) and is re-sorted whenever a time changes (`Row.cs:439-443`). `List.Sort` is unstable, so upstream's order among effects with identical start and end is arbitrary and can change on any re-sort, which changes their lanes. Use a stable sort, as `docs/PORTING_GUIDE.md` says; the lane order of such twins is unspecified either way.
**KEEP Q4:** because the walk starts at the first visible effect, an effect's count can change as the user scrolls. Example: A[0,10], B[5,6], C[5.5,8], D[9,20]. With the whole range visible D is index 1 of 3. With visible start 15 the cluster is [A,D], so D is index 1 of 2 and taller. Pass the visible range to the pure function. Scan optimisations may not change results.

**F41. The same effect in several rows.**
*Does:* one effect object per effect node, added to every row for each target node (`Form.cs:4059-4080`). Selecting, moving or resizing it shows in all its rows, and selection and time are shared. Row lists stay sorted through a per-row handler (`Row.cs:439-443`).
**FIX Q3 (continued):** stack index and count are per occurrence. Upstream stores them on the effect, so they hold whatever the last painted row wrote, and hit testing a row uses another row's height and offset whenever the rows differ (`Element.cs:103-106`, `Grid.cs:3042-3044`). On the Mac paint and hit test both use the occurrence's values, so what the user sees is what they hit.
*Rules:* `Row.ContainsElement` and `RowContainingElement` semantics stay: the first row in flat order containing the effect (`Grid.cs:1686-1689`).

**F42. Effect appearance.**
*Does:* content is the thumbnail for the visible slice, from `vixen_session::thumbnail::rasterize`, with visible offsets relative to the effect start, except that when the effect ends inside the view the end offset is upstream's absolute `EndTime`, which only means "no clipping" (`Element.cs:388-416`, `TSE/TimedSequenceElement.cs:18-22`).
Border inside the rectangle `(0,0,w-1,h-1)`: top and bottom always; left only if the effect's start is visible, right only if its end is visible (`Element.cs:320-360`, `Element.cs:427`). 1 pt black. Selected: 3 pt black plus 1 pt dashed `[1,2]` in `ForeColor` (235). Red instead of black when lanes do not fit.
During a resize the existing picture is stretched to the new width, because the effect is not re-rendered until release (`Element.cs:410-415`). Exactly: while an update is open, both visible offsets are multiplied by `originalDuration / currentDuration` before rasterizing, so the same slice of the old content maps onto the new width. During a plain move that factor is 1.

**F43. Placeholder and background render.**
*Does:* an effect whose render is dirty draws gray (122,122,122) with both side borders (`Element.cs:439-455`, `Element.cs:542-547`). Rendering is queued: when the queue is empty a request is enqueued, otherwise it runs immediately on a task (`Grid.cs:2890-2907`). Workers call `PreRender`, drop the cached picture and repaint if the effect is in view (`Element.cs:366-374`, `Grid.cs:2815-2884`). Progress percent is `processed / queued * 100` and drives the status bar's "Rendering Elements:" label and progress control, hidden at 100 (`Grid.cs:2862-2863`, `Form.cs:2152-2176`, sequencer spec F261). A render that throws is logged and reports 100, which hides the bar even if more are queued (`Grid.cs:2866-2870`). At load, invalidation and rendering are held off, then all unrendered effects are queued (`Form.cs:1478-1479`, `Form.cs:1492-1494`, `Grid.cs:2913-2952`); re-queueing everything first empties the queue, zeroes both counters and reports 100 (`Grid.cs:2939-2952`). A plain time move does not dirty an effect but a duration change does (`TSE/TimedSequenceElement.cs:24-40`; dirtying lives in the effect module).

**F44. Hit testing.**
*Does:* row at canvas y: walk visible rows accumulating heights; first row with `y < accumulated + height`. Negative y gives the first row, below the end gives none (`Grid.cs:1562-1575`). Effects at a point (`Grid.cs:1659-1684`): in that row's order, stop at the first with `px(start) > x`; hit if `px(start) <= x <= px(start) + px(duration)` and `rowTop + rowTopOffset <= y < rowTop + rowTopOffset + h`. Topmost single hit (`Grid.cs:1627-1652`): same test, first match, no early stop. X is not clipped to the view.
Public helpers for the host, from screen point: element, row, grid point, time (`Grid.cs:1597-1620`). `ElementsAtTime`: visible rows, `start <= t < end`, not distinct (`Grid.cs:1726-1737`). `ElementsWithinRange`: start in the inclusive window, distinct (`Grid.cs:1697-1719`).

**F45. Hover effect info.**
*Do:* hover with View > "Show Effect Info" on.
*Does:* the set of effects under the pointer is tracked on every move. When it changes, flags update and the grid repaints (`Grid_Mouse.cs:320-332`, `Grid_Mouse.cs:336`). For the first effect, drawn at `(effectX, rowTop, effectWidth, rowH)` of the row under the pointer (`Grid.cs:3035-3052`). Text, in this exact form: `{EffectName} \r\n Layer: {layer} \r\n Start: {m:ss.fff} \r\n End: {m:ss.fff} \r\n Length: {m:ss.fff}` (lines after the first start with a space) (`Element.cs:467-472`). Font: system, 7 pt, white on ARGB(128,0,0,0). Position: box is text size plus 2. If `rect.y - boxHeight` would be above the visible clip, the box goes below (`y += rectHeight + 2 - 8`), otherwise above (`y -= textHeight + 2 - 4`). If its x is left of the clip, `x = clipX + 5` (`Element.cs:474-500`). The dark fill is that box offset by (1,1) and 1 larger each way; the text is drawn in the box itself.
The set is tracked in every drag state, on the raw pointer location before the F59 freezes, so the info shows on whatever the pointer crosses during a drag. The layer name comes from the sequence's layer manager (`Grid.cs:3048`).
**KEEP Q33:** there is no mouse-leave handling on the grid. When the pointer leaves, the box stays where it was until the pointer comes back and the set changes, and it can name an effect that has since been deleted (**GUARD:** drop a tracked effect that no longer exists).
*Default:* on. *Persist:* profile `ShowEffectInfo` (`Form.cs:375-376`, `Form.cs:5836`). Menu shortcut ⌘I (`TSE/TimedSequenceEditorForm.Designer.cs:890`).

### E. Selecting effects

**F46. Left or right press on the grid.** Exact order (`Grid_Mouse.cs:44-197`). On every press, record the canvas point, the row there, and the effects there.
1. Middle: F78.
2. Left or right only from here. Working effect = topmost hit.
3. Draw start: if Draw Mode is on and ⌥ is not held and an effect type is selected and the pointer is not in a resize zone (F77).
4. Left press on background (no effects there): if ⌘ is not held, start the rubber band and clear the shift anchor (F51). With ⌘ held nothing happens at all.
5. Otherwise, press on effect(s), or right press anywhere:
   - **Some effect there is already selected:** holding ⌘ deselects those. Otherwise holding ⇧ with an anchor selects the rectangle (F48). Otherwise no change, so a multi-selection can be dragged.
   - **None selected there:** holding ⇧ with an anchor selects the rectangle. Otherwise: if ⌘ is not held, clear selected effects, selected rows and active rows, with selection events suppressed. Then offer the effects to the host (F50). If the host accepts: select them all, anchor = this point, row here becomes active, raise selection changed, move the cursor (`⌘ ? last action effect : earliest selected`, F92). If the host declined and ⌘ is not held, raise selection changed.
   - Then, left button only: if no resize zone, wait for a drag (F58). If in a zone and ⌘ is not held: if ⌥ is held and more than one effect is selected, show the warning "Too many effects selected.\nMaximum selected effects for this action is 1", titled "Warning", OK only, and stop, which leaves the state normal, so release takes the F49 path; otherwise begin a resize (F66). In a zone with ⌘ held, nothing starts.
"No anchor" means the anchor is (0,0): upstream uses `Point.Empty` as the marker (`Grid_Mouse.cs:106`, `Grid_Mouse.cs:137`), so a press at canvas (0,0) leaves none. Use an `Option` on the Mac; the difference is one point.
*Note:* on the Mac recompute the resize zone at press as well as on hover, since a tap can arrive without a move. The zone is otherwise whatever the last hover computed (`Grid_Mouse.cs:461`, `Grid_Mouse.cs:465-490`).

**F47. ⌘-click.** Covered by F46 and F49: toggles the clicked effects into or out of the selection. Deselecting does not move the cursor at press, but release (F49) does.

**F48. ⇧-click range.** Selecting "between" clears selected rows and active rows, then selects with the normalised rectangle from the anchor to the pointer, same geometry as F51 and always "crossing" (`Grid.cs:1744-1780`, `Grid.cs:1796`). It replaces the selection: the saved list only re-adds during a rubber band. The anchor is set by a single selection, a drag release, or a collapse (`Grid_Mouse.cs:160`, `Grid_Mouse.cs:257`, `Grid_Mouse.cs:905`), and cleared by a background press or a draw start.

**F49. Left release without a drag.** (`Grid_Mouse.cs:247-272`). First, always, alignment guides turn off (`Grid_Mouse.cs:204`), and the working effect is cleared.
- Effects were under the press, and neither ⌘ nor ⇧ is held now: clear all selected, offer the effects to the host; if accepted, select only the first, anchor = release point, selection changed, row at release becomes active, cursor to that effect's start. This collapses a multi-selection to the clicked effect. Active rows are not cleared in this path, so if the press landed on an already selected effect the old active row keeps its highlight beside the new one.
- Effects were under the press and ⇧ or ⌘ is held now: clear active rows, row at release becomes active, cursor to the last effect pressed. This runs after a ⌘-click that deselected the effect too.

The modifiers are read at release, not press. A resize-zone press does not come here, so clicking an edge never collapses a selection (F66).

**F50. Choosing between overlapping effects.**
*Does:* when more than one effect is under the press, the host cancels automatic selection and pops up a menu at the pointer with one item per effect: `<TypeName> (m:ss.fff)`. Choosing one selects it (`Form.cs:2824-2858`). Exactly one effect is selected automatically. On the Mac an `NSMenu` built from the returned list.
Choosing goes through `SelectElement`, so it moves the cursor (F92) but sets no anchor and no active row. Without ⌘ the selection was already cleared at press. Dismissing the menu leaves it empty.
*Rules:* **FIX Q47:** three things to get right on the Mac, because `NSMenu` runs its own tracking loop and swallows the up event:
- The press path goes on to "wait for a drag" after the menu opens, and the release path offers the list a second time (F49). When the menu returns, end the press in the normal state and do not run the release path, so no second menu and no stranded drag.
- A right press offers the list too, and the release then opens the context menu over it (F54), which closes it (framework behaviour, not visible in this source). So for a right press skip this menu: leave the selection cleared and go to the context menu with the same effects.
- With ⌘ held and a chooser pick, the pick adds to the selection, as `SelectElement` does not clear.

**F51. Rubber band.**
*Do:* left-drag from background, without ⌘.
*Does:*
- Start (`Grid_Mouse.cs:629-641`): unless ⇧ is held, clear selected effects. Otherwise remember the current selection. Clear selected rows except the row under the press. In legacy mode (F93) clear active rows except that row.
- Each move (`Grid_Mouse.cs:645-656`, `Grid.cs:1796-1865`): rectangle from start to pointer. If its size is exactly zero both ways, stop. Suppress events, clear selected effects, re-add the remembered list if ⇧ is held. For each row touching the rectangle (`Grid.cs:1577-1595`, which may include one extra row when a boundary lands exactly on a row edge): for each effect in order, stop at `start > selEnd`, and set `Selected` to the test below. `selStart = time(rect.left)`, `selEnd = time(rect.right)`. `top = rowTop + rowTopOffset`, `bottom = top + h`.
  - Crossing: `start < selEnd && end > selStart && top < selBottom && bottom > rect.top`.
  - Enclosed (F52): `start > selStart && end < selEnd && top > rect.top && bottom < selBottom`.
  - Holding ⇧ and in the remembered list always selects. Drag Box Filter also requires the type to be in the filter (F53).
  - Selection changed once per move. **KEEP:** an effect in two rows inside the rectangle takes the last row's answer.
- Paint: fill `SelectionColor`, 1 pt `SelectionBorder` (`Grid.cs:3079-3092`).
- Release (`Grid_Mouse.cs:658-683`): width and height both under 2 is a background click: in non-legacy mode clear active rows except the press row, then row at release becomes active and the cursor moves to the release time (F92). Otherwise, non-legacy: press row becomes active and the cursor goes to the earliest selected start. Then clear the rectangle and the remembered list.

*Rules:* edge auto-scroll, F63.

**F52. CAD Style Selection Box.**
*Do:* toggle Edit > "CAD Style Selection Box" (`TSE/TimedSequenceEditorForm.Designer.cs:561-578`; upstream has no Options menu).
*Does:* when on, dragging rightwards (rectangle left edge not before the press) is "enclosed" with a green border; leftwards is "crossing" with a blue border. When off, always crossing and blue (`Grid.cs:1803-1809`).
*Default:* off. *Persist:* profile `CadStyleSelectionBox` (`Form.cs:366`, `Menu.cs:282-285`).

**F53. Drag Box Filter.**
*Do:* toggle the toolbar button and tick effect types in its dropdown.
*Does:* rubber band and row selection pick only listed types (`Grid.cs:368-369`, `Grid.cs:626-632`, `Grid.cs:1834-1846`). Host: `Form.cs:1234-1263`, `Toolstrip.cs:1833-1835`. The dropdown is sequencer spec F123: "Invert Selection", then one checkable item per effect type; ticking any type also turns the filter button on. The list starts empty, so turning the button on with nothing ticked makes rubber bands select nothing new. ⇧-click ranges (F48) test the filter too, since they share the selection routine.
*Default:* off. *Persist:* session only.

**F54. Right press and release on the grid.**
*Does:* press acts as F46 step 5 with an empty or real effect list, so right-clicking blank space clears the selection, and right-clicking an unselected effect replaces the selection with it. Right-clicking inside a multi-selection keeps it. On blank space that path still runs its "accepted" branch with no effects (`Grid_Mouse.cs:154-165`): the anchor moves to this point and the row here becomes active. ⌘-right-click on blank keeps the selection, and ⇧-right-click with an anchor selects the rectangle. Release (`Grid_Mouse.cs:274-284`): clear active rows; if there is no row at the pointer, stop; that row becomes active; if clicking the grid sets the cursor (always, `Grid.cs:100`) and (legacy mode, or no effects were under the press), the cursor moves to the pointer time; raise context selected with the effects under the press, the time, and the row. The host builds and shows the effect context menu (`ContextMenu.cs:56`, shown at `ContextMenu.cs:489`). That menu's items and rules are in the sequencer spec: Add Effect(s), Layer, Alignment, Manipulation (Divide at cursor, Clone, Clone to selected effects), Edit Time, Cut, Copy, Paste, Paste Special, Delete Effect(s), Add Marks to Effects, Collections.
*Rules:* the time given to the menu is the **release** point's, not the press's. **FIX Q5:** with Draw Mode on, a right press starts a draw box (step 3) and nothing ever ends it, because only left and middle releases do, so the box follows the pointer with no button held. On the Mac a right press in Draw Mode goes straight to the menu path: it runs step 5 and the release above as if Draw Mode were off.
Also the host clears the mark selection on any non-right press on the grid (`Form.cs:2187-2195`).

**F55. Double-click an effect.**
*Does:* raises an element-double-clicked event (`Grid_Mouse.cs:301-311`). Nothing in the editor subscribes, so nothing further happens. The two presses behave as F46.

**F56. Select all effects.**
*Do:* ⌘A, or Edit > "Select All Elements".
*Does:* every row, hidden or not, selects all of its effects (`Menu.cs:198-201`, `TimelineControl.cs:653-658`). The cursor does not move. **FIX Q6:** upstream has a selected-list cache that only `_SelectionChanged` clears (`Grid.cs:300-312`, `Grid.cs:450-456`), and Select All never calls it, so afterwards commands act on the stale list. On the Mac selection state is always authoritative and this path raises selection changed.

**F57. Selected list order.**
*Does:* `SelectedElements` is rows top to bottom, within a row in the order selected, distinct (`Grid.cs:301-312`, `Row.cs:445-453`). Order matters: `First()` picks the primary for adjoining resize (F67), and distribute reads first and last. Keep it. *Consumers:* copy, cut and delete enabling (`Form.cs:2180-2185`), the effect editor panel.

### F. Moving effects

**F58. Drag to move.**
*Do:* press on an effect (not in a resize zone) and drag.
*Does:*
- Waiting: ignore until the pointer leaves the rectangle `(x-8, y-8, 16, 16)`, left and top inclusive, right and bottom exclusive (`Grid_Mouse.cs:701-708`, `Grid.cs:97`, `Grid_Mouse.cs:405-425`).
- Begin (`Grid_Mouse.cs:710-719`): pointer becomes move (`SizeAll`), the row index under the pointer is recorded, snap points are built (F69), each selected effect's start, duration and last row are saved along with the visible start and the current location. Motion before crossing the threshold does not count, so the effect does not jump.
- Each move (`Grid_Mouse.cs:858-901`): `dt = time(x - x0)`, `dy = y - y0`; both zero stops. Clamp: `dt >= -earliestOriginalStart`, `dt <= TotalTime - latestOriginalEnd`. Then snap (F71). Note the order: **KEEP Q7:** snap comes after the clamp, so a snap can push a selection slightly out of bounds; start is then clamped at 0 by the setter (`Element.cs:156-157`). Each effect's start = original start + dt; its row re-sorts. If `dy != 0`, F61.
- The effect node is not updated during the drag (`Element.cs:53-60`, `TSE/TimedSequenceElement.cs:24-40`). The per-move time change notification still reaches the host, so the sequence is marked modified from the first move (`Element.cs:80-90`, `Form.cs:2204-2208`).
- Release (`Grid_Mouse.cs:903-909`, `Grid_Mouse.cs:532-558`): anchor = release point; each effect commits and is queued for render; finished-moving is raised; an undo entry is recorded; the snap snapshot is cleared.
*Rules:* with nothing selected every move stops at once (`Grid_Mouse.cs:861-862`). That state is reachable: ⌘-press deselects the only selected effect (F46), then the ⌘ key is released before the threshold.
*Undo:* "Moved N effect(s)", holding all original times and rows (`TSE/Undo/ElementsTimeChangedUndoAction.cs:62-63`). Recorded even if nothing changed net, and even with no effects at all ("Moved 0 effects"). **KEEP Q35.**

**F59. Drag modifiers.** Checked on every move, exact matches, and only if effects were under the most recent press (`Grid_Mouse.cs:377-389`):

| Exactly | Effect |
|---|---|
| ⇧, or ⌘⇧, or ⌥⌘ | X frozen at the previous event's X: vertical only |
| ⇧⌥ | Y frozen at the previous event's Y: horizontal only |

Freezing replaces the coordinate with the last value, so it keeps the offset reached so far. **KEEP Q8:** it applies in every state, because the press list persists after release. So ⇧ held during a resize stops the edge, and ⇧ while hovering uses a stale X for the resize zone and pointer.

**F60. Clone by dragging.**
*Do:* hold ⌘ at the moment the drag threshold is crossed.
*Does:* effects at the press point become selected if they were not; a ⌘-press that deselected them at press re-selects them here. The host clones every selected effect: same type, cloned data, same time, same last row, same layer, added to the sequence and timeline with an undo entry (`Form.cs:3568-3607`). Selection moves to the clones, and the drag moves them while the originals stay (`Grid_Mouse.cs:408-422`, `Grid.cs:828-837`). A clone is built on the last row's node only, so a clone of an effect with several target nodes has one target.
*Undo:* two entries: added effects, then the move. ⌘⇧ clones and constrains to vertical (F59).

**F61. Moving between rows.**
*Does:* on each move with `dy != 0` (`Grid.cs:2177-2306`). `R` is the row under the pointer, `cur` the tracked row, `A` the anchor row (the press row, tracked).
1. No row there, or same flat index as `cur`: stop. Destination Deprecated: stop (F62).
2. `n = visibleIndex(R) - visibleIndex(cur)`.
3. Find the top and bottom limit rows among the visible rows, walking from each end (`Grid.cs:1934-1983`). For each row and each moving effect in it: if the row is `A`, that row is the limit. Otherwise decrement the effect's remaining occurrences (from `Grid.cs:1907-1921`); at or below 0 this row is the limit. So duplicates in other rows do not limit movement.
4. Clip `n` to `[-topIndex, count - 1 - bottomIndex]`. Zero stops.
5. Effects in `A` move by their occurrence in `A` only. Walk visible rows top down; for each effect not yet moved and (not in `A` or this row is `A`) and in this row: decrement remaining; if the target would be off the grid, skip this occurrence and try the next (**GUARD Q9:** upstream throws when none remain, skip instead); otherwise remove from this row, add to `row[i + n]`, mark moved, raise changed-rows. If this occurrence was the anchor's, the anchor becomes the destination.
6. `cur` advances by `n`, then the anchor is updated.

*Rules:* **GUARD Q38:** the tracked row is recorded where the threshold is crossed (F58), so it can be "none" if that is below the last row. Moving back onto a row then indexes the row list at -1 and throws in the move handler (`Grid_Mouse.cs:715`, `Grid.cs:2198`). On the Mac, with no tracked row, set it to the destination row and move nothing this time. **GUARD Q39:** if no moving effect is in any visible row, both limit rows are "none", index -1, and step 4 turns any upward `n` into `+1` (`Grid.cs:2205-2211`). Move nothing instead.
Host on changed-rows (`Form.cs:2220-2251`): retarget the effect to the new node (taking effect at commit, `Element.cs:69-72`), remove it from all other rows of the old node, add it to all other rows of the new node. So an effect on a node with two rows shows in both. The new target list is that one node alone, whatever it held before. Sequence marked modified.
*Tests:* `Tests/GridCloneDragDuplicateRowsTests.cs`.
*Undo:* part of the move entry; undo swaps rows back and re-raises changed-rows (F88).

**F62. Deprecated rows refuse effects.**
*Does:* dragging into a row whose node has the built-in Deprecated tag does nothing: effect, anchor and tracked row stay put (`Grid.cs:2187-2194`, `Tests/GridCloneDragDuplicateRowsTests.cs:87-124`). New effect drops are refused too (F89).

**F63. Edge auto-scroll while dragging.**
*Does:* in Moving, Selecting, Drawing or Resizing, not Waiting. Margin 24 on all edges (`Grid.cs:83`). `outside.x = x <= 24 ? -(24 - x) : x > W - 24 ? x - W + 24 : 0`, the same for y (`Grid_Mouse.cs:345-355`). Non-zero starts a 50 ms repeating tick; zero stops it (`Grid_Mouse.cs:357-364`, `Grid_Mouse.cs:1173-1178`). Each tick: scroll by `outside / 4` with integer division, clamped to `[0, canvas size]`; sync view and row list; then replay the last pointer event so the drag updates under the new scroll (`Grid_Mouse.cs:1180-1197`). The rate grows with distance outside the view, with no upper limit.

**F64. Esc during a drag.**
*Do:* press Esc with the grid focused.
*Does:* applies to a move or a resize, including its adjoining effects, because those have saved times. A rubber band or draw box has none, so Esc does nothing to it. Restores the visible start and every saved start and duration, then ends the drag (`Grid.cs:747-755`, `Grid_Mouse.cs:561-575`). Releasing the button afterwards runs the no-drag path, so selection collapses (F49). The editor's own Esc handling also turns off Draw Mode and clears the effect tree selection, unless drawing (`Hotkeys.cs:190-197`). That runs first, because the form sees keys first, and on Windows the key still reaches the grid.
**FIX Q10:** upstream does not restore rows, does not end the elements' update state, does not clear the snap snapshot or the move record. An effect moved between rows stays displayed in the new row while the effect still targets the old node, and pressing Esc again restores stale times. On the Mac cancel restores times and rows, clears all drag state, and records no undo entry. **KEEP:** clones made by this drag remain, with their undo entry.

### G. Resizing effects

**F65. Resize zones and pointer.**
*Does:* on hover, using the topmost effect under the pointer (`Grid_Mouse.cs:465-490`): `grab = min(12, (int)(px(duration) / 2))`. Front if `px(start) <= x < px(start) + grab`. Back if `px(end) - grab < x <= px(end)`. Otherwise none. An effect under 2 pixels wide has `grab` 0 and no zone. Front wins where the two meet. Pointer: left-right resize in a zone; otherwise crosshair if Draw Mode is on, ⌥ is not held and an effect type is selected; otherwise arrow (all states: F150). The zone is the same for all stacks; y only decides which effect.

**F66. Resize.**
*Do:* press in a zone and drag, ⌘ not held.
*Does:* begins at the press, with no threshold (F58's 8-point box does not apply): tracked row, snap snapshot, saved state and an empty adjoining list (`Grid_Mouse.cs:917-925`). Vertical motion is ignored and rows never change. Each move (`Grid_Mouse.cs:943-1148`):
1. `dt = time(x - x0)`. Zero stops. ⌥ held with more than one selected stops.
2. Snap first (F71, F73).
3. Then bounds, which override snap. `shortest` = shortest original duration among selected. `min = time(10)` (10 pixels at the current zoom, `Grid_Mouse.cs:915`).
   - Front: `earliest + dt >= 0`, else `dt = -earliest`. If `shortest - dt < min`, `dt = shortest - min`.
   - Back: `latest + dt <= TotalTime`, else `dt = TotalTime - latest`. If `shortest + dt < min`, `dt = min - shortest`.
4. Apply to all selected: Front: start = orig start + dt, end = orig end. Back: end = orig end + dt.

The minimum width is checked after the sequence bounds, so it wins: an effect narrower than 10 pixels widens to 10 on the first move, even past the sequence end (the start setter still stops at 0, `Element.cs:156-157`). `time(10)` is also the public `MinimumElementDuration` that the Edit Time dialog is given (`Grid_Mouse.cs:22-25`, `ContextMenu.cs:370`).
Release: commit, queue render, one undo entry (`Grid_Mouse.cs:1150-1156`).
*Rules:* all selected effects resize on the same edge by the same amount. **KEEP Q34:** a press and release in a zone with no movement still runs the release, so a plain click on an effect's edge records a "Resize N effect(s)" undo entry, queues a render, and does not collapse a multi-selection.
*Undo:* "Resize N effect(s)".

**F67. Adjoining resize.**
*Do:* hold ⌥ during a resize of exactly one effect. Add ⇧ to limit it to effects on the same layer.
*Does:* on every move while the neighbour list is empty, so it keeps looking until it finds one, using the primary's current times as left by the previous move (`Grid_Mouse.cs:988-1020`, `Grid_Mouse.cs:1039-1072`), from the effect's last row:
- Front: walk back from a start index for at most 20 entries, keeping effects whose end is at or before this start (`Row.cs:605-620`). The start index is the entry before the first one after the primary with a later end than it (`Row.cs:727-740`), so the window includes the primary and any short effects after it and can hold fewer than 20 real candidates. Take those with the latest end, with ⇧ filtering by layer level.
- Back: walk forward from just after the primary for at most 20 entries, keeping effects that start at or after this end (`Row.cs:628-645`). Take those with the earliest start, ⇧ filtering likewise.
The ⇧ test compares against the primary's layer level (`Grid_Mouse.cs:967`). An effect already held by another update is skipped (`Grid_Mouse.cs:1013`, `Grid_Mouse.cs:1065`).
Their state is saved with the drag, so undo restores them.
Each move while ⌥ is held (`Grid_Mouse.cs:1099-1145`):
- Front: neighbour's end follows the primary's start. If the neighbour would be at or below 10 pixels: neighbour's end = neighbour's start + `time(10)`, primary's start = that, and primary's end stays at its original.
- Back: neighbour's start follows the primary's end, with its end held. At or below 10 pixels: neighbour's start = saved end - `time(10)`, its duration = `time(10)`, primary's end = that start.
The neighbour is queued for render during the drag. Releasing ⌥ stops neighbours following; they keep their state and commit at release. **KEEP.**
*Note:* since ⇧ alone would freeze X (F59), the same-layer form is ⌥⇧, which freezes Y instead.

### H. Snapping

**F68. Snap on and off, and strength.**
*Do:* toolbar "Snap To" (tooltip "Snap To Marks / Effect"), Edit > "Snap To Marks / Effects", strength dropdown (tooltip "Snap Strength") 1, 2, 3, 4.
*Does:* enable flag (`Grid.cs:231`, `Menu.cs:203-207`, `Toolstrip.cs:1846`). Strength rebuilds mark snap points (`Grid.cs:233-241`, `Toolstrip.cs:1850-1860`). Off: no snap points are built at drag start and the offset is returned as is (`Grid_Mouse.cs:723`, `Grid.cs:2048`). Turning it on mid-drag finds no points.
*Default:* on, strength 2. *Persist:* profile `SnapToSelected`, `SnapStrength` (`Form.cs:356-358`, `Form.cs:5814`, `Form.cs:5820`).

**F69. What can be snapped to.**
*Does:* snapshot at drag start (`Grid_Mouse.cs:721-773`):
- Every non-selected effect in every visible row: start and end, each with level 5 (`SnapPriorityForElements`, `Grid.cs:99`), its row as scope, and which edge it is. Clones snap to their originals, since originals are not selected.
- Static points from marks: for each mark in a collection with `ShowGridLines`: its start, and also its end if `ShowTailGridLines`. Level is the collection's Level (weight 1 to 6 from the docker), no row scope (`Grid.cs:2398-2413`, `Grid.cs:2506-2527`).

**F70. Snap window.**
*Does:* `SnapStart = t - TimePerPixel * level * strength`, `SnapEnd = t + TimePerPixel * level * strength` (`Grid.cs:2319-2339`). So effect edges at default strength reach ±10 pixels, and a level 1 mark ±2 pixels. Mark windows are recalculated in place on zoom (`Grid.cs:183-190`, `Grid.cs:2308-2317`). **KEEP:** effect-edge windows in the snapshot keep the zoom at drag start if the user zooms mid-drag.

**F71. Snap algorithm.** `find_snap(selected, offset, resizeZone)` (`Grid.cs:2045-2127`). Applied to the raw offset, from each effect's original times.
1. Offset zero or snapping off: return offset.
2. Candidates: if `OnlySnapToCurrentRow` (always true, `Grid.cs:96`) and the tracked row index is above 0: only selected effects in the tracked row (F61, which can lag the pointer's row when a move was clipped), paired with that row. **KEEP Q11:** otherwise, including when the tracked row is flat row 0 or none: every selected effect, each paired with the first row containing it, hidden rows included (`Grid.cs:1686-1689`), in which case effect-edge points never match it.
3. For each candidate, for each snapshot point in time order, for each detail at that time:
   - Skip if a best exists and `level <= best.level`. The best carries across candidates. Higher level always wins; **KEEP Q12:** at equal level the first found wins, which is the earliest time, not the nearest. The offset tested is always the raw one passed in, not an earlier candidate's result.
   - Skip if the detail has a row scope other than this effect's row.
   - Skip for opposing edges (F74).
   - `startIn = SnapStart < origStart + offset < SnapEnd`; `endIn` likewise for the end; both strict. Resize Front ignores `endIn`; Back ignores `startIn`.
   - Neither: skip. One: `snapped = SnapTime - origEdge`. Both: **KEEP Q13:** `if (SnapTime - origStart + offset) < (origEnd + offset - SnapTime)` use start, else end, written with exactly that grouping. The offset is on both sides and cancels, so this is `2 * SnapTime < origStart + origEnd`: it asks which **original** edge is nearer, not which moved edge. In practice a rightward drag snaps the end edge and a leftward drag the start edge. Ties go to the end.
   - Record best.
4. Return snapped, or the offset if nothing matched.

*Rules:* **GUARD:** upstream indexes the saved list by effect and throws if missing; skip instead.

**F72. Row scope.**
*Does:* an effect-edge point only applies to an effect paired with the same row; mark points apply everywhere (`Grid.cs:2088-2089`). Combined with candidate selection (F71 step 2), dragging a multi-row selection usually snaps only the part in the pointer's row. The tracked row updates during vertical moves (`Grid.cs:2301-2302`).

**F73. Move versus resize.**
*Does:* Move: clamp to the sequence first, then snap either edge. Resize: snap the active edge only, then clamp and enforce the minimum width, so limits beat snap (`Grid_Mouse.cs:873-882`, `Grid_Mouse.cs:958-986`).

**F74. Opposing edges.**
*Does:* when resizing, some effect edges are skipped so an effect cannot snap itself shut (`Grid.cs:2129-2139`). Resizing Front: skip an effect **Start** edge sitting exactly at this effect's original **end**. Resizing Back: skip an effect **End** edge sitting exactly at this effect's original **start**. Moves skip nothing. Mark points are never skipped. An effect End edge at this effect's original end is still a valid target when resizing Front.

**F75. Marks at identical times.**
*Does:* static points are a time-sorted map to a list of details, with one separate detail per mark and per edge (`Grid.cs:2341-2352`, `Tests/GridMarkSnapPointTests.cs:35-50`). Each mark has a registration holding exactly its start detail and optional end detail (`LM/MarkSnapPointRegistration.cs`). Moving a mark removes those exact details by identity, dropping a time's key only when its list empties, then re-registers (`Grid.cs:2360-2396`, `Grid.cs:581-594`, `Tests/GridMarkSnapPointTests.cs:116-135`). Marks moved in one event are de-duplicated by identity. A collection without grid lines registers nothing. Identity, not time, is the rule: use mark ids in Rust.

**F76. Mark lines on the grid.**
*Does:* for each time in the static map with `visibleStart <= t < visibleEnd` (`Grid.cs:2736-2767`; the end is exclusive, unlike the effects'): pick one detail: the first, replaced by a later one with a higher level and a non-empty colour. Draw over full height in its colour, width 3 if bold else 1, dashed `[level, level]` unless solid. Times in the current alignment list are skipped. When the grid is not dragging, each alignment time gets a yellow dashed `[2,2]` line instead (`Grid.cs:2769-2778`). So the guide replaces the mark line while it is being moved.
Rebuild triggers: full rebuild on collection list change, collection or decorator property change (including a mark added or removed, which raises it), strength change, or the host's call after marks finish moving, delete and paste (`Grid.cs:2541-2553`, `Form.cs:3134`, `Form.cs:3149`, `Form.cs:3163`). Incremental on marks moving (F75). **GUARD:** Level 0 or missing gives an invalid dash pattern upstream; use 1.

### I. Drawing effects

**F77. Draw Mode.**
*Do:* with toolbar "Draw Mode" on and an effect type selected, drag on the grid, anywhere not in a resize zone, ⌥ not held.
*Does:* press (`Grid_Mouse.cs:71-81`, `Grid_Mouse.cs:581-589`): needs a row at the pointer; records the time; clears selected effects, other selected rows, other active rows; clears the shift anchor. Move (`Grid_Mouse.cs:591-610`): box from start to pointer, stretched to whole rows: `top = min(startY, firstRow.top) + 1`, `bottom = max(startY, lastRow.bottom) - 2`, using rows from the top of the box to its bottom (`Grid.cs:1870-1898`). Paint: fill ARGB(100,255,255,255), border black 2 pt (`Grid.cs:91-92`, `Grid.cs:3094-3106`). Release (`Grid_Mouse.cs:232-237`, `Grid_Mouse.cs:612-624`): report effect type, rows in the box, start = earlier of press and release times, duration = their difference (`EventArgs.cs:13-41`). If the box stayed under 2 by 2, also treat it as a background click. Host: if duration is above 10 ms, make one effect per row with default data, select them, undo entry (`Form.cs:2263-2292`). Times are not snapped.
The pointer can leave the grid while drawing, so at the left edge at scroll 0 the press or release time is negative, and past the end it is beyond `TotalTime`. **FIX Q40:** upstream stores those times as they are, which gives an effect outside the sequence. Clamp both to `[0, TotalTime]` before reporting.
Since a move stretches the box to whole rows, "under 2 by 2" only happens when there was no move at all. The rows reported on release come from the stretched box.
*Rules:* Draw and Selection are exclusive toolbar buttons (`Toolstrip.cs:1815`, `Toolstrip.cs:1822`). Esc returns to Selection Mode, except mid-draw (`Hotkeys.cs:190-197`). Which effect type is "selected" is set from the Effects panel or the effect strip, and cleared by clicking it again or by Esc (`TSE/Forms/Form_Effects.cs:178-210`, `Toolstrip.cs:1429-1447`; sequencer spec F287). With no type selected, Draw Mode behaves as Selection Mode.
*Default:* Selection Mode. *Persist:* profile `DrawModeSelected`, `SelectionModeSelected` (`Form.cs:362-363`).
*Undo:* "added effects" entry from the host.

**F78. Middle-button draw.**
*Do:* middle-drag with an effect type selected, in any mode.
*Does:* same as F77 with crosshair pointer, ignoring ⌥ and resize zones, and with no effect selection logic (`Grid_Mouse.cs:57-67`, `Grid_Mouse.cs:208-224`). The pointer goes back to arrow on release. A middle release with no such draw open does nothing except turn off guides (F49). Mac: `otherMouseDown` with button number 2. Trackpad users have Draw Mode.

**F79. Resize / Draw Indicator.**
*Do:* Edit > "Resize / Draw Indicator", whose submenu has colour choices Blue, Yellow, Green, White, Red (`TSE/TimedSequenceEditorForm.Designer.cs:711`). Choosing a colour also turns it on (`Menu.cs:237-280`).
*Does:* in the chosen named colour, 1 pt, full canvas height (`Grid.cs:3123-3151`, `Grid.cs:3108-3115`): while resizing, one line at the active edge (start, or end minus 1). While waiting or moving, two lines at start and end minus 1 of the working effect, which during a move becomes the last selected effect under the pointer (`Grid_Mouse.cs:429-433`). While drawing, at the box's left and right minus 1. When resizing, waiting or moving it also publishes those times as alignment guides; drawing does not. The waveform shows them as yellow guides (F143). The grid does not: it draws yellow guides only in the normal state, so during these drags the only effect on the grid is that a mark line at exactly one of those times disappears (F76).
**FIX Q14:** upstream raises that from inside paint, and the event requests another paint 16 ms later, so a drag repaints continuously. Publish guides from the press (waiting) and each drag update instead. **GUARD:** the working effect can be gone while one of these states is open; upstream would throw inside paint. Draw nothing and publish nothing then.
*Default:* off, "Red". `Color.FromName`: Green is (0,128,0). *Persist:* profile `ResizeIndicatorEnabled`, `ResizeIndicatorColor` (`Form.cs:361`, `Form.cs:368`).

### J. Commands on effects

**F80. Align commands.** All take the selected effects and a reference effect, and apply atomically through F81's batch. Null reference does nothing (`Tests/GridAlignmentNullReferenceTests.cs`). Hotkeys need: drag state normal, an effect under the pointer that is selected, and more than one selected (`Hotkeys.cs:139-289`). The pointer is read at key time, from wherever the mouse is now, and the effect is the topmost hit there. Mac: `mouseLocationOutsideOfEventStream`, converted to the grid; the same for ⌘D (F84). The context menu versions use the effect under the press (`ContextMenu.cs:127-145`). S, D, E below are start, duration, end of the effect being changed; rS, rE, rD are the reference's.

| Command | Key | Skip if | New (start, end) | Undo label | Cite |
|---|---|---|---|---|---|
| Align Start Times | S; ⇧S holds duration | S = rS | If E <= rS: (rS, rS+D). ⇧: (rS, rS+D). Else (rS, E) | Start Align N effect(s) | `Grid.cs:1021-1052` |
| Align End Times | E; ⇧E holds duration | E = rE | If S >= rE: (rE-D, rE). ⇧: (rE-D, rE). Else (S, rE) | End Align N effect(s) | `Grid.cs:1061-1092` |
| Align Both Times | B | S = rS and E = rE | (rS, rE) | Align N effect(s) | `Grid.cs:1134-1156` |
| Match Duration | U; ⇧U holds end | D = rD | ⇧: (E-rD, E). Else (S, S+rD) | Duration Align N effect(s) | `Grid.cs:1101-1126` |
| Align Start to End | R; ⇧R moves start only | E = rE | start = rE; end = ⇧ ? E : start+D; if end-start < 50 ms, end = start+D; if end > Total, end = Total | Start to End Align N effect(s) | `Grid.cs:1165-1193` |
| Align End to Start | N; ⇧N moves end only | S = rS | end = rS; start = ⇧ ? S : end-D; if end-start < 50 ms, start = end-D; if start < 0, start = 0 | End to Start Align N effect(s) | `Grid.cs:1202-1230` |
| Align Centerpoints | C | S = rS (**KEEP Q15**, it should arguably test centres) | c = rS + rD/2 in double seconds; (c-D/2, that + D) | Center Align N effect(s) | `Grid.cs:1238-1262` |

Hold-flag source: `e.Shift` for S and E (`Hotkeys.cs:152`, `Hotkeys.cs:168`); exact ⇧ for U, R, N (`Hotkeys.cs:248`, `Hotkeys.cs:261`, `Hotkeys.cs:274`).

**F81. Batch move and its limits.**
*Does:* `MoveResizeElements` (`Grid.cs:1350-1376`): for each entry, skip if the new end is past `TotalTime` or the duration is below 1 ms. Otherwise set times, commit, queue render. Raise finished-moving and one undo entry of the given type, covering all entries, skipped ones too, so **KEEP Q35:** the entry is recorded even when every effect was already aligned ("Start Align 0 effects"). A start below 0 is not skipped; the setter makes it 0 and then the end sets the duration. Single-effect form (`Grid.cs:1387-1409`) refuses a start or end past total or a duration below 1 ms, and returns false; by start and end refuses `start >= end` (`Grid.cs:1419-1429`).
Selection limit: `OkToUseAlignmentHelper` is false if any selected effect's last row has more than 32 selected effects. Align commands then show a warning instead, titled "Warning", OK only. **KEEP Q16:** the text says "Too many effects selected on the same row for this action.\nMax selected effects per row for this action is 4", though the limit is 32 (`Grid.cs:57`, `Grid.cs:975-984`). The context menu also disables Edit Time past the limit, with the tooltip "Disabled, maximum selected effects per row is 32." (`ContextMenu.cs:386-388`).

**F82. Distribute.**
*Do:* D (equally), I (dialog).
*Does:* host code computing times and calling F81 with type Distribute (`Form.cs:2646-2711`, `Form.cs:2717-2819`, `Hotkeys.cs:216-225`, `Hotkeys.cs:279-289`). Sequencer spec. Undo "Distributed N effect(s)".

**F83. Align to nearest marks.**
*Do:* ⌘⇧S, ⌘⇧E, ⌘⇧B.
*Does:* host code (`Hotkeys.cs:144-148`, `Hotkeys.cs:160-164`, `Hotkeys.cs:177-181`, `Form.cs:6223`). With no selection it warns "No effects have been selected and action will be applied to your entire sequence. This can take a considerable length of time, are you sure ?", titled "Align effects to marks". Sequencer spec.

**F84. Split at pointer.**
*Do:* ⌘D, or context menu Divide at cursor.
*Does:* ⌘D: if there is a selection and the pointer time is strictly inside the visible range, split selected effects with `start < t < end` at t (`Grid.cs:1282-1294`, `Hotkeys.cs:216-220`). Context menu (shown only over an effect, on a row not tagged Deprecated, `ContextMenu.cs:299`) uses the menu's time, which is the release point's (F54), on the selected effects with `start < t < end`, or if nothing is selected the effect under the press with no such test (`ContextMenu.cs:307-319`). Split (`Grid.cs:1269-1280`): clone them, then originals become `(start, t - start)` and clones `(t, end - t)`.
*Undo:* **KEEP Q17:** one "added" entry plus one move entry per effect, per half. On the Mac they may be grouped only if the Edit menu then undoes the whole split at once; the owner can decide.

**F85. Close Gaps.**
*Do:* toolbar Close Gaps; dropdown sets the threshold: .100, .200, .300 or .400.
*Does:* (`Grid.cs:1301-1343`). With nothing selected, asks "No effects have been selected and action will be applied to your entire sequence. This can take a considerable length of time, are you sure ?", title "Close element gaps"; on No stops. It then processes all rows; with a selection it processes visible rows and selected effects only. In each row, walking pairs in order: if `next.start - active.end < threshold` (in seconds, doubles) and `next.start != active.end` and not already queued, queue `next` to `(active.end, next.end)`. Then `active = next`. This also shortens overlaps. Run through F81 (entries with a result under 1 ms are skipped).
**KEEP Q41:** with nothing selected "in order" is the row's time order. With a selection it is the order the effects were selected in (`Row.cs:445-453`), so a pair can run backwards in time. That counts as a negative gap, so it is queued too, and F81 drops it if the result is under 1 ms. Parse the threshold with a fixed `.` decimal point; upstream's `Convert.ToDouble` follows the Windows locale.
*Default:* threshold ".100" seconds, string. *Persist:* profile `CloseGapThreshold` (`Form.cs:359`, `Toolstrip.cs:1760`).
*Undo:* "Moved N effect(s)".

**F86. Bulk move.**
*Do:* Tools > Bulk Move dialog.
*Does:* `MoveElementsInRangeByTime` (`Grid.cs:1449-1453`, `Menu.cs:710-715`): effects with start in the inclusive window, optionally visible rows only, moved by `MoveElementsByTime`. Dialog is the sequencer spec's.

**F87. Nudge selected effects.**
*Do:* ⌘← or ⌘→.
*Does:* moves by 2 pixels' worth of time (`Hotkeys.cs:123-137`) through `MoveElementsByTime` without clipping. `MoveElementsByTime` (`Grid.cs:1472-1551`):
- No clipping: offset is limited so that the earliest stays at or after 0 and the latest at or before total.
- Clipping: entirely before 0 or after total: removed from its last row, with a deleted marker in the undo data. Partly before 0: start = 0 and **KEEP Q18** `duration += 0 + offset`, because start was already zeroed. Partly past total: start += offset, end = total. Otherwise start += offset.
- Returns the offset used. One undo entry, type Move. No snapping.

**F88. Undo and redo of times and rows.**
*Does:* `SwapElementPlacement` (`Grid.cs:2141-2165`). For each entry: a null entry (deleted) re-adds to its last row. Otherwise swap start, duration and last row with the saved values. If the rows differ, remove from the saved row, add to the current, and raise changed-rows. Render if dirty. The entry then holds the redo state. Redo validates: if the effect is no longer in its row, find by effect node (`TSE/Undo/ElementsTimeChangedUndoAction.cs:28-54`). Labels: `TSE/Undo/ElementsTimeChangedUndoAction.cs:56-86`, used as `UndoManager` action names.

**F89. Drop on the grid.**
*Do:* drag an effect type, gradient, curve, colour or a file onto the grid.
*Does:* host handlers use row, time and element at the pointer (`Form.cs:861-1014`). Effect type: `EffectDropped(type, time, row)`, refused on Deprecated rows. Holding ⌘ with effects selected replaces them instead, after asking "This action will replace N effects, are you sure ?" (`Form.cs:4232-4263`). Gradient, curve, colour: applied to the effect under the pointer if compatible. Files: copy effect, `HandleFileDrop`. Mac: `NSDraggingDestination` on the grid, calling the same host logic with the Rust hit test. Validity and the handlers are the sequencer spec's F305 to F311; this control supplies only `row_at`, `element_at` and the time at the pointer, and shows no drag indicator of its own.

**F90. Reveal effects.**
*Does:* Find Effects picks effects: selects them, shows their rows, opens and shows all ancestors, scrolls so the last is visible: `VisibleTimeStart = last.start`, `VerticalOffset = last's row top`, then selection changed (`Grid.cs:987-1012`, `TSE/Forms/FindEffectForm.cs:230`).

### K. Cursors and playback

**F91. Edit cursor.**
*Does:* one time per editor (`TimeLineGlobalStateManager.cs:37-45`). Setting it always raises the moved event, even to the same value. Grid: ARGB(150,50,50,50), 2.5 pt, full height, above effects (`Grid.cs:93-94`, `Grid.cs:3157-3160`). Ruler and waveform: blue, 1 pt (`Ruler.cs:231-238`, `Waveform.cs:368-375`); under everything else on the ruler (F97), on top on the waveform (F141). It is the paste time unless the paste row is selected (F27), the add-effect time (`Menu.cs:173`, `Form.cs:1279-1280`), the bulk-move default (`Menu.cs:710`), and the base for cursor placement when pasting translated effects (`Form.cs:6167-6170`). Nothing but the triggers in F92 changes it. Mac: a separate layer per surface, moved only by F92, never by playback.
*Default:* 0.

**F92. When the cursor moves.**

| Trigger | New time | Cite |
|---|---|---|
| Single click on an unselected effect, no ⌘ | Earliest start among selected | `Grid_Mouse.cs:164` |
| ⌘-click selects | That effect's start (last action) | `Grid_Mouse.cs:164` |
| Release: collapse to one effect | That effect's start | `Grid_Mouse.cs:261` |
| Release with ⇧ or ⌘ on effects | Last effect pressed | `Grid_Mouse.cs:270` |
| Rubber band release | Earliest selected start | `Grid.cs:486-501`, `Grid_Mouse.cs:672` |
| `SelectElement` | Its start | `Grid.cs:2603-2608` |
| `SelectElements` | Earliest selected start; none if the list is empty | `Grid.cs:2623-2629` |
| Background click, no drag | Pointer time | `Grid_Mouse.cs:668`, `Grid_Mouse.cs:687-697` |
| Right-click on background | Pointer time | `Grid_Mouse.cs:281-282` |
| Ruler click, no ⌘ | Click time | `Form.cs:2894` |

All effect-driven moves are suppressed in legacy mode (F93). The `DeselectElement` call and a ⌘-press that deselects do not move it, but the ⌘-release after that press does (row "Release with ⇧ or ⌘"). Row clicks (F25), ⌘A (F56) and rubber bands that select nothing do not move it. A draw box with no movement counts as a background click (F77). Each move repaints and updates the status text (F96).
*Tests:* `Tests/TimelineCursorSelectionTests.cs`.

**F93. Legacy Cursor / Active Row.**
*Do:* toggle Edit > "Legacy Cursor / Active Row".
*Does:* when on: selecting effects does not move the cursor (`Grid.cs:460-463`); a rubber band does not finalize active row or cursor (`Grid.cs:488-491`); starting one clears active rows at once (`Grid_Mouse.cs:635-638`); a background click keeps other active rows until the click sets one; right-clicking an effect also moves the cursor to the pointer (`Grid_Mouse.cs:281`).
*Default:* off. *Persist:* profile `LegacyCursorActiveRow` (`Form.cs:367`, `Menu.cs:287-290`).

**F94. Playback line and arrow.**
*Does:* while playing: grid draws a 1 pt green (0,128,0) line at the position (`Grid.cs:3162-3165`), ruler a green triangle at its top: `(x, 10)`, `(x-8, 0)`, `(x+8, 0)` (`Ruler.cs:220-228`, `Ruler.cs:44-46`). Waveform draws nothing, its handler is empty (`Waveform.cs:282-285`) **KEEP Q19**. The edit cursor does not move. Upstream updates from a 40 ms timer, which starts when the sequence starts and stops when it ends (`TSE/TimedSequenceEditorForm.Designer.cs:1220`, `Form.cs:3457-3463`, `Form.cs:3428-3445`). A pause holds the line where it is. The position goes to null when the context ends, which removes the line and arrow and restores the remembered range (`Form.cs:3447-3455`). Mac: display link reading the session's playback position (`ShowPlayback::position`, `crates/vixen-session/src/playback.rs:409`), moving separate layers. No grid redraw.

**F95. View follows playback.**
*Does:* `if position > start + 0.9 * span: start = position - 0.1 * span`. `if position < start: start = position - 0.1 * span`. Both run, in that order, using the grid's span. Through the usual clamp (`TimelineControl.cs:1275-1287`). This is a page flip. The rule can stay on the main thread at display rate.
*Rules:* it runs whenever the position value changes, with no other test, so a user who scrolls away during playback is snapped back at the next tick. A paused position does not change, so the event does not fire (`TimeInfo.cs:95-96`) and the view can be scrolled freely while paused. On the Mac apply the rule only when the position read differs from the last one. Not persisted differently from any other view change (F6): the flip marks editor state.

**F96. Current time status.**
*Does:* the status label shows `m:ss.fff` of the playback position, empty when stopped, and also the cursor time whenever it moves (`Form.cs:3465-3479`). Status bar is the sequencer's.

### L. Ruler

**F97. Ruler surface.**
*Does:* background Gray (128,128,128) (`Ruler.cs:35`). Paint order: edit cursor, major ticks, minor ticks, labels, bottom line, playback indicators, marks (`Ruler.cs:94-106`). Bottom line: black, 2 pt, y = height - 1, from 0 to `px(TotalTime)` (`Ruler.cs:99-102`).
*Default:* height 50. *Persist:* F113.

**F98. Ticks.**
*Does:* recalculated on resize, zoom and construction (`Ruler.cs:242-252`). `t = time(10)`, in seconds, minutes, hours, days as doubles, then the first match (`Ruler.cs:277-347`):

| t | minor | minor per major |
|---|---|---|
| over 0.05 s and under 0.1 s | 100 ms | 5 |
| under 0.25 s | 250 ms | 4 |
| under 0.5 s | 500 ms | 4 |
| under 1 s | 1 s | 5 |
| under 5 s | 5 s | 6 |
| under 10 s | 10 s | 6 |
| under 15 s | 15 s | 4 |
| under 30 s | 30 s | 4 |
| under 1 min | 1 min | 5 |
| under 5 min | 5 min | 3 |
| under 10 min | 10 min | 3 |
| under 15 min | 15 min | 4 |
| under 30 min | 30 min | 2 |
| under 1 h | 1 h | 6 |
| under 6 h | 6 h | 4 |
| under 1 day | 1 day | 7 |
| otherwise | 7 days | 1 |

If `t` is 0.05 s or less (`Ruler.cs:349-372`): `d = 0.000001`, loop: if `t < d`, minor `d`, per major 5; `d *= 5`; if `t < d`, minor `d`, per major 5; `d *= 5`; if `t < d`, minor `d`, per major 4; `d *= 4`. Minor ticks = `(long)(10000 * 1000 * d)`. Exact f64 arithmetic in that order matters at the boundaries.
Drawing (`Ruler.cs:119-133`): first tick is the first multiple strictly after the visible start, then step in float pixels to the visible end. Line from `y = height * (1 - k)` to height: major width 2, `k = 0.4`; minor width 1, `k = 0.20`. Black.

**F99. Time labels.**
*Does:* font height `height / 3` in pixels (`Ruler.cs:265-269`), white. Drawn interval (`Ruler.cs:145-157`): measure the label of `VisibleTimeEnd`; `n = (int)((width + 10) / px(major)) + 1`; `interval = major * n`; first label at `start - start % interval + interval`, in ticks; until the visible end. For each: skip if `lastPixel + 10 + width/2 >= centre`, where `lastPixel` starts at 0, otherwise draw at `(centre - width/2, (height/4 - textHeight/2) + 9)` and set `lastPixel = (int)(centre + width/2)` (`Ruler.cs:159-169`). `height/4` is integer division.
Format, by minor tick and time `t` (`Ruler.cs:377-440`). `m`, `s`, `h` are components, not totals:

| Minor tick | Rounding | t >= 1 h | t >= 1 min | else |
|---|---|---|---|---|
| >= 1 h | truncate to whole hours | `h:mm` | `h:mm` | `h:mm` |
| >= 1 min | truncate to whole minutes | `h:mm:ss` | `m:ss` | `m:ss` |
| >= 1 s | truncate to whole seconds | `h:mm:ss` | `m:ss` | `m:ss` |
| >= 100 ms | none | `h:mm:ss.f` | `m:ss.f` | `s.f` |
| >= 10 ms | none | `h:mm:ss.ff` | `m:ss.ff` | `s.ff` |
| >= 1 ms | none | `h:mm:ss.fff` | `m:ss.fff` | `s.fff` |
| below | none | `h:mm:ss.ffffff` | `m:ss.ffffff` | `s.ffffff` |

Fractions are truncated, not rounded. Mac: system font with `.monospacedDigit()`.

**F100. Click the ruler: set the playback start and cursor.**
*Do:* left click, with at most 2 points of horizontal movement, away from any mark and the resize strip.
*Does:* press with any button clears the mark selection (`Ruler.cs:493`). Release (`Ruler.cs:644-647`) reports the time (`time(x) + VisibleTimeStart`), modifiers and button. Host (`Form.cs:2868-2896`): remembered range = (time, none); `PlaybackStartTime = time`, `PlaybackEndTime = null`; sequence marked modified; `CursorPosition = time`. **KEEP:** the time is not clamped to the sequence. Nothing happens with no playback context, which only logs an error (`Form.cs:2862-2866`).
*Note:* `Ruler.cs:629-633` is written for `Clicks == 2`, but WinForms reports 1 to mouse-up events as far as can be determined, so a double-click is two clicks. Either path has the same effect, so treat a double-click as two clicks.
*Persist:* range saved in the sequence (see Data and persistence). *Undo:* none.

**F101. ⌘-click the ruler: play from here.**
*Does:* holding ⌘: the existing start and end are remembered, the start is set and end cleared, and playback starts from there to the end (`Form.cs:2870-2877`, `Form.cs:2888-2891`). The cursor does not move. When playback ends the remembered range is restored (`Form.cs:3447-3452`).

**F102. Drag the ruler: select a range.**
*Do:* left-drag more than 2 points.
*Does:* on crossing: host remembers the current range (`Form.cs:3168-3172`), start is set from the pointer, end cleared (`Ruler.cs:511-518`). Each move: start and end are the lower and higher of press X and pointer X, as `time(x) + VisibleTimeStart`; start clamped at 0, end clamped at total (`Ruler.cs:520-538`, `Ruler.cs:601-614`). The press is kept as a client X, not a time, so if the view scrolls during the drag (wheel, playback follow) the whole range shifts with it. Pointer may leave the control; there is no auto-scroll. Release without ⌘: this becomes the remembered range (`Form.cs:3193-3198`). The edit cursor does not move.
*Persist and undo:* as F100, except that **KEEP:** a range drag does not mark the sequence modified, though a plain click does, so a changed range is only saved with some other change.

**F103. ⌘-drag the ruler: play the range.**
*Does:* release with ⌘ held plays from range start to range end (`Form.cs:3182-3192`). When playback ends the remembered previous range returns.

**F104. Range indicators.**
*Does:* DarkGray (169,169,169), shifted up by `base/2` with `base = 16`, `len = 10` (`Ruler.cs:44-46`, `Ruler.cs:177-217`). `x` is the integer pixel of the time. Start set: triangle `(x, H - 8)`, `(x + 10, H - 16)`, `(x + 10, H)` plus a line at `x` from `H - 16` to `H`. End set: mirrored. Both: a 4 pt line at `y = H - 8` from `x1 + 10` to `x2 - 10`. All values before the shift.

**F105. Start and End toolbar buttons.**
*Does:* Start: range start = 0, remembered start = 0, visible start = 0 (`Toolstrip.cs:1596-1600`). End: range end = sequence length, remembered end = length, visible start = `TotalTime - span` (`Toolstrip.cs:1612-1616`). Play reads `PlaybackStartTime ?? 0` and `PlaybackEndTime ?? MaxValue` (`Form.cs:3323-3324`).

**F106. Marks on the ruler.**
*Does:* for each collection with `IsVisible` (`ShowGridLines || ShowMarkBar`, `Marks/MarkCollection.cs:160`): colour from the decorator, dashed `[Level, Level]` unless solid (**GUARD** Level 0), for each mark start in the visible range: full-height line, width 3 if selected else 1 (`Ruler.cs:881-916`). **KEEP Q20:** `IsBold` is overwritten per mark, so bold has no effect here. Locked collections are drawn.

**F107. Mark hit test on the ruler.**
*Does:* over visible, **unlocked** collections, highest Level first: marks with `|start - t| <= 20 ms` (`Ruler.cs:856-879`). **KEEP Q21:** this is a fixed time window, not pixels. Zoomed far out it is under a pixel; zoomed in it is many.

**F108. Hover on the ruler.**
*Does:* with no left drag (`Ruler.cs:580-598`), which includes moving with the right button held: over a mark: left-right pointer (`VSplit`) and the mark's start published as an alignment guide, so a yellow line shows in the grid and waveform. Else in the bottom strip (`y` from `height - 6` to `height - 1`) when not locked: up-down pointer, and a guide from a mark just left stays up until the pointer moves off the strip (**KEEP Q44**). Else hand pointer and guides cleared. Enter: hand; leave: arrow and guides cleared (`Ruler.cs:714-725`).

**F109. Click a mark on the ruler.**
*Do:* press on a mark, optionally with exactly ⌘.
*Does:* (`Ruler.cs:468-488`, any button). Not exactly ⌘: clear the mark selection. If the first hit mark is selected, deselect it; otherwise select it. Then prepare a mark drag, saving the times of the selected marks.

**F110. Drag marks on the ruler.**
*Do:* left-drag from a mark.
*Does:* each move (`Ruler.cs:542-568`): `t = time(x) + VisibleTimeStart`; every selected mark's start `+= t - last`; `last = t`. The first move therefore jumps the pressed mark's start onto the pointer. **KEEP Q22:** no clamp, no snap, so a start can go negative. Selected marks keep their durations. Publish moving marks (which re-sorts their collections, `LM/TimeLineGlobalEventManager.cs:56-64`) and alignment times: start and end if the pressed mark's collection shows in the marks bar, otherwise start only.
Paint during the drag (`Ruler.cs:918-937`): yellow dashed `[2,2]` 1 pt line at the live pointer, and at the top next to it the time in white, bold, size 8: `mm:ss.fff` if minutes component >= 10, `m:ss.fff` if >= 1, `ss.fff` if seconds >= 10, else `s.fff`. The font is Arial upstream; use the system font. The line shows from the press; the text is only set by a move, so between press and first move it still reads the previous drag's value. **FIX Q48:** don't show it until the first move on the Mac; nothing depends on the stale value.
Release (`Ruler.cs:653-662`): if the selection is non-empty and X differs from the press: raise marks moved with the saved times, clear guides. Host: undo entry, rebuild snap points, re-render effects that depend on marks, sequence modified (`Form.cs:3131-3137`). **KEEP Q43:** the test is only on X. Dragging away and back to the press X leaves the mark moved up to 20 ms (the first move's jump) with no undo entry, and a press and release with no movement leaves guides on until the next hover. A left press on a mark never reaches the click path of F100.
*Undo:* `MarksTimeChangedUndoAction`, **KEEP Q23** labelled "Moved N effect(s)" (`TSE/Undo/MarksTimeChangedUndoAction.cs:55-58`).

**F111. Right-click the ruler: add a mark.**
*Do:* right-click away from marks, with: nothing; exactly ⌘ (fill gap); exactly ⇧ (prompt for label); exactly ⌘⇧ (both).
*Does:* release with no mark at that time (`Ruler.cs:672-691`, `Form.cs:2897-2900`), `AddMarkAtTime` (`Form.cs:3032-3096`):
1. Prompt option: `TextDialog("Enter the Mark label text.")` with an empty title. Cancel keeps an empty label and still adds. The press cleared the mark selection already (`Ruler.cs:493`), and the new mark is not selected.
2. No collections: pause if playing, then ask "Marks are stored in Mark Collections. There are no mark collections available to store this mark. Would you like to create a new one?", title "Create a Mark Collection". Yes creates "Default Marks" in white.
3. Otherwise the default collection if visible. If none: pause if playing, ask "The active mark collection is not visible on the timeline. Would you like to enable it to add the mark?", title "New Mark". OK sets `ShowGridLines` and `ShowMarkBar` true and `ShowTailGridLines` false on the default collection.
4. If a collection was found and no mark has exactly that start: add a mark (default duration 450 ms, `Marks/Mark.cs:21`). Fill gap (F125's algorithm). Sequence modified, re-render dependents.
*Rules:* ⇧Space adds a mark at the playback position during playback (`Hotkeys.cs:112-116`).
*Undo:* "Added 1 Mark" (`TSE/Undo/MarksAddedUndoAction .cs:29`). Sheets on the Mac, with verbs on the buttons.

**F112. Right-click a mark on the ruler: delete.**
*Does:* press selected it (F109). Release with no movement opens a one-item menu, "Delete Selected Marks", at the pointer (`Ruler.cs:672-684`). Moved: nothing. Choosing it (`Ruler.cs:703-712`): remove each selected mark from its collection, raise deleted, clear selection. Host: undo entry "Removed N Mark(s)", re-render, rebuild snap, modified (`Form.cs:3139-3151`).
*Rules:* the ruler has no key handling. The Delete key only reaches marks through the bar (F134), and the host's call to the ruler's delete is commented out (`Hotkeys.cs:65-67`, `Menu.cs:194`). Marks selected from the ruler can still be deleted with the key if the bar holds focus, since selection is shared.

**F113. Drag the ruler's height.**
*Do:* press in the bottom strip (pointer already up-down) and drag.
*Does:* `height = y + 1` when `y > 40` (`Ruler.cs:489-490`, `Ruler.cs:571-575`). Mark hit takes priority over the strip. Grid shrinks accordingly. Font and everything recompute.
*Rules:* needs lock off, tested both at press and while moving (`Tests/RulerLockHeightTests.cs:64-75`).
*Default:* 50. *Persist:* profile `RulerHeight` (`Form.cs:372`, `Form.cs:5832`).

**F114. Reset ruler height.**
*Do:* double-click the bottom strip.
*Does:* height 50 (`Ruler.cs:616-625`). Not when locked (`Tests/RulerLockHeightTests.cs:77-88`).

**F115. Lock Ruler Height.**
*Do:* View > "Lock Ruler Height" toggle (`TSE/TimedSequenceEditorForm.Designer.cs:775-791`).
*Does:* disables F113, F114 and the strip's pointer (`Ruler.cs:83`, `TimelineControl.cs:337-347`, `Menu.cs:216-219`).
*Default:* off. *Persist:* profile `LockRulerHeight` (`Form.cs:373`).

**F116. Ruler pointer shapes.** Hand by default, left-right resize over a mark, up-down over the strip, arrow outside. See F108.

### M. Marks bar

**F117. Mark rows.**
*Does:* one row per mark collection in collection order; a row is shown when `ShowMarkBar` is on (`MarkRow.cs:59`). Rows follow collection add, remove and property changes (`MarksBar.cs:91-97`, `MarksBar.cs:125-159`, `MarkRow.cs:45-53`). An add appends a row at the end whatever the collection's index, and any other kind of change (reset, move, replace) rebuilds in collection order, so row order can lag collection order after an insert. Background Gray; separators `TimeLineGridColor` at row bottom minus 1 across the viewport (`MarksBar.cs:42`, `MarksBar.cs:1190-1212`). With no shown rows the height is 0 and the grid takes the space (`MarksBar.cs:78-89`, `MarksBar.cs:1155-1158`).

**F118. Mark stacking and bar height.**
*Does:* same algorithm as F40, per collection, for the visible range (`MarkRow.cs:78-123`, `MarkRow.cs:125-187`). A row's `StackCount` is the most lanes in any cluster in view, and `Height = 20 * StackCount` (`MarkRow.cs:14`, `MarkRow.cs:120-121`). Bar height is the sum of visible rows (`MarksBar.cs:1155-1158`). So the bar **grows and shrinks as you scroll** into overlapping marks (**KEEP Q24**) and the grid moves with it. **FIX Q25:** upstream computes this during paint. Compute it in the layout pass whenever the visible range or marks change. **KEEP Q26:** when a mark is reached by two clusters, it keeps its first values, because upstream's update changes a struct copy (`MarkRow.cs:96-101`).

**F119. Mark appearance.**
*Does:* (`MarksBar.cs:1246-1305`). `h = markStack.StackCount > 1 ? (rowH - 2) / row.StackCount : rowH - 2`; if 0, `h = rowH`. `y = rowTop + h * index`, `x = floor(px(start))`, `w = (int)px(duration)`; `w <= 0` not drawn. Fill the decorator colour; locked collections use a diagonal-cross hatch with each channel +30, or -30 if that passes 255, on the colour. Border black, 3 pt if selected else 1. Selected adds an inset 1 pt dashed `[1,2]` in `ForeColor`. Text: `mark.Text` wrapped in the rectangle, clipped, in the control font. Black if `255 - round(0.299R + 0.587G + 0.114B) <= 105`, else white (`MarksBar.cs:1307-1315`); the round is `Convert.ToInt32`, ties to even. The colour is judged on the plain decorator colour, also on a hatch. Marks are drawn when `end >= visStart`, stopping at the first with `start > visEnd`, per shown row (`MarksBar.cs:1214-1240`).

**F120. Mark hit test in the marks bar.**
*Does:* (`MarksBar.cs:1057-1087`). Row at y over visible rows (`MarksBar.cs:1108-1123`). In that row's marks: skip those ending before the pointer time, stop at the first starting after it. `h = stack.Count == 1 ? rowH : rowH / row.StackCount`, `top = rowTop + h * index`. Hit if inside `x..x+w` inclusive and `top <= y < top + h`. Note that hit height differs from draw height by the 2. **KEEP.**

**F121. Click a mark.**
*Do:* press on a mark with no modifier.
*Does:* the bar takes keyboard focus (`MarksBar.cs:185`), on any button. A locked mark counts as nothing (`MarksBar.cs:192-193`). If the mark is not selected: anchor = this point, clear, select it. Pressing blank does the same with no mark, so it clears the selection and moves the anchor (`MarksBar.cs:212-220`). Dragging from blank does nothing: the bar has no rubber band. Selection is shared with the ruler (`LM/MarksSelectionManager.cs`). On any non-right release without a drag, with neither ⌘ nor ⇧, if more than one mark is selected: collapse to the pressed mark, anchor = release point (`MarksBar.cs:351-359`). A middle click selects nothing at press but collapses here all the same: to the mark under it, or to nothing on blank.

**F122. ⌘-click a mark.**
*Does:* toggles it. Selecting also sets the anchor (`MarksBar.cs:229-240`). ⌘ blocks a resize at that press (`MarksBar.cs:249`).

**F123. ⇧-click marks.**
*Does:* if an anchor exists: clear, then select every mark overlapping the rectangle's time span in the visible rows from the rectangle's top row to its bottom row, `start < selEnd && end > selStart` (`MarksBar.cs:221-227`, `MarksBar.cs:828-913`). **KEEP Q27:** this can select marks in locked collections. Holding ⇧ takes priority over ⌘.

**F124. Double-click a mark: play it.**
*Do:* double-click, with either left or right button, no ⌥.
*Does:* plays from the mark's start to its end (`MarksBar.cs:197-209`, `Form.cs:2903-2906`, `Form.cs:3362-3386`). If playback is paused it stops first. A start below 0 is replaced by the range start or 0, and an end past the sequence by the range end or "to the end". Not on locked marks. The second press runs no selection and starts no drag, and with the right button the release still opens the context menu.

**F125. ⌥ double-click a mark: fill the gaps.**
*Does:* saves the selected marks' times, then `FillGapTimes` on the clicked mark (`MarksBar.cs:199-205`, `Marks/MarkCollection.cs:237-258`): sort; going back from the mark, at the first earlier mark whose end is before this start: start = that end + 1 ms. If not the last mark: duration = next start - 1 ms - start. The test is strict and the walk has no early stop, so a previous mark that overlaps or exactly touches is passed over, and the start can jump back across it (**KEEP**). The next mark is read by the index from before the start changed. The duration floor applies (F127).
*Undo:* raises marks moved with type Resize, for the selected marks.

**F126. Drag marks along their rows.**
*Do:* press on a mark, not in a zone, and drag beyond the threshold.
*Does:* threshold is the same 8 box as F58 (`MarksBar.cs:27`, `MarksBar.cs:541-547`), and needs a mark at the press (`MarksBar.cs:454-457`). Hand pointer. Saves selected marks' times (`MarksBar.cs:549-555`, `MarksBar.cs:806-810`). Each move (`MarksBar.cs:510-539`): `dt = time(x - x0)`; zero stops; clamp so the earliest start stays at 0 or later and the latest end at total or earlier; each mark's start = original + dt. Marks do not change rows, and y is ignored. No snapping. Publish moving and alignment with the pressed mark's start and end.
Release (`MarksBar.cs:557-562`, `MarksBar.cs:812-821`): anchor = release point; guides off; marks moved with type Move.
*Undo:* F110's. Right button never drags (`MarksBar.cs:438`).

**F127. Resize marks.**
*Do:* press in a zone (F65's rule with `MarksBar.cs:475-500`; left-right pointer, otherwise arrow, recomputed on every hover) with no ⌘, and drag.
*Does:* begins at the press, with no threshold; ignored unless drag state is normal (`MarksBar.cs:789-794`). Each move (`MarksBar.cs:677-763`): `dt`; zero stops. Shortest original duration among selected. Front: clamp so the earliest start is 0 or later; if `px(shortest - dt) < 12`, `dt = shortest - time(12)`; publish alignment at the pressed mark's start. Back: clamp so the latest end is at total or earlier; if `px(shortest + dt) < 12`, `dt = time(12) - shortest`; publish alignment at its end. Apply: Front: start = orig + dt, duration = orig - dt. Back: duration = orig + dt. The guide is published before the new times are applied, so it trails by one event; publishing after applying is fine on the Mac. No snapping. Marks do not change rows.
*Rules:* **KEEP Q34:** as for effects, a press and release in a zone with no movement still records a Resize undo entry, rebuilds snap points, re-renders and marks the sequence modified. On a short mark the zones cover nearly all of it, so most clicks on one do this. **KEEP Q35:** in a locked mark's zone (F139) with nothing selected the entry is "Resize 0 effects".
Model floor: `Mark.Duration` setter enforces 25 ms, and the `StartTime` setter raises any shorter duration to 25 ms (`Marks/Mark.cs:41-69`).
*Undo:* type Resize, label "Resize N effect(s)" (**KEEP Q23**).

**F128. Glued resize.**
*Do:* hold ⌥ while resizing with exactly one mark selected.
*Does:* the neighbour by index in the collection (previous for Front, next for Back) follows, joined at the edge (`MarksBar.cs:690`, `MarksBar.cs:697-705`, `MarksBar.cs:724-732`, `MarksBar.cs:767-783`). `shortest = min(shortest, neighbour's current duration)`. Front: `neighbour.duration = mark.start - neighbour.start`. Back: keep neighbour's end; `neighbour.start = mark.end`; `neighbour.duration = end - start`. **KEEP Q28:** the minimum-width check runs in the wrong direction for the neighbour, so it can collapse to the 25 ms floor. **FIX Q29:** upstream does not save the neighbour's times, so undo leaves it changed. Include it in the undo data.

**F129. Auto-scroll in the marks bar.**
*Does:* horizontal only, in Moving or Resizing. Margin 24, 50 ms tick. `outside` as F63; each tick: `VisibleTimeStart = clamp(start + time(outside / 4))` with integer division, clamped so that it is 0 if the whole sequence fits, then replay the last pointer event (`MarksBar.cs:576-671`). *Tests:* `Tests/MarksBarAutoScrollTests.cs`.

**F130. Context menu: Edit Text.**
*Do:* right-click, Edit Text.
*Does:* enabled only when press and release were at the same point on an unlocked mark (`MarksBar.cs:271-273`). `TextDialog("Enter the new text.")`, title "Edit Mark" for exactly one selected, "Edit Multiple Marks" otherwise; initial text, selected, only for one (`MarksBar.cs:973-989`, `Vixen.Common/Controls/TextDialog.cs:20-26`). Enter is OK, Esc cancels (`Vixen.Common/Controls/TextDialog.cs:40-44`). OK sets the text of all selected and raises text changed, with old and new text for each (`LM/MarksTextChangedEventArgs.cs:17-31`). Host: modified, re-render dependents (`Form.cs:3125-3129`).
*Undo:* **KEEP:** none. Item order in the menu: Edit Text, Cut, Copy, Paste, separator, Delete, then the type items below (`MarksBar.cs:268-337`). The menu appears at the pointer on any right release.

**F131. Context menu: Cut.**
*Does:* enabled with a selection. Clipboard = selected marks by reference, then F134's delete (`MarksBar.cs:275-278`, `MarksBar.cs:425-430`).

**F132. Context menu: Copy.**
*Does:* enabled with a selection. Clipboard = selected marks by reference (`MarksBar.cs:280-283`, `MarksBar.cs:419-423`). **KEEP Q30:** not copies, so later edits show up in a paste. The clipboard is private to the bar, not the system pasteboard, and lives as long as the editor.

**F133. Context menu: Paste.**
*Do:* right-click at the target time and row, Paste.
*Does:* enabled when press and release were the same point and the clipboard is non-empty (`MarksBar.cs:289-292`). (`MarksBar.cs:366-417`.) Group the clipboard by original collection, ordered by collection index. Target base row = the row under the press; none stops. `base time = time(press x)`, `offset` = first group's collection index. For each group: target index in **all** rows, shown or not = `baseRow + groupIndex - offset`; past the end skips this group. If the types differ: ask `Warning, {name} is of type {type} and\nthe target {name} is of type {type}. Proceed?`, title "Warning"; No stops the remaining groups. Clone each mark with `start = base time + (orig.start - earliest start in the whole clipboard)`; add to the collection. If any: raise pasted, select the pasted marks. **KEEP:** no clamp to the sequence.
*Undo:* "Added N Mark(s)". Host also rebuilds snap, re-renders (`Form.cs:3153-3165`).

**F134. Context menu and key: Delete.**
*Do:* right-click Delete, or the Delete key with the bar focused.
*Does:* enabled with a selection. Removes each selected mark, raises deleted (`MarksBar.cs:169-176`, `MarksBar.cs:303-305`, `MarksBar.cs:1006-1014`). **FIX Q31:** upstream leaves the deleted marks selected, so a second Delete records another undo entry and undoing both duplicates marks. Clear the selection, as the ruler does. **KEEP Q35:** the key is not gated on a selection, so Delete with nothing selected records "Removed 0 Marks".
*Rules:* the editor's Delete shortcut for effects runs first when any effect is selected, since that menu item is enabled only then (`TSE/TimedSequenceEditorForm.Designer.cs:687`, `Form.cs:2184`, `Menu.cs:192-196`). So with effects selected, Delete removes effects even if the bar has focus. Keep this order. Mac: both ⌫ and ⌦.
*Undo:* "Removed N Mark(s)".

**F135. Context menu: Phoneme.**
*Does:* shown when press and release are the same point on an unlocked mark and all selected marks are Phoneme type. After a separator: submenu "Phoneme" with REST, AI, E, ETC, FV, L, MBP, O, U, WQ (`MarksBar.cs:307-314`, `MarksBar.cs:942-971`). Sets all selected marks' text, raises text changed.
*Rules:* **KEEP Q42:** "all selected" is true of an empty selection. A ⌘-right-click that deselects the only selected mark therefore shows Phoneme, Breakdown Phrase and Breakdown Word together, and they act on nothing. Mixed types show none of the three. Only the first of these items adds a separator.
*Undo:* none.

**F136. Context menu: Breakdown Phrase.**
*Does:* shown as F135 when all selected are Phrase type (`MarksBar.cs:316-322`). Host (`Form.cs:2912-2939`): for each source collection, find or create the linked Word collection (`Form.cs:2999-3029`): an existing one has `LinkedMarkCollectionId` = the source's id and the wanted type, and the first is used. A new one is named `{parent} Word` (made unique, `Form.cs:3113`), with the parent's colour, the link, the type, `ShowMarkBar` on. Split `mark.Text.Trim()` on whitespace (empty entries kept); each word gets an equal share of the duration, integer ticks, consecutive. The 25 ms floor applies to each share (`Marks/Mark.cs:62-65`), so very short marks overrun. Re-render dependents. The add itself marks the sequence modified (`Form.cs:5644-5647`).
*Undo:* "Added N Mark(s)", if any were added.

**F137. Context menu: Breakdown Word.**
*Does:* shown as F135 when all selected are Word type (`MarksBar.cs:324-330`). Host (`Form.cs:2940-2989`): find or create the linked Phoneme collection, named `{parent} Phoneme`, or `{phrase} Phoneme` if the source is itself linked to a Phrase collection (`Form.cs:3011-3020`). Needs the standard phoneme dictionary, otherwise error "Unable to find Standard Phoneme Dictionary", titled "Error", and stops, keeping the new collection and any marks already added. Convert each word's text with punctuation removed; if unknown, ask the user for a mapping (`GetUserMappingForFailedWord`) or skip. Equal-share marks named by phoneme. **GUARD:** if a mapping is given but the conversion still returns nothing, upstream divides by zero (`Form.cs:2972`); skip the word.
*Undo:* as F136. Lip-sync conversion is another spec's.

**F138. Guides from marks.** Every mark move, drag or resize publishes alignment times to the grid and waveform, F76 and F143. Release turns them off.
One list of times, replaced by each publication. Publishers: the ruler hover (F108), the ruler drag (F110), the bar's move (F126) and resize (F127), and the resize indicator (F79). Cleared by: any mouse-up on the grid, whatever button (`Grid_Mouse.cs:204`), the ruler's leave and hover off a mark (`Ruler.cs:596`, `Ruler.cs:724`), the ruler's release after a real drag (`Ruler.cs:660`), and the bar's release (`MarksBar.cs:815`). The grid keeps the list even when told inactive, because inactive always comes with no times (`Grid.cs:575-579`). The grid repaints at most every 16 ms for it (`Grid.cs:79`, `Grid.cs:2423-2432`), and flushes at once when marks finish moving (`Grid.cs:596-599`).

**F139. Locked collections.** Marks in a locked collection are drawn hatched and cannot be pressed in the bar (`MarksBar.cs:192-193`, `MarksBar.cs:1268-1279`) or found on the ruler (`Ruler.cs:856-860`). Resize zone and pointer still appear on them because hover does not check (`MarksBar.cs:478`). **KEEP.**

### N. Waveform

**F140. Waveform visibility.**
*Does:* hidden until audio is set, hidden again when it is removed; a new audio replaces the old (`Waveform.cs:53`, `Waveform.cs:238-260`). Audio is chosen by the host (`Form.cs:1769-1770`, `Form.cs:1872`, `Form.cs:1954`).

**F141. Waveform drawing.**
*Does:* nothing if the visible start is past the media end (`Waveform.cs:290`), not even guides or the cursor. One column per pixel; columns limited to `[0, min(samples, floor(px(mediaDuration)))]`, widened by 1 at each side of the dirty rectangle (`Waveform.cs:125-134`). Background (120,120,120). `workH = H - 2 - H % 2`. Paint order: columns, guides, cursor (`Waveform.cs:313-342`).
- Half (default): `top = workH`, baseline at `workH`; line from baseline up by `top * max`.
- Full: `top = workH / 2`, baseline at `top`; a line up by `top * max`, and a second line by `top * min`, which goes down because min is negative (`Waveform.cs:297-323`).
Pen is a vertical gradient: top half (60,60,60) to (20,20,20) over `top` pixels; bottom half (20,20,20) to (60,60,60) over `top - 1` (`Waveform.cs:359-366`). Both gradients start at y 0 and tile, so the bottom one reads as starting again at about the baseline. The result: dark at the baseline, lighter towards the outer edges.
Peaks per column: min and max of interleaved samples in that column's window (`TSE`-independent, `Vixen.Modules/Media/Audio/Audio.cs:91-111`). `crates/vixen-audio/src/peaks.rs` provides this at any zoom. Upstream recalculates all columns of the whole file after a 125 ms settle on zoom (`Waveform.cs:55`, `Waveform.cs:164-216`, `Waveform.cs:267-270`). Not needed with `Peaks`.

**F142. Building message.**
*Does:* while peaks are not ready: "Building waveform....." in 10 pt black, 15 from the left, vertically centred (`Waveform.cs:346-352`). It replaces everything: no columns, guides or cursor while it shows (`Waveform.cs:292`, `Waveform.cs:342`). Upstream also shows it for good if the audio yields no samples. **FIX Q32:** upstream adds the scroll offset without translating, so the text is off-screen once scrolled. Show it at the viewport's left plus 15. Mac: while the clip decodes or `Peaks::compute` runs on a worker.

**F143. Alignment guides on the waveform.**
*Does:* while active, a yellow dashed `[2,2]` line through each time (`Waveform.cs:67-80`, `Waveform.cs:329-340`). Null or inactive clears (`Tests/WaveformAlignmentRenderingTests.cs:69-79`). Sources: F79, F108, F110, F126, F127. Upstream repaints only the old and new guide columns, a 5-wide strip per guide clipped to the view (`Waveform.cs:87-118`); on the Mac guides are a shape layer, so nothing beneath repaints.

**F144. Cursor on the waveform.** F91. Moves redraw only its layer.

**F145. Drag the waveform's height.**
*Do:* press in the bottom strip and drag.
*Does:* strip: `y` from `height - 6` to `height - 1`, up-down pointer; otherwise a hand (`Waveform.cs:147`). While left is held with that pointer and `y > 30`: `height = y + 1` (`Waveform.cs:141-144`). **KEEP:** the comment says 40 and the constant is 30 (`Waveform.cs:28`, `Waveform.cs:140`). The ruler's limit is 40.
*Rules:* the waveform has no press handling at all. There is no click to seek, no selection, no context menu. The resize test is only "left held and pointer is up-down", so a press elsewhere that drags into the strip starts a resize too (**KEEP**). At `y <= 30` with the button held the height stops following and the pointer goes back to the normal test.
*Default:* 50. *Persist:* profile `WaveFormHeight` (`Form.cs:371`, `Form.cs:5831`).

**F146. Reset waveform height.**
*Do:* double-click the bottom strip. *Does:* height 50 (`Waveform.cs:151-160`).

**F147. Lock Waveform Height.**
*Do:* View > "Lock Waveform Height" toggle. *Does:* disables F145, F146 and the strip's pointer (`Waveform.cs:42`, `TimelineControl.cs:357-367`, `Menu.cs:221-224`). *Tests:* `Tests/WaveformLockHeightTests.cs`.
*Default:* off. *Persist:* profile `LockWaveformHeight` (`Form.cs:374`).

**F148. Full Waveform.**
*Do:* View menu toggle.
*Does:* Half or Full style (`Menu.cs:209-214`, `Waveform.cs:234`).
*Default:* Half. *Persist:* profile `FullWaveform` (`Form.cs:357`).

### O. Keyboard summary

**F149. Keys handled by the editor that act on the timeline.** All in `Hotkeys.cs`, except menu shortcuts from `TSE/TimedSequenceEditorForm.Designer.cs:403-890`. The form sees keys first (`KeyPreview`, `TSE/TimedSequenceEditorForm.Designer.cs:2346`); nothing runs while `IgnoreKeyDownEvents` is set (`Hotkeys.cs:58`). Mac: dispatch from the window with the same priority, skip while a text field is first responder.

| Key on the Mac | Action | Lines |
|---|---|---|
| Home; ⌘Home | Vertical top; time start | 68-73 |
| End; ⌘End | Vertical bottom; `VisibleTimeStart = Total - span` | 75-80 |
| PageUp; ⌘PageUp | Up half height; left half span | 82-87 |
| PageDown; ⌘PageDown | Down half height; right half span | 89-94 |
| ↑ ↓ | F28 | 96-110 |
| Space; ⌘Space; ⇧Space | Play or stop; play or pause; add mark at position (note: ⌘Space is the system Spotlight shortcut, so offer the same action on another key too) | 112-121, 8-26 |
| ⌘← ⌘→ | F87 | 123-137 |
| S E B C U R N | F80, not during a drag | 139-277 |
| ⌘⇧S ⌘⇧E ⌘⇧B | F83 | 144-148, 160-164, 177-181 |
| I | Distribute dialog, same preconditions as F80 | 279-289 |
| D; ⌘D | Distribute equally; split F84 | 216-225 |
| T; ⌘T | F29 | 212-214 |
| Esc | F64 and F77 | 190-197 |
| ⌘-; ⌘= | Zoom time out 1.25; in 0.8 | 199-211 |
| ⌘⇧-; ⌘⇧= | Zoom rows out 0.8; in 1.25 | 199-211 |
| Numpad ⌘+, ⌘-, ⌘⇧+, ⌘⇧- | The same, through the View menu's shortcuts | Designer 811, 820, 842, 852 |
| ⌘A | F56 | Designer 678 |
| ⌫, ⌦ | Delete effects, or marks F134 | Designer 687 |
| ⌘I | Show Effect Info, F45 | Designer 890 |
| F5, F8, F9 | Play, stop, loop (sequencer's) | 291-304 |

Modifiers: Home, End, PageUp, PageDown, ⌘← and ⌘→ test "holding ⌘" and ignore the rest. S, E and B test ⌘⇧ together first. S and E then pass "holding ⇧" as the hold flag; B has none. U, R and N pass "exactly ⇧" as the hold flag. C and I test nothing. T and D test only ⌘. So upstream's Ctrl+E, Ctrl+B, Ctrl+U, Ctrl+R and Ctrl+N run the plain command. Menu shortcuts take their keys first (Ctrl+C is Copy, Ctrl+S Save, Ctrl+I Show Effect Info). Those Ctrl forms are accidents. On the Mac bind the plain and ⇧ forms only, because ⌘N, ⌘R and others belong to the menu bar. Home, End, PageUp and PageDown are fn-arrows on a Mac laptop, and that needs no extra work. ⌘-PageUp and ⌘-PageDown step by half the grid's span.

### O2. Added by the audit

**F150. Pointer shape in each grid state.**
*Does:* upstream sets the pointer in these places and nowhere else, so it stays put between them:

| State | Pointer | Cite |
|---|---|---|
| Normal | Recomputed on every move, F65. A modifier key alone does not update it until the next move | `Grid_Mouse.cs:489` |
| Waiting, Selecting, Resizing | Unchanged from the last hover, so arrow or crosshair, and left-right in a zone | none set |
| Moving | Move (`SizeAll`), from the threshold on | `Grid_Mouse.cs:714` |
| Drawing from Draw Mode | Unchanged, so crosshair | none set |
| Drawing from the middle button | Crosshair, set at press | `Grid_Mouse.cs:63` |
| Any drag ended | Arrow until the next move | `Grid_Mouse.cs:1205`, `Grid_Mouse.cs:215` |
| Effect type deselected or Esc | Arrow | `TSE/Forms/Form_Effects.cs:181` |

Mac: `NSCursor` per the mapping table in SwiftUI and AppKit structure. For F116, F127 and F145 see those entries.

**F151. Drawing failures.**
*Does:* each surface catches exceptions from its paint.
- Grid: logs, then shows "An unexpected error occurred while drawing the grid. Please notify the Vixen team and provide the error logs.", titled "Error", once, and not again until a paint succeeds (`Grid.cs:3190`, `Grid.cs:3195-3208`). A failed image draw for one effect logs the effect name, start, end, rendered flag, image size and destination, and ends that paint at once with no dialog, so later rows and the overlays are missing until the next paint (`Grid.cs:3011-3021`, `Grid.cs:3068-3071`, `Grid.cs:3181-3184`).
- Ruler: `Exception in Timeline.Ruler.OnPaint():\n\n\t<message>\n\nBacktrace:\n\n\t<stack>`, titled "Error", on every failure (`Ruler.cs:109-115`). Marks bar: the same with `Timeline.MarksBar.OnPaint()` (`MarksBar.cs:1182-1187`).

**FIX Q45:** the Mac's frames are built by Rust, which must not panic. A failed frame logs and leaves the last good content. Show the grid's once-only alert, with the grid's text, for a failure on any surface, and no stack traces.

**F152. Undo and redo of marks.**
*Does:* moves and resizes: undo swaps each mark's start and duration with the saved values, start first, so the setters' 25 ms rule applies (`LM/MarkTimeInfo.cs:23-33`, `Form.cs:5526-5537`). It then publishes marks moving, which re-sorts and updates snap points incrementally (F75), and re-renders dependents. It does not mark the sequence modified (**KEEP**). Redo first drops any mark its collection no longer contains (`TSE/Undo/MarksTimeChangedUndoAction.cs:29-46`). Labels: Q23.
Adds and removes: `MarksAddedUndoAction` and `MarksRemovedUndoAction` hold mark and collection pairs and add or remove them on undo and redo, then re-render and mark modified (`TSE/Undo/MarksAddedRemovedUndoAction.cs:17-25`, `Form.cs:3628-3652`). Labels "Added N Mark(s)" and "Removed N Mark(s)", singular for one (`TSE/Undo/MarksAddedUndoAction .cs:29`, `TSE/Undo/MarksRemovedUndoAction.cs:28`). An undone add leaves the mark in the selection if it was selected.
Text edits have no undo (F130, F135). Bulk moves have their own entry, `Bulk Change N Mark(s)` (`TSE/Undo/MarksBulkChangeUndoAction.cs:53-60`), owned by the sequencer spec's bulk move.
*Rules:* mark and effect entries share one stack with the rest of the editor. Mac: register them with the window's `UndoManager` so the Edit menu names them.

**F153. What marks the sequence modified.**
*Does:* two separate flags. *Modified* prompts to save at close. The *editor state* flag saves silently at close when not modified, and is never reset once set (`Vixen.Application/VixenApplication.cs:851-866`, `Form.cs:108`, `Form.cs:5583-5586`).

| Action | Modified | Editor state | Cite |
|---|---|---|---|
| Effect time change, each drag move and undo included | Yes | | `Form.cs:2204-2208` |
| Effect changes rows, drawn, cloned | Yes | | `Form.cs:2250`, `Form.cs:2287`, `Form.cs:3600` |
| Ruler click and ⌘-click | Yes | | `Form.cs:2887` |
| Ruler range drag, Start and End buttons | | | `Form.cs:3168-3199`, `Toolstrip.cs:1595-1616` |
| Marks moved or resized, on release | Yes | | `Form.cs:3136` |
| Undo or redo of a mark move | | | F152 |
| Mark text, add, remove, paste; any collection or decorator property change | Yes | | `Form.cs:3127`, `Form.cs:3086`, `Form.cs:3150`, `Form.cs:3164`, `Form.cs:5644-5647` |
| Zoom, scroll, playback follow | | Yes | `Form.cs:2368-2384` |
| Row height, outside loading | | Yes | `Form.cs:2294-2300` |
| Expand, collapse, row selection, active row, cursor, mark selection, effect selection | | | none |

So an expanded row alone is not saved at close, and neither is a range dragged on the ruler (**KEEP**). `RowSettings`, row height and the range are written at any save (Data and persistence). Sequencer spec F16 owns the close prompt.

**F154. Keyboard focus.**
*Does:* the window sees keys first (F149). Then: a press on the grid or a row label gives that control focus through `UserControl`'s base handler (framework behaviour, not visible in this source; `Grid_Mouse.cs:46`, `RowLabel.cs:111`). The marks bar takes it explicitly on any button because it does not call the base (`MarksBar.cs:185`). The ruler does not call the base and has no focus call, so a press there leaves focus alone (`Ruler.cs:458-496`). The waveform has no press handler, so it gets the base behaviour.
Two commands depend on this: Esc cancels a drag only with the grid focused (F64), and the Delete key deletes marks only with the bar focused (F134), so a press on the grid or the ruler decides what Delete does next.
Mac: `acceptsFirstResponder` true on the grid, row header, marks bar and waveform, false on the ruler. A press makes the surface first responder.

**F155. Mark selection.**
*Does:* one ordered list per editor, shared by bar and ruler, no duplicates, in the order selected (`LM/MarksSelectionManager.cs:41-90`). Not persisted. Selecting null is ignored. Cleared by: any non-right press on the grid (`Form.cs:2187-2195`), a ruler press off a mark and off the strip (`Ruler.cs:493`), a ruler press on a mark without exactly ⌘ (`Ruler.cs:470-473`), a bar press on an unselected mark or blank with no modifier (`MarksBar.cs:212-220`), ruler delete (`Ruler.cs:711`). Replaced by a paste, with the pasted marks (`MarksBar.cs:411-416`). A selected mark draws bold in the bar (F119) and 3 wide on the ruler (F106), and has no special look on the grid. Moves, resizes, edit text, phonemes, cut, copy and delete act on this list, not on the pressed mark.

### P. Quirk register

Everything marked FIX is a deliberate divergence, for the owner's veto. All other tags are as defined in Conventions.

| ID | Tag | Where | Summary |
|---|---|---|---|
| Q1 | FIX | `TimelineControlBase.cs:149` | Float32 pixel mapping replaced by f64 |
| Q2 | FIX | `TimelineControl.cs:1239-1253` | ⌘⌥ wheel hit test ignores scroll and marks bar |
| Q3 | FIX | `Grid.cs:2963-3003`, `Element.cs:103-113` | Per-occurrence geometry computed outside paint |
| Q4 | KEEP | `Row.cs:511-535` | Stack count depends on the scroll position |
| Q5 | FIX | `Grid_Mouse.cs:71-81`, `Grid_Mouse.cs:274` | Right press in Draw Mode strands the Drawing state |
| Q6 | FIX | `Grid.cs:300-312`, `TimelineControl.cs:653-658` | Select All leaves the selected list stale |
| Q7 | KEEP | `Grid_Mouse.cs:873-882` | Move: snap after clamp can exceed bounds |
| Q8 | KEEP | `Grid_Mouse.cs:377-389` | Freeze rules apply in every state |
| Q9 | GUARD | `Grid.cs:2274-2276` | Off-grid move throws; skip instead |
| Q10 | FIX | `Grid_Mouse.cs:561-575` | Esc leaves half state and rows unrestored |
| Q11 | KEEP | `Grid.cs:2055` | Row-scoped candidate test uses `> 0`, so flat row 0 acts unscoped |
| Q12 | KEEP | `Grid.cs:2084` | Equal level: first found wins, not nearest |
| Q13 | KEEP | `Grid.cs:2109` | Both-edges comparison has odd grouping |
| Q14 | FIX | `Grid.cs:3132-3143` | Guides raised in paint, 16 ms repaint loop |
| Q15 | KEEP | `Grid.cs:1256` | Centre align skips on equal start |
| Q16 | KEEP | `Grid.cs:57`, `Grid.cs:980` | Warning text says 4, limit is 32 |
| Q17 | KEEP | `Grid.cs:1269-1280` | Split makes many undo entries |
| Q18 | KEEP | `Grid.cs:1521-1522` | Clip duration uses zeroed start |
| Q19 | KEEP | `Waveform.cs:282-285` | No playback line on the waveform |
| Q20 | KEEP | `Ruler.cs:890`, `Ruler.cs:913` | Bold ignored on the ruler |
| Q21 | KEEP | `Ruler.cs:858` | Mark hit is ±20 ms at any zoom |
| Q22 | KEEP | `Ruler.cs:542-545` | Ruler mark drag jumps to the pointer, unclamped |
| Q23 | KEEP | `TSE/Undo/MarksTimeChangedUndoAction.cs:55-58` | Mark undo labels say "effect" |
| Q24 | KEEP | `MarkRow.cs:121` | Marks bar height follows the view |
| Q25 | FIX | `MarksBar.cs:1197-1200` | Marks bar geometry computed in paint |
| Q26 | KEEP | `MarkRow.cs:96-101` | Struct copy loses a second cluster's values |
| Q27 | KEEP | `MarksBar.cs:895-902` | ⇧ range can select locked marks |
| Q28 | KEEP | `MarksBar.cs:703`, `MarksBar.cs:730` | Glued width check runs the wrong way |
| Q29 | FIX | `MarksBar.cs:809` | Glued neighbour missing from undo |
| Q30 | KEEP | `MarksBar.cs:422` | Mark clipboard holds references |
| Q31 | FIX | `MarksBar.cs:1006-1014` | Delete leaves deleted marks selected |
| Q32 | FIX | `Waveform.cs:349` | Building text position ignores scroll |
| Q33 | KEEP | `Grid_Mouse.cs:320-332` | Hover info stays when the pointer leaves the grid |
| Q34 | KEEP | `Grid_Mouse.cs:1150-1156`, `MarksBar.cs:796-800` | A click in a resize zone with no movement records a Resize undo entry |
| Q35 | KEEP | `Grid_Mouse.cs:553-555`, `Grid.cs:1372-1373`, `MarksBar.cs:1013`, `MarksBar.cs:817` | Undo entries are recorded for zero effects or marks ("Moved 0 effects", "Start Align 0 effects", "Resize 0 effects", "Removed 0 Marks") |
| Q36 | KEEP | `TimelineControl.cs:476` | `rowHeight` has no floor and can stick at 0 |
| Q37 | KEEP | `Grid.cs:324`, `Grid.cs:854-862` | First selected row survives clearing the row selection |
| Q38 | GUARD | `Grid_Mouse.cs:715`, `Grid.cs:2198` | Threshold crossed below the last row leaves tracked row -1, later throws |
| Q39 | GUARD | `Grid.cs:2205-2211` | No moving effect in a visible row: limit -1 flips an upward move to +1 |
| Q40 | FIX | `Grid_Mouse.cs:77`, `Grid_Mouse.cs:234`, `Form.cs:2276` | Draw box times are not clamped to the sequence |
| Q41 | KEEP | `Grid.cs:1323` | Close Gaps on a selection walks selection order |
| Q42 | KEEP | `MarksBar.cs:309-330` | Empty selection counts as "all of type": all three type items show |
| Q43 | KEEP | `Ruler.cs:657` | Ruler mark drag back to the press X keeps the jump with no undo entry |
| Q44 | KEEP | `Ruler.cs:589-592` | A guide stays up moving from a mark into the height strip |
| Q45 | FIX | `Grid.cs:3195-3208`, `Ruler.cs:109-115`, `MarksBar.cs:1182-1187` | Paint failure reporting replaced by one logged, once-only alert |
| Q46 | FIX | `Grid.cs:208-209` | Start is not re-clamped when the sequence shortens |
| Q47 | FIX | `Grid_Mouse.cs:154-179`, `Grid_Mouse.cs:251-263`, `Form.cs:2826-2842` | Overlap chooser: one offer per left press, none on a right press |
| Q48 | FIX | `Ruler.cs:548-561`, `Ruler.cs:933` | Ruler drag label hidden until the first move, instead of the last drag's value |

Other GUARD items: `Rows.ElementAt(0)` with no rows (`TimelineControl.cs:489`, `TimelineControl.cs:1107`); null `SelectedRow` (`TimelineControl.cs:1125`); depth 20 (`RowList.cs:206-208`); dash pattern with Level 0 (`Grid.cs:2758`, `Ruler.cs:894`); `TimePerPixel` zero; `grid.Width` zero (`TimelineControl.cs:457`); `RowMembers.First()` on an empty list (`Grid_Mouse.cs:601`); saved-list index for a missing effect (`Grid.cs:2074`); dictionary add of a duplicate mark (`MarkRow.cs:114`); painting the waveform with no audio (`Waveform.cs:290`); working effect gone during the resize indicator (`Grid.cs:3133`, `Grid.cs:3143`); negative vertical limit when rows fit (`Grid.cs:263-264`); `Rows.IndexOf`-style negative tracked row in the snap candidates (`Grid.cs:2055`, harmless, it reads as unscoped); phoneme share with an empty result (`Form.cs:2972`); mark collections list unset on the ruler (`Ruler.cs:860`); duplicate key in `Dictionary.Add` for row settings at save (`Form.cs:1629`, `Form.cs:1633`).

Open questions for the owner, not tags: F33 keeps upstream's rule that a plain right-click collapses a multi-selection of rows to the clicked row. Finder keeps the selection if the clicked item is in it. F149 does not carry over upstream's accidental Ctrl forms of the single-key commands (Ctrl+U and so on), because ⌘N, ⌘R and others belong to the menu bar.

## Data and persistence

**Per sequence, inside the `.tim`** (`crates/vixen-io/src/sequence.rs:150-157` already reads and writes these; upstream `Form.cs:1613-1636`). Read at load (`Form.cs:1325-1332`, `Form.cs:1343-1378`, `Form.cs:1515-1523`, `Form.cs:563-567`). Written when the sequence is saved, by `SaveGridRowSettings` immediately before the save (`Form.cs:1581`, `Form.cs:1593`, `Form.cs:1599`), except `TimePerPixel` and `VisibleTimeStart`, which are copied on every change (`Form.cs:2368-2384`). Closing saves without asking if only the editor state flag is set (F153).

| Field | Meaning | Rule |
|---|---|---|
| `TimePerPixel` | Zoom | Applied only when above zero. A new sequence holds `TimeSpan.MinValue` |
| `VisibleTimeStart` | Scroll | Applied only when above zero |
| `DefaultRowHeight` | `rowHeight` | 0 means 32 |
| `DefaultPlaybackStartTime`, `DefaultPlaybackEndTime` | Ruler range, nullable | Applied when the end is not exactly zero (null counts as not zero) |
| `RowSettings` | Map TreeId to `{RowHeight, Expanded, Visible}` | Cleared and rebuilt on save, see below |
| `DefaultSplitterDistance` | Unused: the file carries it (`Vixen.Modules/Sequence/Timed/TimedSequenceData.cs:29`), and the editor reads the profile setting instead | Preserve on round trip, never apply |
| `LabeledMarkCollections` | Marks, in collection order | Owned by the marks model |

**`RowSettings` save rule** (`Form.cs:1622-1635`). For each row: if height differs from `rowHeight` by more than 7 either way, save `{height, TreeOpen, Visible}`. Else if `TreeOpen`, save `{height, true, Visible}`. Else nothing.

**TreeId** (`Row.cs:130-162`). `TreePathName = Name + parent.TreePathName()` with nothing between. `TreeId = MD5(UTF-8 of that)` as uppercase hex with no separators. Must be identical so that existing files keep their row state. It is not unique across same-named siblings: **KEEP**, a later row overwrites at save (upstream uses `Dictionary.Add`, which would throw; **GUARD:** last write wins).

**Per user, profile settings** (`XMLProfileSettings`, `AppSettings` section, key `TimedSequenceEditorForm/<name>`, `Form.cs:347-376`, `TSE/TimedSequenceEditorForm.Designer.cs:2349`). Read at editor open, written at close (`Form.cs:5810-5840`). On the Mac, use the app's settings store under the same names so the settings import can map them.

| Key | Default | Feature |
|---|---|---|
| `SnapToSelected` | true | F68 |
| `SnapStrength` | 2 | F68 |
| `CloseGapThreshold` | ".100" | F85 |
| `ResizeIndicatorEnabled` | false | F79 |
| `ResizeIndicatorColor` | "Red" | F79 |
| `DrawModeSelected` | false | F77 |
| `SelectionModeSelected` | true | F77 |
| `CadStyleSelectionBox` | false | F52 |
| `LegacyCursorActiveRow` | false | F93 |
| `ZoomUnderMousePosition` | false | F16 |
| `HighlightActiveElements` | false | F23 |
| `WaveFormHeight` | 50 | F145 |
| `RulerHeight` | 50 | F113 |
| `LockRulerHeight` | false | F115 |
| `LockWaveformHeight` | false | F147 |
| `ShowEffectInfo` | true | F45 |
| `FullWaveform` | false | F148 |
| `SplitterDistance` | 200 | F2 |

**Not persisted:** edit cursor, selection, active row, mark selection, the mark clipboard, Drag Box Filter, Show Hidden Rows.

**Elsewhere:** tags go to the system config immediately (F37).

**Data to reconcile in `vixen-core` and `vixen-io`.** These crates are out of bounds for this spec's author, but timeline behaviour depends on them. State as of the audit:

| Rust today | Upstream | Effect |
|---|---|---|
| Fixed: `Mark::new` is 450 ms (`crates/vixen-core/src/marks.rs:17-19`) | 450 ms (`Marks/Mark.cs:21`) | None now |
| Fixed: `MarkCollection::default` is "Mark Collection", `show_grid_lines` true, `show_marks` false, `show_tail_lines` false, level 1 (`crates/vixen-core/src/marks.rs:67-84`) | The same (`Marks/MarkCollection.cs:28-37`) | None now |
| Fixed: `MarkCollectionType::State` exists and `vixen-io` reads and writes it; other unknown names read as Generic (`crates/vixen-core/src/marks.rs:28-35`, `crates/vixen-io/src/sequence.rs:825-830`, `crates/vixen-io/src/sequence.rs:842-847`) | `State = 4` (`Vixen.Core/Marks/IMarkType.cs:31`, `Tests/MarkCollectionTypeTests.cs:12-19`) | None now |
| `vixen-core` has no duration floor, its fields are public. `vixen-session::timeline::marks` applies the floor on its own edit paths (`crates/vixen-session/src/timeline/marks.rs:26-47`) | 25 ms on both setters (`Marks/Mark.cs:45-48`, `Marks/Mark.cs:62-65`) | Any other writer to a mark must go through those helpers or F127 differs |
| `vixen-core`'s `add_mark` and `sort_marks` order by start only (`crates/vixen-core/src/marks.rs:88-95`). `vixen-session::timeline::marks::ensure_order` has the full order (`crates/vixen-session/src/timeline/marks.rs:48`) | Start then end (`Marks/Mark.cs:76-85`); `List.Sort`, unstable (`Vixen.Core/Extensions/CollectionExtensions.cs:46-55`) | Stack lanes differ for marks with equal starts if anything other than `ensure_order` sorts them. The core helpers should take the same order, by that crate's owner |

## Rust API needed

All in the module `vixen-session::timeline` unless stated, exposed through `crates/vixen-ffi/src/timeline.rs` as a UniFFI object owned per sequence window. Signatures only. Pure functions are free functions so they can be tested without a session. Bulk results are returned per redraw, per FFI call, never per row.

**State.** That module now exists and is being written from this spec (`crates/vixen-session/src/timeline/`), with document calls behind a `TimelineHost` trait (`host.rs`). The signatures below are the plan. Where the code has settled on something else, the code wins, and the behaviour in Features still binds. The lines marked *(audit)* are not obviously there yet.

**Ids and small types**

- `struct ElementId(u64)`: one per effect node, the same in every row.
- `struct RowId(u32)`: one per row occurrence.
- `struct Mods { shift: bool, cmd: bool, opt: bool }`: with `exactly()` and holding tests.
- `enum Button { Left, Right, Middle }`.
- `struct IRect { x: i32, y: i32, w: i32, h: i32 }`.
- `enum ResizeZone { None, Front, Back }`.
- `enum DragState { Normal, Waiting, Moving, Selecting, HResizing, Drawing }`.
- `enum ElementMoveType { Move, Resize, AlignStart, AlignEnd, AlignBoth, AlignDurations, AlignStartToEnd, AlignEndToStart, AlignCenters, Distribute }`.
- `enum CursorKind { Arrow, ResizeLeftRight, ResizeUpDown, Crosshair, Move, Hand }`.
- `struct ElementTimes { start: TimeSpan, duration: TimeSpan }`: with `end()`.

**Time mapping (pure)**

- `struct TimeInfo { time_per_pixel, visible_start, total, playback_start: Option<_>, playback_end: Option<_>, playback_current: Option<_> }`: F4.
- `fn px(t: TimeSpan, tpp: TimeSpan) -> f64`: F5, returning 0 when `tpp` is 0.
- `fn time_at(px: i32, tpp: TimeSpan) -> TimeSpan`.
- `fn visible_span(width_px: i32, tpp: TimeSpan) -> TimeSpan`.
- `fn visible_end(info: &TimeInfo, width_px: i32) -> TimeSpan`.
- `fn clamp_visible_start(v: TimeSpan, total: TimeSpan, span: TimeSpan) -> TimeSpan`: F6.

**Zoom and pan (pure)**

- `enum ZoomDecision { Rejected, Scaled(TimeSpan), ShowsTotal(TimeSpan) }`.
- `fn zoom_decision(tpp: TimeSpan, span: TimeSpan, total: TimeSpan, grid_width: i32, scale: f64) -> ZoomDecision`: F15.
- `fn zoom(info: &TimeInfo, grid_width: i32, scale: f64) -> Option<(TimeSpan, TimeSpan)>`: new tpp and start, left edge held, F15.
- `fn zoom_at(info: &TimeInfo, grid_width: i32, panel_width: i32, pointer_offset_px: i32, scale: f64) -> Option<(TimeSpan, TimeSpan)>`: F16.
- `fn wheel_zoom_scale(delta: i32) -> f64`: `1.0 - delta / 1200.0`.
- `fn wheel_pan(start: TimeSpan, span: TimeSpan, delta: i32) -> TimeSpan`: F11.
- `fn tilt_pan(start: TimeSpan, span: TimeSpan, delta: i32) -> TimeSpan`: F13.
- `fn vertical_wheel_delta(delta: i32) -> i32`: `-(delta / 3)`.
- `fn playback_follow(position: TimeSpan, start: TimeSpan, span: TimeSpan) -> Option<TimeSpan>`: F95.

**Rows (`RowTree` in the session)**

- `fn load_rows(&mut self, tree: &ElementTree, settings: &[(String, RowSetting)], default_height: i32)`: F18.
- `fn tree_id(&self, row: RowId) -> String`: exact MD5 form.
- `fn save_row_settings(&self) -> Vec<(String, RowSetting)>`: the ±7 rule.
- `fn set_tree_open(&mut self, row: RowId, open: bool)`, `fn toggle_subtree(&mut self, row: RowId, open: bool)`: F19, F29.
- `fn set_visibility_filter(&mut self, show_hidden: bool)`: F24.
- `fn visible_rows(&self, offset: i32, count: i32) -> Vec<RowDisplay>`: id, name, depth, height, top, expandable, open, has_effects, subtree_has_effects, selected, active, tag colours. Bulk.
- `fn row_tops(&self) -> Vec<i32>`: cumulative tops for the row header's binary search, returned once per layout change.
- `fn row_at(&self, canvas_y: i32) -> Option<RowId>`, `fn rows_in(&self, r: IRect) -> Vec<(RowId, i32)>`, `fn rows_within(&self, r: IRect) -> Vec<RowId>`: F44, F51, F77.
- `fn set_row_height(&mut self, row: RowId, h: i32)`: floor 10.
- `fn finish_row_resize(&mut self, row: RowId)`: same-name sync, F30.
- `fn zoom_rows(&mut self, scale: f64, grid_height: i32)`, `fn zoom_row_at(&mut self, canvas_y: i32, delta: i32)`, `fn reset_row_heights(&mut self)`, `fn reset_selected_row_heights(&mut self)`, `fn collapse_all(&mut self)`: F31, F32, F34-F36.
- `fn click_row_label(&mut self, row: RowId, mods: Mods) -> SelectionDelta`: F25.
- `fn row_label_zone(&self, row: RowId, x: i32, y: i32, width: i32) -> LabelZone`: expander, label, resize bar. F19, F25, F30. Upstream's are computed in paint.
- `fn move_active_row(&mut self, direction: i32) -> bool`, returning the new offset if it scrolled: F28.
- `fn paste_target_row(&self) -> Option<RowId>`, `fn add_effect_target_row(&self) -> Option<RowId>`: F27, the two different orders. *(audit)*
- `fn connector_lines(rows: &[RowDisplay], inset: i32) -> Vec<((i32, i32), (i32, i32))>`: F22's walk, pure, for the row header's draw pass. *(audit)*
- `fn toggle_selected_rows(&mut self, include_children: bool)`: F29.
- `fn tag_menu_state(&self) -> Vec<TagMenuItem>`, `fn toggle_tag_on_selected(&mut self, tag: Uuid, cascade: bool)`: F37.

**Stacking and layout (pure, per row)**

- `fn stack_row(elements: &[ElementTimes], range_start: TimeSpan, range_end: TimeSpan) -> Vec<StackSlot>`: F40, indexed like the input, `{ index, count }`.
- `fn occurrence_rect(times: ElementTimes, slot: StackSlot, row_top: i32, row_height: i32, info: &TimeInfo, vis_end: TimeSpan) -> OccurrenceRect`: F39, with x, y, w, h, row_top_offset, red_border, overall_width.
- `struct GridFrame { rows: Vec<RowQuad>, elements: Vec<ElementQuad>, mark_lines: Vec<MarkLine>, guides: Vec<TimeSpan>, canvas_width: i64, canvas_height: i64 }`: F10.
- `fn grid_frame(&mut self, viewport: IRect, width_px: i32) -> GridFrame`: rows and occurrences intersecting the viewport only. `ElementQuad` carries id, row, rect, selected, rendered, left and right edge visible, slice offsets, content generation.

**Hit testing (read-only)**

- `fn elements_at(&self, canvas: (i32, i32)) -> Vec<ElementId>`, `fn element_at(&self, canvas: (i32, i32)) -> Option<ElementId>`: F44.
- `fn resize_zone(times: ElementTimes, tpp: TimeSpan, x: i32) -> ResizeZone`: F65, also used for marks with the same constant 12.
- `fn elements_within(&self, start: TimeSpan, end: TimeSpan, visible_only: bool) -> Vec<ElementId>`, `fn elements_at_time(&self, t: TimeSpan) -> Vec<ElementId>`.

**Grid input state machine**

- `struct PointerEvent { x: i32, y: i32, button: Option<Button>, clicks: u8, mods: Mods }`: client coordinates.
- `struct InputOutcome { cursor: CursorKind, dirty: DirtyRegion, selection_changed: bool, view: Option<ViewChange>, autoscroll: bool, requests: Vec<HostRequest> }`: `autoscroll` means run the 50 ms tick.
- `enum HostRequest { ChooseOverlapping { elements, at }, ContextMenu { elements, time, row }, Warning { text, title }, DrawEffects { effect_type, rows, start, duration }, CloneForDrag { elements }, UndoRecorded { label }, HoverInfo(Option<InfoText>), Guides(Vec<TimeSpan>) }`.
- `fn grid_mouse_down(&mut self, ev: PointerEvent) -> InputOutcome`: F46, F77, F78.
- `fn grid_mouse_move(&mut self, ev: PointerEvent) -> InputOutcome`: F45, F58-F67, F51, F77, F79.
- `fn grid_mouse_up(&mut self, ev: PointerEvent) -> InputOutcome`: F49, F54, F58, F66.
- `fn grid_autoscroll_tick(&mut self) -> InputOutcome`: F63, replays the last pointer.
- `fn grid_cancel_drag(&mut self) -> InputOutcome`: F64, restoring times and rows.
- `fn grid_double_click(&mut self, ev: PointerEvent) -> Option<ElementId>`: F55.
- `fn is_drag_in_progress(&self) -> bool`: `IsResizeDragInProgress`, which is any state other than Normal (`Grid.cs:413`), used by hotkeys.
- `fn finish_chooser(&mut self, chosen: Option<ElementId>) -> InputOutcome`: F50. Ends the press once the Mac's modal menu returns, with no release path. A right press never asks for the chooser. *(audit)*
- `fn hover_info_text(name: &str, layer: &str, times: ElementTimes) -> String`: F45's exact form. *(audit)*
- `fn clamp_draw_times(press: TimeSpan, release: TimeSpan, total: TimeSpan) -> (TimeSpan, TimeSpan)`: F77, Q40; start and duration. *(audit)*

**Selection**

- `fn selected_elements(&self) -> Vec<ElementId>`: F57's order.
- `fn select_element(&mut self, e: ElementId)`, `fn select_elements(&mut self, e: &[ElementId])`, `fn deselect_element(&mut self, e: ElementId)`, `fn toggle_element(&mut self, e: ElementId)`, `fn select_all(&mut self)`, `fn clear_selection(&mut self)`: with cursor rules from F92.
- `struct DragBoxFilter { enabled: bool, types: HashSet<Uuid> }`, `fn set_drag_box_filter(&mut self, f: DragBoxFilter)`: F53.

**Snapping (pure core, with session state)**

- `struct SnapDetail { time: TimeSpan, level: i32, row: Option<RowId>, edge: Option<SnapEdge>, color: u32, bold: bool, solid: bool }`.
- `fn snap_window(time: TimeSpan, tpp: TimeSpan, level: i32, strength: i32) -> (TimeSpan, TimeSpan)`: F70.
- `fn build_drag_snap(rows: &RowTree, selected: &HashSet<ElementId>, marks: &MarkSnapIndex, tpp: TimeSpan, strength: i32) -> DragSnap`: F69.
- `fn find_snap(snap: &DragSnap, candidates: &[(ElementTimes, RowId)], offset: TimeSpan, zone: ResizeZone) -> TimeSpan`: F71 and F74. Use a range query so results match a full scan in time order.
- `fn snap_candidates(&self, selected: &[ElementId], tracked_row: i32) -> Vec<(ElementTimes, RowId)>`: F71 step 2, including Q11.
- `struct MarkSnapIndex`: `fn rebuild(&mut self, collections: &[MarkCollection], strength: i32)`, `fn update_marks(&mut self, ids: &[Uuid], collections: &[MarkCollection])`: F75. Keyed by mark id, not time.
- `fn mark_lines(index: &MarkSnapIndex, visible_start: TimeSpan, visible_end: TimeSpan, guides: &[TimeSpan], tpp: TimeSpan) -> Vec<MarkLine>`: F76.

**Move, resize, align**

- `fn move_vertically(&mut self, selected: &[ElementId], canvas_y: i32) -> Vec<RowChange>`: F61. Host retargeting lives in the session too.
- `fn adjoining(&self, primary: ElementId, zone: ResizeZone, same_layer: Option<i32>) -> Vec<ElementId>`: F67, with limit 20.
- `fn align(&mut self, kind: AlignKind, elements: &[ElementId], reference: Option<ElementId>, hold: bool) -> AlignResult`: F80, returning `TooManyPerRow` for the warning.
- `fn move_resize_batch(&mut self, moves: &[(ElementId, TimeSpan, TimeSpan)], kind: ElementMoveType)`: F81.
- `fn ok_to_use_alignment(&self, elements: &[ElementId]) -> bool`: limit 32.
- `fn split_at(&mut self, elements: &[ElementId], t: TimeSpan)`: F84.
- `fn close_gaps(&mut self, threshold_seconds: f64) -> bool`: F85, false when it needs confirmation first.
- `fn move_by_time(&mut self, elements: &[ElementId], offset: TimeSpan, clip: bool) -> TimeSpan`, `fn move_in_range_by_time(&mut self, start: TimeSpan, end: TimeSpan, offset: TimeSpan, visible_only: bool, clip: bool) -> TimeSpan`: F86, F87.
- `fn swap_element_placement(&mut self, entry: &mut MoveUndoEntry)`: F88.
- `fn reveal_elements(&mut self, elements: &[ElementId]) -> ViewChange`: F90.

**Ruler (pure)**

- `struct RulerScale { minor: TimeSpan, minor_per_major: i64 }`, `fn ruler_scale(tpp: TimeSpan) -> RulerScale`: F98.
- `fn ruler_label(t: TimeSpan, minor: TimeSpan) -> String`: F99, the exact table.
- `fn first_tick_px(start_px: f64, step_px: f64) -> f64`: F98, F9.
- `fn label_times(visible_start: TimeSpan, visible_end: TimeSpan, major: TimeSpan, tpp: TimeSpan, end_label_width: f64) -> Vec<TimeSpan>`: F99. Swift measures text and applies the overlap rule.
- `fn mark_drag_label(t: TimeSpan) -> String`: F110.

**Ruler input**

- `fn marks_at(collections: &[MarkCollection], t: TimeSpan) -> Vec<Uuid>`: F107.
- `fn ruler_mouse_down(&mut self, ev: PointerEvent) -> RulerOutcome`, `fn ruler_mouse_move(&mut self, ev: PointerEvent) -> RulerOutcome`, `fn ruler_mouse_up(&mut self, ev: PointerEvent) -> RulerOutcome`: F100-F112. `RulerOutcome` carries cursor, guides, playback range, marks moved, and requests such as `Clicked { time, mods, button }`, `RangeBegun`, `RangeFinished(mods)`, `MarkMenu`.
- `fn add_mark_at(&mut self, t: TimeSpan, fill_gap: bool, label: Option<String>) -> AddMarkResult`: F111, returning `NeedsNewCollection`, `NeedsEnableDefault` when a prompt is required.
- `fn delete_selected_marks(&mut self)`: F112, F134.
- `fn swap_mark_placement(&mut self, entry: &mut MarkMoveUndoEntry)`: F152, start first then duration through the 25 ms rule, re-sort and incremental snap update, not marking modified. Redo drops marks no longer in their collection. *(audit)*
- `fn mark_selection(&self) -> Vec<Uuid>`: F155, in selection order. *(audit)*

**Marks bar**

- `struct MarksBarFrame { total_height: i32, rows: Vec<MarkRowQuad>, marks: Vec<MarkQuad> }`, `fn marks_bar_frame(&mut self, width_px: i32) -> MarksBarFrame`: F117-F119. Height is computed here, not in paint.
- `fn mark_at(&self, canvas: (i32, i32)) -> Option<Uuid>`: F120.
- `fn marks_mouse_down(&mut self, ev: PointerEvent) -> MarksOutcome`, `fn marks_mouse_move(&mut self, ev: PointerEvent) -> MarksOutcome`, `fn marks_mouse_up(&mut self, ev: PointerEvent) -> MarksOutcome`, `fn marks_autoscroll_tick(&mut self) -> MarksOutcome`: F121-F129. Outcomes include `PlayRange`.
- `fn autoscroll_distance(x: i32, width: i32) -> i32`, `fn autoscroll_visible_start(start: TimeSpan, outside: i32, tpp: TimeSpan, total: TimeSpan, span: TimeSpan) -> TimeSpan`: F129.
- `fn marks_context_menu(&self, press: (i32, i32), release: (i32, i32)) -> MarksMenuModel`: enabling and visibility of F130-F137.
- `fn set_selected_marks_text(&mut self, text: &str)`, `fn cut_marks(&mut self)`, `fn copy_marks(&mut self)`, `fn paste_marks(&mut self, press: (i32, i32), proceed_on_type_mismatch: &mut dyn FnMut(&MismatchInfo) -> bool)`: F130-F133, F135. Over FFI, return the mismatch and resume.
- `fn breakdown_phrase(&mut self)`, `fn breakdown_word(&mut self)`: F136, F137.
- `fn ideal_text_color(bg: Color) -> Color`, `fn locked_hatch_color(bg: Color) -> Color`: F119.

**`vixen-core::marks`** (for that crate's owner, see Data and persistence):

- `MarkCollectionType::State`.
- `Mark::set_start(&mut self, t: TimeSpan)`, `Mark::set_duration(&mut self, d: TimeSpan)`: the 25 ms floor.
- `MarkCollection::ensure_order(&mut self)`: start then end.
- `MarkCollection::fill_gap_times(&mut self, mark: Uuid)`: F125.
- `MarkCollection::is_visible(&self) -> bool`.

**Waveform (over `vixen_audio::Peaks`)**

- `fn visible_sample_range(samples: usize, media_duration: TimeSpan, visible_start: TimeSpan, tpp: TimeSpan, clip_left: i32, clip_right: i32) -> (usize, usize)`: F141.
- `fn waveform_columns(peaks: &Peaks, first_column: i64, columns: usize, tpp: TimeSpan) -> Vec<(f32, f32)>`: F141, one per column over `[column * tpp, (column + 1) * tpp)`.
- `fn guide_dirty_rect(t: TimeSpan, visible_start: TimeSpan, tpp: TimeSpan, width: i32, height: i32) -> Option<IRect>`: as upstream's narrow rectangle.

**Rendering**

- Existing `vixen_session::thumbnail::rasterize` does the picture.
- `fn queue_render(&self, e: ElementId)`, `fn render_all(&self)`, `fn render_progress(&self) -> u8`: F43, on a worker pool, never blocking a drag.

**Document flags and failures** *(audit)*

- `fn mark_editor_state_modified(&mut self)`, `fn is_editor_state_modified(&self) -> bool`: F153. Set by zoom, scroll and row height, never cleared. Close saves silently on it.
- `fn take_draw_failure(&mut self) -> Option<String>`: F151, Q45. Logged at the source; returns text once until a frame succeeds.

**Notifications.** The session keeps a per-window change set that Swift drains after each call, replacing upstream's global event managers: cursor moved, selection changed, marks moved, text changed, deleted, pasted, breakdown, play range, guides, row layout changed, render progress. Listeners on the Swift side: marks docker, effect editor, status bar.

## SwiftUI and AppKit structure

**Module:** `app/Sources/VixenTimeline`, importing VixenKit and VixenBridge and no other area, per `docs/UI_GUIDE.md`. It exports `TimelineTestScene`, so it can run alone until `VixenSequencer` assembles the window from it. SwiftUI holds one `NSViewRepresentable`, `TimelinePane`, inside the sequencer's main area. Everything inside is AppKit. Commands reach it through the sequencer's `@Observable` model. `docs/parity/timeline.md` lists every F entry as done or omitted, per the UI guide.

**View tree**

```
TimelinePane (SwiftUI wrapper)
└ TimelineContainerView (NSView)
  └ NSSplitView (vertical, left pane holds its width, min 100)
    ├ RowHeaderPane
    │ ├ blank backdrop (40,40,40), same height as waveform + ruler + marks bar
    │ └ RowHeaderView (custom, draws visible rows only)
    └ TimePanelView (manual vertical layout, no Auto Layout)
      ├ WaveformView (hidden without audio)
      ├ RulerView
      ├ MarksBarView
      └ GridViewportView
        ├ NSScroller horizontal, NSScroller vertical
        └ layers, below
```

**Control mapping**

| Upstream | Mac form | Region |
|---|---|---|
| `SplitContainer` | `NSSplitView` | Same |
| `RowList` + one `RowLabel` control per row | One `RowHeaderView`, drawing visible rows via `row_tops`; connectors drawn in the same pass | Left, under the backdrop |
| `Ruler`, `MarksBar`, `Waveform` | Custom layer-backed `NSView`s, `isFlipped = true` | Stacked above the grid |
| `Grid` with `AutoScroll` | `GridViewportView`, fixed to the viewport, with two `NSScroller`s bound to `VisibleTimeStart` and `VerticalOffset` | Right, filling the rest |
| `ContextMenuStrip`s | `NSMenu` built from Rust's models, via `menu(for:)` | At the pointer |
| `TextDialog` | Sheet with `TextField`, Enter and Esc as upstream | On the window |
| `MessageBoxForm` warnings | `NSAlert` sheet with verbs: "Create Collection", "Enable and Add", "Paste Anyway" | On the window |
| `Cursors.*` | `NSCursor`: SizeWE and VSplit as `resizeLeftRight`, HSplit as `resizeUpDown`, Cross as `crosshair`, SizeAll as `closedHand`, Hand as `pointingHand` | |
| Row label tooltip-less text | Same text, system font, ellipsis | |
| Mouse wheel | `scrollWheel(with:)`, F11-F14 | Everywhere |
| New | `magnify(with:)`, F17 | Time surfaces |
| Yellow, blue, red, black drawn colours | Keep as listed; the rest from `Theme.Timeline` | |

**Why the row header is not the shared `OutlineView`.** The guide makes element trees `NSOutlineView`, but this column is part of the timeline canvas. Its rows must stay locked to the grid's scroll to the point, have arbitrary per-row heights resized by dragging, follow custom selection (F25), and come from the same `row_tops` as the grid, so they cannot drift. It is virtualized the same way. Give it `NSAccessibility` children for visible rows: one row element per visible row, with a disclosure action. The Display Setup trees continue to use `OutlineView`. This choice is flagged for the owner.

**`GridViewportView` layers, back to front**

1. `tileHost`: fixed-size content tiles, 1024 by 1024 points, positioned relative to the current viewport so that large canvas coordinates never reach Core Animation. Each tile's content is painted with Core Graphics from `grid_frame`: gridlines, rows, mark lines, effects. Scrolling only repositions tiles and paints newly exposed ones. Effects draw from a thumbnail cache.
2. `dragLayer`: effects being moved or resized are lifted out of tiles into a small set of layers, up to a cap of a few hundred, so a drag repaints nothing beneath. Past the cap, use dirty rectangles.
3. `overlayLayer`: rubber band, draw box, hover info (`CATextLayer`), resize indicator lines, guides, each a `CAShapeLayer` updated without redrawing tiles.
4. `cursorLayer`: 2.5 pt wide, ARGB(150,50,50,50). Moved only when the cursor time or view changes.
5. `playbackLayer`: 1 pt green. Moved from a display link.

All with `CATransaction.setDisableActions(true)`. Ruler and waveform keep their own 1 pt blue cursor layers, and the ruler has a green triangle layer. Paint order must match F10.

**Display link and playback.** `NSView.displayLink(target:selector:)` on the grid, active only while playing. Each tick reads the session's position once, applies `playback_follow` if it changed (F95), moves layers. A follow page flip is the only thing that dirties content. Stop the link when stopped. The UI guide forbids polling faster than the display.

**Scrolling.** `VisibleTimeStart` and `VerticalOffset` live in Rust, cached in Swift. Horizontal and vertical `NSScroller`s are set from those and write through the clamps. Don't use an `NSScrollView` document view with the canvas size, because at the zoom floor it would exceed what views and layers can handle.

**Input.** One `NSTrackingArea` per surface for `.mouseMoved`, `.cursorUpdate`, `.activeInKeyWindow`, `.inVisibleRect`. `mouseDown`, `mouseDragged`, `mouseUp`, `rightMouseDown` and friends, `otherMouseDown` (button 2) forward to one Rust call each and apply the returned outcome. `mouseDragged` outside the view keeps arriving, matching upstream's capture. A `Timer` at 50 ms runs while `autoscroll` is true. ⌃-click calls the right-button path. Grid, marks bar, row header and waveform accept first responder; a press makes the surface first responder, as `MarksBar.cs:185` does. The ruler does not take it (F154). Upstream reads modifiers from global state at each moment; read them from the event, and for a timer replay use the last event's flags. For an `NSMenu` that opens during a press (F50), the menu's tracking loop eats the up event, so finish the press when it returns.

**Redraw map.** What each change touches, so that nothing else repaints:

| Change | Update |
|---|---|
| Playback position | `playbackLayer`, the ruler's triangle. A follow flip counts as a scroll |
| Edit cursor | `cursorLayer`, and the ruler's and waveform's cursor layers |
| Horizontal scroll | Reposition tiles, ruler, marks bar and waveform; paint exposed tiles. Recompute stacks if the visible range changed the walk (Q4), and repaint only rows whose result changed. Marks bar height may change (Q24) |
| Vertical scroll | Reposition tiles and the row header; paint exposed tiles |
| Zoom | Everything: tiles, ruler ticks and labels, marks bar, waveform, mark snap windows (F70) |
| Effect selection | Border redraw for effects whose flag changed, in the tiles that hold them. No re-raster |
| Row selection or active flag | Those rows' tiles and labels |
| Move or resize drag | `dragLayer` per moved effect, tiles for rows whose stacks changed, overlay lines |
| Effect moved between rows | Both rows' tiles; row labels if F23 is on |
| Render finished | That effect's rectangles, only if in view |
| Marks moving | Mark lines on the grid (overlay while dragging, tiles at the end), ruler marks, marks bar, guides. The grid's effects do not repaint |
| Guides | Their shape layers on the grid and waveform |
| Hover set | Hover layer only |
| Rubber band or draw box | Overlay layer, plus the effect selection diffs above |
| Row height or expand | Re-layout from `row_tops`, re-clamp (F8), then tiles below the first changed row and the row header |
| Bar or ruler or waveform height | Stack layout only, and grid viewport size |

**Text.** `CTLine` or `NSAttributedString` with the system font; digits use `.monospacedDigitSystemFont`. Mark text wraps in its rectangle and is clipped (F119). Sizes per the 4/3 rule in Conventions.

**Threading.** Swift 6: views are `MainActor`. Thumbnails, peaks and effect renders are on Rust workers; completion hops to main and dirties tiles for those effects. Never synchronously wait on a render.

**SwiftUI state.** The sequencer's `@Observable` model holds what menus and toolbars need: snap on, snap strength, draw mode, the option toggles above. Pass changes down with a single method on the representable's coordinator, and do not re-create the view tree.

**UI test hooks.** Give each surface an accessibility identifier: `timeline-grid`, `timeline-ruler`, `timeline-marks`, `timeline-waveform`, `timeline-rows`, so the `VIXEN_UI_TEST` harness can snapshot and dump them. Effects are not accessibility elements individually; expose the selected count as the grid's value.

## Performance hazards

| Upstream hazard | Cite | Do instead |
|---|---|---|
| Every playback tick (25 Hz) invalidates and repaints the whole grid and ruler | `Form.cs:3457-3463`, `TimelineControlBase.cs:190-195`, `Grid.cs:3162-3165` | Playback line and arrow are layers; nothing else redraws |
| Every cursor move repaints the grid, ruler and waveform | `Grid.cs:357-365`, `Ruler.cs:49-52`, `Waveform.cs:62-65` | Cursor layers; move only them |
| Paint walks all expanded rows, even off-screen, with no vertical culling | `Grid.cs:2695`, `Grid.cs:3057` | `grid_frame` returns the viewport's rows only; binary search on `row_tops` |
| Stacks recomputed per row per paint. The backward scan never stops early: O(n) per cluster | `Grid.cs:3058`, `Row.cs:585-594` | Cache per row, invalidate on that row's change or a change of visible range. Scan with a running max end for early exit, with identical results |
| Thumbnails rasterized on the UI thread during paint, calling `effect.Render()` | `Element.cs:417-433`, `TSE/EffectRasterizer.cs:31` | Rust worker; draw placeholder or the previous picture until ready |
| The cached picture is a visible slice, so each pan re-rasterizes any effect clipped at the edges | `Element.cs:419-421` | Cache whole-effect pictures per zoom, up to a width cap; only slice extremely wide effects, by tile |
| The cache key holds the absolute end time when the effect ends in view, so every move of an effect misses the cache and re-rasterizes it, on every pointer event | `Element.cs:405-408`, `Element.cs:419-421` | Key content on effect identity, render generation and whole width only, so a moved effect is the same picture in a new place |
| Selection is baked into the picture, so any selection change drops it | `Element.cs:202`, `Element.cs:427` | Draw borders separately from content. `Tests/TimelineGridDrawingTests.cs:24-37` then changes to "no re-raster" |
| Rubber band: each move clears all selected effects, so their pictures are dropped, re-tests every effect, and calls `Update()` to paint synchronously | `Grid.cs:1813`, `Grid_Mouse.cs:653-655` | Compute the new selected set in Rust and diff. Repaint the changed effects' tiles, at most once per display frame. Rectangle is a shape layer |
| Whole-grid repaint when the hover set changes | `Grid_Mouse.cs:325-331` | Hover info is an overlay layer |
| A placeholder bitmap allocated per draw | `Element.cs:439-455` | Fill a rectangle |
| A lock per effect during drawing, against render threads | `Grid.cs:3005` | Immutable snapshots: no shared mutable bitmap |
| Snap: at each move, every selected effect loops every snap point of the sequence. `RowContainingElement` is a linear search per effect | `Grid.cs:2064`, `Grid.cs:2081-2123`, `Grid.cs:1686-1689` | Range query in the sorted index for points whose window can include the edges. Cache row membership |
| Snap snapshot of all effects in all visible rows at each drag start | `Grid_Mouse.cs:728-760` | Fine once, but build it off the event path if over a few thousand points; cancel nothing |
| Linear `Row.ContainsElement`, and `Rows.IndexOf` in vertical moves | `Row.cs:708-714`, `Grid.cs:2184`, `Grid.cs:2265` | Hash sets per row, row index map |
| Marks: `MarksAt` scans all marks of all collections at each ruler hover, then guides repaint the grid and waveform | `Ruler.cs:583-587`, `Ruler.cs:860-876` | Binary search per collection. Guides are shape layers |
| Mark drag paths re-sort collections on every move | `LM/TimeLineGlobalEventManager.cs:58-62` | Sort moved marks in place; near-sorted is cheap. Documented cost in `upstream/Vixen/docs/reviews/marks-bar-drag-performance-remediation-plan.md`: 4886 ms sampled in a 10 s drag before remediation |
| A full snap rebuild per collection property change, including each added mark | `Grid.cs:2549-2553`, `Marks/MarkCollection.cs:162-168` | Batch adds; rebuild once |
| Waveform: all columns of the whole file recomputed per zoom (125 ms settle), with an allocation per column's samples | `Waveform.cs:164-216`, `SampleProviders/MaxPeakProvider.cs` | `Peaks::range` over visible tiles; O(columns * log n), with no settle needed |
| Waveform: per-column line with a gradient pen | `Waveform.cs:313-323` | Paint into cached tiles. Guide lines are layers, so a mark drag never touches them |
| Marks bar stacks and height computed in paint, and the height change relays the grid | `MarksBar.cs:1197-1200`, `MarkRow.cs:121` | In the layout pass. Relay only when the height really changed |
| Synchronous `Refresh()` on scroll of ruler, marks bar and the whole control | `Ruler.cs:254-258`, `MarksBar.cs:1149-1153`, `TimelineControl.cs:863` | Set `needsDisplay` or reposition tiles; coalesced by the display cycle |
| Row labels are one child control each ("Addint a control is VERY slow!"), and layout walks them all | `RowList.cs:159`, `RowList.cs:184-198` | One view; draw visible rows only |
| 1-second dashed gridlines loop in sub-pixel steps when zoomed far out | `Grid.cs:2721-2732` | Skip drawing when the interval is under 2 points, which looks the same as a solid wash. The owner can veto, since this differs at extreme zoom only |
| Full-height mark lines drawn to the whole canvas height | `Grid.cs:2764` | Clip to the tile |
| Guides raised from paint, with a 16 ms repaint loop | `Grid.cs:3132-3143` | Q14 |
| `SelectedElements` rebuilt with a distinct across all rows after each selection change | `Grid.cs:307` | Keep an ordered set, updated incrementally |
| Row expansion events walk the whole subtree, each raising events and invalidating | `Row.cs:295-313`, `Row.cs:208-218` | Batch: one layout per user action |
| Each drag move re-sorts every row holding each moved effect, once per effect, through the time-changed notification | `Grid_Mouse.cs:889`, `Grid_Mouse.cs:1087`, `Row.cs:439-443` | One pass per affected row per move, after all times are set. Near-sorted, so an insertion sort is cheap |
| Hover: `elementsAt` plus two LINQ set differences on every pointer move, and two more `elementsAt` per move with the resize indicator on | `Grid_Mouse.cs:323-325`, `Grid_Mouse.cs:429-430` | Binary search in the row; compare the small sets directly |
| `rowAt`, `RowsIn` and `TopVisibleRow` walk the visible rows from the top per query | `Grid.cs:1562-1575`, `Grid.cs:1577-1595` | Binary search on `row_tops` |
| `ActiveRow`, `SelectedRows` and "visible" filters scan all rows per read; `Rows.IndexOf` in row selection | `Grid.cs:316`, `Grid.cs:331`, `Grid.cs:649-650`, `Grid_Mouse.cs:728` | Keep these as maintained sets and an index map |
| Rubber band and row selection test membership in plain lists | `Grid.cs:1838-1857`, `Grid.cs:630` | Hash sets |
| Selection limit counts each selected effect's row again for every effect, on every context menu open | `Grid.cs:975-984`, `ContextMenu.cs:116` | Count once per row |
| Each row label allocates brushes, pens, a font and a string format per paint; tag dots measure text and look up tags per paint | `RowLabel.cs:186-234`, `TSE/TimedSequenceRowLabel.cs:55-94` | Shared paint objects, a font per size, cached text layout and tag colours per row, invalidated on change |
| A property change on any mark collection synchronously refreshes every open marks bar, through a static event | `MarkRow.cs:32`, `MarksBar.cs:78-89`, `MarksBar.cs:161-164` | Per-window change set, coalesced to the display cycle |
| Marks bar paint: a new brush per mark, never disposed; `SelectedMarks.Contains` per mark, here and on the ruler | `MarksBar.cs:1282`, `MarksBar.cs:1286`, `Ruler.cs:913` | Prebuilt frame from Rust, with a selected flag on each quad |
| After each mark move, text edit, delete or paste, every effect in the sequence is scanned for "dirty" on a task | `Form.cs:3128`, `Form.cs:3135`, `Form.cs:4106-4122` | Keep an index of effects that read marks |
| Zoom shows stale waveform columns until the 125 ms settle and rebuild end, then flashes "Building waveform" | `Waveform.cs:267-270`, `Waveform.cs:292`, `Waveform.cs:346-352` | `Peaks::range` per tile, with the message only during the first decode |

**Pinch.** Re-rendering tiles on every magnify event would stutter. Apply a horizontal transform to `tileHost` during the gesture, and repaint at most every 100 ms and at the end.

**Budget.** Pointer events must return from Rust well under one frame. A move drag does constant work per effect moved, plus a range query. No disk, no re-render until release, per the UI guide.

## Tests worth writing

Rust unit tests in `vixen-session::timeline` (no window). Expected values read from the code. Upstream tests to port behaviourally are named after each group.

**Mapping and view** (`Tests/MarksBarMouseWheelTests.cs`, `Tests/TimelineZoomTests.cs`)

- Default `TimePerPixel` is 100000 ticks; `time_at(10)` is 100 ms; `px(1 s)` is 100.
- `clamp_visible_start`: below 0 gives 0; total 60 s, span 10 s, value 55 s gives 50 s; total 5 s, span 10 s, any value gives 0.
- Zoom: 2222 ticks, scale 0.9 is rejected because `(long)(2222 * 0.9)` is 1999, not above 2000; time per pixel and start unchanged.
- Zoom out past total: `TimePerPixel = total / width`, start 0, same for `zoom` and `zoom_at`.
- `zoom_at` scale 0.9 with pointer 400 into the panel: `TimePerPixel` scaled, start increased.
- Wheel: delta 120 scale 0.9; delta -120 scale 1.1; pan -120 moves start +10%; +120 moves -10%; -30 moves +2.5%. Clamps at both ends. ⇧ wins over ⌘ and ⌥ on the marks bar. ⌘⌥ and no modifier do nothing there.
- Tilt: +120 is +10%, -120 is -10%, -1 is -10%.
- `vertical_wheel_delta(120)` is -40.
- `playback_follow`: start 0, span 100: position 90 is none, 91 gives 81 (clamped by caller), position below start gives position minus 10.

**Rows** (`Tests/TimelineActiveRowNavigationTests.cs`, `Tests/RowEventScopeTests.cs`)

- All seven `MoveActiveRow` cases: next, previous, skip hidden, no wrap at both ends, no rows, no active row.
- Height floor: set 5 gives 10.
- `zoom_rows` 1.25: 32 becomes 40; 0.8: 32 becomes 25, 25 becomes 20. A row skipped when `h * scale > grid height`, while `rowHeight` still scales.
- TreeId: "Arches" is `0A9E1587CEC47A218E3B7B1E84D5F22F`; child "Arch 1" of "Arches" hashes "Arch 1Arches": `D490FDFA296E6757D918AB22E8F6F39D`.
- Row settings: `rowHeight` 32, row 40, closed: saved (differs by 8). Row 39, closed: not saved (differs by 7). Row 32, open: saved with `Expanded = true`.
- Visibility: filtered child stays hidden when the parent opens; filter removing all children closes the parent; `HasExpandableChildRows` follows it.
- Row selection: plain, ⌘ toggle, ⇧ range in both directions over hidden rows, Drag Box Filter deselecting other types.
- Same-name height sync on release, and only on release.
- Resize bar: label height 32, y 28 is not in the bar, y 29 is (`y > h - 4`). The same test ignores a click there.
- Double-click on an expander toggles once (`clickCount` 2 does nothing).
- `rowHeight` from 32 through repeated 0.8: 25, 20, 16, 12, 9, 7, 5, 4, 3, 2, 1, 0, while rows stop at 10. From 0, 1.25 stays 0 (Q36).
- Targets: with a selected row and a different active row, paste takes the selected and add-effect takes the active. With neither, paste takes the top visible row and add-effect does nothing.
- ⇧-click on a row after an effect click cleared row selection still ranges from the old first row (Q37).
- Vertical clamp with rows shorter than the viewport: offset stays 0.
- Connectors with depths [0,1,1,0], row height 32, inset 24, offset 0: stubs (12,48) to (24,48) and (12,80) to (24,80), and at the depth 0 row one vertical (12,80) to (12,32).

**Stacking and geometry** (`Row.cs`, `Grid.cs:2956-3033`)

- A[0,10], B[5,15], C[10,20]: lanes 0,1,0, count 2. `C.start >= A.end` shares a lane.
- Row height 32, count 2: `h = 15`, offsets 0 and 15.
- Row height 32, count 40: `h` is 0, so red border and `h = 32`.
- A[0,10], B[5,6], C[5.5,8], D[9,20] at full range: D is index 1 of 3. At start 15: index 1 of 2 (Q4).
- Clipped rect: all four width branches, `x = floor(px(start))`, a width at or below 0 is not drawn.
- Occurrence in two rows with different heights: each row's hit test uses that row's values (Q3).

**Hit testing and zones**

- `y` negative gives the first row; beyond the end gives none.
- X inclusive both ends; y half-open.
- Zone: duration 100 px at x: `grab = 12`. Duration 10 px: `grab = 5`. x at start is Front; at end is Back; at the exact middle of a short effect is None.
- Marks use the same 12 limit.

**Selection and cursor** (`Tests/TimelineCursorSelectionTests.cs`, `Tests/GridAlignmentNullReferenceTests.cs`)

- All cases in those files, including legacy on and off, empty lists, lasso finalization.
- Press decision table F46: each row of it. Release F49.
- Select All then selected list is complete (Q6).
- Right press on blank: anchor moves to it and its row is active, no cursor move on press, cursor moves on release (F54). The context time is the release point's.
- Overlap chooser: a right press does not offer it; a left press does, once; a pick moves the cursor and sets neither anchor nor active row (F50).
- Release collapse when the press hit a selected effect in another row: the old active row keeps its flag and the release row is added (F49).
- ⌘-press deselects at press with no cursor move; the release moves the cursor to the effect anyway (F92).
- Hover set survives the pointer leaving (Q33), and a deleted effect drops out of it.
- `selected_elements` order: rows top down, then selection order.

**Rubber band**

- Crossing against enclosed: effect [10,20] in a band over time 15-25: selected crossing, not enclosed; band 5-25: both.
- Under 2 by 2 is a background click.
- ⇧ keeps the remembered list.
- CAD: rightward enclosed and green; leftward crossing and blue.

**Move and resize**

- Threshold: `x0 + 7` is inside, `x0 + 8` is outside, `x0 - 8` is inside (half-open).
- Clamp: selection from 2 s, dt -5 s gives -2 s. Latest end 58 s of 60 s, dt +5 s gives +2 s.
- Modifier table F59, exact matching: ⇧⌘⌥ freezes neither.
- Clone: originals untouched, clones selected, two undo entries. A clone of a two-target effect has one target, the last row's node.
- Nothing selected and a drag past the threshold: no time change, one undo entry with count 0 (Q35).
- Threshold crossed below the last row, then moved back onto a row: no failure, nothing moves that time, tracking resumes (Q38).
- Effects in hidden rows only: an upward row move does nothing (Q39).
- Click in a resize zone with no motion: one Resize undo entry, no time change, a two-effect selection stays selected (Q34).
- Resize a 5-pixel effect by any motion: width becomes 10 pixels, even if that passes the sequence end.
- Minimum width: `tpp` 10 ms: `time(10)` is 100 ms. Front drag on a 500 ms effect, dt +450 ms gives +400 ms. Back drag on a 500 ms effect, dt -450 ms gives -400 ms.
- Adjoining: neighbour follows; limit stop keeps the neighbour at 10 pixels; ⌥⇧ filters by layer; only 20 entries are inspected, and on the Front side the 20 include the primary and any short effects after it; an empty list is searched again next move from the primary's current times; undo restores neighbours.
- Vertical (`Tests/GridCloneDragDuplicateRowsTests.cs`, 7 cases): tracks the pointer row through duplicate rows; clips to the last valid row with a selection spanning rows; hidden occurrence; Deprecated destination leaves all unchanged; unchanged destination; existing effect; up and down from a repeated occurrence.
- Cancel: times and rows restored, no undo entry, snapshot cleared (Q10). Works for a resize with neighbours. Does nothing to a rubber band or a draw box.
- Draw box past both ends of the sequence: start 0, end `TotalTime` (Q40). No movement at all: background click, and no effect since the duration is 0.
- Pointer table F150, one case per row.

**Snapping** (`Tests/GridMarkSnapPointTests.cs`, 15 cases: registration, identical times, disabled lines, zoom preserving identity or equivalent, moving, duplicates, multiple collections, coalescing, guides with hidden grid lines, disposal)

With `tpp` 100 ms and strength 2, so a level 5 window is ±1 s and level 1 is ±200 ms; effect [10,12]:

- Off or zero offset returns the offset.
- Neighbour edge at 20, dt 7.5 s: end 19.5 is in range: result 8 s.
- Mark level 1 at 19.6 and edge at 20, dt 7.5: edge wins by level: 8 s.
- Mark level 6 at 19.6 and edge at 20: mark wins: 7.6 s.
- Edges at 19.0 and 20.0, dt 7.9: 19.0 wins by time: 7.0 s (Q12).
- Strict window: end exactly at `SnapEnd` is out.
- Both in range: [10,11] with dt 9 and a point at 19.5: result 8.5 (end). [30,31] with dt -11 and a point at 19.5: result -10.5 (start) (Q13).
- Both in range, snap point inside the original span: [10,11] with dt 0.1 and a point at 10.4 (`2 * 10.4 < 21`): result 0.4 (start). Point at 10.5 (a tie): result -0.5 (end). With dt 0.2 the answers are the same, because the offset cancels (Q13).
- Best level carries across candidates: two selected effects in a row, the first in range of a level 5 point and the second of a level 1 point: the level 5 result stands.
- Tracked row clipped below the pointer's row: candidates come from the tracked row (F71).
- Row scope: a point on another row is ignored, a mark applies.
- Row 0: pointer in flat row 0 considers all selected effects (Q11).
- Opposing: resizing Front on [10,12], Start edge at 12 skipped, End edge at 12 used. Resizing Back, End edge at 10 skipped, Start edge at 10 used. Mark at those times used.
- Resize: Front ignores end, Back ignores start.
- Resize limit beats snap, move snap beats clamp.
- Effect-edge windows fixed at drag start; mark windows update on zoom.

**Alignment** (`Tests/GridAlignmentNullReferenceTests.cs`)

- Each row of F80, including the 50 ms rule, the total and zero clamps, and Q15.
- Null reference: unchanged, no error.
- 33 selected on a row: warning. 32: allowed.
- Batch skips entries past total or under 1 ms, and still records one undo.
- Close gaps: gap below threshold closes; exact contact is skipped; overlap shortens. On a selection, pairs follow selection order (Q41).
- Already aligned: one undo entry with count 0 (Q35).
- `move_by_time` no clip limits; clip deletes and Q18.
- Undo and redo swap restores times and rows, and re-adds deleted.

**Ruler** (`Tests/RulerLockHeightTests.cs`)

- Scale table, from this f64 arithmetic: `tpp` in ms gives minor and per-major: 0.5 gives 10 ms, 5; 1 gives 50 ms, 5; 2 gives 50 ms, 5; 5 gives 250 ms, 4; 10 gives 250 ms, 4; 20 gives 250 ms, 4; 25 gives 500 ms, 4; 50 gives 1 s, 5; 100 gives 5 s, 6; 250 gives 5 s, 6; 500 gives 10 s, 6; 1000 gives 15 s, 4; 5000 gives 1 min, 5. 2001 ticks gives 2.5 ms, 4.
- Labels: minor 250 ms: 5.25 s is `5.2`, 65.5 s is `1:05.5`, 3661 s is `1:01:01.0`. Minor 5 s: 65 s is `1:05`. Minor 1 h: 5400 s is `1:00`.
- `first_tick_px`: start 0, step 100 is 100.
- Click threshold: dx 2 is a click, dx 3 is a drag.
- Range: drag left and right give the same ordered range; start clamped at 0, end at total.
- `marks_at`: exactly ±20 ms in, 21 ms out; locked excluded; level order.
- Mark drag: first move lands on the pointer. Back to the press X on release: no undo entry, the jump stays (Q43).
- Ruler click marks the sequence modified and moves the cursor; a range drag does neither; ⌘-click plays and does not move the cursor.
- Hover from a mark into the strip keeps the guide; hover onto plain ruler clears it (Q44).
- Press on the strip with the right button then release: adds a mark, as anywhere else off a mark.
- Lock: all the cases in that file.
- Mark drag label for each of the four formats.

**Marks bar** (`Tests/MarksBarAutoScrollTests.cs`)

- All 8 cases: width 100 and `tpp` 100 ms: x 10 is -14, x 90 is 14, x 50 is 0; start 500 ms, outside -400 gives 0; start 49 s, 80 gives 50 s; total 5 s gives 0; start 10 s, 80 gives 12 s; -80 gives 8 s.
- Row height is `20 * lanes` in view; total height is the sum of shown rows.
- `ideal_text_color`: white gives black, black gives white, delta 150 gives black, delta 149 gives white.
- Hatch: channel 200 gives 230, channel 240 gives 210.
- Move clamp; min width 12 px; glued; undo includes the neighbour (Q29).
- `Mark` setters: duration 10 ms becomes 25 ms.
- Fill gaps: marks at 1 s, 3 s, 5 s, all 500 ms: fill the middle: start 1.501 s, duration 3.498 s. Last mark keeps its duration.
- Paste: offsets preserved relative to the earliest in the whole clipboard, row offset by collection index, type mismatch stop, past-end group skipped.
- Delete twice records one undo entry (Q31). Delete with nothing selected records "Removed 0 Marks" (Q35).
- Click in a mark's zone with no motion: one Resize undo entry, sequence modified (Q34). In a locked mark's zone: "Resize 0 effects" (Q35).
- Press on blank: selection cleared, anchor moved. Drag from blank: nothing.
- Menu with an empty selection and a same-point press on a mark: all three type items show (Q42).
- Undo of a move: times restored, re-sorted, sequence not newly modified. Redo drops a mark no longer in its collection (F152).
- Clear rules of F155, one case each.
- `FillGapTimes` when the previous mark's end equals this start: with no earlier mark the start stays. With one earlier still, the start goes to that one's end + 1 ms, overlapping the previous mark (F125).
- Marks bar height with no shown rows is 0.
- Breakdown phrase: "a b c" over 3 s makes three 1 s marks.
- Menu model: Edit Text enabled only with same press and release on an unlocked mark; type items per all-selected type.

**Waveform** (`Tests/WaveformAlignmentRenderingTests.cs`, `Tests/WaveformLockHeightTests.cs`)

- `tpp` 1 s, start 10 s, size 100 by 60: guide 10 s gives (0,0,3,60); 60 s gives (48,0,5,60); 109 s gives (97,0,3,60); 113 s gives none. Two guides 10 s and 20 s union to (0,0,13,60).
- Sample range with 120 samples, 150 s media: clip (0,5) gives (9,16); clip (95,100) gives (104,111); clip (200,205) gives (120,120); whole gives (9,111).
- Inactive or null times clear.
- No guides or cursor during "Building", or when the visible start is past the media end.
- Lock: all the cases in that file.
- `H = 50`: `workH` 48. Full: `top` 24. Half: `top` 48. `H = 51`: `workH` 48.

**Rendering pictures** (`Tests/TimelineGridDrawingTests.cs`)

- Same slice reuses the cached picture; selection does not re-rasterize on the Mac; a dirty effect shows a placeholder and no raster; a later render shows content.
- Slice offsets during a resize: original 2 s, current 4 s, visible offsets 1 s to 3 s become 0.5 s to 1.5 s (F42). During a plain move they are unchanged.

**Document flags** (F153)

- Each row of F153's table. A zoom followed by a close saves without a prompt; a row expansion alone does not save.
- Playback follow: position 91 of span 100 from start 0 moves to 81, then the same position again does nothing; a paused position does not snap a manual scroll back (F95).

**UI harness.** A snapshot of a real sequence copied from the owner's show: rows, ruler, marks, waveform. Check order, colours and labels against `upstream/Vixen/Assets/Editor.png`.

**Benchmark, in `cargo bench` or a timed test.** 50,000 rows, 200,000 effects: `grid_frame` for a viewport under 2 ms; `grid_mouse_move` during a 1,000-effect move under 2 ms.

## Windows-only omissions

| Upstream | Why it has no place on a Mac |
|---|---|
| `WM_MOUSEHWHEEL` handling in `WndProc` (`TimelineControlBase.cs:207-258`) | AppKit reports horizontal scrolling in `scrollWheel(with:)` |
| `ScalingTools` DPI factors (`TimelineControl.cs:60`, `Ruler.cs:44-46`, `MarkRow.cs:14`) | Points are already resolution independent |
| `SetStyle(OptimizedDoubleBuffer ...)`, `AutoScaleMode` (`TimelineControlBase.cs:16`, `TimelineControlBase.cs:28-32`) | Layers and display cycle |
| `PenAlignment`, `Graphics.VisibleClipBounds`, GDI+ `HatchBrush`, `Pen` lifetime | Core Graphics equivalents; a hatch is a `CGPattern` |
| `BackgroundWorker`, `Parallel.ForEach`, `BlockingCollection` and its Microsoft extension file, `DelegateBasedObserver.cs` (`Grid.cs:2785-2884`) | Rust worker pool |
| `InvokeRequired`, `Invoke`, `BeginInvoke`, `Application.DoEvents` (`Grid.cs:202`, `Grid.cs:141`, `Form.cs:4216-4217`) | Swift concurrency; loading stays off the main actor |
| `Element.DrawLock`, `DrawV2`'s ownership result, the obsolete `Draw` wrapper (`Element.cs:231-234`, `Element.cs:519-547`) | Exists only to protect a shared mutable GDI bitmap. Immutable `CGImage`s need none |
| `[Serializable]` and `[NonSerialized]` on `Element`, `Element` copy constructor (`Element.cs:7`, `Element.cs:24`, `Element.cs:36-43`) | Not used by the timeline in this form |
| `Dispose` chains, `cEventHelper.RemoveAllEventHandlers`, manager `CloseManager` (`TimelineControl.cs:87-127`, `Form.cs:851-853`) | Ordinary ownership in Rust and ARC |
| `Form_Grid` key broadcast (`TSE/Forms/Form_Grid.cs:31-34`) | The responder chain does this; floating panels are a sequencer concern |
| `ThemeToolStripRenderer`, `ThemeColorTable` as a static class | Native menus; `Theme.Timeline` |
| `MessageBoxForm.msgIcon` static | `NSAlert` style |
| `Application.DoEvents` while a stuck render worker is forced to stop, and the 5 s wait at dispose (`Grid.cs:130-146`) | Rust worker pool shuts down on its own |
| Stack traces in the ruler's and marks bar's error boxes (`Ruler.cs:112`, `MarksBar.cs:1184`) | Replaced by F151's alert and the log |
| `Keys.Oemplus` and `Keys.OemMinus` layout names, `ProcessCmdKey` | Key equivalents and `keyDown` |
| Unused APIs: `BackgroundClicked` event subscribers, `ElementDoubleClicked` subscribers, `AlignSelectedElementsLeft` (`Grid.cs:2007-2034`), `AddSnapPoint`, `RemoveSnapPoint` (`Grid.cs:2456-2467`), `ClearSnapPoints`, `SelectedMarkMoveEventArgs`, `MarksBulkChangeInfo` in the control, `ResetTimeLineSettings` (`Form.cs:6304`), `BottomVisibleRow` (`Grid.cs:352-355`), `GridPoint`, `ElementsAtTime`, `EarliestStartTime`, `LatestEndTime`, `MoveResizeElementByStartEnd`, `ToggleElementSelection`, `SelectedElementTypes`, `ClearAllElements` (`TimelineControl.cs:631-636`), `Row.AddUniqueElement`, `Row.RemoveChildRow`, `MarkCollectionAt` (`MarksBar.cs:1089-1101`), `TimeRangeDraggedEventArgs`, the ruler's `BeginDraw` and `EndDraw` (`Ruler.cs:944-955`), `Grid.Context`, and commented code | No caller in the editor. `MarksBulkChangeInfo` is used by the host's bulk undo, which the sequencer spec covers. F44 and F81 still describe a few of these, in case a later spec wants them |
| Mark `Copy_Click`, `Paste_Click` text helpers (`MarksBar.cs:1016-1041`) | Their menu items are commented out (`MarksBar.cs:285-296`) |
