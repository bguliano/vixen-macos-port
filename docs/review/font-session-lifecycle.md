# Font-session callback lifecycle

`FontPanelSession.finish()` previously cleared `current` and `start` after invoking the completed edit's callback. If that callback began another font edit synchronously, the old cleanup erased the new edit's state. The fix captures the completed value and clears the old session before invoking its callback. Native panel presentation, font-manager actions and ordinary commit behavior are unchanged.

The seven-case optimized public-API probe reproduces two failures before the fix. It passes all seven cases against both an isolated candidate and the actual optimized objects from the complete production build. Cases cover ordinary finish, sequential takeover, unchanged values, recursive finish, a new edit begun by the callback, its later commit, and invisible/inactive execution. The test supplies values through the existing `testSetFont` helper; it does not claim native Font-panel input. The current production caller audit did not identify a synchronous reentry route, so the defect is established at the public API rather than as a reproduced end-user workflow.

Evidence:

- `target/font-session-reentrancy-audit-01/result.json`: baseline, two failures retained.
- `target/font-session-reentrancy-isolated-fixed-01/comparison-review.json`: identical probe against the isolated fix.
- `target/font-session-reentrancy-integrated-fixed-01/{result,integration-review}.json`: seven passes against actual optimized production objects; foreground and owner preferences unchanged.
- `target/native-font-session-source-delta.json`: only `FontPicker.swift` changed among 808 build inputs; the other 807 match the prior accepted color-well build.
- `target/native-font-session-build-report.json`: complete debug/release builds and resource/signature audits.
- `target/native-font-session-independent-review.json`: 34 read-only checks of source/build/probe links, individual native rows, preferences and retained prior builds. Owner preference hashes match the preceding focused probe through the audit interval; this is not a dedicated relocation-only before/after snapshot.
- `target/native-font-session-promotion.json`: both accepted bundles copied and hash/signature verified, with previous c7/f386 builds and checkpoint retained under `target/accepted-bundles-before-font-session`.
- `target/native-font-session-relocation/result.json`: 15 startup/resource steps plus 11 bundled-media fallback steps, from a private clone moved to a spaced/Unicode path. The temporary copy was removed normally; its run has path/audit/log linkage, without a separately retained executable hash receipt.

Candidate executable hashes are debug `bfda298deace8b65c19269f11abbaf7640c53e8ff24f7ff7e6e6802be65ecc7c` and release `40e9b807aab7c083b0cdc456ce26bd5d5fd724caa3f9f9804db41001e51e89de`. Source SHA is `80ff36106abf9247448394ac43fb46e8a32c2f62b193142433e4774ca09cad60`.

The previous release's 517 native steps retain their original f386 binary scope. The prepared four-minute foreground batch likewise remains pinned to f386 and the older Font implementation; all of its frozen inputs still match. This Font fix's seven focused assertions are separate evidence. The current acceptance checkpoint and portable refresh status are recorded in `target/completion-checkpoint.json`.
