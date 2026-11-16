# Eight supplemental Bitmap storage probes (schema 1)

Preparation only: no Windows execution or raster parity claim. Run in the same future batch as the unchanged 80-case detail kit. This separate source-only capsule does not load Vixen, a show, controllers, profiles, or private files; it uses the existing Windows Framework compiler/System.Drawing. Do not install anything or change security settings to run it.

`Watch-Raster.ps1` bounds bootstrap to five minutes. The reused supervisor assigns each child to a suspended-start Windows Job with KILL_ON_JOB_CLOSE before resuming, closes the Job even after leader exit, caps each stream to 1 MiB, and preserves independent case failures. Eight case processes have five-second limits; the supervisor has a 180-second total budget and a separate 210-second hard process deadline. Completed/partial raw buffers and return ZIP remain in the owned result directory.

Every raw file is ARGB8, row-major, top-down, with no tolerance, pixel fitting or image normalization. Seed every row of the 6x3 destination using SetPixel:
`[0,17,43,89] [1,88,72,202] [127,159,101,59] [128,48,140,108] [254,177,177,177] [255,190,198,78]`.

The cases observe ownership only; partial opaque/zero-alpha/alpha1 white fill; explicit 1:1 source/destination DrawImage rectangles; two Clear operations; an antialiased fractional fill and separate white coverage diagnostic; and direct original grayscale output versus GetPixel/SetPixel reconstructed source on a later integer-overload 1:1 DrawImage. Geometry/arguments are literal in RasterCapture.cs.in and event/observer ordering is fixed in cases.json.

All paired observations call GetPixel first, then LockBits(ReadOnly, Format32bppArgb), converting BGRA bytes into ARGB without copying anything back. Direct grayscale source A has only GetPixel before its subsequent draw: its first LockBits is after that draw. Reconstructed C similarly has only GetPixel before drawing. B/C exact raw values are retained; readback disagreements are observations, never silently normalized. Coverage diagnostic is not assumed to be an exact write mask.

Original ConvertToGrayScale is copied byte-for-byte from the already pinned upstream source helper. This is an original helper plus Framework primitive experiment, not an installed Vixen/full-effect test. The 3x5 input is exactly the previously captured74-case alpha-grayscale input; its hash is checked during local preparation.

Use `validate_return.py` against a returned result ZIP/folder. It requires exactly eight outcomes and strict complete-case event order/provenance/files, while preserving partial evidence as incomplete. It never calls a renderer or declares pixel parity. Root must schedule Windows execution through the reviewed owned-process batch; this preparation does not deliver or launch it.
