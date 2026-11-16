#!/usr/bin/env python3
"""Offline native-discovery runner failures; all processes and desktop calls mocked."""
import contextlib,copy,io,json,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
import test_vixen2_physical as r
BASE=r.ROOT/'target/vixen2-physical-discovery-01'
FG=dict(passed=True,discoveryOnly=True,normalQuitMenu=True,targetExited=True,restoredPrevious=True,restoredPointer=True,noInputHeld=True,inputCompleted=True,cleanupErrors=[],mouseDowns=0,mouseUps=0,keyDowns=4,keyUps=4,events=[])
class Controls(unittest.TestCase):
    def fixture(self,path):
        m=json.loads((BASE/'prepared.json').read_text());r.shutil.copytree(BASE/'profile',path/'profile');(path/'settings').mkdir();m.update(profile=str(path/'profile'),settings=str(path/'settings'),pins={});r.save(path/'prepared.json',m)
        names=['Channel 1','Channel 6','V2 Fixture 1','Set Import Color']
        rows=[dict(title=n,description='',value='') for n in names]+[dict(title='',description=n,value='') for n in ['Vixen 2 channel mappings','Vixen 2 destination elements']]
        r.save(path/'mapper-discovery.ax.json',dict(rows=rows));r.save(path/'color-discovery.ax.json',dict(rows=[dict(title='Set Import Color')]))
        return m
    def test_actual_semantic_discovery_contract(self):
        with tempfile.TemporaryDirectory(dir=r.ROOT/'target') as tmp:
            out=Path(tmp);m=self.fixture(out);self.assertTrue(r.accept_discovery(out,m,dict(FG))['mapNotSaved'])
    def test_semantic_false_acceptance_refused(self):
        with tempfile.TemporaryDirectory(dir=r.ROOT/'target') as tmp:
            out=Path(tmp);m=self.fixture(out)
            for key in ['targetExited','restoredPrevious','restoredPointer','noInputHeld','normalQuitMenu','discoveryOnly']:
                fg=dict(FG);fg[key]=False
                with self.subTest(key=key),self.assertRaises(AssertionError):r.accept_discovery(out,m,fg)
            fg=dict(FG);fg['mouseDowns']=fg['mouseUps']=1
            with self.assertRaises(AssertionError):r.accept_discovery(out,m,fg)
    def test_web_enabled_fixture_refused(self):
        with tempfile.TemporaryDirectory(dir=r.ROOT/'target') as tmp:
            out=Path(tmp);self.fixture(out);p=out/'profile/SystemData/ModuleStore.xml';xml=r.ET.parse(p)
            for e in xml.getroot().iter():
                if r.local(e.tag)=='IsEnabled' and 'WebServer' in e.tag:e.text='true'
            xml.write(p)
            with self.assertRaises(AssertionError):r.assert_services(out/'profile')
    def test_saved_map_during_discovery_refused(self):
        with tempfile.TemporaryDirectory(dir=r.ROOT/'target') as tmp:
            out=Path(tmp);m=self.fixture(out);p=out/'profile/SystemData/ModuleStore.xml';xml=r.ET.parse(p);r.ET.SubElement(xml.getroot(),'Key').text=r.MAP_NAME;xml.write(p)
            with self.assertRaises(AssertionError):r.accept_discovery(out,m,dict(FG))
    def execute(self,mode):
        with tempfile.TemporaryDirectory(dir=r.ROOT/'target') as tmp:
            out=Path(tmp);m=self.fixture(out);children=[];signals=[];clock=[0.0]
            desktop=dict(application=dict(pid=1,bundleIdentifier='com.example.privateowner',executable='/owner'),pointer=[100,100],accessibility=True,eventPosting=True,noPhysicalInputHeld=True)
            class Child:
                def __init__(self,cmd,**kw):
                    self.pid=999900+len(children);self.returncode=None;children.append(self)
                    if len(children)==1:
                        assert kw['env']['VIXEN_SETTINGS_DIR']==m['environment']['VIXEN_SETTINGS_DIR']
                        assert 'VIXEN_UI_TEST' not in kw['env'] and cmd[1:]==['no_controllers','no_execution']
                        if mode=='app-slow':clock[0]+=16
                    else:
                        if mode=='helper-spawn':raise OSError('injected helper spawn')
                        fg=dict(FG)
                        if mode=='held':fg['noInputHeld']=False
                        r.save(out/'foreground.json',fg);self.returncode=0 if mode!='abort-io' else None
                        if mode!='abort-io':children[0].returncode=0
                def poll(self):
                    if mode=='abort-io' and self is children[0] and len(children)==2 and not hasattr(self,'injected'):self.injected=True;raise RuntimeError('monitor failed')
                    return self.returncode
                def wait(self,timeout):self.returncode=0;return 0
            def kill(pid,sig):
                self.assertIn(pid,[c.pid for c in children]);signals.append(pid)
                for c in children:
                    if c.pid==pid:c.returncode=-int(sig)
            def validate(*args):
                if mode=='budget':clock[0]+=11
                if mode=='invalid':raise AssertionError('invalid preparation')
            original=r.save
            def write(path,value):
                if mode=='abort-io' and Path(path).name=='abort-request.json':raise OSError('abort write failed')
                original(path,value)
            state={'calls':0}
            def prefs():
                state['calls']+=1
                if mode=='prefs-error' and state['calls']>1:raise OSError('read failed')
                return {'owner':'unchanged'}
            with patch.object(r,'validate_prepared',validate),patch.object(r,'owner_preferences',prefs),patch.object(r,'tree',return_value=m['sourceProfileHashes']),patch.object(r,'sha',return_value=m['sourceVixSha256']),patch.object(r,'save',write),patch.object(r.subprocess,'check_output',side_effect=['',json.dumps(desktop)]),patch.object(r.subprocess,'Popen',Child),patch.object(r.os,'killpg',kill),patch.object(r.time,'monotonic',side_effect=lambda:clock[0]),contextlib.redirect_stdout(io.StringIO()):result=r.run(out)
            self.assertTrue((out/'result.json').exists());return result,len(children),signals
    def test_normal_mocked_run_isolated(self):
        result,count,_=self.execute('success');self.assertTrue(result['passed']);self.assertEqual(count,2)
    def test_prelaunch_deadlines_and_invalid_preparation(self):
        for mode,count in [('budget',0),('invalid',0),('app-slow',1)]:
            with self.subTest(mode=mode):result,actual,_=self.execute(mode);self.assertFalse(result['passed']);self.assertEqual(actual,count)
    def test_failed_helper_spawn_cleans_owned_app(self):
        result,count,signals=self.execute('helper-spawn');self.assertFalse(result['passed']);self.assertEqual(signals,[999900]);self.assertTrue(result['processExited'])
    def test_abort_write_failure_still_cleans_and_reports(self):
        result,_,signals=self.execute('abort-io');self.assertFalse(result['passed']);self.assertTrue(result['processExited']);self.assertTrue(result['driverExited']);self.assertTrue(any('abort write failed' in e for e in result['cleanupErrors']));self.assertEqual(signals,[999900])
    def test_held_input_and_preference_error_fail_closed(self):
        for mode in ['held','prefs-error']:
            with self.subTest(mode=mode):result,_,_=self.execute(mode);self.assertFalse(result['passed'])
if __name__=='__main__':unittest.main(verbosity=2)
