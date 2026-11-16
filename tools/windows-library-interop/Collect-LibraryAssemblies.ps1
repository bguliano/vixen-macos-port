# Read/copy only. No Assembly.Load, application startup, GUI, network, or install.
[CmdletBinding()]
param([Parameter(Mandatory=$true)][string]$VixenBin, [Parameter(Mandatory=$true)][string]$Output)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2
function Local([string]$Path) {
    $full = [IO.Path]::GetFullPath($Path)
    if ($full.StartsWith('\\') -or (New-Object IO.DriveInfo ([IO.Path]::GetPathRoot($full))).DriveType -eq 'Network') { throw 'Use local disk paths.' }
    for ($p=$full; $p; $p=[IO.Path]::GetDirectoryName($p)) {
        if ((Test-Path -LiteralPath $p) -and (([IO.File]::GetAttributes($p) -band [IO.FileAttributes]::ReparsePoint) -ne 0)) { throw 'Links/cloud placeholders are excluded.' }
    }
    return $full
}
function Sha([string]$Path) { return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() }
$report = [ordered]@{schema=1;status='partial';scope='Read-only installed DLL inventory and private copy; no assemblies executed';files=@();errors=@()}
$out = $null; $ownsOutput = $false
try {
    $bin = Local $VixenBin; $out = Local $Output
    if ((Test-Path -LiteralPath $out) -or $out.StartsWith($bin.TrimEnd('\')+'\',[StringComparison]::OrdinalIgnoreCase)) { throw 'Output must be a fresh folder outside the installation.' }
    if (-not [IO.Directory]::Exists([IO.Path]::GetDirectoryName($out))) { throw 'Output parent must exist.' }
    [void][IO.Directory]::CreateDirectory($out); $ownsOutput = $true; $private = Join-Path $out 'assemblies'; [void][IO.Directory]::CreateDirectory($private)
    $report.install=$bin; $report.output=$out; $report.startedUtc=[DateTime]::UtcNow.ToString('o')
    $required=@('Module.App.Curves.dll','Module.App.ColorGradients.dll','Vixen.Core.dll','Controls.dll')
    $optional=@('Resources.dll','DiscreteColorPicker.dll','NLog.dll','NCalc.dll','NCalc2.dll')
    foreach ($name in @($required+$optional)) {
        $path=Local (Join-Path $bin $name)
        if (-not [IO.File]::Exists($path)) { if ($required -contains $name) { throw ('Required original assembly missing: '+$name) }; continue }
        Write-Host ('Copying read-only original assembly: '+$name)
        $info=Get-Item -LiteralPath $path
        if ($info.Length -gt 67108864) { throw 'Unexpectedly large assembly.' }
        $before=Sha $path
        $identity=[Reflection.AssemblyName]::GetAssemblyName($path).FullName
        $version=[Diagnostics.FileVersionInfo]::GetVersionInfo($path)
        if (($required -contains $name) -and $version.ProductVersion -ne '3.12.1371.4+41c3f4f7e3ea13e8b21648a7398753fb60ad9a5a') { throw ('Installed version differs from the reviewed 3.12.4 source: '+$name+' '+$version.ProductVersion) }
        $dest=Join-Path $private $name; [IO.File]::Copy($path,$dest,$false)
        if ((Sha $dest) -ne $before -or (Sha $path) -ne $before) { throw ('Assembly changed during read-only copy: '+$name) }
        $report.files += [ordered]@{name=$name;bytes=$info.Length;sha256=$before;identity=$identity;productVersion=$version.ProductVersion;fileVersion=$version.FileVersion;source=$path;copied=$dest;sourceUnchanged=$true}
    }
    $report.status='copied-awaiting-metadata-audit'
} catch { $report.errors += $_.Exception.Message }
finally {
    $report.finishedUtc=[DateTime]::UtcNow.ToString('o')
    if ($ownsOutput -and $out -and [IO.Directory]::Exists($out)) { [IO.File]::WriteAllText((Join-Path $out 'inventory.json'),($report|ConvertTo-Json -Depth 8),(New-Object Text.UTF8Encoding $false)) }
    Write-Output ($report|ConvertTo-Json -Depth 8 -Compress)
}
if ($report.status -ne 'copied-awaiting-metadata-audit') { exit 1 }
