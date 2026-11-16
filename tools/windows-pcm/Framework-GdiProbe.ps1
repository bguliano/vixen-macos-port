# Bounded by Run-Capture.ps1 in a separate Windows PowerShell5.1 process.
# No Vixen assemblies, profile, audio or network are used.
param([Parameter(Mandatory=$true)][string]$Out)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2
$destination = [IO.Path]::GetFullPath($Out)
$pack = [IO.Path]::GetFullPath($PSScriptRoot).TrimEnd('\') + '\'
if (-not $destination.StartsWith($pack, [StringComparison]::OrdinalIgnoreCase) -or -not (Test-Path -LiteralPath $destination -PathType Container)) {
    throw 'The caller must supply its existing private result folder below this pack.'
}
if (-not [IO.Path]::GetDirectoryName($destination).Equals($pack.TrimEnd('\'), [StringComparison]::OrdinalIgnoreCase) -or [IO.Path]::GetFileName($destination) -notlike 'results-*') {
    throw 'The caller must supply a direct results-* child of this pack.'
}
$ancestor = $destination
while ($ancestor) {
    if (((Get-Item -LiteralPath $ancestor -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'No linked/cloud result paths are allowed.' }
    $ancestor = [IO.Path]::GetDirectoryName($ancestor)
}
$json = Join-Path $destination 'framework-gdi.json'
if (Test-Path -LiteralPath $json) { throw 'Framework GDI report already exists.' }
$bitmap = $null; $graphics = $null; $pen = $null; $font = $null
$status = [ordered]@{ status = 'failed'; scope = '.NET Framework availability only; not a Vixen raster oracle' }
$exitCode = 1
try {
    Write-Output 'Loading Windows PowerShell System.Drawing...'
    Add-Type -AssemblyName System.Drawing
    Write-Output 'Constructing Bitmap/Graphics...'
    $bitmap = New-Object Drawing.Bitmap 16,16
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $pen = New-Object Drawing.Pen ([Drawing.Color]::Red)
    Write-Output 'Constructing Arial font...'
    $font = New-Object Drawing.Font 'Arial',12
    Write-Output 'Drawing and writing the private PNG...'
    $graphics.Clear([Drawing.Color]::Black); $graphics.DrawLine($pen, 0, 0, 15, 15)
    $graphics.DrawString('A', $font, [Drawing.Brushes]::White, 1, 1)
    $bitmap.Save((Join-Path $destination 'framework-gdi.png'), [Drawing.Imaging.ImageFormat]::Png)
    $status.status = 'available'; $status.requestedFont = 'Arial'; $status.resolvedFont = $font.Name
    $status.drawingAssembly = [Drawing.Bitmap].Assembly.FullName
    $exitCode = 0
} catch { $status.error = $_.Exception.Message }
finally {
    foreach ($resource in @($font, $pen, $graphics, $bitmap)) { if ($resource) { $resource.Dispose() } }
    $status | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $json -Encoding UTF8
}
exit $exitCode
