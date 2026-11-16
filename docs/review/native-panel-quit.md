# Normal-app Quit diagnostic — November 11, 2026

The normal-app file-panel run reached Open Cancel, Unicode Open, Save As Cancel, Unicode Save As, Close and Reopen, then did not exit after its injected Command-Q. The user subsequently explained that **Vorssaint requires holding Command-Q on this Mac**. The driver posted an immediate down/up tap, which does not satisfy that configured quit gesture. This explains the apparent shortcut failure as an environment/test mismatch; no Vixen production fix is indicated.

That setting is user-reported rather than independently inspected. The failed run remains retained and is not relabeled as a normal-shutdown pass. The next normal-app check uses the uniquely observed enabled Quit Vixen menu item directly. Command-Q acceptance is not claimed on this host.

## Observed evidence

`target/native-panels-directory/{foreground.json,result.json,failure.ax.json,process.log}` records the private app executable hash `030460815c17fae87f9233b62b4fd1715ee0c835481ed0481473247cc77444bf` and PID 65984. Before posting Command-Q, frontmost, key and main window ownership all belonged to that PID; key/main titles identified the reopened saved sequence. The driver constructed virtual key 12 with the Command flag, posting both down and up through the same routine that had successfully invoked Open, Save As and Close earlier.

After a 15-second wait the process remained alive. The failure snapshot contains the reopened sequence and Administration windows, with no recorded sheet. The parent then terminated only its owned child; its exit status was -15 and the profile lock remained. Owner preference-file hashes were unchanged. This is not normal-shutdown acceptance.

The initial snapshot traversed windows only: it recorded neither the actual Quit menu item nor its enabled/key-equivalent state. It also lacked AXEdited and does not establish that the key event reached the app or that its termination delegate ran. The process log and profile log contain startup records, with no termination-stage instrumentation. No conclusion about the earlier Windows capture workflow follows from this unrelated Mac UI observation.

## Read-only source trace

- `VixenApp/VixenApp.swift:42` forwards `applicationShouldTerminate` to `AppHooks.shouldTerminate`.
- `VixenAdmin/AdminModel.swift:107` installs that hook at startup. `shouldTerminate` at line 312 starts an asynchronous shutdown task, returns `.terminateLater`, and eventually replies to AppKit.
- `runShutdown` first runs auxiliary document guards, then prompts on modified editors, closes editors, awaits recent-list saving/options, waits for core saving and finally shuts down the core. The modified-editor confirmation is an Administration sheet titled “Close Vixen?”. A later per-editor save prompt is also possible.
- The failure snapshot still contains the reopened sequence. Therefore a late wait in core shutdown after clean editor closure is not supported by this evidence. An earlier guard, prompt, event/menu issue, or undelivered key remains possible. Absence of a sheet in one bounded snapshot does not prove the modified state was false.
- `VixenSequencer/SequencerModel.swift:898` installs the local key monitor. For modified keys it tries `NSApp.mainMenu.performKeyEquivalent` before the Rust key table. The Rust table in `crates/vixen-session/src/sequencer/keys.rs` has no Q action. The focused-editor helper explicitly reserves Command/Control combinations for menus.
- `VixenSequencer/SequencerScene.swift:109` rearranges Sequence/Tools and hides Administration's System/Scheduler menus; it does not hide or remove the app menu. No source command replaces the standard app termination group. `AdministrationCommands.swift:120` separately invokes `NSApp.terminate` for System → Shutdown and Exit; that route has prior profile-lock acceptance but does not prove this Command-Q path.

The reopened document URL depends on loaded UI state, so the retained document observation is stronger than a title-only loading check. It still does not establish command receipt or menu validation.

## Superseded diagnostic and next check

Before the user's explanation, the proposed diagnostic was to compare Command-Q with an observed Quit-menu action, retaining menu metadata, AXEdited, prompt state and separate outcomes. A delayed first quit would have prevented conclusively attributing an exit to the fallback. That keyboard comparison is no longer needed for this environment: the immediate tap is not the user's configured gesture.

Keep the unchanged product binary and a fresh private fixture. Retain owned-window AXEdited and the actual app menu, then press only the uniquely observed enabled “Quit Vixen” menu item. Require successful AXPress status, normal process exit, removal of the profile lock and unchanged owner preferences. Record the method as an observed menu action, not Command-Q. Preserve any unexpected sheet without answering it. No app change or interceptor configuration change is necessary.

Menu snapshots and AXEdited support this direct-menu check; the parent runner also retains a one-second owned-process sample with a five-second outer bound if cleanup becomes necessary. A sample can locate a synchronous wait but cannot alone prove asynchronous command receipt. These are test diagnostics only. No new app launch or foreground action was performed during this independent review; actual rerun evidence remains separate.
