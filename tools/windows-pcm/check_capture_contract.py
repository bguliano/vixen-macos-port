#!/usr/bin/env python3
"""Local provenance-validator cases only; synthetic manifests are not Windows evidence."""
import copy
import json
from pathlib import Path
from compare_capture import validate_binary_contract

ROOT = Path(__file__).resolve().parent
contract = json.loads((ROOT / 'legacy-net8/contract.json').read_text())
sample = {
    'referenceBinaryContract': contract,
    'runtime': {'framework': '.NET 8.0.6', 'processArchitecture': 'X64'},
    'decoder': {'type': 'NAudio.Wave.MediaFoundationReader, NAudio.Wasapi, Version=2.2.1.0'},
    'managedAssemblies': [{'identity': row['name'][:-4] + ', Version=1.0.0.0',
                           'file': {'bytes': row['bytes'], 'sha256': row['sha256']}}
                          for row in contract['files']],
}
checks = []


def case(name, mutation=None, valid=False):
    value = copy.deepcopy(sample)
    if mutation:
        mutation(value)
    try:
        validate_binary_contract(value)
    except ValueError:
        assert not valid, name
    else:
        assert valid, name
    checks.append(name)


case('known binary contract', valid=True)
case('changed contract identity', lambda x: x['referenceBinaryContract'].update(id='unknown'))
case('changed source archive', lambda x: x['referenceBinaryContract'].update(inventoryArchiveSha256='0'*64))
case('missing pin', lambda x: x['referenceBinaryContract']['files'].pop())
case('duplicate pin', lambda x: x['referenceBinaryContract']['files'].__setitem__(1, x['referenceBinaryContract']['files'][0]))
case('changed binary hash', lambda x: x['referenceBinaryContract']['files'][0].update(sha256='0'*64))
case('changed binary size', lambda x: x['referenceBinaryContract']['files'][0].update(bytes=0))
case('wrong runtime major', lambda x: x['runtime'].update(framework='.NET 10.0.12'))
case('wrong process architecture', lambda x: x['runtime'].update(processArchitecture='Arm64'))
case('replacement decoder', lambda x: x['decoder'].update(type='Replacement.Decoder'))
case('missing loaded reader', lambda x: x['managedAssemblies'].pop(0))
case('changed loaded reader', lambda x: x['managedAssemblies'][0]['file'].update(sha256='0'*64))
case('duplicate loaded reader', lambda x: x['managedAssemblies'].append(x['managedAssemblies'][0]))
case('missing all provenance', lambda x: x.pop('referenceBinaryContract'))
case('unchanged strict source contract', lambda x: (x.pop('referenceBinaryContract'), x.update(referenceSourceContract=json.loads((ROOT/'reference.json').read_text()))), valid=True)
print(json.dumps({'passed': True, 'checks': checks, 'count': len(checks),
                  'scope': 'Synthetic provenance validation only; no decoder or Windows runtime executed'}, indent=2))
