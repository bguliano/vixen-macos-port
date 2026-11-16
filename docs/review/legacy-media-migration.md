# Legacy media migration — November 11, 2026

The completion audit found that the version0 migrator discarded absolute audio paths without copying reachable files. The earlier assumption that every old path was unreachable on macOS was incorrect.

Upstream `TimedSequenceMigrator.cs:182–195` renames each `MediaSurrogate.FilePath` to its basename, copies the source into the profile's Media directory when the source exists and the destination does not, and updates a Media timing provider's source name. Nutcracker image/Glediator paths are renamed but not copied by this migration.

The XML loader now retains pending source/name pairs without writing. `SequenceSession::open` restores media in its selected profile before loading audio; this covers the editor, scheduler, export and web playback. Package import performs the same restoration before serializing current-version XML. Package destination symlink checks remain in force. Inspection, comparison and package export remain read-only.

Copies use a private sibling file and atomic exclusive rename on macOS. Existing destination files win, including concurrent publication. Failure removes only a temporary file created by this operation, preserves the source and sequence, and can be retried. Unavailable Windows paths still open as missing-media references. Saving a migrated copy does not change the original sequence.

Verification uses private synthetic sequences, audio and packages only:

- Seven session integration tests pass: read-only decode, restored PCM audio/reopen, existing-file precedence, missing Windows source, copy-failure cleanup/retry, concurrent no-clobber publication, and cancellation before any source read or restoration. `target/legacy-media-tests.log`.
- All13 package tests pass, including two new regressions covering source/embedded/existing-media precedence and failure preserving an existing destination sequence. `target/legacy-media-package-tests.log`.
- All283 I/O tests pass. `target/legacy-media-io-tests.log`.
- Independent review identified the package bypass and temporary ownership cleanup; both are addressed. No original show data or audio device was used.

The rebuilt debug application's actual editor-opening path passes11 native steps for generated PCM WAV and another11 for WMA decoded through the bundled fallback. Both show a ready nonblank waveform, preserve original sequence/audio hashes, restore byte-identical media and leave the document unmodified. Reports and screenshots are in `target/progress-ui/debug-legacy-integrated` and `target/progress-ui/debug-wma-integrated`. The original checkpoint's missing-copy failure is retained in `target/progress-ui/legacy-media-before-fix`.

The current full workspace suite passes3,497 tests. Final optimized and relocated acceptance is tracked in `docs/PROGRESS.md`. No physical keyboard, audible output or Windows-runtime claim is made.
