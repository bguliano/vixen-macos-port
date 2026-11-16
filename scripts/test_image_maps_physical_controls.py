#!/usr/bin/env python3
"""Focused saved XML and physical-event consumer controls; no native operations."""
import copy,json,unittest
from pathlib import Path
import xml.etree.ElementTree as ET
import image_maps_physical_contract as c
ROOT=Path(__file__).resolve().parents[1]
BEFORE=ROOT/'target/image-maps-physical-helper-01/source-profile/SystemData/ModuleStore.xml'
OLD_AFTER=ROOT/'target/image-maps-native/rename-fixed-table-profile/SystemData/ModuleStore.xml'
class ContractControls(unittest.TestCase):
 def good(self):
  before=BEFORE.read_bytes();root=ET.fromstring(before);web=[m for m in root.iter('Module') if m.get('assemblyName')=='Module.App.WebServer'];assert len(web)==1
  after=OLD_AFTER.read_bytes().replace(b'</ModuleData>',ET.tostring(web[0])+b'</ModuleData>')
  c.require_store(before,after);return before,after
 def test_retained_actual_saved_rename(self):self.good()
 def test_cancelled_notes_change_refused(self):
  b,a=self.good()
  with self.assertRaises(AssertionError):c.require_store(b,a.replace(b'Original Rename Me',b'Discarded editor notes'))
 def test_wrong_reference_default_or_unrelated_value_refused(self):
  b,a=self.good()
  for mutation in [a.replace(b'>Renamed Map<',b'>Wrong Map<',1),a.replace(b'<IsDefaultMapping>true',b'<IsDefaultMapping>false'),a.replace(b'Original Active Map',b'Changed Active Map')]:
   with self.subTest(mutation=mutation[-40:]),self.assertRaises(AssertionError):c.require_store(b,mutation)
 def test_missing_or_extra_module_refused(self):
  b,a=self.good()
  for mutation in [a.replace(b'</ModuleData>',b'<Module /></ModuleData>'),a.replace(b'Module.App.WebServer',b'Module.App.Other')]:
   with self.assertRaises(AssertionError):c.require_store(b,mutation)
 def test_doubleclick_counts_are_required(self):
  # A complete synthetic transport record is distinct from native acceptance.
  f=dict(passed=True,inputCompleted=True,cleanupCompleted=True,noInputHeld=True,restoredPrevious=True,restoredPointer=True,ownershipLost=False,cleanupErrors=[],pendingKeyUpAcknowledgements=[],physicalAfterRestoration={'keys':[],'buttons':[],'modifiers':0},mouseDowns=7,mouseUps=7,keyDowns=2,keyUps=2,pendingMouseUp=False,doubleClickTiming={'elapsedSeconds':.2,'maximumSeconds':.5})
  kinds=[5,1,2]*5+[5,1,2,1,2];f['events']=[dict(kind=k,point=[100,100],window=1,clickCount=n) for k,n in zip(kinds,[1]*18+[2,2])]+[dict(key=k,down=d) for k,d in [(53,True),(53,False),(36,True),(36,False)]]
  f['observations']=[dict(stage=s,buttons=v) for s,v in c.BUTTONS.items()]+[dict(stage=s,value=v) for s,v in [('pending-rename-cancel','Discarded Rename'),('pending-rename-accept','Renamed Map'),('pending-editor-notes','Discarded editor notes'),('cancelled-editor-notes','Original Rename Me')]]+[dict(stage=s,moduleBytesUnchanged=True) for s in ['cancel-keeps-saved-bytes','editor-cancel-keeps-saved-bytes']]
  f['observations'] += [dict(stage='selected-name-delay',seconds=.8)]*3
  c.require_foreground(f)
  for mutate in ['click','cancel','selection','interference','pending','interval']:
   wrong=copy.deepcopy(f)
   if mutate=='click':wrong['events'][-5]['clickCount']=1
   if mutate=='cancel':next(o for o in wrong['observations'] if o['stage']=='pending-editor-notes')['value']='Original Rename Me'
   if mutate=='selection':wrong['observations'][4]['buttons']=[True,False,False,False]
   if mutate=='interference':wrong['ownershipLost']=True
   if mutate=='pending':wrong['pendingMouseUp']=True
   if mutate=='interval':wrong['doubleClickTiming']['elapsedSeconds']=.6
   with self.subTest(mutation=mutate),self.assertRaises(AssertionError):c.require_foreground(wrong)
if __name__=='__main__':unittest.main()
