# Independent outer watchdog also bounds early bootstrap file/hash/compiler setup.
# Inherited console only: no redirected output streams and no ReadToEnd waits.
$ErrorActionPreference='Stop'
$child=$null
try {
    $child=New-Object Diagnostics.Process
    $child.StartInfo.FileName=Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
    $child.StartInfo.Arguments='-NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "'+(Join-Path $PSScriptRoot 'Run-Raster.ps1')+'"'
    $child.StartInfo.WorkingDirectory=$PSScriptRoot
    $child.StartInfo.UseShellExecute=$false
    if (-not $child.Start()) {throw 'Raster bootstrap could not start.'}
    # No progress console writes occur while this independent timer is running.
    if (-not $child.WaitForExit(300000)) {
        $killer=New-Object Diagnostics.Process
        $killer.StartInfo.FileName=Join-Path $env:SystemRoot 'System32\taskkill.exe'
        $killer.StartInfo.Arguments='/PID '+$child.Id+' /T /F'
        $killer.StartInfo.UseShellExecute=$false
        try {if ($killer.Start() -and -not $killer.WaitForExit(5000)) {$killer.Kill();[void]$killer.WaitForExit(1000)}} finally {$killer.Dispose()}
        if (-not $child.HasExited) {$child.Kill();[void]$child.WaitForExit(3000)}
        Write-Host 'The independent 5-minute limit stopped this diagnostic process tree.'
        Write-Host 'Copy the newest results folder back. If none exists, copy this extracted folder.'
        exit 2
    }
    exit $child.ExitCode
} catch {
    Write-Host ('Raster watchdog: '+$_.Exception.Message)
    Write-Host 'Keep the extracted folder and any results folder for diagnosis.'
    exit 2
} finally {
    if ($child) {try {if (-not $child.HasExited) {$child.Kill();[void]$child.WaitForExit(3000)}} catch {}; $child.Dispose()}
}
