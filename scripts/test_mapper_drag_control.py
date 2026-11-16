#!/usr/bin/env python3
"""No native processes/input: Mapper driver runner and false-acceptance controls."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import test_mapper_drag_foreground as runner
ROOT=Path(__file__).resolve().parents[1]
BASE=ROOT/'target/mapper-drag-foreground-01'
class MapperControls(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory(prefix='mapper-controls-',dir=BASE);self.out=Path(self.temp.name)
        self.m=json.loads((BASE/'prepared.json').read_text());self.m['savedMap']=str(self.out/'saved.vmap')
        self.rows=[dict(ok=True,detail='') for _ in range(self.m['steps'])]
        for o in self.m['observations']:self.rows[o['index']]['detail']='name=Map modified=true tree=true rows=3 '+ '|'.join(f'Name[{k}]={v}' for k,v in o['expected'].items())
        self.rows[self.m['cancelResultIndex']]['detail']='cancelled'
        self.fg=dict(passed=True,inputCompleted=True,targetExited=True,restoredPrevious=True,restoredPointer=True,noInputHeld=True,cleanupErrors=[],mouseDowns=8,mouseUps=8,keyDowns=1,keyUps=1)
        for s in self.m['stages']:runner.write(self.out/('driver-'+s['marker']+'.json'),dict(completed=True,stage=s['marker']))
        names={'Source A':'','Source B':'Destination B','Source Group':'Destination Group'}
        self.saved=dict(name='Mapper Drag Saved',sourceTree=self.m['sourceTree'],elementMappings=[dict(sourceId=runner.IDS[n],sourceName=n,targetId=self.m['expectedSaved'][runner.IDS[n]],targetName=t) for n,t in names.items()]);runner.write(self.m['savedMap'],self.saved)
        self.fg.update(cancelDismissalObserved=True,cancelAXStatus=-25204,unsavedMappingBefore='Source B → Destination B',unsavedMappingAfter='Source B → Destination A')
        for key in ['savedHashBeforeUnsavedChange','savedHashAfterCancel','savedHashAfterReopen']:self.fg[key]=runner.sha(self.m['savedMap'])
    def tearDown(self):self.temp.cleanup()
    def test_complete_semantic_contract(self):self.assertTrue(runner.validate(self.out,self.m,self.fg,self.rows)['cancelledUnsavedMapping'])
    def test_blank_drop_mutation_is_rejected(self):
        o=self.m['observations'][3];self.rows[o['index']]['detail']=self.rows[o['index']]['detail'].replace(runner.IDS['Destination A'],runner.IDS['Destination B'])
        with self.assertRaises(AssertionError):runner.validate(self.out,self.m,self.fg,self.rows)
    def test_saved_target_corruption_is_rejected(self):
        self.saved['elementMappings'][1]['targetId']=runner.IDS['Destination A'];runner.write(self.m['savedMap'],self.saved)
        with self.assertRaises(AssertionError):runner.validate(self.out,self.m,self.fg,self.rows)
    def test_wrong_cancel_result_is_rejected(self):
        self.rows[self.m['cancelResultIndex']]['detail']='accepted'
        with self.assertRaises(AssertionError):runner.validate(self.out,self.m,self.fg,self.rows)
    def test_missing_native_dismissal_is_rejected(self):
        self.fg['cancelDismissalObserved']=False
        with self.assertRaises(AssertionError):runner.validate(self.out,self.m,self.fg,self.rows)
    def test_saved_file_change_after_cancel_is_rejected(self):
        self.fg['savedHashAfterCancel']='f'*64
        with self.assertRaises(AssertionError):runner.validate(self.out,self.m,self.fg,self.rows)
    def test_no_op_final_mutation_is_rejected(self):
        self.fg['unsavedMappingBefore']=self.fg['unsavedMappingAfter']
        with self.assertRaises(AssertionError):runner.validate(self.out,self.m,self.fg,self.rows)
    def test_held_input_is_rejected(self):
        self.fg['noInputHeld']=False
        with self.assertRaises(AssertionError):runner.validate(self.out,self.m,self.fg,self.rows)
    def test_missing_physical_drag_marker_is_rejected(self):
        (self.out/'driver-outline-group.json').unlink()
        with self.assertRaises(FileNotFoundError):runner.validate(self.out,self.m,self.fg,self.rows)
    def runtime_fixture(self):
        self.m.update(pins={},sourceProfileHashes={},profileHashes={},inputHashes={});runner.write(self.out/'prepared.json',self.m)
        return dict(application=dict(pid=1,bundleIdentifier='com.example.owner',executable='/owner'),pointer=[20,30],accessibility=True,eventPosting=True,noPhysicalInputHeld=True)
    def test_budget_refuses_app_launch(self):
        desktop=self.runtime_fixture()
        with patch.object(runner,'hashes',return_value={}),patch.object(runner,'sha',return_value=runner.D129),patch.object(runner,'owner_preferences',return_value={}),patch.object(runner.subprocess,'check_output',side_effect=['',json.dumps(desktop)]),patch.object(runner.subprocess,'Popen') as popen,patch.object(runner.time,'monotonic',side_effect=[0,20,20]):
            result=runner.run(self.out)
        self.assertFalse(result['passed']);self.assertEqual(popen.call_count,0);self.assertIn('budget',result['error'])
        self.assertTrue((self.out/'result.json').exists())
    def test_existing_native_helper_refuses_launch(self):
        self.runtime_fixture()
        with patch.object(runner,'hashes',return_value={}),patch.object(runner,'sha',return_value=runner.D129),patch.object(runner,'owner_preferences',return_value={}),patch.object(runner.subprocess,'check_output',return_value='1234 /private/foreground-other-check'),patch.object(runner.subprocess,'Popen') as popen,patch.object(runner.time,'monotonic',return_value=0):
            result=runner.run(self.out)
        self.assertFalse(result['passed']);self.assertEqual(popen.call_count,0);self.assertIn('foreground helper',result['error'])
    def test_initial_physical_input_refuses_launch(self):
        desktop=self.runtime_fixture();desktop['noPhysicalInputHeld']=False
        with patch.object(runner,'hashes',return_value={}),patch.object(runner,'sha',return_value=runner.D129),patch.object(runner,'owner_preferences',return_value={}),patch.object(runner.subprocess,'check_output',side_effect=['',json.dumps(desktop)]),patch.object(runner.subprocess,'Popen') as popen,patch.object(runner.time,'monotonic',return_value=0):
            result=runner.run(self.out)
        self.assertFalse(result['passed']);self.assertEqual(popen.call_count,0)
    def test_abort_write_failure_still_cleans_and_records(self):
        desktop=self.runtime_fixture();kills=[]
        class Child:
            def __init__(self,pid,fail=False):self.pid=pid;self.returncode=None;self.fail=fail
            def poll(self):
                if self.fail:self.fail=False;raise RuntimeError('injected monitor error')
                return self.returncode
            def wait(self,timeout):
                if self.returncode is None:self.returncode=0
                return self.returncode
        app,driver=Child(987650,True),Child(987651)
        def kill(pid,sig):self.assertEqual(pid,app.pid);kills.append(pid);app.returncode=-int(sig)
        original_write=runner.write
        def write(path,value):
            if Path(path).name=='abort-request.json':raise OSError('injected abort write error')
            original_write(path,value)
        with patch.object(runner,'hashes',return_value={}),patch.object(runner,'sha',return_value=runner.D129),patch.object(runner,'owner_preferences',return_value={}),patch.object(runner.subprocess,'check_output',side_effect=['',json.dumps(desktop)]),patch.object(runner.subprocess,'Popen',side_effect=[app,driver]),patch.object(runner.os,'killpg',side_effect=kill),patch.object(runner.time,'monotonic',return_value=0),patch.object(runner,'write',side_effect=write):
            result=runner.run(self.out)
        self.assertFalse(result['passed']);self.assertTrue(result['processExited']);self.assertTrue(result['driverExited']);self.assertEqual(kills,[app.pid]);self.assertTrue(any('abort write error' in e for e in result['finalizationErrors']));self.assertTrue((self.out/'result.json').exists())
    def test_driver_spawn_error_cleans_exact_owned_app(self):
        desktop=self.runtime_fixture();calls=[]
        class Child:
            pid=987654;returncode=None
            def poll(self):return self.returncode
            def wait(self,timeout):return self.returncode
        child=Child()
        def kill(pid,sig):self.assertEqual(pid,child.pid);calls.append((pid,int(sig)));child.returncode=-int(sig)
        with patch.object(runner,'hashes',return_value={}),patch.object(runner,'sha',return_value=runner.D129),patch.object(runner,'owner_preferences',return_value={}),patch.object(runner.subprocess,'check_output',side_effect=['',json.dumps(desktop)]),patch.object(runner.subprocess,'Popen',side_effect=[child,RuntimeError('injected helper spawn failure')]),patch.object(runner.os,'killpg',side_effect=kill),patch.object(runner.time,'monotonic',return_value=0):
            result=runner.run(self.out)
        self.assertFalse(result['passed']);self.assertTrue(result['processExited']);self.assertFalse(result['driverExited']);self.assertEqual(calls,[(child.pid,15)]);self.assertIn('spawn failure',result['error']);self.assertTrue((self.out/'result.json').exists())
if __name__=='__main__':unittest.main()
