# Deleted effect render ownership — November 11, 2026

Deleting an effect while its changed parameters were still rendering could make
Undo restore stale output as complete. The snapshot retained cached intents but
lost their generation. Restoration assigned generation 1 and treated any cached
intents as current. This also allowed a delayed generation-1 result from before
the deletion to overwrite a restored effect.

Snapshots now retain both the current generation and the generation covered by
their cached intents. Restoration advances beyond the removed generation, so
every pre-delete result is rejected. A completed cache remains reusable under
the new generation; an already dirty snapshot remains dirty and queues a fresh
render of its current parameters. Production changes are confined to the
snapshot fields and removal/restoration paths.

`crates/vixen-session/src/sequence_restore_tests.rs` replaces the queue before
adding a real Set Level effect, then explicitly orders real render results. It
does not depend on worker timing or sleeps. Both tests fail with the old code
(`target/sequence-restore-before.log`) and pass with the fix
(`target/sequence-restore-after.log`):

- Complete A, edit to B, delete before applying B, then Undo: cached A remains
  available but is not complete; the retired B result is rejected and the fresh
  B render produces the new intensity.
- Hold A's generation-1 result, complete B, then delete and Undo: B's valid cache
  is reused without rendering; late A is rejected. Repeated Redo/Undo advances
  ownership again and preserves the valid cache.

All eight existing sequence integration tests also pass
(`target/sequence-restore-session.log`). A separate Video regression exercises
the same pending Delete/Undo flow with two actual media files and verifies fresh
B metadata and intents; all 11 combined Video cache/metadata tests pass
(`target/video-metadata-session-regression.log`). This checkpoint does not claim
a new native UI or full-workspace run.

```sh
source env.sh
cargo test -p vixen-session --lib sequence::restore_tests --jobs 3
cargo test -p vixen-session --test session --jobs 3
```
