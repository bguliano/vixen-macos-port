#!/usr/bin/env python3
"""Strict installed-library return consumer. Requires production Mac readback too."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def read(path):
    return json.loads(Path(path).read_text(encoding='utf-8-sig'))


def samples(expected, capture):
    assert capture['schema'] == 1
    assert capture['status'] == 'captured-validation-pending'
    assert capture['productCommit'] == '41c3f4f7e3ea13e8b21648a7398753fb60ad9a5a'
    assert capture['runtime'].startswith('8.') and capture['architecture'] == 'X64'
    assert capture['inputBinariesUnchanged'] is True
    counts = {}
    for kind in ('curves', 'gradients'):
        wanted = [{key: row[key] for key in ('name', 'reference', 'current', 'samples')}
                  for row in expected[kind]]
        actual = capture[kind]
        assert actual['samples'] == wanted, f'{kind}: actual original evaluator differs'
        assert actual['roundtripSamples'] == wanted, f'{kind}: original roundtrip differs'
        counts[kind] = sum(len(row['samples']) for row in wanted)
    return counts


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--run', type=Path, required=True)
    p.add_argument('--fixtures', type=Path, required=True)
    p.add_argument('--approved-contract', type=Path, required=True)
    p.add_argument('--mac-reader', type=Path, required=True)
    p.add_argument('--mac-reader-sha256', required=True)
    p.add_argument('--out', type=Path, required=True)
    a = p.parse_args()
    assert not a.out.exists(), 'Preserve earlier acceptance evidence'
    contract = read(a.approved_contract)
    assert contract['status'] == 'approved-metadata-and-il-audit'
    assert contract['moduleInitializersAbsent'] is True
    run = read(a.run / 'run.json')
    assert run['status'] == 'captured-validation-pending'
    assert run['installUnchanged'] is True and run['errors'] == []
    assert run['approvedContractSha256'] == sha(a.approved_contract)
    assert [s['name'] for s in run['stages']] == ['compile', 'capture']
    assert all(s['exitCode'] == 0 and s['timedOut'] is False for s in run['stages'])
    expected = read(a.fixtures / 'expected.json')
    # Only reviewed expectation/fixture bytes are eligible as the oracle.
    fixture_pins = {e['name']: e['sha256'] for e in contract['packFiles'] if e['kind'] == 'fixture'}
    assert set(fixture_pins) == {'Mac.vcl', 'Mac.vgl', 'fixture-pins.tsv', 'expected.json'}
    assert all(sha(a.fixtures / name) == h for name, h in fixture_pins.items())
    capture_dir = a.run / 'capture'
    capture = read(capture_dir / 'capture.json')
    counts = samples(expected, capture)
    for kind, name in [('curves', 'Mac.vcl'), ('gradients', 'Mac.vgl')]:
        assert capture[kind]['inputSha256'] == fixture_pins[name]
        assert capture[kind]['roundtripSha256'] == sha(capture_dir / name)
    loaded = {e['identity'].split(',')[0] + '.dll': e for e in capture['loadedAssemblies']}
    original = {e['name']: e['sha256'] for e in contract['files']}
    required = {'Module.App.Curves.dll', 'Module.App.ColorGradients.dll', 'Vixen.Core.dll', 'Controls.dll'}
    assert required <= loaded.keys()
    assert all(loaded[n]['sha256'] == h for n, h in original.items() if n in loaded)
    assert sha(a.mac_reader) == a.mac_reader_sha256
    before = {n: sha(capture_dir / n) for n in ('Mac.vcl', 'Mac.vgl', 'capture.json')}
    r = subprocess.run([str(a.mac_reader.resolve()), '--readback', str(capture_dir.resolve())],
                       capture_output=True, text=True, timeout=20)
    assert r.returncode == 0, r.stderr
    readback = json.loads(r.stdout)
    assert readback == expected, 'Production Mac readback or independent arithmetic changed'
    assert before == {n: sha(capture_dir / n) for n in before}
    report = dict(passed=True, scope='Actual installed Windows3.12.4 data-contract library read/write/evaluator plus production Mac typed readback. No GUI, module-library mutation, profile, or newer3.13.4 claim.',
                  curveEntries=4, gradientEntries=4, samples=counts,
                  binaries={n: original[n] for n in required},
                  pins={str(p): sha(p) for p in (a.approved_contract, a.run / 'run.json', capture_dir / 'capture.json', a.mac_reader)},
                  inputsUnchanged=True)
    a.out.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report))


if __name__ == '__main__':
    main()
