# Eight-case Bitmap storage supplement

Prepared 2026-11-13 for the same future Windows batch as the immutable 80-case raster-detail kit. No Windows run, delivery, renderer change, or pixel-parity acceptance has occurred.

The supplement is `target/windows-bitmap-storage-pack-01/Vixen-Windows-Bitmap-Storage-01.zip` (58,411 bytes; SHA256 `d1edcb51739545ce9f8f4b305046540602430f5df651ac29045bfd11968067aa`). Its 22 archive members include source, exact case/observer contract, provenance, original Picture source/license, and local-check evidence. No executable or runtime is bundled.

`tools/windows-bitmap-storage/` implements the eight scenarios specified in `target/bitmap-storage-design-01/DESIGN.md`: Graphics ownership alone; partial opaque/alpha0/alpha1 fills; partial DrawImage; two Clears; antialiased fill plus separate coverage diagnostic; and original grayscale output used directly versus reconstructed through GetPixel/SetPixel before a later DrawImage. The six exact ARGB seeds and literal operations are in the source and README. The 3×5 grayscale input hash is `01d6b1418332b41afd7b0780cf81d4ec7c48bc9e00480984deddd2b07ca7d2d8`, identical to the retained 74-case Windows input.

The observer order is part of the contract. Paired observations use GetPixel, then **ReadOnly** LockBits, with no copy-back. Direct grayscale source A and reconstructed C have only GetPixel before the later draw; their LockBits observations occur afterward. Actual drawing settings, signed stride, raw ARGB buffers and hashes are retained. The coverage image is diagnostic and is not treated as a proven write mask. Complete capture means well-formed observations; seed mismatch flags and cross-API disagreements remain visible. No expected output math, tolerance or image fitting is applied.

Local evidence: `target/windows-bitmap-storage-prepare-01/local-checks.json` records 38 checks, including C#5 compilation with cached net10 references, compiled case/input-digest checks, and 23 synthetic complete/partial consumer controls. No Windows drawing, Framework compiler, PowerShell or Windows Job execution was performed on this Mac. Every .NET build set `DOTNET_GENERATE_ASPNET_CERTIFICATE=false`. Root independently accepted the eight scoped review findings and 18 source pins in `root-independent-readiness.json` in that folder.

The exact existing suspended-start Job/KILL_ON_JOB_CLOSE engine, integrity/partial-archive worker and compiler worker are reused. Each case is independently bounded to five seconds; the supervisor has a 180-second budget plus a separate 210-second process deadline, with outer bootstrap/watchdog bounds. Partial bytes and per-stage diagnostics survive a case failure. `Watch-Raster.ps1` is the Windows entry point; execution remains pending the shared Windows batch.

Validate a returned ZIP or directory with:

```sh
python3 tools/windows-bitmap-storage/validate_return.py \
  --windows '/path/to/returned-results.zip' \
  --out target/windows-bitmap-storage-results-01.json
```

The validator requires exactly eight ordered outcomes, complete-case observer order, provenance, file hashes, stages and actual Windows runtime metadata. Partial or malformed returns never become parity success. The original 80-case ZIP remains SHA256 `25ce97652ea7a971d361cada439ae8003ccaac8ba2ba281ec8541675f1fdd505`, and its source/consumer were not changed. Current 811 production inputs remain unchanged. Use the new observations to decide the pending straight-storage/write policy before implementing any global Bitmap representation change.
