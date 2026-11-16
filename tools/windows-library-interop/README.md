# Installed Windows library interoperability

Preparation only. This closes a different gap from the earlier local rebuilt-.NET
contract tests: Mac-generated library files must be accepted by the user's actual
Windows Vixen **3.12.4** types, reserialized there, and read back by the Mac.
It does not verify library dialogs, duplicate-name handling, profiles, controllers,
or the newer pinned upstream 3.13.4 application.

The installed product commit is `41c3f4f7e3ea13e8b21648a7398753fb60ad9a5a`.
Exact source and its download hashes are retained under
`target/windows-library-interop-prepare-01/source`. Both original library forms
use text XML `DataContractSerializer(Dictionary<string, Curve/ColorGradient>)`.

The fixtures contain four curves and four gradients: interpolation/clamping,
ties-to-even integer values, color/alpha interpolation, nondefault focus, gamma
and current-library flags, titles, Unicode/escaped names. Every library reference
is empty. Independent arithmetic supplies **25 curve + 22 gradient samples**.
The generator calls the production `vixen_session::sequencer::libraries`
write/read APIs and validates complete typed readback. Its isolated Cargo target
is `target/windows-library-interop-build`; no app or profile is initialized.

The first focus expectation incorrectly assumed byte truncation at the inserted
focus point. Its failed fixture/log remain preserved. Exact original `XYZ`
clipping and `RGB.ToArgb` ties-to-even rounding independently yield `(128,128,127)`
at that point; the subsequent float interpolation truncates. This derivation is
retained as `independent-focus-math.json`. Final fixtures are `fixtures-02`.

## Run sequence, owned by the root agent

1. After authenticated SSH is available, upload this small folder and final
   fixtures. Run `Collect-LibraryAssemblies.ps1 -VixenBin <actual-install>
   -Output <fresh-private-folder>`. It reads/copies only the four required data
   assemblies and up to five named optional dependencies, records exact product
   versions/hashes, and verifies the installation bytes stayed unchanged. No
   original assembly is loaded or executed.
2. Return that inventory and private DLL copies. Run `audit_collection.py` using
   the existing metadata-only host from `tools/windows-pcm/audit-metadata`.
   It rejects global module methods, records exact API IL and dependency lists,
   and never loads supplied DLLs. Review the actual IL against the retained
   source-path audit, including serialization callbacks and the empty-reference
   branch. **Do not run the Windows evaluator before that audit.**
3. Only after review, create an `approved-contract.json` with schema1,
   status `approved-metadata-and-il-audit`, exact productCommit above,
   `moduleInitializersAbsent:true`, the returned inventory SHA256, and `files`
   entries `{name,sha256}` matching the complete private folder. Its `packFiles`
   must pin all seven inputs: host LibraryCapture.cs, runtimeconfig JSON,
   Run-LibraryCapture.ps1; fixture Mac.vcl, Mac.vgl, fixture-pins.tsv, expected.json.
   Entries have `{kind:"host"|"fixture",name,sha256}`. No approved contract is
   included in the prepared pack because actual DLLs have not returned yet.
4. Run `Run-LibraryCapture.ps1` with the collection, approved contract, fixtures,
   a fresh output folder, and the existing x64 dotnet path. It uses installed
   Framework csc and existing **.NET8 Desktop**, with no SDK/runtime installation.
   Compile and capture each have a30-second bound; child cleanup is bounded.
   Output is inherited directly, so no redirected-stream drain can stall.
   It invokes only the exact data types, DCS, and evaluator methods on private
   copies; original installation hashes are rechecked before and after.
5. Return the output and use `consume.py`. It requires every exact sample,
   original serialize/reload equality, binary/fixture pins, and successful
   complete typed Mac readback through the compiled production serializer reader.
   A successful capture alone remains `captured-validation-pending`.

No Windows commands have been executed by this preparation. Local evidence:
C#5 language syntax build against the existing .NET10 reference pack (not a
Windows Framework compile claim), help/argument/platform-refusal checks, nine
strict consumer corruption controls, and actual production Mac fixture readback.
`--readback` never writes to the returned capture folder.
