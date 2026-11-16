#!/usr/bin/env python3
"""Offline native Export driver/checkpoint contract; no desktop or app launch."""
import argparse,copy,json,shutil,struct,tempfile,time,unittest,unicodedata
from pathlib import Path
from unittest.mock import patch
import xml.etree.ElementTree as ET
import test_sequence_export_physical as r

class Controls(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory(dir=r.ROOT/'target',prefix='export-physical-controls-');self.out=Path(self.temp.name);f=r.fixture.prepare(self.out/'fixture');self.m=dict(f,fixtureManifest=str(self.out/'fixture/prepared.json'),settings=str(self.out/'fixture/settings'))
    def tearDown(self):self.temp.cleanup()
    def synthetic_output(self,default=False):
        data=bytearray((r.ROOT/'target/export-sheet-native-final/profile/complete.csv').read_bytes());struct.pack_into('<I',data,14,60);Path(self.m['defaultExportFile' if default else 'exportFile']).write_bytes(data)
    def request(self,name,**extra):r.save(self.out/('checkpoint-'+name+'.request'),dict(checkpoint=name,**extra))
    def test_checkpoint_cancel_then_export_strict_native_file_oracle(self):
        complete={};pending={};self.request('cancelled');r.service_checkpoints(self.out,self.m,complete,pending,time.monotonic()+20);self.assertTrue(complete['cancelled']['passed'])
        self.synthetic_output();self.request('exported');r.service_checkpoints(self.out,self.m,complete,pending,time.monotonic()+20);self.assertEqual(complete['exported']['fseq']['frames'],60)
        self.synthetic_output(default=True);self.request('default-exported');r.service_checkpoints(self.out,self.m,complete,pending,time.monotonic()+20);self.assertEqual(complete['default-exported']['defaultFseq']['frames'],60)
        Path(self.m['cancelSentinel']).write_text('changed')
        with self.assertRaises(AssertionError):r.checkpoint_state(self.out,self.m,'exported')
    def test_checkpoint_bad_order_schema_and_invalid_file_never_acknowledged(self):
        self.request('exported')
        with self.assertRaisesRegex(AssertionError,'before Cancel'):r.service_checkpoints(self.out,self.m,{}, {},time.monotonic()+20)
        (self.out/'checkpoint-exported.request').unlink();self.request('cancelled',extra=True)
        with self.assertRaisesRegex(AssertionError,'Unexpected'):r.service_checkpoints(self.out,self.m,{}, {},time.monotonic()+20)
        self.request('cancelled');Path(self.m['exportFile']).write_bytes(b'incomplete');pending={'cancelled':time.monotonic()-1};completed={}
        with self.assertRaisesRegex(AssertionError,'checkpoint failed'):r.service_checkpoints(self.out,self.m,completed,pending,time.monotonic()+20)
        self.assertFalse(completed);self.assertIs(json.loads((self.out/'checkpoint-cancelled.json').read_text())['passed'],False)
    def fg(self):
        go=[dict(key=k,down=d) for k,d in [(55,True),(56,True),(5,True),(5,False),(56,False),(55,False),(36,True),(36,False)]];tab=[dict(key=48,down=True),dict(key=48,down=False)]
        obs=[dict(stage=s,startEnabled=True,stopEnabled=False,closeEnabled=True) for s in ['initial-export-idle','cancelled-export-idle','completed-export-idle','default-completed-export-idle']]
        obs += [dict(stage=s+'-offered-default',filename=Path(self.m['defaultExportFile']).name,expected=Path(self.m['defaultExportFile']).name,directories=['Export']) for s in ['cancel','save','default']]
        obs += [dict(stage=s+'-destination',directory=Path(p).parent.name,filename=Path(p).name) for s,p in [('cancel',self.m['cancelSentinel']),('save',self.m['exportFile'])]]
        obs += [dict(stage=s,action='AXPress',status=-25204) for s in ['start-cancelled-export','cancel-export-save','start-successful-export','confirm-export-save','start-default-export','confirm-default-export-save','close-export','quit-menu']]
        return dict(passed=True,sequenceExportWorkflow=True,normalQuitMenu=True,targetExited=True,restoredPrevious=True,restoredPointer=True,noInputHeld=True,inputCompleted=True,cleanupErrors=[],pendingKeyUpAcknowledgements=[],physicalAfterRestoration=dict(buttons=[],keys=[],modifiers=0),mouseDowns=0,mouseUps=0,keyDowns=14,keyUps=14,events=go+go+tab+go+tab,observations=obs)
    def test_native_event_path_idle_and_restoration_contract(self):
        fg=self.fg();self.assertEqual(r.accept_observations(fg,self.m)['keyEvents'],28)
        for row in fg['observations']:
            if 'filename' in row:
                row['filename']=unicodedata.normalize('NFD',row['filename'])
                if 'directory' in row:row['directory']=unicodedata.normalize('NFD',row['directory'])
        self.assertEqual(r.accept_observations(fg,self.m)['keyEvents'],28)
        for change in [lambda x:x.__setitem__('pendingKeyUpAcknowledgements',[55]),lambda x:x['events'].pop(),lambda x:x['events'].append(dict(kind=1)),lambda x:x.__setitem__('restoredPrevious',False),lambda x:x['observations'][1].__setitem__('stopEnabled',True),lambda x:x['observations'][4].__setitem__('filename','wrong.fseq'),lambda x:x['observations'].pop()]:
            modified=copy.deepcopy(fg);change(modified)
            with self.assertRaises(AssertionError):r.accept_observations(modified,self.m)
    def test_default_name_and_save_order_are_strict(self):
        fg=self.fg()
        for stage in ['cancel','save','default']:
            modified=copy.deepcopy(fg);row=next(o for o in modified['observations'] if o['stage']==stage+'-offered-default');row['filename']+='.fseq'
            with self.assertRaises(AssertionError):r.accept_observations(modified,self.m)
        self.request('default-exported')
        with self.assertRaisesRegex(AssertionError,'before explicit export'):r.service_checkpoints(self.out,self.m,{}, {},time.monotonic()+20)
    def test_explicit_export_mutation_between_saves_refused(self):
        self.synthetic_output()
        r.save(self.out/'checkpoint-exported.request',dict(checkpoint='exported'));exported=r.checkpoint_state(self.out,self.m,'exported')
        complete={'cancelled':{'passed':True},'exported':exported};self.synthetic_output(default=True)
        data=bytearray(Path(self.m['exportFile']).read_bytes());data[24]^=1;Path(self.m['exportFile']).write_bytes(data)
        self.request('default-exported')
        with self.assertRaisesRegex(AssertionError,'Explicit export changed'):r.service_checkpoints(self.out,self.m,complete,{'default-exported':time.monotonic()-1},time.monotonic()+20)
        self.assertNotIn('default-exported',complete)
    def test_normal_close_schema_and_unknown_outputs_refused(self):
        self.synthetic_output();self.synthetic_output(default=True)
        ref=json.loads(r.REFERENCE.read_text())
        for p in ref['files']:shutil.copyfile(p,Path(self.m['settings'])/Path(p).name)
        root=ET.Element('VixenApplicationData');ET.SubElement(root,'DataFormatVersion').text='3';recent=ET.SubElement(root,'RecentSequences');ET.SubElement(recent,'SequenceFile').text=unicodedata.normalize('NFD',self.m['openFile']);ET.SubElement(root,'FilterShapePositions');ET.SubElement(root,'FilterSetupFormHighQualityRendering',value='false');ET.ElementTree(root).write(Path(self.m['profile'])/'VixenApplicationData.xml',encoding='utf-8',xml_declaration=True)
        self.assertTrue(r.accept_profile(self.m)['passed'])
        for p in [Path(self.m['profile'])/'unexpected',Path(self.m['profile'])/'.lock',Path(self.m['settings'])/'unexpected.xml']:
            p.write_text('unexpected')
            with self.assertRaises(AssertionError):r.accept_profile(self.m)
            p.unlink()

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--report',type=Path,required=True);a=p.parse_args();result=unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(Controls));a.report.parent.mkdir(parents=True,exist_ok=True);r.save(a.report,dict(passed=result.wasSuccessful(),testsRun=result.testsRun,errors=len(result.errors),failures=len(result.failures),nativeOperations=0,scope='Offline consumer/checkpoint/normal-close and native-event contract. Synthesized intended60-frame file is derived from retained600-frame parser evidence, never native execution.',files={str(p):r.sha(p) for p in [Path(__file__).resolve(),Path(r.__file__)]}));raise SystemExit(0 if result.wasSuccessful() else 1)
