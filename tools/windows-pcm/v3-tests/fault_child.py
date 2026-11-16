#!/usr/bin/env python3
"""Private subprocess fixtures for the capture supervisor's local fault checks.

These deliberately fail or stall only under a fresh target/ test directory.
They never load Vixen, open media, or contact a device or network.
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import time


def main():
    if sys.argv[1] == '--exec':
        # Match the Windows adapter's direct OS-file redirection, without
        # managed pipe-copy tasks. A fresh process group models job ownership.
        stdout, stderr, executable, *arguments = sys.argv[2:]
        os.setsid()
        for destination, descriptor in ((stdout, 1), (stderr, 2)):
            fd = os.open(destination, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
            os.dup2(fd, descriptor)
            os.close(fd)
        os.execv(executable, [executable, *arguments])
    mode, output = sys.argv[1:3]
    root = Path(output).resolve()
    if 'target' not in root.parts:
        raise RuntimeError('Fault fixture output must be inside target/')
    root.mkdir(exist_ok=True)
    (root / 'started.json').write_text(json.dumps({'pid': os.getpid(), 'mode': mode}))
    if mode == 'legacy-workflow':
        scenario, stage = sys.argv[3:5]
        out = root.parent
        if stage=='inventory' and scenario=='deadline-exhausted': time.sleep(120)
        if stage == 'inventory':
            selected = out / 'selected-install'; selected.mkdir()
            (out / 'inventory-choice.json').write_text(json.dumps({
                'selectedBin':str(selected),'assemblyReady':scenario=='strict-preferred',
                'hosts':[] if scenario=='no-host' else [str(Path(sys.executable).resolve())],
                'reason':'private legacy scheduler fixture'}))
        if stage == 'reference-copy':
            (out / 'reference-assemblies').mkdir()
        failures = {'prepare-failed':'legacy-prepare','compiler-failed':'legacy-compile',
                    'contract-mismatch':'legacy-contract','preflight-failed':'legacy-preflight',
                    'pcm-failed':'pcm','integrity-failed':'integrity','copy-failed':'reference-copy',
                    'package-failed':'package','input-failed':'input-recheck','runtime-failed':'runtimes-0'}
        if failures.get(scenario)==stage or (scenario=='copy-failed' and stage=='legacy-contract'):
            print('intentional '+scenario+' stage fault',file=sys.stderr,flush=True)
            return 23
        if stage=='runtimes-0':
            print('Microsoft.NETCore.App '+('6.0.29' if scenario=='runtime-missing' else '8.0.6')+' [private fixture]')
            if scenario in ('both-runtimes','strict-preferred'): print('Microsoft.NETCore.App 10.0.0 [private fixture]')
        if stage=='legacy-prepare':
            host=out/'legacy-host';host.mkdir()
            (host/'LegacyCapture.runtimeconfig.json').write_text('{"fixture":true}')
        if stage=='legacy-compile':
            host=out/'legacy-host'
            if scenario!='compiler-missing-output': (host/'LegacyCapture.exe').write_text('' if scenario=='compiler-empty-output' else 'fixture compiler output; never executed')
            if scenario=='compiler-missing-runtimeconfig': (host/'LegacyCapture.runtimeconfig.json').unlink()
        if stage=='legacy-preflight' and scenario!='preflight-missing-report':
            preflight=out/'legacy-preflight';preflight.mkdir()
            (preflight/'preflight.json').write_text('{"fixture":true}')
        if stage=='pcm' and scenario!='pcm-missing-manifest':
            capture=out/'capture';capture.mkdir()
            (capture/'manifest.json').write_text('{"fixture":true}')
            if scenario=='strict-preferred': (out/'capture\\manifest.json').write_text('{"fixture":true}')
        if stage=='package': Path(str(out)+'.zip').write_text('fixture archive marker, not a ZIP')
    elif mode == 'success':
        print('stdout-ready', flush=True)
        print('stderr-ready', file=sys.stderr, flush=True)
        (root / 'worker.json').write_text(json.dumps({'status': 'complete', 'value': 42}))
    elif mode == 'inventory':
        (root.parent / 'inventory-choice.json').write_text(json.dumps({'selectedBin':'','assemblyReady':False,'hosts':[],'reason':'fault-test inventory'}))
    elif mode == 'inventory-malformed':
        (root.parent / 'inventory-choice.json').write_text('{bad inventory')
    elif mode == 'inventory-empty-object':
        (root.parent / 'inventory-choice.json').write_text('{}')
    elif mode == 'inventory-pcm-missing':
        (root.parent / 'inventory-choice.json').write_text(json.dumps({
            'selectedBin': str(root / 'vanished-install'), 'assemblyReady': True,
            'hosts': [str(Path(sys.executable).resolve())], 'reason': 'intentional vanished install'}))
    elif mode == 'runtime-net10':
        print('Microsoft.NETCore.App 10.0.0 [private fixture]')
    elif mode == 'nonzero':
        (root / 'partial.txt').write_text('retained before failure')
        print('intentional fault', file=sys.stderr, flush=True)
        return 23
    elif mode == 'hang':
        (root / 'worker.json').write_text('{"status":"running"')
        print('waiting after partial output', flush=True)
        time.sleep(120)
    elif mode == 'flood':
        chunk = b'x' * 65536
        for _ in range(4096):
            os.write(1, chunk)
            os.write(2, chunk)
            time.sleep(.001)
    elif mode == 'large':
        for _ in range(32):
            os.write(1, b'o' * 65536)
            os.write(2, b'e' * 65536)
    elif mode == 'malformed':
        (root / 'worker.json').write_text('{ invalid json')
    elif mode == 'descendant':
        child = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(120)'])
        (root / 'descendant.pid').write_text(str(child.pid))
        print('parent finished; descendant retains stdout/stderr', flush=True)
    elif mode == 'slow':
        time.sleep(.8)
    else:
        raise ValueError(mode)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
