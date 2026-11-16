#!/usr/bin/env python3
"""Exercise actual name/tab runner refusal and cleanup with all process calls mocked."""
import contextlib
import io
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
import test_layer_name_tab_foreground as runner

class Control(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory(dir=runner.ROOT/'target',prefix='layer-name-control-')
        self.out=Path(self.temp.name)
        self.m=json.loads((runner.ROOT/'target/layer-name-tab-candidate-v4/prepared.json').read_text())
        shutil.copytree(Path(self.m['source']),self.out/'profile')
        self.m['sequence']=str(self.out/'profile/Sequence/Layer Drag Verification.tim')
        self.argv=['runner','--app',self.m['app'],'--out',str(self.out),'--helper',self.m['helper'],'--profile',self.m['source'],'--run-prepared']
    def tearDown(self):self.temp.cleanup()
    def run_case(self):
        (self.out/'prepared.json').write_text(json.dumps(self.m))
        with patch.object(sys,'argv',self.argv),contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaises((AssertionError,subprocess.TimeoutExpired,OSError)):runner.main()
        return json.loads((self.out/'result.json').read_text())
    def test_changed_pin_refuses_all_launches_and_records_failure(self):
        self.m['pins'][self.m['helper']]='0'*64
        with patch.object(runner.subprocess,'Popen') as popen:
            r=self.run_case();popen.assert_not_called()
        self.assertFalse(r['passed']);self.assertFalse(r['inputsUnchanged']);self.assertTrue(r['ownerPreferencesUnchanged'])
    def test_expired_native_interval_refuses_app(self):
        with patch.object(runner.time,'monotonic',side_effect=[0,11,12,12]),patch.object(runner.subprocess,'Popen') as popen:
            r=self.run_case();popen.assert_not_called()
        self.assertIn('Insufficient',r['error']);self.assertFalse(r['noInputHeld']);self.assertTrue(r['inputsUnchanged'])
    def test_app_timeout_terminates_only_owned_children_and_retains_failure(self):
        class Child:
            def __init__(self,pid):self.pid=pid;self.returncode=None
            def poll(self):return self.returncode
            def wait(self,timeout):
                if self.returncode is None:raise subprocess.TimeoutExpired('mock-owned-child',timeout)
                return self.returncode
        app,driver=Child(99101),Child(99102);calls=[]
        def kill(pid,sig):
            calls.append((pid,sig));{app.pid:app,driver.pid:driver}[pid].returncode=-15
        with patch.object(runner.subprocess,'Popen',side_effect=[app,driver]),patch.object(runner.os,'killpg',side_effect=kill):
            r=self.run_case()
        self.assertEqual([p for p,s in calls],[app.pid,driver.pid]);self.assertFalse(r['passed'])
        self.assertTrue(r['processExited'] and r['driverExited']);self.assertFalse(r['noInputHeld']);self.assertFalse(r['restoredPrevious'])
    def test_launch_failure_still_records_report(self):
        with patch.object(runner.subprocess,'Popen',side_effect=OSError('mock startup refused')):
            r=self.run_case()
        self.assertIn('mock startup refused',r['error']);self.assertFalse(r['processExited']);self.assertTrue(r['inputsUnchanged'])

if __name__=='__main__':unittest.main()
