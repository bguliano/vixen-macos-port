#!/usr/bin/env python3
"""Finite two-way original-contract NBFX comparison on private files. No app/runtime startup."""
import argparse
import base64
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[3]
UNKNOWN = 'd0f4abf0-d543-4fbc-ad4a-75c6b14ea131'
IDS = ['cbd76d3b-c924-40ff-bad6-d1437b3dbdc0', '4633de41-bb29-443f-a390-23ea0c464ab2', 'e53bbb2d-d415-4e64-8f88-8cca96f2c8ff']


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def read(path):
    return json.loads(Path(path).read_text())


def utc(text):
    match = re.fullmatch(r'(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d)(?:\.(\d{1,7}))?Z', text)
    assert match, text
    return match[1] + '.' + (match[2] or '').ljust(7, '0') + 'Z'


def metadata(entries):
    return [(e['typeId'], e['name'], e['model'], utc(e['savedUtc']), e['payload']) for e in entries]


def typed_payloads(report):
    return {e['model']: e['typedPayload'] for e in report['typed'] if not e['opaque']}


def local(tag):
    return tag.rsplit('}', 1)[-1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True, help='Fresh directory below target')
    parser.add_argument('--rust-program', type=Path, default=ROOT/'target/debug/examples/effect_defaults_oracle')
    parser.add_argument('--existing-store', type=Path, help='Optional read-only existing file; never authoritative generation')
    args = parser.parse_args()
    out = args.out.resolve()
    assert out.is_relative_to(ROOT/'target')
    out.mkdir()
    rust = str(args.rust_program.resolve())
    dotnet = str(ROOT/'target/golden/dotnet-root/dotnet')
    oracle = str(ROOT/'target/golden/defaults/bin/DefaultsGolden.dll')
    def run(name, command):
        with (out/(name+'.log')).open('x') as log:
            subprocess.run(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=30)
    run('create', [rust,'create',str(out/'port-original')])
    source = out/'port-original/Effect Defaults/EffectDefaults.vfd'
    original_hash = sha(source)
    run('read-port', [dotnet,oracle,str(source),str(out/'dotnet-read-port'),'--write'])
    generated = out/'dotnet-read-port/dotnet.vfd'
    generated_hash = sha(generated)
    run('read-dotnet', [dotnet,oracle,str(generated),str(out/'dotnet-self-read')])
    run('port-roundtrip', [rust,'roundtrip',str(generated),str(out/'port-roundtrip')])
    run('read-imported', [dotnet,oracle,str(out/'port-roundtrip/imported.vfd'),str(out/'dotnet-read-imported')])
    run('read-resaved', [dotnet,oracle,str(out/'port-roundtrip/Effect Defaults/EffectDefaults.vfd'),str(out/'dotnet-read-resaved')])
    original = read(out/'port-original/port.json')
    original_read = read(out/'dotnet-read-port/read.json')
    authored = read(out/'dotnet-self-read/read.json')
    imported = read(out/'port-roundtrip/imported.json')
    imported_read = read(out/'dotnet-read-imported/read.json')
    resaved = read(out/'port-roundtrip/resaved.json')
    resaved_read = read(out/'dotnet-read-resaved/read.json')
    assert [e['typeId'] for e in original['entries']] == IDS
    assert metadata(original['entries']) == metadata(original_read['entries'])
    assert all(report['version'] == 1 for report in [original_read,authored,imported_read,resaved_read])
    expected_order = [IDS[2], UNKNOWN, IDS[1], IDS[0]]
    assert [e['typeId'] for e in authored['entries']] == expected_order
    assert metadata(authored['entries']) == metadata(imported['entries']) == metadata(imported_read['entries'])
    assert metadata(resaved['entries']) == metadata(resaved_read['entries'])
    assert [e['typeId'] for e in resaved['entries']] == expected_order
    assert typed_payloads(original_read) == typed_payloads(authored) == typed_payloads(imported_read) == typed_payloads(resaved_read)
    source_values = {e['typeId']: e['values'] for e in original['entries']}
    for report in [imported,resaved]:
        for entry in report['entries']:
            if entry['typeId'] in source_values:
                assert entry['values'] == source_values[entry['typeId']], entry['name']
    for entry in authored['entries']:
        expected_date = '2024-02-29T01:02:03.7654321Z' if entry['typeId'] == UNKNOWN else '2026-11-11T12:34:56.1234567Z'
        assert entry['savedUtc'] == expected_date
    unknown = next(e for e in authored['entries'] if e['typeId'] == UNKNOWN)
    opaque = base64.b64decode(unknown['payload'])
    assert len(opaque) == 66001 and opaque == bytes((i*73+19)&255 for i in range(66001))
    for report in [imported,imported_read,resaved,resaved_read]:
        assert metadata([next(e for e in report['entries'] if e['typeId'] == UNKNOWN)]) == metadata([unknown])
    assert next(e for e in imported['entries'] if e['typeId'] == UNKNOWN)['values'] is None
    assert next(e for e in resaved['entries'] if e['typeId'] == UNKNOWN)['values'] is None
    # Read the actual original data-contract serializer's XML for sentinel values;
    # successful deserialization alone could otherwise conceal dropped members.
    pulse = ET.parse(out/'dotnet-read-port'/f'{IDS[0]}.xml').getroot()
    curve = next(e for e in pulse if local(e.tag) == 'LevelCurve')
    assert [e.text for e in curve.iter() if local(e.tag) == 'Y'] == ['42','42']
    pattern = ET.parse(out/'dotnet-read-port'/f'{IDS[1]}.xml').getroot()
    values = {local(e.tag): e.text for e in pattern}
    assert values['WeaveThickness'] == '37' and values['WeaveHorizontalThickness'] == '21' and values['WeaveVerticalThickness'] == '63'
    wave = ET.parse(out/'dotnet-read-port'/f'{IDS[2]}.xml').getroot()
    marks = [e.text for e in wave.iter() if local(e.tag) == 'MarkCollectionId']
    assert marks == ['00000000-0000-0000-0000-000000000000']*2
    assert [e.text for e in wave.iter() if local(e.tag) == 'PhaseShift'] == ['13','97']
    assert sha(source) == original_hash and sha(generated) == generated_hash
    existing = None
    if args.existing_store:
        path = args.existing_store.resolve()
        checksum = sha(path)
        run('existing-read-only', [dotnet,oracle,str(path),str(out/'existing-read-only')])
        assert sha(path) == checksum
        existing = {'path':str(path),'sha256':checksum,'entries':len(read(out/'existing-read-only/read.json')['entries'])}
    shutil.copyfile(ROOT/'target/golden/defaults/source-manifest.sha256',out/'source-manifest.sha256')
    files = {str(p.relative_to(out)):sha(p) for p in out.rglob('*') if p.is_file()}
    result = {'passed':True,'knownModels':3,'opaqueBytes':len(opaque),'entriesAfterDotnet':4,
        'checks':['original C# reads current port store and all nested model fields','module identities and outer metadata/order','typed C# NBFX to port import/export without payload changes','fractional UTC tick dates','unknown opaque payload survives import/export and known-model saves','all typed C# payloads identical after port parameter resave','private inputs unchanged'],
        'existingReadOnly':existing,'filesSha256':files,
        'scope':'Original managed data contracts on .NET 10/macOS; no Windows application, GDI, effect rendering, profile runtime or devices.'}
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({k:v for k,v in result.items() if k != 'filesSha256'},indent=2))


if __name__ == '__main__':
    main()
