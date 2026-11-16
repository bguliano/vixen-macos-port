#!/usr/bin/env python3
"""Targeted real-runner lifecycle faults with mocked processes; zero native input."""
import argparse,contextlib,io,json,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
import test_editor_family_physical as r

class Clock:
    def __init__(self):self.now=0.
    def monotonic(self):return self.now
    def sleep(self,seconds):self.now+=seconds

class Controls(unittest.TestCase):
    def exercise(self,mode):
        with tempfile.TemporaryDirectory(dir=r.ROOT/'target',prefix='family-runner-controls-') as temp:
            out=Path(temp);(out/'profile').mkdir();(out/'profile/ColorPalette.xml').write_text('private')
            clock=Clock();children=[];actions=[];calls=[]
            m=dict(kind='gradient',app='/private/app',helper='/private/foreground-helper',source='/private/source',sourceHashes={},frozenHashes={},appSha256='fake',sourceExecutableSha256='fake',scope='mock-only')
            class Process:
                def __init__(self,cmd,**kw):
                    self.pid=888810+len(children);self.returncode=None;children.append(self);calls.append(cmd)
                    if len(children)==1:
                        assert cmd==['/private/app','no_controllers','no_execution']
                        assert kw['env']['VIXEN_SETTINGS_DIR']==str(out/'ui') and kw['env']['VIXEN_MUTE']=='1'
                        if mode=='pass':self.returncode=0
                    else:
                        if mode=='spawn-failure':raise OSError('mock driver spawn failure')
                        if mode=='pass':self.returncode=0
                        (out/'checkpoint-ready.request').write_text('unexpected' if mode=='bad-request' else 'ready')
                        (out/'foreground.json').write_text(json.dumps(dict(passed=mode=='pass',cleanupCompleted=True,noInputHeld=True,restoredPrevious=True)))
                def poll(self):return self.returncode
                def wait(self,timeout):
                    if self.pid==888811 and self.returncode is None:
                        if not (out/'abort-request.json').exists():raise r.subprocess.TimeoutExpired('mock helper',timeout)
                        actions.append('helper observed abort');self.returncode=1
                    else:self.returncode=0
                    return self.returncode
            def signal(pid,sig):
                actions.append(('kill',pid))
                for child in children:
                    if child.pid==pid:child.returncode=-15
            def validation(*args):
                if mode=='expired':clock.now=10.
            def checkpoint(*args):raise AssertionError('strict persisted state mismatch')
            original_save=r.save
            def save(path,value):
                if mode=='abort-write-failure' and path.name=='abort-request.json':raise OSError('mock abort I/O error')
                original_save(path,value)
            with patch.object(r,'validate',validation),patch.object(r,'verify',return_value=None),patch.object(r,'preferences',return_value={'private':'same'}),patch.object(r,'save',save),patch.object(r,'sha',return_value='same'),patch.object(r,'tree',return_value={}),patch.object(r.contract,'family_state',return_value={}),patch.object(r.contract,'accept_checkpoint',checkpoint),patch.object(r.time,'monotonic',clock.monotonic),patch.object(r.time,'sleep',clock.sleep),patch.object(r.subprocess,'check_output',return_value=''),patch.object(r.subprocess,'Popen',Process),patch.object(r.os,'killpg',signal),contextlib.redirect_stdout(io.StringIO()):code=r.run(out,m)
            result=json.loads((out/'result.json').read_text());return code,result,calls,actions,(out/'checkpoint-ready.json').exists()
    def test_private_success_and_expired_budget_no_launch(self):
        code,result,calls,_,_=self.exercise('pass');self.assertEqual(code,0);self.assertTrue(result['passed']);self.assertEqual(len(calls),2)
        code,result,calls,_,_=self.exercise('expired');self.assertEqual(code,1);self.assertFalse(calls);self.assertFalse(result['passed'])
    def test_checkpoint_failure_requests_helper_cleanup_before_app_termination(self):
        code,result,_,actions,acked=self.exercise('checkpoint-failure');self.assertEqual(code,1);self.assertFalse(acked)
        self.assertLess(actions.index('helper observed abort'),actions.index(('kill',888810)));self.assertTrue(result['noInputHeld']);self.assertTrue(result['processExited']);self.assertTrue(result['driverExited'])
    def test_abort_write_error_preserved_and_exact_owned_cleanup(self):
        code,result,_,actions,acked=self.exercise('abort-write-failure');self.assertEqual(code,1);self.assertFalse(acked)
        self.assertTrue(any('abort request' in e for e in result['cleanupErrors']));self.assertTrue(all(a[1] in [888810,888811] for a in actions if isinstance(a,tuple)))
    def test_malformed_request_and_helper_spawn_fail_closed(self):
        for mode in ['bad-request','spawn-failure']:
            code,result,_,_,acked=self.exercise(mode);self.assertEqual(code,1);self.assertFalse(acked);self.assertFalse(result['passed']);self.assertTrue(result['processExited'])

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--report',type=Path,required=True);a=p.parse_args();result=unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(Controls));a.report.parent.mkdir(parents=True,exist_ok=True)
    r.save(a.report,dict(passed=result.wasSuccessful(),testsRun=result.testsRun,errors=len(result.errors),failures=len(result.failures),nativeProcessesLaunched=0,scope='Actual Python runner with all processes/input mocked: cooperative helper-first cleanup, abort I/O error, strict request refusal and prelaunch budget.',sourcePins={str(p.resolve()):r.sha(p) for p in [Path(__file__),Path(r.__file__)]}));raise SystemExit(0 if result.wasSuccessful() else 1)
