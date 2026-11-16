#!/usr/bin/env python3
"""Offline archive/provenance checks. Does not execute Windows or PowerShell."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import stat
import struct
import zipfile

ROOT = Path(__file__).resolve().parents[2]


def validate(archive):
    with zipfile.ZipFile(archive) as zipped:
        names = zipped.namelist()
        assert len(names) == len(set(names)), 'Duplicate archive entries'
        assert zipped.testzip() is None, 'Damaged compressed entry'
        files = {}
        for entry in zipped.infolist():
            path = PurePosixPath(entry.filename)
            assert not path.is_absolute() and '..' not in path.parts and '\\' not in entry.filename and ':' not in entry.filename
            assert path.parts[0] == 'Vixen-Windows-Capture'
            assert not stat.S_ISLNK(entry.external_attr >> 16), 'No linked pack content'
            assert not entry.is_dir()
            files['/'.join(path.parts[1:])] = zipped.read(entry)
    manifest = json.loads(files['pack-manifest.json'])
    declared = {row['path'] for row in manifest['files']}
    assert len(declared) == len(manifest['files']) and declared == files.keys() - {'pack-manifest.json'}
    for row in manifest['files']:
        data = files[row['path']]
        assert row['bytes'] == len(data) and hashlib.sha256(data).hexdigest() == row['sha256'], row['path']
    for name in ('LegacyCapture.cs.in', 'LegacyCapture.runtimeconfig.json', 'contract.json'):
        assert files['legacy-net8/' + name] == (ROOT / 'tools/windows-pcm/legacy-net8' / name).read_bytes()
    legacy_config = json.loads(files['legacy-net8/LegacyCapture.runtimeconfig.json'])['runtimeOptions']
    assert legacy_config == {'tfm': 'net8.0', 'rollForward': 'LatestPatch', 'framework': {'name': 'Microsoft.NETCore.App', 'version': '8.0.0'}}
    legacy_contract = json.loads(files['legacy-net8/contract.json'])
    assert legacy_contract['id'] == 'vixen-3.12.4-net8-installed-audio-1' and len(legacy_contract['files']) == 9
    assert legacy_contract['inventoryArchiveSha256'] == '8741a09e9920869227bee6667cb069152a97fe737534284aa9ad5858419e304d'
    for row in legacy_contract['files']:
        returned = ROOT / 'target/windows-results/attempt-04/reference-assemblies' / row['name']
        assert row['bytes'] == returned.stat().st_size and row['sha256'] == hashlib.sha256(returned.read_bytes()).hexdigest()
    song = files['input/Coldest Winter Pentatonix.mp3']
    assert hashlib.sha256(song).hexdigest() == 'c1724f9916c61c623b9a19abbc80a7b161a0df79c8c504e427b8ba576289ada3'
    assert not any(PurePosixPath(name).name.lower() in ('audioplayer.dll', 'vixen.exe', 'vixenapplication.exe') for name in files)
    assert not any(name.lower().endswith('.exe') for name in files), 'No native apphost/runtime was claimed or packed'
    for name, data in files.items():
        if name.endswith('.dll'):
            assert data[:2] == b'MZ'
            pe = struct.unpack_from('<I', data, 0x3c)[0]
            assert data[pe:pe + 4] == b'PE\x00\x00'
            optional = pe + 24
            magic = struct.unpack_from('<H', data, optional)[0]
            directories = optional + (96 if magic == 0x10b else 112)
            assert struct.unpack_from('<I', data, directories + 14 * 8)[0] != 0, 'Expected managed CLI assembly'
    for area, name in [('capture', 'WindowsPcmCapture'), ('gdi', 'GdiAvailability')]:
        config = json.loads(files[f'{area}/{name}.runtimeconfig.json'])['runtimeOptions']
        assert config['tfm'] == 'net10.0' and config['framework'] == {'name': 'Microsoft.NETCore.App', 'version': '10.0.0'}
    assert files['source/windows-pcm/Program.cs'] == (ROOT / 'tools/windows-pcm/Program.cs').read_bytes()
    assert files['source/golden/media/GdiProbe.cs'] == (ROOT / 'tools/golden/media/GdiProbe.cs').read_bytes()
    assert files['capture/WindowsPcmCapture.dll'] == (ROOT / 'target/windows-pcm/bin/WindowsPcmCapture.dll').read_bytes()
    windows_events_hash = hashlib.sha256(files['gdi/Microsoft.Win32.SystemEvents.dll']).hexdigest()
    assert windows_events_hash == 'b6438e57241f9beda1c457db6ff1b4ed3e94b37ffcc3dab42c80b0dc0f1aa2af', 'Require cached Windows runtime implementation, not portable stub'
    provenance = json.loads(files['provenance.json'])
    assert provenance['drawingPackages']['Microsoft.Win32.SystemEvents']['selectedAsset'] == 'runtimes/win/lib/net10.0/Microsoft.Win32.SystemEvents.dll'
    assert provenance['drawingPackages']['Microsoft.Win32.SystemEvents']['sha256'] == windows_events_hash
    for name in ('System.Drawing.Common.dll', 'System.Private.Windows.Core.dll', 'System.Private.Windows.GdiPlus.dll'):
        assert files['gdi/' + name] == (ROOT / 'target/windows-gdi/bin' / name).read_bytes()
    assert b'The MIT License' in files['licenses/system.drawing.common/LICENSE.TXT']
    assert b'<license type="expression">MIT</license>' in files['licenses/microsoft.win32.systemevents/microsoft.win32.systemevents.nuspec']
    script = files['Run-Capture.ps1'].decode('utf-8')
    worker = files['Capture-Worker.ps1'].decode('utf-8')
    supervisor = files['Supervisor.cs'].decode('utf-8')
    for name in ('Run-Capture.ps1', 'Run-Capture.cmd', 'Framework-GdiProbe.ps1', 'Capture-Worker.ps1', 'Compile-Supervisor.ps1', 'Supervisor.cs', 'START-HERE.txt'):
        assert files[name] == (ROOT / 'tools/windows-pcm' / name).read_bytes(), name
    for forbidden in ('Invoke-WebRequest', 'Invoke-RestMethod', 'Win32_Product ', 'winget install', 'Set-ExecutionPolicy', 'Start-Transcript', 'Stop-Transcript'):
        assert forbidden.lower() not in (script + worker).lower()
    assert provenance['legacyContractSha256'] == hashlib.sha256(files['legacy-net8/contract.json']).hexdigest()
    assert provenance['runnerVersion'] == 3 and provenance['patchVersion'] == '3.2'
    assert script.index('$pack = Local') < script.index('[void][IO.Directory]::CreateDirectory($result)')
    assert script.index('if (Within $pack $VixenBin)') < script.index('[void][IO.Directory]::CreateDirectory($result)')
    assert "WaitOwned $owned 60" in script and "WaitOwned $owned 720" in script
    assert 'RedirectStandard' not in script and '.WaitForExit()' not in script
    assert 'Capture-Worker.ps1' in supervisor and 'reserveSeconds = 70' in supervisor
    assert 'AssignProcessToJobObject' in supervisor and '0x2000; // KILL_ON_JOB_CLOSE' in supervisor
    assert '0x08080004' in supervisor and '0x20002' in supervisor
    assert 'attrsInitialized' in supervisor and 'LogLimit = 32L * 1024 * 1024' in supervisor
    assert 'patchVersion = "3.2"' in supervisor
    assert '[IO.File]::Replace($temporary, $path, $previous)' in worker
    assert '[IO.File]::Replace($temporary, $path, $null)' not in worker
    assert 'ReadJson<InventoryChoice>' in supervisor and 'skipped-incompatible-or-ambiguous-install' in supervisor
    workflow = supervisor[supervisor.index("public int Run(string pack"):supervisor.index("public static class Paths")]
    assert workflow.index('Worker(pack, "inventory"') < workflow.index('Name = "pcm"')
    assert workflow.index('Worker(pack, "integrity"') < workflow.index("RunLegacyCapture(pack, dotnet8")
    assert all(stage in supervisor for stage in ("legacy-prepare", "legacy-compile", "legacy-contract", "legacy-preflight"))
    assert "depth = 3; roots = 8; directories = 128; entries = 8192" in worker
    assert "metadataFiles = 256; hashBytes = 67108864" in worker
    assert "Save $data 'inventory.json'" in worker and "'inventory-choice.json'" in worker
    assert "No AudioPlayer.dll found" in worker and "Multiple AudioPlayer directories" in worker
    assert "[IO.File]::Move($partial, $zip)" in worker and "'package'" in worker
    assert "'--vixen-bin'" not in worker  # optional capture is launched directly by the supervisor
    return {'passed': True, 'files': len(files), 'zipBytes': archive.stat().st_size,
            'zipSha256': hashlib.sha256(archive.read_bytes()).hexdigest(),
            'scope': 'Archive/managed-PE/runtime/provenance/hash/static-script checks; no Windows or PowerShell execution'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archive', type=Path)
    args = parser.parse_args()
    print(json.dumps(validate(args.archive.resolve()), indent=2))


if __name__ == '__main__':
    main()
