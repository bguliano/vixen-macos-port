#!/usr/bin/env python3
"""Minimal copied-runner boundary controls: no native calls or real processes."""
import contextlib,io,json,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
import test_collection_picker_physical as r
class Controls(unittest.TestCase):
    def execute(self,mode):
        with tempfile.TemporaryDirectory(dir=r.ROOT/'target') as tmp:
            out=Path(tmp);m=json.loads((r.ROOT/'target/collection-picker-physical-02/prepared.json').read_text());r.save(out/'prepared.json',m);children=[];clock=[0.0];signals=[]
            desktop=dict(application=dict(pid=1,bundleIdentifier='com.example.owner',executable='/owner'),pointer=[100,100],accessibility=True,eventPosting=True,noPhysicalInputHeld=True)
            class Child:
                def __init__(self,cmd,**kw):
                    self.pid=999900+len(children);self.returncode=None;children.append(self)
                    if len(children)==1:
                        assert kw['env']['VIXEN_SETTINGS_DIR']==m['environment']['VIXEN_SETTINGS_DIR'] and 'VIXEN_UI_TEST' not in kw['env']
                        if mode=='slow-app':clock[0]+=16
                    else:
                        cfg=json.loads((out/'driver-config.json').read_text());assert cfg['workflow']=='collection-picker' and cfg['sequenceTitle']==m['sequenceTitle'];raise OSError('mock helper spawn failure')
                def poll(self):return self.returncode
                def wait(self,timeout):self.returncode=0;return 0
            def kill(pid,sig):
                assert pid in [c.pid for c in children];signals.append(pid)
                for c in children:
                    if c.pid==pid:c.returncode=-int(sig)
            def validate(*_):
                if mode=='expired':clock[0]+=11
            with patch.object(r,'validate_prepared',validate),patch.object(r,'owner_preferences',return_value={}),patch.object(r.subprocess,'check_output',side_effect=['',json.dumps(desktop)]),patch.object(r.subprocess,'Popen',Child),patch.object(r.os,'killpg',kill),patch.object(r.time,'monotonic',side_effect=lambda:clock[0]),contextlib.redirect_stdout(io.StringIO()):result=r.run(out)
            self.assertFalse(result['passed']);self.assertTrue((out/'result.json').exists());return len(children),signals
    def test_deadlines_prevent_late_app_or_helper_launch(self):
        self.assertEqual(self.execute('expired'),(0,[]));self.assertEqual(self.execute('slow-app'),(1,[999900]))
    def test_configuration_and_failed_helper_cleanup(self):self.assertEqual(self.execute('helper-fails'),(2,[999900]))
if __name__=='__main__':unittest.main(verbosity=2)
