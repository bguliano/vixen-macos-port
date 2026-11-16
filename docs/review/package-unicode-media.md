# Unicode media in sequence packages

Sequence package export now includes media whose filename uses a different canonical Unicode spelling from the unchanged TIM reference. The reproduced failure used NFC `Café.wav` / `Étoile.png` references and NFD disk names. This Mac resolved both spellings to the same inode, and a real sequence session decoded the audio, but the old export omitted both assets. A second case omitted wildcard matches; a third included only the NFD files and lost the NFC aliases required by the original references.

This is a Mac portability fix. [Apple's APFS documentation](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/APFS_Guide/FAQ/FAQ.html) describes normalization-insensitive lookup. It does not establish equivalent behavior on Windows. The original exporter in `upstream/Vixen/src/Vixen.Modules/App/TimedSequenceMapper/SequencePackageExport/SequencePackageExportSummaryStage.cs` discovers assets through `Directory.GetFiles` patterns and retains raw sequence bytes.

The Rust exporter now matches lowercased filenames in NFC. It preserves ASCII case matching and wildcard syntax; it neither strips accents nor applies compatibility normalization. Literal references receive their exact archive basename when canonical spelling differs. Case-only matches and wildcard matches keep the disk basename. Separate source paths and archive names preserve multiple literal aliases of one file, deduplicate repeated aliases, and reject different sources claiming the same archive path. Audio excluded by export settings is not discovered or validated. Existing path restrictions and atomic package replacement remain in place. The raw TIM is unchanged.

## Verification

- `target/package-unicode-reproduction-01/baseline-evidence.json` preserves the actual failing baseline, original source, all three ZIPs, and exact raw TIM payloads. `target/package-unicode-independent-review-01/baseline.json` independently confirms the omission.
- `target/package-unicode-reproduction-01/result.json` pins the fixed source and artifacts. One integration test covers literal references, wildcard discovery, and multiple canonical aliases for both audio and Picture assets. Each case performs real production export and import into a separate private profile. Exact ZIP names/bytes, retained references, remapped targets, and extracted payloads pass. Literal/alias sessions decode all eight expected audio frames before and after import; Picture pixels match. Wildcard playback is not claimed.
- `fixed-unit-tests.log` records **17 passing package tests**: 13 existing and four new matching, alias, collision, and excluded-audio tests. Accent removal and compatibility folding are explicitly rejected.
- `target/package-unicode-independent-review-01/final-readback.json` independently accepts 30 source/artifact checks: all 47 pins, exact ZIP/imported payloads and XML, dependency lock delta, seven original license texts, and temporary-directory cleanup. No test rerun was required.
- `default-integration.log` confirms the normal test invocation works without `VIXEN_PACKAGE_UNICODE_REPORT_ROOT` and removes its owned temporary directory. An explicit report override retains diagnostic artifacts. The filesystem integration is macOS-only.

No native window, audio device, controller, or external network service was started by these tests. No Windows Vixen execution is claimed; that interoperability gate remains open.

## Dependencies and build scope

`unicode-normalization` is pinned to **0.1.26**; Cargo.lock adds only that package and **tinyvec 1.13.3**, with no upgrades or removals. This tinyvec version has no separate `tinyvec_macros` dependency; its compatibility feature of that name is empty. `lock-delta.json` records the exact comparison against the preserved original lockfile.

`app/Resources/Licenses/unicode-normalization.txt` contains the unmodified registry-source notices for both packages, including Unicode data licensing. `dependency-license-provenance.json` records their source checksums and notice hashes; `docs/LICENSES.md` lists the licenses. Upstream Vixen's license is unchanged.

The integrated build tracks **811 inputs**: the prior 808 plus the previously untracked session dependency manifest, this new integration test, and the new bundled notice. Complete app build results and physical Package verification are separate evidence; these focused tests do not claim either.
