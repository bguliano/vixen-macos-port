# Native launch ownership — November 11, 2026

The old `.app` and Terminal branches retained `/usr/bin/open`. Killing that
launcher did not reliably stop the actual program, so the F48a/F63 launch path
was incomplete despite passing command-construction tests.

`scheduler/native_launch.rs` now uses the bundled
`Contents/Helpers/vixen-launch-helper`. Each action creates a private0700
directory, a Unix control socket and JSON arguments. The Terminal `.command`
contains only the quoted helper/context paths. It never interpolates user
arguments into shell code. The helper starts an owned process group; Stop kills
that group and the unreaped direct child, including a child that changes groups.
The explicit spec choice remains: a completed non-waited action survives
scheduler Stop; Program Output Window Close stops its owned program.

For `.app`, `NSWorkspace` requests a new instance. The helper refuses ownership
if its returned PID existed before launch or its process start time predates
the request. It retains `NSRunningApplication` and stops only that owned instance.
An older running copy remains alive. Ordinary prompts are suppressed where
the API permits; no automation script or process-name termination is used.

NSWorkspace has no launch-cancellation API. At30seconds a pending launch reports
an explicit cancellation-pending result to the caller; the helper remains as a
retired guard until the callback arrives. It then stops only a newly created
owned instance. Caller completion is bounded; an OS-blocked callback can retain
the small helper/reaper until the OS resolves it. Gatekeeper UI can still be
presented by macOS, and no Gatekeeper dialog behavior was tested. Killing the
helper at timeout would allow a late application to survive, so it is avoided.

Evidence:

- `target/native-launch-scheduler-tests.log`:40scheduler tests pass, including
  seven native-wrapper tests. They execute the actual `.command` wrapper directly
  without opening Terminal, and cover process/descendant death, changed process
  group, literal argv, failure, pre-start Stop, owner EOF and private-file cleanup.
- `target/native-launch-app-tests.log`:two explicit native tests pass. Generated
  `LSBackgroundOnly` apps have no windows and prohibited activation. Two actual
  instances verify that stopping the new one preserves the old one. A separately
  compiled test-only helper delays launch1500ms with a50ms timeout: caller
  completion precedes app creation, then the late app is stopped and cleanup ends.
  Production helpers contain no delay/test configuration.
- `scripts/test_native_launch_helper.py` validates a helper in a moved bundle
  through its private protocol. The script itself passes against a moved private
  helper-only fixture (`target/native-launch-relocation-probe/result/result.json`).
  Final full-bundle relocation is a separate integrated checkpoint.

`scripts/build_app.sh` compiles/signs the helper; `check_app_bundle.py` requires
its executable and audits all Mach-O dependencies. A relocated `.app` resolves
only `Contents/Helpers`; a missing bundle helper is an error, never a fallback
to a developer path. Standalone Cargo tools use their matching build output.
Physical Terminal windows and user applications were not launched by these tests.
