#!/usr/bin/env python3
"""Frozen, bounded Color-fix Layer regression; no launch during preparation.

Keeps the original five gestures and 59 model/persistence assertions. Adds
private Rust settings, tracked input cleanup, exact input pins and an all-in bound.
"""
import argparse
import json
import os
from pathlib import Path
import plistlib
import signal
import subprocess
import sys
import time
from test_layer_drag_foreground import digest, hashes, layer_xml
from test_library_family_foreground import preferences

ROOT = Path(__file__).resolve().parents[1]
LEGACY = ROOT/'scripts/test_layer_drag_foreground.py'
SOURCE = ROOT/'scripts/foreground_layer_drag_regression.swift'

def save(path, value):
    temporary=path.with_suffix(path.suffix+'.tmp')
    temporary.write_text(json.dumps(value,indent=2)+'\n'); temporary.replace(path)

def validate(out, m):
    assert out.is_relative_to(ROOT/'target')
    assert not (out/'result.json').exists() and not (out/'ui').exists() and not (out/'runtime').exists()
    for name, expected in m['frozenHashes'].items(): assert digest(Path(name)) == expected, 'Frozen input changed: '+name
    assert hashes(Path(m['source'])) == m['sourceHashes'], 'Fixture source changed'
    assert hashes(out/'profile') == m['preparedProfileHashes'], 'Prepared private profile changed'
    assert m['fixture']['controllers']==0 and m['fixture']['muted']
    assert m['steps']==59 and m['bundleIdentifier'].startswith('com.vixenlights.verification.')
    exe=Path(m['app']); info=plistlib.loads((exe.parents[1]/'Info.plist').read_bytes())
    assert info['CFBundleIdentifier']==m['bundleIdentifier']
    assert digest(exe)==m['appSha256'] and digest(Path(m['helper']))==m['helperSha256']
    assert str(out/'profile') in m['sequence']

def ensure_launch_budget(deadline, required=102):
    assert deadline-time.monotonic() >= required, 'Full process watchdog and cleanup no longer fit'

def terminate(process, deadline):
    if process is None or process.poll() is not None: return
    try: os.killpg(process.pid,signal.SIGTERM)
    except ProcessLookupError: pass
    try: process.wait(timeout=min(1.5,max(.01,deadline-time.monotonic())))
    except subprocess.TimeoutExpired:
        try: os.killpg(process.pid,signal.SIGKILL)
        except ProcessLookupError: pass
        process.wait(timeout=min(1,max(.01,deadline-time.monotonic())))

def verify(out,m,return_code,report):
    source=Path(m['source']); app=Path(m['app']); helper=Path(m['helper']); sequence=Path(m['sequence'])
    rows=json.loads((out/'ui/result.json').read_text())
    foreground=json.loads((out/'foreground.json').read_text())
    report.update(steps=len(rows),foreground=foreground)
    assert return_code == 0 and len(rows) == m["steps"] and all(r["ok"] for r in rows), [r for r in rows if not r["ok"]]
    assert foreground.get("passed") and foreground.get("restoredPrevious") and foreground.get("targetExited"), foreground
    assert len(foreground["drags"]) == 5 and all(r["observedOrder"] == r["expectedOrder"] for r in foreground["drags"])
    observed = [r["detail"] for r in rows if r.get("detail", "").startswith(("Zebra Layer:", "Middle Layer:"))]
    names = lambda value: [row.split(":")[0] for row in value.split(" | ")]
    initial = ["Zebra Layer", "Alpha Layer", "Middle Layer", "Default"]
    final = ["Middle Layer", "Alpha Layer", "Zebra Layer", "Default"]
    assert [names(value) for value in observed] == [initial]*4 + [["Middle Layer","Zebra Layer","Alpha Layer","Default"]] + [final]*3, observed
    states = [r["detail"] for r in rows if r.get("detail", "").startswith("effects=")]
    assert len(states) == 8 and all("undo=-" in s and "effects=0 " in s for s in states), states
    assert ["modified=true" in s for s in states] == [False]*4 + [True, True, False, False], states
    transport = [r["detail"] for r in rows if r.get("detail", "").startswith("running=")]
    assert len(transport) == 8 and all("running=false paused=false" in s for s in transport), transport
    saved = layer_xml(sequence)
    original = m["originalXML"]
    by_name = {layer["LayerName"]:layer for layer in original["layers"]}
    assert [row["LayerName"] for row in saved["layers"]] == final
    for level, row in zip([3,2,1,0], saved["layers"]):
        expected = dict(by_name[row["LayerName"]], LayerLevel=str(level))
        assert row == expected, (row, expected)
    assert saved["surrogates"] == original["surrogates"] and saved["models"] == original["models"], saved
    assert hashes(source) == m["sourceHashes"]
    assert digest(app) == m["appSha256"] and digest(helper) == m["helperSha256"]
    assert foreground.get('noInputHeld') is True and foreground.get('cleanupCompleted') is True
    assert foreground.get('inputOwnershipMaintained') is True
    assert foreground.get('mouseDownCount') == foreground.get('mouseUpCount') == 5
    assert foreground.get('heldButtonAfterCleanup') is False and foreground.get('heldKeyAfterCleanup') is False
    report.update(states=states,layerStates=observed,savedXML=saved)

def run(out,m):
    began=time.monotonic(); deadline=began+110
    report={'passed':False,'sourceExecutableSha256':m['privateApp']['sourceExecutableSha256'],
            'appSha256':m['appSha256'],'scope':m['scope'],'inputDeadlineSeconds':75,'allInDeadlineSeconds':110}
    process=driver=None; before=None; owned={}; error=None
    try:
        before=preferences()
        validate(out,m)
        inventory=subprocess.check_output(['/bin/ps','-A','-o','pid=,comm='],text=True,timeout=2)
        assert not any(line.strip().endswith('/Contents/MacOS/Vixen') or line.strip().endswith(m['helper']) for line in inventory.splitlines()), 'Existing Vixen/owned helper process'
        for name in ['ui','runtime']: (out/name).mkdir()
        env={k:v for k,v in os.environ.items() if not k.startswith(('VIXEN_','DYLD_'))}
        env.update(VIXEN_DATA=str(out/'profile'),VIXEN_MUTE='1',VIXEN_UI_TEST=str(out/'steps.json'),
            VIXEN_UI_TEST_OUT=str(out/'ui'),VIXEN_UI_TEST_TRACE='1',VIXEN_SETTINGS_DIR=str(out/'ui'),TMPDIR=str(out/'runtime')+'/')
        with (out/'process.log').open('w') as log, (out/'foreground.log').open('w') as dlog:
            ensure_launch_budget(deadline)
            process=subprocess.Popen([m['app']],env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
            owned.update(appPID=process.pid,appExecutable=m['app'],appProcessGroup=process.pid);save(out/'owned-processes.json',owned)
            ensure_launch_budget(deadline,94)
            driver=subprocess.Popen([m['helper'],str(process.pid),str(out),m['app']],stdout=dlog,stderr=subprocess.STDOUT,start_new_session=True)
            owned.update(driverPID=driver.pid,driverExecutable=m['helper'],driverProcessGroup=driver.pid);save(out/'owned-processes.json',owned)
            process_deadline=min(deadline-12,time.monotonic()+95)
            while process.poll() is None and time.monotonic()<process_deadline:
                if driver.poll() is not None:
                    if (out/'foreground.json').exists():
                        fg=json.loads((out/'foreground.json').read_text())
                        assert fg.get('passed') is True, 'Layer helper stopped: '+str(fg.get('error'))
                    else: raise AssertionError('Layer helper exited without cleanup evidence')
                time.sleep(.1)
            assert process.poll() is not None, 'App watchdog expired'
            driver.wait(timeout=min(4,max(.01,deadline-time.monotonic()-6)))
        report.update(exitCode=process.returncode,driverExitCode=driver.returncode)
        assert driver.returncode==0
        verify(out,m,process.returncode,report)
        report['passed']=True
    except Exception as exc:
        error=repr(exc);report['error']=error
    finally:
        cleanup_errors=[]
        # Stop app first so an intact helper can finish its guarded cleanup.
        for child in [process,driver]:
            try: terminate(child,deadline)
            except Exception as exc: cleanup_errors.append(repr(exc))
        if cleanup_errors: report['cleanupErrors']=cleanup_errors
        fgpath=out/'foreground.json'
        try: fg=json.loads(fgpath.read_text()) if fgpath.exists() else {}
        except Exception as exc: fg={};report['foregroundReadError']=repr(exc)
        report['foreground']=fg
        report.update(processExited=process is not None and process.poll() is not None,
            cleanupCompleted=not cleanup_errors and driver is not None and driver.poll() is not None and fg.get('cleanupCompleted') is True,
            noInputHeld=fg.get('noInputHeld') is True,restoredPrevious=fg.get('restoredPrevious') is True,
            ownerPreferenceHashesBefore=before)
        try:
            report['ownerPreferenceHashesAfter']=preferences()
            report['ownerPreferencesUnchanged']=before is not None and before==report['ownerPreferenceHashesAfter']
        except Exception as exc:
            report['ownerPreferencesUnchanged']=False;report['preferenceReadError']=repr(exc)
        try:
            report['inputsUnchanged']=all(digest(Path(p))==v for p,v in m['frozenHashes'].items()) and hashes(Path(m['source']))==m['sourceHashes']
            report['sourceUnchanged']=hashes(Path(m['source']))==m['sourceHashes']
            report['appUnchanged']=digest(Path(m['app']))==m['appSha256']
            report['driverUnchanged']=digest(Path(m['helper']))==m['helperSha256']
        except Exception as exc: report['unchangedCheckError']=repr(exc)
        report['elapsedSeconds']=time.monotonic()-began
        report['withinDeadline']=time.monotonic()<=deadline
        required=['processExited','cleanupCompleted','noInputHeld','restoredPrevious','ownerPreferencesUnchanged','inputsUnchanged','appUnchanged','driverUnchanged','withinDeadline']
        report['passed']=report['passed'] and all(report.get(k) is True for k in required)
        save(out/'result.json',report)
    print(json.dumps({'passed':report['passed'],'steps':report.get('steps'),'elapsedSeconds':report['elapsedSeconds'],'error':report.get('error')}))
    return 0 if report['passed'] else 1

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--out',type=Path,required=True)
    p.add_argument('--app',type=Path);p.add_argument('--profile',type=Path);p.add_argument('--helper',type=Path)
    g=p.add_mutually_exclusive_group(required=True);g.add_argument('--prepare-only',action='store_true');g.add_argument('--run-prepared',action='store_true')
    args=p.parse_args();out=args.out.resolve();assert out.is_relative_to(ROOT/'target')
    if args.prepare_only:
        assert args.app and args.profile and args.helper and not out.exists()
        subprocess.run([sys.executable,'-B',str(LEGACY),'--app',str(args.app.resolve()),'--profile',str(args.profile.resolve()),'--helper',str(args.helper.resolve()),'--out',str(out),'--prepare-only'],check=True,timeout=10)
        m=json.loads((out/'prepared.json').read_text());m['helper']=str(args.helper.resolve())
        frozen=[Path(__file__).resolve(),LEGACY,SOURCE,ROOT/'scripts/test_library_family_foreground.py',
            out/'steps.json',Path(m['app']),Path(m['helper']),args.app.resolve().parent/'private-app.json',args.app.resolve()/'Contents/Info.plist']
        m['frozenHashes']={str(v):digest(v) for v in frozen};m['preparedProfileHashes']=hashes(out/'profile')
        m.update(inputDeadlineSeconds=75,helperCleanupDeadlineSeconds=88,allInDeadlineSeconds=110,
            runnerSha256=digest(Path(__file__).resolve()),helperSourceSha256=digest(SOURCE),launchPerformed=False,
            scope=m['scope']+'; exact per-event window/deadline guards and paired-input cleanup; private Rust settings')
        save(out/'prepared.json',m);print(json.dumps({'prepared':str(out/'prepared.json'),'steps':m['steps'],'launched':False}));return 0
    return run(out,json.loads((out/'prepared.json').read_text()))

if __name__=='__main__':sys.exit(main())
