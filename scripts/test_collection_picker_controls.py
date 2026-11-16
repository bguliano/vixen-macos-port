#!/usr/bin/env python3
"""Offline collection XML, saved checkpoint, profile and observation controls."""
import argparse,copy,json,shutil,tempfile,time,unittest
from pathlib import Path
from unittest.mock import patch
import xml.etree.ElementTree as ET
import test_collection_picker_physical as r
import collection_picker_settings_contract as s

class Controls(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory(prefix='collection-picker-controls-',dir=r.ROOT/'target');self.out=Path(self.temp.name);self.settings=self.out/'settings';shutil.copytree(r.SETTINGS,self.settings)
        self.file=self.settings/'ColorCollections.xml';self.profile=self.out/'profile';(self.profile/'SystemData').mkdir(parents=True)
        (self.profile/'SystemData/ModuleStore.xml').write_text('<ModuleStore/>');(self.profile/'Sequence').mkdir();self.open=self.profile/'Sequence/Editor Library.tim';self.open.write_text('<Sequence/>')
        self.m=dict(profileHashes=r.tree(self.profile),openFile=str(self.open),settingsHashes=r.tree(self.settings),inspector=str(r.INSPECTOR));self.expected_xml()
    def tearDown(self):self.temp.cleanup()
    def expected_xml(self):
        root=ET.parse(self.file).getroot();new=copy.deepcopy(root[0][0][0]);new.find(r.D+'value').text='4294901760';root[0][0].append(new);ET.ElementTree(root).write(self.file,encoding='utf-8',xml_declaration=True);self.good=self.file.read_bytes()
    def test_exact_numeric_schema_and_rust_loaded_values(self):
        self.assertEqual(r.inspect_saved(self.out,self.m)['collections'],r.EXPECTED)
        r.require_collections(r.SETTINGS/'ColorCollections.xml',r.INITIAL)
    def test_wrong_owner_order_description_color_representation_refused(self):
        variants=[lambda x:x[1][0].append(copy.deepcopy(x[0][0][-1])),lambda x:x[0][0].remove(x[0][0][-1]),lambda x:x[0][1].__setattr__('text','changed'),lambda x:x[0][0][-1].find(r.D+'state').__setattr__('text','1'),lambda x:x[0][0][-1].find(r.D+'value').__setattr__('text','4278190335'),lambda x:x.append(copy.deepcopy(x[0])),lambda x:x[0].append(ET.Element(r.T+'Unknown'))]
        for change in variants:
            root=ET.fromstring(self.good);change(root);ET.ElementTree(root).write(self.file,encoding='utf-8',xml_declaration=True)
            with self.subTest(change=str(change)),self.assertRaises(AssertionError):r.require_collections(self.file,r.EXPECTED)
    def test_xml_entities_comments_and_extra_attributes_refused(self):
        for content in [self.good.replace(b'<ns0:ArrayOfColorCollection',b'<!DOCTYPE x><ns0:ArrayOfColorCollection'),self.good.replace(b'<ns0:ColorCollection>',b'<ns0:ColorCollection unexpected="yes">',1),self.good+b'<!--comment-->']:
            self.file.write_bytes(content)
            with self.assertRaises(AssertionError):r.require_collections(self.file,r.EXPECTED)
    def request(self,name,**kw):r.save(self.out/('checkpoint-'+name+'.request'),dict(checkpoint=name,**kw))
    def test_checkpoint_success_and_byte_exact_cancel(self):
        completed={};pending={}
        with patch.object(r.common,'assert_services'):
            for name in r.CHECKPOINTS:
                self.request(name);r.service_checkpoints(self.out,self.m,completed,pending,time.monotonic()+20)
                self.assertTrue(completed[name]['passed'])
            self.file.write_bytes(self.good+b'\n')
            with self.assertRaisesRegex(AssertionError,'Cancel changed'):r.checkpoint_state(self.out,self.m,'cancelled',completed['accepted'])
    def test_checkpoint_order_schema_and_bad_state_refused(self):
        self.request('cancelled')
        with self.assertRaisesRegex(AssertionError,'Out-of-order'):r.service_checkpoints(self.out,self.m,{}, {},time.monotonic()+20)
        (self.out/'checkpoint-cancelled.request').unlink();self.request('accepted',foreign=True)
        with self.assertRaisesRegex(AssertionError,'Unexpected'):r.service_checkpoints(self.out,self.m,{}, {},time.monotonic()+20)
        self.request('accepted');shutil.copyfile(r.SETTINGS/'ColorCollections.xml',self.file);completed={};pending={'accepted':time.monotonic()-1}
        with self.assertRaisesRegex(AssertionError,'checkpoint failed'):r.service_checkpoints(self.out,self.m,completed,pending,time.monotonic()+20)
        self.assertFalse(completed);self.assertIs(json.loads((self.out/'checkpoint-accepted.json').read_text())['passed'],False)
    def normal_outputs(self):
        refs=json.loads(s.PROVENANCE.read_text())
        for name in refs['files']:shutil.copyfile(name,self.settings/Path(name).name)
        data=ET.Element('VixenApplicationData');ET.SubElement(data,'DataFormatVersion').text='3';recent=ET.SubElement(data,'RecentSequences');ET.SubElement(recent,'SequenceFile').text=str(self.open);ET.SubElement(data,'FilterShapePositions');ET.SubElement(data,'FilterSetupFormHighQualityRendering',value='false');ET.ElementTree(data).write(self.profile/'VixenApplicationData.xml',encoding='utf-8',xml_declaration=True)
    def test_normal_close_outputs_and_unknown_changed_missing_refused(self):
        self.normal_outputs();self.assertTrue(r.accept_profile(self.out,self.m)['passed'])
        cases=[(self.profile/'unexpected','unknown'),(self.profile/'.lock','stale'),(self.open,'changed'),(self.settings/'extra.xml','unexpected'),(self.settings/'Settings.xml','<settings/>'),(self.settings/'TimedSequenceEditorLayout.json','{}'),(self.settings/'profile_ToolStripState.xml','<bad/>'),(self.profile/'VixenApplicationData.xml','<VixenApplicationData/>')]
        for path,value in cases:
            prior=path.read_bytes() if path.exists() else None;path.write_text(value)
            with self.subTest(path=str(path)),self.assertRaises(AssertionError):r.accept_profile(self.out,self.m)
            if prior is None:path.unlink()
            else:path.write_bytes(prior)
        missing=self.settings/'Settings.xml';missing.unlink()
        with self.assertRaises(AssertionError):r.accept_profile(self.out,self.m)
    def fg(self):
        return dict(passed=True,collectionPickerWorkflow=True,normalQuitMenu=True,targetExited=True,restoredPrevious=True,restoredPointer=True,noInputHeld=True,inputCompleted=True,cleanupErrors=[],physicalAfterRestoration=dict(buttons=[],keys=[],modifiers=0),mouseDowns=0,mouseUps=0,keyDowns=4,keyUps=4,events=[dict(key=k,down=d) for k,d in [(55,True),(56,True),(5,True),(5,False),(56,False),(55,False),(36,True),(36,False)]],observations=[dict(stage=stage,before=before,selected=selected,nativeMenuItem=selected) for stage,before,selected in [('initial-A','','Collection A'),('pending-red-switch-B','Collection A','Collection B'),('reopened-A','','Collection A')]]+[dict(stage='color-hex',value=h) for h in ['FF0000','0000FF']]+[dict(stage='picker-start-hex',expected='FFFFFF',observed='FFFFFF')]*2+[dict(stage=x,action='AXPress',status=-25204) for x in ['accept-collections','discard-pending-blue','quit-menu']])
    def test_observed_native_sequence_and_cleanup_contract(self):
        fg=self.fg();fg['observations'].append(dict(stage='color-hex',action='AXConfirm',status=0));self.assertEqual(r.accept_observations(fg)['pointerEvents'],0)
        changes=[lambda x:x['observations'][0].__setitem__('before','Unrelated collection'),lambda x:x['events'].append(dict(kind=1)),lambda x:x.__setitem__('restoredPointer',False),lambda x:x['physicalAfterRestoration'].__setitem__('keys',[55]),lambda x:x['observations'][1].__setitem__('selected','Collection A'),lambda x:x['observations'][4].__setitem__('value','FF0000'),lambda x:x['observations'].__setitem__(-2,dict(stage='missing-quit-action'))]
        for change in changes:
            changed=copy.deepcopy(fg);change(changed)
            with self.assertRaises(AssertionError):r.accept_observations(changed)

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--report',type=Path,required=True);a=p.parse_args();result=unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(Controls));a.report.parent.mkdir(parents=True,exist_ok=True);r.save(a.report,dict(passed=result.wasSuccessful(),testsRun=result.testsRun,errors=len(result.errors),failures=len(result.failures),nativeOperations=0,scope='Offline consumer/checkpoint/normal-close profile and native-observation contracts; synthetic intended state is not physical evidence.',files={str(p):r.sha(p) for p in [Path(__file__).resolve(),Path(r.__file__),Path(s.__file__)]}));raise SystemExit(0 if result.wasSuccessful() else 1)
