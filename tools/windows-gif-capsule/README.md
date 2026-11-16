# Eight-case GIF comparison capsule

Prepared for a later bounded Windows SSH run. It does not launch Vixen, load
installed Vixen assemblies, open a show, initialize controllers, or install anything.

Four explicit 4x4 GIF89a streams exercise full opaque frames and transparent partial
frames with Keep, Background, and Previous disposal. Every stream has unequal
1/7/13/2-centisecond delays and an infinite-loop extension. Two Picture speeds
(1 and 3), a600ms effect and50ms frame interval give eight cases. Speed3 includes
backward frame selection at loop boundaries. Inputs are generated directly from
palette indices; their exact bytes are pinned.

Windows `GifCapture` reads every sequential GDI+ decoded frame separately. A second
fresh image follows the **source-derived** original Picture schedule and calls an
unchanged copy of `Picture.ScaleImage` at100%, retaining its GDI+ DrawImage call.
The source is commit41c3f4f7e3ea13e8b21648a7398753fb60ad9a5a, corresponding to the
user's3.12.4 installation. This is not execution of the installed effect.

Actual installed `Picture.SetupRender` remains gated on returned DLL metadata/IL:
base-class NLog initializers, PictureData's ancestry, and TypeDescriptor providers
must be audited first. The constructor reads VixenSystem settings. This capsule
therefore makes **no installed-effect, profile, GUI or full effect-output claim**.
It also records the Windows Framework runtime used; it does not silently equate
that managed runtime with the installed Vixen .NET8Desktop runtime.

The Mac capture copies the production graphics crate, keeps its original bytes,
and appends a private observer. It calls the production `decode_frames` and actual
`Picture.setup_render`, recording cache bytes before rendering consumes them. The
observer's render callback is empty, so full rendered-effect output is excluded.
All production input hashes are checked unchanged. No production files are edited.

Preparation evidence is in `target/windows-gif-prepare-01`. The initial adapter
compile failure (wrong enum variant) remains under `mac`; corrected `mac-02`
compiles and captures8cases/32decodedframes/44cachedframes. Local C#5 compilation
and synthetic strict-comparator controls do not constitute a Windows run.

After the one scoped review, root may transfer the small prepared folder and run:

```powershell
powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File .\Run-Gif.ps1 -Output C:\path\to\new-gif-results
```

Use an ordinary local folder outside the Vixen install/show. The parent of Output
must already exist; Output itself must be new. The script uses the existing x64
Framework compiler, bounds compilation at30s and each case at5s, reserves cleanup
within90s, retains partial cases and never redirects child pipes. Copy the result
folder back; root compares separately decoded and drawn-cache ARGB bytes with
`compare.py`. Every channel, including transparent alpha/RGB, is checked exactly.
Unresolved differences remain failures; no channel mask or numerical tolerance.
