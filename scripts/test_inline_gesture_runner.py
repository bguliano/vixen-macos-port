#!/usr/bin/env python3
"""Exercise the actual foreground runner with native processes fully mocked."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('inline_runner',ROOT/'scripts/check_inline_gestures_foreground.py')
runner = importlib.util.module_from_spec(spec);spec.loader.exec_module(runner)


class InlineRunnerTests(unittest.TestCase):
    def run_case(self,mode):
        with tempfile.TemporaryDirectory(prefix='inline-runner-mock-',dir=ROOT/'target') as raw:
            out=Path(raw);exe=out/'InlineGestures.app/Contents/MacOS/InlineGestures';exe.parent.mkdir(parents=True)
            driver=out/'inline-gesture-driver';pin=out/'pin'
            for path in [exe,driver,pin]:path.write_text(path.name)
            m={'executable':str(exe),'driver':str(driver),'executableSha256':runner.digest(exe),
               'driverSha256':runner.digest(driver),'bundleIdentifier':'com.vixenlights.verification.mock',
               'pinnedInputHashes':{str(pin):runner.digest(pin)},
               'originalInputHashes':{str(Path(runner.__file__).resolve()):runner.digest(runner.__file__)},'scope':'mocked native boundaries only'}
            (out/'prepared.json').write_text(json.dumps(m))
            if mode=='changed-pin':pin.write_text('changed')
            made=[];signals=[];calls=[];clock=[100.0]
            fg={'passed':mode!='helper-failed','targetExited':True,'noInputHeld':mode!='held-input',
                'restoredPrevious':mode!='restore-failed','normalFinish':True,'observations':[{'name':'mock'}]}
            class Process:
                def __init__(self,command,**kwargs):
                    self.name='host' if command[0]==str(exe) else 'driver';self.pid=980000+len(made)
                    self.returncode=None;self.waited=0;made.append(self)
                    if self.name=='host':
                        (out/'events.json').write_text(json.dumps({'terminated':True,'testMode':False,
                            'current':{'commits':{'level':1,'point':1,'gradient':1},'tooltips':[]}}))
                    elif mode!='missing-helper-report':(out/'foreground.json').write_text(json.dumps(fg))
                def wait(self,timeout):
                    self.waited+=1
                    if mode=='helper-timeout' and self.name=='driver' and self.waited==1:raise subprocess.TimeoutExpired('driver',timeout)
                    self.returncode=0;return 0
                def poll(self):return self.returncode
                def terminate(self):signals.append((self.name,'terminate'));self.returncode=-15
                def kill(self):signals.append((self.name,'kill'));self.returncode=-9
            def run(command,**kwargs):
                calls.append(command)
                if mode=='validation-budget' and command[0]=='/usr/bin/codesign':clock[0]+=3
                if len(command)>1 and command[1]=='--desktop':
                    Path(command[2]).write_text(json.dumps({'accessibilityTrusted':mode!='permissions-missing',
                        'eventPostingPreflight':True,'application':{'pid':100,'bundleIdentifier':'previous.app','executable':'/previous'}}))
                return subprocess.CompletedProcess(command,0)
            error=None
            with patch.object(sys,'argv',['inline','--out',str(out),'--run-prepared']),patch.object(runner.time,'monotonic',lambda:clock[0]),patch.object(runner,'preferences',lambda:{'/mock-owner':'stable'}),patch.object(runner.subprocess,'Popen',Process),patch.object(runner.subprocess,'run',run),contextlib.redirect_stdout(io.StringIO()):
                try:runner.main()
                except (AssertionError,FileNotFoundError,subprocess.TimeoutExpired) as value:error=type(value).__name__
            p=out/'result.json';result=json.loads(p.read_text()) if p.exists() else None
            return result,error,made,signals,calls

    def test_success_requires_all_cleanup_fields(self):
        result,error,made,signals,_=self.run_case('pass')
        self.assertIsNone(error);self.assertTrue(result['passed']);self.assertEqual(len(made),2)
        self.assertTrue(result['processExited'] and result['driverExited'] and result['cleanupCompleted'])
        self.assertTrue(result['inputsUnchanged'] and result['appUnchanged'] and result['driverUnchanged'] and result['ownerPreferencesUnchanged'])
        self.assertEqual(signals,[])
    def test_missing_permissions_launches_no_host_or_input_helper(self):
        result,error,made,signals,_=self.run_case('permissions-missing')
        self.assertEqual(error,'AssertionError');self.assertFalse(result['passed']);self.assertEqual(made,[]);self.assertEqual(signals,[])
    def test_changed_frozen_pin_refuses_every_native_boundary(self):
        result,error,made,signals,calls=self.run_case('changed-pin')
        self.assertEqual(error,'AssertionError');self.assertIsNone(result);self.assertEqual(made,[]);self.assertEqual(calls,[])
    def test_validation_time_cannot_renew_native_budget(self):
        result,error,made,signals,calls=self.run_case('validation-budget')
        self.assertEqual(error,'AssertionError');self.assertFalse(result['passed']);self.assertEqual(made,[])
        self.assertFalse(any(len(c)>1 and c[1]=='--desktop' for c in calls))
        self.assertEqual(result['elapsedSeconds'],6)
    def test_helper_failure_never_becomes_acceptance(self):
        for mode in ['helper-failed','held-input','restore-failed','missing-helper-report']:
            with self.subTest(mode=mode):
                result,error,made,signals,_=self.run_case(mode)
                self.assertIsNotNone(error);self.assertFalse(result['passed']);self.assertEqual(result['failureKind'],'fatal')
                self.assertTrue(result['processExited'] and result['driverExited']);self.assertIn(('host','terminate'),signals)
    def test_timeout_records_cleanup_but_never_input_release_claim(self):
        result,error,made,signals,_=self.run_case('helper-timeout')
        self.assertEqual(error,'TimeoutExpired');self.assertFalse(result['passed'])
        self.assertTrue(result['processExited'] and result['driverExited'])
        self.assertIn(('driver','terminate'),signals);self.assertIn(('host','terminate'),signals)


if __name__=='__main__':unittest.main(verbosity=2)
