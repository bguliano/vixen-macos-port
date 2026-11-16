# Windows PowerShell 5.1 bootstrap only. No transcript or redirected Process streams.
# The separately compiled supervisor owns all diagnostic stages and Windows jobs.
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2
function Local([string]$Path) {
    $full = [IO.Path]::GetFullPath($Path)
    if ($full.StartsWith('\\') -or $full.Contains('"') -or $full.Contains("`n") -or $full.Contains("`r")) { throw 'Use a fully extracted ordinary local folder.' }
    if ((New-Object IO.DriveInfo ([IO.Path]::GetPathRoot($full))).DriveType -eq [IO.DriveType]::Network) { throw 'Network drives are excluded.' }
    $cursor = $full
    while ($cursor) {
        if ([IO.File]::Exists($cursor) -or [IO.Directory]::Exists($cursor)) {
            if (([IO.File]::GetAttributes($cursor) -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'Linked/cloud folders are excluded. Extract to a normal local folder.' }
        }
        $cursor = [IO.Path]::GetDirectoryName($cursor)
    }
    return $full
}
function Within([string]$Path, [string]$Root) {
    return $Path.Equals($Root, [StringComparison]::OrdinalIgnoreCase) -or $Path.StartsWith($Root.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase)
}
function Quote([string]$Value) {
    if ($Value.Contains('"') -or $Value.Contains("`r") -or $Value.Contains("`n")) { throw 'Unexpected character in argument.' }
    return '"' + ($Value -replace '(\\+)$', '$1$1') + '"'
}
$pack = Local $PSScriptRoot
$ancestor = $pack
while ($ancestor) {
    foreach ($marker in @('AudioPlayer.dll','Vixen.exe','Vixen.Application.exe','VixenApplication.exe','SystemData\SystemConfig.xml')) {
        if (Test-Path -LiteralPath (Join-Path $ancestor $marker)) { throw 'Move the extracted diagnostic folder outside your Vixen installation/show first.' }
    }
    $ancestor = [IO.Path]::GetDirectoryName($ancestor)
}
# Reject the manufacturer parent too, before any output is created.
foreach ($programRoot in @($env:ProgramFiles, ${env:ProgramFiles(x86)}, $env:ProgramW6432)) {
    if ($programRoot -and (Within $pack (Join-Path $programRoot 'Vixen Lights'))) { throw 'Extract this diagnostic outside Program Files\Vixen Lights.' }
}
$result = Join-Path $pack ('results-' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '-' + [Guid]::NewGuid().ToString('N').Substring(0,8))
[void][IO.Directory]::CreateDirectory($result)
Write-Host 'Vixen Windows raster detail diagnostic - offline; no Vixen launch or installation.'
Write-Host "RESULTS ARE SAVED HERE: $result"
Write-Host 'Focused Picture, ellipse-path and fractional-line cases run independently. A failure preserves partial results and later cases continue.'
$bootstrap = Join-Path $result 'bootstrap.txt'
function Log([string]$Message) {
    $line = [DateTime]::UtcNow.ToString('o') + ' ' + $Message
    Write-Host $line
    try { [IO.File]::AppendAllText($bootstrap, $line + [Environment]::NewLine) } catch { Write-Host 'Could not write bootstrap log; keep this console text.' }
}
function StopOwned([Diagnostics.Process]$Process, [string]$Name) {
    if ($Process.HasExited) { return }
    # Before the supervisor has compiled, Windows' built-in taskkill bounds cleanup
    # of only this known diagnostic process tree (including its compiler child).
    Log "Stopping owned $Name tree, PID $($Process.Id)."
    $killer = $null
    try {
        $killer = New-Object Diagnostics.Process
        $killer.StartInfo.FileName = Join-Path $env:SystemRoot 'System32\taskkill.exe'
        $killer.StartInfo.Arguments = '/PID ' + $Process.Id + ' /T /F'
        $killer.StartInfo.UseShellExecute = $false
        if ($killer.Start()) {
            if (-not $killer.WaitForExit(5000)) { $killer.Kill(); [void]$killer.WaitForExit(1000); Log 'Diagnostic tree cleanup command timed out.' }
            else { Log ('Diagnostic tree cleanup exit: ' + $killer.ExitCode) }
        }
    } catch { Log ('Diagnostic tree cleanup failed: ' + $_.Exception.Message) }
    finally {
        if ($killer) { try { if (-not $killer.HasExited) { $killer.Kill() } } catch { } }
        if (-not $Process.HasExited) { $Process.Kill(); if (-not $Process.WaitForExit(3000)) { Log 'Owned process did not stop within3s; retain its PID above.' } }
    }
}
function WaitOwned([Diagnostics.Process]$Process, [int]$Seconds, [string]$Name) {
    $watch = [Diagnostics.Stopwatch]::StartNew(); $next = 0
    while (-not $Process.WaitForExit(250)) {
        if ($watch.Elapsed.TotalSeconds -ge $Seconds) {
            Log "$Name exceeded ${Seconds}s. Stopping only this diagnostic process (PID $($Process.Id))."
            StopOwned $Process $Name
            throw "$Name timed out. The saved result folder remains available."
        }
        if ($watch.Elapsed.TotalSeconds -ge $next) {
            Log "$Name is running ($([int]$watch.Elapsed.TotalSeconds)s)."
            $next = $watch.Elapsed.TotalSeconds + 10
        }
    }
    return $Process.ExitCode
}
$owned = $null
$resultCode = 2
try {
    Log ('PowerShell ' + $PSVersionTable.PSVersion + '; OS ' + [Environment]::OSVersion.VersionString)
    $env:DOTNET_CLI_TELEMETRY_OPTOUT = '1'; $env:DOTNET_SKIP_FIRST_TIME_EXPERIENCE = '1'
    $env:DOTNET_CLI_WORKLOAD_UPDATE_NOTIFY_DISABLE = '1'; $env:DOTNET_NOLOGO = '1'
    # Validate executable source before compiling; full pack integrity is a bounded stage.
    $manifestPath = Join-Path $pack 'pack-manifest.json'
    if ((Get-Item -LiteralPath $manifestPath).Length -gt 1048576) { throw 'Manifest is too large.' }
    $manifest = [IO.File]::ReadAllText($manifestPath) | ConvertFrom-Json
    foreach ($name in @('SupervisorEngine.cs.in','RasterWorkflow.cs.in','Shared.cs.in','Compile-Supervisor.ps1','Pack-Worker.ps1','RasterCapture.cs.in','OriginalPictureMethods.cs.in','OriginalCountDownMethods.cs.in','OriginalFastPixel.cs.in')) {
        $sourceEntries = @($manifest.files | Where-Object { $_.path -eq $name })
        $source = Local (Join-Path $pack $name)
        if ((Get-Item -LiteralPath $source).Length -gt 1048576) { throw 'Diagnostic source exceeds1MiB. Extract a fresh copy.' }
        if ($sourceEntries.Count -ne 1 -or (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash.ToLowerInvariant() -ne $sourceEntries[0].sha256) { throw "Pack source integrity failed: $name. Extract a fresh copy." }
    }
    $compiler = ''
    foreach ($relative in @('Microsoft.NET\Framework64\v4.0.30319\csc.exe','Microsoft.NET\Framework\v4.0.30319\csc.exe')) {
        $candidate = Local (Join-Path $env:SystemRoot $relative)
        if ([IO.File]::Exists($candidate)) { $compiler = $candidate; break }
    }
    if (-not $compiler) { throw 'The built-in .NET Framework C# compiler is unavailable. Nothing will be installed; return this result folder.' }
    $executable = Join-Path $result 'RasterSupervisor.exe'
    Log 'Compiling the small diagnostic supervisor using the already-installed Windows Framework compiler (limit60s).'
    $owned = New-Object Diagnostics.Process
    $owned.StartInfo.FileName = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
    $arguments = @('-NoLogo','-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-File',(Join-Path $pack 'Compile-Supervisor.ps1'),'-Out',$result,'-Compiler',$compiler)
    $owned.StartInfo.Arguments = (@($arguments | ForEach-Object { Quote $_ }) -join ' ')
    $owned.StartInfo.UseShellExecute = $false
    $owned.StartInfo.WorkingDirectory = $pack
    if (-not $owned.Start()) { throw 'Framework compiler could not start.' }
    $code = WaitOwned $owned 60 'Framework compiler'
    Log "Framework compiler exited $code."
    if ($code -ne 0 -or -not [IO.File]::Exists($executable)) { throw 'Supervisor compilation failed. Its compiler.log is saved; return this folder.' }
    $owned = New-Object Diagnostics.Process
    $owned.StartInfo.FileName = $executable
    $owned.StartInfo.Arguments = (@($pack, $result, $compiler) | ForEach-Object { Quote $_ }) -join ' '
    $owned.StartInfo.UseShellExecute = $false
    $owned.StartInfo.WorkingDirectory = $pack
    Log 'Starting independent supervisor. Each stage has its own deadline; overall watchdog limit720s.'
    if (-not $owned.Start()) { throw 'Supervisor could not start.' }
    $code = WaitOwned $owned 720 'Diagnostic supervisor'
    Log "Supervisor exited $code."
    $resultCode = $code
    if ([IO.File]::Exists($result + '.zip')) { Log ('COPY THIS ZIP BACK: ' + $result + '.zip') }
    else { Log ('COPY THIS ENTIRE FOLDER BACK: ' + $result) }
} catch {
    Log ('STOPPED: ' + $_.Exception.Message)
    Log ('COPY THIS ENTIRE FOLDER BACK: ' + $result)
} finally {
    if ($owned) {
        try { StopOwned $owned 'diagnostic' } catch { Log ('Diagnostic cleanup: ' + $_.Exception.Message) }
    }
}
exit $resultCode
