# What has been checked, and how to check it again

## Current Curve-tooltip release — November 12, 2026 UTC

Current-release foreground follow-up now passes **29 Gradient-library steps / three actual OS drags**, including exact saved-state checks, and **26 system Color/Font AX actions**, including native-well ownership transfer and normal close/finish. Independent review passes69 checks over57 pinned artifacts (`target/gradient-system-foreground-v5-independent-review.json`). Color-library reorder/copy still fails its saved-state assertions and remains open. No production change was made in this follow-up. See [November12 foreground evidence](review/foreground-resume-20261112.md).

The level handle now shows its current-value tooltip on press, as required by F42, while preserving the strict greater-than-two-point travel threshold and the documented apply-every-move behavior. The identical optimized native-handler probe improves from36/37 to37/37; deferred single commit, tooltip text/cleanup and both threshold boundaries pass. Independent review passes23 checks. These offscreen handlers do not establish physical pointer or visible tooltip appearance. See [Curve handler evidence](review/curve-handle-native.md).

Additional F39/F46 verification passes71 actual optimized handler checks: Curve-point tooltip values, strict2-point threshold/arming, gradient shared-stop movement, repeated updates and deferred single commits. No further production change was needed. Original gradient handles disable their tooltip; the Mac drag feedback is recorded as existing port behavior. Physical pointer/cursor and visible-tooltip appearance remain unverified. See [inline strip evidence](review/inline-strip-native.md).

Preview F5 now passes34 additional assertions using the exact unmodified Preview module source and current optimized dependencies. Real unshown windows restore saved bounds; the native move notification and supplied end-resize callback persist the expected XML, a fresh Core/model reopens it, and invalid-position/right-corner-only recovery pass. No source change was needed. Full SwiftUI layout, visible appearance, physical dragging and Finder remain separate. See [bounds evidence](review/preview-bounds-native.md).

Standard debug `fa1c55ad12cd55f3eb3c947f090eb71a92d9c98493cf82cf0980d77f58efe2dc` and release `3484a1ad1f8f7456f908b8a6754d0f90bfaca705044ac36bf845cab3b96ec8dc` pass full build audits and26 startup/resource/media steps. Only CurveStripView.swift changed among808 production inputs; the actual optimized probe objects match the full build. The prior Font-release164 portable steps and7 Font assertions remain pinned to40e9, preserved in `target/accepted-bundles-before-curve-tooltip`. The current portable archive `target/vixen-macos26-verification-pack-native-curve-tooltip.zip` (37,031,391 bytes; SHA `972ab82ee7b69be250a4c07bfb3727275fbf72dd78268984dbb324ead5987677`) passes482 file hashes, five executable modes/signatures and the full bundle signature after fresh extraction. No current extracted164-step smoke run is claimed. The macOS26 gate refuses this host before app launch. Transfer evidence: `target/second-mac-pack-native-curve-tooltip-transfer-verification.json`.

The bundled fixed-point MP3 decoder was also compared against all13,111,008 captured Windows samples. It differs at6,523,911 samples (maximum two PCM16 steps), ruling it out as an exact replacement for this input. Production decoding stays unchanged. The foregroundv4 diagnostic failures and subsequent v5 Color failure remain retained; v5 Gradient and current-source system ownership checks now pass within their recorded scopes. Windows raster return, actual macOS26 and remaining external checks are still pending. The completion goal is active.

## Previous Font-session release — 40e9

The Font-session cleanup fix is promoted to debug `bfda298deace8b65c19269f11abbaf7640c53e8ff24f7ff7e6e6802be65ecc7c` and release `40e9b807aab7c083b0cdc456ce26bd5d5fd724caa3f9f9804db41001e51e89de`. It prevents completion callbacks from losing the state of a new font edit. Seven focused checks pass against the actual optimized production objects; complete bundle audits and 26 startup/media steps pass. Independent review passes34 checks. Only one of808 source inputs changed. The current production caller audit did not identify a synchronous reentry route; this is a demonstrated public-API defect. See [Font lifecycle evidence](review/font-session-lifecycle.md).

The refreshed portable archive `target/vixen-macos26-verification-pack-native-font-session.zip` (37,030,056 bytes; SHA `c7e51e093d1364453430966bec2aadb1a416765014c8b6462d498d647ae4369d`) passes all164 host steps, 482 transferred file hashes and five executable modes/signatures. Owner preferences and source files are unchanged; owned processes exited. The standalone26 startup/media steps repeat workflows included in164 and are not added. Actual macOS 26 remains unverified; its gate refuses this host before app launch. The previous c7/f386 bundles and their517-step checkpoint are preserved in `target/accepted-bundles-before-font-session`; those earlier results are not attributed to this new executable. The frozen Color/Gradient/system-panel foreground batchv4 is ready, unlaunched and waiting for the existing four-minute desktop-readiness response. It retains its reviewed f386/earlier Font scope. Windows raster results, actual macOS26 and the other external checks below remain pending; the completion goal is active.

## Previous native color-well release — f386

The shared color panel now uses AppKit's exclusive native-well ownership so switching between native/SwiftUI wells and Vixen editors commits the previous edit and prevents cross-document color changes. Prop double-click editing uses that session; unchanged final colors no longer reload the row and deactivate an incoming well. Standard debug `c7f0a8bf03b8a9f798c2e21b59b36efbb573346af89224bf7ed43641ca9f4cdd` and release `f38663efd0dbcd7bdae7bbb9a5eaf5e7666d6fbae022a7f04213dc6690e2e900` are promoted after complete build/resource/signature audits and **353 optimized native steps** (279 sequencer, 74 Vixen2 mapping/conversion). Their private clone has identical code/data; owner preferences and source artifacts are unchanged. Build/promotion records: `target/native-color-well-{build-report,promotion}.json`.

Focused final-source verification passes **83 assertions including64 sheet steps**:24 Prop ownership/lifecycle,18 Vixen2,13 shared callbacks,12 sheet assertions and16 palette checks. Actual forwarded AppKit ordering records90 observations with transparent activation and13 hidden active-owner returns. Separate production SwiftUI picker coverage passes13 checks on its precisely recorded earlier candidate; it is not included in83. Independent39/26/36-check reviews preserve all baseline and intermediate failures. Details: `target/prop-color-ownership-final/result.json`, `target/prop-color-ownership-final-scope-review.json`, [Prop color review](review/prop-color-native.md) and [SwiftUI review](review/swiftui-color-takeover.md). These scopes do not establish physical picker gestures.

The fresh portable archive `target/vixen-macos26-verification-pack-native-color-well.zip` (37,026,890 bytes; SHA `c3508867cf7db612673bbf9cf5777f9ae2622b2907721072311c756e9dada85d`) passes164 transferred host steps on the first run, bringing this release to517 scoped steps. All482 file hashes, five executable modes/signatures and owner preferences pass; the independent audit passes43 checks. Acceptance: `target/second-mac-pack-native-color-well-acceptance.json`. Actual macOS26 remains unverified; its gate correctly refuses this host before cloning or launching. Previous f68/f75 bundles and their517-step checkpoint remain intact at `target/accepted-bundles-before-color-well`. Foreground picker verification is being prepared, with no input authorized beyond the elapsed unattended window. The goal remains active; Windows raster/GUI, actual macOS26, production audio residuals, remaining native gestures, hardware and distribution checks remain open.

## Previous Vixen2 color release — f75

The standard bundles now include the Vixen2 import-color ownership fix: debug `f68d80b5a23080c120d4e89d55c666b1436d3ae59eb0357ab596b5aebded226a` and release `f75a472246f4d3fe9aabe417862e08f03a6ea27aaf93063a831826579143ac4e`. Both pass resource, architecture and signature audits. The release's isolated, code-identical clone passes **353 native harness steps**:279 sequencer regressions and74 Vixen2 mapping/conversion steps. Map save/reopen, Cancel preservation and all seven converted effects pass; source artifacts, foreground app and owner preferences are unchanged. The separate callback fixture passes18 assertions and reproduces eight failures in the previous implementation.

Evidence: `target/vixen2-color-{integrated-build-report,promotion}.json`, `target/vixen2-color-release-regression/result.json` and `target/vixen2-release-ui-regression-v2/result.json`. Previous accepted ec067/b97 bundles and their472-step checkpoint remain intact at `target/accepted-bundles-before-vixen2-color`; those physical library-drag checks are not relabeled as f75 runs. The refreshed portable archive `target/vixen-macos26-verification-pack-vixen2-color.zip` (37,019,358 bytes; SHA `b013ab6a069d1c44eca7754c595c25cad28bf7ea32b1f00510d3c7deb8d65fd6`) passes164 extracted host steps, bringing this release to517 scoped steps. All482 file hashes, five executable permissions/signatures and owner preferences pass; actual macOS26 remains unverified. Acceptance: `target/second-mac-pack-vixen2-color-acceptance.json`. This suite does not cover the subsequently reproduced native color-well ownership bug; its correction is tracked separately. The goal remains active; Windows raster/GUI, actual macOS26, remaining native gesture categories, production audio residuals and hardware/distribution verification are still open.

## Previous accepted native evidence — release b97

Final native integration passes **472 steps** on release `b97c15b0…`: 279 sequencer key/ownership/modifier checks, 29 actual WindowServer library-drag steps and 164 transferred-pack host steps. Both audited bundles are now promoted: debug `ec06799c6b17238ecc6c34c1207cf15a8b32b201cec66a75c7dc66e32a33f95a` at `build/Vixen.app`, release `b97c15b0a18a1555ab7926d6930c94a18dd69f8727e7306213866a3e184e5258` at `target/build-release/Vixen.app`. Their 222 resources and four native executables pass audit. The verification clone `f337e14d…` has identical release code/data with an isolated Cocoa identity. Build and promotion records: `target/native-integrated-{build-report,promotion}.json`. Prior standard debug001c/releasefd5 bundles remain intact under `target/accepted-bundles-before-native-integration`.

The refreshed portable pack passes all 164 host steps,482 transferred file hashes, five executable modes and signatures with an isolated Cocoa clone and unchanged owner preferences. Its macOS26 gate correctly refuses this macOS27 host. Archive `target/vixen-macos26-verification-pack-native-integrated-v2.zip`, SHA `93eae4f2d3ca216f5283e7ba550f97eb5bc9ab67d27bcf758b44e01a3989be2d`; acceptance `target/second-mac-pack-native-integrated-v2-acceptance.json`. The first attempt failed because OCR read bmp with a Cyrillic p; a verification-only native popup locator correction passes91 focused steps and the full transferred rerun. App code is unchanged by that correction. The independent scopes below retain their original binaries and are not added to the472-step release count. Earlier b3/3f and 6894 build candidates are superseded by the integrated release.

| Scope | Verified evidence and limits |
|---|---|
| Integrated release | `target/native-integrated-release-regression/result.json` passes **279 local AppKit steps**. `target/library-panel-to-toolbar-integrated-release/result.json` passes **29 steps / three OS drags** on clone `f337e14d…`: multi-selection reorder, move to empty space, and actual panel→toolbar naming/copy with exact persisted values. This is release-b97 acceptance; the 26 exact-boundary steps below retain their earlier6f scope. |
| Library WindowServer drags | Private `6f2f5da5…` passes **29 normal + 26 edge steps**: complete-selection reorder, empty-space move, toolbar copy/name/persistence, whole-list exit/re-entry, and actual +5/+10 rejection versus +11 acceptance. Reports: `target/library-drag-shared-visibility-{foreground,edges}-v6/result.json`. This binary predates the later palette-payload and reverse-routing fixes. |
| Hidden native destinations and drop routing | Native view-tree checks verify **60 library states and 9 Effect Editor states**. The latest compiled-handler/actual-representable fixture passes **78 checks**, retaining the prior 68 threshold/re-entry/routing cases and adding cross-model copy/linked rejection versus same-model floating-toolbar reorder. Its five reproduced cross-model failures now pass; this supplied-metadata fixture does not establish physical cross-window dragging. See `target/native-library-owner-review.json`, `target/native-library-drop-routing-review.json` and [library review](review/library-drag-native.md). |
| Colour/gradient/palette ownership | **25 assertions**, including **64 sheet-harness steps**, cover creation, pending edits, reverted previews and collection ownership; **16 palette assertions** cover mutations, persistence and captured drag payloads. Native controls and the actual compiled collection Binding are distinguished in [colour review](review/native-verification-audit.md). These are separate from physical picker/menu gestures. |
| Actual system Color/Font panels | `target/system-panels-native-final-v2/result.json` passes in **5.13 seconds**: native RGB edit and Arial size edit each commit once on normal panel close before explicit Finish, without watchdog termination. Initial font selection is ArialMT 12 despite a blank AX Size value; visual initial-field rendering is not inferred. Prior app restored. See [system panels](review/system-panels-native.md). |
| Prop tree and Order | Private `4d609535…`, cloned from the earlier standard debug `001c6d37…`, passes **175 principal + 118 edge steps** with actual OS drags. Independent reviews pass **83 + 74 checks**. Selection, insertion, reorder, Command-reverse and kind-compatible cycle rejection persist correctly in synthetic packages. No Windows-authored package or normal-Quit claim. See [prop review](review/prop-native-drag.md). |
| Normal sequence file panels | Private `03046081…` passes **16 acceptance conditions**, independently rechecked in **21 checks**: actual Open/Save As Cancel, Unicode paths, close/reopen, Quit-menu exit and lock cleanup. Eleven selected persisted XML sections and original/owner hashes are checked. The retained failed postprocessing result is not a failed app run; the owner's hold-to-quit setting remains unchanged and Command-Q is not claimed. See [file panels](review/native-file-panels.md). |
| Layer dragging | The existing private release-fd5 clone passes **59 steps and five OS drags**, including normal save/reopen and exact layer UUID/filter/settings preservation. See [layer review](review/layer-drag-native.md). |

Remaining work includes the still-uncovered native routes and panel categories, and the unchanged external gates: Windows GUI/file interoperability and original GDI rasters; the production decoder's six residual values; actual macOS 26; clean-user/distribution; physical audio, controllers and remote devices. The goal remains active with zero consecutive blocked turns. Historical failed attempts and their exact binary scopes remain retained.

## Latest continuation: verified Windows v3.2 capture and isolated native tests

The returned **v3.2 Windows capture succeeded**. `target/windows-results/attempt-05-returned.zip` is 34,100,215 bytes / 63 entries, SHA-256 `b963da14ab946d15c2a6e32034ee617e34ad433c8988e4027f672786859b4ea3`; extracted evidence is in `target/windows-results/attempt-05`. All eleven reported pre-archive stages completed without errors, timeouts or output-limit failures, including actual Windows Framework compilation, .NET 8.0.6 X64 preflight and the original Vixen 3.12.4 / AudioPlayer 3.12.1371.4 / NAudio 2.2.1 capture. The source/config/contract match the immutable v3.2 pack; all nine original dependency hashes match. The valid ZIP proves artifact publication, while its deliberate pre-archive checkpoint does not establish final supervisor exit. Independent review: `target/windows-results/attempt-05-independent-review.json`; details: [capture audit](review/windows-v3-2-capture.md).

The original reader negotiated **signed PCM16, stereo 44,100 Hz**. Its standalone and cache decodes agree exactly: 6,555,504 frames / 52,444,032 float bytes, SHA-256 `e5fd049950f041a9cd88d267d91a097a65ffb2b184a3a05f1573a6327e14a5a5`. Using this unchanged buffer, the full Coldest Winter FSEQ comparison has **zero nonrandom differing values** at the original 10 ms render / 25 ms export intervals (`target/windows-pcm/comparison-v3-2/{report.json,comparison.log}`). The unchanged C# source oracle and Rust also produce **273,949 identical mono samples and 104 identical levels/sample indices**; all 14 private-profile files are unchanged. Hash-pinned result: `target/windows-pcm/vertical-meter-capture-v3-2/result.json`; [meter review](review/windows-pcm-meter-oracle.md).

Production Mac decoding is unchanged: its existing float input still produces the six recorded nonrandom values. Truncation to PCM16 matches 99.7781% of Windows samples but is not exact, so no blanket approximate quantization is applied. Random effects, Windows app/file interoperability, original GDI raster comparisons, actual macOS 26 and physical-device/distribution checks remain outside these passes. The goal remains **active**, with consecutive blocked turns **0**.

Historical v3.1 evidence remains retained in `target/windows-results/attempt-04-returned.zip`, `attempt-04-review.json` and `attempt-04-independent-review.json`: six successful diagnostic stages and nine copied DLLs, with PCM correctly skipped by the strict .NET10/NAudio2.3 host. The delivered v3.2 archive remains `target/windows-capture-pack-v3-2-final/Vixen-Windows-Capture.zip`, SHA `e697ccbc0c39b77c37eb56d5944d3804a9c186885fb8729977c0e52fa340c0a2`; its 462 supervisor checks / 22 workflows, 24 host cases + 6 helpers, 15 provenance cases, 11 worker source invariants + 6 filesystem cases and 103 independent archive checks remain preparation evidence. The returned run now supplies the previously pending Windows execution evidence. No runtime installation or original-profile write was needed.

The last full-workspace run passes **3,523 tests, zero failures, nine ignored** (`target/workspace-settings-isolation-result.json`). Five settings cases cover the explicit `VIXEN_SETTINGS_DIR` override, unchanged UI-test precedence and preservation of an owner-settings sentinel. Promoted debug `ec06799c…` and release `b97c15b0…` include the settings and combined Swift/native fixes. The new release passes 308 integrated native steps; the prior release-fd5 443-step release/portable evidence remains historical and separately pinned. Unique Cocoa preference domains, private settings and muted controller-free profiles preserve original inputs and owner preferences. Earlier unrouted library attempts remain diagnostic history; the later 29/26-step foreground acceptance supersedes them for its recorded scope.

Preview Save is already fixed and verified (84 focused actions on candidate9c, then the prior release-fd5 443 native regression/portable checks); it is not an open defect. Full verification remains active with the production audio residual, Windows raster/interop, macOS26, remaining native interaction scopes, clean-user/distribution and real audio/output-device gates still open. Earlier dated sections below retain their historical evidence.


Unless explicitly identified as returned Windows evidence, checks below ran on this Mac. "The C#" means original Vixen code on the local.NET10 runtime: earlier compiled assemblies plus effects compiled directly from unchanged `upstream/Vixen/src`. The returned v3.2 run supplies original Windows PCM and exact-input parity evidence as recorded above. Original Windows app interoperability, raster parity and the six-value production Mac decoder residual remain open.

Current unresolved scopes and reproducible external checks: `docs/REMAINING_VERIFICATION.md`. Dated checkpoints below preserve the evidence available at that time; later sections supersede earlier missing-work notes.

## November 11 local integration checkpoint

Historical pre-settings source passed **3,518 workspace tests, zero failures, nine ignored** (`target/progress-workspace-render-probe.log`), including three read-only render-observer regressions. That checkpoint’s debug/release bundles (`d08e77d0…` / `0d7edc26…`) pass both audits (`target/{debug,release}-bundle-image-map-final.json`). The final Swift-only rename correction does not change the Rust test source. The earlier 3,515-test integration and native checkpoint below remains associated with release `4c98baee…`. Seven meaningful ignored checks have separate passing evidence: real audio-device enumeration, four release performance checks and two generated background-app ownership checks. Two others are fixture generators.

At that earlier checkpoint, both complete debug and optimized bundles build and pass static audits of222 source resources, the app and three signed helpers. All native executables are arm64, declare macOS26.0 and use only system libraries. The exact optimized app passes **569 native harness steps**:45 modifiers,187 key/workflow,47 editor ownership,15 relocated startup/resource checks,11 relocated legacy WMA/waveform checks,91 Video metadata checks,139 MOV cache checks and34 AVI checks. Relocated helper start/Stop/descendant cleanup, fatal cleanup and three-process profile lock/normal Shutdown/reopen also pass. Hashes and report paths are in `target/completion-checkpoint.json`; builds are `target/progress-{debug,release}-acceptance.log`, and static audits are `target/{debug,release}-bundle-acceptance.json`.

The integrated fixes include HTTPS/redirects, owned launches, legacy media restoration, bundled fallback codecs, persistent Video cache/cleanup, saved audio-device startup and media-load cancellation. The later authoring audit fixed Video duration/size/quality refresh and multi-selection, Pattern loaded aliases, Fixture new-function construction/current capability snapshots, and generic Delete/Undo render-generation ownership. Actual screenshots show3-second Video metadata for a2-second effect, quality resets to50, unchanged selection, correct cache sizes and successful reopen. Native tests remain muted and preserve original files. Evidence: [Video metadata](review/video-metadata.md), [Video cache](review/video-cache.md), [computed properties](review/computed-effect-properties.md), [Fixture authoring](review/fixture-function-authoring.md), [render Undo ownership](review/deleted-effect-render-ownership.md).

No unresolved finding remains within those completed audits. Full verification still requires the foreground, physical, Windows and minimum-OS scopes in `docs/REMAINING_VERIFICATION.md`; this checkpoint is not a physical focus, hardware, Windows-runtime or macOS26-runtime claim.

### Follow-up verification-tool checks

Effect Defaults binary XML now passes a finite two-way comparison with unchanged
original C# contracts and verbatim binary reader/writer methods. Pulse, Pattern
and Wave nested settings, outer identity/order, Unicode names, fractional UTC
ticks and a 66,001-byte opaque unknown entry survive C# write → Rust import/export
and parameter resave → C# read. Original C# typed payload bytes remain identical;
32 retained artifact hashes pass. The older private defaults file is read without
changing its hash. Evidence: `target/golden/defaults/verified-final/result.json`;
reproduction and exact limits: `tools/golden/defaults/README.md`. This is managed
serialization on this Mac, not Windows application interoperability. A separate
bounded source audit confirms all ten `[ExcludeFromEffectDefault]` members are
represented, including nested Wave/Liquid rows, with empty-GUID resets and library
links preserved. No production changes were needed.

Selected-only lyric conversion passes **43 native steps** on the unchanged release
(`target/lyric-selection-ui-final/result.json`). An actual NSTextView UTF-16 range
selects `hello` after a non-BMP prefix while unknown surrounding words are excluded.
The empty-selection case converts both words; selected `world` with Clear after
conversion empties the field on successful conversion. Actual controls drive conversion;
the private pasteboard has exact expected phonemes, lyrics and 250 ms durations.
The original fixtures and copied sequence file remain unchanged. Selection setup
uses the existing native text-view hook, so physical text dragging is not claimed.
A preliminary OCR click matched the window title instead of the footer button;
that failed setup is retained and excluded from acceptance. The final runner
uses the actual footer-button coordinates confirmed in its screenshot.

The unchanged release additionally passes **77 native Fixture steps**: actual
Functions picker/Add, Star-to-Circle index-dependent Range visibility, RGB Add,
Edit Undo/Redo and Save/reopen. Screenshots confirm the header/field transitions;
independent read-only file inspection verifies the exact Gobo command and full
white RGB output on both fixture leaves. Reopen preserves the file hash and
shows a clean document with no Undo history. The successful 57-step authoring
run was retained after an OCR postprocessing timeout; guarded resume validated
the same script/app and ran the remaining 20 reopen steps. Failed preliminary
menu-label attempts are excluded. Evidence:
`target/fixture-authoring-ui-retry/native/result.json` and
`docs/review/fixture-function-authoring.md`. No application code changed.

The original GDI availability probe now returns exit1 if either Bitmap/Graphics
or Font initialization fails, while retaining both diagnostics. Its host builds
with zero warnings/errors; the actual macOS run reports both missing-GDI failures
and exit1 (`target/gdi-probe-build.log`, `target/gdi-probe-exit.json`). This corrects
the diagnostic's earlier false-success exit, without changing app rendering or
establishing Windows raster parity.

The Export Controllers Command-A retry used baseline release `4c98baee…`'s CGEvent-backed
local key hook. Selection still remained1of2, and Space toggled only that row
(`target/export-command-a-cg/assertions.json`). The harness's mechanical actions
completed, but the selection acceptance condition did not. This run is excluded
from passing-step totals. Foreground key/main-window ownership was not established;
the existing offscreen sequencer-menu fallback prevents inferring a production
shortcut defect from this run. Command-A remains a foreground verification item.

The standalone Windows PCM capture host builds with zero warnings/errors and
passes13 local argument/integrity checks. It loads supplied original Vixen/NAudio
assemblies, records negotiated format, and requires the original reader and
CachedSampleSource float hashes/counts to match before publishing a capture.
The comparison wrapper validates source/PCM identity and records profile hashes;
the diagnostic Rust example now rejects a missing original clip instead of
silently ignoring a requested raw-PCM override. That refusal was verified in a
real private diagnostic process. Instructions and evidence paths are in
`tools/windows-pcm/README.md`. This was preparation evidence; actual v3.2 Windows capture and exact-input parity now pass, with reports at the top of this document.

The portable second-Mac pack copies the exact audited release, generated
controller-free profiles, local media and a signed arm64 OCR helper targeting
macOS 26. Its final run from an unrelated `/private/tmp` path containing spaces
and a non-ASCII character passes **164 native steps** plus relocated helper
ownership. Every packaged file hash remains unchanged. The report records
macOS 27.2.1 and `minimumOSVerified=false`; the separate macOS 26 mode refuses
this host before any app launch. The preliminary OCR timeout is retained as a
failed preparation run; the final test-only 45-second bound accommodates cold
Vision initialization. Evidence: `target/second-mac-pack-host-smoke-final/result.json`
and `target/second-mac-pack-final-os-gate/result.json`. These 164 steps repeat
existing workflows to prove pack portability and are not added to the integration
step total. Destination procedure: `docs/SECOND_MAC_CHECKS.md`.

## Against your real show

| Check | Result | Run it |
|---|---|---|
| Rendered output against your 14 real FSEQ exports (90,586 frames, 3,367 channels) | Outside Twinkle, Dissolve and Candle Flicker, whose random numbers differ per run, 48 values differ. 42 are in "Dance of the Sugar Plum Fairy", from a Chase bug upstream has since fixed. 6 are in "Coldest Winter": one Vertical Meter pixel across two frames, only 0.000053 dB above threshold. The later actual Windows capture gives zero nonrandom differences for Coldest Winter, 273,949 identical mono values and 104 exact C#/Rust levels. Production decoding remains unchanged, retaining those six values; see [captured-PCM oracle](review/windows-pcm-meter-oracle.md) | `cargo test --release -p vixen-session --test real_show` |
| An FSEQ the port exports against a real one | Same header fields and every frame | Same test |
| FSEQ files through your own Python parser | All 14 identical | Done by hand, using a stand-in for the `zstandard` package |
| Sequences the port saved, loaded and saved by the C# | All 17 load. What the C# writes is byte-identical to what it writes for your originals, apart from the test folder's name | `tools/golden/filecheck.sh` |
| Effects the port built itself, loaded and saved by the C#: 38 of the 58 effect types (all 16 you use), each at defaults and with every value changed, plus marks of each type and a layer for each of the 9 mixing filters | About 12,300 elements. Every value comes back as written, apart from 5 that the C#'s own load logic changes (old members and out-of-range values). The port's load does the same in memory, and keeps the file's text until the effect is edited | Same script |
| Files the port saved, compared directly | `SystemConfig.xml` byte-identical. `ModuleStore.xml` the same XML, element for element: 3 empty elements lose an unused namespace declaration | Same script |

**Limits of the file check.** The compiled C# has no sequence migrator, so version 8 files were relabelled version 9 for it. That is only safe because that migration touches SetPosition and Nutcracker effects, and these files have neither; the script stops if that changes. The compiled C# also lacks some modules (output controllers, previews, colour breakdown filters, scheduler, shows, web server, lip sync maps, custom props, Vixen 2 import, export wizard), so their data was compared directly but never read by the C#. The other 20 effect types aren't in the compiled C#, so effects of those types the port builds haven't been through this check.

## Against the C#, effect by effect

| Check | Result | Run it |
|---|---|---|
| 38 effects, 17,153 cases, 140,850,974 frame values | 0 mismatches, with no mismatch exemptions; original failure cases compare their empty/partial output | `cargo test --release -p vixen-effect-registry --test golden` (regenerate with `tools/golden/build.sh run`) |
| Random effects | Statistics over 24 renders, and one render frame for frame with the C# reading the port's random stream | Same test |
| All 9 fixture effects, 1,069 cases, 408,500 state values | 0 mismatches against unchanged upstream fixture source; edited and reloaded lifecycles, exact intent spans/metadata; 34 deliberately malformed cases compare C# failure output | `cargo test -p vixen-effect-registry --test fixture_golden` (`tools/golden/fixtures/README.md`) |
| State, 1,022 cases, 167,280 state values; 146 State/Face color contracts | 0 mismatches against unchanged upstream State/mark sources and original data contracts; see scope limits below | `cargo test -p vixen-effect-registry --test state_golden` (`tools/golden/states/README.md`) |
| LipSync FaceDefinition mode, 684 cases, 193,828 state values | 0 mismatches against unchanged upstream LipSync/Face/mark sources; image-map branch excluded | `cargo test -p vixen-effect-registry --test lipsync_golden` (`tools/golden/lipsync/README.md`) |
| Glediator, 277 cases, 167,392 state values | 0 mismatches for fresh/reloaded original C# output, including three malformed-file failure contracts; dirty-rerender phase intentionally differs | `cargo test -p vixen-effect-registry --test glediator_golden` (`tools/golden/media/README.md`) |
| Picture movement/sampling, 306 cases, 542,400 pixel values | 0 mismatches on supplied opaque pixels, including location offsets; image decode/resize not covered | `cargo test -p fx-pixel-graphics original_picture_movement` (`tools/golden/picture/README.md`) |
| Shapes geometry, 5,393 cases, 390,833 coordinates | 0 mismatches for generated vertices/dimensions; no SVG/GDI raster claim | `cargo test -p fx-pixel-graphics original_shape_geometry` (`tools/golden/shapes/README.md`) |
| CountDown numeric helpers, 405 cases, 23,764 frames | 0 mismatches for text, fade, direction state and spinner angle; no raster/font parity claim | `cargo test -p fx-pixel-graphics original_countdown_timing` (`tools/golden/countdown/README.md`) |
| Snowflakes, 343 cases, 5,126,164 state values | 0 mismatches against unchanged source with a shared seeded random stream; production RNG statistics not covered | `cargo test -p vixen-effect-registry --test snowflakes_golden` (`tools/golden/snowflakes/README.md`) |
| Wave/Whirlpool nested lists (included in the 38-effect total): 510 added cases, 8,978,660 frame values | 0 mismatches for ordered child edits and reloads; all wave types/movements, individual whirl configuration, marks, curves and gradients. Wave includes 35 seeded/statistical cases | `cargo test -p vixen-effect-registry --test golden -- --exact wave whirlpool` (`tools/golden/lists/README.md`) |
| Layer mixing: all 9 filters, 17,600 combinations | 0 mismatches against the real `LayeredStateCombinator` | Same test |
| Editor metadata for every effect: labels, categories, order, ranges | 0 findings on 15,257 values, 3 accepted differences | `python3 scripts/check_effect_metadata.py` |
| Beat and bar detection against the original C++ | Exact on real music | `crates/vixen-analysis` tests and `tools/qm-golden` |
| Morph and Liquid against the C# physics | See `tools/golden-physics` | |

## Not checked

- **Remaining effect golden gaps:** LipSync image-map rendering, CountDown raster/font output, Picture decoding/scaling/GIF/transparency, Shapes raster/animation output, Text, Video.
- **GDI+ drawing:** the port follows GDI+'s rules as documented and as tested here (`gfx.rs`, `gdi_rules`), but nothing has been compared with GDI+ on Windows. Text can't match exactly, because the fonts and the text renderer differ.
- **Real hardware output.** Tests use muted output, pseudo-terminals and loopback stand-ins; no real controllers or remote devices were driven.
- **A real Windows install of Vixen** opening a file the port saved.
- **Limits of the golden "C#":** audio effects read synthetic decoded signals; original Vixen providers and the exact NAudio.Core 2.3.0 assembly now perform mono conversion. Bars uses original WPF v10.0.0 point/matrix methods. These paths do not establish Windows decoder/native-runtime parity. Twinkle, Candle and Wave have their random source swapped. See `tools/golden/helpers/README.md`.

## Read-only folders

Your show has shown no changes since November 1. In the earlier Codex folder two directory timestamps changed on November 8, when a `dotnet build` and a `git status` ran there, before builds moved to a private root under `target/golden`: `work/dotnet/metadata` and `outputs/Vixen/.git`. No file contents changed.

## The finished areas together

On November 9 the four finished areas (Administration, the dialogs, Display Setup, the timeline) were built as one app and launched on a copy of your show through the test harness. The Administration window listed your recent sequences, Display Setup showed your elements and both controllers, and the timeline opened "Coldest Winter" with its waveform, effects and marks. Every shared check passed after Display Setup's change to the loader: 1,225 session tests, the real-show comparison, vixen-io, golden (39 of 39) and the C# file round trip. The sequencer window, which joins these together, wasn't in that build.

## Administration, reviewed independently

On November 9 a second reviewer went through the Administration area without relying on the builder's table or tests. The findings are in `docs/review/administration.md`.

**Checked:**

- The app, built alone (`admin-review`), on a copy of your show through the test harness: the main window, every menu, About, Installed Modules, Instrumentation, Release Notes, Check for Updates, Options, Setup Previews, Data Profile, the Zip Wizard's first page, Stop and Start, and System > Shutdown and Exit. Each picture was read against the spec and the C#.
- A profile whose lock couldn't be read: the alert and the log lines match upstream's.
- About 95 behaviours against the C# line by line: the profile lock (stale, another machine's, unreadable), the recent list, what Options writes, the archive's contents, start and shutdown order, config failures, the update check's version comparison and its log lines.
- 35 rows the parity table marks done, spread across the table. None was wrong.
- Fifteen defects were fixed over the two review runs. Ten have a test; of those, the ones for the application data version, the update check's warnings and the lock's log line were confirmed to fail on the old code.
- vixen-io's tests (251, plus 28 across files), vixen-session's library tests (1,297) and the file check against the original's loader all pass.

**Not checked:**

- Check for Updates only ever talked to a stand-in on this Mac. GitHub itself was never contacted.
- No controller was driven, so Instrumentation's engine rows showed nothing moving.
- File panels, typing into the Options fields, the Documents permission prompt, a second copy of the app open at once, and a crash were not driven through the harness.
- Five fixes are in Swift and have no automated test: saving Options at quit, the recent list at quit, the order of the development warning, the Open Sequence log line, and the archive's `.zip` trimming.
- The real-show render comparison wasn't re-run, because nothing that renders changed.

### Administration, the review checked

Later on November 9 a third pass tried to prove each of the fifteen fixes wrong against the C#. Details are under "Verification" in `docs/review/administration.md`.

**Checked:**

- Fourteen fixes stand as made. One was corrected: a lock file that isn't valid text is now treated as locked without a line in the error log, which is what the original does. Only a lock file that can't be read at all logs the error. The line above about "a profile whose lock couldn't be read" is true for that second case.
- In the app, on a copy of your show: one `Process info` line per launch and no lock left after Shutdown and Exit; the four update warnings, in order, against a stand-in that answers 404; the lock alert for a lock file of junk bytes; an Options change made with the window still open at quit reaching `SystemConfig.xml`, with nothing else in the file changed; Installed Modules opening and closing; Data Profile cancelled without the restart message.
- Ten more rows the parity table marks done (F1, F5, F8, F9, F10, F20, F22, F34, F40, F41), read against the C#. None was wrong.
- vixen-io's tests, vixen-ffi's and vixen-session's library tests, the real-show comparison (3 tests) and the file check against the original's loader. All pass. Three scheduler tests failed on the first run while that area was being edited by its own worker, and passed when run again.

**Not checked:**

- Five fixes were read but not driven in the app: the archive's default run on a Mac with no settings folder (it has a Rust test), the archive's `.zip` trimming, the development-build warning (this was a release build), the Open Sequence log line, and the recent list's save at quit (this build has no sequencer window).
- A `.lock` that is a folder rather than a file.

## Timeline, reviewed independently

On November 9 a second reviewer went through the sequencer timeline without relying on the builder's table or tests. The findings are in `docs/review/timeline.md`.

**Checked:**

- The timeline, built alone as a release build (`timeline-review`), on a copy of your show through the test harness, with "Coldest Winter": selecting, rubber band, moving and resizing effects, row labels and heights, the ruler range, the marks bar (select, drag, resize, paste, delete), the waveform and its guides, zoom and undo. Every picture was read against the spec and the C#.
- 59 behaviours against the C# line by line: the grid's pointer handling, snapping, align, split, zoom and wheel routing, rows and their saved state, the marks bar, and the ruler.
- 36 rows the parity table marks done, spread across the table. None was wrong.
- Five defects were fixed: a pause took down the playback line, tag changes didn't reach the labels until reopening, a pasted mark could be shorter than 25 ms, the pointer shape during a row height drag, and a right-click released over another row label. Four have a test.
- Frame timing on "Christmas Eve Sarajevo" (1,095 effects, 1,136 rows), with the display awake at 144 Hz, so a frame is 6.9 ms. Work per frame, 95th percentile: scrolling 0.3 ms sideways and 1.1 ms up and down, zooming 1.5 ms by wheel and 1.7 ms by pinch, dragging every effect at once 1.3 ms (worst single frame 10.5 ms). One or two late frames in the whole run.
- The timeline's 286 unit tests pass.

**Not checked:**

- Each timing case ran once.
- The ruler's table of label spacings was not re-derived from the C#; it has unit tests and looked right in the pictures.
- The pause fix is in Swift and has no automated test.
- What a drop on the grid does, the menus and the keyboard shortcuts belong to the sequencer window, which wasn't in this build.
- The real-show render comparison and the file check weren't re-run in the second pass, because it changed no code.

### Timeline, the review checked

Later on November 9 a third pass tried to prove each of the five fixes wrong against the C#. Details are under "Verification" in `docs/review/timeline.md`.

**Checked:**

- Three fixes stand as made (tags reaching the labels, the pasted mark's 25 ms floor, the right-click released over another label). Two were corrected. Pause: a pause that arrives before the timeline has noticed playback started now still shows the line, as the original's timer does. Row height drag: the code was right but its comment said the pointer is tested the way the original does it; it is tested after the row changes height, so the pointer doesn't flicker on a fast drag. That is now in `docs/DIFFERENCES.md`.
- With the four fixes that have tests undone in a scratch copy, each of the four tests fails, so the tests do test the fixes.
- In the app, on a copy of your show with "Coldest Winter": the line holding still through a pause (two pictures 1.5 seconds apart are identical), moving again on resume, and going away on stop; a row height drag growing the row and stopping at 10; a right press on one label released on another leaving the selection alone.
- Ten more rows the parity table marks done (F8, F14, F29, F34, F40, F44, F90, F105, F113, F118), read against the C#. None was wrong.
- The timeline's 286 unit tests and the real-show comparison (3 tests) pass. The reviewer's own harness scripts give the same output on the corrected build.
- Frame timing three more times on "Christmas Eve Sarajevo", release build, display awake at 144 Hz, while other work kept the machine busy. Work per frame, 95th percentile, worst of the three runs: scrolling 0.4 ms sideways and 0.6 ms up and down, zooming 2.2 ms by wheel and 2.5 ms by pinch, dragging every effect at once 1.5 ms. The worst single frame of work in any of those was 5.6 ms. The 10.5 ms frame from the earlier run did not come back. One late frame in the three runs, during playback.

**Not checked:**

- The pointer's shape and any menu opening can't be seen through the harness, so those two fixes rest on their unit tests and on reading the code.
- The pause correction was not caught taking its new path in a run; the harness can't control whether the pause lands inside the 0.1 second window.
- The ruler range coming back when playback ends was not driven in the app, because the test window's Play doesn't play a range. It has a unit test.
- The tags fix was not driven through the tag manager's window.
- The file check against the original's loader wasn't run, because nothing that reads or writes a file changed.

## Slips during testing

Tests are meant to touch no outside network. On November 9 one dialog test sent a single host-name lookup for a made-up name, `myhost.locallights.local`, before a guard went in. Muted apps now refuse DDP name lookups, Elexol Test and RDS Send in Rust.


## Continuation checks, November 11, 2026

- The full app builds with all areas: `VIXEN_BUILD_ID=main scripts/build_app.sh debug`, bundle `build/Vixen.app`, final log `target/resume-build-final.log`. Existing compiler warnings remain, including Swift concurrency warnings and native objects built with a newer deployment target. A successful local build does not establish compatibility with every supported macOS release.
- Ran `cargo test --workspace --offline --no-fail-fast` (`target/resume-tests.log`). The restricted run passed 3,169 tests, failed 47 tests requiring macOS video decoding or loopback network access, and skipped 7 ignored tests. Its FFI documentation check also overlapped an in-progress mapper edit.
- Re-ran all tests for the affected packages (`fx-pixel-graphics`, `vixen-fpp`, `vixen-output`, `vixen-session`, `vixen-video`) with native access: 1,769 passed, 0 failed, 5 ignored (`target/resume-tests-native.log`). This includes all three real-show comparisons and 1,378 session library tests. The video decoder probe passed with native access; no software fallback was introduced. These counts overlap the initial run and must not be added together.
- Re-ran `cargo test --offline -p vixen-ffi` after interface changes: compile and doc checks pass (`target/resume-ffi-tests.log`; this crate currently defines no runnable unit tests). Five mapper tests pass, including a peer-reviewed fix for clearing a never-mapped source-tree row; eight face-node selection tests pass.
- The combined app ran muted on `target/show-codex-resume`, copied from `target/show-copy`. The original show was never opened for writing. All 17 initial UI steps passed: opening the sequencer’s LipSync selector and the correct Export Sequence sheet, then cancelling/closing; screenshots inspected (`target/resume-ui/smoke`).
- All 24 LipSync workflow steps passed (`target/resume-ui/face-workflow`): recursive selection produced 555 chosen rows for Light Strip L in this copy; Reset cleared them; two selected strip groups reached Face Setup; Cancel returned to the sequencer. Face Setup’s screenshot was inspected. `SystemData/SystemConfig.xml` and `SystemData/ModuleStore.xml` remain byte-identical to the source test copy.
- Dialog review regressions passed 180 Rust tests and 48 UI steps, including group/child assignment selection and deleting a State item without stale assignments. See `docs/review/dialogs.md` for evidence and limits.

The tests use temporary files, pseudo-terminals and loopback stand-ins. No real controllers, remote FPP devices or paid APIs were used. The original Windows app/file-loader check was not repeated in this pass; the new mapper’s JSON format was checked against the C# source and synthetic roundtrips.

Element Mapper integration validation (`target/resume-ui/`):

- `mapping.json`: all 19 steps passed. A two-effect synthetic `.tim` contains one unknown target and one valid target. Mapping the unknown target to White Bottom keeps both effect instance IDs, rewrites only the intended target ID/name, and marks the sequence modified. Saving clears that flag. `mapping-fixture/check_saved.py mapped` confirms exact saved IDs/names and no ghost references.
- `mapping-reopen.json`: all 8 steps passed. The saved sequence opens without the unmapped prompt, with two effects and modified=false.
- `mapping-cancel.json`: all 12 steps passed. Cancel removes only the unknown effect and leaves modified=true, matching the original behavior with the port’s modified-state fix. Saving and `check_saved.py cancel` confirm the untouched effect survives exactly.
- `mapping-leave.json`: all 8 steps passed. Leave Unmapped drops the unknown effect during load and leaves modified=false for a sequence inside the profile, as upstream does.
- The initial mapper UI run failed because the hosting controller reduced the window to its fitting size. After correcting the SwiftUI minimum and restoring the content size after attachment, the complete 800×600 sheet and both timeline screenshots were inspected. Mapping was assigned through the harness hook; the actual Map Elements, OK, Cancel, Leave Unmapped and Save controls were exercised. Native map file panels, drag gestures and large-tree timings remain unverified.

Across the final combined-app runs (88 steps) and the dialog reviewer’s runs (48 steps), 136 UI harness steps passed. The initial mapper layout failure was corrected and rerun; it is not counted as a successful run.

## Sequence packages, November 11 continuation

The package export and import windows now run from Administration > Tools. Rust state and file operations are in `crates/vixen-session/src/sequence_package/` and `sequence_package.rs`; Swift is `VixenScheduler/PackageWizardWindow.swift`.

- Twelve package tests pass (`cargo test -p vixen-session sequence_package --lib`): raw sequence bytes and ZIP entry names, wildcard/media rules, saved settings, exact-name import, first-target mapping, missing targets, package selection changes, existing media, traversal and symlink refusal, cancellation and preserving existing files after failure.
- The dedicated scheduler/administration app builds. `target/package-ui/roundtrip.json` ran 54 steps successfully, `edges.json` ran 47, and `cancel.json` ran 14. Every saved stage screenshot was inspected. These used `target/package-ui/profile`, copied SystemData with independent Sequence/Media/Module Data Files folders; no real show folder or hardware was written.
- The round trip exported an untouched real source sequence into the Windows-style package, created and saved its map through the package's mapper, accepted the actual overwrite prompt, and imported it. `target/package-ui/check.py` independently read ZIP/XML/JSON: source bytes match, source tree exists, all 12 effects retain their expected targets, and map/export preferences are saved.
- The edge run exercised the actual import checkbox, Back, replacement with a two-sequence package, Edit Map/Cancel, invalid output-folder gating, and declining the actual package overwrite prompt. SHA-256 confirmed package and map files stayed unchanged where canceled.
- A generated 64 MiB effect-media file kept an export busy while cancellation was requested. The screenshot shows the packaging step running; afterwards the previous package bytes remained intact and no partial file remained.

Still outside this check: navigating the native file panels and a genuine Windows installation opening the resulting package. Repeated media references are deduplicated in the archive, with identical extraction results; deliberate differences are recorded in `docs/DIFFERENCES.md`.

## Audio and scheduled contexts, November 11 continuation

- Audio: 27 tests pass, one hardware-device test remains ignored (`target/progress-context-tests.log`). Added actual output-frequency/media-clock checks for rate and tempo at10%,50%,100%,210%,400%, stereo, 44.1/48kHz conversion, mode changes, end/seek, and callback allocations. Mode selection applies on the next speed assignment, matching upstream.
- Engine: 28 tests pass (`target/progress-engine-tests.log`). New coverage includes exact-end/zero-after-progress completion, nonzero loop restart, independent context stop/drop/shutdown, cross-context raw-layer masking, equal-level mixing, per-context clocks, different element orders and retired-schedule reclamation. The final zero-after-progress change passed Rust tests after the current app snapshot; the next integrated build must include it.
- Scheduler: 34 tests pass (`target/progress-scheduler-tests.log`), including real synthetic sequence actions, eight simultaneous cache consumers sharing one render, overlapping run leases, edited-file reload while the old version remains usable, failed loads and Stop before playback.
- Session library snapshot: 1,397 passed,2 loopback failures,5 ignored under sandbox (`target/progress-session-tests.log`); both loopback tests passed with native access (`target/progress-session-native-tests.log`). Counts predate later in-progress editor/import additions. FFI check passed (`target/progress-ffi-check.log`). Full all-area app built (`target/progress-main-build-final.log`), with existing compiler/deployment warnings.
- Native scheduler: all49 steps passed (`target/progress-ui/scheduler-verified`), with explicit log assertions for Sequential and Background completion, Shutdown completion, Show complete and Stop Now. Screenshots inspected. Profile is an isolated copy under `target/progress-ui/show`; no real hardware or network services run.
- Native audio: all30 steps passed (`target/progress-ui/audio-asserted`), with six independent assertions confirming400%, tempo=true/false, play, natural end, loop continuation and stop. Generated8-second WAV; actual Audio menu toggles; screenshot confirms the red400 gauge. Speed Max invokes the same UI command through `sequencer.transport` because the offscreen toolbar omits AX children. Two pointer probes did not change the gauge; do not count the physical double-click as verified. Backend signal tests cover audio output; native UI playback was muted.

The first scheduler script omitted Edit after Add, and the first audio fixture omitted its timing-provider association. Both test fixtures were corrected. Only the final asserted runs above count as verification. This stage does not establish real audio-device/controller operation, Windows-loader compatibility, or support on other macOS versions.

## Sequencer tag manager, November 11 continuation

The all-area build in `target/progress-tags-build.log` includes the latest engine zero-after-progress fix, the tag-manager registration, and current importer/editor additions. Nine tag tests pass (`target/progress-tag-tests.log`), including disk persistence without stopping a context, preservation of user tags, and rollback on failed save.

The native sequence ran muted with looping playback while opening Manage Tags through the sequencer's presenter. All 30 UI steps passed (`target/progress-ui/tags-out`): stage a color and Cancel, reopen to confirm discard, stage and Save, then reopen to confirm `#123456`. State assertions confirm playback continues and the sequence remains unmodified. Parsed SystemConfig before/after is identical after excluding tag definitions. The saved sheet screenshot was inspected. Color staging used a test hook; actual Save and Cancel buttons were clicked. The original show was untouched.

## Vixen 2 importer, November 11 continuation

- `CARGO_TARGET_DIR=target/vixen-session cargo test -p vixen-session vixen2 --lib --offline`: 11 passed. The original upstream C# state machine is linked unchanged by `scripts/vixen2_oracle`; its 150 deterministic input/output cases are stored in `crates/vixen-session/tests/fixtures/vixen2-effects.json` and compared in Rust.
- Dedicated scheduler app builds. `target/vixen2-ui/roundtrip.json` completed 48 native UI steps and `edges.json` completed 26, with no failures. These use `target/package-ui/profile`, a copied profile with independent media directories, and synthetic `.vix` files. No hardware, external servers or original show writes.
- UI coverage: empty converter, blank mapping-name alert, six mapped channels including an RGB group, persistent map reload, edit cancellation resetting selection, conversion to an unsaved modified 400ms session, save, overwrite decline, mixing toggle/repeat controls and readable parse errors. Screenshots were inspected; inspection found/fixed initial hidden table columns and error-enum text.
- Independent checks: the saved `.tim` has exactly seven effects with expected start/duration values, two Pulses and five Set Levels. The local .NET DataContractSerializer read the actual UI-saved six-channel map, proving the hashed dictionary contract, named RGB colors, arbitrary ARGB, destinations and canceled edit preservation (`target/vixen2-golden/readback.log`).
- Integrated full-app `.vix` entry smoke passed 12 steps (`target/progress-ui/vixen2-entry-out`): actual Convert Sequence opened a modified unsaved timeline with seven effects and length 400ms; screenshot inspected by the root agent. Remaining manual verification: gesture-level destination drag/color-panel/native file-panel navigation. No Windows application runtime was used.


## Song Inventory and large mapper, November 11 continuation

- Five Song Inventory Rust tests pass, plus the Singing Faces xTiming-routing test and current-locale collation test. Input covers malformed inventories, missing fields, nested categories, bad URLs, provider-error continuation, retained song selection and empty timing. The dedicated scheduler/admin/marks app builds.
- `target/song-ui/roundtrip3`: all 45 steps passed. The actual Marks toolbar Import, Singing Faces radio, browser Select, candidate OK, error OK and Cancel controls were exercised against a local HTTP fixture. Independent before/after JSON confirms one new collection with exactly two marks; missing timing and cancellation preserve that exact state. All observed requests used `Mozilla/5.0 (compatible; Vixen/1.0)` and stayed on loopback.
- `target/song-ui/edges4`: all 18 steps passed. Failed first provider shows an alert, later valid provider loads; the actual native tree song link opens Song Info and Select downloads the expected XML. `target/song-ui/vendor-error`: all 10 recorded steps passed; invalid vendor JSON leaves zero providers and closes cleanly. Screenshots inspected. Earlier native song-row text click did not select under the harness; native link buttons now match the upstream link behavior and the final run passes.
- Mapper: six Rust tests pass, including 50,000 active sources grouped into 500 lazily fetched branches. Full state construction, current-locale sort and all branch reads took 90.3ms in `target/mapper-large-tests.log`. `target/mapper-large/out` passed nine steps: 50,000-row sheet presented in 536ms; both virtualized map and source-tree screenshots inspected. The gesture probe did not establish a successful drag and is not counted as drag verification. Native file-panel navigation remains manual.
- Only `target/package-ui/profile` copies and generated fixtures were written. Browser tests reject non-loopback URLs and redirects. No paid services, actual inventory providers, original shows or hardware were accessed.

## Marks, layers and independent playback review (November 11, 2026)

- `target/progress-ui/marks-final`: 42 native steps passed. Empty marks-panel click takes focus; queued Space controls playback, while a collection-name field keeps keyboard input. Default layer Name accepts a real pointer/text/Return edit to `Base Layer`. Screenshots reviewed.
- `target/progress-ui/dictionary-complete`: 31 native steps passed, plus assertions. A private profile with `user_dictionary` deliberately replaced by a directory raised the expected Error; dismissing it completed timeline paste, and a second conversion reused the in-memory mapping. Earlier probe failures were offscreen OCR choosing title text and an unnecessary post-paste dismissal; only this final run is counted.
- Four effect-selection normalization tests mirror upstream C# expectations (`target/progress-mark-selection-tests.log`). Eight lyric converter tests include cache eviction at profile close (`target/progress-lyric-tests.log`). Thirteen layer tests cover upstream export key order and import round trips (`target/progress-layers-tests.log`).
- Independent review found and fixed old-editor render callbacks replacing a newer editor’s schedule and old clock completions stopping/rewinding a new Play. Regression run: 15 engine playback tests and 6 session playback tests passed (`target/progress-playback-race-tests.log`). The callback now holds a generation claim; terminal clock mutations validate the captured source/generation under the transport lock.
- Audio EOF now retains the final clock anchor until the last sample is audible, including output latency. Full audio suite: 28 passed, one hardware test ignored (`target/progress-audio-latency-tests.log`); the new fake-device test pins a 100 ms output latency and replay after EOF. The later ClockHandle status consistency assertion needs the next rerun.
- Named live contexts and persistent State Preview are under integration: four live-context tests plus nine existing preview-live tests passed (`target/progress-live-tests.log`). These are backend checks; the newly changed State Preview has not yet had its integrated native rerun.


## Web transport and bundled page, November 11 continuation

- `CARGO_TARGET_DIR=target/vixen-session cargo test -p vixen-web --offline`: 11 passed (`target/web-tests.log`). Fake API loopback tests cover every route/verb, form/route/query precedence and bracket dictionaries, JSON-only groups, exact error MIME negotiation, chunked bodies/100 Continue, conflicting length refusal, content limits, static MIME/HEAD/traversal/symlink guards, RFC SHA-1 vectors, masked fragmented frames, control/close errors, handshake, one shared broadcast clock, and shutdown/rebind with stalled HTTP/WebSocket clients. Every bundled file is byte-identical to the corresponding upstream wwwroot file; no assets downloaded.
- `scripts/test_web_signalr.cjs` runs the unchanged bundled SignalR6.0.1 in a local browser-like VM using Node native fetch/WebSocket. Two clients passed 17.7 seconds of operation: 16/18 state updates, four explicit type6 pings, no duplicate timers, first client stop does not interrupt the second (`target/web-signalr-client.log`). The server is the fake-only `vixen-web/examples/protocol_fixture.rs`, bound127.0.0.1 ephemeral.
- With no CUA browser enabled, `scripts/test_web_page.swift` uses system WebKit and a nonpersistent store against that same fake API. Ten checks pass (`target/web-page/result.json`): initial data and static resources, real hub Now Playing, actual DOM On/Timed/Off controls and form bodies, sequence selection/Play/Pause/Stop, controller Start enabling Save, Save request, persistent time setting, debounced search and zero JavaScript errors. Request/state evidence in `state.json`; 1100×900 page snapshot inspected. This is page/transport verification; fake API responses do not establish real controller or session execution.
- Production binding address and bundle location are supplied by the session service. Transport limits:64 client workers,32KiB request headers,1MiB bodies/messages,10-second request deadline,2-second socket-write timeout, bounded negotiated tokens and client queues. Shutdown joins every worker; no services were exposed beyond loopback during these checks.


## Export Wizard direct upload, November 11 continuation

The previous parity table was stale: the direct-upload backend and native controls were already present in the handoff artifacts. They have now been verified.

- 19 wizard unit tests pass (`target/direct-upload-tests.log`). The comprehensive real-copy direct integration test passes (`target/direct-upload-integration-tests.log`), covering FPP sequence/audio/universe/backup/restart, audio rename off, ESP individual upload, five-plus-one ZIP batches/reboot, duplicate names, upload/archive failures, failed detection, stale direct mode on other formats, and temporary cleanup. FPP crate passes14 unit +11 loopback tests; a subsequent reachability regression additionally passes failed127.0.0.2 then successful127.0.0.1 three times.
- Native final runs: FPP28 steps (`fpp-out2`), ZIP28, failure31, cancellation32, failed-load Continue31, failed-load Stop31, unreachable-host warning/recovery22 (`host-out3`), Save export settings30. Total233 passing steps under `target/direct-ui`. Screenshots inspected for format controls, FPP/ESP summaries, running upload, error, host warning and completion. The first test retained the copied profile's v1 format; it was corrected to2.6+. Earlier keyboard host-commit probes did not present the alert; the production commit action was invoked via the harness, then the actual alert OK clicked in the passing run. These failed probes are excluded.
- `scripts/check_direct_upload.py target/direct-ui` independently reads captured bytes and request logs: six FSEQ2 files each40 frames×25ms with the correct renamed media header; six exact generated WAVs; one universe backup/upload/restart; ZIP contents5+1; no ESP audio/universe calls; first-upload failure/cancel stops subsequent sends; Stop on a bad sequence sends nothing. It also parses ModuleStore and verifies the saved direct-upload host and options.
- `scripts/test_fpp_device.py` binds only127.0.0.1 on an ephemeral port. Native tests use `VIXEN_TEST_FPP_PORT` only while `VIXEN_UI_TEST` is set; that path refuses non-loopback hosts, and ping refuses them too. Port80 was unavailable without privileged binding, so no privileged server or real device was used. Production addresses and host validation are unchanged.
- Only the copied `target/direct-ui/profile`, generated fixtures and temporary exports were written. Real FPP/ESPixelStick hardware, multicast output and native folder-panel navigation were not exercised.


### Export summary network preparation

F27 no longer rebuilds controller network data synchronously inside every `view()` call. Rust now caches preparation per Summary visit, gates Next, and rejects late, repeated or canceled results. The UI shows preparation progress or the specified failure/retry message. The integrated main app passed28 preparation-success steps (`target/direct-ui/prepare-out`) and35 injected-failure/Back/recovery steps (`prepare-error-out`); screenshots inspected. A test-only delay made the progress and disabled Next observable. Failure leaves Next disabled; returning to Format and disabling universe generation recovers. The Rust state test separately covers retry with universe generation still enabled and successful cached network data, abandoned generations and cancellation. The latest cancellation-token guard was added after this native build and is included in the scoped Rust rerun; the next integrated rebuild must include it.


## Web configuration and integrated regression — November 11, 2026

- Added the real profile-owned server service and native Tools > Web Server configuration sheet. The upstream logo is extracted unchanged. Settings are a local draft; Cancel does nothing, OK atomically saves the ModuleStore and applies changed settings, and failed writes restore the in-memory store. Listener errors stay visible without crashing startup. Execution reopen replaces its retired live context; shutdown drains services and then clears any contexts started by a finishing request.
- Eight lifecycle tests pass in `target/progress-web-lifecycle-tests.log`. Native checks:55 steps with typed port, checkbox, bounds, Cancel and save/reopen (`target/progress-ui/web-settings-complete`);14 failed-save steps against a private read-only SystemData folder (`web-save-error`);8 fresh-launch persistence steps (`web-reload`). Results include independent assertions and inspected screenshots. All UI runs disable listeners and hardware output.
- The initial integrated workspace run stopped at a loop-range test that included the earlier idle preview at zero; the test now drains that initial frame before measuring the new play. The subsequent run completed3321 passing tests and7 ignored tests before stopping at the web cancellation test being developed by the independent reviewer. This is a checkpoint, not a final all-green suite result. Logs:`target/progress-workspace-integrated*.log`.
- Empty live web contexts now park the frame thread, wake when an effect arrives, and park again after clear; empty timed sequences still advance and end. The integrated engine suite passes31 tests. Audio's final run passes28 tests (one physical-device test ignored); its published clock stops only after the queued tail becomes audible. `target/progress-playback-audio-final.log`.
- `scripts/build_app.sh` now sets `MACOSX_DEPLOYMENT_TARGET=26.0`, matching SwiftPM and Info.plist. A full debug rebuild no longer reports C/C++ objects targeting the host's newer macOS version. Running on an actual macOS26 installation remains an external verification limit.


### Web API, transport review and real bundled page (2026-11-11)

- `target/web-api-tests.log`: 18 passing `web::api::tests` checks on private muted profiles. Covers exact messages/defaults/error status, tree relationships, group behavior, typed generic effects, real mixed frames and expiry, controller/save behavior, backup listing, path containment and literal names, a relative-profile list→play round trip, repeated/editor/scheduled contexts, natural completion, cancellation and render-work bounds.
- `target/web-transport-review-tests.log`: 15 passing transport tests. Review reproduced two failures before fixes: a buffered oversized chunk-extension line bypassed the metadata limit, and invalid UTF-8 text could span separate complete WebSocket messages. Regressions now reject both while permitting character bytes split across continuation frames. Added callback-owned final-server-drop and explicit dual-stack (`only_v6(false)`) loopback checks. Default `WebApi::invoke_cancellable` preserves existing implementors; SessionApi supplies cooperative cancellation.
- Reusable real integration fixture: `cargo run -p vixen-session --example web_profile_fixture -- target/NEW_PROFILE`, then `target/test-web-page http://127.0.0.1:PORT target/RESULTS real`. The fixture refuses existing folders, always mutes output, creates only an unpatched debug controller, and binds only loopback. Enter stops and writes frame evidence. Compile the WebKit harness with the command at the top of `scripts/test_web_page.swift`.
- Final native result `target/web-page-real-verified/result.json`: **17 passed**, all API requests HTTP 200, no JavaScript errors; actual load/render and SignalR Playing→Paused→resumed→stopped transitions. `state.json` records requests/responses and persisted settings; `page.png` inspected with settled loading indicator. `target/web-real-profile-nov11-verified/frame-evidence.json` records **55 frames, 52 colored, 43 red sequence frames**, muted controllers still stopped and **0 contexts after shutdown**. The initial integration run exposed the relative sequence-path issue and led to its dedicated regression; final run is clean. No real controller or external network was contacted.


### Full integrated regression checkpoint

`cargo test --workspace` with native macOS/loopback access completed successfully: **3390 passed,0 failed,7 ignored** (`target/progress-workspace-latest.log`). Includes the real-show comparisons, C# golden effect comparisons, web cancellation/dual-stack guards and xModel backend. Ignored: physical audio-device enumeration, two fixture generators, and four release/performance cases. Later edits require their targeted checks and an appropriate final regression rerun.

State Preview native follow-up:35 steps passed in `target/progress-ui/state-preview-final`, with16 assigned Relays leaves, On held for2seconds, Off/On, Cancel and clean reopen. The SystemData XML hashes remained unchanged. Frame persistence/mixing and release are separately proven by `session/live.rs` tests; the native screenshots verify the controls and draft lifecycle, not physical lights.


### Marks and panel keyboard follow-up

- 47 native steps verify real Shift-Space events: stopped no-op; live tap pauses for Create Collection; paused duplicate ignored; live and paused additions; explicit Stop; Undo and actual Edit > Redo; save. Independent assertions find three saved marks, each450ms with blank text (`target/progress-ui/tap-marks-complete`).
- 46 native steps verify Evenly Divided Marks: zero-range error and retry, End Time entered through native digit keys, four divisions yielding0/2000/4000/6000/8000ms, Cancel and mark-panel hide/reopen (`target/progress-ui/even-marks-verified`). The offscreen time field is focused through a test hook; it then receives actual key events.
- Layers focus had depended on Marks' background responder. With Marks closed, clicking Layers blank space left its name editor focused and Space typed into it. Shared `VixenKit/PanelFocusCapture` fixes this. The reproducer now passes23 steps and starts playback; `target/progress-ui/layer-isolated-focus` is the failing-before evidence and `layer-isolated-fixed` the after result.
- `Mark.CompareTo` sorts equal starts by end time. Core insert/sort previously used start only; fixed with a regression. Core53 and engine31 tests pass (`target/progress-marks-engine-tests.log`).


### Optimized app checkpoint

`scripts/build_app.sh release` produced `target/build-release/Vixen.app`; ad-hoc signature verification passed (`codesign --verify --deep --strict`), executable is arm64 and Mach-O minimum OS is26.0. Release native runs pass47 playback/tapping/undo/redo/save steps and55 web-settings steps, with strict state assertions (`target/progress-ui/release-playback`, `release-web`). The four ignored performance cases pass in optimized mode:10,000-shape scene,100,000-leaf patching stats,1000×100 grid template and whole-display wire numbering (`target/progress-release-performance.log`,7.10seconds total test execution). This build is a checkpoint; fixture-oracle and vendor fixes continuing afterward require a final rebuild. It has not been run on macOS26 or Intel hardware.


## Fixture renderer differential follow-up — November 11, 2026

`tools/golden/fixtures` compiles unchanged upstream fixture models, properties, bases and all nine fixture effects against the existing native .NET support assemblies. Source/assembly hashes and explicit harness limits are in its README and `source-manifest.sha256`; unused Windows UI/image methods throw if reached. It uses only a generated private profile and no controllers.

The checked-in `tests/golden/fixtures.json.gz` now verifies **1,069 cases and 408,500 state values with zero mismatches**, both edited and data-contract reloaded. Cases cover heterogeneous/shared/duplicate/mixed targets, 1–6 heads, authored whitespace, timing boundaries and intervals, range/index/reversed curves, all Line Dance modes/directions/centering/inversion and advanced boundary values, plus generic Fixture range/index/color-wheel/RGB rows. Exact intent types/spans and command metadata are compared in addition to evaluated states (numeric tolerance 1e-10).

The comparison found and fixed three defects: fixture parsing trimmed authored function names/tags/labels; zero-duration/out-of-span Range evaluation retained metadata instead of C# default metadata; and malformed empty/single-point curves generated fallback output instead of stopping the fixture render at the original failure point. Partial output preceding a failing generic row or Line Dance center is preserved. The 34 malformed cases explicitly expect the upstream insufficient-point error (68 edited/reloaded attempts); all other oracle cases must succeed without logged errors. Rust returns the matching empty/partial output without throwing.

Verification: 75 fixture unit tests and 54 core unit tests passed (`target/fixture-golden-unit.log`); ordinary checked-in differential test passed (`target/fixture-golden-checked-in.log`). Regeneration and comparison use private `target/golden` artifacts. No original show or upstream source was modified. This closes the missing fixture render oracle evidence; it does not verify physical fixtures, Windows UI/timeline images, library refresh or every malformed fixture profile. LipSync and State still lack a full C# render oracle at this checkpoint.


### Mapper native gesture attempt

A single CUA `getApp` call targeting the dedicated test build returned no app state for 1,901 seconds and was interrupted. No drag, selection or file-panel action was performed through CUA. The temporary onscreen mapper harness completed its 180-second wait and exited normally (`target/mapper-large/onscreen-out/result.json`, process session 66613), closing its windows. Earlier offscreen pointer probes also did not establish a completed drag. F92 remains partial for physical drag and standard file-panel navigation; no gesture success is claimed. The test-only `elementMapper show` hook can make a future dedicated verification window visible. Mapper error messages now use the readable FFI message instead of the generic localized enum description.

### Export profile and controller native checks — November 11, 2026

`target/scheduler-profile-ui` uses a private copy of the generated direct-upload fixture. The dedicated scheduler build passed. Four completed native scripts contain **148 passing steps**: `profiles2-out` (55), `persistence-out` (21), `sequences-out` (34), and `controllers-multiple-out` (38). `python3 scripts/check_scheduler_controls.py` independently verifies saved profile XML, the sequence-count progression 1→1→6→5→4, multi-row activity/selection, and exact controller start/end channels.

Actual +, rename and delete buttons exercised the name prompts, blank rejection/retry, trimming, duplicate warning/retry cancellation, delete cancellation and confirmed deletion. A separate add/rename/close process persisted exactly `Native Persistence Renamed` in ModuleStore. Sequence Add used supplied file results to check duplicate/outside-folder rejection; the actual Add All and remove buttons and Delete key changed the list. Controller Enable All/Disable All and the zero-active Next guard were inspected in the earlier `sequence-controller-out` snapshots. The final controller script uses native table selection/focus, posted key events and the actual checkbox: Space disables both selected controllers, ticking one enables both, first Escape clears selection, second Escape closes the wizard. This found and fixed missing Space handling and an Escape handler that consumed the key even without a selection.

Limits: standard `NSOpenPanel` opens but its remote contents render blank offscreen (`file-panel-out`); no native file navigation is claimed. Command-A is consumed before reaching the focused table in the dedicated harness, including with posted native events; no select-all success is claimed. Physical controller/Layers/mapper drag remains unverified. Failed probes are retained separately and are excluded from the 148 passing steps. All dedicated test processes exited; no hardware or external service was used.

### Administration fatal exit — November 11, 2026

`cargo test -p vixen-session --test fatal_exit` passes **2 tests** (`target/scheduler-profile-ui/fatal-rust.log`). The parent test runs three disposable subprocesses: idle fatal exit, fatal exit while saving, and a caught Rust panic. Assertions cover exit status 1, all 500 queued log entries flushed through the last marker, exact fatal prefix/message, profile lock removal, the full five-second grace period and a background save completing during it. A caught panic logs Fatal but keeps the process and its lock alive, preserving render/controller panic isolation. Each child has a 15-second timeout and private temporary files.

`python3 scripts/test_fatal_exit.py --out target/admin-fatal-ui-fixed` passes against the dedicated muted app in 2.89 seconds. The hook is available only under `UITestHarness` and additionally requires `VIXEN_TEST_FATAL=uncaught-exception`, matching `VIXEN_DATA`, and an existing profile lock. It raises a real `NSException` on a dedicated thread. The production handler writes `Application.Fatal.log` with the exception name, reason and stack trace, exits 1, and removes `.lock`. The checker hashes the source SystemData before and after; it is unchanged. Result: `target/admin-fatal-ui-fixed/result.json`.

The first native run exposed a production defect: a callback closure created inside `AdminModel.start` inherited MainActor isolation. A background exception reached `__handleUncaughtException`, then trapped in `_dispatch_assert_queue_fail` before logging or cleanup. The failed private fixture is retained at `target/admin-fatal-ui`; it touched no original profile. Registering an explicitly `nonisolated` file-level callback fixes this, and the rerun verified the complete fatal path. Both test app processes have exited.


## State/Face differential and WebSocket close follow-up — November 11, 2026

The unchanged full C# State renderer, planner, discovery, mark parser/coalescer, actual Mark/MarkCollection classes and State property models now run in `tools/golden/states`. Its checked-in oracle passes **1,022 cases / 167,280 state values**, both assigned and serialized/reloaded models. Cases include all render sources, playback modes, iteration/cycle limits, custom grouping, duplicate/shared targets, item/group assignments, RGB/single-color targets and mark clipping/overlap/text boundaries. Every case succeeds without oracle warnings/errors. **146 original State/Face data contracts** separately verify non-system KnownColors, Empty/unknown colors and ARGB alpha, plus distinct whitespace phoneme keys.

This found and fixed numeric `knownColor` IDs being ignored in State/Face property XML (named colors rendered white), whitespace phoneme keys overwriting distinct entries, and the old shared DarkSeaGreen table value ending in `8B` instead of .NET's `8F`. The complete KnownColor table moved to `vixen-core::known_color`; `vixen-io::values` preserves its existing public helper API by re-export. No upstream files changed. The State checkpoint does not cover multiple-discrete color sets or known-color-versus-ARGB identity equality; full LipSync rendering and Windows editor/timeline drawing remain unverified here. See the oracle README/source manifest for provenance and all limits.

A full-suite WebSocket failure exposed a TCP close race: rejecting an invalid text message while another frame remained unread could reset the connection before the close payload reached the client. The transport now half-closes and drains at most64 KiB/200 ms, checking shutdown every20 ms. All15 transport tests pass; the expanded16-connection invalid/fragmented UTF-8 regression passed12 consecutive runs (192 repetitions), with no reset (`target/web-close-drain-tests.log`, `target/web-close-drain-repeat.log`). Only local loopback was used.


The separate `tools/golden/states/build.sh run --discrete-color-probe` confirms one existing deliberate color-equality difference: named `Color.Blue` falls back to red in C# State on the default multiple-discrete RGB set, while Rust renders blue; identical value-only ARGB blue matches. The probe records two expected edited/reloaded mismatches (`target/state-discrete-probe-compare.log`) and is deliberately separate from the passing checked-in matrix. `docs/DIFFERENCES.md` now records this actual render impact instead of claiming none was found. The port's existing value-equality choice remains unchanged. The final scoped checks pass57 audio/state unit tests and255 IO unit plus28 IO integration/round-trip tests (`target/state-golden-unit.log`), and the focused core KnownColor regression passes.


## Controller error logging and recovery — November 11, 2026

Controller runners now receive the application logging callback both when a profile opens and when its graph is rebuilt. Caught start/stop/pause/resume and update failures use upstream logger/message text; ordinary module I/O errors recorded on ModuleShared also reach the log. Callback failures are contained. An unexpected worker failure logs its controller error and shutdown, marks it finished, wakes configuration/stop waiters and leaves other controllers running. A start panic cannot send updates. The visible Start command reaps failed workers and retries directly, without requiring Stop first; independent review found that recovery gap and the regression covers both start and worker failure.

All21 output runner tests pass (`target/progress-controller-logging-tests.log`) including local UDP and pseudo-terminal fixtures; `vixen-session` compiles with the log sink installed. No physical devices were used. The first restricted test invocation passed19 cases but could not bind two loopback receivers; the elevated rerun passed all21. Native controller-window recovery still requires a synthetic failing module to drive through that window; these tests establish the public runner behavior and log callback, not a physical device recovery.


## LipSync FaceDefinition differential follow-up — November 11, 2026

`tools/golden/lipsync` compiles the original LipSync effect, Face property/model and actual mark classes unchanged. All **684 cases /193,828 state values** match, including both explicit model assignment and actual file-reload lifecycle order. Cases cover all phonemes and eye modes, outline/component branch combinations, group/leaf/shared/duplicate targets, RGB/single-color leaves, case/whitespace keys, all mark collection types/missing IDs, clipping/overlap/minimum duration, strict >10ms rest-gap boundaries and frame/duration boundaries. Intent types and time spans are exact; every C# render succeeds without logged warnings/errors.

The initial missing-mark-ID discrepancies came from the harness assigning loaded marks before ModuleData and applying load normalization to explicit assigned models; these were corrected before publishing the checked-in artifact. No further production LipSync rendering defect was found. The earlier Face known-color/whitespace fixes remain covered by the separate146 original State/Face contracts.

The image-map library and FastPicture entry points are throwing stubs; thumbnail dictionaries hold null placeholders and no drawing runs. Thus **LipSync image-map/bitmap rendering remains unverified by this oracle**. README/source hashes record these limits. The ordinary `lipsync_golden` test is enabled and uses `tests/golden/lipsync.json.gz` (`target/lipsync-golden-checked-in.log`). Only fresh private profiles were created; no original show/hardware was used.


### Integrated State/controller checkpoint

`cargo test --workspace` with `MACOSX_DEPLOYMENT_TARGET=26.0` passes **3,410 tests,0 failures,7 ignored** (`target/progress-workspace-state.log`). This includes the State oracle, fixture oracle, KnownColor fixes, controller logging/recovery and WebSocket close fix. LipSync's later artifact and ongoing native editor work need their targeted verification and the final rerun. The normally ignored `lists_real_output_devices` test was then run separately and passed, detecting the Mac's `Pebble V3` output (`target/progress-audio-device-enumeration.log`). This is discovery only: it does not establish audible output or audio/light synchronization. The other ignored cases are two fixture generators and the four performance cases already verified in release mode.


## Glediator raw-frame differential follow-up — November 11, 2026

Unchanged C# Glediator source now matches **277 cases / 167,392 state values** for fresh and reloaded instances, with exact array intent spans/types and RGB bytes. Synthetic recordings cover matrix/string/uneven/shared-target layouts, orientations, speed/iteration values, timing boundaries, brightness curves, missing/blank files, full/trailing frames and malformed short files. Three original divide-by-zero failure cases revealed a real defect: Rust emitted black array intents that could mask lower layers, while C# emitted no intents. The port now drops that failed render; all 6 Glediator unit tests pass (`target/glediator-golden-unit.log`).

A separate repeated-dirty snapshot confirms C# retains its speed accumulator, changing 186/330 element/time samples for Iterations/Speed. The existing deterministic fresh-instance phase is deliberately retained, documented in `DIFFERENCES.md`, and asserted stable on repeated Rust renders. The 277-case exact claim excludes those stateful C# repeat snapshots. Source/provenance/limits are in `tools/golden/media/README.md`; default test uses checked-in `tests/golden/glediator.json.gz`. Only generated private recordings/profiles were used.


## Snowflakes differential and GDI availability follow-up — November 11, 2026

The complete unchanged Snowflakes C# source matches **343 cases / 5,126,164 state values**, with exact array intent types/spans and RGB bytes, for both assigned and reloaded data. The existing `GoldenRandom.cs` supplies the original ThreadSafeRandom thread-local with the same seeded stream as Rust; the effect source is untouched. Coverage includes all ten shapes, five effects, six movements, orientations, four seeds, color/fade/cycling/brightness, buildup, eleven curve extremes, grid/count variants, sizes 1×1–21×23 and timing boundaries. Every original render succeeds without logged warnings/errors; no Snowflakes production fix was needed. This is conditional same-stream differential evidence, not production RNG distribution equivalence. Checked-in artifact and provenance are in `tests/golden/snowflakes.json.gz` and `tools/golden/snowflakes`.

The reusable `tools/golden/media/build.sh run --gdi-probe` attempted an actual 8×8 Bitmap/Graphics line render and Font construction using the same original .NET dependency. Both fail during GDI+ initialization with `DllNotFoundException: gdiplus.dll` (`target/gdi-oracle-probe.log`). No installed/cached libgdiplus was found in local Homebrew/MacPorts paths. Therefore the current environment cannot run unchanged GDI raster/font paths for Shapes, CountDown, Text, Picture or LipSync image maps. This is a limitation of the local C# oracle runtime; it is not evidence that macOS native rendering is unavailable. Source/algorithm and native-port checks must be reported separately from C# raster equality.


### Timeline row outlines

F26 now paints left/right selection borders in two fixed one-point viewport strips using the same painter as the grid. The first implementation repainted entire edge tile columns; independent review found that cached tiles could briefly leave a border inside the view during scrolling. The final strips preserve row/mark/effect compositing and prevent that interior ghost. Grid invalidation, image arrivals, scale/color-space changes and benchmark readiness include all three planes. Review also found a new one-point tile-index overflow near the i32 canvas boundary; indices now clamp before Swift casts and reserve arithmetic uses native Int.

`target/progress-ui/row-edge-strips` passes29 native steps and independent pixel/state assertions: selection of one/two/no rows,100-point scroll, resize, correct two border columns with no stale interior lines, and actual cache/prefetch/resize/zoom near Int32.max settling without a trap. The zero-time cursor correctly composites over the left border. Performance is being rechecked with the final strip implementation; earlier full-width tile timings are superseded.

## CountDown numeric differential follow-up — November 11, 2026

The complete unchanged C# CountDown source is compiled, but its GDI/font constructors and render methods are deliberately not invoked. Reflection initializes only the fields consumed by the original timing/fade/direction/spinner helpers. **405 cases / 23,764 frame transitions** match the port, including all count/fade modes, random/fixed directions under a shared seeded stream, minute boundaries, positive/negative Int16 counts, per-iteration movement, frame/duration boundaries, and every remaining millisecond in a second.

This caught a real spinner defect: C# uses decimal arithmetic, so 550 ms remaining gives exactly 198 degrees; Rust floating-point division could produce 198.00000000000003 and ceil to 199. Exact integer ceiling now matches all oracle cases. The checked-in `tests/golden/countdown-math.json.gz` runs as a normal graphics-crate test. Source hashes and explicit limits are in `tools/golden/countdown/README.md`; generation and comparison logs are `target/countdown-math-{generate,compare}.log`. This does not verify font metrics, raster output, whole-effect lifecycle, or arbitrary malformed values.

The graphics crate's earlier 47-test native pass included two video tests that failed only inside the sandbox. As required, a minimal elevated video-decoding probe passed (`target/pixel-video-elevated-probe.log`), then all 47 graphics tests passed with native access (`target/pixel-graphics-oracle-elevated.log`). No software fallback was introduced.

## Shapes geometry differential follow-up — November 11, 2026

The unchanged original `Shapes/GeometricShapes.cs` now matches **5,393 cases / 390,833 coordinate values**. The oracle uses value-only SVG output containers, with no replacement geometry algorithms, and invokes every geometric builder: circle/ellipse/rectangle, triangle/polygon, intersecting/nonintersecting/north stars, cross and arrow. Radii, ratios, side counts, skip points and inner sizes cover normal settings and broader numeric combinations. Polygon coordinates compare exactly; dimension strings must encode the same Float32 value.

The comparison found 34 one-pixel star-coordinate differences. Original PointF coordinate multiplication/addition occurs in Float32 before conversion to Math.Sqrt's Float64 input; the port had promoted the operands too early. Keeping those operations in Float32 fixes every case. The ordinary graphics test uses `tests/golden/shapes-geometry.json.gz`; source/provenance and boundaries are in `tools/golden/shapes/README.md`. Logs: `target/shapes-geometry-{generate,compare}.log`. This does not verify SVG parsing/rasterization, animation, marks, full-effect lifecycle, or embedded-art appearance against C#.

Graphics follow-up: all **50 graphics library tests pass** with native access after the CountDown and Shapes fixes and Picture sampling checkpoint (`target/pixel-graphics-oracle-final.log`). Video test fixtures now use a separate UUID directory per test; previously parallel tests shared a process-wide directory and could delete another test's clip. This is a test-isolation fix, separate from the observed sandbox decoding restriction.


Controller logging follow-up: driver-owned OpenDMX/RDS/launcher workers can report outside the runner's update call or after Stop. Polling error counts around operations could miss these. ModuleShared now sends each ordinary recorded error directly to its callback, outside all shared locks; caught runner exceptions retain their specific lifecycle log message without a duplicate generic line. Callback panic isolation remains. All22 runner tests pass (`target/progress-controller-logging-final.log`), including a background error after Stop and logging installed after controllers were added.

### Native function keys and final strip performance

Queued native F8/F9 events were claimed by the closed SwiftUI Playback submenu without running their actions. The window now dispatches F5/F8/F9 through its existing tested transport-command table after text/editor/sheet focus guards. Actual native F5/F8/F9, Space and Option-Space pass60 steps and13 exact transport/loop snapshots (`target/progress-ui/key-routing-fixed`). Previous probes without/with the function modifier did not establish F8/F9 success; only the fixed result is counted.

Final viewport-strip scrolling benchmark on a private `Christmas Eve Sarajevo` copy (1,095 effects,1,136 rows) ran600 display-paced frames at60Hz in the debug app: main-thread work p95=2.335ms, maximum3.809ms; no frame exceeded two periods. Strips paint asynchronously during scrolling and all tiles settled14.92ms afterward (`target/progress-ui/timeline-edge-strips-bench/timeline-bench.json`). This is the final strip design, not the superseded full-column repaint. Original show and physical output were untouched.

## Picture movement/sampling differential follow-up — November 11, 2026

The complete unchanged Picture source and original FastPixel reader compile against the same read-only native dependencies. The oracle supplies opaque image bytes directly and invokes the actual movement, per-frame color/brightness and sampling methods. **306 cases / 542,400 pixel values** match, covering all 16 movement selectors, orientations, three string grid sizes, two image sizes, tile direction boundaries, speed/movement rate, center stop, custom color/brightness/offsets and 64 location cases with zero and mixed-sign offsets. No Picture algorithm correction was needed.

This bypasses image decoding, GDI resize/grayscale, GIF scheduling and partial-alpha storage; Resize is tested only with an already-sized image. It does not establish full Picture raster parity. An initial arbitrary-alpha input probe exposed premultiplied-buffer representation differences, which cannot be equated to original post-GDI input without the unavailable drawing path; the published comparison is explicitly opaque. The original FastPixel byte reader is compiled unchanged, not replaced with a port of its math. Source hashes and these limits are in `tools/golden/picture/README.md`; checked-in fixture `tests/golden/picture-sampling.json.gz` runs in the ordinary graphics suite. Logs: `target/picture-sampling-{generate,compare,checked-in}.log`.


### Second optimized bundle checkpoint

The refreshed optimized arm64 bundle builds and passes strict ad-hoc signature validation (`target/progress-release-refresh.log`). It passes60 native transport-key steps and29 row-edge/coordinate-limit steps with the same strict state/pixel assertions (`target/progress-ui/release-key-routing`, `release-row-edge-strips`). The dedicated native crash test also passes against this optimized bundle: exit1, fatal exception/stack logged, lock removed and source hashes unchanged in2.29seconds (`target/progress-ui/release-fatal/result.json`). Later background-driver logging and graphics arithmetic edits require another final optimized rebuild.

The remaining Text and CountDown full-image paths depend on original GDI font metrics/brush drawing, Shapes on Svg.NET/GDI rasterization, and Picture/LipSync image maps on original bitmap decode/scale paths. Video's original `InitialRender` also unconditionally opens frame images and resizes them with GDI after its ffmpeg extraction. Those paths cannot be promoted to exact differential verification by the available non-raster probes. Native port tests cover actual CoreText/image/AVFoundation execution, including video order/start offset, but remain distinct evidence. No new Text/Video/LipSync matrix parity claim is made.


### Native D/I distribution and menu routing

A real posted D key was consumed by AppKit's disabled system `Start Dictation` menu entry without invoking distribution. Plain window keys now dispatch through the existing Rust table after the window/sheet/text/editor guards; modified menu shortcuts retain priority. An independent review found no new focus/menu regression. F5/F8/F9 keep the same direct route described above.

The expanded native script passes **80 steps**, including posted D and I, exact three-effect distribution, dialog Cancel, specified500ms duration,100ms spacing, Last Effect ordering, and actual Edit-menu Undo/Redo. `scripts/check_sequencer_distribution.py target/progress-ui/key-distribution-apply-final` independently asserts six full timeline states, exact integer ticks (including the original floating-point truncation), undo entries and command routes. Screenshots verify real controls and radio choices. The reusable script is `scripts/ui/sequencer-distribution.json`; this run used `VIXEN_DATA=target/progress-ui/key-distribution-show`, `VIXEN_TEST_SEQUENCE=.../Sequence/Ruler.tim`, `VIXEN_MUTE=1`, and a fresh output directory. The offscreen pointer fixture supplies the selected-effect-under-pointer precondition; key dispatch and controls are native. Earlier probe/menu-label failures are not counted.

The owner confirmed on November11 that the port should keep value-based colors; named Blue stays blue rather than reproducing .NET's identity-based fallback. This is an accepted difference, not an outstanding question.

The expanded whole-workspace checkpoint passes **3,419 tests,0 failures,8 ignored** (`target/progress-workspace-graphics.log`). Picture's fixture was still being published when that run compiled; it is now enabled and all50 graphics tests pass separately. The remaining seven normally ignored cases are real audio-device enumeration (separately passed), two fixture generators, and four release performance tests (separately passed). Later native/editor changes still require the final integrated run.

## Bundle self-containment and relocation — November 11, 2026

New read-only `scripts/check_app_bundle.py` verifies bundle metadata/version, executable permissions, arm64 architecture, macOS load-command minimum, all resources, code signature, contained symlinks, native dependencies and rpaths. The current optimized bundle passes: **220 resources / 9,702,124 bytes** match the source exactly, its single Mach-O image targets macOS 26.0, all 47 dependency install names are system libraries/frameworks, and its only rpath is `@loader_path`. Strict ad-hoc signature verification passes. Negative private-copy probes reject a missing License.txt, escaping resource symlink and added `/opt/homebrew/lib` rpath (`target/bundle-verification/negative-probes.json`).

`python3 scripts/test_bundle_relocation.py --out target/bundle-verification/relocation-final` copies the app outside the checkout into a temporary path containing spaces/non-ASCII, then launches it from an unrelated working directory with a fresh empty profile and isolated test settings. **15 native steps pass** in 4.05 seconds: Administration is ready, About loads the bundled image/license, and Release Notes loads the bundled fallback with external update transport disabled. Text assertions and all three snapshots were inspected. The web listener stays stopped, audio/controller output is muted, and no original profile is read or written. The first run's only failed assertion incorrectly expected the persisted default web-enabled flag to be false; the corrected assertion checks that the listener is stopped. No production packaging defect was found.

The relocation script directly launches the executable and the harness exits directly. It does not establish Finder/Gatekeeper behavior, Developer ID/notarization, graceful shutdown, Intel support or execution on macOS 26. The checked app is arm64 and ad-hoc signed; current-host native launch is verified. Re-run both scripts after the final integrated release rebuild. Commands and boundaries are in `docs/RELEASE_CHECKS.md`.

## Preview dialog completion — November 11, 2026

`python3 scripts/check_preview_dialogs.py` passes **421 native steps, zero failed**, plus independent saved-XML assertions (`target/preview-partials`). It verifies invalid-link close/Keep Editing/Close Anyway; native Boolean/enum double-click; Find/Replace and Reverse Elements; Prop tags and tag-color Save; Create Template invalid-name/OK/Cancel; group string picker and Sync; the complete eight-page fixture wizard creating three uniquely linked heads; fixture selection OK/Cancel/clear; linked-element Copy/Reverse/OK/changed-clone Cancel, native canvas and tree context menus, and native live context menus/border toggling. A second tree run hashes both system files while edited rows/tags are visible and confirms no write before designer Save. Dedicated preview/admin/setup build passes (`build-linker.log`), and representative snapshots were inspected.

Review of the linker menu found that the right-click row was calculated but discarded in favor of potentially stale `NSTableView.clickedRow`. `ContextTable` now passes the event row directly to its menu provider. Actual native context menus then copied all 50 rows, persisted them, reversed a distinct three-node assignment, and preserved the saved values when those later edits were cancelled. The offscreen Outline double-click assignment uses a guarded hook that calls the same sheet callback with native table/tree selections; physical Outline double-click/drag remains partial. Test fixture creation uses a local existing Test Head definition and generated controller-free profiles; no original show, audio output, hardware, network or paid API is involved.

Twelve Preview parity rows moved from partial to done (F4,F24,F27,F30a,F37,F39,F63,F64,F86,F91,F92,F94). Remaining rows explicitly retain physical tree/Finder drag, window position, click-through and native input limits. No further computer-use calls were attempted after the earlier service timeout.


### Genuine keyboard-event follow-up and document shortcuts

An isolated AppKit/SwiftUI probe showed that manually constructed `NSEvent.keyEvent` values do not establish physical shortcut behavior: Command-Shift-Z could fail even with character strings identical to a real event. The same command succeeds when generated by `CGEvent` and converted to `NSEvent`. The earlier Shift-Z failure was a harness artifact. `sequencer.keyEvent` now uses CGEvent-backed local application events; none are posted globally. Earlier manual-event interception observations for D/function keys are not by themselves proof of a physical-key defect. Their direct command routing is independently reviewed and subsequently checked with CG-backed events. Probe evidence: `target/key-probe/findings.md`.

The actual hidden SwiftUI Command-Y button does not register a key equivalent. The sequencer now dispatches exact Command-S/Option-Command-S and Command-Z/Shift-Command-Z/Command-Y to the same document methods as its menus after window/sheet/text/editor guards. Command-Shift-S remains alignment. CG-backed native checks pass **47 steps** in both debug and optimized builds, covering selection, copy, paste, both redo routes, undo and saving. `scripts/check_sequencer_commands.py` verifies all nine snapshots and the actual saved six-effect XML. Artifacts: `target/progress-ui/key-commands-cgevent`, `release-commands-cgevent`; reusable script: `scripts/ui/sequencer-commands.json`. Optimized D/I distribution also passes all80 steps with CG-backed keys (`release-distribution-cgevent`), checked by its durable verifier. Offscreen key ownership and cross-window focus remain separately under investigation; these successful document-command results do not claim global OS shortcut dispatch.

The fresh integrated Rust run passes **3,421 tests,0 failures,7 ignored** (`target/progress-workspace-final-integrated2.log`). An earlier attempt stopped at an existing random Liquid test. The test compared unrelated continuous particle runs and failed1/100 repeats. Its replacement compares the same seeded single-pulse release with/without the barrier and checks retained/escaped particles across16 seeds;100 repeated runs and all156 physics tests pass. Production physics did not change. New ownership/prop/evaluator work after this checkpoint requires another final integration run.

Preview keyboard probes must be distinguished from the 421 successful dialog steps: `keys-active2-out` obtained real key ownership and proves CG-backed ⌘Z/⌘Y/⌘Z/⇧⌘Z with counts 1→2→1→2→1. The exact private process exited and the previous foreground application was restored. Dedicated CG-backed Save also persists before close. Combined-app Save still needs real key **and main** ownership after Commands settle. `keys-settled-out` acquired neither (previous foreground reported SecurityAgent), so its results are discarded and no production Save/Undo defect is inferred. Synthetic NSEvent strings alone had caused an earlier false Save failure; CG backing matters. Preview's inactive Save shortcut is now unregistered, and its harness registry falls back only when no real key/main window exists.

## Curve generator against the exact NCalc package — November 11

The original `CurveEditor.cs` function handler and point evaluator, extracted
unchanged, run against the upstream-pinned NCalc2 2.1.0 and Antlr4.Runtime 4.6.4.
**4,282 cases / 8,650 points, zero mismatches**, including 2,129 error cases, compare
point values/order and syntax-versus-runtime error titles. This corrected
parameter-case handling, typed arithmetic and bitwise operations, exact Decimal
behavior from `Abs`, math boundaries and partial graph retention after errors.
The pinned MIT `rust_decimal` 1.43.0 dependency supplies checked 96-bit arithmetic;
its license is bundled. All original source and binary hashes are retained.

Run `cargo test -p vixen-session --lib original_ncalc_curve_contracts`; the fixture
is checked in, so normal tests need no .NET or downloaded reference. Regeneration,
source provenance and scope are in `tools/golden/ncalc/README.md`. The matrix uses
invariant culture on .NET 10 and does not compare diagnostic wording, all locale
number/date formats, or every possible grammar input. Native sheet interaction is
separate; zero/negative Draw Interval remains a deliberate nontermination guard.


### Integrated completion checkpoint — November 11, 2026

The all-area debug and optimized arm64 bundles build after the playback-ownership, NCalc, Custom Prop, Preview and accessibility fixes (`target/progress-debug-verified.log`, `target/progress-release-verified.log`). The complete Rust workspace passes **3,430 tests, 0 failed, 7 ignored** (`target/progress-workspace-verified.log`). The seven ignored cases remain two fixture generators, audio-device enumeration and four release-performance cases; the latter five passed separately at the earlier documented checkpoints.

The preceding full run found a real completion race: the engine could stop between the UI's transport tick and its decision to continue ticking. The controller now requests the final tick until it observes and processes the stop, clearing the time label and restoring the playback range. A deterministic real-engine regression exercises that exact gap; a pure controller test proves the cleanup is emitted once. All208 sequencer tests pass before the complete rerun (`target/transport-final-tick.log`). The failed checkpoint remains in `target/progress-workspace-complete.log`; it is not counted as a passing run.

Native debug verification passes **187 steps** for transport and document keys/distribution (`target/progress-ui/debug-keys-final`). The reusable runner `scripts/test_sequencer_keys.py` copies a private Ruler fixture for each case and checks exact state and saved XML through three independent checkers. Its transport checker explicitly preserves the original behavior that Stop hides the playhead while retaining the owning editor's clock. The initial checker incorrectly required that clock to become zero; correcting that assertion did not change app behavior. Keys use CGEvent-backed NSEvents posted only into the test app; physical keyboard/global WindowServer routing is not claimed.

Native two-editor verification passes **47 steps** with the final debug bundle (`target/progress-ui/ownership-verified-debug`), as well as the earlier47-step checkpoint. `scripts/test_editor_ownership.py` opens two private copies and reads each real FFI transport/position plus its actual canvas playhead. It verifies that only the owning editor advances, replacement clears the old editor, stale Stop does not stop the new owner, closing the old editor preserves a paused owner, and resume/Stop behave correctly.

Both debug and release bundles pass strict static audits with **221 exact resources /9,703,191 bytes**,47 system-only native dependencies and only `@loader_path` as rpath (`target/debug-bundle-verified.json`, `target/release-bundle-verified.json`). The optional debug audit found SwiftPM's unused absolute PackageFrameworks path; the build now removes that exact checkout path from the copied executable before ad-hoc signing. Release had no such path. A relocated optimized copy passes15 native startup/About/Release Notes checks from an unrelated working directory with spaces and non-ASCII characters (`target/progress-ui/release-relocation-complete`); screenshots were inspected. This relocation checkpoint precedes only the natural-completion fix. No Finder/Gatekeeper, Developer ID/notarization, Intel or actual macOS26 runtime claim is made; host is arm64 macOS27.2.1.

Final optimized native follow-up: **187 key/distribution steps and47 two-editor ownership steps pass** against the natural-completion-fixed release (`target/progress-ui/release-keys-verified`, `ownership-verified-release`). Saved XML, all playback snapshots and screenshots are checked. The deliberate native exception test also passes (`target/progress-ui/release-fatal-verified/result.json`): exit1, fatal exception and stack logged, profile lock removed and source profile hashes unchanged. All tests use fresh copies, muted output and local-only events.

## Original rotation and stereo helper checkpoint — November 11

Bars now executes the unchanged Microsoft WPF v10.0.0 RotateTransform and extracted
Matrix/MatrixUtil/Transform methods, with only property-system hosts substituted.
The three audio effects execute unchanged Vixen cached/mono sample providers and
GetMonoSamples against the exact upstream-pinned NAudio.Core 2.3.0 assembly.
Both dependencies are MIT; source/package URLs, hashes and notices are recorded in
`tools/golden/helpers`.

Regeneration yields byte-identical JSON for **2,311 cases** (Bars 1,335, Vertical
Meter 342, VU Meter 288, Waveform 346). All four Rust golden tests pass:
**16,954,580 frame values, zero mismatches**. No production rendering or fixture
change was needed. Reproduce with `tools/golden/helpers/README.md`; evidence is in
`target/helper-reference/{regenerate,rust-golden}.log` and
`target/golden/helpers/comparison.json`. This closes the memory-based helper gap;
Windows native execution, decoder behavior and GDI remain separate limits.


### Native concurrent profile ownership and normal shutdown — November 11, 2026

`scripts/test_profile_lock.py --app target/build-release/Vixen.app --out target/progress-ui/profile-lock-shutdown` passes with a fresh empty private profile, muted output and three real app launches. While the first process holds the profile, the second shows the actual already-in-use alert and exits without starting a second core or removing/rewriting the live owner's lock. The first exits via **System > Shutdown and Exit**, logs the complete engine close and removes its lock. A third launch starts successfully on that same profile and also shuts down normally with no retained lock. The alert and reopened-window screenshots were inspected. This adds actual concurrent-process UI and graceful-shutdown evidence; it does not exercise Finder/Gatekeeper, another macOS user, Documents permission prompts or hardware.

Two preliminary harness assertions were corrected before the successful run: offscreen AX may omit otherwise-rendered Administration controls, so readiness is verified with actual startup log entries; the script must wait after invoking Shutdown so its own direct finish/exit cannot outrun the asynchronous production shutdown task. Those probe runs are not counted as production shutdown failures.

## Modifier isolation follow-up — November 11

The sequencer preserves real menu equivalents first, then leaves unclaimed
Control combinations to AppKit instead of dropping Control before the Rust key
table. The same guard applies to events forwarded from Preview. Preview-only
Command-L/U/Option-U/? equivalents are registered only while a designer is active;
window Save/Undo/Redo still call the same scoped methods as their menus.

`scripts/test_sequencer_modifier_guards.py` passes **45 native steps** in the
rebuilt full debug app (`target/progress-ui/modifier-guards-native2`). Control-D/I/F5
and forwarded counterparts leave all three effect ranges, undo/modified state,
transport and playhead unchanged. Command-U reaches Align Durations and changes
1/2/3-second durations to 1/1/1; Command-Z restores 1/2/3 exactly. The screenshot
was inspected. These are CGEvent-backed AppKit events local to the test app, not
physical/global WindowServer-keyboard coverage. The first script probe needed
selection timing, pointer-position and quit-count corrections; production code
was unchanged between those probes. Final release integration follows separately.


### Final local acceptance checkpoint — November 11, 2026

After the Wave lifecycle and keyboard modifier fixes, the complete workspace passes **3,431 tests, 0 failed, 7 ignored** (`target/progress-workspace-final.log`). Final debug/release builds pass (`target/progress-debug-final.log`, `target/progress-release-final.log`), and both bundles pass221-resource audits (`target/debug-bundle-final.json`, `target/release-bundle-final.json`). No production edits followed this build.

The exact final optimized bundle passes **294 native steps**:45 modifier isolation/alignment,187 document/transport/distribution,47 two-editor ownership and15 relocated startup/resource steps. Dedicated native fatal cleanup also passes: exit1 in2.35seconds, exception/stack logged, private lock removed, source hashes unchanged. Artifacts are `target/progress-ui/release-{modifiers,keys,ownership,fatal,relocation}-final`. Prior three-process profile-lock/normal-Shutdown evidence remains in `profile-lock-shutdown`; that application code did not change afterward. Tests stay muted and use disposable profiles.

`target/completion-checkpoint.json` records report paths and exact final executable SHA256 hashes. Release: `707391ae256738d63a34812e587fcc946f221403b7a3955684fefad6b70ac9ac`; debug: `0743330cbff0fdbb08fc74cfebf088308a716018eac181803431d8cf8b117a40`. The local implementation/regression work is complete. The overall goal remains active: actual foreground gestures/focus/panels, Windows interoperability/GDI/PCM, physical devices/audio and minimum macOS26 execution remain unverified as listed in `REMAINING_VERIFICATION.md`. Preview's final Save focus attempt remains invalid because a SecurityAgent dialog prevented real main/key ownership; no claim was inferred from it.

## FPP HTTPS and redirect completion

- The earlier HTTP-only and no-redirect differences were implementation gaps. The wizard still accepts the same host/IP values and starts HTTP, but an HTTP device can redirect; the public client also now accepts HTTPS URLs.
- `crates/vixen-fpp/native/fpp_http.c` uses macOS system libcurl with certificate and hostname verification, HTTP/1.1, streamed disk uploads and bounded response buffers. An explicit redirect loop preserves the whole-call timeout and cancellation, changes POST to GET for 300/301/302/303, replays exact file bytes for 307/308, follows at most 50 redirects, and refuses HTTPS→HTTP at every hop. The independently generated 33-case C#/.NET reference is `tools/golden/httpclient` and `target/golden/httpclient-reference/results.json`; local reference cases also establish final 3xx return at the hop limit and no follow for 304/305/306.
- Independent review found no remaining blocker in native ownership, callback/file lifetimes, cookie locking, redirect policy or test-network containment. All 31 FPP tests pass (20 unit + 11 existing loopback integration), `target/fpp-https-test.log`. New `src/http/tests.rs` and `tests/transport_server.py` cover trusted local TLS, untrusted certificate and wrong-hostname rejection, direct TLS and HTTP→TLS, TLS downgrade refusal including HTTP→TLS→HTTP, relative/absolute/missing Location, method conversion and 300 KB file replay across two redirects, whole-chain deadline, HTTP/TLS/upload cancellation, early 413, oversized response/header and truncated file errors. Cookie tests verify persistence across redirects, successive calls, eight concurrent calls and an upload; separate clients and hostnames keep their cookies isolated. UI-test absolute and protocol-relative redirects to an external address are refused before connecting. Existing device route, JSON, chunked/close-delimited framing and upload tests also pass.
- Certificates and fixture profiles are temporary; the optional private CA is supplied only by internal unit tests. Production never disables certificate/hostname verification or changes system trust. While `VIXEN_UI_TEST` is set, every initial/redirect URL and actual socket address must stay loopback; proxies are disabled in that mode. No real player, external network, hardware or paid API was used. Integrated application build and real-device verification are recorded separately.

## RDS HTTP(S) completion audit — November 11, 2026

The documented HTTPS omission was a real upstream capability gap. RDS now uses system libcurl with certificate/hostname validation, challenge authentication, explicit HttpClient redirect rules, per-request cookies, bounded responses and cancellation/join on controller Stop. All **301 output tests pass** in `target/rds-output-tests.log`, including trusted/private TLS, unknown CA and hostname rejection, Basic/Digest, redirect credential stripping and cookie isolation, deadlines and restart. No device or external service was contacted. See [RDS transport review](review/rds-https.md).

The published local framework oracle passes **33 cases on .NET 10.0.12**: `bash tools/golden/httpclient/build.sh run`, with request traces in `target/golden/httpclient-reference/results.json`. It pins GET/POST redirect behavior for RDS and FPP; it uses a test-only TLS callback and makes no Windows or production certificate-validation claim. Rust transport tests separately verify real TLS rejection. Final integrated bundle linking is a separate checkpoint.

## November 11 — legacy reachable media restoration

The version0 media copy is now performed by sequence-opening sessions and package import before the original absolute source path is lost. Six private session tests, all13 package tests and all283 I/O tests pass. Existing media is preserved; atomic publication, missing Windows sources, failure cleanup/retry and concurrent copies are covered. See [migration review](review/legacy-media-migration.md). This source change still awaits the next integrated bundle acceptance.

## November 11 — Video decoder and persistent cache checkpoint

The elevated native audit confirmed that a valid MPEG-4 Part2 AVI is readable for metadata but fails AVAssetReader decoding. `vixen-video` now retains successful native behavior and retries unsupported formats through bundled FFmpeg. All14 elevated decoder integration tests and all20 unit tests pass (`target/video-fallback-native-decode.log`, `target/video-fallback-native-unit.log`), including native byte identity, actual AVI output/start/EOF, software rotation, cancellation, reset before retry and12 persistent-cache tests. This includes a600-frame cache exceeding1GiB of total decoded pixels with lazy bounded reads. See [decoder review](review/video-decode-fallback.md) and [cache review](review/video-cache.md). These are backend/native-media checks; the next integrated bundle, actual window interaction and minimum-macOS26 execution remain separate checkpoints.

## Persistent Video cache completion

The Video cache Options now control actual BMP/PNG files and close-time cleanup. The native decoder streams into atomic cache staging, and Video renders through lazy disk frames with at most two decoded images resident. Retained caches reopen from disk; deleted effects retain their cache through Undo until close; concurrent windows/readers and late workers are protected. Read-only Cache Size uses encoded image bytes. See [cache implementation and lifetime review](review/video-cache.md).

Scoped results: **12 cache tests**, **6 existing Video effect render tests**, and **5 Rust sequence integration tests** pass in `target/video-cache-{tests,effect-tests,session-tests}.log`. The sequence tests use generated local MOV/AVI media, muted AppCore instances and native/software decoders; they assert the effect-editor panel-model Label but do **not** constitute an observed AppKit/SwiftUI label run. Independent code review passes. Rebuilt-application verification is a separate checkpoint. No original show, audio output, hardware, external network or paid API was used.

## Audio-device startup restoration

The final active-settings audit found that a saved non-default audio device was restored only when an editor opened. Scheduler or web playback started first therefore used the system default. Upstream `AudioOutputManager.Activate` loads preferences before `GetAudioOutput`, independently of an editor. `ShowPlayback` now restores the choice at construction and resolves missing devices to the system default; opening another editor no longer reapplies an older saved choice. All12 playback tests pass in `target/audio-device-startup-tests.log`. The two new regressions use a private preferences file, injected device availability and muted contexts; they inspect configuration and confirm no audio stream opens. Physical device routing remains unverified. No further active-setting omission was found in the audited global Options, sequencer preferences, live-preview flags, web settings, prop preferences or advertised media filters; the separately identified codec gaps are covered by their ongoing media acceptance work.


### Single Export Sequence and Image Maps controls — November 11, 2026

The existing optimized release `4c98baee…` passes **118 native steps** in the
single Export Sequence sheet: pause/resume (including stable paused position),
Stop with existing destination preservation and staging cleanup, a real
ENOTDIR failure alert/OK followed by recovery, and normal completion/resume.
The completed artifact is a valid 78-byte FSEQ with 600 frames, 50 ms timing,
zero patched channels and two complete empty Zstandard blocks. This proves
lifecycle/publication, not channel values; the destination is test-supplied.
Report: `target/export-sheet-native-final/native/result.json`; details and
excluded exploratory runs: `docs/review/export-sheet.md`.

Image Maps on the same release passes **106 native steps**: 34 Delete
Cancel/confirmation, 40 AI/E mapping-popup choices and 32 Overwrite. Saved
readback and reopened renders confirm changed colors/default fallback. Reports:
`target/image-maps-native/{delete-isolated,mapping-accepted,overwrite-accepted}/result.json`.
These do not yet establish live open-effect cached-color replacement or a
physical rename-start gesture. A bounded read-only render observer and guarded
registration of the existing Image Maps test hooks are being built for the
remaining live-render and rename-field key checks.


### Final Image Maps rename correction and live-render acceptance

Release `0d7edc262607ad317134ab6f2fc03e8730d72151d7d6b0b74342d0b6331f992d`
passes **174 native steps**:61 rename/selection,45 image-map acceptance,34 Face
Cancel and34 Face acceptance. Both debug/release bundles build and pass the
222-resource/four-executable audits. Original `AfterLabelEdit` always repopulates
the list, clearing selection. The Swift sheet had reselected the renamed entry,
so a deferred callback incorrectly enabled Edit/Remove/Clone. Both commit and
Cancel now clear selection. The retained pre-fix state fails this condition;
final native states and screenshots show only New after Enter/Escape, all
actions after native table reselection, then only New after commit.

Live cached-intent observations hold the same three effect IDs: accepted image
changes replace32 red pixels with32 green, Face acceptance replaces an empty
render with one white intent, and both LipSync generations advance1→2. Cancel
and editor-only staged image changes preserve cached output; unrelated Set Level
remains byte-equivalent at generation1. The observer only reads cached intents
at50ms, with explicit bounded/truncated reporting; it cannot trigger a render or
apply pending results. Three focused Rust tests and the3518-test workspace pass.

Reports and failed exploratory attempts are distinguished in
`docs/review/image-maps-native.md`. All four final reports are under
`target/image-maps-native/final-{rename-table,map,face-cancel,face-accept}`.
Physical rename-start and row-click gestures remain unverified; entering rename
uses the existing hook and reselection invokes `NSTableView.selectRowIndexes`.
Actual posted Enter/Escape, checkboxes and footer controls are exercised.
Independent review found no remaining issue in this bounded correction.

The preceding diagnostic build `98d898de…` separately passed168 related native
steps and the187-step key/workflow regression; they remain associated with that
exact hash and are not added to the final174 count.

### Windows copy-and-run handoff

Historical preparation/partial-return evidence; current v3.2 capture results are recorded at the top.

The user confirmed an existing Windows Vixen PC and accepted a local-copy
workflow. The reviewed ZIP at
`target/windows-capture-pack-final/Vixen-Windows-Capture.zip` contains39 files
(4,022,436 bytes), including the exact diagnostic MP3 and prebuilt managed
capture/GDI-availability hosts. SHA256:
`cc60e4fa8caaccc3b567b2dc65a3948697766d421b6d02310b6cf0b1a68ef345`.
Archive hashes, managed PE/runtime metadata, original capture sources and Windows
SystemEvents dependency provenance pass local review. Both hosts build with
zero warnings/errors and the13 local capture checks still pass. Cached dependency
licenses are included. The Windows PowerShell script is statically reviewed;
Windows execution has not occurred here.

Instructions were delivered: copy/extract outside the Vixen/show folders, close
Vixen, run `Run-Capture.cmd`, then return the generated results ZIP. The script
records installed Vixen/.NET versions and returns diagnostics when the matching
.NET10/NAudio2.3 prerequisites are absent. No SDK/runtime install or remote access
is required by this workflow. At that handoff Windows results were pending; graphics availability
probes do not establish original effect raster parity. Reproduction and details:
`tools/windows-pcm/START-HERE.txt`, `README.md` and
`target/windows-capture-pack-final-check.json`.


### Current-release portable Mac pack

The renamed-selection-corrected release `0d7edc26…` is packaged at
`target/vixen-macos26-verification-pack-image-map-final.zip` (37,241,115 bytes;
SHA256 `e87e875535fd5421591b422e01929e7360761ea2228f9f85c33d92ec4333312b`).
A fresh extraction to a private path outside the checkout, including spaces and
`é`, verifies all481 hashes, five executable permissions, deep/strict app signing
and strict OCR signing. The copied pack then passes164 native steps (15startup,
11WMA,47playback ownership,91Video metadata) plus helper ownership/Stop cleanup.
All packaged inputs remain unchanged. All processes exit; independent review
passes. Reports: `target/second-mac-pack-image-map-{acceptance.json,extraction-check.json,host-smoke/result.json}`.

The host remains macOS27.2.1; `minimumOSVerified=false`. The macOS26 gate rejects
this host before app launch. The prior baseline archive retains its original
`4ca71561…7cb24` hash. Packaging provenance now names the historical569-step
app hash separately from the current release. The164 steps repeat those portable
workflow scopes on the new binary; they do not add new Windows, physical gesture,
audible/hardware, Finder/Gatekeeper or actual macOS26 evidence.


### Imported-mark selection completion

The unchanged current release `0d7edc26…` passes94 native steps:64 accepted-subset
and30 Cancel. The first row starts selected/all included; posted Space toggles
only the selected checkbox. All-off disables OK, and actual Return leaves the
sheet and sequence unchanged. Toggling Middle then Zebra commits Zebra/Middle in
source order, with exact IDs, mark times/durations/text verified in saved XML.
Modal Space leaves transport stopped. A separate Cancel process preserves
sequence/input/app hashes and complete rows/dirty state. The generator performs
a real three-collection `.v3m` roundtrip. Independent review and screenshots pass;
failed entry/geometry/composite attempts are excluded. No app change or physical
file-panel/pointer claim. See `docs/review/mark-import-selection-native.md` and
`target/mark-import-native/{accepted-subset,cancelled}/result.json`.

The current same-hash native checkpoint is432 steps:174 Image Maps/Face/rename,
164 transferred-pack startup/playback/media and94 imported-mark selection.
The full Rust result remains3518 passed/0failed/9ignored.

### Returned Windows diagnostic and launcher correction

Historical preparation/partial-return evidence; current v3.2 capture results are recorded at the top.

The user returned the first actual Windows test folder after reporting a stall.
Its four files are preserved unchanged under `target/windows-results/attempt-01`;
hashes and interpretation are in `attempt-01-review.json`. The transcript records
Windows build22631 and PowerShell5.1.22621.4391. A297-byte Framework GDI PNG
confirms that probe reached image output. The first dotnet host lists
Microsoft.NETCore.App6.0.29/8.0.6, not10. No modern-GDI or PCM log, capture
manifest, installed Vixen inventory or final report exists. Those artifacts
locate the reported stall around the first runtime probe; they do not establish
the exact blocking call or a Windows decoder failure.

Review found the launcher was silent after installation selection; Framework
GDI was unbounded in-process, and timeout cleanup used an unbounded wait. The
replacement uses direct.NET Process with separate asynchronous byte-copy tasks,
bounded process/output waits and retained PID ownership during cleanup. Stage
and elapsed messages persist to progress files; Vixen inventory is saved before
child checks. Framework GDI runs in its own bounded child. The original PCMhost,
all managed binaries and diagnostic MP3 are unchanged. The replacement41-file
ZIP is `target/windows-capture-pack-progress-v2-reviewed/Vixen-Windows-Capture.zip`,
4,029,694bytes, SHA256
`7f93636aa9b50d190a83eea486222cb493636754714065bc6dc0d761f6d8f800`.
Independent static/API review and archive integrity pass. The user received the
replacement; v2Windows execution was pending at that checkpoint. No installation or actual PCM
comparison is claimed, and the original archive/results remain retained.

## Windows diagnostic v3 — historical preparation

**Historical checkpoint:** this records the state at that earlier handoff. The v3.2 capture and exact-input comparisons are now complete as described at the top; earlier pending-run statements below are not current blockers.

The reviewed replacement is `target/windows-capture-pack-v3-final/Vixen-Windows-Capture.zip`
(4,063,665 bytes,50 files; SHA256 `3d3e2243cfb18c003966d0510204515d606e49f37b624f2b9c1577e13bb7045d`).
The exact bundled supervisor passes48 local fault checks, including hangs,
nonzero exits, output floods, descendant cleanup, malformed reports, failed
checkpoint writes, overall deadlines, ZIP failure and a blocked console writer.
Thirteen original capture-host checks pass; its rebuilt DLL is byte-identical.
Archive hashes/provenance and PowerShell5.1/Windows Job APIs received independent
review. Evidence: `target/windows-v3-faults-reviewed/result.json`,
`target/windows-capture-pack-v3-root-review.json`.

V3 saves nested installation inventory first and runs each stage independently
with stage and overall deadlines. Compiler errors and partial results are durable;
ZIP failure leaves the original results folder. A bounded private copy of the
identified audio dependency DLLs supports compatibility analysis without another
inventory-only roundtrip. It installs/downloads nothing and does not launch Vixen
or play audio. The strict net10 PCM host is unchanged and will skip incompatible
installs; the known PC's net6/net8 inventory is not itself a compatible capture.
At this historical v3 handoff, Windows execution and PCM were still unverified. Later attempts are recorded above. Run/return instructions
are in `tools/windows-pcm/START-HERE.txt`; earlier archives and both returned
partial attempts remain retained.

### November 11: v3.1 delivered; Preview Save candidate ready

**Historical checkpoint:** this records the state at that earlier handoff. The v3.2 capture and exact-input comparisons are now complete as described at the top; earlier pending-run statements below are not current blockers.

Windows v3.1 is delivered at `target/windows-capture-pack-v3-1-final/Vixen-Windows-Capture.zip` (4,071,750 bytes, 52 files, SHA `540f197cc5a7a1d848219285d71a05058b4f89e8cea8e5528a09c8c00187007e`). It fixes the returned PowerShell checkpoint replacement error with an explicit private backup path; preserves prior reports; isolates unreadable inventory entries and DLL copy errors; includes failed partial copies in the byte budget; separates compiler error encoding; gracefully skips missing inventory dependencies; distinguishes unknown runtime state from verified listings; and continues to input recheck/packaging after optional PCM preflight failure. The capture host, GDI payloads and input MP3 remain byte-identical to v3.

Validation: 79 production-supervisor/local-fault checks (`target/windows-v3-1-faults-reviewed/result.json`), 11 PowerShell-source invariants plus 6 real .NET filesystem cases (`target/windows-worker-v3-1-compat-reviewed/result.json`), 52-entry archive checks, independent source/delta review and root hash comparison. At that checkpoint these local checks did not substitute for PowerShell5/Windows execution of v3.1. The later attempt-04 return verified that run, and attempt-05 verified v3.2 PCM capture. The returned v3 evidence proves its compiler, basic native stage launching/error continuation and archive path ran on Windows, not inventory/PCM compatibility.

Preview follow-up: the guarded private be97 bundle completed84 actions with valid foreground ownership. Settled registry focus is now correct and Preview Command-S no longer changes the sequence; actual Sequence Save and cross-window Undo/Redo remain correct. Preview Save itself still lacked its File item. A controlled one-line change from the `.saveItem` insertion anchor to `.newItem` restored both Save and Import Prop in the combined menu; removing menu reordering did not. Candidate9c matches the retained production source and is prepared for strict foreground acceptance after the pending45-second readiness response. No standard bundle or release pack has yet been refreshed for this source change.

### November 11 completion of Preview Save repair and release integration

The retained one-line Preview menu-anchor correction, settled sequencer focus reconciliation and invocation-time Save guards pass84 strict foreground actions on same-source private debug9c. Every disk checkpoint was observed before closing: Preview shape counts2→1→1→2, Preview saves leave the sequence hash unchanged, and Sequence Save persists its edit without saving the unsaved Preview. Both windows’ Undo/Redo remain isolated. The foreground guard confirms correct ownership and restores Chrome; all owned processes exit. Exact reports: `target/preview-foreground-newitem-acceptance/{result.json,focus-verification.json}`. Preview parity F6 is now done in this scope.

Standard debugc869fe00… and releasefd5c974c… rebuild and pass222-resource/four-executable audits (`target/preview-save-standard-builds.json`). Release passes279 key/workflow/two-editor ownership/modifier steps (`target/preview-save-release-regression.json`). The refreshed portable archive `target/vixen-macos26-verification-pack-preview-save.zip` (37,260,705bytes, SHA426da32f20b7c7eb52ee6c0a7572b4e76918febbeebd4f151afa420a4d971cf8) verifies481 hashes,5 executable modes/signatures and164 additional host startup/playback/media steps with unchanged inputs. Current exact-release native scope is443; the84 foreground proof remains separately qualified by its debug binary hash. Older464-step0d7 results/archives remain intact and are not reattributed. Existing compiler warnings are retained in the build report; Rust did not change and its3,518-test baseline remains valid.

No foreground readiness question is pending. The full goal remains active: the remaining production audio residual, Windows compatibility/GDI, actualmacOS26, remaining physical drags/panels, clean-user/distribution checks and real audio/controllers/remote devices remain outside these passes. The current portable pack correctly refuses to claimmacOS26 verification on this27.2.1 host.
