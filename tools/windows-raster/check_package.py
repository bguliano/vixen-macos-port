#!/usr/bin/env python3
"""Read-only verification of the source-only raster ZIP; no extraction/execution."""
import argparse, hashlib, json, re, stat, zipfile
from pathlib import Path, PurePosixPath

PREFIX = 'Vixen-Windows-Raster/'
def digest(raw): return hashlib.sha256(raw).hexdigest()

def verify(archive):
    with zipfile.ZipFile(archive) as z:
        entries = z.infolist()
        assert 10 <= len(entries) <= 100, 'entry count'
        names = [e.filename for e in entries]
        assert len(names) == len(set(n.casefold() for n in names)), 'duplicate path'
        payload = {}
        for e in entries:
            assert e.filename.startswith(PREFIX), 'root folder'
            name = e.filename[len(PREFIX):]
            assert re.fullmatch(r'[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)*', name), 'unsafe path'
            assert all(part not in ('.', '..') for part in PurePosixPath(name).parts), 'path traversal'
            assert not e.flag_bits & 1 and not e.is_dir(), 'encrypted/directory entry'
            assert stat.S_IFMT(e.external_attr >> 16) in (0, stat.S_IFREG), 'nonregular entry'
            assert e.file_size <= 1048576, 'oversized entry'
            raw = z.read(e)
            assert len(raw) == e.file_size, 'length mismatch'
            assert not raw.startswith(b'MZ'), 'executable content'
            assert not name.lower().endswith(('.exe', '.dll', '.mp3', '.tim', '.fseq', '.pdb')), 'binary/private payload'
            payload[name] = raw
        manifest = json.loads(payload['pack-manifest.json'])
        assert manifest['schema'] == 1, 'schema'
        records = manifest['files']
        assert len(records) == len(payload) - 1, 'manifest count'
        expected = set(payload) - {'pack-manifest.json'}
        for record in records:
            name = record['path']
            assert name in expected, 'unexpected or duplicate manifest record'
            expected.remove(name)
            assert record['bytes'] == len(payload[name]) and record['sha256'] == digest(payload[name]), 'checksum'
        assert not expected, 'unlisted entries'
        contract = json.loads(payload['source-contract.json'])
        for field, name in [('picture', 'reference/Picture.cs.txt'), ('countDown', 'reference/CountDown.cs.txt'), ('fastPixel', 'reference/FastPixel.cs.txt'), ('supervisor', 'reference/Supervisor-parent.cs.txt')]:
            assert digest(payload[name]) == contract[field]['sha256'], 'original source hash'
        for name, value in contract['generated'].items():
            assert digest(payload[name]) == value, 'generated source hash'
        provenance = json.loads(payload['provenance.json'])
        checks = json.loads(payload['audit/local-result.json'])
        assert checks['passed'] and not checks['windowsDrawingExecuted'], 'local result scope'
        assert not any(provenance[k] for k in ['windowsDrawingExecuted', 'frameworkCompilerExecuted', 'powershellExecuted', 'nativeWindowsJobObjectsExecuted', 'runtimeDownloadOrInstall', 'privateUserFilesIncluded', 'runtimeBinariesIncluded', 'installedVixenBinaryContract']), 'unsupported claim'
        assert provenance['sourceContractSha256'] == digest(payload['source-contract.json']), 'source contract pin'
        assert provenance['caseGridSha256'] == digest(payload['cases.json']), 'grid pin'
        assert provenance['localChecksSha256'] == digest(payload['audit/local-result.json']), 'check report pin'
        assert checks['externalTestInputs']['tools/windows-pcm/v3-tests/fault_child.py'] == digest(payload['audit/tests/fault_child.py']), 'external fault fixture pin'
        for name, raw in payload.items():
            source = name.removeprefix('audit/')
            key = 'tools/windows-raster/' + source
            if key in checks['sourceHashes']:
                assert checks['sourceHashes'][key] == digest(raw), 'checked source drift'
        grid=json.loads(payload['cases.json']);assert grid['schema']==2 and len(grid['cases'])==74, 'case schema/count'
        assert {family:sum(c['family']==family for c in grid['cases']) for family in ['picture','countdown','primitive']}=={'picture':36,'countdown':20,'primitive':18}, 'family counts'
        return {'passed': True, 'files': len(entries), 'bytes': Path(archive).stat().st_size, 'sha256': digest(Path(archive).read_bytes()), 'sourceOnly': True, 'windowsExecuted': False}

def main():
    p = argparse.ArgumentParser(description=__doc__);p.add_argument('archive',type=Path);a=p.parse_args()
    print(json.dumps(verify(a.archive), indent=2))
if __name__ == '__main__': main()
