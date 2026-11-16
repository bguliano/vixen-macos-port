# Timeline parity

Status of every entry in `docs/spec/timeline.md`. This table is how passes hand over to each other.

Status values: **done** (with where it lives), **partial** (with what is missing), **omitted** (with why).

Paths: `T/` is `crates/vixen-session/src/timeline/`, `FFI` is `crates/vixen-ffi/src/timeline.rs`, `UI/` is `app/Sources/VixenTimeline/`.

The last column says how an entry has been checked. **U**: a unit test in `T/`. **H**: driven with the harness on a copy of the real show, with the result read back from a dump. **S**: looked at in a snapshot. **-**: compiled, and not exercised. Entries owned by the sequencer window say so, and were not exercised through it here.

The drive scripts are `target/timeline-out/t3.py` (grid, rows, ruler), `t4.py` (marks bar and marks on the ruler) and `t5.py` (zoom, rows, draw mode, CAD, indicator, keys), over `drive.py`. The frame timing is `TimelineBench.swift`, run with `VIXEN_TIMELINE_BENCH=1` through `bench.py`.

| F | Feature | Status | Where, and what differs | Checked |
|---|---|---|---|---|
| F1 | Timeline panel | done | `UI/TimelinePane.swift`, the main area of the window. No tab title or closable panel, as the spec's Mac note says. No separate 1 px border: the split view's edges do that | S |
| F2 | Splitter | done | `UI/Surfaces.swift` `TimelineContainerView`; restore rule and minimum 100. Persisted through `FFI` `save_profile_settings` | S |
| F3 | Stacked regions | done | `UI/Surfaces.swift` `TimePanelView`, `RowPaneView` | S |
| F4 | Shared time state | done | `T/view.rs` `TimeInfo`, `T/model.rs` `Timeline`: one per window, not global dictionaries | U |
| F5 | Time to pixel mapping | done | `T/view.rs` `px`, `time_at`, with Q1's f64. `Timeline::to_canvas`, `ruler_time`, `marks_canvas` | U, H |
| F6 | Visible range and clamping | done | `T/view.rs`, `T/model.rs` `set_visible_start`, `set_viewport`. Copied to the document by `store_view_state` just before a save rather than on every change, and the editor state flag is set on every change (F153) | U |
| F7 | Horizontal scroll bar | done | `UI/Surfaces.swift` `TimelineScroller`, `GridViewportView`. Always shown (dimmed when there's nothing to scroll), because upstream's are real bars you drag; Q46 in `T/model.rs` `set_time_per_pixel`, `set_total` | U, H |
| F8 | Vertical scroll bar and offset | done | `T/model.rs` `set_vertical_offset` with the negative-limit guard, `rows_changed` | U, H |
| F9 | Vertical gridlines | done | `T/model.rs` `grid_frame`, `UI/Painters.swift`. Skipped under 2 points apart, as the hazards table allows | U, S |
| F10 | Grid paint order | done | `UI/Painters.swift` `paintGridTile`, layers in `UI/Surfaces.swift` `GridCanvasView`. Nothing paints before the load finishes, because the controller is built complete off the main thread | S |
| F11 | Wheel over grid, ruler, waveform, row list | done | `T/commands.rs` `scroll_wheel`, `apply_wheel_delta`; `UI/TimelineCanvas.swift` `TimelineSurface.scrollWheel` | U, H |
| F12 | Wheel over the marks bar | done | `T/commands.rs` `apply_wheel_delta` | U |
| F13 | Tilt | done | `T/commands.rs` `scroll_wheel`, on every time surface | U |
| F14 | Trackpad scroll | done | `T/commands.rs` `scroll_precise` | U, H |
| F15 | Zoom time | done | `T/view.rs` `zoom_decision`, `zoom` | U, H |
| F16 | Zoom under the pointer | done | `T/view.rs` `zoom_at`; option `zoom_under_mouse` | U |
| F17 | Pinch | done | `T/commands.rs` `magnify`, anchored on the pointer. `UI/TilePlane.swift`: during any zoom, wheel or pinch, the tiles only stretch, by a layer transform, and a new set starts at most every 100 ms and when it stops. `TimePerPixel` itself follows each event, since nothing heavy hangs off it. ⇧ scales rows | U |
| F18 | Row tree | done | `T/rows.rs` `load_rows` | U, H |
| F19 | Expand and collapse | done | `T/row_input.rs` `row_mouse_up`, which acts only on an odd click count | U, H |
| F20 | Row visibility rules | done | `T/rows.rs` `recompute_visibility`, `set_visibility_filter` | U |
| F21 | Row label appearance | done | `UI/Painters.swift` `paintRowsTile`, `paintExpander`, with F23's yellow. A drawn plus or minus rather than Vixen's bitmap | S |
| F22 | Tree connector lines | done | `T/rows.rs` `connector_lines` | U, S |
| F23 | Highlight rows with effects | done | `T/rows.rs`, `FFI` `row_frame`; option `highlight_rows` | U |
| F24 | Show Hidden Rows | done | `T/row_input.rs` `set_show_hidden_rows`. The menu item is the sequencer's | U |
| F25 | Click a row label | done | `T/row_input.rs` `click_row_label`, with Q37 | U, H |
| F26 | Row highlight painting | done | `UI/Painters.swift` `paintGridTile`, `GridCanvasView.edgePlanes`: fixed one-point viewport strips use the same row/mark/effect painter, so async scrolling tiles cannot leave interior border ghosts.29 native steps plus exact edge pixels verify select/multiselect/clear,100pt scroll, resize and i32-boundary cache/zoom guard (`target/progress-ui/row-edge-strips`) | S |
| F27 | Active row | done | `T/model.rs` `paste_target_row`, `add_effect_target_row` | U |
| F28 | Arrow keys move the active row | done | `T/row_input.rs` `move_active_row` | U |
| F29 | Toggle selected rows | done | `T/row_input.rs` `toggle_selected_rows` | U |
| F30 | Drag a row's height | done | `T/row_input.rs` `row_mouse_move`, `row_mouse_up`; `T/rows.rs` `finish_row_resize` | U |
| F31 | Zoom all rows | done | `T/rows.rs` `zoom_rows`, with Q36 | U, H |
| F32 | Zoom the row under the pointer | done | `T/rows.rs` `zoom_row_at`, with Q2 | U |
| F33 | Row label context menu | done | `T/row_input.rs` `row_mouse_up`, with upstream's doubled click; `UI/TimelineCanvas.swift` `showRowMenu` | U |
| F34 | Collapse All Groups | done | `T/rows.rs` `collapse_all` | U |
| F35 | Reset All Rows | done | `T/rows.rs` `reset_row_heights` | U |
| F36 | Reset Selected and Child rows | done | `T/rows.rs` `reset_selected_row_heights`, disabled with no selected row | U |
| F37 | Tags submenu | done | `T/row_input.rs` `tag_menu_state`, `toggle_tag_on_selected`; `FFI` `toggle_tag_on_selected` saves system config; `UI/TimelineCanvas.swift` `showRowMenu` | U |
| F38 | Manage Tags | done | Delegate `timelineManageTags`, then `FFI` `tags_changed`. The window is another area's | - |
| F39 | Effect rectangle | done | `T/stack.rs` `occurrence_rect`, `lane_geometry`, with Q3. A tile takes the whole effect and clips, which draws the same | U, S |
| F40 | Stacking | done | `T/stack.rs` `walk_stacks`, with Q4. `T/rows.rs` keeps a running max end, so the scan shortcut gives the same answers | U, S |
| F41 | The same effect in several rows | done | `T/model.rs` `add_element_to_targets`, per-occurrence lanes | U |
| F42 | Effect appearance | done | `UI/Pictures.swift`, `T/thumb.rs`, `UI/Surfaces.swift` `OccurrenceLayer`. A whole-effect picture per zoom, stretched during a resize, which gives the slice mapping the spec spells out | S |
| F43 | Placeholder and background render | partial | `T/model.rs` `update_picture_state`, `T/session_link.rs` `refresh_render_state`, `FFI` `render_progress`. Rendering is `SequenceSession`'s batch worker, so placeholders clear a batch at a time, and the percent moves per batch | U, S |
| F44 | Hit testing | done | `T/model.rs` `elements_at`, `element_at`, `elements_at_time`, `elements_within` | U, H |
| F45 | Hover effect info | done | `T/model.rs` `hover_info`, `hover_info_text`, `hover_box_origin`; `UI/Surfaces.swift` `applyHover`, with Q33 | U, S |
| F46 | Press on the grid | done | `T/grid_input.rs` `press` | U, H |
| F47 | ⌘-click | done | `T/grid_input.rs` `press`, `mouse_up_no_drag` | U |
| F48 | ⇧-click range | done | `T/grid_input.rs` `select_elements_between` | U |
| F49 | Release without a drag | done | `T/grid_input.rs` `mouse_up_no_drag` | U, H |
| F50 | Overlap chooser | done | `T/grid_input.rs` `offer_to_host`, `finish_chooser`, with Q47; `UI/TimelineCanvas.swift` `showOverlapMenu`. The real `NSMenu` can't run under the harness | U |
| F51 | Rubber band | done | `T/grid_input.rs` `begin_drag_select`, `mouse_move_drag_select`, `select_elements_within`, `mouse_up_drag_select` | U, H, S |
| F52 | CAD selection box | done | `T/grid_input.rs` `select_elements_within`, `selection_green` | U |
| F53 | Drag Box Filter | done | `T/model.rs` `DragBoxFilter`. The toolbar is the sequencer's | U |
| F54 | Right press and release | done | `T/grid_input.rs` `grid_mouse_up`, Q5; the context menu is the sequencer's, through `timelineShowEffectMenu` | U, H |
| F55 | Double-click an effect | done | `T/grid_input.rs` `grid_double_click`. Nothing listens, as upstream | U |
| F56 | Select all | done | `T/model.rs` `select_all`, Q6 | U, H |
| F57 | Selected list order | done | `T/model.rs` `selected_elements` | U |
| F58 | Drag to move | done | `T/grid_input.rs` `begin_drag_move`, `mouse_move_drag_moving`, `elements_finished_moving`; lifted layers in `UI/Surfaces.swift`. Modified from the first move | U, H, S |
| F59 | Drag modifiers | done | `T/grid_input.rs` `handle_mouse_move`, Q8 | U |
| F60 | Clone by dragging | done | `T/grid_input.rs` `clone_selected_for_move`, `clone_elements` | U |
| F61 | Moving between rows | done | `T/grid_input.rs` `move_elements_vertically`, `element_changed_rows`, Q9, Q38, Q39 | U, H |
| F62 | Deprecated rows refuse effects | done | `T/grid_input.rs` `move_elements_vertically` | U |
| F63 | Edge auto-scroll | done | `T/grid_input.rs` `edge_distance`, `grid_autoscroll_tick`; `UI/TimelineCanvas.swift` 50 ms timer | U |
| F64 | Esc during a drag | done | `T/grid_input.rs` `grid_cancel_drag`, Q10. Draw Mode and the effect tree are the sequencer's Esc | U, H |
| F65 | Resize zones | done | `T/grid_input.rs` `resize_zone`, `update_resize_zone` | U, H |
| F66 | Resize | done | `T/grid_input.rs` `mouse_move_h_resizing`, Q34 | U, H |
| F67 | Adjoining resize | partial | `T/grid_input.rs` `adjoining`, `apply_adjoining`. The neighbour isn't queued for render until release; it stretches until then like the effect itself. A render mid-drag would take the session's lock on every move | U |
| F68 | Snap on and off, strength | done | `T/model.rs` `set_snap_strength`, option `snap_enabled` | U |
| F69 | What can be snapped to | done | `T/grid_input.rs` `calculate_snap_points`, `T/snap.rs` `MarkSnapIndex` | U |
| F70 | Snap window | done | `T/snap.rs` `snap_window` | U |
| F71 | Snap algorithm | done | `T/snap.rs` `find_snap`, a range query matching a full scan, Q11, Q12, Q13 | U |
| F72 | Row scope | done | `T/snap.rs` | U |
| F73 | Move versus resize | done | `T/grid_input.rs` | U |
| F74 | Opposing edges | done | `T/snap.rs` | U |
| F75 | Marks at identical times | done | `T/snap.rs` `update_marks`, by mark id | U |
| F76 | Mark lines on the grid | done | `T/snap.rs` `mark_lines`, `T/marks.rs` `without_moving_marks`, `moving_mark_lines`; guides in `UI/Surfaces.swift` | U, S |
| F77 | Draw Mode | done | `T/grid_input.rs` `begin_draw_box`, `mouse_move_draw_select`, `finish_drawing`, `clamp_draw_times`, Q40 | U, H, S |
| F78 | Middle-button draw | done | `T/grid_input.rs` `press`; `UI/TimelineCanvas.swift` `otherMouseDown` | U |
| F79 | Resize / Draw indicator | done | `T/grid_input.rs` `resize_indicator_lines`, `publish_indicator_guides`, Q14 | U, H |
| F80 | Align commands | done | `T/commands.rs` `align`; keys in the sequencer's table and `UI/Surfaces.swift` `timelineKeyDown` | U |
| F81 | Batch move and limits | done | `T/commands.rs` `move_resize_batch`, `move_resize_element`, `ok_to_use_alignment` | U |
| F82 | Distribute | omitted | The sequencer spec's: it computes times and calls `move_resize_batch` | - |
| F83 | Align to nearest marks | omitted | The sequencer spec's | - |
| F84 | Split | done | `T/commands.rs` `split_at`, `split_selected_at`, Q17 | U |
| F85 | Close Gaps | done | `T/commands.rs` `close_gaps`, Q41 | U |
| F86 | Bulk move | done | `T/commands.rs` `move_in_range_by_time`. The dialog is the sequencer's | U |
| F87 | Nudge, and move by time | done | `T/commands.rs` `nudge_selected`, `move_by_time`, Q18. A clipped effect leaves its last row only, as upstream, and stays in the sequence | U |
| F88 | Undo and redo of times and rows | done | `crates/vixen-session/src/undo.rs` `PlacementChanged`, `RowsRestored`; `T/session_link.rs` `sync_from_session`. Redo of a clip puts the effect back, as `SwapElementPlacement` does, without upstream's duplicate row entry | U, H |
| F89 | Drop on the grid | done | `UI/Surfaces.swift` `draggingUpdated`, `performDragOperation`, delegate `timelineValidateDrop`, `timelinePerformDrop`. What a drop does is the sequencer's | - |
| F90 | Reveal effects | done | `T/commands.rs` `reveal_elements` | U |
| F91 | Edit cursor | done | `T/model.rs` `set_cursor`; layers in `UI/Surfaces.swift` | U, H, S |
| F92 | When the cursor moves | done | `T/model.rs`, `T/grid_input.rs`, `T/ruler.rs` | U, H |
| F93 | Legacy Cursor | done | Option `legacy_cursor` | U |
| F94 | Playback line and arrow | done | `UI/TimelineCanvas.swift` `playbackFrame`, `UI/Surfaces.swift` `setPlayback`. Q19 | - |
| F95 | View follows playback | done | `T/commands.rs` `follow_playback` | U |
| F96 | Current time status | done | Delegate `timelineCursorMoved`; the status bar is the sequencer's | - |
| F97 | Ruler surface | done | `UI/Painters.swift` `paintRulerTile`, `UI/Surfaces.swift` `RulerView` | S |
| F98 | Ticks | done | `T/ruler.rs` `ruler_scale`, `ruler_frame` | U, S |
| F99 | Time labels | done | `T/ruler.rs` `ruler_label`, `label_interval`, `ruler_frame` | U, S |
| F100 | Click the ruler | done | `T/ruler.rs` `ruler_clicked` | U, H |
| F101 | ⌘-click the ruler | done | `T/ruler.rs` `ruler_clicked`, `playback_ended` | U |
| F102 | Drag the ruler | done | `T/ruler.rs` `ruler_drag_range`, `ruler_range_dragged` | U, S |
| F103 | ⌘-drag the ruler | done | `T/ruler.rs` `ruler_range_dragged` | U |
| F104 | Range indicators | done | `UI/Painters.swift` `paintRange` | S |
| F105 | Start and End buttons | done | `T/ruler.rs` `playback_range_to_start`, `playback_range_to_end`. The buttons are the sequencer's | U |
| F106 | Marks on the ruler | done | `T/ruler.rs` `ruler_frame`, Q20 | S |
| F107 | Mark hit test on the ruler | done | `T/ruler.rs` `marks_at`, Q21 | U |
| F108 | Hover on the ruler | done | `T/ruler.rs` `ruler_mouse_move`, Q44 | U, H |
| F109 | Click a mark on the ruler | done | `T/ruler.rs` `ruler_mouse_down` | U |
| F110 | Drag marks on the ruler | done | `T/ruler.rs` `ruler_drag_mark`, Q22, Q43, Q48 | U, H, S |
| F111 | Right-click the ruler | done | `T/ruler.rs` `ruler_right_clicked`, `T/marks.rs` `add_mark_at`; sheets in `UI/TimelineCanvas.swift` | U, H |
| F112 | Right-click a mark on the ruler | done | `T/ruler.rs` `ruler_mouse_up`, `T/marks.rs` `delete_marks_key`, `UI/TimelineCanvas.swift` `showMarkDeleteMenu` | U |
| F113 | Drag the ruler's height | done | `T/ruler.rs` `ruler_mouse_move` | U |
| F114 | Reset ruler height | done | `T/ruler.rs` `ruler_double_click` | U |
| F115 | Lock Ruler Height | done | `T/ruler.rs` `set_ruler_locked`. The menu is the sequencer's | U |
| F116 | Ruler pointer shapes | done | `T/ruler.rs` `ruler_mouse_move` | U |
| F117 | Mark rows | partial | `T/marks.rs` `mark_row_tops`, `marks_bar_frame`. Rows are always in collection order. Upstream's add appends a row whatever the collection's index, so its order can lag until a rebuild; the Mac doesn't reproduce that lag | U, S |
| F118 | Mark stacking and bar height | done | `T/marks.rs` `refresh_layout`, Q24, Q25, Q26 | U, S |
| F119 | Mark appearance | done | `T/marks.rs` `marks_bar_frame`, `ideal_text_color`, `locked_hatch_color`; `UI/Painters.swift` `paintMarksTile` | U, S |
| F120 | Mark hit test | done | `T/marks.rs` `mark_at` | U |
| F121 | Click a mark | done | `T/marks.rs` `marks_mouse_down`, `marks_mouse_up` | U, H |
| F122 | ⌘-click a mark | done | `T/marks.rs` `marks_mouse_down` | U |
| F123 | ⇧-click marks | done | `T/marks.rs` `select_marks_between`, Q27 | U |
| F124 | Double-click a mark | done | `T/marks.rs` `marks_mouse_down` | U |
| F125 | ⌥ double-click a mark | done | `T/marks.rs` `fill_gap_times`, `marks_fill_gap` | U |
| F126 | Drag marks along their rows | done | `T/marks.rs` `marks_drag_moving` | U, H, S |
| F127 | Resize marks | done | `T/marks.rs` `marks_h_resizing`, Q34, Q35 | U, H |
| F128 | Glued resize | done | `T/marks.rs` `marks_h_resizing`, Q28, Q29 | U |
| F129 | Auto-scroll in the marks bar | done | `T/marks.rs` `autoscroll_distance`, `autoscroll_visible_start`, `marks_autoscroll_tick` | U |
| F130 | Edit Text | done | `T/marks.rs` `edit_text_prompt`, `set_selected_marks_text`; `UI/TimelineCanvas.swift` `editMarkText` | U |
| F131 | Cut | done | `T/marks.rs` `cut_marks` | U |
| F132 | Copy | done | `T/marks.rs` `copy_marks`, Q30 | U |
| F133 | Paste | done | `T/marks.rs` `paste_marks`, `paste_questions` | U |
| F134 | Delete | done | `T/marks.rs` `delete_selected_marks` for the menu, `delete_marks_key` for the key, Q31, Q35 | U, H |
| F135 | Phoneme | done | `T/marks.rs` `marks_menu_model`, `PHONEMES`, Q42 | U |
| F136 | Breakdown Phrase | done | `T/marks.rs` `breakdown_phrase`, `linked_collection` | U |
| F137 | Breakdown Word | done | `T/marks.rs` `breakdown_word`, delegate `timelinePhonemes`. The lip sync dictionary and the mapping dialog are another area's | U |
| F138 | Guides from marks | done | `T/grid_input.rs` `set_guides` | U |
| F139 | Locked collections | done | `T/marks.rs`, `T/ruler.rs` `marks_at` | U |
| F140 | Waveform visibility | done | `T/waveform.rs` `set_audio`, `FFI` `audio_changed` | S |
| F141 | Waveform drawing | done | `T/waveform.rs` `waveform_frame`, `waveform_blank`; `UI/Painters.swift` `paintWaveformTile` | U, S |
| F142 | Building message | done | `FFI` `build_peaks`, `UI/Surfaces.swift` `WaveformView`, Q32 | U |
| F143 | Guides on the waveform | done | `UI/Surfaces.swift` `WaveformView`, a shape layer. `T/waveform.rs` `guides_dirty_rect` keeps upstream's rectangles for its tests | U |
| F144 | Cursor on the waveform | done | `UI/Surfaces.swift` `WaveformView` | S |
| F145 | Drag the waveform's height | done | `T/waveform.rs` `waveform_mouse_move` | U |
| F146 | Reset waveform height | done | `T/waveform.rs` `waveform_double_click` | U |
| F147 | Lock Waveform Height | done | `T/waveform.rs` `set_waveform_locked` | U |
| F148 | Full Waveform | done | `T/waveform.rs` `set_waveform_style` | S |
| F149 | Keys | done | `crates/vixen-session/src/sequencer/keys.rs`, `SequencerModel.handleKey`, `UI/Surfaces.swift` fallback. Native60 transport/loop steps,47 tap-mark steps and80 D/I distribution steps pass. Plain keys bypass AppKit/SwiftUI menu interception; modified menu shortcuts retain priority. Option-Space is the native pause alias (Command-Space is reserved by Spotlight). Sheet/text/editor focus guards remain. See `docs/VERIFICATION.md` | U, native |
| F150 | Pointer shape in each state | done | `T/grid_input.rs` `cursor_kind`, `T/model.rs` `set_selected_effect` | U |
| F151 | Drawing failures | done | `FFI` `frame`, `take_draw_failure`; `T/model.rs` `DrawFailures`; `UI/TilePlane.swift` keeps the last image; `UI/TimelineCanvas.swift` `refreshTick`. Q45 | U |
| F152 | Undo and redo of marks | done | `crates/vixen-session/src/undo.rs` `MarkTimesChanged`, `MarksAdded`, `MarksRemoved`; `apply_inverse` in `sequence.rs` | U, H |
| F153 | What marks the sequence modified | done | `T/grid_input.rs` `note_drag_changed`, `T/model.rs` `mark_editor_state_modified`, `is_editor_state_modified`, delegate `timelineEditorStateModified` | U, H |
| F154 | Keyboard focus | done | `UI/TimelineCanvas.swift` `TimelineSurface`, `UI/Surfaces.swift` `RulerView.acceptsFirstResponder` | - |
| F155 | Mark selection | done | `T/marks.rs`, `T/grid_input.rs` `grid_mouse_down`, `T/model.rs` `mark_selection` | U, H |

## Quirk decisions

Everything the register marks FIX is as the spec says: Q1, Q2, Q3, Q5, Q6, Q10, Q14, Q25, Q29, Q31, Q32, Q40, Q45, Q46, Q47, Q48. KEEP and GUARD entries are ported as written.

Where the Mac goes beyond the register, for the owner's veto:

- Tiles are 512 by 256 points, not the spec's 1024 by 1024: at 2x a large tile takes too long to paint for a scroll to keep up.
- An ⌥-resize neighbour doesn't render until release (F67).
- Row order in the marks bar never lags collection order (F117).
- F26 now draws its viewport edges in fixed narrow strips; native pixels and extreme-coordinate guards are verified.
- Pressing Delete twice in the bar: the second press records "Removed 0 Marks", which is Q31 and Q35 together. Undoing it does nothing.
- The hazards table's gridline skip under 2 points is in (F9).
- F153 asks that a mark's jump left by Q43 doesn't mark the sequence modified, and it doesn't.

Upstream behaviour kept that the owner may want changed: a clipped effect in Bulk Move stays in the sequence and keeps playing though it's gone from the row (F87), as `move_by_time` says.
