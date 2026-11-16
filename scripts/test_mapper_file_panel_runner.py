#!/usr/bin/env python3
"""Actual mapper runner with process/desktop boundaries mocked; no native execution."""
from pathlib import Path
import importlib.util
import json
import subprocess
import tempfile
import unittest
from unittest.mock import patch

ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('mapper_runner',ROOT/'scripts/check_mapper_file_panels.py')
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)

class RunnerTests(unittest.TestCase):
    def scenario(self, mode):
        with tempfile.TemporaryDirectory(prefix='mapper-runner-test-',dir=ROOT/'target') as folder:
            out=Path(folder);home=out/'fake-home';settings=home/'Library/Application Support/Vixen';settings.mkdir(parents=True)
            (settings/'Settings.xml').write_text('owner settings sentinel')
            prefs=home/'Library/Preferences';prefs.mkdir(parents=True)
            (prefs/'io.github.bguliano.vixen-macos-port.plist').write_text('owner preference sentinel')
            for name in ['host','helper','frozen-input','source-profile']:(out/name).write_text(name)
            profile=out/'profile';profile.mkdir();(profile/'source.xml').write_text('private profile')
            source=out/'Input Café.vmap';saved=out/'Saved Noël.vmap'
            doc={'id':'map-id','name':source.stem,'isDirty':True,'elementMappings':[{'sourceId':'source','sourceName':'Source A','targetId':'target','targetName':'Destination A','isDirty':True}]}
            source.write_text(json.dumps(doc));(out/'host-config.json').write_text('{}')
            manifest={'scope':'mocked native boundaries only','executable':str(out/'host'),'driver':str(out/'helper'),
                'bundleIdentifier':'com.vixenlights.verification.mapper.mock',
                'executableSha256':module.sha(out/'host'),'driverSha256':module.sha(out/'helper'),
                'originalInputHashes':{str(Path(module.__file__).resolve()):module.sha(module.__file__)},
                'pinnedInputHashes':{str(out/'frozen-input'):module.sha(out/'frozen-input')},
                'sourceProfileHashes':{str(out/'source-profile'):module.sha(out/'source-profile')},
                'profile':str(profile),'initialProfileHashes':module.hashes(profile),
                'sourceMap':str(source),'savedMap':str(saved),'sourceDocument':doc}
            if mode=='wrong-runner-pin':manifest['originalInputHashes'][str(Path(module.__file__).resolve())]='0'*64
            (out/'prepared.json').write_text(json.dumps(manifest))
            launched=[];signals=[];clock={'value':0.,'calls':0};processes={}
            def now():
                clock['calls']+=1
                if mode=='validation-budget' and clock['calls']>1:return 107.
                return clock['value']
            def evidence():
                saved.write_text(json.dumps({**doc,'name':saved.stem}));saved_hash=module.sha(saved)
                steps=[]
                for stage in ['initial','open-cancelled','save-cancelled','saved-live','original-reopened','saved-reopened']:
                    late=stage in ['saved-live','original-reopened','saved-reopened']
                    row={'stage':stage,'savedExists':late,'observedMapPath':str(saved if stage in ['saved-live','saved-reopened'] else source),'observedModified':False}
                    if late:row['savedSha256']=saved_hash
                    steps.append(row)
                fg={'pid':101,'passed':True,'targetExited':True,'noInputHeld':mode!='held-input','restoredPrevious':mode!='restore-failure','steps':steps}
                if mode!='missing-foreground':(out/'foreground.json').write_text(json.dumps(fg))
                rows=[{k:v for k,v in x.items() if k!='isDirty'} for x in doc['elementMappings']]
                events={'pid':101,'events':[{'event':'ready','webRunning':False,'testMode':False},{'event':'mapper-state','path':str(source),'modified':False,'rows':rows},{'event':'mapper-state','path':str(saved),'modified':False,'rows':rows},{'event':'mapper-completed','cancelled':True,'path':None},{'event':'shutdown-complete'},{'event':'terminated'}]}
                (out/'events.json').write_text(json.dumps(events))
                if mode=='missing-final-hash':(out/'frozen-input').unlink()
                if mode in ['normal-created-appdata','unexpected-created-file','changed-initial-profile']:
                    (profile/'VixenApplicationData.xml').write_text('<VixenApplicationData><DataFormatVersion>3</DataFormatVersion><RecentSequences/><FilterShapePositions/><FilterSetupFormHighQualityRendering value="false"/></VixenApplicationData>')
                if mode=='unexpected-created-file':(profile/'unexpected.xml').write_text('not permitted')
                if mode=='changed-initial-profile':(profile/'source.xml').write_text('changed')
            class Process:
                def __init__(self, command, **kwargs):
                    self.name=Path(command[0]).name;self.pid=101 if self.name=='host' else 102;self.returncode=None;self.calls=0
                    if self.name=='helper':
                        owned=json.loads((out/'owned-processes.json').read_text())
                        assert owned['appPID']==owned['appProcessGroup']==101 and 'driverPID' not in owned
                    launched.append(self.name);processes[self.pid]=self
                def wait(self, timeout):
                    self.calls+=1
                    if self.name=='helper' and self.calls==1:
                        owned=json.loads((out/'owned-processes.json').read_text())
                        assert owned['appPID']==owned['appProcessGroup']==101 and owned['driverPID']==owned['driverProcessGroup']==102
                        assert owned['appExecutable']==str(out/'host') and owned['driverExecutable']==str(out/'helper')
                        assert not (out/'.owned-processes.json.tmp').exists()
                        if mode in ['timeout','cleanup-failure']:raise subprocess.TimeoutExpired('helper',timeout)
                        evidence()
                        if mode=='over-budget':clock['value']=111.
                    if self.returncode is None:self.returncode=0
                    return self.returncode
                def poll(self):return self.returncode
            def kill(pid, sig):
                signals.append((pid,sig))
                if mode=='cleanup-failure' and pid==102:raise PermissionError('synthetic kill refusal')
                processes[pid].returncode=-int(sig)
            desktop={'accessibility':True,'eventPosting':True,'vixenProcesses':[],
                     'application':{'pid':55,'bundleIdentifier':'private.previous','executable':'/previous'}}
            error=None
            with patch.object(Path,'home',return_value=home),patch.object(module.time,'monotonic',side_effect=now),patch.object(module.subprocess,'Popen',Process),patch.object(module.subprocess,'check_output',return_value=json.dumps(desktop)),patch.object(module.os,'killpg',side_effect=kill):
                try:module.run(out)
                except (AssertionError,FileNotFoundError,subprocess.TimeoutExpired) as e:error=type(e).__name__
            result=json.loads((out/'result.json').read_text()) if (out/'result.json').exists() else None
            return result,error,launched,signals
    def test_normal_flow_has_all_strict_batch_aliases(self):
        r,e,launched,_=self.scenario('normal');self.assertIsNone(e);self.assertEqual(launched,['host','helper'])
        for key in ['passed','processExited','cleanupCompleted','noInputHeld','restoredPrevious','inputsUnchanged','appUnchanged','driverUnchanged','ownerPreferencesUnchanged','withinTotalBudget']:self.assertIs(r[key],True,key)
        self.assertEqual(r['finalizationErrors'],[])
    def test_normal_shutdown_default_creation_is_valid_but_not_unchanged(self):
        r,e,_,_=self.scenario('normal-created-appdata');self.assertIsNone(e)
        self.assertTrue(r['passed']);self.assertTrue(r['privateProfileValid']);self.assertFalse(r['privateProfileUnchanged'])
        self.assertEqual(list(r['privateProfileValidation']['allowedCreatedFiles']),['VixenApplicationData.xml'])
    def test_unexpected_outputs_or_changed_initial_files_fail_closed(self):
        for mode in ['unexpected-created-file','changed-initial-profile']:
            with self.subTest(mode=mode):
                r,e,_,_=self.scenario(mode);self.assertEqual(e,'AssertionError');self.assertFalse(r['passed']);self.assertFalse(r['privateProfileValid'])
    def test_missing_helper_evidence_fails_closed_and_writes_result(self):
        r,e,_,_=self.scenario('missing-foreground');self.assertEqual(e,'FileNotFoundError');self.assertFalse(r['passed']);self.assertFalse(r['noInputHeld']);self.assertFalse(r['restoredPrevious']);self.assertFalse(r['cleanupCompleted'])
    def test_held_input_and_failed_restoration_cannot_pass(self):
        for mode in ['held-input','restore-failure']:
            with self.subTest(mode=mode):
                r,e,_,_=self.scenario(mode);self.assertEqual(e,'AssertionError');self.assertFalse(r['passed']);self.assertFalse(r['cleanupCompleted'])
    def test_final_hash_failure_is_retained_instead_of_masking_result(self):
        r,e,_,_=self.scenario('missing-final-hash');self.assertEqual(e,'AssertionError');self.assertFalse(r['passed']);self.assertFalse(r['inputsUnchanged']);self.assertTrue(any(x['phase']=='final-hash' for x in r['finalizationErrors']))
    def test_timeout_cleans_owned_groups_and_preserves_failure(self):
        r,e,_,signals=self.scenario('timeout');self.assertEqual(e,'TimeoutExpired');self.assertFalse(r['passed']);self.assertTrue(r['processExited']);self.assertTrue(r['helperExited']);self.assertTrue(r['forcedCleanup']);self.assertEqual([x[0] for x in signals],[102,101])
    def test_cleanup_failure_does_not_erase_original_timeout(self):
        r,e,_,_=self.scenario('cleanup-failure');self.assertEqual(e,'TimeoutExpired');self.assertFalse(r['helperExited']);self.assertFalse(r['passed']);self.assertFalse(r['cleanupCompleted']);self.assertTrue(any(x['phase']=='helper-cleanup' for x in r['finalizationErrors']))
    def test_over_budget_normal_output_cannot_pass(self):
        r,e,_,_=self.scenario('over-budget');self.assertEqual(e,'AssertionError');self.assertEqual(r['elapsedSeconds'],111);self.assertFalse(r['withinTotalBudget']);self.assertFalse(r['passed'])
    def test_full_budget_required_before_native_launch(self):
        r,e,launched,_=self.scenario('validation-budget');self.assertEqual(e,'AssertionError');self.assertEqual(launched,[]);self.assertFalse(r['passed'])
    def test_runner_hash_change_prevents_any_native_launch(self):
        r,e,launched,_=self.scenario('wrong-runner-pin');self.assertEqual(e,'AssertionError');self.assertEqual(launched,[]);self.assertIsNone(r)

if __name__=='__main__':unittest.main(verbosity=2)
