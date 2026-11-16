#!/usr/bin/env python3
"""Inspect returned PE metadata/IL on Mac; never load any supplied original DLL."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

TYPES = {
    'Module.App.Curves.dll': ['VixenModules.App.Curves.Curve', 'ZedGraph.PointPair', 'ZedGraph.PointPairBase', 'ZedGraph.PointPairList'],
    'Module.App.ColorGradients.dll': ['VixenModules.App.ColorGradients.ColorGradient',
        'VixenModules.App.ColorGradients.ColorGradient+Point', 'VixenModules.App.ColorGradients.ColorPoint',
        'VixenModules.App.ColorGradients.AlphaPoint', 'VixenModules.App.ColorGradients.ColorGradient+PointList`1'],
    'Vixen.Core.dll': ['Common.Controls.ColorManagement.ColorModels.XYZ', 'Common.Controls.ColorManagement.ColorModels.RGB'],
    'Controls.dll': ['Common.Controls.ControlsEx.CollectionBase`1'],
}
OPTIONAL = {'Resources.dll', 'DiscreteColorPicker.dll', 'NLog.dll', 'NCalc.dll', 'NCalc2.dll'}


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--collection', type=Path, required=True)
    p.add_argument('--dotnet', type=Path, required=True)
    p.add_argument('--audit-host', type=Path, required=True)
    p.add_argument('--audit-host-sha256', required=True)
    p.add_argument('--out', type=Path, required=True)
    a = p.parse_args()
    assert sha(a.audit_host) == a.audit_host_sha256
    inventory = json.loads((a.collection / 'inventory.json').read_text(encoding='utf-8-sig'))
    assert inventory['status'] == 'copied-awaiting-metadata-audit' and not inventory['errors']
    files = {e['name']: e for e in inventory['files']}
    assert len(files) == len(inventory['files']) and TYPES.keys() <= files.keys() <= TYPES.keys() | OPTIONAL
    folder = a.collection / 'assemblies'
    assert {p.name for p in folder.iterdir()} == files.keys()
    assert all(p.is_file() and not p.is_symlink() and p.stat().st_size <= 67108864 for p in folder.iterdir())
    assert all(sha(folder / n) == e['sha256'] and (folder / n).stat().st_size == e['bytes'] and e['sourceUnchanged'] is True for n, e in files.items())
    a.out.mkdir()
    env = dict(os.environ, DOTNET_ROOT=str(a.dotnet.resolve().parent),
               DOTNET_CLI_TELEMETRY_OPTOUT='1', DOTNET_SKIP_FIRST_TIME_EXPERIENCE='1', DOTNET_GENERATE_ASPNET_CERTIFICATE='false')
    reports = []
    for name, entry in files.items():
        result = subprocess.run([str(a.dotnet.resolve()), str(a.audit_host.resolve()), str((folder / name).resolve()),
                                 '<Module>', *TYPES.get(name, [])], env=env, capture_output=True, text=True, timeout=15)
        output = a.out / (name + '.metadata.json')
        output.write_text(result.stdout)
        (a.out / (name + '.stderr.txt')).write_text(result.stderr)
        assert result.returncode == 0, (name, result.stderr)
        report = json.loads(result.stdout)
        assert report['sha256'] == entry['sha256'] and report['inputUnchanged'] is True
        modules = [t for t in report['types'] if t['name'] == '<Module>']
        assert len(modules) == 1 and not modules[0]['methods'], 'Module initializer/global method requires separate safety review'
        assert set(TYPES.get(name, [])) <= {t['name'] for t in report['types']}, 'Exact source API type absent'
        assert sha(folder / name) == entry['sha256']
        reports.append(dict(name=name, sha256=entry['sha256'], metadata=str(output), metadataSha256=sha(output),
                            references=report['references'], moduleGlobalMethodsAbsent=True))
    summary = dict(schema=1, status='metadata-collected-IL-review-required', scope='Metadata only; no supplied original assembly executed; this report does not authorize the Windows capture',
                   inventorySha256=sha(a.collection / 'inventory.json'), files=reports,
                   auditHostSha256=sha(a.audit_host), moduleInitializersAbsent=True)
    (a.out / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    print(json.dumps(summary))


if __name__ == '__main__':
    main()
