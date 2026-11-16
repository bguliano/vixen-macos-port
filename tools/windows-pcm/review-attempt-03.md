# Returned Windows v3 run

The returned ZIP is safely retained as
`target/windows-results/attempt-03-returned.zip`, SHA256
`b37dfdb7442cb27ebe705bb5e6f583324a8834ff0cf941e7667737cc3abe4002`.
Its25 entries total7,673 uncompressed bytes. Every path was checked for traversal,
links and duplication before extraction to the fresh `attempt-03` folder. Member
hashes are in `attempt-03-manifest.json`; findings are in `attempt-03-review.json`.
No supplied Windows binary was executed.

**PCM did not run and no audio DLLs were copied.** The installed Framework compiler
completed successfully. The native supervisor launched workers, retained failures,
continued independent stages and published a ZIP within a few seconds. Inventory
failed after0.413seconds at its second checkpoint:

> The path is not of a legal form.

The failing call was PowerShell `[IO.File]::Replace($temporary,$path,$null)`.
The existing current checkpoint and the other valid paths narrow this to the
backup-string argument. PowerShell provides a special `NullString` specifically
to pass a real null to a .NET string parameter; an ordinary null can bind as an
empty string. `File.Replace` accepts a real null backup but rejects an empty path.
This is a supported diagnosis of the binding failure, not evidence that the
prior v1/v2 silent waits had the same cause. See Microsoft's
[NullString documentation](https://learn.microsoft.com/en-us/dotnet/api/system.management.automation.language.nullstring)
and [File.Replace contract](https://learn.microsoft.com/en-us/dotnet/api/system.io.file.replace?view=netframework-4.8.1).

Reference-copy then failed because inventory-choice.json did not exist. The
supervisor still completed pack integrity, Framework GDI and final input hashing.
The exact MP3 hash remained unchanged. `skipped-no-net10` in this returned report
is misleading: no runtime inventory ran here. The previous returned runtime list
showed6/8, but this v3 archive itself cannot establish installed runtime versions.

The archive contains its designed pre-archive diagnostics checkpoint. Its
`package:pending` and incomplete bootstrap footer are not evidence of a new stall:
the archive was published successfully. Final supervisor/bootstrap exit and the
package-stage record remain outside the returned ZIP. This proves the ordinary
Windows compiler/worker/log/archive path; no timeout or descendant-cleanup fault
was exercised on Windows.

## Corrections in v3.1

PowerShell Save uses a nonempty private `.previous` backup path, avoiding nullable
string binding and preserving the prior complete checkpoint. The first checkpoint
still uses a two-string Move. C#'s typed-null File.Replace was audited separately;
that actual Windows path already worked for repeated supervisor diagnostics.
XML resolver nulls are object-property assignments, not nullable string calls.

Missing/incomplete inventory now skips dependent reference copying explicitly.
Runtime status distinguishes unavailable inventory, unavailable runtime probes,
no discovered host and no verified.NET10 host. A malformed choice or a vanished
optional PCM directory does not prevent the remaining diagnostics and ZIP.
Individual inaccessible inventory entries/copy sources no longer prevent later
entries. Partial copies count toward the64MiB copy budget. Compiler exception
text uses a separate file, avoiding mixed UTF16/UTF8 with PS5 native redirection.

No local PowerShell executable or cached PowerShell runtime was found. Therefore
`check_worker_compat.py` explicitly performs11 static checks of the production
Save/worker source and six actual C# filesystem tests of the same explicit-backup
operation: first write, repeated replacement, backup rotation, empty-path refusal,
failure preservation and recovery. Its report is
`target/windows-worker-v3-1-compat-reviewed/result.json`. These are not claimed as
PowerShell5.1 execution. The returned user run remains the real Windows evidence.

The compatibility next step is still obtaining the bounded nested-install inventory
and verified private audio DLL copies. No actual3.12.4 AudioPlayer binary has yet
been returned, so changing the strict capture host's runtime/dependency contract
would be premature.
