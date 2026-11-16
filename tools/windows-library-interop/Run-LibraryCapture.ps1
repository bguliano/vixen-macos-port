# After returned DLL metadata/IL audit only. No runtime/SDK install or GUI.
[CmdletBinding()]
param([Parameter(Mandatory=$true)][string]$Collection, [Parameter(Mandatory=$true)][string]$ApprovedContract,
      [Parameter(Mandatory=$true)][string]$Fixtures, [Parameter(Mandatory=$true)][string]$Output,
      [string]$DotNet='C:\Program Files\dotnet\dotnet.exe')
$ErrorActionPreference='Stop'; Set-StrictMode -Version 2
$report=[ordered]@{schema=1;status='partial-or-failed';stages=@();errors=@();installUnchanged=$false};$out=$null;$ownsOutput=$false;$pins=@()
function Local([string]$Path) {
    $full=[IO.Path]::GetFullPath($Path)
    if($full.StartsWith('\\') -or (New-Object IO.DriveInfo ([IO.Path]::GetPathRoot($full))).DriveType -eq 'Network'){throw 'Use local disk paths.'}
    for($p=$full;$p;$p=[IO.Path]::GetDirectoryName($p)){if((Test-Path -LiteralPath $p)-and(([IO.File]::GetAttributes($p)-band[IO.FileAttributes]::ReparsePoint)-ne 0)){throw 'Links/cloud placeholders are excluded.'}}
    return $full
}
function Q([string]$Value) { return '"' + [regex]::Replace($Value, '(\\+)$', '$1$1') + '"' }
function Sha([string]$Path){return(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function SaveReport { if($ownsOutput -and $out -and [IO.Directory]::Exists($out)){[IO.File]::WriteAllText((Join-Path $out 'run.json'),($report|ConvertTo-Json -Depth 10),(New-Object Text.UTF8Encoding $false))} }
function Run([string]$Name,[string]$Exe,[string]$Arguments,[int]$Seconds){
    $p=New-Object Diagnostics.Process;$p.StartInfo.FileName=$Exe;$p.StartInfo.Arguments=$Arguments;$p.StartInfo.UseShellExecute=$false;$p.StartInfo.WorkingDirectory=$out
    $record=[ordered]@{name=$Name;limitSeconds=$Seconds;exitCode=$null;timedOut=$false};$started=$false
    Write-Host ('Starting '+$Name+' (limit '+$Seconds+' seconds).')
    try{
        if(-not $p.Start()){throw 'Could not start owned child.'};$started=$true;$record.pid=$p.Id
        if(-not $p.WaitForExit($Seconds*1000)){$record.timedOut=$true;throw ($Name+' exceeded its deadline.')}
        $record.exitCode=$p.ExitCode;if($p.ExitCode-ne 0){throw ($Name+' failed with exit '+$p.ExitCode)}
    }finally{
        try{if($started -and -not $p.HasExited){$p.Kill();if(-not $p.WaitForExit(3000)){throw 'Owned child did not stop.'}}}finally{$p.Dispose();$report.stages+=$record;SaveReport}
    }
}
try{
    $collection=Local $Collection;$approval=Local $ApprovedContract;$fixtures=Local $Fixtures;$out=Local $Output;$dotnet=Local $DotNet
    if(Test-Path -LiteralPath $out){throw 'Output must be fresh.'}
    if(-not [IO.Directory]::Exists([IO.Path]::GetDirectoryName($out))){throw 'Output parent must exist.'}
    $inventory=[IO.File]::ReadAllText((Join-Path $collection 'inventory.json'))|ConvertFrom-Json
    $contract=[IO.File]::ReadAllText($approval)|ConvertFrom-Json
    if($contract.inventorySha256 -cne (Sha (Join-Path $collection 'inventory.json'))){throw 'Approved original inventory changed.'}
    if($inventory.status-ne'copied-awaiting-metadata-audit' -or $contract.schema-ne 1 -or $contract.status-ne'approved-metadata-and-il-audit' -or $contract.productCommit-ne'41c3f4f7e3ea13e8b21648a7398753fb60ad9a5a' -or $contract.moduleInitializersAbsent-ne$true){throw 'Actual installed dependency audit has not approved this capture.'}
    $bins=Local (Join-Path $collection 'assemblies');$pins=@($contract.files)
    if(@($pins.name|Select-Object -Unique).Count-ne$pins.Count){throw 'Duplicate binary pins.'}
    foreach($base in @($collection,$fixtures,(Local $inventory.install))){if($out.Equals($base,[StringComparison]::OrdinalIgnoreCase) -or $out.StartsWith($base.TrimEnd('\')+'\',[StringComparison]::OrdinalIgnoreCase)){throw 'Output must be outside copied/installed binaries and fixtures.'}}
    if($pins.Count-ne@($inventory.files).Count -or $pins.Count-lt 4 -or $pins.Count-gt 9){throw 'Approved dependency inventory differs.'}
    foreach($p in $pins){if([IO.Path]::GetFileName($p.name)-cne$p.name -or $p.sha256 -cnotmatch '^[0-9a-f]{64}$'){throw 'Malformed binary pin.'};$entry=@($inventory.files|Where-Object {$_.name-ceq$p.name});if($entry.Count-ne 1 -or $entry[0].sha256-cne$p.sha256 -or (Sha (Local (Join-Path $bins $p.name)))-cne$p.sha256 -or (Sha (Local $entry[0].source))-cne$p.sha256){throw 'Copied or installed dependency changed before capture.'}}
    [void][IO.Directory]::CreateDirectory($out);$ownsOutput=$true;$report.startedUtc=[DateTime]::UtcNow.ToString('o');$report.approvedContractSha256=Sha $approval
    # Contract also pins exact fixtures and driver sources reviewed on the Mac.
    $requiredPack=@('host/LibraryCapture.cs','host/LibraryCapture.runtimeconfig.json','host/Run-LibraryCapture.ps1','fixture/Mac.vcl','fixture/Mac.vgl','fixture/fixture-pins.tsv','fixture/expected.json')
    $actualPack=@($contract.packFiles|ForEach-Object {$_.kind+'/'+$_.name})
    if($actualPack.Count-ne$requiredPack.Count -or (Compare-Object ($actualPack|Sort-Object) ($requiredPack|Sort-Object))){throw 'Incomplete package pins.'}
    foreach($p in $contract.packFiles){$base=if($p.kind-eq'fixture'){$fixtures}elseif($p.kind-eq'host'){$PSScriptRoot}else{throw 'Unknown package pin kind.'};if([IO.Path]::GetFileName($p.name)-cne$p.name -or (Sha (Local (Join-Path $base $p.name)))-cne$p.sha256){throw 'Reviewed package input changed.'}}
    $tsv=Join-Path $out 'pins.tsv';[IO.File]::WriteAllLines($tsv,@($pins|ForEach-Object {$_.name+"`t"+$_.sha256}),(New-Object Text.UTF8Encoding $false))
    $csc=Local (Join-Path $env:SystemRoot 'Microsoft.NET\Framework64\v4.0.30319\csc.exe');$exe=Join-Path $out 'LibraryCapture.exe'
    Run 'compile' $csc ('/nologo /langversion:5 /target:exe /platform:anycpu /r:System.Runtime.Serialization.dll /r:System.Drawing.dll /out:'+(Q $exe)+' '+(Q (Join-Path $PSScriptRoot 'LibraryCapture.cs'))) 30
    [IO.File]::Copy((Join-Path $PSScriptRoot 'LibraryCapture.runtimeconfig.json'),(Join-Path $out 'LibraryCapture.runtimeconfig.json'),$false)
    Run 'capture' $dotnet ((@($exe,$bins,$tsv,$fixtures,(Join-Path $out 'capture'))|ForEach-Object { Q $_ }) -join ' ') 30
    if(-not[IO.File]::Exists((Join-Path $out 'capture\capture.json'))){throw 'Capture report absent.'}
    $report.status='captured-validation-pending'
}catch{$report.errors+=$_.Exception.Message}
finally{
    try{if($pins.Count-gt 0){foreach($p in $pins){if([IO.Path]::GetFileName($p.name)-cne$p.name -or $p.sha256 -cnotmatch '^[0-9a-f]{64}$'){throw 'Malformed binary pin.'};$entry=@($inventory.files|Where-Object {$_.name-ceq$p.name});if($entry.Count-ne 1 -or (Sha (Local $entry[0].source))-cne$p.sha256 -or (Sha (Local (Join-Path $bins $p.name)))-cne$p.sha256){throw 'Original or private dependency changed.'}};$report.installUnchanged=$true}}catch{$report.errors+=$_.Exception.Message}
    if($report.errors.Count-gt 0){$report.status='partial-or-failed'};$report.finishedUtc=[DateTime]::UtcNow.ToString('o');SaveReport
    Write-Output ($report|ConvertTo-Json -Depth 10 -Compress)
}
if($report.status-ne'captured-validation-pending'){exit 1}
