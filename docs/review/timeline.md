# Timeline review

Independent review of the sequencer timeline (spec `docs/spec/timeline.md`, parity `docs/parity/timeline.md`).
Build id `timeline-review`, areas `timeline,admin`, show copy `target/show-timeline-review`.

Paths: `T/` is `crates/vixen-session/src/timeline/`, `FFI` is `crates/vixen-ffi/src/timeline.rs`, `UI/` is
`app/Sources/VixenTimeline/`. C# paths are as in the spec's citation table.

## Findings

(each: what, C# file:line, Rust or Swift file:line, fixed or left and why)

1. **Pausing playback cleared the playback line and arrow and restored the remembered ruler range.** F94 says a pause holds
   the line where it is; the range comes back only when the context ends. `Form.cs:3447-3455` (`context_ContextEnded`) is the
   only place upstream nulls `PlaybackCurrentTime` and restores `_mPrevPlaybackStart/End`; a pause changes nothing because the
   position stops changing (`TimeInfo.cs:95-96`). `UI/TimelineCanvas.swift` `refreshTick` treated `.paused` as stopped and called
   `stopPlaybackLink` (clear lines, `playbackEnded`). Status: fixed by the first (interrupted) review run; confirmed in the code by the second run.
2. **`tags_changed` re-applied the hidden filter with the tags copied at load.** After Manage Tags saves, upstream repaints the
   labels, whose tag dots read the live tags (`Form.cs:2320-2326`, `TSE/TimedSequenceRowLabel.cs:55-94`), and its filter reads live
   tags at every recompute (`Form.cs:2345-2360`). `FFI` `tags_changed` used `Row.tags` and `Timeline.tags` from load, so neither
   new tags nor new colours showed until the sequence was reopened. Status: fixed by the first (interrupted) review run; confirmed in the code by the second run.
3. **A pasted mark kept a duration under the 25 ms floor.** `Mark.Clone` sets `Duration` through its setter (`Marks/Mark.cs:89-97`,
   `Marks/Mark.cs:55-69`), so a paste can't make a mark shorter than 25 ms. `T/marks.rs` `paste_marks` copied the field. Upstream can
   only hold such a mark briefly, but `vixen-io` loads one from a file as it is. Status: fixed by the first (interrupted) review run; confirmed in the code by the second run.
4. **A row height drag showed the up-down pointer whatever the pointer was over.** `RowLabel.cs:142-156` sets `HSplit` over the
   label's bar and the arrow elsewhere on every move, resizing or not. `T/row_input.rs` `row_mouse_move` set up-down for the whole drag.
   Status: fixed by the first (interrupted) review run; confirmed in the code by the second run.
5. **A right press on one row label released over another opened no menu.** The pressed label keeps the mouse, so its `OnMouseUp`
   runs with the point outside it: no click, and `_RowContextMenuSelect` still fires (`RowLabel.cs:132-139`). `T/row_input.rs`
   `row_mouse_up` returned before the menu. Status: fixed by the first (interrupted) review run; confirmed in the code by the second run.


## Second run (resumed 2026-11-09 15:55)

The first run stopped at finding 5. All five are in the code (`TimelineCanvas.swift` `refreshTick`, `FFI` `tags_changed`,
`T/marks.rs` `paste_marks`, `T/row_input.rs` `row_mouse_move` and `row_mouse_up`). The area's 286 unit tests passed before
any change of this run.


### Behaviours checked line by line, no defect (grid pointer handling)

`GM` = `upstream/Vixen/src/Vixen.Common/Controls/TimeLineControl/Grid_Mouse.cs`, `G` = `Grid.cs` there, `GI` = `crates/vixen-session/src/timeline/grid_input.rs`.

1. Press flow, right press on the background, and the ⌘ and ⇧ forms (GM:45-197 against GI `grid_mouse_down`, `press`).
2. Draw mode press: left only (FIX Q5), box starts at the press (GM:150-165, GI `begin_draw_box`).
3. Overlap chooser offered only with more than one effect under the pointer (Form.cs:2824, GI `offer_to_host`, FIX Q47).
4. Wait-for-drag rectangle, 2 x the drag threshold around the press (GM:701-712, GI `wait_for_drag_move`).
5. Resize start refused with more than one effect and ⌥, with the upstream message (GM:120-140).
6. Release without a drag: single selection and the cursor rule (GM:200-286, GI `mouse_up_no_drag`).
7. Modifier freeze rules during a drag, exact matches (GM:374-454, Q8).
8. Resize zone: 12 px threshold, halved for narrow effects, and the pointer shape (GM:465-490, GI `resize_zone`).
9. Draw box stretched to whole rows, +1 and -2 (GM:581-624, GI `mouse_move_draw_select`).
10. Background click rule: both sides under 2 px (GM:687-697, GI `on_background_click`).
11. Rubber band begin, move and release, ⇧ keeping the earlier selection (GM:629-683, G:1796-1865, GI `select_elements_within`).
12. Crossing against enclosing selection: enclosing only when CAD style is on and the box runs right of the press (G:1806).
13. Drag box filter by effect type, in both directions (G:1834-1846).
14. ⇧-click range: always crossing, and it clears selected and active rows (G:1744-1780, GI `select_elements_between`).
15. Lasso end: origin row becomes the only active row and the cursor goes to the earliest selected start, not in legacy mode (G:486-501, GI `finalize_lasso`).
16. Autoscroll: distance past the margin divided by 4, every 50 ms, no upper limit (GM:1180-1197, GI `grid_autoscroll_tick`, `edge_distance`).
17. Move arithmetic: clamp to 0 and to the sequence length from the originals, then snap, so snap can push past the clamp (GM:858-901, Q7, GI `mouse_move_drag_moving`).
18. Vertical move: destination limits from the top and bottom occurrences, duplicates counted, tracked row advanced (G:2177-2306, GI `move_elements_vertically`). Q9, Q38, Q39 guards are as the register says.
19. Deprecated rows refuse a dragged effect (G:2190-2194, F62).
20. Resize: snap first, then bounds, minimum 10 px from the shortest selected original (GM:943-1036, GI `mouse_move_h_resizing`).
21. Adjoining search: 20 entries back from the end-time index or forward from the effect, same layer with ⇧, taken once per drag (Row.cs:605-645, 727-740, GM:988-1072, GI `adjoining`).
22. Adjoining follow and the 10 px stop, front and back (GM:1099-1140, GI `apply_adjoining`).
23. Snap search: strict windows, highest level wins, first found at a level wins, own opposite edge ignored on resize, row scoping with the `> 0` quirk (G:2045-2139, `snap.rs` `find_snap`, GI `snap_candidates`, Q11, Q13).
24. Snap points for a drag: both edges of every unselected effect in visible rows, element edges filed before marks at one time (GM:722-773, GI `calculate_snap_points`, `snap.rs` `visit`).
25. Finish: one undo entry per move or resize, recorded even with no net change (GM:532-558, GI `elements_finished_moving`).

### Behaviours checked line by line, no defect (commands, zoom, rows, saved state)

`C` = `crates/vixen-session/src/timeline/commands.rs`, `TC` = `TimelineControl.cs`, `Form` = `TimedSequenceEditorForm.cs`.

26. Align, all seven kinds: refusal above 32 selected in one row with the upstream warning text kept (KEEP Q16), reference is the element under the pointer, Centers keeps Q15's arithmetic, one `TimesChanged` undo entry holding every original. `G:975-1267` against `C` `align`.
27. `MoveResizeElements`: skips an element whose new start is below zero or new end past the total, sets start then duration, one undo entry with all originals. `G:1350-1385` against `C` `move_resize_batch`.
28. `MoveResizeElement` and by start and end: refusals (start below zero, end past total, duration not positive) and no undo entry of its own. `G:1387-1447`.
29. Split at a time and at the cursor (KEEP Q17: the second half is a clone placed after the first is shortened; undo is one group of resize then add). `G:1269-1299`.
30. Close gaps: threshold, selection-order lists per row, and the rule for an element that both ends at and starts at a neighbour. `G:1301-1348`.
31. `MoveElementsByTime` and the in-range form: clamp at zero, clipping at the sequence end takes the effect off its last row (KEEP Q18) and the undo group restores rows. `G:1449-1550`.
32. Nudge by 2 pixels of time, selection only, same undo kind as a move. `C` `nudge_selected`, through `MoveElementsByTime` (`G:1472`).
33. Selection order of a row's selected elements is the order they were selected in both (`Row.cs:445-450`, `model.rs:641`), which the align and close-gap references depend on.
34. Zoom limits: rejected at 2000 ticks per pixel or below, at scale 0 or below; past the total it becomes `TotalTime.Ticks / grid.Width` by integer division and the start goes to 0. `TC:448-466` against `view.rs:89-107`.
35. Zoom anchors. `Zoom` keeps the left edge unless the end would pass the total (`TC:396-412`, `view.rs:110-124`); `ZoomTime` moves the start by `(int)(px(new - old) * fraction)` pixels of the new scale, fraction from the panel width and the pointer's distance from the splitter (`TC:423-446`, `view.rs:128-147`). The float32 product upstream is f64 here (FIX Q1).
36. Wheel routing: ⌘⌥ one row, ⌘⇧ all rows at 0.8 or 1.25, ⌘ zoom at `1 - delta/1200`, ⇧ pan by `span * -(delta/1200)`, plain `-(delta/3)` rows; over the marks bar ⇧ wins, ⌘ without ⌥ zooms, nothing else. `TC:1144-1228` against `C:478-532`.
37. One-row wheel zoom uses the row under the pointer (FIX Q2: upstream's walk ignores the scroll offset and the marks bar, `TC:1231-1255`); height is `(int)(h * scale)` with the floor of 10 from the `Height` setter (`Row.cs:38-51`).
38. `ZoomRows`: the default height scales without a floor, each row is skipped when `h * scale` is above the grid height, others truncate. `TC:468-490` against `rows.rs:588-600`.
39. Playback follow: flips when the position passes 90% of the view or falls before its start, to position minus 10% of the span. `TC:1275-1287` against `view.rs:174-180`.
40. Rows at load: one row per occurrence of each node depth first, roots visible, then saved settings in flat order as open, height, visible, then the hidden filter. `Form:1318-1384, 4193-4225` against `rows.rs:198-256`.
41. Saved row settings: a row is saved when its height is more than 7 from the default or it is open; the default row height and the playback range go with them; zoom and scroll are copied into the sequence. `Form:1613-1636, 2370-2381` against `rows.rs:291-310`, `session_link.rs:125-134`. Upstream's `Dictionary.Add` throws on two same-named paths; here the later one wins (GUARD).
42. Saved playback range is applied only when the saved end isn't exactly zero, null included (`Form:563-567`, `session_link.rs:40-44`); zoom and scroll only when above zero (`Form:1515-1523`).
43. Row visibility: `TreeOpen` goes through `RecomputeVisibility` so a hidden parent can't leak visible onto a filtered child, the filter collapses a row left with nothing to expand. `Row.cs:181-313` against `rows.rs:316-381`.

### Behaviours checked line by line, no defect (marks bar)

`MB` = `Vixen.Common/Controls/TimeLineControl/MarksBar.cs`, `M` = `crates/vixen-session/src/timeline/marks.rs`.

44. Press on the bar (`MB:183-255` / `M` `marks_mouse_down`): a mark in a locked collection counts as no mark; a double click plays the mark's range, or with ⌥ fills the gap; a plain press on an unselected mark clears and selects it; ⇧ selects between the last single press and here; ⌘ toggles; a left press outside a resize zone waits for a drag, inside one starts a resize unless ⌘ is down. Same.
45. Release and the menu (`MB:263-362` / `M` `marks_mouse_up`, `marks_menu_model`): Edit Text needs the same point and a mark; Cut, Copy and Delete need a selection; Paste needs the same point and a non-empty clipboard; Phoneme, Breakdown Phrase and Breakdown Word need the same point, a mark and every selected mark of that type; a plain release with several selected collapses to the pressed mark. Same.
46. Dragging marks (`MB:510-539` / `M` `marks_drag_moving`): the offset is clamped so the earliest original stays at or after 0 and the latest end at or before the total; guides at the pressed mark's start and end. Same.
47. Resizing marks (`MB:677-786` / `M` `marks_h_resizing`): with ⌥ and exactly one selected the glued neighbour moves too (KEEP Q28); the shortest duration includes the neighbour's; 12 px minimum; the guide is raised before the change. FIX Q29 (the neighbour is saved for undo, which upstream's `MarksMoveResizeInfo` doesn't hold) is as the register says.
48. Autoscroll while dragging (`MB:583-660` / `M:1410-1421`): distance divided by 4 as whole pixels, times time per pixel; 0 when the candidate is negative or the whole sequence fits; otherwise capped at total minus the visible span. Same.
49. ⇧ range selection (`MB:828-911` / `M` `select_marks_between`, KEEP Q27). Same.
50. Fill gap (`MB:199-204`, `MarkCollection.cs:237-258` / `M:53-68, 1012-1019`): the undo entry holds the selected marks, not the pressed one, with type Resize; start becomes the previous end plus 1 ms for the nearest earlier mark that ends before it; duration reaches 1 ms before the next mark, through the setter's 25 ms floor. Same.
51. `Mark` setters (`Vixen.Modules/App/Marks/Mark.cs:38-69` / `M:31-44`): setting the start raises a duration under 25 ms; the duration floor is 25 ms. Same. (Upstream's duration setter returns early when the value equals the present one, so a mark read from a file with a duration under 25 ms keeps it until something else changes it; the Rust setter always floors. Not reachable from the bar's own edits, which always change the value.)
52. Undo descriptions (`Undo/MarksTimeChangedUndoAction.cs:48-58`, `MarksAddedUndoAction.cs:29`, `MarksRemovedUndoAction.cs:28` / `M` `mark_label_undo`, `marks_added_label`, `marks_removed_label`): "Moved N effect(s)" and "Resize N effect(s)" for marks (KEEP Q23), "Added N Mark(s)", "Removed N Mark(s)", singular only at exactly 1. Same.
53. Paste (`MB:366-417` / `M:1099-1163`): groups by source collection in collection order; target row is the pressed row plus the source's distance from the first group; groups past the last row are skipped; a type mismatch asks, and No stops the remaining groups while keeping what was already pasted; each clone starts at the press time plus its distance from the earliest clipboard mark; pasted marks become the selection; one "Added N Mark(s)" entry. Same. The clipboard holds references upstream (KEEP Q30), so later edits show in a paste: the Rust looks the mark up again by id.
54. Cut, Copy, Delete (`MB:419-431, 1006-1014`, `Form:3139-3151` / `M:1043-1095`): Cut is Copy then Delete; Delete records "Removed N Mark(s)". FIX Q31 (selection cleared after a delete) is as the register says.
55. Text edits (`MB:956-989`, `Form:3125-3129` / `M:1022-1040`): Phoneme and Edit Text set every selected mark's text and record no undo (KEEP); the dialog's title is "Edit Mark" or "Edit Multiple Marks", and the present text is offered only for one mark. Same.

### Behaviours checked line by line, no defect (ruler)

`R` = `Vixen.Common/Controls/TimeLineControl/Ruler.cs`, `RR` = `crates/vixen-session/src/timeline/ruler.rs`.

56. Ruler press (`R:458-496` / `RR:436-465`): a mark within 20 ms in an unlocked, visible collection (highest level first, `R:856-879` / `RR:212-228`) is toggled, after clearing the selection unless the modifiers are exactly ⌘; any button can press a mark; otherwise the height strip, otherwise the selection clears and a click or range drag begins. Same.
57. Ruler drag (`R:498-600` / `RR:468-546`): a range starts past 2 px; it runs from the lower to the higher of press and pointer, start clamped at 0 only and end at the total only; a mark drag moves every selected start by the pointer's change with no clamp (KEEP Q22), shows the time in one of four forms, and guides at the pressed mark's start, plus its end when its bar shows; the height follows only below y 40 and unlocked. Same.
58. Ruler release (`R:627-712`, `Form:2860-2901, 3168-3198` / `RR:549-630`): a click sets the range start and cursor, or with ⌘ plays from there and keeps the previous range to restore; a dragged range is kept, or with ⌘ played; a moved mark records "Moved N effect(s)" only if the release X differs from the press X (KEEP Q43); a right release on a mark opens Delete Selected Marks only at the press X, and off a mark adds one. The modifiers of the right click: fill gap is exactly ⌘ or ⌘⇧, the label prompt exactly ⇧ or ⌘⇧ (`AddMarkAtTime(time, suppressUndo, fillGap, promptForName)`, `Form:2899, 3032`). Same.
59. Adding a mark (`Form:3032-3096` / `M:1302-1328`): no collections asks to create "Default Marks" in white; a default that isn't visible asks to enable it (grid lines and bar on, tail lines off); nothing is added when a mark already starts at that time; fill gap runs after the add; "Added 1 Mark". Same, with one small difference: when no collection is the default at all, upstream still asks to enable it and then does nothing on OK, and the port doesn't ask. Left: the question has no effect upstream.

### Harness run on the show copy (release build, `target/build-timeline-review`, "Coldest Winter.tim")

The test window opens 1080 points wide on this machine (the builder's scripts assumed 1400), so one press of the builder's `t3.py` fell outside the window; the copy in `target/timeline-review-out` uses x within the window. Not an area defect.

Read back from the state dump, all as the spec says: a drag with snap off moves the effect by exactly the pointer's distance and records "Moved 1 effect"; undo and redo restore and reapply it; a drag across rows lands on the row under the pointer; a front-edge resize records "Resize 1 effect"; Esc in mid-drag puts everything back with no undo entry (Q10); a rubber band selects the three effects it crosses and makes its row active; ⌘ wheel zooms (64000 to 57600 ticks a point), ⇧ wheel pans, plain wheel scrolls rows; collapsing a group takes 19 rows to 3; a row label click selects the row and its 42 effects; a ruler click puts the cursor at that time; a ruler right click adds a mark ("Added 1 Mark"); a ruler drag shows the range.

Pictures read: `t3-drag-mid` (info box with effect name, layer, start, end, length; cursor line), `t3-band` and `t3-band-done` (rubber band, active row fill, selected borders), `t3-range` (range arrow on the ruler), `t3-row-click` (selected row band behind the effects; the label itself isn't highlighted, as upstream's `RowLabel.OnPaint` has no selected state). Nothing wrong seen.

### Frame timing, re-measured (release build, "Christmas Eve Sarajevo.tim", 1,095 effects, 1,136 rows, 2x)

The display was awake and the display link fired: 144 Hz, paced by the display, so a frame is 6.94 ms. Milliseconds; "busy" is main-thread time in a frame, "gap" the time between frames.

| Case | Frames | Busy p50 | Busy p95 | Busy p99 | Busy max | Gap p95 | Gap max | Late frames |
|---|---|---|---|---|---|---|---|---|
| Scroll horizontally, 10 points a frame | 600 | 0.11 | 0.31 | 0.85 | 1.12 | 6.97 | 13.90 | 1 |
| Scroll vertically, 6 points a frame | 300 | 0.16 | 1.07 | 1.33 | 2.77 | 6.96 | 7.00 | 0 |
| Zoom with ⌘ wheel, in and out | 300 | 0.12 | 1.48 | 2.80 | 3.33 | 6.97 | 7.04 | 0 |
| Pinch, in and out | 300 | 0.13 | 1.74 | 2.90 | 3.56 | 6.97 | 7.02 | 0 |
| Fast flick, 80 points a frame | 300 | 0.09 | 0.77 | 1.43 | 2.20 | 6.96 | 7.37 | 0 |
| Whole sequence in view, scroll rows | 300 | 0.07 | 0.38 | 0.44 | 1.08 | 6.95 | 6.96 | 0 |
| Playback line and follow | 600 | 0.07 | 0.17 | 0.56 | 1.63 | 6.96 | 15.14 | 1 |
| Drag one effect | 240 | 0.15 | 0.88 | 1.28 | 1.38 | 6.96 | 8.93 | 0 |
| Drag every effect (select all, 1,095) | 240 | 0.50 | 1.32 | 2.61 | 10.49 | 6.97 | 10.54 | 0 |
| Rubber band across the view | 240 | 0.08 | 0.14 | 0.27 | 1.02 | 6.96 | 7.34 | 0 |
| Resize one effect | 240 | 0.23 | 1.23 | 1.40 | 1.65 | 6.96 | 7.02 | 0 |

During a zoom every frame has some tile still repainting (the old picture stays, stretched) and the tiles catch up 55 to 62 ms after the last step; everywhere else they catch up within one frame. One run each. The builder's numbers aren't in `docs/parity/timeline.md` as the brief said; its last run is `target/timeline-out/timeline-bench.json` (same machine, 144 Hz) and agrees: busy p95 under 3 ms in every case there, under 1.8 ms here. Its first run (`bench-run1.txt`, 120 Hz, 3 rows) had a 150 ms worst gap in zoom that neither later run shows.

### Harness run, marks bar and the rest (`t4.py`, `t5.py`, same build and show copy)

Read back from the dump: the marks bar shows (height 260, 45 marks in view of 1,391); a click selects one mark; a 40 px drag moves it 30 px, because the move is counted from where the pointer leaves the 8 px wait area, as `MarksBar.cs:453-456, 806-808` does, and records "Moved 1 effect" (Q23); nothing is recorded or marked modified in mid-drag; undo puts it back; a 20 px front-edge resize moves the start 20 px and shortens it 20 px ("Resize 1 effect"); Delete removes it ("Removed 1 Mark") and undo restores it; a grid press clears the mark selection; a ruler press off a mark leaves it; hovering a mark on the ruler puts a guide at its start; a ruler drag from a mark moves it. Row zoom in gives 78 point rows; far zoom gives 2,249 ticks a point; zoom to total starts at 0 and fits the whole sequence to the grid's 862 points; Expand All on "Christmas Eve Sarajevo" gives 1,136 rows; a Draw Mode box over two rows adds two selected effects with one undo entry, and undo removes them; Esc in a drag leaves no undo entry; after Stop the cursor hasn't moved.

Pictures read (all 14 of `t5-*`, 6 of `t4-*`, `t3-hover`, `t3-moved-row`): mark bars with stacking, the selected mark's dotted border, yellow guides at a dragged mark's start and end on waveform, ruler and grid, the time label beside a mark dragged on the ruler, the full (mirrored) waveform, yellow labels on rows with effects, row heights in and out, far zoom ticks at 20 ms, total zoom, deep expanded rows with thin lanes, the draw box and the two drawn effects, the CAD band with its green border, the red resize indicator line, the green playback arrow and line at 2.0 s, and their absence after Stop. Two things looked wrong and weren't: the hover box sits above the hovered effect, over the row above (`Element.cs:482-491`, `model.rs:1544`), and its first line has no leading space while the others do (`Element.cs:467`). The left arrow at 0 on Sarajevo's ruler is the sequence's saved playback start. Nothing wrong seen.

### Parity rows marked done, confirmed in the code (36, none wrong)

Each row's named file and function exists and does what the row says. Line numbers are as of this run.

- F5 `T/view.rs:39, 48` `px`, `time_at`; `T/ruler.rs:336` `ruler_time`; `T/marks.rs:585` `marks_canvas`.
- F13 `T/commands.rs:428-442`: a tilt pans on every time surface and not on the row list.
- F17 `T/commands.rs:536` `magnify`, anchors as items 34 and 35; the pinch bench case above.
- F19 `T/row_input.rs:98-130`: the expander acts only on an odd click count.
- F24 `T/row_input.rs:252`. F28 `T/row_input.rs:202-218`, clamped at both ends and scrolled into view.
- F30 `T/row_input.rs:68, 98-107`, `T/rows.rs:574` (finding 4's code).
- F35 `T/rows.rs:616-622` against `TimelineControl.cs:1091-1108`. F36 `T/rows.rs:625-635` against `TimelineControl.cs:1122-1141`: the first selected row and every descendant take the current default height.
- F39 `T/stack.rs:139-176` against `Grid.cs:2956-3003`: lane height, the zero-height red border, the four width cases with the same truncations, x floored.
- F45 `T/model.rs:1374, 1491, 1544` against `Element.cs:457-500` and `Grid.cs:3035-3052`. F57 `T/model.rs:991-1000`.
- F48, F51, F62, F66 `T/grid_input.rs:806, 747-814, 1079, 1215` (items 11 to 14, 19, 20). F70 `T/snap.rs:20` (item 23). F76 `T/snap.rs:174`, `T/marks.rs:1353, 1381`, and the pictures.
- F84 `T/commands.rs:254, 267` (item 29). F95 `T/commands.rs:555` (item 39).
- F89 `UI/Surfaces.swift:290-297`, delegate in `UI/TimelineCanvas.swift:28-30`: the timeline only reports time, row and element.
- F97 `UI/Painters.swift:269-341` against `Ruler.cs:174-232`. F102, F108, F112 `T/ruler.rs:468-630` (items 57, 58), `UI/TimelineCanvas.swift:503`.
- F120, F124 `T/marks.rs:493, 599` (item 44). F129 `T/marks.rs:1410-1421, 834` (item 48). F133 `T/marks.rs:1099, 1166` (item 53). F139 `T/ruler.rs:212` and item 44.
- F141 `T/waveform.rs:91-141` against `Waveform.cs:299-320`. F145 `T/waveform.rs:149-165` against `Waveform.cs:136-148` (30 px floor, the 6 px strip). F146 `T/waveform.rs:168` against `Waveform.cs:151-159` (50). F148 `T/waveform.rs:182` and the picture of the mirrored waveform.
- F150: `cursor_kind` is a field, set in `T/grid_input.rs:202, 348, 523, 663, 912`, `T/row_input.rs:84, 90` and `T/model.rs:1200`. The row's location is right.
- F153, F155: `T/grid_input.rs:510`, `T/model.rs:583, 590, 1205`, and their two tests in `T/tests.rs` (`with_session`).

Not re-derived: F99's scale table (`ruler_scale`, `label_interval`), which has unit tests and matched in the pictures; F151.

### Final test run

`cargo test -p vixen-session --lib timeline` (target dir `target/timeline-review-cargo`): 286 passed. `vixen-ffi` has no tests of its own. The second run changed no source, so `real_show.rs` and `tools/golden/filecheck.sh` were not re-run.

The tests that cover the five findings: `a_pasted_mark_takes_the_duration_floor_as_clone_does` (`T/marks.rs:1691`), `a_height_drag_shows_up_down_over_the_bar_and_an_arrow_off_it_f30` (`T/tests.rs:366`), `a_right_press_on_one_label_released_on_another_opens_the_menu_and_clicks_nothing_f33` (`T/tests.rs:381`), `tags_the_tag_manager_saved_reach_the_labels_and_the_hidden_filter_f38` (`T/tests.rs`, through `reload_tags`). Finding 1 is in Swift (`UI/TimelineCanvas.swift:286-291`) and has no automated test.

### Left

- Adding a mark when collections exist and none is the default: upstream asks to enable "the active mark collection" and OK does nothing (`Form.cs:3058-3072`); `T/marks.rs:1308` doesn't ask. In `docs/DIFFERENCES.md`.
- The builder's frame timings aren't in `docs/parity/timeline.md`, though the brief says so; they are only in `target/timeline-out/`. This run's are above.
- Each bench case ran once.

## Verification

Adversarial pass over the five fixes above (started 2026-11-09). Each: confirmed, corrected or reverted, with what was read.


### The five fixes

Each of the four tested fixes was also undone in a scratch copy of `crates/vixen-session` (the shared crate wasn't touched) and the timeline unit tests run there: exactly the four new tests failed and the other 282 passed (`target/timeline-review-out/verify-mutation.log`). So each test does catch its defect.

1. **Pause holds the playback line: confirmed, with one small correction.** Read `Form.cs:3395-3463` and `TimeInfo.cs:80-105`. `PauseSequence` only calls `_context.Pause()`; `timerPlaying` starts in `context_SequenceStarted`, stops in `context_SequenceEnded`, and the line is cleared and the remembered range restored only in `context_ContextEnded`. The Swift does the same. Upstream's timer keeps ticking through a pause, so a pause that lands before the first tick still gets its line at the held position; the fix drew nothing in that case and, because it had never seen "playing", also skipped `playbackEnded` at stop, so the remembered ruler range wasn't restored. Corrected in `refreshTick`: a pause seen before any playing tick draws one frame and counts as active. Still no automated test: the harness can't pause.
2. **`tags_changed` reloads tags: confirmed.** Read `Form.cs:2320-2326, 2345-2360` and `TimedSequenceRowLabel.cs:40-100`. Upstream's labels and its hidden filter read the live tags at every paint and recompute, so repainting is all its handler needs; the port keeps copies, so the reload plus re-applying the hidden filter gives the same result. `build_row_frame` takes the dot colours from the reloaded tags. The lock order comment is right: the system lock is released before the timeline's is taken.
3. **A pasted mark takes the 25 ms floor: confirmed.** Read `Mark.cs:20-110` and `MarksBar.cs:364-420`. `Clone()` sets `Duration` through the setter, then paste sets `StartTime` through its setter; the port does the same in the same order.
4. **Pointer during a row height drag: corrected (comment and record, not behaviour).** Read `RowLabel.cs:84-156`. The fix's intent is right: upstream takes the pointer from the hit test on every move, it isn't forced to up-down during a drag. But upstream tests *before* the row changes height, against the label as last painted, so a move of 3 pixels or more in one event shows the arrow until the next small move. The fix tests *after* the height change, so the pointer stays up-down while it tracks the bar and turns to the arrow only at the 10 point floor or sideways off the label. Its comment said "before resizing", which the code doesn't do. I kept the steady pointer (a resize drag on the Mac keeps its resize pointer; the flicker is nothing anyone relies on), rewrote the comment to say what the code does, and recorded it in `docs/DIFFERENCES.md`.
5. **Right press on one label released on another: confirmed.** Read `RowLabel.cs:91-139` and `TimelineControl.cs:912-935`. The pressed label has the mouse and gets the release; the label click runs only when the release is inside it, and `_RowContextMenuSelect()` runs regardless. The menu acts on the selected rows, not the pressed one, in both.

### Ten more parity rows

Rows marked done that the first pass didn't check, each read in the C# and in the port. None was wrong.

| Row | Read | Result |
|---|---|---|
| F8 vertical offset | `Grid.cs:255-272, 2568-2592`, `TimelineControl.cs:852-885`; `model.rs` `set_vertical_offset`, `rows_changed` | Clamps in upstream's order, then the guard at 0; unchanged does nothing; re-clamped after every row change |
| F14 trackpad scroll (new on the Mac) | spec F14; `commands.rs` `scroll_precise`, `apply_wheel_delta`, `row_step` | A point per point; ⇧ pans with either axis; ⌘ forms go through the wheel table at `delta * 10`, row heights stepping at 120; over the marks bar only sideways scroll pans |
| F29 toggle selected rows | `Grid.cs:2637-2671`, `Row.cs:185-218`, `Hotkeys.cs:212-214`; `row_input.rs` `toggle_selected_rows` | Selected rows or else the active row; each row's new state is read as its turn comes, as upstream, so a selected parent and child with ⌘T end the same way; one layout at the end; nothing marked modified |
| F34 Collapse All Groups | `TimelineControl.cs:1110-1120`; `rows.rs` `collapse_all`, `row_input.rs` `collapse_all_rows` | Every open row closes; not marked modified, as upstream (only a height change is) |
| F40 stacking | `Row.cs:509-597`; `stack.rs` `walk_stacks` | Skip, stop, forward cover, backward with no early stop, first fit, and the advance past the cluster all match. The running-max shortcuts only skip items that could not match |
| F44 hit testing | `Grid.cs:1562-1575, 1627-1737`; `model.rs` `elements_at`, `element_at`, `elements_at_time`, `elements_within`, `rows.rs` `row_at` | Same tests and bounds. `element_at` stops at the first effect starting past the point, which can't change the answer in a sorted row |
| F90 reveal effects | `Grid.cs:987-1012`; `commands.rs` `reveal_elements` | Selects, shows the row, opens and shows every ancestor, then visible start and vertical offset from the last effect, then selection changed |
| F105 Start and End | `Toolstrip.cs:1596-1616`, `TimeInfo.cs:64-75`; `ruler.rs` `playback_range_to_start`, `playback_range_to_end` | Range, remembered range and visible start set as upstream |
| F113 ruler height drag | `Ruler.cs:458-497, 571-575, 600-626`, `Form.cs:372, 5832`; `ruler.rs` `ruler_mouse_down`, `ruler_mouse_move` | A mark under the press wins; the strip is the bottom 6 points; `height = y + 1` only when `y > 40`; the lock is tested at press and on each move |
| F118 mark stacking and bar height | `MarkRow.cs:14, 78-187, 235`, `MarksBar.cs:1155-1158`; `marks.rs` `refresh_layout` | Same walk as F40; a mark reached twice keeps its first values (the struct copy); row height 20 times the most lanes in view; the bar is the sum of shown rows. The cache key (first item still running, first item past the range) does decide the whole walk |

### Builds and tests after the corrections

- Release build of the app with the two corrections in: `VIXEN_BUILD_ID=timeline-review VIXEN_AREAS=timeline,admin scripts/build_app.sh release`, clean (`target/timeline-review-out/verify-build-release.log`).
- `cargo test -p vixen-session --lib timeline`: 286 passed, 0 failed (`verify-unit-3.log`). A first try at 16:35 did not compile because of another area's test in `crates/vixen-session/src/marks/map_library.rs` (it imported two `pub(crate)` functions of `vixen-io`); that was fixed by its owner by the time of the second try and was nothing to do with the timeline.
- `cargo test -p vixen-session --test real_show`: 3 passed (`verify-real-show.log`).
- `tools/golden/filecheck.sh` was not run: nothing that reads or writes a file changed in this pass (one Rust comment, one Swift playback tick, documents), and the reviewer's fixes do not touch saving either (a pasted mark's duration is floored in memory before it is ever saved).
- The reviewer's `t3.py` run again against the new build prints exactly what it printed for the reviewer (`t3.log` and `verify-t3.log` are the same line for line), including the assertion at its line 81 that the reviewer's own run stopped on (the rubber band case the reviewer then redid in `t3b.py`). So no change there.
- The reviewer's `t3b.py`, `t4.py` and `t5.py` against the new build: all three end `OK` with the same lines the reviewer got (`verify-t3b.log`, `verify-t4.log`, `verify-t5.log`; the only difference is the random id of the mark made in t4).

### The harness on what the fixes touched

Script `target/timeline-review-out/v1.py`, release build, "Coldest Winter.tim". Logs `verify-v1-rows.log` and `verify-v1-pause.log`.

- Fix 1, pause. Play, wait 2 s, Pause: transport `paused`, line at x 312.7 (2001 ms), and two pictures taken 1.5 s apart while paused are the same file byte for byte, with the green line in them (`v1-paused-a.png`, `v1-paused-b.png`). Resume: playing again and the line has moved on (3085 ms, x 482). Stop after a pause: transport `stopped`, line gone (`playbackLineX` -1), the ruler range as it was. Play followed at once by Pause (the case the correction covers): paused at 51 ms with the line at x 8, and Stop then clears it. The harness cannot tell whether the 0.1 s tick saw "playing" before the pause in that last run, so the correction is confirmed by reading and by the line being there, not by a run that is known to have taken the new branch.
- The test scene's Play starts from zero and leaves the ruler range alone, so "the remembered range comes back at the end" is not exercised by the harness. It is covered by the unit test `command_click_plays_and_restores_the_range_afterwards` in `ruler.rs` and by reading `stopPlaybackLink`, which is the only caller of `playbackEnded`.
- Fix 4, row height drag. A press on White Bottom's bar dragged 30 down: height 51 to 81. Dragged 200 up: stops at 10. The pointer's shape is not in the dump or the pictures; only the unit test covers it.
- Fix 5, right press across labels. With White Bottom selected, a right press on White Middle released over Color Bottom leaves White Bottom the only selected row (no click). The control, a right click that stays on White Middle, selects it. The menu itself cannot be seen under the harness (`popUp` returns early there), so "the menu opens for the pressed row" rests on the unit test and on reading `showRowMenu`.
- Fixes 2 and 3 (tags reaching the rows, the pasted mark's floor) have no harness path. Unit tests only, plus the mutation run above.

### Performance, release build, "Christmas Eve Sarajevo.tim"

Three runs of `bench.py`, one after the other, nothing else of mine running (`verify-bench-1.txt` to `-3.txt`). The machine was busy with other workers (load average 17 at the start). The display was awake: every run reports `pacedBy: display` at 144 Hz, a 6.94 ms frame. 1136 rows, 1095 effects.

Main thread busy per frame, ms, as p95 / max over the three runs:

| Case | Run 1 | Run 2 | Run 3 | Reviewer's one run |
|---|---|---|---|---|
| Scroll horizontally, 10 points a frame | 0.42 / 1.63 | 0.42 / 1.60 | 0.23 / 1.04 | 0.31 / 1.12 |
| Scroll vertically, 6 points a frame | 0.60 / 4.83 | 0.63 / 5.59 | 0.34 / 3.67 | 1.07 / 2.77 |
| Zoom with the command wheel | 2.24 / 2.46 | 2.21 / 2.97 | 1.78 / 4.49 | 1.48 / 3.33 |
| Pinch | 2.31 / 3.78 | 2.30 / 4.21 | 2.47 / 3.97 | 1.74 / 3.56 |
| Drag every effect (1095 selected) | 0.44 / 0.57 | 1.47 / 1.75 | 0.67 / 0.77 | 1.32 / 10.49 |
| Playback line and follow | 0.19 / 2.23 | 0.30 / 2.15 | 0.27 / 2.00 | 0.17 / 1.63 |

Frame gaps: p99 at most 7.03 ms in every case of every run. No frame longer than two periods in any scroll, zoom or drag case; one in playback in run 2 (16.8 ms). The reviewer's 10.49 ms worst frame while dragging everything did not come back in three runs (worst 1.75 ms), so it reads as a one-off on a busy machine, not something the drag does. While zooming, every frame has a tile still repainting (the old picture is stretched meanwhile) and the sharp picture is back 55 to 69 ms after the last step; that is the same as the reviewer saw.

### Not verified in this pass

- The pointer's shape anywhere, and any context menu opening, in the running app: the harness shows neither.
- The correction to fix 1 by a run known to take the new branch (see above).
- The tags fix end to end through the tag manager's window; the unit test drives the session directly.
- The reviewer's left-over defects were not re-examined beyond reading them.
- `tools/golden/filecheck.sh` (not relevant, see above).
- Performance on anything but this one sequence and this machine while it was busy.

### For other areas (not fixed here)

- `transport()` and `position_ms()` in `crates/vixen-ffi/src/sequence.rs` are the session's, not a sequence's, so a second open sequence's timeline would show the first one's playback line. Sequencer area.
- `vixen-io` does not floor a mark's duration when loading as upstream's setters do; the paste fix floors at paste only. File area.
- The port's tag manager (`crates/vixen-session/src/setup/tags.rs`) can add tags of the user's own, where upstream's only recolours the built-in ones. Setup area; check it is in `docs/DIFFERENCES.md`.
- The reviewer's log says the harness cannot pause. It can: the test scene has Pause and Resume in its menu, which is what `v1.py` uses.


### Later resolutions — November 11, 2026

The first three cross-area findings above are now superseded by current code:

- SequenceSession has an immutable playback identity. FFI transport/position and
  transport commands are scoped to that identity; the shell no longer uses a
  process-global owner. Replacement, stale close/completion/commands, preview
  ownership and independent profiles pass focused regressions in
  `target/session-ownership-*.log`. Native integration is tracked in
  `docs/REMAINING_VERIFICATION.md`.
- The sequence loader calls `mark.apply_minimum_duration()` in
  `crates/vixen-io/src/sequence.rs`; the 25 ms floor is no longer paste-only.
- Manage Tag Colors uses `built_in_color_rows` in
  `crates/vixen-session/src/setup/tags.rs`, filtering `is_built_in`; custom tag
  creation belongs to the separate tag-definition editor, not this dialog.

These updates close stale source findings, not the older pass's unrelated native
pointer/menu or performance limitations.
