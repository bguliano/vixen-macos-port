#!/usr/bin/env python3
"""Package existing offline-built managed hosts and the exact private MP3 copy."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import zipfile

ROOT = Path(__file__).resolve().parents[2]
EXPECTED = 'c1724f9916c61c623b9a19abbc80a7b161a0df79c8c504e427b8ba576289ada3'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--input', type=Path, required=True)
    parser.add_argument('--out', type=Path, default=ROOT / 'target/windows-capture-pack')
    parser.add_argument('--packages', type=Path, help='Existing offline NuGet cache (private SDK CLI cache by default)')
    parser.add_argument('--base-pack', type=Path, help='Retained exact v3.1 archive for unchanged GDI assets/licenses when the original package cache is absent')
    args = parser.parse_args()
    source = args.input.resolve(strict=True)
    assert source.is_file() and digest(source) == EXPECTED, 'Requires the exact diagnostic MP3'
    out = args.out.resolve()
    assert out.is_relative_to(ROOT / 'target') and not out.exists(), 'Use a fresh target directory'
    folder = out / 'Vixen-Windows-Capture'
    folder.mkdir(parents=True)
    packages = args.packages.resolve() if args.packages else ROOT / 'target/golden/cli-home/.nuget/packages'
    drawing = packages / 'system.drawing.common/10.0.10'
    events = packages / 'microsoft.win32.systemevents/10.0.10'

    def copy(path, relative):
        destination = folder / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, destination)

    for name in ('Run-Capture.ps1', 'Run-Capture.cmd', 'Framework-GdiProbe.ps1', 'Capture-Worker.ps1', 'Compile-Supervisor.ps1', 'Supervisor.cs', 'START-HERE.txt'):
        copy(ROOT / 'tools/windows-pcm' / name, name)
    for name in ('LegacyCapture.cs.in', 'LegacyCapture.runtimeconfig.json', 'contract.json'):
        copy(ROOT / 'tools/windows-pcm/legacy-net8' / name, 'legacy-net8/' + name)
    copy(source, 'input/Coldest Winter Pentatonix.mp3')
    for name in ('WindowsPcmCapture.dll', 'WindowsPcmCapture.deps.json', 'WindowsPcmCapture.runtimeconfig.json'):
        copy(ROOT / 'target/windows-pcm/bin' / name, 'capture/' + name)
    gdi = ROOT / 'target/windows-gdi/bin'
    for path in sorted(gdi.iterdir()):
        if path.suffix == '.dll' or path.name in ('GdiAvailability.deps.json', 'GdiAvailability.runtimeconfig.json'):
            copy(path, 'gdi/' + path.name)
    assert (folder / 'gdi/GdiAvailability.dll').is_file()
    base_provenance = None
    if args.base_pack:
        base = args.base_pack.resolve(strict=True)
        assert digest(base) == '540f197cc5a7a1d848219285d71a05058b4f89e8cea8e5528a09c8c00187007e', 'Only the retained reviewed v3.1 archive is accepted'
        with zipfile.ZipFile(base) as zipped:
            payloads = {name.removeprefix('Vixen-Windows-Capture/'): zipped.read(name) for name in zipped.namelist()}
        manifest = json.loads(payloads['pack-manifest.json'])
        assert {row['path'] for row in manifest['files']} == payloads.keys() - {'pack-manifest.json'}
        for row in manifest['files']:
            assert len(payloads[row['path']]) == row['bytes'] and hashlib.sha256(payloads[row['path']]).hexdigest() == row['sha256']
        for name, data in payloads.items():
            if name.startswith(('capture/', 'input/')) or name.startswith('gdi/') and not name.endswith('Microsoft.Win32.SystemEvents.dll'):
                assert (folder / name).read_bytes() == data, 'Existing payload changed: ' + name
            if name.startswith('licenses/') or name == 'gdi/Microsoft.Win32.SystemEvents.dll':
                destination = folder / name
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(data)
        drawing_packages = json.loads(payloads['provenance.json'])['drawingPackages']
        base_provenance = {'archiveSha256': digest(base), 'scope': 'Unchanged managed GDI assets and notices from retained reviewed v3.1; original package-cache files were not needed'}
    else:
        # The Mac build copies a portable fallback; ship the cached Windows asset.
        for name in ('System.Drawing.Common.dll', 'System.Private.Windows.Core.dll', 'System.Private.Windows.GdiPlus.dll'):
            assert digest(gdi / name) == digest(drawing / 'lib/net10.0' / name)
        windows_events = events / 'runtimes/win/lib/net10.0/Microsoft.Win32.SystemEvents.dll'
        assert digest(windows_events) == 'b6438e57241f9beda1c457db6ff1b4ed3e94b37ffcc3dab42c80b0dc0f1aa2af'
        copy(windows_events, 'gdi/Microsoft.Win32.SystemEvents.dll')
        for directory, names in [(drawing, ('LICENSE.TXT', 'THIRD-PARTY-NOTICES.TXT', 'system.drawing.common.nuspec')),
                                 (events, ('THIRD-PARTY-NOTICES.TXT', 'microsoft.win32.systemevents.nuspec'))]:
            for name in names:
                copy(directory / name, 'licenses/' + directory.parent.name + '/' + name)
        (folder / 'licenses/README.txt').write_text(
            'System.Drawing.Common and Microsoft.Win32.SystemEvents10.0.10 are Microsoft/.NET packages.\n'
            'Unmodified cached licenses/notices are included; no package was downloaded.\n')
        drawing_packages = {'System.Drawing.Common': {'version': '10.0.10', 'packageSha256': digest(drawing / 'system.drawing.common.10.0.10.nupkg')},
                            'Microsoft.Win32.SystemEvents': {'version': '10.0.10', 'packageSha256': digest(events / 'microsoft.win32.systemevents.10.0.10.nupkg'),
                                'selectedAsset': 'runtimes/win/lib/net10.0/Microsoft.Win32.SystemEvents.dll', 'sha256': digest(windows_events)}}
    for path in sorted((ROOT / 'tools/windows-pcm').rglob('*')):
        if path.is_file() and path.suffix in ('.cs', '.csproj', '.props', '.json', '.config', '.md', '.ps1', '.py', '.txt', '.cmd', '.in'):
            if '__pycache__' not in path.parts:
                copy(path, 'source/windows-pcm/' + path.relative_to(ROOT / 'tools/windows-pcm').as_posix())
    copy(ROOT / 'tools/golden/media/GdiProbe.cs', 'source/golden/media/GdiProbe.cs')
    copy(ROOT / 'docs/review/windows-v3-1-compatibility.md', 'docs/review/windows-v3-1-compatibility.md')
    provenance = {
        'runnerVersion': 3,
        'patchVersion': '3.2',
        'capture': 'Unchanged reflection host invoking only supplied-install AudioFileReader/CachedSampleSource; no original Vixen DLLs included',
        'gdi': 'Unchanged tools/golden/media/GdiProbe.cs linked into a separate availability-only host with cached System.Drawing dependencies',
        'frameworkGdi': 'Windows PowerShell5.1 System.Drawing availability; distinct from the .NET10 probe',
        'supervisor': 'Source compiled locally with already-installed Windows Framework csc.exe; isolated timed Windows Job stages; no SDK/runtime installation',
        'runtime': 'Unchanged net10.0 hosts plus separate exact-hash net8 capture source compiled by existing Windows Framework compiler; no SDK or runtime installed or bundled',
        'windowsExecution': 'Returned v3.1 Windows diagnostics completed; new v3.2 net8 capture awaits actual Windows execution',
        'legacyCapture': 'Exact Vixen3.12.4 binary contract from returned v3.1; two original decodes, no replacement decoder',
        'legacyContractSha256': digest(ROOT / 'tools/windows-pcm/legacy-net8/contract.json'),
        'offlinePublish': 'NU1100: Microsoft.NETCore.App.Runtime.win-x64 and ASP.NET runtime10.0.12 unavailable in offline cache',
        'expectedInputSha256': EXPECTED,
        'drawingPackages': drawing_packages,
        'retainedAssetProvenance': base_provenance,
    }
    (folder / 'provenance.json').write_text(json.dumps(provenance, indent=2) + '\n')
    files = [{'path': path.relative_to(folder).as_posix(), 'bytes': path.stat().st_size, 'sha256': digest(path)}
             for path in sorted(folder.rglob('*')) if path.is_file()]
    (folder / 'pack-manifest.json').write_text(json.dumps({'schema': 1, 'files': files}, indent=2) + '\n')
    archive = out / 'Vixen-Windows-Capture.zip'
    with zipfile.ZipFile(archive, 'x', compression=zipfile.ZIP_DEFLATED) as zipped:
        for path in sorted(folder.rglob('*')):
            if path.is_file():
                zipped.write(path, path.relative_to(out).as_posix())
    assert digest(source) == EXPECTED, 'Original input changed during packaging'
    result = {'zip': str(archive), 'sha256': digest(archive), 'bytes': archive.stat().st_size,
              'files': len(files) + 1, 'inputUnchanged': True, 'windowsExecution': False}
    (out / 'package-result.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
