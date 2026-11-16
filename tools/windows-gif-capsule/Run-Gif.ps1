# Windows PowerShell5.1, installed Framework compiler/GDI+ only. No Vixen load.
[CmdletBinding()]
param([Parameter(Mandatory=$true)][string]$Output)
$ErrorActionPreference='Stop';Set-StrictMode -Version 2
$report=[ordered]@{schema=1;status='partial-or-failed';stages=@();errors=@();inputUnchanged=$false};$out=$null;$owns=$false;$pins=@();$clock=[Diagnostics.Stopwatch]::StartNew()
function Local([string]$Path){$v=[IO.Path]::GetFullPath($Path);if($v.StartsWith('\\') -or (New-Object IO.DriveInfo ([IO.Path]::GetPathRoot($v))).DriveType-eq'Network'){throw 'Local disk paths required.'};for($p=$v;$p;$p=[IO.Path]::GetDirectoryName($p)){if((Test-Path -LiteralPath $p)-and(([IO.File]::GetAttributes($p)-band[IO.FileAttributes]::ReparsePoint)-ne 0)){throw 'Links/cloud placeholders excluded.'}};return $v}
function Q([string]$V){if($V.Contains('"')-or$V.Contains("`n")-or$V.Contains("`r")){throw 'Invalid native argument.'};return '"'+[regex]::Replace($V,'(\\+)$','$1$1')+'"'}
function Sha([string]$P){return(Get-FileHash -LiteralPath $P -Algorithm SHA256).Hash.ToLowerInvariant()}
function SaveReport{if($owns){[IO.File]::WriteAllText((Join-Path $out 'run.json'),($report|ConvertTo-Json -Depth 8),(New-Object Text.UTF8Encoding $false))}}
function Run([string]$Name,[string]$Exe,[string]$NativeArguments,[int]$Seconds){
    if($clock.Elapsed.TotalSeconds+$Seconds+3-gt 90){throw 'Overall90s capture budget exhausted.'}
    $p=New-Object Diagnostics.Process;$p.StartInfo.FileName=$Exe;$p.StartInfo.Arguments=$NativeArguments;$p.StartInfo.UseShellExecute=$false;$p.StartInfo.WorkingDirectory=$out
    $record=[ordered]@{name=$Name;exitCode=$null;timedOut=$false;limitSeconds=$Seconds};$started=$false
    Write-Host ('Starting '+$Name+' (limit '+$Seconds+' seconds).')
    try{if(-not$p.Start()){throw 'Could not start child.'};$started=$true;$record.pid=$p.Id;if(-not$p.WaitForExit($Seconds*1000)){$record.timedOut=$true;throw ($Name+' timed out.')};$record.exitCode=$p.ExitCode;if($p.ExitCode-ne 0){throw ($Name+' failed with exit '+$p.ExitCode)}}
    finally{try{if($started-and-not$p.HasExited){$p.Kill();if(-not$p.WaitForExit(3000)){throw 'Owned child did not stop.'}}}finally{$p.Dispose();$report.stages+=$record;SaveReport}}
}
try{
    $pack=Local $PSScriptRoot;$out=Local $Output
    if(Test-Path -LiteralPath $out){throw 'Output must be a new directory.'}
    if(-not[IO.Directory]::Exists([IO.Path]::GetDirectoryName($out))){throw 'Output parent must exist.'}
    $manifest=Join-Path $pack 'pack-pins.json';if((Get-Item -LiteralPath $manifest).Length-gt 32768){throw 'Manifest too large.'}
    $pins=@(([IO.File]::ReadAllText($manifest)|ConvertFrom-Json).files)
    $expected=@('GifCapture.cs','OriginalPictureScale.cs','Run-Gif.ps1','cases.json','full.gif','keep.gif','background.gif','previous.gif')
    if($pins.Count-ne 8 -or (Compare-Object ($pins.name|Sort-Object) ($expected|Sort-Object))){throw 'Unexpected package file set.'}
    foreach($pin in $pins){$path=Local(Join-Path $pack $pin.name);if((Get-Item -LiteralPath $path).Length-gt 65536 -or (Sha $path)-cne$pin.sha256){throw 'Package pin mismatch.'}}
    for($p=[IO.Path]::GetDirectoryName($out);$p;$p=[IO.Path]::GetDirectoryName($p)){foreach($marker in @('Vixen.exe','VixenApplication.exe','AudioPlayer.dll','SystemData\SystemConfig.xml')){if(Test-Path -LiteralPath(Join-Path $p $marker)){throw 'Output must be outside Vixen installation/show.'}}}
    foreach($root in @($env:ProgramFiles,${env:ProgramFiles(x86)},$env:ProgramW6432)){if($root-and$out.StartsWith((Join-Path $root 'Vixen Lights')+'\',[StringComparison]::OrdinalIgnoreCase)){throw 'Output must be outside installed Vixen.'}}
    [void][IO.Directory]::CreateDirectory($out);$owns=$true;$report.startedUtc=[DateTime]::UtcNow.ToString('o');$report.packPinsSha256=Sha $manifest
    $compiler=Local(Join-Path $env:SystemRoot 'Microsoft.NET\Framework64\v4.0.30319\csc.exe');$exe=Join-Path $out 'GifCapture.exe'
    Run 'compile' $compiler ('/nologo /langversion:5 /target:exe /platform:x64 /r:System.Drawing.dll /out:'+(Q $exe)+' '+(Q(Join-Path $pack 'GifCapture.cs'))+' '+(Q(Join-Path $pack 'OriginalPictureScale.cs'))) 30
    foreach($name in @('full','keep','background','previous')){foreach($speed in @(1,3)){$id=$name+'-speed'+$speed;try{Run $id $exe ((@($pack,$id,(Join-Path $out ($id+'.json')))|ForEach-Object{Q $_})-join' ') 5;if(-not[IO.File]::Exists((Join-Path $out ($id+'.json')))){throw 'Missing case JSON.'}}catch{$report.errors+=($id+': '+$_.Exception.Message);SaveReport}}}
}catch{$report.errors+=$_.Exception.Message}
finally{
    try{if($pins.Count-eq 8){foreach($pin in $pins){if((Sha(Local(Join-Path $pack $pin.name)))-cne$pin.sha256){throw 'Package changed during capture.'}};$report.inputUnchanged=$true}}catch{$report.errors+=$_.Exception.Message}
    if($report.errors.Count-eq 0-and$report.stages.Count-eq 9-and$report.inputUnchanged){$report.status='captured-comparison-pending'}
    $report.elapsedSeconds=$clock.Elapsed.TotalSeconds;SaveReport;Write-Output($report|ConvertTo-Json -Depth 8 -Compress)
}
if($report.status-ne'captured-comparison-pending'){exit 1}
