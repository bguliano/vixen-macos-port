# Windows PowerShell 5.1. Every invocation is independently bounded by a job.
param([Parameter(Mandatory=$true)][ValidateSet('integrity','package')][string]$Stage,
      [Parameter(Mandatory=$true)][string]$Out)
$ErrorActionPreference='Stop'
Set-StrictMode -Version 2
$pack=[IO.Path]::GetFullPath($PSScriptRoot)
$results=[IO.Path]::GetFullPath($Out)
function Local([string]$Path) {
    $full=[IO.Path]::GetFullPath($Path)
    if ($full.StartsWith('\\') -or $full.Contains('"') -or $full.Contains("`n") -or $full.Contains("`r")) {throw 'Ordinary local paths required.'}
    if ((New-Object IO.DriveInfo ([IO.Path]::GetPathRoot($full))).DriveType -eq [IO.DriveType]::Network) {throw 'Network drive excluded.'}
    $cursor=$full
    while ($cursor) {
        if (([IO.File]::Exists($cursor) -or [IO.Directory]::Exists($cursor)) -and (([IO.File]::GetAttributes($cursor) -band [IO.FileAttributes]::ReparsePoint) -ne 0)) {throw 'Linked/cloud path excluded.'}
        $cursor=[IO.Path]::GetDirectoryName($cursor)
    }
    return $full
}
[void](Local $pack);[void](Local $results)
if (-not [IO.Directory]::Exists($results) -or -not [IO.Path]::GetDirectoryName($results).Equals($pack,[StringComparison]::OrdinalIgnoreCase) -or [IO.Path]::GetFileName($results) -notlike 'results-*') {throw 'Expected direct private results folder.'}
if ($Stage -eq 'integrity') {
    $path=Join-Path $pack 'pack-manifest.json'
    if ((Get-Item -LiteralPath $path).Length -gt 1048576) {throw 'Pack manifest exceeds size bound.'}
    $manifest=[IO.File]::ReadAllText($path) | ConvertFrom-Json
    if ($manifest.schema -ne 1 -or @($manifest.files).Count -lt 10 -or @($manifest.files).Count -gt 100) {throw 'Pack manifest shape invalid.'}
    $names=New-Object 'Collections.Generic.HashSet[string]' ([StringComparer]::OrdinalIgnoreCase)
    foreach ($file in $manifest.files) {
        $name=[string]$file.path
        if ($name -notmatch '^[A-Za-z0-9_.-]+(/[A-Za-z0-9_.-]+)*$' -or $name.Split('/') -contains '..' -or -not $names.Add($name)) {throw 'Unexpected/duplicate pack path.'}
        $source=Local (Join-Path $pack ($name.Replace('/','\')))
        if (-not [IO.File]::Exists($source) -or $file.bytes -lt 0 -or $file.bytes -gt 1048576 -or (Get-Item -LiteralPath $source).Length -ne $file.bytes) {throw "Missing/oversized pack file: $name"}
        if ((Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash.ToLowerInvariant() -ne $file.sha256) {throw "Pack checksum mismatch: $name"}
    }
    Write-Output ('Pack integrity passed: '+$names.Count+' files.')
    exit 0
}
# A snapshot of finalized capture files only. The changing supervisor/bootstrap
# logs and current package-stage files are retained in the folder, not raced.
$excluded=@('bootstrap.txt','progress.log','diagnostics.json','package.stage.json','package.stdout.txt','package.stderr.txt','package-result.json')
$pending=New-Object 'Collections.Generic.Stack[string]'
$pending.Push($results)
$files=New-Object 'Collections.Generic.List[object]'
[long]$total=0
while ($pending.Count) {
    $dir=$pending.Pop()
    foreach ($path in [IO.Directory]::EnumerateFileSystemEntries($dir)) {
        [void](Local $path)
        if ([IO.Directory]::Exists($path)) {$pending.Push($path);continue}
        $name=$path.Substring($results.Length+1).Replace('\','/')
        if ($excluded -contains $name -or $name.EndsWith('.tmp')) {continue}
        $length=(Get-Item -LiteralPath $path).Length
        if ($length -gt 2097152 -or $files.Count -ge 1000) {throw 'Return snapshot exceeds a file/count bound; copy the folder instead.'}
        $total+=$length
        if ($total -gt 67108864) {throw 'Return snapshot exceeds64MiB; copy the folder instead.'}
        $files.Add([pscustomobject]@{path=$name;bytes=$length;sha256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant();full=$path})
    }
}
if (-not [IO.File]::Exists((Join-Path $results 'capture-summary.json'))) {throw 'Final capture summary missing; keep the result folder.'}
$records=@($files | Sort-Object path | ForEach-Object {[ordered]@{path=$_.path;bytes=$_.bytes;sha256=$_.sha256}})
$returnManifest=[ordered]@{schema=1;scope='Finalized capture files. Changing bootstrap/progress/package logs remain in the folder. Captured pixels still require Mac comparison.';files=$records}
$manifestBytes=(New-Object Text.UTF8Encoding $false).GetBytes(($returnManifest | ConvertTo-Json -Depth 8))
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip=$results+'.zip';$temporary=$zip+'.tmp'
if ([IO.File]::Exists($zip) -or [IO.File]::Exists($temporary)) {throw 'Archive output already exists.'}
$stream=$null;$archive=$null
try {
    $stream=New-Object IO.FileStream($temporary,[IO.FileMode]::CreateNew,[IO.FileAccess]::ReadWrite,[IO.FileShare]::Read)
    $archive=New-Object IO.Compression.ZipArchive($stream,[IO.Compression.ZipArchiveMode]::Create,$true)
    foreach ($file in ($files | Sort-Object path)) {
        $entry=$archive.CreateEntry($file.path,[IO.Compression.CompressionLevel]::Optimal)
        $input=$null;$output=$null
        try {
            $input=[IO.File]::OpenRead($file.full);$output=$entry.Open();$input.CopyTo($output)
        } finally {if ($output) {$output.Dispose()};if ($input) {$input.Dispose()}}
        if ((Get-FileHash -LiteralPath $file.full -Algorithm SHA256).Hash.ToLowerInvariant() -ne $file.sha256) {throw 'A snapshot file changed during packaging.'}
    }
    $entry=$archive.CreateEntry('return-manifest.json');$entryStream=$entry.Open()
    try {$entryStream.Write($manifestBytes,0,$manifestBytes.Length)} finally {$entryStream.Dispose()}
    $archive.Dispose();$archive=$null;$stream.Dispose();$stream=$null
    [IO.File]::Move($temporary,$zip)
    $record=[ordered]@{status='packaged';zip=$zip;bytes=(Get-Item -LiteralPath $zip).Length;sha256=(Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLowerInvariant();files=$files.Count+1}
    [IO.File]::WriteAllText((Join-Path $results 'package-result.json'),($record | ConvertTo-Json))
    Write-Output ('COPY THIS ZIP BACK: '+$zip)
} finally {if ($archive) {$archive.Dispose()};if ($stream) {$stream.Dispose()}}
