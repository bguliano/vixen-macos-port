# Isolated compile worker. Parent bounds this complete process to60s, including
# native-output redirection and final status writing. No SDK/install/network.
param([Parameter(Mandatory=$true)][string]$Out, [Parameter(Mandatory=$true)][string]$Compiler)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2
$pack = [IO.Path]::GetFullPath($PSScriptRoot)
$destination = [IO.Path]::GetFullPath($Out)
if (-not [IO.Directory]::Exists($destination) -or -not [IO.Path]::GetDirectoryName($destination).Equals($pack, [StringComparison]::OrdinalIgnoreCase) -or [IO.Path]::GetFileName($destination) -notlike 'results-*') { throw 'Expected existing private results folder.' }
$allowed = @((Join-Path $env:SystemRoot 'Microsoft.NET\Framework64\v4.0.30319\csc.exe'), (Join-Path $env:SystemRoot 'Microsoft.NET\Framework\v4.0.30319\csc.exe'))
if ($Compiler -notin $allowed -or -not [IO.File]::Exists($Compiler)) { throw 'Only the existing Windows Framework compiler is allowed.' }
foreach ($path in @($destination, $Compiler, (Join-Path $pack 'SupervisorEngine.cs.in'))) {
    $ancestor = [IO.Path]::GetFullPath($path)
    while ($ancestor) {
        if (([IO.File]::GetAttributes($ancestor) -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'Linked/cloud compile paths are excluded.' }
        $ancestor = [IO.Path]::GetDirectoryName($ancestor)
    }
}
$executable = Join-Path $destination 'RasterSupervisor.exe'
$log = Join-Path $destination 'compiler.log'
if ([IO.File]::Exists($log) -or [IO.File]::Exists($executable)) { throw 'Compiler output already exists.' }
$arguments = @('/nologo','/noconfig','/target:exe','/platform:anycpu','/langversion:5','/reference:System.dll','/reference:System.Core.dll','/reference:System.Xml.dll','/reference:System.Runtime.Serialization.dll',('/out:' + $executable),(Join-Path $pack 'SupervisorEngine.cs.in'),(Join-Path $pack 'RasterWorkflow.cs.in'),(Join-Path $pack 'Shared.cs.in'))
Write-Host "Framework compiler output is saved in: $log"
try {
    & $Compiler @arguments *> $log
    $code = $LASTEXITCODE
    [IO.File]::WriteAllText((Join-Path $destination 'compiler.exit.txt'), [string]$code)
    exit $code
} catch {
    # Native PS5 redirection may use UTF-16; keep exception text in its own UTF-8 file.
    try { [IO.File]::WriteAllText((Join-Path $destination 'compiler-error.txt'), $_.Exception.ToString()) } catch { }
    exit 2
}
