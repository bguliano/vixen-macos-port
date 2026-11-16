# Windows PowerShell 5.1 stage worker. Every invocation is a separate supervised Job.
[CmdletBinding()]
param([Parameter(Mandatory=$true)][ValidateSet('inventory','reference-copy','integrity','framework-gdi','legacy-prepare','input-recheck','package')][string]$Stage,
      [Parameter(Mandatory=$true)][string]$Out, [AllowEmptyString()][string]$VixenRoot = '')
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2
$pack = [IO.Path]::GetFullPath($PSScriptRoot)
function Local([string]$Path) {
    $full = [IO.Path]::GetFullPath($Path)
    if ($full.StartsWith('\\') -or $full.Contains('"') -or $full.Contains("`n") -or $full.Contains("`r")) { throw 'Only ordinary local paths are supported.' }
    if ((New-Object IO.DriveInfo ([IO.Path]::GetPathRoot($full))).DriveType -eq [IO.DriveType]::Network) { throw 'Network paths are excluded.' }
    $cursor = $full
    while ($cursor) {
        if ([IO.File]::Exists($cursor) -or [IO.Directory]::Exists($cursor)) {
            if (([IO.File]::GetAttributes($cursor) -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw "Linked/cloud path excluded: $cursor" }
        }
        $cursor = [IO.Path]::GetDirectoryName($cursor)
    }
    return $full
}
function Within([string]$Path, [string]$Root) {
    return $Path.Equals($Root, [StringComparison]::OrdinalIgnoreCase) -or $Path.StartsWith($Root.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase)
}
function Save($Value, [string]$Name) {
    $path = Join-Path $Out $Name; $temporary = $path + '.tmp'
    $Value | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $temporary -Encoding UTF8
    if ([IO.File]::Exists($path)) {
        # PS5 converts ordinary $null arguments to an empty .NET string here.
        # A real private backup path avoids that binding and preserves the prior checkpoint.
        $previous = $path + '.previous'
        if ([IO.File]::Exists($previous)) { [IO.File]::Delete($previous) }
        [IO.File]::Replace($temporary, $path, $previous)
    } else { [IO.File]::Move($temporary, $path) }
}
function Say([string]$Text) { Write-Host (([DateTime]::UtcNow.ToString('o')) + ' ' + $Text) }
$Out = Local $Out
if (-not [IO.Directory]::Exists($Out) -or -not [IO.Path]::GetDirectoryName($Out).Equals($pack, [StringComparison]::OrdinalIgnoreCase) -or [IO.Path]::GetFileName($Out) -notlike 'results-*') { throw 'Expected an existing direct results-* folder inside this pack.' }
if ($VixenRoot) { $VixenRoot = Local $VixenRoot; if (Within $pack $VixenRoot) { throw 'Pack must be outside the Vixen installation.' } }
Say "Starting isolated $Stage worker; output: $Out"

if ($Stage -eq 'inventory') {
    $data = [ordered]@{ schema = 1; status = 'partial'; utc = [DateTime]::UtcNow.ToString('o'); os = [Environment]::OSVersion.VersionString;
        powershell = $PSVersionTable.PSVersion.ToString(); process64Bit = [Environment]::Is64BitProcess;
        registry = @(); roots = @(); directories = @(); files = @(); hosts = @(); errors = @();
        limits = [ordered]@{ depth = 3; roots = 8; directories = 128; entries = 8192; metadataFiles = 256; hashBytes = 67108864; runtimeConfigBytes = 65536 } }
    Save $data 'inventory.json'
    $roots = New-Object 'System.Collections.Generic.List[string]'
    function AddRoot([string]$Path) {
        if (-not $Path -or $roots.Count -ge 8) { return }
        try {
            $candidate = Local $Path.Trim().Trim('"')
            if ([IO.File]::Exists($candidate)) { $candidate = [IO.Path]::GetDirectoryName($candidate) }
            if (-not [IO.Directory]::Exists($candidate)) { return }
            if (Within $pack $candidate) { throw 'Install candidate contains this diagnostic pack; excluded.' }
            # Do not turn a missing installation hint into a whole-drive/program-folder scan.
            if ([IO.Path]::GetPathRoot($candidate).TrimEnd('\') -eq $candidate.TrimEnd('\')) { throw 'Whole-drive scanning is excluded.' }
            foreach ($programRoot in @($env:ProgramFiles, ${env:ProgramFiles(x86)}, $env:ProgramW6432, $env:LOCALAPPDATA)) {
                if ($programRoot -and $candidate.TrimEnd('\').Equals($programRoot.TrimEnd('\'), [StringComparison]::OrdinalIgnoreCase)) { throw 'Whole program/user folder scanning is excluded.' }
            }
            if (-not $roots.Contains($candidate)) { $roots.Add($candidate) }
        } catch { $data.errors += ('Root excluded: ' + $_.Exception.Message) }
    }
    if ($VixenRoot) { AddRoot $VixenRoot }
    foreach ($programRoot in @($env:ProgramFiles, ${env:ProgramFiles(x86)}, $env:ProgramW6432)) {
        if ($programRoot) { AddRoot (Join-Path $programRoot 'Vixen Lights') }
    }
    # Registry values only, no WMI Win32_Product, uninstall commands, or app launches.
    foreach ($key in @('HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*', 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*', 'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*')) {
        try {
            foreach ($item in @(Get-ItemProperty $key -ErrorAction SilentlyContinue)) {
                if (-not $item.PSObject.Properties['DisplayName'] -or $item.DisplayName -notmatch 'Vixen') { continue }
                $row = [ordered]@{ name = [string]$item.DisplayName; version = ''; installLocation = ''; displayIcon = '' }
                foreach ($mapping in @(@('DisplayVersion','version'), @('InstallLocation','installLocation'), @('DisplayIcon','displayIcon'))) {
                    if ($item.PSObject.Properties[$mapping[0]]) { $row[$mapping[1]] = [string]$item.($mapping[0]) }
                }
                $data.registry += $row
                if ($row.installLocation) { AddRoot $row.installLocation }
                # Cached bootstrapper icons are version evidence, not application roots.
                if ($row.displayIcon -and $row.displayIcon -notmatch '(?i)\\Package Cache\\') { AddRoot (($row.displayIcon -replace ',\s*-?\d+$', '').Trim('"')) }
                if ($data.registry.Count -ge 32) { break }
            }
        } catch { $data.errors += ('Registry: ' + $_.Exception.Message) }
        Save $data 'inventory.json'
    }
    if ($env:LOCALAPPDATA) { AddRoot (Join-Path $env:LOCALAPPDATA 'Programs\Vixen Lights'); AddRoot (Join-Path $env:LOCALAPPDATA 'Programs\Vixen') }
    $data.roots = @($roots.ToArray()); Save $data 'inventory.json'
    $queue = New-Object 'System.Collections.Generic.Queue[object]'
    foreach ($root in $roots) { $queue.Enqueue(@{ path = $root; depth = 0 }) }
    $visited = New-Object 'System.Collections.Generic.HashSet[string]' ([StringComparer]::OrdinalIgnoreCase)
    $entries = 0; [long]$hashedBytes = 0
    $names = @('AudioPlayer.dll','NAudio.dll','NAudio.Core.dll','NAudio.Wasapi.dll','NAudio.WinMM.dll','NAudio.Vorbis.dll','NVorbis.dll','NLog.dll','dotnet.exe','hostfxr.dll','hostpolicy.dll','coreclr.dll','System.Private.CoreLib.dll')
    while ($queue.Count -gt 0 -and $data.directories.Count -lt 128 -and $entries -lt 8192 -and $data.files.Count -lt 256) {
        $next = $queue.Dequeue(); $directory = [string]$next.path
        if (-not $visited.Add($directory)) { continue }
        try {
            [void](Local $directory)
            Say "Inspecting directory (depth $($next.depth)): $directory"
            $data.directories += $directory; Save $data 'inventory.json'
            foreach ($entry in [IO.Directory]::EnumerateFileSystemEntries($directory)) {
                $entries++; if ($entries -gt 8192 -or $data.files.Count -ge 256) { break }
                try {
                $attributes = [IO.File]::GetAttributes($entry)
                if (($attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { $data.errors += "Skipped link/cloud entry: $entry"; continue }
                if (($attributes -band [IO.FileAttributes]::Directory) -ne 0) {
                    $leaf = [IO.Path]::GetFileName($entry)
                    if ($next.depth -lt 3 -and $queue.Count -lt 128 -and $leaf -notin @('SystemData','Sequences','Media','Logs','VideoCache','Cache','Profiles','Preview')) { $queue.Enqueue(@{ path = $entry; depth = ($next.depth + 1) }) }
                    continue
                }
                $name = [IO.Path]::GetFileName($entry)
                if ($name -notin $names -and $name -notlike '*Vixen*.exe' -and $name -notlike '*Vixen*.dll' -and $name -notlike '*.runtimeconfig.json' -and $name -notlike '*.exe.config') { continue }
                [void](Local $entry)
                $file = New-Object IO.FileInfo $entry
                $row = [ordered]@{ path = $entry; name = $name; bytes = $file.Length; fileVersion = ''; productVersion = ''; assemblyIdentity = ''; sha256 = ''; error = '' }
                try {
                    if ($name -like '*.dll' -or $name -like '*.exe') {
                        $version = [Diagnostics.FileVersionInfo]::GetVersionInfo($entry)
                        $row.fileVersion = $version.FileVersion; $row.productVersion = $version.ProductVersion
                        try { $row.assemblyIdentity = [Reflection.AssemblyName]::GetAssemblyName($entry).FullName } catch { $row.error = 'No managed assembly identity: ' + $_.Exception.Message }
                    }
                    if ($name -like '*.runtimeconfig.json' -and $file.Length -le 65536) {
                        $row.runtimeConfiguration = [IO.File]::ReadAllText($entry) | ConvertFrom-Json
                    }
                    if ($name -like '*.exe.config' -and $file.Length -le 65536) {
                        # Retain only runtime declarations, never application settings/user configuration.
                        $settings = New-Object Xml.XmlReaderSettings
                        $settings.DtdProcessing = [Xml.DtdProcessing]::Prohibit; $settings.XmlResolver = $null
                        $reader = [Xml.XmlReader]::Create($entry, $settings)
                        try {
                            $xml = New-Object Xml.XmlDocument; $xml.XmlResolver = $null; $xml.Load($reader)
                            $row.supportedRuntimes = @($xml.SelectNodes('/configuration/startup/supportedRuntime') | ForEach-Object { [ordered]@{ version = $_.GetAttribute('version'); sku = $_.GetAttribute('sku') } })
                        } finally { $reader.Dispose() }
                    }
                    if ($file.Length -le 16777216 -and $hashedBytes + $file.Length -le 67108864) {
                        $row.sha256 = (Get-FileHash -LiteralPath $entry -Algorithm SHA256).Hash.ToLowerInvariant(); $hashedBytes += $file.Length
                    }
                } catch { $row.error += ' Metadata: ' + $_.Exception.Message }
                $data.files += $row
                Save $data 'inventory.json'
                } catch { $data.errors += ('Entry ' + $entry + ': ' + $_.Exception.Message); Save $data 'inventory.json' }
            }
        } catch { $data.errors += ('Directory: ' + $_.Exception.Message); Save $data 'inventory.json' }
    }
    $data.entriesVisited = $entries; $data.hashBytes = $hashedBytes
    $data.truncated = ($queue.Count -gt 0 -or $entries -ge 8192 -or $data.files.Count -ge 256)
    $bins = @($data.files | Where-Object { $_.name -eq 'AudioPlayer.dll' } | ForEach-Object { [IO.Path]::GetDirectoryName($_.path) } | Select-Object -Unique)
    $selected = ''; $ready = $false; $reason = ''
    if ($bins.Count -eq 1) { $selected = $bins[0] }
    elseif ($bins.Count -gt 1) { $reason = 'Multiple AudioPlayer directories found; optional PCM skipped rather than choosing a different install.' }
    else { $reason = 'No AudioPlayer.dll found in the bounded installation scan; inventory is retained for compatibility work.' }
    if ($selected) {
        $ready = $true
        foreach ($name in @('AudioPlayer.dll','NAudio.Core.dll','NAudio.Wasapi.dll','NAudio.WinMM.dll','NAudio.Vorbis.dll','NVorbis.dll','NLog.dll')) {
            $match = @($data.files | Where-Object { $_.name -eq $name -and [IO.Path]::GetDirectoryName($_.path) -eq $selected })
            if ($match.Count -ne 1) { $ready = $false; $reason += " Missing $name;" }
            elseif ($name -in @('NAudio.Core.dll','NAudio.Wasapi.dll','NAudio.WinMM.dll') -and $match[0].assemblyIdentity -notmatch ', Version=2\.3\.0\.0,') { $ready = $false; $reason += " $name is not the required 2.3.0.0;" }
        }
    }
    $hosts = New-Object 'System.Collections.Generic.List[string]'
    $hostCandidates = New-Object 'System.Collections.Generic.List[string]'
    foreach ($base in @($env:ProgramFiles, ${env:ProgramFiles(x86)}, $env:ProgramW6432, $selected)) {
        if ($base) { $hostCandidates.Add((Join-Path $base 'dotnet\dotnet.exe')); $hostCandidates.Add((Join-Path $base 'dotnet.exe')) }
    }
    foreach ($file in $data.files) { if ($file.name -eq 'dotnet.exe') { $hostCandidates.Add($file.path) } }
    # PATH is inspected as local filesystem entries only; no command is executed here.
    foreach ($entry in ($env:PATH -split ';' | Select-Object -First 64)) {
        if ($entry -and [IO.Path]::IsPathRooted($entry)) { $hostCandidates.Add((Join-Path $entry 'dotnet.exe')) }
    }
    foreach ($candidate in $hostCandidates) {
        if ($hosts.Count -ge 8) { break }
        try { $full = Local $candidate; if ([IO.File]::Exists($full) -and -not $hosts.Contains($full)) { $hosts.Add($full) } } catch { $data.errors += ('Host excluded: ' + $_.Exception.Message) }
    }
    $data.hosts = @($hosts.ToArray()); $data.status = 'complete'; Save $data 'inventory.json'
    Save ([ordered]@{ selectedBin = $selected; assemblyReady = $ready; hosts = @($hosts.ToArray()); reason = $reason }) 'inventory-choice.json'
    Say "Inventory saved. Selected audio directory: $selected. Compatible with strict capture: $ready. $reason"
    exit 0
}

if ($Stage -eq 'reference-copy') {
    $choicePath = Join-Path $Out 'inventory-choice.json'
    $inventoryPath = Join-Path $Out 'inventory.json'
    $copyReport = [ordered]@{ status = 'partial'; scope = 'Private compatibility audit only; these assemblies are never executed by the inventory/copy stage.'; sourceDirectory = ''; maxFiles = 16; maxBytes = 67108864; files = @(); errors = @() }
    Save $copyReport 'reference-copy.json'
    if (-not [IO.File]::Exists($choicePath) -or -not [IO.File]::Exists($inventoryPath)) {
        $copyReport.status = 'skipped-inventory-unavailable'; Save $copyReport 'reference-copy.json'; exit 0
    }
    if ((Get-Item -LiteralPath $choicePath).Length -gt 65536 -or (Get-Item -LiteralPath $inventoryPath).Length -gt 1048576) { throw 'Inventory exceeds copy-stage size limit.' }
    $choice = [IO.File]::ReadAllText($choicePath) | ConvertFrom-Json
    $inventory = [IO.File]::ReadAllText($inventoryPath) | ConvertFrom-Json
    if ($inventory.status -ne 'complete') { $copyReport.status = 'skipped-inventory-incomplete'; Save $copyReport 'reference-copy.json'; exit 0 }
    if (-not $choice.selectedBin) { $copyReport.status = 'skipped-no-unique-audio-directory'; Save $copyReport 'reference-copy.json'; exit 0 }
    $sourceDirectory = Local $choice.selectedBin
    $allowed = $false
    foreach ($root in $inventory.roots) { if (Within $sourceDirectory (Local $root)) { $allowed = $true } }
    if (-not $allowed -or (Within $pack $sourceDirectory)) { throw 'Audio directory is outside the inventoried installation roots or contains this pack.' }
    $copyReport.sourceDirectory = $sourceDirectory
    $destinationDirectory = Join-Path $Out 'reference-assemblies'
    if ([IO.Directory]::Exists($destinationDirectory)) { throw 'Reference destination already exists.' }
    [void][IO.Directory]::CreateDirectory($destinationDirectory)
    [long]$copiedBytes = 0; [long]$reservedBytes = 0
    foreach ($name in @('AudioPlayer.dll','NAudio.dll','NAudio.Core.dll','NAudio.Wasapi.dll','NAudio.WinMM.dll','NAudio.Vorbis.dll','NVorbis.dll','NLog.dll','Libraries.dll','Preferences.dll')) {
        $source = Join-Path $sourceDirectory $name
        if (-not [IO.File]::Exists($source)) { continue }
        $row = [ordered]@{ name = $name; source = $source; status = 'started'; bytes = 0; sha256Before = ''; sha256Copy = ''; sha256After = ''; error = '' }
        $copyReport.files += $row; Save $copyReport 'reference-copy.json'
        try {
            $source = Local $source
            if ($copyReport.files.Count -gt 16) { throw 'Reference file count limit exceeded.' }
            $length = (Get-Item -LiteralPath $source).Length
            if ($length -gt 16777216 -or $reservedBytes + $length -gt 67108864) { throw 'Reference copy byte limit exceeded.' }
            $reservedBytes += $length # Counts failed/partial copies toward the total as well.
            Say "Copying only private audit dependency: $name ($length bytes)"
            $row.bytes = $length
            $row.sha256Before = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash.ToLowerInvariant()
            $destination = Join-Path $destinationDirectory $name
            $partial = $destination + '.partial'
            $reader = [IO.File]::Open($source, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
            $writer = $null
            try {
                if ($reader.Length -ne $length) { throw 'Source changed before read-only copy.' }
                $writer = [IO.File]::Open($partial, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::Read)
                $reader.CopyTo($writer)
            } finally { if ($writer) { $writer.Dispose() }; $reader.Dispose() }
            $row.sha256Copy = (Get-FileHash -LiteralPath $partial -Algorithm SHA256).Hash.ToLowerInvariant()
            $row.sha256After = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash.ToLowerInvariant()
            if ($row.sha256Before -ne $row.sha256Copy -or $row.sha256Before -ne $row.sha256After) { throw 'Source or copy changed; partial copy was not promoted.' }
            [IO.File]::Move($partial, $destination); $copiedBytes += $length; $row.status = 'copied-verified'
        } catch { $row.status = 'failed'; $row.error = $_.Exception.Message; $copyReport.errors += $row.error }
        Save $copyReport 'reference-copy.json'
    }
    $copyReport.status = if ($copyReport.errors.Count -eq 0) { 'complete' } else { 'completed-with-errors' }
    $copyReport.copiedBytes = $copiedBytes; $copyReport.reservedBytes = $reservedBytes
    Save $copyReport 'reference-copy.json'
    exit 0
}

if ($Stage -eq 'legacy-prepare') {
    # No install files are loaded/executed here. The independent .NET8 host will
    # reject anything outside the exact audited nine-DLL contract before loading.
    $legacySource = Local (Join-Path $pack 'legacy-net8')
    $hostDirectory = Local (Join-Path $Out 'legacy-host')
    if ([IO.Directory]::Exists($hostDirectory) -or [IO.File]::Exists($hostDirectory)) { throw 'Legacy host directory already exists.' }
    $configPath = Local (Join-Path $legacySource 'LegacyCapture.runtimeconfig.json')
    if ((Get-Item -LiteralPath $configPath).Length -gt 65536) { throw 'Legacy runtime configuration exceeds limit.' }
    $config = [IO.File]::ReadAllText($configPath) | ConvertFrom-Json
    if ($config.runtimeOptions.tfm -ne 'net8.0' -or $config.runtimeOptions.rollForward -ne 'LatestPatch' -or
        $config.runtimeOptions.framework.name -ne 'Microsoft.NETCore.App' -or $config.runtimeOptions.framework.version -ne '8.0.0' -or
        @($config.PSObject.Properties.Name).Count -ne 1 -or @($config.runtimeOptions.PSObject.Properties.Name).Count -ne 3 -or
        @($config.runtimeOptions.framework.PSObject.Properties.Name).Count -ne 2) { throw 'Unexpected legacy runtime configuration; no roll-forward to a different runtime is allowed.' }
    [void][IO.Directory]::CreateDirectory($hostDirectory)
    foreach ($name in @('LegacyCapture.cs.in','LegacyCapture.runtimeconfig.json','contract.json')) {
        $source = Local (Join-Path $legacySource $name)
        if ((Get-Item -LiteralPath $source).Length -gt 1048576) { throw 'Legacy host source exceeds limit.' }
        $destination = Join-Path $hostDirectory $name
        [IO.File]::Copy($source, $destination, $false)
        if ((Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash -ne (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash) { throw 'Legacy host copy hash mismatch.' }
    }
    Say 'Prepared private .NET8 capture host source; compilation is a separate supervised stage.'
    exit 0
}

if ($Stage -eq 'integrity' -or $Stage -eq 'input-recheck') {
    if ($Stage -eq 'integrity') {
        $manifestFile = Join-Path $pack 'pack-manifest.json'
        if ((Get-Item -LiteralPath $manifestFile).Length -gt 1048576) { throw 'Manifest is too large.' }
        $manifest = [IO.File]::ReadAllText($manifestFile) | ConvertFrom-Json
        if ($manifest.files.Count -gt 512) { throw 'Too many manifest entries.' }
        foreach ($entry in $manifest.files) {
            $path = Local (Join-Path $pack $entry.path)
            if (-not (Within $path $pack) -or -not [IO.File]::Exists($path)) { throw "Unsafe/missing manifest entry: $($entry.path)" }
            $file = Get-Item -LiteralPath $path
            if ($file.Length -ne $entry.bytes -or $file.Length -gt 33554432 -or (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $entry.sha256) { throw "Pack integrity failed: $($entry.path)" }
        }
    }
    $inputFile = Local (Join-Path $pack 'input\Coldest Winter Pentatonix.mp3')
    $hash = (Get-FileHash -LiteralPath $inputFile -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($hash -ne 'c1724f9916c61c623b9a19abbc80a7b161a0df79c8c504e427b8ba576289ada3') { throw 'The diagnostic input MP3 hash does not match.' }
    Save ([ordered]@{ status = 'passed'; inputSha256 = $hash }) ($Stage + '.json')
    exit 0
}
if ($Stage -eq 'framework-gdi') { & (Join-Path $pack 'Framework-GdiProbe.ps1') -Out $Out; exit $LASTEXITCODE }
if ($Stage -eq 'package') {
    # This entire archive stage has its own timeout. If interrupted, the original
    # result directory remains complete and the .partial archive is never promoted.
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = $Out + '.zip'; $partial = $zip + '.partial'
    if ([IO.File]::Exists($zip) -or [IO.File]::Exists($partial)) { throw 'Refusing to replace an existing result archive.' }
    $stream = [IO.File]::Open($partial, [IO.FileMode]::CreateNew, [IO.FileAccess]::ReadWrite, [IO.FileShare]::Read)
    $archive = $null
    try {
        $archive = New-Object IO.Compression.ZipArchive $stream,([IO.Compression.ZipArchiveMode]::Create),$true
        $files = New-Object 'System.Collections.Generic.List[string]'
        $folders = New-Object 'System.Collections.Generic.Queue[string]'
        $folders.Enqueue($Out); $folderCount = 0
        while ($folders.Count -gt 0) {
            $directory = $folders.Dequeue(); $folderCount++
            if ($folderCount -gt 32) { throw 'Too many result directories to package.' }
            foreach ($file in [IO.Directory]::EnumerateFileSystemEntries($directory)) {
                $attributes = [IO.File]::GetAttributes($file)
                if (($attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'Result links/cloud placeholders are not packaged.' }
                if (($attributes -band [IO.FileAttributes]::Directory) -ne 0) { $folders.Enqueue($file) }
                else { $files.Add($file); if ($files.Count -gt 512) { throw 'Too many result files to package.' } }
            }
        }
        [long]$total = 0
        foreach ($file in $files) {
            [void](Local $file)
            $name = $file.Substring($Out.Length + 1).Replace('\','/')
            if ($name -like 'package.*' -or $name -like '*.tmp' -or $name -eq 'CaptureSupervisor.exe') { continue }
            $size = (Get-Item -LiteralPath $file).Length; $total += $size
            if ($total -gt 536870912) { throw 'Result archive exceeds512MiB limit; copy the folder instead.' }
            Say "Archiving $name ($size bytes)"
            $entry = $archive.CreateEntry($name, [IO.Compression.CompressionLevel]::Fastest)
            $source = [IO.File]::Open($file, [IO.FileMode]::Open, [IO.FileAccess]::Read, ([IO.FileShare]::ReadWrite))
            $destination = $entry.Open()
            try { $source.CopyTo($destination) } finally { $destination.Dispose(); $source.Dispose() }
        }
    } finally { if ($archive) { $archive.Dispose() }; $stream.Dispose() }
    [IO.File]::Move($partial, $zip)
    Say "Copy-back archive ready: $zip"
    exit 0
}
