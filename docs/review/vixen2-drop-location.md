# Vixen2 mapping drop location

A physical drop below all six channel rows changed an existing mapping in the baseline. The two table delegates trusted AppKit's proposed row even when the pointer had moved into blank space. The original Windows handler converts the actual pointer position and rejects an empty hit.

Both delegates now convert `draggingLocation` into table coordinates and use `row(at:)` during validation and acceptance. Invalid rows are rejected before any model change. Mapping assignment, grouping, overwrite confirmation, colors and persistence are unchanged.

Release `945941c664fc2796c32c7783b2eb8137f502dab0e6587a2ba0680746f2700865` and debug `604c5ad2871cd0b88998742608dc52833a3197513a5e84001b4a665741978d84` are promoted to the standard local build outputs. Only `Vixen2MappingViews.swift` changed among 808 production inputs since 100105. Both bundles pass resource, executable and signature audits. Previous 100105/66ad bundles are preserved under `target/accepted-bundles-before-vixen2-drop`.

Scoped physical acceptance covers leaf and flattened-group assignments, five actual OS drags including a confirmed blank target, physical Delete, and overwrite Cancel. The fixed app preserves every cell of all six rows after the blank drop. The baseline fails that comparison. These checks ran through normal app controls without UI-test substitutions, using muted private profiles, disabled controllers/web and isolated settings.

The complete Vixen2 color/save/reopen/Quit workflow now passes in full07 on the same release: five physical drags, 11 exact row observations, two persisted checkpoints, and normal Quit. The accepted, cancelled and final ModuleStore hashes are all `44b4b2c235e03e11a14d868c26b129221364559de656cb72ab6f6c3b42981205`. All 40 unrelated Curve primitive type names retain their XML Schema identity, and source files, owner preferences and desktop/input state remain unchanged. Independent acceptance passes 14 checks; complementary review reruns the complete consumer assertions.

Earlier attempts remain historical. The retained full03 and full04 reports are failures of that larger test: full03 stopped on the test's native-alert geometry assumption; full04 proceeded through overwrite Cancel and RGB editing, then stopped on an unavailable accessibility reference after Color-panel close. They have not been rewritten as full passes. Full06 subsequently stopped because the test fixture had stripped namespace declarations referenced only by xsi:type values. The final preparation preserves the source bytes, and a namespace-aware comparison rejects unbound or changed type names. No production change was needed for these test corrections. The drop handlers were promoted on their focused evidence before full07 completed.

Evidence:

- `target/vixen2-full-physical-02/result.json`: actual baseline blank-drop failure.
- `target/vixen2-drop-location-fix/independent-source-review.json`: exact two-handler delta and pinned Windows contract.
- `target/vixen2-full-physical-03/independent-partial-blank-drop-proof.json`: fixed physical blank-drop proof.
- `target/vixen2-drop-integration/scoped-drop-acceptance-and-promotion-readiness.json`: scoped acceptance, both audits, source and bundle hashes.
- `target/vixen2-drop-integration/promotion.json`: promoted bytes/modes/signatures and preserved prior outputs.
- `target/vixen2-full-physical-07/acceptance.json` and `complementary-acceptance.json`: complete normal-app workflow, persisted XML and protected-state verification.

No broad media or Rust suite was repeated: those production inputs are unchanged. The refreshed 945941 portable archive passes 482 transferred file hashes and five executable modes/signatures after fresh extraction (`target/second-mac-pack-vixen2-drop-transfer-verification.json`). The prior 97498 portable host run and 100105 transfer verification retain their original executable scopes. Actual macOS 26 and other external gates remain open.
